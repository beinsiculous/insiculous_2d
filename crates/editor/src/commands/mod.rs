//! Undo/redo command system managing action stacks and command merging.

use std::any::Any;
use std::collections::VecDeque;

use ecs::{EntityId, World};

use crate::selection::Selection;

mod component_commands;
mod entity_commands;
mod rebase;
mod set_commands;

pub use component_commands::{
    AddComponentCommand, RemoveComponentCommand, SetComponentValueCommand,
};
pub use entity_commands::{CreateEntityCommand, DeleteEntityCommand, MacroCommand};
pub use rebase::Rebase;
pub(crate) use rebase::{entity_is_alive, patch_changed_leaves, script_references_resolve};
pub use set_commands::{
    NudgeCommand, RenameEntityCommand, SetAudioSourceCommand, SetBehaviorCommand, SetColliderCommand,
    SetComponentCommand, SetEntityTagCommand, SetGridBackdropCommand, SetNameCommand,
    SetRigidBodyCommand, SetScriptsCommand, SetSpriteCommand, SetTransformCommand,
    SetUiButtonCommand, SetUiLabelCommand, SetUiPanelCommand, GIZMO_FIELD_HINT,
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

    /// Adjust this command to re-apply onto a world that was just
    /// restored from the Play snapshot, and say whether it still applies.
    ///
    /// Called once per retained entry immediately before its re-execute.
    /// The default keeps the command untouched — only commands carrying a
    /// simulated before-image or a structural precondition need to act.
    ///
    /// The world is `&mut` for one reason: a macro must rebase each child
    /// against the world the child before it produced, which means running
    /// them. A macro that does so leaves the world already updated and its
    /// own `execute` skips the duplicate pass; nothing else mutates here.
    fn rebase_onto(&mut self, _world: &mut World) -> Rebase {
        Rebase::Apply
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
/// Also the **source of truth for whether the scene is dirty**:
/// every recorded mutation carries a unique id, and [`is_dirty`] compares
/// the id on top of the undo stack against the id captured at the last
/// [`mark_saved`]. Undoing back to the saved command reads clean again;
/// merging into a post-save command reassigns its id, so undo past a
/// merged edit correctly stays dirty.
///
/// [`is_dirty`]: CommandHistory::is_dirty
/// [`mark_saved`]: CommandHistory::mark_saved
/// One recorded mutation plus the selection context needed to make
/// undo/redo restore what the user had selected.
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

/// The Play boundary: where the undo stack stood when the session began,
/// and the id no session entry can be below.
///
/// Membership is the POSITION — `try_merge_or_push` reassigns a merged
/// entry's id, so an id comparison alone cannot tell a session entry from
/// a pre-session one. The floor id only names the redo entries to purge.
#[derive(Debug, Clone, Copy)]
struct Session {
    start_len: usize,
    floor_id: u64,
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
    /// onto every NEW entry as its before-image.
    pending_selection: Vec<EntityId>,
    /// Selection to restore after the last undo/redo — collected by the
    /// host via [`take_selection_restore`].
    ///
    /// [`note_selection`]: Self::note_selection
    /// [`take_selection_restore`]: Self::take_selection_restore
    selection_restore: Option<Vec<EntityId>>,
    /// The open play session's boundary, while one is open.
    session: Option<Session>,
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
            session: None,
        }
    }

    /// Note the CURRENT selection as the before-image for any commands
    /// recorded from now on. Hosts call this before their handlers
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
    /// Ordering is load-bearing: the CURRENT selection (the host's
    /// last [`note_selection`]) is stamped as the entry's after-image, THEN
    /// the command undoes, THEN the before-image (pruned to entities alive
    /// in the restored world — undo is id-exact, pruning only
    /// defends cross-entry staleness) becomes the restore target.
    ///
    /// [`note_selection`]: Self::note_selection
    pub fn undo(&mut self, world: &mut World) -> bool {
        if !self.can_undo() {
            self.selection_restore = None;
            return false;
        }
        // A command issued after an undo must start a fresh entry: merging
        // into the pre-session top would reassign that entry's id and drag
        // it above the session floor.
        self.merge_sealed = true;
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
        if !self.can_redo() {
            self.selection_restore = None;
            return false;
        }
        self.merge_sealed = true;
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
    ///
    /// Inside a play session this stops at the Play boundary: the paused
    /// edits are undoable, the authored history behind them is not until
    /// Stop has decided their fate.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.len() > self.session_start_len()
    }

    /// Whether there is a command to redo — inside a session, only entries
    /// recorded since Play.
    pub fn can_redo(&self) -> bool {
        match self.session {
            Some(session) => self
                .redo_stack
                .last()
                .is_some_and(|entry| entry.id >= session.floor_id),
            None => !self.redo_stack.is_empty(),
        }
    }

    /// The undo stack length at the Play boundary (0 outside a session).
    fn session_start_len(&self) -> usize {
        self.session.map_or(0, |session| session.start_len)
    }

    /// Display name of the command that would be undone, if any.
    pub fn undo_name(&self) -> Option<&str> {
        self.can_undo()
            .then(|| self.undo_stack.back().map(|entry| entry.cmd.display_name()))
            .flatten()
    }

    /// Display name of the command that would be redone, if any.
    pub fn redo_name(&self) -> Option<&str> {
        self.can_redo()
            .then(|| self.redo_stack.last().map(|entry| entry.cmd.display_name()))
            .flatten()
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
        // would stamp wrong before-images onto the next commands.
        self.pending_selection.clear();
        self.selection_restore = None;
        self.session = None;
    }

    /// Push a pre-executed command onto the undo stack without calling execute().
    /// Use when the action was already performed and you just need to record it for undo.
    pub fn push_already_executed(&mut self, cmd: Box<dyn EditorCommand>) {
        self.push_entry(cmd);
    }

    /// [`push_already_executed`] with an EXPLICIT before-image, bypassing
    /// the noted pending selection — for commands whose gesture began long
    /// before the push (a cross-frame API batch: the macro must carry the
    /// selection from `batch begin`, not from the last frame's note).
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

    /// Record already-applied commands as ONE undo entry: none = nothing, one = itself,
    /// many = a `MacroCommand` named `name`.
    pub fn push_as_one(&mut self, name: &str, mut commands: Vec<Box<dyn EditorCommand>>) {
        match commands.len() {
            0 => {}
            1 => {
                if let Some(cmd) = commands.pop() {
                    self.push_already_executed(cmd);
                }
            }
            _ => self.push_already_executed(Box::new(MacroCommand::new(name, commands))),
        }
    }

    /// `execute` counterpart for commands not yet applied.
    /// None records nothing, one executes and records raw, many execute and record
    /// as a `MacroCommand` named `name`.
    pub fn execute_as_one(
        &mut self,
        name: &str,
        mut commands: Vec<Box<dyn EditorCommand>>,
        world: &mut World,
    ) {
        match commands.len() {
            0 => {}
            1 => {
                if let Some(cmd) = commands.pop() {
                    self.execute(cmd, world);
                }
            }
            _ => self.execute(Box::new(MacroCommand::new(name, commands)), world),
        }
    }

    /// Try to merge `cmd` with the last undo command. If merging fails, execute normally.
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
                // Merged state = new id + no redo;
                // selection_before keeps the FIRST before-image.
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

    /// Mark the Play boundary: the stack position AND the id floor, taken
    /// together. Every entry recorded from here belongs to the session.
    pub fn begin_session(&mut self) {
        self.break_merge();
        self.session = Some(Session {
            start_len: self.undo_stack.len(),
            floor_id: self.next_id,
        });
    }

    /// Close the session (Stop, after the entries have been kept or
    /// dropped). The history limit applies again from here.
    pub fn end_session(&mut self) {
        self.session = None;
        self.enforce_limit();
    }

    /// Whether a play session's boundary is currently marked.
    pub fn in_session(&self) -> bool {
        self.session.is_some()
    }

    /// How many undoable entries were recorded since the Play boundary.
    pub fn session_entry_count(&self) -> usize {
        self.undo_stack.len() - self.session_start_len()
    }

    /// Discard every entry recorded since the Play boundary — the undo
    /// stack truncates to it and the redo stack loses its session entries.
    /// Returns how many undo entries went.
    ///
    /// The world is NOT touched: the caller restores the snapshot, which
    /// is what these entries were applied to.
    pub fn drop_session_entries(&mut self) -> usize {
        let Some(session) = self.session else {
            return 0;
        };
        let dropped = self.undo_stack.len() - session.start_len;
        self.undo_stack.truncate(session.start_len);
        self.redo_stack.retain(|entry| entry.id < session.floor_id);
        dropped
    }

    /// Replay the session's undo-stack entries onto the world `replace`
    /// installs, and return how many could NOT be applied.
    ///
    /// The order is load-bearing. Each session entry undoes on the PAUSED
    /// world first (newest-first), so a create re-captures its components
    /// and knows to resurrect on the next execute; `replace` then installs
    /// the authored world; then each entry rebases against the world the
    /// entries before it already produced and re-executes (oldest-first).
    /// Ids are stable across the restore, so the entries resurrect their
    /// entities under the ids they had.
    ///
    /// Session entries on the REDO stack are purged, never replayed: Keep
    /// keeps what was applied at Stop, and an edit the user undid while
    /// paused stays undone.
    pub fn rebase_session_entries(
        &mut self,
        world: &mut World,
        replace: impl FnOnce(&mut World),
    ) -> usize {
        let Some(session) = self.session else {
            replace(world);
            return 0;
        };
        let mut entries = Vec::with_capacity(self.undo_stack.len() - session.start_len);
        while self.undo_stack.len() > session.start_len {
            if let Some(mut entry) = self.undo_stack.pop_back() {
                entry.cmd.undo(world);
                entries.push(entry);
            }
        }
        entries.reverse();
        self.redo_stack.retain(|entry| entry.id < session.floor_id);

        replace(world);

        let mut dropped = 0;
        for mut entry in entries {
            match entry.cmd.rebase_onto(world) {
                Rebase::Apply => {
                    entry.cmd.execute(world);
                    self.undo_stack.push_back(entry);
                }
                Rebase::Partial { dropped: children } => {
                    dropped += children;
                    entry.cmd.execute(world);
                    self.undo_stack.push_back(entry);
                }
                Rebase::Drop { entries } => dropped += entries,
            }
        }
        dropped
    }

    fn enforce_limit(&mut self) {
        while self.undo_stack.len() > self.max_history {
            // Eviction is suspended for the session's own entries until
            // Stop has replayed or dropped them: Keep must find the paused
            // create a hundred-and-first edit depends on. Only authored
            // entries behind the boundary go, and the boundary follows
            // them down; once the session ends the cap means what it
            // always did, oldest first.
            if let Some(session) = &mut self.session {
                if session.start_len == 0 {
                    return;
                }
                session.start_len -= 1;
            }
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
#[cfg(test)]
mod session_tests;
