//! The song's undo history, with a save watermark of its own — never the scene's.
//!
//! It copies the scene history's dirty contract and nothing more: ids count from 1 (0 is
//! the empty stack), the history is dirty when the id on top is not the id saved, a merge
//! into the entry on top reassigns its id and clears the redo stack, and
//! [`SongHistory::break_merge`] seals a gesture. The scene history's play-session floor has
//! no song analogue: song writes are refused for the whole play session.

use std::collections::VecDeque;

use super::change::Change;
use super::edit::SongEdit;

/// Undo entries kept; the oldest falls off past it.
const MAXIMUM_ENTRIES: usize = 100;

/// Whether an edit starts an undo entry of its own or continues the gesture on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// An entry of its own, and the end of any gesture: a click, a typed value, an API line.
    Discrete,
    /// One frame of a gesture (a drag): merged into the entry on top while the two edits
    /// merge and no [`SongHistory::break_merge`] sealed it.
    Continuing,
}

/// One recorded edit: what it was, and the exact changes that undo and redo it.
#[derive(Debug, Clone)]
pub(super) struct SongHistoryEntry {
    id: u64,
    pub edit: SongEdit,
    pub undo: Change,
    pub redo: Change,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub struct SongHistory {
    undo_stack: VecDeque<SongHistoryEntry>,
    redo_stack: Vec<SongHistoryEntry>,
    next_id: u64,
    saved_id: u64,
    merge_sealed: bool,
}

impl Default for SongHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl SongHistory {
    /// An empty history, clean at id 0.
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            next_id: 1,
            saved_id: 0,
            merge_sealed: false,
        }
    }

    /// Whether the song changed since the last [`SongHistory::mark_saved`] — or since
    /// the history was made. A song never saved at all is dirty for that reason too, which
    /// the document adds; the watermark alone calls a fresh history clean.
    pub fn is_dirty(&self) -> bool {
        self.top_id() != self.saved_id
    }

    /// The position on top becomes the clean baseline.
    pub fn mark_saved(&mut self) {
        self.saved_id = self.top_id();
    }

    /// End the gesture on top: the next continuing edit starts an entry of its own.
    pub fn break_merge(&mut self) {
        self.merge_sealed = true;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Entries on the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Which state of the song the history stands at: it changes with every edit, merge,
    /// undo and redo, and returns to a value only with the song it named — what a host that
    /// prepared the song from the document compares to know the document moved under it.
    pub fn revision(&self) -> u64 {
        self.top_id()
    }

    fn top_id(&self) -> u64 {
        self.undo_stack.back().map(|entry| entry.id).unwrap_or(0)
    }

    fn fresh_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Record an edit already applied. A continuing edit that merges with the entry on
    /// top replaces its edit and its redo, keeps its undo, and takes a new id; anything
    /// else is a new entry. Either way the redo stack is cleared.
    pub(super) fn record(&mut self, edit: SongEdit, undo: Change, redo: Change, gesture: Gesture) {
        let id = self.fresh_id();
        self.redo_stack.clear();
        if gesture == Gesture::Continuing && !self.merge_sealed {
            if let Some(top) = self.undo_stack.back_mut() {
                if let Some(merged) = top.edit.try_merge(&edit) {
                    top.edit = merged;
                    top.redo = redo;
                    top.id = id;
                    return;
                }
            }
        }
        self.undo_stack.push_back(SongHistoryEntry {
            id,
            edit,
            undo,
            redo,
        });
        if self.undo_stack.len() > MAXIMUM_ENTRIES {
            self.undo_stack.pop_front();
        }
        self.merge_sealed = gesture == Gesture::Discrete;
    }

    /// Take the entry on top to undo it; give it to [`SongHistory::push_undone`] once
    /// undone, or back to [`SongHistory::restore_undo`] if undoing failed.
    pub(super) fn pop_undo(&mut self) -> Option<SongHistoryEntry> {
        self.undo_stack.pop_back()
    }

    pub(super) fn push_undone(&mut self, entry: SongHistoryEntry) {
        self.redo_stack.push(entry);
    }

    pub(super) fn restore_undo(&mut self, entry: SongHistoryEntry) {
        self.undo_stack.push_back(entry);
    }

    /// Take the last undone entry to redo it; it goes back on top, its id with it, through
    /// [`SongHistory::push_redone`] — or [`SongHistory::restore_redo`] if redoing failed.
    pub(super) fn pop_redo(&mut self) -> Option<SongHistoryEntry> {
        self.redo_stack.pop()
    }

    pub(super) fn push_redone(&mut self, entry: SongHistoryEntry) {
        self.undo_stack.push_back(entry);
    }

    pub(super) fn restore_redo(&mut self, entry: SongHistoryEntry) {
        self.redo_stack.push(entry);
    }
}
