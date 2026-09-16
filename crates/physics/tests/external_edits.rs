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

#[test]
fn test_missing_valid_colliders_recover_but_refused_shapes_stay_absent() {
    for with_body in [false, true] {
        let mut world = World::new();
        let mut system = no_gravity_system();
        let entity = world.create_entity();
        world.add_component(&entity, Transform2D::new(Vec2::ZERO)).expect("transform");
        if with_body {
            world.add_component(&entity, RigidBody::new_kinematic()).expect("body");
        }
        let valid_shape = ColliderShape::circle(10.0);
        world.add_component(&entity, Collider::new(valid_shape.clone())).expect("collider");
        system.update(&mut world, DT);
        assert!(system.physics_world().has_collider(entity), "initial shape builds");

        system.physics_world_mut().remove_collider(entity);
        assert!(!system.physics_world().has_collider(entity), "low-level removal took effect");
        system.update(&mut world, DT);
        assert!(system.physics_world().has_collider(entity), "unchanged valid shape is restored");

        world.get_mut::<Collider>(entity).expect("collider").shape =
            ColliderShape::Compound(vec![ColliderShape::Compound(vec![])]);
        system.update(&mut world, DT);
        assert!(!system.physics_world().has_collider(entity), "an empty edit removes the old shape");
        system.update(&mut world, DT);
        assert!(!system.physics_world().has_collider(entity), "a refused shape stays absent");
        assert_eq!(system.external_edits_pushed_last_update(), 0, "refusal is not retried");

        world.get_mut::<Collider>(entity).expect("collider").shape = valid_shape;
        system.update(&mut world, DT);
        assert!(system.physics_world().has_collider(entity), "a valid edit clears the refusal");
    }
}

/// The two-armed jaw: a compound of capsules from a shared hinge to two tips.
fn v_jaw() -> ColliderShape {
    ColliderShape::compound(vec![
        ColliderShape::capsule(Vec2::ZERO, Vec2::new(-30.0, 40.0), 6.0),
        ColliderShape::capsule(Vec2::ZERO, Vec2::new(30.0, 40.0), 6.0),
    ])
}

/// The flat shape a closed tong carries.
fn flat_jaw() -> ColliderShape {
    ColliderShape::capsule_y(78.0, 11.0)
}

fn touching(system: &mut PhysicsSystem, tong: ecs::EntityId, probe: ecs::EntityId) -> bool {
    system
        .take_collision_events()
        .iter()
        .any(|collision| collision.event.started && collision.event.involves(tong, probe))
}

#[test]
fn test_kinematic_shape_swap_between_a_compound_and_a_capsule_rebuilds_each_time() {
    let mut world = World::new();
    let mut system = no_gravity_system();
    // Sensors, so the swap can be observed over several updates without the
    // jaw pushing the probe out of reach of the next shape.
    let tong = spawn_body(
        &mut world,
        Vec2::ZERO,
        RigidBody::new_kinematic(),
        Collider::new(v_jaw()).as_sensor(),
    );
    // On the V's left arm, off the flat capsule's axis.
    let probe_position = Vec2::new(-30.0, 40.0);
    let probe = spawn_body(
        &mut world,
        probe_position,
        floating_body(),
        Collider::circle_collider(8.0).as_sensor(),
    );

    system.update(&mut world, DT);
    assert_eq!(system.external_edits_pushed_last_update(), 0, "the first update only adds");
    assert!(touching(&mut system, tong, probe), "sanity: the probe sits on a drawn arm");

    world.get_mut::<Collider>(tong).expect("collider").shape = flat_jaw();
    system.update(&mut world, DT);
    assert_eq!(system.external_edits_pushed_last_update(), 1, "the shape swap rebuilds the collider");
    assert!(!touching(&mut system, tong, probe), "the flat shape does not reach the arm's tip");

    world.get_mut::<Collider>(tong).expect("collider").shape = v_jaw();
    system.update(&mut world, DT);
    assert_eq!(system.external_edits_pushed_last_update(), 1, "and the swap back rebuilds it again");
    assert!(touching(&mut system, tong, probe), "the arms are back where rapier sees them");
    assert!(
        (position_of(&world, probe) - probe_position).length() < 0.01,
        "a sensor never moves the probe, so every check above is about the tong (it sits at {:?})",
        position_of(&world, probe)
    );
}
