//! Time resource for tracking game timing.
//!
//! This module provides a simple time resource for tracking delta time
//! and elapsed time in games.

/// Time resource for tracking delta time and elapsed time.
///
/// This struct is designed to be used as a resource in game loops
/// to track frame timing information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Time {
    /// Delta time in seconds since last frame
    pub delta_seconds: f32,
    /// Total elapsed time in seconds since game started
    pub elapsed_seconds: f32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta_seconds: 0.0,
            elapsed_seconds: 0.0,
        }
    }
}

impl Time {
    /// Create a new Time with the given delta and elapsed times.
    pub fn new(delta_seconds: f32, elapsed_seconds: f32) -> Self {
        Self {
            delta_seconds,
            elapsed_seconds,
        }
    }

    /// Create a Time with just delta time (elapsed starts at 0).
    pub fn with_delta(delta_seconds: f32) -> Self {
        Self {
            delta_seconds,
            elapsed_seconds: 0.0,
        }
    }

    /// Update the time by adding delta to elapsed.
    pub fn tick(&mut self, delta_seconds: f32) {
        self.delta_seconds = delta_seconds;
        self.elapsed_seconds += delta_seconds;
    }
}
