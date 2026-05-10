mod gui;

use std::sync::Arc;

use silica_gui::{Hotkey, Point};
use silica_wgpu::{AdapterFeatures, Context, CurrentSurfaceTexture, Surface, SurfaceSize, wgpu};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, ModifiersState, PhysicalKey, SmolStr},
    window::WindowId,
};
pub use winit::{
    event::MouseButton,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard,
    window::{Icon, Window, WindowAttributes},
};

pub use crate::gui::*;

pub struct KeyboardEvent {
    state: ElementState,
    physical_key: KeyCode,
    text: Option<SmolStr>,
    modifiers: ModifiersState,
}

impl KeyboardEvent {
    pub fn is_pressed(&self) -> bool {
        self.state == ElementState::Pressed
    }
    pub fn physical_key(&self) -> KeyCode {
        self.physical_key
    }
}
impl silica_gui::KeyboardEvent for KeyboardEvent {
    fn to_hotkey(&self) -> Option<Hotkey> {
        if self.is_pressed() {
            self.text.as_ref().map(|text| Hotkey {
                key: text.chars().next().unwrap(),
                mod1: self.modifiers.control_key(),
                mod2: self.modifiers.alt_key(),
            })
        } else {
            None
        }
    }
}

pub struct MouseButtonEvent(MouseButton, ElementState);

impl MouseButtonEvent {
    pub fn button(&self) -> MouseButton {
        self.0
    }
}
impl silica_gui::MouseButtonEvent for MouseButtonEvent {
    fn is_primary_button(&self) -> bool {
        self.0 == MouseButton::Left
    }
    fn is_pressed(&self) -> bool {
        self.1.is_pressed()
    }
}

pub type InputEvent = silica_gui::InputEvent<KeyboardEvent, MouseButtonEvent>;

pub trait App {
    fn close_window(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.exit();
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize);
    fn input(&mut self, event_loop: &ActiveEventLoop, window: &Window, event: InputEvent);
    fn render(
        &mut self,
        event_loop: &ActiveEventLoop,
        context: &Context,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    );
}

// Drop order matters here, the program may crash on exit if Window is dropped before Surface.
pub struct WindowApp<T> {
    surface: Surface,
    window: Option<Arc<Window>>,
    window_attributes: WindowAttributes,
    context: Context,
    modifiers: ModifiersState,
    app: T,
}

impl<T: App> WindowApp<T> {
    pub fn new(context: Context, window_attributes: WindowAttributes, app: T) -> Self {
        WindowApp {
            surface: Surface::new(),
            window: None,
            window_attributes,
            context,
            modifiers: ModifiersState::empty(),
            app,
        }
    }
    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.window.as_ref().unwrap();
        let frame = match self.surface.acquire(&self.context) {
            CurrentSurfaceTexture::Render(frame) => frame,
            CurrentSurfaceTexture::Skip => return,
            CurrentSurfaceTexture::Recreate => {
                let size = window.inner_size();
                self.surface.resume(
                    &mut self.context,
                    window.clone(),
                    SurfaceSize::new(size.width, size.height),
                );
                return;
            }
        };
        let view: wgpu::TextureView = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.app.render(event_loop, &self.context, &view, &mut encoder);
        self.context.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        frame.present();
    }
}

impl<T: App> ApplicationHandler for WindowApp<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(self.window_attributes.clone()).unwrap());
        let size = window.inner_size();
        self.window = Some(window.clone());
        self.surface
            .resume(&mut self.context, window, SurfaceSize::new(size.width, size.height));
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.surface.suspend();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                self.app.close_window(event_loop);
            }
            WindowEvent::Resized(size) => {
                let size = SurfaceSize::new(size.width, size.height);
                self.surface.resize(&self.context, size);
                self.app.resize_window(&self.context, size);
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.render(event_loop);
                if event_loop.control_flow() == ControlFlow::Poll && !event_loop.exiting() {
                    self.window.as_ref().unwrap().request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.app.input(
                    event_loop,
                    window,
                    InputEvent::MouseMotion(Point::new(position.x as i32, position.y as i32)),
                );
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.app.input(
                    event_loop,
                    window,
                    InputEvent::MouseButton(MouseButtonEvent(button, state)),
                );
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if let PhysicalKey::Code(key_code) = event.physical_key
                    && !event.repeat
                {
                    self.app.input(
                        event_loop,
                        window,
                        InputEvent::Keyboard(KeyboardEvent {
                            state: event.state,
                            physical_key: key_code,
                            text: event.text,
                            modifiers: self.modifiers,
                        }),
                    );
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            _ => {}
        }
    }
}

pub trait EventLoopExt {
    fn create_context(&self, features: AdapterFeatures) -> Context;
    fn create_context_default_features(&self) -> Context;
}

impl EventLoopExt for EventLoop<()> {
    fn create_context(&self, features: AdapterFeatures) -> Context {
        Context::init(Box::new(self.owned_display_handle()), features)
    }
    fn create_context_default_features(&self) -> Context {
        self.create_context(AdapterFeatures::default())
    }
}
