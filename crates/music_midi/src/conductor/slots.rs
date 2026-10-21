//! A song's instrument slots, one per instrument: an [`InstrumentSlot`] — one synthesizer
//! playing one SoundFont preset for every source that triggers that instrument, its notes
//! kept by a [`VoiceLedger`] — or a [`SampleSlot`]. A trigger plays either through one
//! [`Slot`], so scheduled note-offs, a source's release, a wrap and the transport treat
//! both kinds alike.
//!
//! Isolation is structural. Two instruments are two slots, so a program change or a
//! note-off on one cannot reach the other; two sources on one SoundFont instrument are two
//! lanes (MIDI channels), so a note-off from one cannot release the other's note of that
//! key, and two sources on one sample hold voices of their own.

use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use super::ledger::{LedgerAction, LedgerCounters, NoteStart, NoteTicket, VoiceLedger};
use super::sample_slot::SampleSlot;
use crate::error::ConductorError;
use crate::sources::SourceId;
use crate::triggers::TriggerSink;

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

/// One instrument's slot, of either kind. A synthesizer is several hundred bytes and a
/// sample slot a few dozen, so the synthesizer is boxed, once, at load.
pub(crate) enum Slot {
    Synth(Box<InstrumentSlot>),
    Sample(SampleSlot),
}

impl Slot {
    fn note_on(&mut self, source: SourceId, key: u8, velocity: u8) -> NoteTicket {
        match self {
            Slot::Synth(slot) => slot.note_on(source, key, velocity).ticket,
            Slot::Sample(slot) => slot.note_on(source, key, velocity),
        }
    }

    fn note_off(&mut self, ticket: NoteTicket) {
        match self {
            Slot::Synth(slot) => slot.note_off(ticket),
            Slot::Sample(slot) => slot.note_off(ticket),
        }
    }

    fn is_sounding(&self, ticket: NoteTicket) -> bool {
        match self {
            Slot::Synth(slot) => slot.is_sounding(ticket),
            Slot::Sample(slot) => slot.is_sounding(ticket),
        }
    }

    fn release_source(&mut self, source: SourceId) {
        match self {
            Slot::Synth(slot) => slot.release_source(source),
            Slot::Sample(slot) => slot.release_source(source),
        }
    }

    /// End every note: a synthesizer's ring out as tails, a sample's voices fade.
    pub(crate) fn release_all(&mut self) {
        match self {
            Slot::Synth(slot) => slot.release_all(),
            Slot::Sample(slot) => slot.release_all(),
        }
    }

    /// Add this slot's next `mix_left.len()` frames into the mix. A synthesizer renders
    /// into the scratch pair first, since it overwrites what it renders into.
    pub(crate) fn mix_into(
        &mut self,
        mix_left: &mut [f32],
        mix_right: &mut [f32],
        scratch_left: &mut [f32],
        scratch_right: &mut [f32],
    ) {
        match self {
            Slot::Synth(slot) => {
                slot.render(scratch_left, scratch_right);
                for (mixed, rendered) in mix_left.iter_mut().zip(scratch_left.iter()) {
                    *mixed += rendered;
                }
                for (mixed, rendered) in mix_right.iter_mut().zip(scratch_right.iter()) {
                    *mixed += rendered;
                }
            }
            Slot::Sample(slot) => slot.render_add(mix_left, mix_right),
        }
    }

    /// Sample voices playing, fading ones included; none for a synthesizer.
    pub(crate) fn sample_voice_count(&self) -> usize {
        match self {
            Slot::Synth(_) => 0,
            Slot::Sample(slot) => slot.voice_count(),
        }
    }
}

/// The instrument slots as a trigger engine plays them.
pub(crate) struct SlotSink<'a>(pub &'a mut [Slot]);

impl TriggerSink for SlotSink<'_> {
    fn note_on(
        &mut self,
        slot: usize,
        source: SourceId,
        key: u8,
        velocity: u8,
    ) -> Option<NoteTicket> {
        Some(self.0.get_mut(slot)?.note_on(source, key, velocity))
    }

    fn note_off(&mut self, slot: usize, ticket: NoteTicket) {
        if let Some(slot) = self.0.get_mut(slot) {
            slot.note_off(ticket);
        }
    }

    fn is_sounding(&self, slot: usize, ticket: NoteTicket) -> bool {
        self.0
            .get(slot)
            .is_some_and(|slot| slot.is_sounding(ticket))
    }

    fn release_source(&mut self, source: SourceId) {
        for slot in self.0.iter_mut() {
            slot.release_source(source);
        }
    }
}
