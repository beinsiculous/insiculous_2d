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
mod tests {
    use super::*;
    use crate::sprite_components::{AnimationClip, SheetGrid};
    use glam::Vec2;

    /// A 4-cell sheet with a three-frame one-shot (`close`) and a looping
    /// clip (`hum`).
    fn animated() -> SpriteAnimation {
        SpriteAnimation::new(SheetGrid::new(4, 1))
            .with_clip("close", AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
            .with_clip("open", AnimationClip::new(vec![3, 2, 1], 10.0).with_looping(false))
            .with_clip("hum", AnimationClip::new(vec![0, 1], 10.0))
    }

    fn row(state: &str, clip: &str, on_finished: OnFinished) -> (String, ClipState) {
        (state.to_string(), ClipState::new(clip, on_finished))
    }

    /// An entity whose machine starts in `initial`.
    fn machine_entity(
        world: &mut World,
        initial: &str,
        states: Vec<(String, ClipState)>,
    ) -> Result<EntityId, crate::EcsError> {
        let entity = world.create_entity();
        world.add_component(&entity, animated())?;
        world.add_component(&entity, ClipStateMachine::new(initial, states))?;
        Ok(entity)
    }

    fn step(world: &mut World, frames: usize, delta_time: f32) {
        let mut system = ClipStateMachineSystem;
        for _ in 0..frames {
            crate::System::update(&mut crate::SpriteAnimationSystem, world, delta_time);
            system.update(world, delta_time);
        }
    }

    fn state_of(world: &World, entity: EntityId) -> String {
        world
            .get::<ClipStateMachine>(entity)
            .expect("machine")
            .state()
            .to_string()
    }

    fn frame_of(world: &World, entity: EntityId) -> usize {
        world
            .get::<SpriteAnimation>(entity)
            .expect("animation")
            .current_frame
    }

    fn clip_of(world: &World, entity: EntityId) -> Option<String> {
        world
            .get::<SpriteAnimation>(entity)
            .expect("animation")
            .current_clip
            .clone()
    }

    #[test]
    fn test_next_chains_three_states_as_each_oneshot_clip_ends() -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "closing",
            vec![
                row("closing", "close", OnFinished::Next("opening".to_string())),
                row("opening", "open", OnFinished::Next("wide".to_string())),
                row("wide", "hum", OnFinished::Stay),
            ],
        )?;

        // The first step selects the state's clip.
        step(&mut world, 1, 0.1);
        assert_eq!(clip_of(&world, entity), Some("close".to_string()));
        assert_eq!(state_of(&world, entity), "closing");
        assert_eq!(frame_of(&world, entity), 0);

        // Three 10fps frames in: the frame `close` stops on moves the state,
        // and the new state's clip starts from the top in the same frame.
        step(&mut world, 2, 0.1);
        assert_eq!(state_of(&world, entity), "closing");
        step(&mut world, 1, 0.1);
        assert_eq!(state_of(&world, entity), "opening");
        assert_eq!(clip_of(&world, entity), Some("open".to_string()));
        assert_eq!(frame_of(&world, entity), 0, "a transition restarts the new clip");

        step(&mut world, 3, 0.1);
        assert_eq!(state_of(&world, entity), "wide");
        assert_eq!(clip_of(&world, entity), Some("hum".to_string()));

        // The last state holds: a looping clip never finishes.
        step(&mut world, 30, 0.1);
        assert_eq!(state_of(&world, entity), "wide");
        assert!(world.get::<SpriteAnimation>(entity).expect("animation").playing);
        Ok(())
    }

    #[test]
    fn test_despawn_removes_the_entity_when_a_oneshot_ends_and_never_for_a_looping_clip(
    ) -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let one_shot = machine_entity(
            &mut world,
            "burst",
            vec![row("burst", "close", OnFinished::Despawn)],
        )?;
        let looping = machine_entity(
            &mut world,
            "idle",
            vec![row("idle", "hum", OnFinished::Despawn)],
        )?;

        step(&mut world, 4, 0.1);
        assert!(!world.entities().contains(&one_shot), "the one-shot despawned the entity");
        step(&mut world, 30, 0.1);
        assert!(world.entities().contains(&looping), "a looping clip never finishes");
        Ok(())
    }

    #[test]
    fn test_re_asserting_the_current_state_neither_restarts_the_clip_nor_moves_the_entity(
    ) -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "closing",
            vec![row("closing", "close", OnFinished::Stay)],
        )?;
        world.add_component(&entity, crate::Transform2D::new(Vec2::new(3.0, 4.0)))?;

        step(&mut world, 1, 0.1);
        step(&mut world, 1, 0.1);
        assert_eq!(frame_of(&world, entity), 1);

        // A transition to the state it is already in is a no-op: the frame
        // counter keeps its position instead of restarting.
        for _ in 0..5 {
            let _ = world
                .get_mut::<ClipStateMachine>(entity)
                .expect("machine")
                .transition_to("closing");
            step(&mut world, 1, 0.1);
        }
        assert_eq!(state_of(&world, entity), "closing");
        assert_eq!(frame_of(&world, entity), 2, "the clip ran on, it did not restart");
        assert_eq!(
            world.get::<crate::Transform2D>(entity).expect("transform").position,
            Vec2::new(3.0, 4.0)
        );
        Ok(())
    }

    #[test]
    fn test_an_explicit_transition_out_of_a_finished_state_plays_the_new_clip_before_judging_it(
    ) -> Result<(), crate::EcsError> {
        // The completion still on the animation belongs to the state just
        // left; the entered state's Despawn must wait for its own clip.
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "closing",
            vec![
                row("closing", "close", OnFinished::Stay),
                row("gone", "open", OnFinished::Despawn),
            ],
        )?;
        step(&mut world, 4, 0.1);
        assert!(world.get::<SpriteAnimation>(entity).expect("animation").is_finished());

        let _ = world
            .get_mut::<ClipStateMachine>(entity)
            .expect("machine")
            .transition_to("gone");
        step(&mut world, 1, 0.1);
        assert!(world.entities().contains(&entity), "the entry frame starts the clip, it does not judge it");
        assert_eq!(clip_of(&world, entity), Some("open".to_string()));
        assert_eq!(frame_of(&world, entity), 0);

        step(&mut world, 2, 0.1);
        assert!(world.entities().contains(&entity), "the last frame is still showing");
        step(&mut world, 1, 0.1);
        assert!(!world.entities().contains(&entity), "despawned once its own clip ran out");
        Ok(())
    }

    #[test]
    fn test_two_states_sharing_a_clip_each_play_it_through() -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "first_hit",
            vec![
                row("first_hit", "close", OnFinished::Next("second_hit".to_string())),
                row("second_hit", "close", OnFinished::Next("done".to_string())),
                row("done", "hum", OnFinished::Stay),
            ],
        )?;

        step(&mut world, 4, 0.1);
        assert_eq!(state_of(&world, entity), "second_hit");
        assert_eq!(frame_of(&world, entity), 0, "the shared clip starts over for the new state");
        step(&mut world, 2, 0.1);
        assert_eq!(state_of(&world, entity), "second_hit", "its own three frames are still playing");
        step(&mut world, 1, 0.1);
        assert_eq!(state_of(&world, entity), "done");
        Ok(())
    }

    #[test]
    fn test_selecting_a_clip_shows_its_first_frame_in_the_same_step() -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "closing",
            vec![
                row("closing", "close", OnFinished::Next("opening".to_string())),
                row("opening", "open", OnFinished::Stay),
            ],
        )?;
        world.add_component(&entity, crate::sprite_components::Sprite::new(0))?;
        let region = |world: &World| world.get::<crate::sprite_components::Sprite>(entity).expect("sprite").tex_region;
        let uv_of = |world: &World, clip: &str, frame: usize| {
            let mut probe = world.get::<SpriteAnimation>(entity).expect("animation").clone();
            assert!(probe.play(clip));
            probe.current_frame = frame;
            probe.current_uv().expect("resolves")
        };

        step(&mut world, 1, 0.1);
        assert_eq!(region(&world), uv_of(&world, "close", 0), "the initial selection is visible at once");

        step(&mut world, 3, 0.1);
        assert_eq!(state_of(&world, entity), "opening");
        assert_eq!(region(&world), uv_of(&world, "open", 0), "the step that switches state shows the new clip");
        Ok(())
    }

    #[test]
    fn test_a_clip_paused_on_its_last_frame_does_not_end_the_state() -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "burst",
            vec![row("burst", "close", OnFinished::Despawn)],
        )?;
        step(&mut world, 3, 0.1);
        assert_eq!(frame_of(&world, entity), 2, "the last frame is showing");
        world.get_mut::<SpriteAnimation>(entity).expect("animation").pause();

        step(&mut world, 10, 0.1);
        assert!(world.entities().contains(&entity), "a freeze-frame is not a completion");
        Ok(())
    }

    #[test]
    fn test_a_next_into_the_same_state_holds_the_last_frame() -> Result<(), crate::EcsError> {
        // The documented no-op: a clip that ends back into its own state is
        // not restarted, so "play once and hold" can be written as a self
        // `Next` as well as a `Stay`.
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "dying",
            vec![row("dying", "close", OnFinished::Next("dying".to_string()))],
        )?;

        step(&mut world, 20, 0.1);
        assert_eq!(state_of(&world, entity), "dying");
        assert_eq!(frame_of(&world, entity), 2, "held on the last frame");
        assert!(world.get::<SpriteAnimation>(entity).expect("animation").is_finished());
        Ok(())
    }

    #[test]
    fn test_transition_to_refuses_an_unknown_state_and_warns() -> Result<(), crate::EcsError> {
        let mut world = World::new();
        let entity = machine_entity(
            &mut world,
            "closing",
            vec![row("closing", "close", OnFinished::Stay)],
        )?;

        let moved = world
            .get_mut::<ClipStateMachine>(entity)
            .expect("machine")
            .transition_to("sideways");

        assert!(!moved);
        assert_eq!(state_of(&world, entity), "closing");
        Ok(())
    }
}
