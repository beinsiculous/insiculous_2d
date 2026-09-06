use common::Transform2D;
use ecs::blackboard::Blackboard;
use ecs::script::ScriptValue;
use ecs::ui_components::UiLabel;
use ecs::World;
use engine_core::prelude::*;
use engine_core::test_support::{frame, StubResolver};
use engine_core::ScriptRunner;
use input::{InputEvent, InputHandler};

// These tests run with `physics: None`, so a script's `me.velocity` is always zero: the
// no-physics fallback integrates `set_velocity` into the transform and never writes the
// body's velocity. ball.rhai's speed-maintenance branch (`update` past the `< 0.1` guard)
// and paddle.rhai's AI chase (`diff.abs() > dead_zone`) therefore never execute here; their
// verbs are held by reading the Rhai and engine registries, and by Play in the browser.

#[test]
fn test_pong_rules_headless() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let asset_base = format!("{manifest_dir}/assets/projects/pong/assets");
    let scene_path = format!("{asset_base}/scenes/pong.scene.ron");
    let scene_ron = common::vfs::read_to_string(std::path::Path::new(&scene_path))
        .expect("pong.scene.ron must exist");

    let parsed = SceneLoader::parse(&scene_ron).expect("pong scene parses");
    let mut world = World::new();
    let mut resolver = StubResolver::default();
    let instance = SceneLoader::instantiate(&parsed, &mut world, &mut resolver)
        .expect("pong scene instantiates");

    let ball = instance.named_entities["Ball"];
    let left_goal = instance.named_entities["Left Goal"];
    let scoreboard = instance.named_entities["Scoreboard"];

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, &asset_base);

    let mut input = InputHandler::new();
    let players = InputSettings::default();

    // 1 frame with no input
    runner.early_update(&mut world, &input, &players, 1.0 / 60.0, None);
    runner.update(&mut world, &input, &players, 1.0 / 60.0, &[], None);

    let ball_pos = world
        .get::<Transform2D>(ball)
        .expect("ball has transform")
        .position;
    assert_eq!(ball_pos, Vec2::ZERO);
    assert!(runner.errors().is_empty(), "unexpected errors: {:?}", runner.errors());

    // Space press (player 0 Action1)
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Space)]);
    runner.early_update(&mut world, &input, &players, 1.0 / 60.0, None);

    let ball_pos_after_serve = world
        .get::<Transform2D>(ball)
        .expect("ball has transform")
        .position;
    assert_ne!(ball_pos_after_serve, Vec2::ZERO);

    let bb = world.resource::<Blackboard>().expect("blackboard exists");
    assert_eq!(bb.get("serving"), Some(&ScriptValue::Bool(false)));

    // Inject left_goal collision
    let collision = CollisionData {
        event: CollisionEvent {
            entity_a: ball,
            entity_b: left_goal,
            started: true,
            stopped: false,
        },
        contacts: vec![],
    };

    // Release space so no stray input
    frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Space)]);
    runner.update(&mut world, &input, &players, 1.0 / 60.0, &[collision], None);

    let bb = world.resource::<Blackboard>().expect("blackboard exists");
    assert_eq!(bb.get("right_score"), Some(&ScriptValue::I32(1)));
    assert_eq!(bb.get("last_scorer"), Some(&ScriptValue::Str("right".to_string())));
    assert_eq!(bb.get("serving"), Some(&ScriptValue::Bool(true)));

    let ball_pos_reset = world
        .get::<Transform2D>(ball)
        .expect("ball has transform")
        .position;
    assert_eq!(ball_pos_reset, Vec2::ZERO);

    // One more update to let scoreboard observe the blackboard update
    runner.update(&mut world, &input, &players, 1.0 / 60.0, &[], None);
    let label = world
        .get::<UiLabel>(scoreboard)
        .expect("scoreboard has UiLabel");
    assert_eq!(label.text, "0 : 1");
    assert!(runner.errors().is_empty(), "unexpected errors: {:?}", runner.errors());
}

#[test]
fn test_pong_game_over_and_restart() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let asset_base = format!("{manifest_dir}/assets/projects/pong/assets");
    let scene_path = format!("{asset_base}/scenes/pong.scene.ron");
    let scene_ron = common::vfs::read_to_string(std::path::Path::new(&scene_path))
        .expect("pong.scene.ron must exist");

    let parsed = SceneLoader::parse(&scene_ron).expect("pong scene parses");
    let mut world = World::new();
    let mut resolver = StubResolver::default();
    let instance = SceneLoader::instantiate(&parsed, &mut world, &mut resolver)
        .expect("pong scene instantiates");

    let ball = instance.named_entities["Ball"];
    let right_goal = instance.named_entities["Right Goal"];
    let scoreboard = instance.named_entities["Scoreboard"];

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, &asset_base);

    // Set left_score to 6 on blackboard
    world
        .resource_mut::<Blackboard>()
        .expect("blackboard exists")
        .set("left_score", ScriptValue::I32(6));

    let mut input = InputHandler::new();
    let players = InputSettings::default();

    // Inject right-goal collision
    let collision = CollisionData {
        event: CollisionEvent {
            entity_a: ball,
            entity_b: right_goal,
            started: true,
            stopped: false,
        },
        contacts: vec![],
    };

    runner.update(&mut world, &input, &players, 1.0 / 60.0, &[collision], None);

    // One more no-input update for scoreboard to observe left_score == 7
    runner.update(&mut world, &input, &players, 1.0 / 60.0, &[], None);

    let label = world
        .get::<UiLabel>(scoreboard)
        .expect("scoreboard has UiLabel");
    assert!(
        label.text.starts_with("LEFT WINS"),
        "label expected to start with 'LEFT WINS', got: {}",
        label.text
    );

    let bb = world.resource::<Blackboard>().expect("blackboard exists");
    assert_eq!(bb.get("game_over"), Some(&ScriptValue::Bool(true)));

    // Press Space, run both phases
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Space)]);
    runner.early_update(&mut world, &input, &players, 1.0 / 60.0, None);
    runner.update(&mut world, &input, &players, 1.0 / 60.0, &[], None);

    let ball_pos = world
        .get::<Transform2D>(ball)
        .expect("ball has transform")
        .position;
    assert_eq!(ball_pos, Vec2::ZERO, "ball should not leave center on restart press");

    let bb = world.resource::<Blackboard>().expect("blackboard exists");
    assert_eq!(bb.get("left_score"), Some(&ScriptValue::I32(0)));
    assert_eq!(bb.get("right_score"), Some(&ScriptValue::I32(0)));
    assert_eq!(bb.get("game_over"), Some(&ScriptValue::Bool(false)));
    assert_eq!(bb.get("serving"), Some(&ScriptValue::Bool(true)));

    assert!(runner.errors().is_empty(), "unexpected errors: {:?}", runner.errors());
}
