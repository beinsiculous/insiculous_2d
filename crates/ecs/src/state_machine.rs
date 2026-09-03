//! Hierarchical state machine component for entity behaviors.
//!
//! Replaces flat enum-matching with per-entity state machines that track
//! current state, previous state, transition timing, and state history.
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
//! if let Some(sm) = world.get_mut::<StateMachine<PlayerState>>(player) {
//!     sm.transition_to(PlayerState::Running { speed: 200.0 });
//!     assert!(sm.just_entered());
//!     assert_eq!(*sm.previous().unwrap(), PlayerState::Idle);
//! }
//! ```

use std::fmt::Debug;

/// A state machine component that tracks state transitions for an entity.
///
/// `S` is the state type — typically an enum defining all possible states.
/// The state machine tracks the current state, previous state, whether a
/// transition just occurred, and how long the entity has been in the
/// current state.
///
/// State machines are pure data — systems read the current state and
/// decide what to do. No callbacks or closures, keeping things ECS-friendly.
#[derive(Debug, Clone)]
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

/// A hierarchical state machine that supports nested states.
///
/// Each state can have a parent state, enabling shared behavior for
/// state groups. For example, `OnGround` could be a parent of both
/// `Idle` and `Running`, sharing ground-detection logic.
///
/// `S` is the leaf state type, `P` is the parent/group state type.
#[derive(Debug, Clone)]
pub struct HierarchicalStateMachine<S, P>
where
    S: Clone + PartialEq + Debug + Send + Sync + 'static,
    P: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    inner: StateMachine<S>,
    parent_state: P,
    previous_parent: Option<P>,
    parent_map: fn(&S) -> P,
}

impl<S, P> HierarchicalStateMachine<S, P>
where
    S: Clone + PartialEq + Debug + Send + Sync + 'static,
    P: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    /// Create a hierarchical state machine with a parent-mapping function.
    ///
    /// The `parent_map` function defines which parent state each leaf state
    /// belongs to.
    ///
    /// # Example
    /// ```
    /// # use ecs::HierarchicalStateMachine;
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum PlayerState { Idle, Running, Jumping, Falling }
    ///
    /// #[derive(Clone, PartialEq, Debug)]
    /// enum PlayerGroup { OnGround, InAir }
    ///
    /// let mut sm = HierarchicalStateMachine::new(PlayerState::Idle, |s| match s {
    ///     PlayerState::Idle | PlayerState::Running => PlayerGroup::OnGround,
    ///     PlayerState::Jumping | PlayerState::Falling => PlayerGroup::InAir,
    /// });
    /// assert_eq!(sm.parent(), &PlayerGroup::OnGround);
    ///
    /// sm.transition_to(PlayerState::Jumping);
    /// assert_eq!(sm.parent(), &PlayerGroup::InAir);
    /// ```
    pub fn new(initial: S, parent_map: fn(&S) -> P) -> Self {
        let parent_state = parent_map(&initial);
        Self {
            inner: StateMachine::new(initial),
            parent_state,
            previous_parent: None,
            parent_map,
        }
    }

    /// Get the current leaf state.
    pub fn current(&self) -> &S {
        self.inner.current()
    }

    /// Get the current parent/group state.
    pub fn parent(&self) -> &P {
        &self.parent_state
    }

    /// Get the previous leaf state.
    pub fn previous(&self) -> Option<&S> {
        self.inner.previous()
    }

    /// Get the previous parent state.
    pub fn previous_parent(&self) -> Option<&P> {
        self.previous_parent.as_ref()
    }

    /// Transition to a new leaf state, automatically updating the parent.
    pub fn transition_to(&mut self, new_state: S) {
        if *self.inner.current() == new_state {
            return;
        }
        let new_parent = (self.parent_map)(&new_state);
        self.previous_parent = Some(std::mem::replace(&mut self.parent_state, new_parent));
        self.inner.transition_to(new_state);
    }

    /// Returns true if the leaf state just changed.
    pub fn just_entered(&self) -> bool {
        self.inner.just_entered()
    }

    /// Returns true if the parent/group state just changed.
    pub fn parent_just_changed(&self) -> bool {
        self.inner.just_entered()
            && self.previous_parent.as_ref() != Some(&self.parent_state)
    }

    /// How long in the current leaf state.
    pub fn elapsed(&self) -> f32 {
        self.inner.elapsed()
    }

    /// Advance time. Clears `just_entered` flag.
    pub fn tick(&mut self, delta_time: f32) {
        self.inner.tick(delta_time);
    }

    /// Check if the current leaf state matches.
    pub fn is(&self, state: &S) -> bool {
        self.inner.is(state)
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
        Falling,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum PlayerGroup {
        OnGround,
        InAir,
    }

    fn player_group(state: &PlayerState) -> PlayerGroup {
        match state {
            PlayerState::Idle | PlayerState::Running { .. } => PlayerGroup::OnGround,
            PlayerState::Jumping { .. } | PlayerState::Falling => PlayerGroup::InAir,
        }
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

    #[test]
    fn test_hierarchical_transition_reports_parent_change_only_across_groups() {
        let mut machine = HierarchicalStateMachine::new(PlayerState::Idle, player_group);
        assert_eq!(*machine.parent(), PlayerGroup::OnGround, "the parent derives from the state");
        machine.tick(0.016);

        // Idle -> Running stays OnGround.
        machine.transition_to(PlayerState::Running { speed: 200.0 });
        assert!(machine.just_entered());
        assert!(!machine.parent_just_changed());
        assert_eq!(*machine.parent(), PlayerGroup::OnGround);
        machine.tick(0.016);

        // Running -> Jumping crosses into InAir.
        machine.transition_to(PlayerState::Jumping { velocity: 300.0 });
        assert!(machine.parent_just_changed());
        assert_eq!(*machine.parent(), PlayerGroup::InAir);
        assert_eq!(machine.previous_parent(), Some(&PlayerGroup::OnGround));
        machine.tick(0.016);

        // Jumping -> Falling stays InAir.
        machine.transition_to(PlayerState::Falling);
        assert!(!machine.parent_just_changed());
        assert_eq!(*machine.parent(), PlayerGroup::InAir);
    }
}
