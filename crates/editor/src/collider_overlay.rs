//! Collider outline overlay for the scene view.
//!
//! Draws the physics shapes over the rendered sprites so mismatches between
//! visuals and colliders are visible at a glance. The geometry mirrors how
//! `PhysicsWorld` places colliders: the entity `Transform2D` provides the
//! world position and rotation, the collider `offset` is body-local (it
//! rotates with the body), and `Transform2D.scale` is ignored — physics
//! collider sizes are absolute pixels.

use common::Transform2D;
use ecs::World;
use glam::Vec2;
use physics::components::{Collider, ColliderShape};
use ui::{Color, Rect, UIContext};

use crate::selection::Selection;
use crate::viewport::SceneViewport;

/// Number of segments used to approximate a full circle outline.
const CIRCLE_SEGMENTS: usize = 32;
/// Number of segments used to approximate each capsule end cap (semicircle).
const CAP_SEGMENTS: usize = 12;
/// Outline width for unselected colliders, in screen pixels.
const OUTLINE_WIDTH: f32 = 1.5;
/// Outline width for the primary selected entity's collider, in screen pixels.
const OUTLINE_WIDTH_SELECTED: f32 = 2.5;
/// Outline width for every other selected entity's collider — between
/// the primary and the unselected, the same three-way split the viewport
/// selection outline draws.
const OUTLINE_WIDTH_SECONDARY: f32 = 2.0;

/// Outline colors for the collider overlay.
///
/// Normally sourced from the theme via `EditorTheme::collider_overlay_colors()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderOverlayColors {
    /// Solid (non-sensor) colliders
    pub solid: Color,
    /// Sensor colliders (trigger-only)
    pub sensor: Color,
    /// Colliders on selected entities
    pub selected: Color,
}

impl ColliderOverlayColors {
    /// Pick the outline color for a collider. Selection wins over sensor.
    pub fn color_for(&self, collider: &Collider, is_selected: bool) -> Color {
        if is_selected {
            self.selected
        } else if collider.is_sensor {
            self.sensor
        } else {
            self.solid
        }
    }
}

/// Rotate a vector by an angle in radians (counter-clockwise).
fn rotate_vec(v: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

/// Unit vector pointing along `angle` (radians from +X).
fn dir(angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin())
}

/// Evenly spaced points along an arc, inclusive of both endpoints.
fn arc_points(center: Vec2, radius: f32, start_angle: f32, end_angle: f32, segments: usize) -> Vec<Vec2> {
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            center + dir(angle) * radius
        })
        .collect()
}

/// Append the line segments connecting consecutive points.
fn polyline_segments(points: &[Vec2], out: &mut Vec<(Vec2, Vec2)>) {
    for pair in points.windows(2) {
        out.push((pair[0], pair[1]));
    }
}

/// Outline for a capsule whose long axis points along `axis_angle`
/// (radians from +X): two outward-facing end-cap arcs plus the two
/// straight sides connecting them.
fn capsule_segments(center: Vec2, half_height: f32, radius: f32, axis_angle: f32) -> Vec<(Vec2, Vec2)> {
    use std::f32::consts::{FRAC_PI_2, PI};

    let axis = dir(axis_angle);
    let top = center + axis * half_height;
    let bottom = center - axis * half_height;
    let side = dir(axis_angle + FRAC_PI_2) * radius;

    let mut segments = Vec::with_capacity(2 * CAP_SEGMENTS + 2);
    let top_arc = arc_points(top, radius, axis_angle - FRAC_PI_2, axis_angle + FRAC_PI_2, CAP_SEGMENTS);
    let bottom_arc = arc_points(bottom, radius, axis_angle + FRAC_PI_2, axis_angle + FRAC_PI_2 + PI, CAP_SEGMENTS);
    polyline_segments(&top_arc, &mut segments);
    polyline_segments(&bottom_arc, &mut segments);
    segments.push((top + side, bottom + side));
    segments.push((top - side, bottom - side));
    segments
}

/// World-space outline segments for a collider attached to `transform`.
///
/// Matches the physics simulation exactly: the offset rotates with the body
/// and `transform.scale` plays no part (rapier colliders are unscaled).
pub fn collider_outline_segments(transform: &Transform2D, collider: &Collider) -> Vec<(Vec2, Vec2)> {
    let rotation = transform.rotation;
    let center = transform.position + rotate_vec(collider.offset, rotation);

    match &collider.shape {
        ColliderShape::Box { half_extents } => {
            let corners = [
                Vec2::new(-half_extents.x, -half_extents.y),
                Vec2::new(half_extents.x, -half_extents.y),
                Vec2::new(half_extents.x, half_extents.y),
                Vec2::new(-half_extents.x, half_extents.y),
            ];
            let world: Vec<Vec2> = corners.iter().map(|c| center + rotate_vec(*c, rotation)).collect();
            (0..4).map(|i| (world[i], world[(i + 1) % 4])).collect()
        }
        ColliderShape::Circle { radius } => {
            let points = arc_points(center, *radius, 0.0, std::f32::consts::TAU, CIRCLE_SEGMENTS);
            let mut segments = Vec::with_capacity(CIRCLE_SEGMENTS);
            polyline_segments(&points, &mut segments);
            segments
        }
        ColliderShape::CapsuleY { half_height, radius } => {
            capsule_segments(center, *half_height, *radius, rotation + std::f32::consts::FRAC_PI_2)
        }
        ColliderShape::CapsuleX { half_height, radius } => {
            capsule_segments(center, *half_height, *radius, rotation)
        }
    }
}

/// Draw collider outlines for every entity that has both a `Transform2D`
/// and a `Collider`, clipped to the scene-view `bounds`.
pub fn render_collider_overlay(
    ui: &mut UIContext,
    world: &World,
    viewport: &SceneViewport,
    selection: &Selection,
    colors: &ColliderOverlayColors,
    bounds: Rect,
) {
    ui.push_clip_rect(bounds);
    for entity in world.entities() {
        let Some(transform) = world.get::<Transform2D>(entity) else { continue };
        let Some(collider) = world.get::<Collider>(entity) else { continue };

        let is_selected = selection.contains(entity);
        let color = colors.color_for(collider, is_selected);
        let width = if selection.primary() == Some(entity) {
            OUTLINE_WIDTH_SELECTED
        } else if is_selected {
            OUTLINE_WIDTH_SECONDARY
        } else {
            OUTLINE_WIDTH
        };

        crate::world_lines::draw_world_segments(
            ui,
            viewport,
            collider_outline_segments(transform, collider),
            color,
            width,
        );
    }
    ui.pop_clip_rect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_viewport;
    use std::f32::consts::FRAC_PI_2;
    use ui::DrawCommand;

    fn transform_at(pos: Vec2, rotation: f32) -> Transform2D {
        Transform2D::from_parts(pos, rotation, Vec2::ONE)
    }

    fn assert_vec2_near(actual: Vec2, expected: Vec2) {
        assert!((actual - expected).length() < 1e-4, "expected {expected:?}, got {actual:?}");
    }

    /// Farthest reach of any endpoint along +X and +Y.
    fn max_reach(segments: &[(Vec2, Vec2)]) -> Vec2 {
        segments
            .iter()
            .flat_map(|(a, b)| [*a, *b])
            .fold(Vec2::splat(f32::MIN), |acc, p| acc.max(p))
    }

    fn colors() -> ColliderOverlayColors {
        ColliderOverlayColors {
            solid: Color::new(0.0, 1.0, 0.0, 1.0),
            sensor: Color::new(0.0, 1.0, 1.0, 1.0),
            selected: Color::new(1.0, 1.0, 0.0, 1.0),
        }
    }

    #[test]
    fn test_collider_offset_and_box_corners_rotate_with_the_body() {
        // Offset (10, 0) on a body rotated 90° CCW lands at (0, 10) —
        // rapier's body-local collider placement.
        let transform = transform_at(Vec2::new(5.0, 5.0), FRAC_PI_2);
        let circle = Collider::circle_collider(1.0).with_offset(Vec2::new(10.0, 0.0));

        let segments = collider_outline_segments(&transform, &circle);

        assert_eq!(segments.len(), CIRCLE_SEGMENTS);
        for (start, _) in &segments {
            let distance = (*start - Vec2::new(5.0, 15.0)).length();
            assert!((distance - 1.0).abs() < 1e-3, "point {start} is off the rotated circle");
        }

        // A 40×20 box rotated 90° puts its bottom-left corner (-20,-10) at
        // (10,-20); the outline stays a closed loop.
        let boxed = Collider::box_collider(40.0, 20.0);
        let segments = collider_outline_segments(&transform_at(Vec2::ZERO, FRAC_PI_2), &boxed);
        assert_eq!(segments.len(), 4);
        assert_vec2_near(segments[0].0, Vec2::new(10.0, -20.0));
        assert_vec2_near(segments[3].1, segments[0].0);
    }

    #[test]
    fn test_capsule_reaches_half_height_plus_radius_along_its_axis() {
        // Total height 120, cap radius 10 → half_height 50, full reach 60.
        let along_y = Collider::new(ColliderShape::capsule_y(120.0, 10.0));
        let reach = max_reach(&collider_outline_segments(&transform_at(Vec2::ZERO, 0.0), &along_y));
        assert_vec2_near(reach, Vec2::new(10.0, 60.0));

        let along_x = Collider::new(ColliderShape::capsule_x(120.0, 10.0));
        let reach = max_reach(&collider_outline_segments(&transform_at(Vec2::ZERO, 0.0), &along_x));
        assert_vec2_near(reach, Vec2::new(60.0, 10.0));
    }

    #[test]
    fn test_transform_scale_is_ignored_like_physics() {
        let unscaled = transform_at(Vec2::ZERO, 0.0);
        let scaled = Transform2D::from_parts(Vec2::ZERO, 0.0, Vec2::new(5.0, 5.0));
        let collider = Collider::box_collider(40.0, 20.0);

        assert_eq!(
            collider_outline_segments(&scaled, &collider),
            collider_outline_segments(&unscaled, &collider),
            "collider sizes are absolute pixels: a sprite scaled up drifts from its collider, and the overlay must show that drift"
        );
    }

    #[test]
    fn test_overlay_draws_each_collider_at_its_screen_corners_in_its_state_color() {
        let mut world = World::new();
        let solid = world.create_entity();
        world.add_component(&solid, Transform2D::new(Vec2::new(100.0, 50.0))).ok();
        world.add_component(&solid, Collider::box_collider(40.0, 20.0)).ok();
        let sensor = world.create_entity();
        world.add_component(&sensor, Transform2D::new(Vec2::new(-200.0, 0.0))).ok();
        world.add_component(&sensor, Collider::box_collider(10.0, 10.0).as_sensor()).ok();
        let selected_sensor = world.create_entity();
        world.add_component(&selected_sensor, Transform2D::new(Vec2::new(-200.0, -100.0))).ok();
        world.add_component(&selected_sensor, Collider::box_collider(10.0, 10.0).as_sensor()).ok();
        let secondary_selected = world.create_entity();
        world.add_component(&secondary_selected, Transform2D::new(Vec2::new(200.0, -100.0))).ok();
        world.add_component(&secondary_selected, Collider::box_collider(10.0, 10.0)).ok();
        let bare = world.create_entity();
        world.add_component(&bare, Transform2D::new(Vec2::ZERO)).ok();
        let mut selection = Selection::new();
        selection.select(selected_sensor);
        selection.add(secondary_selected);
        let colors = colors();
        let mut ui = UIContext::new();

        render_collider_overlay(&mut ui, &world, &test_viewport(), &selection, &colors, Rect::new(0.0, 0.0, 800.0, 600.0));

        let lines: Vec<(Vec2, Vec2, Color, f32)> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Line { start, end, color, width, .. } => Some((*start, *end, *color, *width)),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 16, "four boxes of four edges; the collider-less entity draws nothing");

        // The solid box's world corners (80,40)…(120,60) land on screen
        // through the viewport mapping, Y flipped.
        let solid_lines: Vec<_> = lines.iter().filter(|line| line.2 == colors.solid).collect();
        let corners = [Vec2::new(480.0, 260.0), Vec2::new(520.0, 260.0), Vec2::new(520.0, 240.0), Vec2::new(480.0, 240.0)];
        assert_eq!(solid_lines.len(), 4);
        for (i, (start, end, _, width)) in solid_lines.iter().enumerate() {
            assert_eq!(*start, corners[i]);
            assert_eq!(*end, corners[(i + 1) % 4]);
            assert_eq!(*width, OUTLINE_WIDTH);
        }

        // Sensors draw in the sensor colour; selection wins over sensor, the
        // primary draws widest and other selected entities in between.
        assert_eq!(lines.iter().filter(|line| line.2 == colors.sensor && line.3 == OUTLINE_WIDTH).count(), 4);
        assert_eq!(lines.iter().filter(|line| line.2 == colors.selected && line.3 == OUTLINE_WIDTH_SELECTED).count(), 4);
        assert_eq!(lines.iter().filter(|line| line.2 == colors.selected && line.3 == OUTLINE_WIDTH_SECONDARY).count(), 4);
    }
}
