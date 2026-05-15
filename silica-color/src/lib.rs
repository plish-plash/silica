//! Simple color types for easy gamma correction.
//!
//! See the wgpu wiki on sRGB: https://github.com/gfx-rs/wgpu/wiki/Texture-Color-Formats-and-Srgb-conversions
//!
//! In short:
//! - sRGB is a color space. In the sRGB color space, colors can be either linear or gamma-corrected.
//! - User-supplied colors, such as images and hex codes, are almost always gamma-corrected.
//! - Colors must be linear in order to be blended or shaded correctly.
//! - Colors displayed on a screen must be gamma-corrected.
//!
//! You should use linear colors everywhere that you can, including in shaders. The graphics driver can
//! do the linear-to-gamma-corrected conversion for you, so shaders should always work with linear
//! colors. The only time gamma-corrected colors should be used is when reading or writing image data.
//!
//! This crate provides [`Rgba`], which stores linear colors as four f32s, and [`Rgba32Gamma`], which
//! stores gamma-corrected colors as four u8s.

use std::{
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};

use bytemuck::{Pod, Zeroable};
use fast_srgb8::{f32_to_srgb8, srgb8_to_f32};
use serde::{Deserialize, Deserializer, Serialize, de};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[serde(from = "Rgba32Gamma", into = "Rgba32Gamma")]
#[repr(C)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const BLACK: Rgba = Rgba::new_opaque(0.0, 0.0, 0.0);
    pub const WHITE: Rgba = Rgba::new_opaque(1.0, 1.0, 1.0);
    pub const RED: Rgba = Rgba::new_opaque(1.0, 0.0, 0.0);
    pub const GREEN: Rgba = Rgba::new_opaque(0.0, 1.0, 0.0);
    pub const BLUE: Rgba = Rgba::new_opaque(0.0, 0.0, 1.0);
    pub const YELLOW: Rgba = Rgba::new_opaque(1.0, 1.0, 0.0);
    pub const MAGENTA: Rgba = Rgba::new_opaque(1.0, 0.0, 1.0);
    pub const CYAN: Rgba = Rgba::new_opaque(0.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Rgba { r, g, b, a }
    }
    pub const fn new_opaque(r: f32, g: f32, b: f32) -> Self {
        Rgba { r, g, b, a: 1.0 }
    }
    pub fn with_alpha(self, a: f32) -> Self {
        Rgba { a, ..self }
    }
    pub fn mul_alpha(self, rhs: f32) -> Self {
        Rgba {
            a: self.a * rhs,
            ..self
        }
    }
    pub fn lerp(self, rhs: Rgba, t: f32) -> Self {
        fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
            (a * (1.0 - t)) + (b * t)
        }
        Rgba {
            r: lerp_f32(self.r, rhs.r, t),
            g: lerp_f32(self.g, rhs.g, t),
            b: lerp_f32(self.b, rhs.b, t),
            a: lerp_f32(self.a, rhs.a, t),
        }
    }
    pub fn lighten(self, amount: f32) -> Self {
        Rgba {
            a: self.a,
            ..self.lerp(Self::WHITE, amount)
        }
    }
    pub fn darken(self, amount: f32) -> Self {
        Rgba {
            a: self.a,
            ..self.lerp(Self::BLACK, amount)
        }
    }
}
impl Default for Rgba {
    fn default() -> Self {
        Rgba::WHITE
    }
}

impl From<Rgba> for u32 {
    fn from(value: Rgba) -> Self {
        Rgba32Gamma::from(value).into()
    }
}

impl std::ops::Mul<f32> for Rgba {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Rgba {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a,
        }
    }
}
impl std::ops::MulAssign<f32> for Rgba {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
    }
}
impl std::ops::Mul for Rgba {
    type Output = Self;
    fn mul(self, rhs: Rgba) -> Self {
        Rgba {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
            a: self.a * rhs.a,
        }
    }
}
impl std::ops::MulAssign for Rgba {
    fn mul_assign(&mut self, rhs: Rgba) {
        self.r *= rhs.r;
        self.g *= rhs.g;
        self.b *= rhs.b;
        self.a *= rhs.a;
    }
}

/// Deterministically hash an `f32`, treating all NANs as equal, and ignoring the sign of zero.
#[inline]
fn f32_hash<H: Hasher>(state: &mut H, f: f32) {
    if f == 0.0 {
        state.write_u8(0);
    } else if f.is_nan() {
        state.write_u8(1);
    } else {
        f.to_bits().hash(state);
    }
}
impl Hash for Rgba {
    fn hash<H: Hasher>(&self, state: &mut H) {
        f32_hash(state, self.r);
        f32_hash(state, self.g);
        f32_hash(state, self.b);
        f32_hash(state, self.a);
    }
}

fn default_alpha() -> u8 {
    u8::MAX
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[serde(into = "u32")]
#[repr(C)]
pub struct Rgba32Gamma {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default = "default_alpha")]
    pub a: u8,
}

impl Rgba32Gamma {
    pub const BLACK: Rgba32Gamma = Rgba32Gamma::new_opaque(0, 0, 0);
    pub const WHITE: Rgba32Gamma = Rgba32Gamma::new_opaque(255, 255, 255);
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba32Gamma { r, g, b, a }
    }
    pub const fn new_opaque(r: u8, g: u8, b: u8) -> Self {
        Rgba32Gamma { r, g, b, a: u8::MAX }
    }
}
impl Default for Rgba32Gamma {
    fn default() -> Self {
        Rgba32Gamma::WHITE
    }
}

impl From<u32> for Rgba32Gamma {
    fn from(value: u32) -> Self {
        let bytes = value.to_be_bytes();
        Rgba32Gamma::new(bytes[1], bytes[2], bytes[3], bytes[0])
    }
}
impl From<Rgba32Gamma> for u32 {
    fn from(value: Rgba32Gamma) -> Self {
        u32::from_be_bytes([value.a, value.r, value.g, value.b])
    }
}
impl From<Rgba> for Rgba32Gamma {
    fn from(value: Rgba) -> Self {
        Rgba32Gamma {
            r: f32_to_srgb8(value.r),
            g: f32_to_srgb8(value.g),
            b: f32_to_srgb8(value.b),
            a: (value.a * 255.0) as u8,
        }
    }
}
impl From<Rgba32Gamma> for Rgba {
    fn from(value: Rgba32Gamma) -> Self {
        Rgba {
            r: srgb8_to_f32(value.r),
            g: srgb8_to_f32(value.g),
            b: srgb8_to_f32(value.b),
            a: value.a as f32 / 255.0,
        }
    }
}

impl FromStr for Rgba32Gamma {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let has_alpha = if s.len() == 8 {
            true
        } else if s.len() == 6 {
            false
        } else {
            return Err("wrong length".to_string());
        };
        let mut value = u32::from_str_radix(s, 16).map_err(|e| e.to_string())?;
        if !has_alpha {
            value |= 0xFF000000;
        }
        Ok(value.into())
    }
}
impl fmt::Display for Rgba32Gamma {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:08X}", u32::from(*self))
    }
}

struct Rgba32GammaVisitor;

impl<'de> de::Visitor<'de> for Rgba32GammaVisitor {
    type Value = Rgba32Gamma;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer, string, or sequence")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::try_from(v).map(Into::into).map_err(E::custom)
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.into())
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        u32::try_from(v).map(Into::into).map_err(E::custom)
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        FromStr::from_str(value).map_err(E::custom)
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        use de::Error;
        let r = seq.next_element()?.ok_or(A::Error::invalid_length(0, &"3 or 4"))?;
        let g = seq.next_element()?.ok_or(A::Error::invalid_length(1, &"3 or 4"))?;
        let b = seq.next_element()?.ok_or(A::Error::invalid_length(2, &"3 or 4"))?;
        let a = seq.next_element()?.unwrap_or(default_alpha());
        Ok(Rgba32Gamma { r, g, b, a })
    }
}
impl<'de> Deserialize<'de> for Rgba32Gamma {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(Rgba32GammaVisitor)
    }
}
