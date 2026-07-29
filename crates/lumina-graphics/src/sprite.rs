//! 2D sprite batcher. Collects quads into a single dynamic vertex buffer
//! per texture and draws them all in one draw call.

use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub uv:       [f32; 2],
    pub color:    [f32; 4],
}

impl SpriteVertex {
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

/// A sprite - a textured quad with a tint, a sub-rectangle (in UV space),
/// a transform (position/rotation/scale in 2D), and an origin (the
/// "center" of the sprite in 0..=1 UV space).
#[derive(Clone, Debug)]
pub struct Sprite {
    pub texture: Texture,
    pub position: [f32; 2],
    pub rotation: f32, // radians
    pub scale:    [f32; 2],
    pub color:    [f32; 4],
    pub sub_rect: [f32; 4], // x, y, w, h in 0..=1 UV space
}

impl Sprite {
    pub fn new(texture: Texture) -> Self {
        let (w, h) = (texture.width as f32, texture.height as f32);
        Self {
            texture,
            position: [0.0, 0.0],
            rotation: 0.0,
            scale: [w, h],
            color: [1.0, 1.0, 1.0, 1.0],
            sub_rect: [0.0, 0.0, 1.0, 1.0],
        }
    }

    /// Build the 6 vertices (two triangles) for this sprite and append
    /// them to `out`.
    pub fn push_vertices(&self, out: &mut Vec<SpriteVertex>) {
        let [ox, oy, sw, sh] = self.sub_rect;
        let [sx, sy] = self.scale;
        let (cos, sin) = (self.rotation.cos(), self.rotation.sin());

        // 4 corners in local space (centered).
        let corners = [
            [-0.5, -0.5],
            [ 0.5, -0.5],
            [ 0.5,  0.5],
            [-0.5,  0.5],
        ];
        let uvs = [
            [ox,      oy + sh],
            [ox + sw, oy + sh],
            [ox + sw, oy],
            [ox,      oy],
        ];
        let color = self.color;
        let transform = |lx: f32, ly: f32| -> [f32; 2] {
            let x = lx * sx;
            let y = ly * sy;
            [
                self.position[0] + x * cos - y * sin,
                self.position[1] + x * sin + y * cos,
            ]
        };
        let v0 = SpriteVertex { position: transform(corners[0][0], corners[0][1]), uv: uvs[0], color };
        let v1 = SpriteVertex { position: transform(corners[1][0], corners[1][1]), uv: uvs[1], color };
        let v2 = SpriteVertex { position: transform(corners[2][0], corners[2][1]), uv: uvs[2], color };
        let v3 = SpriteVertex { position: transform(corners[3][0], corners[3][1]), uv: uvs[3], color };
        // Two triangles: 0,1,2 and 0,2,3.
        out.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
    }
}

/// Batched draw list. All sprites in a single batch share one texture
/// (the texture of the first sprite pushed). For multi-texture scenes,
/// create one batch per texture. Call `clear` after each frame.
#[derive(Default)]
pub struct SpriteBatch {
    pub vertices: Vec<SpriteVertex>,
    pub texture: Option<Texture>,
}

impl SpriteBatch {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, sprite: &Sprite) {
        if self.texture.is_none() {
            self.texture = Some(sprite.texture.clone());
        }
        sprite.push_vertices(&mut self.vertices);
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.texture = None;
    }

    pub fn len(&self) -> usize { self.vertices.len() }
    pub fn is_empty(&self) -> bool { self.vertices.is_empty() }
}
