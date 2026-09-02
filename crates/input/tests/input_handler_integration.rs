//! Integration tests: InputMapping action evaluation against InputHandler device state.

mod common;

use common::frame;
use input::prelude::*;

fn stick_x(value: f32) -> InputEvent {
    InputEvent::GamepadAxisUpdated(0, GamepadAxis::LeftStickX, value)
}

fn stick_y(value: f32) -> InputEvent {
    InputEvent::GamepadAxisUpdated(0, GamepadAxis::LeftStickY, value)
}

#[test]
fn test_action_edges_fire_once_per_press_and_release_across_frames() {
    let mut input = InputHandler::new();
    let actions = InputMapping::with_default_bindings();

    // Frame 1: Space pressed — active with an activation edge
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Space)]);
    assert!(actions.is_active(GameAction::Action1, &input));
    assert!(actions.just_activated(GameAction::Action1, &input));
    assert!(!actions.just_deactivated(GameAction::Action1, &input));
    input.end_frame();

    // Frame 2: still held — active, the edge is gone
    frame(&mut input, &[]);
    assert!(actions.is_active(GameAction::Action1, &input));
    assert!(!actions.just_activated(GameAction::Action1, &input));
    input.end_frame();

    // Frame 3: released — inactive with a deactivation edge
    frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Space)]);
    assert!(!actions.is_active(GameAction::Action1, &input));
    assert!(!actions.just_activated(GameAction::Action1, &input));
    assert!(actions.just_deactivated(GameAction::Action1, &input));
    input.end_frame();

    // Frame 4: the same action from another device (left click) re-arms
    frame(&mut input, &[InputEvent::MouseButtonPressed(MouseButton::Left)]);
    assert!(actions.just_activated(GameAction::Action1, &input));
}

#[test]
fn test_overlapping_sources_edge_only_on_the_first_press_and_the_last_release() {
    let mut input = InputHandler::new();
    let actions = InputMapping::with_default_bindings();

    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
    assert!(actions.just_activated(GameAction::MoveUp, &input));
    input.end_frame();

    // ArrowUp (also MoveUp) pressed while W is held: no second activation
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::ArrowUp)]);
    assert!(actions.is_active(GameAction::MoveUp, &input));
    assert!(!actions.just_activated(GameAction::MoveUp, &input));
    input.end_frame();

    // W released while ArrowUp is held: still active, no deactivation
    frame(&mut input, &[InputEvent::KeyReleased(KeyCode::KeyW)]);
    assert!(actions.is_active(GameAction::MoveUp, &input));
    assert!(!actions.just_deactivated(GameAction::MoveUp, &input));
    input.end_frame();

    // ArrowUp released too: now the action deactivates
    frame(&mut input, &[InputEvent::KeyReleased(KeyCode::ArrowUp)]);
    assert!(!actions.is_active(GameAction::MoveUp, &input));
    assert!(actions.just_deactivated(GameAction::MoveUp, &input));
}

#[test]
fn test_pads_register_on_first_event_or_connect_and_disconnect_drops_state_without_an_edge() {
    let mut input = InputHandler::new();
    let actions = InputMapping::with_default_bindings();

    // No register call: the first event registers pad 0 and drives Action1
    frame(&mut input, &[InputEvent::GamepadButtonPressed(0, GamepadButton::A)]);
    assert!(input.gamepads().get_gamepad(0).is_some());
    assert!(actions.just_activated(GameAction::Action1, &input));
    input.end_frame();

    // A connect event registers a pad before any button arrives
    frame(&mut input, &[InputEvent::GamepadConnected(1)]);
    assert!(input.gamepads().get_gamepad(1).is_some());
    input.end_frame();

    // Unplugging pad 1 mid-hold: its sources read released, with no just-released edge
    let pad1_a = InputSource::Gamepad(1, GamepadButton::A);
    frame(&mut input, &[InputEvent::GamepadButtonPressed(1, GamepadButton::A)]);
    assert!(input.is_source_pressed(&pad1_a));
    input.end_frame();
    frame(&mut input, &[InputEvent::GamepadDisconnected(1)]);
    assert!(input.gamepads().get_gamepad(1).is_none());
    assert!(!input.is_source_pressed(&pad1_a));
    assert!(!input.is_source_just_released(&pad1_a));
    // Pad 0 is untouched by pad 1's departure
    assert!(actions.is_active(GameAction::Action1, &input));
}

#[test]
fn test_axis_bound_action_edges_on_threshold_crossings_in_its_own_direction_only() {
    let mut input = InputHandler::new();
    let mut actions = InputMapping::new();
    actions.bind(
        GameAction::MoveRight,
        InputSource::GamepadAxis(0, GamepadAxis::LeftStickX, AxisDirection::Positive),
    );
    actions.bind(
        GameAction::MoveLeft,
        InputSource::GamepadAxis(0, GamepadAxis::LeftStickX, AxisDirection::Negative),
    );

    // Frame 1: stick right past the threshold — MoveRight fires, MoveLeft ignores it
    frame(&mut input, &[stick_x(0.8)]);
    assert!(actions.just_activated(GameAction::MoveRight, &input));
    assert!(!actions.is_active(GameAction::MoveLeft, &input));
    input.end_frame();

    // Frame 2: still held — active, no re-trigger
    frame(&mut input, &[stick_x(0.9)]);
    assert!(actions.is_active(GameAction::MoveRight, &input));
    assert!(!actions.just_activated(GameAction::MoveRight, &input));
    input.end_frame();

    // Frame 3: centered — deactivation edge
    frame(&mut input, &[stick_x(0.0)]);
    assert!(!actions.is_active(GameAction::MoveRight, &input));
    assert!(actions.just_deactivated(GameAction::MoveRight, &input));
    input.end_frame();

    // Frame 4: stick left — MoveLeft fires, MoveRight stays off
    frame(&mut input, &[stick_x(-0.6)]);
    assert!(actions.just_activated(GameAction::MoveLeft, &input));
    assert!(!actions.is_active(GameAction::MoveRight, &input));
}

/// No threshold is passed anywhere here: the mapping layer applies the
/// engine-wide `AXIS_ACTIVATION_THRESHOLD`, and every game's stick feel
/// depends on it staying at half deflection.
#[test]
fn test_axis_activation_threshold_is_half_deflection_by_default() {
    let mut input = InputHandler::new();
    let mut actions = InputMapping::new();
    actions.bind(
        GameAction::MoveRight,
        InputSource::GamepadAxis(0, GamepadAxis::LeftStickX, AxisDirection::Positive),
    );

    frame(&mut input, &[stick_x(0.49)]);
    assert!(
        !actions.is_active(GameAction::MoveRight, &input),
        "just under half deflection must not activate"
    );
    input.end_frame();

    frame(&mut input, &[stick_x(0.5)]);
    assert!(actions.is_active(GameAction::MoveRight, &input), "half deflection activates");
}

#[test]
fn test_default_pad_bindings_read_stick_up_as_positive_y_and_ignore_a_sub_threshold_nudge() {
    let mut input = InputHandler::new();
    let actions = InputMapping::with_default_bindings();

    frame(&mut input, &[InputEvent::GamepadButtonPressed(0, GamepadButton::DPadLeft)]);
    assert!(actions.is_active(GameAction::MoveLeft, &input));
    input.end_frame();

    // A nudge inside the threshold is not movement in either direction
    frame(&mut input, &[stick_y(0.3)]);
    assert!(!actions.is_active(GameAction::MoveUp, &input));
    assert!(!actions.is_active(GameAction::MoveDown, &input));
    input.end_frame();

    // Full deflection up: gilrs convention, positive Y = up
    frame(&mut input, &[stick_y(1.0)]);
    assert!(actions.is_active(GameAction::MoveUp, &input));
    assert!(!actions.is_active(GameAction::MoveDown, &input));
}

/// `BehaviorRunner` consumes this preset in production; every row a scene
/// behavior or demo relies on is pinned by evaluation, not by reading the table.
#[test]
fn test_default_preset_drives_menu_select_actions_and_movement_from_keys_and_pad_zero() {
    let mut input = InputHandler::new();
    let actions = InputMapping::with_default_bindings();

    // Escape → Menu, edge fires once per press
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Escape)]);
    assert!(actions.is_active(GameAction::Menu, &input));
    assert!(actions.just_activated(GameAction::Menu, &input));
    input.end_frame();
    frame(&mut input, &[]);
    assert!(actions.is_active(GameAction::Menu, &input));
    assert!(!actions.just_activated(GameAction::Menu, &input));
    input.end_frame();
    frame(&mut input, &[InputEvent::KeyReleased(KeyCode::Escape)]);
    input.end_frame();

    // Pad-0 Start → Menu, Tab → Select
    frame(&mut input, &[
        InputEvent::GamepadButtonPressed(0, GamepadButton::Start),
        InputEvent::KeyPressed(KeyCode::Tab),
    ]);
    assert!(actions.just_activated(GameAction::Menu, &input));
    assert!(actions.is_active(GameAction::Select, &input));
    input.end_frame();

    // Enter and pad B → Action2, each on its own
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::Enter)]);
    assert!(actions.is_active(GameAction::Action2, &input));
    input.end_frame();
    frame(&mut input, &[
        InputEvent::KeyReleased(KeyCode::Enter),
        InputEvent::GamepadButtonPressed(0, GamepadButton::B),
    ]);
    assert!(actions.is_active(GameAction::Action2, &input));
    input.end_frame();

    // Movement: S and ArrowDown → MoveDown, DPadRight → MoveRight
    frame(&mut input, &[InputEvent::KeyPressed(KeyCode::KeyS)]);
    assert!(actions.is_active(GameAction::MoveDown, &input));
    input.end_frame();
    frame(&mut input, &[
        InputEvent::KeyReleased(KeyCode::KeyS),
        InputEvent::KeyPressed(KeyCode::ArrowDown),
        InputEvent::GamepadButtonPressed(0, GamepadButton::DPadRight),
    ]);
    assert!(actions.is_active(GameAction::MoveDown, &input));
    assert!(actions.is_active(GameAction::MoveRight, &input));
    assert!(!actions.is_active(GameAction::MoveUp, &input));
}
