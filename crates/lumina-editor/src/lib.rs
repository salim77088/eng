//! Lumina Editor - the integrated egui-based editor UI.
//!
//! Provides a docking-style layout (Hierarchy | Viewport | Inspector,
//! with an Asset Browser tab and a Console tab). The editor is built to
//! be embedded into the main engine window, sharing the wgpu surface
//! via egui-wgpu.

use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiState;
use parking_lot::RwLock;
use std::sync::Arc;
use winit::window::Window;

pub mod panels;
pub mod state;

pub use state::EditorState;

/// Top-level editor facade. Owns the egui context, the winit integration,
/// and the wgpu renderer for egui itself (separate from the game's
/// renderer, which draws into the same surface via a sub-pass).
pub struct Editor {
    pub ctx: egui::Context,
    pub state: Arc<RwLock<EditorState>>,
    pub winit_state: RwLock<EguiState>,
    pub renderer: RwLock<EguiRenderer>,
}

impl Editor {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        initial_state: EditorState,
    ) -> Self {
        let ctx = egui::Context::default();
        let viewport = egui::ViewportId::ROOT;
        let winit_state = EguiState::new(
            ctx.clone(),
            viewport,
            window,
            None,
            None,
            None,
        );
        let renderer = EguiRenderer::new(
            device,
            surface_format,
            None, // no depth for egui
            1,    // msaa_samples
            false, // dithering
        );
        Self {
            ctx,
            state: Arc::new(RwLock::new(initial_state)),
            winit_state: RwLock::new(winit_state),
            renderer: RwLock::new(renderer),
        }
    }

    /// Handle a winit event. Returns true if egui consumed the event
    /// (so the game should not also act on it).
    pub fn on_event(&self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let mut state = self.winit_state.write();
        let response = state.on_window_event(window, event);
        response.consumed
    }

    /// Run the editor UI for this frame. The closure is called with the
    /// `egui::Context` so panels can be added. After this returns, the
    /// engine should call `paint()` to render egui on top of the game.
    pub fn run_ui(
        &self,
        window: &Window,
        run: impl FnOnce(&egui::Context),
    ) -> (Vec<egui::epaint::ClippedShape>, egui::TexturesDelta) {
        let raw_input = self.winit_state.write().take_egui_input(window);
        // egui's `Context::run` takes an `FnMut`, but our caller gave us
        // an `FnOnce`. We tuck it into an Option and take it on the first
        // invocation so it gets consumed exactly once.
        let mut run_opt = Some(run);
        let full_output = self.ctx.run(raw_input, |ctx| {
            if let Some(f) = run_opt.take() {
                f(ctx);
            }
        });
        (full_output.shapes, full_output.textures_delta)
    }

    /// Render egui's shapes onto a texture view using the egui-wgpu renderer.
    pub fn paint(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        shapes: Vec<egui::epaint::ClippedShape>,
        textures_delta: egui::TexturesDelta,
    ) {
        let pixels_per_point = screen_descriptor.pixels_per_point;
        let clipped_primitives = self.ctx.tessellate(shapes, pixels_per_point);
        let mut renderer = self.renderer.write();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("lumina egui encoder"),
        });

        for (id, image_delta) in textures_delta.set.into_iter() {
            renderer.update_texture(device, queue, id, &image_delta);
        }
        renderer.update_buffers(device, queue, &mut encoder, &clipped_primitives, &screen_descriptor);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lumina egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // SAFETY: `rpass` does not escape this scope. The `'static`
            // bound on `egui_wgpu::Renderer::render` is an API limitation,
            // not a real safety requirement - the render pass is consumed
            // before this block ends.
            let rpass_static: &mut wgpu::RenderPass<'static> =
                unsafe { std::mem::transmute::<&mut wgpu::RenderPass<'_>, &mut wgpu::RenderPass<'static>>(&mut rpass) };
            renderer.render(rpass_static, &clipped_primitives, &screen_descriptor);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}
