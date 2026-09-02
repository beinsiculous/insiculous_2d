//! Integration tests: the InputHandler event queue and frame lifecycle.

mod common;

use common::frame;
use input::prelude::*;

#[test]
fn test_queued_events_apply_only_on_process_and_in_queue_order() {
    let mut input = InputHandler::new();
    input.gamepads_mut().register_gamepad(0);

    input.queue_event(InputEvent::KeyPressed(KeyCode::KeyA));
    input.queue_event(InputEvent::KeyPressed(KeyCode::KeyB));
    input.queue_event(InputEvent::KeyReleased(KeyCode::KeyA));
    input.queue_event(InputEvent::MouseButtonPressed(MouseButton::Left));
    input.queue_event(InputEvent::MouseMoved(100.0, 200.0));
    input.queue_event(InputEvent::GamepadButtonPressed(0, GamepadButton::A));
    input.queue_event(InputEvent::GamepadAxisUpdated(0, GamepadAxis::LeftStickX, 0.5));

    // Queued, not yet applied
    assert!(!input.is_key_pressed(KeyCode::KeyA));
    assert!(!input.is_key_pressed(KeyCode::KeyB));
    assert!(!input.is_mouse_button_pressed(MouseButton::Left));
    assert_eq!(input.mouse_position(), MousePosition { x: 0.0, y: 0.0 });
    let pad = input.gamepads().get_gamepad(0).expect("registered above");
    assert!(!pad.is_button_pressed(GamepadButton::A));
    assert_eq!(pad.axis_value(GamepadAxis::LeftStickX), 0.0);

    input.process_queued_events();

    // Applied in queue order: A was pressed and then released within the frame
    assert!(!input.is_key_pressed(KeyCode::KeyA));
    assert!(input.is_key_just_released(KeyCode::KeyA));
    assert!(input.is_key_pressed(KeyCode::KeyB));
    assert!(input.is_key_just_pressed(KeyCode::KeyB));
    assert!(input.is_mouse_button_just_pressed(MouseButton::Left));
    assert_eq!(input.mouse_position(), MousePosition { x: 100.0, y: 200.0 });
    let pad = input.gamepads().get_gamepad(0).expect("registered above");
    assert!(pad.is_button_just_pressed(GamepadButton::A));
    assert_eq!(pad.axis_value(GamepadAxis::LeftStickX), 0.5);
}

#[test]
fn test_update_clears_edges_but_keeps_held_inputs() {
    let mut input = InputHandler::new();
    frame(&mut input, &[
        InputEvent::KeyPressed(KeyCode::KeyA),
        InputEvent::MouseButtonPressed(MouseButton::Left),
    ]);
    assert!(input.is_key_just_pressed(KeyCode::KeyA));
    assert!(input.is_mouse_button_just_pressed(MouseButton::Left));

    input.update();

    assert!(!input.is_key_just_pressed(KeyCode::KeyA));
    assert!(!input.is_mouse_button_just_pressed(MouseButton::Left));
    assert!(input.is_key_pressed(KeyCode::KeyA));
    assert!(input.is_mouse_button_pressed(MouseButton::Left));
}
