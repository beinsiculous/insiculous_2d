//! The one channel from the game thread to the conductor: transport commands, sent
//! without blocking, applied by the conductor at its next block boundary in the order
//! they were sent.

use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::Arc;

use crate::position::SharedPosition;

/// A transport command. Every one takes effect at the start of a rendered block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorCommand {
    /// Play the song from its beginning, whatever it was doing.
    Play,
    /// Release every note and stop the clock; the next `Play` starts from the top.
    Stop,
    /// Release every note and hold the clock where it is. Ignored unless playing.
    Pause,
    /// Continue from where `Pause` held the clock (or where a seek moved it since),
    /// rebuilding the channels there; nothing held before the pause sounds again.
    /// Ignored unless paused.
    Resume,
    /// Move to a tick, rebuilding the channels there. While paused it moves the point
    /// `Resume` continues from; while stopped it has no effect, since `Play` starts at
    /// the top.
    Seek(u64),
    /// Loop at the song's end, or stop there; takes effect at the next seam.
    SetLooping(bool),
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
/// Cloning it shares both.
#[derive(Debug, Clone)]
pub struct ConductorHandle {
    sender: SyncSender<ConductorCommand>,
    position: Arc<SharedPosition>,
}

impl ConductorHandle {
    /// Queue `command` without blocking. A refused command is counted in the published
    /// position's `commands_rejected`.
    pub fn send(&self, command: ConductorCommand) -> Result<(), CommandRejected> {
        self.sender.try_send(command).map_err(|error| {
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
}

/// A bounded channel holding `capacity` commands (at least one), its handle, and the
/// receiver the conductor drains.
pub(crate) fn command_channel(
    capacity: usize,
) -> (
    ConductorHandle,
    Receiver<ConductorCommand>,
    Arc<SharedPosition>,
) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(capacity.max(1));
    let position = Arc::new(SharedPosition::default());
    let handle = ConductorHandle {
        sender,
        position: Arc::clone(&position),
    };
    (handle, receiver, position)
}
