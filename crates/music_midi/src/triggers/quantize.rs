//! Where a trigger lands in the song: the producer's stamp turned into a song sample, then
//! snapped forward to the part's grid.
//!
//! **The stamp's sample.** The conductor knows two things at a drain: the song sample its
//! block starts on and the wall clock. A trigger stamped `age` before the drain is read as
//! having happened `age × sample rate` samples before that block. The audio thread renders
//! in bursts — a device period's worth of blocks back to back, then nothing — so the
//! wall clock does not advance evenly with the song: the reading is off by at most one
//! device period (the drain gap) plus one block. That is the error bound of every
//! placement below.
//!
//! **The grid** is the song's beats, or eighth or sixteenth notes laid from each beat's
//! start; a subdivision longer than the beat (an eighth in 6/16) is the beat. A trigger
//! stamped no more than one block after a grid boundary was on time and plays now; one
//! stamped later waits for the next boundary — unless the queue held it past that
//! boundary too, and then it plays now. A trigger never plays before the block it is
//! drained in.

use std::time::Duration;

use crate::song_file::Quantize;
use crate::tempo_map::TempoMap;

/// When a trigger's notes start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// At the start of the block it was drained in.
    Now,
    /// On the first frame of the block holding this song sample.
    At { sample: u64 },
}

/// What a placement needs to know about the song and the block.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grid<'a> {
    pub tempo_map: &'a TempoMap,
    pub sample_rate: u32,
    pub block_frames: usize,
    /// The song sample the draining block starts on.
    pub position_sample: u64,
}

/// The song sample a trigger `age` old at the drain was stamped on (zero at the latest).
pub(crate) fn stamp_sample(grid: &Grid<'_>, age: Duration) -> u64 {
    let age_samples = (age.as_secs_f64() * f64::from(grid.sample_rate)).round() as u64;
    grid.position_sample.saturating_sub(age_samples)
}

/// Where a trigger stamped on `stamp_sample` lands under `quantize`. See the module docs.
pub(crate) fn place(quantize: Quantize, stamp_sample: u64, grid: &Grid<'_>) -> Placement {
    let Some(step_ticks) = step_ticks(quantize, grid.tempo_map) else {
        return Placement::Now;
    };
    let tick = grid
        .tempo_map
        .tick_at_sample(stamp_sample, grid.sample_rate);
    let (previous_tick, next_tick) = boundaries(grid.tempo_map, tick, step_ticks);
    let previous_sample = grid
        .tempo_map
        .sample_at_tick(previous_tick, grid.sample_rate);
    if stamp_sample.saturating_sub(previous_sample) <= grid.block_frames as u64 {
        return Placement::Now;
    }
    let next_sample = grid.tempo_map.sample_at_tick(next_tick, grid.sample_rate);
    if next_sample <= grid.position_sample {
        Placement::Now
    } else {
        Placement::At {
            sample: next_sample,
        }
    }
}

/// The grid's step in ticks, or `None` when the part does not quantize.
fn step_ticks(quantize: Quantize, tempo_map: &TempoMap) -> Option<u64> {
    let quarter = u64::from(tempo_map.ticks_per_quarter());
    match quantize {
        Quantize::Off => None,
        Quantize::Beat => Some(u64::MAX),
        Quantize::Eighth => Some((quarter / 2).max(1)),
        Quantize::Sixteenth => Some((quarter / 4).max(1)),
    }
}

/// The grid boundaries at or before `tick` and after it: the beat's start plus whole
/// steps, never past the next beat.
fn boundaries(tempo_map: &TempoMap, tick: u64, step_ticks: u64) -> (u64, u64) {
    let beat = tempo_map.position_at_tick(tick).beat;
    let beat_start = tempo_map.tick_at_beat(beat);
    let next_beat_start = tempo_map.tick_at_beat(beat + 1);
    let steps = (tick - beat_start) / step_ticks;
    let previous = beat_start + steps * step_ticks;
    let next = previous.saturating_add(step_ticks).min(next_beat_start);
    (previous, next)
}
