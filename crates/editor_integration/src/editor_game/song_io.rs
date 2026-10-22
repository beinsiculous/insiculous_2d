//! The song's hosted half: new, open, save and close; the `song new|open|save|close`
//! API verbs that reach them; and the refusals of the preview verbs, whose transport does
//! not exist yet.
//!
//! A song is a second document beside the scene. Its writes are refused for the whole play
//! session, and New, Open and Close never silently drop a dirty song: the GUI asks through
//! the confirm machine (`scene_confirm.rs`), and the API, which has no dialog, refuses unless
//! the line ends in `discard`. A preview verb is a song write too, so the play-session and
//! scene-batch refusal answers it before "not available yet" does.

use std::path::{Path, PathBuf};

use editor::command_api::{song_write_refusal, ApiError, HostedWrite};
use editor::SongDocument;
use engine_core::Game;

use super::scene_confirm::PendingSceneAction;
use super::EditorGame;

impl<G: Game> EditorGame<G> {
    /// `Some(message)` while a play session runs: the song cannot change under it.
    pub(super) fn song_change_refusal(&self) -> Option<&'static str> {
        self.editor
            .in_play_session()
            .then_some("the song cannot change during a play session — stop Play first")
    }

    /// Whether the song's saved file exists, so a prompt may offer to play it.
    pub(super) fn song_saved_exists(&self) -> bool {
        self.song
            .as_ref()
            .and_then(|song| song.path())
            .is_some_and(|path| common::vfs::file_size(path).is_ok())
    }

    /// Save the song to `path`, else to its own path, else — never saved — to
    /// [`Self::default_song_path`]. Returns where it saved, or `None` when `path` is another
    /// song's file: the save is parked behind the overwrite prompt instead. With no `path`
    /// it never parks — the song's own file and the default's free name are no other song's.
    pub(super) fn save_song(&mut self, path: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
        if let Some(refusal) = self.song_change_refusal() {
            return Err(refusal.to_string());
        }
        let own_path = self
            .song
            .as_ref()
            .ok_or("no song is open")?
            .path()
            .map(Path::to_path_buf);
        if let Some(target) = path.as_ref().filter(|target| self.is_another_songs_file(target)) {
            self.scene_confirm.pending_action = Some(PendingSceneAction::OverwriteSong(target.clone()));
            return Ok(None);
        }
        let target = path
            .or(own_path)
            .unwrap_or_else(|| self.default_song_path());
        self.save_song_to(target).map(Some)
    }

    /// Save the song to exactly `target`, whatever is there: the overwrite prompt's
    /// Overwrite and the API's `overwrite` come here past the check `save_song` makes.
    pub(super) fn save_song_to(&mut self, target: PathBuf) -> Result<PathBuf, String> {
        if let Some(refusal) = self.song_change_refusal() {
            return Err(refusal.to_string());
        }
        let asset_base = self.asset_base.clone();
        let song = self.song.as_mut().ok_or("no song is open")?;
        song.save(&target, &asset_base)
            .map_err(|error| error.to_string())?;
        self.sync_dirty_mirror();
        self.editor.composer.rescan_songs(&self.asset_base);
        self.editor.status_bar.show_message("Song saved");
        log::info!("Song saved to: {}", target.display());
        Ok(target)
    }

    /// Whether `target`'s `.mid` or `.mid.ron` exists and is not the open song's own file:
    /// another song, which a save there would replace. The two paths are compared by
    /// their components, so a typed `music/./theme.mid` is still the song's own file.
    pub(super) fn is_another_songs_file(&self, target: &Path) -> bool {
        let own = self.song.as_ref().and_then(SongDocument::path);
        let own_file = own.is_some_and(|own| own.components().eq(target.components()));
        !own_file && song_files_exist(target)
    }

    /// Replace the song with a new, never-saved one.
    pub(super) fn new_song(&mut self) {
        match SongDocument::new_song() {
            Ok(song) => {
                self.song = Some(song);
                self.song_generation += 1;
                self.follow_song();
                self.editor.composer.rescan_songs(&self.asset_base);
                self.editor.status_bar.show_message("New song");
            }
            Err(error) => self
                .editor
                .status_bar
                .show_error(format!("New song failed: {error}")),
        }
        self.sync_dirty_mirror();
    }

    /// Open the song at `path`, settling what an interrupted save left; returns what the
    /// open settled.
    pub(super) fn open_song(&mut self, path: &Path) -> Result<Vec<String>, String> {
        let opened =
            SongDocument::open(path, &self.asset_base).map_err(|error| error.to_string())?;
        self.song = Some(opened.document);
        self.song_generation += 1;
        self.follow_song();
        self.editor.composer.rescan_songs(&self.asset_base);
        self.sync_dirty_mirror();
        match opened.notes.first() {
            Some(note) => self.editor.status_bar.show_error(note.clone()),
            None => self.editor.status_bar.show_message("Song opened"),
        }
        Ok(opened.notes)
    }

    pub(super) fn close_song(&mut self) {
        self.song = None;
        self.song_generation += 1;
        self.follow_song();
        self.sync_dirty_mirror();
        self.editor.status_bar.show_message("Song closed");
    }

    /// Report a song open that failed on the status bar.
    pub(super) fn report_song_failure(&mut self, what: &str, result: Result<(), String>) {
        if let Err(error) = result {
            self.editor
                .status_bar
                .show_error(format!("{what} failed: {error}"));
            log::error!("{what} failed: {error}");
        }
    }

    /// The hosted `song …` verbs.
    pub(super) fn run_hosted_song_write(
        &mut self,
        hosted: &HostedWrite,
    ) -> Result<serde_json::Value, ApiError> {
        if let Some(refusal) =
            song_write_refusal(self.editor.play_state(), self.api.batch.is_some())
        {
            return Err(refusal);
        }
        if let Some(verb) = preview_verb(hosted) {
            return Err(ApiError::Refused(format!(
                "song {verb} is not available yet: the composer's preview transport arrives \
                 in Conductor batch 11"
            )));
        }
        let dirty = self.song.as_ref().is_some_and(SongDocument::is_dirty);
        let dirty_refusal = |verb: &str| {
            ApiError::Refused(format!(
                "the song has unsaved edits — `song save` first, or end the line with `discard` \
                 (`song {verb} … discard`) to drop them"
            ))
        };
        match hosted {
            HostedWrite::SongNew { discard } => {
                if dirty && !discard {
                    return Err(dirty_refusal("new"));
                }
                self.new_song();
                Ok(serde_json::json!({ "song": "new", "dirty": true }))
            }
            HostedWrite::SongOpen { path, discard } => {
                if dirty && !discard {
                    return Err(dirty_refusal("open"));
                }
                let path = self.resolve_asset_path(Path::new(path));
                let notes = self.open_song(&path).map_err(ApiError::Refused)?;
                let song = self.song.as_ref();
                Ok(serde_json::json!({
                    "opened": path.display().to_string(),
                    "path": song.and_then(|song| song.path()).map(|path| path.display().to_string()),
                    "dirty": song.is_some_and(SongDocument::is_dirty),
                    "notes": notes,
                }))
            }
            HostedWrite::SongSave { path, overwrite } => {
                let path = path
                    .as_ref()
                    .map(|path| self.resolve_asset_path(Path::new(path)));
                if let Some(target) = path.as_ref().filter(|target| self.is_another_songs_file(target)) {
                    if !overwrite {
                        return Err(ApiError::Refused(format!(
                            "{} is another song — end the line with `overwrite` \
                             (`song save <path> overwrite`) to replace it",
                            target.display()
                        )));
                    }
                }
                let saved = match path {
                    Some(target) => self.save_song_to(target),
                    None => self
                        .save_song(None)
                        .and_then(|saved| saved.ok_or_else(|| "the save was parked".to_string())),
                }
                .map_err(|error| ApiError::Refused(format!("song save failed: {error}")))?;
                Ok(serde_json::json!({ "saved": saved.display().to_string() }))
            }
            HostedWrite::SongClose { discard } => {
                if dirty && !discard {
                    return Err(dirty_refusal("close"));
                }
                self.close_song();
                Ok(serde_json::json!({ "song": "closed" }))
            }
            other => Err(ApiError::Invalid(format!(
                "{other:?} is not a hosted song write"
            ))),
        }
    }
}

/// Whether a song's `.mid` or its `.mid.ron` exists at `target`.
pub(super) fn song_files_exist(target: &Path) -> bool {
    let mut sidecar = target.as_os_str().to_owned();
    sidecar.push(".ron");
    common::vfs::file_size(target).is_ok() || common::vfs::file_size(Path::new(&sidecar)).is_ok()
}

/// The preview verb a hosted write is, if it is one.
fn preview_verb(hosted: &HostedWrite) -> Option<&'static str> {
    match hosted {
        HostedWrite::SongPlay { .. } => Some("play"),
        HostedWrite::SongStop => Some("stop"),
        HostedWrite::SongSeek { .. } => Some("seek"),
        HostedWrite::SongTrigger { .. } => Some("trigger"),
        HostedWrite::Create { .. }
        | HostedWrite::Save { .. }
        | HostedWrite::SongNew { .. }
        | HostedWrite::SongOpen { .. }
        | HostedWrite::SongSave { .. }
        | HostedWrite::SongClose { .. } => None,
    }
}
