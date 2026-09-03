//! Contract tests for [`InteractionManager`]: the widget gesture state
//! machine, mouse-gesture ownership (`wants_mouse`), overlay blocking, and
//! the per-widget persistent-state lifecycle.

use super::*;
use crate::test_support::{input_with_mouse, next_frame, release_mouse};

const BUTTON: Rect = Rect { x: 40.0, y: 40.0, width: 50.0, height: 50.0 };
const OVER_BUTTON: Vec2 = Vec2::new(50.0, 50.0);
const FAR_AWAY: Vec2 = Vec2::new(300.0, 300.0);

#[test]
fn test_widget_id_is_stable_and_indexed_variants_do_not_collide() {
    assert_eq!(WidgetId::from_str("button_1"), WidgetId::from_str("button_1"));
    assert_ne!(WidgetId::from_str("button_1"), WidgetId::from_str("button_2"));

    // List rows: the same base string with different indices are distinct
    // widgets, and distinct from the bare string.
    assert_ne!(WidgetId::from_str_index("item", 0), WidgetId::from_str_index("item", 1));
    assert_ne!(WidgetId::from_str_index("item", 0), WidgetId::from_str("item"));

    let from_tuple: WidgetId = ("list", 5).into();
    assert_eq!(from_tuple, WidgetId::from_str_index("list", 5));
    let from_str: WidgetId = "test".into();
    assert_eq!(from_str, WidgetId::from_str("test"));
}

#[test]
fn test_widget_gesture_runs_hovered_active_then_clicks_on_the_release_frame() {
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("test_button");

    let mut input = input_with_mouse(OVER_BUTTON, false);
    manager.begin_frame(&input);
    let hovered = manager.interact(id, BUTTON, true);
    assert_eq!(hovered.state, WidgetState::Hovered);
    assert!(!hovered.clicked);
    manager.end_frame();

    next_frame(&mut input);
    input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
    manager.begin_frame(&input);
    let pressed = manager.interact(id, BUTTON, true);
    assert_eq!(pressed.state, WidgetState::Active);
    assert!(!pressed.clicked, "the press frame is not a click");
    assert!(pressed.dragging);
    manager.end_frame();

    release_mouse(&mut input);
    manager.begin_frame(&input);
    let released = manager.interact(id, BUTTON, true);
    assert!(released.clicked, "the click fires on the release frame");
    assert_eq!(released.state, WidgetState::Hovered, "the release frame is NOT Active");
    manager.end_frame();
}

#[test]
fn test_wants_mouse_holds_from_widget_press_through_release_frame() {
    // Known footgun: raw-input consumers (viewport picking) must gate on
    // `wants_mouse`, not on `WidgetState::Active` — the release frame,
    // where their own click handlers fire, is Hovered.
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("toolbar_button");

    let mut input = input_with_mouse(OVER_BUTTON, true);
    manager.begin_frame(&input);
    manager.interact(id, BUTTON, true);
    assert!(manager.wants_mouse(), "a widget press claims the gesture");
    manager.end_frame();

    next_frame(&mut input);
    manager.begin_frame(&input);
    manager.interact(id, BUTTON, true);
    assert!(manager.wants_mouse(), "a held press keeps the gesture claimed");
    manager.end_frame();

    release_mouse(&mut input);
    manager.begin_frame(&input);
    manager.interact(id, BUTTON, true);
    assert!(manager.wants_mouse(), "the release frame must still report the gesture as widget-owned");
    manager.end_frame();

    next_frame(&mut input);
    manager.begin_frame(&input);
    assert!(!manager.wants_mouse(), "the gesture releases after end_frame");

    // A press that lands on no widget belongs to the viewport.
    let input = input_with_mouse(FAR_AWAY, true);
    manager.begin_frame(&input);
    manager.interact(WidgetId::from_str("far_widget"), BUTTON, true);
    assert!(!manager.wants_mouse(), "a press outside every widget is not widget-owned");
}

#[test]
fn test_missed_release_event_frees_the_mouse_gesture() {
    let mut manager = InteractionManager::new();
    let input = input_with_mouse(OVER_BUTTON, true);
    manager.begin_frame(&input);
    manager.interact(WidgetId::from_str("widget"), BUTTON, true);
    assert!(manager.wants_mouse());
    manager.end_frame();

    // The release event never arrives (window lost focus mid-press): the
    // next frame's input shows the button up with no just-released edge.
    let input = input_with_mouse(OVER_BUTTON, false);
    manager.begin_frame(&input);
    assert!(!manager.wants_mouse(), "a stale press must not block picking forever");
}

#[test]
fn test_blocking_rect_makes_widgets_under_it_inert_except_in_overlay_scope() {
    let mut manager = InteractionManager::new();
    let dropdown = Rect::new(0.0, 0.0, 100.0, 100.0);
    let input = input_with_mouse(OVER_BUTTON, true);
    manager.begin_frame(&input);
    manager.push_blocking_rect(dropdown);

    let under = manager.interact(WidgetId::from_str("widget_under_dropdown"), BUTTON, true);
    assert_eq!(under.state, WidgetState::Normal, "no hover under a blocking rect");
    assert!(!under.clicked && !under.dragging);
    assert!(manager.active_widget.is_none(), "the press must not activate a blocked widget");
    assert!(manager.hot_widget.is_none());

    manager.set_overlay_scope(true);
    let item = manager.interact(WidgetId::from_str("dropdown_item"), BUTTON, true);
    assert_eq!(item.state, WidgetState::Active, "an overlay widget receives the press");
    assert!(item.dragging);
    manager.set_overlay_scope(false);

    // Blocking only applies under the rect...
    let mut manager = InteractionManager::new();
    manager.begin_frame(&input_with_mouse(FAR_AWAY, false));
    manager.push_blocking_rect(dropdown);
    let far = manager.interact(WidgetId::from_str("far_widget"), Rect::new(280.0, 280.0, 50.0, 50.0), true);
    assert_eq!(far.state, WidgetState::Hovered);

    // ...and only for the frame that registered it.
    manager.set_overlay_scope(true);
    assert!(manager.is_blocked_at(OVER_BUTTON));
    manager.begin_frame(&InputHandler::new());
    assert!(!manager.is_blocked_at(OVER_BUTTON), "begin_frame clears blocking rects");
    assert!(!manager.overlay_scope, "begin_frame clears the overlay scope");
}

#[test]
fn test_unseen_widget_state_is_collected_unless_focused_or_blocked() {
    let mut manager = InteractionManager::new();
    let transient = WidgetId::from_str("transient");
    let editing = WidgetId::from_str("text_input");
    let blocked = WidgetId::from_str("blocked_text_input");

    manager.get_state(transient).edit.text = "data".to_string();
    manager.get_state(editing).edit.text = "editing".to_string();
    manager.set_focus(editing);
    assert!(manager.has_focus() && manager.is_focused(editing));
    manager.end_frame();
    assert!(manager.get_state_if_exists(transient).is_some(), "seen state survives its frame");

    // A blocked widget is inert, but submitting it still counts as seen.
    manager.get_state(blocked).edit.text = "edit buffer".to_string();
    manager.begin_frame(&input_with_mouse(OVER_BUTTON, false));
    manager.push_blocking_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
    manager.interact(blocked, Rect::new(40.0, 40.0, 20.0, 20.0), true);
    manager.end_frame();

    assert!(manager.get_state_if_exists(transient).is_none(), "unseen state is collected");
    assert_eq!(
        manager.get_state_if_exists(editing).map(|s| s.edit.text.as_str()),
        Some("editing"),
        "the focused field keeps its buffer when its panel skips a frame"
    );
    assert_eq!(manager.get_state_if_exists(blocked).map(|s| s.edit.text.as_str()), Some("edit buffer"));

    // Dropping focus is what lets the buffer go.
    manager.clear_focus();
    assert!(!manager.has_focus() && !manager.is_focused(editing));
    manager.begin_frame(&InputHandler::new());
    manager.end_frame();
    assert!(manager.get_state_if_exists(editing).is_none(), "unfocused and unseen: collected");
}
