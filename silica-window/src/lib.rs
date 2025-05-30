use std::sync::Arc;

use silica_gui::{
    theme::{Theme, ThemeLoader},
    Gui, GuiRenderer, Hotkey, Point,
};
use silica_wgpu::{wgpu, AdapterFeatures, Context, Surface, SurfaceSize, Texture, TextureConfig};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct KeyboardEvent;

impl silica_gui::KeyboardEvent for KeyboardEvent {
    fn to_hotkey(&self) -> Option<Hotkey> {
        todo!()
    }
}

struct MouseButtonEvent(MouseButton, ElementState);

impl silica_gui::MouseButtonEvent for MouseButtonEvent {
    fn is_primary_button(&self) -> bool {
        self.0 == MouseButton::Left
    }
    fn is_pressed(&self) -> bool {
        self.1.is_pressed()
    }
}

type InputEvent = silica_gui::InputEvent<KeyboardEvent, MouseButtonEvent>;

enum GuiRendererInit {
    Uninit(Option<(TextureConfig, Texture, Box<dyn Theme>)>),
    Init(GuiRenderer),
}

impl GuiRendererInit {
    fn init(&mut self, context: &Context, surface_format: wgpu::TextureFormat) {
        if let GuiRendererInit::Uninit(opt) = self {
            let (texture_config, theme_texture, theme) = opt.take().unwrap();
            *self = GuiRendererInit::Init(GuiRenderer::with_preloaded_theme(
                context,
                surface_format,
                &texture_config,
                theme_texture,
                theme,
            ));
        }
    }
    #[track_caller]
    fn unwrap(&mut self) -> &mut GuiRenderer {
        match self {
            GuiRendererInit::Uninit(..) => {
                panic!("attempted to unwrap an uninitialized GuiRenderer")
            }
            GuiRendererInit::Init(renderer) => renderer,
        }
    }
}

struct App {
    window: Option<Arc<Window>>,
    context: Context,
    surface: Surface,
    gui: Gui,
    gui_renderer: GuiRendererInit,
}

impl App {
    fn request_redraw_if_needed(&self) {
        if self.gui.dirty() {
            self.window.as_ref().unwrap().request_redraw();
        }
    }
    fn render(&mut self) {
        let gui_renderer = self.gui_renderer.unwrap();
        let frame = self.surface.acquire(&self.context);
        let view: wgpu::TextureView = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let clear_color = gui_renderer.background_color();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color.r as f64,
                            g: clear_color.g as f64,
                            b: clear_color.b as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            gui_renderer.render(&self.context, &mut pass, &mut self.gui);
        }
        self.context.queue.submit([encoder.finish()]);
        self.window.as_ref().unwrap().pre_present_notify();
        frame.present();
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let size = window.inner_size();
        self.window = Some(window.clone());
        self.surface.resume(
            &self.context,
            window,
            SurfaceSize::new(size.width, size.height),
        );
        self.gui_renderer
            .init(&self.context, self.surface.config().format);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.surface.suspend();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                let size = SurfaceSize::new(size.width, size.height);
                self.surface.resize(&self.context, size);
                self.gui_renderer
                    .unwrap()
                    .surface_resize(&self.context, size);
                self.gui.set_surface_size(size);
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (events, _) = self.gui.input_event(InputEvent::MouseMotion(Point::new(
                    position.x as f32,
                    position.y as f32,
                )));
                events.execute(&mut self.gui);
                self.request_redraw_if_needed();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let (events, _) = self
                    .gui
                    .input_event(InputEvent::MouseButton(MouseButtonEvent(button, state)));
                events.execute(&mut self.gui);
                self.request_redraw_if_needed();
            }
            _ => {}
        }
    }
}

pub fn run_app(gui: Gui, theme_loader: impl ThemeLoader) -> Result<(), EventLoopError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .filter_module("calloop", log::LevelFilter::Info)
        .filter_module("wgpu_core", log::LevelFilter::Info)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Info)
        .filter_module("cosmic_text", log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let context = Context::init(AdapterFeatures::default());
    let texture_config = TextureConfig::new(&context, wgpu::FilterMode::Linear);
    let theme_texture = theme_loader.load_texture(&context, &texture_config);
    let theme = theme_loader.load_theme();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        window: None,
        context,
        surface: Surface::new(),
        gui,
        gui_renderer: GuiRendererInit::Uninit(Some((texture_config, theme_texture, theme))),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
