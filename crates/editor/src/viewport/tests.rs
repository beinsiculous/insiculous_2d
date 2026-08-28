use super::*;

#[test]
fn test_viewport_new() {
    let viewport = SceneViewport::new();
    assert_eq!(viewport.camera_position(), Vec2::ZERO);
    assert_eq!(viewport.camera_zoom(), 1.0);
}

#[test]
fn test_viewport_set_bounds() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(100.0, 50.0, 800.0, 600.0));

    assert_eq!(viewport.viewport_bounds().x, 100.0);
    assert_eq!(viewport.viewport_bounds().y, 50.0);
    assert_eq!(viewport.viewport_size(), Vec2::new(800.0, 600.0));
    assert_eq!(viewport.viewport_center(), Vec2::new(500.0, 350.0));
}

#[test]
fn test_viewport_screen_to_world_no_offset() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));

    // Center of viewport should map to camera position (origin)
    let world = viewport.screen_to_world(Vec2::new(400.0, 300.0));
    assert!((world.x).abs() < 0.001);
    assert!((world.y).abs() < 0.001);
}

#[test]
fn test_viewport_world_to_screen_no_offset() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));

    // World origin should map to viewport center
    let screen = viewport.world_to_screen(Vec2::ZERO);
    assert!((screen.x - 400.0).abs() < 0.001);
    assert!((screen.y - 300.0).abs() < 0.001);
}

#[test]
fn test_viewport_coordinate_roundtrip() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(100.0, 50.0, 800.0, 600.0));
    viewport.set_camera_position(Vec2::new(100.0, -50.0));
    viewport.set_camera_zoom(2.0);

    let original_screen = Vec2::new(350.0, 200.0);
    let world = viewport.screen_to_world(original_screen);
    let back_to_screen = viewport.world_to_screen(world);

    assert!((back_to_screen.x - original_screen.x).abs() < 0.001);
    assert!((back_to_screen.y - original_screen.y).abs() < 0.001);
}

#[test]
fn test_viewport_visible_world_bounds() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));

    let (min_x, min_y, max_x, max_y) = viewport.visible_world_bounds();
    assert_eq!(min_x, -400.0);
    assert_eq!(min_y, -300.0);
    assert_eq!(max_x, 400.0);
    assert_eq!(max_y, 300.0);
}

#[test]
fn test_viewport_visible_world_bounds_with_zoom() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));
    viewport.set_camera_zoom(2.0);

    let (min_x, min_y, max_x, max_y) = viewport.visible_world_bounds();
    // At 2x zoom, visible area is halved
    assert_eq!(min_x, -200.0);
    assert_eq!(min_y, -150.0);
    assert_eq!(max_x, 200.0);
    assert_eq!(max_y, 150.0);
}

#[test]
fn test_viewport_pan() {
    let mut viewport = SceneViewport::new();
    viewport.pan_immediate(Vec2::new(50.0, -25.0));

    assert_eq!(viewport.camera_position(), Vec2::new(50.0, -25.0));
}

#[test]
fn test_viewport_zoom_clamp() {
    let mut viewport = SceneViewport::new();

    viewport.set_camera_zoom(0.01);
    assert_eq!(viewport.camera_zoom(), 0.1); // Clamped to min

    viewport.set_camera_zoom(100.0);
    assert_eq!(viewport.camera_zoom(), 10.0); // Clamped to max
}

#[test]
fn test_update_converges_camera_on_targets() {
    // The audit's dead wire: zoom_at/pan/reset_camera write target_*
    // only — update() is what moves the live camera toward them.
    let mut viewport = SceneViewport::new();
    viewport.set_target_camera_position(Vec2::new(100.0, -40.0));
    viewport.set_target_zoom(2.0);

    for _ in 0..120 {
        viewport.update(1.0 / 60.0);
    }
    assert!((viewport.camera_position() - Vec2::new(100.0, -40.0)).length() < 0.01);
    assert!((viewport.camera_zoom() - 2.0).abs() < 0.001);
}

#[test]
fn test_update_is_frame_rate_independent() {
    // Same wall-clock time must cover the same distance regardless of
    // step size: two 1/120s steps compose to exactly one 1/60s step.
    let mut coarse = SceneViewport::new();
    let mut fine = SceneViewport::new();
    for v in [&mut coarse, &mut fine] {
        v.set_target_camera_position(Vec2::new(100.0, 0.0));
        v.set_target_zoom(3.0);
    }

    coarse.update(1.0 / 60.0);
    fine.update(1.0 / 120.0);
    fine.update(1.0 / 120.0);

    assert!((coarse.camera_position() - fine.camera_position()).length() < 0.001);
    assert!((coarse.camera_zoom() - fine.camera_zoom()).abs() < 0.001);
}

#[test]
fn test_update_with_zero_delta_leaves_camera_unchanged() {
    let mut viewport = SceneViewport::new();
    viewport.set_target_camera_position(Vec2::new(100.0, 0.0));

    viewport.update(0.0);
    assert_eq!(viewport.camera_position(), Vec2::ZERO, "no time passed, no movement");
}

#[test]
fn test_viewport_reset_camera() {
    let mut viewport = SceneViewport::new();
    viewport.set_camera_position(Vec2::new(100.0, 200.0));
    viewport.set_camera_zoom(3.0);

    viewport.reset_camera_immediate();

    assert_eq!(viewport.camera_position(), Vec2::ZERO);
    assert_eq!(viewport.camera_zoom(), 1.0);
}

#[test]
fn test_viewport_contains_screen_point() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(100.0, 50.0, 400.0, 300.0));

    assert!(viewport.contains_screen_point(Vec2::new(200.0, 150.0)));
    assert!(!viewport.contains_screen_point(Vec2::new(50.0, 150.0)));
    assert!(!viewport.contains_screen_point(Vec2::new(200.0, 400.0)));
}

/// Assert the viewport overlay mapping and the GPU render-camera mapping
/// agree for a set of world points. This equivalence is the contract that
/// keeps gizmo/picking/grid aligned with the rendered sprites.
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
fn test_window_render_camera_matches_overlay_default_view() {
    // Regression: with a NONZERO panel origin (dock chrome on all sides),
    // the old to_render_camera produced a panel_center-to-window_center
    // offset between sprites and the editor overlay.
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(300.0, 100.0, 800.0, 600.0));
    assert_overlay_matches_render_camera(&viewport, Vec2::new(1600.0, 900.0));
}

#[test]
fn test_window_render_camera_matches_overlay_pan_zoom() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(300.0, 100.0, 800.0, 600.0));
    viewport.set_camera_position(Vec2::new(120.0, -40.0));
    viewport.set_camera_zoom(2.0);
    assert_overlay_matches_render_camera(&viewport, Vec2::new(1600.0, 900.0));
}

#[test]
fn test_window_render_camera_matches_overlay_play_follow_pose() {
    // Issue #42: while following the game camera during Play, the
    // viewport adopts the main-camera entity's position AND zoom. The
    // overlay/GPU equivalence must hold for that adopted pose too —
    // picking stays truthful under camera-zoom gameplay.
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(300.0, 100.0, 800.0, 600.0));
    // Exactly what sync_viewport_from_main_camera writes while following:
    viewport.set_camera_position(Vec2::new(320.0, -75.0));
    viewport.set_camera_zoom(2.5);
    assert_overlay_matches_render_camera(&viewport, Vec2::new(1600.0, 900.0));
}

#[test]
fn test_window_render_camera_screen_roundtrip() {
    // viewport.screen_to_world must be the inverse of the GPU camera's
    // world_to_screen, so clicks land on the sprite under the cursor.
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(300.0, 100.0, 800.0, 600.0));
    viewport.set_camera_position(Vec2::new(-60.0, 25.0));
    viewport.set_camera_zoom(1.5);
    let camera = viewport.to_window_render_camera(Vec2::new(1600.0, 900.0));

    for screen in [
        Vec2::new(700.0, 400.0),
        Vec2::new(310.0, 110.0),
        Vec2::new(1050.0, 650.0),
    ] {
        let world = viewport.screen_to_world(screen);
        let back = camera.world_to_screen(world);
        assert!(
            (back - screen).length() < 0.01,
            "roundtrip mismatch at {screen}: got {back}"
        );
    }
}

#[test]
fn test_viewport_focus_on() {
    let mut viewport = SceneViewport::new();
    viewport.focus_on(Vec2::new(500.0, 300.0));

    // Target should be set (actual position updates on update())
    viewport.update(0.016);

    // After interpolation, should be moving toward target
    let pos = viewport.camera_position();
    assert!(pos.x > 0.0); // Moving toward 500
    assert!(pos.y > 0.0); // Moving toward 300
}

#[test]
fn test_focus_on_bounds_targets_center_and_zooms_to_fit() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));

    let positions = vec![Vec2::new(-90.0, -40.0), Vec2::new(110.0, 60.0)];
    viewport.focus_on_bounds(&positions);

    // Targets are what focus writes; the camera itself interpolates later.
    assert_eq!(viewport.target_camera_position(), Vec2::new(10.0, 10.0));
    // Zoom-to-fit: min(800/(200+100), 600/(100+100)) = min(2.667, 3.0)
    assert!((viewport.target_camera_zoom() - 800.0 / 300.0).abs() < 0.001);
}

#[test]
fn test_focus_on_bounds_with_no_positions_leaves_targets_unchanged() {
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));
    viewport.set_target_camera_position(Vec2::new(42.0, 7.0));

    viewport.focus_on_bounds(&[]);

    assert_eq!(viewport.target_camera_position(), Vec2::new(42.0, 7.0));
    assert_eq!(viewport.target_camera_zoom(), 1.0);
}
