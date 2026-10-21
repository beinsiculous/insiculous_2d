//! One sample voice: a decoded sample played from its start, pitched from a root note by
//! linear interpolation, until its pitch-scaled end or the end of its fade.
//!
//! **Pitch is speed.** A voice steps `2^(semitones / 12) × source_rate / output_rate`
//! source frames per output frame, the semitones clamped to ±24, so a sample an octave up
//! plays twice as fast and lasts half as long. There is no time stretch and no
//! interpolation filter: pitching up aliases.
//!
//! **Ending.** Every end is a linear fade of
//! [`SAMPLE_FADE_SECONDS`](crate::SAMPLE_FADE_SECONDS): a gated note's end, every forced
//! termination, and the sample's own end — a voice within one fade of its last frame
//! starts fading so it reaches silence on that frame, whatever level the recording ends
//! at. That shorter ramp runs from full level over its own length, so a sample shorter
//! than the fade is not quieter for it. Nothing a voice does steps the output.

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
    /// The fade's whole length in output frames: what `fade_remaining_frames` is a
    /// fraction of. A fade started within one fade of the sample's end is shorter than
    /// the slot's, and the ramp is its own length, so it starts at full level.
    fade_length_frames: f32,
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
            fade_length_frames: 1.0,
        }
    }

    /// Start fading over `fade_frames`; a voice already fading keeps its fade.
    pub(crate) fn fade(&mut self, fade_frames: u32) {
        self.start_fade(fade_frames);
    }

    fn start_fade(&mut self, fade_frames: u32) {
        if self.fade_remaining_frames.is_none() {
            self.fade_remaining_frames = Some(fade_frames);
            self.fade_length_frames = fade_frames.max(1) as f32;
        }
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
        for (left_out, right_out) in left.iter_mut().zip(right.iter_mut()) {
            let whole = self.position_frames as usize;
            if whole >= last_frame || self.fade_remaining_frames == Some(0) {
                return false;
            }
            if self.fade_remaining_frames.is_none() {
                let frames_to_end = (last_frame as f64 - self.position_frames) / self.step_frames;
                if frames_to_end <= f64::from(fade_frames) {
                    // Started at the frames left rather than a whole fade, so the ramp
                    // lands on silence at the last frame instead of being cut by it.
                    if frames_to_end < 1.0 {
                        return false;
                    }
                    self.start_fade(frames_to_end.floor() as u32);
                }
            }
            let fraction = (self.position_frames - whole as f64) as f32;
            let envelope = self
                .fade_remaining_frames
                .map_or(1.0, |remaining| remaining as f32 / self.fade_length_frames);
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
