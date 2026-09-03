//! The unsaved-changes confirm state machine (#52): a clean world proceeds,
//! a dirty world parks the action for the modal, a play session refuses
//! outright, and while the dialog shows it owns the keyboard.

use ecs::World;
use editor::PlayControlAction;
use winit::keyboard::KeyCode;

use super::scene_confirm::PendingSceneAction;
use super::test_support::{dirty_editor, editor_game};

#[test]
fn test_scene_replace_proceeds_when_clean_parks_when_dirty_and_refuses_mid_session() {
    // Clean: no dialog, proceed now.
    let mut editor = editor_game();
    assert!(editor.request_scene_replace(PendingSceneAction::NewScene));
    assert_eq!(editor.pending_scene_action, None);

    // Dirty: park the action until the user decides.
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    assert!(!editor.request_scene_replace(PendingSceneAction::NewScene));
    assert_eq!(editor.pending_scene_action, Some(PendingSceneAction::NewScene));

    // Entering Play drops the pending dialog (defensive — the blocked UI
    // cannot reach Play, but a stale action must never fire after Stop).
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor.pending_scene_action, None);

    // Mid-session (Paused too): the #22 refusal wins — no dialog.
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(!editor.request_scene_replace(PendingSceneAction::NewScene));
    assert_eq!(editor.pending_scene_action, None);
    assert!(
        editor.editor.status_bar.message().is_some_and(|m| m.contains("stop Play")),
        "the refusal reaches the status bar"
    );
}

#[test]
fn test_dialog_swallows_keys_enter_queues_save_and_escape_cancels() {
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.request_scene_replace(PendingSceneAction::NewScene);

    // Every other editor key is swallowed while the dialog shows — Delete
    // or Ctrl+S acting under a modal would mutate state the user is being
    // asked about.
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Delete));
    assert!(editor.confirm_dialog_consumes_key(KeyCode::KeyS));
    assert_eq!(editor.pending_scene_action, Some(PendingSceneAction::NewScene));
    assert_eq!(editor.pending_dialog_choice, None, "swallowed keys choose nothing");

    // Enter = the primary (Save) action, queued for the next render, which
    // owns the GameContext (kimi F4).
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Enter));
    assert_eq!(editor.pending_dialog_choice, Some(editor::ConfirmChoice::Confirm));

    // Escape cancels the dialog (ahead of the normal cancel cascade) and
    // drops the queued choice — no ghost Save after a cancel.
    assert!(editor.confirm_dialog_consumes_key(KeyCode::Escape));
    assert_eq!(editor.pending_scene_action, None);
    assert_eq!(editor.pending_dialog_choice, None);

    // With no dialog, keys pass through untouched.
    assert!(!editor.confirm_dialog_consumes_key(KeyCode::Delete));
}
