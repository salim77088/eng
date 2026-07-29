//! 3D mesh - a vertex+index buffer pair plus a bound texture and a tint.
//! The renderer uploads these once and re-renders as needed.

use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use lumina_core::math::Vec3;
use parking_lot::RwLock;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal:   [f32; 3],
    pub uv:       [f32; 2],
}

impl Vertex {
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
    ];
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: Arc<wgpu::Buffer>,
    pub index_buffer: Arc<wgpu::Buffer>,
    pub index_count: u32,
    pub texture: Texture,
    pub tint: [f32; 4],
    /// A user-settable model matrix that overrides the transform component
    /// when present. Used by the editor for gizmo previewing.
    pub override_model: RwLock<Option<[[f32; 4]; 4]>>,
}

impl Mesh {
    /// Build a mesh from vertices and indices. The texture is the engine's
    /// white fallback if none is supplied.
    pub fn new(
        device: &wgpu::Device,
        vertices: &[Vertex],
        indices: &[u32],
        texture: Texture,
    ) -> Self {
        let vertex_buffer = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("lumina mesh vb"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        ));
        let index_buffer = Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("lumina mesh ib"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        ));
        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            texture,
            tint: [1.0, 1.0, 1.0, 1.0],
            override_model: RwLock::new(None),
        }
    }

    /// A unit cube - useful as a default primitive and for the editor's
    /// placeholder gizmo.
    pub fn cube(device: &wgpu::Device, texture: Texture) -> Self {
        // 24 verts (4 per face), 36 indices. Normals point outward.
        let s = 0.5;
        let positions = [
            // +X
            ([ s,-s,-s],[ s,-s, s],[ s, s, s],[ s, s,-s]),
            // -X
            ([-s,-s, s],[-s,-s,-s],[-s, s,-s],[-s, s, s]),
            // +Y
            ([-s, s,-s],[ s, s,-s],[ s, s, s],[-s, s, s]),
            // -Y
            ([-s,-s, s],[ s,-s, s],[ s,-s,-s],[-s,-s,-s]),
            // +Z
            ([-s,-s, s],[ s,-s, s],[ s, s, s],[-s, s, s]),
            // -Z
            ([ s,-s,-s],[-s,-s,-s],[-s, s,-s],[ s, s,-s]),
        ];
        let uvs = [[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (i, face) in positions.iter().enumerate() {
            let normal = match i {
                0 => [1.0,0.0,0.0], 1 => [-1.0,0.0,0.0],
                2 => [0.0,1.0,0.0], 3 => [0.0,-1.0,0.0],
                4 => [0.0,0.0,1.0], 5 => [0.0,0.0,-1.0],
                _ => [0.0,0.0,0.0],
            };
            let corners = [face.0, face.1, face.2, face.3];
            for (j, p) in corners.iter().enumerate() {
                vertices.push(Vertex { position: *p, normal, uv: uvs[j] });
            }
            let base = (i * 4) as u32;
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
        Self::new(device, &vertices, &indices, texture)
    }

    /// Build a flat ground plane (1x1, centered at origin, in the XZ plane).
    pub fn plane(device: &wgpu::Device, texture: Texture) -> Self {
        let s = 0.5;
        let vertices = [
            Vertex { position: [-s, 0.0,  s], normal: [0.0,1.0,0.0], uv: [0.0,0.0] },
            Vertex { position: [ s, 0.0,  s], normal: [0.0,1.0,0.0], uv: [1.0,0.0] },
            Vertex { position: [ s, 0.0, -s], normal: [0.0,1.0,0.0], uv: [1.0,1.0] },
            Vertex { position: [-s, 0.0, -s], normal: [0.0,1.0,0.0], uv: [0.0,1.0] },
        ];
        let indices = [0u32, 1, 2, 0, 2, 3];
        Self::new(device, &vertices, &indices, texture)
    }

    pub fn from_obj_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
        fallback_texture: Texture,
    ) -> anyhow::Result<Self> {
        // Minimal OBJ loader - positions + faces only. Good enough for the
        // demo assets shipped with Lumina. For full glTF support, use the
        // `gltf` crate (TODO in v0.2).
        let src = std::fs::read_to_string(path)?;
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut verts: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for line in src.lines() {
            let mut tok = line.split_whitespace();
            match tok.next() {
                Some("v") => {
                    let x: f32 = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let y: f32 = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let z: f32 = tok.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    positions.push([x, y, z]);
                }
                Some("f") => {
                    let mut face_idx = Vec::new();
                    for t in tok {
                        let vidx = t.split('/').next().and_then(|s| s.parse::<usize>().ok());
                        if let Some(i) = vidx {
                            let p = positions.get(i - 1).copied().unwrap_or([0.0;3]);
                            let v = Vertex { position: p, normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] };
                            face_idx.push(verts.len() as u32);
                            verts.push(v);
                        }
                    }
                    // Fan-triangulate.
                    if face_idx.len() >= 3 {
                        for i in 1..face_idx.len() - 1 {
                            indices.push(face_idx[0]);
                            indices.push(face_idx[i]);
                            indices.push(face_idx[i + 1]);
                        }
                    }
                }
                _ => {}
            }
        }
        if verts.is_empty() {
            anyhow::bail!("OBJ {:?} had no geometry", path);
        }
        // Recompute flat normals.
        for i in (0..indices.len()).step_by(3) {
            let a = verts[indices[i] as usize].position;
            let b = verts[indices[i + 1] as usize].position;
            let c = verts[indices[i + 2] as usize].position;
            let n = Vec3::from(b) - Vec3::from(a);
            let m = Vec3::from(c) - Vec3::from(a);
            let normal = n.cross(m).normalize_or_zero().to_array();
            verts[indices[i] as usize].normal = normal;
            verts[indices[i + 1] as usize].normal = normal;
            verts[indices[i + 2] as usize].normal = normal;
        }
        Ok(Self::new(device, &verts, &indices, fallback_texture))
    }
}

// We need BufferInitDescriptor - bring in the util trait.
use wgpu::util::DeviceExt;
