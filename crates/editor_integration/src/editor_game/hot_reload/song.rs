//! `ReloadSong`: the song files the editor watches and a change to them, dispatched by what
//! named them. The open document follows its two files — a new score adopted, Editing and
//! Play alike; a new map reloaded when the document is clean, asked about when it is not. A
//! play session's song, played from its saved file, is replaced at its next bar line when its
//! files or its closure change, the track diff refusing first a score the sidecar can no
//! longer play. The web, which has no watcher, files an import of the open song in a mailbox
//! the frame drains into the same adoption, and what became of each replacement reaches the
//! status bar and the console.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use audio::{AudioManager, SongReplacement};
use editor::console::Severity;
use editor::external_tools::{
    diff_tracks, is_midi_path, midi_sidecar_path, read_song_pair, sidecar_midi_path, TrackDiff, TrackListing,
};
use editor::SongDocument;
use engine_core::Game;

use super::super::run_options::SongImport;
use super::super::scene_confirm::PendingSceneAction;
use super::super::EditorGame;
use super::WatchOwner;

/// Which song a watched path belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::editor_game) enum SongOwner {
    /// The song open in the Conductor panel: its `.mid` and its sidecar.
    Document,
    /// The song a play session plays from its saved file: its `.mid`, its sidecar, and every
    /// SoundFont and sample its preparations read or tried to.
    Session,
}

/// The song changes one drain collected, so each owner is dispatched once: a `.mid` and its
/// sidecar settle in the same poll and arrive as two changes.
#[derive(Debug, Default)]
pub(in crate::editor_game) struct SongChanges {
    /// The open document's `.mid`, as the watched set names it.
    document_score: Option<PathBuf>,
    /// The open document's sidecar, as the watched set names it.
    document_map: Option<PathBuf>,
    session_pair: bool,
    session_closure: bool,
}

impl SongChanges {
    /// Take `path`'s change for each song among `owners`; any other owner is not this one's.
    pub(in crate::editor_game) fn collect(&mut self, path: &Path, owners: &BTreeSet<WatchOwner>) {
        let is_score = is_midi_path(path);
        for owner in owners {
            match owner {
                WatchOwner::Song(SongOwner::Document) if is_score => self.document_score = Some(path.to_path_buf()),
                WatchOwner::Song(SongOwner::Document) => self.document_map = Some(path.to_path_buf()),
                WatchOwner::Song(SongOwner::Session) if is_score || sidecar_midi_path(path).is_some() => {
                    self.session_pair = true;
                }
                WatchOwner::Song(SongOwner::Session) => self.session_closure = true,
                _ => {}
            }
        }
    }
}

/// A diff's lines with their severities, refusals first.
fn diff_lines(diff: &TrackDiff) -> Vec<(Severity, String)> {
    diff.lines.iter().map(|line| (line.severity, line.text.clone())).collect()
}

impl<G: Game> EditorGame<G> {
    /// Add the songs to `watched`: the open document's `.mid` and sidecar, and — while a
    /// session runs — its song's two files and closure.
    pub(in crate::editor_game) fn watch_songs(&self, watched: &mut BTreeMap<PathBuf, BTreeSet<WatchOwner>>, audio: &AudioManager) {
        if let Some(midi) = self.song.as_ref().and_then(SongDocument::path) {
            let midi = self.resolve_asset_path(midi);
            for path in [midi_sidecar_path(&midi), midi] {
                watched.entry(path).or_default().insert(WatchOwner::Song(SongOwner::Document));
            }
        }
        if !self.editor.in_play_session() {
            return;
        }
        if let Some(midi) = audio.song_request() {
            let midi = self.resolve_asset_path(midi);
            for path in [midi_sidecar_path(&midi), midi] {
                watched.entry(path).or_default().insert(WatchOwner::Song(SongOwner::Session));
            }
        }
        for path in audio.song_closure() {
            watched.entry(self.resolve_asset_path(&path)).or_default().insert(WatchOwner::Song(SongOwner::Session));
        }
    }

    /// The song paths known to be missing, so a file that arrives is a change whenever it
    /// arrives: in a session, the file the latest preparation could not find — the loader's
    /// own knowledge, never a look at the disk, which would miss a file that arrived in the
    /// half second before the tick, and a mark kept from an earlier tick would outlive the
    /// arrival — and the open song's sidecar while an unmapped `.mid` is open in place. The
    /// frame stats nothing.
    pub(in crate::editor_game) fn absent_song_paths(&self, audio: &AudioManager) -> BTreeSet<PathBuf> {
        let mut absent = BTreeSet::new();
        if let Some(song) = self.song.as_ref().filter(|song| song.map_fingerprint().is_none()) {
            if let Some(midi) = song.path() {
                absent.insert(self.resolve_asset_path(&midi_sidecar_path(midi)));
            }
        }
        if self.editor.in_play_session() {
            absent.extend(audio.song_missing_paths().iter().map(|path| self.resolve_asset_path(path)));
        }
        absent
    }

    /// After a successful song save to `midi`: its sidecar — and its `.mid` when the save
    /// wrote it — become the watcher's baseline, and a map change waiting for the song, or
    /// its prompt, is dropped, since the sidecar now holds the editor's own map.
    pub(in crate::editor_game) fn note_song_written(&mut self, midi: &Path, wrote_score: bool) {
        let midi = self.resolve_asset_path(midi);
        if self.hot_reload.waiting_song_map.as_deref() == Some(midi.as_path()) {
            self.hot_reload.waiting_song_map = None;
        }
        if matches!(&self.scene_confirm.pending_action, Some(PendingSceneAction::ReloadSongMap(parked)) if *parked == midi) {
            self.scene_confirm.pending_action = None;
            self.scene_confirm.pending_choice = None;
        }
        self.note_written(&midi_sidecar_path(&midi));
        if wrote_score {
            self.note_written(&midi);
        }
    }

    /// Dispatch one drain's song changes, each owner once: the document's map before its
    /// score, so a score that arrived with its map is diffed against the map it came with,
    /// then the session's song.
    pub(in crate::editor_game) fn dispatch_song_changes(&mut self, changes: SongChanges, audio: &mut AudioManager) {
        if let Some(sidecar) = changes.document_map {
            self.follow_document_map(&sidecar);
        }
        if let Some(midi) = changes.document_score.filter(|midi| self.is_open_song(midi)) {
            match common::vfs::read(&midi) {
                Ok(bytes) => self.adopt_score_bytes(&midi, bytes),
                Err(error) => {
                    let shown = self.asset_relative(&midi);
                    self.report(Severity::Error, format!("{shown} could not be read, the song keeps its score — {error}"), None);
                }
            }
        }
        if changes.session_pair || changes.session_closure {
            self.replace_session_song(audio, changes.session_pair);
        }
    }

    /// Whether `midi`, resolved, is the open document's `.mid`.
    fn is_open_song(&self, midi: &Path) -> bool {
        let open = self.song.as_ref().and_then(SongDocument::path);
        open.is_some_and(|open| self.resolve_asset_path(open) == self.resolve_asset_path(midi))
    }

    /// Take `bytes` as the open song's score: the track diff's lines reported, the first on
    /// the bar, a refusal at error severity. Bytes that do not read as MIDI change nothing,
    /// and bytes the document already holds — another program wrote the file back as it was
    /// — change nothing and stop no preview.
    fn adopt_score_bytes(&mut self, midi: &Path, bytes: Vec<u8>) {
        let shown = self.asset_relative(midi);
        let Some(song) = self.song.as_mut() else { return };
        if song.source_bytes() == bytes.as_slice() {
            return self.report(Severity::Info, format!("{shown} was written as the song holds it"), None);
        }
        let before = TrackListing::of(song.file(), song.data());
        if let Err(error) = song.adopt_score(bytes) {
            let message = format!("{shown} changed but does not read as MIDI, the song keeps its score — {error}");
            return self.report(Severity::Error, message, None);
        }
        let diff = diff_tracks(&before, &TrackListing::of(song.file(), song.data()));
        self.stop_preview_for_external_change();
        self.sync_dirty_mirror();
        let mut lines = diff_lines(&diff);
        if lines.is_empty() {
            lines.push((Severity::Info, format!("{shown}: the song follows the score on disk, its map kept")));
        }
        self.report_lines(lines);
    }

    /// The open document's sidecar changed: a clean document reloads it at once, a dirty one
    /// waits to ask.
    fn follow_document_map(&mut self, sidecar: &Path) {
        let Some(midi) = sidecar_midi_path(sidecar).filter(|midi| self.is_open_song(midi)) else { return };
        if !self.song.as_ref().is_some_and(SongDocument::is_dirty) {
            return self.reload_song_map(&midi);
        }
        if self.editor.in_play_session() {
            let shown = self.asset_relative(sidecar);
            self.report(Severity::Info, format!("{shown}: the song has unsaved edits, so its map is asked about when Play stops"), None);
        }
        self.hot_reload.waiting_song_map = Some(midi);
    }

    /// The map's change waiting for a dirty document, once nothing holds it: as the scene's
    /// waits, out of a play session, a gesture, a focused field and a dialog. A song no longer
    /// open drops it, and one saved clean since reloads without asking.
    pub(in crate::editor_game) fn try_waiting_song_map(&mut self, edit_field_focused: bool) {
        let Some(midi) = self.hot_reload.waiting_song_map.clone() else { return };
        if !self.is_open_song(&midi) {
            self.hot_reload.waiting_song_map = None;
            return;
        }
        if self.reload_is_held(edit_field_focused) {
            return;
        }
        self.hot_reload.waiting_song_map = None;
        if self.song.as_ref().is_some_and(SongDocument::is_dirty) {
            self.scene_confirm.pending_action = Some(PendingSceneAction::ReloadSongMap(midi));
        } else {
            self.reload_song_map(&midi);
        }
    }

    /// Read the sidecar of the song at `midi` anew, the score and its unsaved edits kept; a
    /// sidecar that does not read or parse leaves the map as it was and says why.
    pub(in crate::editor_game) fn reload_song_map(&mut self, midi: &Path) {
        let sidecar = midi_sidecar_path(midi);
        let shown = self.asset_relative(&sidecar);
        if !self.is_open_song(midi) {
            return self.report(Severity::Info, format!("{shown}: its song is no longer open, nothing reloads"), None);
        }
        let reloaded = common::vfs::read_to_string(&sidecar)
            .map_err(|error| error.to_string())
            .and_then(|text| match self.song.as_mut() {
                Some(song) => song.reload_map(&text).map_err(|error| error.to_string()),
                None => Ok(()),
            });
        match reloaded {
            Ok(()) => {
                self.stop_preview_for_external_change();
                self.sync_dirty_mirror();
                self.report(Severity::Info, format!("{shown}: map reloaded from disk"), None);
            }
            Err(error) => self.report(Severity::Error, format!("{shown} did not reload, the map is kept — {error}"), None),
        }
    }

    /// The session's song changed on disk: replaced at its next bar line. A change to its
    /// `.mid` or sidecar is diffed first — the playing song's parts against the new pair's —
    /// and a refusing diff keeps the old song playing, nothing sent; a SoundFont or a sample
    /// changed leaves the parts as they were and needs no diff.
    fn replace_session_song(&mut self, audio: &mut AudioManager, pair_changed: bool) {
        if !self.editor.in_play_session() {
            return;
        }
        let Some(midi) = audio.song_request().map(Path::to_path_buf) else { return };
        let shown = self.asset_relative(&midi);
        if pair_changed {
            let Some(playing) = audio.playing_song() else { return };
            let (file, data) = match read_song_pair(&midi) {
                Ok(pair) => pair,
                Err(error) => {
                    return self.report(Severity::Error, format!("{shown} did not reload, the song plays on — {error}"), None);
                }
            };
            let diff = diff_tracks(&TrackListing::of(playing.file(), playing.data()), &TrackListing::of(&file, &data));
            let refused = diff.refuses();
            self.report_lines(diff_lines(&diff));
            if refused {
                self.editor.console.push(Severity::Info, format!("{shown}: not replaced, the song plays on as it was"), None);
                return;
            }
        }
        if let Err(error) = audio.replace_song_at_seam(&midi) {
            self.report(Severity::Error, format!("{shown} did not reload, the song plays on — {error}"), None);
        }
    }

    /// Every replacement outcome since the last frame, through `report`: a swap names its bar
    /// as musicians count them, from one; a refusal and a failure are errors; a dropped
    /// preparation, superseded or stopped, says nothing.
    pub(in crate::editor_game) fn report_song_replacements(&mut self, audio: &mut AudioManager) {
        for outcome in audio.take_song_replacements() {
            match outcome {
                SongReplacement::Swapped { bar, .. } => {
                    self.report(Severity::Info, format!("song replaced in bar {}", bar + 1), None);
                }
                SongReplacement::Refused { reason } => {
                    self.report(Severity::Error, format!("the song was not replaced: {reason}"), None);
                }
                SongReplacement::Failed(error) => {
                    self.report(Severity::Error, format!("the song was not replaced, it plays on — {error}"), None);
                }
                SongReplacement::Dropped => {}
            }
        }
    }

    /// The frame's drain of the web's import mailbox, before the watcher's changes.
    pub(in crate::editor_game) fn drain_song_import(&mut self) {
        let import = self.song_import.as_ref().and_then(|mailbox| mailbox.lock().ok().and_then(|mut slot| slot.take()));
        if let Some(import) = import {
            self.adopt_song_import(import);
        }
    }

    /// An import a host wrote over a song's `.mid`: the open document adopts it as it adopts
    /// a change on disk, and one for a song no longer open is dropped with a line.
    pub(in crate::editor_game) fn adopt_song_import(&mut self, import: SongImport) {
        if !self.is_open_song(&import.path) {
            let shown = self.asset_relative(&import.path);
            return self.report(Severity::Info, format!("{shown} was imported, but it is no longer the open song"), None);
        }
        self.adopt_score_bytes(&import.path, import.bytes);
    }

    /// `lines` through `report`, the first on the bar and in the console, the rest in the
    /// console alone.
    fn report_lines(&mut self, lines: Vec<(Severity, String)>) {
        let mut lines = lines.into_iter();
        if let Some((severity, text)) = lines.next() {
            self.report(severity, text, None);
        }
        for (severity, text) in lines {
            self.editor.console.push(severity, text, None);
        }
    }
}
