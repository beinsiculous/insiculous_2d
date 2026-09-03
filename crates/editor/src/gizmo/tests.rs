//! Gizmo behavior: drags simulated headlessly through the real ui
//! interaction machinery (press/move/release frames), locking the annulus
//! hit-test, cumulative deltas, the release flag, the per-axis scale ratio
//! and the cancel latch.

use super::*;
use crate::test_support::{move_to, press_at, release, WINDOW};
use input::InputHandler;

/// The gizmo sits at the center of [`WINDOW`].
const CENTER: Vec2 = Vec2::new(WINDOW.x * 0.5, WINDOW.y * 0.5);

#[test]
fn test_translate_drag_is_cumulative_from_the_press_axis_locked_and_released_once() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    let start = press_at(&mut ui, &mut input, CENTER, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(start.handle, Some(GizmoHandle::Center));
    assert!(gizmo.is_active());

    let mid = move_to(&mut ui, &mut input, CENTER + Vec2::new(4.0, 2.0), |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(mid.translation, Vec2::new(4.0, 2.0));
    // The cursor leaves the scene panel mid-drag (interactive = false):
    // the in-flight drag keeps reporting, cumulative from the press.
    let far = move_to(&mut ui, &mut input, CENTER + Vec2::new(300.0, 5.0), |ui| gizmo.render(ui, CENTER, false));
    assert_eq!(far.handle, Some(GizmoHandle::Center));
    assert_eq!(far.translation, Vec2::new(300.0, 5.0), "cumulative from the press, not per frame");
    assert!(!far.released);

    let released = release(&mut ui, &mut input, |ui| gizmo.render(ui, CENTER, true));
    assert!(released.released, "the release frame signals the commit point");
    assert_eq!(released.handle, None);
    assert!(!gizmo.is_active());

    // The X arrow projects the drag onto its axis.
    let x_handle = CENTER + Vec2::new(gizmo.axis_length(), 0.0);
    let start = press_at(&mut ui, &mut input, x_handle, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(start.handle, Some(GizmoHandle::AxisX));
    let dragged = move_to(&mut ui, &mut input, x_handle + Vec2::new(7.0, 30.0), |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(dragged.translation, Vec2::new(7.0, 0.0), "X-axis drags drop the Y component");
}

#[test]
fn test_rotate_ring_is_an_annulus_so_a_dead_center_press_falls_through_to_picking() {
    let mut gizmo = Gizmo::new();
    gizmo.set_mode(GizmoMode::Rotate);
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Dead center: inside the old 148px square, far off the band.
    let dead = press_at(&mut ui, &mut input, CENTER, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(dead.handle, None);
    assert!(!gizmo.is_active());
    assert!(!ui.wants_mouse(), "a dead-center press must not claim a widget — it falls through to picking");
    release(&mut ui, &mut input, |ui| gizmo.render(ui, CENTER, true));

    // On the band (radius = axis_length * 0.8): a rotation drag starts and
    // sliding screen-up on the right side reads as CCW-positive.
    let on_ring = CENTER + Vec2::new(gizmo.axis_length() * 0.8, 0.0);
    let start = press_at(&mut ui, &mut input, on_ring, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(start.handle, Some(GizmoHandle::Ring));
    let dragged = move_to(&mut ui, &mut input, CENTER + Vec2::new(60.0, -25.0), |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(dragged.handle, Some(GizmoHandle::Ring));
    assert!(dragged.rotation_delta > 0.0, "got {}", dragged.rotation_delta);
    let released = release(&mut ui, &mut input, |ui| gizmo.render(ui, CENTER, true));
    assert!(released.released);
    assert!(!gizmo.is_active());
}

#[test]
fn test_scale_is_a_per_axis_offset_ratio_that_mirrors_through_the_center_and_floors() {
    let mut gizmo = Gizmo::new();
    gizmo.set_mode(GizmoMode::Scale);
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Corner handles sit at center ± axis_length * 0.6 / 2.
    let half = gizmo.axis_length() * 0.6 / 2.0;
    let corner = CENTER + Vec2::new(half, half);
    let start = press_at(&mut ui, &mut input, corner, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(start.handle, Some(GizmoHandle::ScaleCorner(Corner::BottomRight)));

    // Double the X offset, keep Y: a per-axis multiplicative factor. The
    // drag must survive frame 2 (regression: the old still_dragging check
    // re-interacted with the wrong rect and killed it one frame in).
    let dragged = move_to(&mut ui, &mut input, CENTER + Vec2::new(half * 2.0, half), |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(dragged.handle, Some(GizmoHandle::ScaleCorner(Corner::BottomRight)), "scale drag must survive frame 2");
    assert_eq!(dragged.scale_factor, Vec2::new(2.0, 1.0));

    // Dragging THROUGH the center mirrors via abs() and bottoms out at the
    // 0.01 floor instead of flipping sign or dividing by zero.
    let collapsed = move_to(&mut ui, &mut input, CENTER, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(collapsed.handle, Some(GizmoHandle::ScaleCorner(Corner::BottomRight)));
    assert_eq!(collapsed.scale_factor, Vec2::splat(0.01));
}

#[test]
fn test_cancel_latch_suppresses_rest_of_gesture_until_mouse_up() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    press_at(&mut ui, &mut input, CENTER, |ui| gizmo.render(ui, CENTER, true));
    assert!(gizmo.is_active());

    // Escape mid-drag
    gizmo.cancel();
    assert!(!gizmo.is_active());

    // Mouse still held: the gesture must not resume
    let while_held = move_to(&mut ui, &mut input, CENTER + Vec2::new(15.0, 0.0), |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(while_held.handle, None, "cancelled gesture must stay dead");
    assert!(!gizmo.is_active());

    // Release clears the latch (polled state); a fresh press drags again
    release(&mut ui, &mut input, |ui| gizmo.render(ui, CENTER, true));
    let fresh = press_at(&mut ui, &mut input, CENTER, |ui| gizmo.render(ui, CENTER, true));
    assert_eq!(fresh.handle, Some(GizmoHandle::Center), "latch must clear on release");
}

#[test]
fn test_switching_tool_mid_drag_releases_the_stale_handle_instead_of_wedging() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Live translate drag...
    press_at(&mut ui, &mut input, CENTER, |ui| gizmo.render(ui, CENTER, true));
    assert!(gizmo.is_active());

    // ...then the user presses E (Rotate) while still holding the mouse.
    // The rotate renderer manages no Center handle — the stale drag must
    // release (so the caller commits it), never stay active forever.
    gizmo.set_mode(GizmoMode::Rotate);
    move_to(&mut ui, &mut input, CENTER + Vec2::new(5.0, 0.0), |ui| gizmo.render(ui, CENTER, true));
    assert!(!gizmo.is_active(), "stale handle from the old mode must release");
}

#[test]
fn test_outside_the_scene_panel_the_gizmo_draws_but_grabs_nothing() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Press exactly on the center handle while the mouse is outside the
    // scene panel (interactive = false): nothing may start, but the
    // handles still draw — the gizmo is visible, just not grabbable.
    let (interaction, handle_rects) = press_at(&mut ui, &mut input, CENTER, |ui| {
        let interaction = gizmo.render(ui, CENTER, false);
        let rects = ui
            .draw_list()
            .commands()
            .iter()
            .filter(|c| matches!(c, ui::DrawCommand::Rect { .. }))
            .count();
        (interaction, rects)
    });

    assert_eq!(interaction.handle, None);
    assert!(!gizmo.is_active());
    assert!(!ui.wants_mouse());
    assert!(handle_rects > 0, "handles draw even when not interactive");
}

#[test]
fn test_gizmo_palette_default_matches_editor_theme_palette() {
    assert_eq!(GizmoPalette::default(), crate::theme::EditorTheme::default().gizmo_palette());
}
