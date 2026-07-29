//! Real audio backend via `kira`.

use crate::AudioBackend;
use anyhow::{Context, Result};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundSettings};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub struct KiraAudioEngine {
    manager: Arc<RwLock<Option<AudioManager<DefaultBackend>>>>,
    sounds: RwLock<HashMap<String, StaticSoundData>>,
}

impl KiraAudioEngine {
    pub fn new() -> Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| anyhow::anyhow!("kira AudioManager init: {e}"))?;
        Ok(Self {
            manager: Arc::new(RwLock::new(Some(manager))),
            sounds: RwLock::new(HashMap::new()),
        })
    }

    /// Convert a 0.0..=1.0 linear volume to decibels. 0.0 maps to
    /// silence (-60 dB), 1.0 maps to unity (0 dB).
    fn vol_to_db(volume: f64) -> Decibels {
        if volume <= 0.0 {
            Decibels::SILENCE
        } else {
            Decibels(20.0 * (volume as f32).log10())
        }
    }
}

impl AudioBackend for KiraAudioEngine {
    fn is_available(&self) -> bool {
        self.manager.read().is_some()
    }

    fn load(&self, name: &str, path: &Path) -> Result<()> {
        let data =
            StaticSoundData::from_file(path).with_context(|| format!("load sfx {:?}", path))?;
        self.sounds.write().insert(name.to_string(), data);
        Ok(())
    }

    fn play(&self, name: &str, volume: f64) -> Result<()> {
        let mut mgr_guard = self.manager.write();
        let mgr = mgr_guard.as_mut().context("audio manager unavailable")?;
        let data = {
            let sounds = self.sounds.read();
            sounds
                .get(name)
                .with_context(|| format!("sound '{name}' not loaded"))?
                .clone()
        };
        let settings = StaticSoundSettings::new().volume(Self::vol_to_db(volume));
        mgr.play(data.with_settings(settings))
            .map_err(|e| anyhow::anyhow!("kira play: {e}"))?;
        Ok(())
    }

    fn play_music(&self, path: &Path, volume: f64, loops: bool) -> Result<()> {
        let mut mgr_guard = self.manager.write();
        let mgr = mgr_guard.as_mut().context("audio manager unavailable")?;
        let mut settings = StreamingSoundSettings::new().volume(Self::vol_to_db(volume));
        if loops {
            settings = settings.loop_region(..);
        }
        let data = StreamingSoundData::from_file(path)
            .with_context(|| format!("stream music {:?}", path))?
            .with_settings(settings);
        mgr.play(data)
            .map_err(|e| anyhow::anyhow!("kira play_music: {e}"))?;
        Ok(())
    }

    fn set_master_volume(&self, volume: f64) -> Result<()> {
        let mut mgr_guard = self.manager.write();
        let mgr = mgr_guard.as_mut().context("audio manager unavailable")?;
        mgr.main_track()
            .set_volume(Self::vol_to_db(volume), Tween::default());
        Ok(())
    }

    fn stop_all(&self) -> Result<()> {
        let mut guard = self.manager.write();
        // Dropping the manager stops all sounds immediately.
        *guard = None;
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(m) => *guard = Some(m),
            Err(e) => log::warn!("failed to re-init audio manager: {e}"),
        }
        Ok(())
    }
}
