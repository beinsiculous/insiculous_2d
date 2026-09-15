//! Headless tests for the clip bindings: `view.current_clip` /
//! `view.clip_finished` / `view.clip_state` and `cmd.play_clip` /
//! `cmd.ensure_clip` / `cmd.set_clip_state`, driven through a real `.rhai`
//! file.

use std::fs;

use tempfile::tempdir;

use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::sprite_components::{AnimationClip, SheetGrid, SpriteAnimation};
use ecs::{ClipState, ClipStateMachine, OnFinished};

use crate::scripting::runner::ScriptRunner;

use super::{test_inputs, test_world};

/// A 4-cell sheet: `walk` loops, `hit` is a one-shot.
fn animation() -> SpriteAnimation {
    SpriteAnimation::new(SheetGrid::new(4, 1))
        .with_clip("walk", AnimationClip::new(vec![0, 1], 10.0))
        .with_clip("hit", AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
}

/// One frame, in the order the engine really runs it: the script phase (whose
/// commands apply at its end), then the frame tail's systems.
fn frame(
    world: &mut ecs::World,
    runner: &mut ScriptRunner,
    input: &input::InputHandler,
    players: &input::InputSettings,
    delta_time: f32,
) {
    runner.early_update(world, input, players, delta_time, None);
    runner.update(world, input, players, delta_time, &[], None);
    crate::game::frame_tail::step_world_systems(world, delta_time);
}

/// An entity whose script is `clip.rhai`, with the clip name as a parameter.
fn scripted_entity(world: &mut ecs::World, clip: &str) -> ecs::EntityId {
    let entity = world.create_entity();
    let mut script_ref = ScriptRef::new("clip");
    script_ref.source_path = "clip.rhai".to_string();
    script_ref
        .params
        .insert("clip".to_string(), ScriptValue::Str(clip.to_string()));
    world.add_component(&entity, Scripts(vec![script_ref])).unwrap();
    entity
}

/// The `play_clip`/`ensure_clip` script, with the clip name as a parameter.
fn write_clip_script(dir: &tempfile::TempDir, command: &str) {
    fs::write(
        dir.path().join("clip.rhai"),
        format!(
            r#"// @param clip: str = "walk"
               fn update(me, view, params, cmd, dt) {{ cmd.{command}(me, params.clip); }}"#
        ),
    )
    .expect("wrote script");
}

fn clip_of(world: &ecs::World, entity: ecs::EntityId) -> Option<String> {
    world
        .get::<SpriteAnimation>(entity)
        .expect("animation")
        .current_clip
        .clone()
}

fn frame_of(world: &ecs::World, entity: ecs::EntityId) -> usize {
    world
        .get::<SpriteAnimation>(entity)
        .expect("animation")
        .current_frame
}

#[test]
fn test_play_clip_overrides_a_running_clip_for_a_looping_and_a_oneshot_one() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    write_clip_script(&dir, "play_clip");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));

    let walking = scripted_entity(&mut world, "walk");
    let hitting = scripted_entity(&mut world, "hit");
    for entity in [walking, hitting] {
        world.add_component(&entity, animation()).unwrap();
    }

    let delta_time = 0.1;
    frame(&mut world, &mut runner, &input, &players, delta_time);
    assert_eq!(clip_of(&world, walking).as_deref(), Some("walk"));
    assert_eq!(clip_of(&world, hitting).as_deref(), Some("hit"));

    // `play_clip` is a transition call, so re-asserting it each frame holds
    // the clip at its start — for a looping clip and a one-shot alike. The
    // hold lands one frame in, not on frame 0: the command applies at the end
    // of the script phase, and the tail's animation pass then advances the
    // freshly started clip by this frame's delta.
    for _ in 0..3 {
        frame(&mut world, &mut runner, &input, &players, delta_time);
    }
    assert_eq!(frame_of(&world, walking), 1);
    assert_eq!(frame_of(&world, hitting), 1);
    assert!(runner.errors().is_empty(), "{:?}", runner.errors());
}

#[test]
fn test_ensure_clip_from_a_script_advances_the_clip_instead_of_restarting_it() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    write_clip_script(&dir, "ensure_clip");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));
    let entity = scripted_entity(&mut world, "walk");
    world.add_component(&entity, animation()).unwrap();

    let delta_time = 0.1;
    for _ in 0..3 {
        frame(&mut world, &mut runner, &input, &players, delta_time);
    }

    assert_eq!(clip_of(&world, entity).as_deref(), Some("walk"));
    assert_eq!(frame_of(&world, entity), 1, "three 10fps steps over a 2-frame loop");
    assert!(runner.errors().is_empty(), "{:?}", runner.errors());
}

#[test]
fn test_view_reports_the_clip_the_finished_flag_and_the_machine_state() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("observe.rhai"),
        r#"fn update(me, view, params, cmd, dt) {
               cmd.set_blackboard_str("clip", view.current_clip(me));
               cmd.set_blackboard_bool("finished", view.clip_finished(me));
               cmd.set_blackboard_str("state", view.clip_state(me));
           }"#,
    )
    .expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));

    let entity = world.create_entity();
    let mut script_ref = ScriptRef::new("observe");
    script_ref.source_path = "observe.rhai".to_string();
    world.add_component(&entity, Scripts(vec![script_ref])).unwrap();
    world.add_component(&entity, animation()).unwrap();
    world
        .add_component(
            &entity,
            ClipStateMachine::new(
                "closing",
                vec![("closing".to_string(), ClipState::new("hit", OnFinished::Stay))],
            ),
        )
        .unwrap();

    let delta_time = 0.1;
    // The first frame's snapshot predates the machine selecting its clip (the
    // tail does that after the script phase), so the second frame is the one
    // that reads the clip and the state.
    frame(&mut world, &mut runner, &input, &players, delta_time);
    frame(&mut world, &mut runner, &input, &players, delta_time);
    let blackboard = world.resource::<Blackboard>().expect("blackboard").clone();
    assert_eq!(blackboard.get("clip"), Some(&ScriptValue::Str("hit".to_string())));
    assert_eq!(blackboard.get("state"), Some(&ScriptValue::Str("closing".to_string())));
    assert_eq!(
        blackboard.get("finished"),
        Some(&ScriptValue::Bool(false)),
        "the machine's clip is still running"
    );

    // The completion shows up on a later frame: the view is read before the
    // phase's commands apply, and the tail is what stops the clip.
    for _ in 0..4 {
        frame(&mut world, &mut runner, &input, &players, delta_time);
    }
    let blackboard = world.resource::<Blackboard>().expect("blackboard").clone();
    assert_eq!(
        blackboard.get("finished"),
        Some(&ScriptValue::Bool(true)),
        "a stopped one-shot reads as finished"
    );
    assert_eq!(
        blackboard.get("clip"),
        Some(&ScriptValue::Str("hit".to_string())),
        "a finished clip keeps its selection, so the state's clip is still the one shown"
    );
}

#[test]
fn test_play_clip_is_refused_on_a_machine_entity_and_the_machine_keeps_its_clip() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    write_clip_script(&dir, "play_clip");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));

    let door = scripted_entity(&mut world, "walk");
    world.add_component(&door, animation()).unwrap();
    world
        .add_component(
            &door,
            ClipStateMachine::new(
                "closed",
                vec![("closed".to_string(), ClipState::staying("hit"))],
            ),
        )
        .unwrap();

    let delta_time = 0.1;
    for _ in 0..3 {
        frame(&mut world, &mut runner, &input, &players, delta_time);
    }

    assert_eq!(
        clip_of(&world, door).as_deref(),
        Some("hit"),
        "the machine's state clip wins over the script's play_clip"
    );
    assert_eq!(
        world.get::<ClipStateMachine>(door).expect("machine").state(),
        "closed"
    );
    assert!(runner.errors().is_empty(), "the refusal is a warning, not an error");
}

#[test]
fn test_clip_commands_for_a_despawned_entity_never_panic_and_a_missing_name_is_the_documented_error() {
    let mut world = test_world();
    let (input, players) = test_inputs();
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("gone.rhai"),
        r#"fn update(me, view, params, cmd, dt) {
               cmd.despawn(me);
               cmd.set_clip_state(me, "open");
               cmd.play_clip(me, "walk");
           }"#,
    )
    .expect("wrote script");
    fs::write(
        dir.path().join("watch.rhai"),
        r#"fn update(me, view, params, cmd, dt) { cmd.set_clip_state("door", "open"); }"#,
    )
    .expect("wrote script");

    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, dir.path().to_str().expect("utf-8 path"));

    let door = world.create_entity();
    let mut door_script = ScriptRef::new("gone");
    door_script.source_path = "gone.rhai".to_string();
    world.add_component(&door, Scripts(vec![door_script])).unwrap();
    world.add_component(&door, animation()).unwrap();
    world
        .add_component(
            &door,
            ClipStateMachine::new(
                "closed",
                vec![
                    ("closed".to_string(), ClipState::staying("hit")),
                    ("open".to_string(), ClipState::staying("walk")),
                ],
            ),
        )
        .unwrap();
    world.add_component(&door, ecs::Name::new("door")).unwrap();

    let watcher = world.create_entity();
    let mut watcher_script = ScriptRef::new("watch");
    watcher_script.source_path = "watch.rhai".to_string();
    world.add_component(&watcher, Scripts(vec![watcher_script])).unwrap();

    let delta_time = 0.1;
    frame(&mut world, &mut runner, &input, &players, delta_time);

    // Despawns apply last: the state write landed on a live entity, the
    // despawn then removed it, and nothing panicked or came back.
    assert!(!world.entities().contains(&door), "the script's own despawn applied");
    assert!(world.get::<ClipStateMachine>(door).is_none());
    assert!(runner.errors().is_empty(), "{:?}", runner.errors());

    // The next frame the entity is gone and stays gone. The watcher's command
    // is the documented behaviour for a target that cannot be resolved: a
    // missing-target report, once, never a panic.
    frame(&mut world, &mut runner, &input, &players, delta_time);
    frame(&mut world, &mut runner, &input, &players, delta_time);
    assert!(!world.entities().contains(&door));
    let missing: Vec<_> = runner
        .errors()
        .iter()
        .filter(|error| error.message.contains("door"))
        .collect();
    assert_eq!(missing.len(), 1, "reported once: {:?}", runner.errors());
}
