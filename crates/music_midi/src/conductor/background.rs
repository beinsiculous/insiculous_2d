//! The background player: the score's sixteen channels on a pair of synthesizers that
//! take turns, driven by the [`Sequencer`](crate::Sequencer) as its sink.
//!
//! **Why two.** A synthesizer applies a message only at the start of its next internal
//! block, and nothing in rustysynth moves that block grid except `reset`, which also cuts
//! every voice dead. So a reconstruction (a seek, a loop wrap, a resume) releases every
//! note on the playing synthesizer, which keeps rendering only its tails, and starts the
//! other one fresh: reset, so its first block starts on the reconstruction's exact frame,
//! then given the folded channel state. The loop's downbeat lands on its exact sample and
//! the tails ring out undisturbed. A tail still ringing on the idle synthesizer when the
//! next reconstruction resets it is cut — one that outlives a whole loop or comes from two
//! seeks back.

use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer};

use super::slots::new_synthesizer;
use crate::error::ConductorError;
use crate::sequencer::{ChannelMessage, ChannelStateFold, SequencerSink, CHANNEL_COUNT};

const NOTE_OFF: i32 = 0x80;
const NOTE_ON: i32 = 0x90;
const CONTROL_CHANGE: i32 = 0xB0;
const PROGRAM_CHANGE: i32 = 0xC0;
const PITCH_BEND: i32 = 0xE0;

/// See the module docs.
pub(crate) struct BackgroundPlayer {
    synthesizers: [Synthesizer; 2],
    /// The synthesizer the score's messages go to; the other only rings out tails.
    active: usize,
    left: Vec<f32>,
    right: Vec<f32>,
    scratch_left: Vec<f32>,
    scratch_right: Vec<f32>,
    /// Frames of the current block written to `left` and `right` so far.
    written: usize,
}

impl BackgroundPlayer {
    pub(crate) fn new(
        sound_font: &Arc<SoundFont>,
        sample_rate: u32,
        block_size: usize,
        polyphony: usize,
    ) -> Result<Self, ConductorError> {
        let synthesizer = || new_synthesizer(sound_font, sample_rate, block_size, polyphony);
        Ok(Self {
            synthesizers: [synthesizer()?, synthesizer()?],
            active: 0,
            left: vec![0.0; block_size],
            right: vec![0.0; block_size],
            scratch_left: vec![0.0; block_size],
            scratch_right: vec![0.0; block_size],
            written: 0,
        })
    }

    /// Start a block: what [`SequencerSink::render`] renders next lands at its start.
    pub(crate) fn begin_block(&mut self) {
        self.written = 0;
    }

    /// The frames rendered since [`BackgroundPlayer::begin_block`].
    pub(crate) fn output(&self) -> (&[f32], &[f32]) {
        (&self.left[..self.written], &self.right[..self.written])
    }

    fn send_to(synthesizer: &mut Synthesizer, channel: u8, message: ChannelMessage) {
        let channel = i32::from(channel);
        let (command, first, second) = match message {
            ChannelMessage::NoteOn { key, velocity } => (NOTE_ON, key.into(), velocity.into()),
            ChannelMessage::NoteOff { key } => (NOTE_OFF, key.into(), 0),
            ChannelMessage::Controller { controller, value } => {
                (CONTROL_CHANGE, controller.into(), value.into())
            }
            ChannelMessage::Program { program } => (PROGRAM_CHANGE, program.into(), 0),
            ChannelMessage::PitchBend { value } => {
                (PITCH_BEND, i32::from(value & 0x7F), i32::from(value >> 7))
            }
        };
        synthesizer.process_midi_message(channel, command, first, second);
    }
}

impl SequencerSink for BackgroundPlayer {
    fn send(&mut self, channel: u8, message: ChannelMessage) {
        Self::send_to(&mut self.synthesizers[self.active], channel, message);
    }

    fn reconstruct(&mut self, state: &ChannelStateFold) {
        self.synthesizers[self.active].note_off_all(false);
        self.active = 1 - self.active;
        let incoming = &mut self.synthesizers[self.active];
        incoming.reset();
        for channel in 0..CHANNEL_COUNT as u8 {
            state.replay(channel, |message| Self::send_to(incoming, channel, message));
        }
    }

    fn release_all(&mut self) {
        for synthesizer in &mut self.synthesizers {
            synthesizer.note_off_all(false);
        }
    }

    /// Render both synthesizers, summed, after what this block already holds. A request
    /// past the block's end renders only what fits.
    fn render(&mut self, frames: usize) {
        let frames = frames.min(self.left.len() - self.written);
        let range = self.written..self.written + frames;
        let scratch_left = &mut self.scratch_left[..frames];
        let scratch_right = &mut self.scratch_right[..frames];
        let [first, second] = &mut self.synthesizers;
        first.render(
            &mut self.left[range.clone()],
            &mut self.right[range.clone()],
        );
        second.render(scratch_left, scratch_right);
        for (output, tail) in self.left[range.clone()].iter_mut().zip(scratch_left.iter()) {
            *output += tail;
        }
        for (output, tail) in self.right[range].iter_mut().zip(scratch_right.iter()) {
            *output += tail;
        }
        self.written += frames;
    }
}
