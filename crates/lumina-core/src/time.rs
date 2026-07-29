//! Time tracking: delta time, total elapsed time, fixed-update accumulator,
//! and a small FPS counter. All values are in seconds.

use std::time::{Duration, Instant};

/// Fixed timestep used by the simulation (60 Hz by default).
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Tracks wall-clock time between frames and an accumulator for the
/// fixed-update loop.
#[derive(Debug)]
pub struct Time {
    start: Instant,
    last: Instant,
    pub delta: f32,
    pub elapsed: f32,
    pub frame_count: u64,
    accumulator: f32,
    fps: f32,
    fps_accum: f32,
    fps_frames: u32,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last: now,
            delta: 0.0,
            elapsed: 0.0,
            frame_count: 0,
            accumulator: 0.0,
            fps: 0.0,
            fps_accum: 0.0,
            fps_frames: 0,
        }
    }

    /// Call once per frame, at the top of the loop, to refresh `delta`
    /// and `elapsed`. Returns the number of fixed-update steps that
    /// should run this frame.
    pub fn tick(&mut self) -> u32 {
        let now = Instant::now();
        let dur = now - self.last;
        self.last = now;
        self.delta = dur.as_secs_f32().min(0.25); // clamp huge spikes (e.g. debug pauses)
        self.elapsed = (now - self.start).as_secs_f32();
        self.frame_count += 1;

        // FPS rolling average over ~0.5s windows.
        self.fps_accum += self.delta;
        self.fps_frames += 1;
        if self.fps_accum >= 0.5 {
            self.fps = self.fps_frames as f32 / self.fps_accum;
            self.fps_accum = 0.0;
            self.fps_frames = 0;
        }

        self.accumulator += self.delta;
        let mut steps = 0u32;
        while self.accumulator >= FIXED_DT {
            self.accumulator -= FIXED_DT;
            steps += 1;
            // Hard-cap to avoid the "spiral of death" after a stall.
            if steps >= 5 {
                self.accumulator = 0.0;
                break;
            }
        }
        steps
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Time in seconds since the engine started.
    pub fn since_start(&self) -> Duration {
        self.start.elapsed()
    }
}
