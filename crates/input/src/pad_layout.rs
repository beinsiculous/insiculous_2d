//! Standard gamepad layout constants.

use crate::gamepad::{AxisDirection, GamepadAxis, GamepadButton};
use crate::input_mapping::GameAction;
use crate::player::PlayerSource;

/// The standard pad layout, device-relative: dpad + left stick -> movement,
/// A/B/X/Y -> Action1-4, Start -> Menu, Select -> Select.
pub const STANDARD_PAD_LAYOUT: &[(GameAction, PlayerSource)] = &[
    (GameAction::MoveUp, PlayerSource::PadButton(GamepadButton::DPadUp)),
    (GameAction::MoveDown, PlayerSource::PadButton(GamepadButton::DPadDown)),
    (GameAction::MoveLeft, PlayerSource::PadButton(GamepadButton::DPadLeft)),
    (GameAction::MoveRight, PlayerSource::PadButton(GamepadButton::DPadRight)),
    (
        GameAction::MoveUp,
        PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Positive),
    ),
    (
        GameAction::MoveDown,
        PlayerSource::PadAxis(GamepadAxis::LeftStickY, AxisDirection::Negative),
    ),
    (
        GameAction::MoveLeft,
        PlayerSource::PadAxis(GamepadAxis::LeftStickX, AxisDirection::Negative),
    ),
    (
        GameAction::MoveRight,
        PlayerSource::PadAxis(GamepadAxis::LeftStickX, AxisDirection::Positive),
    ),
    (GameAction::Action1, PlayerSource::PadButton(GamepadButton::A)),
    (GameAction::Action2, PlayerSource::PadButton(GamepadButton::B)),
    (GameAction::Action3, PlayerSource::PadButton(GamepadButton::X)),
    (GameAction::Action4, PlayerSource::PadButton(GamepadButton::Y)),
    (GameAction::Menu, PlayerSource::PadButton(GamepadButton::Start)),
    (GameAction::Select, PlayerSource::PadButton(GamepadButton::Select)),
];
