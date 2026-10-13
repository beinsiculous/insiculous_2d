//! The clip machine's shorthand constructors and the game-side transition, driven
//! through a world the way the engine's frame tail runs them.

use ecs::sprite_components::*;
use ecs::{set_clip_state, ClipStateMachine, ClipStateMachineSystem, SpriteAnimationSystem, System, World};

/// A 4-cell sheet with a looping `march` and a two-frame one-shot `burst`.
fn animation() -> SpriteAnimation {
    SpriteAnimation::new(SheetGrid::new(4, 1))
        .with_clip("march", AnimationClip::new(vec![0, 1], 10.0))
        .with_clip("burst", AnimationClip::new(vec![2, 3], 10.0).with_looping(false))
}

fn spawn(world: &mut World, machine: ClipStateMachine) -> ecs::EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, Sprite::new(0)).ok();
    world.add_component(&entity, animation()).ok();
    world.add_component(&entity, machine).ok();
    entity
}

/// Step the animation then the machine, in the frame tail's order.
fn step(world: &mut World, frames: usize) {
    let mut animations = SpriteAnimationSystem;
    let mut machines = ClipStateMachineSystem;
    for _ in 0..frames {
        animations.update(world, 0.1);
        machines.update(world, 0.1);
    }
}

#[test]
fn test_a_single_state_machine_plays_its_one_clip_and_keeps_the_entity() {
    let mut world = World::new();
    let entity = spawn(&mut world, ClipStateMachine::single("march"));
    step(&mut world, 10);
    assert!(world.entities().contains(&entity));
    let machine = world.get::<ClipStateMachine>(entity).expect("the machine stays");
    assert_eq!(machine.state(), "march");
    assert_eq!(machine.states().len(), 1);
    assert_eq!(world.get::<SpriteAnimation>(entity).and_then(|a| a.current_clip.clone()).as_deref(), Some("march"));
}

#[test]
fn test_a_one_shot_machine_plays_its_clip_once_then_despawns_the_entity() {
    let mut world = World::new();
    let entity = spawn(&mut world, ClipStateMachine::one_shot("burst"));
    step(&mut world, 1);
    assert!(world.entities().contains(&entity), "the clip is still playing");
    step(&mut world, 5);
    assert!(!world.entities().contains(&entity), "a finished one-shot is gone");
}

#[test]
fn test_set_clip_state_moves_a_machine_and_leaves_everything_else_alone() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, ClipStateMachine::new("march", vec![
        ("march".to_string(), ecs::ClipState::staying("march")),
        ("burst".to_string(), ecs::ClipState::staying("burst")),
    ])).ok();
    assert!(set_clip_state(&mut world, entity, "burst"));
    assert_eq!(world.get::<ClipStateMachine>(entity).map(|m| m.state().to_string()).as_deref(), Some("burst"));

    assert!(!set_clip_state(&mut world, entity, "no such state"), "an unknown state is refused");
    assert_eq!(world.get::<ClipStateMachine>(entity).map(|m| m.state().to_string()).as_deref(), Some("burst"));

    let bare = world.create_entity();
    assert!(!set_clip_state(&mut world, bare, "march"), "an entity without a machine is left alone");
}
