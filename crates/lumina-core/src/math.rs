//! Math primitives - thin re-exports over `glam` so the rest of the
//! engine has a single, stable vocabulary for vectors and matrices.

pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

/// Linear interpolation between two floats.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep - cubic ease between two values.
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Convert degrees to radians.
#[inline]
pub fn to_radians(deg: f32) -> f32 {
    deg * std::f32::consts::PI / 180.0
}

/// Convert radians to degrees.
#[inline]
pub fn to_degrees(rad: f32) -> f32 {
    rad * 180.0 / std::f32::consts::PI
}

/// Clamp a value into `[0, 1]`.
#[inline]
pub fn saturate(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}
