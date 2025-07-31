use silica_game::{
    EventLoop, Game, GameError, InputEvent, Rgba, app_info,
    render::{Context, SurfaceSize, wgpu},
    run_game,
};

struct ErrorGame;

impl Game for ErrorGame {
    fn load(_context: &Context) -> Result<Self, GameError> {
        Err(GameError::from_string(
            "An error occurred while loading the game.".to_string(),
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
