//! A script reloaded mid-session: the new source runs from the next frame with the session's
//! state kept, a broken one leaves the old running and comes back to its caller every time,
//! and a header error names its script.

use std::fs;
use std::path::Path;

use glam::Vec2;
use tempfile::tempdir;

use common::Transform2D;
use ecs::blackboard::Blackboard;
use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::{EntityId, World};

use crate::contexts::GameContext;
use crate::game::Game;
use crate::game_config::GameConfig;
use crate::scripting::runner::{ScriptErrorKind, ScriptRunner};
use crate::test_support::GameHarness;

use super::{test_inputs, test_world};

const SCRIPT: &str = "scripts/mover.rhai";

/// A mover that stands at `x` and counts its frames on the blackboard.
fn counting_mover(x: f32) -> String {
    format!(
        "fn update(me, view, params, cmd, dt) {{\n    cmd.set_position(me, vec2({x:.1}, 0.0));\n    \
         cmd.set_blackboard_int(\"count\", view.blackboard_int(\"count\", 0) + 1);\n}}\n"
    )
}

/// A mover that stands at `x`.
fn mover(x: f32) -> String {
    format!("fn update(me, view, params, cmd, dt) {{\n    cmd.set_position(me, vec2({x:.1}, 0.0));\n}}\n")
}

/// A script whose second line does not compile.
const BROKEN: &str = "fn update(me, view, params, cmd, dt) {\n    let x = ;\n}\n";

fn write_script(project: &Path, source: &str) {
    let scripts = project.join("scripts");
    fs::create_dir_all(&scripts).expect("the scripts folder");
    fs::write(project.join(SCRIPT), source).expect("the script");
}

fn spawn_mover(world: &mut World, script: ScriptRef) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, Scripts(vec![script])).ok();
    entity
}

fn mover_ref() -> ScriptRef {
    let mut script = ScriptRef::new("mover");
    script.source_path = SCRIPT.to_string();
    script
}

fn position(world: &World, entity: EntityId) -> Vec2 {
    world.get::<Transform2D>(entity).expect("the mover's transform").position
}

/// One frame of the runner over `world`, both phases.
fn step(runner: &mut ScriptRunner, world: &mut World) {
    let (input, players) = test_inputs();
    runner.early_update(world, &input, &players, 1.0 / 60.0, None);
    runner.update(world, &input, &players, 1.0 / 60.0, &[], None);
}

/// A session over a fresh project holding `source`, with one mover on it.
fn session(source: &str) -> (tempfile::TempDir, ScriptRunner, World, EntityId) {
    let project = tempdir().expect("a temporary project");
    write_script(project.path(), source);
    let mut world = test_world();
    let mut runner = ScriptRunner::new();
    runner.reset(&mut world, &project.path().to_string_lossy());
    let entity = spawn_mover(&mut world, mover_ref());
    (project, runner, world, entity)
}

/// A game that resets the runner at `init`, as a session starts, and steps it every frame.
struct ScriptedGame {
    mover: Option<EntityId>,
}

impl Game for ScriptedGame {
    fn init(&mut self, ctx: &mut GameContext) {
        let asset_base = ctx.assets.base_path().to_string();
        ctx.scripts.reset(ctx.world, &asset_base);
        self.mover = Some(spawn_mover(ctx.world, mover_ref()));
    }

    fn update(&mut self, ctx: &mut GameContext) {
        ctx.scripts.early_update(ctx.world, ctx.input, ctx.players, ctx.delta_time, None);
        ctx.scripts.update(ctx.world, ctx.input, ctx.players, ctx.delta_time, &[], None);
    }
}

#[test]
fn a_reloaded_constant_moves_the_entity_next_frame_and_the_blackboard_and_frame_count_survive() {
    let project = tempdir().expect("a temporary project");
    write_script(project.path(), &counting_mover(7.0));
    let config = GameConfig::new("reload").with_asset_base_path(project.path().to_string_lossy());
    let mut harness = GameHarness::new(ScriptedGame { mover: None }, config);
    harness.steps(2, 1.0 / 60.0);
    let mover = harness.game().mover.expect("init spawned the mover");
    assert_eq!(position(harness.world(), mover).x, 7.0);

    write_script(project.path(), &counting_mover(9.0));
    let reloaded = harness.context(|_, ctx| ctx.scripts.reload_source(SCRIPT));
    assert_eq!(reloaded, Ok(()));
    harness.step(1.0 / 60.0, &[]);

    assert_eq!(position(harness.world(), mover).x, 9.0, "the new constant runs");
    let count = harness.world().resource::<Blackboard>().and_then(|board| board.get("count").cloned());
    assert_eq!(count, Some(ScriptValue::I32(3)), "the blackboard counts on across the reload");
    assert_eq!(harness.context(|_, ctx| ctx.scripts.frames_run()), 3);
}

#[test]
fn a_changed_param_default_takes_effect_and_a_value_the_component_set_stays() {
    let header = |offset: f32, scale: f32| {
        format!(
            "// @param offset: f32 = {offset:.1}\n// @param scale: f32 = {scale:.1}\n\
             fn update(me, view, params, cmd, dt) {{\n    cmd.set_position(me, vec2(params.offset, params.scale));\n}}\n"
        )
    };
    let (project, mut runner, mut world, _) = session(&header(1.0, 2.0));
    let mut script = mover_ref();
    script.params.insert("scale".to_string(), ScriptValue::F32(5.0));
    let entity = spawn_mover(&mut world, script);
    step(&mut runner, &mut world);
    assert_eq!(position(&world, entity), Vec2::new(1.0, 5.0));

    write_script(project.path(), &header(3.0, 4.0));
    runner.reload_source(SCRIPT).expect("the new header compiles");
    step(&mut runner, &mut world);

    assert_eq!(position(&world, entity), Vec2::new(3.0, 5.0), "the new default, and the scene's own value");
}

#[test]
fn a_syntax_error_keeps_the_old_source_running_and_returns_the_error_with_its_line() {
    let (project, mut runner, mut world, entity) = session(&mover(7.0));
    step(&mut runner, &mut world);

    write_script(project.path(), BROKEN);
    let error = runner.reload_source(SCRIPT).expect_err("the broken source does not compile");
    assert_eq!((error.file.as_str(), error.line, &error.kind), (SCRIPT, 2, &ScriptErrorKind::Syntax));

    world.get_mut::<Transform2D>(entity).expect("the transform").position = Vec2::ZERO;
    step(&mut runner, &mut world);
    assert_eq!(position(&world, entity).x, 7.0, "the old unit still runs");
    assert!(runner.errors().is_empty(), "the reload's error is the caller's to report");
}

#[test]
fn a_good_save_after_a_broken_one_clears_the_quarantine() {
    let (project, mut runner, mut world, entity) = session("fn update(me, view, params, cmd, dt) {\n    loop {}\n}\n");
    step(&mut runner, &mut world);
    assert_eq!(runner.quarantined_count(), 1, "the runaway loop is quarantined");

    write_script(project.path(), BROKEN);
    assert!(runner.reload_source(SCRIPT).is_err());
    assert_eq!(runner.quarantined_count(), 1, "a broken save changes nothing");

    write_script(project.path(), &mover(7.0));
    runner.reload_source(SCRIPT).expect("the fix compiles");
    assert_eq!(runner.quarantined_count(), 0);
    step(&mut runner, &mut world);
    assert_eq!(position(&world, entity).x, 7.0, "the fixed script runs");
}

#[test]
fn the_same_syntax_error_after_a_fix_is_returned_again() {
    let (project, mut runner, _world, _) = session(&mover(7.0));

    write_script(project.path(), BROKEN);
    let first = runner.reload_source(SCRIPT).expect_err("broken");
    write_script(project.path(), &mover(8.0));
    runner.reload_source(SCRIPT).expect("fixed");
    write_script(project.path(), BROKEN);
    let again = runner.reload_source(SCRIPT).expect_err("broken again");

    assert_eq!(again, first, "the dedup never swallows a reload's error");
}

#[test]
fn a_runtime_error_on_the_first_frame_after_a_reload_is_reported_fresh_says_so_and_quarantines() {
    let throwing = |text: &str| format!("fn update(me, view, params, cmd, dt) {{\n    throw \"{text}\";\n}}\n");
    let (project, mut runner, mut world, _) = session(&throwing("old"));
    step(&mut runner, &mut world);
    step(&mut runner, &mut world);
    assert_eq!(runner.errors().len(), 1, "an error every frame is recorded once");
    assert_eq!(runner.quarantined_count(), 0, "an ordinary runtime error runs again next frame");

    write_script(project.path(), &throwing("new"));
    runner.reload_source(SCRIPT).expect("the new source compiles");
    step(&mut runner, &mut world);

    let errors = runner.errors();
    assert_eq!(errors.len(), 2, "the same line's error after a reload is new");
    let message = &errors[1].message;
    assert!(message.contains("new"), "{message}");
    assert!(
        message.ends_with("reloaded script failed on its first frame — Stop and Play to reset its state"),
        "{message}"
    );
    assert_eq!(runner.quarantined_count(), 1);
}

#[test]
fn a_header_error_names_its_script_and_line() {
    let source = "// a mover\n\n// @param speed f32 = 1.0\nfn update(me, view, params, cmd, dt) {}\n";
    let (_project, mut runner, mut world, _) = session(source);
    step(&mut runner, &mut world);

    let error = runner.errors().first().cloned().expect("the header error");
    assert_eq!((error.file.as_str(), error.line, &error.kind), (SCRIPT, 3, &ScriptErrorKind::Header));
    assert!(error.to_string().starts_with("scripts/mover.rhai:3: [Header]"), "{error}");
}

#[test]
fn a_session_started_after_an_edit_compiles_the_new_source_without_a_reload() {
    let (project, mut runner, mut world, entity) = session(&mover(7.0));
    step(&mut runner, &mut world);

    write_script(project.path(), &mover(9.0));
    runner.reset(&mut world, &project.path().to_string_lossy());
    step(&mut runner, &mut world);

    assert_eq!(position(&world, entity).x, 9.0);
}
