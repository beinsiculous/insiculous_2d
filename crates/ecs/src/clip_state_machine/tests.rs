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
