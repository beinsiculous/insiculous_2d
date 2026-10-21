//! Error types for the audio system.

use std::io;
use thiserror::Error;

use music_midi::CommandRejected;

use crate::manager::song::load::SongProblem;
use crate::sample_loader::SampleProblem;

/// Errors that can occur in the audio system.
#[derive(Debug, Error)]
pub enum AudioError {
    /// Failed to initialize the audio device.
    #[error("Failed to initialize audio device: {0}")]
    DeviceInitError(String),

    /// Failed to create an audio stream.
    #[error("Failed to create audio stream: {0}")]
    StreamError(String),

    /// Failed to load an audio file.
    #[error("Failed to load audio file: {0}")]
    LoadError(String),

    /// Failed to decode audio data.
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    /// The specified sound handle is invalid.
    #[error("Invalid sound handle: {0}")]
    InvalidHandle(u32),

    /// I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// A file that cannot be a sample instrument's audio, named by its path.
    #[error("Sample '{path}' {problem}")]
    Sample {
        path: String,
        problem: SampleProblem,
    },

    /// A song that cannot be played: the song, the sidecar entry that names the file
    /// at fault, and that file's path.
    #[error("Song '{song}': {entry} '{path}' {problem}")]
    Song {
        song: String,
        entry: String,
        path: String,
        problem: Box<SongProblem>,
    },

    /// A trigger naming an action the playing song does not bind.
    #[error("Song '{song}' binds no action '{action}'")]
    UnknownAction { song: String, action: String },

    /// A song call that needs a playing song: none was started, or its output died.
    #[error("No song is playing")]
    SongNotPlaying,

    /// The conductor's command queue refused a trigger.
    #[error("The song refused the command: {0}")]
    CommandRefused(CommandRejected),
}

/// Result type for audio operations.
pub type AudioResult<T> = Result<T, AudioError>;
