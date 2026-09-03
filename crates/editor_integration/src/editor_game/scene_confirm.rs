//! The unsaved-changes confirm flow for scene-replacing actions:
//! New Scene / Open Scene on a dirty world raise a real
//! Save / Discard / Cancel modal instead of a status-bar warning.
//!
//! State machine: [`request_scene_replace`] either proceeds immediately
//! (clean world), refuses (play session — the standing refusal), or
//! parks the action in `pending_scene_action`; the dialog renders EARLY in
//! the frame (the drag-ghost pattern — its full-window blocking rect must
//! land before the widgets it protects) and routes the choice.
//!
//! [`request_scene_replace`]: EditorGame::request_scene_replace

use std::path::PathBuf;

use editor::{ConfirmChoice, ConfirmDialog};
use engine_core::contexts::GameContext;
use engine_core::Game;

use super::EditorGame;

/// A scene-replacing action awaiting the user's Save/Discard/Cancel choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingSceneAction {
    NewScene,
    OpenScene(PathBuf),
}

impl PendingSceneAction {
    /// The "…before X?" phrase for the dialog message.
    fn phrase(&self) -> &'static str {
        match self {
            PendingSceneAction::NewScene => "creating a new scene",
            PendingSceneAction::OpenScene(_) => "opening another scene",
        }
    }
}

impl<G: Game> EditorGame<G> {
    /// Gate a scene-replacing action behind the dirty check: `true` means
    /// proceed NOW (clean world). A dirty world parks the action for the
    /// confirm dialog; a play session refuses outright (unchanged —
    /// the dialog is Editing-only, so it never needs to hide Save).
    pub(super) fn request_scene_replace(&mut self, action: PendingSceneAction) -> bool {
        if let Some(msg) = self.scene_replace_refusal() {
            self.editor.status_bar.show_error(msg.to_string());
            return false;
        }
        if self.command_history.is_dirty() {
            self.pending_scene_action = Some(action);
            return false;
        }
        true
    }

    /// Render the pending confirm dialog and route its outcome. Called
    /// right after the drag ghost (frame step 2c): the modal's full-window
    /// blocking rect makes every later widget this frame inert.
    pub(super) fn render_scene_confirm_dialog(&mut self, ctx: &mut GameContext) {
        let Some(action) = self.pending_scene_action.clone() else {
            return;
        };
        // A modal is keyboard-modal: whatever text field held focus loses
        // it, so it neither swallows the dialog's keys nor keeps receiving
        // typed characters underneath.
        ctx.ui.clear_text_focus();
        let dialog = ConfirmDialog::unsaved_changes(action.phrase());
        // Enter pressed since last frame = the primary action;
        // mouse clicks come from the rendered buttons.
        let key_choice = self.pending_dialog_choice.take();
        match dialog.render(ctx.ui, ctx.window_size, &self.editor.theme).or(key_choice) {
            Some(ConfirmChoice::Confirm) => {
                // Save, then proceed — but a FAILED save keeps the dialog
                // open: closing it would silently drop the user's decision
                // while their work is still unsaved.
                match self.save_scene(ctx.world, ctx.assets) {
                    Ok(()) => {
                        self.pending_scene_action = None;
                        self.perform_scene_action(ctx, action);
                    }
                    Err(e) => {
                        self.editor.status_bar.show_error(format!("Save failed: {e}"));
                        log::error!("Save before scene replace failed: {e}");
                    }
                }
            }
            Some(ConfirmChoice::Alt) => {
                self.pending_scene_action = None;
                self.perform_scene_action(ctx, action);
            }
            Some(ConfirmChoice::Cancel) => {
                self.pending_scene_action = None;
                self.editor.status_bar.show_message("Cancelled");
            }
            None => {}
        }
    }

    /// Execute a (now confirmed) scene-replacing action.
    pub(super) fn perform_scene_action(&mut self, ctx: &mut GameContext, action: PendingSceneAction) {
        match action {
            PendingSceneAction::NewScene => self.new_scene(ctx.world),
            PendingSceneAction::OpenScene(path) => {
                self.load_scene_with_feedback(ctx.world, ctx.assets, &path);
            }
        }
    }

    /// Keyboard policy while the dialog shows: Escape cancels it (ahead of
    /// the normal cancel cascade), Enter activates the primary action
    /// (Save — routed through the next render, which owns the GameContext),
    /// and EVERY other key is swallowed — Ctrl+S or Delete acting under a
    /// modal would mutate state the user is being asked about. Tab cycling
    /// is deliberately out of scope for now (mouse-first).
    /// Returns whether the key was consumed.
    pub(super) fn confirm_dialog_consumes_key(&mut self, key: winit::keyboard::KeyCode) -> bool {
        if self.pending_scene_action.is_none() {
            return false;
        }
        match key {
            winit::keyboard::KeyCode::Escape => {
                self.pending_scene_action = None;
                self.pending_dialog_choice = None;
                self.editor.status_bar.show_message("Cancelled");
            }
            winit::keyboard::KeyCode::Enter | winit::keyboard::KeyCode::NumpadEnter => {
                self.pending_dialog_choice = Some(ConfirmChoice::Confirm);
            }
            _ => {}
        }
        true
    }
}
