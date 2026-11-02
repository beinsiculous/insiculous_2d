//! The preview stopped by a change the editor did not make: another program's score adopted
//! or map reloaded under it. The moved-under check in the drain compares the history's
//! revision, which an adoption of a clean document leaves where it was, so the stop is
//! explicit here.

use editor::console::Severity;
use engine_core::Game;

use super::super::EditorGame;
use super::{SlotHolder, SongAudioIntent};

impl<G: Game> EditorGame<G> {
    /// A sounding preview plays the score it prepared, which the document just left for
    /// the one on disk: stop it as an edit stops it, and say why.
    pub(in crate::editor_game) fn stop_preview_for_external_change(&mut self) {
        let preview = &mut self.song_preview;
        if !preview.is_playing() || preview.slot != SlotHolder::Preview {
            return;
        }
        preview.intents.insert(0, SongAudioIntent::Stop);
        preview.park();
        self.report(Severity::Info, "Song preview stopped: the song changed on disk".to_string(), None);
    }
}
