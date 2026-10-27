//! The double-click every editor surface shares: a second left press on the same target
//! within [`DOUBLE_CLICK_INTERVAL`] and [`DOUBLE_CLICK_DISTANCE`] of the first, with no
//! drag started between them. The `ui` crate knows nothing of it — each surface keeps
//! its own last press and says when a drag started.

use std::time::Duration;

use common::clock::Instant;
use glam::Vec2;

/// The longest gap between the two presses of a double-click.
pub const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(400);
/// The farthest the second press may land from the first, in pixels.
pub const DOUBLE_CLICK_DISTANCE: f32 = 4.0;

/// The last press on one surface, waiting to become a double-click.
#[derive(Debug, Clone)]
pub struct DoubleClick<Target> {
    last: Option<LastPress<Target>>,
}

#[derive(Debug, Clone)]
struct LastPress<Target> {
    target: Target,
    at: Instant,
    position: Vec2,
}

impl<Target> Default for DoubleClick<Target> {
    fn default() -> Self {
        Self { last: None }
    }
}

impl<Target: PartialEq> DoubleClick<Target> {
    /// Record a left press on `target`; true when it completes a double-click. A completed
    /// double-click is forgotten, so a triple press is one double-click and a new first press.
    pub fn press(&mut self, target: Target, at: Instant, position: Vec2) -> bool {
        let completes = self.last.as_ref().is_some_and(|last| {
            last.target == target
                && at.saturating_duration_since(last.at) <= DOUBLE_CLICK_INTERVAL
                && last.position.distance(position) <= DOUBLE_CLICK_DISTANCE
        });
        self.last = (!completes).then_some(LastPress { target, at, position });
        completes
    }

    /// Forget the last press: a drag started since, and a drag's end is no first click.
    pub fn forget(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: Vec2 = Vec2::new(100.0, 100.0);

    fn after(start: Instant, milliseconds: u64) -> Instant {
        start + Duration::from_millis(milliseconds)
    }

    #[test]
    fn test_a_second_press_inside_both_bounds_is_a_double_click() {
        let start = Instant::now();
        let mut presses = DoubleClick::default();
        assert!(!presses.press("hero.png", start, AT), "a first press is only a press");
        assert!(presses.press("hero.png", after(start, 400), AT + Vec2::new(4.0, 0.0)));
    }

    #[test]
    fn test_a_press_too_late_too_far_or_on_another_target_starts_afresh() {
        let start = Instant::now();
        let mut late = DoubleClick::default();
        late.press("hero.png", start, AT);
        assert!(!late.press("hero.png", after(start, 401), AT), "401 ms is two clicks");

        let mut far = DoubleClick::default();
        far.press("hero.png", start, AT);
        assert!(!far.press("hero.png", after(start, 100), AT + Vec2::new(5.0, 0.0)), "5 px is two clicks");

        let mut other = DoubleClick::default();
        other.press("hero.png", start, AT);
        assert!(!other.press("enemy.png", after(start, 100), AT), "another target is two clicks");
        assert!(other.press("enemy.png", after(start, 200), AT), "the second press became the new first");
    }

    #[test]
    fn test_a_drag_between_the_presses_forgets_the_first() {
        let start = Instant::now();
        let mut presses = DoubleClick::default();
        presses.press("hero.png", start, AT);
        presses.forget();
        assert!(!presses.press("hero.png", after(start, 100), AT));
    }

    #[test]
    fn test_a_triple_press_is_one_double_click() {
        let start = Instant::now();
        let mut presses = DoubleClick::default();
        let completed: Vec<bool> =
            (0..3).map(|press| presses.press("hero.png", after(start, press * 100), AT)).collect();
        assert_eq!(completed, vec![false, true, false]);
    }
}
