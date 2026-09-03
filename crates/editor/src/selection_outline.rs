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
    crate::world_lines::draw_world_segments(
        ui,
        viewport,
        outline_segments(pickable.position, size),
        color,
        width,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{entity, pickable, test_viewport, WINDOW};
    use ui::DrawCommand;

    const SIZE: Vec2 = Vec2::new(50.0, 50.0);

    fn colors() -> SelectionOutlineColors {
        SelectionOutlineColors {
            primary: Color::new(1.0, 0.5, 0.1, 1.0),
            secondary: Color::new(0.7, 0.35, 0.07, 1.0),
            hovered: Color::new(1.0, 0.5, 0.1, 0.4),
        }
    }

    fn bounds() -> Rect {
        Rect::new(0.0, 0.0, WINDOW.x, WINDOW.y)
    }

    fn lines(ui: &UIContext) -> Vec<(Vec2, Vec2, Color, f32)> {
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Line { start, end, color, width, .. } => Some((*start, *end, *color, *width)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_primary_outline_is_wider_and_brighter_and_traces_the_sprite_on_screen() {
        let viewport = test_viewport();
        let colors = colors();
        let mut selection = Selection::new();
        // The primary sits BEHIND the secondary at the same spot: outlines
        // draw back to front, so the primary's edges come first.
        let back = pickable(1, Vec2::new(-100.0, 0.0), SIZE, 0.0);
        let front = pickable(2, Vec2::new(-100.0, 0.0), SIZE, 5.0);
        let unselected = pickable(3, Vec2::new(100.0, 0.0), SIZE, 0.0);
        selection.select(back.entity_id);
        selection.add(front.entity_id);
        let mut ui = UIContext::new();

        render_selection_outline(&mut ui, &viewport, &selection, None, &[front, back, unselected], &colors, bounds());

        let drawn = lines(&ui);
        assert_eq!(drawn.len(), 8, "two selected outlines of four edges; the unselected entity draws none");
        // World corners (-125,-25)…(-75,25) land on screen with Y flipped.
        let corners = [Vec2::new(275.0, 325.0), Vec2::new(325.0, 325.0), Vec2::new(325.0, 275.0), Vec2::new(275.0, 275.0)];
        for (i, (start, end, color, width)) in drawn[..4].iter().enumerate() {
            assert_eq!(*start, corners[i]);
            assert_eq!(*end, corners[(i + 1) % 4]);
            assert_eq!(*color, colors.primary);
            assert_eq!(*width, PRIMARY_WIDTH);
        }
        for (_, _, color, width) in &drawn[4..] {
            assert_eq!(*color, colors.secondary);
            assert_eq!(*width, SECONDARY_WIDTH);
        }
    }

    #[test]
    fn test_hover_hint_is_translucent_and_never_doubles_a_selected_outline() {
        let viewport = test_viewport();
        let colors = colors();
        let sprite = pickable(1, Vec2::ZERO, SIZE, 0.0);

        let mut hovered_only = UIContext::new();
        render_selection_outline(
            &mut hovered_only, &viewport, &Selection::new(), Some(sprite.entity_id), std::slice::from_ref(&sprite), &colors, bounds(),
        );
        let drawn = lines(&hovered_only);
        assert_eq!(drawn.len(), 4);
        for (_, _, color, width) in drawn {
            assert_eq!(color, colors.hovered, "an unselected hover draws the translucent hint");
            assert_eq!(width, SECONDARY_WIDTH);
        }

        let mut selection = Selection::new();
        selection.select(sprite.entity_id);
        let mut selected_and_hovered = UIContext::new();
        render_selection_outline(
            &mut selected_and_hovered, &viewport, &selection, Some(sprite.entity_id), &[sprite], &colors, bounds(),
        );
        let drawn = lines(&selected_and_hovered);
        assert_eq!(drawn.len(), 4, "hover must not double-outline a selected entity");
        assert!(drawn.iter().all(|line| line.2 == colors.primary));
    }

    #[test]
    fn test_hover_picks_topmost_by_depth_then_id_and_honours_flip_and_zero_size() {
        let viewport = test_viewport();
        let center = viewport.world_to_screen(Vec2::ZERO);

        let stack = [
            pickable(1, Vec2::ZERO, SIZE, 0.0),
            pickable(2, Vec2::ZERO, SIZE, 10.0),
            pickable(3, Vec2::ZERO, SIZE, 5.0),
        ];
        assert_eq!(hover_entity_at(center, &viewport, &stack), Some(entity(2)), "highest depth wins");
        let tied = [pickable(7, Vec2::ZERO, SIZE, 1.0), pickable(4, Vec2::ZERO, SIZE, 1.0)];
        assert_eq!(hover_entity_at(center, &viewport, &tied), Some(entity(7)), "equal depths: the higher id, deterministically");
        let far = viewport.world_to_screen(Vec2::new(500.0, 500.0));
        assert_eq!(hover_entity_at(far, &viewport, &stack), None, "a miss hovers nothing");

        // A negative (flip) scale hovers and outlines at its absolute size.
        let flipped = [pickable(1, Vec2::ZERO, Vec2::new(-50.0, 50.0), 0.0)];
        assert_eq!(hover_entity_at(viewport.world_to_screen(Vec2::new(10.0, 10.0)), &viewport, &flipped), Some(entity(1)));
        let mut selection = Selection::new();
        selection.select(entity(1));
        let mut ui = UIContext::new();
        render_selection_outline(&mut ui, &viewport, &selection, None, &flipped, &colors(), bounds());
        assert_eq!(lines(&ui).len(), 4);

        // A zero-size pickable neither hovers nor draws as a dot.
        let point = [pickable(1, Vec2::ZERO, Vec2::ZERO, 0.0)];
        assert_eq!(hover_entity_at(center, &viewport, &point), None);
        let mut ui = UIContext::new();
        render_selection_outline(&mut ui, &viewport, &selection, Some(entity(1)), &point, &colors(), bounds());
        assert_eq!(lines(&ui).len(), 0, "a degenerate outline is skipped");
    }
}
