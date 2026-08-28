//! Inspector panel: editable component fields with undo-recorded writeback,
//! read-only view during play, remove buttons, and the add-component popup.

use glam::Vec2;

use editor::{
    available_components, categorized_components, edit_all_components,
    inspect_all_components, CommandHistory, ComponentKind, EditorContext,
    FieldId, InspectorStyle,
};
use engine_core::contexts::GameContext;

/// Inspector — component inspection for the selected entity.
///
/// During Editing/Paused: renders editable fields with live writeback.
/// During Playing: renders read-only view via `inspect_component()`.
pub(super) fn render_inspector(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    let line_height = 20.0;
    let padding = 8.0;
    let content_x = bounds.x + padding;

    let entity_id = match editor.selection.primary() {
        Some(id) => id,
        None => {
            ctx.ui.label("No selection", Vec2::new(content_x, bounds.y + padding));
            return;
        }
    };

    // A different entity starts at the top — a leaked offset would open
    // entity B scrolled to wherever entity A was (kimi round 6 F2).
    if editor.inspector_scroll_entity != Some(entity_id) {
        editor.inspector_scroll = Default::default();
        editor.inspector_scroll_entity = Some(entity_id);
    }

    // Panel scroll (audit §3.3): offset the whole walk; content height is
    // measured at the end (partial rows bleed one row past the panel edge
    // until #41 lands GPU scissoring — clipping today is cull-only).
    // While the add-component popup is open the wheel is NOT ours: the
    // window-anchored popup would detach from its scrolling button
    // (kimi round 7 F2).
    let wheel = if editor.is_add_component_popup_open() {
        0.0
    } else {
        ctx.ui.scroll_delta()
    };
    let offset = editor.inspector_scroll.begin_frame(
        bounds,
        ctx.ui.mouse_pos(),
        wheel,
        bounds.height,
    );
    let top = bounds.y + padding - offset;
    let mut y = top;

    let heading = format!("Entity: {}", entity_id.value());
    match editor.fonts.bold {
        Some(bold) => ctx.ui.label_with_font(
            &heading,
            Vec2::new(content_x, y),
            bold,
            editor.theme.fonts.heading,
        ),
        None => ctx.ui.label(&heading, Vec2::new(content_x, y)),
    }
    y += line_height;

    let content_width = bounds.width - 2.0 * padding;
    let final_y = if editor.is_playing() {
        render_inspector_readonly(ctx, entity_id, content_x, y, &editor.theme.inspector_style())
    } else {
        render_inspector_editable(
            editor, ctx, entity_id, content_x, content_width, y, command_history,
        )
    };
    editor
        .inspector_scroll
        .end_frame(final_y - top + padding, bounds.height);
}

/// Read-only inspector using the editor's component registry (used during
/// Playing). Returns the next Y (for scroll content measurement).
fn render_inspector_readonly(
    ctx: &mut GameContext,
    entity_id: ecs::EntityId,
    content_x: f32,
    y: f32,
    style: &InspectorStyle,
) -> f32 {
    let line_height = 20.0;
    inspect_all_components(
        ctx.ui, ctx.world, entity_id, content_x, y, style, line_height * 0.5,
    )
}

/// Editable inspector with live writeback (used during Editing/Paused).
/// Returns the next Y (for scroll content measurement).
fn render_inspector_editable(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    entity_id: ecs::EntityId,
    content_x: f32,
    content_width: f32,
    mut y: f32,
    command_history: &mut CommandHistory,
) -> f32 {
    let line_height = 20.0;
    let inspect_style = editor.theme.inspector_style();
    let field_style = editor.theme.editable_field_style();

    // Resolve the sprite texture's display path up front (the editor crate
    // cannot see AssetManager) and hand the drag-drop coordinator to the
    // registry-generated inspector so the Texture slot can accept drops.
    let texture_display = ctx
        .world
        .get::<ecs::sprite_components::Sprite>(entity_id)
        .and_then(|s| ctx.assets.texture_path(s.texture_handle).map(str::to_string));
    let mut extras = editor::InspectorExtras {
        drag_drop: &mut editor.drag_drop,
        texture_display,
    };

    // Every per-component block (field editors, undo-recorded writeback,
    // remove buttons, read-only fallbacks) is generated from the editor's
    // component registry — adding a component to the registry is all it
    // takes to appear here.
    let (next_y, component_index) = edit_all_components(
        ctx.ui,
        ctx.world,
        entity_id,
        command_history,
        content_x,
        content_width,
        y,
        &inspect_style,
        &field_style,
        line_height * 0.5,
        &mut extras,
    );
    y = next_y;

    // --- [+ Add Component] button ---
    y += line_height;
    let btn_bounds = ui::Rect::new(content_x, y, 160.0, 24.0);
    let add_btn_id = FieldId::new(component_index + 50, 0, 0);
    if ctx.ui.button(add_btn_id, "+ Add Component", btn_bounds) {
        editor.toggle_add_component_popup();
    }
    y += 28.0;

    // --- Add Component Popup ---
    if editor.is_add_component_popup_open() {
        let available = available_components(ctx.world, entity_id);
        if available.is_empty() {
            ctx.ui.label("(all components added)", Vec2::new(content_x + 8.0, y));
            y += line_height;
        } else {
            // Height first, then anchor against the WINDOW (the popup lives
            // on the Floating layer, free of the panel clip): open below
            // the button, flip up when it would overflow the window bottom.
            let popup_height = categorized_popup_height(&available);
            let popup_y0 = popup_anchor_y(y, 28.0, popup_height, ctx.window_size.y);
            let popup_bounds = ui::Rect::new(content_x, popup_y0, 180.0, popup_height);
            // Floating layer + input blocking: escapes the inspector clip
            // rect and widgets underneath go inert while the mouse is on it.
            ctx.ui.begin_overlay(popup_bounds);
            ctx.ui.panel_styled(
                popup_bounds,
                editor.theme.surface_4,
                editor.theme.popup_border,
                1.0,
            );

            let mut popup_y = popup_y0 + 4.0;
            let mut popup_btn_idx: usize = 0;

            for (category, kinds) in categorized_components() {
                let visible: Vec<ComponentKind> = kinds.iter()
                    .copied()
                    .filter(|k| available.contains(k))
                    .collect();
                if visible.is_empty() {
                    continue;
                }

                ctx.ui.label_styled(
                    category.label(),
                    Vec2::new(content_x + 8.0, popup_y),
                    editor.theme.text_muted,
                    editor.theme.fonts.small,
                );
                popup_y += 18.0;

                for kind in visible {
                    let btn_bounds = ui::Rect::new(content_x + 16.0, popup_y, 148.0, 22.0);
                    let btn_id = FieldId::new(component_index + 60 + popup_btn_idx, 0, 0);
                    if ctx.ui.button(btn_id, kind.display_name(), btn_bounds) {
                        let cmd = editor::commands::AddComponentCommand::new(entity_id, kind);
                        command_history.execute(Box::new(cmd), ctx.world);
                        editor.close_add_component_popup();
                        log::info!("Added component: {}", kind.display_name());
                    }
                    popup_y += 24.0;
                    popup_btn_idx += 1;
                }
            }
            ctx.ui.end_overlay();
        }
    }

    let _ = component_index;
    y
}

/// Calculate the height needed for the categorized popup.
fn categorized_popup_height(available: &[ComponentKind]) -> f32 {
    let mut height = 8.0; // padding
    for (_, kinds) in categorized_components() {
        let visible_count = kinds.iter().filter(|k| available.contains(k)).count();
        if visible_count > 0 {
            height += 18.0; // category label
            height += visible_count as f32 * 24.0; // buttons
        }
    }
    height
}

/// Where the popup opens: below its anchor when it fits the window,
/// flipped above (`anchor - button_height - popup`) otherwise, clamped to
/// the window top. The popup is window-anchored (not panel-anchored)
/// because the Floating layer frees it from the panel clip.
fn popup_anchor_y(below_y: f32, button_height: f32, popup_height: f32, window_bottom: f32) -> f32 {
    if below_y + popup_height <= window_bottom {
        below_y
    } else {
        (below_y - button_height - popup_height).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::popup_anchor_y;

    #[test]
    fn test_popup_flips_up_when_it_would_overflow_window() {
        // Fits below: opens at the anchor.
        assert_eq!(popup_anchor_y(100.0, 28.0, 200.0, 720.0), 100.0);
        // Would overflow the window bottom: flips above the button.
        assert_eq!(popup_anchor_y(650.0, 28.0, 200.0, 720.0), 650.0 - 28.0 - 200.0);
        // Taller than everything: clamps to the window top, never negative.
        assert_eq!(popup_anchor_y(100.0, 28.0, 900.0, 720.0), 0.0);
    }
}
