//! Stop's Keep / Discard / Cancel contract: edits made while Paused are
//! never silently erased, undo inside a session stops at the Play
//! boundary, and every Stop path goes through the dialog.

use ecs::script::{ScriptRef, ScriptValue, Scripts};
use ecs::sprite_components::Name;
use ecs::World;
use editor::PlayControlAction;
use glam::Vec2;
use physics::components::Collider;
use winit::keyboard::KeyCode;

use super::play_session::PausedEdits;
use super::scene_confirm::PendingSceneAction;
use super::test_support::{api_line, assert_ok, dirty_editor, editor_game, position, spawn_at};

/// Play, then Pause, on a session whose world already holds `entity`.
fn play_then_pause<G: engine_core::Game>(editor: &mut super::EditorGame<G>, world: &mut World) {
    editor.handle_play_action(PlayControlAction::Play, world);
    editor.handle_play_action(PlayControlAction::Pause, world);
}

#[test]
fn test_stop_without_paused_edits_restores_immediately() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    world.get_mut::<common::Transform2D>(entity).expect("alive").position = Vec2::new(40.0, 0.0);

    assert!(editor.handle_play_action(PlayControlAction::Stop, &mut world), "nothing to ask about");
    assert!(!editor.stop_confirm.pending);
    assert_eq!(position(&world, entity), Vec2::ZERO, "the simulated pose is gone");
}

#[test]
fn test_stop_with_paused_edits_freezes_the_game_and_parks_the_dialog() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, &format!("rename #{} Edited", entity.value())));
    // Resume and Stop from Playing: the dialog parks the session Paused so
    // it is never a modal over a running simulation.
    editor.handle_play_action(PlayControlAction::Play, &mut world);

    assert!(!editor.handle_play_action(PlayControlAction::Stop, &mut world));
    assert!(editor.stop_confirm.pending, "the dialog is up");
    assert!(editor.editor.is_paused(), "and the simulation is frozen under it");
    assert_eq!(editor.command_history.session_entry_count(), 1);
}

#[test]
fn test_keep_applies_the_paused_edit_to_the_restored_world_and_undo_returns_to_the_authored_value() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    world.get_mut::<common::Transform2D>(entity).expect("alive").position = Vec2::new(100.0, 50.0);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[120.0,50.0]}}", entity.value()),
    ));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    editor.stop_with_paused_edits(&mut world, PausedEdits::Keep);

    assert_eq!(position(&world, entity), Vec2::new(120.0, 0.0), "only the edited leaf survives");
    editor.undo_with_feedback(&mut world);
    assert_eq!(position(&world, entity), Vec2::ZERO, "undo returns the authored value");
}

#[test]
fn test_rebase_keeps_the_authored_y_when_only_x_was_edited_while_paused() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    world.get_mut::<common::Transform2D>(entity).expect("alive").position = Vec2::new(100.0, 50.0);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[120.0,50.0]}}", entity.value()),
    ));
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    editor.stop_with_paused_edits(&mut world, PausedEdits::Keep);

    assert_eq!(position(&world, entity), Vec2::new(120.0, 0.0));
}

#[test]
fn test_keep_replays_an_open_api_batch_made_while_paused() {
    for (choice, expected) in [(PausedEdits::Keep, "Kept"), (PausedEdits::Discard, "start")] {
        let mut world = World::new();
        let entity = spawn_at(&mut world, Vec2::ZERO);
        world.add_component(&entity, Name::new("start")).ok();
        let mut editor = editor_game();

        play_then_pause(&mut editor, &mut world);
        assert_ok(&api_line(&mut editor, &mut world, "batch begin paused-edits"));
        assert_ok(&api_line(&mut editor, &mut world, &format!("rename #{} Kept", entity.value())));
        assert_ok(&api_line(
            &mut editor,
            &mut world,
            &format!("set #{} Transform2D {{\"position\":[1.0,1.0]}}", entity.value()),
        ));

        editor.handle_play_action(PlayControlAction::Stop, &mut world);
        assert_eq!(editor.command_history.session_entry_count(), 1, "the batch counts as one edit");
        editor.stop_with_paused_edits(&mut world, choice);

        let name = world.get::<Name>(entity).map(|n| n.as_str().to_string());
        assert_eq!(name.as_deref(), Some(expected), "{choice:?}");
    }
}

#[test]
fn test_discard_restores_the_snapshot_and_truncates_the_history_to_the_play_boundary() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[9.0,9.0]}}", entity.value()),
    ));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    editor.stop_with_paused_edits(&mut world, PausedEdits::Discard);

    assert_eq!(position(&world, entity), Vec2::ZERO);
    assert!(!editor.command_history.can_undo(), "the paused edit left no undo entry");
}

#[test]
fn test_cancel_keeps_the_session_paused_with_its_edits() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[9.0,9.0]}}", entity.value()),
    ));
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    assert!(editor.confirm_dialog_consumes_key(KeyCode::Escape), "Escape answers the dialog");
    assert!(!editor.stop_confirm.pending);
    assert!(editor.editor.is_paused(), "still paused");
    assert_eq!(position(&world, entity), Vec2::new(9.0, 9.0), "with the edit intact");
    assert_eq!(editor.command_history.session_entry_count(), 1);
}

#[test]
fn test_the_stop_dialog_swallows_every_key_while_it_is_pending() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[9.0,9.0]}}", entity.value()),
    ));
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    assert!(editor.confirm_dialog_consumes_key(KeyCode::Delete), "an unrelated key is eaten");
    assert!(editor.stop_confirm.pending, "and changes nothing");
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Enter));
    assert_eq!(
        editor.stop_confirm.pending_choice,
        Some(editor::ConfirmChoice::Confirm),
        "Enter queues Keep for the next render"
    );
}

#[test]
fn test_undo_while_paused_stops_at_the_play_boundary() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();
    assert_ok(&api_line(&mut editor, &mut world, &format!("rename #{} Authored", entity.value())));

    play_then_pause(&mut editor, &mut world);
    editor.undo_with_feedback(&mut world);
    assert_eq!(
        world.get::<Name>(entity).map(|n| n.as_str().to_string()).as_deref(),
        Some("Authored"),
        "the authored rename is behind the boundary"
    );
    assert!(editor
        .editor
        .status_bar
        .message()
        .is_some_and(|message| message.contains("Play boundary")));

    let refused = api_line(&mut editor, &mut world, "undo");
    assert!(refused.contains("Play boundary"), "the API says the same: {refused}");
}

#[test]
fn test_every_stop_path_routes_through_the_dialog() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[9.0,9.0]}}", entity.value()),
    ));

    // The play-control action (the toolbar button and the shortcut share it).
    assert!(!editor.handle_play_action(PlayControlAction::Stop, &mut world));
    assert!(editor.stop_confirm.pending);
    editor.confirm_dialog_consumes_key(KeyCode::Escape);

    // The API has no stop verb at all, so no request line can reach the
    // restore behind the dialog's back.
    let unknown = api_line(&mut editor, &mut world, "stop");
    assert!(!unknown.contains("\"ok\":true"), "no stop verb exists: {unknown}");
    assert!(editor.editor.is_paused(), "the session is untouched");
    assert_eq!(position(&world, entity), Vec2::new(9.0, 9.0));
}

#[test]
fn test_play_while_the_stop_dialog_is_pending_cancels_the_dialog() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"position\":[9.0,9.0]}}", entity.value()),
    ));
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(editor.stop_confirm.pending);

    // Play from Paused RESUMES: a queued Keep would restore a session the
    // user has just put back in motion.
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(!editor.stop_confirm.pending);
    assert!(editor.editor.is_playing());
    assert_eq!(editor.command_history.session_entry_count(), 1, "the session is intact");
}

#[test]
fn test_rebase_drops_a_paused_delete_of_a_runtime_only_entity_so_undo_creates_no_phantom() {
    let mut world = World::new();
    let authored = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    let runtime_only = spawn_at(&mut world, Vec2::new(5.0, 5.0));
    assert_ok(&api_line(&mut editor, &mut world, &format!("delete #{}", runtime_only.value())));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    let unapplied = editor.stop_with_paused_edits(&mut world, PausedEdits::Keep).dropped;

    assert_eq!(unapplied, 1, "the delete could not be applied");
    assert_eq!(world.entities(), vec![authored]);
    editor.undo_with_feedback(&mut world);
    editor.redo_with_feedback(&mut world);
    assert_eq!(world.entities(), vec![authored], "no phantom appears through undo or redo");
}

#[test]
fn test_rebase_drops_a_paused_add_over_an_authored_component() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    world.add_component(&entity, Collider::player_box(10.0, 10.0)).ok();
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    world.remove_component::<Collider>(&entity).ok();
    assert_ok(&api_line(&mut editor, &mut world, &format!("add #{} Collider", entity.value())));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    let unapplied = editor.stop_with_paused_edits(&mut world, PausedEdits::Keep).dropped;

    assert_eq!(unapplied, 1);
    let shape = world.get::<Collider>(entity).map(|collider| collider.shape.clone());
    assert_eq!(
        shape,
        Some(Collider::player_box(10.0, 10.0).shape),
        "the authored collider survives Keep"
    );
    editor.undo_with_feedback(&mut world);
    assert!(world.get::<Collider>(entity).is_some(), "and Undo does not strip it");
}

#[test]
fn test_rebase_drops_a_paused_creation_the_simulation_destroyed() {
    let mut world = World::new();
    let authored = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, "create empty Doomed"));
    let doomed = *world
        .entities()
        .iter()
        .find(|&&candidate| candidate != authored)
        .expect("the created entity");
    // Resume, and let the simulation destroy it.
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    world.remove_entity(&doomed).ok();

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    let unapplied = editor.stop_with_paused_edits(&mut world, PausedEdits::Keep).dropped;

    assert_eq!(unapplied, 1);
    assert_eq!(world.entities(), vec![authored], "no empty phantom under the dead id");
}

#[test]
fn test_rebase_drops_a_script_reference_to_a_runtime_only_entity() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut authored_scripts = ScriptRef::new("follower");
    authored_scripts.params.insert("target".to_string(), ScriptValue::F32(1.0));
    world.add_component(&entity, Scripts(vec![authored_scripts])).ok();
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    let runtime_only = spawn_at(&mut world, Vec2::new(5.0, 5.0));
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!(
            "set #{} Scripts [{{\"script_id\":\"follower\",\"source_path\":\"\",\"params\":{{\"target\":{{\"Entity\":{}}}}}}}]",
            entity.value(),
            serde_json::to_string(&runtime_only).expect("EntityId serializes")
        ),
    ));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    let unapplied = editor.stop_with_paused_edits(&mut world, PausedEdits::Keep).dropped;

    assert_eq!(unapplied, 1, "a binding that cannot survive Stop is dropped, not silently lost");
    let target = world
        .get::<Scripts>(entity)
        .and_then(|scripts| scripts.0.first())
        .and_then(|script| script.params.get("target").cloned());
    assert_eq!(target, Some(ScriptValue::F32(1.0)), "the authored parameter stands");
}

#[test]
fn test_keep_rebases_a_macro_child_by_child() {
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, "batch begin two-writes"));
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"rotation\":0.5}}", entity.value()),
    ));
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        &format!("set #{} Transform2D {{\"scale\":[2.0,2.0]}}", entity.value()),
    ));

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    editor.stop_with_paused_edits(&mut world, PausedEdits::Keep);

    let transform = world.get::<common::Transform2D>(entity).cloned().expect("alive");
    assert_eq!(transform.rotation, 0.5, "the first write is not erased by the second");
    assert_eq!(transform.scale, Vec2::new(2.0, 2.0));
}

#[test]
fn test_api_writes_are_refused_while_the_stop_dialog_is_pending() {
    // A write landing under the dialog would change the answer the user
    // is being asked for, and a batch opened under it would carry
    // simulated images across the restore.
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = editor_game();

    play_then_pause(&mut editor, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, &format!("rename #{} Edited", entity.value())));
    assert!(!editor.handle_play_action(PlayControlAction::Stop, &mut world));
    assert!(editor.stop_confirm.pending);

    let refused = api_line(&mut editor, &mut world, &format!("rename #{} Again", entity.value()));
    assert!(refused.contains("dialog is pending"), "got: {refused}");
    let refused = api_line(&mut editor, &mut world, "batch begin");
    assert!(refused.contains("dialog is pending"), "got: {refused}");
    assert!(editor.api.batch.is_none(), "no batch opened under the dialog");
    assert_eq!(editor.command_history.session_entry_count(), 1, "the count did not move");

    assert!(editor.confirm_dialog_consumes_key(KeyCode::Escape));
    assert!(!editor.stop_confirm.pending);
    assert_ok(&api_line(&mut editor, &mut world, &format!("rename #{} Again", entity.value())));
}

#[test]
fn test_api_writes_are_refused_under_the_scene_dialog_too() {
    // The scene dialog asks "keep or discard the unsaved changes?"; a write
    // landing under it would change what Cancel keeps.
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::ZERO);
    let mut editor = dirty_editor(&mut world);

    assert!(!editor.request_scene_replace(PendingSceneAction::NewScene), "the dialog parks");
    let refused = api_line(&mut editor, &mut world, &format!("rename #{} Under", entity.value()));
    assert!(refused.contains("dialog is pending"), "got: {refused}");
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Escape));
    assert_ok(&api_line(&mut editor, &mut world, &format!("rename #{} After", entity.value())));
}
