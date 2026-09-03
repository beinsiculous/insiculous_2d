//! Shared menu-screen input: one frame's navigation signals plus wraparound
//! list navigation.
//!
//! Every arcade game's title/select screens read the same four signals
//! (up, down, confirm, back) and move a cursor through a wrapping list.
//! Signals come from the keyboard (W/↑, S/↓, Space/Enter/NumpadEnter, Escape) and from
//! **every connected gamepad** (dpad / left stick, A or Start, B) — menus
//! don't care which player navigates. The engine owns that mechanism; games
//! own what each screen and selection *means*.
//!
//! ```no_run
//! use engine_core::prelude::*;
//!
//! # fn update(ctx: &mut GameContext, selection: u8) {
//! let input = MenuInput::read(ctx.input);
//! let selection = input.navigate(selection, 3);
//! if input.confirm { /* enter the selected item */ }
//! if input.back { /* return to the previous screen */ }
//! # }
//! ```

use input::{AxisDirection, GamepadAxis, GamepadButton, GamepadState, InputHandler, AXIS_ACTIVATION_THRESHOLD};
use winit::keyboard::KeyCode;

/// One frame's worth of menu signals, read once per screen update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuInput {
    /// W/ArrowUp, or any pad's DPadUp / left-stick-up edge, was just pressed.
    pub up: bool,
    /// S/ArrowDown, or any pad's DPadDown / left-stick-down edge, was just pressed.
    pub down: bool,
    /// Space/Enter/NumpadEnter, or any pad's A or Start, was just pressed.
    pub confirm: bool,
    /// Escape, or any pad's B, was just pressed.
    pub back: bool,
}

impl MenuInput {
    /// Read this frame's menu signals from the input handler (keyboard plus
    /// every connected gamepad).
    pub fn read(input: &InputHandler) -> Self {
        let mut menu = Self {
            up: input.is_key_just_pressed(KeyCode::ArrowUp)
                || input.is_key_just_pressed(KeyCode::KeyW),
            down: input.is_key_just_pressed(KeyCode::ArrowDown)
                || input.is_key_just_pressed(KeyCode::KeyS),
            confirm: input.is_key_just_pressed(KeyCode::Space)
                || input.is_key_just_pressed(KeyCode::Enter)
                || input.is_key_just_pressed(KeyCode::NumpadEnter),
            back: input.is_key_just_pressed(KeyCode::Escape),
        };
        for (_, pad) in input.gamepads().iter() {
            menu.up |= pad_up(pad);
            menu.down |= pad_down(pad);
            menu.confirm |= pad.is_button_just_pressed(GamepadButton::A)
                || pad.is_button_just_pressed(GamepadButton::Start);
            menu.back |= pad.is_button_just_pressed(GamepadButton::B);
        }
        menu
    }

    /// Move `current` through a `count`-item list with wraparound.
    /// `up` takes precedence when both directions fire on the same frame.
    pub fn navigate(&self, current: u8, count: u8) -> u8 {
        if count == 0 {
            return 0;
        }
        if self.up {
            if current == 0 { count - 1 } else { current - 1 }
        } else if self.down {
            (current + 1) % count
        } else {
            current
        }
    }
}

/// Pad "up" edge: DPadUp just pressed, or the left stick just crossed the
/// activation threshold upward (stick Y positive = up). Edge detection (not
/// level) so a held stick doesn't repeat-scroll every frame.
fn pad_up(pad: &GamepadState) -> bool {
    pad.is_button_just_pressed(GamepadButton::DPadUp)
        || pad.axis_just_activated(
            GamepadAxis::LeftStickY,
            AxisDirection::Positive,
            AXIS_ACTIVATION_THRESHOLD,
        )
}

/// Pad "down" edge: DPadDown just pressed, or the left stick just crossed
/// the activation threshold downward.
fn pad_down(pad: &GamepadState) -> bool {
    pad.is_button_just_pressed(GamepadButton::DPadDown)
        || pad.axis_just_activated(
            GamepadAxis::LeftStickY,
            AxisDirection::Negative,
            AXIS_ACTIVATION_THRESHOLD,
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::frame;
    use input::InputEvent;

    fn keys(up: bool, down: bool) -> MenuInput {
        MenuInput { up, down, confirm: false, back: false }
    }

    #[test]
    fn navigate_wraps_prefers_up_and_survives_an_empty_list() {
        for (up, down, current, count, expected, why) in [
            (true, false, 0, 3, 2, "up from the top wraps to the last row"),
            (false, true, 2, 3, 0, "down from the bottom wraps to the first row"),
            (false, false, 1, 3, 1, "no input holds the row"),
            (true, true, 1, 3, 0, "up wins over down on the same frame"),
            (true, false, 0, 0, 0, "an empty list must not underflow `count - 1`"),
            (false, true, 0, 0, 0, "an empty list must not wrap either"),
        ] {
            assert_eq!(keys(up, down).navigate(current, count), expected, "{why}");
        }
    }

    #[test]
    fn held_stick_scrolls_once_and_every_pad_drives_menus_alongside_the_keyboard() {
        let mut handler = InputHandler::new();

        // Frame 1: pad 1's stick pushed up — the EDGE fires (not just pad 0).
        frame(&mut handler, &[InputEvent::GamepadAxisUpdated(1, GamepadAxis::LeftStickY, 0.9)]);
        assert!(MenuInput::read(&handler).up);

        // Frame 2: stick still held — no repeat, a menu scrolls one row per push.
        frame(&mut handler, &[InputEvent::GamepadAxisUpdated(1, GamepadAxis::LeftStickY, 0.9)]);
        assert!(!MenuInput::read(&handler).up);

        // Frame 3: stick pushed down — the down edge fires, up stays clear.
        frame(&mut handler, &[InputEvent::GamepadAxisUpdated(1, GamepadAxis::LeftStickY, -0.9)]);
        let input = MenuInput::read(&handler);
        assert_eq!((input.up, input.down), (false, true));

        // Frame 4: dpad on pad 1 navigates; A on pad 0 confirms and B on pad 1
        // backs out in the same frame — every connected pad counts.
        frame(&mut handler, &[
            InputEvent::GamepadAxisUpdated(1, GamepadAxis::LeftStickY, 0.0),
            InputEvent::GamepadButtonPressed(1, GamepadButton::DPadDown),
            InputEvent::GamepadButtonPressed(0, GamepadButton::A),
            InputEvent::GamepadButtonPressed(1, GamepadButton::B),
        ]);
        let input = MenuInput::read(&handler);
        assert_eq!(input, MenuInput { up: false, down: true, confirm: true, back: true });

        // Frame 5: released dpad no longer navigates; the numpad's Enter
        // confirms like the main Enter, with an idle pad still connected.
        frame(&mut handler, &[
            InputEvent::GamepadButtonReleased(1, GamepadButton::DPadDown),
            InputEvent::GamepadButtonReleased(0, GamepadButton::A),
            InputEvent::GamepadButtonReleased(1, GamepadButton::B),
            InputEvent::KeyPressed(KeyCode::NumpadEnter),
        ]);
        let input = MenuInput::read(&handler);
        assert_eq!(input, MenuInput { up: false, down: false, confirm: true, back: false });

        // Frame 6: keyboard W with the idle pad connected reads as plain up.
        frame(&mut handler, &[
            InputEvent::KeyReleased(KeyCode::NumpadEnter),
            InputEvent::KeyPressed(KeyCode::KeyW),
        ]);
        let input = MenuInput::read(&handler);
        assert_eq!(input, MenuInput { up: true, down: false, confirm: false, back: false });
    }
}
