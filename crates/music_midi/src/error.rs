//! The crate's errors: [`SongError`] for reading and writing a MIDI file, [`EditError`]
//! for a mutation [`SongData`](crate::SongData) refuses, and [`SongFileError`] for a song
//! sidecar that fails validation — naming the file, the collection and the entry.

use std::fmt;

use crate::song_data::NoteId;

/// A MIDI file that cannot be read, or a song that cannot be written back out.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SongError {
    #[error("MIDI file is {bytes} bytes; the limit is {limit}")]
    TooLarge { bytes: usize, limit: usize },
    #[error("MIDI file holds more than {limit} events")]
    TooManyEvents { limit: usize },
    #[error("not a readable Standard MIDI File: {reason}")]
    Malformed { reason: String },
    #[error("MIDI file has no tracks")]
    NoTracks,
    #[error("SMF type 2 (independent sequences) is not supported; export the song as type 0 or 1")]
    SequentialFormat,
    #[error(
        "SMPTE time division ({frames_per_second} frames a second) is not supported; \
         export the song with ticks per quarter note"
    )]
    SmpteDivision { frames_per_second: u8 },
    #[error("MIDI file's division is zero ticks per quarter note")]
    ZeroDivision,
    #[error("track {track} sets a tempo of zero microseconds per quarter note at tick {tick}")]
    ZeroTempo { track: usize, tick: u64 },
    #[error(
        "track {track} has time signature {numerator}/2^{denominator_exponent} at tick {tick}, \
         which does not divide {ticks_per_quarter} ticks per quarter note into whole beats"
    )]
    InvalidTimeSignature {
        track: usize,
        tick: u64,
        numerator: u8,
        denominator_exponent: u8,
        ticks_per_quarter: u16,
    },
    #[error("MIDI writer refused the song: {reason}")]
    WriteFailed { reason: String },
}

/// A mutation [`SongData`](crate::SongData) refused. Nothing was changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("{0} does not exist")]
    UnknownNote(NoteId),
    #[error("track {track} does not exist (the song has {track_count})")]
    TrackOutOfRange { track: usize, track_count: usize },
    #[error("track {track} has no MIDI channel to write notes on")]
    TrackHasNoChannel { track: usize },
    #[error("key {key} is outside MIDI's 0 to 127")]
    KeyOutOfRange { key: u8 },
    #[error("velocity {velocity} is outside 1 to 127 (velocity 0 is a note-off)")]
    VelocityOutOfRange { velocity: u8 },
    #[error("a note must last at least one tick")]
    ZeroDuration,
    #[error("{note} would move before the start of the song")]
    BeforeSongStart { note: NoteId },
    #[error("{note} would end past tick {limit}, the last a MIDI file can write")]
    PastSongEnd { note: NoteId, limit: u64 },
    #[error("a new note at tick {tick} on key {key} would end past tick {limit}, the last a MIDI file can write")]
    NewNotePastSongEnd { tick: u64, key: u8, limit: u64 },
    #[error(
        "tempo {microseconds_per_quarter} microseconds per quarter note is outside 1 to 16777215"
    )]
    TempoOutOfRange { microseconds_per_quarter: u32 },
    #[error(
        "time signature {numerator}/{denominator} does not divide {ticks_per_quarter} ticks per \
         quarter note into whole beats"
    )]
    InvalidTimeSignature {
        numerator: u8,
        denominator: u8,
        ticks_per_quarter: u16,
    },
}

/// One of a song file's three named collections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collection {
    Parts,
    Instruments,
    Bindings,
}

impl fmt::Display for Collection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Collection::Parts => "parts",
            Collection::Instruments => "instruments",
            Collection::Bindings => "bindings",
        })
    }
}

/// A song sidecar that failed to parse or validate: the file's label (its path, as the
/// loader names it) and what is wrong with it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{label}: {problem}")]
pub struct SongFileError {
    pub label: String,
    pub problem: SongFileProblem,
}

/// What is wrong with a song sidecar. Every entry-level problem names its collection and
/// the entry, as `parts['lead']` — a binding is named by its action.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SongFileProblem {
    #[error("malformed song file: {reason}")]
    Malformed { reason: String },
    #[error("unsupported song version {found} (this engine reads version {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("{collection}[{index}] has an empty name")]
    EmptyName {
        collection: Collection,
        index: usize,
    },
    #[error("{collection}['{name}'] is declared more than once")]
    DuplicateName {
        collection: Collection,
        name: String,
    },
    #[error("{entry} has an empty path")]
    EmptyPath { entry: String },
    #[error("bindings['{action}'] plays part '{part}', which parts does not declare")]
    UnknownPart { action: String, part: String },
    #[error(
        "bindings['{action}'] plays instrument '{instrument}', which instruments does not declare"
    )]
    UnknownInstrument { action: String, instrument: String },
    #[error("instruments['{instrument}'] has gain {gain}, which must be finite and at least zero")]
    InvalidGain { instrument: String, gain: f32 },
    #[error("instruments['{instrument}'] has root note {root_note}; MIDI notes run 0 to 127")]
    RootNoteOutOfRange { instrument: String, root_note: u8 },
    #[error(
        "instruments['{instrument}'] asks for bank {bank} preset {preset}; banks run 0 to 128 \
         and presets 0 to 127"
    )]
    PresetOutOfRange {
        instrument: String,
        bank: u16,
        preset: u8,
    },
    #[error("instruments declares {count} entries; a song has {capacity} instrument slots")]
    TooManyInstruments { count: usize, capacity: usize },
    #[error("parts['{part}'] selects {selector}, which the MIDI file does not have ({track_count} tracks)")]
    MissingTrack {
        part: String,
        selector: String,
        track_count: usize,
    },
    #[error(
        "parts['{part}'] selects track name '{name}', which tracks {first} and {second} share; \
         select it by Index instead"
    )]
    AmbiguousTrackName {
        part: String,
        name: String,
        first: usize,
        second: usize,
    },
    #[error("parts['{part}'] plays track {track}, which has no notes")]
    EmptyTriggerPart { part: String, track: usize },
    #[error(
        "bindings['{action}'] plays key {key} at tick {tick} on sample instrument '{instrument}', \
         more than {range} semitones from its root note {root_note}"
    )]
    NoteOutsideSampleRange {
        action: String,
        instrument: String,
        key: u8,
        tick: u64,
        root_note: u8,
        range: u8,
    },
    #[error(
        "instruments['{instrument}'] asks for bank {bank} preset {preset} in '{soundfont}', \
         which that SoundFont does not have"
    )]
    PresetNotInSoundFont {
        instrument: String,
        soundfont: String,
        bank: u16,
        preset: u8,
    },
    #[error(
        "instruments['{instrument}'] layers {layering} voices a note; {sounding_cap} sounding \
         notes need {needed} voices, more than the synthesizer's {polyphony}"
    )]
    LayeringExceedsPolyphony {
        instrument: String,
        layering: usize,
        sounding_cap: usize,
        needed: usize,
        polyphony: usize,
    },
    #[error("soundfont '{path}' plays the background score, but the loader did not decode it")]
    SoundFontNotSupplied { path: String },
    #[error("instruments['{instrument}'] plays sample '{path}', which the loader did not decode")]
    SampleNotSupplied { instrument: String, path: String },
    #[error(
        "instruments['{instrument}'] plays a sample {seconds} s long; samples run {minimum} s \
         to {maximum} s"
    )]
    SampleLengthOutOfRange {
        instrument: String,
        seconds: f64,
        minimum: f64,
        maximum: f64,
    },
}

/// A conductor that cannot be built, or a song it cannot load. Nothing was changed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConductorError {
    #[error("the conductor's {capacity} capacity is zero; every capacity holds at least one")]
    ZeroCapacity { capacity: &'static str },
    #[error("sample rate {sample_rate} Hz is outside the synthesizer's 16000 to 192000")]
    SampleRateOutOfRange { sample_rate: u32 },
    #[error("block size {block_size} is outside the synthesizer's 8 to 1024 frames")]
    BlockSizeOutOfRange { block_size: usize },
    #[error("a pool of {polyphony} voices is outside the synthesizer's 8 to 256")]
    PolyphonyOutOfRange { polyphony: usize },
    #[error(
        "a pool of {polyphony} voices cannot hold {sounding_cap} sounding notes of even one \
         voice each"
    )]
    PoolBelowSoundingCap {
        polyphony: usize,
        sounding_cap: usize,
    },
    #[error("{lanes} lanes a slot is more than the {maximum} MIDI channels a slot can lease")]
    TooManyLanes { lanes: usize, maximum: usize },
    #[error("the song names SoundFont '{path}', but its bytes were not supplied")]
    SoundFontNotSupplied { path: String },
    #[error("SoundFont '{path}' cannot be read: {reason}")]
    SoundFont { path: String, reason: String },
    #[error("the synthesizer refused its settings: {reason}")]
    Synthesizer { reason: String },
    #[error("the song does not fit its MIDI file: {0}")]
    SongDoesNotFit(SongFileProblem),
    #[error("the song declares {count} instruments; the conductor has {capacity} slots")]
    TooManyInstruments { count: usize, capacity: usize },
    #[error(
        "instruments['{instrument}'] asks for bank {bank} preset {preset} in '{soundfont}', \
         which that SoundFont does not have"
    )]
    PresetMissing {
        instrument: String,
        soundfont: String,
        bank: u16,
        preset: u8,
    },
    #[error(
        "instruments['{instrument}'] layers {layering} voices a note; {sounding_cap} sounding \
         notes need {needed} voices, more than the pool's {polyphony}"
    )]
    LayeringExceedsPool {
        instrument: String,
        layering: usize,
        sounding_cap: usize,
        needed: usize,
        polyphony: usize,
    },
}
