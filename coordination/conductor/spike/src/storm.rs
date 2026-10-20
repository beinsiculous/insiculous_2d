//! A deterministic trigger storm: the worst case the voice budget has to survive, the
//! same sequence on every run and on every target.
//!
//! Triggers arrive at a fixed rate, round-robin across the instrument slots. Each trigger
//! is a chord held for a while from a randomly chosen source, so with the default numbers
//! (16 triggers a second, 8 slots, 4-note chords held 3 s) every slot sits at the 24-note
//! sounding cap. Every fourth trigger also fires a one-shot on the sample slot, and one
//! source hammers one key on slot 0 with every trigger, retriggering it on one lane.

use crate::instrument_slot::SourceKey;
use crate::mixer::{SlotTarget, SpikeCommand};

#[derive(Debug, Clone, Copy)]
pub struct StormConfig {
    pub instrument_slots: usize,
    pub triggers_per_second: f64,
    pub notes_per_trigger: usize,
    pub hold_seconds: f64,
    /// Every this many triggers, a one-shot fires on the sample slot (0: never).
    pub sample_every: usize,
    /// One source re-strikes one key on slot 0 with every trigger.
    pub hammer_one_key: bool,
    /// Every trigger goes to the sample slot and none to the instruments.
    pub sample_slot_only: bool,
}

/// Decision 2's worst case.
pub const WORST_CASE: StormConfig = StormConfig {
    instrument_slots: 8,
    triggers_per_second: 16.0,
    notes_per_trigger: 4,
    hold_seconds: 3.0,
    sample_every: 4,
    hammer_one_key: true,
    sample_slot_only: false,
};

/// The same storm aimed only at the sample slot.
pub const SAMPLE_SLOT_ONLY: StormConfig = StormConfig { sample_slot_only: true, ..WORST_CASE };

const CHORD_INTERVALS: [u8; 4] = [0, 4, 7, 12];
const SOURCES: u64 = 64;
const HAMMER_SOURCE: SourceKey = 1_000;
const HAMMER_KEY: u8 = 60;

pub struct Storm {
    config: StormConfig,
    random_state: u64,
    trigger_index: usize,
    next_trigger_seconds: f64,
    /// (due time, command), released in time order.
    pending_note_offs: Vec<(f64, SpikeCommand)>,
}

impl Storm {
    pub fn new(config: StormConfig, seed: u64) -> Self {
        Self {
            config,
            random_state: seed.max(1),
            trigger_index: 0,
            next_trigger_seconds: 0.0,
            pending_note_offs: Vec::new(),
        }
    }

    /// Append every command due up to `now_seconds` to `commands`.
    pub fn commands_until(&mut self, now_seconds: f64, commands: &mut Vec<SpikeCommand>) {
        while self.next_trigger_seconds <= now_seconds {
            let at = self.next_trigger_seconds;
            self.trigger(at, commands);
            self.trigger_index += 1;
            self.next_trigger_seconds += 1.0 / self.config.triggers_per_second;
        }
        self.pending_note_offs.sort_by(|left, right| left.0.total_cmp(&right.0));
        let due = self.pending_note_offs.partition_point(|(due_seconds, _)| *due_seconds <= now_seconds);
        commands.extend(self.pending_note_offs.drain(..due).map(|(_, command)| command));
    }

    fn trigger(&mut self, at_seconds: f64, commands: &mut Vec<SpikeCommand>) {
        let source = (self.next_random() % SOURCES) as SourceKey;
        let root_key = 48 + (self.next_random() % 24) as u8;
        let velocity = 70 + (self.next_random() % 50) as u8;
        let release_at = at_seconds + self.config.hold_seconds;
        if self.config.sample_slot_only {
            self.note(SlotTarget::Sample, source, root_key, velocity, release_at, commands);
            return;
        }
        let target = SlotTarget::Instrument(self.trigger_index % self.config.instrument_slots);
        for interval in CHORD_INTERVALS.iter().take(self.config.notes_per_trigger) {
            self.note(target, source, root_key + interval, velocity, release_at, commands);
        }
        if self.config.sample_every > 0 && self.trigger_index.is_multiple_of(self.config.sample_every) {
            self.note(SlotTarget::Sample, source, root_key, velocity, release_at, commands);
        }
        if self.config.hammer_one_key {
            commands.push(SpikeCommand::NoteOn {
                target: SlotTarget::Instrument(0),
                source: HAMMER_SOURCE,
                key: HAMMER_KEY,
                velocity,
            });
        }
    }

    fn note(&mut self, target: SlotTarget, source: SourceKey, key: u8, velocity: u8, release_at: f64, commands: &mut Vec<SpikeCommand>) {
        commands.push(SpikeCommand::NoteOn { target, source, key, velocity });
        self.pending_note_offs.push((release_at, SpikeCommand::NoteOff { target, source, key }));
    }

    /// xorshift64*: small, fast, and identical on every target.
    fn next_random(&mut self) -> u64 {
        self.random_state ^= self.random_state >> 12;
        self.random_state ^= self.random_state << 25;
        self.random_state ^= self.random_state >> 27;
        self.random_state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}
