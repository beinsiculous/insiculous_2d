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
    /// Buttons held at the end of the previous frame
    previous: HashSet<T>,
    /// Buttons that transitioned to pressed this frame, in chronological order
    just_pressed: Vec<T>,
    /// Buttons that transitioned to released this frame
    just_released: HashSet<T>,
}

impl<T: Copy + Eq + Hash> Default for ButtonTracker<T> {
    fn default() -> Self {
        Self {
            pressed: HashSet::new(),
            previous: HashSet::new(),
            just_pressed: Vec::new(),
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
            self.just_pressed.push(button);
        }
    }

    /// Record a release event. A release of a button that is not held records
    /// no edge (focus-loss or synthetic releases must not fake a `just_released`).
    pub fn release(&mut self, button: T) {
        if self.pressed.remove(&button) {
            self.just_released.insert(button);
        }
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

    /// Check if a button was held as of the end of the previous frame
    pub fn was_pressed(&self, button: T) -> bool {
        self.previous.contains(&button)
    }

    /// Buttons that transitioned to pressed this frame, in chronological order
    pub fn just_pressed_buttons(&self) -> &[T] {
        &self.just_pressed
    }

    /// Clear the one-shot just-pressed / just-released sets for the next frame;
    /// also snapshots the held set into `previous` (what `was_pressed` reads).
    pub fn clear_frame_state(&mut self) {
        self.previous.clone_from(&self.pressed);
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
        assert!(tracker.was_pressed(1));
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
        assert!(!tracker.was_pressed(1));
        assert!(!tracker.is_just_released(1));
        assert!(tracker.is_pressed(2));
        assert!(tracker.was_pressed(2));
    }

    #[test]
    fn test_release_without_prior_press_records_no_release_edge() {
        let mut tracker = ButtonTracker::new();

        // Synthetic release without preceding press
        tracker.release(42u32);

        assert!(!tracker.is_pressed(42));
        assert!(!tracker.is_just_pressed(42));
        assert!(!tracker.is_just_released(42));
        assert!(!tracker.was_pressed(42));
    }

    #[test]
    fn test_press_and_release_within_one_frame_records_both_edges_and_no_residual_hold() {
        let mut tracker = ButtonTracker::new();

        tracker.press(10u32);
        tracker.release(10u32);

        assert!(!tracker.is_pressed(10));
        assert!(tracker.is_just_pressed(10));
        assert!(tracker.is_just_released(10));
        assert!(!tracker.was_pressed(10));

        tracker.clear_frame_state();
        assert!(!tracker.is_pressed(10));
        assert!(!tracker.is_just_pressed(10));
        assert!(!tracker.is_just_released(10));
        assert!(!tracker.was_pressed(10));
    }

    #[test]
    fn test_just_pressed_buttons_preserves_chronological_press_order() {
        let mut tracker = ButtonTracker::new();

        tracker.press(7u32);
        tracker.press(2u32);
        tracker.press(9u32);

        assert_eq!(tracker.just_pressed_buttons(), &[7, 2, 9]);
    }
}
