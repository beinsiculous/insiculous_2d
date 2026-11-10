//! The Conductor panel's song: a second document beside the scene, with its own history and save
//! watermark.
//!
//! A [`SongDocument`] holds the sidecar ([`SongFile`]) it saves as `<stem>.mid.ron` — the
//! map — and the MIDI data ([`SongData`]) of the `<stem>.mid` it maps. The score is written
//! in MIDI Evil, which exports the `.mid`; the engine never edits it. Every change to the
//! map goes through [`SongDocument::apply`] — the GUI's and the command API's alike — and
//! lands on the song's [`SongHistory`], never the scene's. A song is dirty when its history
//! moved past the save watermark, when it has never been saved at all, or when its sidecar
//! is not on disk yet. Saving and opening are [`io`].
//!
//! The document follows the score MIDI Evil owns: it keeps the `.mid` exactly as it read it
//! and the fingerprints of both files, so a save of the map leaves the `.mid` alone, and
//! [`SongDocument::adopt_score`] and [`SongDocument::reload_map`] take either file anew
//! without losing the other's unsaved edits (`follow.rs`).

mod change;
mod edit;
mod follow;
mod history;
mod io;

#[cfg(test)]
mod edit_tests;
#[cfg(test)]
mod folder_tests;
#[cfg(test)]
mod follow_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod io_tests;
#[cfg(test)]
mod map_save_tests;
#[cfg(test)]
mod open_tests;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod track_edit_tests;
#[cfg(test)]
mod test_support;

use std::path::{Path, PathBuf};

use music_midi::{SongData, SongError, SongFile, SONG_FILE_VERSION};

pub use edit::{SongEdit, SongEditError};
pub use history::{Gesture, SongHistory};
pub use io::{
    init_song, midi_fingerprint, remove_from_license, rename_in_license, sample_directory, saveable_song_path, OpenedSong, SaveIntent,
    SidecarIntent, SongInitError, SongOpenError, SongSaveError, HOME_DIRECTORIES,
};
/// The author and license `SongEdit::SetProvenance` carries, re-exported so a host names it
/// with no song crate of its own.
pub use music_midi::Provenance;

/// The SoundFont a sidecar-less song plays its background on when the project has no bank
/// of its own to offer, by its path under the asset folder.
pub const DEFAULT_SOUND_FONT_PATH: &str = "soundfonts/cast.sf2";

/// See the module docs.
#[derive(Debug, Clone)]
pub struct SongDocument {
    file: SongFile,
    data: SongData,
    history: SongHistory,
    /// Where the song saves; `None` until its first save into a song folder.
    path: Option<PathBuf>,
    /// The file it was imported from, which no save ever writes.
    imported_from: Option<PathBuf>,
    /// The `.mid` exactly as it was read: what a map save fingerprints and a Save As copies.
    source_bytes: Vec<u8>,
    /// `midi_fingerprint` of `source_bytes`: the score the document follows.
    score_fingerprint: u64,
    /// The sidecar's fingerprint as last read or written; `None` while no sidecar of this
    /// song is on disk.
    map_fingerprint: Option<u64>,
}

impl SongDocument {
    /// A song read from MIDI bytes with `file` as its sidecar, never saved.
    pub fn from_smf(bytes: &[u8], file: SongFile) -> Result<Self, SongError> {
        let (data, _) = SongData::from_smf(bytes)?;
        Ok(Self {
            file,
            data,
            history: SongHistory::new(),
            path: None,
            imported_from: None,
            source_bytes: bytes.to_vec(),
            score_fingerprint: midi_fingerprint(bytes),
            map_fingerprint: None,
        })
    }

    pub fn file(&self) -> &SongFile {
        &self.file
    }

    pub fn data(&self) -> &SongData {
        &self.data
    }

    pub fn history(&self) -> &SongHistory {
        &self.history
    }

    /// The `.mid` the song saves to, once it has one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Follow a successful file rename without changing the loaded bytes, fingerprints or history.
    pub fn follow_rename(&mut self, new_path: PathBuf) {
        self.path = Some(new_path);
    }

    /// The file the song was imported from; Save never writes it.
    pub fn imported_from(&self) -> Option<&Path> {
        self.imported_from.as_deref()
    }

    /// The `.mid` exactly as the document read it.
    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }

    /// The fingerprint of [`SongDocument::source_bytes`].
    pub fn score_fingerprint(&self) -> u64 {
        self.score_fingerprint
    }

    /// The sidecar's fingerprint as last read or written; `None` while it is not on disk.
    pub fn map_fingerprint(&self) -> Option<u64> {
        self.map_fingerprint
    }

    /// Whether the song has edits no save holds: its history past the watermark, no save
    /// at all, or a sidecar not yet on disk.
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty() || self.path.is_none() || self.map_fingerprint.is_none()
    }

    /// Apply one edit and record it — the one way the editor changes a song. A refused
    /// edit changes nothing and records nothing.
    pub fn apply(&mut self, edit: SongEdit, gesture: Gesture) -> Result<(), SongEditError> {
        let applied = edit.apply_to(&self.data, &mut self.file)?;
        self.history
            .record(edit, applied.undo, applied.redo, gesture);
        Ok(())
    }

    /// Undo the edit on top; `None` when there is none, else the undone edit's name.
    /// Either step ends the gesture on top: a continuing edit after it would otherwise
    /// merge into the older entry that surfaces, whose own gesture ended long ago.
    pub fn undo(&mut self) -> Option<&'static str> {
        self.history.break_merge();
        let entry = self.history.pop_undo()?;
        entry.undo.apply(&mut self.file);
        let name = entry.edit.name();
        self.history.push_undone(entry);
        Some(name)
    }

    /// Redo the last undone edit; `None` when there is none, else its name.
    pub fn redo(&mut self) -> Option<&'static str> {
        self.history.break_merge();
        let entry = self.history.pop_redo()?;
        entry.redo.apply(&mut self.file);
        let name = entry.edit.name();
        self.history.push_redone(entry);
        Some(name)
    }

    /// End the gesture on top: the next continuing edit is an undo entry of its own.
    pub fn break_merge(&mut self) {
        self.history.break_merge();
    }
}

/// The bank a scaffold sidecar plays its background on: the project's first under
/// `soundfonts/`, or [`DEFAULT_SOUND_FONT_PATH`] when it has none — which the
/// Conductor panel then warns about.
pub fn first_sound_font(asset_base: &Path) -> String {
    crate::external_tools::scan_sound_fonts(asset_base)
        .banks
        .into_iter()
        .next()
        .unwrap_or_else(|| DEFAULT_SOUND_FONT_PATH.to_string())
}

/// The sidecar `song init` writes and an unmapped `.mid` opens with: no parts, instruments
/// or bindings, looping, its background on [`first_sound_font`].
pub fn scaffold_song_file(asset_base: &Path) -> SongFile {
    SongFile {
        soundfont: first_sound_font(asset_base),
        ..default_song_file()
    }
}

/// A sidecar with no parts, instruments or bindings, playing its background on
/// [`DEFAULT_SOUND_FONT_PATH`].
pub fn default_song_file() -> SongFile {
    SongFile {
        version: SONG_FILE_VERSION,
        soundfont: DEFAULT_SOUND_FONT_PATH.to_string(),
        looping: true,
        parts: Vec::new(),
        instruments: Vec::new(),
        bindings: Vec::new(),
        provenance: None,
        midi_fingerprint: None,
        resources: Default::default(),
    }
}
