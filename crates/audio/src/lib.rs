//! Audio system for the insiculous_2d game engine.
//!
//! This crate provides audio playback functionality including:
//! - Sound effect playback with volume, speed and a stereo pan, each playback an
//!   instance ([`PlaybackId`]) that can be stopped and re-mixed on its own
//! - Background music playback (looping or one-shot)
//! - Audio resource management and caching
//! - Sample instruments' audio: a WAV decoded once into the conductor's `OwnedPcm`
//!   ([`load_sample_pcm`])
//! - Songs: a MIDI file and its `.mid.ron` sidecar played by the conductor
//!   ([`AudioManager::play_song`]), its background on the music bus and its
//!   triggered instruments on the sfx bus, with beat queries and triggers
//!
//! # Example
//! ```
//! use audio::{AudioManager, AudioResult};
//!
//! # fn wav_bytes() -> Vec<u8> {
//! #     // Minimal valid WAV (PCM, mono, one silent sample) so this doctest runs headless.
//! #     let mut bytes = Vec::new();
//! #     bytes.extend_from_slice(b"RIFF");
//! #     bytes.extend_from_slice(&38u32.to_le_bytes());
//! #     bytes.extend_from_slice(b"WAVEfmt ");
//! #     bytes.extend_from_slice(&16u32.to_le_bytes());
//! #     bytes.extend_from_slice(&1u16.to_le_bytes());
//! #     bytes.extend_from_slice(&1u16.to_le_bytes());
//! #     bytes.extend_from_slice(&44100u32.to_le_bytes());
//! #     bytes.extend_from_slice(&88200u32.to_le_bytes());
//! #     bytes.extend_from_slice(&2u16.to_le_bytes());
//! #     bytes.extend_from_slice(&16u16.to_le_bytes());
//! #     bytes.extend_from_slice(b"data");
//! #     bytes.extend_from_slice(&2u32.to_le_bytes());
//! #     bytes.extend_from_slice(&0i16.to_le_bytes());
//! #     bytes
//! # }
//! # fn main() -> AudioResult<()> {
//! // Never fails: without an audio device the manager still loads and
//! // validates sounds, and playback becomes a no-op.
//! let mut audio = AudioManager::new_or_disabled();
//!
//! // Load from disk with `load_sound("assets/jump.wav")`, or from memory:
//! let sound = audio.load_sound_from_bytes(wav_bytes())?;
//! audio.play(sound)?;
//! # Ok(())
//! # }
//! ```

mod conductor_source;
mod error;
mod manager;
mod pan_source;
mod sample_loader;
mod sound;
#[cfg(test)]
mod test_support;

pub use conductor_source::{
    SourceReadings, LIMITER_ATTACK_SECONDS, LIMITER_RELEASE_SECONDS, LIMITER_THRESHOLD,
};
pub use error::{AudioError, AudioResult};
pub use manager::song::load::{SongProblem, SONG_FILE_BYTE_LIMIT, SOUND_FONT_BYTE_LIMIT};
pub use manager::song::{SongPosition, SongSnapshot, Subdivision};
pub use manager::AudioManager;
/// The conductor's song types, so a game names them through the audio crate alone.
pub use music_midi::{
    PositionSnapshot, Song, SongData, SongFile, SourceId, TransportState,
};
pub use sample_loader::{decode_sample_pcm, load_sample_pcm, SampleProblem, SAMPLE_BYTE_LIMIT};
pub use sound::{Bus, PlaybackId, SoundHandle, SoundSettings};
