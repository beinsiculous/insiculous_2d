//! Pure math helpers for gizmo interactions.
//!
//! Kept separate from `gizmo.rs` so the coordinate conventions (screen Y grows
//! downward, world rotation is CCW-positive) are testable without a UI context.

use glam::Vec2;

/// World-space rotation delta for a mouse drag around `center`, in radians.
///
/// Screen Y grows downward while world rotation is CCW-positive, so the angle
/// is measured with a flipped Y. The result is wrapped to the shortest arc in
/// `(-PI, PI]` — without the wrap, a drag crossing the atan2 seam at ±PI would
/// produce a spurious ~2π jump.
pub fn world_rotation_delta(center: Vec2, last_mouse: Vec2, current_mouse: Vec2) -> f32 {
    let angle_of = |p: Vec2| (center.y - p.y).atan2(p.x - center.x);
    wrap_angle(angle_of(current_mouse) - angle_of(last_mouse))
}

/// Wrap an angle to the shortest arc in `(-PI, PI]`.
fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let wrapped = angle.rem_euclid(TAU);
    if wrapped > PI { wrapped - TAU } else { wrapped }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPS: f32 = 1e-5;

    #[test]
    fn test_rotation_delta_is_ccw_positive_under_screen_y_down() {
        let center = Vec2::new(100.0, 100.0);
        let right = center + Vec2::new(50.0, 0.0);
        let table = [
            (right, center + Vec2::new(0.0, -50.0), PI / 2.0, "screen-up on the right side is world CCW"),
            (right, center + Vec2::new(0.0, 50.0), -PI / 2.0, "screen-down on the right side is world CW"),
            (right, right, 0.0, "no movement is no rotation"),
            (right, center + Vec2::new(-50.0, 0.0), PI, "a half turn is +PI, the top of the (-PI, PI] range"),
        ];
        for (from, to, expected, why) in table {
            let delta = world_rotation_delta(center, from, to);
            assert!((delta - expected).abs() < EPS, "{why}: got {delta}, expected {expected}");
        }
    }

    #[test]
    fn test_seam_crossing_returns_a_small_delta_not_a_full_turn() {
        // Both points hug the -X axis (world angle ~±PI) on either side:
        // naive subtraction yields ~2π; the wrap keeps the shortest arc.
        let just_above = Vec2::new(-50.0, 1.0);
        let just_below = Vec2::new(-50.0, -1.0);

        let delta = world_rotation_delta(Vec2::ZERO, just_above, just_below);

        assert!(delta.abs() < 0.1, "expected the short way round, got {delta}");
        assert!(
            (world_rotation_delta(Vec2::ZERO, just_below, just_above) + delta).abs() < EPS,
            "crossing back is the opposite short arc"
        );
    }
}
