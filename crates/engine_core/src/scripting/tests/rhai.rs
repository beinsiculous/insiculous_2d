//! Headless tests verifying Rhai script execution and contracts.

use std::fs;

use glam::Vec2;
use tempfile::tempdir;

use common::Transform2D;
use ecs::script::{ScriptRef, Scripts};
use ecs::Name;

use crate::scripting::param_header::parse_param_header;
use crate::scripting::runner::{ScriptErrorKind, ScriptRunner};

use super::{test_inputs, test_world};

#[test]
fn test_rhai_moves_toward_named_target_over_three_frames() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let script_code = r#"
        fn update(me, view, params, cmd, dt) {
            let target_pos = view.position("target");
            let dir = (target_pos - me.position).normalize();
            cmd.set_position(me, me.position + dir * 10.0);
        }
    "#;
    let script_path = dir.path().join("chase.rhai");
    fs::write(&script_path, script_code).expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let paddle = world.create_entity();
    world
        .add_component(&paddle, Name("paddle".to_string()))
        .unwrap();
    world
        .add_component(&paddle, Transform2D::new(Vec2::new(0.0, 0.0)))
        .unwrap();
    let mut script_ref = ScriptRef::new("paddle_script");
    script_ref.source_path = "chase.rhai".to_string();
    world
        .add_component(&paddle, Scripts(vec![script_ref]))
        .unwrap();

    let target = world.create_entity();
    world
        .add_component(&target, Name("target".to_string()))
        .unwrap();
    world
        .add_component(&target, Transform2D::new(Vec2::new(100.0, 0.0)))
        .unwrap();

    let dt = 1.0;
    for frame in 1..=3 {
        runner.early_update(&mut world, &input, &players, dt, None);
        runner.update(&mut world, &input, &players, dt, &[], None);

        let pos = world.get::<Transform2D>(paddle).unwrap().position;
        let expected_x = (frame * 10) as f32;
        assert!(
            (pos.x - expected_x).abs() < 1e-3,
            "Frame {frame}: expected x={expected_x}, got {}",
            pos.x
        );
    }
}

#[test]
fn test_two_instances_each_move_their_own_entity() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let script_code = r#"
        fn update(me, view, params, cmd, dt) {
            cmd.set_position(me, me.position + vec2(10.0, 5.0));
        }
    "#;
    fs::write(dir.path().join("move.rhai"), script_code).expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity1 = world.create_entity();
    world
        .add_component(&entity1, Transform2D::new(Vec2::new(0.0, 0.0)))
        .unwrap();
    let mut ref1 = ScriptRef::new("move_script");
    ref1.source_path = "move.rhai".to_string();
    world.add_component(&entity1, Scripts(vec![ref1])).unwrap();

    let entity2 = world.create_entity();
    world
        .add_component(&entity2, Transform2D::new(Vec2::new(100.0, 200.0)))
        .unwrap();
    let mut ref2 = ScriptRef::new("move_script");
    ref2.source_path = "move.rhai".to_string();
    world.add_component(&entity2, Scripts(vec![ref2])).unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    let pos1 = world.get::<Transform2D>(entity1).unwrap().position;
    let pos2 = world.get::<Transform2D>(entity2).unwrap().position;

    assert_eq!(pos1, Vec2::new(10.0, 5.0), "Entity 1 moved under its own me");
    assert_eq!(pos2, Vec2::new(110.0, 205.0), "Entity 2 moved under its own me");
}

#[test]
fn test_script_on_a_resets_b_by_name() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let script_code = r#"
        fn update(me, view, params, cmd, dt) {
            cmd.reset_body("entity_b", vec2(150.0, 250.0));
        }
    "#;
    fs::write(dir.path().join("reset_b.rhai"), script_code).expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity_a = world.create_entity();
    world
        .add_component(&entity_a, Name("entity_a".to_string()))
        .unwrap();
    let mut ref_a = ScriptRef::new("reset_b");
    ref_a.source_path = "reset_b.rhai".to_string();
    world.add_component(&entity_a, Scripts(vec![ref_a])).unwrap();

    let entity_b = world.create_entity();
    world
        .add_component(&entity_b, Name("entity_b".to_string()))
        .unwrap();
    world
        .add_component(&entity_b, Transform2D::new(Vec2::ZERO))
        .unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    let pos_b = world.get::<Transform2D>(entity_b).unwrap().position;
    assert_eq!(pos_b, Vec2::new(150.0, 250.0));
}

#[test]
fn test_missing_target_name_reported_once_and_rest_apply() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let script_code = r#"
        fn update(me, view, params, cmd, dt) {
            cmd.set_position("ghost", vec2(999.0, 999.0));
            cmd.set_position(me, vec2(42.0, 42.0));
        }
    "#;
    fs::write(dir.path().join("missing_target.rhai"), script_code).expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity = world.create_entity();
    world
        .add_component(&entity, Transform2D::new(Vec2::ZERO))
        .unwrap();
    let mut script_ref = ScriptRef::new("missing_target");
    script_ref.source_path = "missing_target.rhai".to_string();
    world
        .add_component(&entity, Scripts(vec![script_ref]))
        .unwrap();

    for _ in 0..2 {
        runner.early_update(&mut world, &input, &players, 0.016, None);
        runner.update(&mut world, &input, &players, 0.016, &[], None);
    }

    assert_eq!(runner.errors().len(), 1, "Missing target reported exactly once");
    assert_eq!(runner.errors()[0].kind, ScriptErrorKind::MissingTarget);
    assert_eq!(
        world.get::<Transform2D>(entity).unwrap().position,
        Vec2::new(42.0, 42.0),
        "Remaining commands in the frame applied successfully"
    );
}

#[test]
fn test_hook_error_after_command_leaves_world_untouched() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let script_code = r#"
        fn update(me, view, params, cmd, dt) {
            cmd.set_position(me, vec2(999.0, 999.0));
            throw "simulated error";
        }
    "#;
    fs::write(dir.path().join("failing.rhai"), script_code).expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity = world.create_entity();
    world
        .add_component(&entity, Transform2D::new(Vec2::new(10.0, 20.0)))
        .unwrap();
    let mut script_ref = ScriptRef::new("failing");
    script_ref.source_path = "failing.rhai".to_string();
    world
        .add_component(&entity, Scripts(vec![script_ref]))
        .unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    assert_eq!(
        world.get::<Transform2D>(entity).unwrap().position,
        Vec2::new(10.0, 20.0),
        "Commands issued by a failing hook must be discarded"
    );
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].kind, ScriptErrorKind::Runtime);
}

#[test]
fn test_rhai_param_header_defaults_run_and_catalog_lists_them() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let script_code = r#"
        // @param speed: f32 = 45.0
        fn update(me, view, params, cmd, dt) {
            cmd.set_position(me, me.position + vec2(params.speed * dt, 0.0));
        }
    "#;
    fs::write(dir.path().join("header.rhai"), script_code).expect("wrote script");

    // Header parser extracts defaults
    let defaults = parse_param_header(script_code).expect("parsed header");
    assert!(defaults.contains_key("speed"));

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity = world.create_entity();
    world
        .add_component(&entity, Transform2D::new(Vec2::ZERO))
        .unwrap();
    let mut script_ref = ScriptRef::new("header_script");
    script_ref.source_path = "header.rhai".to_string();
    // ref has empty params map, so header default 45.0 will be used
    world
        .add_component(&entity, Scripts(vec![script_ref]))
        .unwrap();

    let dt = 2.0;
    runner.early_update(&mut world, &input, &players, dt, None);
    runner.update(&mut world, &input, &players, dt, &[], None);

    let pos = world.get::<Transform2D>(entity).unwrap().position;
    assert!((pos.x - 90.0).abs() < 1e-3, "Expected 90.0, got {}", pos.x);
}

#[test]
fn test_compile_error_reaches_error_list_and_leaves_others_running() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let broken_code = "fn broken_syntax {{{";
    fs::write(dir.path().join("broken.rhai"), broken_code).expect("wrote broken");

    let good_code = r#"
        fn update(me, view, params, cmd, dt) {
            cmd.set_position(me, vec2(50.0, 50.0));
        }
    "#;
    fs::write(dir.path().join("good.rhai"), good_code).expect("wrote good");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity_broken = world.create_entity();
    let mut ref_broken = ScriptRef::new("broken");
    ref_broken.source_path = "broken.rhai".to_string();
    world
        .add_component(&entity_broken, Scripts(vec![ref_broken]))
        .unwrap();

    let entity_good = world.create_entity();
    world
        .add_component(&entity_good, Transform2D::new(Vec2::ZERO))
        .unwrap();
    let mut ref_good = ScriptRef::new("good");
    ref_good.source_path = "good.rhai".to_string();
    world
        .add_component(&entity_good, Scripts(vec![ref_good]))
        .unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    assert!(runner.errors().iter().any(|e| e.kind == ScriptErrorKind::Syntax));
    assert_eq!(
        world.get::<Transform2D>(entity_good).unwrap().position,
        Vec2::new(50.0, 50.0),
        "Good script continues executing despite syntax error in sibling"
    );
}

#[test]
fn test_runaway_loop_reported_once_across_many_frames() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");

    let loop_code = r#"
        fn update(me, view, params, cmd, dt) {
            while true {}
        }
    "#;
    fs::write(dir.path().join("loop.rhai"), loop_code).expect("wrote loop");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().unwrap());

    let entity = world.create_entity();
    let mut script_ref = ScriptRef::new("infinite_loop");
    script_ref.source_path = "loop.rhai".to_string();
    world
        .add_component(&entity, Scripts(vec![script_ref]))
        .unwrap();

    for _ in 0..5 {
        runner.early_update(&mut world, &input, &players, 0.016, None);
        runner.update(&mut world, &input, &players, 0.016, &[], None);
    }

    assert_eq!(runner.errors().len(), 1, "Runaway loop reported once");
    assert_eq!(runner.errors()[0].kind, ScriptErrorKind::RunawayLoop);
}

#[test]
fn test_set_velocity_x_on_a_named_target_keeps_its_vertical_velocity() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("serve.rhai"),
        r#"fn update(me, view, params, cmd, dt) { cmd.set_velocity_x("Ball", 250); }"#,
    )
    .expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));

    let ball = world.create_entity();
    world.add_component(&ball, Name("Ball".to_string())).unwrap();
    world.add_component(&ball, Transform2D::new(Vec2::ZERO)).unwrap();
    let mut body = physics::RigidBody::new_dynamic();
    body.velocity = Vec2::new(0.0, 40.0);
    world.add_component(&ball, body).unwrap();

    let referee = world.create_entity();
    let mut script_ref = ScriptRef::new("serve");
    script_ref.source_path = "serve.rhai".to_string();
    world.add_component(&referee, Scripts(vec![script_ref])).unwrap();

    runner.early_update(&mut world, &input, &players, 1.0, None);
    runner.update(&mut world, &input, &players, 1.0, &[], None);

    let position = world.get::<Transform2D>(ball).unwrap().position;
    assert_eq!(position, Vec2::new(250.0, 40.0), "the named form keeps the y velocity");
    assert!(runner.errors().is_empty(), "{:?}", runner.errors());
}

#[test]
fn test_blackboard_int_outside_i32_is_a_runtime_error_not_zero() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("big.rhai"),
        r#"fn update(me, view, params, cmd, dt) { cmd.set_blackboard_int("big", 5000000000); }"#,
    )
    .expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));
    let entity = world.create_entity();
    let mut script_ref = ScriptRef::new("big");
    script_ref.source_path = "big.rhai".to_string();
    world.add_component(&entity, Scripts(vec![script_ref])).unwrap();

    runner.early_update(&mut world, &input, &players, 0.016, None);
    runner.update(&mut world, &input, &players, 0.016, &[], None);

    let errors = runner.errors();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].kind, ScriptErrorKind::Runtime);
    assert!(errors[0].message.contains("i32"), "{}", errors[0].message);
    let blackboard = world.resource::<ecs::Blackboard>().expect("blackboard installed by reset");
    assert!(blackboard.get("big").is_none(), "the failing hook's write was discarded");
}

#[test]
fn test_source_edited_mid_play_runs_as_compiled_until_the_next_play() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("step.rhai");
    let step_by = |amount: u32| {
        format!("fn update(me, view, params, cmd, dt) {{ cmd.set_position(me, me.position + vec2({amount}.0, 0.0)); }}")
    };
    fs::write(&path, step_by(10)).expect("wrote script");

    let mut runner = ScriptRunner::new();
    let base = dir.path().to_str().expect("utf-8 path").to_string();
    runner.reset(&mut world, &base);
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).unwrap();
    let mut script_ref = ScriptRef::new("step");
    script_ref.source_path = "step.rhai".to_string();
    world.add_component(&entity, Scripts(vec![script_ref])).unwrap();

    let frame = |runner: &mut ScriptRunner, world: &mut ecs::World| {
        runner.early_update(world, &input, &players, 1.0, None);
        runner.update(world, &input, &players, 1.0, &[], None);
        world.get::<Transform2D>(entity).unwrap().position.x
    };

    assert_eq!(frame(&mut runner, &mut world), 10.0);
    fs::write(&path, step_by(20)).expect("rewrote script");
    assert_eq!(frame(&mut runner, &mut world), 20.0, "mid-Play the compiled unit keeps running");

    runner.reset(&mut world, &base);
    assert_eq!(frame(&mut runner, &mut world), 40.0, "the next Play recompiles the edited source");
}

#[test]
fn test_set_velocity_x_then_y_in_one_hook_lands_as_one_write() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("launch.rhai"),
        r#"fn update(me, view, params, cmd, dt) {
            cmd.set_velocity_x("Ball", 250);
            cmd.set_velocity_y("Ball", -100);
        }"#,
    )
    .expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));

    let ball = world.create_entity();
    world.add_component(&ball, Name("Ball".to_string())).unwrap();
    world.add_component(&ball, Transform2D::new(Vec2::ZERO)).unwrap();
    let mut body = physics::RigidBody::new_dynamic();
    body.velocity = Vec2::new(5.0, 40.0);
    world.add_component(&ball, body).unwrap();

    let referee = world.create_entity();
    let mut script_ref = ScriptRef::new("launch");
    script_ref.source_path = "launch.rhai".to_string();
    world.add_component(&referee, Scripts(vec![script_ref])).unwrap();

    runner.early_update(&mut world, &input, &players, 1.0, None);
    runner.update(&mut world, &input, &players, 1.0, &[], None);

    let position = world.get::<Transform2D>(ball).unwrap().position;
    assert_eq!(position, Vec2::new(250.0, -100.0), "both axes land, and only once");
    assert!(runner.errors().is_empty(), "{:?}", runner.errors());
}
