pub mod audio;
pub mod locale;
pub mod particles;
pub mod util;
pub mod world2d;

use std::{
    io::{Error as IoError, ErrorKind},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

pub use euclid as math;
use serde::{Serialize, de::DeserializeOwned};
pub use silica_asset as asset;
pub use silica_asset::AssetError;
pub use silica_env::{AppInfo, app_info};
pub use silica_gui as gui;
pub use silica_gui::{Rgba, Rgba32Gamma};
use silica_gui::{Gui, Theme, theme::StandardTheme};
pub use silica_wgpu as render;
use silica_wgpu::{Context, SurfaceSize, TextureConfig, wgpu};
use silica_window::{App, AppRequest, GuiApp, run_app};
pub use silica_window::{
    Icon, InputEvent, KeyboardEvent, MouseButton, MouseButtonEvent, Window, WindowAttributes, keyboard,
};

pub struct LocalSpace;
pub struct WorldSpace;
pub struct ScreenSpace;

pub type GameAssets = silica_asset::DirectorySource;

#[derive(Debug, Clone, Copy)]
pub enum GameLoop {
    Run,
    Exit,
}

impl GameLoop {
    fn to_app_request(self) -> AppRequest {
        match self {
            GameLoop::Run => AppRequest::Redraw,
            GameLoop::Exit => AppRequest::Exit,
        }
    }
}

pub trait Game: Sized {
    fn window_attributes() -> WindowAttributes;
    fn load(window: Arc<Window>, context: &Context, assets: GameAssets) -> Result<Self, AssetError>;
    fn close_window(&mut self) -> bool {
        true
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize);
    fn input(&mut self, event: InputEvent);
    fn update(&mut self, dt: f32) -> GameLoop;
    fn clear_color(&self) -> Option<Rgba>;
    fn render(&mut self, context: &Context, pass: &mut wgpu::RenderPass);
}

struct GameApp<T> {
    game: Result<T, GuiApp>,
    last_update: Instant,
}

impl<T: Game> App for GameApp<T> {
    type Param = ();
    fn start(window: Arc<Window>, context: &Context, _param: Self::Param) -> Self {
        let game = T::load(window.clone(), context, GameAssets::new("assets".into())).map_err(|error| {
            GuiApp::start(
                window,
                context,
                ("assets/theme".into(), Box::new(move |gui| error_gui(gui, error))),
            )
        });
        GameApp {
            game,
            last_update: Instant::now(),
        }
    }
    fn close_window(&mut self) -> bool {
        match &mut self.game {
            Ok(game) => game.close_window(),
            Err(gui) => gui.close_window(),
        }
    }
    fn resize_window(&mut self, context: &Context, size: SurfaceSize) {
        match &mut self.game {
            Ok(game) => game.resize_window(context, size),
            Err(gui) => gui.resize_window(context, size),
        }
    }
    fn input(&mut self, event: InputEvent) -> AppRequest {
        match &mut self.game {
            Ok(game) => {
                game.input(event);
                AppRequest::None
            }
            Err(gui) => gui.input(event),
        }
    }
    fn render(
        &mut self,
        context: &Context,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> AppRequest {
        match &mut self.game {
            Ok(game) => {
                let now = Instant::now();
                let dt = (now - self.last_update).as_secs_f32();
                self.last_update = now;
                let game_loop = game.update(dt);

                let load_op = if let Some(clear_color) = game.clear_color() {
                    wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_color.r as f64,
                        g: clear_color.g as f64,
                        b: clear_color.b as f64,
                        a: 1.0,
                    })
                } else {
                    unsafe { wgpu::LoadOp::DontCare(wgpu::LoadOpDontCare::enabled()) }
                };
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                game.render(context, &mut pass);
                game_loop.to_app_request()
            }
            Err(gui) => gui.render(context, view, encoder),
        }
    }
}

fn error_gui(gui: &mut Gui, error: AssetError) -> gui::NodeId {
    use gui::*;
    let error = error.to_string();
    log::error!("{error}");
    NodeBuilder::new()
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
                        .build_label(gui);
                    NodeBuilder::new()
                        .modify_style(|style| style.max_size.width = 480)
                        .build_widget(gui, label)
                })
                .child(
                    ButtonBuilder::new()
                        .label(gui, "Exit")
                        .button_style(ButtonStyle::Delete)
                        .build(gui, |mut context| context.gui().request_exit()),
                )
                .build(gui),
        )
        .build(gui)
}

pub fn run_game<T: Game>(app_info: AppInfo) {
    silica_env::setup_env(app_info);
    run_app::<GameApp<T>>(T::window_attributes(), ());
}

pub fn load_gui_theme(
    context: &Context,
    texture_config: &TextureConfig,
    assets: &mut GameAssets,
) -> Result<Rc<dyn Theme>, AssetError> {
    let theme = StandardTheme::load(
        context,
        texture_config,
        &mut asset::SubdirectorySource::new(assets, "theme".into()),
    )?;
    Ok(Rc::new(theme))
}

pub fn load_data<T: DeserializeOwned>(path: &str) -> Result<T, IoError> {
    let buf = std::fs::read(path)?;
    postcard::from_bytes(&buf).map_err(|e| IoError::new(ErrorKind::InvalidData, e))
}

pub fn save_data<T: Serialize>(path: &str, data: &T) -> Result<(), IoError> {
    let buf = postcard::to_stdvec(data).map_err(IoError::other)?;
    std::fs::write(path, buf)
}
