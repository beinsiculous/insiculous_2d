//! Where a replacement lands: the old song's bar and the fraction of it played, carried to
//! the same bar of the new one, and the commands that start the new conductor there.

use music_midi::{ConductorCommand, Song, TempoMap};

use super::super::{SongPosition, SONG_SAMPLE_RATE};

/// How the new song starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Start {
    Playing(u64),
    Paused(u64),
    StoppedAtEnd,
}

/// A song's bars as the swap reads them.
#[derive(Clone, Copy)]
pub(super) struct Bars<'a> {
    pub(super) map: &'a TempoMap,
    pub(super) end_tick: u64,
}

impl<'a> Bars<'a> {
    pub(super) fn of(song: &'a Song) -> Self {
        Self {
            map: song.background().tempo_map(),
            end_tick: song.background().end_tick(),
        }
    }

    /// Bar `bar`'s ticks, the song's last bar ending at its end; a bar at or past the end
    /// is a whole bar of its meter.
    fn span(self, bar: u64) -> (u64, u64) {
        let (start, end) = self.map.bar_span(bar);
        if start < self.end_tick {
            (start, end.min(self.end_tick))
        } else {
            (start, end)
        }
    }

    /// The song's bars, a partial last bar counted: the end's bar, plus one where the
    /// end is not on a bar line. At least one, so a modulo never divides by zero.
    fn count(self) -> u64 {
        let last = self.map.position_at_tick(self.end_tick).bar;
        let partial = u64::from(self.map.bar_span(last).0 != self.end_tick);
        (last + partial).max(1)
    }
}

/// The bar `old_tick` is in, and the tick at the same fraction of the same bar of `new` —
/// the bar modulo `new`'s bar count when `looping` — clamped inside that bar. `None` for
/// a bar `new` does not have.
pub(super) fn same_place(
    old: Bars<'_>,
    new: Bars<'_>,
    old_tick: u64,
    looping: bool,
) -> (u64, Option<u64>) {
    let old_tick = old_tick.min(old.end_tick);
    let bar = old.map.position_at_tick(old_tick).bar;
    let (old_start, old_end) = old.span(bar);
    let new_bar = if looping { bar % new.count() } else { bar };
    let (new_start, new_end) = new.span(new_bar);
    if new_start >= new.end_tick {
        return (bar, None);
    }
    let old_length = u128::from(old_end.saturating_sub(old_start).max(1));
    let elapsed = u128::from(old_tick.saturating_sub(old_start)).min(old_length);
    let offset = elapsed * u128::from(new_end - new_start) / old_length;
    let tick = new_start
        .saturating_add(u64::try_from(offset).unwrap_or(u64::MAX))
        .min(new_end - 1);
    (bar, Some(tick))
}

/// The commands that start a fresh conductor as `start` says, the transport's looping
/// flag first.
pub(super) fn start_commands(start: Start, looping: bool, end_tick: u64) -> Vec<ConductorCommand> {
    match start {
        Start::Playing(tick) => vec![
            ConductorCommand::SetLooping(looping),
            ConductorCommand::Seek(tick),
            ConductorCommand::Play,
        ],
        // A fresh conductor is stopped, and a pause only holds a playing one.
        Start::Paused(tick) => vec![
            ConductorCommand::SetLooping(looping),
            ConductorCommand::Seek(tick),
            ConductorCommand::Play,
            ConductorCommand::Pause,
        ],
        // Played from its end without looping, it ends on its first block.
        Start::StoppedAtEnd => vec![
            ConductorCommand::SetLooping(false),
            ConductorCommand::Seek(end_tick),
            ConductorCommand::Play,
        ],
    }
}

pub(super) fn position_at(map: &TempoMap, tick: u64) -> SongPosition {
    let musical = map.position_at_tick(tick);
    SongPosition {
        bar: musical.bar,
        beat: musical.beat,
        beat_in_bar: musical.beat_in_bar,
        seconds: map.sample_at_tick(tick, SONG_SAMPLE_RATE) as f64 / f64::from(SONG_SAMPLE_RATE),
        latency_samples: 0,
    }
}
