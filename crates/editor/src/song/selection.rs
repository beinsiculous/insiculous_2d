//! The notes selected in a song. Like the scene's selection it is never on the undo stack;
//! unlike it, it addresses notes by their stable `NoteId`, so an edit never moves it onto
//! another note, and a note an undo or a delete removed is dropped from it.

use std::collections::BTreeSet;

use music_midi::{NoteId, SongData};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SongSelection {
    notes: BTreeSet<NoteId>,
}

impl SongSelection {
    pub fn new() -> Self {
        Self::default()
    }

    /// The selected notes, in id order.
    pub fn notes(&self) -> impl Iterator<Item = NoteId> + '_ {
        self.notes.iter().copied()
    }

    pub fn contains(&self, note: NoteId) -> bool {
        self.notes.contains(&note)
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    /// Select exactly these notes.
    pub fn replace(&mut self, notes: impl IntoIterator<Item = NoteId>) {
        self.notes = notes.into_iter().collect();
    }

    pub fn clear(&mut self) {
        self.notes.clear();
    }

    /// Drop every selected note `data` no longer holds.
    pub(super) fn retain_existing(&mut self, data: &SongData) {
        self.notes.retain(|note| data.note_image(*note).is_some());
    }
}
