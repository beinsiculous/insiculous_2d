//! Tests for the PhysicsSystem ECS driver: sync, deferred ops, the
//! fixed-timestep driver, collision event delivery, and the footguns
//! `CLAUDE.md` documents (absolute-pixel colliders, root entities, live
//! `RigidBody` config edits).
use glam::Vec2;

use ecs::sprite_components::Transform2D;
use ecs::{System, World, WorldHierarchyExt};

use crate::components::{Collider, RigidBody};
use crate::test_support::{no_gravity_system, spawn_body};

use super::{DeferredBodyOp, PhysicsSystem, MAX_STEPS_PER_UPDATE};

const DT: f32 = 1.0 / 60.0;

/// A dynamic body that gravity leaves alone.
fn floating_body() -> RigidBody {
    RigidBody::new_dynamic().with_gravity_scale(0.0)
}

fn position_of(world: &World, entity: ecs::EntityId) -> Vec2 {
    world.get::<Transform2D>(entity).expect("transform exists").position
}

// === Defaults ===

#[test]
fn test_new_system_runs_at_sixty_hertz_under_earth_gravity_and_knows_no_bodies() {
    let system = PhysicsSystem::new();
    assert_eq!(system.fixed_timestep, 1.0 / 60.0);
    assert_eq!(system.physics_world().gravity(), Vec2::new(0.0, -980.0));
    assert_eq!(system.get_body_velocity(ecs::EntityId::new()), None, "an unknown entity has no body");
}

// === ECS ↔ rapier sync ===

#[test]
fn test_gravity_moves_a_dynamic_body_but_not_a_static_one() {
    let mut world = World::new();
    let mut system = PhysicsSystem::new();
    let start = Vec2::new(0.0, 100.0);
    let falling = spawn_body(&mut world, start, RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));
    let floor = spawn_body(&mut world, start, RigidBody::new_static(), Collider::box_collider(32.0, 32.0));

    for _ in 0..10 {
        system.update(&mut world, DT);
    }

    assert!(position_of(&world, falling).y < start.y, "the dynamic body should fall");
    assert_eq!(position_of(&world, floor), start, "a static body never moves");
}

#[test]
fn test_direct_world_removal_cleans_up_physics_state() {
    let mut world = World::new();
    let mut system = PhysicsSystem::new();
    let synced = spawn_body(&mut world, Vec2::ZERO, RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));
    system.update(&mut world, DT);
    assert!(system.physics_world().has_rigid_body(synced), "sanity: the body was synced");
    // A same-frame spawn with a buffered launch, removed before it ever syncs.
    let unsynced = spawn_body(&mut world, Vec2::ZERO, RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));
    system.set_velocity(unsynced, Vec2::new(100.0, 0.0), 0.0);
    assert_eq!(system.pending_ops.len(), 1, "sanity: the launch is buffered");

    // Bypass destroy_entity: remove straight from the ECS.
    world.remove_entity(&synced).expect("entity exists");
    world.remove_entity(&unsynced).expect("entity exists");
    system.update(&mut world, DT);

    assert!(!system.physics_world().has_rigid_body(synced), "orphaned rapier body is garbage-collected");
    assert!(!system.physics_world().has_collider(synced), "orphaned rapier collider is garbage-collected");
    assert!(!system.physics_world().has_rigid_body(unsynced), "a removed entity is never synced late");
    assert!(system.pending_ops.is_empty(), "pending ops for a dead entity are pruned");
}

#[test]
fn test_clear_keeps_the_config_and_resyncs_bodies_from_the_ecs() {
    let mut world = World::new();
    let config = crate::PhysicsConfig::new(Vec2::new(0.0, -500.0)).with_scale(50.0);
    let mut system = PhysicsSystem::with_config(config);
    let start = Vec2::new(0.0, 100.0);
    let entity = spawn_body(&mut world, start, RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));
    for _ in 0..30 {
        system.update(&mut world, DT);
    }
    assert!(position_of(&world, entity).y < start.y, "sanity: the body fell");

    // Snapshot restore: the ECS holds the original values again.
    world.get_mut::<Transform2D>(entity).expect("transform").position = start;
    world.get_mut::<RigidBody>(entity).expect("body").velocity = Vec2::ZERO;
    system.clear();

    assert!(!system.physics_world().has_rigid_body(entity), "clear drops every rapier body");
    assert_eq!(system.physics_world().gravity(), Vec2::new(0.0, -500.0), "clear keeps gravity");
    assert_eq!(system.physics_world().config().pixels_per_meter, 50.0, "clear keeps the scale");

    system.update(&mut world, 0.0); // sync only, no step
    assert!(system.physics_world().has_rigid_body(entity), "the body is rebuilt from the ECS");
    assert_eq!(position_of(&world, entity), start, "the rebuilt body starts from the restored transform");
}

// === Deferred body ops ===

#[test]
fn test_reset_body_and_set_velocity_apply_in_call_order_live_or_deferred() {
    let mut world = World::new();
    let mut system = no_gravity_system();

    // Same-frame spawn: reset then launch, before any update() has synced
    // the body. Both are buffered, and the reset must not clobber the launch.
    let entity = spawn_body(&mut world, Vec2::new(50.0, 50.0), RigidBody::new_dynamic(), Collider::box_collider(8.0, 8.0));
    system.reset_body(entity, Vec2::ZERO);
    system.set_velocity(entity, Vec2::new(200.0, 0.0), 0.0);
    assert_eq!(system.pending_ops.len(), 2, "reset + launch are both buffered");
    assert!(matches!(system.pending_ops[0].1, DeferredBodyOp::Reset { .. }), "ops drain in call order: reset first");

    system.update(&mut world, DT);

    let (velocity, _) = system.get_body_velocity(entity).expect("body exists");
    assert!((velocity.x - 200.0).abs() < 1.0, "the deferred launch velocity lands intact (got {velocity:?})");
    let after_launch = position_of(&world, entity);
    assert!(
        (after_launch - Vec2::new(200.0 * DT, 0.0)).length() < 1.0,
        "the body was reset to the origin before the launch (got {after_launch:?})"
    );
    assert!(system.pending_ops.is_empty());

    // Live body: reset moves it and zeroes the velocity immediately.
    system.set_velocity(entity, Vec2::new(999.0, 999.0), 0.0);
    system.reset_body(entity, Vec2::new(100.0, 200.0));
    let (velocity, _) = system.get_body_velocity(entity).expect("body exists");
    assert_eq!(velocity, Vec2::ZERO, "reset zeroes the velocity");
    system.update(&mut world, DT);
    let position = position_of(&world, entity);
    assert!(
        (position - Vec2::new(100.0, 200.0)).length() < 0.01,
        "reset moves the body to the requested position (got {position:?})"
    );
}

// === Fixed-timestep driver ===

#[test]
fn test_catch_up_steps_are_capped_after_a_stall() {
    let mut world = World::new();
    // Tiny fixed timestep: a 0.1s update would need 100 catch-up steps
    // uncapped; the cap drops the excess instead of simulating it.
    let step = 1.0 / 1000.0;
    let mut system = PhysicsSystem::new();
    system.fixed_timestep = step;
    let entity = spawn_body(&mut world, Vec2::new(0.0, 100.0), RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));

    system.update(&mut world, 0.1);

    // At most MAX_STEPS_PER_UPDATE steps of 1ms ran, so at most 8ms of
    // gravity was simulated (~0.03 units of fall), not 100ms (~4.9 units).
    let fallen = 100.0 - position_of(&world, entity).y;
    let max_simulated = MAX_STEPS_PER_UPDATE as f32 * step;
    let max_fall = 0.5 * 980.0 * max_simulated * max_simulated + 1.0;
    assert!(fallen <= max_fall, "fell {fallen} units; catch-up steps were not capped");

    // The dropped backlog must not be simulated later: a follow-up tiny
    // update runs at most one step.
    let y_before = position_of(&world, entity).y;
    system.update(&mut world, step);
    let y_after = position_of(&world, entity).y;
    assert!((y_before - y_after).abs() < 0.01, "accumulated backlog leaked into the next update");
}

// === Collision event delivery ===

/// A no-gravity system over two overlapping dynamic bodies, returned with
/// the pair's entity ids.
fn overlapping_pair(world: &mut World) -> (PhysicsSystem, [ecs::EntityId; 2]) {
    let pair = [0.0, 10.0]
        .map(|x| spawn_body(world, Vec2::new(x, 0.0), floating_body(), Collider::box_collider(32.0, 32.0)));
    (no_gravity_system(), pair)
}

fn started_count(system: &mut PhysicsSystem) -> usize {
    system.take_collision_events().iter().filter(|c| c.event.started).count()
}

#[test]
fn test_started_event_is_delivered_once_and_zero_step_frames_emit_nothing() {
    let mut world = World::new();
    let (mut system, _) = overlapping_pair(&mut world);

    system.update(&mut world, DT); // exactly one fixed step: the collision starts
    assert_eq!(started_count(&mut system), 1);

    // Too small to step: the last step's events must not be re-delivered.
    system.update(&mut world, 0.001);
    assert!(system.take_collision_events().is_empty(), "a frame with zero physics steps emits no events");
    system.update(&mut world, 0.001);
    assert_eq!(started_count(&mut system), 0, "started is never re-emitted on a zero-step frame");
}

#[test]
fn test_second_take_collision_events_in_a_frame_returns_empty() {
    let mut world = World::new();
    let (mut system, _) = overlapping_pair(&mut world);
    system.update(&mut world, DT);

    let first = system.take_collision_events();
    let second = system.take_collision_events();

    assert!(!first.is_empty(), "the first take returns the frame's events");
    assert!(second.is_empty(), "taking is a drain: a second take in the same frame gets nothing");
    assert!(system.physics_world().collision_events().is_empty(), "the underlying buffer is empty after take");
}

#[test]
fn test_events_from_all_sub_steps_in_one_update_survive() {
    let mut world = World::new();
    let (mut system, [left, right]) = overlapping_pair(&mut world);
    system.update(&mut world, DT); // the collision starts

    // Two catch-up sub-steps, each reporting the still-overlapping pair;
    // the second sub-step must not wipe the first one's events.
    system.update(&mut world, 2.0 * DT);

    let ongoing = system.take_collision_events().iter().filter(|e| !e.event.stopped).count();
    assert_eq!(ongoing, 2, "every sub-step's events in one update are delivered");
    let separation = position_of(&world, right).x - position_of(&world, left).x;
    assert!(separation > 10.0, "the sub-steps actually simulated: the overlap is being resolved (separation {separation})");
}

// === Collider placement footguns ===

#[test]
fn test_collider_size_is_absolute_pixels_and_ignores_transform_scale() {
    // The first footgun in CLAUDE.md: a sprite scaled up via Transform2D.scale
    // keeps its authored collider size. A 20px box scaled ×10 would reach
    // the body 100px away if scale applied; it must not.
    let mut world = World::new();
    let mut system = no_gravity_system();
    let scaled = spawn_body(&mut world, Vec2::ZERO, floating_body(), Collider::box_collider(20.0, 20.0));
    world.get_mut::<Transform2D>(scaled).expect("transform").scale = Vec2::splat(10.0);
    spawn_body(&mut world, Vec2::new(100.0, 0.0), floating_body(), Collider::box_collider(20.0, 20.0));

    system.update(&mut world, DT);

    assert!(system.take_collision_events().is_empty(), "Transform2D.scale does not grow the collider");
    assert_eq!(position_of(&world, scaled), Vec2::ZERO, "nothing pushed the scaled body");
}

#[test]
fn test_collider_offset_moves_the_collision_shape_away_from_the_body() {
    let mut world = World::new();
    let mut system = no_gravity_system();
    let offset_box = Collider::box_collider(20.0, 20.0).with_offset(Vec2::new(100.0, 0.0));
    let carrier = spawn_body(&mut world, Vec2::ZERO, floating_body(), offset_box);
    let at_offset = spawn_body(&mut world, Vec2::new(100.0, 0.0), RigidBody::new_static(), Collider::box_collider(20.0, 20.0));
    let at_body = spawn_body(&mut world, Vec2::ZERO, RigidBody::new_static(), Collider::box_collider(20.0, 20.0));

    system.update(&mut world, DT);

    let events = system.take_collision_events();
    assert!(
        events.iter().any(|c| c.event.started && c.event.involves(carrier, at_offset)),
        "the offset collider collides where the offset puts it"
    );
    assert!(
        !events.iter().any(|c| c.event.involves(carrier, at_body)),
        "an unoffset collider would overlap the box at the body's own position; the offset one does not"
    );
}

#[test]
fn test_colliders_in_non_overlapping_collision_groups_produce_no_events() {
    let mut world = World::new();
    let mut system = no_gravity_system();
    // Two overlapping boxes: A is in group 1 and only talks to group 1,
    // B is in group 2 and only talks to group 2.
    let mut col_a = Collider::box_collider(32.0, 32.0);
    col_a.collision_groups = 0b01;
    col_a.collision_filter = 0b01;
    let a = spawn_body(&mut world, Vec2::ZERO, floating_body(), col_a);

    let mut col_b = Collider::box_collider(32.0, 32.0);
    col_b.collision_groups = 0b10;
    col_b.collision_filter = 0b10;
    let b = spawn_body(&mut world, Vec2::new(10.0, 0.0), floating_body(), col_b);
    system.update(&mut world, DT);
    assert!(system.take_collision_events().is_empty(), "groups that do not overlap never collide");
    assert_eq!(position_of(&world, a), Vec2::ZERO, "and are not pushed apart");

    // Putting B in both groups, talking to both, makes the same overlap collide.
    let collider_b = world.get_mut::<Collider>(b).expect("collider");
    collider_b.collision_groups = 0b11;
    collider_b.collision_filter = 0b11;
    system.update(&mut world, DT);
    assert!(
        system.take_collision_events().iter().any(|c| c.event.started && c.event.involves(a, b)),
        "the collision appears once the filters overlap"
    );
}

// === Body types and live edits ===

#[test]
fn test_kinematic_body_ignores_gravity_moves_to_its_target_and_is_not_pushed() {
    let mut world = World::new();
    let mut system = PhysicsSystem::new(); // default gravity
    let paddle = spawn_body(&mut world, Vec2::ZERO, RigidBody::new_kinematic(), Collider::box_collider(40.0, 40.0));
    let ball = spawn_body(&mut world, Vec2::new(10.0, 0.0), floating_body(), Collider::box_collider(40.0, 40.0));

    system.update(&mut world, DT);
    assert!(
        system.take_collision_events().iter().any(|c| c.event.started && c.event.involves(paddle, ball)),
        "a kinematic body still reports collisions"
    );
    for _ in 0..10 {
        system.update(&mut world, DT);
    }
    assert_eq!(position_of(&world, paddle), Vec2::ZERO, "gravity and the overlapping ball leave the kinematic body where it is");
    assert!(position_of(&world, ball).x > 10.0, "the dynamic body is the one pushed out");

    system.set_kinematic_target(paddle, Vec2::new(50.0, 0.0), 0.0);
    system.update(&mut world, DT);

    let position = position_of(&world, paddle);
    assert!((position - Vec2::new(50.0, 0.0)).length() < 0.01, "the kinematic body moves to its target (got {position:?})");
}

#[test]
fn test_live_rigid_body_config_edit_needs_the_body_rebuilt() {
    // Pins the documented limitation: unlike Transform2D and Collider edits,
    // a RigidBody config edit on a live body is not pushed into rapier.
    // `clear()` rebuilds the body from the ECS and the edit takes effect.
    let mut world = World::new();
    let mut system = PhysicsSystem::new();
    let entity = spawn_body(&mut world, Vec2::new(0.0, 100.0), RigidBody::new_dynamic(), Collider::box_collider(32.0, 32.0));
    system.update(&mut world, DT);

    world.get_mut::<RigidBody>(entity).expect("body").gravity_scale = 0.0;
    let before = position_of(&world, entity).y;
    system.update(&mut world, DT);
    assert_eq!(system.external_edits_pushed_last_update(), 0, "a RigidBody edit is not detected as an external edit");
    assert!(position_of(&world, entity).y < before, "the live body still falls under its old config");

    system.clear();
    world.get_mut::<RigidBody>(entity).expect("body").velocity = Vec2::ZERO;
    system.update(&mut world, DT);
    let rebuilt = position_of(&world, entity).y;
    system.update(&mut world, DT);
    assert_eq!(position_of(&world, entity).y, rebuilt, "the rebuilt body honours the edited gravity scale");
}

#[test]
fn test_parented_entity_with_rigid_body_is_treated_as_world_space() {
    // Physics reads and writes Transform2D as world-space and ignores the
    // ECS hierarchy: a child's LOCAL transform becomes the body's WORLD
    // position, and the result is written straight back into the local
    // transform. Rule: physics entities must be root entities.
    let mut world = World::new();
    let mut system = no_gravity_system();
    let parent = world.spawn().with(Transform2D::new(Vec2::new(100.0, 0.0))).id();
    let child = spawn_body(&mut world, Vec2::new(0.0, 50.0), floating_body(), Collider::box_collider(16.0, 16.0));
    world.set_parent(child, parent).expect("reparent");

    system.update(&mut world, DT);

    let (body_position, _) = system.physics_world().get_body_transform(child).expect("child body exists");
    assert!(
        (body_position - Vec2::new(0.0, 50.0)).length() < 1.0,
        "the parent's offset is not applied to the body (body at {body_position:?})"
    );
    let local = position_of(&world, child);
    assert!(
        (local - Vec2::new(0.0, 50.0)).length() < 1.0,
        "the body position is written into the local transform unchanged (got {local:?})"
    );
}
