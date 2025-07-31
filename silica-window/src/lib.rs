mod gui;

use std::sync::Arc;

use silica_gui::{Hotkey, Point};
use silica_wgpu::{Context, Surface, SurfaceSize, wgpu};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{ModifiersState, SmolStr},
    window::{Window, WindowId},
};

pub use crate::gui::*;

pub struct KeyboardEvent(ElementState, SmolStr, ModifiersState);

impl silica_gui::KeyboardEvent for KeyboardEvent {
    fn to_hotkey(&self) -> Option<Hotkey> {
        if self.0 == ElementState::Pressed {
            Some(Hotkey {
                key: self.1.chars().next().unwrap(),
                mod1: self.2.control_key(),
                mod2: self.2.alt_key(),
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

type InputEvent = silica_gui::InputEvent<KeyboardEvent, MouseButtonEvent>;

pub trait App {
    fn input_event(&mut self, window: &Window, event: InputEvent);
    fn resize(&mut self, context: &Context, size: SurfaceSize);
    fn render(
        &mut self,
        context: &Context,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    );
}

struct WindowApp<T> {
    window: Option<Arc<Window>>,
    context: Context,
    surface: Surface,
    modifiers: ModifiersState,
    app: T,
}

impl<T: App> WindowApp<T> {
    fn render(&mut self) {
        let frame = self.surface.acquire(&self.context);
        let view: wgpu::TextureView = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.app.render(&self.context, &view, &mut encoder);
        self.context.queue.submit([encoder.finish()]);
        self.window.as_ref().unwrap().pre_present_notify();
        frame.present();
    }
}

impl<T: App> ApplicationHandler for WindowApp<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
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
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                let size = SurfaceSize::new(size.width, size.height);
                self.surface.resize(&self.context, size);
                self.app.resize(&self.context, size);
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.app.input_event(
                    window,
                    InputEvent::MouseMotion(Point::new(position.x as i32, position.y as i32)),
                );
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.app.input_event(
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
                    if let Some(text) = event.text {
                        self.app.input_event(
                            window,
                            InputEvent::Keyboard(KeyboardEvent(event.state, text, self.modifiers)),
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

pub fn run_app<T: App>(context: Context, app: T) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut window_app = WindowApp {
        window: None,
        context,
        surface: Surface::new(),
        modifiers: ModifiersState::empty(),
        app,
    };
    event_loop.run_app(&mut window_app)?;
    Ok(())
}
