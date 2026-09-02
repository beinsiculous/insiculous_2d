//! Engine-level regression: a fast CCD ball fired at a static,
//! restitution-1.0 box must bounce off, not pass through.
//!
//! (Breakout's "ball ploughs through bricks" bug was NOT this — rapier
//! reflects clean face hits fine. That bug came from the game destroying
//! bricks on contact start, which cancels the impulse on corner/gap hits;
//! the fix and its regression test live in the breakout game.)

mod common;

use common::spawn_body;
use ecs::sprite_components::Transform2D;
use ecs::{System, World};
use glam::Vec2;
use physics::{Collider, PhysicsConfig, PhysicsSystem, RigidBody};

const BALL_SPEED: f32 = 360.0;
const DT: f32 = 1.0 / 60.0;

fn spawn_brick(world: &mut World, pos: Vec2) -> ecs::EntityId {
    spawn_body(
        world,
        pos,
        RigidBody::new_static(),
        Collider::box_collider(70.0, 24.0).with_friction(0.0).with_restitution(1.0),
    )
}

fn spawn_ball(world: &mut World, pos: Vec2) -> ecs::EntityId {
    spawn_body(
        world,
        pos,
        RigidBody::new_dynamic()
            .with_gravity_scale(0.0)
            .with_rotation_locked(true)
            .with_linear_damping(0.0)
            .with_angular_damping(0.0)
            .with_ccd(true),
        Collider::circle_collider(8.0).with_friction(0.0).with_restitution(1.0),
    )
}

#[test]
fn ball_bounces_off_static_brick() {
    let mut world = World::new();
    let mut physics = PhysicsSystem::with_config(PhysicsConfig::top_down());

    let _brick = spawn_brick(&mut world, Vec2::new(0.0, 100.0));
    let ball = spawn_ball(&mut world, Vec2::new(0.0, 0.0));
    physics.set_velocity(ball, Vec2::new(0.0, BALL_SPEED), 0.0);

    let mut reflected = false;
    let mut max_y = f32::MIN;
    let mut saw_collision = false;

    for _ in 0..120 {
        physics.update(&mut world, DT);
        if !physics.take_collision_events().is_empty() {
            saw_collision = true;
        }
        if let Some((vel, _)) = physics.get_body_velocity(ball) {
            if vel.y < -1.0 {
                reflected = true;
            }
        }
        if let Some(t) = world.get::<Transform2D>(ball) {
            max_y = max_y.max(t.position.y);
        }
    }

    assert!(saw_collision, "ball never registered a collision with the brick");
    assert!(
        reflected,
        "ball never reflected off the brick (max_y reached: {max_y})"
    );
    // Brick bottom face is at y=88; ball center + radius should never pass
    // far beyond it (a little solver slop is fine).
    assert!(
        max_y < 100.0,
        "ball ploughed through the brick (max_y: {max_y})"
    );
}
