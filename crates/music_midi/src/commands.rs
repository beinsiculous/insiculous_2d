//! The one channel from the game thread to the conductor: triggers, source lifecycle and
//! transport, sent without blocking and applied by the conductor at block boundaries in
//! the order they were sent.
//!
//! **One channel, so order holds.** A trigger sent before a deactivation is drained before
//! it, always. **The reserve is accounting, not a second queue:** an in-flight count the
//! sender raises before it sends and the conductor lowers for every command it drains. A
//! trigger is admitted only while fewer than `capacity − lifecycle_reserve` commands are
//! in flight, any other command while fewer than `capacity`, so a queue full of triggers
//! always has room for a stop, a pause, a deactivation or a flush.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;

use common::clock::Instant;

use crate::position::SharedPosition;
use crate::sources::SourceId;
use crate::triggers::BindingIndex;

/// A command. Every one takes effect at the start of a rendered block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorCommand {
    /// Play the song from its beginning, whatever it was doing.
    Play,
    /// Release every note and stop the clock; the next `Play` starts from the top.
    Stop,
    /// Release every note, drop every scheduled one and hold the clock where it is;
    /// triggers are refused until `Resume`. Ignored unless playing.
    Pause,
    /// Continue from where `Pause` held the clock (or where a seek moved it since),
    /// rebuilding the channels there; nothing held before the pause sounds again.
    /// Ignored unless paused.
    Resume,
    /// Move to a tick, rebuilding the channels there and releasing every triggered note.
    /// While paused it moves the point `Resume` continues from; while stopped it has no
    /// effect, since `Play` starts at the top.
    Seek(u64),
    /// Loop at the song's end, or stop there; takes effect at the next seam.
    SetLooping(bool),
    /// Play `binding`'s part on its instrument for `source`, placed from `stamp`, the
    /// moment the producer saw the action. A source's first trigger activates it.
    Trigger {
        binding: BindingIndex,
        source: SourceId,
        stamp: Instant,
    },
    /// The source is over: its notes are released on every instrument, its scheduled
    /// ones dropped, and its activation is never live again.
    DeactivateSource { source: SourceId },
    /// End every source activated at an epoch below `flush_epoch`, and refuse every such
    /// epoch from now on; a source activated at or after it is untouched.
    StopAllSources { flush_epoch: u32 },
}

impl ConductorCommand {
    fn is_trigger(&self) -> bool {
        matches!(self, ConductorCommand::Trigger { .. })
    }
}

/// Why a command was not queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandRejected {
    #[error("the conductor's command queue is full")]
    QueueFull,
    #[error("the conductor is gone")]
    ConductorGone,
}

/// The game thread's end of a conductor: the command sender and the published position.
/// Cloning it shares both, and the in-flight count with them.
#[derive(Debug, Clone)]
pub struct ConductorHandle {
    sender: SyncSender<ConductorCommand>,
    position: Arc<SharedPosition>,
    in_flight: Arc<AtomicUsize>,
    capacity: usize,
    trigger_limit: usize,
}

impl ConductorHandle {
    /// Queue `command` without blocking. A trigger is refused once the queue is full up
    /// to the lifecycle reserve; anything else only once it is full. A refused command
    /// is counted in the published position's `commands_rejected`. A full queue drains
    /// over several blocks, `commands_per_block` at a time, so a command queued deep
    /// behind a burst takes effect a few blocks later than the next boundary.
    pub fn send(&self, command: ConductorCommand) -> Result<(), CommandRejected> {
        let limit = if command.is_trigger() {
            self.trigger_limit
        } else {
            self.capacity
        };
        // The place is claimed before the send and given back if the send fails, so two
        // senders racing for the last place cannot both take it.
        let claimed =
            self.in_flight
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |in_flight| {
                    (in_flight < limit).then_some(in_flight + 1)
                });
        if claimed.is_err() {
            self.position.count_rejected_command();
            return Err(CommandRejected::QueueFull);
        }
        self.sender.try_send(command).map_err(|error| {
            self.in_flight.fetch_sub(1, Ordering::AcqRel);
            self.position.count_rejected_command();
            match error {
                TrySendError::Full(_) => CommandRejected::QueueFull,
                TrySendError::Disconnected(_) => CommandRejected::ConductorGone,
            }
        })
    }

    pub fn position(&self) -> &Arc<SharedPosition> {
        &self.position
    }

    /// Commands sent and not yet drained.
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }
}

/// The conductor's end of the channel.
#[derive(Debug)]
pub(crate) struct CommandReceiver {
    receiver: Receiver<ConductorCommand>,
    in_flight: Arc<AtomicUsize>,
}

impl CommandReceiver {
    /// The next command, if one is queued; it no longer counts as in flight.
    pub(crate) fn try_recv(&self) -> Option<ConductorCommand> {
        match self.receiver.try_recv() {
            Ok(command) => {
                self.in_flight.fetch_sub(1, Ordering::AcqRel);
                Some(command)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

/// A bounded channel holding `capacity` commands (at least one), `lifecycle_reserve` of
/// them closed to triggers; its handle, the receiver the conductor drains, and the
/// position they share.
pub(crate) fn command_channel(
    capacity: usize,
    lifecycle_reserve: usize,
) -> (ConductorHandle, CommandReceiver, Arc<SharedPosition>) {
    let capacity = capacity.max(1);
    let (sender, receiver) = std::sync::mpsc::sync_channel(capacity);
    let position = Arc::new(SharedPosition::default());
    let in_flight = Arc::new(AtomicUsize::new(0));
    let handle = ConductorHandle {
        sender,
        position: Arc::clone(&position),
        in_flight: Arc::clone(&in_flight),
        capacity,
        trigger_limit: capacity.saturating_sub(lifecycle_reserve),
    };
    (
        handle,
        CommandReceiver {
            receiver,
            in_flight,
        },
        position,
    )
}
