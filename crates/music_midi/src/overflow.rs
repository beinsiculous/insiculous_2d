//! The bounded half of triggering: the scheduled events a conductor holds, and the rule
//! that refuses a trigger rather than let them overflow.
//!
//! **Scheduled events** are a quantized trigger's note-ons waiting for their grid
//! boundary and every triggered note's note-off waiting for its authored end. They live
//! in one buffer allocated at load and never grown.
//!
//! **A trigger is refused whole, before its cursor moves,** when its notes do not fit:
//! the buffer has no room for one event per note, or — for a trigger scheduled ahead —
//! its notes added to the instrument's note-ons already waiting would pass the
//! instrument's sounding cap. Sounding notes are the ledger's to steal; notes not yet
//! started would only steal each other at their boundary, so the newest is refused. A
//! note-off whose note the ledger has already released is dead weight, and is purged
//! before a trigger is refused for room.

use crate::conductor::NoteTicket;
use crate::sources::SourceId;

/// What a scheduled event does when its sample comes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingAction {
    /// Start a note, then schedule its note-off `duration_samples` later.
    Start {
        key: u8,
        velocity: u8,
        duration_samples: u64,
    },
    /// End the note `ticket` started, if it still sounds.
    Release { ticket: NoteTicket },
}

/// One scheduled event: on which slot, for which source, at which song sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingEvent {
    pub due_sample: u64,
    pub slot: usize,
    pub source: SourceId,
    pub action: PendingAction,
}

/// Why a trigger's notes were refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overflow {
    /// The buffer has no room for one event per note.
    PendingFull,
    /// The instrument already waits to start as many notes as it can hold sounding.
    InstrumentFull,
}

/// The scheduled events, bounded at construction. See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct PendingEvents {
    /// In the order they were scheduled, so events due together fire in command order.
    events: Vec<PendingEvent>,
    capacity: usize,
    /// Note-ons waiting, per slot.
    waiting_starts: Vec<usize>,
}

impl PendingEvents {
    /// Room for `capacity` events over `slot_count` slots, allocated now.
    pub(crate) fn new(capacity: usize, slot_count: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            capacity,
            waiting_starts: vec![0; slot_count],
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether `notes` more events fit for `slot`; a scheduled trigger also needs its
    /// instrument's waiting note-ons to stay within `sounding_cap`. `is_sounding` tells
    /// the purge which note-offs still have a note to end.
    pub(crate) fn admit(
        &mut self,
        slot: usize,
        notes: usize,
        scheduled: bool,
        sounding_cap: usize,
        is_sounding: impl Fn(usize, NoteTicket) -> bool,
    ) -> Result<(), Overflow> {
        let waiting = self.waiting_starts.get(slot).copied().unwrap_or(0);
        if scheduled && waiting + notes > sounding_cap {
            return Err(Overflow::InstrumentFull);
        }
        if self.events.len() + notes > self.capacity {
            self.events.retain(|event| match event.action {
                PendingAction::Release { ticket } => is_sounding(event.slot, ticket),
                PendingAction::Start { .. } => true,
            });
        }
        if self.events.len() + notes > self.capacity {
            return Err(Overflow::PendingFull);
        }
        Ok(())
    }

    /// Add `event`; [`PendingEvents::admit`] said it fits. One that does not is dropped.
    pub(crate) fn push(&mut self, event: PendingEvent) {
        if self.events.len() >= self.capacity {
            return;
        }
        if matches!(event.action, PendingAction::Start { .. }) {
            if let Some(waiting) = self.waiting_starts.get_mut(event.slot) {
                *waiting += 1;
            }
        }
        self.events.push(event);
    }

    /// Hand `fire` every event due before `due_before`, in the order they were scheduled.
    /// `fire` returns the event that replaces it (a started note's note-off) or `None`.
    pub(crate) fn fire_due(
        &mut self,
        due_before: u64,
        mut fire: impl FnMut(&PendingEvent) -> Option<PendingEvent>,
    ) {
        let waiting_starts = &mut self.waiting_starts;
        self.events.retain_mut(|event| {
            if event.due_sample >= due_before {
                return true;
            }
            if matches!(event.action, PendingAction::Start { .. }) {
                if let Some(waiting) = waiting_starts.get_mut(event.slot) {
                    *waiting = waiting.saturating_sub(1);
                }
            }
            match fire(event) {
                Some(replacement) => {
                    *event = replacement;
                    true
                }
                None => false,
            }
        });
    }

    /// Drop every event of `source`.
    pub(crate) fn remove_source(&mut self, source: SourceId) {
        let waiting_starts = &mut self.waiting_starts;
        self.events.retain(|event| {
            if event.source != source {
                return true;
            }
            if matches!(event.action, PendingAction::Start { .. }) {
                if let Some(waiting) = waiting_starts.get_mut(event.slot) {
                    *waiting = waiting.saturating_sub(1);
                }
            }
            false
        });
    }

    /// Drop every event; the caller releases the notes their note-offs would have ended.
    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.waiting_starts.fill(0);
    }
}
