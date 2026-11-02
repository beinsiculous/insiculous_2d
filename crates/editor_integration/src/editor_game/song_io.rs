//! The song's hosted half: new, open, save, close and init; the `song
//! new|open|init|save|close` API verbs that reach them; and the preview's `song
//! play|stop|seek|trigger`, which push the preview's intents (`song_preview.rs`).
//!
//! A song is a second document beside the scene. Its writes are refused for the whole play
//! session, and New, Open and Close never silently drop a dirty song: the GUI asks through
//! the confirm machine (`scene_confirm.rs`), and the API, which has no dialog, refuses unless
//! the line ends in `discard`. A save over a sidecar changed on disk since the song read it
//! asks Overwrite / Cancel in the GUI, and the API answers it with a structured refusal
//! naming both files that `song save <path> overwrite` passes. A preview verb is a song
//! write too, so the play-session and scene-batch refusal answers it first.

use std::fmt;
use std::path::{Path, PathBuf};

use editor::command_api::{song_write_refusal, ApiError, ChangedFile, HostedWrite};
use editor::console::Severity;
use editor::song::{SaveIntent, SongSaveError};
use editor::SongDocument;
use engine_core::Game;

use super::scene_confirm::{OverwriteCause, PendingSceneAction};
use super::EditorGame;

/// Why a host save landed nothing: the host's own refusal, or the save's.
pub(super) enum SongWriteError {
    Refused(String),
    Save(SongSaveError),
}

impl fmt::Display for SongWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SongWriteError::Refused(reason) => formatter.write_str(reason),
            SongWriteError::Save(error) => error.fmt(formatter),
        }
    }
}

impl<G: Game> EditorGame<G> {
    /// `Some(message)` while a play session runs: the song cannot change under it.
    pub(super) fn song_change_refusal(&self) -> Option<&'static str> {
        self.editor
            .in_play_session()
            .then_some("the song cannot change during a play session — stop Play first")
    }

    /// Whether the song's saved file exists, so a prompt may offer to play it.
    pub(super) fn song_saved_exists(&self) -> bool {
        // Both files: an unmapped `.mid` opened in place has a path and no sidecar yet, and
        // a Play of it would ask the loader for a sidecar that is not there.
        self.song.as_ref().and_then(|song| song.path()).is_some_and(|path| {
            common::vfs::file_size(path).is_ok()
                && common::vfs::file_size(&editor::external_tools::midi_sidecar_path(path)).is_ok()
        })
    }

    /// Save the song to `path`, else to its own path, else — never saved — to
    /// [`Self::default_song_path`]. Returns where it saved, or `None` when the save was
    /// parked behind the overwrite prompt instead: `path` is another song's file, or the
    /// song's sidecar changed on disk since it was read. With no `path` it never parks for
    /// another song — the song's own file and the default's free name are no other song's.
    pub(super) fn save_song(&mut self, path: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
        if let Some(refusal) = self.song_change_refusal() {
            return Err(refusal.to_string());
        }
        if let Some(target) = path.as_ref().filter(|target| self.is_another_songs_file(target)) {
            self.scene_confirm.pending_action =
                Some(PendingSceneAction::OverwriteSong(target.clone(), OverwriteCause::AnotherSong));
            return Ok(None);
        }
        let target = self.song_save_target(path)?;
        self.save_song_to(target, SaveIntent::default())
    }

    /// Save the song to exactly `target`, whatever song is there: the overwrite prompt's
    /// Overwrite and the API's `overwrite` come here past the check `save_song` makes. A
    /// sidecar changed on disk since the song read it parks the Overwrite / Cancel prompt
    /// (`Ok(None)`) unless `intent` overwrites it.
    pub(super) fn save_song_to(&mut self, target: PathBuf, intent: SaveIntent) -> Result<Option<PathBuf>, String> {
        match self.write_song(&target, intent) {
            Ok(_) => Ok(Some(target)),
            Err(SongWriteError::Save(SongSaveError::SidecarChanged { .. })) => {
                self.scene_confirm.pending_action =
                    Some(PendingSceneAction::OverwriteSong(target, OverwriteCause::MapChanged));
                Ok(None)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Where a save with this `path` goes: the path, else the song's own, else the default.
    fn song_save_target(&self, path: Option<PathBuf>) -> Result<PathBuf, String> {
        let own_path = self.song.as_ref().ok_or("no song is open")?.path().map(Path::to_path_buf);
        Ok(path.or(own_path).unwrap_or_else(|| self.default_song_path()))
    }

    /// The save itself, refused during a play session or with no song: the status bar says
    /// it saved, and what the save settled — a score adopted, a recovery kept — goes to the
    /// bar and the console, the first line on the bar. Returns those lines.
    pub(super) fn write_song(&mut self, target: &Path, intent: SaveIntent) -> Result<Vec<String>, SongWriteError> {
        if let Some(refusal) = self.song_change_refusal() {
            return Err(SongWriteError::Refused(refusal.to_string()));
        }
        let asset_base = self.asset_base.clone();
        let song = self
            .song
            .as_mut()
            .ok_or_else(|| SongWriteError::Refused("no song is open".to_string()))?;
        let notes = song.save_with(target, &asset_base, intent).map_err(SongWriteError::Save)?;
        self.sync_dirty_mirror();
        self.editor.composer.rescan_songs(&self.asset_base);
        let shown = self.asset_relative(target);
        self.editor.status_bar.show_message(format!("Song saved · {shown}"));
        log::info!("Song saved to: {}", target.display());
        if let Some((first, rest)) = notes.split_first() {
            self.report(Severity::Info, first.clone(), None);
            for note in rest {
                self.editor.console.push(Severity::Info, note.clone(), None);
            }
        }
        Ok(notes)
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
    /// A new song, its background on the project's first bank under `soundfonts/` — the
    /// default path, which the composer then warns about, when it has none.
    pub(super) fn new_song(&mut self) {
        match SongDocument::new_song_on(&editor::song::first_sound_font(&self.asset_base)) {
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

    /// Open the song at `path` — the open song's own path included, which reads both its
    /// files anew — keeping what an interrupted save left; returns what the open settled.
    pub(super) fn open_song(&mut self, path: &Path) -> Result<Vec<String>, String> {
        let opened =
            SongDocument::open(path, &self.asset_base).map_err(|error| error.to_string())?;
        self.song = Some(opened.document);
        self.song_generation += 1;
        self.follow_song();
        self.editor.composer.rescan_songs(&self.asset_base);
        self.sync_dirty_mirror();
        // The bar holds the first note; the console keeps them all, the diff's lines with them.
        for note in &opened.notes {
            self.editor.console.push(Severity::Info, note.clone(), None);
        }
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
            HostedWrite::SongInit { path } => {
                let path = self.resolve_asset_path(Path::new(path));
                let sidecar = editor::song::init_song(&path, &self.asset_base)
                    .map_err(|error| ApiError::Refused(format!("song init failed: {error}")))?;
                self.editor.composer.rescan_songs(&self.asset_base);
                Ok(serde_json::json!({ "initialized": sidecar.display().to_string() }))
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
                let target = self.song_save_target(path).map_err(ApiError::Refused)?;
                let intent = if *overwrite { SaveIntent::OVERWRITE } else { SaveIntent::default() };
                let notes = self.write_song(&target, intent).map_err(save_refusal)?;
                Ok(serde_json::json!({ "saved": target.display().to_string(), "notes": notes }))
            }
            HostedWrite::SongClose { discard } => {
                if dirty && !discard {
                    return Err(dirty_refusal("close"));
                }
                self.close_song();
                Ok(serde_json::json!({ "song": "closed" }))
            }
            HostedWrite::SongPlay { from_beat } => {
                let from_tick = match (from_beat, self.song.as_ref()) {
                    (Some(beat), Some(song)) => song.data().tempo_map().tick_at_beat(*beat),
                    _ => 0,
                };
                self.preview_play(from_tick).map_err(ApiError::Refused)?;
                Ok(serde_json::json!({ "preview": "playing", "from_tick": from_tick }))
            }
            HostedWrite::SongStop => {
                self.preview_stop();
                Ok(serde_json::json!({ "preview": "stopped" }))
            }
            HostedWrite::SongSeek { beat } => {
                self.preview_seek(*beat).map_err(ApiError::Refused)?;
                Ok(serde_json::json!({ "preview": "playing", "beat": beat }))
            }
            HostedWrite::SongTrigger { binding } => {
                let bound = self
                    .song
                    .as_ref()
                    .is_some_and(|song| song.file().bindings.iter().any(|known| known.action == *binding));
                if !bound {
                    return Err(ApiError::NotFound(format!("no binding has the action '{binding}'")));
                }
                self.preview_trigger(binding).map_err(ApiError::Refused)?;
                Ok(serde_json::json!({ "triggered": binding }))
            }
            other => Err(ApiError::Invalid(format!(
                "{other:?} is not a hosted song write"
            ))),
        }
    }
}

/// The API's answer to a save that landed nothing: a sidecar changed on disk is refused
/// naming both files and each one's fingerprints, as read and as on disk.
fn save_refusal(error: SongWriteError) -> ApiError {
    match error {
        SongWriteError::Save(SongSaveError::SidecarChanged {
            sidecar,
            loaded,
            on_disk,
            midi,
            score_loaded,
            score_on_disk,
        }) => ApiError::FilesChanged {
            message: format!(
                "the map changed on disk since you loaded it: {} — `song save {} overwrite` replaces it \
                 with this song's map, `song open {}` reads it",
                sidecar.display(),
                midi.display(),
                midi.display()
            ),
            files: vec![
                ChangedFile::new(&midi, Some(score_loaded), score_on_disk),
                ChangedFile::new(&sidecar, loaded, on_disk),
            ],
        },
        other => ApiError::Refused(format!("song save failed: {other}")),
    }
}

/// Whether a song's MIDI file or its sidecar (`midi_sidecar_path`) exists at `target`.
pub(super) fn song_files_exist(target: &Path) -> bool {
    let sidecar = editor::external_tools::midi_sidecar_path(target);
    common::vfs::file_size(target).is_ok() || common::vfs::file_size(&sidecar).is_ok()
}
