//! Selection and hover outlines for the scene view.
//!
//! Draws an axis-aligned rectangle around every selected entity (primary
//! selection brighter than the rest) and a translucent one under the cursor.
//! The outline consumes the same `PickableEntity` list viewport picking uses,
//! so it can never disagree with what a click would select — which also means
//! it is deliberately scoped to sprite-bearing entities: what the viewport
//! cannot pick (cameras, empties, UI elements) it does not outline.

use std::cmp::Ordering;

use ecs::EntityId;
use glam::Vec2;
use ui::{Color, Rect, UIContext};

use crate::picking::{PickableEntity, AABB};
use crate::selection::Selection;
use crate::viewport::SceneViewport;

/// Outline width for the primary selection.
const PRIMARY_WIDTH: f32 = 2.5;
/// Outline width for secondary selections and the hover hint.
const SECONDARY_WIDTH: f32 = 1.5;
/// World-unit size below which an outline degenerates to a point — skip it.
const MIN_OUTLINE_SIZE: f32 = 1e-3;

/// Colors for the selection/hover outlines, derived from the editor theme
/// via `EditorTheme::selection_outline_colors()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionOutlineColors {
    /// Outline of the primary selected entity.
    pub primary: Color,
    /// Outline of every other selected entity.
    pub secondary: Color,
    /// Translucent outline of the entity under the cursor.
    pub hovered: Color,
}

impl SelectionOutlineColors {
    /// Color for a selected entity's outline.
    pub fn color_for(&self, is_primary: bool) -> Color {
        if is_primary {
            self.primary
        } else {
            self.secondary
        }
    }
}

/// World-space edges of an axis-aligned outline rectangle.
pub fn outline_segments(center: Vec2, size: Vec2) -> [(Vec2, Vec2); 4] {
    let half = size.abs() * 0.5;
    let bl = center - half;
    let tr = center + half;
    let br = Vec2::new(tr.x, bl.y);
    let tl = Vec2::new(bl.x, tr.y);
    [(bl, br), (br, tr), (tr, tl), (tl, bl)]
}

/// The entity the cursor is over: topmost by `(depth, entity_id)` — the same
/// front-to-back preference picking uses, with a deterministic tiebreak.
/// Deliberately margin- and cycle-free (hover is a hint, not a click), and
/// tolerant of negative (flip) scales via the absolute size. On same-depth
/// stacks the hint is best-effort: the picker cycles through the stack on
/// repeated clicks, which no static hint can predict.
pub fn hover_entity_at(
    screen_pos: Vec2,
    viewport: &SceneViewport,
    pickables: &[PickableEntity],
) -> Option<EntityId> {
    let world_pos = viewport.screen_to_world(screen_pos);
    pickables
        .iter()
        .filter(|p| visible_size(p).is_some())
        .filter(|p| AABB::from_position_size(p.position, p.size.abs()).contains_point(world_pos))
        .max_by(|a, b| {
            a.depth
                .total_cmp(&b.depth)
                .then(a.entity_id.value().cmp(&b.entity_id.value()))
        })
        .map(|p| p.entity_id)
}

/// Draw outlines for every selected pickable (back-to-front so overlapping
/// outlines layer deterministically) plus a translucent hover hint, clipped
/// to the scene-view `bounds`.
pub fn render_selection_outline(
    ui: &mut UIContext,
    viewport: &SceneViewport,
    selection: &Selection,
    hovered: Option<EntityId>,
    pickables: &[PickableEntity],
    colors: &SelectionOutlineColors,
    bounds: Rect,
) {
    ui.push_clip_rect(bounds);

    let mut selected: Vec<&PickableEntity> = pickables
        .iter()
        .filter(|p| selection.contains(p.entity_id))
        .collect();
    // Sort by depth so overlapping outlines layer correctly regardless of
    // selection order (front-most drawn last, ids break depth ties).
    selected.sort_by(|a, b| depth_then_id(a, b));

    for pickable in selected {
        let is_primary = selection.primary() == Some(pickable.entity_id);
        let width = if is_primary { PRIMARY_WIDTH } else { SECONDARY_WIDTH };
        draw_outline(ui, viewport, pickable, colors.color_for(is_primary), width);
    }

    // Hover hint — skipped when the entity is already outlined as selected.
    if let Some(hover_id) = hovered {
        if !selection.contains(hover_id) {
            if let Some(pickable) = pickables.iter().find(|p| p.entity_id == hover_id) {
                draw_outline(ui, viewport, pickable, colors.hovered, SECONDARY_WIDTH);
            }
        }
    }

    ui.pop_clip_rect();
}

/// Ascending draw order: lower depth (further back) first.
fn depth_then_id(a: &PickableEntity, b: &PickableEntity) -> Ordering {
    a.depth
        .total_cmp(&b.depth)
        .then(a.entity_id.value().cmp(&b.entity_id.value()))
}

/// The outline size for a pickable, or `None` when it degenerates to a point
/// (zero scale) — negative flip scales outline at their absolute size.
fn visible_size(pickable: &PickableEntity) -> Option<Vec2> {
    let size = pickable.size.abs();
    (size.x > MIN_OUTLINE_SIZE && size.y > MIN_OUTLINE_SIZE).then_some(size)
}

fn draw_outline(
    ui: &mut UIContext,
    viewport: &SceneViewport,
    pickable: &PickableEntity,
    color: Color,
    width: f32,
) {
    let Some(size) = visible_size(pickable) else { return };
    for (start, end) in outline_segments(pickable.position, size) {
        ui.line(
            viewport.world_to_screen(start),
            viewport.world_to_screen(end),
            color,
            width,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui::DrawCommand;

    fn viewport() -> SceneViewport {
        let mut v = SceneViewport::new();
        v.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));
        v
    }

    fn pickable(id: u64, pos: Vec2, size: Vec2, depth: f32) -> PickableEntity {
        PickableEntity::new(EntityId::with_generation(id, 1), pos, size, depth)
    }

    fn colors() -> SelectionOutlineColors {
        SelectionOutlineColors {
            primary: Color::new(1.0, 0.5, 0.1, 1.0),
            secondary: Color::new(0.7, 0.35, 0.07, 1.0),
            hovered: Color::new(1.0, 0.5, 0.1, 0.4),
        }
    }

    fn lines(ui: &UIContext) -> Vec<(Color, f32)> {
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Line { color, width, .. } => Some((*color, *width)),
                _ => None,
            })
            .collect()
    }

    fn bounds() -> Rect {
        Rect::new(0.0, 0.0, 800.0, 600.0)
    }

    #[test]
    fn test_outline_segments_trace_the_aabb_corners() {
        let segments = outline_segments(Vec2::new(10.0, 20.0), Vec2::new(4.0, 6.0));
        let corners = [
            Vec2::new(8.0, 17.0),
            Vec2::new(12.0, 17.0),
            Vec2::new(12.0, 23.0),
            Vec2::new(8.0, 23.0),
        ];
        for (i, (start, end)) in segments.iter().enumerate() {
            assert_eq!(*start, corners[i]);
            assert_eq!(*end, corners[(i + 1) % 4]);
        }
        // A negative (flip) scale outlines the same rectangle
        assert_eq!(
            outline_segments(Vec2::new(10.0, 20.0), Vec2::new(-4.0, 6.0)),
            segments
        );
    }

    #[test]
    fn test_render_outline_emits_four_lines_for_selected_sprite() {
        let mut ui = UIContext::new();
        let mut selection = Selection::new();
        let p = pickable(1, Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0);
        selection.select(p.entity_id);

        render_selection_outline(&mut ui, &viewport(), &selection, None, &[p], &colors(), bounds());
        assert_eq!(lines(&ui).len(), 4);
    }

    #[test]
    fn test_render_outline_skips_unselected_entities() {
        let mut ui = UIContext::new();
        let p = pickable(1, Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0);

        render_selection_outline(
            &mut ui, &viewport(), &Selection::new(), None, &[p], &colors(), bounds(),
        );
        assert!(lines(&ui).is_empty());
    }

    #[test]
    fn test_selected_and_hovered_entity_outlined_once_not_twice() {
        let mut ui = UIContext::new();
        let mut selection = Selection::new();
        let p = pickable(1, Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0);
        selection.select(p.entity_id);

        render_selection_outline(
            &mut ui, &viewport(), &selection, Some(p.entity_id), &[p], &colors(), bounds(),
        );
        assert_eq!(lines(&ui).len(), 4, "hover must not double-outline a selected entity");
    }

    #[test]
    fn test_hovered_only_entity_gets_translucent_outline() {
        let mut ui = UIContext::new();
        let p = pickable(1, Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0);

        render_selection_outline(
            &mut ui, &viewport(), &Selection::new(), Some(p.entity_id), &[p], &colors(), bounds(),
        );
        let drawn = lines(&ui);
        assert_eq!(drawn.len(), 4);
        for (color, _) in drawn {
            assert_eq!(color, colors().hovered);
        }
    }

    #[test]
    fn test_primary_outline_brighter_and_wider_than_secondary() {
        let c = colors();
        assert_eq!(c.color_for(true), c.primary);
        assert_eq!(c.color_for(false), c.secondary);

        let mut ui = UIContext::new();
        let mut selection = Selection::new();
        let a = pickable(1, Vec2::new(-100.0, 0.0), Vec2::new(50.0, 50.0), 0.0);
        let b = pickable(2, Vec2::new(100.0, 0.0), Vec2::new(50.0, 50.0), 0.0);
        selection.select(a.entity_id);
        selection.add(b.entity_id);

        render_selection_outline(
            &mut ui, &viewport(), &selection, None, &[a, b], &c, bounds(),
        );
        let drawn = lines(&ui);
        assert_eq!(drawn.len(), 8);
        assert_eq!(drawn.iter().filter(|(col, w)| *col == c.primary && *w > 2.0).count(), 4);
        assert_eq!(drawn.iter().filter(|(col, w)| *col == c.secondary && *w < 2.0).count(), 4);
    }

    #[test]
    fn test_overlapping_selection_outlines_draw_back_to_front() {
        let mut ui = UIContext::new();
        let mut selection = Selection::new();
        // Primary is the BACK entity — the front one must still draw last.
        let back = pickable(1, Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0);
        let front = pickable(2, Vec2::ZERO, Vec2::new(50.0, 50.0), 5.0);
        selection.select(back.entity_id);
        selection.add(front.entity_id);

        render_selection_outline(
            &mut ui, &viewport(), &selection, None, &[front.clone(), back], &colors(), bounds(),
        );
        let drawn = lines(&ui);
        assert_eq!(drawn.len(), 8);
        for (color, _) in &drawn[..4] {
            assert_eq!(*color, colors().primary, "back (primary) entity draws first");
        }
        for (color, _) in &drawn[4..] {
            assert_eq!(*color, colors().secondary, "front entity draws last, on top");
        }
    }

    #[test]
    fn test_hover_picks_topmost_by_depth_with_stable_tiebreak() {
        let vp = viewport();
        let stack = [
            pickable(1, Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0),
            pickable(2, Vec2::ZERO, Vec2::new(50.0, 50.0), 10.0),
            pickable(3, Vec2::ZERO, Vec2::new(50.0, 50.0), 5.0),
        ];
        let center = vp.world_to_screen(Vec2::ZERO);
        assert_eq!(
            hover_entity_at(center, &vp, &stack),
            Some(EntityId::with_generation(2, 1))
        );

        // Equal depths: the higher entity id wins, deterministically
        let tied = [
            pickable(7, Vec2::ZERO, Vec2::new(50.0, 50.0), 1.0),
            pickable(4, Vec2::ZERO, Vec2::new(50.0, 50.0), 1.0),
        ];
        assert_eq!(
            hover_entity_at(center, &vp, &tied),
            Some(EntityId::with_generation(7, 1))
        );

        // A miss hovers nothing
        let far = vp.world_to_screen(Vec2::new(500.0, 500.0));
        assert_eq!(hover_entity_at(far, &vp, &stack), None);
    }

    #[test]
    fn test_zero_size_pickable_never_outlines_or_hovers() {
        let vp = viewport();
        let mut ui = UIContext::new();
        let mut selection = Selection::new();
        let p = pickable(1, Vec2::ZERO, Vec2::ZERO, 0.0);
        selection.select(p.entity_id);

        render_selection_outline(
            &mut ui, &vp, &selection, Some(p.entity_id), std::slice::from_ref(&p), &colors(), bounds(),
        );
        assert!(lines(&ui).is_empty(), "a degenerate outline is skipped, not drawn as a dot");
        assert_eq!(hover_entity_at(vp.world_to_screen(Vec2::ZERO), &vp, &[p]), None);
    }

    #[test]
    fn test_negative_flip_scale_outlines_and_hovers_at_absolute_size() {
        let vp = viewport();
        let mut ui = UIContext::new();
        let mut selection = Selection::new();
        let p = pickable(1, Vec2::ZERO, Vec2::new(-50.0, 50.0), 0.0);
        selection.select(p.entity_id);

        render_selection_outline(
            &mut ui, &vp, &selection, None, std::slice::from_ref(&p), &colors(), bounds(),
        );
        assert_eq!(lines(&ui).len(), 4);
        assert_eq!(
            hover_entity_at(vp.world_to_screen(Vec2::new(10.0, 10.0)), &vp, &[p]),
            Some(EntityId::with_generation(1, 1))
        );
    }
}
