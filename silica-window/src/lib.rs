mod gui;

use std::sync::Arc;

use silica_gui::{Hotkey, Point};
use silica_wgpu::{Context, Surface, SurfaceSize, wgpu};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey, SmolStr},
    window::WindowId,
};
pub use winit::{
    event_loop::ActiveEventLoop,
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
    const RUN_CONTINUOUSLY: bool;
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

struct WindowApp<T> {
    window_attributes: WindowAttributes,
    window: Option<Arc<Window>>,
    context: Context,
    surface: Surface,
    modifiers: ModifiersState,
    app: T,
}

impl<T: App> WindowApp<T> {
    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let frame = self.surface.acquire(&self.context);
        let view: wgpu::TextureView = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.app
            .render(event_loop, &self.context, &view, &mut encoder);
        self.context.queue.submit([encoder.finish()]);
        self.window.as_ref().unwrap().pre_present_notify();
        frame.present();
    }
}

impl<T: App> ApplicationHandler for WindowApp<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes.clone())
                .unwrap(),
        );
        let size = window.inner_size();
        self.window = Some(window.clone());
        self.surface.resume(
            &mut self.context,
            window,
            SurfaceSize::new(size.width, size.height),
        );
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
                if T::RUN_CONTINUOUSLY && !event_loop.exiting() {
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
                if !event.repeat {
                    if let PhysicalKey::Code(key_code) = event.physical_key {
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
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            _ => {}
        }
    }
}

pub fn run_app<T: App>(
    window_attributes: WindowAttributes,
    context: Context,
    app: T,
) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(if T::RUN_CONTINUOUSLY {
        ControlFlow::Poll
    } else {
        ControlFlow::Wait
    });
    let mut window_app = WindowApp {
        window_attributes,
        window: None,
        context,
        surface: Surface::new(),
        modifiers: ModifiersState::empty(),
        app,
    };
    event_loop.run_app(&mut window_app)?;
    Ok(())
}
