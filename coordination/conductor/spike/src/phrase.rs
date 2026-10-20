//! The phrase both instruments share — the opening bars of Bach's Invention 2 — as timed
//! commands, and the queue that releases timed commands as the clock passes them.

use crate::instrument_slot::SourceKey;
use crate::mixer::{SlotTarget, SpikeCommand};
use crate::score::{Score, ScoreMessage};

/// A note of the phrase, in seconds from its start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhraseNote {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub key: u8,
    pub velocity: u8,
}

/// The notes of `score` that start in its first `bars` bars of 4/4, at its first tempo.
pub fn opening_bars(score: &Score, bars: u64) -> Vec<PhraseNote> {
    let ticks_per_quarter = u64::from(score.ticks_per_quarter);
    let end_tick = bars * 4 * ticks_per_quarter;
    let microseconds_per_quarter = score
        .events
        .iter()
        .find_map(|event| match event.message {
            ScoreMessage::Tempo { microseconds_per_quarter } => Some(microseconds_per_quarter),
            _ => None,
        })
        .unwrap_or(500_000);
    let seconds_per_tick = f64::from(microseconds_per_quarter) / 1_000_000.0 / ticks_per_quarter as f64;
    let mut notes = Vec::new();
    for (index, event) in score.events.iter().enumerate() {
        let ScoreMessage::NoteOn { key, velocity } = event.message else {
            continue;
        };
        if event.tick >= end_tick {
            break;
        }
        let end = score.events[index + 1..]
            .iter()
            .find(|later| later.channel == event.channel && later.message == ScoreMessage::NoteOff { key })
            .map_or(event.tick + ticks_per_quarter, |later| later.tick);
        notes.push(PhraseNote {
            start_seconds: event.tick as f64 * seconds_per_tick,
            end_seconds: end as f64 * seconds_per_tick,
            key,
            velocity,
        });
    }
    notes
}

/// Commands waiting for their time, released in time order.
#[derive(Debug, Default)]
pub struct CommandQueue {
    pending: Vec<(f64, SpikeCommand)>,
}

impl CommandQueue {
    pub fn push(&mut self, at_seconds: f64, command: SpikeCommand) {
        self.pending.push((at_seconds, command));
    }

    /// Queue `phrase` on `target` for `source`, starting at `start_seconds`.
    pub fn push_phrase(&mut self, phrase: &[PhraseNote], target: SlotTarget, source: SourceKey, start_seconds: f64) {
        for note in phrase {
            let key = note.key;
            self.push(start_seconds + note.start_seconds, SpikeCommand::NoteOn { target, source, key, velocity: note.velocity });
            self.push(start_seconds + note.end_seconds, SpikeCommand::NoteOff { target, source, key });
        }
    }

    /// Move every command due by `now_seconds` into `commands`, earliest first.
    pub fn drain_due(&mut self, now_seconds: f64, commands: &mut Vec<SpikeCommand>) {
        self.pending.sort_by(|left, right| left.0.total_cmp(&right.0));
        let due = self.pending.partition_point(|(at_seconds, _)| *at_seconds <= now_seconds);
        commands.extend(self.pending.drain(..due).map(|(_, command)| command));
    }
}
