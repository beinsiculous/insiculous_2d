//! The voice ledger: which notes an instrument slot holds sounding, on which lane, for
//! which source — known exactly, because the ledger issues every note-on and note-off.
//!
//! **Sounding notes are bounded exactly.** Past the sounding cap the oldest sounding note
//! gets a real note-off (it becomes a tail) to make room, so the synthesizer's own voice
//! stealer, which cuts without a fade, only ever takes a tail.
//!
//! **Lanes are leased while sounding.** A source holds one lane (a MIDI channel of the
//! slot's synthesizer) while it has sounding notes, so a note-off from one source can
//! never release another's note of the same key. When no lane is free, the source that
//! leased longest ago has its notes released and loses its lane to the newcomer: a new
//! source is never refused.
//!
//! **A same-key strike on a lane** releases the old note first, then sounds the new one.

#[cfg(test)]
mod tests;

/// Whoever triggers notes on a slot. The conductor keeps one per active source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceKey(pub u64);

/// What the ledger tells the synthesizer to do, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerAction {
    NoteOn { lane: usize, key: u8, velocity: u8 },
    NoteOff { lane: usize, key: u8 },
}

/// What one note-on cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoteStart {
    /// The source held no lane and none was free, so another source lost its lane.
    pub lane_stolen: bool,
    /// The same key was sounding on the source's lane and was released first.
    pub retriggered: bool,
    /// Notes released because the slot was at its sounding cap.
    pub notes_stolen: usize,
}

/// Running totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LedgerCounters {
    pub notes_started: u64,
    pub steals: u64,
    pub lane_steals: u64,
    pub retriggers: u64,
    pub peak_sounding: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Lease {
    source: SourceKey,
    /// Leases are ordered by when they were taken; the smallest is the oldest.
    order: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SoundingNote {
    source: SourceKey,
    lane: usize,
    key: u8,
}

/// The ledger of one slot. See the module docs.
#[derive(Debug, Clone)]
pub struct VoiceLedger {
    leases: Vec<Option<Lease>>,
    /// Oldest first, never longer than `sounding_cap`.
    sounding: Vec<SoundingNote>,
    sounding_cap: usize,
    next_lease_order: u64,
    counters: LedgerCounters,
}

impl VoiceLedger {
    /// A ledger over `lane_count` lanes (at least one) holding at most `sounding_cap`
    /// notes (at least one).
    pub fn new(lane_count: usize, sounding_cap: usize) -> Self {
        let sounding_cap = sounding_cap.max(1);
        Self {
            leases: vec![None; lane_count.max(1)],
            sounding: Vec::with_capacity(sounding_cap),
            sounding_cap,
            next_lease_order: 0,
            counters: LedgerCounters::default(),
        }
    }

    /// Start `key` for `source`, telling `act` every note-off it costs and then the
    /// note-on.
    pub fn note_on(
        &mut self,
        source: SourceKey,
        key: u8,
        velocity: u8,
        mut act: impl FnMut(LedgerAction),
    ) -> NoteStart {
        let mut start = NoteStart::default();
        let lane = match self.lane_of(source) {
            Some(lane) => lane,
            None => {
                let (lane, stolen) = self.lease_lane(source, &mut act);
                start.lane_stolen = stolen;
                lane
            }
        };
        if let Some(index) = self
            .sounding
            .iter()
            .position(|note| note.lane == lane && note.key == key)
        {
            self.sounding.remove(index);
            act(LedgerAction::NoteOff { lane, key });
            self.counters.retriggers += 1;
            start.retriggered = true;
        }
        while self.sounding.len() >= self.sounding_cap {
            let oldest = self.sounding.remove(0);
            act(LedgerAction::NoteOff {
                lane: oldest.lane,
                key: oldest.key,
            });
            // The newcomer keeps its lane even when the steal took its own last note.
            if oldest.source != source {
                self.free_lane_if_silent(oldest.source, oldest.lane);
            }
            self.counters.steals += 1;
            start.notes_stolen += 1;
        }
        act(LedgerAction::NoteOn {
            lane,
            key,
            velocity,
        });
        self.sounding.push(SoundingNote { source, lane, key });
        self.counters.notes_started += 1;
        self.counters.peak_sounding = self.counters.peak_sounding.max(self.sounding.len());
        start
    }

    /// Release `key` for `source`. A note already released (stolen, retriggered) sends
    /// nothing: its lane may belong to another source by now.
    pub fn note_off(&mut self, source: SourceKey, key: u8, mut act: impl FnMut(LedgerAction)) {
        let Some(index) = self
            .sounding
            .iter()
            .position(|note| note.source == source && note.key == key)
        else {
            return;
        };
        let note = self.sounding.remove(index);
        act(LedgerAction::NoteOff {
            lane: note.lane,
            key: note.key,
        });
        self.free_lane_if_silent(source, note.lane);
    }

    /// Release every note `source` holds and free its lane.
    pub fn release_source(&mut self, source: SourceKey, mut act: impl FnMut(LedgerAction)) {
        self.sounding.retain(|note| {
            if note.source == source {
                act(LedgerAction::NoteOff {
                    lane: note.lane,
                    key: note.key,
                });
                false
            } else {
                true
            }
        });
        for lease in &mut self.leases {
            if lease.is_some_and(|lease| lease.source == source) {
                *lease = None;
            }
        }
    }

    /// Forget every note and lease; the caller releases the synthesizer's voices.
    pub fn clear(&mut self) {
        self.sounding.clear();
        self.leases.fill(None);
    }

    pub fn sounding_count(&self) -> usize {
        self.sounding.len()
    }

    /// The lane `source` holds, if it has sounding notes.
    pub fn lane_of(&self, source: SourceKey) -> Option<usize> {
        self.leases
            .iter()
            .position(|lease| lease.is_some_and(|lease| lease.source == source))
    }

    pub fn counters(&self) -> LedgerCounters {
        self.counters
    }

    /// A free lane for `source`, or the oldest lease's lane with its notes released.
    fn lease_lane(
        &mut self,
        source: SourceKey,
        act: &mut impl FnMut(LedgerAction),
    ) -> (usize, bool) {
        let order = self.next_lease_order;
        self.next_lease_order += 1;
        let lease = Some(Lease { source, order });
        if let Some(lane) = self.leases.iter().position(Option::is_none) {
            self.leases[lane] = lease;
            return (lane, false);
        }
        let (lane, oldest) = self
            .leases
            .iter()
            .enumerate()
            .filter_map(|(lane, lease)| lease.map(|lease| (lane, lease)))
            .min_by_key(|(_, lease)| lease.order)
            .unwrap_or((0, Lease { source, order }));
        self.release_source(oldest.source, act);
        self.leases[lane] = lease;
        self.counters.lane_steals += 1;
        (lane, true)
    }

    fn free_lane_if_silent(&mut self, source: SourceKey, lane: usize) {
        if !self.sounding.iter().any(|note| note.source == source) {
            self.leases[lane] = None;
        }
    }
}
