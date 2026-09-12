//! Contract tests for Tab / Shift-Tab traversal between text fields.
//!
//! Every test here drives real frames and asserts the OUTCOME — which field
//! holds the keyboard, and how many edits a single key press produced. The
//! mechanism (a target scheduled on the committing frame and claimed on a
//! later one) is deliberately not asserted: an earlier design reasoned about
//! where the target id came from instead, and reasoning that reads correctly
//! still cascaded through every field after the first on one Tab press.

use input::prelude::KeyCode;

use super::*;
use crate::test_support::{focus_field, idle, press_at, release, type_key};
use crate::UiLayer;

/// Three stacked rows, the shape a top-to-bottom inspector has.
const ROW_1: Rect = Rect { x: 10.0, y: 10.0, width: 120.0, height: 20.0 };
const ROW_2: Rect = Rect { x: 10.0, y: 40.0, width: 120.0, height: 20.0 };
const ROW_3: Rect = Rect { x: 10.0, y: 70.0, width: 120.0, height: 20.0 };

/// The three fields, drawn top to bottom; returns each one's commit.
fn fields(ui: &mut UIContext) -> [Option<String>; 3] {
    [
        ui.text_input("first", "one", ROW_1),
        ui.text_input("second", "two", ROW_2),
        ui.text_input("third", "three", ROW_3),
    ]
}

/// One frame with Shift+Tab just pressed.
fn shift_tab(
    ui: &mut UIContext,
    input: &mut input::InputHandler,
    body: impl FnOnce(&mut UIContext) -> [Option<String>; 3],
) -> [Option<String>; 3] {
    input.keyboard_mut().handle_key_press(KeyCode::ShiftLeft);
    let out = type_key(ui, input, KeyCode::Tab, body);
    input.keyboard_mut().handle_key_release(KeyCode::ShiftLeft);
    out
}

#[test]
fn test_one_tab_press_commits_only_the_focused_field_and_moves_focus_one_step_forward() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_2, fields);
    idle(&mut ui, &mut input, fields);

    let commits = type_key(&mut ui, &mut input, KeyCode::Tab, fields);
    assert_eq!(
        commits,
        [None, Some("two".to_string()), None],
        "one Tab press commits the focused field and no field after it"
    );
    assert!(
        ui.wants_keyboard(),
        "no widget is focused on the commit frame, but a traversal is pending — a host must \
         still suppress its own shortcuts, or Delete/Ctrl+Z reaches the scene instead of the \
         field the Tab press is mid-way through reaching"
    );

    idle(&mut ui, &mut input, fields);
    assert!(ui.is_focused("third"), "the next frame advances focus by exactly one field");
}

#[test]
fn test_shift_tab_moves_focus_one_step_backward() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_2, fields);
    idle(&mut ui, &mut input, fields);

    let commits = shift_tab(&mut ui, &mut input, fields);
    assert_eq!(commits, [None, Some("two".to_string()), None]);

    idle(&mut ui, &mut input, fields);
    assert!(ui.is_focused("first"), "Shift-Tab goes back one field");
}

#[test]
fn test_tab_from_the_last_field_wraps_to_the_first() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_3, fields);
    idle(&mut ui, &mut input, fields);

    let commits = type_key(&mut ui, &mut input, KeyCode::Tab, fields);
    assert_eq!(commits, [None, None, Some("three".to_string())]);

    idle(&mut ui, &mut input, fields);
    assert!(ui.is_focused("first"), "Tab past the last field wraps to the first");
}

#[test]
fn test_shift_tab_from_the_first_field_wraps_to_the_last() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_1, fields);
    idle(&mut ui, &mut input, fields);

    let commits = shift_tab(&mut ui, &mut input, fields);
    assert_eq!(commits, [Some("one".to_string()), None, None]);

    idle(&mut ui, &mut input, fields);
    assert!(ui.is_focused("third"), "Shift-Tab before the first field wraps to the last");
}

#[test]
fn test_a_click_discards_a_pending_traversal_instead_of_focusing_it_later() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_2, fields);
    idle(&mut ui, &mut input, fields);
    type_key(&mut ui, &mut input, KeyCode::Tab, fields);

    // The click lands on a different field before the traversal resolves.
    press_at(&mut ui, &mut input, ROW_1.center(), fields);
    release(&mut ui, &mut input, fields);
    assert!(ui.is_focused("first"), "the clicked field takes the keyboard");

    idle(&mut ui, &mut input, fields);
    assert!(ui.is_focused("first"), "the discarded traversal never resurfaces");
    assert!(!ui.is_focused("third"));
}

#[test]
fn test_a_traversal_target_behind_an_overlay_stays_pending_until_it_can_be_seen() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_2, fields);
    idle(&mut ui, &mut input, fields);
    type_key(&mut ui, &mut input, KeyCode::Tab, fields);

    // A modal opens over the third field between the commit and the match.
    idle(&mut ui, &mut input, |ui| {
        ui.interaction.push_blocking_rect(ROW_3, UiLayer::Modal);
        fields(ui)
    });
    assert!(
        !ui.is_focused("third"),
        "a target under an overlay is not focused invisibly behind it"
    );
    assert!(
        !ui.wants_keyboard(),
        "a target an overlay blocks does not shield the keyboard — it can wait indefinitely, \
         and treating that the same as \"about to land next frame\" would deafen every editor \
         shortcut, play controls included, for as long as the overlay stays open"
    );

    // The overlay closes; the traversal it deferred still resolves.
    idle(&mut ui, &mut input, fields);
    assert!(ui.is_focused("third"), "the deferred target gets focus once nothing blocks it");
}

#[test]
fn test_explicit_focus_while_a_traversal_is_blocked_is_not_later_overwritten_by_it() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    focus_field(&mut ui, &mut input, ROW_2, fields);
    idle(&mut ui, &mut input, fields);
    type_key(&mut ui, &mut input, KeyCode::Tab, fields); // schedules "third"

    // An overlay blocks "third" — the shortcut this now permits (F2 rename,
    // say) assigns focus to an unrelated field directly, the way
    // `focus_text_input` does, rather than through a click or a traversal.
    idle(&mut ui, &mut input, |ui| {
        ui.interaction.push_blocking_rect(ROW_3, UiLayer::Modal);
        ui.focus_text_input("rename_field", "Old");
        fields(ui)
    });
    assert!(ui.is_focused("rename_field"), "the explicit focus takes effect");

    // The overlay closes. The traversal it deferred must not resurrect and
    // steal focus back from the field explicitly given it in the meantime.
    idle(&mut ui, &mut input, fields);
    assert!(
        ui.is_focused("rename_field"),
        "explicit focus assigned while a traversal was blocked survives the overlay closing"
    );
    assert!(!ui.is_focused("third"));
}

#[test]
fn test_a_target_that_stops_drawing_is_forgotten_and_does_not_steal_focus_on_reappearance() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    // "third" leaves the drawn set entirely — the panel scrolled it away, or
    // the selection changed — rather than merely being blocked by an overlay.
    let without_third = |ui: &mut UIContext| {
        [
            ui.text_input("first", "one", ROW_1),
            ui.text_input("second", "two", ROW_2),
            None,
        ]
    };

    focus_field(&mut ui, &mut input, ROW_2, fields);
    idle(&mut ui, &mut input, fields);
    type_key(&mut ui, &mut input, KeyCode::Tab, fields); // schedules "third"

    // Absent for two consecutive frames — past the one grace frame.
    idle(&mut ui, &mut input, without_third);
    idle(&mut ui, &mut input, without_third);

    // "third" draws again, as a stand-in for some unrelated field that
    // happens to hash to the same id (e.g. the same field on a different
    // entity — `WidgetId` carries no notion of "this entity").
    idle(&mut ui, &mut input, fields);
    assert!(
        !ui.is_focused("third"),
        "a target that went quiet for longer than its grace window must not steal focus \
         from an unrelated field that later reuses its id"
    );
}
