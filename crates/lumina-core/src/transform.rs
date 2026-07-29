//! Transform component - the spatial description every renderable entity
//! carries. Kept minimal: translation, rotation (quaternion), and scale.

use crate::math::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn from_pos_rot(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Default::default()
        }
    }

    /// Build a 4x4 model matrix suitable for pushing into a uniform buffer.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }

    /// Right / up / forward basis vectors in world space.
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }
    pub fn forward(&self) -> Vec3 {
        self.rotation * -Vec3::Z
    }

    /// Rotate around the world Y axis by `angle` radians.
    pub fn rotate_y(&mut self, angle: f32) {
        self.rotation = Quat::from_rotation_y(angle) * self.rotation;
    }

    /// Translate along local axes.
    pub fn translate_local(&mut self, offset: Vec3) {
        self.position += self.rotation * offset;
    }
}
