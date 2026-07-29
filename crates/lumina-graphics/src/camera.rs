//! 2D and 3D cameras. Both produce a `view_proj` matrix suitable for
//! pushing into a uniform buffer.

use lumina_core::math::{Mat4, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    pub position: Vec3,
    pub rotation: f32, // radians, around Z
    pub zoom: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
}

impl Camera2D {
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: 0.0,
            zoom: 1.0,
            viewport_w,
            viewport_h,
        }
    }

    pub fn view_proj(&self) -> Mat4 {
        let aspect = self.viewport_w.max(1.0) / self.viewport_h.max(1.0);
        let half_h = self.viewport_h * 0.5 / self.zoom.max(0.0001);
        let half_w = half_h * aspect;
        let proj = Mat4::orthographic_rh(
            -half_w, half_w, -half_h, half_h, -1000.0, 1000.0,
        );
        let view = Mat4::from_translation(-self.position)
            * Mat4::from_rotation_z(-self.rotation);
        proj * view
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera3D {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
}

impl Camera3D {
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 6.0),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.2,
            fov: 60.0_f32.to_radians(),
            near: 0.1,
            far: 1000.0,
            viewport_w,
            viewport_h,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cy * cp, sp, sy * cp)
    }

    pub fn view_proj(&self) -> Mat4 {
        let aspect = self.viewport_w.max(1.0) / self.viewport_h.max(1.0);
        let proj = Mat4::perspective_rh(self.fov, aspect, self.near, self.far);
        let target = self.position + self.forward();
        let up = Vec3::Y;
        let view = Mat4::look_at_rh(self.position, target, up);
        proj * view
    }
}

/// Convenience enum for the renderer to know which camera to use this frame.
#[derive(Clone, Copy, Debug)]
pub enum Camera {
    Two(Camera2D),
    Three(Camera3D),
}

impl Camera {
    pub fn view_proj(&self) -> Mat4 {
        match self {
            Camera::Two(c) => c.view_proj(),
            Camera::Three(c) => c.view_proj(),
        }
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        match self {
            Camera::Two(c) => {
                c.viewport_w = w;
                c.viewport_h = h;
            }
            Camera::Three(c) => {
                c.viewport_w = w;
                c.viewport_h = h;
            }
        }
    }
}
