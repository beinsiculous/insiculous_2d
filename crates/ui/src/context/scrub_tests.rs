//! Contract tests for the float-input numeric UX: drag-scrub,
//! arrow nudge, soft vs hard ranges on typed commits, the invalid-buffer
//! state, and the display suffix.

use glam::Vec2;
use input::prelude::KeyCode;

use crate::test_support::{focus_field, frame, move_to, press_at, release, type_key};
use crate::{DrawCommand, FloatFieldOpts, FloatInputResult, Rect, UIContext};

const BOUNDS: Rect = Rect { x: 100.0, y: 100.0, width: 80.0, height: 20.0 };

fn opts() -> FloatFieldOpts {
    FloatFieldOpts::range(-100.0, 100.0)
}

/// The field under test, submitted with `value` and `opts`.
fn field(value: f32, opts: FloatFieldOpts) -> impl FnMut(&mut UIContext) -> FloatInputResult {
    move |ui| ui.float_input("scrub_field", value, opts, BOUNDS)
}

/// Press at the field center: the frame that arms a scrub.
fn press(ui: &mut UIContext, input: &mut input::InputHandler, value: f32) -> FloatInputResult {
    press_at(ui, input, BOUNDS.center(), field(value, opts()))
}

/// Move the held pointer `dx` pixels from the press point.
fn drag(ui: &mut UIContext, input: &mut input::InputHandler, value: f32, dx: f32) -> FloatInputResult {
    move_to(ui, input, BOUNDS.center() + Vec2::new(dx, 0.0), field(value, opts()))
}

/// Focus the field, type `keys`, and commit with Enter.
fn type_and_commit(
    ui: &mut UIContext,
    input: &mut input::InputHandler,
    value: f32,
    opts: FloatFieldOpts,
    keys: &[KeyCode],
) -> FloatInputResult {
    focus_field(ui, input, BOUNDS, field(value, opts));
    for &key in keys {
        type_key(ui, input, key, field(value, opts));
    }
    type_key(ui, input, KeyCode::Enter, field(value, opts))
}

// ================== Drag-scrub ==================

#[test]
fn test_float_scrub_arms_past_the_threshold_scrubs_from_the_press_value_and_commits_on_release() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    let armed = press(&mut ui, &mut input, 5.0);
    assert!(!armed.scrubbing && !armed.changed, "the press frame changes nothing");
    let r = drag(&mut ui, &mut input, 5.0, 10.0);
    assert!(r.scrubbing && r.changed);
    assert_eq!(r.value, 15.0, "step 1.0 → 10px = +10");

    // The host wrote 15.0 back; further travel still scrubs from the START value.
    let r = drag(&mut ui, &mut input, 15.0, 30.0);
    assert_eq!(r.value, 35.0, "scrub output is start_value + dx, not compounding");

    let r = release(&mut ui, &mut input, field(35.0, opts()));
    assert!(r.committed && !r.scrubbing, "release seals the gesture (undo-merge boundary)");
    assert!(!ui.wants_keyboard(), "a scrub must not focus the field");
    assert!(ui.take_edit_commit(), "the commit flag reaches the host");

    // A second gesture on the same id re-seeds from ITS press value (a
    // stale press_x/start_value from the last gesture — possibly another
    // entity's field sharing this widget id — can never leak in)...
    press(&mut ui, &mut input, 70.0);
    assert_eq!(drag(&mut ui, &mut input, 70.0, 10.0).value, 80.0);
    // ...and clamps to the soft range.
    assert_eq!(drag(&mut ui, &mut input, 80.0, 50.0).value, 100.0, "scrub clamps at the soft max");
}

#[test]
fn test_float_scrub_sub_threshold_press_is_a_click_that_focuses() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    press(&mut ui, &mut input, 5.0);
    let r = drag(&mut ui, &mut input, 5.0, 2.0); // below the 4px threshold
    assert!(!r.scrubbing && !r.changed, "sub-threshold travel must not scrub");
    assert_eq!(r.value, 5.0);

    let r = release(&mut ui, &mut input, field(5.0, opts()));
    assert!(!r.committed, "a click is not an edit gesture");
    assert!(ui.wants_keyboard(), "the sub-threshold release focuses the field");
}

#[test]
fn test_float_scrub_escape_restores_the_start_value() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    press(&mut ui, &mut input, 5.0);
    assert_eq!(drag(&mut ui, &mut input, 5.0, 30.0).value, 35.0);

    let r = type_key(&mut ui, &mut input, KeyCode::Escape, field(35.0, opts()));
    assert_eq!(r.value, 5.0, "escape restores the pre-scrub value");
    assert!(r.changed && r.committed && !r.scrubbing);
}

#[test]
fn test_ctrl_scrub_snaps_to_whole_steps_even_in_shift_fine_mode() {
    // Ctrl-held scrubbing lands on exact multiples of the step.
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    input.keyboard_mut().handle_key_press(KeyCode::ControlLeft);

    press(&mut ui, &mut input, 5.0);
    let r = drag(&mut ui, &mut input, 5.0, 4.3); // raw 9.3
    assert!(r.scrubbing);
    assert_eq!(r.value, 9.0, "ctrl snaps to whole steps");

    // Releasing Ctrl mid-scrub resumes smooth values.
    input.keyboard_mut().handle_key_release(KeyCode::ControlLeft);
    let r = drag(&mut ui, &mut input, 9.0, 4.3);
    assert!((r.value - 9.3).abs() < 1e-4, "smooth again without ctrl: {}", r.value);

    // Ctrl beats Shift's ×0.1 fine mode: snap applies to the value the
    // modifiers produced (fine dx of 12.3px → +1.23 → snapped to +1.0).
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    input.keyboard_mut().handle_key_press(KeyCode::ControlRight); // either Ctrl
    input.keyboard_mut().handle_key_press(KeyCode::ShiftLeft);
    press(&mut ui, &mut input, 5.0);
    assert_eq!(drag(&mut ui, &mut input, 5.0, 12.3).value, 6.0, "ctrl+shift: fine value 6.23 snaps to 6.0");
}

// ================== Arrow nudge ==================

#[test]
fn test_float_arrow_up_down_steps_the_value_and_shift_is_10x() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let stepped = opts().with_step(0.5);

    focus_field(&mut ui, &mut input, BOUNDS, field(5.0, stepped));

    let r = type_key(&mut ui, &mut input, KeyCode::ArrowUp, field(5.0, stepped));
    assert!(r.changed);
    assert_eq!(r.value, 5.5, "ArrowUp nudges by the step");

    input.keyboard_mut().handle_key_press(KeyCode::ShiftLeft);
    let r = type_key(&mut ui, &mut input, KeyCode::ArrowDown, field(5.5, stepped));
    assert_eq!(r.value, 0.5, "Shift+ArrowDown nudges by 10× the step");
}

// ================== Typed commits ==================

#[test]
fn test_float_typed_commit_soft_range_accepts_and_flags_while_hard_range_clamps_quietly() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let nines = [KeyCode::Digit9, KeyCode::Digit9];

    // Soft range 0..=10: a typed 99 is ACCEPTED — the audit's "type 1500,
    // get 1000" silent clamp is gone — and reported so the host can warn.
    let soft = type_and_commit(&mut ui, &mut input, 5.0, FloatFieldOpts::range(0.0, 10.0), &nines);
    assert!(soft.committed && soft.changed);
    assert_eq!(soft.value, 99.0, "a soft range must not clamp typed commits");
    assert!(soft.out_of_range, "99 lies outside the soft 0..=10");

    let in_range = type_and_commit(&mut ui, &mut input, 5.0, FloatFieldOpts::range(0.0, 10.0), &[KeyCode::Digit7]);
    assert_eq!(in_range.value, 7.0);
    assert!(!in_range.out_of_range, "in-range commits stay quiet");

    // hard_clamp (color channels): the clamp is the contract, so nothing to warn about.
    let hard = type_and_commit(&mut ui, &mut input, 5.0, FloatFieldOpts::hard(0.0, 10.0), &nines);
    assert_eq!(hard.value, 10.0, "99 must clamp to the max of 10");
    assert!(!hard.out_of_range);
}

#[test]
fn test_float_commit_of_an_unchanged_or_unparsable_buffer_reverts_and_never_flags() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let soft = FloatFieldOpts::range(0.0, 10.0);

    // A value ALREADY outside the soft range (a scene
    // author's 500 on a 0..=10 field) is not the user's doing — Enter
    // with nothing typed must stay quiet...
    let untouched = type_and_commit(&mut ui, &mut input, 500.0, soft, &[]);
    assert!(untouched.committed && !untouched.changed);
    assert!(!untouched.out_of_range, "no new value was typed");

    // ...and so must a parse failure, which reverts to that value.
    let garbage = type_and_commit(&mut ui, &mut input, 500.0, soft, &[KeyCode::KeyX]);
    assert_eq!(garbage.value, 500.0, "reverted");
    assert!(!garbage.changed && !garbage.out_of_range);
}

#[test]
fn test_float_invalid_buffer_flags_red_and_reverts_on_commit() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let invalid_border = ui.theme().text_input.border_invalid;

    // Focus, then type a minus over the selected text → "-" alone is not a
    // number: the field flags invalid and draws the red border...
    focus_field(&mut ui, &mut input, BOUNDS, field(5.0, opts()));
    let r = type_key(&mut ui, &mut input, KeyCode::Minus, field(5.0, opts()));
    assert!(r.invalid, "a non-numeric buffer must be flagged");
    assert!(
        ui.draw_list().commands().iter().any(|c| matches!(c, DrawCommand::RectBorder { color, .. } if *color == invalid_border)),
        "the invalid border color is drawn"
    );

    // ...and an Enter commit reverts to the pre-edit value instead of
    // committing garbage or silently keeping half a number.
    let r = type_key(&mut ui, &mut input, KeyCode::Enter, field(5.0, opts()));
    assert!(r.committed);
    assert!(!r.changed, "revert is not a change");
    assert_eq!(r.value, 5.0);
}

#[test]
fn test_float_suffix_renders_but_never_enters_the_buffer() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let degrees = opts().with_suffix("°");

    frame(&mut ui, &input, field(90.0, degrees));
    let has_suffix = ui.draw_list().commands().iter().any(|c| match c {
        DrawCommand::TextPlaceholder { text, .. } => text == "90.00°",
        DrawCommand::Text { data, .. } => data.text == "90.00°",
        _ => false,
    });
    assert!(has_suffix, "the display shows the suffix");

    // Focused, the edit buffer is the bare number: an untouched commit
    // parses cleanly (a suffix in the buffer would fail and revert).
    let r = type_and_commit(&mut ui, &mut input, 90.0, degrees, &[]);
    assert!(r.committed && !r.changed && !r.invalid);
    assert_eq!(r.value, 90.0);
}
