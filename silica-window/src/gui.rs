use std::{path::PathBuf, rc::Rc, sync::Arc};

use silica_asset::DirectorySource;
use silica_gui::{Gui, NodeId, Point, Rect, render::GuiResources, theme::StandardTheme};
use silica_wgpu::{Context, SurfaceSize, TextureConfig, wgpu};
use winit::window::{Window, WindowAttributes};

use crate::{App, AppRequest, InputEvent};

pub struct GuiApp {
    gui: Gui,
    resources: GuiResources,
}

impl App for GuiApp {
    type Param = (PathBuf, Box<dyn FnOnce(&mut Gui) -> NodeId>);
    fn start(_window: Arc<Window>, context: &Context, (theme, build_gui): Self::Param) -> Self {
        let texture_config = TextureConfig::new(context, wgpu::FilterMode::Linear);
        let resources = GuiResources::new(context, &texture_config);
        let theme = StandardTheme::load(context, &texture_config, &mut DirectorySource::new(theme)).unwrap();
        let mut gui = Gui::new(Rc::new(theme));
        let root = build_gui(&mut gui);
        gui.set_root(root);
        GuiApp { gui, resources }
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize) {
        self.gui.set_area(Rect::new(Point::origin(), size.to_i32().cast_unit()));
        self.resources.surface_resize(context, size);
    }
    fn input(&mut self, event: InputEvent) -> AppRequest {
        let (executor, _) = self.gui.handle_input(event);
        let redraw = executor.needs_redraw();
        executor.execute(&mut self.gui);
        if self.gui.exit_requested() {
            AppRequest::Exit
        } else if redraw || self.gui.needs_layout() {
            AppRequest::Redraw
        } else {
            AppRequest::None
        }
    }
    fn render(
        &mut self,
        context: &Context,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> AppRequest {
        let background_color = self.gui.background_color();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
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
            multiview_mask: None,
        });
        self.gui.render(context, &mut pass, &mut self.resources);
        AppRequest::None
    }
}

pub fn run_gui_app<F>(window_attributes: WindowAttributes, theme: &str, f: F)
where
    F: FnOnce(&mut Gui) -> NodeId + 'static,
{
    crate::run_app::<GuiApp>(window_attributes, (theme.into(), Box::new(f)));
}
