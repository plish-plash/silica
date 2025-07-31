use std::rc::Rc;

use silica_gui::{
    Gui, Point, Rect,
    render::GuiResources,
    theme::{StandardTheme, Theme},
};
use silica_wgpu::{Context, SurfaceSize, TextureConfig, wgpu};
use winit::{error::EventLoopError, event_loop::ActiveEventLoop, window::Window};

use crate::{App, InputEvent, run_app};

struct GuiApp {
    gui: Gui,
    texture_config: TextureConfig,
    theme: Rc<dyn Theme>,
    resources: Option<GuiResources>,
}

impl App for GuiApp {
    const RUN_CONTINUOUSLY: bool = false;
    fn resize_window(&mut self, context: &Context, size: SurfaceSize) {
        self.gui
            .set_area(Rect::new(Point::origin(), size.to_i32().cast_unit()));
        let resources = self.resources.get_or_insert_with(|| {
            GuiResources::new(context, &self.texture_config, self.theme.clone())
        });
        resources.surface_resize(context, size);
    }
    fn input(&mut self, event_loop: &ActiveEventLoop, window: &Window, event: InputEvent) {
        let (executor, _) = self.gui.handle_input(event);
        let redraw = executor.needs_redraw();
        executor.execute(&mut self.gui);
        if self.gui.exit_requested() {
            event_loop.exit();
        } else if redraw || self.gui.needs_layout() {
            window.request_redraw();
        }
    }
    fn render(
        &mut self,
        _event_loop: &ActiveEventLoop,
        context: &Context,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let resources = self.resources.as_mut().unwrap();
        let background_color = resources.background_color();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: background_color.r as f64,
                        g: background_color.g as f64,
                        b: background_color.b as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.gui.render(context, &mut pass, resources);
    }
}

pub fn run_gui_app(context: Context, gui: Gui, theme_data: &[u8]) -> Result<(), EventLoopError> {
    let texture_config = TextureConfig::new(&context, wgpu::FilterMode::Linear);
    let theme = Rc::new(StandardTheme::new(&context, &texture_config, theme_data));
    run_app(
        context,
        GuiApp {
            gui,
            texture_config,
            theme,
            resources: None,
        },
    )
}
