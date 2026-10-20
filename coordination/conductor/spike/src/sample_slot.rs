//! A sample instrument: a user WAV decoded once, outside rendering, into owned PCM, then
//! played as voices pitched from a root key by linear interpolation.
//!
//! Pitching by resampling changes duration with pitch (an octave up plays twice as
//! fast, so half as long). There is no time stretching here, and nothing claims any.
//! There is no interpolation filter either, so pitching up aliases.

use std::fmt;
use std::io::Cursor;
use std::sync::Arc;

use rodio::{Decoder, Source};

use crate::instrument_slot::SourceKey;

/// Decoded samples, interleaved, owned, shareable with the render thread.
#[derive(Debug, Clone)]
pub struct OwnedPcm {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: Arc<[f32]>,
}

impl OwnedPcm {
    pub fn frame_count(&self) -> usize {
        self.frames.len() / usize::from(self.channels)
    }
}

/// The limits a sample is held to before anything large is built.
#[derive(Debug, Clone, Copy)]
pub struct SampleLimits {
    pub maximum_bytes: usize,
    pub maximum_seconds: f32,
    pub minimum_sample_rate: u32,
    pub maximum_sample_rate: u32,
}

/// The proposed limits: 4 MiB of file, 10 s decoded, 8–96 kHz.
pub const PROPOSED_LIMITS: SampleLimits = SampleLimits {
    maximum_bytes: 4 * 1024 * 1024,
    maximum_seconds: 10.0,
    minimum_sample_rate: 8_000,
    maximum_sample_rate: 96_000,
};

#[derive(Debug, Clone, PartialEq)]
pub enum SampleError {
    TooManyBytes(usize),
    Undecodable(String),
    UnsupportedChannels(u16),
    UnsupportedSampleRate(u32),
    TooLong { limit_seconds: f32 },
    NonFiniteSample,
}

impl fmt::Display for SampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyBytes(bytes) => write!(formatter, "the file is {bytes} bytes, over the limit"),
            Self::Undecodable(reason) => write!(formatter, "not a decodable sound: {reason}"),
            Self::UnsupportedChannels(channels) => write!(formatter, "{channels} channels; mono or stereo only"),
            Self::UnsupportedSampleRate(rate) => write!(formatter, "{rate} Hz is outside 8–96 kHz"),
            Self::TooLong { limit_seconds } => write!(formatter, "longer than {limit_seconds} s decoded"),
            Self::NonFiniteSample => write!(formatter, "a decoded sample is not a finite number"),
        }
    }
}

/// Decode a sound file's bytes into [`OwnedPcm`]. The byte limit is checked before
/// parsing and the decode stops one sample past the length limit, so an oversized file
/// is never decoded in full only to be rejected.
pub fn decode_sample(bytes: Vec<u8>, limits: SampleLimits) -> Result<OwnedPcm, SampleError> {
    if bytes.len() > limits.maximum_bytes {
        return Err(SampleError::TooManyBytes(bytes.len()));
    }
    let decoder = Decoder::new(Cursor::new(bytes)).map_err(|error| SampleError::Undecodable(error.to_string()))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    if !(1..=2).contains(&channels) {
        return Err(SampleError::UnsupportedChannels(channels));
    }
    if !(limits.minimum_sample_rate..=limits.maximum_sample_rate).contains(&sample_rate) {
        return Err(SampleError::UnsupportedSampleRate(sample_rate));
    }
    let maximum_samples = (limits.maximum_seconds * sample_rate as f32) as usize * usize::from(channels);
    let samples: Vec<f32> = decoder.convert_samples::<f32>().take(maximum_samples + 1).collect();
    if samples.len() > maximum_samples {
        return Err(SampleError::TooLong { limit_seconds: limits.maximum_seconds });
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(SampleError::NonFiniteSample);
    }
    Ok(OwnedPcm { sample_rate, channels, frames: samples.into() })
}

/// Whether a note-off ends a voice or the sample plays to its end regardless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    OneShot,
    Gated,
}

/// The farthest a sample is pitched from its root, either way.
pub const MAXIMUM_SEMITONES_FROM_ROOT: i32 = 24;
/// A voice that is cut (stolen, or gated off) fades over this long instead of clicking.
pub const FADE_SECONDS: f32 = 0.005;

#[derive(Debug, Clone)]
struct SampleVoice {
    source: SourceKey,
    key: i32,
    position_frames: f64,
    step_frames: f64,
    gain: f32,
    fade_remaining_frames: Option<u32>,
}

/// Running totals a measurement reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SampleCounters {
    pub voices_started: u64,
    pub steals: u64,
    pub peak_live_voices: usize,
}

pub struct SampleSlot {
    pcm: OwnedPcm,
    root_key: i32,
    mode: TriggerMode,
    output_sample_rate: u32,
    live_voice_cap: usize,
    fade_frames: u32,
    /// Oldest first.
    voices: Vec<SampleVoice>,
    counters: SampleCounters,
}

impl SampleSlot {
    pub fn new(pcm: OwnedPcm, root_key: i32, mode: TriggerMode, output_sample_rate: u32, live_voice_cap: usize) -> Self {
        Self {
            pcm,
            root_key,
            mode,
            output_sample_rate,
            live_voice_cap,
            fade_frames: (FADE_SECONDS * output_sample_rate as f32).round() as u32,
            voices: Vec::with_capacity(live_voice_cap * 2),
            counters: SampleCounters::default(),
        }
    }

    /// Source frames advanced per output frame at `key`: `2^(semitones / 12) ·
    /// source_rate / output_rate`, the semitones clamped to ±24 from the root.
    pub fn step_for_key(&self, key: i32) -> f64 {
        let semitones = (key - self.root_key).clamp(-MAXIMUM_SEMITONES_FROM_ROOT, MAXIMUM_SEMITONES_FROM_ROOT);
        2f64.powf(f64::from(semitones) / 12.0) * f64::from(self.pcm.sample_rate) / f64::from(self.output_sample_rate)
    }

    /// The exact length, in output frames, of a one-shot at `key`.
    pub fn duration_frames_for_key(&self, key: i32) -> f64 {
        (self.pcm.frame_count() - 1) as f64 / self.step_for_key(key)
    }

    pub fn note_on(&mut self, source: SourceKey, key: i32, velocity: i32) {
        let live_voices = self.live_voice_count();
        if live_voices >= self.live_voice_cap {
            if let Some(oldest) = self.voices.iter_mut().find(|voice| voice.fade_remaining_frames.is_none()) {
                oldest.fade_remaining_frames = Some(self.fade_frames);
                self.counters.steals += 1;
            }
        }
        self.voices.push(SampleVoice {
            source,
            key,
            position_frames: 0.0,
            step_frames: self.step_for_key(key),
            gain: velocity.clamp(0, 127) as f32 / 127.0,
            fade_remaining_frames: None,
        });
        self.counters.voices_started += 1;
        self.counters.peak_live_voices = self.counters.peak_live_voices.max(self.live_voice_count());
    }

    /// A gated voice fades out over [`FADE_SECONDS`]; a one-shot ignores the note-off.
    pub fn note_off(&mut self, source: SourceKey, key: i32) {
        if self.mode == TriggerMode::OneShot {
            return;
        }
        for voice in &mut self.voices {
            if voice.source == source && voice.key == key && voice.fade_remaining_frames.is_none() {
                voice.fade_remaining_frames = Some(self.fade_frames);
            }
        }
    }

    pub fn release_all(&mut self) {
        for voice in &mut self.voices {
            voice.fade_remaining_frames.get_or_insert(self.fade_frames);
        }
    }

    /// Voices not fading out.
    pub fn live_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.fade_remaining_frames.is_none()).count()
    }

    /// Every voice still producing sound, fading ones included.
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn counters(&self) -> SampleCounters {
        self.counters
    }

    /// Mix every voice into `left` and `right` (added to what is there).
    pub fn render_add(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frame_count = self.pcm.frame_count();
        let channels = usize::from(self.pcm.channels);
        let samples = &self.pcm.frames;
        let fade_frames = self.fade_frames.max(1) as f32;
        for voice in &mut self.voices {
            for (left_out, right_out) in left.iter_mut().zip(right.iter_mut()) {
                let whole = voice.position_frames as usize;
                if whole + 1 >= frame_count || voice.fade_remaining_frames == Some(0) {
                    voice.fade_remaining_frames = Some(0);
                    break;
                }
                let fraction = (voice.position_frames - whole as f64) as f32;
                let fade_gain = voice.fade_remaining_frames.map_or(1.0, |remaining| remaining as f32 / fade_frames);
                let gain = voice.gain * fade_gain;
                let left_now = samples[whole * channels];
                let left_next = samples[(whole + 1) * channels];
                let (right_now, right_next) = if channels == 2 {
                    (samples[whole * channels + 1], samples[(whole + 1) * channels + 1])
                } else {
                    (left_now, left_next)
                };
                *left_out += (left_now + (left_next - left_now) * fraction) * gain;
                *right_out += (right_now + (right_next - right_now) * fraction) * gain;
                voice.position_frames += voice.step_frames;
                if let Some(remaining) = voice.fade_remaining_frames.as_mut() {
                    *remaining -= 1;
                }
            }
        }
        self.voices.retain(|voice| voice.fade_remaining_frames != Some(0));
    }
}
