//! Scene container. A scene is just an ECS world plus a list of asset
//! references. Scenes serialize to RON (`*.lumina` files).

use crate::ecs::World;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    #[serde(skip)]
    pub world: World,
    pub background: [f32; 4],
    pub gravity: [f32; 3],
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            world: World::new(),
            background: [0.05, 0.06, 0.08, 1.0],
            gravity: [0.0, -9.81, 0.0],
        }
    }

    /// Save to a `.lumina` RON file.
    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        let pretty = ron::ser::PrettyConfig::default()
            .struct_names(true)
            .depth_limit(4);
        let s = ron::ser::to_string_pretty(self, pretty)?;
        std::fs::write(path, s)?;
        Ok(())
    }

    /// Load from a `.lumina` RON file. The ECS world is not serialized
    /// (entity layouts are runtime-only) - only the metadata is restored.
    pub fn load(path: &std::path::Path) -> crate::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        let mut scene: Scene = ron::from_str(&s)?;
        scene.world = World::new();
        Ok(scene)
    }
}
