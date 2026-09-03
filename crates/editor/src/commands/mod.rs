//! Undo/redo command system for the editor.
//!
//! Implements the Command pattern: each user action is represented as an
//! `EditorCommand` that can be executed, undone, and redone. The `CommandHistory`
//! manages undo/redo stacks with optional command merging for continuous edits
//! (e.g., dragging a gizmo or scrubbing a slider).

use std::any::Any;
use std::collections::VecDeque;

use ecs::{EntityId, World};

use crate::selection::Selection;

mod component_commands;
mod entity_commands;
mod set_commands;

pub use component_commands::{
    AddComponentCommand, AddDynamicComponentCommand, RemoveComponentCommand,
    RemoveDynamicComponentCommand, SetComponentValueCommand,
};
pub use entity_commands::{CreateEntityCommand, DeleteEntityCommand, MacroCommand};
pub use set_commands::{
    NudgeCommand, RenameEntityCommand, SetAudioSourceCommand, SetBehaviorCommand, SetColliderCommand,
    SetEntityTagCommand, SetGridBackdropCommand, SetNameCommand, SetRigidBodyCommand,
    SetScriptsCommand, SetSpriteCommand, SetTransformCommand, SetUiButtonCommand, SetUiLabelCommand,
    SetUiPanelCommand, TransformGizmoCommand,
};

// The registry-generated ComponentKind is re-exported here so existing
// `editor::commands::ComponentKind` paths keep working.
pub use crate::stored_component::ComponentKind;

// ---------------------------------------------------------------------------
// EditorCommand trait
// ---------------------------------------------------------------------------

/// A reversible editor action.
pub trait EditorCommand: Send {
    /// Apply the action to the world.
    fn execute(&mut self, world: &mut World);

    /// Reverse the action.
    fn undo(&mut self, world: &mut World);

    /// Human-readable name shown in Edit menu (e.g., "Move Entity").
    fn display_name(&self) -> &str;

    /// Attempt to merge `other` into `self`. Returns `true` if merged.
    ///
    /// When merged, `self` is updated in-place and `other` is discarded.
    /// Default implementation returns `false` (no merging).
    fn try_merge(&mut self, _other: &dyn EditorCommand) -> bool {
        false
    }

    /// Downcast to `&dyn Any` for type-based merging.
    fn as_any(&self) -> &dyn Any;

    /// Downcast to `&mut dyn Any` for type-based merging.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ---------------------------------------------------------------------------
// CommandHistory
// ---------------------------------------------------------------------------

/// Manages undo/redo stacks for editor commands.
///
/// Also the **source of truth for whether the scene is dirty** (issue #24):
/// every recorded mutation carries a unique id, and [`is_dirty`] compares
/// the id on top of the undo stack against the id captured at the last
/// [`mark_saved`]. Undoing back to the saved command reads clean again;
/// merging into a post-save command reassigns its id, so undo past a
/// merged edit correctly stays dirty.
///
/// [`is_dirty`]: CommandHistory::is_dirty
/// [`mark_saved`]: CommandHistory::mark_saved
/// One recorded mutation plus the selection context needed to make
/// undo/redo restore what the user had selected (#59).
struct HistoryEntry {
    id: u64,
    cmd: Box<dyn EditorCommand>,
    /// The selection as noted BEFORE the command's gesture began (merges
    /// keep the FIRST before-image). Undo restores this.
    selection_before: Vec<EntityId>,
    /// The selection captured at undo time (honors whatever the user
    /// selected since the command ran). Redo restores this — AFTER
    /// re-executing, so a redone create exists again before pruning.
    selection_after: Vec<EntityId>,
}

pub struct CommandHistory {
    undo_stack: VecDeque<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    max_history: usize,
    /// Next command id; starts at 1 (0 is the empty-stack sentinel).
    next_id: u64,
    /// Id on top of the undo stack when the scene was last saved
    /// (0 = saved at empty history, the initial state).
    saved_id: u64,
    /// The next mergeable command must start a fresh entry (gesture
    /// boundary) — see [`Self::break_merge`].
    merge_sealed: bool,
    /// The selection as of the host's last [`note_selection`] — stamped
    /// onto every NEW entry as its before-image (#59).
    pending_selection: Vec<EntityId>,
    /// Selection to restore after the last undo/redo — collected by the
    /// host via [`take_selection_restore`].
    ///
    /// [`note_selection`]: Self::note_selection
    /// [`take_selection_restore`]: Self::take_selection_restore
    selection_restore: Option<Vec<EntityId>>,
}

impl CommandHistory {
    /// Create a new command history with default max history (100).
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_history: 100,
            next_id: 1,
            saved_id: 0,
            merge_sealed: false,
            pending_selection: Vec::new(),
            selection_restore: None,
        }
    }

    /// Note the CURRENT selection as the before-image for any commands
    /// recorded from now on (#59). Hosts call this before their handlers
    /// mutate selection: once per frame at the top of the editor update,
    /// and per line on the command-API write path.
    pub fn note_selection(&mut self, selection: &Selection) {
        self.pending_selection = selection.selected().collect();
    }

    /// The selection undo/redo wants restored, pruned to live entities —
    /// `None` when the last undo/redo did nothing (or was already taken).
    /// Hosts apply it right after a successful [`undo`]/[`redo`].
    ///
    /// [`undo`]: Self::undo
    /// [`redo`]: Self::redo
    pub fn take_selection_restore(&mut self) -> Option<Vec<EntityId>> {
        self.selection_restore.take()
    }

    fn fresh_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn top_id(&self) -> u64 {
        self.undo_stack.back().map(|entry| entry.id).unwrap_or(0)
    }

    fn push_entry(&mut self, cmd: Box<dyn EditorCommand>) {
        let id = self.fresh_id();
        let selection_before = self.pending_selection.clone();
        self.undo_stack.push_back(HistoryEntry {
            id,
            cmd,
            selection_before,
            selection_after: Vec::new(),
        });
        self.redo_stack.clear();
        self.enforce_limit();
    }

    /// Whether the world has changed since the last [`mark_saved`]
    /// (or since creation, for a never-saved history).
    ///
    /// [`mark_saved`]: CommandHistory::mark_saved
    pub fn is_dirty(&self) -> bool {
        self.top_id() != self.saved_id
    }

    /// Record that the world was just saved: the current history position
    /// becomes the clean baseline.
    pub fn mark_saved(&mut self) {
        self.saved_id = self.top_id();
    }

    /// Execute a command and push it onto the undo stack. Clears the redo stack.
    pub fn execute(&mut self, mut cmd: Box<dyn EditorCommand>, world: &mut World) {
        cmd.execute(world);
        self.push_entry(cmd);
    }

    /// Undo the most recent command. Returns `true` if a command was applied.
    ///
    /// Ordering is load-bearing (#59): the CURRENT selection (the host's
    /// last [`note_selection`]) is stamped as the entry's after-image, THEN
    /// the command undoes, THEN the before-image (pruned to entities alive
    /// in the restored world — undo is id-exact per GPP-14, pruning only
    /// defends cross-entry staleness) becomes the restore target.
    ///
    /// [`note_selection`]: Self::note_selection
    pub fn undo(&mut self, world: &mut World) -> bool {
        if let Some(mut entry) = self.undo_stack.pop_back() {
            entry.selection_after = self.pending_selection.clone();
            entry.cmd.undo(world);
            let restore = prune_to_live(&entry.selection_before, world);
            self.selection_restore = Some(restore.clone());
            self.pending_selection = restore;
            self.redo_stack.push(entry);
            true
        } else {
            self.selection_restore = None;
            false
        }
    }

    /// Redo the most recently undone command. Returns `true` if a command was applied.
    ///
    /// Re-executes FIRST, then restores the after-image — so a redone
    /// create/duplicate/paste exists again before pruning and stays
    /// selected; "redo of delete re-clears" falls out naturally.
    pub fn redo(&mut self, world: &mut World) -> bool {
        if let Some(mut entry) = self.redo_stack.pop() {
            entry.cmd.execute(world);
            let restore = prune_to_live(&entry.selection_after, world);
            self.selection_restore = Some(restore.clone());
            self.pending_selection = restore;
            self.undo_stack.push_back(entry);
            true
        } else {
            self.selection_restore = None;
            false
        }
    }

    /// Whether there is a command to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether there is a command to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Display name of the command that would be undone, if any.
    pub fn undo_name(&self) -> Option<&str> {
        self.undo_stack.back().map(|entry| entry.cmd.display_name())
    }

    /// Display name of the command that would be redone, if any.
    pub fn redo_name(&self) -> Option<&str> {
        self.redo_stack.last().map(|entry| entry.cmd.display_name())
    }

    /// Clear both undo and redo stacks **and reset the saved watermark**:
    /// a cleared history reads clean. Only call where the fresh world IS
    /// the on-disk state (scene load, new scene) — clearing after edits
    /// would silently discard the dirty flag along with the undo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.saved_id = 0;
        // A cleared history must be FULLY clean — stale selection state
        // would stamp wrong before-images onto the next commands (#59
        // kimi F2).
        self.pending_selection.clear();
        self.selection_restore = None;
    }

    /// Push a pre-executed command onto the undo stack without calling execute().
    /// Use when the action was already performed and you just need to record it for undo.
    pub fn push_already_executed(&mut self, cmd: Box<dyn EditorCommand>) {
        self.push_entry(cmd);
    }

    /// [`push_already_executed`] with an EXPLICIT before-image, bypassing
    /// the noted pending selection — for commands whose gesture began long
    /// before the push (a cross-frame API batch: the macro must carry the
    /// selection from `batch begin`, not from the last frame's note — #59
    /// kimi F1).
    ///
    /// [`push_already_executed`]: Self::push_already_executed
    pub fn push_already_executed_with_before(
        &mut self,
        cmd: Box<dyn EditorCommand>,
        selection_before: Vec<EntityId>,
    ) {
        let saved = std::mem::replace(&mut self.pending_selection, selection_before);
        self.push_entry(cmd);
        self.pending_selection = saved;
    }

    /// Try to merge `cmd` with the last undo command. If merging fails, execute normally.
    ///
    /// Used for continuous edits like gizmo drags or slider scrubs to avoid
    /// flooding the undo history with one entry per frame.
    pub fn try_merge_or_execute(&mut self, cmd: Box<dyn EditorCommand>, world: &mut World) {
        if std::mem::take(&mut self.merge_sealed) {
            self.execute(cmd, world);
            return;
        }
        if let Some(entry) = self.undo_stack.back_mut() {
            if entry.cmd.try_merge(cmd.as_ref()) {
                // Merged into existing command — no new push, but the
                // command's resulting state changed, so it gets a fresh id
                // (a post-save merge must read dirty even after undo) and,
                // like any other new mutation, invalidates redo history
                // (redoing an old command on top of the merged state could
                // land on the saved id with a different world). The entry's
                // selection_before deliberately stays untouched — a merged
                // gesture keeps its FIRST before-image (#59).
                entry.id = self.next_id;
                self.next_id += 1;
                self.redo_stack.clear();
                return;
            }
        }
        self.execute(cmd, world);
    }

    /// Try to merge `cmd` with the last undo command, or push without executing if merge fails.
    ///
    /// Use when the change was already applied to the world manually (e.g., inspector
    /// writeback for immediate visual feedback). The command is recorded for undo/redo
    /// but `execute()` is not called.
    pub fn try_merge_or_push(&mut self, cmd: Box<dyn EditorCommand>) {
        if std::mem::take(&mut self.merge_sealed) {
            self.push_already_executed(cmd);
            return;
        }
        if let Some(entry) = self.undo_stack.back_mut() {
            if entry.cmd.try_merge(cmd.as_ref()) {
                // See try_merge_or_execute: merged state = new id + no redo;
                // selection_before keeps the FIRST before-image (#59).
                entry.id = self.next_id;
                self.next_id += 1;
                self.redo_stack.clear();
                return;
            }
        }
        self.push_already_executed(cmd);
    }

    /// Seal the top of the undo stack against further merging: the NEXT
    /// mergeable command starts a fresh entry. Hosts call this at edit
    /// gesture boundaries (scrub release, typed commit) so two separate
    /// gestures on the same field become two undo entries — without it,
    /// field_hint merging is unbounded in time.
    pub fn break_merge(&mut self) {
        self.merge_sealed = true;
    }

    fn enforce_limit(&mut self) {
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
    }
}

/// The subset of `ids` still alive in `world`, original order kept.
fn prune_to_live(ids: &[EntityId], world: &World) -> Vec<EntityId> {
    let live = world.entities();
    ids.iter().copied().filter(|id| live.contains(id)).collect()
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests;
#[cfg(test)]
mod dirty_tests;
#[cfg(test)]
mod selection_restore_tests;
