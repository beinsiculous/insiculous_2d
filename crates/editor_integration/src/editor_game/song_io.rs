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
    /// [`Self::default_song_path`]. Returns where it saved.
    pub(super) fn save_song(&mut self, path: Option<PathBuf>) -> Result<PathBuf, String> {
        if let Some(refusal) = self.song_change_refusal() {
            return Err(refusal.to_string());
        }
        let default_path = self.default_song_path();
        let asset_base = self.asset_base.clone();
        let song = self.song.as_mut().ok_or("no song is open")?;
        let target = path
            .or_else(|| song.path().map(Path::to_path_buf))
            .unwrap_or(default_path);
        song.save(&target, &asset_base)
            .map_err(|error| error.to_string())?;
        self.sync_dirty_mirror();
        self.editor.status_bar.show_message("Song saved");
        log::info!("Song saved to: {}", target.display());
        Ok(target)
    }

    /// Replace the song with a new, never-saved one.
    pub(super) fn new_song(&mut self) {
        match SongDocument::new_song() {
            Ok(song) => {
                self.song = Some(song);
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
        self.sync_dirty_mirror();
        match opened.notes.first() {
            Some(note) => self.editor.status_bar.show_error(note.clone()),
            None => self.editor.status_bar.show_message("Song opened"),
        }
        Ok(opened.notes)
    }

    pub(super) fn close_song(&mut self) {
        self.song = None;
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
            HostedWrite::SongSave { path } => {
                let path = path
                    .as_ref()
                    .map(|path| self.resolve_asset_path(Path::new(path)));
                let saved = self
                    .save_song(path)
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
