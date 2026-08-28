//! Panel content rendering for editor dock panels.
//!
//! Extracted from editor_demo.rs — renders the content inside each dock panel
//! (scene view, hierarchy tree, inspector, asset browser).

use glam::Vec2;

use editor::{CommandHistory, EditorContext, HierarchyPanel, PanelId};
use engine_core::contexts::GameContext;

/// Render the content of a specific dock panel.
pub fn render_panel_content(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    panel_id: PanelId,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    let padding = 8.0;
    let content_x = bounds.x + padding;
    let y = bounds.y + padding;

    match panel_id {
        PanelId::SCENE_VIEW => render_scene_view(editor, ctx, bounds),
        PanelId::HIERARCHY => render_hierarchy(editor, ctx, bounds),
        PanelId::INSPECTOR => render_inspector(editor, ctx, bounds, command_history),
        PanelId::ASSET_BROWSER => {
            asset_browser::render_asset_browser(editor, ctx, bounds, command_history)
        }
        _ => render_default(ctx, content_x, y),
    }
}

pub(crate) use asset_browser::render_drag_ghost;

/// Scene view — grid info, viewport origin crosshair, and play-state border.
fn render_scene_view(editor: &EditorContext, ctx: &mut GameContext, bounds: common::Rect) {
    let theme = &editor.theme;
    let padding = 8.0;
    let content_x = bounds.x + padding;
    let y = bounds.y + padding;

    if editor.is_grid_visible() {
        ctx.ui.label_styled(
            &format!("Grid: {}px", editor.grid_size()),
            Vec2::new(content_x, y),
            theme.text_muted,
            theme.fonts.small,
        );
    }

    // Draw the world-origin crosshair where (0,0) actually is under the
    // current pan/zoom (the panel clip rect trims any overshoot).
    let center = editor.world_to_screen(Vec2::ZERO);
    ctx.ui.circle(center, 5.0, theme.border_subtle);
    ctx.ui.line(
        Vec2::new(center.x - 20.0, center.y),
        Vec2::new(center.x + 20.0, center.y),
        theme.separator,
        1.0,
    );
    ctx.ui.line(
        Vec2::new(center.x, center.y - 20.0),
        Vec2::new(center.x, center.y + 20.0),
        theme.separator,
        1.0,
    );

    // Collider outlines — drawn over the rendered sprites so physics shapes
    // can be compared against the visuals and tuned until they line up.
    if editor.is_colliders_visible() {
        editor::render_collider_overlay(
            ctx.ui,
            ctx.world,
            &editor.viewport,
            &editor.selection,
            &editor.theme.collider_overlay_colors(),
            bounds,
        );
    }

    // Selection + hover outlines — an editing affordance, so hidden while
    // Playing (picking and gizmos are disabled then too). Built from the
    // same pickable list picking uses, so the outline always matches what a
    // click selects; hover reads the same input-frame mouse state picking
    // will read one step later, so hint and click agree.
    if !editor.is_playing() {
        let pickables = crate::editor_game::build_pickable_entities(ctx.world);
        let mouse = ctx.ui.mouse_pos();
        let hover_allowed = editor.viewport.contains_screen_point(mouse)
            && !crate::editor_game::chrome_owns_mouse(ctx.ui)
            && !editor.drag_drop.suppresses_click()
            && !editor.gizmo_has_priority();
        let hovered = if hover_allowed {
            editor::hover_entity_at(mouse, &editor.viewport, &pickables)
        } else {
            None
        };
        editor::render_selection_outline(
            ctx.ui,
            &editor.viewport,
            &editor.selection,
            hovered,
            &pickables,
            &editor.theme.selection_outline_colors(),
            ui::Rect::new(bounds.x, bounds.y, bounds.width, bounds.height),
        );
    }

    // Play-state border tint
    let border_color = theme.play_state_border(editor.play_state());
    let w = if editor.in_play_session() { 3.0 } else { 1.0 };

    // Top
    ctx.ui.line(
        Vec2::new(bounds.x, bounds.y),
        Vec2::new(bounds.x + bounds.width, bounds.y),
        border_color, w,
    );
    // Bottom
    ctx.ui.line(
        Vec2::new(bounds.x, bounds.y + bounds.height),
        Vec2::new(bounds.x + bounds.width, bounds.y + bounds.height),
        border_color, w,
    );
    // Left
    ctx.ui.line(
        Vec2::new(bounds.x, bounds.y),
        Vec2::new(bounds.x, bounds.y + bounds.height),
        border_color, w,
    );
    // Right
    ctx.ui.line(
        Vec2::new(bounds.x + bounds.width, bounds.y),
        Vec2::new(bounds.x + bounds.width, bounds.y + bounds.height),
        border_color, w,
    );
}

/// Hierarchy — tree view with click-to-select and Ctrl toggle.
fn render_hierarchy(editor: &mut EditorContext, ctx: &mut GameContext, bounds: common::Rect) {
    let clicked = editor.hierarchy.render(
        ctx.ui,
        ctx.world,
        &mut editor.selection,
        bounds,
        &editor.theme,
    );

    if !clicked.is_empty() {
        editor.close_add_component_popup();
    }
    for entity_id in clicked {
        if ctx.input.keyboard().is_key_pressed(winit::keyboard::KeyCode::ControlLeft) {
            editor.selection.toggle(entity_id);
        } else {
            editor.selection.select(entity_id);
        }
        log::info!(
            "Selected entity: {} ({})",
            HierarchyPanel::entity_display_name(ctx.world, entity_id),
            entity_id.value()
        );
    }
}


/// Fallback for unknown panels.
fn render_default(ctx: &mut GameContext, content_x: f32, y: f32) {
    ctx.ui.label("Panel", Vec2::new(content_x, y));
}

mod asset_browser;
mod inspector;
use inspector::render_inspector;

#[cfg(test)]
mod tests;
