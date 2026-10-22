//! A drag of notes in the composer's piano roll, owned by `EditorGame` like a gizmo drag:
//! the press captures the dragged notes' images, each frame holds the *total* delta from
//! the press — snapped to a sixteenth, clamped so no note leaves the song — and the roll
//! draws the notes there from the preview while the document is untouched; the release
//! applies one `SongEdit::Batch` as a discrete entry, or nothing for a drag that ended where
//! it began. Escape, and everything that takes the composer's focus away, drops the
//! preview and applies nothing, so it leaves no undo entry, no redo entry and nothing
//! older touched. While one is live no history operation runs and no API line is answered.

use editor::composer::{clamp_move, moved_notes_edit, note_images, MovePreview, NoteId, NoteImage, RollPoint};
use editor::{Gesture, SongEdit};
use engine_core::Game;

use super::EditorGame;

/// See the module docs.
#[derive(Debug, Clone)]
pub(super) struct ComposerDrag {
    /// The dragged notes as they were at the press.
    images: Vec<NoteImage>,
    press: RollPoint,
    /// The time step a drag moves by: a sixteenth of the song's quarter.
    step_ticks: u64,
    preview: MovePreview,
}

impl ComposerDrag {
    pub(super) fn new(images: Vec<NoteImage>, press: RollPoint, step_ticks: u64) -> Self {
        let preview = MovePreview {
            notes: images.iter().map(|image| image.id).collect(),
            tick_delta: 0,
            key_delta: 0,
        };
        Self {
            images,
            press,
            step_ticks: step_ticks.max(1),
            preview,
        }
    }

    pub(super) fn preview(&self) -> &MovePreview {
        &self.preview
    }

    /// Follow the pointer: the delta from the press, the time snapped to the step, both
    /// clamped. Idempotent — a frame never adds to the last one's delta.
    pub(super) fn follow(&mut self, pointer: RollPoint) {
        let step = i64::try_from(self.step_ticks).unwrap_or(i64::MAX);
        let raw_ticks = pointer.tick.saturating_sub(self.press.tick);
        let snapped_ticks = (raw_ticks as f64 / step as f64).round() as i64 * step;
        let raw_keys = pointer.key.saturating_sub(self.press.key);
        let (tick_delta, key_delta) = clamp_move(&self.images, snapped_ticks, raw_keys);
        self.preview.tick_delta = tick_delta;
        self.preview.key_delta = key_delta;
    }

    /// The one edit the release commits, if the notes moved at all.
    pub(super) fn commit_edit(&self) -> Option<SongEdit> {
        moved_notes_edit(&self.images, self.preview.tick_delta, self.preview.key_delta)
    }
}

impl<G: Game> EditorGame<G> {
    /// Begin dragging `notes` from a press at `point`. Refused, with no drag begun, while
    /// a play session runs.
    pub(super) fn begin_composer_drag(&mut self, notes: Vec<NoteId>, point: RollPoint) {
        if self.composer_write_refusal().is_some() {
            return;
        }
        let Some(song) = self.song.as_ref() else {
            return;
        };
        let images = note_images(song, notes);
        if images.is_empty() {
            return;
        }
        let step = editor::composer::piano_roll::sixteenth_ticks(song);
        self.composer_drag = Some(ComposerDrag::new(images, point, step));
    }

    /// One frame of a live drag: follow the pointer while the button is held, commit on
    /// the release.
    pub(super) fn update_composer_drag(&mut self, pointer: Option<RollPoint>, mouse_down: bool) {
        let Some(drag) = self.composer_drag.as_mut() else {
            return;
        };
        if let Some(pointer) = pointer {
            drag.follow(pointer);
        }
        if !mouse_down {
            self.finish_composer_drag();
        }
    }

    /// The release: one discrete entry, or nothing for an unmoved drag. An edit the
    /// document still refuses ends the drag with the reason on the status bar.
    pub(super) fn finish_composer_drag(&mut self) {
        let Some(drag) = self.composer_drag.take() else {
            return;
        };
        if let Some(edit) = drag.commit_edit() {
            self.apply_composer_edit(edit, Gesture::Discrete);
        }
    }

    /// Drop a live drag without touching the document: Escape, and every way the
    /// composer loses its focus. Returns whether one was live.
    pub(super) fn drop_composer_drag(&mut self) -> bool {
        self.composer_drag.take().is_some()
    }
}
