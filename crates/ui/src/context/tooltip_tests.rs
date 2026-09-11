//! Contract tests for [`UIContext::tooltip`].
//!
//! Every frame here is driven with `begin_frame_dt` at a 0.1 s step, so
//! "rested for the delay" is a whole number of frames and the assertions
//! read as the seconds a person would experience them as.

use glam::Vec2;
use input::prelude::{InputHandler, MouseButton};

use super::tooltip::TOOLTIP_DELAY;
use crate::context::UIContext;
use crate::test_support::{next_frame, WINDOW};
use crate::{DrawCommand, Rect, UiLayer};

/// Frame delta every test steps by — [`TOOLTIP_DELAY`] is five of them.
const STEP: f32 = 0.1;

/// The anchor the pointer rests on in most tests: a toolbar-button-sized
/// rect well inside the window.
fn anchor() -> Rect {
    Rect::new(100.0, 100.0, 80.0, 30.0)
}

/// A second anchor, far enough from the first that no pointer is inside
/// both.
fn other_anchor() -> Rect {
    Rect::new(400.0, 300.0, 80.0, 30.0)
}

/// The text of every command drawn on the Tooltip band — the panel's words,
/// which is what the contract is about. Separator rects and borders are
/// ignored: the band is either empty or carries one panel.
fn tooltip_text(ui: &UIContext) -> Vec<String> {
    let band = UiLayer::Tooltip.depth_base()..UiLayer::DragGhost.depth_base();
    ui.draw_list()
        .commands()
        .iter()
        .filter(|command| band.contains(&command.depth()))
        .filter_map(|command| match command {
            DrawCommand::Text { data, .. } => Some(data.text.clone()),
            DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// One frame with the pointer moved to `pointer`, offering the tooltip of
/// whatever `body` names.
fn step(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pointer: Vec2,
    body: impl FnOnce(&mut UIContext),
) -> Vec<String> {
    next_frame(input);
    input.mouse_mut().update_position(pointer.x, pointer.y);
    ui.begin_frame_dt(input, WINDOW, STEP);
    body(ui);
    ui.end_frame();
    tooltip_text(ui)
}

/// Rest the pointer on `anchor` for `frames` frames and offer its tooltip
/// each one, returning the last frame's Tooltip band.
fn rest(
    ui: &mut UIContext,
    input: &mut InputHandler,
    pointer: Vec2,
    anchor: Rect,
    text: &str,
    frames: usize,
) -> Vec<String> {
    let mut last = Vec::new();
    for _ in 0..frames {
        last = step(ui, input, pointer, |ui| ui.tooltip(anchor, text));
    }
    last
}

/// A frame that offers an anchor whose owner skips it entirely — the
/// pointer is elsewhere and nothing names the tooltip.
fn ignored(ui: &mut UIContext, input: &mut InputHandler, pointer: Vec2) -> Vec<String> {
    step(ui, input, pointer, |_| {})
}

#[test]
fn test_no_tooltip_appears_before_the_pointer_has_rested_for_the_delay() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let pointer = anchor().center();

    let resting = (TOOLTIP_DELAY / STEP) as usize;
    for _ in 1..resting {
        let band = rest(&mut ui, &mut input, pointer, anchor(), "Select", 1);
        assert!(band.is_empty(), "the panel must wait for the pointer to rest");
    }
}

#[test]
fn test_the_tooltip_carries_the_anchors_text_once_the_delay_has_passed() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let pointer = anchor().center();

    let band = rest(
        &mut ui,
        &mut input,
        pointer,
        anchor(),
        "Move the selection",
        (TOOLTIP_DELAY / STEP) as usize + 1,
    );
    assert_eq!(band, vec!["Move the selection".to_string()]);
}

#[test]
fn test_a_pointer_moving_inside_the_anchor_restarts_the_delay_before_it_shows() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();

    // Rest just short of the delay, then keep drifting inside the anchor:
    // a moving pointer never raises a panel however long it wanders.
    rest(&mut ui, &mut input, anchor.center(), anchor, "Select", 3);
    let mut pointer = anchor.center();
    for _ in 0..8 {
        pointer.x += 1.0;
        let band = step(&mut ui, &mut input, pointer, |ui| ui.tooltip(anchor, "Select"));
        assert!(band.is_empty(), "a moving pointer is not resting");
    }
}

#[test]
fn test_a_pointer_moving_inside_the_anchor_keeps_a_shown_tooltip() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();

    rest(
        &mut ui,
        &mut input,
        anchor.center(),
        anchor,
        "Select",
        (TOOLTIP_DELAY / STEP) as usize + 2,
    );
    let mut pointer = anchor.center();
    for _ in 0..5 {
        pointer.x += 2.0;
        let band = step(&mut ui, &mut input, pointer, |ui| ui.tooltip(anchor, "Select"));
        assert_eq!(band, vec!["Select".to_string()], "the panel stays while the pointer does");
    }
}

#[test]
fn test_leaving_the_anchor_clears_the_tooltip_the_same_frame() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();

    rest(
        &mut ui,
        &mut input,
        anchor.center(),
        anchor,
        "Select",
        (TOOLTIP_DELAY / STEP) as usize + 2,
    );
    let away = Vec2::new(anchor.right() + 20.0, anchor.center().y);
    let band = step(&mut ui, &mut input, away, |ui| ui.tooltip(anchor, "Select"));
    assert!(band.is_empty(), "the panel goes the frame the pointer leaves");
}

#[test]
fn test_a_mouse_press_clears_the_tooltip_and_the_rest_starts_over() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();
    let pointer = anchor.center();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    rest(&mut ui, &mut input, pointer, anchor, "Select", resting);

    next_frame(&mut input);
    input.mouse_mut().update_position(pointer.x, pointer.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    ui.begin_frame_dt(&input, WINDOW, STEP);
    ui.tooltip(anchor, "Select");
    ui.end_frame();
    assert!(tooltip_text(&ui).is_empty(), "a press takes the panel down");

    input.mouse_mut().handle_button_release(MouseButton::Left);
    let band = rest(&mut ui, &mut input, pointer, anchor, "Select", 1);
    assert!(band.is_empty(), "the pointer must rest again from zero");
    let band = rest(&mut ui, &mut input, pointer, anchor, "Select", resting);
    assert_eq!(band, vec!["Select".to_string()], "and it comes back once rested");
}

#[test]
fn test_two_anchors_in_turn_show_only_the_last_one() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    let band = rest(&mut ui, &mut input, anchor().center(), anchor(), "Select", resting);
    assert_eq!(band, vec!["Select".to_string()]);

    // The pointer arrives on the second anchor: the first panel is gone on
    // that very frame, and only the second rests.
    let band = step(&mut ui, &mut input, other_anchor().center(), |ui| {
        ui.tooltip(other_anchor(), "Rotate")
    });
    assert!(band.is_empty(), "a new anchor starts from zero");

    let band = rest(
        &mut ui,
        &mut input,
        other_anchor().center(),
        other_anchor(),
        "Rotate",
        resting,
    );
    assert_eq!(band, vec!["Rotate".to_string()]);
}

#[test]
fn test_an_anchor_under_a_modal_scrim_never_raises_a_tooltip() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();
    let pointer = anchor.center();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    let scrim = Rect::new(0.0, 0.0, WINDOW.x, WINDOW.y);
    let mut last = Vec::new();
    for _ in 0..resting {
        next_frame(&mut input);
        input.mouse_mut().update_position(pointer.x, pointer.y);
        ui.begin_frame_dt(&input, WINDOW, STEP);
        // The strip draws first and offers the button's tooltip; the modal
        // and its scrim are pushed afterwards, as the real frame orders it.
        ui.tooltip(anchor, "Select");
        ui.begin_overlay_in(UiLayer::Modal, scrim);
        ui.end_overlay();
        ui.end_frame();
        last = tooltip_text(&ui);
    }
    assert!(last.is_empty(), "an inert anchor must not raise a panel");
}

#[test]
fn test_an_anchor_inside_its_own_overlay_still_raises_a_tooltip() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();
    let pointer = anchor.center();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    let mut last = Vec::new();
    for _ in 0..resting {
        next_frame(&mut input);
        input.mouse_mut().update_position(pointer.x, pointer.y);
        ui.begin_frame_dt(&input, WINDOW, STEP);
        ui.begin_overlay_in(UiLayer::Floating, anchor);
        ui.tooltip(anchor, "Select");
        ui.end_overlay();
        ui.end_frame();
        last = tooltip_text(&ui);
    }
    assert_eq!(last, vec!["Select".to_string()], "an overlay does not blind its own widgets");
}

#[test]
fn test_only_the_anchor_under_the_pointer_claims_the_frame() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;
    let pointer = anchor().center();

    // A whole toolbar's worth of widgets offer their tooltips every frame;
    // the pointer is on one of them, and the ones it is not on are not what
    // it is resting on.
    let mut last = Vec::new();
    for _ in 0..resting {
        last = step(&mut ui, &mut input, pointer, |ui| {
            ui.tooltip(other_anchor(), "Rotate");
            ui.tooltip(anchor(), "Select");
            ui.tooltip(Rect::new(600.0, 400.0, 80.0, 30.0), "Scale");
        });
    }
    assert_eq!(last, vec!["Select".to_string()]);
}

#[test]
fn test_the_innermost_of_two_nested_anchors_is_the_one_the_pointer_rests_on() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let resting = (TOOLTIP_DELAY / STEP) as usize;

    // A panel header and the chevron inside it, offered in the order the
    // dock draws them; the pointer is on the chevron, and so inside both.
    let header = Rect::new(0.0, 0.0, 300.0, 24.0);
    let chevron = Rect::new(280.0, 4.0, 16.0, 16.0);
    let pointer = chevron.center();
    let both = |ui: &mut UIContext| {
        ui.tooltip(header, "The scene's entities");
        ui.tooltip(chevron, "Collapse");
    };

    for _ in 1..resting {
        let band = step(&mut ui, &mut input, pointer, both);
        assert!(band.is_empty(), "the rest is not over yet");
    }
    let mut last = Vec::new();
    for _ in 0..2 {
        last = step(&mut ui, &mut input, pointer, both);
    }
    assert_eq!(last, vec!["Collapse".to_string()], "the chevron, not the band around it");
}

#[test]
fn test_a_held_button_keeps_the_tooltip_down_until_it_is_released() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();
    let pointer = anchor.center();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    // Press on the anchor and hold still: a gesture in progress, however
    // long, is not a rest.
    input.mouse_mut().update_position(pointer.x, pointer.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    for _ in 0..(resting * 2) {
        let band = rest(&mut ui, &mut input, pointer, anchor, "Select", 1);
        assert!(band.is_empty(), "no panel while the button is held");
    }

    input.mouse_mut().handle_button_release(MouseButton::Left);
    let band = rest(&mut ui, &mut input, pointer, anchor, "Select", resting);
    assert_eq!(band, vec!["Select".to_string()], "released and rested, it shows");
}

#[test]
fn test_an_overlay_opened_after_a_smaller_anchor_it_covers_shows_its_own_tooltip() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    // A small widget offers first; then an overlay opens over it and a
    // larger widget inside the overlay offers under the same pointer. The
    // small one is the smaller anchor, and it is the one that is covered.
    let covered = Rect::new(100.0, 100.0, 16.0, 16.0);
    let overlay = Rect::new(60.0, 80.0, 200.0, 60.0);
    let pointer = covered.center();
    let mut last = Vec::new();
    for _ in 0..resting {
        last = step(&mut ui, &mut input, pointer, |ui| {
            ui.tooltip(covered, "Covered");
            ui.begin_overlay_in(UiLayer::Floating, overlay);
            ui.tooltip(overlay, "Overlay");
            ui.end_overlay();
        });
    }
    assert_eq!(last, vec!["Overlay".to_string()], "the live anchor, not the smaller covered one");
}

#[test]
fn test_a_click_that_begins_and_ends_inside_one_frame_takes_the_tooltip_down() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let anchor = anchor();
    let pointer = anchor.center();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    rest(&mut ui, &mut input, pointer, anchor, "Select", resting);

    // A slow frame: the press and the release both arrive before the UI
    // sees either, so the button is up again but the press edge is set.
    next_frame(&mut input);
    input.mouse_mut().update_position(pointer.x, pointer.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    input.mouse_mut().handle_button_release(MouseButton::Left);
    ui.begin_frame_dt(&input, WINDOW, STEP);
    ui.tooltip(anchor, "Select");
    ui.end_frame();
    assert!(tooltip_text(&ui).is_empty(), "the click took the panel down");

    let band = rest(&mut ui, &mut input, pointer, anchor, "Select", 1);
    assert!(band.is_empty(), "and the rest starts over");
}

#[test]
fn test_a_frame_that_names_no_anchor_clears_the_tooltip() {
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let resting = (TOOLTIP_DELAY / STEP) as usize + 2;

    rest(&mut ui, &mut input, anchor().center(), anchor(), "Select", resting);
    let band = ignored(&mut ui, &mut input, anchor().center());
    assert!(band.is_empty(), "no call names the anchor, so the panel goes");
}
