//! Everything the audio thread owns: the background score, the instrument slots and the
//! sample slot, mixed one 64-frame block at a time. The game thread never touches any of
//! it; it sends [`SpikeCommand`]s, which the source applies between blocks.

use crate::instrument_slot::{InstrumentSlot, SourceKey};
use crate::sample_slot::SampleSlot;
use crate::score::BackgroundScore;

/// The frames the conductor renders between two command drains.
pub const BLOCK_FRAMES: usize = 64;

/// A fixed mix level. Nine synthesizers at rustysynth's own master volume summed at unity
/// would clip on the first loud chord; gain staging is the song manager's job, not the
/// spike's.
const MIX_GAIN: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotTarget {
    Instrument(usize),
    Sample,
}

/// A message from the game thread to the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpikeCommand {
    NoteOn { target: SlotTarget, source: SourceKey, key: u8, velocity: u8 },
    NoteOff { target: SlotTarget, source: SourceKey, key: u8 },
    ProgramChange { slot: usize, bank: u8, program: u8 },
    Seek { tick: u64 },
    ReleaseAll,
    /// End the source: the next sample pulled is `None`, and rodio frees the sink.
    Stop,
}

/// What one rendered block looked like.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BlockReport {
    pub sounding_notes: usize,
    pub sample_voices: usize,
    pub non_finite_samples: u32,
    pub largest_step: f32,
}

pub struct Mixer {
    pub background: Option<BackgroundScore>,
    pub slots: Vec<InstrumentSlot>,
    pub samples: Option<SampleSlot>,
    stopped: bool,
    scratch_left: Vec<f32>,
    scratch_right: Vec<f32>,
    previous_left: f32,
    previous_right: f32,
}

impl Mixer {
    pub fn new(background: Option<BackgroundScore>, slots: Vec<InstrumentSlot>, samples: Option<SampleSlot>) -> Self {
        Self {
            background,
            slots,
            samples,
            stopped: false,
            scratch_left: vec![0.0; BLOCK_FRAMES],
            scratch_right: vec![0.0; BLOCK_FRAMES],
            previous_left: 0.0,
            previous_right: 0.0,
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn apply(&mut self, command: SpikeCommand) {
        match command {
            SpikeCommand::NoteOn { target: SlotTarget::Instrument(slot), source, key, velocity } => {
                if let Some(slot) = self.slots.get_mut(slot) {
                    slot.note_on(source, i32::from(key), i32::from(velocity));
                }
            }
            SpikeCommand::NoteOff { target: SlotTarget::Instrument(slot), source, key } => {
                if let Some(slot) = self.slots.get_mut(slot) {
                    slot.note_off(source, i32::from(key));
                }
            }
            SpikeCommand::NoteOn { target: SlotTarget::Sample, source, key, velocity } => {
                if let Some(samples) = self.samples.as_mut() {
                    samples.note_on(source, i32::from(key), i32::from(velocity));
                }
            }
            SpikeCommand::NoteOff { target: SlotTarget::Sample, source, key } => {
                if let Some(samples) = self.samples.as_mut() {
                    samples.note_off(source, i32::from(key));
                }
            }
            SpikeCommand::ProgramChange { slot, bank, program } => {
                if let Some(slot) = self.slots.get_mut(slot) {
                    slot.program_change(i32::from(bank), i32::from(program));
                }
            }
            SpikeCommand::Seek { tick } => {
                if let Some(background) = self.background.as_mut() {
                    background.seek(tick);
                }
            }
            SpikeCommand::ReleaseAll => {
                for slot in &mut self.slots {
                    slot.release_all();
                }
                if let Some(samples) = self.samples.as_mut() {
                    samples.release_all();
                }
            }
            SpikeCommand::Stop => self.stopped = true,
        }
    }

    /// Render one block into `left` and `right` (overwritten, [`BLOCK_FRAMES`] long or
    /// shorter). Every output sample is finite: a non-finite one becomes 0 and is counted,
    /// so nothing downstream (WebAudio least of all) is ever handed a NaN.
    pub fn render_block(&mut self, left: &mut [f32], right: &mut [f32]) -> BlockReport {
        let frames = left.len();
        left.fill(0.0);
        right.fill(0.0);
        let scratch_left = &mut self.scratch_left[..frames];
        let scratch_right = &mut self.scratch_right[..frames];
        if let Some(background) = self.background.as_mut() {
            background.render(scratch_left, scratch_right);
            add_into(left, right, scratch_left, scratch_right);
        }
        for slot in &mut self.slots {
            slot.render(scratch_left, scratch_right);
            add_into(left, right, scratch_left, scratch_right);
        }
        if let Some(samples) = self.samples.as_mut() {
            samples.render_add(left, right);
        }
        let mut report = BlockReport {
            sounding_notes: self.slots.iter().map(InstrumentSlot::sounding_count).sum(),
            sample_voices: self.samples.as_ref().map_or(0, SampleSlot::voice_count),
            ..BlockReport::default()
        };
        for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
            for (sample, previous) in [(left_sample, &mut self.previous_left), (right_sample, &mut self.previous_right)] {
                *sample *= MIX_GAIN;
                if !sample.is_finite() {
                    *sample = 0.0;
                    report.non_finite_samples += 1;
                }
                report.largest_step = report.largest_step.max((*sample - *previous).abs());
                *previous = *sample;
            }
        }
        report
    }
}

fn add_into(left: &mut [f32], right: &mut [f32], from_left: &[f32], from_right: &[f32]) {
    for (target, source) in left.iter_mut().zip(from_left) {
        *target += source;
    }
    for (target, source) in right.iter_mut().zip(from_right) {
        *target += source;
    }
}
