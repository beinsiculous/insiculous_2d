//! The song triggers scripts asked for this frame, held until the frame tail submits
//! them: a script's commands apply to the world alone and cannot reach the audio manager.

use audio::AudioManager;
use ecs::EntityId;

use super::runner::ScriptRunner;
use crate::sound_sources::SoundSources;

/// One `cmd.trigger`: the binding to play, for an entity's source or, with no entity,
/// the game's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedTrigger {
    pub source: Option<EntityId>,
    pub binding: String,
}

/// The frame's triggers in the order their commands applied: phase by phase, each in
/// the runner's `(entity, script index)` order, a script's own in the order it pushed
/// them.
#[derive(Debug, Default)]
pub struct TriggerQueue {
    triggers: Vec<QueuedTrigger>,
    dropped: u64,
}

/// The most triggers one frame holds. Only the runner the engine lends as `ctx.scripts`
/// is drained by the frame tail; a host that steps a `ScriptRunner` of its own never
/// hears its triggers, and without a cap would grow this queue every frame.
pub const TRIGGER_QUEUE_CAPACITY: usize = 256;

impl TriggerQueue {
    /// Queue a trigger; past the capacity it is dropped and counted, logged at debug on
    /// the powers of two so a mis-wired host shows up in the log without flooding it.
    pub fn push(&mut self, source: Option<EntityId>, binding: String) {
        if self.triggers.len() >= TRIGGER_QUEUE_CAPACITY {
            self.dropped += 1;
            if self.dropped.is_power_of_two() {
                log::debug!(
                    "trigger '{binding}' dropped: {} queued and never drained — only the \
                     runner lent as ctx.scripts reaches the song; {} dropped so far",
                    self.triggers.len(),
                    self.dropped
                );
            }
            return;
        }
        self.triggers.push(QueuedTrigger { source, binding });
    }

    /// Triggers dropped past the capacity since the queue was made.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Everything queued, oldest first; the queue is left empty.
    pub fn take(&mut self) -> Vec<QueuedTrigger> {
        std::mem::take(&mut self.triggers)
    }

    pub fn clear(&mut self) {
        self.triggers.clear();
    }
}

impl ScriptRunner {
    /// The song triggers this frame's phases queued, in the order they applied; the
    /// engine's frame tail submits them.
    pub fn take_triggers(&mut self) -> Vec<QueuedTrigger> {
        self.triggers.take()
    }
}

/// Play each trigger for its entity's source, or the game's, in order. A refusal is
/// counted by `sources`, never a script error.
pub(crate) fn submit(triggers: Vec<QueuedTrigger>, sources: &mut SoundSources, audio: &mut AudioManager) {
    for trigger in triggers {
        let source = match trigger.source {
            Some(entity) => sources.entity(entity),
            None => sources.game(),
        };
        sources.trigger(audio, &trigger.binding, source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queue_nobody_drains_stops_growing_at_its_capacity_and_counts_the_rest() {
        let mut queue = TriggerQueue::default();
        for _ in 0..TRIGGER_QUEUE_CAPACITY + 3 {
            queue.push(None, "hit".to_string());
        }
        assert_eq!(queue.dropped(), 3);
        assert_eq!(queue.take().len(), TRIGGER_QUEUE_CAPACITY);
        assert_eq!(queue.dropped(), 3, "taking the queue keeps the count");
    }
}
