//! Keyboard input handling.

use crate::button_tracker::ButtonTracker;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Represents the state of a keyboard
#[derive(Debug, Default, Clone)]
pub struct KeyboardState {
    keys: ButtonTracker<KeyCode>,
}

impl KeyboardState {
    /// Create a new keyboard state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the keyboard state with a key press event
    pub fn handle_key_press(&mut self, key: KeyCode) {
        self.keys.press(key);
    }

    /// Update the keyboard state with a key release event
    pub fn handle_key_release(&mut self, key: KeyCode) {
        self.keys.release(key);
    }

    /// Check if a key is currently pressed
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys.is_pressed(key)
    }

    /// Check if a key was just pressed this frame
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys.is_just_pressed(key)
    }

    /// Check if a key was just released this frame
    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys.is_just_released(key)
    }

    /// Check if a key was held at the end of the previous frame
    pub fn was_key_pressed(&self, key: KeyCode) -> bool {
        self.keys.was_pressed(key)
    }

    /// Keys that transitioned to pressed this frame, in chronological order
    pub fn just_pressed_keys(&self) -> &[KeyCode] {
        self.keys.just_pressed_buttons()
    }

    /// Clear the just pressed and just released sets for the next frame
    pub fn clear_frame_state(&mut self) {
        self.keys.clear_frame_state();
    }
}

/// Convert a winit physical key to a key code
pub fn convert_physical_key(key: PhysicalKey) -> Option<KeyCode> {
    match key {
        PhysicalKey::Code(code) => Some(code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::NativeKeyCode;

    /// The winit boundary: a winit upgrade that changes `PhysicalKey` breaks
    /// here, not silently in every game's key handling.
    #[test]
    fn test_convert_physical_key_maps_known_codes_and_drops_unidentified_keys() {
        let known = PhysicalKey::Code(KeyCode::KeyA);
        let unknown = PhysicalKey::Unidentified(NativeKeyCode::Unidentified);

        assert_eq!(convert_physical_key(known), Some(KeyCode::KeyA));
        assert_eq!(convert_physical_key(unknown), None);
    }
}
