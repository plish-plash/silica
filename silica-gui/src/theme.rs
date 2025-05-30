use std::borrow::Cow;

use euclid::{point2, vec2, Box2D, SideOffsets2D};
use silica_color::Rgba;
use silica_wgpu::{draw::*, wgpu, Context, Texture, TextureConfig, TextureRect, TextureSize, Uv};

use crate::{
    render::{GuiBatcher, Quad},
    ButtonState, ButtonTheme, Rect,
};

#[derive(Clone, Copy, PartialEq)]
pub enum ThemeColor {
    Background,
    Accent,
    Text,
    Custom(Rgba),
}

pub trait Theme {
    fn color(&self, color: ThemeColor) -> Rgba;
    fn button_text_color(&self, state: ButtonState) -> Rgba;
    fn draw_border(&self, batcher: &mut GuiBatcher, rect: Rect);
    fn draw_gutter(&self, batcher: &mut GuiBatcher, rect: Rect);
    fn draw_button(
        &self,
        batcher: &mut GuiBatcher,
        rect: Rect,
        theme: ButtonTheme,
        state: ButtonState,
    );
    fn draw_dropdown(&self, batcher: &mut GuiBatcher, rect: Rect, state: ButtonState) -> Rect;
}

pub trait ThemeLoader {
    fn load_texture(&self, context: &Context, texture_config: &TextureConfig) -> Texture;
    fn load_theme(self) -> Box<dyn Theme>;
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
    palette: StandardPalette,
    gutter: NineSlice,
    normal_button: NineSlice,
    toggled_button: NineSlice,
    confirm_button: NineSlice,
    delete_button: NineSlice,
    dropdown_icon: TextureRect,
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
    pub fn new(texture_data: &[u8]) -> Self {
        assert_eq!(texture_data.len(), Self::TEXTURE_SIZE.area() as usize * 4);
        StandardTheme {
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
            dropdown_icon: TextureRect::new(point2(2, 2), point2(14, 14)),
        }
    }
}
impl Theme for StandardTheme {
    fn color(&self, color: ThemeColor) -> Rgba {
        match color {
            ThemeColor::Background => self.palette.background_color,
            ThemeColor::Accent => self.palette.accent_color,
            ThemeColor::Text => self.palette.text_color,
            ThemeColor::Custom(rgba) => rgba,
        }
    }
    fn button_text_color(&self, state: ButtonState) -> Rgba {
        Self::state_color(self.palette.text_color, state)
    }
    fn draw_border(&self, batcher: &mut GuiBatcher, rect: Rect) {
        draw_border(batcher, rect.cast_unit(), self.palette.border_color);
    }
    fn draw_gutter(&self, batcher: &mut GuiBatcher, rect: Rect) {
        self.gutter.draw(batcher, rect.cast_unit(), Rgba::WHITE);
    }
    fn draw_button(
        &self,
        batcher: &mut GuiBatcher,
        rect: Rect,
        theme: ButtonTheme,
        state: ButtonState,
    ) {
        let rect = rect.cast_unit();
        let color = Self::state_color(Rgba::WHITE, state);
        match theme {
            ButtonTheme::Normal => self.normal_button.draw(batcher, rect, color),
            ButtonTheme::Toggled => self.toggled_button.draw(batcher, rect, color),
            ButtonTheme::Confirm => self.confirm_button.draw(batcher, rect, color),
            ButtonTheme::Delete => self.delete_button.draw(batcher, rect, color),
            ButtonTheme::Flat => {
                let color = if state == ButtonState::Hover || state == ButtonState::Press {
                    Self::state_color(self.palette.accent_background_color, state)
                } else {
                    self.palette.background_color
                };
                batcher.queue_theme_quad(Quad {
                    rect: rect.cast_unit(),
                    uv: Uv::ZERO,
                    color,
                });
            }
            ButtonTheme::Tab => self.normal_button.draw_top(batcher, rect, color),
            ButtonTheme::TabCurrent => self.toggled_button.draw_top(batcher, rect, color),
        };
    }
    fn draw_dropdown(&self, batcher: &mut GuiBatcher, mut rect: Rect, state: ButtonState) -> Rect {
        self.draw_button(batcher, rect, ButtonTheme::Normal, state);
        rect.max.x -= Self::DROPDOWN_ICON_RECT_SIZE;
        let icon_rect_min = point2(rect.max.x, rect.min.y);
        batcher.queue_theme_quad(Quad {
            rect: Box2D::new(icon_rect_min, point2(rect.max.x + 1.0, rect.max.y)),
            uv: Uv::ZERO,
            color: Self::state_color(self.palette.border_color, state),
        });
        let icon_size = self.dropdown_icon.size().to_f32().cast_unit();
        let icon_point = icon_rect_min + (vec2(Self::DROPDOWN_ICON_RECT_SIZE, rect.height()) / 2.0)
            - (icon_size / 2.0);
        batcher.queue_theme_quad(Quad {
            rect: Box2D::from_origin_and_size(icon_point, icon_size),
            uv: Uv::normalize(self.dropdown_icon, Self::TEXTURE_SIZE),
            color: Self::state_color(self.palette.text_color, state),
        });
        rect
    }
}

pub struct StandardThemeLoader<'a>(Cow<'a, [u8]>);

impl<'a> StandardThemeLoader<'a> {
    pub fn new(data: impl Into<Cow<'a, [u8]>>) -> Self {
        StandardThemeLoader(data.into())
    }
}
impl ThemeLoader for StandardThemeLoader<'_> {
    fn load_texture(&self, context: &Context, texture_config: &TextureConfig) -> Texture {
        Texture::new_with_data(
            context,
            texture_config,
            StandardTheme::TEXTURE_SIZE,
            wgpu::TextureFormat::Rgba8Unorm,
            self.0.as_ref(),
        )
    }
    fn load_theme(self) -> Box<dyn Theme> {
        Box::new(StandardTheme::new(self.0.as_ref()))
    }
}
