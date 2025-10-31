use euclid::{Box2D, SideOffsets2D};
use serde::Deserialize;
use silica_asset::{AssetError, AssetSource, serde_util::string_or_struct};
use silica_wgpu::{Context, Texture, TextureConfig, TextureRect, TextureSize, draw::*, wgpu::TextureFormat};

use crate::{
    Color, FontSystem, Pixel, Rect, Rgba,
    render::{GuiRenderer, Quad},
    widget::{ButtonState, ButtonStyle},
};

pub trait Theme {
    fn font_system(&self) -> &FontSystem;
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

#[derive(Deserialize)]
struct StandardPalette {
    #[serde(deserialize_with = "string_or_struct")]
    background_color: Rgba,
    #[serde(deserialize_with = "string_or_struct")]
    border_color: Rgba,
    #[serde(deserialize_with = "string_or_struct")]
    gutter_color: Rgba,
    #[serde(deserialize_with = "string_or_struct")]
    text_color: Rgba,
    #[serde(deserialize_with = "string_or_struct")]
    accent_color: Rgba,
    #[serde(deserialize_with = "string_or_struct")]
    accent_background_color: Rgba,
}

#[derive(Deserialize)]
struct NineSliceConfig {
    rect: TextureRect,
    insets: SideOffsets2D<u32, Texture>,
}

impl NineSliceConfig {
    fn with_texture_size(self, size: TextureSize) -> NineSlice<Pixel> {
        NineSlice::new(size, self.rect, self.insets)
    }
}

#[derive(Deserialize)]
struct ButtonThemeConfig {
    normal: NineSliceConfig,
    hover: Option<NineSliceConfig>,
    press: Option<NineSliceConfig>,
    disable: Option<NineSliceConfig>,
}

impl ButtonThemeConfig {
    fn with_texture_size(self, size: TextureSize) -> ButtonTheme {
        ButtonTheme {
            normal: self.normal.with_texture_size(size),
            hover: self.hover.map(|ns| ns.with_texture_size(size)),
            press: self.press.map(|ns| ns.with_texture_size(size)),
            disable: self.disable.map(|ns| ns.with_texture_size(size)),
        }
    }
}

#[derive(Deserialize)]
struct StandardThemeConfig {
    font: String,
    texture: String,
    palette: StandardPalette,
    gutter: NineSliceConfig,
    button: ButtonThemeConfig,
    button_toggled: ButtonThemeConfig,
    button_confirm: Option<ButtonThemeConfig>,
    button_delete: Option<ButtonThemeConfig>,
    tab: ButtonThemeConfig,
    tab_active: NineSliceConfig,
}

#[derive(Clone)]
struct ButtonTheme {
    normal: NineSlice<Pixel>,
    hover: Option<NineSlice<Pixel>>,
    press: Option<NineSlice<Pixel>>,
    disable: Option<NineSlice<Pixel>>,
}

impl ButtonTheme {
    fn draw<F>(&self, renderer: &mut GuiRenderer, rect: Box2D<i32, Pixel>, state: ButtonState, state_color: F)
    where
        F: FnOnce(Rgba, ButtonState) -> Rgba,
    {
        let draw_with_fallback = |ns: Option<&NineSlice<Pixel>>| {
            if let Some(ns) = ns {
                ns.draw(renderer, rect, Rgba::WHITE);
            } else {
                self.normal.draw(renderer, rect, state_color(Rgba::WHITE, state));
            }
        };
        match state {
            ButtonState::Normal => self.normal.draw(renderer, rect, Rgba::WHITE),
            ButtonState::Hover => draw_with_fallback(self.hover.as_ref()),
            ButtonState::Press => draw_with_fallback(self.press.as_ref()),
            ButtonState::Disable => draw_with_fallback(self.disable.as_ref()),
        }
    }
}

pub struct StandardTheme {
    font_system: FontSystem,
    texture: Texture,
    palette: StandardPalette,
    gutter: NineSlice<Pixel>,
    button: ButtonTheme,
    button_toggled: ButtonTheme,
    button_confirm: ButtonTheme,
    button_delete: ButtonTheme,
    tab: ButtonTheme,
    tab_active: NineSlice<Pixel>,
}

impl StandardTheme {
    fn state_color(color: Rgba, state: ButtonState) -> Rgba {
        match state {
            ButtonState::Normal => color,
            ButtonState::Hover => color * 1.1,
            ButtonState::Press => color * 0.9,
            ButtonState::Disable => color.mul_alpha(0.5),
        }
    }
    pub fn load<S: AssetSource>(
        context: &Context,
        texture_config: &TextureConfig,
        asset_source: &mut S,
    ) -> Result<Self, AssetError> {
        let config: StandardThemeConfig = silica_asset::load_yaml(asset_source, "config.yaml")?;
        let font_system = FontSystem::with_font_asset(asset_source, &config.font)?;
        let image = silica_asset::load_image(asset_source, &config.texture)?;
        let texture = Texture::new_with_data(
            context,
            texture_config,
            TextureSize::new(image.width, image.height),
            TextureFormat::Rgba8Unorm,
            &image.data,
        );
        let texture_size = texture.size();
        let button = config.button.with_texture_size(texture_size);
        Ok(StandardTheme {
            font_system,
            texture,
            palette: config.palette,
            gutter: config.gutter.with_texture_size(texture_size),
            button: button.clone(),
            button_toggled: config.button_toggled.with_texture_size(texture_size),
            button_confirm: config
                .button_confirm
                .map(|button| button.with_texture_size(texture_size))
                .unwrap_or(button.clone()),
            button_delete: config
                .button_delete
                .map(|button| button.with_texture_size(texture_size))
                .unwrap_or(button),
            tab: config.tab.with_texture_size(texture_size),
            tab_active: config.tab_active.with_texture_size(texture_size),
        })
    }
}
impl Theme for StandardTheme {
    fn font_system(&self) -> &FontSystem {
        &self.font_system
    }
    fn texture(&self) -> &Texture {
        &self.texture
    }
    fn color(&self, color: Color) -> Rgba {
        match color {
            Color::Background => self.palette.background_color,
            Color::Border => self.palette.border_color,
            Color::Gutter => self.palette.gutter_color,
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
        match style {
            ButtonStyle::Normal => {
                if toggled {
                    self.button_toggled.draw(renderer, rect, state, Self::state_color);
                } else {
                    self.button.draw(renderer, rect, state, Self::state_color);
                }
            }
            ButtonStyle::Confirm => self.button_confirm.draw(renderer, rect, state, Self::state_color),
            ButtonStyle::Delete => self.button_delete.draw(renderer, rect, state, Self::state_color),
            ButtonStyle::Flat => {
                let color = if state == ButtonState::Hover || state == ButtonState::Press {
                    Self::state_color(self.palette.accent_background_color, state)
                } else {
                    self.palette.background_color
                };
                renderer.draw_theme_quad(Quad {
                    rect,
                    uv: GuiRenderer::UV_WHITE,
                    color,
                });
            }
            ButtonStyle::Tab => {
                if toggled {
                    self.tab_active.draw(renderer, rect, Rgba::WHITE);
                } else {
                    self.tab.draw(renderer, rect, state, Self::state_color);
                }
            }
        };
    }
}
