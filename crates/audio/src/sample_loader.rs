//! A sample instrument's audio: a WAV file turned into the conductor's [`OwnedPcm`].
//!
//! The file is read through `common::vfs`, so the same path works natively and on the
//! web. Everything that can refuse it before decoding does: the byte limit first, then the
//! RIFF header — WAV is the one format imported as a *pitched* sample, whatever the file
//! is named and whatever else rodio's decoder could read (recorded MP3 and OGG still play
//! as ordinary sounds). The decoder's header is checked against the sample rate and
//! channel limits, and the decode stops one frame past the length limit, so an oversized
//! file is never decoded in full only to be refused. [`OwnedPcm::new`] checks the rest.
//!
//! Decoding happens here, once per unique sample, off the audio thread; the conductor's
//! `music_midi` crate never decodes.

use std::io::{self, Cursor};
use std::path::Path;

use music_midi::{OwnedPcm, PcmError};
use rodio::{Decoder, Source};

use crate::error::{AudioError, AudioResult};

#[cfg(test)]
mod tests;

/// The largest sample file read, in bytes, checked before decoding: 10 s of 16-bit stereo
/// at 96 kHz fits.
pub const SAMPLE_BYTE_LIMIT: usize = 4 * 1024 * 1024;

/// Why a file cannot be a sample instrument's audio.
#[derive(Debug, thiserror::Error)]
pub enum SampleProblem {
    #[error("cannot be read: {0}")]
    Unreadable(#[source] io::Error),
    #[error("is {bytes} bytes; a sample file holds at most {limit}")]
    TooManyBytes { bytes: usize, limit: usize },
    #[error("is not a WAV file; WAV is the supported format for a pitched sample")]
    NotWav,
    #[error("cannot be decoded as WAV: {reason}")]
    Undecodable { reason: String },
    #[error("{0}")]
    OutsideLimits(#[from] PcmError),
}

/// Read and decode the WAV at `path` as a sample instrument's audio. Every error names
/// the path: [`AudioError::Sample`].
pub fn load_sample_pcm(path: impl AsRef<Path>) -> AudioResult<OwnedPcm> {
    let path = path.as_ref();
    let named = |problem| AudioError::Sample {
        path: path.display().to_string(),
        problem,
    };
    let bytes = common::vfs::read(path).map_err(|error| named(SampleProblem::Unreadable(error)))?;
    decode_sample_pcm(bytes).map_err(named)
}

/// Decode a WAV file's bytes as a sample instrument's audio. See the module docs.
pub fn decode_sample_pcm(bytes: Vec<u8>) -> Result<OwnedPcm, SampleProblem> {
    if bytes.len() > SAMPLE_BYTE_LIMIT {
        return Err(SampleProblem::TooManyBytes {
            bytes: bytes.len(),
            limit: SAMPLE_BYTE_LIMIT,
        });
    }
    if !is_riff_wave(&bytes) {
        return Err(SampleProblem::NotWav);
    }
    let decoder =
        Decoder::new_wav(Cursor::new(bytes)).map_err(|error| SampleProblem::Undecodable {
            reason: error.to_string(),
        })?;
    let (channels, sample_rate) = (decoder.channels(), decoder.sample_rate());
    OwnedPcm::check_format(sample_rate, channels)?;
    let most_samples = OwnedPcm::maximum_frames(sample_rate) * usize::from(channels);
    let samples: Vec<f32> = decoder
        .convert_samples::<f32>()
        .take(most_samples + usize::from(channels))
        .collect();
    if samples.len() > most_samples {
        return Err(PcmError::TooLong {
            sample_rate,
            maximum_seconds: music_midi::SAMPLE_SECONDS_MAXIMUM,
        }
        .into());
    }
    Ok(OwnedPcm::new(sample_rate, channels, samples)?)
}

/// Whether `bytes` open as a RIFF file of WAVE form: the format is the header's word, not
/// the file name's.
fn is_riff_wave(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}
