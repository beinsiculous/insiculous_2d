//! External ECS-side edit detection, through the public API:
//! live `Transform2D` edits teleport rapier bodies, live `Collider` edits
//! rebuild rapier colliders, and the physics writeback is never mistaken
//! for an external edit.

mod common;

use common::spawn_body;
use ecs::sprite_components::Transform2D;
use ecs::{System, World};
use glam::Vec2;
use physics::{Collider, ColliderShape, PhysicsConfig, PhysicsSystem, RigidBody};

const DT: f32 = 1.0 / 60.0;

fn no_gravity_system() -> PhysicsSystem {
    PhysicsSystem::with_config(PhysicsConfig::new(Vec2::ZERO))
}

fn floating_body() -> RigidBody {
    RigidBody::new_dynamic().with_gravity_scale(0.0)
}

fn position_of(world: &World, entity: ecs::EntityId) -> Vec2 {
    world.get::<Transform2D>(entity).expect("transform exists").position
}

#[test]
fn test_external_transform_edit_teleports_live_body_and_keeps_its_velocity() {
    let mut world = World::new();
    let mut system = no_gravity_system();
    let entity = spawn_body(&mut world, Vec2::ZERO, floating_body(), Collider::box_collider(16.0, 16.0));
    system.update(&mut world, DT); // synced into rapier
    system.set_velocity(entity, Vec2::new(50.0, 0.0), 0.0);
    system.update(&mut world, DT);

    // Game/editor code teleports the entity by writing Transform2D directly
    // (once a silent no-op: the "sync only ADDS" footgun).
    world.get_mut::<Transform2D>(entity).expect("transform").position = Vec2::new(500.0, 300.0);
    system.update(&mut world, DT);

    assert_eq!(system.external_edits_pushed_last_update(), 1);
    let position = position_of(&world, entity);
    assert!(
        (position - Vec2::new(500.0, 300.0)).length() < 5.0,
        "the body lives at the teleport target (got {position:?}; the writeback would have snapped it back)"
    );
    let (velocity, _) = system.get_body_velocity(entity).expect("body exists");
    assert!((velocity.x - 50.0).abs() < 1.0, "a teleport preserves the body's velocity (got {velocity:?})");
}

#[test]
fn test_physics_writeback_and_identical_writes_are_not_external_edits() {
    let mut world = World::new();
    let mut system = PhysicsSystem::new(); // default gravity: the body falls
    let entity = spawn_body(&mut world, Vec2::new(0.0, 100.0), RigidBody::new_dynamic(), Collider::box_collider(16.0, 16.0));
    system.update(&mut world, DT); // creation frame

    let mut last_y = 100.0;
    for _ in 0..10 {
        system.update(&mut world, DT);
        assert_eq!(
            system.external_edits_pushed_last_update(),
            0,
            "rapier-driven motion written back into the ECS must not read as an external edit"
        );
        let y = position_of(&world, entity).y;
        assert!(y < last_y, "sanity: the body is falling");
        last_y = y;
    }

    // Writing back the values the transform already holds (the sleeping-body
    // writeback pattern) is a value comparison, not a push.
    let current = position_of(&world, entity);
    world.get_mut::<Transform2D>(entity).expect("transform").position = current;
    system.update(&mut world, DT);
    assert_eq!(system.external_edits_pushed_last_update(), 0);
}

#[test]
fn test_collider_edit_rebuilds_and_collider_removal_drops_the_rapier_collider() {
    let mut world = World::new();
    let mut system = no_gravity_system();
    // Two bodies 100px apart with small colliders: no contact.
    let a = spawn_body(&mut world, Vec2::ZERO, floating_body(), Collider::box_collider(20.0, 20.0));
    let b = spawn_body(&mut world, Vec2::new(100.0, 0.0), floating_body(), Collider::box_collider(20.0, 20.0));
    system.update(&mut world, DT);
    assert!(system.take_collision_events().is_empty(), "sanity: small colliders do not touch");

    // Editor-style live edit: grow A's collider until the two overlap
    // (once a silent no-op: the editor collider-edit footgun).
    world.get_mut::<Collider>(a).expect("collider").shape = ColliderShape::box_shape(240.0, 40.0);
    system.update(&mut world, DT);
    assert_eq!(system.external_edits_pushed_last_update(), 1, "the collider edit is detected and pushed");
    assert!(
        system.take_collision_events().iter().any(|c| c.event.started && c.event.involves(a, b)),
        "the rebuilt (larger) collider actually collides in rapier"
    );

    world.remove_component::<Collider>(&a).expect("collider present");
    system.update(&mut world, DT);
    assert!(!system.physics_world().has_collider(a), "removing the Collider component removes the rapier collider");
    assert_eq!(system.external_edits_pushed_last_update(), 1);
    assert!(system.physics_world().has_rigid_body(a), "the body itself stays");
}
