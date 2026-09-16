//! What a compound of angled capsules does to a ball: an arm bounces it
//! off-axis, a flat capsule does not.
//!
//! The open jaw of a tong is a compound of two capsules from a shared hinge
//! to two tips, and the shape exists for this behaviour — a ball met by an
//! angled arm leaves with a vertical component the flat closed capsule
//! cannot give it. Stepped through `PhysicsSystem`, as the games step it.

mod common;

use common::spawn_body;
use ecs::sprite_components::Transform2D;
use ecs::{System, World};
use glam::Vec2;
use physics::{Collider, ColliderShape, PhysicsConfig, PhysicsSystem, RigidBody};

const DT: f32 = 1.0 / 60.0;
const BALL_SPEED: f32 = 240.0;
/// Enough frames for the ball to cross the gap, bounce, and be read while
/// nothing is touching it any more.
const FRAMES_TO_SETTLE: usize = 60;

/// The open jaw: two arms from a shared hinge to two tips, 30 out and 40 up.
fn v_jaw() -> ColliderShape {
    ColliderShape::compound(vec![
        ColliderShape::capsule(Vec2::ZERO, Vec2::new(-30.0, 40.0), 6.0),
        ColliderShape::capsule(Vec2::ZERO, Vec2::new(30.0, 40.0), 6.0),
    ])
}

/// A kinematic jaw that stays where it was spawned.
fn spawn_jaw(world: &mut World, shape: ColliderShape) -> ecs::EntityId {
    spawn_body(
        world,
        Vec2::ZERO,
        RigidBody::new_kinematic(),
        Collider::new(shape).with_friction(0.0).with_restitution(1.0),
    )
}

/// A ball fired at `target` from `start_offset` away, travelling toward it.
fn fire_ball_at(world: &mut World, system: &mut PhysicsSystem, target: Vec2, start_offset: Vec2) -> ecs::EntityId {
    let ball = spawn_body(
        world,
        target + start_offset,
        RigidBody::new_dynamic()
            .with_gravity_scale(0.0)
            .with_rotation_locked(true)
            .with_linear_damping(0.0)
            .with_angular_damping(0.0)
            .with_ccd(true),
        Collider::circle_collider(8.0).with_friction(0.0).with_restitution(1.0),
    );
    system.set_velocity(ball, -start_offset.normalize() * BALL_SPEED, 0.0);
    ball
}

fn velocity_after_settling(system: &mut PhysicsSystem, world: &mut World, ball: ecs::EntityId) -> Vec2 {
    for _ in 0..FRAMES_TO_SETTLE {
        system.update(world, DT);
    }
    system.get_body_velocity(ball).expect("the ball's body exists").0
}

#[test]
fn test_ball_fired_at_a_v_jaws_outer_arm_leaves_with_the_arms_downward_deflection() {
    let mut world = World::new();
    let mut system = PhysicsSystem::with_config(PhysicsConfig::new(Vec2::ZERO));
    spawn_jaw(&mut world, v_jaw());

    // The middle of the right arm's outward face: half way along the axis
    // (15, 20), pushed 6px along the outward normal (0.8, -0.6). A cap hit
    // would be a round surface with no single normal to reason about; the
    // flat middle is what the tong's ball meets.
    let target = Vec2::new(15.0, 20.0) + Vec2::new(0.8, -0.6) * 6.0;
    let ball = fire_ball_at(&mut world, &mut system, target, Vec2::new(40.0, 0.0));

    let velocity = velocity_after_settling(&mut system, &mut world, ball);

    assert!(
        velocity.x > BALL_SPEED * 0.1,
        "the ball bounces back off the angled arm (velocity {velocity:?})"
    );
    assert!(
        velocity.y < -BALL_SPEED * 0.5,
        "and leaves downward, off the outward face's own normal (velocity {velocity:?}) — \
         the vertical component an angled arm gives a horizontally fired ball"
    );
}

#[test]
fn test_ball_fired_at_a_flat_capsule_leaves_horizontally() {
    let mut world = World::new();
    let mut system = PhysicsSystem::with_config(PhysicsConfig::new(Vec2::ZERO));
    // The shape a closed tong carries: no angle anywhere on its side.
    let flat = ColliderShape::capsule_y(78.0, 11.0);
    spawn_jaw(&mut world, flat);

    // The middle of the flat capsule's right side: its axis runs (0, -28) to
    // (0, 28), so the face sits 11px out from there.
    let ball = fire_ball_at(&mut world, &mut system, Vec2::new(11.0, 0.0), Vec2::new(40.0, 0.0));

    let velocity = velocity_after_settling(&mut system, &mut world, ball);

    assert!(
        velocity.x > BALL_SPEED * 0.9,
        "a head-on hit comes straight back at the speed it arrived (velocity {velocity:?})"
    );
    assert!(
        velocity.y.abs() < BALL_SPEED * 0.05,
        "and with no vertical component at all: the flat face has no angle to give it one \
         (velocity {velocity:?})"
    );
}

#[test]
fn test_ball_fired_at_a_v_jaws_outer_arm_is_at_the_hinge_s_side_before_the_tips() {
    // Guards the geometry the assertions above rest on: whatever the jaw
    // does, the arm is a solid the ball reaches — a jaw the ball passed
    // through would satisfy a "no downward velocity" check by never touching
    // anything.
    let mut world = World::new();
    let mut system = PhysicsSystem::with_config(PhysicsConfig::new(Vec2::ZERO));
    let jaw = spawn_jaw(&mut world, v_jaw());

    let target = Vec2::new(15.0, 20.0) + Vec2::new(0.8, -0.6) * 6.0;
    let ball = fire_ball_at(&mut world, &mut system, target, Vec2::new(40.0, 0.0));

    let mut touched = false;
    for _ in 0..FRAMES_TO_SETTLE {
        system.update(&mut world, DT);
        touched |= system
            .take_collision_events()
            .iter()
            .any(|collision| collision.event.started && collision.event.involves(jaw, ball));
    }
    assert!(touched, "the ball meets the arm rather than tunnelling through it");
    let ball_position = world.get::<Transform2D>(ball).expect("transform exists").position;
    assert!(
        ball_position.x > 0.0,
        "and it met the right arm, on the far side of the hinge (ball at {ball_position:?})"
    );
}
