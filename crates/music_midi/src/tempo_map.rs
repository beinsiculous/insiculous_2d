//! Tick ↔ seconds ↔ sample ↔ bar conversions across tempo and time-signature changes.
//!
//! Every conversion is exact integer arithmetic. Time is counted in *time units* of one
//! microsecond divided by the file's ticks per quarter note, so a tick at any tempo lands
//! on a whole number of units and nothing accumulates rounding across a long song. Only
//! [`TempoMap::seconds_at_tick`] and [`TempoMap::tick_at_seconds`] touch floating point,
//! for display.
//!
//! **Sample rounding has one rule:** an event at a tick sounds on the first sample at or
//! after its exact time ([`TempoMap::sample_at_tick`] rounds up), and the tick a sample
//! is playing is the last tick at or before it ([`TempoMap::tick_at_sample`] rounds
//! down). The two agree: `tick_at_sample(s) >= t` exactly when `sample_at_tick(t) <= s`,
//! which is what lets [`TempoMap::beats_crossed`] count beat boundaries by integers.
//!
//! **A beat is the time signature's denominator note** — a half note in 3/2, a quarter
//! in 3/4, an eighth in 6/8. Bars, beats and beats-in-bar count from zero. A time
//! signature that lands mid-bar closes the partial bar (it counts as a bar, and a partial
//! beat as a beat) and starts a new bar at its tick.

use crate::error::EditError;

#[cfg(test)]
mod tests;

/// The tempo before a file's first tempo event, and of a file with none: 120 BPM.
pub const DEFAULT_MICROSECONDS_PER_QUARTER: u32 = 500_000;

const DEFAULT_NUMERATOR: u8 = 4;
const DEFAULT_DENOMINATOR: u8 = 4;
const MICROSECONDS_PER_SECOND: u128 = 1_000_000;
/// MIDI's tempo meta event carries 24 bits.
const MAXIMUM_MICROSECONDS_PER_QUARTER: u32 = 0x00FF_FFFF;

/// A tempo that takes effect at a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoChange {
    pub tick: u64,
    pub microseconds_per_quarter: u32,
}

impl TempoChange {
    /// The tempo in quarter notes a minute.
    pub fn beats_per_minute(&self) -> f64 {
        60_000_000.0 / f64::from(self.microseconds_per_quarter.max(1))
    }

    pub(crate) fn check(&self) -> Result<(), EditError> {
        if self.microseconds_per_quarter == 0
            || self.microseconds_per_quarter > MAXIMUM_MICROSECONDS_PER_QUARTER
        {
            return Err(EditError::TempoOutOfRange {
                microseconds_per_quarter: self.microseconds_per_quarter,
            });
        }
        Ok(())
    }
}

/// A time signature that takes effect at a tick. `denominator` is the note value itself
/// (2, 4, 8 …), not MIDI's power-of-two exponent; the last two fields are carried so an
/// exported file keeps what the original said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSignature {
    pub tick: u64,
    pub numerator: u8,
    pub denominator: u8,
    pub clocks_per_click: u8,
    pub thirty_second_notes_per_quarter: u8,
}

impl TimeSignature {
    /// A time signature with MIDI's customary metronome fields: a click every quarter
    /// note (24 clocks) and eight thirty-second notes to the quarter.
    pub fn new(tick: u64, numerator: u8, denominator: u8) -> Self {
        Self {
            tick,
            numerator,
            denominator,
            clocks_per_click: 24,
            thirty_second_notes_per_quarter: 8,
        }
    }

    /// Ticks in one beat, or `None` when the signature cannot divide the file's quarter
    /// note into whole beats (or has no beats at all).
    pub(crate) fn ticks_per_beat(&self, ticks_per_quarter: u16) -> Option<u64> {
        let ticks_per_whole_note = u64::from(ticks_per_quarter) * 4;
        let denominator = u64::from(self.denominator);
        let valid = self.numerator > 0
            && denominator.is_power_of_two()
            && ticks_per_whole_note % denominator == 0;
        valid.then(|| ticks_per_whole_note / denominator)
    }

    pub(crate) fn check(&self, ticks_per_quarter: u16) -> Result<(), EditError> {
        match self.ticks_per_beat(ticks_per_quarter) {
            Some(_) => Ok(()),
            None => Err(EditError::InvalidTimeSignature {
                numerator: self.numerator,
                denominator: self.denominator,
                ticks_per_quarter,
            }),
        }
    }
}

/// Where a tick falls in the bar structure, every count from zero: `beat` counts beats
/// from the song's start, `beat_in_bar` within the current bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicalPosition {
    pub bar: u64,
    pub beat: u64,
    pub beat_in_bar: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TempoSegment {
    start_tick: u64,
    start_time_units: u128,
    microseconds_per_quarter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MeterSegment {
    start_tick: u64,
    start_bar: u64,
    start_beat: u64,
    beats_per_bar: u64,
    ticks_per_beat: u64,
}

/// A song's timing: every tempo and time signature, with the 120 BPM and 4/4 defaults
/// filling any span before the first of each. Built by
/// [`SongData::tempo_map`](crate::SongData::tempo_map) from validated data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMap {
    ticks_per_quarter: u16,
    tempo_segments: Vec<TempoSegment>,
    meter_segments: Vec<MeterSegment>,
}

impl TempoMap {
    /// Build the map. `tempo_changes` and `time_signatures` must be sorted by tick, one
    /// per tick, and already checked; an invalid signature degrades to one-tick beats
    /// rather than dividing by zero.
    pub(crate) fn new(
        ticks_per_quarter: u16,
        tempo_changes: &[TempoChange],
        time_signatures: &[TimeSignature],
    ) -> Self {
        let ticks_per_quarter = ticks_per_quarter.max(1);
        Self {
            ticks_per_quarter,
            tempo_segments: build_tempo_segments(tempo_changes),
            meter_segments: build_meter_segments(ticks_per_quarter, time_signatures),
        }
    }

    pub fn ticks_per_quarter(&self) -> u16 {
        self.ticks_per_quarter
    }

    /// The tempo in effect at `tick`.
    pub fn microseconds_per_quarter_at(&self, tick: u64) -> u32 {
        self.tempo_segment_at_tick(tick).microseconds_per_quarter
    }

    /// The time at `tick`, in seconds from the song's start.
    pub fn seconds_at_tick(&self, tick: u64) -> f64 {
        self.time_units_at_tick(tick) as f64 / self.time_units_per_second() as f64
    }

    /// The last tick at or before `seconds`; zero for a negative or non-finite time.
    pub fn tick_at_seconds(&self, seconds: f64) -> u64 {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 0;
        }
        let time_units = (seconds * self.time_units_per_second() as f64) as u128;
        let index = self
            .tempo_segments
            .partition_point(|segment| segment.start_time_units <= time_units);
        let segment = self.tempo_segments[index.saturating_sub(1)];
        let elapsed_ticks =
            (time_units - segment.start_time_units) / u128::from(segment.microseconds_per_quarter);
        segment
            .start_tick
            .saturating_add(saturate_to_u64(elapsed_ticks))
    }

    /// The first sample at or after `tick`'s exact time. A zero `sample_rate` is read as 1.
    pub fn sample_at_tick(&self, tick: u64, sample_rate: u32) -> u64 {
        let scaled = self
            .time_units_at_tick(tick)
            .saturating_mul(u128::from(sample_rate.max(1)));
        saturate_to_u64(scaled.div_ceil(self.time_units_per_second()))
    }

    /// The last tick at or before `sample_position`. A zero `sample_rate` is read as 1.
    pub fn tick_at_sample(&self, sample_position: u64, sample_rate: u32) -> u64 {
        let sample_rate = u128::from(sample_rate.max(1));
        let target = u128::from(sample_position).saturating_mul(self.time_units_per_second());
        let index = self.tempo_segments.partition_point(|segment| {
            segment.start_time_units.saturating_mul(sample_rate) <= target
        });
        let segment = self.tempo_segments[index.saturating_sub(1)];
        let elapsed = target - segment.start_time_units.saturating_mul(sample_rate);
        let units_per_tick = u128::from(segment.microseconds_per_quarter) * sample_rate;
        segment
            .start_tick
            .saturating_add(saturate_to_u64(elapsed / units_per_tick))
    }

    /// Where `tick` falls in the bars.
    pub fn position_at_tick(&self, tick: u64) -> MusicalPosition {
        let segment = self.meter_segment_at_tick(tick);
        let beats_elapsed = (tick - segment.start_tick) / segment.ticks_per_beat;
        MusicalPosition {
            bar: segment.start_bar + beats_elapsed / segment.beats_per_bar,
            beat: segment.start_beat + beats_elapsed,
            beat_in_bar: beats_elapsed % segment.beats_per_bar,
        }
    }

    /// The tick on which beat number `beat` (from zero) begins.
    pub fn tick_at_beat(&self, beat: u64) -> u64 {
        let index = self
            .meter_segments
            .partition_point(|segment| segment.start_beat <= beat);
        let segment = self.meter_segments[index.saturating_sub(1)];
        segment
            .start_tick
            .saturating_add((beat - segment.start_beat).saturating_mul(segment.ticks_per_beat))
    }

    /// How many beat boundaries fall in `(from_sample, to_sample]` — each counted once,
    /// on the first sample at or after its exact time. Zero when `to_sample` is not after
    /// `from_sample`.
    pub fn beats_crossed(&self, from_sample: u64, to_sample: u64, sample_rate: u32) -> u64 {
        if to_sample <= from_sample {
            return 0;
        }
        let beat_at = |sample_position| {
            self.position_at_tick(self.tick_at_sample(sample_position, sample_rate))
                .beat
        };
        beat_at(to_sample) - beat_at(from_sample)
    }

    fn time_units_per_second(&self) -> u128 {
        u128::from(self.ticks_per_quarter) * MICROSECONDS_PER_SECOND
    }

    fn time_units_at_tick(&self, tick: u64) -> u128 {
        let segment = self.tempo_segment_at_tick(tick);
        let elapsed_ticks = u128::from(tick - segment.start_tick);
        segment.start_time_units + elapsed_ticks * u128::from(segment.microseconds_per_quarter)
    }

    fn tempo_segment_at_tick(&self, tick: u64) -> TempoSegment {
        let index = self
            .tempo_segments
            .partition_point(|segment| segment.start_tick <= tick);
        self.tempo_segments[index.saturating_sub(1)]
    }

    fn meter_segment_at_tick(&self, tick: u64) -> MeterSegment {
        let index = self
            .meter_segments
            .partition_point(|segment| segment.start_tick <= tick);
        self.meter_segments[index.saturating_sub(1)]
    }
}

/// The first segment always starts at tick 0, so every lookup's `index - 1` is in range.
fn build_tempo_segments(tempo_changes: &[TempoChange]) -> Vec<TempoSegment> {
    let mut segments = vec![TempoSegment {
        start_tick: 0,
        start_time_units: 0,
        microseconds_per_quarter: DEFAULT_MICROSECONDS_PER_QUARTER,
    }];
    for change in tempo_changes {
        let previous = segments[segments.len() - 1];
        let microseconds_per_quarter = change.microseconds_per_quarter.max(1);
        if change.tick == previous.start_tick {
            segments.pop();
        }
        let elapsed_ticks = u128::from(change.tick - previous.start_tick);
        let start_time_units = if change.tick == previous.start_tick {
            previous.start_time_units
        } else {
            previous.start_time_units
                + elapsed_ticks * u128::from(previous.microseconds_per_quarter)
        };
        segments.push(TempoSegment {
            start_tick: change.tick,
            start_time_units,
            microseconds_per_quarter,
        });
    }
    segments
}

fn build_meter_segments(
    ticks_per_quarter: u16,
    time_signatures: &[TimeSignature],
) -> Vec<MeterSegment> {
    let segment_for = |signature: &TimeSignature, start_bar, start_beat| MeterSegment {
        start_tick: signature.tick,
        start_bar,
        start_beat,
        beats_per_bar: u64::from(signature.numerator.max(1)),
        ticks_per_beat: signature.ticks_per_beat(ticks_per_quarter).unwrap_or(1),
    };
    let default_signature = TimeSignature::new(0, DEFAULT_NUMERATOR, DEFAULT_DENOMINATOR);
    let mut segments = vec![segment_for(&default_signature, 0, 0)];
    for signature in time_signatures {
        let previous = segments[segments.len() - 1];
        if signature.tick == previous.start_tick {
            segments.pop();
            segments.push(segment_for(
                signature,
                previous.start_bar,
                previous.start_beat,
            ));
            continue;
        }
        let beats_elapsed =
            (signature.tick - previous.start_tick).div_ceil(previous.ticks_per_beat);
        let bars_elapsed = beats_elapsed.div_ceil(previous.beats_per_bar);
        segments.push(segment_for(
            signature,
            previous.start_bar + bars_elapsed,
            previous.start_beat + beats_elapsed,
        ));
    }
    segments
}

fn saturate_to_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
