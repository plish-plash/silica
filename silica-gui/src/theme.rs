use euclid::{SideOffsets2D, point2};
use silica_wgpu::{
    Context, Texture, TextureConfig, TextureRect, TextureSize, Uv, draw::*, wgpu::TextureFormat,
};

use crate::{
    Color, Pixel, Rect, Rgba,
    render::{GuiRenderer, Quad},
    widget::{ButtonState, ButtonStyle},
};

pub trait Theme {
    fn texture(&self) -> &Texture;
    fn color(&self, color: Color) -> Rgba;
    fn button_foreground_color(&self, state: ButtonState) -> Rgba;
    fn draw_gutter(&self, renderer: &mut GuiRenderer, rect: Rect);
    fn draw_button(
        &self,
        renderer: &mut GuiRenderer,
        rect: Rect,
        style: ButtonStyle,
        toggled: bool,
        state: ButtonState,
    );
}

struct StandardPalette {
    background_color: Rgba,
    border_color: Rgba,
    text_color: Rgba,
    accent_color: Rgba,
    accent_background_color: Rgba,
}

impl StandardPalette {
    fn palette_color(texture_data: &[u8], slot: usize) -> Rgba {
        let i = slot * 4 * StandardTheme::TEXTURE_SIZE.width as usize;
        Rgba::from_u8(
            texture_data[i],
            texture_data[i + 1],
            texture_data[i + 2],
            texture_data[i + 3],
        )
    }
    fn new(texture_data: &[u8]) -> Self {
        StandardPalette {
            background_color: Self::palette_color(texture_data, 1),
            border_color: Self::palette_color(texture_data, 2),
            text_color: Self::palette_color(texture_data, 3),
            accent_color: Self::palette_color(texture_data, 4),
            accent_background_color: Self::palette_color(texture_data, 5),
        }
    }
}

pub struct StandardTheme {
    texture: Texture,
    palette: StandardPalette,
    gutter: NineSlice<Pixel>,
    normal_button: NineSlice<Pixel>,
    toggled_button: NineSlice<Pixel>,
    confirm_button: NineSlice<Pixel>,
    delete_button: NineSlice<Pixel>,
    // dropdown_icon: TextureRect,
}

impl StandardTheme {
    pub const TEXTURE_SIZE: TextureSize = TextureSize::new(64, 32);
    pub const DROPDOWN_ICON_RECT_SIZE: f32 = 32.0;
    fn state_color(color: Rgba, state: ButtonState) -> Rgba {
        match state {
            ButtonState::Normal => color,
            ButtonState::Hover => color * 1.2,
            ButtonState::Press => color * 0.9,
            ButtonState::Disable => color.mul_alpha(0.5),
        }
    }
    pub fn new(context: &Context, texture_config: &TextureConfig, texture_data: &[u8]) -> Self {
        assert_eq!(texture_data.len(), Self::TEXTURE_SIZE.area() as usize * 4);
        let texture = Texture::new_with_data(
            context,
            texture_config,
            Self::TEXTURE_SIZE,
            TextureFormat::Rgba8Unorm,
            texture_data,
        );
        StandardTheme {
            texture,
            palette: StandardPalette::new(texture_data),
            gutter: NineSlice::new(
                Self::TEXTURE_SIZE,
                TextureRect::new(point2(16, 0), point2(32, 16)),
                SideOffsets2D::new_all_same(7),
            ),
            normal_button: NineSlice::new(
                Self::TEXTURE_SIZE,
                TextureRect::new(point2(32, 0), point2(48, 16)),
                SideOffsets2D::new_all_same(7),
            ),
            toggled_button: NineSlice::new(
                Self::TEXTURE_SIZE,
                TextureRect::new(point2(48, 0), point2(64, 16)),
                SideOffsets2D::new_all_same(7),
            ),
            confirm_button: NineSlice::new(
                Self::TEXTURE_SIZE,
                TextureRect::new(point2(32, 16), point2(48, 32)),
                SideOffsets2D::new_all_same(7),
            ),
            delete_button: NineSlice::new(
                Self::TEXTURE_SIZE,
                TextureRect::new(point2(48, 16), point2(64, 32)),
                SideOffsets2D::new_all_same(7),
            ),
            // dropdown_icon: TextureRect::new(point2(2, 2), point2(14, 14)),
        }
    }
}
impl Theme for StandardTheme {
    fn texture(&self) -> &Texture {
        &self.texture
    }
    fn color(&self, color: Color) -> Rgba {
        match color {
            Color::Background => self.palette.background_color,
            Color::Border => self.palette.border_color,
            Color::Accent => self.palette.accent_color,
            Color::Foreground => self.palette.text_color,
            Color::Custom(rgba) => rgba,
        }
    }
    fn button_foreground_color(&self, state: ButtonState) -> Rgba {
        Self::state_color(self.palette.text_color, state)
    }
    fn draw_gutter(&self, renderer: &mut GuiRenderer, rect: Rect) {
        self.gutter.draw(renderer, rect.to_box2d(), Rgba::WHITE);
    }
    fn draw_button(
        &self,
        renderer: &mut GuiRenderer,
        rect: Rect,
        style: ButtonStyle,
        toggled: bool,
        state: ButtonState,
    ) {
        let rect = rect.to_box2d();
        let color = Self::state_color(Rgba::WHITE, state);
        match style {
            ButtonStyle::Normal => {
                if toggled {
                    self.toggled_button.draw(renderer, rect, color);
                } else {
                    self.normal_button.draw(renderer, rect, color);
                }
            }
            ButtonStyle::Confirm => self.confirm_button.draw(renderer, rect, color),
            ButtonStyle::Delete => self.delete_button.draw(renderer, rect, color),
            ButtonStyle::Flat => {
                let color = if state == ButtonState::Hover || state == ButtonState::Press {
                    Self::state_color(self.palette.accent_background_color, state)
                } else {
                    self.palette.background_color
                };
                renderer.draw_theme_quad(Quad {
                    rect,
                    uv: Uv::ZERO,
                    color,
                });
            }
            ButtonStyle::Tab => {
                if toggled {
                    self.toggled_button.draw_top(renderer, rect, color);
                } else {
                    self.normal_button.draw_top(renderer, rect, color);
                }
            }
        };
    }
    // fn draw_dropdown(&self, renderer: &mut GuiRenderer, mut rect: Rect, state: ButtonState) ->
    // Rect {     self.draw_button(batcher, rect, ButtonStyle::Normal, state);
    //     rect.max.x -= Self::DROPDOWN_ICON_RECT_SIZE;
    //     let icon_rect_min = point2(rect.max.x, rect.min.y);
    //     batcher.queue_theme_quad(Quad {
    //         rect: Box2D::new(icon_rect_min, point2(rect.max.x + 1.0, rect.max.y)),
    //         uv: Uv::ZERO,
    //         color: Self::state_color(self.palette.border_color, state),
    //     });
    //     let icon_size = self.dropdown_icon.size().to_f32().cast_unit();
    //     let icon_point = icon_rect_min + (vec2(Self::DROPDOWN_ICON_RECT_SIZE, rect.height()) /
    // 2.0)
    //         - (icon_size / 2.0);
    //     batcher.queue_theme_quad(Quad {
    //         rect: Box2D::from_origin_and_size(icon_point, icon_size),
    //         uv: Uv::normalize(self.dropdown_icon, Self::TEXTURE_SIZE),
    //         color: Self::state_color(self.palette.text_color, state),
    //     });
    //     rect
    // }
}
