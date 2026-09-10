//! The Keep / Discard / Cancel flow for edits recorded while Paused.
//!
//! Paused is editable by construction — every edit guard tests
//! `is_playing()` — so a user can move an entity, then press Stop, and the
//! snapshot restore would erase the move without a word. Stop asks instead:
//! Keep rebases those edits onto the restored world, Discard truncates the
//! history to the Play boundary, Cancel leaves the session paused with its
//! edits intact.
//!
//! The dialog renders on the Modal layer right after the scene dialog (the
//! drag-ghost pattern — a full-window blocking rect only protects widgets
//! registered after it).

use editor::{ConfirmChoice, ConfirmDialog};
use engine_core::contexts::GameContext;
use engine_core::Game;

use super::play_session::PausedEdits;
use super::EditorGame;

/// The stop dialog's state machine; `pending` blocks the frame's input.
#[derive(Default)]
pub(super) struct StopConfirm {
    /// A Stop awaiting the user's Keep/Discard/Cancel choice.
    pub pending: bool,
    /// Enter pressed while the dialog is up — consumed by the next dialog
    /// render as the primary (Keep) action.
    pub pending_choice: Option<ConfirmChoice>,
}

impl<G: Game> EditorGame<G> {
    /// The one door to Stop. Returns `true` when the session actually
    /// stopped this call (so the caller notifies the inner game); a parked
    /// dialog returns `false` and the session stays Paused.
    pub(super) fn request_stop(&mut self, world: &mut ecs::World) -> bool {
        if !self.editor.in_play_session() {
            return false;
        }
        // The batch was applied to the paused world and held outside the
        // history; committing it here is what lets Keep replay it and
        // Discard drop it with everything else — and, with the API refused
        // while the dialog is up, it is also why nothing a batch collected
        // can cross the restore.
        self.commit_open_api_batch("Stop");
        if self.command_history.session_entry_count() == 0 {
            return self.stop_play_session(world, PausedEdits::Discard);
        }
        // A modal over a running simulation is wrong, and pausing makes
        // Cancel mean "stay Paused" whichever state Stop was pressed from.
        self.pause_for_dialog();
        self.stop_confirm.pending = true;
        false
    }

    /// Render the pending stop dialog and route its outcome. Returns
    /// `true` when the session stopped this frame.
    pub(super) fn render_stop_confirm_dialog(&mut self, ctx: &mut GameContext) -> bool {
        if !self.stop_confirm.pending {
            return false;
        }
        ctx.ui.clear_text_focus();
        let count = self.command_history.session_entry_count();
        let dialog = ConfirmDialog::keep_paused_edits(count);
        // `count` is what the dialog asked about; the Keep arm reports what
        // the stop actually retained, which can be fewer.
        let key_choice = self.stop_confirm.pending_choice.take();
        let choice = dialog
            .render(ctx.ui, ctx.window_size, &self.editor.theme)
            .or(key_choice);
        match choice {
            Some(ConfirmChoice::Confirm) => {
                self.stop_confirm.pending = false;
                let outcome = self.stop_with_paused_edits(ctx.world, PausedEdits::Keep);
                self.save_preferences_now();
                self.editor.status_bar.show_message(if outcome.dropped == 0 {
                    format!("Kept {} paused edit(s)", outcome.kept)
                } else {
                    format!(
                        "Kept {} paused edit(s); {} could not be applied",
                        outcome.kept, outcome.dropped
                    )
                });
                true
            }
            Some(ConfirmChoice::Alt) => {
                self.stop_confirm.pending = false;
                self.stop_with_paused_edits(ctx.world, PausedEdits::Discard);
                self.save_preferences_now();
                self.editor
                    .status_bar
                    .show_message(format!("Discarded {count} paused edit(s)"));
                true
            }
            Some(ConfirmChoice::Cancel) => {
                self.stop_confirm.pending = false;
                self.editor.status_bar.show_message("Cancelled — still paused");
                false
            }
            None => false,
        }
    }

    /// Keyboard policy while EITHER modal shows: Escape clears whichever is
    /// up, Enter queues the primary action on it, and every other key is
    /// swallowed — Ctrl+S or Delete acting under a modal would mutate the
    /// state the user is being asked about.
    /// Returns whether the key was consumed.
    pub(super) fn stop_dialog_consumes_key(&mut self, key: winit::keyboard::KeyCode) -> bool {
        if !self.stop_confirm.pending {
            return false;
        }
        match key {
            winit::keyboard::KeyCode::Escape => {
                self.stop_confirm.pending = false;
                self.stop_confirm.pending_choice = None;
                self.editor.status_bar.show_message("Cancelled — still paused");
            }
            winit::keyboard::KeyCode::Enter | winit::keyboard::KeyCode::NumpadEnter => {
                self.stop_confirm.pending_choice = Some(ConfirmChoice::Confirm);
            }
            _ => {}
        }
        true
    }
}
