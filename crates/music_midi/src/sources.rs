//! Sources: whoever triggers a song's bindings — an entity, a menu — named by an identity
//! that a recycled entity id or a re-entry can never revive, and the conductor's table of
//! which of them are live.
//!
//! **An identity is three numbers.** `id` and `generation` mirror the host's entity id, so
//! a recycled id is a new generation. `epoch` is the host's activation counter, one
//! number rising across every source, and each activation takes a new one: a source that
//! leaves the screen and comes back is a new epoch, and a flush can name the point in
//! that count before which every activation is over.
//!
//! **Activation is lazy.** A source's first trigger makes it live; there is no activate
//! command, so a trigger can never arrive before its activation. A source stops being
//! live when it is deactivated, flushed, superseded (a newer generation or epoch of the
//! same id was named), or evicted (the table was full); the conductor then releases its
//! notes and drops its scheduled events.
//!
//! **A dead activation stays dead.** The table remembers, per id, the newest activation
//! it has seen; anything naming an older one, or one already ended, is refused, and a
//! flush raises a floor below which every epoch is refused even once its entry is gone.

#[cfg(test)]
mod tests;

/// A source's identity. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId {
    pub id: u64,
    pub generation: u64,
    pub epoch: u32,
}

impl SourceId {
    /// Activations of one id order by generation, then by epoch.
    fn activation(&self) -> (u64, u32) {
        (self.generation, self.epoch)
    }
}

/// What the table made of a trigger's source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Live already, or activated now. `ended` is the source that stopped being live to
    /// make way — an older activation of the same id, or the table's oldest when it was
    /// full — whose notes the caller releases.
    Live { ended: Option<SourceId> },
    /// An activation already over, or older than one the table has seen.
    Dead,
}

/// What a deactivation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deactivation {
    /// These sources stopped being live: the one named, an older activation of its id
    /// the host had already moved past, either, or neither when the source never
    /// triggered. The caller releases their notes.
    Ended {
        named: Option<SourceId>,
        older: Option<SourceId>,
    },
    /// The activation named was already over.
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    source: SourceId,
    live: bool,
    /// When it was activated; the smallest is the oldest.
    order: u64,
}

/// The live sources and the newest activation seen of each id, bounded at construction.
/// See the module docs.
#[derive(Debug, Clone)]
pub struct SourceTable {
    entries: Vec<Entry>,
    capacity: usize,
    /// Every epoch below it was flushed.
    flush_floor: u32,
    next_order: u64,
}

impl SourceTable {
    /// A table holding at most `capacity` ids (at least one), allocated now.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
            flush_floor: 0,
            next_order: 0,
        }
    }

    /// Admit a trigger's source, activating it if this is its first trigger.
    pub fn admit(&mut self, source: SourceId) -> Admission {
        if source.epoch < self.flush_floor {
            return Admission::Dead;
        }
        let order = self.next_order;
        if let Some(entry) = self.entry_mut(source.id) {
            let ended = match source.activation().cmp(&entry.source.activation()) {
                std::cmp::Ordering::Less => return Admission::Dead,
                std::cmp::Ordering::Equal if entry.live => return Admission::Live { ended: None },
                std::cmp::Ordering::Equal => return Admission::Dead,
                std::cmp::Ordering::Greater => entry.live.then_some(entry.source),
            };
            *entry = Entry {
                source,
                live: true,
                order,
            };
            self.next_order += 1;
            return Admission::Live { ended };
        }
        let ended = self.insert(Entry {
            source,
            live: true,
            order,
        });
        self.next_order += 1;
        Admission::Live { ended }
    }

    /// Deactivate `source`: it is over, and so is any older activation of its id.
    pub fn deactivate(&mut self, source: SourceId) -> Deactivation {
        if source.epoch < self.flush_floor {
            return Deactivation::Dead;
        }
        let order = self.next_order;
        let dead = Entry {
            source,
            live: false,
            order,
        };
        if let Some(entry) = self.entry_mut(source.id) {
            let ended = match source.activation().cmp(&entry.source.activation()) {
                std::cmp::Ordering::Less => return Deactivation::Dead,
                std::cmp::Ordering::Equal if entry.live => Deactivation::Ended {
                    named: Some(source),
                    older: None,
                },
                std::cmp::Ordering::Equal => return Deactivation::Dead,
                std::cmp::Ordering::Greater => Deactivation::Ended {
                    named: None,
                    older: entry.live.then_some(entry.source),
                },
            };
            *entry = dead;
            self.next_order += 1;
            return ended;
        }
        // Remembered only where it costs no live source its place.
        if self.entries.len() < self.capacity || self.entries.iter().any(|entry| !entry.live) {
            self.insert(dead);
            self.next_order += 1;
        }
        Deactivation::Ended {
            named: None,
            older: None,
        }
    }

    /// End every source activated below `flush_epoch`, telling `ended` each live one, and
    /// refuse every such epoch from now on.
    pub fn flush(&mut self, flush_epoch: u32, mut ended: impl FnMut(SourceId)) {
        self.flush_floor = self.flush_floor.max(flush_epoch);
        let floor = self.flush_floor;
        self.entries.retain(|entry| {
            if entry.source.epoch >= floor {
                return true;
            }
            if entry.live {
                ended(entry.source);
            }
            false
        });
    }

    /// Forget every source, the flush floor kept: a new song starts with none live.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Every epoch below it is refused.
    pub fn flush_floor(&self) -> u32 {
        self.flush_floor
    }

    pub fn is_live(&self, source: SourceId) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.live && entry.source == source)
    }

    pub fn live_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.live).count()
    }

    fn entry_mut(&mut self, id: u64) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| entry.source.id == id)
    }

    /// Add `entry`, making room when full: the oldest dead entry goes first, else the
    /// oldest live source, which is returned so the caller can release it.
    fn insert(&mut self, entry: Entry) -> Option<SourceId> {
        if self.entries.len() < self.capacity {
            self.entries.push(entry);
            return None;
        }
        let oldest_dead = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, candidate)| !candidate.live)
            .min_by_key(|(_, candidate)| candidate.order)
            .map(|(index, _)| index);
        let (index, evicted) = match oldest_dead {
            Some(index) => (index, None),
            None => {
                let (index, oldest) = self
                    .entries
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, candidate)| candidate.order)
                    .map(|(index, candidate)| (index, candidate.source))?;
                (index, Some(oldest))
            }
        };
        self.entries[index] = entry;
        evicted
    }
}
