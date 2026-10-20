//! An instrument slot: one synthesizer playing one SoundFont preset for every source that
//! triggers that instrument, its notes kept by a [`VoiceLedger`].
//!
//! Isolation is structural. Two instruments are two synthesizers, so a program change or
//! a note-off on one cannot reach the other; two sources on one instrument are two lanes
//! (MIDI channels), so a note-off from one cannot release the other's note of that key.

use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use super::ledger::{LedgerAction, LedgerCounters, NoteStart, NoteTicket, VoiceLedger};
use crate::error::ConductorError;
use crate::sources::SourceId;

/// rustysynth treats MIDI channel 10 (index 9) as General MIDI percussion and adds 128
/// to any bank selected on it, so a melodic preset cannot live there: lanes skip it.
const PERCUSSION_CHANNEL: usize = Synthesizer::PERCUSSION_CHANNEL;
/// The most lanes a slot can lease: every channel but the percussion one.
pub const MAXIMUM_LANES: usize = Synthesizer::CHANNEL_COUNT - 1;
const CONTROL_CHANGE: i32 = 0xB0;
const PROGRAM_CHANGE: i32 = 0xC0;
const BANK_SELECT: i32 = 0x00;

/// How one slot is built: the conductor's rate and block, and the voice budget.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SlotShape {
    pub sample_rate: u32,
    pub block_size: usize,
    pub polyphony: usize,
    pub sounding_cap: usize,
    pub lanes: usize,
}

/// One instrument's synthesizer and ledger. See the module docs.
pub struct InstrumentSlot {
    synthesizer: Synthesizer,
    ledger: VoiceLedger,
}

/// The MIDI channel a lane plays on.
fn channel_of(lane: usize) -> i32 {
    if lane < PERCUSSION_CHANNEL {
        lane as i32
    } else {
        lane as i32 + 1
    }
}

impl InstrumentSlot {
    /// A slot playing `bank:preset` of `sound_font` on every lane, reverb and chorus off.
    pub(crate) fn new(
        sound_font: &Arc<SoundFont>,
        shape: SlotShape,
        bank: u16,
        preset: u8,
    ) -> Result<Self, ConductorError> {
        let mut synthesizer = new_synthesizer(
            sound_font,
            shape.sample_rate,
            shape.block_size,
            shape.polyphony,
        )?;
        for lane in 0..shape.lanes {
            let channel = channel_of(lane);
            synthesizer.process_midi_message(channel, CONTROL_CHANGE, BANK_SELECT, i32::from(bank));
            synthesizer.process_midi_message(channel, PROGRAM_CHANGE, i32::from(preset), 0);
        }
        Ok(Self {
            synthesizer,
            ledger: VoiceLedger::new(shape.lanes, shape.sounding_cap),
        })
    }

    /// Start `key` for `source`; see [`VoiceLedger::note_on`].
    pub fn note_on(&mut self, source: SourceId, key: u8, velocity: u8) -> NoteStart {
        let synthesizer = &mut self.synthesizer;
        self.ledger
            .note_on(source, key, velocity, |action| apply(synthesizer, action))
    }

    /// Release the note `ticket` started, if it is still sounding.
    pub fn note_off(&mut self, ticket: NoteTicket) {
        let synthesizer = &mut self.synthesizer;
        self.ledger
            .note_off(ticket, |action| apply(synthesizer, action));
    }

    pub fn is_sounding(&self, ticket: NoteTicket) -> bool {
        self.ledger.is_sounding(ticket)
    }

    /// The lane `source` holds, if it has sounding notes.
    pub fn lane_of(&self, source: SourceId) -> Option<usize> {
        self.ledger.lane_of(source)
    }

    /// Release every note `source` holds.
    pub fn release_source(&mut self, source: SourceId) {
        let synthesizer = &mut self.synthesizer;
        self.ledger
            .release_source(source, |action| apply(synthesizer, action));
    }

    /// Release every note of every source; the tails ring out.
    pub fn release_all(&mut self) {
        self.synthesizer.note_off_all(false);
        self.ledger.clear();
    }

    /// Render into `left` and `right` (overwritten; equal lengths).
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synthesizer.render(left, right);
    }

    pub fn sounding_count(&self) -> usize {
        self.ledger.sounding_count()
    }

    pub fn counters(&self) -> LedgerCounters {
        self.ledger.counters()
    }
}

fn apply(synthesizer: &mut Synthesizer, action: LedgerAction) {
    match action {
        LedgerAction::NoteOn {
            lane,
            key,
            velocity,
        } => synthesizer.note_on(channel_of(lane), i32::from(key), i32::from(velocity)),
        LedgerAction::NoteOff { lane, key } => {
            synthesizer.note_off(channel_of(lane), i32::from(key));
        }
    }
}

/// A synthesizer with the conductor's settings: its block is the conductor's grain, and
/// reverb and chorus are off, as the measured budget assumed.
pub(crate) fn new_synthesizer(
    sound_font: &Arc<SoundFont>,
    sample_rate: u32,
    block_size: usize,
    polyphony: usize,
) -> Result<Synthesizer, ConductorError> {
    let sample_rate = i32::try_from(sample_rate).unwrap_or(i32::MAX);
    let mut settings = SynthesizerSettings::new(sample_rate);
    settings.block_size = block_size;
    settings.maximum_polyphony = polyphony;
    settings.enable_reverb_and_chorus = false;
    Synthesizer::new(sound_font, &settings).map_err(|error| ConductorError::Synthesizer {
        reason: error.to_string(),
    })
}
