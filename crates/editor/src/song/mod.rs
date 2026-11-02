//! The composer's song: a second document beside the scene, with its own history, save
//! watermark and selection.
//!
//! A [`SongDocument`] holds the sidecar ([`SongFile`]) and the MIDI data ([`SongData`]) it
//! saves as `<stem>.mid.ron` and `<stem>.mid`. Every change goes through
//! [`SongDocument::apply`] — the GUI's and the command API's alike — and lands on the
//! song's [`SongHistory`], never the scene's. A song is dirty when its history moved past
//! the save watermark, when it has never been saved at all, or when its sidecar is not on
//! disk yet. Saving and opening are the journal in [`io`].
//!
//! The document follows the score another program owns: it keeps the `.mid` exactly as it
//! read or last wrote it and the fingerprints of both files, so a save of the map leaves
//! the `.mid` alone, and [`SongDocument::adopt_score`] and [`SongDocument::reload_map`]
//! take either file anew without losing the other's unsaved edits (`follow.rs`).

mod change;
mod edit;
mod follow;
mod history;
mod io;
mod selection;

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

use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::{
    NoteId, SongData, SongError, SongFile, UnsupportedReport, DEFAULT_MICROSECONDS_PER_QUARTER,
    SONG_FILE_VERSION,
};

pub use edit::{Meter, SongEdit, SongEditError};
pub use history::{Gesture, SongHistory};
pub use io::{
    init_song, midi_fingerprint, sample_directory, saveable_song_path, OpenedSong, SaveIntent,
    SidecarIntent, SongInitError, SongOpenError, SongSaveError, HOME_DIRECTORIES,
};
pub use selection::SongSelection;
/// The author and license `SongEdit::SetProvenance` carries, re-exported so a host names it
/// with no song crate of its own.
pub use music_midi::Provenance;

/// The SoundFont a new or sidecar-less song plays its background on when the project has
/// no bank of its own to offer, by its path under the asset folder.
pub const DEFAULT_SOUND_FONT_PATH: &str = "soundfonts/cast.sf2";

/// The division of a new song, in ticks per quarter note.
const NEW_SONG_TICKS_PER_QUARTER: u16 = 480;

/// Whether the composer may edit a song.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditingPolicy {
    Full,
    /// The MIDI file holds something the editing model cannot represent, or its data would
    /// not write back as a MIDI file; every score edit is refused with this reason, while
    /// the map — parts, instruments, bindings, loop, quantize, modes, provenance — edits and
    /// saves as on any other song.
    Refused {
        reason: String,
    },
}

impl EditingPolicy {
    /// The policy for a score: refused with the report's features joined when any of them
    /// makes the file uneditable, refused when the data has no export to be told from
    /// (`exportable` false — nothing could write a score edit back), else full. The one
    /// place the rule lives, so what `editing()` reports is what `apply` enforces.
    pub fn for_score(report: &UnsupportedReport, exportable: bool) -> Self {
        if report.is_editable() {
            return if exportable {
                EditingPolicy::Full
            } else {
                EditingPolicy::Refused {
                    reason: "its score cannot be written back as a MIDI file".to_string(),
                }
            };
        }
        let reasons: Vec<String> = report
            .unsupported
            .iter()
            .map(|feature| feature.to_string())
            .collect();
        EditingPolicy::Refused {
            reason: reasons.join("; "),
        }
    }
}

/// See the module docs.
#[derive(Debug, Clone)]
pub struct SongDocument {
    file: SongFile,
    data: SongData,
    history: SongHistory,
    selection: SongSelection,
    /// Where the song saves; `None` until its first save into a song folder.
    path: Option<PathBuf>,
    /// The file it was imported from, which no save ever writes.
    imported_from: Option<PathBuf>,
    editing: EditingPolicy,
    /// The `.mid` exactly as it was last read or written: what a map save fingerprints and
    /// a Save As copies while the engine has not edited the score.
    source_bytes: Vec<u8>,
    /// `midi_fingerprint` of `source_bytes`: the score the document follows.
    score_fingerprint: u64,
    /// The sidecar's fingerprint as last read or written; `None` while no sidecar of this
    /// song is on disk.
    map_fingerprint: Option<u64>,
    /// The data's own export (`write_smf`) as the score was last loaded or written: the
    /// score is the engine's — a save writes `write_smf`'s bytes — exactly while the data's
    /// export now differs from this. Content, not the history: the history merges a
    /// gesture's frames into one entry and evicts its oldest past its cap, and an edit
    /// undone after a save followed by another edit nets to nothing in any count — while
    /// the bytes tell. Not the loaded bytes themselves: an unedited export may re-encode
    /// them. `None` when the data would not export: the score is then the file's for good —
    /// a save keeps the loaded bytes and refuses every score edit, since nothing could
    /// write one back.
    baseline_export: Option<Vec<u8>>,
}

impl SongDocument {
    /// An empty song: one tempo track and one melody track on channel 1, 120 BPM in 4/4,
    /// never saved.
    pub fn new_song() -> Result<Self, SongError> {
        Self::from_smf(&blank_song_bytes()?, default_song_file())
    }

    /// An empty song as [`SongDocument::new_song`] makes it, its background on
    /// `sound_font`, asset-relative.
    pub fn new_song_on(sound_font: &str) -> Result<Self, SongError> {
        let mut file = default_song_file();
        file.soundfont = sound_font.to_string();
        Self::from_smf(&blank_song_bytes()?, file)
    }

    /// A song read from MIDI bytes with `file` as its sidecar, never saved: the report's
    /// unsupported features decide the [`EditingPolicy`].
    pub fn from_smf(bytes: &[u8], file: SongFile) -> Result<Self, SongError> {
        let (data, report) = SongData::from_smf(bytes)?;
        let baseline_export = data.write_smf().ok();
        Ok(Self {
            file,
            data,
            history: SongHistory::new(),
            selection: SongSelection::new(),
            path: None,
            imported_from: None,
            editing: EditingPolicy::for_score(&report, baseline_export.is_some()),
            source_bytes: bytes.to_vec(),
            score_fingerprint: midi_fingerprint(bytes),
            map_fingerprint: None,
            baseline_export,
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

    pub fn selection(&self) -> &SongSelection {
        &self.selection
    }

    /// The `.mid` the song saves to, once it has one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file the song was imported from; Save never writes it.
    pub fn imported_from(&self) -> Option<&Path> {
        self.imported_from.as_deref()
    }

    pub fn editing(&self) -> &EditingPolicy {
        &self.editing
    }

    /// The `.mid` exactly as the document last read or wrote it.
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

    /// Whether the score differs from the bytes last loaded or written by an edit the
    /// engine made: the data's export against the export it had then. An export is a few
    /// milliseconds for the largest song the loader takes; a save, an adoption and a backup
    /// ask, a frame never does.
    pub fn is_score_authored(&self) -> bool {
        match (&self.baseline_export, self.data.write_smf()) {
            (Some(baseline), Ok(export)) => *baseline != export,
            (Some(_), Err(_)) => true,
            (None, _) => false,
        }
    }

    /// Whether the song has edits no save holds: its history past the watermark, no save
    /// at all, or a sidecar not yet on disk.
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty() || self.path.is_none() || self.map_fingerprint.is_none()
    }

    /// Apply one edit and record it — the one way the editor changes a song. A refused
    /// edit changes nothing and records nothing; under [`EditingPolicy::Refused`] only a
    /// score edit is refused.
    pub fn apply(&mut self, edit: SongEdit, gesture: Gesture) -> Result<(), SongEditError> {
        if let EditingPolicy::Refused { reason } = &self.editing {
            if edit.changes_score() {
                return Err(SongEditError::Refused {
                    reason: reason.clone(),
                });
            }
        }
        let applied = edit.apply_to(&mut self.data, &mut self.file)?;
        self.history
            .record(edit, applied.undo, applied.redo, gesture);
        self.selection.retain_existing(&self.data);
        Ok(())
    }

    /// Undo the edit on top; `Ok(None)` when there is none, else the undone edit's name.
    /// Either step ends the gesture on top: a continuing edit after it would otherwise
    /// merge into the older entry that surfaces, whose own gesture ended long ago.
    pub fn undo(&mut self) -> Result<Option<&'static str>, SongEditError> {
        self.history.break_merge();
        let Some(entry) = self.history.pop_undo() else {
            return Ok(None);
        };
        match entry.undo.apply(&mut self.data, &mut self.file) {
            Ok(()) => {
                let name = entry.edit.name();
                self.history.push_undone(entry);
                self.selection.retain_existing(&self.data);
                Ok(Some(name))
            }
            Err(error) => {
                self.history.restore_undo(entry);
                Err(error.into())
            }
        }
    }

    /// Redo the last undone edit; `Ok(None)` when there is none, else its name.
    pub fn redo(&mut self) -> Result<Option<&'static str>, SongEditError> {
        self.history.break_merge();
        let Some(entry) = self.history.pop_redo() else {
            return Ok(None);
        };
        match entry.redo.apply(&mut self.data, &mut self.file) {
            Ok(()) => {
                let name = entry.edit.name();
                self.history.push_redone(entry);
                self.selection.retain_existing(&self.data);
                Ok(Some(name))
            }
            Err(error) => {
                self.history.restore_redo(entry);
                Err(error.into())
            }
        }
    }

    /// End the gesture on top: the next continuing edit is an undo entry of its own.
    pub fn break_merge(&mut self) {
        self.history.break_merge();
    }

    /// Select exactly the numbered notes; an empty list clears the selection.
    pub fn select_notes(&mut self, numbers: &[u64]) -> Result<(), SongEditError> {
        let notes = if numbers.is_empty() {
            Vec::new()
        } else {
            edit::note_ids(&self.data, numbers)?
        };
        self.selection.replace(notes);
        Ok(())
    }

    /// Select exactly these notes by id, the composer's vocabulary; an id the song no longer
    /// holds is left out, and an empty set clears the selection.
    pub fn select(&mut self, notes: impl IntoIterator<Item = NoteId>) {
        let data = &self.data;
        self.selection.replace(
            notes
                .into_iter()
                .filter(|note| data.note_image(*note).is_some()),
        );
    }

    /// The note with this number, if the song holds it.
    pub fn note_id(&self, number: u64) -> Option<NoteId> {
        edit::note_ids(&self.data, &[number])
            .ok()
            .and_then(|ids| ids.first().copied())
    }
}

/// The bank a new sidecar plays its background on: the project's first under
/// `soundfonts/`, or [`DEFAULT_SOUND_FONT_PATH`] when it has none — which the composer then
/// warns about.
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

/// A type-1 file: a tempo track at 120 BPM in 4/4, and a track named `melody` whose
/// program change gives it channel 1 to write notes on.
fn blank_song_bytes() -> Result<Vec<u8>, SongError> {
    let event = |kind| TrackEvent {
        delta: u28::new(0),
        kind,
    };
    let tempo_track = vec![
        event(TrackEventKind::Meta(MetaMessage::Tempo(u24::new(
            DEFAULT_MICROSECONDS_PER_QUARTER,
        )))),
        event(TrackEventKind::Meta(MetaMessage::TimeSignature(
            4, 2, 24, 8,
        ))),
        event(TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let melody_track = vec![
        event(TrackEventKind::Meta(MetaMessage::TrackName(b"melody"))),
        event(TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::ProgramChange {
                program: u7::new(0),
            },
        }),
        event(TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let header = Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(NEW_SONG_TICKS_PER_QUARTER)),
    );
    let mut bytes = Vec::new();
    Smf {
        header,
        tracks: vec![tempo_track, melody_track],
    }
    .write_std(&mut bytes)
    .map_err(|error| SongError::WriteFailed {
        reason: error.to_string(),
    })?;
    Ok(bytes)
}
