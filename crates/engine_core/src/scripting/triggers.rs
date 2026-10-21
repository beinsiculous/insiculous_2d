//! The sounds scripts asked for this frame — song triggers and emitter plays and stops —
//! held in one queue until the frame tail submits them: a script's commands apply to the
//! world alone and cannot reach the audio manager. One queue, so `stop_sound(me)` then
//! `trigger(me, "hit")` keeps its order.

use audio::AudioManager;
use ecs::{EntityId, SoundEmitter, World};
use glam::Vec2;

use super::runner::ScriptRunner;
use crate::sound_emitters::SoundEmitters;
use crate::sound_sources::SoundSources;

/// One script sound request.
#[derive(Debug, Clone, PartialEq)]
pub enum QueuedSound {
    /// `cmd.trigger`: the binding to play, for an entity's source or, with no entity,
    /// the game's.
    Trigger { source: Option<EntityId>, binding: String },
    /// `cmd.play_sound`: the entity's emitter and position as they were when the command
    /// applied, so a one-shot can still sound after its entity despawns that frame.
    Play { entity: EntityId, emitter: Option<SoundEmitter>, position: Option<Vec2> },
    /// `cmd.stop_sound`.
    Stop { entity: EntityId },
}

/// The frame's sound requests in the order their commands applied: phase by phase, each
/// in the runner's `(entity, script index)` order, a script's own in the order it pushed
/// them.
#[derive(Debug, Default)]
pub struct TriggerQueue {
    sounds: Vec<QueuedSound>,
    dropped: u64,
}

/// The most requests one frame holds. Only the runner the engine lends as `ctx.scripts`
/// is drained by the frame tail; a host that steps a `ScriptRunner` of its own never
/// hears its sounds, and without a cap would grow this queue every frame.
pub const TRIGGER_QUEUE_CAPACITY: usize = 256;

impl TriggerQueue {
    /// Queue a request; past the capacity it is dropped and counted, logged at debug on
    /// the powers of two so a mis-wired host shows up in the log without flooding it.
    pub fn push(&mut self, sound: QueuedSound) {
        if self.sounds.len() >= TRIGGER_QUEUE_CAPACITY {
            self.dropped += 1;
            if self.dropped.is_power_of_two() {
                log::debug!(
                    "sound request {sound:?} dropped: {} queued and never drained — only the \
                     runner lent as ctx.scripts reaches the audio; {} dropped so far",
                    self.sounds.len(),
                    self.dropped
                );
            }
            return;
        }
        self.sounds.push(sound);
    }

    /// Requests dropped past the capacity since the queue was made.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Everything queued, oldest first; the queue is left empty.
    pub fn take(&mut self) -> Vec<QueuedSound> {
        std::mem::take(&mut self.sounds)
    }

    pub fn clear(&mut self) {
        self.sounds.clear();
    }
}

impl ScriptRunner {
    /// The sound requests this frame's phases queued, in the order they applied; the
    /// engine's frame tail submits them.
    pub fn take_sounds(&mut self) -> Vec<QueuedSound> {
        self.triggers.take()
    }
}

/// Serve each request in order: a trigger plays for its entity's source or the game's; a
/// binding emitter's play or stop goes to its source here, in order with the triggers; a
/// recorded emitter's is handed on, in order, to the emitter pass after the hierarchy.
/// The emitter is the entity's own while it lives, the one the command saw once it is
/// gone. A refusal is counted, never a script error.
pub(crate) fn submit(
    sounds: Vec<QueuedSound>,
    world: &World,
    sources: &mut SoundSources,
    emitters: &mut SoundEmitters,
    audio: &mut AudioManager,
) {
    for sound in sounds {
        match sound {
            QueuedSound::Trigger { source, binding } => {
                let source = match source {
                    Some(entity) => sources.entity(entity),
                    None => sources.game(),
                };
                sources.trigger(audio, &binding, source);
            }
            QueuedSound::Play { entity, emitter, position } => {
                let current = if world.validate_entity(&entity).is_ok() {
                    world.get::<SoundEmitter>(entity)
                } else {
                    emitter.as_ref()
                };
                match current.map(|current| &current.sound) {
                    Some(ecs::SoundRef::Binding(binding)) => {
                        let binding = binding.clone();
                        emitters.play_binding(entity, &binding, sources, audio);
                    }
                    _ => emitters.hand_on_play(entity, emitter.map(|emitter| (emitter, position))),
                }
            }
            QueuedSound::Stop { entity } => {
                if world.get::<SoundEmitter>(entity).is_some_and(|emitter| emitter.sound.is_binding()) {
                    emitters.stop_binding(entity, sources, audio);
                } else {
                    emitters.stop(entity);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queue_nobody_drains_stops_growing_at_its_capacity_and_counts_the_rest() {
        let mut queue = TriggerQueue::default();
        for _ in 0..TRIGGER_QUEUE_CAPACITY + 3 {
            queue.push(QueuedSound::Trigger { source: None, binding: "hit".to_string() });
        }
        assert_eq!(queue.dropped(), 3);
        assert_eq!(queue.take().len(), TRIGGER_QUEUE_CAPACITY);
        assert_eq!(queue.dropped(), 3, "taking the queue keeps the count");
    }
}
