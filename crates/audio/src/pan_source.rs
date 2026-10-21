//! A recorded sound's stereo pan, changeable while it plays.
//!
//! rodio's `ChannelVolume` fixes its gains when it is built, so a sound that follows its
//! emitter across the screen needs a source of its own: [`PanSource`] wraps a decoder and
//! reads the pan from a [`PanLevel`] — an f32 bit pattern in an atomic, the way the
//! conductor's bus gains cross — once an output frame.
//!
//! **The law is unity at centre**: `left = min(1, 1 − pan)`, `right = min(1, 1 + pan)`.
//! Centred, both gains are exactly one and every sample passes untouched, so a sound
//! played without a pan is what it was before the adapter existed; hard left silences the
//! right channel and leaves the left at full level. Mono input becomes two channels, each
//! sample emitted twice; stereo input is panned channel by channel; input above two
//! channels is reported at its own count and passed through unpanned.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::{Sample, Source};

/// A pan shared between the game thread and a playing source: −1 is hard left, 1 hard
/// right. Stored as written; clamped where it is read.
#[derive(Debug)]
pub(crate) struct PanLevel {
    bits: AtomicU32,
}

impl PanLevel {
    pub(crate) fn new(pan: f32) -> Self {
        Self {
            bits: AtomicU32::new(pan.to_bits()),
        }
    }

    pub(crate) fn set(&self, pan: f32) {
        self.bits.store(pan.to_bits(), Ordering::Release);
    }

    fn get(&self) -> f32 {
        f32::from_bits(self.bits.load(Ordering::Acquire))
    }
}

/// `pan` held to −1..=1, a non-finite pan centred: the fields that carry it are public.
pub(crate) fn clamp_pan(pan: f32) -> f32 {
    if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

/// The left and right gains for `pan` under the unity-at-centre law (see the module docs).
fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = clamp_pan(pan);
    ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0))
}

/// A gain of exactly one leaves the sample as it was, whatever its type.
fn apply_gain<S: Sample>(sample: S, gain: f32) -> S {
    if gain == 1.0 {
        sample
    } else {
        sample.amplify(gain)
    }
}

/// See the module docs.
pub(crate) struct PanSource<I>
where
    I: Source,
    I::Item: Sample,
{
    input: I,
    pan: Arc<PanLevel>,
    /// The frame's gains, read when its left sample is.
    gains: (f32, f32),
    /// A mono sample's right copy, emitted after its left.
    pending_right: Option<I::Item>,
    /// Whether a stereo input's next sample is the right channel's.
    on_right: bool,
}

impl<I> PanSource<I>
where
    I: Source,
    I::Item: Sample,
{
    pub(crate) fn new(input: I, pan: Arc<PanLevel>) -> Self {
        Self {
            input,
            pan,
            gains: (1.0, 1.0),
            pending_right: None,
            on_right: false,
        }
    }
}

impl<I> Iterator for PanSource<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if let Some(right) = self.pending_right.take() {
            return Some(right);
        }
        match self.input.channels() {
            1 => {
                let sample = self.input.next()?;
                let (left, right) = pan_gains(self.pan.get());
                self.pending_right = Some(apply_gain(sample, right));
                Some(apply_gain(sample, left))
            }
            2 => {
                let sample = self.input.next()?;
                let gain = if self.on_right {
                    self.gains.1
                } else {
                    self.gains = pan_gains(self.pan.get());
                    self.gains.0
                };
                self.on_right = !self.on_right;
                Some(apply_gain(sample, gain))
            }
            _ => self.input.next(),
        }
    }
}

impl<I> Source for PanSource<I>
where
    I: Source,
    I::Item: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        let pending = usize::from(self.pending_right.is_some());
        let input_length = self.input.current_frame_len()?;
        if self.input.channels() == 1 {
            Some(input_length * 2 + pending)
        } else {
            Some(input_length + pending)
        }
    }

    fn channels(&self) -> u16 {
        self.input.channels().max(2)
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use rodio::buffer::SamplesBuffer;

    use super::*;

    fn panned<S: Sample + Send + 'static>(channels: u16, samples: Vec<S>, pan: f32) -> (u16, Vec<S>) {
        let source = PanSource::new(SamplesBuffer::new(channels, 44_100, samples), Arc::new(PanLevel::new(pan)));
        (source.channels(), source.collect())
    }

    #[test]
    fn a_mono_sample_is_doubled_and_panned_with_unity_at_centre() {
        let mono = vec![0.5_f32, -0.25];
        assert_eq!(panned(1, mono.clone(), -1.0), (2, vec![0.5, 0.0, -0.25, 0.0]));
        assert_eq!(panned(1, mono.clone(), 0.0), (2, vec![0.5, 0.5, -0.25, -0.25]));
        assert_eq!(panned(1, mono.clone(), 1.0), (2, vec![0.0, 0.5, 0.0, -0.25]));
        assert_eq!(panned(1, mono, 0.5), (2, vec![0.25, 0.5, -0.125, -0.25]), "half right halves the left");
    }

    #[test]
    fn a_stereo_frame_is_panned_channel_by_channel() {
        let stereo = vec![0.5_f32, 0.25, -0.5, -0.25];
        assert_eq!(panned(2, stereo.clone(), -1.0), (2, vec![0.5, 0.0, -0.5, 0.0]));
        assert_eq!(panned(2, stereo.clone(), 0.0), (2, stereo.clone()));
        assert_eq!(panned(2, stereo, 1.0), (2, vec![0.0, 0.25, 0.0, -0.25]));
    }

    #[test]
    fn a_centred_play_passes_every_sample_untouched() {
        let decoded = vec![i16::MIN, -3, 0, 7, i16::MAX, 12_345];
        assert_eq!(panned(2, decoded.clone(), 0.0).1, decoded);
        assert_eq!(crate::SoundSettings::default().pan, 0.0, "a default play is centred");
    }

    #[test]
    fn six_channels_are_reported_as_six_and_passed_through_unpanned() {
        let surround: Vec<f32> = (0..12).map(|index| index as f32 / 12.0).collect();
        assert_eq!(panned(6, surround.clone(), 1.0), (6, surround));
    }

    #[test]
    fn a_pan_beyond_the_range_is_clamped_and_a_non_finite_one_centred() {
        let mono = vec![0.5_f32];
        assert_eq!(panned(1, mono.clone(), 7.0), panned(1, mono.clone(), 1.0));
        assert_eq!(panned(1, mono.clone(), -7.0), panned(1, mono.clone(), -1.0));
        assert_eq!(panned(1, mono.clone(), f32::NAN), panned(1, mono, 0.0));
    }

    #[test]
    fn a_pan_written_while_the_sound_plays_reaches_the_next_frame() {
        let level = Arc::new(PanLevel::new(0.0));
        let mut source = PanSource::new(SamplesBuffer::new(1, 44_100, vec![0.5_f32, 0.5]), Arc::clone(&level));
        assert_eq!((source.next(), source.next()), (Some(0.5), Some(0.5)));
        level.set(1.0);
        assert_eq!((source.next(), source.next()), (Some(0.0), Some(0.5)));
    }
}
