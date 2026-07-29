//! Lumina Core - foundational primitives for the Lumina Engine.
//!
//! Contains math helpers, timekeeping, logging setup, input state,
//! an ECS wrapper around `hecs`, transform/scene types, and a small
//! color utility. Other Lumina crates build on top of these.

pub mod color;
pub mod ecs;
pub mod input;
pub mod log;
pub mod math;
pub mod scene;
pub mod time;
pub mod transform;

pub use color::Color;
pub use ecs::World;
pub use input::Input;
pub use math::{Mat4, Quat, Vec2, Vec3, Vec4};
pub use scene::Scene;
pub use time::Time;
pub use transform::Transform;

/// Engine-wide result type.
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

/// Lumina engine version, baked at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Branding string used in window titles and the splash screen.
pub const ENGINE_NAME: &str = "Lumina Engine";

/// Friendly banner printed at startup.
pub fn banner() -> String {
    format!(
        "\n\
         ╔══════════════════════════════════════╗\n\
         ║         LUMINA ENGINE  v{:<8}        ║\n\
         ║   Lightweight 2D/3D + Editor + Lang  ║\n\
         ╚══════════════════════════════════════╝",
        VERSION
    )
}
