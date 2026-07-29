//! Lumina Particles - a small CPU-driven particle system. Particles are
//! emitted from an origin, simulated on the CPU, and rendered as
//! tinted quads via the sprite pipeline.

use bytemuck::{Pod, Zeroable};
use lumina_core::math::{lerp, Vec3};
use lumina_graphics::sprite::SpriteVertex;
use parking_lot::RwLock;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Particle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub color:    [f32; 4],
    pub life:     f32,    // seconds remaining
    pub max_life: f32,
    pub size:     f32,
}

#[derive(Clone, Debug)]
pub struct EmitterConfig {
    pub rate: f32,             // particles per second
    pub lifetime: [f32; 2],    // min, max
    pub speed:    [f32; 2],
    pub size:     [f32; 2],
    pub color_start: [f32; 4],
    pub color_end:   [f32; 4],
    pub gravity: [f32; 3],
    pub spread: f32,           // radians (full cone angle)
    pub origin: [f32; 3],
    pub direction: [f32; 3],   // normalized
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            rate: 30.0,
            lifetime: [0.5, 1.5],
            speed: [20.0, 60.0],
            size: [4.0, 10.0],
            color_start: [1.0, 0.85, 0.3, 1.0],
            color_end:   [1.0, 0.2, 0.0, 0.0],
            gravity: [0.0, -30.0, 0.0],
            spread: 1.0,
            origin: [0.0; 3],
            direction: [0.0, 1.0, 0.0],
        }
    }
}

pub struct ParticleSystem {
    pub config: EmitterConfig,
    particles: Vec<Particle>,
    emit_accum: f32,
    max_particles: usize,
}

impl ParticleSystem {
    pub fn new(config: EmitterConfig, max_particles: usize) -> Self {
        Self {
            config,
            particles: Vec::with_capacity(max_particles),
            emit_accum: 0.0,
            max_particles,
        }
    }

    /// Advance the simulation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        // Emit.
        self.emit_accum += self.config.rate * dt;
        while self.emit_accum >= 1.0 {
            self.emit_accum -= 1.0;
            if self.particles.len() < self.max_particles {
                self.particles.push(self.spawn());
            }
        }
        // Integrate.
        for p in &mut self.particles {
            p.velocity[0] += self.config.gravity[0] * dt;
            p.velocity[1] += self.config.gravity[1] * dt;
            p.velocity[2] += self.config.gravity[2] * dt;
            p.position[0] += p.velocity[0] * dt;
            p.position[1] += p.velocity[1] * dt;
            p.position[2] += p.velocity[2] * dt;
            p.life -= dt;
        }
        // Reap dead.
        self.particles.retain(|p| p.life > 0.0);
    }

    fn spawn(&self) -> Particle {
        let mut rng = TinyRng::from_entropy();
        let life = rng.range(self.config.lifetime[0], self.config.lifetime[1]);
        let speed = rng.range(self.config.speed[0], self.config.speed[1]);
        let size = rng.range(self.config.size[0], self.config.size[1]);
        // Random direction inside a cone around `direction`.
        let dir = random_in_cone(&self.config.direction, self.config.spread, &mut rng);
        let vel = [dir.x * speed, dir.y * speed, dir.z * speed];
        Particle {
            position: self.config.origin,
            velocity: vel,
            color: self.config.color_start,
            life,
            max_life: life,
            size,
        }
    }

    /// Build vertex data for rendering. Each particle becomes a small
    /// quad (two triangles) with the color interpolated from start to end
    /// based on remaining life.
    pub fn build_vertices(&self, out: &mut Vec<SpriteVertex>) {
        for p in &self.particles {
            let t = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
            let r = lerp(self.config.color_start[0], self.config.color_end[0], t);
            let g = lerp(self.config.color_start[1], self.config.color_end[1], t);
            let b = lerp(self.config.color_start[2], self.config.color_end[2], t);
            let a = lerp(self.config.color_start[3], self.config.color_end[3], t);
            let color = [r, g, b, a];
            let s = p.size;
            let [x, y, _z] = p.position;
            // Billboard-ish quad in screen space (z ignored by the
            // 2D-style pipeline we feed it into).
            let corners = [
                [-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5],
            ];
            let uvs = [[0.0,1.0],[1.0,1.0],[1.0,0.0],[0.0,0.0]];
            let v0 = SpriteVertex { position: [x + corners[0][0]*s, y + corners[0][1]*s], uv: uvs[0], color };
            let v1 = SpriteVertex { position: [x + corners[1][0]*s, y + corners[1][1]*s], uv: uvs[1], color };
            let v2 = SpriteVertex { position: [x + corners[2][0]*s, y + corners[2][1]*s], uv: uvs[2], color };
            let v3 = SpriteVertex { position: [x + corners[3][0]*s, y + corners[3][1]*s], uv: uvs[3], color };
            out.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
        }
    }

    pub fn count(&self) -> usize { self.particles.len() }
}

/// Random direction inside a cone of half-angle `spread` around `axis`.
fn random_in_cone(axis: &[f32; 3], spread: f32, rng: &mut TinyRng) -> Vec3 {
    let axis_v = Vec3::from_array(*axis).normalize_or_zero();
    let half = spread * 0.5;
    let cos_a = (half * 0.5).cos(); // pick within half the spread for nicer look
    let z = rng.range(cos_a, 1.0);
    let phi = rng.range(0.0, std::f32::consts::TAU);
    let r = (1.0 - z * z).sqrt();
    let local = Vec3::new(r * phi.cos(), r * phi.sin(), z);
    // Rotate `local` so that +Z aligns with `axis_v`.
    let up = if axis_v.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    let right = axis_v.cross(up).normalize_or_zero();
    let true_up = axis_v.cross(right).normalize_or_zero();
    right * local.x + true_up * local.y + axis_v * local.z
}

/// Tiny xorshift PRNG - good enough for particles, no need to pull in
/// a crate for v0.1. Seed from a hash of `std::time` + an atomic counter
/// so successive emitters don't all start in sync.
struct TinyRng { state: u64 }
impl TinyRng {
    fn from_entropy() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xC0FFEE);
        // Mix in a static counter so two emitters created in the same
        // nanosecond still diverge.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let c = COUNTER.fetch_add(0x9E3779B97F4A7C15, std::sync::atomic::Ordering::Relaxed);
        let mut s = nanos ^ c.rotate_left(17) ^ 0xDEADBEEF;
        if s == 0 { s = 0xDEADBEEF; }
        Self { state: s }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.state = x;
        x
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u64() >> 11) as f32 / ((1u64 << 53) as f32);
        lo + (hi - lo) * u
    }
}

/// Convenience wrapper for the engine to hold multiple named particle systems.
#[derive(Default)]
pub struct ParticleRegistry {
    pub systems: RwLock<Vec<Arc<RwLock<ParticleSystem>>>>,
}

impl ParticleRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn register(&self, sys: ParticleSystem) -> usize {
        let mut guard = self.systems.write();
        let idx = guard.len();
        guard.push(Arc::new(RwLock::new(sys)));
        idx
    }
    pub fn update_all(&self, dt: f32) {
        for sys in self.systems.read().iter() {
            sys.write().update(dt);
        }
    }
    pub fn build_vertices(&self, out: &mut Vec<SpriteVertex>) {
        for sys in self.systems.read().iter() {
            sys.read().build_vertices(out);
        }
    }
}
