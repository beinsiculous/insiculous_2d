//! Beat edges: which musical boundaries the song crossed between two updates, counted on
//! the published sample position by integer boundaries through the tempo map, never by
//! comparing floating-point times.
//!
//! An update reports each subdivision at most once, however many of its boundaries the
//! gap since the last update held — a suspended tab or a long frame never floods a
//! game with beats. A loop wrap lands on tick 0, a boundary of every subdivision, so it
//! reports all of them; the first update of a playing song reports all of them, for the
//! downbeat it starts on.
//!
//! **A seek the host sent is not a crossing**, forward or back: the conductor applies it at
//! a block boundary, in order, so the edges are told the seek's ordinal among the commands
//! the host sent and wait for the conductor's applied count to reach it; nothing is a
//! crossing while a seek is in flight — its position may jump at any block — and the reading
//! after the one that shows it applied re-seeds: the conductor publishes its counters before
//! its position, one atomic at a time, so the showing reading may pair the new count with the
//! old position. The count is the commands', not the sequencer's
//! reconstructions, which a loop wrap or a resume also raise, and which two seeks sent before
//! a block renders would collapse into one. A conductor that has rendered nothing yet has
//! published nothing, so its reading is skipped rather than taken for a song at its start.

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
    /// The host's last seek, from its send until the reading after the one that showed it
    /// applied.
    seek: Option<SeekInFlight>,
}

/// Where the host's last seek stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeekInFlight {
    /// Sent as this ordinal among the host's commands; applied once the conductor's count
    /// reaches it.
    Sent { ordinal: u64 },
    /// Applied under the last reading, whose position may still be the old one: the next
    /// reading re-seeds.
    Applied,
}

#[derive(Debug, Clone, Copy)]
struct Seen {
    sample_position: u64,
    loop_wraps: u64,
}

impl Seen {
    fn of(snapshot: &PositionSnapshot) -> Self {
        Self {
            sample_position: snapshot.sample_position,
            loop_wraps: snapshot.loop_wraps,
        }
    }
}

impl BeatEdges {
    pub(super) fn crossed(&self, subdivision: Subdivision) -> bool {
        self.crossed[subdivision.index()]
    }

    /// Count from `snapshot`'s reading on, nothing crossed. Never `None`, which would
    /// report every subdivision at the next update.
    pub(super) fn reseed(&mut self, snapshot: &PositionSnapshot) {
        self.seen = Some(Seen::of(snapshot));
        self.crossed = [false; 4];
    }

    /// The host sent a seek as the `ordinal`th command the conductor accepted: nothing is
    /// a crossing until the conductor has applied that many, and the reading that shows it
    /// has re-seeds instead of counting.
    pub(super) fn expect_seek(&mut self, ordinal: u64) {
        self.seek = Some(SeekInFlight::Sent { ordinal });
    }

    /// Whether a host seek is sent and not yet landed.
    pub(super) fn seek_in_flight(&self) -> bool {
        self.seek.is_some()
    }

    /// The edges a replacement song continues with: this update's crossings kept for the
    /// game to read, the next update counted from `snapshot`, the replacement's reading on
    /// its own map.
    pub(super) fn handed_over(&self, snapshot: &PositionSnapshot) -> Self {
        let mut edges = Self::default();
        edges.reseed(snapshot);
        edges.crossed = self.crossed;
        edges
    }

    /// Latch the edges between the last update's position and `snapshot`'s. `true` when
    /// this reading is the host's seek landing, which re-seeds and crosses nothing.
    pub(super) fn update(
        &mut self,
        snapshot: &PositionSnapshot,
        tempo_map: &TempoMap,
        sample_rate: u32,
    ) -> bool {
        if snapshot.blocks_rendered == 0 {
            self.crossed = [false; 4];
            return false;
        }
        match self.seek {
            Some(SeekInFlight::Applied) => {
                self.seek = None;
                self.reseed(snapshot);
                return true;
            }
            Some(SeekInFlight::Sent { ordinal }) => {
                if snapshot.commands_applied >= ordinal {
                    self.seek = Some(SeekInFlight::Applied);
                }
                // In flight, or applied under a reading that may pair the new count with
                // the old position: this reading counts nothing and is kept current.
                self.crossed = [false; 4];
                self.seen = Some(Seen::of(snapshot));
                return false;
            }
            None => {}
        }
        let now = Seen::of(snapshot);
        let previous = if snapshot.transport == TransportState::Playing {
            self.seen.replace(now)
        } else {
            self.crossed = [false; 4];
            if self.seen.is_some() {
                self.seen = Some(now);
            }
            return false;
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
        false
    }
}
