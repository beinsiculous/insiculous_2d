//! Contract tests for ScriptRunner lifecycle, Blackboard, and command dispatch.

use std::cell::Cell;
use std::collections::BTreeMap;


use glam::Vec2;

use common::Transform2D;
use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::{Name, System};
use physics::{
    Collider, CollisionData, CollisionEvent, PhysicsConfig, PhysicsSystem, RigidBody,
};

use crate::scripting::commands::{ScriptCommand, ScriptCommands, Target};
use crate::scripting::registry::{ScriptBehavior, ScriptDescriptor};
use crate::scripting::runner::{ScriptErrorKind, ScriptErrors, ScriptRunner};
use crate::scripting::view::{ScriptView, SelfView};

use super::{test_inputs, test_world};

#[test]
fn test_engine_rotate_rotates_by_degrees_per_second_dt() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, "");

    let entity = world.create_entity();
    world
        .add_component(&entity, Transform2D::default())
        .expect("added transform");
    world
        .add_component(&entity, Scripts(vec![ScriptRef::new("engine::rotate")]))
        .expect("added scripts");

    let dt = 0.5;
    runner.early_update(&mut world, &input, &players, dt, None);
    runner.update(&mut world, &input, &players, dt, &[], None);

    let transform = world.get::<Transform2D>(entity).expect("has transform");
    let expected_radians = 90.0_f32.to_radians() * dt;
    assert!((transform.rotation - expected_radians).abs() < 1e-4);
}

#[test]
fn test_unresolved_id_reported_once() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, "");

    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            Scripts(vec![ScriptRef::new("unresolved::custom_script")]),
        )
        .expect("added scripts");

    for _ in 0..3 {
        runner.early_update(&mut world, &input, &players, 0.016, None);
        runner.update(&mut world, &input, &players, 0.016, &[], None);
    }

    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].kind, ScriptErrorKind::UnresolvedScript);
    assert_eq!(runner.errors()[0].file, "unresolved::custom_script");
}

#[test]
fn test_reset_clears_blackboard() {
    let mut world = test_world();
    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, "");

    {
        let bb = world.resource_mut::<Blackboard>().expect("has blackboard");
        bb.set("score", ScriptValue::I32(100));
    }
    assert_eq!(
        world
            .resource::<Blackboard>()
            .and_then(|b| b.get("score")),
        Some(&ScriptValue::I32(100))
    );

    runner.reset(&mut world, "");
    assert_eq!(
        world
            .resource::<Blackboard>()
            .and_then(|b| b.get("score")),
        None
    );
}

#[test]
fn test_blackboard_read_unset_key_returns_default() {
    let view = ScriptView {
        delta_time: 0.016,
        frame: 0,
        blackboard: BTreeMap::new(),
        entities: BTreeMap::new(),
        collisions: Vec::new(),
        player_axes: Vec::new(),
        player_actions_active: Vec::new(),
        player_actions_just_activated: Vec::new(),
    };

    assert!(view.blackboard_bool("missing", true));
    assert!(!view.blackboard_bool("missing", false));
    assert_eq!(view.blackboard_int("missing", 42), 42);
    assert!((view.blackboard_float("missing", 3.5) - 3.5).abs() < 1e-4);
    assert_eq!(view.blackboard_str("missing", "fallback"), "fallback");
}

#[test]
fn test_two_instances_issue_velocity_and_reset_in_either_order_ends_reset_and_still() {
    // Permutation 1: SetVelocity then ResetBody
    {
        let mut world = test_world();
        let entity = world.create_entity();
        world
            .add_component(&entity, Transform2D::new(Vec2::ZERO))
            .unwrap();
        world
            .add_component(&entity, RigidBody::new_dynamic())
            .unwrap();

        let mut commands = ScriptCommands::new();
        commands.commands.push(ScriptCommand::SetVelocity {
            target: Target::Entity(entity),
            velocity: Vec2::new(100.0, 50.0),
        });
        commands.commands.push(ScriptCommand::ResetBody {
            target: Target::Entity(entity),
            position: Vec2::new(10.0, 20.0),
        });

        let mut errors = ScriptErrors::new();
        commands.apply(&mut world, None, 0.016, &mut errors);

        let transform = world.get::<Transform2D>(entity).unwrap();
        let rb = world.get::<RigidBody>(entity).unwrap();
        assert_eq!(transform.position, Vec2::new(10.0, 20.0));
        assert_eq!(rb.velocity, Vec2::ZERO);
    }

    // Permutation 2: ResetBody then SetVelocity
    {
        let mut world = test_world();
        let entity = world.create_entity();
        world
            .add_component(&entity, Transform2D::new(Vec2::ZERO))
            .unwrap();
        world
            .add_component(&entity, RigidBody::new_dynamic())
            .unwrap();

        let mut commands = ScriptCommands::new();
        commands.commands.push(ScriptCommand::ResetBody {
            target: Target::Entity(entity),
            position: Vec2::new(10.0, 20.0),
        });
        commands.commands.push(ScriptCommand::SetVelocity {
            target: Target::Entity(entity),
            velocity: Vec2::new(100.0, 50.0),
        });

        let mut errors = ScriptErrors::new();
        commands.apply(&mut world, None, 0.016, &mut errors);

        let transform = world.get::<Transform2D>(entity).unwrap();
        let rb = world.get::<RigidBody>(entity).unwrap();
        assert_eq!(transform.position, Vec2::new(10.0, 20.0));
        assert_eq!(rb.velocity, Vec2::ZERO);
    }
}

thread_local! {
    static EARLY_SEEN: Cell<bool> = const { Cell::new(false) };
    static UPDATE_SEEN: Cell<bool> = const { Cell::new(false) };
}

struct CollisionProbeBehavior;

impl ScriptBehavior for CollisionProbeBehavior {
    fn early_update(
        &mut self,
        _me: &SelfView,
        view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        _commands: &mut ScriptCommands,
    ) {
        EARLY_SEEN.with(|c| c.set(view.has_collision("entity_a", "entity_b")));
    }

    fn update(
        &mut self,
        _me: &SelfView,
        view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        _commands: &mut ScriptCommands,
    ) {
        UPDATE_SEEN.with(|c| c.set(view.has_collision("entity_a", "entity_b")));
    }
}

fn make_collision_probe() -> Box<dyn ScriptBehavior> {
    Box::new(CollisionProbeBehavior)
}

#[test]
fn test_update_view_carries_collision_and_early_update_does_not() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let mut runner = ScriptRunner::new();

    EARLY_SEEN.with(|c| c.set(false));
    UPDATE_SEEN.with(|c| c.set(false));

    runner.registry_mut().register(ScriptDescriptor {
        id: "test::probe",
        display_name: "Probe",
        category: "Test",
        params: &[],
        make: make_collision_probe,
    });

    runner.reset(&mut world, "");

    let entity_a = world.create_entity();
    world
        .add_component(&entity_a, Name("entity_a".to_string()))
        .unwrap();
    world
        .add_component(&entity_a, Scripts(vec![ScriptRef::new("test::probe")]))
        .unwrap();

    let entity_b = world.create_entity();
    world
        .add_component(&entity_b, Name("entity_b".to_string()))
        .unwrap();

    let collision = CollisionData {
        event: CollisionEvent {
            entity_a,
            entity_b,
            started: true,
            stopped: false,
        },
        contacts: Vec::new(),
    };

    runner.early_update(&mut world, &input, &players, 0.016, None);
    assert!(
        !EARLY_SEEN.with(|c| c.get()),
        "early_update must not see collisions"
    );

    runner.update(&mut world, &input, &players, 0.016, &[collision], None);
    assert!(
        UPDATE_SEEN.with(|c| c.get()),
        "update must see injected collisions"
    );
}

#[test]
fn test_kinematic_target_routes_through_physics_when_present_and_transform_when_absent() {
    let mut world = test_world();
    let mut physics = PhysicsSystem::with_config(PhysicsConfig {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    // Case 1: Physics body present
    let body_entity = world.create_entity();
    world
        .add_component(&body_entity, Transform2D::new(Vec2::ZERO))
        .unwrap();
    world
        .add_component(&body_entity, RigidBody::new_kinematic())
        .unwrap();
    world
        .add_component(&body_entity, Collider::box_collider(10.0, 10.0))
        .unwrap();
    physics.initialize(&mut world).unwrap();
    physics.update(&mut world, 1.0 / 60.0);

    // Case 2: Physics body absent (pure Transform2D)
    let non_body_entity = world.create_entity();
    world
        .add_component(&non_body_entity, Transform2D::new(Vec2::ZERO))
        .unwrap();

    let mut commands = ScriptCommands::new();
    commands.commands.push(ScriptCommand::SetKinematicTarget {
        target: Target::Entity(body_entity),
        position: Vec2::new(100.0, 200.0),
    });
    commands.commands.push(ScriptCommand::SetKinematicTarget {
        target: Target::Entity(non_body_entity),
        position: Vec2::new(30.0, 40.0),
    });

    let mut errors = ScriptErrors::new();
    commands.apply(&mut world, Some(&mut physics), 0.016, &mut errors);

    // Non-body entity immediately updated Transform2D
    let non_body_transform = world.get::<Transform2D>(non_body_entity).unwrap();
    assert_eq!(non_body_transform.position, Vec2::new(30.0, 40.0));

    // Body entity target lands during physics step
    physics.update(&mut world, 1.0 / 60.0);
    let body_transform = world.get::<Transform2D>(body_entity).unwrap();
    assert_eq!(body_transform.position, Vec2::new(100.0, 200.0));
}

struct MoveKinematicBehavior;
impl ScriptBehavior for MoveKinematicBehavior {
    fn early_update(
        &mut self,
        me: &SelfView,
        _view: &ScriptView,
        _params: &BTreeMap<String, ScriptValue>,
        commands: &mut ScriptCommands,
    ) {
        commands.commands.push(ScriptCommand::SetKinematicTarget {
            target: Target::Entity(me.entity),
            position: Vec2::new(0.0, 0.0),
        });
    }
}

fn make_mover() -> Box<dyn ScriptBehavior> {
    Box::new(MoveKinematicBehavior)
}

#[test]
fn test_early_update_kinematic_target_lands_before_step_and_update_sees_collision() {
    let dt = 1.0 / 60.0;
    let mut world = test_world();
    let (input, players) = test_inputs();
    let mut physics = PhysicsSystem::with_config(PhysicsConfig {
        gravity: Vec2::ZERO,
        ..Default::default()
    });

    let moving_entity = world.create_entity();
    world
        .add_component(&moving_entity, Name("mover".to_string()))
        .unwrap();
    world
        .add_component(&moving_entity, Transform2D::new(Vec2::new(-50.0, 0.0)))
        .unwrap();
    world
        .add_component(&moving_entity, RigidBody::new_kinematic())
        .unwrap();
    world
        .add_component(&moving_entity, Collider::box_collider(20.0, 20.0))
        .unwrap();

    let static_entity = world.create_entity();
    world
        .add_component(&static_entity, Name("ball".to_string()))
        .unwrap();
    world
        .add_component(&static_entity, Transform2D::new(Vec2::ZERO))
        .unwrap();
    world
        .add_component(
            &static_entity,
            RigidBody::new_dynamic().with_gravity_scale(0.0),
        )
        .unwrap();
    world
        .add_component(&static_entity, Collider::box_collider(20.0, 20.0))
        .unwrap();

    physics.initialize(&mut world).unwrap();
    physics.update(&mut world, dt);
    let _ = physics.take_collision_events();

    let mut runner = ScriptRunner::new();
    runner.registry_mut().register(ScriptDescriptor {
        id: "test::mover",
        display_name: "Mover",
        category: "Test",
        params: &[],
        make: make_mover,
    });
    runner.reset(&mut world, "");

    world
        .add_component(
            &moving_entity,
            Scripts(vec![ScriptRef::new("test::mover")]),
        )
        .unwrap();

    runner.early_update(&mut world, &input, &players, dt, Some(&mut physics));

    physics.update(&mut world, dt);
    let collisions = physics.take_collision_events();

    assert!(
        !collisions.is_empty(),
        "Collision should have occurred after kinematic target step"
    );
    assert!(collisions
        .iter()
        .any(|c| c.event.involves(moving_entity, static_entity)));

    runner.update(
        &mut world,
        &input,
        &players,
        dt,
        &collisions,
        Some(&mut physics),
    );
}

#[test]
fn test_two_missing_target_names_are_both_reported() {
    let mut world = test_world();
    let mut commands = ScriptCommands::new();
    commands.set_position("Ball", Vec2::ZERO);
    commands.set_position("Goal", Vec2::ZERO);

    let mut errors = ScriptErrors::new();
    commands.apply(&mut world, None, 0.016, &mut errors);

    let missing: Vec<&str> = errors.as_slice().iter().map(|error| error.file.as_str()).collect();
    assert_eq!(missing, ["Ball", "Goal"], "one report per missing name, not one per kind");
}
