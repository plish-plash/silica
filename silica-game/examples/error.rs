use std::sync::Arc;

use silica_game::{
    render::{Context, SurfaceSize, wgpu},
    *,
};

struct ErrorGame;

impl Game for ErrorGame {
    fn window_attributes() -> WindowAttributes {
        Window::default_attributes().with_title("Error Example")
    }
    fn load(_window: Arc<Window>, _context: &Context, assets: GameAssets) -> Result<Self, AssetError> {
        Err(AssetError::new(
            &assets,
            std::io::Error::other("An error occurred while loading the game."),
        ))
    }
    fn resize_window(&mut self, _context: &Context, _size: SurfaceSize) {}
    fn input(&mut self, _event: InputEvent) {}
    fn update(&mut self, _dt: f32) -> GameLoop {
        GameLoop::Run
    }
    fn clear_color(&self) -> Option<Rgba> {
        Some(Rgba::BLACK)
    }
    fn render(&mut self, _context: &Context, _pass: &mut wgpu::RenderPass) {}
}

fn main() {
    run_game::<ErrorGame>(app_info!());
}
