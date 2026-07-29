//! Lumina Engine - main entry point.
//!
//! Boots the window, renderer, audio, script watcher, particle registry,
//! and editor, then runs the main loop. The editor and the game share
//! the same wgpu surface: the game renders first into the surface, then
//! egui renders on top.

use anyhow::{Context, Result};
use lumina_audio::AudioEngine;
use lumina_core::{
    banner, ecs::Name, input::KeyState, log as lumina_log, Input, Scene, Time, Transform, World,
};
use lumina_editor::{panels, Editor, EditorState};
use lumina_graphics::{Camera, Camera3D, Mesh, Renderer, Sprite, SpriteBatch};
use lumina_particles::{EmitterConfig, ParticleRegistry, ParticleSystem};
use lumina_script::ScriptWatcher;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

fn main() -> Result<()> {
    lumina_log::init();
    log::info!("{}", banner());

    let event_loop = EventLoop::new()?;
    let mut app = LuminaApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// The winit 0.30 `ApplicationHandler` implementation for Lumina.
struct LuminaApp {
    window: Option<Arc<Window>>,
    renderer: Option<Arc<Renderer>>,
    editor: Option<Arc<Editor>>,
    audio: Option<Arc<AudioEngine>>,
    scene: Option<Arc<RwLock<Scene>>>,
    time: Option<Arc<RwLock<Time>>>,
    input: Option<Arc<RwLock<Input>>>,
    particles: Option<Arc<ParticleRegistry>>,
    scripts: Option<Arc<ScriptWatcher>>,
    cube_mesh: Option<Arc<Mesh>>,
    plane_mesh: Option<Arc<Mesh>>,
    camera: Option<Arc<RwLock<Camera>>>,
    sprite_batch: Option<Arc<RwLock<SpriteBatch>>>,
}

impl LuminaApp {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            editor: None,
            audio: None,
            scene: None,
            time: None,
            input: None,
            particles: None,
            scripts: None,
            cube_mesh: None,
            plane_mesh: None,
            camera: None,
            sprite_batch: None,
        }
    }

    fn boot(&mut self, window: Arc<Window>) -> Result<()> {
        // Boot the renderer (async - pollster makes it sync for us).
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: lumina_graphics::preferred_backends(),
            flags: wgpu::InstanceFlags::default(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });
        let surface = instance
            .create_surface(window.clone())
            .context("create surface")?;
        let renderer = pollster::block_on(Renderer::new(surface, window.inner_size()))?;
        let renderer = Arc::new(renderer);

        // Boot audio.
        let audio = Arc::new(AudioEngine::new());
        if !audio.is_available() {
            log::warn!("Audio backend unavailable - running without sound.");
        }

        // Boot the editor UI.
        let editor_state = EditorState {
            assets_root: Some(PathBuf::from("assets")),
            ..EditorState::default()
        };
        let editor = Arc::new(Editor::new(
            &window,
            &renderer.device,
            renderer.surface_format(),
            editor_state,
        ));

        // Build the demo scene.
        let mut scene = Scene::new("Main");
        scene.world.spawn((
            Name("Cube".into()),
            Transform {
                position: glam::Vec3::new(0.0, 0.5, 0.0),
                ..Default::default()
            },
        ));
        scene
            .world
            .spawn((Name("Ground".into()), Transform::default()));
        scene.world.spawn((
            Name("Logo".into()),
            Transform {
                position: glam::Vec3::new(0.0, 1.5, 0.0),
                ..Default::default()
            },
        ));

        let cube_mesh = Arc::new(Mesh::cube(&renderer.device, renderer.white.clone()));
        let plane_mesh = Arc::new(Mesh::plane(&renderer.device, renderer.white.clone()));

        let mut particles = ParticleRegistry::new();
        particles.register(ParticleSystem::new(
            EmitterConfig {
                origin: [0.0, 1.0, 0.0],
                ..EmitterConfig::default()
            },
            2048,
        ));

        let scripts = ScriptWatcher::new();
        let demo_script_path = PathBuf::from("assets/demo.lumi");
        if demo_script_path.exists() {
            match scripts.add(&demo_script_path) {
                Ok(_) => log::info!("loaded demo script: {:?}", demo_script_path),
                Err(e) => log::warn!("demo script load failed: {e}"),
            }
        } else {
            log::info!(
                "no demo script at {:?}; running without scripts",
                demo_script_path
            );
        }

        let camera = Camera::Three(Camera3D::new(1280.0, 720.0));

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.editor = Some(editor);
        self.audio = Some(audio);
        self.scene = Some(Arc::new(RwLock::new(scene)));
        self.time = Some(Arc::new(RwLock::new(Time::new())));
        self.input = Some(Arc::new(RwLock::new(Input::new())));
        self.particles = Some(Arc::new(particles));
        self.scripts = Some(Arc::new(scripts));
        self.cube_mesh = Some(cube_mesh);
        self.plane_mesh = Some(plane_mesh);
        self.camera = Some(Arc::new(RwLock::new(camera)));
        self.sprite_batch = Some(Arc::new(RwLock::new(SpriteBatch::new())));

        Ok(())
    }

    fn tick(&mut self) {
        let time = self.time.as_ref().unwrap();
        let scripts = self.scripts.as_ref().unwrap();
        let particles = self.particles.as_ref().unwrap();
        let input = self.input.as_ref().unwrap();

        // 1. Time update.
        let _steps = time.write().tick();

        // 2. Hot-reload scripts.
        let reloaded = scripts.poll();
        if !reloaded.is_empty() {
            for i in &reloaded {
                log::info!("reloaded script #{i}");
            }
        }

        // 3. Update particles.
        let dt = time.read().delta;
        particles.update_all(dt);

        // 4. Clear per-frame input edges.
        input.write().end_frame();
    }

    fn render(&mut self) {
        let renderer = self.renderer.as_ref().unwrap();
        let camera = self.camera.as_ref().unwrap();
        let time = self.time.as_ref().unwrap();
        let scene = self.scene.as_ref().unwrap();
        let cube_mesh = self.cube_mesh.as_ref().unwrap();
        let plane_mesh = self.plane_mesh.as_ref().unwrap();
        let sprite_batch = self.sprite_batch.as_ref().unwrap();
        let particles = self.particles.as_ref().unwrap();
        let editor = self.editor.as_ref().unwrap();
        let input = self.input.as_ref().unwrap();

        // Update camera uniform.
        let cam = *camera.read();
        renderer.update_camera(&cam);

        // Build sprite batch for this frame.
        {
            let mut batch = sprite_batch.write();
            batch.clear();
            let t = time.read().elapsed;
            let x = (t * 60.0).sin() * 200.0;
            let sprite = Sprite {
                texture: renderer.white.clone(),
                position: [x, 200.0],
                rotation: t * 0.5,
                scale: [64.0, 64.0],
                color: [0.0, 0.86, 0.90, 1.0],
                sub_rect: [0.0, 0.0, 1.0, 1.0],
            };
            batch.push(&sprite);
        }

        // Build particle vertex buffer.
        let mut particle_vertices = Vec::new();
        particles.build_vertices(&mut particle_vertices);

        // Snapshot entities for rendering.
        let scene_guard = scene.read();
        let mut mesh_draws: Vec<(Transform, String)> = Vec::new();
        let mut query = scene_guard.world.raw().query::<(&Transform, &Name)>();
        for (id, (t, n)) in query.iter() {
            let _ = id;
            mesh_draws.push((*t, n.0.clone()));
        }
        drop(query);
        drop(scene_guard);

        // ---- Single-acquire render frame ----
        // Acquire the surface texture ONCE. Render the game scene into it,
        // then render the egui editor on top with LoadOp::Load, then present
        // ONCE. Acquiring the surface twice per frame corrupts the swapchain
        // on DX12 (Windows), causing "Surface does not exist" panics.
        let surface_texture = match renderer.acquire_frame() {
            Ok(Some(tex)) => tex,
            Ok(None) => return, // surface reconfigured — skip this frame
            Err(e) => {
                log::error!("fatal surface error: {e}");
                return;
            }
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Game pass: clear + draw meshes, particles, sprites.
        let clear = [0.05, 0.06, 0.08, 1.0];
        let cube_mesh = cube_mesh.clone();
        let plane_mesh = plane_mesh.clone();
        let sprite_batch = sprite_batch.clone();
        renderer.render_to_view(&view, clear, |rec| {
            for (t, name) in &mesh_draws {
                let mesh = if name == "Ground" {
                    &plane_mesh
                } else {
                    &cube_mesh
                };
                let mut t2 = *t;
                if name == "Ground" {
                    t2.scale = glam::Vec3::new(20.0, 1.0, 20.0);
                } else if name == "Cube" {
                    t2.scale = glam::Vec3::new(2.0, 2.0, 2.0);
                }
                rec.draw_mesh(mesh, &t2);
            }
            rec.draw_particles(&particle_vertices);
            let batch = sprite_batch.read();
            rec.draw_sprites(&batch);
        });

        // Egui pass: build UI, then paint on top of the game scene using the
        // SAME surface texture view (LoadOp::Load keeps the game's pixels).
        self.run_editor_ui(&view);

        // Present the single surface texture exactly once.
        surface_texture.present();
    }

    fn run_editor_ui(&mut self, view: &wgpu::TextureView) {
        let editor = self.editor.as_ref().unwrap();
        let scene = self.scene.as_ref().unwrap();
        let time = self.time.as_ref().unwrap();
        let particles = self.particles.as_ref().unwrap();
        let renderer = self.renderer.as_ref().unwrap();
        let window = self.window.as_ref().unwrap();

        // Collect all entity info in a single query pass to avoid borrow issues.
        let entities: Vec<(u64, String, Option<[f32; 3]>, Option<[f32; 3]>)> = {
            let scene_guard = scene.read();
            let mut query = scene_guard
                .world
                .raw()
                .query::<(&Name, Option<&Transform>)>();
            let result: Vec<_> = query
                .iter()
                .map(|(e, (name, transform))| {
                    (
                        e.id() as u64,
                        name.0.clone(),
                        transform.map(|t| t.position.to_array()),
                        transform.map(|t| t.scale.to_array()),
                    )
                })
                .collect();
            // `query` is dropped here (reverse declaration order), releasing
            // the world borrow before `scene_guard` is dropped.
            result
        };

        let names: Vec<(u64, String)> = entities
            .iter()
            .map(|(id, name, _, _)| (*id, name.clone()))
            .collect();

        let selected_id = editor.state.read().selected_entity;
        let inspector_info = selected_id.and_then(|id| {
            let (_, name, pos, scale) = entities.iter().find(|(eid, _, _, _)| *eid == id)?;
            let asset = if name == "Ground" {
                "plane.lumina".to_string()
            } else {
                "cube.lumina".to_string()
            };
            Some(panels::InspectorInfo {
                name: name.clone(),
                position: pos.unwrap_or([0.0; 3]),
                scale: scale.unwrap_or([1.0; 3]),
                asset,
                script: "demo.lumi".into(),
            })
        });

        // Update stats.
        {
            let mut state = editor.state.write();
            state.stats.fps = time.read().fps();
            state.stats.entity_count = scene.read().world.entity_count();
            state.stats.particles = particles
                .systems
                .read()
                .iter()
                .map(|s| s.read().count())
                .sum();
        }

        // Run egui.
        let (shapes, textures_delta) = editor.run_ui(window, |ctx| {
            let mut state = editor.state.write();
            panels::top_menu(ctx, &mut state);
            panels::console(ctx, &mut state);
            panels::asset_browser(ctx, &mut state, list_assets());
            panels::hierarchy(ctx, &mut state, names.clone());
            panels::inspector(ctx, &mut state, inspector_info.clone());
            panels::viewport(ctx, &mut state, None);
            panels::about(ctx, &mut state);
        });

        // Paint egui on top of the game scene using the SAME surface view
        // that was already acquired in render(). The editor's paint() uses
        // LoadOp::Load so the game pixels are preserved.
        let (w, h) = renderer.surface_size();
        let scale = window.scale_factor() as f32;
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [w, h],
            pixels_per_point: scale,
        };
        editor.paint(
            &renderer.device,
            &renderer.queue,
            view,
            screen_descriptor,
            shapes,
            textures_delta,
        );
    }
}

impl ApplicationHandler for LuminaApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already booted.
        }
        let attrs = WindowAttributes::default()
            .with_title("Lumina Engine")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        if let Err(e) = self.boot(window) {
            log::error!("boot failed: {e}");
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(window), Some(editor), Some(input)) = (
            self.window.as_ref(),
            self.editor.as_ref(),
            self.input.as_ref(),
        ) else {
            return;
        };

        // Let egui handle the event first.
        let consumed = editor.on_event(window, &event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &self.renderer {
                    renderer.resize(size.width, size.height);
                }
                if let Some(camera) = &self.camera {
                    camera.write().resize(size.width as f32, size.height as f32);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                if !consumed {
                    let scancode = code_to_scancode(code);
                    let ks = match state {
                        ElementState::Pressed => KeyState::Pressed,
                        ElementState::Released => KeyState::Released,
                    };
                    input.write().on_key(scancode, ks);
                    if code == KeyCode::Escape {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                input
                    .write()
                    .on_cursor(position.x as f32, position.y as f32);
            }
            WindowEvent::RedrawRequested => {
                self.tick();
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn list_assets() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir("assets") {
        for e in rd.flatten() {
            if let Some(name) = e.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Translate a KeyCode to a rough Linux evdev scancode, used by the
/// input module. We only need a handful for the demo (WASD, space, esc).
fn code_to_scancode(code: KeyCode) -> u32 {
    match code {
        KeyCode::KeyW => 17,
        KeyCode::KeyA => 30,
        KeyCode::KeyS => 31,
        KeyCode::KeyD => 32,
        KeyCode::Space => 57,
        KeyCode::Escape => 1,
        KeyCode::Enter => 28,
        KeyCode::ArrowUp => 103,
        KeyCode::ArrowDown => 108,
        KeyCode::ArrowLeft => 105,
        KeyCode::ArrowRight => 106,
        _ => 0,
    }
}
