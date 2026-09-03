//! Contract tests for [`UIContext`]: the placeholder text path and its
//! measurement, the baseline-vs-box footgun, the two-frame button click,
//! the slider, the text/float input editing lifecycle, programmatic focus,
//! and the `begin_overlay` back-compat contract. The numeric-field UX
//! (scrub, nudge, ranges) lives in `scrub_tests.rs`.

use input::prelude::KeyCode;

use super::*;
use crate::test_support::{
    focus_field, frame, idle, move_to, press_at, release, type_key, FIXTURE_FONT,
};
use crate::DrawCommand;

const FIELD: Rect = Rect { x: 10.0, y: 10.0, width: 120.0, height: 20.0 };

/// Every string the frame drew, placeholder or laid-out, in draw order.
fn drawn_texts(ui: &UIContext) -> Vec<String> {
    ui.draw_list()
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
            DrawCommand::Text { data, .. } => Some(data.text.clone()),
            _ => None,
        })
        .collect()
}

fn first_placeholder(ui: &UIContext) -> (String, Vec2, Color, f32) {
    match &ui.draw_list().commands()[0] {
        DrawCommand::TextPlaceholder { text, position, color, font_size, .. } => {
            (text.clone(), *position, *color, *font_size)
        }
        other => panic!("expected a TextPlaceholder first, got {other:?}"),
    }
}

// ================== Placeholder text and measurement ==================

#[test]
fn test_label_without_a_font_emits_a_placeholder_carrying_text_position_color_and_size() {
    let mut ui = UIContext::new();
    let theme_color = ui.theme().text.color;
    let theme_size = ui.theme().text.font_size;
    let cases = [
        ("Test", Vec2::new(10.0, 20.0), None),
        ("Styled Text", Vec2::new(50.0, 60.0), Some((Color::RED, 24.0))),
    ];

    for (text, position, style) in cases {
        ui.begin_frame(&input::InputHandler::new(), crate::test_support::WINDOW);
        match style {
            None => ui.label(text, position),
            Some((color, size)) => ui.label_styled(text, position, color, size),
        }
        let (expected_color, expected_size) = style.unwrap_or((theme_color, theme_size));

        let (drawn, drawn_at, color, size) = first_placeholder(&ui);
        assert_eq!(drawn, text);
        assert_eq!(drawn_at, position, "the baseline position passes through untouched");
        assert_eq!(color, expected_color);
        assert_eq!(size, expected_size);
        assert_eq!(ui.draw_list().len(), 1, "one label is one command");
    }
}

#[test]
fn test_measure_text_without_a_font_is_the_character_count_estimate() {
    // The estimate is the same one the placeholder path draws with, so
    // centering and alignment computed against it match what appears.
    let ui = UIContext::new();
    let cases = [("hello", 16.0, Vec2::new(48.0, 19.2)), ("hello", 12.0, Vec2::new(36.0, 14.4)), ("", 16.0, Vec2::new(0.0, 19.2))];

    for (text, font_size, expected) in cases {
        let measured = ui.measure_text_styled(text, font_size);
        assert!(
            (measured - expected).abs().max_element() < 1e-4,
            "{text:?} at {font_size}: expected {expected}, got {measured}"
        );
    }
    assert_eq!(ui.measure_text("hello"), ui.measure_text_styled("hello", ui.theme().text.font_size));
}

#[test]
fn test_label_centered_offsets_by_half_the_measured_width() {
    let mut ui = UIContext::new();
    let center = Vec2::new(400.0, 300.0);

    ui.label_centered_styled("styled", center, Color::WHITE, 24.0);

    let (_, position, _, _) = first_placeholder(&ui);
    assert_eq!(position.x, center.x - ui.measure_text_styled("styled", 24.0).x / 2.0);
    assert_eq!(position.y, center.y, "center.y is the baseline");
}

#[test]
fn test_label_in_bounds_styled_keeps_glyphs_inside_bounds_at_every_alignment() {
    // Known footgun: `label_styled`'s y is the BASELINE, so text in a box
    // drawn with it straddles the border. The bounded variant centers via
    // font metrics (estimate: ascent = 0.8 × size) — the dock panel header
    // geometry, 24px tall.
    let bounds = Rect::new(0.0, 100.0, 200.0, 24.0);
    let (font_size, padding) = (14.0, 8.0);
    let cases = [
        (TextAlign::Left, bounds.x + padding),
        (TextAlign::Center, bounds.x + (bounds.width - 5.0 * font_size * 0.6) / 2.0),
        (TextAlign::Right, bounds.x + bounds.width - 5.0 * font_size * 0.6 - padding),
    ];

    for (align, expected_x) in cases {
        let mut ui = UIContext::new();
        ui.label_in_bounds_styled("Hiera", bounds, align, Color::WHITE, font_size, padding);

        let (_, position, _, size) = first_placeholder(&ui);
        let glyph_top = position.y - size * 0.8;
        assert!(
            glyph_top >= bounds.y,
            "{align:?}: glyph top {glyph_top} rises above the bounds top {} (border strike-through)",
            bounds.y
        );
        assert!(position.y <= bounds.y + bounds.height, "{align:?}: baseline stays inside");
        assert!((position.x - expected_x).abs() < 1e-4, "{align:?}: x {} != {expected_x}", position.x);
    }
}

#[test]
fn test_float_input_with_an_unresolvable_font_still_draws_its_box_and_value() {
    // #54: a stale handle must not panic or draw nothing — the field takes
    // the placeholder path a missing default font takes, box included.
    let mut ui = UIContext::new();
    ui.begin_frame(&input::InputHandler::new(), crate::test_support::WINDOW);
    let bounds = Rect::new(50.0, 50.0, 100.0, 24.0);
    let opts = FloatFieldOpts::range(0.0, 100.0).with_font(Some(FontHandle { id: 999 }));

    let result = ui.float_input("stale_font", 42.0, opts, bounds);

    assert_eq!(result.value, 42.0);
    let commands = ui.draw_list().commands();
    assert!(
        matches!(commands[0], DrawCommand::Rect { bounds: b, .. } if b == bounds),
        "background fills the field: {:?}",
        commands[0]
    );
    assert!(
        matches!(commands[1], DrawCommand::RectBorder { bounds: b, .. } if b == bounds),
        "border outlines the field: {:?}",
        commands[1]
    );
    assert_eq!(drawn_texts(&ui), vec!["42.00".to_string()], "the value is drawn as a placeholder");
}

#[test]
fn test_progress_bar_fill_width_is_the_fraction_of_the_track() {
    let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
    let cases = [(0.5, Some(100.0)), (1.5, Some(200.0)), (0.0, None)];

    for (value, fill_width) in cases {
        let mut ui = UIContext::new();
        ui.progress_bar(value, bounds);

        let commands = ui.draw_list().commands();
        assert!(matches!(commands[0], DrawCommand::Rect { bounds: b, .. } if b == bounds), "track spans the bounds");
        match (fill_width, commands.get(1)) {
            (Some(width), Some(DrawCommand::Rect { bounds: fill, .. })) => {
                assert!((fill.width - width).abs() < 1e-4, "value {value}: fill {} != {width}", fill.width);
                assert_eq!((fill.x, fill.height), (bounds.x, bounds.height));
            }
            (None, None) => {}
            (expected, got) => panic!("value {value}: expected fill {expected:?}, got {got:?}"),
        }
    }
}

// ================== Button and slider ==================

#[test]
fn test_button_clicks_on_the_release_frame_and_owns_the_mouse_until_then() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = Rect::new(100.0, 100.0, 200.0, 50.0);
    let button = |ui: &mut UIContext| ui.button("test_button", "Click Me!", bounds);

    // `wants_mouse` is read mid-frame, where a raw-input consumer would.
    let button_and_gesture = |ui: &mut UIContext| (button(ui), ui.wants_mouse());

    let (clicked, owned) = move_to(&mut ui, &mut input, bounds.center(), button_and_gesture);
    assert!(!clicked, "hovering is not a click");
    assert!(!owned);

    let (clicked, owned) = press_at(&mut ui, &mut input, bounds.center(), button_and_gesture);
    assert!(!clicked, "the press frame is not a click");
    assert!(owned, "a widget press claims the gesture");

    let (clicked, owned) = release(&mut ui, &mut input, button_and_gesture);
    assert!(clicked, "the click fires on the release frame");
    assert!(owned, "the gesture stays widget-owned on the release frame");

    let (_, owned) = idle(&mut ui, &mut input, button_and_gesture);
    assert!(!owned, "the gesture is over the frame after");
}

#[test]
fn test_press_and_release_delivered_in_one_frame_still_click() {
    // A fast tap on the web build delivers both edges before one frame
    // runs: just-pressed AND just-released are true together, and the
    // click must fire from that single frame at both levels.
    use input::prelude::{InputEvent, InputHandler, MouseButton};
    let bounds = Rect::new(100.0, 100.0, 200.0, 50.0);
    let mut input = InputHandler::new();
    input.queue_event(InputEvent::MouseMoved(200.0, 125.0));
    input.queue_event(InputEvent::MouseButtonPressed(MouseButton::Left));
    input.queue_event(InputEvent::MouseButtonReleased(MouseButton::Left));
    input.process_queued_events();

    let mut manager = crate::InteractionManager::new();
    manager.begin_frame(&input);
    assert!(manager.interact(WidgetId::hashed("tap"), bounds, true).clicked, "interact reports the click");

    let mut ui = UIContext::new();
    ui.begin_frame(&input, crate::test_support::WINDOW);
    assert!(ui.button("tap", "Tap", bounds), "the button clicks in the same frame");
    ui.end_frame();
}

#[test]
fn test_button_does_not_click_when_pressed_outside_or_released_outside() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = Rect::new(100.0, 100.0, 200.0, 50.0);
    let outside = Vec2::new(50.0, 50.0);
    let button = |ui: &mut UIContext| ui.button("test_button", "Click Me!", bounds);

    press_at(&mut ui, &mut input, outside, button);
    assert!(!release(&mut ui, &mut input, button), "a click that starts outside never fires");

    press_at(&mut ui, &mut input, bounds.center(), button);
    move_to(&mut ui, &mut input, outside, button);
    assert!(!release(&mut ui, &mut input, button), "press inside, release outside cancels the click");
}

#[test]
fn test_slider_maps_pointer_x_to_value_while_dragging_and_holds_it_after_release() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = Rect::new(100.0, 200.0, 200.0, 30.0);
    let y = bounds.center().y;
    let slider = |value: f32| move |ui: &mut UIContext| ui.slider("test_slider", value, bounds);

    let pressed = press_at(&mut ui, &mut input, Vec2::new(200.0, y), slider(0.2));
    assert_eq!(pressed, 0.5, "the press frame already maps the pointer to a value");
    let dragged = move_to(&mut ui, &mut input, Vec2::new(250.0, y), slider(pressed));
    assert_eq!(dragged, 0.75);
    let released = release(&mut ui, &mut input, slider(dragged));
    assert_eq!(released, dragged, "release keeps the last dragged value");
    let after = move_to(&mut ui, &mut input, Vec2::new(120.0, y), slider(released));
    assert_eq!(after, released, "moving the pointer without a press does not drag");
}

#[test]
fn test_slider_clamps_to_the_range_ends_when_dragged_past_the_track() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = Rect::new(100.0, 200.0, 200.0, 30.0);
    let y = bounds.center().y;
    let slider = |value: f32| move |ui: &mut UIContext| ui.slider("edge_slider", value, bounds);

    let at_right_edge = press_at(&mut ui, &mut input, Vec2::new(300.0, y), slider(0.5));
    assert_eq!(at_right_edge, 1.0);
    let past_right = move_to(&mut ui, &mut input, Vec2::new(1000.0, y), slider(at_right_edge));
    assert_eq!(past_right, 1.0, "dragging past the right end clamps to 1.0");
    let past_left = move_to(&mut ui, &mut input, Vec2::new(-50.0, y), slider(past_right));
    assert_eq!(past_left, 0.0, "dragging past the left end clamps to 0.0");
    let at_left_edge = move_to(&mut ui, &mut input, Vec2::new(bounds.x, y), slider(past_left));
    assert_eq!(at_left_edge, 0.0);
}

// ================== Text input lifecycle ==================

#[test]
fn test_text_input_click_focuses_with_the_value_selected_and_typing_replaces_it_until_enter_commits() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let field = |ui: &mut UIContext| ui.text_input("txt_sel", "old name", FIELD);

    assert_eq!(press_at(&mut ui, &mut input, FIELD.center(), field), None);
    assert!(!ui.wants_keyboard(), "the press frame does not focus yet");
    assert_eq!(release(&mut ui, &mut input, field), None);
    assert!(ui.wants_keyboard(), "the click focuses the field");

    // The seeded value is fully selected: the first key replaces it, and
    // the frame draws the edit buffer, not the pre-edit value.
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::KeyH, field), None);
    assert_eq!(drawn_texts(&ui), vec!["h".to_string()]);
    type_key(&mut ui, &mut input, KeyCode::KeyI, field);
    input.keyboard_mut().handle_key_press(KeyCode::ShiftLeft);
    type_key(&mut ui, &mut input, KeyCode::KeyA, field);
    type_key(&mut ui, &mut input, KeyCode::Minus, field);
    input.keyboard_mut().handle_key_release(KeyCode::ShiftLeft);
    type_key(&mut ui, &mut input, KeyCode::Space, field);
    type_key(&mut ui, &mut input, KeyCode::Digit2, field);
    assert_eq!(drawn_texts(&ui), vec!["hiA_ 2".to_string()], "shift = uppercase and underscore");

    let committed = type_key(&mut ui, &mut input, KeyCode::Enter, field);
    assert_eq!(committed, Some("hiA_ 2".to_string()));
    assert!(!ui.wants_keyboard(), "commit releases the keyboard");
    assert!(ui.take_edit_commit(), "the commit flag reaches the host");
}

#[test]
fn test_text_input_escape_cancels_and_click_away_commits() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let field = |ui: &mut UIContext| ui.text_input("txt_field", "keep me", FIELD);

    focus_field(&mut ui, &mut input, FIELD, field);
    type_key(&mut ui, &mut input, KeyCode::KeyX, field);
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Escape, field), None, "escape never commits");
    assert!(!ui.wants_keyboard(), "escape drops focus");
    assert_eq!(drawn_texts(&ui), vec!["keep me".to_string()], "the field shows the untouched value again");

    focus_field(&mut ui, &mut input, FIELD, field);
    type_key(&mut ui, &mut input, KeyCode::KeyZ, field);
    let committed = press_at(&mut ui, &mut input, Vec2::new(500.0, 400.0), field);
    assert_eq!(committed, Some("z".to_string()), "a press outside the field commits the buffer");
}

#[test]
fn test_float_input_edits_at_the_cursor_and_commits_on_enter() {
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let opts = FloatFieldOpts::range(-100000.0, 100000.0);
    let field = |value: f32| move |ui: &mut UIContext| ui.float_input("float_edit", value, opts, FIELD).value;

    // Click-to-focus selects the whole "168.40": typing '5' replaces it.
    focus_field(&mut ui, &mut input, FIELD, field(168.4));
    type_key(&mut ui, &mut input, KeyCode::Digit5, field(168.4));
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Enter, field(168.4)), 5.0);
    assert!(!ui.wants_keyboard());

    // Home collapses the selection to the start; '9' inserts before the '1'.
    focus_field(&mut ui, &mut input, FIELD, field(12.0));
    type_key(&mut ui, &mut input, KeyCode::Home, field(12.0));
    type_key(&mut ui, &mut input, KeyCode::Digit9, field(12.0));
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Enter, field(12.0)), 912.0, "insert at the cursor, not the end");

    // End, ArrowLeft ×2 puts the cursor between '.' and '0'; Backspace removes the '.'.
    focus_field(&mut ui, &mut input, FIELD, field(12.0));
    for key in [KeyCode::End, KeyCode::ArrowLeft, KeyCode::ArrowLeft, KeyCode::Backspace] {
        type_key(&mut ui, &mut input, key, field(12.0));
    }
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Enter, field(12.0)), 1200.0, "\"12.00\" minus its '.'");

    // Escape discards the edit.
    focus_field(&mut ui, &mut input, FIELD, field(7.5));
    type_key(&mut ui, &mut input, KeyCode::Digit3, field(7.5));
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Escape, field(7.5)), 7.5);
    assert!(!ui.wants_keyboard());
}

#[test]
fn test_click_inside_a_focused_field_places_the_cursor_at_the_nearest_boundary() -> Result<(), FontError> {
    // Widget-level plumbing of `cursor_from_click`: the prefix widths come
    // from the field's own face, so a click lands where that face drew.
    let mut ui = UIContext::new();
    ui.load_font(FIXTURE_FONT)?;
    let mut input = input::InputHandler::new();
    let field = |ui: &mut UIContext| ui.text_input("cursor_field", "abcd", FIELD);
    let (font_size, padding) = (ui.theme().text_input.font_size, ui.theme().text_input.padding);
    let between_b_and_c = FIELD.x + padding + ui.measure_text_styled("ab", font_size).x + 0.5;

    focus_field(&mut ui, &mut input, FIELD, field);
    press_at(&mut ui, &mut input, Vec2::new(between_b_and_c, FIELD.center().y), field);
    release(&mut ui, &mut input, field);
    type_key(&mut ui, &mut input, KeyCode::KeyX, field);

    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Enter, field), Some("abxcd".to_string()));
    Ok(())
}

#[test]
fn test_focus_text_input_arms_an_edit_without_a_click_until_commit_or_escape() {
    // The F2-rename path: the host focuses the field from a shortcut.
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let field = |ui: &mut UIContext| ui.text_input("rename_field", "OldName", FIELD);

    ui.focus_text_input("rename_field", "OldName");
    assert!(ui.is_focused("rename_field"));
    assert!(!ui.is_focused("some_other_widget"));
    assert_eq!(frame(&mut ui, &input, field), None, "the next frame renders in edit mode");
    assert!(ui.wants_keyboard(), "programmatic focus owns the keyboard");

    // Seeded text is fully selected — typing replaces it, Enter commits.
    type_key(&mut ui, &mut input, KeyCode::KeyZ, field);
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Enter, field), Some("z".to_string()));
    assert!(!ui.wants_keyboard(), "commit releases the keyboard");
    assert!(!ui.is_focused("rename_field"));

    ui.focus_text_input("rename_field", "OldName");
    frame(&mut ui, &input, field);
    assert_eq!(type_key(&mut ui, &mut input, KeyCode::Escape, field), None, "escape never commits");
    assert!(!ui.is_focused("rename_field"), "escape drops focus");
}

// ================== Overlays ==================

#[test]
fn test_begin_overlay_records_floating_and_blocks_input_under_the_rect() {
    // Back-compat contract: begin_overlay = Floating layer + input
    // blocking, exactly as before the UiLayer bands existed.
    let mut ui = UIContext::new();
    let rect = Rect::new(10.0, 10.0, 100.0, 100.0);

    ui.begin_overlay(rect);
    assert_eq!(ui.draw_list().current_layer(), UiLayer::Floating);
    ui.rect(Rect::new(20.0, 20.0, 10.0, 10.0), Color::RED);
    ui.end_overlay();
    assert_eq!(ui.draw_list().current_layer(), UiLayer::Content);

    assert!(ui.is_input_blocked_at(Vec2::new(50.0, 50.0)));
    assert!(!ui.is_input_blocked_at(Vec2::new(500.0, 500.0)));

    ui.end_frame();
    assert!(
        ui.draw_list().commands().iter().any(|c| c.depth() >= UiLayer::Floating.depth_base()),
        "the overlay command reaches the flushed stream in the Floating band"
    );
}
