use engine_core::input_settings_io::load_or_create;
use input::{
    AxisDirection, GameAction, GamepadAxis, GamepadButton, PlayerId, PlayerSource,
};
use std::path::Path;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

#[test]
fn test_input_settings_fixture_matches_contract() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/input_settings.json");
    assert!(fixture_path.exists(), "fixture must be checked in");

    let loaded = load_or_create(&fixture_path);

    assert_eq!(loaded.player_count(), 2);
    assert_eq!(loaded.pad_of(PlayerId::P1), Some(0));
    assert_eq!(loaded.pad_of(PlayerId::P2), Some(3));

    let p1 = loaded.player(PlayerId::P1).expect("P1 exists");
    let p2 = loaded.player(PlayerId::P2).expect("P2 exists");

    // Player 1 bindings
    assert_eq!(
        p1.bindings(GameAction::MoveUp),
        &[
            PlayerSource::Keyboard(KeyCode::KeyW),
            PlayerSource::PadButton(GamepadButton::DPadUp),
            PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Positive),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::MoveDown),
        &[
            PlayerSource::Keyboard(KeyCode::KeyS),
            PlayerSource::PadButton(GamepadButton::DPadDown),
            PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Negative),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::MoveLeft),
        &[
            PlayerSource::Keyboard(KeyCode::KeyA),
            PlayerSource::PadButton(GamepadButton::DPadLeft),
            PlayerSource::PadAxis(GamepadAxis::LeftStickX, AxisDirection::Negative),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::MoveRight),
        &[
            PlayerSource::Keyboard(KeyCode::KeyD),
            PlayerSource::PadButton(GamepadButton::DPadRight),
            PlayerSource::PadAxis(GamepadAxis::LeftStickX, AxisDirection::Positive),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::Action1),
        &[
            PlayerSource::Keyboard(KeyCode::Space),
            PlayerSource::Mouse(MouseButton::Left),
            PlayerSource::PadButton(GamepadButton::A),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::Action2),
        &[
            PlayerSource::Keyboard(KeyCode::ShiftLeft),
            PlayerSource::PadButton(GamepadButton::B),
        ]
    );
    // Non-default binding added for P1
    assert_eq!(
        p1.bindings(GameAction::Action3),
        &[
            PlayerSource::PadButton(GamepadButton::X),
            PlayerSource::Keyboard(KeyCode::KeyQ),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::Action4),
        &[PlayerSource::PadButton(GamepadButton::Y)]
    );
    assert_eq!(
        p1.bindings(GameAction::Menu),
        &[
            PlayerSource::Keyboard(KeyCode::Escape),
            PlayerSource::PadButton(GamepadButton::Start),
        ]
    );
    assert_eq!(
        p1.bindings(GameAction::Select),
        &[PlayerSource::PadButton(GamepadButton::Select)]
    );

    // Player 2 bindings
    assert_eq!(
        p2.bindings(GameAction::MoveUp),
        &[
            PlayerSource::Keyboard(KeyCode::ArrowUp),
            PlayerSource::PadButton(GamepadButton::DPadUp),
            PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Positive),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::MoveDown),
        &[
            PlayerSource::Keyboard(KeyCode::ArrowDown),
            PlayerSource::PadButton(GamepadButton::DPadDown),
            PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Negative),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::MoveLeft),
        &[
            PlayerSource::Keyboard(KeyCode::ArrowLeft),
            PlayerSource::PadButton(GamepadButton::DPadLeft),
            PlayerSource::PadAxis(GamepadAxis::LeftStickX, AxisDirection::Negative),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::MoveRight),
        &[
            PlayerSource::Keyboard(KeyCode::ArrowRight),
            PlayerSource::PadButton(GamepadButton::DPadRight),
            PlayerSource::PadAxis(GamepadAxis::LeftStickX, AxisDirection::Positive),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::Action1),
        &[
            PlayerSource::Keyboard(KeyCode::Enter),
            PlayerSource::PadButton(GamepadButton::A),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::Action2),
        &[
            PlayerSource::Keyboard(KeyCode::ShiftRight),
            PlayerSource::PadButton(GamepadButton::B),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::Action3),
        &[PlayerSource::PadButton(GamepadButton::X)]
    );
    assert_eq!(
        p2.bindings(GameAction::Action4),
        &[PlayerSource::PadButton(GamepadButton::Y)]
    );
    assert_eq!(
        p2.bindings(GameAction::Menu),
        &[
            PlayerSource::Keyboard(KeyCode::Escape),
            PlayerSource::PadButton(GamepadButton::Start),
        ]
    );
    assert_eq!(
        p2.bindings(GameAction::Select),
        &[PlayerSource::PadButton(GamepadButton::Select)]
    );
}
