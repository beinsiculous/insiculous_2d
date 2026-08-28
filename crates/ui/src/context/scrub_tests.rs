//! Behavior tests for the float-input numeric UX (#30): drag-scrub, arrow
//! nudge, soft vs hard ranges, the invalid-buffer state, and the display
//! suffix.

use glam::Vec2;
use input::prelude::{KeyCode, MouseButton};

use crate::{DrawCommand, FloatFieldOpts, FloatInputResult, Rect, UIContext};

const BOUNDS: Rect = Rect { x: 100.0, y: 100.0, width: 80.0, height: 20.0 };
const CENTER: Vec2 = Vec2::new(140.0, 110.0);

fn frame(
    ui: &mut UIContext,
    input: &input::InputHandler,
    value: f32,
    opts: FloatFieldOpts,
) -> FloatInputResult {
    ui.begin_frame(input, Vec2::new(800.0, 600.0));
    let out = ui.float_input("scrub_field", value, opts, BOUNDS);
    ui.end_frame();
    out
}

fn opts() -> FloatFieldOpts {
    FloatFieldOpts::range(-100.0, 100.0)
}

/// Press at the field center; returns the press-frame result.
fn press(ui: &mut UIContext, input: &mut input::InputHandler, value: f32) -> FloatInputResult {
    input.mouse_mut().update_position(CENTER.x, CENTER.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    frame(ui, input, value, opts())
}

/// Move the held pointer by `dx` pixels; returns that frame's result.
fn drag(ui: &mut UIContext, input: &mut input::InputHandler, value: f32, dx: f32) -> FloatInputResult {
    input.update();
    input.mouse_mut().update_position(CENTER.x + dx, CENTER.y);
    frame(ui, input, value, opts())
}

fn release(ui: &mut UIContext, input: &mut input::InputHandler, value: f32) -> FloatInputResult {
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    frame(ui, input, value, opts())
}

#[test]
fn test_float_scrub_requires_threshold_click_still_focuses() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    // Press + release with sub-threshold travel = a plain click-to-focus.
    let r = press(&mut ui, &mut input, 5.0);
    assert!(!r.scrubbing && !r.changed);
    let r = drag(&mut ui, &mut input, 5.0, 2.0); // below the 4px threshold
    assert!(!r.scrubbing, "sub-threshold travel must not scrub");
    assert_eq!(r.value, 5.0);
    let r = release(&mut ui, &mut input, 5.0);
    assert!(!r.committed, "a click is not an edit gesture");
    assert!(ui.wants_keyboard(), "sub-threshold release focuses the field");
}

#[test]
fn test_float_scrub_emits_per_frame_values_and_commits_on_release() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    press(&mut ui, &mut input, 5.0);
    let r = drag(&mut ui, &mut input, 5.0, 10.0);
    assert!(r.scrubbing && r.changed);
    assert_eq!(r.value, 15.0, "step 1.0 → 10px = +10");

    // The editor wrote 15.0 back; further travel scrubs from the START value.
    let r = drag(&mut ui, &mut input, 15.0, 20.0);
    assert!(r.scrubbing);
    assert_eq!(r.value, 25.0, "scrub output is start_value + dx, not compounding");

    let r = release(&mut ui, &mut input, 25.0);
    assert!(r.committed, "release seals the gesture (undo-merge boundary)");
    assert!(!r.scrubbing);
    assert!(!ui.wants_keyboard(), "a scrub must not focus the field");
    assert!(ui.take_edit_commit(), "the commit flag reaches the host");
}

#[test]
fn test_float_scrub_clamps_to_soft_range() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    press(&mut ui, &mut input, 95.0);
    let r = drag(&mut ui, &mut input, 95.0, 50.0);
    assert_eq!(r.value, 100.0, "scrub clamps at the soft max");
}

#[test]
fn test_float_scrub_escape_restores_start_value() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    press(&mut ui, &mut input, 5.0);
    let r = drag(&mut ui, &mut input, 5.0, 30.0);
    assert_eq!(r.value, 35.0);

    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::Escape);
    let r = frame(&mut ui, &input, 35.0, opts());
    assert_eq!(r.value, 5.0, "escape restores the pre-scrub value");
    assert!(r.changed && r.committed);
}

#[test]
fn test_float_scrub_press_reseeds_state() {
    // Kimi plan-review F10 lock: arming on press re-seeds press_x and
    // start_value, so a previous gesture (possibly on another entity whose
    // field shares this widget id) can never leak into a new one.
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    press(&mut ui, &mut input, 5.0);
    drag(&mut ui, &mut input, 5.0, 30.0);
    release(&mut ui, &mut input, 35.0);
    ui.take_edit_commit();

    // Second gesture on the same id with a different value: output derives
    // from THIS press's value, not the last gesture's.
    press(&mut ui, &mut input, 70.0);
    let r = drag(&mut ui, &mut input, 70.0, 10.0);
    assert_eq!(r.value, 80.0, "second gesture scrubs from its own press value");
}

#[test]
fn test_float_arrow_up_down_steps_value_shift_is_10x() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let o = opts().with_step(0.5);

    // Focus via click.
    press(&mut ui, &mut input, 5.0);
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    frame(&mut ui, &input, 5.0, o);
    assert!(ui.wants_keyboard());

    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::ArrowUp);
    let r = frame(&mut ui, &input, 5.0, o);
    assert!(r.changed);
    assert_eq!(r.value, 5.5, "ArrowUp nudges by the step");
    input.keyboard_mut().handle_key_release(KeyCode::ArrowUp);

    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::ShiftLeft);
    input.keyboard_mut().handle_key_press(KeyCode::ArrowDown);
    let r = frame(&mut ui, &input, 5.5, o);
    assert_eq!(r.value, 0.5, "Shift+ArrowDown nudges by 10× the step");
    input.keyboard_mut().handle_key_release(KeyCode::ArrowDown);
    input.keyboard_mut().handle_key_release(KeyCode::ShiftLeft);
}

#[test]
fn test_float_invalid_buffer_flags_and_reverts_on_commit() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    // Focus, then type a minus over the selected text → "-" alone is not a
    // number: the field flags invalid while focused...
    press(&mut ui, &mut input, 5.0);
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    frame(&mut ui, &input, 5.0, opts());

    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::Minus);
    let r = frame(&mut ui, &input, 5.0, opts());
    assert!(r.invalid, "a non-numeric buffer must be flagged");
    input.keyboard_mut().handle_key_release(KeyCode::Minus);

    // ...and an Enter commit reverts to the pre-edit value instead of
    // committing garbage or silently keeping half a number.
    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::Enter);
    let r = frame(&mut ui, &input, 5.0, opts());
    assert!(r.committed);
    assert!(!r.changed, "revert is not a change");
    assert_eq!(r.value, 5.0);
}

#[test]
fn test_float_suffix_renders_but_never_enters_the_buffer() {
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    let o = opts().with_suffix("°");

    ui.begin_frame(&input, Vec2::new(800.0, 600.0));
    ui.float_input("deg_field", 90.0, o, BOUNDS);
    let has_suffix = ui.draw_list().commands().iter().any(|c| match c {
        DrawCommand::TextPlaceholder { text, .. } => text == "90.00°",
        DrawCommand::Text { data, .. } => data.text == "90.00°",
        _ => false,
    });
    ui.end_frame();
    assert!(has_suffix, "display shows the suffix");

    // Focus it: the edit buffer is the bare number.
    let mut input = input::InputHandler::new();
    let mut ui2 = UIContext::new();
    input.mouse_mut().update_position(CENTER.x, CENTER.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    frame(&mut ui2, &input, 90.0, o);
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    frame(&mut ui2, &input, 90.0, o);
    assert!(ui2.wants_keyboard());
    // Commit untouched: parses cleanly (a suffix in the buffer would fail).
    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::Enter);
    let r = frame(&mut ui2, &input, 90.0, o);
    assert!(r.committed && !r.changed);
    assert_eq!(r.value, 90.0);
}

// === typed-commit range semantics (moved from tests.rs for file size) ===

fn type_and_commit(
    ui: &mut UIContext,
    input: &mut input::InputHandler,
    id: &str,
    bounds: Rect,
    value: f32,
    opts: crate::FloatFieldOpts,
    keys: &[input::prelude::KeyCode],
) -> crate::FloatInputResult {
    use input::prelude::{KeyCode, MouseButton};
    // Focus with a raw two-frame click (opts differ per test).
    let center = Vec2::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
    input.mouse_mut().update_position(center.x, center.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    ui.begin_frame(&*input, Vec2::new(800.0, 600.0));
    ui.float_input(id, value, opts, bounds);
    ui.end_frame();
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    ui.begin_frame(&*input, Vec2::new(800.0, 600.0));
    ui.float_input(id, value, opts, bounds);
    ui.end_frame();

    for &key in keys {
        input.update();
        input.keyboard_mut().handle_key_press(key);
        ui.begin_frame(&*input, Vec2::new(800.0, 600.0));
        ui.float_input(id, value, opts, bounds);
        ui.end_frame();
        input.keyboard_mut().handle_key_release(key);
    }
    input.update();
    input.keyboard_mut().handle_key_press(KeyCode::Enter);
    ui.begin_frame(&*input, Vec2::new(800.0, 600.0));
    let committed = ui.float_input(id, value, opts, bounds);
    ui.end_frame();
    input.keyboard_mut().handle_key_release(KeyCode::Enter);
    committed
}

#[test]
fn test_float_typed_commit_beyond_soft_range_not_clamped() {
    use input::prelude::KeyCode;
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = Rect::new(10.0, 10.0, 80.0, 20.0);

    // Soft range 0..=10: a typed 99 is ACCEPTED — the audit's "type 1500,
    // get 1000" silent clamp is gone; the field reports the real value.
    let committed = type_and_commit(
        &mut ui, &mut input, "soft", bounds, 5.0,
        crate::FloatFieldOpts::range(0.0, 10.0),
        &[KeyCode::Digit9, KeyCode::Digit9],
    );
    assert!(committed.committed);
    assert!(committed.changed);
    assert_eq!(committed.value, 99.0, "soft range must not clamp typed commits");
}

#[test]
fn test_float_hard_clamp_clamps_typed_commit() {
    use input::prelude::KeyCode;
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = Rect::new(10.0, 10.0, 80.0, 20.0);

    // hard_clamp (color channels): the old clamping behavior on commit.
    let committed = type_and_commit(
        &mut ui, &mut input, "hard", bounds, 5.0,
        crate::FloatFieldOpts::hard(0.0, 10.0),
        &[KeyCode::Digit9, KeyCode::Digit9],
    );
    assert_eq!(committed.value, 10.0, "99 must clamp to the max of 10");
}
