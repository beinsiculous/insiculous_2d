//! Triggers: a game action played as its bound part's next notes on its bound instrument,
//! for one source, at the moment the producer stamped.
//!
//! **One cursor per part.** Every binding of a part shares it, whichever instrument and
//! whichever source fires it, so alternating two enemies' attacks walks one melody in
//! their two voices; distinct triggers drained in one block take successive units in
//! command order.
//!
//! **Authored notes, authored lengths.** A trigger plays its unit's notes at their own
//! velocities and their own lengths — converted through the tempo map from where they
//! play — with each note-off scheduled through the ledger's ticket for that note, never
//! through the background sequencer. A note shorter than a block sounds for one block.
//!
//! **What a trigger can meet,** each counted: a binding from another song or an ended
//! source (stale); a song not playing, or no room for its notes (rejected, cursor
//! unmoved); a stamp older than one block plus [`DEVICE_PERIOD_ALLOWANCE`] at the drain
//! (late); an exhausted `Sequence` or a rest under `AtSongPosition` (silent). Its source
//! is admitted only after every one of those checks, so a refused, late or silent trigger
//! activates nothing and never evicts a live source from a full table.
//!
//! Nothing here allocates once the engine is built: the parts, the bindings, the source
//! table and the scheduled events are sized at load.

use std::time::Duration;

use common::clock::Instant;

use crate::conductor::{NoteTicket, Song};
use crate::error::ConductorError;
use crate::overflow::{PendingAction, PendingEvent, PendingEvents};
use crate::song_data::MusicalUnit;
use crate::song_file::{InstrumentSource, Quantize};
use crate::sources::{Admission, Deactivation, SourceId, SourceTable};
use crate::tempo_map::TempoMap;
use crate::DEVICE_PERIOD_ALLOWANCE;

mod cursor;
mod quantize;
#[cfg(test)]
mod tests;

use cursor::PartCursor;
use quantize::{Grid, Placement};

/// A binding of the loaded song, resolved from its action on the game thread by
/// [`Song::binding_index`], so the audio thread never compares a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingIndex(pub(crate) usize);

/// What a trigger engine plays through: the conductor's instrument slots, or a test's
/// recorder.
pub(crate) trait TriggerSink {
    /// Start `key` for `source` on `slot`; the ticket ends that note. `None` when the
    /// slot does not exist.
    fn note_on(
        &mut self,
        slot: usize,
        source: SourceId,
        key: u8,
        velocity: u8,
    ) -> Option<NoteTicket>;
    fn note_off(&mut self, slot: usize, ticket: NoteTicket);
    fn is_sounding(&self, slot: usize, ticket: NoteTicket) -> bool;
    /// Release every note `source` holds on every slot.
    fn release_source(&mut self, source: SourceId);
}

/// Running totals, published with the position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TriggerCounters {
    pub accepted: u64,
    pub rejected: u64,
    pub late: u64,
    pub stale: u64,
    pub silent: u64,
}

/// The song and the block a command is drained in.
#[derive(Clone, Copy)]
pub(crate) struct DrainClock<'a> {
    pub song: &'a Song,
    pub tempo_map: &'a TempoMap,
    pub sample_rate: u32,
    pub block_frames: usize,
    pub drained_at: Instant,
    /// The song sample the draining block starts on.
    pub position_sample: u64,
    pub playing: bool,
}

/// A trigger as it arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Trigger {
    pub binding: BindingIndex,
    pub source: SourceId,
    pub stamp: Instant,
}

#[derive(Debug, Clone, Copy)]
struct PartPlayer {
    track: usize,
    quantize: Quantize,
    cursor: PartCursor,
}

#[derive(Debug, Clone, Copy)]
struct BoundTarget {
    part: usize,
    /// `None` for an instrument the song does not declare, which only a `SongFile` built
    /// without the parser can name.
    slot: Option<usize>,
}

/// One song's parts, bindings, sources and scheduled events. See the module docs.
pub(crate) struct TriggerEngine {
    parts: Vec<PartPlayer>,
    bindings: Vec<BoundTarget>,
    sources: SourceTable,
    pending: PendingEvents,
    /// Each slot's cap on note-ons waiting to start: the sounding cap for a SoundFont
    /// instrument, the live voice cap for a sample.
    slot_caps: Vec<usize>,
    counters: TriggerCounters,
}

/// The capacities a trigger engine is built to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EngineShape {
    pub sources: usize,
    pub pending_events: usize,
    pub sounding_cap: usize,
    pub sample_voices: usize,
}

impl TriggerEngine {
    /// The engine for `song`: each part's track resolved, each binding to its part and
    /// to its instrument's slot (the `n`th instrument plays on slot `n`, whatever its kind).
    pub(crate) fn new(song: &Song, shape: EngineShape) -> Result<Self, ConductorError> {
        let file = song.file();
        let mut parts = Vec::with_capacity(file.parts.len());
        for part in &file.parts {
            parts.push(PartPlayer {
                track: part
                    .resolve_track(song.data())
                    .map_err(ConductorError::SongDoesNotFit)?,
                quantize: part.quantize,
                cursor: PartCursor::new(part.mode),
            });
        }
        let slot_of = |name: &str| {
            file.instruments
                .iter()
                .position(|instrument| instrument.name == name)
        };
        let mut bindings = Vec::with_capacity(file.bindings.len());
        for binding in &file.bindings {
            let Some(part) = file.parts.iter().position(|part| part.name == binding.part) else {
                return Err(ConductorError::BindingUnresolved {
                    action: binding.action.clone(),
                });
            };
            bindings.push(BoundTarget {
                part,
                slot: slot_of(&binding.instrument),
            });
        }
        let slot_caps: Vec<usize> = file
            .instruments
            .iter()
            .map(|instrument| match instrument.source {
                InstrumentSource::SoundFont { .. } => shape.sounding_cap,
                InstrumentSource::Sample { .. } => shape.sample_voices,
            })
            .collect();
        Ok(Self {
            parts,
            bindings,
            sources: SourceTable::with_capacity(shape.sources),
            pending: PendingEvents::new(shape.pending_events, slot_caps.len()),
            slot_caps,
            counters: TriggerCounters::default(),
        })
    }

    /// Refuse every epoch `previous` refused: a flush outlives the song it was sent
    /// under, so a straggling trigger of a flushed epoch cannot activate on the next.
    pub(crate) fn keep_flush_floor_of(&mut self, previous: &TriggerEngine) {
        self.sources.flush(previous.sources.flush_floor(), |_| {});
    }

    pub(crate) fn counters(&self) -> TriggerCounters {
        self.counters
    }

    pub(crate) fn live_sources(&self) -> usize {
        self.sources.live_count()
    }

    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// The unit a part's cursor plays next (`Loop`) or may play first (`Sequence`).
    pub(crate) fn cursor_of(&self, part: usize) -> Option<usize> {
        self.parts.get(part).map(|player| player.cursor.next())
    }

    /// Play one trigger, or count why not. See the module docs.
    pub(crate) fn trigger(
        &mut self,
        trigger: Trigger,
        clock: &DrainClock<'_>,
        sink: &mut impl TriggerSink,
    ) {
        let Some(target) = self.bindings.get(trigger.binding.0).copied() else {
            self.counters.stale += 1;
            return;
        };
        let (Some(slot), true) = (target.slot, clock.playing) else {
            self.counters.rejected += 1;
            return;
        };
        let age = clock.drained_at.saturating_duration_since(trigger.stamp);
        let block =
            Duration::from_secs_f64(clock.block_frames as f64 / f64::from(clock.sample_rate));
        if age > block + DEVICE_PERIOD_ALLOWANCE {
            self.counters.late += 1;
            return;
        }
        let grid = Grid {
            tempo_map: clock.tempo_map,
            sample_rate: clock.sample_rate,
            block_frames: clock.block_frames,
            position_sample: clock.position_sample,
        };
        let player = self.parts[target.part];
        let stamp_sample = quantize::stamp_sample(&grid, age);
        let placement = quantize::place(player.quantize, stamp_sample, &grid);
        let play_sample = match placement {
            Placement::Now => clock.position_sample,
            Placement::At { sample } => sample,
        };
        let play_tick = clock
            .tempo_map
            .tick_at_sample(play_sample, clock.sample_rate);
        let units = clock.song.data().tracks()[player.track].units();
        let Some(selection) = player.cursor.select(units, play_tick) else {
            self.counters.silent += 1;
            return;
        };
        let unit = &units[selection.unit];
        let notes = sounding_notes(unit, selection.elapsed_ticks).count();
        let scheduled = matches!(placement, Placement::At { .. });
        let slot_cap = self.slot_caps.get(slot).copied().unwrap_or(0);
        if self
            .pending
            .admit(slot, notes, scheduled, slot_cap, |slot, ticket| {
                sink.is_sounding(slot, ticket)
            })
            .is_err()
        {
            self.counters.rejected += 1;
            return;
        }
        // Admitted only once nothing can refuse the trigger: admission into a full table
        // ends the oldest live source, which a refused trigger must never cost.
        match self.sources.admit(trigger.source) {
            Admission::Dead => {
                self.counters.stale += 1;
                return;
            }
            Admission::Live { ended } => {
                if let Some(ended) = ended {
                    self.end_source(ended, sink);
                }
            }
        }
        self.parts[target.part]
            .cursor
            .consume(selection, units.len());
        for note in sounding_notes(unit, selection.elapsed_ticks) {
            // Measured from the sample it plays on to the tick it ends on, so a note of
            // `AtSongPosition` ends exactly where it was written to.
            let remaining_ticks = note.duration_ticks - selection.elapsed_ticks;
            let duration_samples = clock
                .tempo_map
                .sample_at_tick(play_tick.saturating_add(remaining_ticks), clock.sample_rate)
                .saturating_sub(play_sample)
                .max(clock.block_frames as u64);
            let event = if scheduled {
                PendingEvent {
                    due_sample: play_sample,
                    slot,
                    source: trigger.source,
                    action: PendingAction::Start {
                        key: note.key,
                        velocity: note.velocity,
                        duration_samples,
                    },
                }
            } else {
                let Some(ticket) = sink.note_on(slot, trigger.source, note.key, note.velocity)
                else {
                    continue;
                };
                PendingEvent {
                    due_sample: play_sample + duration_samples,
                    slot,
                    source: trigger.source,
                    action: PendingAction::Release { ticket },
                }
            };
            self.pending.push(event);
        }
        self.counters.accepted += 1;
    }

    /// Fire every scheduled event due before `due_before`: the end of the block, or the
    /// loop seam when that comes first, since a wrap drops what lies beyond it.
    pub(crate) fn fire_due(&mut self, due_before: u64, sink: &mut impl TriggerSink) {
        self.pending
            .fire_due(due_before, |event| match event.action {
                PendingAction::Start {
                    key,
                    velocity,
                    duration_samples,
                } => {
                    let ticket = sink.note_on(event.slot, event.source, key, velocity)?;
                    Some(PendingEvent {
                        due_sample: event.due_sample + duration_samples,
                        action: PendingAction::Release { ticket },
                        ..*event
                    })
                }
                PendingAction::Release { ticket } => {
                    sink.note_off(event.slot, ticket);
                    None
                }
            });
    }

    /// End `source`: its notes released, its scheduled events dropped, its activation
    /// never live again. A deactivation naming an ended activation is counted stale.
    pub(crate) fn deactivate(&mut self, source: SourceId, sink: &mut impl TriggerSink) {
        match self.sources.deactivate(source) {
            Deactivation::Dead => self.counters.stale += 1,
            Deactivation::Ended { named, older } => {
                for ended in named.into_iter().chain(older) {
                    self.end_source(ended, sink);
                }
            }
        }
    }

    /// End every source activated below `flush_epoch`.
    pub(crate) fn flush(&mut self, flush_epoch: u32, sink: &mut impl TriggerSink) {
        let pending = &mut self.pending;
        self.sources.flush(flush_epoch, |ended| {
            sink.release_source(ended);
            pending.remove_source(ended);
        });
    }

    /// The song moved to `tick` (a seek, a wrap, `Play`): every scheduled event is
    /// dropped and each cursor set to the first unit at or after it. The caller releases
    /// every slot's notes.
    pub(crate) fn restart_at(&mut self, tick: u64, song: &Song) {
        self.pending.clear();
        for player in &mut self.parts {
            let units = song.data().tracks()[player.track].units();
            player.cursor.reset_to(units, tick);
        }
    }

    /// Drop every scheduled event, the cursors kept: a pause, a stop, the song's end.
    /// The caller releases every slot's notes.
    pub(crate) fn clear_pending(&mut self) {
        self.pending.clear();
    }

    fn end_source(&mut self, source: SourceId, sink: &mut impl TriggerSink) {
        sink.release_source(source);
        self.pending.remove_source(source);
    }
}

/// The notes of `unit` still sounding `elapsed_ticks` after it starts.
fn sounding_notes(
    unit: &MusicalUnit,
    elapsed_ticks: u64,
) -> impl Iterator<Item = &crate::song_data::Note> {
    unit.notes
        .iter()
        .filter(move |note| note.duration_ticks > elapsed_ticks)
}

impl Song {
    /// The binding a game action names, resolved once on the game thread.
    pub fn binding_index(&self, action: &str) -> Option<BindingIndex> {
        self.file()
            .bindings
            .iter()
            .position(|binding| binding.action == action)
            .map(BindingIndex)
    }
}
