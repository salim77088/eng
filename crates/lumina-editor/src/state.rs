//! Editor state - everything the editor needs to remember between frames:
//! selected entity, open panels, console log, asset list, simulator mode.

use egui::TextureHandle;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    Edit,
    Play,
    Pause,
}

impl Default for EditorMode {
    fn default() -> Self {
        EditorMode::Edit
    }
}

/// Mutable, frame-to-frame editor state.
pub struct EditorState {
    pub mode: EditorMode,
    pub selected_entity: Option<u64>, // raw entity id (hecs::Entity isn't stable across frames)
    pub console_log: Vec<String>,
    pub assets_root: Option<PathBuf>,
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub show_asset_browser: bool,
    pub show_console: bool,
    pub show_about: bool,
    pub stats: Stats,
    /// Cached logo texture handle (so we don't re-upload every frame).
    pub logo: Option<TextureHandle>,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub fps: f32,
    pub entity_count: usize,
    pub draw_calls: u32,
    pub particles: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            mode: EditorMode::Edit,
            selected_entity: None,
            console_log: vec![
                "Lumina Engine editor ready.".into(),
                "Tip: press Ctrl+P to enter Play mode.".into(),
            ],
            assets_root: None,
            show_hierarchy: true,
            show_inspector: true,
            show_asset_browser: true,
            show_console: true,
            show_about: false,
            stats: Stats::default(),
            logo: None,
        }
    }
}

impl EditorState {
    pub fn log<S: Into<String>>(&mut self, msg: S) {
        self.console_log.push(msg.into());
        if self.console_log.len() > 1000 {
            self.console_log.drain(0..500);
        }
    }
}
