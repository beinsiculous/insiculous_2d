//! Public-API contracts of `BehaviorRunner` that games feel directly: the
//! platformer jump reads the mapped action (pad or keyboard), and
//! `FollowEntity` actually closes on its named target. The FSM phases of
//! Patrol / Chase are unit-tested inline in `behavior_runner/mod.rs`.

use ecs::behavior::{Behavior, BehaviorState};
use ecs::sprite_components::Transform2D;
use ecs::World;
use engine_core::behavior_runner::BehaviorRunner;
use engine_core::test_support::frame;
use glam::Vec2;
use input::{GamepadButton, InputEvent, InputHandler};
use winit::keyboard::KeyCode;

const DT: f32 = 1.0 / 60.0;

#[test]
fn test_follow_entity_moves_toward_its_named_target_and_stops_at_follow_distance() {
    let mut world = World::new();
    let mut runner = BehaviorRunner::new();
    let input = InputHandler::new();
    let player = world.create_entity();
    world.add_component(&player, Transform2D::new(Vec2::ZERO)).ok();
    let follower = world.create_entity();
    world.add_component(&follower, Transform2D::new(Vec2::new(100.0, 0.0))).ok();
    world
        .add_component(
            &follower,
            Behavior::FollowEntity { target_name: "player".to_string(), follow_distance: 50.0, follow_speed: 100.0 },
        )
        .ok();
    runner.set_named_entities([("player".to_string(), player)].into_iter().collect());

    runner.update(&mut world, &input, DT, None);
    let after_one_frame = world.get::<Transform2D>(follower).expect("follower").position.x;
    assert!(after_one_frame < 100.0, "the follower moved toward the player, x = {after_one_frame}");

    for _ in 0..120 {
        runner.update(&mut world, &input, DT, None);
    }
    let settled = world.get::<Transform2D>(follower).expect("follower").position.x;
    assert!((settled - 50.0).abs() < 2.0, "the follower stops at follow_distance, x = {settled}");
}

#[test]
fn test_platformer_jump_fires_from_gamepad_action_and_from_space() {
    // Jump reads GameAction::Action1 (not a raw Space key), so pad-0 A must
    // trigger it. Observable headlessly: the jump cooldown timer arms.
    for event in [
        InputEvent::GamepadButtonPressed(0, GamepadButton::A),
        InputEvent::KeyPressed(KeyCode::Space),
    ] {
        let mut world = World::new();
        let mut runner = BehaviorRunner::new();
        let mut input = InputHandler::new();
        let player = world.create_entity();
        world.add_component(&player, Transform2D::new(Vec2::ZERO)).ok();
        world
            .add_component(
                &player,
                Behavior::PlayerPlatformer {
                    move_speed: 120.0,
                    jump_impulse: 420.0,
                    jump_cooldown: 0.3,
                    tag: "player".to_string(),
                },
            )
            .ok();

        frame(&mut input, std::slice::from_ref(&event));
        runner.update(&mut world, &input, DT, None);

        let state = world.get::<BehaviorState>(player).expect("platformer keeps state");
        assert!(state.timer > 0.0, "jump cooldown should arm after {event:?} — jump did not fire");
    }
}
