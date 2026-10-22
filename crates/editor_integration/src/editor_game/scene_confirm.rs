//! The unsaved-changes confirm flow for actions that would lose a document's
//! edits: New Scene / Open Scene on a dirty world, and New / Open / Close Song
//! and Play on a dirty song, raise a real modal instead of a
//! status-bar warning.
//!
//! State machine: [`request_document_action`] either proceeds immediately
//! (the document the action names is clean), refuses (a play session — the
//! standing refusal), or parks the action in `scene_confirm.pending_action`;
//! the dialog renders EARLY in the frame (the drag-ghost pattern — its
//! full-window blocking rect must land before the widgets it protects) and
//! routes the choice. The song is a second document: a scene action never
//! prompts for the song, which opening another scene does not touch, and a
//! song action never prompts for the scene.
//!
//! [`request_document_action`]: EditorGame::request_document_action

use std::path::PathBuf;

use editor::{ConfirmChoice, ConfirmDialog};
use engine_core::contexts::GameContext;
use engine_core::Game;

use super::EditorGame;

/// An action awaiting the user's choice about the edits it would lose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingSceneAction {
    NewScene,
    OpenScene(PathBuf),
    NewSong,
    OpenSong(PathBuf),
    CloseSong,
    /// Play over a dirty song: the game must not start against a song it lacks.
    StartPlay,
    /// A Save As onto another song's file, waiting for Overwrite or Cancel.
    OverwriteSong(PathBuf),
}

/// The document whose edits an action would lose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Document {
    Scene,
    Song,
}

impl PendingSceneAction {
    /// The "…before X?" phrase for the dialog message.
    fn phrase(&self) -> &'static str {
        match self {
            PendingSceneAction::NewScene => "creating a new scene",
            PendingSceneAction::OpenScene(_) => "opening another scene",
            PendingSceneAction::NewSong => "creating a new song",
            PendingSceneAction::OpenSong(_) => "opening another song",
            PendingSceneAction::CloseSong => "closing it",
            PendingSceneAction::StartPlay => "Play",
            PendingSceneAction::OverwriteSong(_) => "replacing another song",
        }
    }

    pub(super) fn document(&self) -> Document {
        match self {
            PendingSceneAction::NewScene | PendingSceneAction::OpenScene(_) => Document::Scene,
            PendingSceneAction::NewSong
            | PendingSceneAction::OpenSong(_)
            | PendingSceneAction::CloseSong
            | PendingSceneAction::StartPlay
            | PendingSceneAction::OverwriteSong(_) => Document::Song,
        }
    }
}

/// The unsaved-changes dialog's state machine; a `Some` action blocks the frame's input.
#[derive(Default)]
pub(super) struct SceneConfirm {
    /// A scene-replacing action (New/Open) awaiting the unsaved-changes
    /// confirm dialog. While set, a modal blocks the frame's input.
    pub pending_action: Option<PendingSceneAction>,
    /// Enter pressed while the dialog is up — consumed by the next dialog
    /// render as the primary (Save) action.
    pub pending_choice: Option<editor::ConfirmChoice>,
}

impl<G: Game> EditorGame<G> {
    /// Gate an action behind the dirty check of the document it names: `true`
    /// means proceed NOW (that document is clean). A dirty one parks the action
    /// for the confirm dialog; a play session refuses outright (unchanged —
    /// the dialog is Editing-only, so it never needs to hide Save). Play itself
    /// is only requested while Editing.
    pub(super) fn request_document_action(&mut self, action: PendingSceneAction) -> bool {
        let refusal = match action.document() {
            Document::Scene => self.scene_replace_refusal(),
            Document::Song => self.song_change_refusal(),
        };
        if let Some(msg) = refusal.filter(|_| action != PendingSceneAction::StartPlay) {
            self.editor.status_bar.show_error(msg.to_string());
            return false;
        }
        if self.is_document_dirty(action.document()) {
            self.scene_confirm.pending_action = Some(action);
            return false;
        }
        true
    }

    fn is_document_dirty(&self, document: Document) -> bool {
        match document {
            Document::Scene => self.command_history.is_dirty(),
            Document::Song => self.song.as_ref().is_some_and(editor::SongDocument::is_dirty),
        }
    }

    /// Render the pending confirm dialog and route its outcome. Called
    /// right after the drag ghost (frame step 2c): the modal's full-window
    /// blocking rect makes every later widget this frame inert.
    pub(super) fn render_scene_confirm_dialog(&mut self, ctx: &mut GameContext) {
        let Some(action) = self.scene_confirm.pending_action.clone() else {
            return;
        };
        // A modal is keyboard-modal: whatever text field held focus loses
        // it, so it neither swallows the dialog's keys nor keeps receiving
        // typed characters underneath.
        ctx.ui.clear_text_focus();
        let dialog = match (&action, action.document()) {
            (PendingSceneAction::StartPlay, _) => ConfirmDialog::play_with_unsaved_song(self.song_saved_exists()),
            (PendingSceneAction::OverwriteSong(path), _) => ConfirmDialog::overwrite_song(path),
            (_, Document::Song) => ConfirmDialog::unsaved_song(action.phrase()),
            (_, Document::Scene) => ConfirmDialog::unsaved_changes(action.phrase()),
        };
        // Enter pressed since last frame = the primary action;
        // mouse clicks come from the rendered buttons.
        let key_choice = self.scene_confirm.pending_choice.take();
        let choice = dialog.render(ctx.ui, ctx.window_size, &self.editor.theme).or(key_choice);
        if action.document() == Document::Song {
            if let Some(choice) = choice {
                self.resolve_song_choice(action, choice, ctx.world);
            }
            return;
        }
        match choice {
            Some(ConfirmChoice::Confirm) => {
                // Save, then proceed — but a FAILED save keeps the dialog
                // open: closing it would silently drop the user's decision
                // while their work is still unsaved.
                match self.save_scene(ctx.world, ctx.assets) {
                    Ok(()) => {
                        self.scene_confirm.pending_action = None;
                        self.perform_scene_action(ctx, action);
                    }
                    Err(e) => {
                        self.editor.status_bar.show_error(format!("Save failed: {e}"));
                        log::error!("Save before scene replace failed: {e}");
                    }
                }
            }
            Some(ConfirmChoice::Alt) => {
                self.scene_confirm.pending_action = None;
                self.perform_scene_action(ctx, action);
            }
            Some(ConfirmChoice::Cancel) => {
                self.scene_confirm.pending_action = None;
                self.editor.status_bar.show_message("Cancelled");
            }
            None => {}
        }
    }

    /// Route the choice made on a song's prompt: Save saves the song (a failed
    /// save keeps the dialog, as the scene's does) and proceeds; Discard — or,
    /// on Play, "Play saved" — proceeds keeping nothing but the saved file;
    /// Cancel leaves everything as it was. The overwrite prompt's Overwrite is
    /// the save it parked, completed in one click past the check that parked
    /// it. World-only, so the headless suite drives it.
    pub(super) fn resolve_song_choice(
        &mut self,
        action: PendingSceneAction,
        choice: ConfirmChoice,
        world: &mut ecs::World,
    ) {
        if let (PendingSceneAction::OverwriteSong(path), ConfirmChoice::Confirm) = (&action, choice) {
            match self.save_song_to(path.clone()) {
                Ok(_) => self.scene_confirm.pending_action = None,
                Err(error) => self.editor.status_bar.show_error(format!("Song save failed: {error}")),
            }
            return;
        }
        match choice {
            ConfirmChoice::Confirm => match self.save_song(None) {
                Ok(_) => {
                    self.scene_confirm.pending_action = None;
                    self.perform_song_action(action, world);
                }
                Err(error) => {
                    self.editor.status_bar.show_error(format!("Song save failed: {error}"));
                    log::error!("Song save before {} failed: {error}", action.phrase());
                }
            },
            ConfirmChoice::Alt => {
                // The overwrite prompt and a never-saved song's Play prompt draw no
                // alternative; this guards a choice arriving some other way.
                let no_such_arm = matches!(action, PendingSceneAction::OverwriteSong(_))
                    || (action == PendingSceneAction::StartPlay && !self.song_saved_exists());
                if no_such_arm {
                    return;
                }
                self.scene_confirm.pending_action = None;
                self.perform_song_action(action, world);
            }
            ConfirmChoice::Cancel => {
                self.scene_confirm.pending_action = None;
                self.editor.status_bar.show_message("Cancelled");
            }
        }
    }

    /// Execute a (now confirmed) song action, or Play.
    pub(super) fn perform_song_action(&mut self, action: PendingSceneAction, world: &mut ecs::World) {
        match action {
            PendingSceneAction::NewSong => self.new_song(),
            PendingSceneAction::OpenSong(path) => {
                // Reopening the open song's own file would only replace it under its
                // history, so a clean song is left as it is.
                if self.song.as_ref().and_then(|song| song.path()) == Some(path.as_path()) {
                    self.editor.status_bar.show_message("Song already open");
                    return;
                }
                let result = self.open_song(&path).map(drop);
                self.report_song_failure("Song open", result);
            }
            PendingSceneAction::CloseSong => self.close_song(),
            PendingSceneAction::StartPlay => self.begin_play(world),
            PendingSceneAction::OverwriteSong(path) => {
                let result = self.save_song_to(path).map(drop);
                self.report_song_failure("Song save", result);
            }
            PendingSceneAction::NewScene | PendingSceneAction::OpenScene(_) => {
                log::error!("{action:?} is a scene action, routed by the scene's Save branch");
            }
        }
    }

    /// Execute a (now confirmed) scene-replacing action.
    pub(super) fn perform_scene_action(&mut self, ctx: &mut GameContext, action: PendingSceneAction) {
        match action {
            PendingSceneAction::NewScene => self.new_scene(ctx.world),
            PendingSceneAction::OpenScene(path) => {
                self.load_scene_with_feedback(ctx.world, ctx.assets, &path);
            }
            song_action => self.perform_song_action(song_action, ctx.world),
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
        if self.stop_dialog_consumes_key(key) {
            return true;
        }
        if self.scene_confirm.pending_action.is_none() {
            return false;
        }
        match key {
            winit::keyboard::KeyCode::Escape => {
                self.scene_confirm.pending_action = None;
                self.scene_confirm.pending_choice = None;
                self.editor.status_bar.show_message("Cancelled");
            }
            winit::keyboard::KeyCode::Enter | winit::keyboard::KeyCode::NumpadEnter => {
                self.scene_confirm.pending_choice = Some(ConfirmChoice::Confirm);
            }
            _ => {}
        }
        true
    }
}
