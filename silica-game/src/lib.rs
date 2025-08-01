mod error;
mod image;
pub mod locale;
pub mod particles;
pub mod util;
pub mod world2d;

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

pub use euclid as math;
pub use silica_env::{AppInfo, app_info};
pub use silica_gui as gui;
pub use silica_gui::Rgba;
use silica_gui::{
    FontSystem,
    glyphon::fontdb,
    theme::{StandardTheme, Theme},
};
pub use silica_wgpu as render;
use silica_wgpu::{AdapterFeatures, Context, SurfaceSize, TextureConfig, wgpu};
pub use silica_window::{
    ActiveEventLoop as EventLoop, Icon, InputEvent, KeyboardEvent, MouseButtonEvent, Window,
    WindowAttributes, keyboard,
};
use silica_window::{App, run_app, run_gui_app};

pub use crate::{
    error::{GameError, ResultExt},
    image::*,
};

pub struct LocalSpace;
pub struct WorldSpace;
pub struct ScreenSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetPath(pub &'static str);

impl AssetPath {
    pub fn path(&self) -> PathBuf {
        Path::new("assets").join(self.0)
    }
}
impl std::fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

pub fn load_asset<T, F>(path: AssetPath, f: F) -> Result<T, GameError>
where
    F: FnOnce(&Path) -> Result<T, GameError>,
{
    log::debug!("Loading asset {path}");
    load_file(path.path(), f)
}
pub fn load_file<P, T, F>(path: P, f: F) -> Result<T, GameError>
where
    P: AsRef<Path>,
    F: FnOnce(&Path) -> Result<T, GameError>,
{
    let path = path.as_ref();
    // log::debug!("Loading file {}", path.display());
    f(path).map_err(|e| e.with_read(path.to_path_buf()))
}
pub fn save_file<P, T, F>(path: P, f: F) -> Result<T, GameError>
where
    P: AsRef<Path>,
    F: FnOnce(&Path) -> Result<T, GameError>,
{
    let path = path.as_ref();
    log::debug!("Saving file {}", path.display());
    f(path).map_err(|e| e.with_write(path.to_path_buf()))
}

pub fn load_asset_directory<F>(path: AssetPath, mut f: F) -> Result<(), GameError>
where
    F: FnMut(&Path) -> Result<(), GameError>,
{
    let asset_path = path.path();
    let mut entries: Vec<_> = std::fs::read_dir(&asset_path)
        .map_err(|e| GameError::from_string(e.to_string()).with_read(asset_path.clone()))?
        .filter_map(|res| {
            res.ok()
                .filter(|e| e.file_type().unwrap().is_file())
                .map(|e| e.path())
        })
        .collect();
    entries.sort();
    log::info!("Loading {} assets from {}", entries.len(), path);
    for path in entries {
        load_file(path, &mut f)?;
    }
    Ok(())
}

pub fn load_fonts() -> Result<FontSystem, GameError> {
    let mut db = fontdb::Database::new();
    load_asset_directory(AssetPath("fonts"), |path| {
        db.load_font_source(fontdb::Source::Binary(Arc::new(std::fs::read(path)?)));
        Ok(())
    })?;
    Ok(FontSystem::new(silica_env::get_locale(), db))
}

pub fn load_gui_theme(
    context: &Context,
    texture_config: &TextureConfig,
) -> Result<Rc<dyn Theme>, GameError> {
    let image = Image::load_asset(AssetPath("theme.png"))?;
    Ok(Rc::new(StandardTheme::new(
        context,
        texture_config,
        &image.data,
    )))
}

pub trait Game: Sized {
    fn window_attributes() -> WindowAttributes;
    fn load(context: &Context) -> Result<Self, GameError>;
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

pub fn run_game<T: Game>(app_info: AppInfo) {
    silica_env::setup_env(app_info);
    let context = Context::init(AdapterFeatures::default());
    match T::load(&context) {
        Ok(game) => run_app(
            T::window_attributes(),
            context,
            GameApp {
                game,
                last_update: Instant::now(),
            },
        ),
        Err(error) => run_gui_app(
            T::window_attributes(),
            context,
            error::error_gui(error),
            &Image::load_asset(AssetPath("theme.png"))
                .unwrap_display()
                .data,
        ),
    }
    .unwrap_display()
}
