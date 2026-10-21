//! What the conductor publishes for the game thread: where the song is and what it has
//! done, in atomics written once per rendered block and readable from any thread.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

/// Whether the song's clock is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportState {
    /// No song is playing: nothing loaded, never played, stopped, or played to its end.
    #[default]
    Stopped,
    Playing,
    /// The clock is held; every note was released and resuming continues from here.
    Paused,
}

impl TransportState {
    fn from_code(code: u8) -> Self {
        match code {
            1 => TransportState::Playing,
            2 => TransportState::Paused,
            _ => TransportState::Stopped,
        }
    }

    fn code(self) -> u8 {
        match self {
            TransportState::Stopped => 0,
            TransportState::Playing => 1,
            TransportState::Paused => 2,
        }
    }
}

/// One block's worth of everything [`SharedPosition`] carries, as plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PositionSnapshot {
    /// The song sample the next rendered frame plays.
    pub sample_position: u64,
    /// The tick that sample plays: the last tick at or before it.
    pub tick: u64,
    pub transport: TransportState,
    /// A song that does not loop has played to its end.
    pub ended: bool,
    pub blocks_rendered: u64,
    /// Output samples that were not finite and were written as silence instead.
    pub non_finite_samples: u64,
    /// Seeks, resumes and loop wraps that rebuilt the channels.
    pub reconstructions: u64,
    pub loop_wraps: u64,
    pub commands_applied: u64,
    /// Commands a full or abandoned queue refused at the sender.
    pub commands_rejected: u64,
    /// Triggers that played a unit.
    pub accepted_triggers: u64,
    /// Triggers refused with their cursor unmoved: no room for their notes, a song not
    /// playing, or an instrument with nothing to play them on.
    pub rejected_triggers: u64,
    /// Triggers older than the late tolerance when drained, dropped.
    pub late_triggers: u64,
    /// Triggers and deactivations naming an ended source, or a binding of another song.
    pub stale_triggers: u64,
    /// Triggers that found nothing to play: an exhausted sequence, or a rest.
    pub silent_triggers: u64,
    pub live_sources: u64,
    /// Scheduled note-ons and note-offs held.
    pub pending_events: u64,
    /// Sample voices playing across every sample slot, fading ones included.
    pub sample_voices: u64,
}

/// The conductor's published position and counters. Each value is whole; a reader that
/// loads several may see them straddle one block boundary.
#[derive(Debug, Default)]
pub struct SharedPosition {
    sample_position: AtomicU64,
    tick: AtomicU64,
    transport: AtomicU8,
    ended: AtomicBool,
    blocks_rendered: AtomicU64,
    non_finite_samples: AtomicU64,
    reconstructions: AtomicU64,
    loop_wraps: AtomicU64,
    commands_applied: AtomicU64,
    commands_rejected: AtomicU64,
    accepted_triggers: AtomicU64,
    rejected_triggers: AtomicU64,
    late_triggers: AtomicU64,
    stale_triggers: AtomicU64,
    silent_triggers: AtomicU64,
    live_sources: AtomicU64,
    pending_events: AtomicU64,
    sample_voices: AtomicU64,
}

impl SharedPosition {
    /// Everything at once, read value by value.
    pub fn snapshot(&self) -> PositionSnapshot {
        PositionSnapshot {
            sample_position: self.sample_position.load(Ordering::Acquire),
            tick: self.tick.load(Ordering::Acquire),
            transport: TransportState::from_code(self.transport.load(Ordering::Acquire)),
            ended: self.ended.load(Ordering::Acquire),
            blocks_rendered: self.blocks_rendered.load(Ordering::Acquire),
            non_finite_samples: self.non_finite_samples.load(Ordering::Acquire),
            reconstructions: self.reconstructions.load(Ordering::Acquire),
            loop_wraps: self.loop_wraps.load(Ordering::Acquire),
            commands_applied: self.commands_applied.load(Ordering::Acquire),
            commands_rejected: self.commands_rejected.load(Ordering::Acquire),
            accepted_triggers: self.accepted_triggers.load(Ordering::Acquire),
            rejected_triggers: self.rejected_triggers.load(Ordering::Acquire),
            late_triggers: self.late_triggers.load(Ordering::Acquire),
            stale_triggers: self.stale_triggers.load(Ordering::Acquire),
            silent_triggers: self.silent_triggers.load(Ordering::Acquire),
            live_sources: self.live_sources.load(Ordering::Acquire),
            pending_events: self.pending_events.load(Ordering::Acquire),
            sample_voices: self.sample_voices.load(Ordering::Acquire),
        }
    }

    pub fn sample_position(&self) -> u64 {
        self.sample_position.load(Ordering::Acquire)
    }

    pub fn tick(&self) -> u64 {
        self.tick.load(Ordering::Acquire)
    }

    pub fn has_ended(&self) -> bool {
        self.ended.load(Ordering::Acquire)
    }

    pub fn transport(&self) -> TransportState {
        TransportState::from_code(self.transport.load(Ordering::Acquire))
    }

    /// Written by the conductor once per block. `commands_rejected` is the sender's and
    /// is left alone.
    pub(crate) fn publish(&self, snapshot: &PositionSnapshot) {
        self.tick.store(snapshot.tick, Ordering::Release);
        self.transport
            .store(snapshot.transport.code(), Ordering::Release);
        self.ended.store(snapshot.ended, Ordering::Release);
        self.blocks_rendered
            .store(snapshot.blocks_rendered, Ordering::Release);
        self.non_finite_samples
            .store(snapshot.non_finite_samples, Ordering::Release);
        self.reconstructions
            .store(snapshot.reconstructions, Ordering::Release);
        self.loop_wraps
            .store(snapshot.loop_wraps, Ordering::Release);
        self.commands_applied
            .store(snapshot.commands_applied, Ordering::Release);
        for (atomic, value) in [
            (&self.accepted_triggers, snapshot.accepted_triggers),
            (&self.rejected_triggers, snapshot.rejected_triggers),
            (&self.late_triggers, snapshot.late_triggers),
            (&self.stale_triggers, snapshot.stale_triggers),
            (&self.silent_triggers, snapshot.silent_triggers),
            (&self.live_sources, snapshot.live_sources),
            (&self.pending_events, snapshot.pending_events),
            (&self.sample_voices, snapshot.sample_voices),
        ] {
            atomic.store(value, Ordering::Release);
        }
        self.sample_position
            .store(snapshot.sample_position, Ordering::Release);
    }

    pub(crate) fn count_rejected_command(&self) {
        self.commands_rejected.fetch_add(1, Ordering::AcqRel);
    }
}
