//! The command API through the REAL `EditorGame` (Stage B): every request
//! line owes one response from live state, writes land on the same
//! `CommandHistory` the GUI uses, hosted creates use the entity factories,
//! saves route through the choke point, and batches follow the play
//! session (committed by Play, discarded by Stop).

use ecs::World;
use editor::PlayControlAction;

use super::test_support::{api_line, assert_ok, editor_game};

#[test]
fn test_answer_api_lines_answers_each_non_blank_line_in_order_from_live_state() {
    let mut editor = editor_game();
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, ecs::Name::new("Player")).ok();
    let resolver = |_: u32| Some("#white".to_string());
    let lines = ["scene", "", "describe Player", "frobnicate"].map(str::to_string);

    let responses = editor.answer_api_lines(&lines, &mut world, &resolver);

    assert_eq!(responses.len(), 3, "a blank line owes no response; every other line owes one");
    assert!(responses[0].contains("\"dirty\":false"), "scene reads the live history: {}", responses[0]);
    assert_ok(&responses[1]);
    assert!(responses[2].contains("\"ok\":false"), "an unknown verb is one error line: {}", responses[2]);

    // A recorded command flips what the next `scene` reports.
    editor.command_history.push_already_executed(Box::new(
        editor::commands::CreateEntityCommand::already_created(&world, entity),
    ));
    let dirty = api_line(&mut editor, &mut world, "scene");
    assert!(dirty.contains("\"dirty\":true"), "{dirty}");
}

#[test]
fn test_api_script_builds_scene_and_gui_undo_reverts_each_step() {
    // A script builds a scene; each step
    // is one entry on the same CommandHistory the GUI uses, so N undos
    // return the world to empty.
    let mut editor = editor_game();
    let mut world = World::new();

    assert_ok(&api_line(&mut editor, &mut world, "create sprite Hero 100 50"));
    assert_ok(&api_line(&mut editor, &mut world, r#"set Hero Transform2D {"rotation": 1.0}"#));
    assert_ok(&api_line(&mut editor, &mut world, "create static-body Floor 0 -40"));
    assert_ok(&api_line(&mut editor, &mut world, "rename Floor Ground"));

    assert_eq!(world.entities().len(), 2);
    let hero = match editor::HierarchyPanel::resolve_by_name(&world, "Hero") {
        editor::NameResolution::One(e) => e,
        other => panic!("Hero must resolve uniquely, got {other:?}"),
    };
    assert_eq!(world.get::<common::Transform2D>(hero).map(|t| t.rotation), Some(1.0));
    assert!(matches!(
        editor::HierarchyPanel::resolve_by_name(&world, "Ground"),
        editor::NameResolution::One(_)
    ));

    // Four script steps = four undo entries, exactly as if clicked.
    for step in (1..=4).rev() {
        assert!(editor.command_history.undo(&mut world), "undo step {step} must succeed");
    }
    assert!(!editor.command_history.can_undo(), "no extra entries");
    assert!(world.entities().is_empty(), "the script fully reverts to an empty world");
}

#[test]
fn test_api_create_archetypes_all_map_to_factories() {
    // Drift lock over the nine entity factories: every advertised
    // archetype spawns something.
    let mut editor = editor_game();
    let mut world = World::new();

    for archetype in editor::command_api::ARCHETYPES {
        let response = api_line(&mut editor, &mut world, &format!("create {archetype}"));
        assert_ok(&response);
    }

    assert_eq!(world.entities().len(), editor::command_api::ARCHETYPES.len());
}

#[test]
fn test_create_with_empty_name_is_rejected_without_spawning() {
    // The guard runs BEFORE the factory — a rejection must not
    // leak a half-created entity.
    let mut editor = editor_game();
    let mut world = World::new();

    let response = api_line(&mut editor, &mut world, r#"create sprite """#);

    assert!(response.contains("\"invalid\""), "{response}");
    assert!(world.entities().is_empty(), "no entity may be spawned on rejection");
    assert!(!editor.command_history.can_undo());
}

#[test]
fn test_api_save_routes_through_the_choke_point() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut editor = editor_game();
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "create empty Marker"));
    assert!(editor.command_history.is_dirty());
    let path = dir.path().join("api_save.scene.ron");

    // An open batch refuses the save.
    assert_ok(&api_line(&mut editor, &mut world, "batch begin b"));
    let refused = api_line(&mut editor, &mut world, &format!("save {}", path.display()));
    assert!(refused.contains("\"refused\""), "{refused}");
    assert!(!path.exists(), "a refused save must not write");
    assert_ok(&api_line(&mut editor, &mut world, "batch abort"));

    // Otherwise it writes the file and marks the watermark.
    let response = api_line(&mut editor, &mut world, &format!("save {}", path.display()));
    assert_ok(&response);
    assert!(path.exists(), "save must write the file");
    assert!(!editor.command_history.is_dirty(), "save marks the watermark");

    // A play session (Paused counts) is refused by the same choke point.
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    let refused = api_line(&mut editor, &mut world, &format!("save {}", dir.path().join("nope.ron").display()));
    assert!(refused.contains("\"refused\""), "{refused}");
    Ok(())
}

#[test]
fn test_hosted_writes_are_refused_while_playing_but_queries_answer() {
    let mut editor = editor_game();
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "create empty Marker"));
    editor.handle_play_action(PlayControlAction::Play, &mut world);

    // Hosted (create) and pure (rename) writes both see the play state —
    // integration alone plumbs it into `WriteCtx`.
    let refused = api_line(&mut editor, &mut world, "create empty Nope");
    assert!(refused.contains("\"refused\""), "{refused}");
    assert_eq!(world.entities().len(), 1, "nothing spawned into the live simulation");
    let refused = api_line(&mut editor, &mut world, "rename Marker Renamed");
    assert!(refused.contains("\"refused\""), "{refused}");
    assert!(matches!(
        editor::HierarchyPanel::resolve_by_name(&world, "Marker"),
        editor::NameResolution::One(_)
    ), "the live simulation keeps its name");
    assert_ok(&api_line(&mut editor, &mut world, "list"));
}

#[test]
fn test_commands_query_advertises_archetypes_settable_components_and_batching() {
    // What an agent discovers before writing anything: the create
    // vocabulary, the component names `set` accepts, and the batch verbs.
    let mut editor = editor_game();
    let mut world = World::new();

    let response = api_line(&mut editor, &mut world, "commands");

    assert_ok(&response);
    assert!(response.contains("\"archetypes\""), "{response}");
    assert!(response.contains("\"settable\""), "{response}");
    assert!(response.contains("Transform2D"), "{response}");
    assert!(response.contains("batch begin"), "{response}");
}

#[test]
fn test_play_commits_an_open_batch_as_one_entry_and_stop_discards_a_paused_one() {
    let mut editor = editor_game();
    let mut world = World::new();

    // A batch's commands are applied to the world Play snapshots, so Play
    // commits it: a macro pushed after Stop's restore would undo against
    // the wrong world.
    assert_ok(&api_line(&mut editor, &mut world, "batch begin setup"));
    assert_ok(&api_line(&mut editor, &mut world, "create empty A"));
    assert_ok(&api_line(&mut editor, &mut world, "create empty B"));
    assert!(!editor.command_history.can_undo(), "batched commands are held aside");
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.api_batch.is_none(), "Play commits the open batch");
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(editor.command_history.undo(&mut world), "the batch is one undo entry");
    assert!(world.entities().is_empty());
    assert!(!editor.command_history.can_undo());

    // A batch opened while Paused references the mid-simulation
    // world the restore discards — Stop drops it with the runtime state.
    assert_ok(&api_line(&mut editor, &mut world, "create empty Marker"));
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, "batch begin paused-edits"));
    assert_ok(&api_line(&mut editor, &mut world, "create empty Ghost"));
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(editor.api_batch.is_none(), "Stop drops the stale batch");
    assert_eq!(world.entities().len(), 1, "snapshot restore discarded the paused-world entity");
    let refused = api_line(&mut editor, &mut world, "batch end");
    assert!(refused.contains("\"refused\""), "no batch remains open: {refused}");
    assert_ne!(editor.command_history.undo_name(), Some("paused-edits"), "the stale macro never reaches the history");
}
