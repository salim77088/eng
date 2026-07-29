//! Real audio backend via `kira`.

use crate::AudioBackend;
use anyhow::{Context, Result};
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::StaticSoundData;
use kira::sound::streaming::StreamingSoundData;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub struct KiraAudioEngine {
    manager: Arc<RwLock<Option<AudioManager>>>,
    sounds: RwLock<HashMap<String, StaticSoundData>>,
}

impl KiraAudioEngine {
    pub fn new() -> Result<Self> {
        let manager = AudioManager::new(AudioManagerSettings::default())
            .map_err(|e| anyhow::anyhow!("kira AudioManager init: {e}"))?;
        Ok(Self {
            manager: Arc::new(RwLock::new(Some(manager))),
            sounds: RwLock::new(HashMap::new()),
        })
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
        let mgr_guard = self.manager.read();
        let mgr = mgr_guard.as_ref().context("audio manager unavailable")?;
        let data = {
            let sounds = self.sounds.read();
            sounds
                .get(name)
                .with_context(|| format!("sound '{name}' not loaded"))?
                .clone()
        };
        mgr.play(data.with_volume(volume as f32 * 0.5))
            .map_err(|e| anyhow::anyhow!("kira play: {e}"))?;
        Ok(())
    }

    fn play_music(&self, path: &Path, volume: f64, loops: bool) -> Result<()> {
        let mgr_guard = self.manager.read();
        let mgr = mgr_guard.as_ref().context("audio manager unavailable")?;
        let mut data = StreamingSoundData::from_file(path)
            .with_context(|| format!("stream music {:?}", path))?;
        if loops {
            data = data.loop_();
        }
        mgr.play(data.with_volume(volume as f32 * 0.5))
            .map_err(|e| anyhow::anyhow!("kira play_music: {e}"))?;
        Ok(())
    }

    fn set_master_volume(&self, volume: f64) -> Result<()> {
        let mgr_guard = self.manager.read();
        let mgr = mgr_guard.as_ref().context("audio manager unavailable")?;
        mgr.set_main_volume(volume as f32)
            .map_err(|e| anyhow::anyhow!("kira set_main_volume: {e}"))?;
        Ok(())
    }

    fn stop_all(&self) -> Result<()> {
        let mut guard = self.manager.write();
        if let Some(m) = guard.take() {
            let _ = m.stop();
        }
        *guard = AudioManager::new(AudioManagerSettings::default()).ok();
        Ok(())
    }
}
