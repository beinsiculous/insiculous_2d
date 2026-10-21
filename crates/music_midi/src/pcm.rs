//! Decoded sample audio as a sample instrument plays it: owned, interleaved, immutable and
//! shared by every voice that plays it.
//!
//! This crate never decodes. The audio crate turns a file into samples and hands them to
//! [`OwnedPcm::new`], which refuses anything outside the limits a sample instrument keeps:
//! 8 to 96 kHz, mono or stereo, 1 ms to 10 s long, every value finite. A value of this type
//! is within them by construction, so nothing that plays it checks again. A decoder can
//! test the format with [`OwnedPcm::check_format`] before decoding anything, and stop
//! decoding one frame past [`OwnedPcm::maximum_frames`], so an oversized file is never
//! built in full only to be refused.

use std::sync::Arc;

use crate::error::PcmError;
use crate::{
    SAMPLE_RATE_MAXIMUM_HERTZ, SAMPLE_RATE_MINIMUM_HERTZ, SAMPLE_SECONDS_MAXIMUM,
    SAMPLE_SECONDS_MINIMUM,
};

#[cfg(test)]
mod tests;

/// One decoded sample. See the module docs.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedPcm {
    sample_rate: u32,
    channels: u8,
    /// Every frame's values, interleaved: left then right for stereo.
    samples: Arc<[f32]>,
}

impl OwnedPcm {
    /// Decoded audio at `sample_rate` hertz with `channels` interleaved channels, refused
    /// with the limit it breaks when it is outside the module's limits.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, PcmError> {
        Self::check_format(sample_rate, channels)?;
        let channels = if channels == 1 { 1 } else { 2 };
        if !samples.len().is_multiple_of(usize::from(channels)) {
            return Err(PcmError::PartialFrame {
                samples: samples.len(),
                channels,
            });
        }
        let frames = samples.len() / usize::from(channels);
        if frames < Self::minimum_frames(sample_rate) {
            return Err(PcmError::TooShort {
                frames,
                sample_rate,
                minimum_seconds: SAMPLE_SECONDS_MINIMUM,
            });
        }
        if frames > Self::maximum_frames(sample_rate) {
            return Err(PcmError::TooLong {
                sample_rate,
                maximum_seconds: SAMPLE_SECONDS_MAXIMUM,
            });
        }
        if let Some(index) = samples.iter().position(|value| !value.is_finite()) {
            return Err(PcmError::NotFinite { index });
        }
        Ok(Self {
            sample_rate,
            channels,
            samples: samples.into(),
        })
    }

    /// Whether audio at `sample_rate` hertz with `channels` channels may be a sample, so a
    /// decoder can refuse a file from its header alone.
    pub fn check_format(sample_rate: u32, channels: u16) -> Result<(), PcmError> {
        if !(SAMPLE_RATE_MINIMUM_HERTZ..=SAMPLE_RATE_MAXIMUM_HERTZ).contains(&sample_rate) {
            return Err(PcmError::SampleRateOutOfRange {
                sample_rate,
                minimum: SAMPLE_RATE_MINIMUM_HERTZ,
                maximum: SAMPLE_RATE_MAXIMUM_HERTZ,
            });
        }
        if !(1..=2).contains(&channels) {
            return Err(PcmError::UnsupportedChannels { channels });
        }
        Ok(())
    }

    /// The fewest frames a sample at `sample_rate` may hold: 1 ms, rounded up. Never
    /// fewer than two, so a voice always has a frame to interpolate towards.
    pub fn minimum_frames(sample_rate: u32) -> usize {
        ((SAMPLE_SECONDS_MINIMUM * f64::from(sample_rate)).ceil() as usize).max(2)
    }

    /// The most frames a sample at `sample_rate` may hold: 10 s, rounded down.
    pub fn maximum_frames(sample_rate: u32) -> usize {
        (SAMPLE_SECONDS_MAXIMUM * f64::from(sample_rate)) as usize
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 1 for mono, 2 for stereo.
    pub fn channels(&self) -> u8 {
        self.channels
    }

    /// Every frame's values, interleaved.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// The length at its own sample rate, unpitched.
    pub fn seconds(&self) -> f64 {
        self.frame_count() as f64 / f64::from(self.sample_rate)
    }

    /// Whether two values share one decoded buffer, as every voice of one sample does.
    pub fn shares_samples_with(&self, other: &OwnedPcm) -> bool {
        Arc::ptr_eq(&self.samples, &other.samples)
    }
}
