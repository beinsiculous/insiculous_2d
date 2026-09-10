//! Hierarchy panel for displaying entity tree structure.
//!
//! The HierarchyPanel displays all entities in the scene as a tree view,
//! showing parent-child relationships and allowing entity selection. Each
//! entity's `Scripts` appear as pseudo-rows beneath it, and an entity row
//! accepts a dropped `.rhai` asset.

use std::collections::HashSet;

use ecs::{EntityId, Name, Scripts, World, WorldHierarchyExt};
use glam::Vec2;

use crate::drag_drop::{DragDropState, DragPayload};
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
    /// The last row a rename was opened on, kept until that field's
    /// keyboard focus has been accounted for.
    last_rename: Option<EntityId>,
    /// Whether the rename field was drawn in the last render pass. A field
    /// that went undrawn — cancelled from outside, or scrolled off the
    /// panel — would keep the keyboard forever, since only a drawn field
    /// can handle the Escape that would release it.
    rename_field_drawn: bool,
    /// Rows drawn in the last render pass, script rows included. A pass that
    /// drew none — the panel collapsed to a strip, a splitter dragged through
    /// zero — says nothing about the rename field, so it does not end the
    /// rename; a pass that drew rows without the field does.
    rows_drawn: usize,
    /// Consecutive passes that drew no rows while a rename was open. A
    /// splitter dragged through zero is a frame or two; a panel left at
    /// zero would otherwise hold the keyboard for as long as it sits there.
    empty_passes: u8,
    /// Every row of the last render pass in draw order — collapsed
    /// subtrees excluded, off-panel rows included. Shift-click ranges are
    /// computed over it.
    visible_order: Vec<EntityId>,
}

/// What was clicked in the hierarchy panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyClick {
    /// An entity row was clicked.
    Entity(EntityId),
    /// A script pseudo-row was clicked.
    Script { entity: EntityId, index: usize },
}

/// What one hierarchy render pass reported back to the host.
#[derive(Debug, Default)]
pub struct HierarchyResponse {
    /// Items clicked for selection this frame.
    pub clicked: Vec<HierarchyClick>,
    /// An inline rename committed this frame (entity, new text). The text is
    /// raw — the host trims it, ignores empty commits, and records the undo
    /// command.
    pub rename_committed: Option<(EntityId, String)>,
    /// A script asset dropped onto an entity row (entity, asset-relative path).
    pub script_dropped: Option<(EntityId, String)>,
}

/// Shared state for one hierarchy render pass, threaded through the node recursion.
struct NodeRenderCtx<'a> {
    ui: &'a mut ui::UIContext,
    world: &'a World,
    selection: &'a Selection,
    theme: &'a EditorTheme,
    fills: SelectionRowFills,
    bounds: common::Rect,
    drag_drop: &'a mut DragDropState,
    clicked: &'a mut Vec<HierarchyClick>,
    rename_committed: &'a mut Option<(EntityId, String)>,
    script_dropped: &'a mut Option<(EntityId, String)>,
}

impl HierarchyPanel {
    /// Create a new hierarchy panel.
    pub fn new() -> Self {
        Self {
            collapsed: HashSet::new(),
            scroll: crate::ScrollState::default(),
            renaming: None,
            last_rename: None,
            rename_field_drawn: false,
            rows_drawn: 0,
            empty_passes: 0,
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

    /// Inverse of [`entity_display_name`], for name-first entity addressing:
    /// exact match on the `Name` component only — synthesized
    /// display names ("Sprite (Entity 5)") are addressable by id instead.
    /// Nothing enforces name uniqueness, so ambiguity is reported, never
    /// silently resolved to the first match.
    ///
    /// [`entity_display_name`]: crate::entity_names::entity_display_name
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

    /// Render the hierarchy panel.
    ///
    /// Returns the clicks and any committed inline rename or script drop.
    pub fn render(
        &mut self,
        ui: &mut ui::UIContext,
        world: &World,
        selection: &mut Selection,
        bounds: common::Rect,
        theme: &EditorTheme,
        drag_drop: &mut DragDropState,
    ) -> HierarchyResponse {
        let mut clicked = Vec::new();
        let mut rename_committed = None;
        let mut script_dropped = None;

        // A renamed entity that no longer exists (deleted mid-rename, or a
        // scene swap) must not leave the panel armed — a recycled id could
        // otherwise open a brand-new entity in rename mode.
        if let Some(renaming) = self.renaming {
            if world.get_entity(&renaming).is_err() {
                self.renaming = None;
            }
        }

        // A rename field that is not drawn — cancelled from outside the
        // panel, or scrolled out of it — would stay focused and swallow
        // every key the editor and the game would otherwise see, with no
        // field left to take the Escape. Focus follows the drawn field.
        self.settle_rename_focus(ui);

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
            drag_drop,
            clicked: &mut clicked,
            rename_committed: &mut rename_committed,
            script_dropped: &mut script_dropped,
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

        HierarchyResponse {
            clicked,
            rename_committed,
            script_dropped,
        }
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
            self.rows_drawn += 1;
            self.render_row(ctx, entity, depth, y, is_expanded);
        }

        let mut current_y = y + ROW_HEIGHT;
        if let Some(scripts) = ctx.world.get::<Scripts>(entity) {
            for (index, script) in scripts.0.iter().enumerate() {
                let pseudo_row_visible =
                    current_y + ROW_HEIGHT >= bounds.y && current_y <= bounds.y + bounds.height;
                if pseudo_row_visible {
                    self.rows_drawn += 1;
                    self.render_script_row(ctx, entity, index, script, depth + 1, current_y);
                }
                current_y += ROW_HEIGHT;
            }
        }

        // Render children if expanded
        if is_expanded {
            self.render_children(ctx, entity, depth, current_y)
        } else {
            current_y
        }
    }

    fn render_script_row(
        &mut self,
        ctx: &mut NodeRenderCtx<'_>,
        entity: EntityId,
        index: usize,
        script: &ecs::ScriptRef,
        depth: usize,
        y: f32,
    ) {
        let bounds = ctx.bounds;
        let x = bounds.x + BASE_PADDING + (depth as f32 * INDENT_PER_DEPTH);
        let row_rect = common::Rect::new(bounds.x, y, bounds.width, ROW_HEIGHT);

        let row_id = format!("hierarchy_script_{}_{}", entity.value(), index);
        let row_interaction = ctx.ui.interact(row_id.as_str(), row_rect, true);

        if row_interaction.clicked && !ctx.drag_drop.suppresses_click() {
            ctx.clicked.push(HierarchyClick::Script { entity, index });
        }

        if row_interaction.state == ui::WidgetState::Hovered {
            ctx.ui.rect(row_rect, ctx.theme.hover_fill);
        }

        let label = script_display_label(script);
        ctx.ui.label(&label, Vec2::new(x, y + ROW_HEIGHT - 4.0));
    }

    fn render_row(
        &mut self,
        ctx: &mut NodeRenderCtx<'_>,
        entity: EntityId,
        depth: usize,
        y: f32,
        is_expanded: bool,
    ) {
        let bounds = ctx.bounds;
        let x = bounds.x + BASE_PADDING + (depth as f32 * INDENT_PER_DEPTH);
        let has_children = ctx.world.get_children(entity).is_some_and(|children| !children.is_empty());
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

        let is_hovered = row_rect.contains(ctx.ui.mouse_pos());
        let dragging_script = matches!(ctx.drag_drop.dragging_payload(), Some(DragPayload::Script { .. }));
        if dragging_script && is_hovered {
            ctx.ui.rect_border(row_rect, ctx.theme.accent_blue, 1.5, 0.0);
        }

        if let Some((payload, _pos)) = ctx.drag_drop.take_drop_in(row_rect) {
            match payload {
                DragPayload::Script { path } => {
                    *ctx.script_dropped = Some((entity, path));
                }
                DragPayload::Texture { .. } => {}
            }
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
        let geometry = RowGeometry {
            x,
            name_x,
            row_rect,
            has_children,
            is_selected,
        };

        if self.renaming == Some(entity) {
            self.render_rename_field(ctx, entity, name_x, y);
        } else {
            Self::render_row_label(ctx, entity, &geometry, arrow_clicked);
        }
    }

    fn render_rename_field(
        &mut self,
        ctx: &mut NodeRenderCtx<'_>,
        entity: EntityId,
        name_x: f32,
        y: f32,
    ) {
        let bounds = ctx.bounds;
        self.rename_field_drawn = true;
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
            .map(|name| name.as_str().to_string())
            .unwrap_or_default();
        if let Some(committed) = ctx.ui.text_input(rename_id.as_str(), &current, field_rect) {
            self.renaming = None;
            *ctx.rename_committed = Some((entity, committed));
        } else if !ctx.ui.is_focused(rename_id.as_str()) {
            // Escape (or focus lost without a commit) — plain cancel.
            self.renaming = None;
        }
    }

    fn render_row_label(
        ctx: &mut NodeRenderCtx<'_>,
        entity: EntityId,
        row: &RowGeometry,
        arrow_clicked: bool,
    ) {
        let bounds = ctx.bounds;
        // Row interaction - use area after arrow for entities with children
        let row_interact_x = if row.has_children { row.x + ARROW_WIDTH } else { bounds.x };
        let row_interact_width = bounds.x + bounds.width - row_interact_x;
        let row_interact_rect =
            common::Rect::new(row_interact_x, row.row_rect.y, row_interact_width, ROW_HEIGHT);

        let row_id = format!("hierarchy_row_{}", entity.value());
        let row_interaction = ctx.ui.interact(row_id.as_str(), row_interact_rect, true);

        if row_interaction.clicked && !arrow_clicked && !ctx.drag_drop.suppresses_click() {
            ctx.clicked.push(HierarchyClick::Entity(entity));
        }

        // Hover highlight (full row width for visual consistency)
        if row_interaction.state == ui::WidgetState::Hovered && !row.is_selected {
            ctx.ui.rect(row.row_rect, ctx.theme.hover_fill);
        }

        // Entity name (baseline near bottom of row)
        let name = crate::entity_names::entity_display_name(ctx.world, entity);
        ctx.ui.label(&name, Vec2::new(row.name_x, row.row_rect.y + ROW_HEIGHT - 4.0));
    }

    fn render_children(
        &mut self,
        ctx: &mut NodeRenderCtx<'_>,
        entity: EntityId,
        depth: usize,
        y: f32,
    ) -> f32 {
        let mut next_y = y;
        if let Some(children) = ctx.world.get_children(entity) {
            // Clone to avoid borrow issues
            let children_vec: Vec<EntityId> = children.to_vec();
            for child in children_vec {
                next_y = self.render_node(ctx, child, depth + 1, next_y);
            }
        }
        next_y
    }
}

/// One row's placement, computed once so the label half of the row does
/// not take it as seven parameters.
struct RowGeometry {
    x: f32,
    name_x: f32,
    row_rect: common::Rect,
    has_children: bool,
    is_selected: bool,
}

fn script_display_label(script: &ecs::ScriptRef) -> String {
    let script_id = script.script_id.trim();
    if !script_id.is_empty() {
        return script_id.to_string();
    }
    let source_path = script.source_path.trim();
    if !source_path.is_empty() {
        if let Some(stem) = std::path::Path::new(source_path).file_stem().and_then(|s| s.to_str()) {
            let stem = stem.trim();
            if !stem.is_empty() {
                return stem.to_string();
            }
        }
    }
    "script".to_string()
}

mod rename;

#[cfg(test)]
mod tests;
