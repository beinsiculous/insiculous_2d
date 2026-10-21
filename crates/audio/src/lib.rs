//! Audio system for the insiculous_2d game engine.
//!
//! This crate provides audio playback functionality including:
//! - Sound effect playback with volume and speed control
//! - Background music playback (looping or one-shot)
//! - Audio resource management and caching
//! - Sample instruments' audio: a WAV decoded once into the conductor's `OwnedPcm`
//!   ([`load_sample_pcm`])
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

mod error;
mod manager;
mod sample_loader;
mod sound;

pub use error::{AudioError, AudioResult};
pub use manager::AudioManager;
pub use sample_loader::{decode_sample_pcm, load_sample_pcm, SampleProblem, SAMPLE_BYTE_LIMIT};
pub use sound::{Bus, SoundHandle, SoundSettings};
