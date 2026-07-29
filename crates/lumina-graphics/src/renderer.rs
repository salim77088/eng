//! The main renderer. Owns the device/queue/surface, the sprite pipeline,
//! the mesh pipeline, and a frame-local camera uniform buffer. Renders
//! are submitted via `render()`, which takes a closure that records
//! draw calls into an in-memory command list first.

use crate::camera::Camera;
use crate::mesh::{Mesh, Vertex as MeshVertex};
use crate::shader::{MESH_SHADER, PARTICLE_SHADER, SPRITE_SHADER};
use crate::sprite::{SpriteBatch, SpriteVertex};
use crate::texture::Texture;
use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use lumina_core::math::Mat4;
use lumina_core::Transform;
use parking_lot::RwLock;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub config: RwLock<wgpu::SurfaceConfiguration>,
    pub msaa_sample_count: u32,

    // Pipelines
    sprite_pipeline: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    particle_pipeline: wgpu::RenderPipeline,

    // Bind group layouts
    sprite_bg_layout: wgpu::BindGroupLayout,
    mesh_bg_layout: wgpu::BindGroupLayout,
    particle_bg_layout: wgpu::BindGroupLayout,

    // Camera uniform buffer (updated each frame).
    camera_buffer: wgpu::Buffer,

    // White fallback texture (1x1).
    pub white: Texture,

    // Depth texture (recreated on resize).
    depth_texture: RwLock<Texture>,
}

impl Renderer {
    /// Create the renderer against an already-created window surface.
    pub async fn new(
        surface: wgpu::Surface<'static>,
        window_size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: crate::preferred_backends(),
            flags: wgpu::InstanceFlags::default(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        // NOTE: surface was created from the window by the caller.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("no suitable wgpu adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("lumina device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .context("failed to acquire wgpu device")?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(
                caps.formats
                    .first()
                    .copied()
                    .unwrap_or(wgpu::TextureFormat::Bgra8Unorm),
            );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lumina camera ubo"),
            size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // White fallback texture.
        let white = Texture::white_fallback(&device, &queue);

        // Depth texture (matches surface size).
        let depth_texture = Texture::depth(&device, config.width, config.height, "lumina depth");

        // ----- Sprite pipeline -----
        let sprite_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lumina sprite bg layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lumina sprite shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });
        let sprite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("lumina sprite pipeline layout"),
                bind_group_layouts: &[&sprite_bg_layout],
                push_constant_ranges: &[],
            });
        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lumina sprite pipeline"),
            layout: Some(&sprite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sprite_shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[SpriteVertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &sprite_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ----- Mesh pipeline -----
        let mesh_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lumina mesh bg layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lumina mesh shader"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lumina mesh pipeline layout"),
            bind_group_layouts: &[&mesh_bg_layout],
            push_constant_ranges: &[],
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lumina mesh pipeline"),
            layout: Some(&mesh_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[MeshVertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ----- Particle pipeline (reuses SpriteVertex layout, no texture) -----
        let particle_bg_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("lumina particle bg layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lumina particle shader"),
            source: wgpu::ShaderSource::Wgsl(PARTICLE_SHADER.into()),
        });
        let particle_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("lumina particle pipeline layout"),
                bind_group_layouts: &[&particle_bg_layout],
                push_constant_ranges: &[],
            });
        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lumina particle pipeline"),
            layout: Some(&particle_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[SpriteVertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config: RwLock::new(config),
            msaa_sample_count: 1,
            sprite_pipeline,
            mesh_pipeline,
            particle_pipeline,
            sprite_bg_layout,
            mesh_bg_layout,
            particle_bg_layout,
            camera_buffer,
            white,
            depth_texture: RwLock::new(depth_texture),
        })
    }

    pub fn resize(&self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        {
            let mut cfg = self.config.write();
            cfg.width = width;
            cfg.height = height;
            self.surface.configure(&self.device, &cfg);
        }
        // Recreate depth texture.
        let new_depth = Texture::depth(&self.device, width, height, "lumina depth");
        *self.depth_texture.write() = new_depth;
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.read().format
    }

    pub fn surface_size(&self) -> (u32, u32) {
        let c = self.config.read();
        (c.width, c.height)
    }

    /// Update the camera uniform. Call once per frame before rendering.
    pub fn update_camera(&self, camera: &Camera) {
        let vp: Mat4 = camera.view_proj();
        let uniform = CameraUniform {
            view_proj: vp.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Try to acquire the current surface texture. If the surface is lost
    /// or outdated (common after a resize, minimize, or DPI change), the
    /// surface is reconfigured and the caller should skip this frame.
    /// Returns `Ok(texture)` on success, `Ok(None)` if the caller should
    /// skip the frame (surface reconfigured, try again next frame), or
    /// `Err` on a fatal error.
    pub fn acquire_frame(&self) -> Result<Option<wgpu::SurfaceTexture>> {
        match self.surface.get_current_texture() {
            Ok(tex) => Ok(Some(tex)),
            Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                log::warn!("surface lost/outdated - reconfiguring");
                let (w, h) = self.surface_size();
                if w > 0 && h > 0 {
                    let cfg = self.config.read().clone();
                    self.surface.configure(&self.device, &cfg);
                }
                Ok(None)
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("surface acquire timed out - skipping frame");
                Ok(None)
            }
            Err(e) => Err(anyhow::anyhow!("fatal surface error: {e}")),
        }
    }

    /// Render the game scene into a provided texture view. Does NOT acquire
    /// or present the surface — the caller is responsible for that, so the
    /// same surface texture can be reused for an overlay pass (e.g. egui)
    /// in the same frame. This is the correct single-acquire-per-frame
    /// pattern; acquiring the surface twice per frame corrupts the
    /// swapchain on some backends (notably DX12 on Windows).
    pub fn render_to_view(
        &self,
        view: &wgpu::TextureView,
        clear: [f64; 4],
        record: impl FnOnce(&mut FrameRecorder),
    ) {
        let depth = self.depth_texture.read();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("lumina frame encoder"),
            });

        // Game pass: clear + draw everything into the provided view.
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lumina main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: clear[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let mut recorder = FrameRecorder {
                pass: &mut rpass,
                device: &self.device,
                camera_buffer: &self.camera_buffer,
                white: &self.white,
                sprite_pipeline: &self.sprite_pipeline,
                mesh_pipeline: &self.mesh_pipeline,
                particle_pipeline: &self.particle_pipeline,
                sprite_bg_layout: &self.sprite_bg_layout,
                mesh_bg_layout: &self.mesh_bg_layout,
                particle_bg_layout: &self.particle_bg_layout,
            };
            record(&mut recorder);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

/// Per-frame draw command recorder. Passed to the closure in `render_to_view()`.
pub struct FrameRecorder<'a> {
    pass: &'a mut wgpu::RenderPass<'a>,
    device: &'a wgpu::Device,
    camera_buffer: &'a wgpu::Buffer,
    white: &'a Texture,
    sprite_pipeline: &'a wgpu::RenderPipeline,
    mesh_pipeline: &'a wgpu::RenderPipeline,
    particle_pipeline: &'a wgpu::RenderPipeline,
    sprite_bg_layout: &'a wgpu::BindGroupLayout,
    mesh_bg_layout: &'a wgpu::BindGroupLayout,
    particle_bg_layout: &'a wgpu::BindGroupLayout,
}

impl<'a> FrameRecorder<'a> {
    /// Draw all sprites in the batch. The whole batch is uploaded as one
    /// dynamic vertex buffer and drawn in a single call. The texture is
    /// taken from the batch (the texture of the first sprite pushed); if
    /// none is set, the renderer's 1x1 white fallback is used.
    pub fn draw_sprites(&mut self, batch: &SpriteBatch) {
        if batch.vertices.is_empty() {
            return;
        }
        let vb = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("lumina sprite vb"),
                contents: bytemuck::cast_slice(&batch.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let view = batch
            .texture
            .as_ref()
            .map(|t| t.view.clone())
            .unwrap_or_else(|| self.white.view.clone());
        let sampler = batch
            .texture
            .as_ref()
            .map(|t| t.sampler.clone())
            .unwrap_or_else(|| self.white.sampler.clone());
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lumina sprite bg"),
            layout: self.sprite_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        self.pass.set_pipeline(self.sprite_pipeline);
        self.pass.set_bind_group(0, &bg, &[]);
        self.pass.set_vertex_buffer(0, vb.slice(..));
        self.pass.draw(0..batch.vertices.len() as u32, 0..1);
    }

    /// Draw a mesh with the given world transform.
    pub fn draw_mesh(&mut self, mesh: &Mesh, transform: &Transform) {
        let model = match *mesh.override_model.read() {
            Some(m) => m,
            None => transform.to_matrix().to_cols_array_2d(),
        };
        let model_uniform = ModelUniform {
            model,
            color: mesh.tint,
        };
        let model_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("lumina model ubo"),
                contents: bytemuck::cast_slice(&[model_uniform]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lumina mesh bg"),
            layout: self.mesh_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: model_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&mesh.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&mesh.texture.sampler),
                },
            ],
        });
        self.pass.set_pipeline(self.mesh_pipeline);
        self.pass.set_bind_group(0, &bg, &[]);
        self.pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.pass
            .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }

    /// Draw a particle batch (vertices already built by the particle system).
    pub fn draw_particles(&mut self, vertices: &[SpriteVertex]) {
        if vertices.is_empty() {
            return;
        }
        let vb = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("lumina particle vb"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lumina particle bg"),
            layout: self.particle_bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera_buffer.as_entire_binding(),
            }],
        });
        self.pass.set_pipeline(self.particle_pipeline);
        self.pass.set_bind_group(0, &bg, &[]);
        self.pass.set_vertex_buffer(0, vb.slice(..));
        self.pass.draw(0..vertices.len() as u32, 0..1);
    }
}
