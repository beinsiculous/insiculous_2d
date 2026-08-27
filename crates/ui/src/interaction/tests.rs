use super::*;

#[test]
fn test_widget_id_from_str() {
    let id1 = WidgetId::from_str("button_1");
    let id2 = WidgetId::from_str("button_1");
    let id3 = WidgetId::from_str("button_2");

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn test_widget_id_from_str_index() {
    let id1 = WidgetId::from_str_index("item", 0);
    let id2 = WidgetId::from_str_index("item", 1);
    let id3 = WidgetId::from_str_index("item", 0);

    assert_ne!(id1, id2);
    assert_eq!(id1, id3);
}

#[test]
fn test_widget_id_conversions() {
    let id1: WidgetId = "test".into();
    let id2: WidgetId = WidgetId::from_str("test");
    assert_eq!(id1, id2);

    let id3: WidgetId = 12345u64.into();
    assert_eq!(id3.value(), 12345);

    let id4: WidgetId = ("list", 5).into();
    let id5 = WidgetId::from_str_index("list", 5);
    assert_eq!(id4, id5);
}

/// Build an InputHandler with the mouse at `pos`, optionally pressed.
fn input_with_mouse(pos: Vec2, pressed: bool) -> InputHandler {
    use input::prelude::MouseButton;
    let mut input = InputHandler::new();
    input.mouse_mut().update_position(pos.x, pos.y);
    if pressed {
        input.mouse_mut().handle_button_press(MouseButton::Left);
    }
    input
}

#[test]
fn test_blocking_rect_makes_outside_widget_inert() {
    let mut manager = InteractionManager::new();
    let input = input_with_mouse(Vec2::new(50.0, 50.0), true);
    manager.begin_frame(&input);

    // A dropdown covers the widget's area
    manager.push_blocking_rect(Rect::new(0.0, 0.0, 100.0, 100.0));

    let id = WidgetId::from_str("widget_under_dropdown");
    let result = manager.interact(id, Rect::new(40.0, 40.0, 50.0, 50.0), true);

    assert_eq!(result.state, WidgetState::Normal, "no hover under a blocking rect");
    assert!(!result.clicked);
    assert!(!result.dragging);
    assert!(manager.active_widget.is_none(), "press must not activate a blocked widget");
    assert!(manager.hot_widget.is_none());
}

#[test]
fn test_overlay_scope_widget_stays_interactive_over_blocking_rect() {
    let mut manager = InteractionManager::new();
    let input = input_with_mouse(Vec2::new(50.0, 50.0), true);
    manager.begin_frame(&input);

    manager.push_blocking_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
    manager.set_overlay_scope(true);

    let id = WidgetId::from_str("dropdown_item");
    let result = manager.interact(id, Rect::new(40.0, 40.0, 50.0, 50.0), true);

    assert_eq!(result.state, WidgetState::Active, "overlay widget receives the press");
    assert!(result.dragging);
}

#[test]
fn test_widget_outside_blocking_rect_unaffected() {
    let mut manager = InteractionManager::new();
    let input = input_with_mouse(Vec2::new(300.0, 300.0), false);
    manager.begin_frame(&input);

    manager.push_blocking_rect(Rect::new(0.0, 0.0, 100.0, 100.0));

    let id = WidgetId::from_str("far_widget");
    let result = manager.interact(id, Rect::new(280.0, 280.0, 50.0, 50.0), true);
    assert_eq!(result.state, WidgetState::Hovered, "blocking only applies under the rect");
}

#[test]
fn test_blocked_widget_persistent_state_survives_frame() {
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("blocked_text_input");
    manager.get_state(id).edit.text = "edit buffer".to_string();

    let input = input_with_mouse(Vec2::new(50.0, 50.0), false);
    manager.begin_frame(&input);
    manager.push_blocking_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
    manager.interact(id, Rect::new(40.0, 40.0, 20.0, 20.0), true);
    manager.end_frame();

    let state = manager.get_state_if_exists(id).expect("blocked widget state retained");
    assert_eq!(state.edit.text, "edit buffer");
}

#[test]
fn test_begin_frame_clears_blocking_state() {
    let mut manager = InteractionManager::new();
    manager.push_blocking_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
    manager.set_overlay_scope(true);
    assert!(manager.is_blocked_at(Vec2::new(50.0, 50.0)));

    manager.begin_frame(&InputHandler::new());
    assert!(!manager.is_blocked_at(Vec2::new(50.0, 50.0)));
    assert!(!manager.overlay_scope);
}

#[test]
fn test_wants_mouse_holds_from_widget_press_through_release_frame() {
    use input::prelude::MouseButton;
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("toolbar_button");
    let bounds = Rect::new(40.0, 40.0, 50.0, 50.0);

    // Press frame: the press lands on the widget
    let mut input = input_with_mouse(Vec2::new(50.0, 50.0), true);
    manager.begin_frame(&input);
    manager.interact(id, bounds, true);
    assert!(manager.wants_mouse(), "widget press must claim the gesture");
    manager.end_frame();
    input.end_frame();

    // Held frame
    manager.begin_frame(&input);
    manager.interact(id, bounds, true);
    assert!(manager.wants_mouse(), "held press keeps the gesture claimed");
    manager.end_frame();
    input.end_frame();

    // Release frame — the frame viewport picking's `clicked` fires on
    input.mouse_mut().handle_button_release(MouseButton::Left);
    manager.begin_frame(&input);
    manager.interact(id, bounds, true);
    assert!(
        manager.wants_mouse(),
        "release frame must still report the gesture as widget-owned"
    );
    manager.end_frame();
    input.end_frame();

    // Gesture over: the next frame is free for raw-input consumers
    manager.begin_frame(&input);
    assert!(!manager.wants_mouse(), "gesture releases after end_frame");
}

#[test]
fn test_wants_mouse_false_when_press_misses_all_widgets() {
    let mut manager = InteractionManager::new();
    let input = input_with_mouse(Vec2::new(300.0, 300.0), true);
    manager.begin_frame(&input);
    manager.interact(WidgetId::from_str("far_widget"), Rect::new(0.0, 0.0, 50.0, 50.0), true);
    assert!(!manager.wants_mouse(), "a press outside every widget belongs to the viewport");
}

#[test]
fn test_missed_release_event_frees_the_mouse_gesture() {
    let mut manager = InteractionManager::new();
    let input = input_with_mouse(Vec2::new(50.0, 50.0), true);
    manager.begin_frame(&input);
    manager.interact(WidgetId::from_str("widget"), Rect::new(40.0, 40.0, 50.0, 50.0), true);
    assert!(manager.wants_mouse());
    manager.end_frame();

    // The release event never arrives (window lost focus mid-press): the
    // next frame's input shows the button up with no just-released edge.
    let input = input_with_mouse(Vec2::new(50.0, 50.0), false);
    manager.begin_frame(&input);
    assert!(!manager.wants_mouse(), "stale press must not block picking forever");
}

#[test]
fn test_has_focus_tracks_any_focused_widget() {
    let mut manager = InteractionManager::new();
    assert!(!manager.has_focus());

    manager.set_focus(WidgetId::from_str("field"));
    assert!(manager.has_focus());

    manager.clear_focus();
    assert!(!manager.has_focus());
}

#[test]
fn test_interaction_manager_state() {
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("test_widget");

    let state = manager.get_state(id);
    state.edit.text = "hello".to_string();

    let state = manager.get_state_if_exists(id).unwrap();
    assert!(state.seen_this_frame);
    assert_eq!(state.edit.text, "hello");
}

#[test]
fn test_unseen_widget_state_is_garbage_collected() {
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("transient");

    manager.get_state(id).edit.text = "data".to_string();
    manager.end_frame();
    assert!(manager.get_state_if_exists(id).is_some(), "seen state survives the frame");

    // Next frame: widget never submitted
    manager.begin_frame(&InputHandler::new());
    manager.end_frame();
    assert!(manager.get_state_if_exists(id).is_none(), "unseen state is collected");
}

#[test]
fn test_focused_widget_state_survives_unseen_frame() {
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("text_input");

    manager.get_state(id).edit.text = "editing".to_string();
    manager.set_focus(id);
    manager.end_frame();

    // Next frame: widget not submitted (e.g., panel skipped a frame),
    // but it holds focus so its edit buffer must be retained.
    manager.begin_frame(&InputHandler::new());
    manager.end_frame();

    let state = manager.get_state_if_exists(id).expect("focused state retained");
    assert_eq!(state.edit.text, "editing");
}

#[test]
fn test_focus_management() {
    let mut manager = InteractionManager::new();
    let id = WidgetId::from_str("text_input");

    assert!(!manager.is_focused(id));

    manager.set_focus(id);
    assert!(manager.is_focused(id));

    manager.clear_focus();
    assert!(!manager.is_focused(id));
}
