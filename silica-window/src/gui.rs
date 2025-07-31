use std::rc::Rc;

use silica_gui::{
    Gui, Point, Rect,
    render::GuiResources,
    theme::{StandardTheme, Theme},
};
use silica_wgpu::{AdapterFeatures, Context, SurfaceSize, TextureConfig, wgpu};
use winit::{error::EventLoopError, window::Window};

use crate::{App, InputEvent, run_app};

struct GuiApplication {
    gui: Gui,
    texture_config: TextureConfig,
    theme: Rc<dyn Theme>,
    resources: Option<GuiResources>,
}

impl App for GuiApplication {
    fn input_event(&mut self, window: &Window, event: InputEvent) {
        let (executor, _) = self.gui.input_event(event);
        let redraw = executor.needs_redraw();
        executor.execute(&mut self.gui);
        if redraw || self.gui.needs_layout() {
            window.request_redraw();
        }
    }
    fn resize(&mut self, context: &Context, size: SurfaceSize) {
        self.gui
            .set_area(Rect::new(Point::origin(), size.to_i32().cast_unit()));
        let resources = self.resources.get_or_insert_with(|| {
            GuiResources::new(context, &self.texture_config, self.theme.clone())
        });
        resources.surface_resize(context, size);
    }
    fn render(
        &mut self,
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

pub fn run_gui_app(gui: Gui, theme_data: &[u8]) -> Result<(), EventLoopError> {
    let context = Context::init(AdapterFeatures::default());
    let texture_config = TextureConfig::new(&context, wgpu::FilterMode::Linear);
    let theme = Rc::new(StandardTheme::new(&context, &texture_config, theme_data));
    run_app(
        context,
        GuiApplication {
            gui,
            texture_config,
            theme,
            resources: None,
        },
    )
}
