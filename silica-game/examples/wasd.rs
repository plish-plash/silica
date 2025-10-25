use silica_game::{
    keyboard::KeyCode,
    render::{Batcher, Context, SurfaceSize, Texture, TextureConfig, Uv, wgpu},
    texture::{Image, ImageExt},
    world2d::{Camera2D, Pipeline2D, Point, Quad, Rect, Vector},
    *,
};

#[derive(Default)]
struct WasdInput {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl WasdInput {
    fn handle_input(&mut self, event: &InputEvent) {
        if let InputEvent::Keyboard(event) = event {
            match event.physical_key() {
                KeyCode::KeyW | KeyCode::ArrowUp => self.up = event.is_pressed(),
                KeyCode::KeyS | KeyCode::ArrowDown => self.down = event.is_pressed(),
                KeyCode::KeyA | KeyCode::ArrowLeft => self.left = event.is_pressed(),
                KeyCode::KeyD | KeyCode::ArrowRight => self.right = event.is_pressed(),
                _ => (),
            }
        }
    }
    fn movement(&self) -> Vector {
        let mut movement = Vector::zero();
        if self.up {
            movement.y -= 1.0;
        }
        if self.down {
            movement.y += 1.0;
        }
        if self.left {
            movement.x -= 1.0;
        }
        if self.right {
            movement.x += 1.0;
        }
        movement.try_normalize().unwrap_or_default()
    }
}

struct WasdGame {
    texture_config: TextureConfig,
    pipeline: Option<Pipeline2D>,
    batcher: Batcher<Quad>,
    surface_size: SurfaceSize,
    input: WasdInput,
    player_point: Point,
    player_texture: Texture,
}

impl Game for WasdGame {
    fn window_attributes() -> WindowAttributes {
        Window::default_attributes().with_title("WASD Example")
    }
    fn load(mut assets: GameAssets, context: &Context) -> Result<Self, AssetError> {
        let texture_config = TextureConfig::new(context, wgpu::FilterMode::Linear);
        let player_texture = Image::load_texture(context, &texture_config, &mut assets, "player.png")?;
        Ok(WasdGame {
            texture_config,
            pipeline: None,
            batcher: Batcher::new(context),
            surface_size: SurfaceSize::zero(),
            input: WasdInput::default(),
            player_point: Point::zero(),
            player_texture,
        })
    }
    fn resize_window(&mut self, _context: &Context, size: SurfaceSize) {
        self.surface_size = size;
    }
    fn input(&mut self, event: InputEvent) {
        self.input.handle_input(&event);
    }
    fn update(&mut self, _event_loop: &EventLoop, dt: f32) {
        const PLAYER_SPEED: f32 = 200.0;
        self.player_point += self.input.movement() * PLAYER_SPEED * dt;
    }
    fn clear_color(&self) -> Rgba {
        Rgba::BLACK
    }
    fn render(&mut self, context: &Context, pass: &mut wgpu::RenderPass) {
        let pipeline = self
            .pipeline
            .get_or_insert_with(|| Pipeline2D::new(context, &self.texture_config));
        let camera = Camera2D::default().transform(self.surface_size, None);
        pipeline.set_camera(context, camera, self.surface_size);

        self.batcher.clear();
        self.batcher.set_texture(&mut self.player_texture);
        let size = self.player_texture.size().cast().cast_unit();
        let mut rect = Rect::new(self.player_point, size);
        rect = rect.translate(-size.to_vector() / 2.0);
        self.batcher.queue(Quad {
            transform: Quad::rect_transform(rect),
            uv: Uv::FULL,
            color: Rgba::WHITE,
        });
        self.batcher.draw(context, pass, pipeline);
    }
}

fn main() {
    run_game::<WasdGame>(app_info!());
}
