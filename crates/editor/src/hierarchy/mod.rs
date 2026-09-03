//! Hierarchy panel for displaying entity tree structure.
//!
//! The HierarchyPanel displays all entities in the scene as a tree view,
//! showing parent-child relationships and allowing entity selection.

use std::collections::HashSet;

use ecs::{EntityId, Name, Sprite, World, WorldHierarchyExt};
use glam::Vec2;
use physics::components::RigidBody;

use crate::layout::{LINE_HEIGHT, PADDING};
use crate::theme::EditorTheme;
use crate::Selection;
use ui::Color;

/// Row height for each entity in the hierarchy (matches LINE_HEIGHT).
const ROW_HEIGHT: f32 = LINE_HEIGHT;

/// Base left padding (matches standard PADDING).
const BASE_PADDING: f32 = PADDING;

/// Indentation per depth level.
const INDENT_PER_DEPTH: f32 = 16.0;

/// Width of the expand/collapse arrow.
const ARROW_WIDTH: f32 = 16.0;

/// Result of resolving an entity by its `Name` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameResolution {
    /// No entity carries this name.
    None,
    /// Exactly one match.
    One(EntityId),
    /// Multiple entities share the name — callers report, never pick one.
    Ambiguous(Vec<EntityId>),
}

/// Normalize a rename commit: whitespace-trimmed, with empty and unchanged
/// results rejected — an entity can never be stranded with a blank `Name`,
/// and a no-op commit records no undo entry.
pub fn normalized_rename(current: Option<&str>, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || Some(trimmed) == current {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Width of the accent bar marking the primary selected row, in pixels.
pub const PRIMARY_ACCENT_WIDTH: f32 = 3.0;

/// Row fills for selected hierarchy rows, derived from the editor theme via
/// `EditorTheme::selection_row_fills()`. The primary row — the one the
/// inspector shows and gizmos pivot on — reads differently from the rest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionRowFills {
    /// Background of the primary selected row.
    pub primary: Color,
    /// Background of every other selected row.
    pub secondary: Color,
    /// The accent bar on the primary row's left edge.
    pub accent: Color,
}

/// Hierarchy panel for displaying entity tree structure.
#[derive(Debug, Default)]
pub struct HierarchyPanel {
    /// Entities that are collapsed (all expanded by default).
    collapsed: HashSet<EntityId>,
    /// Vertical scroll for long entity lists.
    pub scroll: crate::ScrollState,
    /// Row currently in inline-rename mode (F2), if any.
    renaming: Option<EntityId>,
    /// Every row of the last render pass in draw order — collapsed
    /// subtrees excluded, off-panel rows included. Shift-click ranges are
    /// computed over it.
    visible_order: Vec<EntityId>,
}

/// What one hierarchy render pass reported back to the host.
#[derive(Debug, Default)]
pub struct HierarchyResponse {
    /// Entities clicked for selection this frame.
    pub clicked: Vec<EntityId>,
    /// An inline rename committed this frame (entity, new text). The text is
    /// raw — the host trims it, ignores empty commits, and records the undo
    /// command.
    pub rename_committed: Option<(EntityId, String)>,
}

/// Shared state for one hierarchy render pass, threaded through the node recursion.
struct NodeRenderCtx<'a> {
    ui: &'a mut ui::UIContext,
    world: &'a World,
    selection: &'a Selection,
    theme: &'a EditorTheme,
    fills: SelectionRowFills,
    bounds: common::Rect,
    clicked_entities: &'a mut Vec<EntityId>,
    rename_committed: &'a mut Option<(EntityId, String)>,
}

impl HierarchyPanel {
    /// Create a new hierarchy panel.
    pub fn new() -> Self {
        Self {
            collapsed: HashSet::new(),
            scroll: crate::ScrollState::default(),
            renaming: None,
            visible_order: Vec::new(),
        }
    }

    /// The rows of the last render pass in draw order (collapsed subtrees
    /// excluded, off-panel rows included).
    pub fn visible_order(&self) -> &[EntityId] {
        &self.visible_order
    }

    /// The rows a Shift-click on `target` selects, anchor first: from the
    /// primary when it is a visible row, else from the LAST visible selected
    /// row (a primary hidden under a collapsed parent must not silently
    /// collapse the selection to one row). `None` when
    /// no selected row is visible; the host then adds `target` instead.
    pub fn shift_click_range(&self, selection: &Selection, target: EntityId) -> Option<Vec<EntityId>> {
        let index_of = |entity: EntityId| self.visible_order.iter().position(|&row| row == entity);
        let target_index = index_of(target)?;
        let anchor_index = selection
            .primary()
            .and_then(index_of)
            .or_else(|| self.visible_order.iter().rposition(|&row| selection.contains(row)))?;
        let (low, high) = (anchor_index.min(target_index), anchor_index.max(target_index));
        let mut range = self.visible_order[low..=high].to_vec();
        if anchor_index > target_index {
            range.reverse();
        }
        Some(range)
    }

    /// Check if an entity is expanded (default: true).
    pub fn is_expanded(&self, entity: EntityId) -> bool {
        !self.collapsed.contains(&entity)
    }

    /// Toggle expand/collapse state for an entity.
    pub fn toggle_expanded(&mut self, entity: EntityId) {
        if self.collapsed.contains(&entity) {
            self.collapsed.remove(&entity);
        } else {
            self.collapsed.insert(entity);
        }
    }

    /// Get the display name for an entity.
    ///
    /// Resolution order:
    /// 1. Name component
    /// 2. Sprite component → "Sprite (Entity {id})"
    /// 3. RigidBody component → "RigidBody (Entity {id})"
    /// 4. Fallback → "Entity {id}"
    pub fn entity_display_name(world: &World, entity: EntityId) -> String {
        // Check for Name component first
        if let Some(name) = world.get::<Name>(entity) {
            return name.as_str().to_string();
        }

        // Check for Sprite component
        if world.get::<Sprite>(entity).is_some() {
            return format!("Sprite (Entity {})", entity.value());
        }

        // Check for RigidBody component
        if world.get::<RigidBody>(entity).is_some() {
            return format!("RigidBody (Entity {})", entity.value());
        }

        // Fallback
        format!("Entity {}", entity.value())
    }

    /// Inverse of [`entity_display_name`], for name-first entity addressing:
    /// exact match on the `Name` component only — synthesized
    /// display names ("Sprite (Entity 5)") are addressable by id instead.
    /// Nothing enforces name uniqueness, so ambiguity is reported, never
    /// silently resolved to the first match.
    ///
    /// [`entity_display_name`]: HierarchyPanel::entity_display_name
    pub fn resolve_by_name(world: &World, name: &str) -> NameResolution {
        let mut matches = world
            .entities()
            .into_iter()
            .filter(|e| world.get::<Name>(*e).is_some_and(|n| n.as_str() == name));
        match (matches.next(), matches.next()) {
            (None, _) => NameResolution::None,
            (Some(only), None) => NameResolution::One(only),
            (Some(first), Some(second)) => {
                let mut all = vec![first, second];
                all.extend(matches);
                // World iteration order is hash-based; a deterministic id
                // order keeps ambiguity reports stable across sessions.
                all.sort_by_key(|e| e.value());
                NameResolution::Ambiguous(all)
            }
        }
    }

    /// Widget id of an entity's inline rename field — shared by the panel's
    /// render pass and the host's `focus_text_input` call so F2 lands in an
    /// already-focused field.
    pub fn rename_widget_id(entity: EntityId) -> String {
        format!("hierarchy_rename_{}", entity.value())
    }

    /// Enter inline-rename mode for `entity` (the host focuses the field via
    /// `UIContext::focus_text_input` with the same widget id).
    pub fn begin_rename(&mut self, entity: EntityId) {
        self.renaming = Some(entity);
    }

    /// Row currently in inline-rename mode, if any.
    pub fn renaming(&self) -> Option<EntityId> {
        self.renaming
    }

    /// Render the hierarchy panel.
    ///
    /// Returns the clicks and any committed inline rename.
    pub fn render(
        &mut self,
        ui: &mut ui::UIContext,
        world: &World,
        selection: &mut Selection,
        bounds: common::Rect,
        theme: &EditorTheme,
    ) -> HierarchyResponse {
        let mut clicked_entities = Vec::new();
        let mut rename_committed = None;

        // A renamed entity that no longer exists (deleted mid-rename, or a
        // scene swap) must not leave the panel armed — a recycled id could
        // otherwise open a brand-new entity in rename mode.
        if let Some(renaming) = self.renaming {
            if world.get_entity(&renaming).is_err() {
                self.renaming = None;
            }
        }

        // Get root entities (no parent) and sort by ID for consistent ordering
        let mut roots = world.get_root_entities();
        roots.sort_by_key(|e| e.value());

        self.visible_order.clear();
        let mut ctx = NodeRenderCtx {
            ui,
            world,
            selection,
            theme,
            fills: theme.selection_row_fills(),
            bounds,
            clicked_entities: &mut clicked_entities,
            rename_committed: &mut rename_committed,
        };

        // Render each root and its descendants with top padding, offset by
        // the panel scroll (render_node culls off-panel rows but still
        // advances y, so offset rows lay out for free).
        let offset = self.scroll.begin_frame(
            bounds,
            ctx.ui.mouse_pos(),
            ctx.ui.scroll_delta(),
            bounds.height,
        );
        let top = bounds.y + BASE_PADDING - offset;
        let mut y = top;
        for root in roots {
            y = self.render_node(&mut ctx, root, 0, y);
        }
        self.scroll.end_frame(y - top + BASE_PADDING, bounds.height);

        HierarchyResponse { clicked: clicked_entities, rename_committed }
    }

    /// Render a single node and its children recursively.
    ///
    /// Returns the next Y position after this node and its visible children.
    fn render_node(
        &mut self,
        ctx: &mut NodeRenderCtx<'_>,
        entity: EntityId,
        depth: usize,
        y: f32,
    ) -> f32 {
        let bounds = ctx.bounds;
        self.visible_order.push(entity);

        // Read once, before the arrow click can toggle it: the glyph and the
        // child walk below must agree within the frame.
        let is_expanded = self.is_expanded(entity);
        let row_visible = y + ROW_HEIGHT >= bounds.y && y <= bounds.y + bounds.height;
        if row_visible {
            let x = bounds.x + BASE_PADDING + (depth as f32 * INDENT_PER_DEPTH);
            let has_children = ctx.world.get_children(entity).is_some_and(|c| !c.is_empty());
            let is_selected = ctx.selection.contains(entity);
            let is_primary = ctx.selection.primary() == Some(entity);

            // Row background for selection (full width); the primary row gets
            // its own fill plus a left accent bar.
            let row_rect = common::Rect::new(bounds.x, y, bounds.width, ROW_HEIGHT);
            if is_primary {
                ctx.ui.rect(row_rect, ctx.fills.primary);
                let accent_rect = common::Rect::new(bounds.x, y, PRIMARY_ACCENT_WIDTH, ROW_HEIGHT);
                ctx.ui.rect(accent_rect, ctx.fills.accent);
            } else if is_selected {
                ctx.ui.rect(row_rect, ctx.fills.secondary);
            }

            // Check arrow interaction FIRST for entities with children. The
            // arrow goes inert while this row is being renamed — collapsing the
            // tree under an active text field would reflow it mid-edit.
            let mut arrow_clicked = false;
            if has_children {
                if self.renaming != Some(entity) {
                    let arrow_rect = common::Rect::new(x, y, ARROW_WIDTH, ROW_HEIGHT);
                    let arrow_id = format!("hierarchy_arrow_{}", entity.value());
                    let arrow_interaction = ctx.ui.interact(arrow_id.as_str(), arrow_rect, true);

                    if arrow_interaction.clicked {
                        self.toggle_expanded(entity);
                        arrow_clicked = true;
                    }
                }

                // Draw arrow (baseline near bottom of row)
                let arrow = if is_expanded { "▼" } else { "▶" };
                ctx.ui.label(arrow, Vec2::new(x, y + ROW_HEIGHT - 4.0));
            }

            let name_x = x + if has_children { ARROW_WIDTH } else { 0.0 };

            if self.renaming == Some(entity) {
                // Inline rename replaces the label AND the row's click handling —
                // the text field owns the row while it is open.
                let rename_id = Self::rename_widget_id(entity);
                let field_rect = ui::Rect::new(
                    name_x,
                    y + 1.0,
                    (bounds.x + bounds.width - name_x - BASE_PADDING).max(60.0),
                    ROW_HEIGHT - 2.0,
                );
                let current = ctx
                    .world
                    .get::<Name>(entity)
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_default();
                if let Some(committed) = ctx.ui.text_input(rename_id.as_str(), &current, field_rect) {
                    self.renaming = None;
                    *ctx.rename_committed = Some((entity, committed));
                } else if !ctx.ui.is_focused(rename_id.as_str()) {
                    // Escape (or focus lost without a commit) — plain cancel.
                    self.renaming = None;
                }
            } else {
                // Row interaction - use area after arrow for entities with children
                let row_interact_x = if has_children { x + ARROW_WIDTH } else { bounds.x };
                let row_interact_width = bounds.x + bounds.width - row_interact_x;
                let row_interact_rect =
                    common::Rect::new(row_interact_x, y, row_interact_width, ROW_HEIGHT);

                let row_id = format!("hierarchy_row_{}", entity.value());
                let row_interaction = ctx.ui.interact(row_id.as_str(), row_interact_rect, true);

                if row_interaction.clicked && !arrow_clicked {
                    ctx.clicked_entities.push(entity);
                }

                // Hover highlight (full row width for visual consistency)
                if row_interaction.state == ui::WidgetState::Hovered && !is_selected {
                    ctx.ui.rect(row_rect, ctx.theme.hover_fill);
                }

                // Entity name (baseline near bottom of row)
                let name = Self::entity_display_name(ctx.world, entity);
                ctx.ui.label(&name, Vec2::new(name_x, y + ROW_HEIGHT - 4.0));
            }
        }

        // Render children if expanded
        let mut next_y = y + ROW_HEIGHT;
        if is_expanded {
            if let Some(children) = ctx.world.get_children(entity) {
                // Clone to avoid borrow issues
                let children_vec: Vec<EntityId> = children.to_vec();
                for child in children_vec {
                    next_y = self.render_node(ctx, child, depth + 1, next_y);
                }
            }
        }

        next_y
    }
}


#[cfg(test)]
mod tests;
