//! Gamepad hardware backend: drains gilrs each frame into [`InputHandler`]
//! events.
//!
//! The `input` crate stays hardware-agnostic — it only consumes
//! [`InputEvent`]s. This module owns the gilrs instance and the boundary
//! translation: gilrs buttons/axes → the input crate's `GamepadButton` /
//! `GamepadAxis`, dead-zone normalization, and hat-switch dpads (reported by
//! some pads as `DPadX`/`DPadY` axes) synthesized into dpad button events.
//!
//! Construction never fails the game: [`GamepadBackend::new_or_disabled`]
//! degrades to a no-op backend when gilrs can't initialize (headless CI,
//! missing devices), same pattern as `AudioManager::new_or_disabled`.

use std::collections::HashMap;

use input::{AxisDirection, GamepadAxis, GamepadButton, InputEvent, InputHandler};

/// Stick/trigger deflection below this reads as 0.0; above it, values are
/// rescaled so the usable range still spans 0.0..=1.0.
const DEAD_ZONE: f32 = 0.15;

/// Hat-switch threshold: a `DPadX`/`DPadY` axis past this (in either
/// direction) counts as the corresponding dpad button being held.
const HAT_THRESHOLD: f32 = 0.5;

/// Polls connected gamepads and queues their state changes as input events.
pub struct GamepadBackend {
    /// `None` = disabled backend; `pump()` is a no-op.
    gilrs: Option<gilrs::Gilrs>,
    /// Last seen hat-axis value per (pad, axis), for transition-only dpad
    /// button synthesis (ButtonTracker's `release` is unconditional, so
    /// releases must only be emitted for directions actually held).
    hat_state: HashMap<(u32, HatAxis), f32>,
}

/// The two hat-switch axes some pads report instead of dpad buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HatAxis {
    X,
    Y,
}

impl GamepadBackend {
    /// Initialize gilrs, degrading to a disabled (no-op) backend if the
    /// platform has no gamepad support available.
    pub fn new_or_disabled() -> Self {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Self { gilrs: Some(gilrs), hat_state: HashMap::new() },
            Err(e) => {
                log::warn!("Gamepad backend unavailable ({e}); controllers disabled");
                Self::disabled()
            }
        }
    }

    /// A backend that never produces events (tests, headless).
    pub fn disabled() -> Self {
        Self { gilrs: None, hat_state: HashMap::new() }
    }

    /// Whether a live gilrs instance is behind this backend.
    pub fn is_enabled(&self) -> bool {
        self.gilrs.is_some()
    }

    /// Drain all pending gilrs events into the input handler's queue.
    /// Call once per frame, before `InputHandler::process_queued_events`.
    pub fn pump(&mut self, input: &mut InputHandler) {
        let Some(gilrs) = self.gilrs.as_mut() else { return };
        while let Some(event) = gilrs.next_event() {
            let pad: u32 = usize::from(event.id) as u32;
            match event.event {
                gilrs::EventType::ButtonPressed(button, _) => {
                    if let Some(button) = translate_button(button) {
                        input.queue_event(InputEvent::GamepadButtonPressed(pad, button));
                    }
                }
                gilrs::EventType::ButtonReleased(button, _) => {
                    if let Some(button) = translate_button(button) {
                        input.queue_event(InputEvent::GamepadButtonReleased(pad, button));
                    }
                }
                gilrs::EventType::ButtonChanged(button, value, _) => {
                    // Analog triggers also arrive as button-value changes;
                    // mirror them onto the trigger axes so games can read
                    // pressure via axis_value().
                    let axis = match button {
                        gilrs::Button::LeftTrigger2 => Some(GamepadAxis::LeftTrigger),
                        gilrs::Button::RightTrigger2 => Some(GamepadAxis::RightTrigger),
                        _ => None,
                    };
                    if let Some(axis) = axis {
                        input.queue_event(InputEvent::GamepadAxisUpdated(
                            pad,
                            axis,
                            apply_dead_zone(value),
                        ));
                    }
                }
                gilrs::EventType::AxisChanged(axis, value, _) => match hat_axis(axis) {
                    Some(hat) => {
                        let prev = self.hat_state.insert((pad, hat), value).unwrap_or(0.0);
                        for event in hat_transition_events(pad, hat, prev, value) {
                            input.queue_event(event);
                        }
                    }
                    None => {
                        if let Some(axis) = translate_axis(axis) {
                            input.queue_event(InputEvent::GamepadAxisUpdated(
                                pad,
                                axis,
                                apply_dead_zone(value),
                            ));
                        }
                    }
                },
                gilrs::EventType::Connected => {
                    log::info!("Gamepad {pad} connected");
                    input.queue_event(InputEvent::GamepadConnected(pad));
                }
                gilrs::EventType::Disconnected => {
                    log::info!("Gamepad {pad} disconnected");
                    self.hat_state.retain(|&(p, _), _| p != pad);
                    input.queue_event(InputEvent::GamepadDisconnected(pad));
                }
                _ => {}
            }
        }
    }
}

/// gilrs button → our button. gilrs "LeftTrigger" is the bumper (L1) and
/// "LeftTrigger2" the analog trigger (L2); `Mode`/`C`/`Z`/`Unknown` have no
/// equivalent and are dropped.
pub(crate) fn translate_button(button: gilrs::Button) -> Option<GamepadButton> {
    use gilrs::Button as B;
    match button {
        B::South => Some(GamepadButton::A),
        B::East => Some(GamepadButton::B),
        B::West => Some(GamepadButton::X),
        B::North => Some(GamepadButton::Y),
        B::LeftTrigger => Some(GamepadButton::LeftBumper),
        B::RightTrigger => Some(GamepadButton::RightBumper),
        B::LeftTrigger2 => Some(GamepadButton::LeftTrigger),
        B::RightTrigger2 => Some(GamepadButton::RightTrigger),
        B::LeftThumb => Some(GamepadButton::LeftStick),
        B::RightThumb => Some(GamepadButton::RightStick),
        B::Start => Some(GamepadButton::Start),
        B::Select => Some(GamepadButton::Select),
        B::DPadUp => Some(GamepadButton::DPadUp),
        B::DPadDown => Some(GamepadButton::DPadDown),
        B::DPadLeft => Some(GamepadButton::DPadLeft),
        B::DPadRight => Some(GamepadButton::DPadRight),
        B::C | B::Z | B::Mode | B::Unknown => None,
    }
}

/// gilrs axis → our axis. Sticks map 1:1 (gilrs stick Y is +up, matching
/// `GamepadAxis`'s documented convention); `LeftZ`/`RightZ` are the analog
/// triggers on some drivers. Hat axes are handled separately.
pub(crate) fn translate_axis(axis: gilrs::Axis) -> Option<GamepadAxis> {
    use gilrs::Axis as A;
    match axis {
        A::LeftStickX => Some(GamepadAxis::LeftStickX),
        A::LeftStickY => Some(GamepadAxis::LeftStickY),
        A::RightStickX => Some(GamepadAxis::RightStickX),
        A::RightStickY => Some(GamepadAxis::RightStickY),
        A::LeftZ => Some(GamepadAxis::LeftTrigger),
        A::RightZ => Some(GamepadAxis::RightTrigger),
        A::DPadX | A::DPadY | A::Unknown => None,
    }
}

/// Which hat axis a gilrs axis is, if any.
pub(crate) fn hat_axis(axis: gilrs::Axis) -> Option<HatAxis> {
    match axis {
        gilrs::Axis::DPadX => Some(HatAxis::X),
        gilrs::Axis::DPadY => Some(HatAxis::Y),
        _ => None,
    }
}

/// Dead-zone normalization: values inside `DEAD_ZONE` read 0.0; outside it,
/// the remaining range rescales to 0.0..=1.0 with the sign preserved, so a
/// full deflection still reads exactly ±1.0.
pub(crate) fn apply_dead_zone(value: f32) -> f32 {
    if value.abs() < DEAD_ZONE {
        return 0.0;
    }
    value.signum() * ((value.abs() - DEAD_ZONE) / (1.0 - DEAD_ZONE)).min(1.0)
}

/// Dpad button events for a hat-axis transition `prev` → `now`.
///
/// Emits presses/releases only on actual threshold crossings — releases are
/// never emitted for a direction that wasn't held (ButtonTracker's `release`
/// unconditionally records a just-released edge, so a spurious release would
/// leak phantom edges into action mappings). gilrs hat Y is +up.
pub(crate) fn hat_transition_events(
    pad: u32,
    hat: HatAxis,
    prev: f32,
    now: f32,
) -> Vec<InputEvent> {
    let (negative, positive) = match hat {
        HatAxis::X => (GamepadButton::DPadLeft, GamepadButton::DPadRight),
        HatAxis::Y => (GamepadButton::DPadDown, GamepadButton::DPadUp),
    };
    let mut events = Vec::new();
    for (button, direction) in [(negative, AxisDirection::Negative), (positive, AxisDirection::Positive)] {
        let was = past_hat_threshold(prev, direction);
        let is = past_hat_threshold(now, direction);
        match (was, is) {
            (false, true) => events.push(InputEvent::GamepadButtonPressed(pad, button)),
            (true, false) => events.push(InputEvent::GamepadButtonReleased(pad, button)),
            _ => {}
        }
    }
    events
}

fn past_hat_threshold(value: f32, direction: AxisDirection) -> bool {
    match direction {
        AxisDirection::Positive => value >= HAT_THRESHOLD,
        AxisDirection::Negative => value <= -HAT_THRESHOLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_translation_table_is_exhaustive_and_correct() {
        use gilrs::Button as B;
        let expected = [
            (B::South, Some(GamepadButton::A)),
            (B::East, Some(GamepadButton::B)),
            (B::West, Some(GamepadButton::X)),
            (B::North, Some(GamepadButton::Y)),
            (B::LeftTrigger, Some(GamepadButton::LeftBumper)),
            (B::RightTrigger, Some(GamepadButton::RightBumper)),
            (B::LeftTrigger2, Some(GamepadButton::LeftTrigger)),
            (B::RightTrigger2, Some(GamepadButton::RightTrigger)),
            (B::LeftThumb, Some(GamepadButton::LeftStick)),
            (B::RightThumb, Some(GamepadButton::RightStick)),
            (B::Start, Some(GamepadButton::Start)),
            (B::Select, Some(GamepadButton::Select)),
            (B::DPadUp, Some(GamepadButton::DPadUp)),
            (B::DPadDown, Some(GamepadButton::DPadDown)),
            (B::DPadLeft, Some(GamepadButton::DPadLeft)),
            (B::DPadRight, Some(GamepadButton::DPadRight)),
            (B::C, None),
            (B::Z, None),
            (B::Mode, None),
            (B::Unknown, None),
        ];
        for (gilrs_button, ours) in expected {
            assert_eq!(translate_button(gilrs_button), ours, "{gilrs_button:?}");
        }

        // Axes: sticks and Z triggers map, the hats are dpad buttons instead.
        use gilrs::Axis as A;
        for (gilrs_axis, ours) in [
            (A::LeftStickX, Some(GamepadAxis::LeftStickX)),
            (A::LeftStickY, Some(GamepadAxis::LeftStickY)),
            (A::RightStickX, Some(GamepadAxis::RightStickX)),
            (A::RightStickY, Some(GamepadAxis::RightStickY)),
            (A::LeftZ, Some(GamepadAxis::LeftTrigger)),
            (A::RightZ, Some(GamepadAxis::RightTrigger)),
            (A::DPadX, None),
            (A::DPadY, None),
            (A::Unknown, None),
        ] {
            assert_eq!(translate_axis(gilrs_axis), ours, "{gilrs_axis:?}");
        }
        assert_eq!(hat_axis(A::DPadX), Some(HatAxis::X));
        assert_eq!(hat_axis(A::DPadY), Some(HatAxis::Y));
        assert_eq!(hat_axis(A::LeftStickX), None);
    }

    #[test]
    fn dead_zone_zeroes_small_values_and_rescales_the_rest() {
        assert_eq!(apply_dead_zone(0.0), 0.0);
        assert_eq!(apply_dead_zone(0.1), 0.0);
        assert_eq!(apply_dead_zone(-0.14), 0.0);
        // Full deflection still reads exactly ±1.0
        assert_eq!(apply_dead_zone(1.0), 1.0);
        assert_eq!(apply_dead_zone(-1.0), -1.0);
        // Just past the dead zone reads near zero (rescaled, sign kept)
        let just_past = apply_dead_zone(0.16);
        assert!(just_past > 0.0 && just_past < 0.05, "{just_past}");
        let negative = apply_dead_zone(-0.5);
        assert!((-0.412..=-0.411).contains(&negative), "{negative}");
    }

    #[test]
    fn hat_transitions_press_and_release_only_on_crossings() {
        // Center → right: press right only
        assert_eq!(
            hat_transition_events(0, HatAxis::X, 0.0, 1.0),
            vec![InputEvent::GamepadButtonPressed(0, GamepadButton::DPadRight)]
        );
        // Held right: nothing (no just-pressed retrigger)
        assert_eq!(hat_transition_events(0, HatAxis::X, 1.0, 1.0), Vec::new());
        // Right → center: release right only — never a phantom left release
        assert_eq!(
            hat_transition_events(0, HatAxis::X, 1.0, 0.0),
            vec![InputEvent::GamepadButtonReleased(0, GamepadButton::DPadRight)]
        );
        // Right → left in one event: release right AND press left
        let swap = hat_transition_events(0, HatAxis::X, 1.0, -1.0);
        assert!(swap.contains(&InputEvent::GamepadButtonPressed(0, GamepadButton::DPadLeft)));
        assert!(swap.contains(&InputEvent::GamepadButtonReleased(0, GamepadButton::DPadRight)));
        // Y axis: +1 = up (gilrs convention)
        assert_eq!(
            hat_transition_events(1, HatAxis::Y, 0.0, 1.0),
            vec![InputEvent::GamepadButtonPressed(1, GamepadButton::DPadUp)]
        );
        assert_eq!(
            hat_transition_events(1, HatAxis::Y, 0.0, -1.0),
            vec![InputEvent::GamepadButtonPressed(1, GamepadButton::DPadDown)]
        );

        // A disabled backend (no gilrs on this machine) pumps as a no-op.
        let mut backend = GamepadBackend::disabled();
        assert!(!backend.is_enabled());
        let mut input = InputHandler::new();
        backend.pump(&mut input);
        input.process_queued_events();
        assert!(input.gamepads().connected_ids().is_empty());
    }
}
