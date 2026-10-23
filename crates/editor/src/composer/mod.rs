//! The composer: the song document's panel — a transport, a piano roll, a keyboard-operable
//! note list, and a sidebar holding the song's file, its parts, instruments and bindings.
//!
//! Everything here is data and drawing. The host (`editor_integration`) owns the song, the
//! drag in flight and what each action does: a render returns [`ComposerAction`]s and never
//! touches the document, so every change still goes through `SongDocument::apply`, past
//! the host's one refusal for a play session. One renderer serves both states — while a
//! play session runs the host passes `read_only`, and every field draws its value instead
//! of its control.

pub mod keys;
pub mod layout;
pub mod note_list;
pub mod piano_roll;
pub mod sidebar;
pub mod sound_fonts;
pub mod state;
pub mod transport;

#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod piano_roll_tests;
#[cfg(test)]
mod sidebar_tests;
#[cfg(test)]
mod sound_font_row_tests;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod transport_tests;

use std::collections::BTreeSet;

use music_midi::{Instrument, InstrumentSource, SamplePlayback, TempoMap};

use crate::drag_drop::DragPayload;
use crate::song::{SongDocument, SongEdit};

/// The note types the host names, re-exported so it needs no song crate of its own.
pub use music_midi::{NoteId, NoteImage};

pub use keys::{resolve as resolve_composer_key, ComposerKeyAction, ComposerKeyRoute, Direction};
pub use layout::{ComposerLayout, ComposerMode, PianoRollGeometry};
pub use sound_fonts::{scan_sound_fonts, sound_font_exists, SoundFontScan};
pub use state::{ComposerFocus, ComposerState, ControlRequest, RollCursor, SidebarSection};

/// MIDI's largest delta time: a note may end no later, or no file could write the gap
/// before its release. The song's own edits refuse past it; a drag clamps to it instead,
/// so no frame of a drag can be refused.
pub const LAST_WRITABLE_TICK: u64 = 0x0FFF_FFFF;
/// MIDI's highest key.
pub const HIGHEST_KEY: u8 = 127;

/// A position in the roll under the pointer: a tick and a key, either outside the song's
/// range when the pointer is, so a drag's delta can be measured past the edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollPoint {
    pub tick: i64,
    pub key: i32,
}

/// Something the composer's render or keys ask the host to do.
#[derive(Debug, Clone, PartialEq)]
pub enum ComposerAction {
    /// Give an area the keyboard (a click in it).
    Focus(ComposerFocus),
    /// Select exactly these notes.
    Select(Vec<NoteId>),
    /// Begin dragging these notes from a press at `point`.
    BeginDrag { notes: Vec<NoteId>, point: RollPoint },
    /// One edit through the document.
    Edit(SongEdit),
    /// One frame of a number field's gesture — a scrub, a held arrow. The `first` frame
    /// seals whatever entry was open and starts the gesture's own; the rest continue it,
    /// so the gesture is one undo entry. The sidebar's walk decides which frame is which
    /// from the field's own focus, scrub and drawn state, so no ui-wide flag is read.
    FieldEdit { edit: SongEdit, first: bool },
    SetActiveTrack(usize),
    /// Open the song at this asset-relative path (a Songs row, a dropped `.mid`).
    OpenSong(String),
    /// Save to the path the field holds, as typed (the path field's Enter).
    SaveSongTo(String),
    /// Start the song preview from the roll's cursor, or stop it: the transport's Play
    /// while it is stopped, its Stop while it plays.
    PreviewPlayStop,
    /// Play the binding at this index of the sidecar's bindings, as its action would.
    PreviewTrigger(usize),
    /// Tell the user why something was not done, on the status bar.
    Refused(String),
}

/// Where a drag or a nudge would put notes, drawn by the roll while the document is
/// untouched: every note in `notes` moved by both deltas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MovePreview {
    pub notes: BTreeSet<NoteId>,
    pub tick_delta: i64,
    pub key_delta: i32,
}

impl MovePreview {
    /// Where `image` is drawn: moved when the preview holds it, else where it is.
    pub fn place(&self, image: &NoteImage) -> (u64, u8) {
        if !self.notes.contains(&image.id) {
            return (image.tick, image.key);
        }
        let tick = image.tick.saturating_add_signed(self.tick_delta);
        let key = (i32::from(image.key) + self.key_delta).clamp(0, i32::from(HIGHEST_KEY)) as u8;
        (tick, key)
    }
}

/// `tick_delta` and `key_delta` clamped so that every one of `images` starts at tick 0 or
/// later, ends by [`LAST_WRITABLE_TICK`] and keeps a key from 0 to 127. The tightest note
/// bounds the whole set, so a chord keeps its intervals.
pub fn clamp_move(images: &[NoteImage], tick_delta: i64, key_delta: i32) -> (i64, i32) {
    let earliest_tick = images.iter().map(|image| image.tick).min().unwrap_or(0);
    let latest_end = images
        .iter()
        .map(|image| image.tick.saturating_add(image.duration_ticks))
        .max()
        .unwrap_or(0);
    let lowest_key = images.iter().map(|image| image.key).min().unwrap_or(0);
    let highest_key = images.iter().map(|image| image.key).max().unwrap_or(HIGHEST_KEY);
    let earliest_delta = i64::try_from(earliest_tick).map_or(i64::MIN, |tick| -tick);
    let latest_delta = i64::try_from(LAST_WRITABLE_TICK.saturating_sub(latest_end)).unwrap_or(i64::MAX);
    let tick_delta = tick_delta.clamp(earliest_delta, latest_delta.max(earliest_delta));
    let key_delta = key_delta.clamp(
        -i32::from(lowest_key),
        i32::from(HIGHEST_KEY) - i32::from(highest_key),
    );
    (tick_delta, key_delta)
}

/// The one edit a drag or a nudge commits: a `Batch` of the move in time, then one pitch
/// change per distinct starting key in key order — or `None` when both deltas are zero.
/// The deltas must already be clamped ([`clamp_move`]).
pub fn moved_notes_edit(images: &[NoteImage], tick_delta: i64, key_delta: i32) -> Option<SongEdit> {
    if images.is_empty() || (tick_delta == 0 && key_delta == 0) {
        return None;
    }
    let mut members = Vec::new();
    if tick_delta != 0 {
        members.push(SongEdit::MoveNotes {
            notes: images.iter().map(|image| image.id.number()).collect(),
            tick_delta,
        });
    }
    if key_delta != 0 {
        let starting_keys: BTreeSet<u8> = images.iter().map(|image| image.key).collect();
        for key in starting_keys {
            let moved = (i32::from(key) + key_delta).clamp(0, i32::from(HIGHEST_KEY)) as u8;
            members.push(SongEdit::SetPitch {
                notes: images
                    .iter()
                    .filter(|image| image.key == key)
                    .map(|image| image.id.number())
                    .collect(),
                key: moved,
            });
        }
    }
    Some(SongEdit::Batch(members))
}

/// The notes of `track`, by tick then key: the note list's rows, and what Ctrl+A selects.
pub fn track_notes(song: &SongDocument, track: usize) -> Vec<NoteImage> {
    let Some(song_track) = song.data().tracks().get(track) else {
        return Vec::new();
    };
    let mut images: Vec<NoteImage> = song_track
        .units()
        .iter()
        .flat_map(|unit| &unit.notes)
        .filter_map(|note| song.data().note_image(note.id))
        .collect();
    images.sort_by_key(|image| (image.tick, image.key, image.id));
    images
}

/// The images of `notes` the song still holds.
pub fn note_images(song: &SongDocument, notes: impl IntoIterator<Item = NoteId>) -> Vec<NoteImage> {
    notes
        .into_iter()
        .filter_map(|note| song.data().note_image(note))
        .collect()
}

/// A key's name in scientific pitch, middle C (60) being C4.
pub fn note_name(key: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = i32::from(key / 12) - 1;
    format!("{}{octave}", NAMES[usize::from(key % 12)])
}

/// Where a tick falls, as a musician counts it: "bar 1 · beat 1", both from one.
pub fn position_text(tempo_map: &TempoMap, tick: u64) -> String {
    let position = tempo_map.position_at_tick(tick);
    format!("bar {} · beat {}", position.bar + 1, position.beat_in_bar + 1)
}

/// A track as the selector and the sidebar name it: its index (what the API's `AddNote`
/// takes) and its name, when it has one.
pub fn track_label(song: &SongDocument, track: usize) -> String {
    match song.data().tracks().get(track).and_then(|song_track| song_track.name()) {
        Some(name) => format!("Track {track} · {name}"),
        None => format!("Track {track}"),
    }
}

/// The name a new part or instrument takes: `base`, else `base_2`, `base_3` … — the
/// sidecar refuses a repeated name.
pub fn unique_name<'a>(base: &str, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    let free = |candidate: &str| !taken.clone().any(|name| name == candidate);
    if free(base) {
        return base.to_string();
    }
    (2..)
        .map(|number| format!("{base}_{number}"))
        .find(|candidate| free(candidate))
        .unwrap_or_else(|| base.to_string())
}

/// What a payload dropped on the composer asks for. A `.wav` becomes a one-shot sample
/// instrument rooted on middle C at gain 1.0, named by its file's stem (suffixed while the
/// song has that name); a SoundFont an instrument on its bank 0, preset 0; a song is
/// opened. Any other recorded sound is refused naming WAV — the sample player pitches
/// WAV only — as is anything else, and a sample or a SoundFont with no song open.
pub fn drop_action(song: Option<&SongDocument>, payload: &DragPayload) -> ComposerAction {
    let source = match payload {
        DragPayload::Song { path } => return ComposerAction::OpenSong(path.clone()),
        DragPayload::Sound { path } if is_wav(path) => InstrumentSource::Sample {
            path: path.clone(),
            root_note: 60,
            gain: 1.0,
            playback: SamplePlayback::OneShot,
        },
        DragPayload::Sound { path } => {
            return ComposerAction::Refused(format!(
                "{path} is not a WAV: a sample instrument is pitched from a .wav file"
            ))
        }
        DragPayload::SoundFont { path } => InstrumentSource::SoundFont {
            path: path.clone(),
            bank: 0,
            preset: 0,
        },
        DragPayload::Texture { .. } | DragPayload::Script { .. } => {
            return ComposerAction::Refused(
                "The composer takes a .wav, a .sf2 or a .mid from Assets".to_string(),
            )
        }
    };
    let Some(song) = song else {
        return ComposerAction::Refused("Create or open a song first (File → New Song)".to_string());
    };
    let path = match &source {
        InstrumentSource::Sample { path, .. } | InstrumentSource::SoundFont { path, .. } => path,
    };
    let stem = std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "instrument".to_string());
    let name = unique_name(&stem, song.file().instruments.iter().map(|instrument| instrument.name.as_str()));
    ComposerAction::Edit(SongEdit::AddInstrument {
        instrument: Instrument {
            name,
            source,
            provenance: None,
        },
    })
}

fn is_wav(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}
