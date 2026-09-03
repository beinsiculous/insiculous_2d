//! Debug-draw helpers. Currently only collider outlines.
//!
//! All helpers push [`LineVertex`] pairs into the buffer the game already
//! owns (`ctx.lines`), so the engine's line render pipeline picks them up
//! automatically with no extra plumbing.

use glam::{Vec2, Vec4};
use renderer::line_pipeline::LineVertex;

#[cfg(feature = "physics")]
use ecs::World;
#[cfg(feature = "physics")]
use common::Transform2D;
#[cfg(feature = "physics")]
use physics::{Collider, ColliderShape};

/// How many segments to use when approximating a circle / capsule cap with
/// straight line pieces. 24 keeps the silhouette smooth at gameplay scale
/// without flooding the line buffer.
const CIRCLE_SEGMENTS: u32 = 24;

/// Append a single line segment.
fn push_segment(lines: &mut Vec<LineVertex>, a: Vec2, b: Vec2, color: Vec4, emissive: f32) {
    lines.push(LineVertex::new(a, color, emissive));
    lines.push(LineVertex::new(b, color, emissive));
}

/// Draw an axis-aligned rectangle outline.
///
/// `half_extents` are the half-width and half-height. `center` is where the
/// rectangle is anchored.
pub fn push_box_outline(
    lines: &mut Vec<LineVertex>,
    center: Vec2,
    half_extents: Vec2,
    color: Vec4,
    emissive: f32,
) {
    let tl = center + Vec2::new(-half_extents.x,  half_extents.y);
    let tr = center + Vec2::new( half_extents.x,  half_extents.y);
    let br = center + Vec2::new( half_extents.x, -half_extents.y);
    let bl = center + Vec2::new(-half_extents.x, -half_extents.y);
    push_segment(lines, tl, tr, color, emissive);
    push_segment(lines, tr, br, color, emissive);
    push_segment(lines, br, bl, color, emissive);
    push_segment(lines, bl, tl, color, emissive);
}

/// Draw a circle outline approximated by [`CIRCLE_SEGMENTS`] line segments.
pub fn push_circle_outline(
    lines: &mut Vec<LineVertex>,
    center: Vec2,
    radius: f32,
    color: Vec4,
    emissive: f32,
) {
    push_arc(lines, center, radius, 0.0, std::f32::consts::TAU, color, emissive);
}

/// Internal: walk an arc from `start_angle` to `start_angle + sweep`,
/// emitting line segments. Segment density is derived from the sweep so a
/// full circle uses [`CIRCLE_SEGMENTS`] and partial arcs scale down
/// proportionally. Used by circle + capsule cap drawing.
fn push_arc(
    lines: &mut Vec<LineVertex>,
    center: Vec2,
    radius: f32,
    start_angle: f32,
    sweep: f32,
    color: Vec4,
    emissive: f32,
) {
    let segments =
        ((sweep.abs() / std::f32::consts::TAU * CIRCLE_SEGMENTS as f32).round() as u32).max(1);
    let step = sweep / segments as f32;
    let mut prev = center + Vec2::new(start_angle.cos(), start_angle.sin()) * radius;
    for i in 1..=segments {
        let angle = start_angle + step * i as f32;
        let next = center + Vec2::new(angle.cos(), angle.sin()) * radius;
        push_segment(lines, prev, next, color, emissive);
        prev = next;
    }
}

/// Draw a Y-axis capsule outline: two vertical sides + two semicircular caps.
///
/// `half_height` is the cylindrical middle's half-extent (the part between
/// the cap centers). `radius` is the cap radius. Total visual height is
/// `2 * (half_height + radius)`.
pub fn push_capsule_y_outline(
    lines: &mut Vec<LineVertex>,
    center: Vec2,
    half_height: f32,
    radius: f32,
    color: Vec4,
    emissive: f32,
) {
    let top_cap_center = center + Vec2::new(0.0, half_height);
    let bot_cap_center = center + Vec2::new(0.0, -half_height);
    // Two straight sides between the cap centers.
    push_segment(
        lines,
        top_cap_center + Vec2::new( radius, 0.0),
        bot_cap_center + Vec2::new( radius, 0.0),
        color, emissive,
    );
    push_segment(
        lines,
        top_cap_center + Vec2::new(-radius, 0.0),
        bot_cap_center + Vec2::new(-radius, 0.0),
        color, emissive,
    );
    // Top cap: arc from 0 (right side) sweeping +PI to the left side.
    push_arc(lines, top_cap_center, radius, 0.0, std::f32::consts::PI, color, emissive);
    // Bottom cap: arc from PI (left side) sweeping +PI back to the right side.
    push_arc(lines, bot_cap_center, radius, std::f32::consts::PI, std::f32::consts::PI, color, emissive);
}

/// Draw an X-axis capsule outline. Same as [`push_capsule_y_outline`] but
/// rotated 90°.
pub fn push_capsule_x_outline(
    lines: &mut Vec<LineVertex>,
    center: Vec2,
    half_width: f32,
    radius: f32,
    color: Vec4,
    emissive: f32,
) {
    let right_cap_center = center + Vec2::new(half_width, 0.0);
    let left_cap_center  = center + Vec2::new(-half_width, 0.0);
    push_segment(
        lines,
        right_cap_center + Vec2::new(0.0,  radius),
        left_cap_center  + Vec2::new(0.0,  radius),
        color, emissive,
    );
    push_segment(
        lines,
        right_cap_center + Vec2::new(0.0, -radius),
        left_cap_center  + Vec2::new(0.0, -radius),
        color, emissive,
    );
    push_arc(lines, right_cap_center, radius, -std::f32::consts::FRAC_PI_2, std::f32::consts::PI, color, emissive);
    push_arc(lines, left_cap_center,  radius,  std::f32::consts::FRAC_PI_2, std::f32::consts::PI, color, emissive);
}

/// Walk every entity with a [`Collider`] + `Transform2D` and push its outline
/// into `lines`. Sensors get the same outline shape — sensor-ness is a
/// behavior, not a different geometry.
///
/// `color` and `emissive` apply uniformly. Pick a high emissive value
/// (e.g. 2.0) if you want the outlines to bloom and read clearly over
/// game sprites.
#[cfg(feature = "physics")]
pub fn draw_colliders(
    world: &World,
    lines: &mut Vec<LineVertex>,
    color: Vec4,
    emissive: f32,
) {
    for entity in world.entities() {
        let Some(transform) = world.get::<Transform2D>(entity) else { continue };
        let Some(collider) = world.get::<Collider>(entity) else { continue };
        let center = transform.position + collider.offset;
        match collider.shape {
            ColliderShape::Box { half_extents } => {
                push_box_outline(lines, center, half_extents, color, emissive);
            }
            ColliderShape::Circle { radius } => {
                push_circle_outline(lines, center, radius, color, emissive);
            }
            ColliderShape::CapsuleY { half_height, radius } => {
                push_capsule_y_outline(lines, center, half_height, radius, color, emissive);
            }
            ColliderShape::CapsuleX { half_height, radius } => {
                // CapsuleX stores half_height for the cylindrical middle, but
                // that field is named oddly — it's actually the half-WIDTH of
                // the horizontal capsule. See physics/src/components.rs.
                push_capsule_x_outline(lines, center, half_height, radius, color, emissive);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positions(lines: &[LineVertex]) -> Vec<Vec2> {
        lines.iter().map(|v| Vec2::from_array(v.position)).collect()
    }

    fn assert_segment(lines: &[LineVertex], index: usize, a: Vec2, b: Vec2) {
        let pos = positions(lines);
        assert_eq!((pos[2 * index], pos[2 * index + 1]), (a, b), "segment {index}");
    }

    #[test]
    fn box_outline_walks_the_four_corners_from_center_and_half_extents() {
        let mut lines = Vec::new();
        push_box_outline(&mut lines, Vec2::new(100.0, 50.0), Vec2::new(10.0, 5.0), Vec4::ONE, 0.0);

        assert_eq!(lines.len(), 8, "four segments, two vertices each");
        let (tl, tr, br, bl) = (
            Vec2::new(90.0, 55.0),
            Vec2::new(110.0, 55.0),
            Vec2::new(110.0, 45.0),
            Vec2::new(90.0, 45.0),
        );
        assert_segment(&lines, 0, tl, tr);
        assert_segment(&lines, 1, tr, br);
        assert_segment(&lines, 2, br, bl);
        assert_segment(&lines, 3, bl, tl);
    }

    #[test]
    fn circle_outline_closes_a_ring_of_segments_on_the_radius() {
        let center = Vec2::new(100.0, 50.0);
        let mut lines = Vec::new();
        push_circle_outline(&mut lines, center, 25.0, Vec4::ONE, 0.0);

        assert_eq!(lines.len() as u32, CIRCLE_SEGMENTS * 2);
        let pos = positions(&lines);
        for vertex in &pos {
            assert!((vertex.distance(center) - 25.0).abs() < 0.01, "{vertex:?} not on radius 25");
        }
        // Closed: each segment starts where the previous ended, and the
        // last ends where the first began.
        for pair in pos.chunks(2).collect::<Vec<_>>().windows(2) {
            assert!((pair[0][1] - pair[1][0]).length() < 1e-4, "gap between segments");
        }
        assert!((pos[pos.len() - 1] - pos[0]).length() < 1e-4, "ring not closed");
    }

    #[test]
    fn capsule_y_outline_has_straight_sides_at_plus_minus_radius_and_round_caps() {
        let center = Vec2::new(20.0, -10.0);
        let (half_height, radius) = (50.0, 10.0);
        let mut lines = Vec::new();
        push_capsule_y_outline(&mut lines, center, half_height, radius, Vec4::ONE, 0.0);

        // The two straight sides span the cylindrical middle at x = ±radius.
        assert_segment(&lines, 0, center + Vec2::new(10.0, 50.0), center + Vec2::new(10.0, -50.0));
        assert_segment(&lines, 1, center + Vec2::new(-10.0, 50.0), center + Vec2::new(-10.0, -50.0));
        // Every cap vertex sits on its cap center's radius: the top cap
        // above the middle, the bottom cap below it.
        let top_cap = center + Vec2::new(0.0, half_height);
        let bottom_cap = center + Vec2::new(0.0, -half_height);
        let caps = &positions(&lines)[4..];
        assert!(!caps.is_empty());
        for vertex in caps {
            let cap = if vertex.y >= center.y { top_cap } else { bottom_cap };
            assert!((vertex.distance(cap) - radius).abs() < 0.01, "{vertex:?} off its cap");
        }
    }
}
