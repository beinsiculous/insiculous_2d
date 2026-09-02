//! Shared press/release state tracking for digital inputs.
//!
//! Keyboard keys, mouse buttons, and gamepad buttons all follow the same
//! state model: a set of currently-held buttons plus one-frame "just pressed"
//! and "just released" sets. [`ButtonTracker`] implements that model once so
//! each device type composes it instead of re-implementing it.

use std::collections::HashSet;
use std::hash::Hash;

/// Tracks pressed / just-pressed / just-released state for a digital input type.
///
/// `T` is the button identifier (e.g. `KeyCode`, `MouseButton`, `GamepadButton`).
///
/// # Frame Lifecycle
///
/// - `press()` / `release()` are called while processing input events
/// - `is_*` queries are valid for the rest of the frame
/// - `clear_frame_state()` must be called at end of frame to reset the
///   one-shot "just pressed" / "just released" sets
#[derive(Debug, Clone)]
pub struct ButtonTracker<T: Copy + Eq + Hash> {
    /// Currently held buttons
    pressed: HashSet<T>,
    /// Buttons that transitioned to pressed this frame
    just_pressed: HashSet<T>,
    /// Buttons that transitioned to released this frame
    just_released: HashSet<T>,
}

impl<T: Copy + Eq + Hash> Default for ButtonTracker<T> {
    fn default() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> ButtonTracker<T> {
    /// Create a new, empty tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a press event. Repeated presses while held do not re-trigger "just pressed".
    pub fn press(&mut self, button: T) {
        if self.pressed.insert(button) {
            self.just_pressed.insert(button);
        }
    }

    /// Record a release event
    pub fn release(&mut self, button: T) {
        self.pressed.remove(&button);
        self.just_released.insert(button);
    }

    /// Check if a button is currently held
    pub fn is_pressed(&self, button: T) -> bool {
        self.pressed.contains(&button)
    }

    /// Check if a button transitioned to pressed this frame
    pub fn is_just_pressed(&self, button: T) -> bool {
        self.just_pressed.contains(&button)
    }

    /// Check if a button transitioned to released this frame
    pub fn is_just_released(&self, button: T) -> bool {
        self.just_released.contains(&button)
    }

    /// Clear the one-shot just-pressed / just-released sets for the next frame
    pub fn clear_frame_state(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edges_last_one_frame_and_key_repeat_does_not_retrigger() {
        let mut tracker = ButtonTracker::new();

        tracker.press(1u32);
        assert!(tracker.is_pressed(1));
        assert!(tracker.is_just_pressed(1));
        assert!(!tracker.is_just_released(1));

        // End of frame: the edge clears, the hold does not
        tracker.clear_frame_state();
        assert!(tracker.is_pressed(1));
        assert!(!tracker.is_just_pressed(1));
        assert!(!tracker.is_just_released(1));

        // OS key-repeat while held is not a new edge
        tracker.press(1u32);
        assert!(tracker.is_pressed(1));
        assert!(
            !tracker.is_just_pressed(1),
            "a repeated press while held must not re-trigger just_pressed"
        );

        // Releasing one button leaves the other held
        tracker.press(2u32);
        tracker.release(1u32);
        assert!(!tracker.is_pressed(1));
        assert!(tracker.is_just_released(1));
        assert!(tracker.is_pressed(2));
        assert!(tracker.is_just_pressed(2));
        assert!(!tracker.is_just_released(2));

        tracker.clear_frame_state();
        assert!(!tracker.is_pressed(1));
        assert!(!tracker.is_just_released(1));
        assert!(tracker.is_pressed(2));
    }
}
