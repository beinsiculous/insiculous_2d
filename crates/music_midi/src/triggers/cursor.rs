//! A part's cursor: which unit its next accepted trigger plays, by the part's mode.
//!
//! **Choosing and consuming are two steps**, so a trigger that is then refused (the
//! instrument or the scheduler full) never moves the cursor.

use crate::song_data::MusicalUnit;
use crate::song_file::PartMode;

/// The unit a trigger plays, and how far into it the song already is: notes that have
/// ended by then are left out, the rest play what remains of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Selection {
    pub unit: usize,
    pub elapsed_ticks: u64,
}

/// One part's shared cursor. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartCursor {
    mode: PartMode,
    /// The unit a `Loop` part plays next, or the first a `Sequence` part may still play.
    next: usize,
}

impl PartCursor {
    pub(crate) fn new(mode: PartMode) -> Self {
        Self { mode, next: 0 }
    }

    /// The unit a trigger playing at `play_tick` would play, or `None` for silence: an
    /// exhausted `Sequence`, a rest under `AtSongPosition`, or a part with no notes.
    ///
    /// `Loop` plays its next unit whatever the song's position. `Sequence` skips every
    /// unit that starts before `play_tick`. `AtSongPosition` plays the unit that started
    /// last at or before `play_tick`, if any of its notes still sounds there; an earlier,
    /// longer unit is not looked back for.
    pub(crate) fn select(&self, units: &[MusicalUnit], play_tick: u64) -> Option<Selection> {
        let from_start = |unit| Selection {
            unit,
            elapsed_ticks: 0,
        };
        match self.mode {
            PartMode::Loop => (!units.is_empty()).then(|| from_start(self.next % units.len())),
            PartMode::Sequence => {
                let first_ahead = units.partition_point(|unit| unit.tick < play_tick);
                let unit = self.next.max(first_ahead);
                (unit < units.len()).then(|| from_start(unit))
            }
            PartMode::AtSongPosition => {
                let started = units.partition_point(|unit| unit.tick <= play_tick);
                let unit = started.checked_sub(1)?;
                let elapsed_ticks = play_tick - units[unit].tick;
                units[unit]
                    .notes
                    .iter()
                    .any(|note| note.duration_ticks > elapsed_ticks)
                    .then_some(Selection {
                        unit,
                        elapsed_ticks,
                    })
            }
        }
    }

    /// Move past `selection`, which [`PartCursor::select`] chose; `AtSongPosition`
    /// consumes nothing.
    pub(crate) fn consume(&mut self, selection: Selection, unit_count: usize) {
        match self.mode {
            PartMode::Loop => self.next = (selection.unit + 1) % unit_count.max(1),
            PartMode::Sequence => self.next = selection.unit + 1,
            PartMode::AtSongPosition => {}
        }
    }

    /// Set the cursor to the song's new position: the first unit at or after `tick`
    /// (a `Loop` past its last unit wraps to the first). A wrap or `Play` is `tick` 0.
    pub(crate) fn reset_to(&mut self, units: &[MusicalUnit], tick: u64) {
        let first_ahead = units.partition_point(|unit| unit.tick < tick);
        self.next = match self.mode {
            PartMode::Loop if first_ahead >= units.len() => 0,
            _ => first_ahead,
        };
    }

    /// The unit a `Loop` part plays next, or the first a `Sequence` part may play.
    pub(crate) fn next(&self) -> usize {
        self.next
    }
}
