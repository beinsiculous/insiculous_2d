//! Error types for the audio system.

use std::io;
use thiserror::Error;

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
}

/// Result type for audio operations.
pub type AudioResult<T> = Result<T, AudioError>;
