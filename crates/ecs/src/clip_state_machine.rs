//! `ClipStateMachine` — animation state as data: a state names the
//! [`SpriteAnimation`] clip it plays, and the end of that clip decides what
//! happens next.
//!
//! Game code and scripts transition by *name* (`transition_to("opening")`,
//! `cmd.set_clip_state(me, "opening")`) and never poll clip completion by
//! hand — [`ClipStateMachineSystem`] applies `OnFinished` the frame a clip
//! stops and keeps the current state's clip selected. Guards ("only if the
//! player is grounded") stay with the game or the script.

use serde::{Deserialize, Serialize};

use crate::component_registry::ComponentMeta;
use crate::entity::EntityId;
use crate::sprite_components::SpriteAnimation;
use crate::state_machine::StateMachine;
use crate::system::System;
use crate::world::World;
use crate::DeriveComponentMeta;

/// What happens when the clip a state plays ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OnFinished {
    /// Hold the last frame. The state stays until something transitions it.
    #[default]
    Stay,
    /// Move to this state. A clip that ends back into the state it started in
    /// is a no-op — the clip is not restarted.
    Next(String),
    /// Remove the entity: a one-shot effect that plays itself out and goes.
    Despawn,
}

/// One row of a [`ClipStateMachine`]: the clip a state plays and what
/// finishing it does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipState {
    /// Name of the clip this state plays on its [`SpriteAnimation`].
    pub clip: String,
    /// What the end of that clip does.
    pub on_finished: OnFinished,
}

impl ClipState {
    /// A state that plays `clip` and then does `on_finished`.
    pub fn new(clip: impl Into<String>, on_finished: OnFinished) -> Self {
        Self {
            clip: clip.into(),
            on_finished,
        }
    }

    /// The same state, holding its last frame when the clip ends.
    pub fn staying(clip: impl Into<String>) -> Self {
        Self::new(clip, OnFinished::Stay)
    }
}

/// Named-clip animation state machine.
///
/// Rows are `(state, ClipState)` pairs in declaration order — a `Vec` rather
/// than a map so serialization and the inspector stay deterministic (tables
/// are single-digit; lookup is a linear scan). String-keyed on purpose: this
/// is a scene component, so the table is authored data a playground project
/// declares, and a script transitions by name. A Rust game that wants a typed
/// enum wraps the same table with its enum's names.
///
/// The machine owns the entity's clip while it is present: `play_clip` and
/// `ensure_clip` are refused on an entity carrying one (see
/// `engine_core::scripting`), and the system re-selects the state's clip if
/// something else selected another.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct ClipStateMachine {
    /// The state the machine starts in.
    initial: String,
    /// The state table, in declaration order.
    states: Vec<(String, ClipState)>,
    /// Which state is current and for how long. Runtime position: an authored
    /// scene records the table, and loading re-enters `initial`.
    machine: StateMachine<String>,
    /// The state whose clip the system last selected. Runtime, never
    /// authored: it is what tells a state *entry* (which always starts its
    /// clip, even when two states share a clip name) from a re-assert of the
    /// state the machine is already in (which never restarts it).
    #[serde(skip)]
    applied: Option<String>,
}

impl Default for ClipStateMachine {
    fn default() -> Self {
        Self::new(String::new(), Vec::new())
    }
}

impl ClipStateMachine {
    /// Build a machine that starts in `initial`, playing `initial`'s clip.
    pub fn new(
        initial: impl Into<String>,
        states: impl Into<Vec<(String, ClipState)>>,
    ) -> Self {
        let initial = initial.into();
        Self {
            machine: StateMachine::new(initial.clone()),
            initial,
            states: states.into(),
            applied: None,
        }
    }

    /// One state, named after `clip`, that plays it and holds its last frame: a
    /// subject with a single clip (a marching enemy, a flying shot, a life icon).
    pub fn single(clip: &str) -> Self {
        Self::new(clip, vec![(clip.to_string(), ClipState::staying(clip))])
    }

    /// One state that plays `clip` once and despawns the entity: a detached
    /// effect that ends itself.
    pub fn one_shot(clip: &str) -> Self {
        Self::new(clip, vec![(clip.to_string(), ClipState::new(clip, OnFinished::Despawn))])
    }

    /// The state the machine starts in — what an author edits.
    pub fn initial(&self) -> &str {
        &self.initial
    }

    /// The state table, in declaration order.
    pub fn states(&self) -> &[(String, ClipState)] {
        &self.states
    }

    /// The state the machine is in now.
    pub fn state(&self) -> &str {
        self.machine.current()
    }

    /// The underlying state machine, for `elapsed()` and `just_entered()`
    /// guards.
    pub fn machine(&self) -> &StateMachine<String> {
        &self.machine
    }

    /// Whether `state` has a row.
    pub fn has_state(&self, state: &str) -> bool {
        self.states.iter().any(|(name, _)| name == state)
    }

    /// The clip `state` plays, if it has a row.
    pub fn clip_for_state(&self, state: &str) -> Option<&str> {
        self.row_for_state(state).map(|row| row.clip.as_str())
    }

    /// What finishing `state`'s clip does, if it has a row.
    pub fn on_finished_for_state(&self, state: &str) -> Option<&OnFinished> {
        self.row_for_state(state).map(|row| &row.on_finished)
    }

    /// Move to `state`. Moving to the state the machine is already in is a
    /// no-op, so the clip keeps playing; an unknown state is a warned no-op
    /// and the machine stays where it is.
    #[must_use]
    pub fn transition_to(&mut self, state: &str) -> bool {
        if !self.has_state(state) {
            log::warn!(
                "ClipStateMachine::transition_to: no state named '{}' (staying in '{}'); \
                 known states: {:?}",
                state,
                self.state(),
                self.states.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>()
            );
            return false;
        }
        self.machine.transition_to(state.to_string());
        true
    }

    /// Advance the machine's clock by `delta_time` seconds.
    pub fn tick(&mut self, delta_time: f32) {
        self.machine.tick(delta_time);
    }

    fn row_for_state(&self, state: &str) -> Option<&ClipState> {
        self.states
            .iter()
            .find(|(name, _)| name == state)
            .map(|(_, row)| row)
    }
}

/// Move `entity`'s machine to `state`, as game code does it: an entity without a
/// machine is left alone, and an unknown state is the machine's warned no-op.
/// Returns whether the machine knows the state.
pub fn set_clip_state(world: &mut World, entity: EntityId, state: &str) -> bool {
    world
        .get_mut::<ClipStateMachine>(entity)
        .is_some_and(|machine| machine.transition_to(state))
}

/// Applies every [`ClipStateMachine`]: a state entry starts its clip, a
/// finished clip transitions the machine (or despawns the entity) in the same
/// frame, and the current state's clip is kept selected.
///
/// The engine's frame tail runs this right after `SpriteAnimationSystem`, so
/// a game or a script observes a completion — and the state it moved to — by
/// the next frame at the latest, and its own transition is in effect before
/// the system checks this frame.
///
/// Completion is judged only for a clip this system selected for the current
/// state: the frame a state is entered starts its clip and is not judged,
/// because whatever completion the animation still carries belongs to the
/// previous state's clip.
#[derive(Debug, Default)]
pub struct ClipStateMachineSystem;

impl System for ClipStateMachineSystem {
    fn update(&mut self, world: &mut World, delta_time: f32) {
        // One owned list: applying `Despawn` removes from the world mid-loop.
        for entity in world.entities() {
            // Read the entry flag before the tick clears it: it is what lets
            // the "state has no row" warning fire once, on entry.
            let Some((entered, state)) = world.get_mut::<ClipStateMachine>(entity).map(|machine| {
                let entered = machine.machine().just_entered();
                machine.tick(delta_time);
                (entered, machine.state().to_string())
            }) else {
                continue;
            };
            let Some(row) = row_of(world, entity, &state, entered) else {
                continue;
            };

            let applied = world
                .get::<ClipStateMachine>(entity)
                .and_then(|machine| machine.applied.clone());
            if applied.as_deref() != Some(state.as_str()) {
                select_clip(world, entity, &state, &row.clip);
                continue;
            }

            // The machine owns the clip: something else selected another one
            // (or nothing), so the state's clip is put back.
            let showing = world
                .get::<SpriteAnimation>(entity)
                .and_then(|animation| animation.current_clip.clone());
            if showing.as_deref() != Some(row.clip.as_str()) {
                select_clip(world, entity, &state, &row.clip);
                continue;
            }

            let finished = world
                .get::<SpriteAnimation>(entity)
                .is_some_and(SpriteAnimation::is_finished);
            if !finished {
                continue;
            }
            match row.on_finished {
                OnFinished::Stay => {}
                OnFinished::Next(next) => {
                    // An unknown next state is warned inside and the machine
                    // holds its ground rather than dying. `transition_to`
                    // reports "known state", not "moved": a `Next` back into
                    // the state it started in is the documented no-op, and
                    // only an actual change starts a clip.
                    let moved = world
                        .get_mut::<ClipStateMachine>(entity)
                        .is_some_and(|machine| machine.transition_to(&next) && machine.state() != state);
                    let clip = world
                        .get::<ClipStateMachine>(entity)
                        .and_then(|machine| machine.clip_for_state(&next).map(str::to_string));
                    if let (true, Some(clip)) = (moved, clip) {
                        select_clip(world, entity, &next, &clip);
                    }
                }
                OnFinished::Despawn => {
                    world.remove_entity(&entity).ok();
                }
            }
        }
    }

    fn name(&self) -> &str {
        "ClipStateMachineSystem"
    }
}

/// The row of `state`, cloned out of the machine so the component is not
/// borrowed across the writes the system makes.
///
/// A state with no row is a hole in authored data — a table edited under a
/// live state. It is warned once, on the frame the machine entered that
/// state, rather than every frame it stays there.
fn row_of(world: &World, entity: EntityId, state: &str, entered: bool) -> Option<ClipState> {
    let machine = world.get::<ClipStateMachine>(entity)?;
    if let Some(row) = machine.row_for_state(state) {
        return Some(row.clone());
    }
    if entered {
        log::warn!("ClipStateMachine on {entity:?}: state '{state}' has no row, so it plays no clip");
    }
    None
}

/// Start `clip` for `state` on `entity` and show its first frame at once, so
/// the frame that switches state renders the new clip rather than the
/// previous clip's last frame.
fn select_clip(world: &mut World, entity: EntityId, state: &str, clip: &str) {
    if let Some(animation) = world.get_mut::<SpriteAnimation>(entity) {
        // An unknown clip name is warned inside `play`.
        let _ = animation.play(clip);
    }
    if let Some(machine) = world.get_mut::<ClipStateMachine>(entity) {
        machine.applied = Some(state.to_string());
    }
    crate::sprite_system::sync_sprite_region(world, entity);
}

#[cfg(test)]
mod tests;
