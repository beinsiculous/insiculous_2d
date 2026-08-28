//! Camera-split tests (issue #42): the viewport follows the game camera —
//! position AND zoom — during Play, manual input breaks the follow, the
//! toggle re-arms it, and Stop restores the editing view.

use ecs::World;
use editor::PlayControlAction;
use engine_core::contexts::GameContext;
use engine_core::Game;
use glam::Vec2;

use super::EditorGame;

struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// World with a main camera at (320, -75) zoomed to 2.5.
fn world_with_zoomed_camera() -> World {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(&entity, common::Camera::default().as_main_camera().with_zoom(2.5))
        .ok();
    world
        .add_component(&entity, common::Transform2D::new(Vec2::new(320.0, -75.0)))
        .ok();
    world
}

#[test]
fn test_play_adopts_game_camera_pose_including_zoom() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = world_with_zoomed_camera();
    editor_game.editor.viewport.set_camera_position(Vec2::new(-500.0, 40.0));
    editor_game.editor.viewport.set_camera_zoom(0.5);

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);

    // The old behavior forced zoom 1.0; the game camera's real zoom now
    // renders in the editor exactly as it does outside it.
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(320.0, -75.0));
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 2.5);
    assert!(editor_game.editor.is_camera_following(), "follow armed at session start");
}

#[test]
fn test_play_without_main_camera_keeps_zoom_one_parity() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = World::new();
    editor_game.editor.viewport.set_camera_zoom(3.0);

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);

    // No camera entity = the game renders unzoomed outside the editor too.
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.0);
}

#[test]
fn test_sync_copies_zoom_only_while_playing_and_following() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = world_with_zoomed_camera();
    editor_game.handle_play_action(PlayControlAction::Play, &mut world);

    // Move the game camera mid-play; sync mirrors position AND zoom.
    for e in world.entities() {
        if let Some(t) = world.get_mut::<common::Transform2D>(e) {
            t.position = Vec2::new(1000.0, 5.0);
        }
        if let Some(c) = world.get_mut::<common::Camera>(e) {
            c.zoom = 4.0;
        }
    }
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(1000.0, 5.0));
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 4.0);

    // Follow broken: the sync must leave the user's view alone entirely.
    editor_game.editor.set_camera_follow(false);
    editor_game.editor.viewport.set_camera_position(Vec2::new(-8.0, -9.0));
    editor_game.editor.viewport.set_camera_zoom(0.75);
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(-8.0, -9.0));
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 0.75);

    // Paused never syncs, even while following (frozen view is inspectable).
    editor_game.editor.set_camera_follow(true);
    editor_game.handle_play_action(PlayControlAction::Pause, &mut world);
    editor_game.editor.viewport.set_camera_position(Vec2::new(1.0, 2.0));
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(1.0, 2.0));
}

#[test]
fn test_refollow_snaps_back_to_game_pose() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = world_with_zoomed_camera();
    editor_game.handle_play_action(PlayControlAction::Play, &mut world);

    // Free-move somewhere else...
    editor_game.editor.set_camera_follow(false);
    editor_game.editor.viewport.set_camera_position(Vec2::new(-999.0, 999.0));
    editor_game.editor.viewport.set_camera_zoom(0.1);

    // ...then re-arm the follow: next sync snaps to the game pose.
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    assert!(editor_game.editor.is_camera_following());
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(320.0, -75.0));
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 2.5);
}

#[test]
fn test_pause_resume_preserves_a_broken_follow() {
    // kimi R2-F8: re-arm happens at SESSION START only — resuming from
    // pause must not override the user's explicit free-camera choice.
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = world_with_zoomed_camera();
    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    assert!(!editor_game.editor.is_camera_following());

    editor_game.handle_play_action(PlayControlAction::Pause, &mut world);
    editor_game.handle_play_action(PlayControlAction::Play, &mut world); // resume
    assert!(
        !editor_game.editor.is_camera_following(),
        "resume must not re-arm a follow the user broke"
    );
}

#[test]
fn test_stop_restores_editing_view_and_rearms_follow() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = world_with_zoomed_camera();
    editor_game.editor.viewport.set_camera_position(Vec2::new(77.0, -33.0));
    editor_game.editor.viewport.set_camera_zoom(1.5);

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    editor_game.handle_play_action(PlayControlAction::Stop, &mut world);

    // Editing pan/zoom restored exactly; the NEXT session follows again.
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(77.0, -33.0));
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.5);
    assert!(editor_game.editor.is_camera_following());
}

#[test]
fn test_toggle_is_a_noop_outside_a_play_session() {
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = World::new();
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    assert!(
        editor_game.editor.is_camera_following(),
        "toggling while Editing changes nothing"
    );
}

#[test]
fn test_play_transition_cancels_pending_viewport_gesture() {
    // kimi #42 F5: handle_input runs during Play too now, so a button held
    // across a play-state transition must not complete a phantom
    // click/marquee in the new state.
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = World::new();
    editor_game
        .editor
        .viewport
        .set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    // Press the primary button inside the viewport: a pending gesture arms.
    let mut input = input::InputHandler::new();
    input.mouse_mut().update_position(400.0, 300.0);
    input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
    editor_game.editor.viewport_input.handle_input_simple(
        &mut editor_game.editor.viewport,
        &editor_game.editor.input_mapping,
        &input,
    );
    assert!(editor_game.editor.viewport_input.has_pending_marquee());

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(
        !editor_game.editor.viewport_input.has_pending_marquee(),
        "a play-state transition kills the in-flight gesture"
    );
}

#[test]
fn test_follow_adopts_extreme_zoom_unclamped() {
    // kimi #42 F2: the follow view must match the shipped game exactly —
    // the interactive [0.1, 10] scroll clamp must not apply to an adopted
    // game-camera zoom.
    let mut editor_game = EditorGame::new(DummyGame);
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(&entity, common::Camera::default().as_main_camera().with_zoom(20.0))
        .ok();
    world
        .add_component(&entity, common::Transform2D::new(Vec2::ZERO))
        .ok();

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 20.0);

    // A degenerate authored zoom must never reach the divide-by-zoom math.
    for e in world.entities() {
        if let Some(c) = world.get_mut::<common::Camera>(e) {
            c.zoom = 0.0;
        }
    }
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.0);
}

#[test]
fn test_follow_toggle_chord_resolves_over_focus_binding() {
    // Ctrl+Shift+F = ToggleCameraFollow (exact chord); F alone still frames.
    use winit::keyboard::KeyCode;
    let editor_game = EditorGame::new(DummyGame);
    let mapping = &editor_game.editor.input_mapping;
    assert_eq!(
        mapping.resolve(KeyCode::KeyF, true, true),
        Some(editor::EditorAction::ToggleCameraFollow)
    );
    assert_eq!(
        mapping.resolve(KeyCode::KeyF, false, false),
        Some(editor::EditorAction::FocusSelection)
    );
}
