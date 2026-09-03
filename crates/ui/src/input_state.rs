//! Per-frame input snapshot for UI widgets, plus dt-driven key repeat.
//!
//! Extracted from `interaction.rs`: the snapshot ([`InputState`]) is what
//! widgets read; [`KeyRepeat`] folds held-key repeats into the snapshot's
//! `*_pressed` flags so widgets never need repeat awareness of their own.

use glam::Vec2;
use input::prelude::{InputHandler, KeyCode, MouseButton};

/// Seconds a key must be held before it starts repeating.
pub const REPEAT_DELAY: f32 = 0.4;
/// Seconds between repeats once repeating has started.
pub const REPEAT_INTERVAL: f32 = 0.05;

/// Input state snapshot for UI interaction.
#[derive(Debug, Clone)]
pub struct InputState {
    /// Current mouse position in screen coordinates
    pub mouse_pos: Vec2,
    /// Whether left mouse button is pressed
    pub mouse_down: bool,
    /// Whether left mouse button was just pressed this frame
    pub mouse_just_pressed: bool,
    /// Whether left mouse button was just released this frame
    pub mouse_just_released: bool,
    /// Mouse scroll delta
    pub scroll_delta: f32,
    /// Characters typed this frame (for text input widgets)
    pub typed_chars: Vec<char>,
    /// Whether Enter/Return was just pressed
    pub enter_pressed: bool,
    /// Whether Escape was just pressed
    pub escape_pressed: bool,
    /// Whether Backspace was just pressed (or repeating)
    pub backspace_pressed: bool,
    /// Whether Tab was just pressed
    pub tab_pressed: bool,
    /// Whether ArrowLeft was just pressed (or repeating)
    pub left_pressed: bool,
    /// Whether ArrowRight was just pressed (or repeating)
    pub right_pressed: bool,
    /// Whether Home was just pressed
    pub home_pressed: bool,
    /// Whether End was just pressed
    pub end_pressed: bool,
    /// Whether Delete was just pressed (or repeating)
    pub delete_pressed: bool,
    /// Whether ArrowUp was just pressed (or repeating) — numeric nudge
    pub up_pressed: bool,
    /// Whether ArrowDown was just pressed (or repeating) — numeric nudge
    pub down_pressed: bool,
    /// Whether either Shift key is held (extends selections)
    pub shift_down: bool,
    /// Whether either Ctrl key is held (snaps scrub gestures to whole
    /// steps — issue #56)
    pub ctrl_down: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            mouse_pos: Vec2::ZERO,
            mouse_down: false,
            mouse_just_pressed: false,
            mouse_just_released: false,
            scroll_delta: 0.0,
            typed_chars: Vec::new(),
            enter_pressed: false,
            escape_pressed: false,
            backspace_pressed: false,
            tab_pressed: false,
            left_pressed: false,
            right_pressed: false,
            home_pressed: false,
            end_pressed: false,
            delete_pressed: false,
            up_pressed: false,
            down_pressed: false,
            shift_down: false,
            ctrl_down: false,
        }
    }
}

/// Map a physical KeyCode to a character for text input.
/// Returns None for non-character keys. Covers digits, letters (shift =
/// uppercase), space, and the punctuation text/number fields need.
pub(crate) fn keycode_to_char(key: KeyCode, shift: bool) -> Option<char> {
    use KeyCode::*;
    if let Some(letter) = letter_keycode_to_char(key) {
        return Some(if shift { letter.to_ascii_uppercase() } else { letter });
    }
    match key {
        // Numpad always maps to digits regardless of shift
        Numpad0 => Some('0'),
        Numpad1 => Some('1'),
        Numpad2 => Some('2'),
        Numpad3 => Some('3'),
        Numpad4 => Some('4'),
        Numpad5 => Some('5'),
        Numpad6 => Some('6'),
        Numpad7 => Some('7'),
        Numpad8 => Some('8'),
        Numpad9 => Some('9'),
        NumpadDecimal => Some('.'),
        NumpadSubtract => Some('-'),
        Space => Some(' '),
        // Top-row digits only when shift is not held
        Digit0 if !shift => Some('0'),
        Digit1 if !shift => Some('1'),
        Digit2 if !shift => Some('2'),
        Digit3 if !shift => Some('3'),
        Digit4 if !shift => Some('4'),
        Digit5 if !shift => Some('5'),
        Digit6 if !shift => Some('6'),
        Digit7 if !shift => Some('7'),
        Digit8 if !shift => Some('8'),
        Digit9 if !shift => Some('9'),
        Period if !shift => Some('.'),
        Minus if shift => Some('_'),
        Minus => Some('-'),
        _ => None,
    }
}

/// Lowercase char for a letter keycode, `None` for everything else.
fn letter_keycode_to_char(key: KeyCode) -> Option<char> {
    use KeyCode::*;
    Some(match key {
        KeyA => 'a', KeyB => 'b', KeyC => 'c', KeyD => 'd', KeyE => 'e',
        KeyF => 'f', KeyG => 'g', KeyH => 'h', KeyI => 'i', KeyJ => 'j',
        KeyK => 'k', KeyL => 'l', KeyM => 'm', KeyN => 'n', KeyO => 'o',
        KeyP => 'p', KeyQ => 'q', KeyR => 'r', KeyS => 's', KeyT => 't',
        KeyU => 'u', KeyV => 'v', KeyW => 'w', KeyX => 'x', KeyY => 'y',
        KeyZ => 'z',
        _ => return None,
    })
}

impl InputState {
    /// Create input state from an InputHandler (no key repeat — every
    /// `*_pressed` flag reflects just-pressed edges only).
    pub fn from_input_handler(input: &InputHandler) -> Self {
        Self::from_input_handler_with_repeat(input, &mut KeyRepeat::default(), 0.0)
    }

    /// Create input state from an InputHandler, folding held-key repeats
    /// (arrows, Backspace, Delete) into the `*_pressed` flags via `repeat`.
    pub fn from_input_handler_with_repeat(
        input: &InputHandler,
        repeat: &mut KeyRepeat,
        dt: f32,
    ) -> Self {
        let mouse = input.mouse();
        let pos = mouse.position();
        let kb = input.keyboard();

        let shift = kb.is_key_pressed(KeyCode::ShiftLeft)
            || kb.is_key_pressed(KeyCode::ShiftRight);
        let ctrl = kb.is_key_pressed(KeyCode::ControlLeft)
            || kb.is_key_pressed(KeyCode::ControlRight);

        // Collect typed characters from just-pressed keys (no char repeat)
        let typed_keys = [
            KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7,
            KeyCode::Digit8, KeyCode::Digit9,
            KeyCode::Numpad0, KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3,
            KeyCode::Numpad4, KeyCode::Numpad5, KeyCode::Numpad6, KeyCode::Numpad7,
            KeyCode::Numpad8, KeyCode::Numpad9,
            KeyCode::Period, KeyCode::NumpadDecimal,
            KeyCode::Minus, KeyCode::NumpadSubtract,
            KeyCode::Space,
            KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD,
            KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH,
            KeyCode::KeyI, KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL,
            KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO, KeyCode::KeyP,
            KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT,
            KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX,
            KeyCode::KeyY, KeyCode::KeyZ,
        ];

        let mut typed_chars = Vec::new();
        for &key in &typed_keys {
            if kb.is_key_just_pressed(key) {
                if let Some(ch) = keycode_to_char(key, shift) {
                    typed_chars.push(ch);
                }
            }
        }

        let mut repeating = |slot: RepeatKey, key: KeyCode| {
            repeat.tick(slot, kb.is_key_pressed(key), kb.is_key_just_pressed(key), dt)
        };
        let left_pressed = repeating(RepeatKey::Left, KeyCode::ArrowLeft);
        let right_pressed = repeating(RepeatKey::Right, KeyCode::ArrowRight);
        let backspace_pressed = repeating(RepeatKey::Backspace, KeyCode::Backspace);
        let delete_pressed = repeating(RepeatKey::Delete, KeyCode::Delete);
        let up_pressed = repeating(RepeatKey::Up, KeyCode::ArrowUp);
        let down_pressed = repeating(RepeatKey::Down, KeyCode::ArrowDown);

        Self {
            mouse_pos: Vec2::new(pos.x, pos.y),
            mouse_down: mouse.is_button_pressed(MouseButton::Left),
            mouse_just_pressed: mouse.is_button_just_pressed(MouseButton::Left),
            mouse_just_released: mouse.is_button_just_released(MouseButton::Left),
            scroll_delta: mouse.wheel_delta(),
            typed_chars,
            enter_pressed: kb.is_key_just_pressed(KeyCode::Enter)
                || kb.is_key_just_pressed(KeyCode::NumpadEnter),
            escape_pressed: kb.is_key_just_pressed(KeyCode::Escape),
            backspace_pressed,
            tab_pressed: kb.is_key_just_pressed(KeyCode::Tab),
            left_pressed,
            right_pressed,
            home_pressed: kb.is_key_just_pressed(KeyCode::Home),
            end_pressed: kb.is_key_just_pressed(KeyCode::End),
            delete_pressed,
            up_pressed,
            down_pressed,
            shift_down: shift,
            ctrl_down: ctrl,
        }
    }
}

/// Keys with dt-driven repeat while held.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RepeatKey {
    Left = 0,
    Right = 1,
    Backspace = 2,
    Delete = 3,
    Up = 4,
    Down = 5,
}

/// Per-key hold timer: fires on the initial press, then after
/// [`REPEAT_DELAY`] fires every [`REPEAT_INTERVAL`] while held.
#[derive(Debug, Clone, Copy, Default)]
struct RepeatTimer {
    held: f32,
    since_fire: f32,
}

impl RepeatTimer {
    fn tick(&mut self, held: bool, just_pressed: bool, dt: f32) -> bool {
        if just_pressed {
            self.held = 0.0;
            self.since_fire = 0.0;
            return true;
        }
        if !held {
            self.held = 0.0;
            self.since_fire = 0.0;
            return false;
        }
        let was = self.held;
        self.held += dt;
        // First repeat exactly when the hold crosses the delay...
        if was < REPEAT_DELAY {
            if self.held >= REPEAT_DELAY {
                self.since_fire = 0.0;
                return true;
            }
            return false;
        }
        // ...then every interval while held. Subtract (not reset) so the
        // remainder carries over and the average rate stays 1/INTERVAL even
        // when the frame delta doesn't divide the interval evenly.
        self.since_fire += dt;
        if self.since_fire >= REPEAT_INTERVAL {
            self.since_fire -= REPEAT_INTERVAL;
            return true;
        }
        false
    }
}

/// Repeat timers for all navigation/deletion keys a text input uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyRepeat {
    timers: [RepeatTimer; 6],
}

impl KeyRepeat {
    fn tick(&mut self, key: RepeatKey, held: bool, just_pressed: bool, dt: f32) -> bool {
        self.timers[key as usize].tick(held, just_pressed, dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{input_with_mouse, release_mouse};

    #[test]
    fn test_input_state_mirrors_the_handlers_mouse_snapshot() {
        let idle = InputState::from_input_handler(&InputHandler::new());
        assert_eq!(idle.mouse_pos, Vec2::ZERO);
        assert!(!idle.mouse_down && !idle.mouse_just_pressed && !idle.mouse_just_released);
        assert_eq!(idle.scroll_delta, 0.0);

        let mut input = input_with_mouse(Vec2::new(100.0, 200.0), true);
        input.mouse_mut().update_wheel_delta(1.5);
        let pressed = InputState::from_input_handler(&input);
        assert_eq!(pressed.mouse_pos, Vec2::new(100.0, 200.0));
        assert!(pressed.mouse_down && pressed.mouse_just_pressed);
        assert!(!pressed.mouse_just_released);
        assert_eq!(pressed.scroll_delta, 1.5);

        release_mouse(&mut input);
        let released = InputState::from_input_handler(&input);
        assert!(!released.mouse_down && !released.mouse_just_pressed);
        assert!(released.mouse_just_released);
        assert_eq!(released.scroll_delta, 0.0, "the wheel delta is per frame");
    }

    #[test]
    fn test_keycode_to_char_maps_letters_digits_and_punctuation_with_shift_rules() {
        let cases = [
            (KeyCode::KeyA, false, Some('a')),
            (KeyCode::KeyA, true, Some('A')),
            (KeyCode::KeyZ, true, Some('Z')),
            (KeyCode::Space, false, Some(' ')),
            (KeyCode::Space, true, Some(' ')),
            (KeyCode::Digit0, false, Some('0')),
            (KeyCode::Digit9, false, Some('9')),
            (KeyCode::Digit0, true, None), // Shift+0 = ')', not a digit
            (KeyCode::Numpad5, false, Some('5')),
            (KeyCode::Numpad5, true, Some('5')), // the numpad ignores shift
            (KeyCode::Period, false, Some('.')),
            (KeyCode::Period, true, None), // Shift+. = '>'
            (KeyCode::NumpadDecimal, false, Some('.')),
            (KeyCode::Minus, false, Some('-')),
            (KeyCode::Minus, true, Some('_')),
            (KeyCode::NumpadSubtract, true, Some('-')),
            (KeyCode::Enter, false, None),
            (KeyCode::Backspace, false, None),
            (KeyCode::F1, false, None),
        ];

        for (key, shift, expected) in cases {
            assert_eq!(keycode_to_char(key, shift), expected, "{key:?} shift={shift}");
        }
    }

    #[test]
    fn test_repeat_fires_on_press_then_after_the_delay_at_the_interval_and_resets_on_release() {
        const FRAME: f32 = 0.016;
        let mut timer = RepeatTimer::default();
        assert!(timer.tick(true, true, FRAME), "the initial press fires");

        // Held for just under the delay: silent.
        let mut held_for = 0.0;
        let mut fired = 0;
        while held_for + FRAME < REPEAT_DELAY {
            held_for += FRAME;
            fired += usize::from(timer.tick(true, false, FRAME));
        }
        assert_eq!(fired, 0, "no repeat before the delay");
        assert!(timer.tick(true, false, FRAME), "crossing the delay fires once");

        // Then ~1/REPEAT_INTERVAL per second of holding (a frame of slack).
        let fired = (0..63).filter(|_| timer.tick(true, false, FRAME)).count() as i32; // 63 × 0.016 ≈ 1s
        let expected = (1.0 / REPEAT_INTERVAL) as i32;
        assert!((fired - expected).abs() <= 2, "expected ~{expected} repeats in 1s, got {fired}");

        // Release resets; a fresh press fires and waits the full delay again.
        assert!(!timer.tick(false, false, FRAME));
        assert!(timer.tick(true, true, FRAME));
        assert!(!timer.tick(true, false, FRAME), "the delay applies again after a re-press");
    }

    #[test]
    fn test_key_repeat_timers_are_independent_per_key() {
        // `timers[key as usize]` over a hand-numbered enum: holding one key
        // must never advance another key's slot, and each key's press flag
        // reaches its own `InputState` field.
        let mut input = InputHandler::new();
        let mut repeat = KeyRepeat::default();

        input.keyboard_mut().handle_key_press(KeyCode::ArrowLeft);
        let state = InputState::from_input_handler_with_repeat(&input, &mut repeat, 0.016);
        assert!(state.left_pressed && !state.backspace_pressed);

        input.update();
        let state = InputState::from_input_handler_with_repeat(&input, &mut repeat, REPEAT_DELAY + 0.001);
        assert!(state.left_pressed, "held ArrowLeft repeats after the delay");
        assert!(!state.backspace_pressed && !state.right_pressed && !state.up_pressed && !state.down_pressed && !state.delete_pressed);

        input.update();
        input.keyboard_mut().handle_key_press(KeyCode::Backspace);
        let state = InputState::from_input_handler_with_repeat(&input, &mut repeat, REPEAT_INTERVAL);
        assert!(state.backspace_pressed, "the fresh Backspace press fires");
        assert!(state.left_pressed, "ArrowLeft keeps repeating at the interval");

        input.update();
        let state = InputState::from_input_handler_with_repeat(&input, &mut repeat, REPEAT_INTERVAL);
        assert!(state.left_pressed, "ArrowLeft is still past its delay");
        assert!(!state.backspace_pressed, "Backspace is inside its own delay — ArrowLeft's hold did not advance it");
    }
}
