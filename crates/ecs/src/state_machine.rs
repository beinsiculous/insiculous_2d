//! State machine component for entity behaviors.
//!
//! Replaces flat enum-matching with per-entity state machines that track the
//! current state, the one previous state, a just-entered flag and the time spent
//! in the current state. There is no deeper history; a "return to the state
//! before last" needs a stack of its own.
//! Systems read the state and act on it — no callbacks, pure ECS style.
//!
//! # Example
//! ```
//! use ecs::{World, StateMachine};
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum PlayerState {
//!     Idle,
//!     Running { speed: f32 },
//! }
//!
//! let mut world = World::new();
//! let player = world.create_entity();
//! world.add_component(&player, StateMachine::new(PlayerState::Idle)).ok();
//!
//! // In a system:
//! if let Some(machine) = world.get_mut::<StateMachine<PlayerState>>(player) {
//!     machine.transition_to(PlayerState::Running { speed: 200.0 });
//!     assert!(machine.just_entered());
//!     assert_eq!(*machine.previous().unwrap(), PlayerState::Idle);
//! }
//! ```

use std::fmt::Debug;

use serde::{Deserialize, Serialize};

/// A state machine component that tracks state transitions for an entity.
///
/// `S` is the state type — typically an enum defining all possible states.
/// The state machine tracks the current state, previous state, whether a
/// transition just occurred, and how long the entity has been in the
/// current state.
///
/// State machines are pure data — systems read the current state and
/// decide what to do. No callbacks or closures, keeping things ECS-friendly.
///
/// Serde covers the machine's whole pose, for components that embed one
/// (an authored scene records the table, not the pose — see
/// [`ClipStateMachine`](crate::clip_state_machine::ClipStateMachine)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine<S: Clone + PartialEq + Debug + Send + Sync + 'static> {
    current: S,
    previous: Option<S>,
    just_transitioned: bool,
    elapsed: f32,
}

impl<S: Clone + PartialEq + Debug + Send + Sync + 'static> StateMachine<S> {
    /// Create a new state machine in the given initial state.
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            previous: None,
            just_transitioned: true, // first frame counts as "just entered"
            elapsed: 0.0,
        }
    }

    /// Get the current state.
    pub fn current(&self) -> &S {
        &self.current
    }

    /// Get the previous state (None if no transition has occurred yet).
    pub fn previous(&self) -> Option<&S> {
        self.previous.as_ref()
    }

    /// Transition to a new state.
    ///
    /// If the new state equals the current state (by PartialEq), this is
    /// a no-op — the transition flag is NOT set.
    pub fn transition_to(&mut self, new_state: S) {
        if self.current == new_state {
            return;
        }
        self.previous = Some(std::mem::replace(&mut self.current, new_state));
        self.just_transitioned = true;
        self.elapsed = 0.0;
    }

    /// Returns true during the first tick after a transition (or on creation).
    /// Cleared by `tick()`.
    pub fn just_entered(&self) -> bool {
        self.just_transitioned
    }

    /// How long (in seconds) the entity has been in the current state.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Advance the state machine by `delta_time` seconds.
    /// Clears the `just_entered` flag after the first tick.
    pub fn tick(&mut self, delta_time: f32) {
        self.just_transitioned = false;
        self.elapsed += delta_time;
    }

    /// Check if the current state matches a value.
    pub fn is(&self, state: &S) -> bool {
        self.current == *state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum PlayerState {
        Idle,
        Running { speed: f32 },
        Jumping { velocity: f32 },
    }

    #[test]
    fn test_transition_updates_current_and_previous() {
        let mut machine = StateMachine::new(PlayerState::Idle);
        machine.tick(1.0);

        machine.transition_to(PlayerState::Running { speed: 200.0 });

        assert_eq!(*machine.current(), PlayerState::Running { speed: 200.0 });
        assert_eq!(machine.previous(), Some(&PlayerState::Idle));
        assert!(machine.is(&PlayerState::Running { speed: 200.0 }));
        assert!(machine.just_entered(), "the entered state starts fresh");
        assert_eq!(machine.elapsed(), 0.0, "elapsed restarts on every transition");

        machine.transition_to(PlayerState::Jumping { velocity: 300.0 });
        assert_eq!(
            machine.previous(),
            Some(&PlayerState::Running { speed: 200.0 }),
            "only the most recent previous state is remembered"
        );
    }

    #[test]
    fn test_same_state_transition_is_noop() {
        let mut machine = StateMachine::new(PlayerState::Idle);
        machine.tick(0.016);

        machine.transition_to(PlayerState::Idle);

        assert!(!machine.just_entered(), "the flag is not re-set");
        assert_eq!(machine.previous(), None, "no transition is recorded");
        assert!(machine.elapsed() > 0.0, "elapsed keeps counting");
    }

    #[test]
    fn test_tick_clears_just_entered_and_accumulates_time() {
        let mut machine = StateMachine::new(PlayerState::Idle);
        assert!(machine.just_entered(), "a fresh machine has just entered its initial state");

        machine.tick(0.016);
        assert!(!machine.just_entered());
        assert!((machine.elapsed() - 0.016).abs() < f32::EPSILON);

        machine.tick(0.016);
        assert!((machine.elapsed() - 0.032).abs() < f32::EPSILON);
    }
}
