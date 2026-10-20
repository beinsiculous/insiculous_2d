//! MIDI songs and the conductor that plays them: the `.mid.ron` song sidecar, a Standard
//! MIDI File as owned and editable data, the tempo map that turns ticks into seconds,
//! samples and bars, and the sequencer and conductor that render a song.
//!
//! The crate does no I/O and never touches rodio: bytes come in, data and samples come out.
//! The audio crate reads the files, then hands this crate the bytes and a
//! [`ResourceMetadata`] describing what it loaded; the editor's composer edits a
//! [`SongData`] and writes it back with [`SongData::write_smf`]. The `synth` feature (on
//! by default) owns rustysynth and everything that renders; the rest builds without it.
//!
//! The pieces:
//! - [`SongFile`] — the versioned sidecar: named parts over MIDI tracks, named
//!   instruments, and bindings from game actions to a part played on an instrument.
//!   [`parse_song_file`] checks what the text alone can show; [`SongFile::validate`]
//!   checks it against the MIDI data and the loaded resources.
//! - [`SongData`] — every track's events retained for export, notes grouped into
//!   [`MusicalUnit`]s under stable [`NoteId`]s, and mutation helpers that return the
//!   before-images an undo needs.
//! - [`TempoMap`] — exact tick ↔ seconds ↔ sample ↔ bar conversions across tempo and
//!   time-signature changes.
//! - [`Sequencer`] — the background score's clock: channel messages at the right samples,
//!   a sample-exact loop seam, and the state a seek rebuilds.
//! - `Conductor` (`synth`) — the background synthesizers and the instrument slots,
//!   rendered block by block under the game's transport commands.

#[cfg(feature = "synth")]
pub mod commands;
#[cfg(feature = "synth")]
pub mod conductor;
pub mod error;
#[cfg(feature = "synth")]
pub mod position;
pub mod sequencer;
pub mod song_data;
pub mod song_file;
pub mod tempo_map;
#[cfg(test)]
mod test_support;

#[cfg(feature = "synth")]
pub use commands::{CommandRejected, ConductorCommand, ConductorHandle};
#[cfg(feature = "synth")]
pub use conductor::{
    BackgroundPeak, BusGains, Capacities, Conductor, ConductorConfig, InstrumentSlot, LoadReport,
    LoadWarning, Song, SourceKey,
};
pub use error::{Collection, ConductorError, EditError, SongError, SongFileError, SongFileProblem};
#[cfg(feature = "synth")]
pub use position::{PositionSnapshot, SharedPosition, TransportState};
pub use sequencer::{
    BackgroundScore, ChannelMessage, ChannelStateFold, ScoreEvent, Sequencer, SequencerSink,
};
pub use song_data::{
    MusicalUnit, NewNote, Note, NoteId, NoteImage, RetainedEvent, SameKeyOverlap, SmfFormat,
    SongData, SongTrack, UnsupportedFeature, UnsupportedReport,
};
pub use song_file::{
    parse_song_file, Binding, Instrument, InstrumentSource, Part, PartMode, PresetKey, Quantize,
    ResourceMetadata, SamplePlayback, SongFile, TrackSelector, SONG_FILE_VERSION,
};
pub use tempo_map::{
    MusicalPosition, TempoChange, TempoMap, TimeSignature, DEFAULT_MICROSECONDS_PER_QUARTER,
};

/// The MIDI parser and writer behind [`SongData`]. Retained events are midly's types, so
/// it is re-exported here and no other crate names it in its own manifest.
pub use midly;

/// The SoundFont synthesizer behind the conductor, re-exported so the audio crate builds
/// a [`SoundFont`](rustysynth::SoundFont) from bytes without naming rustysynth itself.
#[cfg(feature = "synth")]
pub use rustysynth;

/// Instrument slots a song may fill. Every instrument gets a slot of its own (its own
/// synthesizer, or a sample slot), so this is also the most instruments a song declares.
pub const INSTRUMENT_SLOT_CAPACITY: usize = 8;

/// Notes one instrument slot holds sounding at once; past it the slot releases its
/// oldest note.
pub const SOUNDING_NOTE_CAP: usize = 24;

/// Voices in every synthesizer's pool. A preset whose layering times
/// [`SOUNDING_NOTE_CAP`] exceeds it could have a sounding note cut by the synthesizer's
/// own voice stealer, which is the one cut the conductor promises never to make.
pub const SYNTHESIZER_POLYPHONY: usize = 128;

/// How far a sample instrument may be pitched from its root note, in semitones either
/// way. The resampler has no filter, so pitching further up aliases audibly.
pub const SAMPLE_SEMITONE_RANGE: u8 = 24;

/// The shortest decoded sample an instrument may play, in seconds.
pub const SAMPLE_SECONDS_MINIMUM: f64 = 0.001;

/// The longest decoded sample an instrument may play, in seconds.
pub const SAMPLE_SECONDS_MAXIMUM: f64 = 10.0;

/// MIDI channels an instrument slot leases to sources: all sixteen but channel 10, which
/// the synthesizer reserves for General MIDI percussion.
pub const LANES_PER_SLOT: usize = 15;

/// Live voices a sample instrument's slot holds at once.
pub const SAMPLE_VOICE_CAP: usize = 16;

/// Frames the conductor renders between two command drains: its synthesizers' block, and
/// the background sequencer's grain.
pub const BLOCK_FRAMES: usize = 64;

/// Commands the conductor's queue holds before a send is refused.
pub const COMMAND_QUEUE_CAPACITY: usize = 256;

/// Scheduled events the conductor holds pending at once.
pub const PENDING_EVENT_CAPACITY: usize = 512;

/// The largest MIDI file [`SongData::from_smf`] reads, checked before parsing.
pub const MIDI_BYTE_LIMIT: usize = 1024 * 1024;

/// The most events [`SongData::from_smf`] reads, counted while parsing so an oversized
/// file is refused before it is built in full. The overture fixture holds 25 230 events
/// in 109 KiB; a file at [`MIDI_BYTE_LIMIT`] that dense holds about 242 000.
pub const MIDI_EVENT_LIMIT: usize = 262_144;
