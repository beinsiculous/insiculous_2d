//! How a conductor is built: its rate, its block and every fixed capacity, checked once at
//! construction so nothing on the audio thread has to.

use super::slots::MAXIMUM_LANES;
use crate::error::ConductorError;
use crate::{
    BLOCK_FRAMES, COMMANDS_PER_BLOCK, COMMAND_QUEUE_CAPACITY, INSTRUMENT_SLOT_CAPACITY,
    LANES_PER_SLOT, LIFECYCLE_RESERVE, PENDING_EVENT_CAPACITY, SAMPLE_VOICE_CAP, SOUNDING_NOTE_CAP,
    SOURCE_CAPACITY, SYNTHESIZER_POLYPHONY,
};

const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 16_000..=192_000;
const BLOCK_SIZES: std::ops::RangeInclusive<usize> = 8..=1024;
const POLYPHONIES: std::ops::RangeInclusive<usize> = 8..=256;

/// Every fixed capacity of a conductor. The defaults are the measured, frozen values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacities {
    pub instrument_slots: usize,
    pub lanes_per_slot: usize,
    pub sounding_cap: usize,
    /// Voices in each synthesizer's pool, release tails included.
    pub polyphony: usize,
    pub sample_voices: usize,
    pub command_queue: usize,
    /// Places in the command queue a trigger may never take, so a lifecycle or transport
    /// command always fits behind a queue full of triggers.
    pub lifecycle_reserve: usize,
    /// Commands drained at the start of one block; the rest wait for the next.
    pub commands_per_block: usize,
    /// Scheduled note-ons and note-offs held at once.
    pub pending_events: usize,
    /// Source identities the conductor tracks at once, live or remembered dead.
    pub sources: usize,
}

impl Default for Capacities {
    fn default() -> Self {
        Self {
            instrument_slots: INSTRUMENT_SLOT_CAPACITY,
            lanes_per_slot: LANES_PER_SLOT,
            sounding_cap: SOUNDING_NOTE_CAP,
            polyphony: SYNTHESIZER_POLYPHONY,
            sample_voices: SAMPLE_VOICE_CAP,
            command_queue: COMMAND_QUEUE_CAPACITY,
            lifecycle_reserve: LIFECYCLE_RESERVE,
            commands_per_block: COMMANDS_PER_BLOCK,
            pending_events: PENDING_EVENT_CAPACITY,
            sources: SOURCE_CAPACITY,
        }
    }
}

/// How a conductor is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConductorConfig {
    pub sample_rate: u32,
    pub block_size: usize,
    pub capacities: Capacities,
}

impl ConductorConfig {
    /// The frozen block and capacities at `sample_rate`.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            block_size: BLOCK_FRAMES,
            capacities: Capacities::default(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ConductorError> {
        let capacities = &self.capacities;
        let named = [
            ("instrument slots", capacities.instrument_slots),
            ("lanes per slot", capacities.lanes_per_slot),
            ("sounding cap", capacities.sounding_cap),
            ("polyphony", capacities.polyphony),
            ("sample voices", capacities.sample_voices),
            ("command queue", capacities.command_queue),
            ("commands per block", capacities.commands_per_block),
            ("pending events", capacities.pending_events),
            ("sources", capacities.sources),
        ];
        if let Some((capacity, _)) = named.iter().find(|(_, value)| *value == 0) {
            return Err(ConductorError::ZeroCapacity { capacity });
        }
        if capacities.lifecycle_reserve >= capacities.command_queue {
            return Err(ConductorError::ReserveFillsQueue {
                reserve: capacities.lifecycle_reserve,
                command_queue: capacities.command_queue,
            });
        }
        if !SAMPLE_RATES.contains(&self.sample_rate) {
            return Err(ConductorError::SampleRateOutOfRange {
                sample_rate: self.sample_rate,
            });
        }
        if !BLOCK_SIZES.contains(&self.block_size) {
            return Err(ConductorError::BlockSizeOutOfRange {
                block_size: self.block_size,
            });
        }
        if !POLYPHONIES.contains(&capacities.polyphony) {
            return Err(ConductorError::PolyphonyOutOfRange {
                polyphony: capacities.polyphony,
            });
        }
        if capacities.polyphony < capacities.sounding_cap {
            return Err(ConductorError::PoolBelowSoundingCap {
                polyphony: capacities.polyphony,
                sounding_cap: capacities.sounding_cap,
            });
        }
        if capacities.lanes_per_slot > MAXIMUM_LANES {
            return Err(ConductorError::TooManyLanes {
                lanes: capacities.lanes_per_slot,
                maximum: MAXIMUM_LANES,
            });
        }
        Ok(())
    }
}

/// The two bus gains the conductor mixes under; master volume is the output's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusGains {
    pub music: f32,
    pub sfx: f32,
}

impl BusGains {
    pub const UNITY: BusGains = BusGains {
        music: 1.0,
        sfx: 1.0,
    };
}
