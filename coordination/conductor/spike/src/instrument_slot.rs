//! An instrument slot: one synthesizer playing one preset for every source that
//! triggers it, with a ledger of the notes it holds.
//!
//! Isolation comes from the structure. Two instruments are two synthesizers, so a
//! program change or a note-off on one cannot reach the other. Two sources on one
//! instrument are two MIDI channels ("lanes"), so `note_off(channel, key)` — which in
//! rustysynth releases every voice matching the pair — cannot cross from one source to
//! the other. The ledger knows exactly which notes are sounding because it issued every
//! note-on and note-off, so it can keep the synthesizer's own stealer (a hard restart,
//! no fade) away from sounding notes: at the cap it releases its oldest note itself.

use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerError, SynthesizerSettings};

/// Identifies whoever triggers notes on a slot (an enemy, a player, the keyboard).
pub type SourceKey = u32;

/// rustysynth treats channel index 9 as General MIDI percussion and adds 128 to any bank
/// selected on it, so a melodic preset cannot live there. Lanes skip it.
pub const PERCUSSION_CHANNEL: usize = 9;
/// The channels a slot leases to sources: all sixteen but the percussion channel.
pub const LANE_COUNT: usize = Synthesizer::CHANNEL_COUNT - 1;

const PROGRAM_CHANGE: i32 = 0xC0;
const CONTROL_CHANGE: i32 = 0xB0;
const BANK_SELECT: i32 = 0x00;

/// The finite capacity of one slot.
#[derive(Debug, Clone, Copy)]
pub struct SlotCapacity {
    /// Notes the ledger lets sound at once; the next note-on releases the oldest.
    pub sounding_cap: usize,
    /// The synthesizer's voice pool, which also holds release tails.
    pub maximum_polyphony: usize,
}

/// What a note-on did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteOutcome {
    Started,
    /// The same source already held this key: the old note was released first.
    StartedAfterRetrigger,
    /// The slot was at its sounding cap: its oldest note was released to make room.
    StartedAfterSteal,
    /// Every lane is leased to a source with sounding notes.
    RejectedNoFreeLane,
}

/// Running totals a measurement reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotCounters {
    pub notes_started: u64,
    pub steals: u64,
    pub retriggers: u64,
    pub lane_rejections: u64,
    pub peak_sounding: usize,
}

#[derive(Debug, Clone, Copy)]
struct SoundingNote {
    source: SourceKey,
    lane: usize,
    key: i32,
}

pub struct InstrumentSlot {
    synthesizer: Synthesizer,
    lane_lessees: [Option<SourceKey>; LANE_COUNT],
    /// Oldest first.
    sounding: Vec<SoundingNote>,
    sounding_cap: usize,
    counters: SlotCounters,
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
    /// A slot playing `bank:program` from `sound_font` at `sample_rate`, rendering in
    /// 64-frame blocks with reverb and chorus off.
    pub fn new(
        sound_font: &Arc<SoundFont>,
        sample_rate: i32,
        capacity: SlotCapacity,
        bank: i32,
        program: i32,
    ) -> Result<Self, SynthesizerError> {
        let mut settings = SynthesizerSettings::new(sample_rate);
        settings.block_size = 64;
        settings.maximum_polyphony = capacity.maximum_polyphony;
        settings.enable_reverb_and_chorus = false;
        let mut slot = Self {
            synthesizer: Synthesizer::new(sound_font, &settings)?,
            lane_lessees: [None; LANE_COUNT],
            sounding: Vec::with_capacity(capacity.sounding_cap),
            sounding_cap: capacity.sounding_cap,
            counters: SlotCounters::default(),
        };
        slot.program_change(bank, program);
        Ok(slot)
    }

    /// Select `bank:program` on every lane. Sounding notes keep the preset they started
    /// with; the next note-on uses the new one.
    pub fn program_change(&mut self, bank: i32, program: i32) {
        for lane in 0..LANE_COUNT {
            let channel = channel_of(lane);
            self.synthesizer.process_midi_message(channel, CONTROL_CHANGE, BANK_SELECT, bank);
            self.synthesizer.process_midi_message(channel, PROGRAM_CHANGE, program, 0);
        }
    }

    /// Start `key` for `source`.
    pub fn note_on(&mut self, source: SourceKey, key: i32, velocity: i32) -> NoteOutcome {
        let Some(lane) = self.lease_lane(source) else {
            self.counters.lane_rejections += 1;
            return NoteOutcome::RejectedNoFreeLane;
        };
        let mut outcome = NoteOutcome::Started;
        if let Some(index) = self.sounding.iter().position(|note| note.lane == lane && note.key == key) {
            self.sounding.remove(index);
            self.synthesizer.note_off(channel_of(lane), key);
            self.counters.retriggers += 1;
            outcome = NoteOutcome::StartedAfterRetrigger;
        }
        while self.sounding.len() >= self.sounding_cap {
            let oldest = self.sounding.remove(0);
            self.synthesizer.note_off(channel_of(oldest.lane), oldest.key);
            self.release_lane_if_silent(oldest.source, oldest.lane);
            self.counters.steals += 1;
            outcome = NoteOutcome::StartedAfterSteal;
        }
        // The steal may have taken this source's own last note and freed its lane;
        // nothing else leased in between, so the lane is still this source's to take.
        self.lane_lessees[lane] = Some(source);
        self.synthesizer.note_on(channel_of(lane), key, velocity);
        self.sounding.push(SoundingNote { source, lane, key });
        self.counters.notes_started += 1;
        self.counters.peak_sounding = self.counters.peak_sounding.max(self.sounding.len());
        outcome
    }

    /// Release `key` for `source`. A note the ledger already released (stolen or
    /// retriggered) is not sent again: its lane may belong to another source by now.
    pub fn note_off(&mut self, source: SourceKey, key: i32) {
        let Some(index) = self.sounding.iter().position(|note| note.source == source && note.key == key) else {
            return;
        };
        let note = self.sounding.remove(index);
        self.synthesizer.note_off(channel_of(note.lane), key);
        self.release_lane_if_silent(source, note.lane);
    }

    /// Release every note of every source; the tails ring out.
    pub fn release_all(&mut self) {
        self.synthesizer.note_off_all(false);
        self.sounding.clear();
        self.lane_lessees = [None; LANE_COUNT];
    }

    /// Render into `left` and `right` (overwritten, same length).
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synthesizer.render(left, right);
    }

    pub fn sounding_count(&self) -> usize {
        self.sounding.len()
    }

    pub fn counters(&self) -> SlotCounters {
        self.counters
    }

    /// The lane `source` holds, if it has sounding notes.
    pub fn lane_of(&self, source: SourceKey) -> Option<usize> {
        self.lane_lessees.iter().position(|lessee| *lessee == Some(source))
    }

    fn lease_lane(&mut self, source: SourceKey) -> Option<usize> {
        if let Some(lane) = self.lane_of(source) {
            return Some(lane);
        }
        let lane = self.lane_lessees.iter().position(Option::is_none)?;
        self.lane_lessees[lane] = Some(source);
        Some(lane)
    }

    fn release_lane_if_silent(&mut self, source: SourceKey, lane: usize) {
        if !self.sounding.iter().any(|note| note.source == source) {
            self.lane_lessees[lane] = None;
        }
    }
}
