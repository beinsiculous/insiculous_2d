//! Editor play state for controlling game simulation.
//!
//! Defines the three-state lifecycle for running a game inside the editor:
//! `Editing` (default), `Playing` (game logic runs), and `Paused` (frozen).

/// The current play state of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditorPlayState {
    /// Normal editing mode — game logic does not run.
    #[default]
    Editing,
    /// Game is running inside the editor.
    Playing,
    /// Game is paused — simulation frozen, state preserved.
    Paused,
}

impl EditorPlayState {
    /// Whether the editor is in normal editing mode.
    pub fn is_editing(self) -> bool {
        self == EditorPlayState::Editing
    }

    /// Whether the game simulation is actively running.
    pub fn is_playing(self) -> bool {
        self == EditorPlayState::Playing
    }

    /// Whether the game is paused.
    pub fn is_paused(self) -> bool {
        self == EditorPlayState::Paused
    }

    /// Whether a play session is active (Playing or Paused).
    ///
    /// Returns `true` when a snapshot exists and the world has been modified
    /// by game logic (even if currently paused).
    pub fn in_play_session(self) -> bool {
        matches!(self, EditorPlayState::Playing | EditorPlayState::Paused)
    }

    /// Human-readable label for the current state.
    ///
    /// Viewport border tinting lives on the theme: `EditorTheme::play_state_border`.
    pub fn label(self) -> &'static str {
        match self {
            EditorPlayState::Editing => "Editing",
            EditorPlayState::Playing => "Playing",
            EditorPlayState::Paused => "Paused",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A paused game is still a play session: the snapshot is live, so
    /// Save/New/Open stay refused and Stop still restores the authored world.
    #[test]
    fn test_paused_is_still_a_play_session() {
        assert!(!EditorPlayState::Editing.in_play_session());
        assert!(EditorPlayState::Playing.in_play_session());
        assert!(EditorPlayState::Paused.in_play_session());
    }
}
