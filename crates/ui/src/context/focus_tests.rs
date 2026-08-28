//! Tests for programmatic focus (`focus_text_input` / `is_focused`) — the
//! host-side hooks that open an inline text edit from a shortcut (F2 rename)
//! instead of requiring a click.

use glam::Vec2;

use crate::{Rect, UIContext};

const BOUNDS: Rect = Rect { x: 10.0, y: 10.0, width: 140.0, height: 20.0 };

fn frame(ui: &mut UIContext, input: &input::InputHandler, value: &str) -> Option<String> {
    ui.begin_frame(input, Vec2::new(800.0, 600.0));
    let out = ui.text_input("rename_field", value, BOUNDS);
    ui.end_frame();
    out
}

#[test]
fn test_focus_text_input_arms_edit_without_a_click() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    ui.focus_text_input("rename_field", "OldName");
    assert!(ui.is_focused("rename_field"));
    assert!(!ui.is_focused("some_other_widget"));

    // The very next frame renders in edit mode: the keyboard is owned and
    // no commit fires yet.
    assert_eq!(frame(&mut ui, &input, "OldName"), None);
    assert!(ui.wants_keyboard(), "programmatic focus must own the keyboard");

    // Seeded text is fully selected — typing replaces it wholesale.
    input.update();
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::KeyZ);
    assert_eq!(frame(&mut ui, &input, "OldName"), None);
    input.keyboard_mut().handle_key_release(input::prelude::KeyCode::KeyZ);

    // Enter commits the replacement.
    input.update();
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);
    let committed = frame(&mut ui, &input, "OldName");
    assert_eq!(committed, Some("z".to_string()));
    assert!(!ui.wants_keyboard(), "commit must release the keyboard");
    assert!(!ui.is_focused("rename_field"));
}

#[test]
fn test_focus_text_input_escape_cancels_without_commit() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    ui.focus_text_input("rename_field", "OldName");
    assert_eq!(frame(&mut ui, &input, "OldName"), None);

    input.update();
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::Escape);
    let committed = frame(&mut ui, &input, "OldName");
    assert_eq!(committed, None, "escape must never commit");
    assert!(!ui.is_focused("rename_field"), "escape must drop focus");
}
