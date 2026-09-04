use std::path::Path;
use engine_core::prelude::*;
use editor::EditorPreferences;

impl<G: Game> super::EditorGame<G> {
    /// Load persisted editor preferences (camera, grid, panel layout) from `self.prefs_slot`.
    pub(super) fn load_preferences(&mut self) {
        let slot = self.prefs_slot.clone();
        self.load_preferences_from(&slot);
    }

    /// Load preferences from one `save_store` slot. An absent slot is the
    /// first run and stays silent; an unreadable or corrupt one falls back to
    /// defaults with a warning, so a broken file never blocks startup and never
    /// hides either.
    pub(crate) fn load_preferences_from(&mut self, slot: &Path) {
        let prefs = match engine_core::save_store::read(slot) {
            Ok(None) => EditorPreferences::default(),
            Ok(Some(json)) => EditorPreferences::from_json(&json).unwrap_or_else(|error| {
                log::warn!("editor preferences at {} ignored: {error}", slot.display());
                EditorPreferences::default()
            }),
            Err(error) => {
                log::warn!("editor preferences at {} unreadable: {error}", slot.display());
                EditorPreferences::default()
            }
        };
        self.editor.set_camera_offset(Vec2::new(prefs.camera_position.0, prefs.camera_position.1));
        self.editor.set_camera_zoom(prefs.camera_zoom);
        self.editor.set_snap_to_grid(prefs.snap_to_grid);
        self.editor.set_grid_size(prefs.grid_size);
        self.editor.set_grid_visible(prefs.grid_visible);
        prefs.apply_panels(&mut self.editor.dock_area);
        self.last_saved_prefs = Some(prefs);
        self.pending_prefs = None;
        self.prefs_stable_time = 0.0;
    }

    /// Capture current editor state into an [`EditorPreferences`] struct.
    pub(super) fn capture_preferences(&self) -> EditorPreferences {
        let mut prefs = EditorPreferences {
            camera_position: (self.editor.camera_offset().x, self.editor.camera_offset().y),
            camera_zoom: self.editor.camera_zoom(),
            last_scene_path: self
                .editor
                .scene_path()
                .and_then(|path| path.to_str())
                .map(|string| string.to_string()),
            snap_to_grid: self.editor.is_snap_to_grid(),
            grid_size: self.editor.grid_size(),
            grid_visible: self.editor.is_grid_visible(),
            panels: Vec::new(),
        };
        prefs.capture_panels(&self.editor.dock_area);
        prefs
    }

    /// Write an [`EditorPreferences`] struct to `self.prefs_slot`; `true` when the
    /// slot holds it afterwards.
    fn write_preferences(&self, prefs: &EditorPreferences) -> bool {
        match prefs.to_json() {
            Ok(json) => match engine_core::save_store::write(&self.prefs_slot, &json) {
                Ok(()) => true,
                Err(error) => {
                    log::warn!("Failed to save editor preferences to {}: {}", self.prefs_slot.display(), error);
                    false
                }
            },
            Err(error) => {
                log::warn!("Failed to serialize editor preferences: {}", error);
                false
            }
        }
    }

    fn record_written(&mut self, written: EditorPreferences) {
        self.last_saved_prefs = Some(written);
        self.pending_prefs = None;
        self.prefs_stable_time = 0.0;
    }

    /// A failed write stays pending and retries after another half second, so a
    /// full or denied localStorage never silences the preferences for the session.
    fn commit_settled(&mut self, current: EditorPreferences) {
        if self.write_preferences(&current) {
            self.record_written(current);
        } else {
            self.pending_prefs = Some(current);
            self.prefs_stable_time = 0.0;
        }
    }

    /// Save preferences immediately if they differ from the last written state.
    pub(super) fn save_preferences_now(&mut self) {
        let current = self.capture_preferences();
        if self.last_saved_prefs.as_ref() != Some(&current) && self.write_preferences(&current) {
            self.record_written(current);
        }
    }

    /// Settle rule for preferences: check every frame from `finish_frame`.
    ///
    /// Accumulates `delta_time` while preferences remain unchanged and writes
    /// once they have been stable for 0.5 seconds. Skipped during play sessions
    /// so the game camera never overwrites editing pan/zoom.
    pub(super) fn save_preferences_if_changed(&mut self, delta_time: f32) {
        if self.editor.play_state().in_play_session() {
            return;
        }
        let current = self.capture_preferences();
        if self.last_saved_prefs.as_ref() == Some(&current) {
            self.pending_prefs = None;
            self.prefs_stable_time = 0.0;
            return;
        }
        if self.pending_prefs.as_ref() == Some(&current) {
            self.prefs_stable_time += delta_time;
            if self.prefs_stable_time >= 0.5 {
                self.commit_settled(current);
            }
        } else {
            // A change is only committed once it has been seen unchanged on a later frame:
            // one hitched half-second frame must not capture a gesture mid-drag.
            self.pending_prefs = Some(current);
            self.prefs_stable_time = delta_time;
        }
    }
}
