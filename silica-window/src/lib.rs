mod gui;

use std::sync::Arc;

use silica_gui::{Hotkey, Point};
use silica_wgpu::{Context, SurfaceSize, wgpu};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, ModifiersState, PhysicalKey, SmolStr},
    window::WindowId,
};
pub use winit::{
    event::MouseButton,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AppRequest {
    #[default]
    None,
    Redraw,
    Exit,
}

pub trait App {
    type Param;
    fn start(window: Arc<Window>, context: &Context, param: Self::Param) -> Self;
    fn close_window(&mut self) -> bool {
        true
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize);
    fn input(&mut self, event: InputEvent) -> AppRequest;
    fn render(&mut self, context: &Context, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder)
    -> AppRequest;
}

// Drop order matters here, the program may crash on exit if Window is dropped before Surface.
struct AppWindow<T> {
    app: T,
    context: Context,
    window: Arc<Window>,
}

impl<T: App> AppWindow<T> {
    fn render(&mut self) -> AppRequest {
        let frame = if let Some(frame) = self.context.get_surface_texture(&self.window) {
            frame
        } else {
            return AppRequest::Redraw;
        };
        let view: wgpu::TextureView = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .context
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let request = self.app.render(&self.context, &view, &mut encoder);
        self.context.queue().submit([encoder.finish()]);
        self.window.pre_present_notify();
        frame.present();
        request
    }
}

struct AppContainer<T: App> {
    param: Option<T::Param>,
    window: Option<AppWindow<T>>,
    window_attributes: WindowAttributes,
    modifiers: ModifiersState,
}

impl<T: App> AppContainer<T> {
    fn new(window_attributes: WindowAttributes, param: T::Param) -> Self {
        AppContainer {
            param: Some(param),
            window: None,
            window_attributes,
            modifiers: ModifiersState::empty(),
        }
    }
}

impl<T: App> ApplicationHandler for AppContainer<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(event_loop.create_window(self.window_attributes.clone()).unwrap());
        let size = window.inner_size();
        let display = Box::new(event_loop.owned_display_handle());
        let context = Context::init(display, window.clone(), SurfaceSize::new(size.width, size.height));
        let app = T::start(window.clone(), &context, self.param.take().unwrap());
        self.window = Some(AppWindow { app, context, window });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let window = self.window.as_mut().unwrap();
        let mut request = AppRequest::None;
        match event {
            WindowEvent::CloseRequested => {
                if window.app.close_window() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                let size = SurfaceSize::new(size.width, size.height);
                window.context.resize(size);
                window.app.resize_window(&window.context, size);
                window.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                request = window.render();
            }
            WindowEvent::CursorMoved { position, .. } => {
                request = window.app.input(InputEvent::MouseMotion(Point::new(
                    position.x as i32,
                    position.y as i32,
                )));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                request = window
                    .app
                    .input(InputEvent::MouseButton(MouseButtonEvent(button, state)));
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if let PhysicalKey::Code(key_code) = event.physical_key
                    && !event.repeat
                {
                    request = window.app.input(InputEvent::Keyboard(KeyboardEvent {
                        state: event.state,
                        physical_key: key_code,
                        text: event.text,
                        modifiers: self.modifiers,
                    }));
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            _ => {}
        }
        match request {
            AppRequest::None => (),
            AppRequest::Redraw => window.window.request_redraw(),
            AppRequest::Exit => event_loop.exit(),
        }
    }
}

pub fn run_app<T: App>(window_attributes: WindowAttributes, param: T::Param) {
    let event_loop = EventLoop::new().unwrap();
    let mut app: AppContainer<T> = AppContainer::new(window_attributes, param);
    event_loop.run_app(&mut app).unwrap();
}
