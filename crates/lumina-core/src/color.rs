//! Linear-RGB / sRGB color type with hex parsing and conversion helpers.
//!
//! Kept free of any graphics-backend dependency so `lumina-core` stays
//! a small, leaf-like crate. The graphics crate performs the final
//! conversion into whatever the backend expects.

use serde::{Deserialize, Serialize};

/// An 8-bit-per-channel RGBA color. The default is opaque white.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const CYAN: Self = Self::rgb(0, 220, 230);

    /// Lumina brand accent (cyan).
    pub const LUMINA_ACCENT: Self = Self::rgb(0, 220, 230);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse `#rrggbb` or `#rrggbbaa`. Returns `None` on malformed input.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let h = hex.strip_prefix('#').unwrap_or(hex);
        let parse = |s: &str| u8::from_str_radix(s, 16).ok();
        match h.len() {
            6 => Some(Self::rgb(
                parse(&h[0..2])?,
                parse(&h[2..4])?,
                parse(&h[4..6])?,
            )),
            8 => Some(Self::rgba(
                parse(&h[0..2])?,
                parse(&h[2..4])?,
                parse(&h[4..6])?,
                parse(&h[6..8])?,
            )),
            _ => None,
        }
    }

    /// Convert to normalized `[r, g, b, a]` in `0.0..=1.0` - the form
    /// suitable for pushing into GPU vertex/uniform buffers.
    pub fn to_array_f32(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// Convert to a `[r, g, b, a]` array of `f64` in `0.0..=1.0` (used by
    /// backends like wgpu that take clear colors as f64).
    pub fn to_array_f64(self) -> [f64; 4] {
        [
            self.r as f64 / 255.0,
            self.g as f64 / 255.0,
            self.b as f64 / 255.0,
            self.a as f64 / 255.0,
        ]
    }
}
