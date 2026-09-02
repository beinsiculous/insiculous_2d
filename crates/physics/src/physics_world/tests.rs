//! Tests for the PhysicsWorld wrapper: body lifecycle, the collision event
//! state machine, sensors, unit conversion and raycasts.

use glam::Vec2;

use ecs::EntityId;

use crate::components::{Collider, RigidBody};

use super::{PhysicsConfig, PhysicsWorld, DEFAULT_PIXELS_PER_METER};

const DT: f32 = 1.0 / 60.0;

/// Add a body with a collider at `position` and return its fresh entity id.
fn add_body(world: &mut PhysicsWorld, position: Vec2, mut body: RigidBody, mut collider: Collider) -> EntityId {
    let entity = EntityId::new();
    world.add_rigid_body(entity, &mut body, position, 0.0);
    world.add_collider(entity, &mut collider, Some(&body));
    entity
}

/// A dynamic body that gravity leaves alone.
fn floating_body() -> RigidBody {
    RigidBody::new_dynamic().with_gravity_scale(0.0)
}

fn no_gravity_world() -> PhysicsWorld {
    PhysicsWorld::new(PhysicsConfig::new(Vec2::ZERO))
}

// === Body lifecycle ===

#[test]
fn test_body_and_collider_lifecycle_add_then_remove() {
    let mut world = PhysicsWorld::default();
    let entity = EntityId::new();
    let mut body = RigidBody::new_dynamic();
    let mut collider = Collider::box_collider(32.0, 32.0);

    world.add_rigid_body(entity, &mut body, Vec2::ZERO, 0.0);
    world.add_collider(entity, &mut collider, Some(&body));
    assert!(world.has_rigid_body(entity) && world.has_collider(entity));
    assert_eq!((world.rigid_body_count(), world.collider_count()), (1, 1));
    assert!(body.handle.is_some() && collider.handle.is_some(), "the components learn their rapier handles");

    world.remove_entity(entity);
    assert!(!world.has_rigid_body(entity) && !world.has_collider(entity));
    assert_eq!((world.rigid_body_count(), world.collider_count()), (0, 0));
}

// === Unit conversion ===

#[test]
fn test_invalid_pixels_per_meter_falls_back_to_the_default_and_bodies_still_fall() {
    // A zero scale would divide by zero and NaN every position.
    let built = PhysicsConfig::default().with_scale(0.0);
    assert_eq!(built.pixels_per_meter, DEFAULT_PIXELS_PER_METER, "with_scale rejects zero");
    let literal = PhysicsConfig { pixels_per_meter: f32::NAN, ..PhysicsConfig::default() };
    let world = PhysicsWorld::new(literal);
    assert_eq!(world.config().pixels_per_meter, DEFAULT_PIXELS_PER_METER, "a struct literal is sanitized at creation");

    let mut world = PhysicsWorld::new(built);
    let entity = add_body(&mut world, Vec2::new(0.0, 100.0), RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));
    world.step(DT);

    // ½·g·dt² in pixels (rapier's solver substeps add a little under a
    // hundredth of a pixel to that). A world that lost the px↔m round trip
    // would fall a hundred times further.
    let expected_fall = 0.5 * 980.0 * DT * DT;
    let (position, _) = world.get_body_transform(entity).expect("body exists");
    let fallen = 100.0 - position.y;
    assert!(
        (fallen - expected_fall).abs() < 0.02,
        "one 1/60 step at 980 px/s² falls about {expected_fall} px, got {fallen}"
    );
}

// === Collision event state machine ===

#[test]
fn test_collision_events_report_started_then_ongoing_then_stopped() {
    let mut world = no_gravity_world();
    let floor = add_body(&mut world, Vec2::ZERO, RigidBody::new_static(), Collider::box_collider(50.0, 50.0));
    let visitor = add_body(&mut world, Vec2::new(10.0, 0.0), floating_body(), Collider::box_collider(50.0, 50.0));

    // Drive three frames the way the fixed-timestep driver does: the buffer
    // is cleared once per frame, then step() appends.
    let mut phases = Vec::new();
    for frame in 0..3 {
        if frame == 2 {
            world.set_body_transform(visitor, Vec2::new(500.0, 0.0), 0.0);
        }
        world.clear_collision_events();
        world.step(DT);
        let event = world
            .collision_events()
            .iter()
            .find(|c| c.event.involves(floor, visitor))
            .unwrap_or_else(|| panic!("frame {frame} should report the pair"));
        phases.push((event.event.started, event.event.stopped));
    }

    assert_eq!(
        phases,
        vec![(true, false), (false, false), (false, true)],
        "(started, stopped) per frame: start, ongoing, stopped after the visitor leaves"
    );
}

#[test]
fn test_sensor_collider_fires_intersection_events_without_contacts() {
    let mut world = no_gravity_world();
    let sensor = add_body(&mut world, Vec2::ZERO, RigidBody::new_static(), Collider::box_collider(100.0, 100.0).as_sensor());
    let visitor = add_body(&mut world, Vec2::new(10.0, 0.0), floating_body(), Collider::box_collider(20.0, 20.0));

    world.step(DT);

    let event = world
        .collision_events()
        .iter()
        .find(|c| c.event.involves(sensor, visitor))
        .expect("a sensor intersection produces a collision event");
    assert!(event.event.started, "the overlap is reported as started");
    assert!(event.contacts.is_empty(), "sensors report no contact points");
    let (position, _) = world.get_body_transform(visitor).expect("visitor exists");
    assert_eq!(position, Vec2::new(10.0, 0.0), "a sensor never pushes the visitor");
}

#[test]
fn test_contact_points_are_in_world_space() {
    // Two overlapping boxes far from the origin. If contact points were
    // reported in collider-local space (the old bug), they would land
    // within ~25px of the origin instead of near the overlap region.
    let mut world = no_gravity_world();
    let anchor = add_body(&mut world, Vec2::new(1000.0, 1000.0), RigidBody::new_static(), Collider::box_collider(50.0, 50.0));
    let visitor = add_body(&mut world, Vec2::new(1010.0, 1000.0), floating_body(), Collider::box_collider(50.0, 50.0));

    world.step(DT);

    let collision = world
        .collision_events()
        .iter()
        .find(|c| c.event.involves(anchor, visitor) && !c.contacts.is_empty())
        .expect("a collision with contact points");
    for contact in &collision.contacts {
        let distance = (contact.point - Vec2::new(1005.0, 1000.0)).length();
        assert!(
            distance < 60.0,
            "contact point {:?} should be near the overlap around (1005, 1000), not collider-local (distance {distance})",
            contact.point
        );
    }
}

// === Raycast ===

#[test]
fn test_raycast_normalizes_direction_so_distance_is_in_pixels() {
    let mut world = PhysicsWorld::default();
    let target = add_body(&mut world, Vec2::new(200.0, 0.0), RigidBody::new_static(), Collider::box_collider(100.0, 100.0));
    world.step(0.0); // refresh the query pipeline

    let (hit, _, distance_unit) = world.raycast(Vec2::ZERO, Vec2::new(1.0, 0.0), 500.0).expect("unit-direction ray hits");
    let (_, _, distance_long) = world.raycast(Vec2::ZERO, Vec2::new(100.0, 0.0), 500.0).expect("unnormalized ray hits");

    assert_eq!(hit, target);
    assert!((distance_unit - 150.0).abs() < 1.0, "the box edge is at x=150, got {distance_unit}");
    assert!(
        (distance_unit - distance_long).abs() < 0.01,
        "distance is independent of the direction's magnitude ({distance_unit} vs {distance_long})"
    );
    assert_eq!(world.raycast(Vec2::ZERO, Vec2::ZERO, 500.0), None, "a zero direction cannot be normalized");
}
