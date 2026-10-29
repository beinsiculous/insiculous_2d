//! `ReloadSample`: a recorded sound edited on disk reaches every holder of its handle — the
//! sound tier re-reads it behind the handle it already has, or retries a path that had
//! failed. Editing, Playing and Paused alike; a sound is not in the play session's snapshot,
//! so Stop has nothing to put back.

use audio::AudioManager;
use editor::console::Severity;
use engine_core::assets::{AssetManager, SoundReload};
use engine_core::Game;

use super::super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// Reload the sound at `sound`, as the scene or the sound tier named it, and say what
    /// came of it: a sound that does not decode keeps the old one and is an error.
    pub(in crate::editor_game) fn reload_sample(&mut self, assets: &mut AssetManager, audio: &mut AudioManager, sound: &str) {
        let line = match assets.reload_sound(sound, audio) {
            Ok(SoundReload::Reloaded { format_change: None }) => format!("{sound} reloaded"),
            Ok(SoundReload::Reloaded { format_change: Some(change) }) => {
                format!("{sound} reloaded; its format changed ({change}), so its loops restarted on it")
            }
            Ok(SoundReload::NowLoads) => format!("{sound} loads now"),
            Ok(SoundReload::NotLoaded) => format!("{sound} is not loaded yet; the next play reads it"),
            Err(error) => {
                return self.report(Severity::Error, format!("{sound} did not reload, the sound before it is kept — {error}"), None);
            }
        };
        self.report(Severity::Info, line, None);
    }
}
