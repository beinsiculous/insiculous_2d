//! Stage B integration tests: request lines through the REAL EditorGame —
//! writes land on its CommandHistory, hosted creates use the entity
//! factories, save routes through the choke point, and the ship point
//! holds: a text script builds a scene and every step is undoable in the
//! GUI exactly as if clicked.

use engine_core::contexts::GameContext;
use engine_core::Game;

use editor::PlayControlAction;

use super::EditorGame;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

fn run(editor: &mut EditorGame<DummyGame>, world: &mut ecs::World, line: &str) -> String {
    let responses = editor.answer_api_lines(&[line.to_string()], world, &|_| "#white".into());
    responses.into_iter().next().expect("one response per line")
}

fn assert_ok(response: &str) {
    assert!(response.contains("\"ok\":true"), "expected ok: {response}");
}

#[test]
fn test_api_script_builds_scene_and_gui_undo_reverts_each_step() {
    // THE ship point (audit §9 Stage B): a script builds a scene; each step
    // is one entry on the same CommandHistory the GUI uses, so N undos
    // return the world to empty.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();

    assert_ok(&run(&mut editor, &mut world, "create sprite Hero 100 50"));
    assert_ok(&run(&mut editor, &mut world, r#"set Hero Transform2D {"rotation": 1.0}"#));
    assert_ok(&run(&mut editor, &mut world, "create static-body Floor 0 -40"));
    assert_ok(&run(&mut editor, &mut world, "rename Floor Ground"));

    assert_eq!(world.entities().len(), 2);
    let hero = match editor::HierarchyPanel::resolve_by_name(&world, "Hero") {
        editor::NameResolution::One(e) => e,
        other => panic!("Hero must resolve uniquely, got {other:?}"),
    };
    assert_eq!(world.get::<common::Transform2D>(hero).unwrap().rotation, 1.0);
    assert!(matches!(
        editor::HierarchyPanel::resolve_by_name(&world, "Ground"),
        editor::NameResolution::One(_)
    ));

    // Four script steps = four undo entries, exactly as if clicked.
    for step in (1..=4).rev() {
        assert!(
            editor.command_history.undo(&mut world),
            "undo step {step} must succeed"
        );
    }
    assert!(!editor.command_history.can_undo(), "no extra entries");
    assert!(world.entities().is_empty(), "the script fully reverts to an empty world");
}

#[test]
fn test_api_create_archetypes_all_map_to_factories() {
    // Drift lock: every advertised archetype spawns something.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    for archetype in editor::command_api::ARCHETYPES {
        let response = run(&mut editor, &mut world, &format!("create {archetype}"));
        assert_ok(&response);
    }
    assert_eq!(world.entities().len(), editor::command_api::ARCHETYPES.len());
}

#[test]
fn test_api_save_writes_file_and_clears_dirty() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    assert_ok(&run(&mut editor, &mut world, "create empty Marker"));
    assert!(editor.command_history.is_dirty());

    let path = std::env::temp_dir().join("api_write_save_test.scene.ron");
    let response = run(&mut editor, &mut world, &format!("save {}", path.display()));
    assert_ok(&response);
    assert!(path.exists(), "save must write the file");
    assert!(!editor.command_history.is_dirty(), "save marks the watermark");
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_api_save_refused_mid_play_session_and_with_open_batch() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    assert_ok(&run(&mut editor, &mut world, "create empty Marker"));

    // Open batch: save refused.
    assert_ok(&run(&mut editor, &mut world, "batch begin b"));
    let refused = run(&mut editor, &mut world, "save /tmp/nope.scene.ron");
    assert!(refused.contains("\"refused\""), "{refused}");
    assert_ok(&run(&mut editor, &mut world, "batch abort"));

    // Play session (Paused counts): the choke point refuses.
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    let refused = run(&mut editor, &mut world, "save /tmp/nope.scene.ron");
    assert!(refused.contains("\"refused\""), "{refused}");
}

#[test]
fn test_play_start_commits_open_batch_as_one_entry() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();

    assert_ok(&run(&mut editor, &mut world, "batch begin setup"));
    assert_ok(&run(&mut editor, &mut world, "create empty A"));
    assert_ok(&run(&mut editor, &mut world, "create empty B"));
    assert!(!editor.command_history.can_undo(), "batched commands are held aside");

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.api_batch.is_none(), "Play commits the open batch");
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    assert!(editor.command_history.undo(&mut world), "the batch is one undo entry");
    assert!(world.entities().is_empty());
    assert!(!editor.command_history.can_undo());
}

#[test]
fn test_api_writes_refused_while_playing_through_the_full_path() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    assert_ok(&run(&mut editor, &mut world, "create empty Marker"));
    editor.handle_play_action(PlayControlAction::Play, &mut world);

    let refused = run(&mut editor, &mut world, "create empty Nope");
    assert!(refused.contains("\"refused\""), "{refused}");
    let refused = run(&mut editor, &mut world, "rename Marker Renamed");
    assert!(refused.contains("\"refused\""), "{refused}");

    // Queries still work while Playing.
    let ok = run(&mut editor, &mut world, "list");
    assert_ok(&ok);
}

#[test]
fn test_commands_query_lists_verbs_and_vocabularies() {
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let response = run(&mut editor, &mut world, "commands");
    assert_ok(&response);
    assert!(response.contains("\"archetypes\""));
    assert!(response.contains("\"settable\""));
    assert!(response.contains("Transform2D"));
    assert!(response.contains("batch begin"));
}

#[test]
fn test_create_with_empty_name_is_rejected_without_spawning() {
    // Kimi F4: the guard runs BEFORE the factory — a rejection must not
    // leak a half-created entity.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    let response = run(&mut editor, &mut world, r#"create sprite """#);
    assert!(response.contains("\"invalid\""), "{response}");
    assert!(world.entities().is_empty(), "no entity may be spawned on rejection");
    assert!(!editor.command_history.can_undo());
}

#[test]
fn test_stop_discards_batch_opened_while_paused() {
    // Kimi F2: a batch opened while Paused holds commands referencing the
    // mid-simulation world; Stop restores the snapshot and must drop the
    // batch with it, or a later `batch end` pushes a macro that undoes
    // against the wrong world.
    let mut editor = EditorGame::new(DummyGame);
    let mut world = ecs::World::new();
    assert_ok(&run(&mut editor, &mut world, "create empty Marker"));

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert_ok(&run(&mut editor, &mut world, "batch begin paused-edits"));
    assert_ok(&run(&mut editor, &mut world, "create empty Ghost"));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(editor.api_batch.is_none(), "Stop drops the stale batch");
    assert_eq!(world.entities().len(), 1, "snapshot restore discarded the paused-world entity");

    let refused = run(&mut editor, &mut world, "batch end");
    assert!(refused.contains("\"refused\""), "no batch remains open: {refused}");
    assert!(!editor.command_history.can_undo() || editor.command_history.undo_name() != Some("paused-edits"),
        "the stale macro never reaches the history");
}
