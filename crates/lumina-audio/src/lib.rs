//! Lumina Audio - audio playback for the Lumina Engine.
//!
//! Two backends are provided, selected at compile time via the `kira`
//! Cargo feature:
//!
//! - `kira` (default): real audio playback via the `kira` crate. Requires
//!   ALSA dev headers on Linux, CoreAudio on macOS, WASAPI on Windows.
//! - disabled: a no-op `NullAudioEngine` that exposes the same API but
//!   does nothing. Lets the engine build on systems without an audio
//!   stack (headless CI, minimal containers, etc.).
//!
//! Both backends implement the [`AudioBackend`] trait, so the rest of
//! the engine is completely unaware of which one is active.

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

/// Trait shared by the real and null audio backends.
pub trait AudioBackend: Send + Sync {
    fn is_available(&self) -> bool;
    fn load(&self, name: &str, path: &Path) -> Result<()>;
    fn play(&self, name: &str, volume: f64) -> Result<()>;
    fn play_music(&self, path: &Path, volume: f64, loops: bool) -> Result<()>;
    fn set_master_volume(&self, volume: f64) -> Result<()>;
    fn stop_all(&self) -> Result<()>;
}

#[cfg(feature = "kira")]
mod kira_backend;

#[cfg(feature = "kira")]
pub use kira_backend::KiraAudioEngine;

/// Null backend - drops every operation silently. Used when the `kira`
/// feature is disabled or when the host has no audio device.
pub struct NullAudioEngine;

impl Default for NullAudioEngine {
    fn default() -> Self { Self }
}

impl AudioBackend for NullAudioEngine {
    fn is_available(&self) -> bool { false }
    fn load(&self, _: &str, _: &Path) -> Result<()> { Ok(()) }
    fn play(&self, _: &str, _: f64) -> Result<()> { Ok(()) }
    fn play_music(&self, _: &Path, _: f64, _: bool) -> Result<()> { Ok(()) }
    fn set_master_volume(&self, _: f64) -> Result<()> { Ok(()) }
    fn stop_all(&self) -> Result<()> { Ok(()) }
}

/// Public engine facade - holds whichever backend is active behind an
/// `Arc<dyn AudioBackend>`. Construct via [`AudioEngine::default`] which
/// prefers the real backend and falls back to null.
pub struct AudioEngine {
    backend: Arc<dyn AudioBackend>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        let backend: Arc<dyn AudioBackend> = cfg::build_backend();
        Self { backend }
    }

    pub fn is_available(&self) -> bool { self.backend.is_available() }
    pub fn load(&self, name: &str, path: &Path) -> Result<()> {
        self.backend.load(name, path)
    }
    pub fn play(&self, name: &str, volume: f64) -> Result<()> {
        self.backend.play(name, volume)
    }
    pub fn play_music(&self, path: &Path, volume: f64, loops: bool) -> Result<()> {
        self.backend.play_music(path, volume, loops)
    }
    pub fn set_master_volume(&self, volume: f64) -> Result<()> {
        self.backend.set_master_volume(volume)
    }
    pub fn stop_all(&self) -> Result<()> {
        self.backend.stop_all()
    }
}

/// Compile-time backend selection.
mod cfg {
    use super::*;
    pub fn build_backend() -> Arc<dyn AudioBackend> {
        #[cfg(feature = "kira")]
        {
            match KiraAudioEngine::new() {
                Ok(b) => return Arc::new(b),
                Err(e) => log::warn!("kira audio init failed, falling back to null: {e}"),
            }
        }
        Arc::new(NullAudioEngine)
    }
}
