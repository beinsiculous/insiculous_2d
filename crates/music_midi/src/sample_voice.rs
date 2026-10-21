//! One sample voice: a decoded sample played from its start, pitched from a root note by
//! linear interpolation, until its pitch-scaled end or the end of its fade.
//!
//! **Pitch is speed.** A voice steps `2^(semitones / 12) × source_rate / output_rate`
//! source frames per output frame, the semitones clamped to ±24, so a sample an octave up
//! plays twice as fast and lasts half as long. There is no time stretch and no
//! interpolation filter: pitching up aliases.
//!
//! **Ending.** A voice ends at the last source frame it can interpolate towards, or at the
//! end of a linear fade of [`SAMPLE_FADE_SECONDS`](crate::SAMPLE_FADE_SECONDS) once one is
//! started — a gated note's end and every forced termination start the same fade, so
//! nothing a voice does steps the output.

use crate::conductor::NoteTicket;
use crate::pcm::OwnedPcm;
use crate::sources::SourceId;
use crate::SAMPLE_SEMITONE_RANGE;

/// Source frames a voice advances per output frame at `key`, the semitones from
/// `root_note` clamped to the sample range.
pub(crate) fn step_frames(root_note: u8, key: u8, source_rate: u32, output_rate: u32) -> f64 {
    let range = i32::from(SAMPLE_SEMITONE_RANGE);
    let semitones = (i32::from(key) - i32::from(root_note)).clamp(-range, range);
    2f64.powf(f64::from(semitones) / 12.0) * f64::from(source_rate) / f64::from(output_rate)
}

/// One playing voice. See the module docs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SampleVoice {
    pub source: SourceId,
    pub ticket: NoteTicket,
    position_frames: f64,
    step_frames: f64,
    gain: f32,
    /// Output frames left in the fade, once it has started.
    fade_remaining_frames: Option<u32>,
}

impl SampleVoice {
    pub(crate) fn new(source: SourceId, ticket: NoteTicket, step_frames: f64, gain: f32) -> Self {
        Self {
            source,
            ticket,
            position_frames: 0.0,
            step_frames,
            gain,
            fade_remaining_frames: None,
        }
    }

    /// Start fading over `fade_frames`; a voice already fading keeps its fade.
    pub(crate) fn fade(&mut self, fade_frames: u32) {
        self.fade_remaining_frames.get_or_insert(fade_frames);
    }

    pub(crate) fn is_fading(&self) -> bool {
        self.fade_remaining_frames.is_some()
    }

    /// Add this voice into `left` and `right` (equal lengths), mono to both channels and
    /// stereo kept. Returns whether it still plays after them.
    pub(crate) fn render_add(
        &mut self,
        pcm: &OwnedPcm,
        fade_frames: u32,
        left: &mut [f32],
        right: &mut [f32],
    ) -> bool {
        let samples = pcm.samples();
        let channels = usize::from(pcm.channels());
        // The 1 ms floor guarantees two frames, so there is always one to step towards.
        let last_frame = pcm.frame_count() - 1;
        let fade_length = fade_frames.max(1) as f32;
        for (left_out, right_out) in left.iter_mut().zip(right.iter_mut()) {
            let whole = self.position_frames as usize;
            if whole >= last_frame || self.fade_remaining_frames == Some(0) {
                return false;
            }
            let fraction = (self.position_frames - whole as f64) as f32;
            let envelope = self
                .fade_remaining_frames
                .map_or(1.0, |remaining| remaining as f32 / fade_length);
            let gain = self.gain * envelope;
            let here = whole * channels;
            let next = here + channels;
            let interpolate = |channel: usize| {
                let from = samples[here + channel];
                from + (samples[next + channel] - from) * fraction
            };
            let left_value = interpolate(0);
            let right_value = if channels == 2 {
                interpolate(1)
            } else {
                left_value
            };
            *left_out += left_value * gain;
            *right_out += right_value * gain;
            self.position_frames += self.step_frames;
            if let Some(remaining) = self.fade_remaining_frames.as_mut() {
                *remaining -= 1;
            }
        }
        self.fade_remaining_frames != Some(0) && (self.position_frames as usize) < last_frame
    }
}
