pub mod locale;
pub mod particles;
pub mod texture;
pub mod util;
pub mod world2d;

use std::{rc::Rc, time::Instant};

pub use euclid as math;
pub use silica_asset as asset;
pub use silica_asset::AssetError;
pub use silica_env::{AppInfo, app_info};
pub use silica_gui as gui;
pub use silica_gui::Rgba;
use silica_gui::{Gui, Theme};
pub use silica_wgpu as render;
use silica_wgpu::{AdapterFeatures, Context, SurfaceSize, wgpu};
pub use silica_window::{
    ActiveEventLoop as EventLoop, Icon, InputEvent, KeyboardEvent, MouseButton, MouseButtonEvent, Window,
    WindowAttributes, keyboard,
};
use silica_window::{App, run_app, run_gui_app};

pub struct LocalSpace;
pub struct WorldSpace;
pub struct ScreenSpace;

pub type GameAssets = silica_asset::DirectorySource;

pub trait Game: Sized {
    fn window_attributes() -> WindowAttributes;
    fn load(assets: GameAssets, context: &Context) -> Result<Self, AssetError>;
    fn close_window(&mut self) -> bool {
        true
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize);
    fn input(&mut self, event: InputEvent);
    fn update(&mut self, event_loop: &EventLoop, dt: f32);
    fn clear_color(&self) -> Rgba;
    fn render(&mut self, context: &Context, pass: &mut wgpu::RenderPass);
}

struct GameApp<T> {
    game: T,
    last_update: Instant,
}

impl<T: Game> App for GameApp<T> {
    const RUN_CONTINUOUSLY: bool = true;
    fn close_window(&mut self, event_loop: &EventLoop) {
        if self.game.close_window() {
            event_loop.exit();
        }
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize) {
        self.game.resize_window(context, size);
    }
    fn input(&mut self, _event_loop: &EventLoop, _window: &Window, event: InputEvent) {
        self.game.input(event);
    }
    fn render(
        &mut self,
        event_loop: &EventLoop,
        context: &Context,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32();
        self.last_update = now;
        self.game.update(event_loop, dt);

        let clear_color = self.game.clear_color();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        self.game.render(context, &mut pass);
    }
}

fn error_gui(theme: Rc<dyn Theme>, error: AssetError) -> Gui {
    use gui::*;
    let error = error.to_string();
    log::error!("{error}");
    let mut gui = Gui::new(theme);
    let root = NodeBuilder::new()
        .modify_style(|style| {
            style.layout = Layout::Stack;
            style.main_align = Align::Center;
            style.cross_align = Align::Center;
        })
        .child(
            NodeBuilder::new()
                .modify_style(|style| {
                    style.direction = Direction::Column;
                    style.cross_align = Align::Center;
                    style.border = SideOffsets::new_all_same(1);
                    style.padding = SideOffsets::new(16, 8, 16, 8);
                    style.gap = 16;
                })
                .child({
                    let label = LabelBuilder::new(&error)
                        .font_size(20.0)
                        .align(TextAlign::Center)
                        .build_label(&gui);
                    NodeBuilder::new()
                        .modify_style(|style| style.max_size.width = 480)
                        .build_widget(&mut gui, label)
                })
                .child(
                    ButtonBuilder::new()
                        .label(&mut gui, "Exit")
                        .button_style(ButtonStyle::Delete)
                        .build(&mut gui, |gui: &mut Gui| gui.request_exit()),
                )
                .build(&mut gui),
        )
        .build(&mut gui);
    gui.set_root(root);
    gui
}

pub fn run_game<T: Game>(app_info: AppInfo) {
    silica_env::setup_env(app_info);
    let context = Context::init(AdapterFeatures::default());
    match T::load(GameAssets::new("assets".into()), &context) {
        Ok(game) => run_app(
            T::window_attributes(),
            context,
            GameApp {
                game,
                last_update: Instant::now(),
            },
        ),
        Err(error) => run_gui_app(T::window_attributes(), context, "assets/theme", |theme| {
            error_gui(theme, error)
        }),
    }
    .unwrap()
}
