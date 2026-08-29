//! Headless tests for the unsaved-changes confirm state machine (#52).
//! The dialog's widget mechanics live in `editor::confirm_dialog`; here we
//! lock the gating, keyboard policy, and play-session interactions.

use ecs::World;
use editor::PlayControlAction;
use engine_core::contexts::GameContext;
use engine_core::Game;
use winit::keyboard::KeyCode;

use super::scene_confirm::PendingSceneAction;
use super::EditorGame;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

fn dirty_editor(world: &mut World) -> EditorGame<DummyGame> {
    let mut editor = EditorGame::new(DummyGame);
    let entity = world.create_entity();
    editor.command_history.execute(
        Box::new(editor::commands::CreateEntityCommand::already_created(world, entity)),
        world,
    );
    assert!(editor.command_history.is_dirty());
    editor
}

#[test]
fn test_clean_world_proceeds_immediately() {
    let editor = &mut EditorGame::new(DummyGame);
    assert!(
        editor.request_scene_replace(PendingSceneAction::NewScene),
        "a clean world needs no dialog"
    );
    assert!(editor.pending_scene_action.is_none());
}

#[test]
fn test_dirty_world_parks_the_action_for_the_dialog() {
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    assert!(
        !editor.request_scene_replace(PendingSceneAction::NewScene),
        "a dirty world must not proceed without a decision"
    );
    assert_eq!(editor.pending_scene_action, Some(PendingSceneAction::NewScene));
}

#[test]
fn test_play_session_refuses_without_raising_the_dialog() {
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);

    assert!(!editor.request_scene_replace(PendingSceneAction::NewScene));
    assert!(
        editor.pending_scene_action.is_none(),
        "mid-session the #22 refusal wins — no dialog"
    );
}

#[test]
fn test_dialog_swallows_keys_and_escape_cancels() {
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.request_scene_replace(PendingSceneAction::NewScene);

    // Every non-Escape editor key is swallowed while the dialog shows.
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Delete));
    assert!(editor.pending_scene_action.is_some(), "Delete did not act OR dismiss");
    assert!(editor.confirm_dialog_consumes_key(KeyCode::KeyS));
    assert!(editor.pending_scene_action.is_some());

    // Escape cancels the dialog (ahead of the normal cancel cascade).
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Escape));
    assert!(editor.pending_scene_action.is_none());

    // With no dialog, keys pass through untouched.
    assert!(!editor.confirm_dialog_consumes_key(KeyCode::Delete));
}

#[test]
fn test_entering_play_drops_a_pending_dialog() {
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.request_scene_replace(PendingSceneAction::NewScene);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(
        editor.pending_scene_action.is_none(),
        "Play defensively clears a pending confirm"
    );
}

#[test]
fn test_enter_queues_the_primary_choice_and_escape_clears_it() {
    // kimi #52 F4: Enter = Save (queued for the next render, which owns the
    // GameContext); Escape drops both the dialog and any queued choice.
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.request_scene_replace(PendingSceneAction::NewScene);

    assert!(editor.confirm_dialog_consumes_key(KeyCode::Enter));
    assert_eq!(editor.pending_dialog_choice, Some(editor::ConfirmChoice::Confirm));

    assert!(editor.confirm_dialog_consumes_key(KeyCode::Escape));
    assert!(editor.pending_scene_action.is_none());
    assert!(editor.pending_dialog_choice.is_none(), "no ghost Save after cancel");
}
