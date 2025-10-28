use std::io::Error as IoError;

use silica_game::{
    render::{Context, SurfaceSize, wgpu},
    *,
};

struct ErrorGame;

impl Game for ErrorGame {
    fn window_attributes() -> WindowAttributes {
        Window::default_attributes().with_title("Error Example")
    }
    fn load(_context: &Context, assets: GameAssets) -> Result<Self, AssetError> {
        Err(AssetError::new(
            &assets,
            IoError::other("An error occurred while loading the game."),
        ))
    }
    fn resize_window(&mut self, _context: &Context, _size: SurfaceSize) {}
    fn input(&mut self, _event: InputEvent) {}
    fn update(&mut self, _event_loop: &EventLoop, _dt: f32) {}
    fn clear_color(&self) -> Rgba {
        Rgba::BLACK
    }
    fn render(&mut self, _context: &Context, _pass: &mut wgpu::RenderPass) {}
}

fn main() {
    run_game::<ErrorGame>(app_info!());
}
