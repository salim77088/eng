//! Lumina Graphics - 2D and 3D rendering on top of `wgpu`.
//!
//! The renderer owns the surface, device, and queue, and exposes
//! high-level draw primitives (sprites, meshes, gizmos). WGSL shaders
//! are embedded as strings so the engine ships as a single binary.

pub mod camera;
pub mod mesh;
pub mod renderer;
pub mod shader;
pub mod sprite;
pub mod texture;

pub use camera::{Camera, Camera2D, Camera3D};
pub use mesh::Mesh;
pub use renderer::Renderer;
pub use sprite::{Sprite, SpriteBatch};
pub use texture::Texture;

/// Backends the renderer will request from wgpu. We let wgpu pick the
/// best one for the platform (Vulkan on Linux/Win, Metal on macOS, DX12
/// on Windows as fallback).
pub fn preferred_backends() -> wgpu::Backends {
    wgpu::Backends::PRIMARY
}
