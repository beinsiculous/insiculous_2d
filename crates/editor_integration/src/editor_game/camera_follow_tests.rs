//! The camera split: while Playing the viewport follows the game's
//! main camera — position and zoom, never rotation — manual input breaks
//! the follow, the toggle re-arms it, and Stop restores the editing view.

use ecs::World;
use editor::PlayControlAction;
use glam::Vec2;

use super::test_support::editor_game;

/// Zoom on the fixture camera — beyond the viewport's interactive clamp,
/// so an adopted zoom proves it is taken unclamped.
const GAME_ZOOM: f32 = 20.0;
const GAME_POSITION: Vec2 = Vec2::new(320.0, -75.0);

/// World with a rotated main camera at [`GAME_POSITION`] zoomed to
/// [`GAME_ZOOM`].
fn world_with_zoomed_camera() -> World {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            common::Camera::default().as_main_camera().with_zoom(GAME_ZOOM).with_rotation(0.7),
        )
        .ok();
    world
        .add_component(&entity, common::Transform2D::from_parts(GAME_POSITION, 0.7, Vec2::ONE))
        .ok();
    world
}

#[test]
fn test_play_adopts_the_game_camera_pose_unclamped_or_zoom_one_without_a_camera() {
    let mut editor_game = editor_game();
    let mut world = world_with_zoomed_camera();
    editor_game.editor.viewport.set_camera_position(Vec2::new(-500.0, 40.0));
    editor_game.editor.viewport.set_camera_zoom(0.5);

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);

    // The game camera's real zoom renders in the editor exactly as it does
    // outside it — the interactive [0.1, 10] scroll clamp must not apply.
    assert_eq!(editor_game.editor.viewport.camera_position(), GAME_POSITION);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), GAME_ZOOM);
    assert!(editor_game.editor.is_camera_following(), "follow armed at session start");

    // No camera entity = the game renders unzoomed outside the editor too.
    let mut editor_game = super::test_support::editor_game();
    let mut empty = World::new();
    editor_game.editor.viewport.set_camera_zoom(3.0);
    editor_game.handle_play_action(PlayControlAction::Play, &mut empty);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.0);
}

#[test]
fn test_main_camera_rotation_is_never_mirrored_onto_the_viewport() {
    // The viewport math has no rotation term, so a rotated game camera
    // must render unrotated in the editor rather than skewing picking.
    let mut editor_game = editor_game();
    let mut world = world_with_zoomed_camera();
    let window = Vec2::new(1280.0, 720.0);

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    editor_game.sync_viewport_from_main_camera(&world);

    let rendered = editor_game.editor.viewport.to_window_render_camera(window);
    assert_eq!(rendered.rotation, 0.0, "rotation is deliberately not synced");
    assert_eq!(rendered.zoom, GAME_ZOOM, "position and zoom still are");
}

#[test]
fn test_sync_copies_pose_only_while_playing_and_following() {
    let mut editor_game = editor_game();
    let mut world = world_with_zoomed_camera();

    // Editing: the game camera must NOT move the editing view.
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::ZERO);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.0);

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

    // A degenerate authored zoom must never reach the divide-by-zoom math.
    for e in world.entities() {
        if let Some(c) = world.get_mut::<common::Camera>(e) {
            c.zoom = 0.0;
        }
    }
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), 1.0, "zoom 0 is sanitized to 1");

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
fn test_pause_resume_preserves_a_broken_follow_and_refollow_snaps_back() {
    // Re-arm happens at SESSION START only — resuming from
    // pause must not override the user's explicit free-camera choice.
    let mut editor_game = editor_game();
    let mut world = world_with_zoomed_camera();

    // Outside a play session the toggle is a no-op: there is nothing to follow.
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    assert!(editor_game.editor.is_camera_following(), "toggling while Editing changes nothing");

    editor_game.handle_play_action(PlayControlAction::Play, &mut world);
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    assert!(!editor_game.editor.is_camera_following());
    editor_game.editor.viewport.set_camera_position(Vec2::new(-999.0, 999.0));
    editor_game.editor.viewport.set_camera_zoom(0.1);

    editor_game.handle_play_action(PlayControlAction::Pause, &mut world);
    editor_game.handle_play_action(PlayControlAction::Play, &mut world); // resume
    assert!(
        !editor_game.editor.is_camera_following(),
        "resume must not re-arm a follow the user broke"
    );
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), Vec2::new(-999.0, 999.0));

    // Re-arming (Follow button / Ctrl+Shift+F): the next sync snaps to the
    // game pose.
    editor_game.handle_play_action(PlayControlAction::ToggleCameraFollow, &mut world);
    assert!(editor_game.editor.is_camera_following());
    editor_game.sync_viewport_from_main_camera(&world);
    assert_eq!(editor_game.editor.viewport.camera_position(), GAME_POSITION);
    assert_eq!(editor_game.editor.viewport.camera_zoom(), GAME_ZOOM);
}

#[test]
fn test_stop_restores_editing_view_and_rearms_follow() {
    let mut editor_game = editor_game();
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
fn test_play_transition_cancels_pending_viewport_gesture() {
    // Handle_input runs during Play too, so a button held
    // across a play-state transition must not complete a phantom
    // click/marquee in the new state.
    let mut editor_game = editor_game();
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
