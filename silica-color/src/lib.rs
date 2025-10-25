//! Usually, linear RGBA is all you need.
//!
//! When you start to learn about how computers use color, all the options quickly become
//! overwhelming. Is my image encoded in linear RGB or sRGB? When do I premultiply alpha? Most of
//! the color-related Rust crates out there provide all sorts of color spaces and allow you to
//! easily convert between them, so you can use whatever color space is most appropriate.
//!
//! However, for modern rendering systems, you should use linear RGB basically all of the time.
//! Colors must be in linear space to be blended correctly. Shaders will convert all input colors to
//! linear if they aren't already, and the graphics pipeline will automatically convert the output
//! to whatever the screen is expecting. So it makes things simpler to use linear colors
//! everywhere. That way CPU-side color operations behave the same way they do in shaders, and
//! there's no need to do any conversions.
//!
//! This crate provides `Rgba` for working with linear RGBA. It contains four f32s and is repr(C),
//! which is what most graphics pipelines expect. It provides all the operations needed for typical
//! rendering applications.
//!
//! This crate is designed for applications that need a simple, universal color type. It is not
//! designed for advanced color manipulation or image processing, for that consider `palette` or
//! `color`.
//!
//! ### Why another color crate?
//!
//! Yes, Rust has quite a few color crates already, why make another one? The answer is that all of
//! the color crates I've seen are *heavily* focused on color space management and conversion, and
//! often fall flat if you need to do more than the simplest of operations on the colors themselves.
//! Here are some specific needs I have:
//! - Stored as repr(C) f32s to be easy to use in wgpu shaders.
//! - Serialize and deserialize in a straightforward way. Alpha should default to 1 if not specified, since opaque
//!   colors are very common.
//! - Convert to and from u32 linear hex codes.
//! - Constants for black and white.
//!
//! These requirements seem quite minimal, but I wasn't able to find a crate that satisfied them
//! all. So, here we are.

use std::hash::{Hash, Hasher};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

fn default_alpha() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    #[serde(default = "default_alpha")]
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
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        fn to_f32(x: u8) -> f32 {
            (x as f32) / 255.0
        }
        Rgba {
            r: to_f32(r),
            g: to_f32(g),
            b: to_f32(b),
            a: to_f32(a),
        }
    }
    pub fn to_u32(&self) -> u32 {
        fn to_u8(x: f32) -> u8 {
            (x * 255.0) as u8
        }
        u32::from_be_bytes([to_u8(self.a), to_u8(self.r), to_u8(self.g), to_u8(self.b)])
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
}
impl Default for Rgba {
    fn default() -> Self {
        Rgba::WHITE
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

impl From<u32> for Rgba {
    fn from(value: u32) -> Self {
        let bytes = value.to_be_bytes();
        Rgba::from_u8(bytes[1], bytes[2], bytes[3], bytes[0])
    }
}
impl std::str::FromStr for Rgba {
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

impl std::fmt::Display for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:08X}", self.to_u32())
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
