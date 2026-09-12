//! The live scene handed to the preview: the visitor's unsaved edits reach
//! the RON, the editor's own file, history and dirty mark do not move, and
//! the request mailbox never lets one generation answer another.

use std::path::PathBuf;

use ecs::World;
use editor::PlayControlAction;
use engine_core::test_support::{test_texture_path, StubResolver};
use glam::Vec2;

use super::snapshot::{SceneSnapshot, SceneSnapshotRequest};
use super::test_support::{editor_game, DummyGame};
use super::EditorGame;

const SCENE: &str = r#"SceneData(
    name: "live",
    entities: [
        EntityData(name: Some("player"), components: [Transform2D(position: (1.0, 2.0))]),
    ],
)"#;

/// An editor holding `SCENE`, loaded through the real load path from a
/// project tree whose asset base the session knows.
fn loaded_session(
    directory: &std::path::Path,
    world: &mut World,
) -> std::io::Result<(EditorGame<DummyGame>, PathBuf)> {
    let scenes = directory.join("assets").join("scenes");
    std::fs::create_dir_all(&scenes)?;
    let path = scenes.join("main.scene.ron");
    std::fs::write(&path, SCENE)?;
    let mut editor = editor_game();
    editor.asset_base = directory.join("assets");
    editor
        .load_scene(world, &mut StubResolver::default(), &path)
        .expect("the scene loads");
    Ok((editor, path))
}

/// The entity the loaded scene names `player`.
fn player(world: &World) -> ecs::EntityId {
    world
        .entities()
        .into_iter()
        .find(|entity| {
            world
                .get::<ecs::Name>(*entity)
                .map(|name| name.0 == "player")
                .unwrap_or(false)
        })
        .expect("the scene names an entity 'player'")
}

#[test]
fn test_scene_snapshot_serializes_the_live_world_with_unsaved_edits_and_leaves_file_history_and_path_alone(
) -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    let mut world = World::new();
    let (mut editor, path) = loaded_session(directory.path(), &mut world)?;
    let bytes_before = std::fs::read(&path)?;

    let target = player(&world);
    editor.command_history.execute(
        Box::new(editor::commands::SetComponentCommand::<common::Transform2D>::new(
            target,
            common::Transform2D::new(Vec2::new(1.0, 2.0)),
            common::Transform2D::new(Vec2::new(77.0, 88.0)),
            "position",
        )),
        &mut world,
    );
    assert!(editor.command_history.is_dirty(), "fixture: the edit is unsaved");
    let top_of_history = editor.command_history.undo_name().map(str::to_string);

    let snapshot = editor
        .scene_snapshot(&world, &test_texture_path)
        .expect("the live world serializes");

    assert!(snapshot.ron.contains("77"), "the unsaved edit reached the RON: {}", snapshot.ron);
    assert_eq!(snapshot.scene_entry, "assets/scenes/main.scene.ron");
    assert_eq!(std::fs::read(&path)?, bytes_before, "the scene file is untouched");
    assert!(editor.command_history.is_dirty(), "the dirty mark survives a snapshot");
    assert_eq!(
        editor.command_history.undo_name().map(str::to_string),
        top_of_history,
        "the snapshot recorded no history entry of its own"
    );
    assert_eq!(editor.editor.scene_path(), Some(path.as_path()));
    Ok(())
}

#[test]
fn test_scene_snapshot_names_unnamed_script_targets_in_the_scratch_world_and_keeps_their_parameters(
) -> std::io::Result<()> {
    use ecs::script::{ScriptRef, ScriptValue, Scripts};

    let directory = tempfile::tempdir()?;
    let mut world = World::new();
    let (mut editor, _) = loaded_session(directory.path(), &mut world)?;

    let target = world.create_entity();
    world.add_component(&target, common::Transform2D::new(Vec2::new(5.0, 6.0))).ok();
    let owner = player(&world);
    let mut script = ScriptRef::new("chase");
    script.params.insert("target".to_string(), ScriptValue::Entity(target));
    world.add_component(&owner, Scripts(vec![script])).ok();

    let snapshot = editor
        .scene_snapshot(&world, &test_texture_path)
        .expect("the live world serializes");

    assert!(
        snapshot.ron.contains("script_target_"),
        "the unnamed target was named in the scratch world: {}",
        snapshot.ron
    );
    assert!(
        world.get::<ecs::Name>(target).is_none(),
        "the LIVE world keeps its unnamed entity — the naming happened on the scratch"
    );

    // The binding survives the round trip and resolves to the entity the
    // author pointed at, which is what the naming exists to protect.
    let reloaded = engine_core::scene_loader::SceneLoader::parse(&snapshot.ron)
        .expect("the snapshot parses back");
    let mut restored = World::new();
    engine_core::scene_loader::SceneLoader::instantiate(
        &reloaded,
        &mut restored,
        &mut StubResolver::default(),
    )
    .expect("the snapshot instantiates");
    let restored_owner = restored
        .entities()
        .into_iter()
        .find(|entity| restored.get::<Scripts>(*entity).is_some())
        .expect("the reloaded scene carries the script");
    let restored_target = restored
        .get::<Scripts>(restored_owner)
        .and_then(|scripts| scripts.0.first().and_then(|script| script.params.get("target").cloned()))
        .expect("the parameter survived");
    let ScriptValue::Entity(resolved) = restored_target else {
        panic!("the parameter is still an entity reference");
    };
    assert_eq!(
        restored.get::<common::Transform2D>(resolved).map(|transform| transform.position),
        Some(Vec2::new(5.0, 6.0)),
        "the binding resolves to the entity it was authored against"
    );
    Ok(())
}

#[test]
fn test_scene_snapshot_is_refused_mid_session() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    let mut world = World::new();
    let (mut editor, _) = loaded_session(directory.path(), &mut world)?;

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing(), "fixture: the session is Playing");
    assert!(
        editor.scene_snapshot(&world, &test_texture_path).is_err(),
        "a Playing world is mid-simulation"
    );

    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.editor.is_paused(), "fixture: the session is Paused");
    assert!(
        editor.scene_snapshot(&world, &test_texture_path).is_err(),
        "a Paused world is mid-simulation too"
    );
    Ok(())
}

#[test]
fn test_play_is_refused_while_a_preview_window_is_open() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    let mut world = World::new();
    let (mut editor, _) = loaded_session(directory.path(), &mut world)?;
    let preview_open = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    editor.preview_open = Some(preview_open.clone());

    editor.handle_play_action(PlayControlAction::Play, &mut world);

    assert!(editor.editor.is_editing(), "Play is refused while the preview holds the scene");
    assert_eq!(
        editor.editor.status_bar.message(),
        Some("A preview window is open — close it to Play here")
    );

    preview_open.store(false, std::sync::atomic::Ordering::Relaxed);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing(), "Play works once the preview released the scene");
    Ok(())
}

#[test]
fn test_snapshot_request_refuses_a_second_generation_while_one_is_pending_and_drops_a_cancelled_answer(
) {
    let request = SceneSnapshotRequest::default();

    assert!(request.file(1), "the first generation is accepted");
    assert!(!request.file(2), "a second generation is refused while one is pending");
    let first = request.subscribe(1).expect("the filer subscribes to its own generation");

    request.cancel(1);
    assert!(first.is_settled(), "a cancelled generation settles for everyone waiting");
    assert!(first.outcome().expect("a cancellation is an outcome").is_err());

    // The editor answers the generation it took before the cancel landed.
    request.answer(
        1,
        Ok(SceneSnapshot { scene_entry: "assets/scenes/a.scene.ron".to_string(), ron: String::new() }),
    );

    assert!(request.file(2), "the cancel freed the slot");
    let second = request.subscribe(2).expect("the new generation is subscribable");
    assert!(!second.is_settled(), "the late answer to generation 1 never resolved generation 2");

    request.answer(
        2,
        Ok(SceneSnapshot { scene_entry: "assets/scenes/b.scene.ron".to_string(), ron: String::new() }),
    );
    let answered = second.outcome().expect("generation 2 has its own answer").expect("it succeeded");
    assert_eq!(answered.scene_entry, "assets/scenes/b.scene.ron");
}

#[test]
fn test_every_subscriber_of_one_generation_reads_its_answer_and_a_later_filing_does_not_erase_it() {
    // A launch files, an Export joins it a moment later: both must read the
    // one answer the editor produces, whichever polls first.
    let request = SceneSnapshotRequest::default();
    assert!(request.file(7));
    let launch = request.subscribe(7).expect("the launch subscribes");
    let export = request.subscribe(7).expect("the export joins the same generation");
    assert_eq!(request.pending_generation(), Some(7), "the generation is owed until it settles");

    request.answer(
        7,
        Ok(SceneSnapshot { scene_entry: "assets/scenes/a.scene.ron".to_string(), ron: "live".to_string() }),
    );

    let first_read = launch.outcome().expect("the launch reads the answer").expect("it succeeded");
    let second_read = export.outcome().expect("the export reads the same answer").expect("it succeeded");
    assert_eq!(first_read, second_read);
    assert_eq!(request.pending_generation(), None, "a settled generation is no longer pending");

    assert!(request.file(8), "the slot is free for the next generation");
    assert_eq!(
        export.outcome().map(|outcome| outcome.map(|snapshot| snapshot.ron)),
        Some(Ok("live".to_string())),
        "filing the next generation does not erase a result a subscriber still holds"
    );
}

#[test]
fn test_snapshot_refuses_a_scene_outside_the_asset_base() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    let mut world = World::new();
    let (mut editor, _) = loaded_session(directory.path(), &mut world)?;
    editor.asset_base = directory.path().join("somewhere_else");

    let error = editor
        .scene_snapshot(&world, &test_texture_path)
        .expect_err("a scene outside the base has no archive entry");
    assert!(error.to_string().contains("outside the project"), "{error}");
    Ok(())
}
