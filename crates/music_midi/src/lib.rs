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
//! - [`OwnedPcm`] — a decoded sample, checked against the sample limits; the audio crate
//!   decodes the file, this crate only holds and plays what it produced.
//! - `Conductor` (`synth`) — the background synthesizers and the instrument slots (a
//!   SoundFont preset's synthesizer, or a sample's voices), rendered block by block under
//!   the game's commands: transport, and triggers that play a binding's part on its
//!   instrument for one source, placed from the producer's stamp.

#[cfg(feature = "synth")]
pub mod commands;
#[cfg(feature = "synth")]
pub mod conductor;
pub mod error;
#[cfg(feature = "synth")]
mod overflow;
pub mod pcm;
#[cfg(feature = "synth")]
pub mod position;
#[cfg(feature = "synth")]
pub mod sample_bank;
#[cfg(feature = "synth")]
mod sample_voice;
pub mod sequencer;
pub mod song_data;
pub mod song_file;
#[cfg(feature = "synth")]
pub mod sources;
pub mod tempo_map;
#[cfg(test)]
mod test_support;
#[cfg(feature = "synth")]
pub mod triggers;

#[cfg(feature = "synth")]
pub use commands::{CommandRejected, ConductorCommand, ConductorHandle};
#[cfg(feature = "synth")]
pub use conductor::{
    parse_sound_font, preset_layering, BackgroundPeak, BusGains, Capacities, Conductor,
    ConductorConfig, InstrumentSlot, LoadReport, LoadWarning, NoteTicket, SampleCounters,
    SampleSlot, Song,
};
pub use error::{
    Collection, ConductorError, EditError, PcmError, SongError, SongFileError, SongFileProblem,
};
pub use pcm::OwnedPcm;
#[cfg(feature = "synth")]
pub use position::{PositionSnapshot, SharedPosition, TransportState};
#[cfg(feature = "synth")]
pub use sample_bank::SampleBank;
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
#[cfg(feature = "synth")]
pub use sources::SourceId;
pub use tempo_map::{
    MusicalPosition, TempoChange, TempoMap, TimeSignature, DEFAULT_MICROSECONDS_PER_QUARTER,
};
#[cfg(feature = "synth")]
pub use triggers::{BindingIndex, TriggerCounters};

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

/// The lowest sample rate a sample may be recorded at, in hertz.
pub const SAMPLE_RATE_MINIMUM_HERTZ: u32 = 8_000;

/// The highest sample rate a sample may be recorded at, in hertz.
pub const SAMPLE_RATE_MAXIMUM_HERTZ: u32 = 96_000;

/// MIDI channels an instrument slot leases to sources: all sixteen but channel 10, which
/// the synthesizer reserves for General MIDI percussion.
pub const LANES_PER_SLOT: usize = 15;

/// Live voices a sample instrument's slot holds at once; past it the oldest live voice
/// fades out to make room.
pub const SAMPLE_VOICE_CAP: usize = 16;

/// Fading voices a sample slot holds beyond its live voices. A burst inside one block
/// fades a stolen voice per trigger; past the live cap plus this budget the oldest fading
/// voice is cut outright, so voices never pile up.
pub const SAMPLE_FADE_BUDGET: usize = 16;

/// The level a sample voice plays at, before its instrument's gain and its velocity: a
/// full-scale recording at gain 1.0 and velocity 127 peaks at a fifth of full scale, where
/// one SoundFont note of the conductor's cast bank peaks (0.15 to 0.21 measured), so a
/// sample instrument sits level with a preset and sixteen of them stay near full scale.
pub const SAMPLE_VOICE_LEVEL: f32 = 0.2;

/// How long a sample voice fades when its gated note ends or it is ended by force (a
/// steal, a deactivation, a seek, a wrap, a pause, a stop), in seconds: long enough not
/// to click, short enough to read as a cut.
pub const SAMPLE_FADE_SECONDS: f64 = 0.005;

/// Frames the conductor renders between two command drains: its synthesizers' block, and
/// the background sequencer's grain.
pub const BLOCK_FRAMES: usize = 64;

/// Commands the conductor's queue holds before a send is refused.
pub const COMMAND_QUEUE_CAPACITY: usize = 256;

/// Places of the command queue held back from triggers, so a stop, a pause or a
/// deactivation always fits behind a queue full of them.
pub const LIFECYCLE_RESERVE: usize = 16;

/// Commands drained at the start of one block. A full queue drains in
/// `COMMAND_QUEUE_CAPACITY / COMMANDS_PER_BLOCK` = 4 blocks, 5.8 ms at 44.1 kHz, on top of
/// the wait for the block itself.
pub const COMMANDS_PER_BLOCK: usize = 64;

/// Scheduled note-ons and note-offs the conductor holds pending at once.
pub const PENDING_EVENT_CAPACITY: usize = 512;

/// Source identities the conductor tracks at once, live or remembered dead.
pub const SOURCE_CAPACITY: usize = 128;

/// How long a command can wait for the audio thread to drain it: the longest device
/// period measured, the browser's 2048-frame pulls at 53 to 61 ms (natively 42.6 ms). A
/// trigger older than this plus one block when it is drained is dropped as late.
pub const DEVICE_PERIOD_ALLOWANCE: std::time::Duration = std::time::Duration::from_millis(61);

/// The largest MIDI file [`SongData::from_smf`] reads, checked before parsing.
pub const MIDI_BYTE_LIMIT: usize = 1024 * 1024;

/// The most events [`SongData::from_smf`] reads, counted while parsing so an oversized
/// file is refused before it is built in full. The overture fixture holds 25 230 events
/// in 109 KiB; a file at [`MIDI_BYTE_LIMIT`] that dense holds about 242 000.
pub const MIDI_EVENT_LIMIT: usize = 262_144;
