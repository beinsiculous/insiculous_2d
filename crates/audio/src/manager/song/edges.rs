//! Beat edges: which musical boundaries the song crossed between two updates, counted on
//! the published sample position by integer boundaries through the tempo map, never by
//! comparing floating-point times.
//!
//! An update reports each subdivision at most once, however many of its boundaries the
//! gap since the last update held — a suspended tab or a long frame never floods a
//! game with beats. A loop wrap lands on tick 0, a boundary of every subdivision, so it
//! reports all of them; a seek backwards reports none; the first update of a playing song
//! reports all of them, for the downbeat it starts on.

use music_midi::{PositionSnapshot, TempoMap, TransportState};

/// A musical grid a game can wait on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subdivision {
    Bar,
    /// The time signature's denominator note.
    Beat,
    /// Eighth notes laid from each beat's start; a beat shorter than an eighth is its
    /// own subdivision.
    Eighth,
    /// Sixteenth notes laid from each beat's start, as eighths are.
    Sixteenth,
}

impl Subdivision {
    const ALL: [Subdivision; 4] = [
        Subdivision::Bar,
        Subdivision::Beat,
        Subdivision::Eighth,
        Subdivision::Sixteenth,
    ];

    fn index(self) -> usize {
        match self {
            Subdivision::Bar => 0,
            Subdivision::Beat => 1,
            Subdivision::Eighth => 2,
            Subdivision::Sixteenth => 3,
        }
    }

    /// Where `sample_position` falls on this grid, ordered as the song plays: a later
    /// sample is never a smaller place.
    fn place(self, tempo_map: &TempoMap, sample_position: u64, sample_rate: u32) -> (u64, u64) {
        let tick = tempo_map.tick_at_sample(sample_position, sample_rate);
        let musical = tempo_map.position_at_tick(tick);
        let quarter = u64::from(tempo_map.ticks_per_quarter());
        let step_ticks = match self {
            Subdivision::Bar => return (musical.bar, 0),
            Subdivision::Beat => return (musical.beat, 0),
            Subdivision::Eighth => (quarter / 2).max(1),
            Subdivision::Sixteenth => (quarter / 4).max(1),
        };
        let beat_start = tempo_map.tick_at_beat(musical.beat);
        (musical.beat, (tick - beat_start) / step_ticks)
    }
}

/// The last update's edges and where it saw the song.
#[derive(Debug, Clone, Default)]
pub(super) struct BeatEdges {
    seen: Option<Seen>,
    crossed: [bool; 4],
}

#[derive(Debug, Clone, Copy)]
struct Seen {
    sample_position: u64,
    loop_wraps: u64,
}

impl BeatEdges {
    pub(super) fn crossed(&self, subdivision: Subdivision) -> bool {
        self.crossed[subdivision.index()]
    }

    /// Latch the edges between the last update's position and `snapshot`'s.
    pub(super) fn update(
        &mut self,
        snapshot: &PositionSnapshot,
        tempo_map: &TempoMap,
        sample_rate: u32,
    ) {
        let now = Seen {
            sample_position: snapshot.sample_position,
            loop_wraps: snapshot.loop_wraps,
        };
        let previous = if snapshot.transport == TransportState::Playing {
            self.seen.replace(now)
        } else {
            self.crossed = [false; 4];
            if self.seen.is_some() {
                self.seen = Some(now);
            }
            return;
        };
        for subdivision in Subdivision::ALL {
            self.crossed[subdivision.index()] = match previous {
                None => true,
                Some(previous) if now.loop_wraps != previous.loop_wraps => true,
                Some(previous) if now.sample_position <= previous.sample_position => false,
                Some(previous) if subdivision == Subdivision::Beat => {
                    tempo_map.beats_crossed(
                        previous.sample_position,
                        now.sample_position,
                        sample_rate,
                    ) > 0
                }
                Some(previous) => {
                    subdivision.place(tempo_map, now.sample_position, sample_rate)
                        > subdivision.place(tempo_map, previous.sample_position, sample_rate)
                }
            };
        }
    }
}
