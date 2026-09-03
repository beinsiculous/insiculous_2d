//! Shared fixtures for the crate's tests.
//!
//! The one fact every widget test needs: **a click is two frames.** The
//! press frame activates the widget; `clicked` fires on the RELEASE frame,
//! and that release frame is `Hovered`, not `Active`. The harness below
//! makes that a single documented step instead of a copy per test file.
//! The input side (`press_mouse`, `release_mouse`, ...) works on a bare
//! [`InputHandler`] for `InteractionManager` tests; the frame runners wrap
//! it for [`UIContext`] tests.

use glam::Vec2;
use input::prelude::{InputHandler, KeyCode, MouseButton};

use crate::{Rect, UIContext};

/// Window size every harness frame runs at.
pub(crate) const WINDOW: Vec2 = Vec2::new(800.0, 600.0);

/// Linux Libertine Semibold, the examples' UI face — a real font with
/// ascenders, descenders and spaces, so text-layout math is testable
/// headless without adding a dependency. The metric bounds in
/// `font::layout`'s tests (capital bottom within 1.5px of the baseline,
/// descender bottom past 2px, heights within a pixel) were derived from
/// THIS face; swapping the fixture means re-deriving them.
pub(crate) const FIXTURE_FONT: &[u8] = include_bytes!("../../../examples/assets/fonts/font.ttf");

/// An input handler with the pointer at `pos`, the left button held when
/// `pressed` (a just-pressed edge, as on a press frame).
pub(crate) fn input_with_mouse(pos: Vec2, pressed: bool) -> InputHandler {
    let mut input = InputHandler::new();
    input.mouse_mut().update_position(pos.x, pos.y);
    if pressed {
        input.mouse_mut().handle_button_press(MouseButton::Left);
    }
    input
}

/// Start the next input frame: last frame's just-pressed / just-released
/// edges and per-frame deltas are cleared, held keys and buttons stay held.
pub(crate) fn next_frame(input: &mut InputHandler) {
    input.update();
}

/// Move the pointer to `pos` and press the left button for the coming frame.
pub(crate) fn press_mouse(input: &mut InputHandler, pos: Vec2) {
    next_frame(input);
    input.mouse_mut().update_position(pos.x, pos.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
}

/// Release the left button for the coming frame (the pointer stays put).
pub(crate) fn release_mouse(input: &mut InputHandler) {
    next_frame(input);
    input.mouse_mut().handle_button_release(MouseButton::Left);
}

/// Run one UI frame around `body` and return what the body returned.
pub(crate) fn frame<T>(
    ui: &mut UIContext,
    input: &InputHandler,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    ui.begin_frame(input, WINDOW);
    let out = body(ui);
    ui.end_frame();
    out
}

/// The PRESS frame of a click at `pos`.
pub(crate) fn press_at<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pos: Vec2,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    press_mouse(input, pos);
    frame(ui, input, body)
}

/// A frame with the pointer moved to `pos` (a drag frame while held, a
/// hover frame otherwise).
pub(crate) fn move_to<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pos: Vec2,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    next_frame(input);
    input.mouse_mut().update_position(pos.x, pos.y);
    frame(ui, input, body)
}

/// The RELEASE frame of a click — the frame `clicked` fires on.
pub(crate) fn release<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    release_mouse(input);
    frame(ui, input, body)
}

/// A frame with nothing new from the input device.
pub(crate) fn idle<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    next_frame(input);
    frame(ui, input, body)
}

/// One frame with `key` just pressed. The key is released afterwards so the
/// next frame carries no edge; modifier keys pressed on `input` beforehand
/// stay held across it.
pub(crate) fn type_key<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    key: KeyCode,
    body: impl FnOnce(&mut UIContext) -> T,
) -> T {
    next_frame(input);
    input.keyboard_mut().handle_key_press(key);
    let out = frame(ui, input, body);
    input.keyboard_mut().handle_key_release(key);
    out
}

/// Click (press frame + release frame) the center of `bounds` so the input
/// field `body` submits gains keyboard focus; returns the release frame's
/// output.
pub(crate) fn focus_field<T>(
    ui: &mut UIContext,
    input: &mut InputHandler,
    bounds: Rect,
    mut body: impl FnMut(&mut UIContext) -> T,
) -> T {
    press_at(ui, input, bounds.center(), &mut body);
    let out = release(ui, input, &mut body);
    assert!(ui.wants_keyboard(), "field must be focused after a click");
    out
}
