//! A Standard MIDI File as owned, editable data.
//!
//! [`SongData::from_smf`] keeps every event of every track (a [`RetainedEvent`], absolute
//! ticks, original order) so the file can be written back out, and lays the notes out
//! as selectable [`SongTrack`]s of [`MusicalUnit`]s — simultaneous note-ons grouped, each
//! note under a [`NoteId`] that never changes and is never reused.
//!
//! **Selectable tracks.** In a type-1 file they are the file's own tracks, index for
//! index. A type-0 file keeps everything on one track, so it is normalized: one
//! selectable track per MIDI channel it uses, named `channel N` with N counted from 1 as
//! musicians count (General MIDI percussion is `channel 10`), while the original track
//! keeps the global tempo and meta events and every original byte of event data.
//!
//! **Editing.** The mutation helpers (`add_note`, `remove_notes`, `move_notes`,
//! `resize_notes`, `set_velocity`, `set_pitch`, `assign_part`, and the tempo and
//! time-signature setters) refuse a bad edit whole and return the before-images an
//! inverse edit needs; [`SongData::restore_notes`] applies them. [`SongData::write_smf`]
//! re-emits the retained events, leaving out the original note pair of every edited or
//! removed note, plus the live notes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use midly::{MetaMessage, TrackEventKind};

use crate::error::SongError;
use crate::tempo_map::{TempoChange, TempoMap, TimeSignature};

mod edit;
mod export;
mod flatten;
#[cfg(test)]
mod tests;

/// A note's identity, stable across every edit. The composer's selection and undo
/// address notes by it, never by a position in a vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(u64);

impl NoteId {
    /// The raw number, for a command API or a log line.
    pub fn number(self) -> u64 {
        self.0
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "note {}", self.0)
    }
}

/// One note: its key and velocity and how long it lasts. Its tick is its unit's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub id: NoteId,
    pub key: u8,
    pub velocity: u8,
    pub duration_ticks: u64,
}

/// The notes that start on one tick of one track — a chord, or a single note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicalUnit {
    pub tick: u64,
    pub notes: Vec<Note>,
}

/// A note to add: where it starts and what it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewNote {
    pub tick: u64,
    pub key: u8,
    pub velocity: u8,
    pub duration_ticks: u64,
}

/// A note's whole state — the before-image an edit returns. `original_pair` records that
/// the note was still written from its original MIDI events, so restoring the image
/// re-links them and an undone edit exports exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteImage {
    pub id: NoteId,
    pub track: usize,
    pub tick: u64,
    pub key: u8,
    pub velocity: u8,
    pub duration_ticks: u64,
    pub original_pair: bool,
}

/// A selectable track: what a part's [`TrackSelector`](crate::TrackSelector) addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongTrack {
    name: Option<String>,
    channel: Option<u8>,
    source_track: usize,
    units: Vec<MusicalUnit>,
}

impl SongTrack {
    /// The track's name: its MIDI track name in a type-1 file, `channel N` in a
    /// normalized type-0 file, `None` for an unnamed type-1 track.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The MIDI channel (0–15) its notes play on, or `None` for a track with no channel
    /// events, such as a type-1 file's tempo track.
    pub fn channel(&self) -> Option<u8> {
        self.channel
    }

    /// The index of the file's own track that holds these events.
    pub fn source_track(&self) -> usize {
        self.source_track
    }

    /// The notes, grouped by start tick, in tick order.
    pub fn units(&self) -> &[MusicalUnit] {
        &self.units
    }

    pub fn note_count(&self) -> usize {
        self.units.iter().map(|unit| unit.notes.len()).sum()
    }
}

/// The two SMF layouts this crate reads. Type 2 is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmfFormat {
    SingleTrack,
    Parallel,
}

/// One event of the original file, kept for export: its absolute tick, the event itself,
/// and the note it opens or closes, if it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedEvent {
    tick: u64,
    kind: TrackEventKind<'static>,
    /// The bytes of a SysEx, escape or text-carrying meta event; `kind` holds an empty
    /// slice in their place, because midly's owned form drops them.
    payload: Box<[u8]>,
    note: Option<NoteId>,
}

impl RetainedEvent {
    fn retain(tick: u64, kind: TrackEventKind<'_>) -> Self {
        let payload: Box<[u8]> = match kind {
            TrackEventKind::SysEx(bytes) | TrackEventKind::Escape(bytes) => bytes.into(),
            TrackEventKind::Meta(meta) => meta_payload(meta).unwrap_or_default().into(),
            TrackEventKind::Midi { .. } => Box::default(),
        };
        Self {
            tick,
            kind: kind.to_static(),
            payload,
            note: None,
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The event exactly as the file held it.
    pub fn kind(&self) -> TrackEventKind<'_> {
        let payload: &[u8] = &self.payload;
        match self.kind {
            TrackEventKind::SysEx(_) => TrackEventKind::SysEx(payload),
            TrackEventKind::Escape(_) => TrackEventKind::Escape(payload),
            TrackEventKind::Meta(meta) => TrackEventKind::Meta(with_meta_payload(meta, payload)),
            midi => midi,
        }
    }

    /// The note this event opens or closes; `None` for every other event, and for a
    /// note-off that closed nothing.
    pub fn note(&self) -> Option<NoteId> {
        self.note
    }
}

fn meta_payload(meta: MetaMessage<'_>) -> Option<&[u8]> {
    match meta {
        MetaMessage::Text(bytes)
        | MetaMessage::Copyright(bytes)
        | MetaMessage::TrackName(bytes)
        | MetaMessage::InstrumentName(bytes)
        | MetaMessage::Lyric(bytes)
        | MetaMessage::Marker(bytes)
        | MetaMessage::CuePoint(bytes)
        | MetaMessage::ProgramName(bytes)
        | MetaMessage::DeviceName(bytes)
        | MetaMessage::SequencerSpecific(bytes)
        | MetaMessage::Unknown(_, bytes) => Some(bytes),
        _ => None,
    }
}

fn with_meta_payload<'a>(meta: MetaMessage<'static>, payload: &'a [u8]) -> MetaMessage<'a> {
    match meta {
        MetaMessage::Text(_) => MetaMessage::Text(payload),
        MetaMessage::Copyright(_) => MetaMessage::Copyright(payload),
        MetaMessage::TrackName(_) => MetaMessage::TrackName(payload),
        MetaMessage::InstrumentName(_) => MetaMessage::InstrumentName(payload),
        MetaMessage::Lyric(_) => MetaMessage::Lyric(payload),
        MetaMessage::Marker(_) => MetaMessage::Marker(payload),
        MetaMessage::CuePoint(_) => MetaMessage::CuePoint(payload),
        MetaMessage::ProgramName(_) => MetaMessage::ProgramName(payload),
        MetaMessage::DeviceName(_) => MetaMessage::DeviceName(payload),
        MetaMessage::SequencerSpecific(_) => MetaMessage::SequencerSpecific(payload),
        MetaMessage::Unknown(kind, _) => MetaMessage::Unknown(kind, payload),
        other => other,
    }
}

/// What [`SongData::from_smf`] read but the composer cannot edit, and what it shows
/// without refusing. Nothing here is dropped from playback or export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnsupportedReport {
    /// Reasons the composer must refuse to edit the file.
    pub unsupported: Vec<UnsupportedFeature>,
    /// A note started on a key its channel was already holding — informational.
    pub same_key_overlaps: Vec<SameKeyOverlap>,
}

impl UnsupportedReport {
    /// Whether the composer may edit the file.
    pub fn is_editable(&self) -> bool {
        self.unsupported.is_empty()
    }
}

/// Something the file holds that the editing model cannot represent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsupportedFeature {
    /// A type-1 track whose notes play on more than one channel: an added or edited note
    /// would have no single channel to be written on.
    MixedChannelTrack { track: usize, channels: Vec<u8> },
}

impl fmt::Display for UnsupportedFeature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnsupportedFeature::MixedChannelTrack { track, channels } => write!(
                formatter,
                "track {track} plays notes on channels {channels:?}; the composer edits one channel per track"
            ),
        }
    }
}

/// A note that started while the same key was already sounding on its channel: the
/// second note-on's selectable track, channel, key and tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SameKeyOverlap {
    pub track: usize,
    pub channel: u8,
    pub key: u8,
    pub tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NoteLocation {
    track: usize,
    tick: u64,
}

/// A MIDI file's events, notes and timing, owned and editable. See the module docs.
#[derive(Debug, Clone)]
pub struct SongData {
    format: SmfFormat,
    ticks_per_quarter: u16,
    retained_tracks: Vec<Vec<RetainedEvent>>,
    tracks: Vec<SongTrack>,
    tempo_changes: Vec<TempoChange>,
    time_signatures: Vec<TimeSignature>,
    /// Notes still written from their original events; every other note is written live.
    original_notes: BTreeSet<NoteId>,
    note_locations: BTreeMap<NoteId, NoteLocation>,
    tempo_edited: bool,
    time_signatures_edited: bool,
    next_note_id: u64,
}

impl SongData {
    /// Read a Standard MIDI File of type 0 or 1 with a ticks-per-quarter division.
    ///
    /// Refuses a file over [`MIDI_BYTE_LIMIT`](crate::MIDI_BYTE_LIMIT) before parsing and
    /// one over [`MIDI_EVENT_LIMIT`](crate::MIDI_EVENT_LIMIT) while parsing, type 2,
    /// SMPTE division, a zero tempo and a time signature that cannot divide the quarter
    /// note. The report names what the composer cannot edit.
    pub fn from_smf(bytes: &[u8]) -> Result<(SongData, UnsupportedReport), SongError> {
        flatten::flatten(bytes)
    }

    pub fn format(&self) -> SmfFormat {
        self.format
    }

    pub fn ticks_per_quarter(&self) -> u16 {
        self.ticks_per_quarter
    }

    /// The selectable tracks a part's selector addresses, in index order.
    pub fn tracks(&self) -> &[SongTrack] {
        &self.tracks
    }

    /// Every track of the original file, its events in their original order.
    pub fn retained_tracks(&self) -> &[Vec<RetainedEvent>] {
        &self.retained_tracks
    }

    /// The tempo changes in tick order, one per tick. The 120 BPM default is implied
    /// before the first, not listed.
    pub fn tempo_changes(&self) -> &[TempoChange] {
        &self.tempo_changes
    }

    /// The time signatures in tick order, one per tick. The 4/4 default is implied
    /// before the first, not listed.
    pub fn time_signatures(&self) -> &[TimeSignature] {
        &self.time_signatures
    }

    /// The song's timing as it stands now, edits included.
    pub fn tempo_map(&self) -> TempoMap {
        TempoMap::new(
            self.ticks_per_quarter,
            &self.tempo_changes,
            &self.time_signatures,
        )
    }

    /// Every note in every track.
    pub fn note_count(&self) -> usize {
        self.note_locations.len()
    }

    /// A note's whole state, or `None` if no note has this id.
    pub fn note_image(&self, id: NoteId) -> Option<NoteImage> {
        let location = self.note_locations.get(&id)?;
        let note = self.find_note(id, *location)?;
        Some(NoteImage {
            id,
            track: location.track,
            tick: location.tick,
            key: note.key,
            velocity: note.velocity,
            duration_ticks: note.duration_ticks,
            original_pair: self.original_notes.contains(&id),
        })
    }

    fn find_note(&self, id: NoteId, location: NoteLocation) -> Option<&Note> {
        let units = &self.tracks.get(location.track)?.units;
        let index = units
            .binary_search_by_key(&location.tick, |unit| unit.tick)
            .ok()?;
        units[index].notes.iter().find(|note| note.id == id)
    }
}
