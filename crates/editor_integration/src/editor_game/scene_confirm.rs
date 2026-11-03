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

use editor::song::SaveIntent;
use editor::{ConfirmChoice, ConfirmDialog};
use engine_core::contexts::GameContext;
use engine_core::Game;

use super::EditorGame;

/// An action awaiting the user's choice about the edits it would lose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingSceneAction {
    NewScene,
    OpenScene(PathBuf),
    OpenSong(PathBuf),
    CloseSong,
    /// Play over a dirty song: the game must not start against a song it lacks.
    StartPlay,
    /// A save waiting for Overwrite or Cancel: onto another song's file, or over the song's
    /// own sidecar changed on disk since it was read.
    OverwriteSong(PathBuf, OverwriteCause),
    /// The open scene changed on disk under unsaved edits: Reload or Keep mine.
    ReloadFromDisk(PathBuf),
    /// The open song's sidecar changed on disk under unsaved edits, the song named by its
    /// `.mid`: Reload or Keep mine.
    ReloadSongMap(PathBuf),
}

/// Why a save asks before it overwrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverwriteCause {
    /// A Save As onto another song's files.
    AnotherSong,
    /// The song's own sidecar changed on disk since the song read it.
    MapChanged,
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
            PendingSceneAction::OpenSong(_) => "opening another song",
            PendingSceneAction::CloseSong => "closing it",
            PendingSceneAction::StartPlay => "Play",
            PendingSceneAction::OverwriteSong(_, OverwriteCause::AnotherSong) => "replacing another song",
            PendingSceneAction::OverwriteSong(_, OverwriteCause::MapChanged) => "replacing the map on disk",
            PendingSceneAction::ReloadFromDisk(_) => "reloading it from disk",
            PendingSceneAction::ReloadSongMap(_) => "reloading its map from disk",
        }
    }

    pub(super) fn document(&self) -> Document {
        match self {
            PendingSceneAction::NewScene
            | PendingSceneAction::OpenScene(_)
            | PendingSceneAction::ReloadFromDisk(_) => Document::Scene,
            PendingSceneAction::OpenSong(_)
            | PendingSceneAction::CloseSong
            | PendingSceneAction::StartPlay
            | PendingSceneAction::OverwriteSong(..)
            | PendingSceneAction::ReloadSongMap(_) => Document::Song,
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
            (PendingSceneAction::OverwriteSong(path, OverwriteCause::AnotherSong), _) => {
                ConfirmDialog::overwrite_song(path)
            }
            (PendingSceneAction::OverwriteSong(path, OverwriteCause::MapChanged), _) => {
                ConfirmDialog::map_changed_on_disk(&editor::external_tools::midi_sidecar_path(path))
            }
            (PendingSceneAction::ReloadFromDisk(path), _) => ConfirmDialog::scene_changed_on_disk(path),
            (PendingSceneAction::ReloadSongMap(path), _) => song_map_changed_on_disk(path),
            (_, Document::Song) => ConfirmDialog::unsaved_song(action.phrase()),
            (_, Document::Scene) => ConfirmDialog::unsaved_changes(action.phrase()),
        };
        // Enter pressed since last frame = the primary action;
        // mouse clicks come from the rendered buttons.
        let key_choice = self.scene_confirm.pending_choice.take();
        let choice = dialog.render(ctx.ui, ctx.window_size, &self.editor.theme).or(key_choice);
        if action.document() == Document::Song {
            if let Some(choice) = choice {
                self.resolve_song_choice(action, choice);
            }
            return;
        }
        if let (PendingSceneAction::ReloadFromDisk(path), Some(choice)) = (&action, choice) {
            self.resolve_scene_reload_choice(ctx.world, ctx.assets, path, choice);
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
    ) {
        if let PendingSceneAction::ReloadSongMap(path) = &action {
            return self.resolve_song_map_reload_choice(path, choice);
        }
        if let (PendingSceneAction::OverwriteSong(path, _), ConfirmChoice::Confirm) = (&action, choice) {
            match self.save_song_to(path.clone(), SaveIntent::OVERWRITE) {
                Ok(_) => self.scene_confirm.pending_action = None,
                Err(error) => self.editor.status_bar.show_error(format!("Song save failed: {error}")),
            }
            return;
        }
        match choice {
            ConfirmChoice::Confirm => match self.save_song(None) {
                Ok(Some(_)) => {
                    self.scene_confirm.pending_action = None;
                    self.perform_song_action(action);
                }
                // The save parked the sidecar's Overwrite / Cancel prompt in its place:
                // nothing was written, so the action it was to clear the way for waits.
                Ok(None) => self
                    .editor
                    .status_bar
                    .show_notice(format!("The map changed on disk; answer the prompt, then {} again", action.phrase())),
                Err(error) => {
                    self.editor.status_bar.show_error(format!("Song save failed: {error}"));
                    log::error!("Song save before {} failed: {error}", action.phrase());
                    if self.song_lacks_provenance() {
                        // The way out is two fields the dialog would cover.
                        self.scene_confirm.pending_action = None;
                        self.reveal_missing_song_provenance();
                    }
                }
            },
            ConfirmChoice::Alt => {
                // The overwrite prompt and a never-saved song's Play prompt draw no
                // alternative; this guards a choice arriving some other way.
                let no_such_arm = matches!(action, PendingSceneAction::OverwriteSong(..))
                    || (action == PendingSceneAction::StartPlay && !self.song_saved_exists());
                if no_such_arm {
                    return;
                }
                self.scene_confirm.pending_action = None;
                self.perform_song_action(action);
            }
            ConfirmChoice::Cancel => {
                self.scene_confirm.pending_action = None;
                self.editor.status_bar.show_message("Cancelled");
            }
        }
    }

    /// Route the choice on the changed-on-disk prompt: Reload replaces the edited scene with
    /// the file; Keep mine — Escape and Enter included — closes the prompt and leaves the scene, its
    /// edits and the watch as they are.
    pub(super) fn resolve_scene_reload_choice(
        &mut self,
        world: &mut ecs::World,
        assets: &mut engine_core::assets::AssetManager,
        path: &std::path::Path,
        choice: ConfirmChoice,
    ) {
        self.scene_confirm.pending_action = None;
        match choice {
            ConfirmChoice::Confirm => self.reload_scene_from_disk(world, assets, path),
            ConfirmChoice::Alt | ConfirmChoice::Cancel => {
                self.editor.status_bar.show_message("Kept your edits; the scene on disk differs");
            }
        }
    }

    /// Route the choice on the song map's changed-on-disk prompt as the scene's is routed:
    /// Reload reads the sidecar anew, dropping the map's unsaved edits and keeping the
    /// score's; Keep mine — Escape and Enter included — leaves the song as it is.
    pub(super) fn resolve_song_map_reload_choice(&mut self, midi: &std::path::Path, choice: ConfirmChoice) {
        self.scene_confirm.pending_action = None;
        match choice {
            ConfirmChoice::Confirm => self.reload_song_map(midi),
            ConfirmChoice::Alt | ConfirmChoice::Cancel => {
                self.editor.status_bar.show_message("Kept your edits; the song's map on disk differs");
            }
        }
    }

    /// Execute a (now confirmed) song action, or Play.
    pub(super) fn perform_song_action(&mut self, action: PendingSceneAction) {
        match action {
            PendingSceneAction::OpenSong(path) => {
                // The open song's own file reads both files anew: the reload of a score or a
                // map another program rewrote.
                let result = self.open_song(&path).map(drop);
                self.report_song_failure("Song open", result);
            }
            PendingSceneAction::CloseSong => self.close_song(),
            PendingSceneAction::StartPlay => self.begin_play(),
            PendingSceneAction::OverwriteSong(path, _) => {
                let result = self.save_song_to(path, SaveIntent::OVERWRITE).map(drop);
                self.report_song_failure("Song save", result);
            }
            PendingSceneAction::ReloadSongMap(path) => self.reload_song_map(&path),
            PendingSceneAction::NewScene | PendingSceneAction::OpenScene(_) | PendingSceneAction::ReloadFromDisk(_) => {
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
            PendingSceneAction::ReloadFromDisk(path) => self.reload_scene_from_disk(ctx.world, ctx.assets, &path),
            song_action => self.perform_song_action(song_action),
        }
    }

    /// Keyboard policy while the dialog shows: Escape cancels it (ahead of
    /// the normal cancel cascade), Enter activates the primary action
    /// (Save — routed through the next render, which owns the GameContext),
    /// and EVERY other key is swallowed — Ctrl+S or Delete acting under a
    /// modal would mutate state the user is being asked about. Tab cycling
    /// is deliberately out of scope for now (mouse-first).
    /// The changed-on-disk prompts, the scene's and the song map's, are the exception: it appears unasked, so an
    /// Enter meant for something else lands on it, and both keys keep the
    /// edits — Reload, which discards them past undo, is a click.
    /// Returns whether the key was consumed.
    pub(super) fn confirm_dialog_consumes_key(&mut self, key: winit::keyboard::KeyCode) -> bool {
        if self.stop_dialog_consumes_key(key) {
            return true;
        }
        let Some(action) = &self.scene_confirm.pending_action else {
            return false;
        };
        let reload_prompt = matches!(action, PendingSceneAction::ReloadFromDisk(_) | PendingSceneAction::ReloadSongMap(_));
        match key {
            winit::keyboard::KeyCode::Escape | winit::keyboard::KeyCode::Enter | winit::keyboard::KeyCode::NumpadEnter
                if reload_prompt =>
            {
                self.scene_confirm.pending_choice = Some(ConfirmChoice::Cancel);
            }
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

/// The prompt when another program rewrote the open song's sidecar while the song had unsaved
/// edits; `midi` names the song.
fn song_map_changed_on_disk(midi: &std::path::Path) -> ConfirmDialog {
    let sidecar = editor::external_tools::midi_sidecar_path(midi);
    let name = sidecar
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| sidecar.display().to_string());
    ConfirmDialog {
        title: "Song map changed on disk".to_string(),
        message: format!("The map changed on disk: {name}. Reload it and discard your edits to the map?"),
        confirm_label: "Reload".to_string(),
        alt_label: None,
        cancel_label: "Keep mine".to_string(),
    }
}
