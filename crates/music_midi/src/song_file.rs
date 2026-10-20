//! The `.mid.ron` song sidecar schema — the single source of truth for how a MIDI file
//! becomes music a game can play.
//!
//! A sidecar sits beside its MIDI file (`music/theme.mid` ↔ `music/theme.mid.ron`) and
//! holds three named collections:
//!
//! ```ron
//! SongFile(
//!     version: 1,
//!     soundfont: "soundfonts/conductor_cast.sf2",   // plays the background score
//!     looping: true,                                // omitted means true
//!     parts: [
//!         // A part reads one track's notes: by name, or by index when names repeat.
//!         (name: "melody", track: Name("flute"), mode: Loop, quantize: Eighth),
//!         (name: "drums", track: Index(2), mode: AtSongPosition),
//!     ],
//!     instruments: [
//!         (name: "trumpet", source: SoundFont(path: "soundfonts/conductor_cast.sf2", bank: 0, preset: 56)),
//!         (name: "blip", source: Sample(path: "music/samples/theme/blip.wav", root_note: 60, gain: 0.8, playback: OneShot)),
//!     ],
//!     bindings: [
//!         // A game action plays a part on an instrument; both may be shared.
//!         (action: "coin", part: "melody", instrument: "trumpet"),
//!         (action: "jump", part: "melody", instrument: "blip"),
//!     ],
//! )
//! ```
//!
//! Every track a part selects is a *trigger* track: it plays only when its bindings
//! fire, never in the background. Every other track is the background score, played with
//! its own program, channel and controller events. A binding's instrument always wins
//! over the track's own program.
//!
//! Authored data fails loud. [`parse_song_file`] rejects what the text alone shows
//! (an unknown version, unknown fields, empty or repeated names, a binding naming
//! nothing, a bad gain, note or preset number, too many instruments);
//! [`SongFile::validate`] adds what needs the MIDI data and the loaded resources (a
//! missing or ambiguous track, an empty trigger part, a note outside a sample's range,
//! a preset the SoundFont lacks or layers too thickly, an undecoded or out-of-range
//! sample). `docs/SONG_FORMAT.md` is the author's guide.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{SongFileError, SongFileProblem};
use crate::song_data::SongData;

#[cfg(test)]
mod tests;
mod validate;

/// Schema version this engine reads. Bump only for a breaking change, and only
/// alongside a migration story — an unknown version is a hard error.
pub const SONG_FILE_VERSION: u32 = 1;

/// A parsed `.mid.ron` sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SongFile {
    /// Schema version; must be [`SONG_FILE_VERSION`].
    pub version: u32,
    /// The SoundFont that plays the background score.
    pub soundfont: String,
    /// Whether the song loops at its end. Omitted means `true`.
    #[serde(default = "default_looping")]
    pub looping: bool,
    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default)]
    pub instruments: Vec<Instrument>,
    #[serde(default)]
    pub bindings: Vec<Binding>,
    /// What the loader learned from the SoundFonts and samples, for
    /// [`SongFile::validate`]. Never written to or read from the file.
    #[serde(skip)]
    pub resources: ResourceMetadata,
}

fn default_looping() -> bool {
    true
}

/// A musical part: one track's notes, how a trigger walks them and when it may sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub name: String,
    pub track: TrackSelector,
    #[serde(default)]
    pub mode: PartMode,
    #[serde(default)]
    pub quantize: Quantize,
}

/// Which track of the MIDI file a part reads: by name, or by index — the escape hatch
/// when two tracks share a name. Indexes count the selectable tracks of
/// [`SongData::tracks`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackSelector {
    Name(String),
    Index(usize),
}

impl fmt::Display for TrackSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrackSelector::Name(name) => write!(formatter, "track name '{name}'"),
            TrackSelector::Index(index) => write!(formatter, "track index {index}"),
        }
    }
}

/// How a part's notes answer its triggers. `docs/SONG_FORMAT.md` § Modes has the full
/// semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PartMode {
    /// Each accepted trigger plays the next unit, wrapping at the end; one cursor shared
    /// by every binding of the part. Its notes need not fit the background's chords.
    #[default]
    Loop,
    /// Each accepted trigger plays the next unit not already passed by the song, until
    /// the part is exhausted; the cursor resets when the song loops.
    Sequence,
    /// A trigger plays the unit sounding at the (quantized) song position, or nothing in
    /// a rest; no cursor.
    AtSongPosition,
}

/// The grid a part's triggers snap forward to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Quantize {
    #[default]
    Off,
    Beat,
    Eighth,
    Sixteenth,
}

/// A named instrument a binding plays a part on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub name: String,
    pub source: InstrumentSource,
}

/// Where an instrument's sound comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstrumentSource {
    /// A preset of a SoundFont. Bank 128 is General MIDI percussion.
    SoundFont { path: String, bank: u16, preset: u8 },
    /// A recorded sample, pitched from `root_note` by resampling, so it also plays
    /// shorter going up and longer going down.
    Sample {
        path: String,
        root_note: u8,
        #[serde(default = "default_gain")]
        gain: f32,
        #[serde(default)]
        playback: SamplePlayback,
    },
}

fn default_gain() -> f32 {
    1.0
}

/// Whether a sample voice ignores its note-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SamplePlayback {
    /// Plays to the end of the sample whatever the note's length.
    #[default]
    OneShot,
    /// Fades out when the note ends.
    Gated,
}

/// A game action mapped to a part played on an instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// The stable id game code triggers.
    pub action: String,
    pub part: String,
    pub instrument: String,
}

/// One preset of one SoundFont.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresetKey {
    pub soundfont: String,
    pub bank: u16,
    pub preset: u8,
}

/// What the loader read from a song's resources, so validation needs no I/O.
///
/// `preset_layering` holds an entry for every preset a loaded SoundFont has — the most
/// voices one of its note-ons starts — so an absent entry means the preset is missing.
/// `sample_seconds` holds each decoded sample's length by path.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceMetadata {
    pub preset_layering: BTreeMap<PresetKey, usize>,
    pub sample_seconds: BTreeMap<String, f64>,
}

/// Reads only the version, so a future file is refused for its version rather than for
/// the fields it added.
#[derive(Deserialize)]
#[serde(rename = "SongFile")]
struct VersionProbe {
    version: u32,
}

/// Parse a sidecar's text, rejecting everything checkable without the MIDI file or the
/// resources. `label` names the file in every error — pass the sidecar path.
pub fn parse_song_file(label: &str, ron_text: &str) -> Result<SongFile, SongFileError> {
    let error = |problem| SongFileError {
        label: label.to_string(),
        problem,
    };
    let malformed = |reason: ron::error::SpannedError| {
        error(SongFileProblem::Malformed {
            reason: reason.to_string(),
        })
    };
    let probe: VersionProbe = ron::from_str(ron_text).map_err(malformed)?;
    if probe.version != SONG_FILE_VERSION {
        return Err(error(SongFileProblem::UnsupportedVersion {
            found: probe.version,
            supported: SONG_FILE_VERSION,
        }));
    }
    let file: SongFile = ron::from_str(ron_text).map_err(malformed)?;
    file.check_schema().map_err(error)?;
    Ok(file)
}

impl SongFile {
    /// Check the file against its MIDI data and [`SongFile::resources`], after
    /// everything [`parse_song_file`] checks. `label` names the file in the error.
    pub fn validate(&self, label: &str, song: &SongData) -> Result<(), SongFileError> {
        self.check_schema()
            .and_then(|()| self.check_against(song))
            .map_err(|problem| SongFileError {
                label: label.to_string(),
                problem,
            })
    }
}

impl Part {
    /// The index in [`SongData::tracks`] this part reads. A name shared by two tracks is
    /// refused — select one by index instead; an index is never refused for its name.
    pub fn resolve_track(&self, song: &SongData) -> Result<usize, SongFileProblem> {
        let tracks = song.tracks();
        let missing = || SongFileProblem::MissingTrack {
            part: self.name.clone(),
            selector: self.track.to_string(),
            track_count: tracks.len(),
        };
        match &self.track {
            TrackSelector::Index(index) => (*index < tracks.len())
                .then_some(*index)
                .ok_or_else(missing),
            TrackSelector::Name(name) => {
                let mut matches = tracks
                    .iter()
                    .enumerate()
                    .filter(|(_, track)| track.name() == Some(name.as_str()))
                    .map(|(index, _)| index);
                let first = matches.next().ok_or_else(missing)?;
                match matches.next() {
                    None => Ok(first),
                    Some(second) => Err(SongFileProblem::AmbiguousTrackName {
                        part: self.name.clone(),
                        name: name.clone(),
                        first,
                        second,
                    }),
                }
            }
        }
    }
}
