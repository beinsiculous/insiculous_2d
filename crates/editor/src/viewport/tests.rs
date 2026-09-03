//! The scene viewport's mapping contracts: screen ↔ world, the overlay ↔
//! GPU camera equivalence that keeps gizmos, picking and the grid on top of
//! the rendered sprites, the interactive zoom range, and the frame-rate
//! independent camera glide.

use super::*;
use crate::test_support::test_viewport;

/// Asserts the viewport overlay mapping and the GPU render-camera mapping
/// agree for a spread of world points — one view, two consumers.
fn assert_overlay_matches_render_camera(viewport: &SceneViewport, window_size: Vec2) {
    let camera = viewport.to_window_render_camera(window_size);
    for world in [
        Vec2::ZERO,
        Vec2::new(100.0, 50.0),
        Vec2::new(-3.5, 77.25),
        Vec2::new(-250.0, -125.0),
    ] {
        let overlay = viewport.world_to_screen(world);
        let gpu = camera.world_to_screen(world);
        assert!(
            (overlay - gpu).length() < 0.01,
            "mismatch at {world}: overlay {overlay} vs gpu {gpu}"
        );
    }
}

#[test]
fn test_screen_and_world_round_trip_through_the_panel_center() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(100.0, 50.0, 800.0, 600.0));

    // The panel center is the camera position, whatever the panel origin.
    assert_eq!(viewport.screen_to_world(Vec2::new(500.0, 350.0)), Vec2::ZERO);
    assert_eq!(viewport.world_to_screen(Vec2::ZERO), Vec2::new(500.0, 350.0));

    viewport.set_camera_position(Vec2::new(100.0, -50.0));
    viewport.set_camera_zoom(2.0);
    let screen = Vec2::new(350.0, 200.0);
    let back = viewport.world_to_screen(viewport.screen_to_world(screen));
    assert!((back - screen).length() < 0.001, "round trip drifted to {back}");
}

#[test]
fn test_visible_world_bounds_shrink_as_zoom_grows() {
    let mut viewport = test_viewport();
    for (zoom, expected) in [
        (1.0, (-400.0, -300.0, 400.0, 300.0)),
        (2.0, (-200.0, -150.0, 200.0, 150.0)),
        (0.5, (-800.0, -600.0, 800.0, 600.0)),
    ] {
        viewport.set_camera_zoom(zoom);
        assert_eq!(viewport.visible_world_bounds(), expected, "at zoom {zoom}");
    }
}

#[test]
fn test_interactive_zoom_clamps_to_the_ux_range_but_a_followed_game_camera_does_not() {
    let mut viewport = SceneViewport::new();

    viewport.set_camera_zoom(0.01);
    assert_eq!(viewport.camera_zoom(), 0.1, "clamped to the minimum");
    viewport.set_camera_zoom(100.0);
    assert_eq!(viewport.camera_zoom(), 10.0, "clamped to the maximum");

    // The play-session follow adopts the game's zoom exactly, but
    // never a value the world↔screen division cannot survive.
    viewport.adopt_camera_zoom(25.0);
    assert_eq!(viewport.camera_zoom(), 25.0);
    viewport.adopt_camera_zoom(0.0);
    assert_eq!(viewport.camera_zoom(), 1.0, "a zero zoom falls back to 1.0");
    viewport.adopt_camera_zoom(f32::NAN);
    assert_eq!(viewport.camera_zoom(), 1.0, "a NaN zoom falls back to 1.0");
}

#[test]
fn test_camera_glide_reaches_its_target_at_any_frame_rate() {
    // zoom_at/pan/reset write targets only; update() moves the live
    // camera, and the same wall-clock time covers the same distance
    // whatever the step size.
    let mut coarse = SceneViewport::new();
    let mut fine = SceneViewport::new();
    for viewport in [&mut coarse, &mut fine] {
        viewport.set_target_camera_position(Vec2::new(100.0, -40.0));
        viewport.set_target_zoom(2.0);
    }

    coarse.update(0.0);
    assert_eq!(coarse.camera_position(), Vec2::ZERO, "no time passed, no movement");

    coarse.update(1.0 / 60.0);
    fine.update(1.0 / 120.0);
    fine.update(1.0 / 120.0);
    assert!((coarse.camera_position() - fine.camera_position()).length() < 0.001);
    assert!((coarse.camera_zoom() - fine.camera_zoom()).abs() < 0.001);

    for _ in 0..120 {
        coarse.update(1.0 / 60.0);
    }
    assert!((coarse.camera_position() - Vec2::new(100.0, -40.0)).length() < 0.01);
    assert!((coarse.camera_zoom() - 2.0).abs() < 0.001);
}

#[test]
fn test_overlay_matches_gpu_camera_at_a_panel_offset_and_the_play_follow_pose() {
    // Regression: with a NONZERO panel origin (dock chrome on all sides)
    // the old to_render_camera offset sprites from the editor overlay.
    let window = Vec2::new(1600.0, 900.0);
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(300.0, 100.0, 800.0, 600.0));
    assert_overlay_matches_render_camera(&viewport, window);

    viewport.set_camera_position(Vec2::new(120.0, -40.0));
    viewport.set_camera_zoom(2.0);
    assert_overlay_matches_render_camera(&viewport, window);

    // Following the game camera adopts its position AND zoom,
    // past the interactive clamp — picking stays truthful there too.
    viewport.set_camera_position(Vec2::new(320.0, -75.0));
    viewport.adopt_camera_zoom(12.5);
    assert_overlay_matches_render_camera(&viewport, window);

    // And screen_to_world is the GPU camera's inverse: clicks land on the
    // sprite under the cursor.
    let camera = viewport.to_window_render_camera(window);
    for screen in [Vec2::new(700.0, 400.0), Vec2::new(310.0, 110.0), Vec2::new(1050.0, 650.0)] {
        let back = camera.world_to_screen(viewport.screen_to_world(screen));
        assert!((back - screen).length() < 0.01, "roundtrip mismatch at {screen}: got {back}");
    }
}
