//! The play session: Play captures ONE snapshot, Stop restores the authored
//! world, and everything that would corrupt that contract is refused or
//! reported — saves and scene replacement mid-simulation, uncapturable
//! components, a stale propagation baseline, and the resources the session
//! toggles (`UiElementsHidden`, `GridBackdropReset`).

use ecs::World;
use editor::PlayControlAction;
use engine_core::test_support::test_texture_path;
use glam::Vec2;

use super::test_support::{dirty_editor, editor_game, position, spawn_at};

#[test]
fn test_play_pause_resume_stop_cycle_captures_one_snapshot() {
    let mut editor = editor_game();
    let mut world = World::new();
    spawn_at(&mut world, Vec2::ZERO);

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing());
    assert!(editor.world_snapshot.is_some(), "Play captures the snapshot");

    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.editor.is_paused());

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing());

    let stopped = editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(stopped, "Stop reports the restore so the game hears on_play_stopped");
    assert!(editor.editor.is_editing());
    assert!(editor.world_snapshot.is_none(), "the snapshot is consumed by Stop");
}

#[test]
fn test_stop_restores_the_authored_world_and_resume_never_recaptures() {
    let mut editor = editor_game();
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::new(10.0, 20.0));

    editor.handle_play_action(PlayControlAction::Play, &mut world);

    // Mutate mid-simulation, pause, resume, mutate again, stop.
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(500.0, 500.0);
    }
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    editor.handle_play_action(PlayControlAction::Play, &mut world); // resume
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(999.0, 999.0);
    }
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    // A resume that re-captured would restore the paused mid-simulation
    // state; Stop must return to the ORIGINAL authored state.
    assert_eq!(position(&world, entity), Vec2::new(10.0, 20.0));
}

#[test]
fn test_stop_resets_the_transform_propagation_baseline() {
    use ecs::System;

    let mut editor = editor_game();
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::new(10.0, 20.0));

    // Propagate once so the transform system has a cached baseline.
    editor.transform_system.update(&mut world, 0.016);
    assert_eq!(editor.transform_system.tracked_entity_count(), 1);

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        t.position = Vec2::new(999.0, 999.0);
    }
    editor.handle_play_action(PlayControlAction::Stop, &mut world);

    // The restore wholesale-replaced the world — the propagation baseline
    // must have been dropped so the next update recomputes from scratch.
    assert_eq!(editor.transform_system.tracked_entity_count(), 0, "Stop resets the cache");
    editor.transform_system.update(&mut world, 0.016);
    assert_eq!(
        world.get::<ecs::GlobalTransform2D>(entity).map(|g| g.position),
        Some(Vec2::new(10.0, 20.0))
    );
}

#[test]
fn test_save_is_refused_mid_session_and_allowed_after_stop() -> std::io::Result<()> {
    // The Ctrl+S path is unreachable while Playing but ran while Paused —
    // the paused world is equally mid-simulation and must not overwrite the
    // authored scene.
    let dir = tempfile::tempdir()?;
    for (label, pause) in [("Playing", false), ("Paused", true)] {
        let mut world = World::new();
        let mut editor = dirty_editor(&mut world);
        editor.handle_play_action(PlayControlAction::Play, &mut world);
        if pause {
            editor.handle_play_action(PlayControlAction::Pause, &mut world);
        }
        let path = dir.path().join(format!("refused_{label}.ron"));

        let err = editor
            .save_scene_with(&mut world, &test_texture_path, path.clone())
            .expect_err("saving mid-simulation must be refused");

        assert!(err.to_string().contains("stop Play"), "{label}: the error tells the user how to proceed: {err}");
        assert!(!path.exists(), "{label}: a refused save must not touch the scene file");
        assert!(editor.command_history.is_dirty(), "{label}: a refused save must not read clean");
    }

    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    let path = dir.path().join("after_stop.ron");
    let result = editor.save_scene_with(&mut world, &test_texture_path, path.clone());
    assert!(result.is_ok(), "saving after Stop must work again: {result:?}");
    assert!(path.exists());
    assert!(!editor.command_history.is_dirty());
    Ok(())
}

#[test]
fn test_scene_replacement_is_refused_mid_session() {
    // New Scene / Open Scene (menu and Ctrl+N / Ctrl+O) share one guard:
    // replacing the world under a pending play snapshot would make the
    // next Stop resurrect the old scene's entities into the new one.
    let mut editor = editor_game();
    let mut world = World::new();
    let entity = spawn_at(&mut world, Vec2::new(1.0, 2.0));
    assert_eq!(editor.scene_replace_refusal(), None, "Editing allows new/open");

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.scene_replace_refusal().is_some(), "Playing refuses new/open");
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.scene_replace_refusal().is_some(), "Paused refuses new/open");

    editor.new_scene(&mut world);
    assert_eq!(world.entity_count(), 1, "the world must not be cleared mid-simulation");
    assert!(editor.world_snapshot.is_some(), "the pending play snapshot must survive");
    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("stop Play")),
        "the refusal reaches the status bar"
    );

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert_eq!(editor.scene_replace_refusal(), None, "Stop re-allows new/open");
    assert_eq!(position(&world, entity), Vec2::new(1.0, 2.0), "Stop still restores cleanly");
}

#[test]
fn test_play_warns_about_uncapturable_components_and_stop_reports_the_drop() {
    struct CustomBrain;

    // A fully capturable world must not nag on Play.
    let mut editor = editor_game();
    let mut world = World::new();
    spawn_at(&mut world, Vec2::ZERO);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor.editor.status_bar.message(), None);

    // An unregistered component cannot be snapshotted: warn on Play...
    let mut editor = editor_game();
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, CustomBrain).ok();
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("lost on Stop")),
        "entering Play must warn about the coming loss"
    );

    // ...and report the loss where it happens, since the Play warning is
    // easy to miss.
    editor.editor.status_bar.clear_message();
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("dropped")),
        "Stop must report what the restore dropped"
    );
    assert!(world.get::<CustomBrain>(entity).is_none(), "the drop itself is the documented loss");
}

#[test]
fn test_scene_authored_ui_is_hidden_outside_play() {
    // `init` inserts the marker; Play removes it so UiLabel/UiPanel/UiButton
    // draw only while the game runs; Stop re-inserts it after the restore.
    let mut editor = editor_game();
    let mut world = World::new();
    world.insert_resource(engine_core::UiElementsHidden);

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(!world.has_resource::<engine_core::UiElementsHidden>(), "Play reveals scene UI");

    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(!world.has_resource::<engine_core::UiElementsHidden>(), "resume keeps it visible");

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(world.has_resource::<engine_core::UiElementsHidden>(), "Stop hides it again");
}

#[test]
fn test_stop_requests_a_grid_backdrop_reset() {
    // Entity ids survive the restore, so a spring grid stopped mid-ripple
    // would stay deformed and frozen without the reset request.
    let mut editor = editor_game();
    let mut world = World::new();

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(!world.has_resource::<engine_core::grid::GridBackdropReset>(), "Play requests nothing");

    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    assert!(world.has_resource::<engine_core::grid::GridBackdropReset>(), "Stop queues the reset");
}

#[test]
fn test_menu_actions_disallowed_while_playing_match_allowed_while_playing() {
    let bar = editor::MenuBar::editor_default();
    for menu in bar.menus() {
        for item in &menu.items {
            if let editor::MenuItem::Action { label, enabled: true, .. } = item {
                let action = editor::action_for_menu_label(label.as_str()).expect("menu label maps to action");
                let is_disallowed = matches!(
                    label.as_str(),
                    "New Scene"
                        | "Open Scene..."
                        | "Cut"
                        | "Copy"
                        | "Paste"
                        | "Delete"
                        | "Duplicate"
                        | "Undo"
                        | "Redo"
                        | "Create Empty"
                        | "Create Sprite"
                        | "Create Camera"
                        | "Create Static Body"
                        | "Create Dynamic Body"
                        | "Create Kinematic Body"
                        | "Create UI Label"
                        | "Create UI Panel"
                        | "Create UI Button"
                );
                assert_eq!(
                    !action.allowed_while_playing(),
                    is_disallowed,
                    "action for '{label}' allowed_while_playing mismatch"
                );
            }
        }
    }
}
