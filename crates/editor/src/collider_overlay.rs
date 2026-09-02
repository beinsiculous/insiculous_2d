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
/// Outline width for every other selected entity's collider (#51) — between
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

        for (start, end) in collider_outline_segments(transform, collider) {
            ui.line(
                viewport.world_to_screen(start),
                viewport.world_to_screen(end),
                color,
                width,
            );
        }
    }
    ui.pop_clip_rect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    fn transform_at(pos: Vec2, rotation: f32) -> Transform2D {
        Transform2D::from_parts(pos, rotation, Vec2::ONE)
    }

    fn assert_vec2_near(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() < 1e-4,
            "expected {expected:?}, got {actual:?}"
        );
    }

    /// Max distance of any segment endpoint from a point.
    fn max_extent_from(segments: &[(Vec2, Vec2)], origin: Vec2) -> f32 {
        segments
            .iter()
            .flat_map(|(a, b)| [a, b])
            .map(|p| (*p - origin).length())
            .fold(0.0, f32::max)
    }

    #[test]
    fn test_box_outline_has_four_segments_at_corners() {
        let transform = transform_at(Vec2::new(100.0, 50.0), 0.0);
        let collider = Collider::box_collider(40.0, 20.0);

        let segments = collider_outline_segments(&transform, &collider);

        assert_eq!(segments.len(), 4);
        // First segment starts at the bottom-left corner.
        assert_vec2_near(segments[0].0, Vec2::new(80.0, 40.0));
        // Loop is closed: last segment ends where the first starts.
        assert_vec2_near(segments[3].1, segments[0].0);
    }

    #[test]
    fn test_box_outline_rotates_with_transform() {
        let transform = transform_at(Vec2::ZERO, FRAC_PI_2);
        let collider = Collider::box_collider(40.0, 20.0);

        let segments = collider_outline_segments(&transform, &collider);

        // Rotating 90° swaps the box's width and height: the corner that was
        // at (-20, -10) moves to (10, -20).
        assert_vec2_near(segments[0].0, Vec2::new(10.0, -20.0));
    }

    #[test]
    fn test_collider_offset_rotates_with_body() {
        // Offset (10, 0) on a body rotated 90° CCW lands at (0, 10),
        // mirroring rapier's body-local collider placement.
        let transform = transform_at(Vec2::new(5.0, 5.0), FRAC_PI_2);
        let collider = Collider::circle_collider(1.0).with_offset(Vec2::new(10.0, 0.0));

        let segments = collider_outline_segments(&transform, &collider);

        let expected_center = Vec2::new(5.0, 15.0);
        for (start, _) in &segments {
            assert!(((*start - expected_center).length() - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn test_circle_outline_points_lie_on_radius() {
        let center = Vec2::new(-30.0, 12.0);
        let transform = transform_at(center, 0.0);
        let collider = Collider::circle_collider(25.0);

        let segments = collider_outline_segments(&transform, &collider);

        assert_eq!(segments.len(), CIRCLE_SEGMENTS);
        for (start, end) in &segments {
            assert!(((*start - center).length() - 25.0).abs() < 1e-3);
            assert!(((*end - center).length() - 25.0).abs() < 1e-3);
        }
    }

    #[test]
    fn test_capsule_y_extends_half_height_plus_radius_vertically() {
        let transform = transform_at(Vec2::ZERO, 0.0);
        // Total height 120, cap radius 10 → half_height 50, full reach 60.
        let collider = Collider::new(ColliderShape::capsule_y(120.0, 10.0));

        let segments = collider_outline_segments(&transform, &collider);

        let max_y = segments
            .iter()
            .flat_map(|(a, b)| [a.y, b.y])
            .fold(f32::MIN, f32::max);
        let max_x = segments
            .iter()
            .flat_map(|(a, b)| [a.x, b.x])
            .fold(f32::MIN, f32::max);
        assert!((max_y - 60.0).abs() < 1e-3);
        assert!((max_x - 10.0).abs() < 1e-3);
    }

    #[test]
    fn test_capsule_x_extends_along_x() {
        let transform = transform_at(Vec2::ZERO, 0.0);
        let collider = Collider::new(ColliderShape::capsule_x(120.0, 10.0));

        let segments = collider_outline_segments(&transform, &collider);

        let max_x = segments
            .iter()
            .flat_map(|(a, b)| [a.x, b.x])
            .fold(f32::MIN, f32::max);
        assert!((max_x - 60.0).abs() < 1e-3);
    }

    #[test]
    fn test_transform_scale_is_ignored_like_physics() {
        let unscaled = transform_at(Vec2::ZERO, 0.0);
        let scaled = Transform2D::from_parts(Vec2::ZERO, 0.0, Vec2::new(5.0, 5.0));
        let collider = Collider::box_collider(40.0, 20.0);

        let a = collider_outline_segments(&unscaled, &collider);
        let b = collider_outline_segments(&scaled, &collider);

        assert_eq!(a.len(), b.len());
        let extent_a = max_extent_from(&a, Vec2::ZERO);
        let extent_b = max_extent_from(&b, Vec2::ZERO);
        assert!((extent_a - extent_b).abs() < 1e-5);
    }

    #[test]
    fn test_overlay_color_selection_wins_over_sensor() {
        let colors = ColliderOverlayColors {
            solid: Color::new(0.0, 1.0, 0.0, 1.0),
            sensor: Color::new(0.0, 1.0, 1.0, 1.0),
            selected: Color::new(1.0, 1.0, 0.0, 1.0),
        };
        let solid = Collider::box_collider(10.0, 10.0);
        let sensor = Collider::box_collider(10.0, 10.0).as_sensor();

        assert_eq!(colors.color_for(&solid, false), colors.solid);
        assert_eq!(colors.color_for(&sensor, false), colors.sensor);
        assert_eq!(colors.color_for(&sensor, true), colors.selected);
        assert_eq!(colors.color_for(&solid, true), colors.selected);
    }

    #[test]
    fn test_render_overlay_emits_line_commands_for_collider_entities() {
        let mut world = World::new();
        let entity = world.create_entity();
        world
            .add_component(&entity, Transform2D::new(Vec2::ZERO))
            .ok();
        world
            .add_component(&entity, Collider::box_collider(32.0, 32.0))
            .ok();

        let mut ui = UIContext::new();
        let mut viewport = SceneViewport::new();
        viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));
        let selection = Selection::new();
        let colors = ColliderOverlayColors {
            solid: Color::new(0.0, 1.0, 0.0, 1.0),
            sensor: Color::new(0.0, 1.0, 1.0, 1.0),
            selected: Color::new(1.0, 1.0, 0.0, 1.0),
        };

        render_collider_overlay(
            &mut ui,
            &world,
            &viewport,
            &selection,
            &colors,
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );

        let lines = ui
            .draw_list()
            .commands()
            .iter()
            .filter(|c| matches!(c, ui::DrawCommand::Line { .. }))
            .count();
        // A box collider draws exactly its 4 edges.
        assert_eq!(lines, 4);
    }

    #[test]
    fn test_primary_collider_outline_is_wider_than_secondary_selected() {
        let mut world = World::new();
        let spawn = |world: &mut World| {
            let entity = world.create_entity();
            world.add_component(&entity, Transform2D::new(Vec2::ZERO)).ok();
            world.add_component(&entity, Collider::box_collider(32.0, 32.0)).ok();
            entity
        };
        let primary = spawn(&mut world);
        let secondary = spawn(&mut world);
        let mut selection = Selection::new();
        selection.select(primary);
        selection.add(secondary);

        let mut ui = UIContext::new();
        let mut viewport = SceneViewport::new();
        viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));
        let colors = ColliderOverlayColors {
            solid: Color::new(0.0, 1.0, 0.0, 1.0),
            sensor: Color::new(0.0, 1.0, 1.0, 1.0),
            selected: Color::new(1.0, 1.0, 0.0, 1.0),
        };
        render_collider_overlay(
            &mut ui,
            &world,
            &viewport,
            &selection,
            &colors,
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );

        let mut widths: Vec<f32> = ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|c| match c {
                ui::DrawCommand::Line { width, .. } => Some(*width),
                _ => None,
            })
            .collect();
        widths.sort_by(|a, b| a.partial_cmp(b).expect("finite widths"));
        widths.dedup();
        // Both selected (same color), but the primary reads wider.
        assert_eq!(widths, vec![OUTLINE_WIDTH_SECONDARY, OUTLINE_WIDTH_SELECTED]);
    }

    #[test]
    fn test_render_overlay_skips_entities_without_collider() {
        let mut world = World::new();
        let entity = world.create_entity();
        world
            .add_component(&entity, Transform2D::new(Vec2::ZERO))
            .ok();

        let mut ui = UIContext::new();
        let viewport = SceneViewport::new();
        let colors = ColliderOverlayColors {
            solid: Color::new(0.0, 1.0, 0.0, 1.0),
            sensor: Color::new(0.0, 1.0, 1.0, 1.0),
            selected: Color::new(1.0, 1.0, 0.0, 1.0),
        };

        render_collider_overlay(
            &mut ui,
            &world,
            &viewport,
            &Selection::new(),
            &colors,
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );

        let lines = ui
            .draw_list()
            .commands()
            .iter()
            .filter(|c| matches!(c, ui::DrawCommand::Line { .. }))
            .count();
        assert_eq!(lines, 0);
    }
}
