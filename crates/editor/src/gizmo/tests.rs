//! Gizmo behavior tests: drags simulated headlessly through the real ui
//! interaction machinery (press/move/release frames), locking the annulus
//! hit-test, cumulative deltas, the release flag, and the cancel latch.

use super::*;
use input::prelude::MouseButton;
use input::InputHandler;

const CENTER: Vec2 = Vec2::new(400.0, 300.0);
const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

/// Run one editor frame: begin, render the gizmo, end.
fn frame(
    gizmo: &mut Gizmo,
    ui: &mut UIContext,
    input: &InputHandler,
    interactive: bool,
) -> GizmoInteraction {
    ui.begin_frame(input, WINDOW);
    let interaction = gizmo.render(ui, CENTER, interactive);
    ui.end_frame();
    interaction
}

fn press_at(input: &mut InputHandler, pos: Vec2) {
    input.mouse_mut().update_position(pos.x, pos.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
}

fn move_to(input: &mut InputHandler, pos: Vec2) {
    input.update();
    input.mouse_mut().update_position(pos.x, pos.y);
}

fn release(input: &mut InputHandler) {
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
}

#[test]
fn test_gizmo_mode_default() {
    assert_eq!(GizmoMode::default(), GizmoMode::Translate);
}

#[test]
fn test_gizmo_mode_names() {
    assert_eq!(GizmoMode::None.name(), "None");
    assert_eq!(GizmoMode::Translate.name(), "Translate");
    assert_eq!(GizmoMode::Rotate.name(), "Rotate");
    assert_eq!(GizmoMode::Scale.name(), "Scale");
}

#[test]
fn test_gizmo_new() {
    let gizmo = Gizmo::new();
    assert_eq!(gizmo.mode(), GizmoMode::Translate);
    assert_eq!(gizmo.position(), Vec2::ZERO);
    assert!(!gizmo.is_active());
}

#[test]
fn test_gizmo_interaction_default() {
    let interaction = GizmoInteraction::default();
    assert!(interaction.handle.is_none());
    assert_eq!(interaction.translation, Vec2::ZERO);
    assert_eq!(interaction.rotation_delta, 0.0);
    assert_eq!(interaction.scale_factor, Vec2::ONE);
    assert!(!interaction.released);
}

#[test]
fn test_translate_drag_reports_cumulative_offset_and_release() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Press on the center handle
    press_at(&mut input, CENTER);
    let start = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(start.handle, Some(GizmoHandle::Center));
    assert!(gizmo.is_active());

    // Two small moves — the reported translation is cumulative from the
    // drag start, not a per-frame delta
    move_to(&mut input, CENTER + Vec2::new(4.0, 2.0));
    let mid = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(mid.translation, Vec2::new(4.0, 2.0));

    move_to(&mut input, CENTER + Vec2::new(10.0, 5.0));
    let end = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(end.translation, Vec2::new(10.0, 5.0));
    assert!(!end.released);

    release(&mut input);
    let released = frame(&mut gizmo, &mut ui, &input, true);
    assert!(released.released, "release frame must signal the commit point");
    assert!(released.handle.is_none());
    assert!(!gizmo.is_active());
}

#[test]
fn test_translate_axis_handle_projects_to_its_axis() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Press on the X arrow (at center + axis_length along +X)
    let x_handle = CENTER + Vec2::new(gizmo.axis_length(), 0.0);
    press_at(&mut input, x_handle);
    let start = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(start.handle, Some(GizmoHandle::AxisX));

    move_to(&mut input, x_handle + Vec2::new(7.0, 30.0));
    let dragged = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(
        dragged.translation,
        Vec2::new(7.0, 0.0),
        "X-axis drags must drop the Y component"
    );
}

#[test]
fn test_rotate_ring_center_press_claims_nothing() {
    let mut gizmo = Gizmo::new();
    gizmo.set_mode(GizmoMode::Rotate);
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Dead center of the ring: inside the old 148px square, far off the band
    press_at(&mut input, CENTER);
    let interaction = frame(&mut gizmo, &mut ui, &input, true);

    assert!(interaction.handle.is_none());
    assert!(!gizmo.is_active());
    assert!(
        !ui.wants_mouse(),
        "a dead-center press must not claim a widget — it falls through to picking"
    );
}

#[test]
fn test_rotate_ring_band_press_starts_rotation_drag() {
    let mut gizmo = Gizmo::new();
    gizmo.set_mode(GizmoMode::Rotate);
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // On the band: ring radius is axis_length * 0.8
    let on_ring = CENTER + Vec2::new(gizmo.axis_length() * 0.8, 0.0);
    press_at(&mut input, on_ring);
    let start = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(start.handle, Some(GizmoHandle::Ring));

    // Slide along the ring — a rotation delta is reported
    move_to(&mut input, CENTER + Vec2::new(60.0, -25.0));
    let dragged = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(dragged.handle, Some(GizmoHandle::Ring));
    assert!(dragged.rotation_delta != 0.0);

    release(&mut input);
    let released = frame(&mut gizmo, &mut ui, &input, true);
    assert!(released.released);
    assert!(!gizmo.is_active());
}

#[test]
fn test_scale_drag_survives_past_the_first_frame() {
    let mut gizmo = Gizmo::new();
    gizmo.set_mode(GizmoMode::Scale);
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Corner handles sit at center ± axis_length * 0.6 / 2
    let half = gizmo.axis_length() * 0.6 / 2.0;
    let corner = CENTER + Vec2::new(half, half); // BottomRight
    press_at(&mut input, corner);
    let start = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(start.handle, Some(GizmoHandle::ScaleCorner(Corner::BottomRight)));

    // Regression: the old still_dragging check re-interacted with the wrong
    // rect and killed the drag one frame in.
    move_to(&mut input, corner + Vec2::new(5.0, 5.0));
    let second = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(
        second.handle,
        Some(GizmoHandle::ScaleCorner(Corner::BottomRight)),
        "scale drag must survive frame 2"
    );

    move_to(&mut input, corner + Vec2::new(6.0, 6.0));
    let third = frame(&mut gizmo, &mut ui, &input, true);
    assert!(third.handle.is_some(), "and keep going");
}

#[test]
fn test_scale_factor_is_offset_ratio_per_axis() {
    let mut gizmo = Gizmo::new();
    gizmo.set_mode(GizmoMode::Scale);
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    let half = gizmo.axis_length() * 0.6 / 2.0; // 24px
    let corner = CENTER + Vec2::new(half, half);
    press_at(&mut input, corner);
    frame(&mut gizmo, &mut ui, &input, true);

    // Double the X offset, keep Y: per-axis multiplicative factor
    move_to(&mut input, CENTER + Vec2::new(half * 2.0, half));
    let dragged = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(dragged.scale_factor, Vec2::new(2.0, 1.0));

    // Dragging THROUGH the center mirrors via abs() and bottoms out at the
    // 0.01 floor instead of flipping sign or dividing by zero
    move_to(&mut input, CENTER);
    let collapsed = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(collapsed.scale_factor, Vec2::splat(0.01));
}

#[test]
fn test_cancel_latch_suppresses_rest_of_gesture_until_mouse_up() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    press_at(&mut input, CENTER);
    frame(&mut gizmo, &mut ui, &input, true);
    assert!(gizmo.is_active());

    // Escape mid-drag
    gizmo.cancel();
    assert!(!gizmo.is_active());

    // Mouse still held: the gesture must not resume
    move_to(&mut input, CENTER + Vec2::new(15.0, 0.0));
    let while_held = frame(&mut gizmo, &mut ui, &input, true);
    assert!(while_held.handle.is_none(), "cancelled gesture must stay dead");
    assert!(!gizmo.is_active());

    // Release clears the latch (polled state), a fresh press drags again
    release(&mut input);
    frame(&mut gizmo, &mut ui, &input, true);
    input.update();
    press_at(&mut input, CENTER);
    let fresh = frame(&mut gizmo, &mut ui, &input, true);
    assert_eq!(fresh.handle, Some(GizmoHandle::Center), "latch must clear on release");
}

#[test]
fn test_tool_switch_mid_drag_releases_the_stale_handle() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Live translate drag...
    press_at(&mut input, CENTER);
    frame(&mut gizmo, &mut ui, &input, true);
    assert!(gizmo.is_active());

    // ...then the user presses E (Rotate) while still holding the mouse.
    // The rotate renderer manages no Center handle — the stale drag must
    // release (so the caller commits it), never wedge active forever.
    gizmo.set_mode(GizmoMode::Rotate);
    move_to(&mut input, CENTER + Vec2::new(5.0, 0.0));
    frame(&mut gizmo, &mut ui, &input, true);
    assert!(!gizmo.is_active(), "stale handle from the old mode must release");
}

#[test]
fn test_non_interactive_render_claims_no_widget() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    // Press exactly on the center handle, but the mouse is outside the
    // scene panel (interactive = false): nothing may start
    press_at(&mut input, CENTER);
    let interaction = frame(&mut gizmo, &mut ui, &input, false);
    assert!(interaction.handle.is_none());
    assert!(!gizmo.is_active());
    assert!(!ui.wants_mouse());

    // Visuals still draw (the gizmo is visible, just not grabbable)
    ui.begin_frame(&input, WINDOW);
    gizmo.render(&mut ui, CENTER, false);
    let rects = ui
        .draw_list()
        .commands()
        .iter()
        .filter(|c| matches!(c, ui::DrawCommand::Rect { .. }))
        .count();
    assert!(rects > 0, "handles draw even when not interactive");
    ui.end_frame();
}

#[test]
fn test_live_drag_survives_leaving_the_interactive_area() {
    let mut gizmo = Gizmo::new();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();

    press_at(&mut input, CENTER);
    frame(&mut gizmo, &mut ui, &input, true);
    assert!(gizmo.is_active());

    // Cursor leaves the panel mid-drag: interactive goes false but the
    // in-flight drag keeps reporting
    move_to(&mut input, CENTER + Vec2::new(300.0, 0.0));
    let dragged = frame(&mut gizmo, &mut ui, &input, false);
    assert_eq!(dragged.handle, Some(GizmoHandle::Center));
    assert_eq!(dragged.translation, Vec2::new(300.0, 0.0));
}
