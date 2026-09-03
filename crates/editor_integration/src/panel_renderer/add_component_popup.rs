//! Add Component popup: category and dynamic component rows and height calculation.

use glam::Vec2;

use ecs::EntityId;
use editor::{
    available_components, categorized_components, layout, CommandHistory,
    ComponentKind, EditorContext, FieldId,
};
use engine_core::contexts::GameContext;

/// One row of the popup, in draw order.
#[derive(Debug, PartialEq, Eq)]
enum PopupRow {
    Heading(&'static str),
    Typed(ComponentKind),
    Game(String),
}

/// The rows the popup shows for this entity: each category heading followed
/// by its addable kinds, then a "Game" heading and the addable dynamic
/// components. Built once per frame — `categorized_components()` allocates,
/// and this is its only call.
fn popup_rows(available: &[ComponentKind], available_dynamic: &[String]) -> Vec<PopupRow> {
    let mut rows = Vec::new();
    for (category, kinds) in categorized_components() {
        let visible: Vec<ComponentKind> = kinds
            .iter()
            .copied()
            .filter(|k| available.contains(k))
            .collect();
        if visible.is_empty() {
            continue;
        }

        rows.push(PopupRow::Heading(category.label()));
        for kind in visible {
            rows.push(PopupRow::Typed(kind));
        }
    }

    if !available_dynamic.is_empty() {
        rows.push(PopupRow::Heading("Game"));
        for name in available_dynamic {
            rows.push(PopupRow::Game(name.clone()));
        }
    }

    rows
}

/// Height of the popup for these rows: top padding plus heading and button rows.
fn popup_height(rows: &[PopupRow]) -> f32 {
    let mut height = 8.0;
    for row in rows {
        match row {
            PopupRow::Heading(_) => height += 18.0,
            PopupRow::Typed(_) | PopupRow::Game(_) => height += 24.0,
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

/// Draw the [+ Add Component] button below the component blocks and, while
/// the popup is open, the popup itself on the Floating layer. Returns the
/// next y for the inspector's scroll measurement.
pub(super) fn render_add_component_section(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    entity_id: EntityId,
    command_history: &mut CommandHistory,
    content_x: f32,
    mut y: f32,
    component_index: usize,
) -> f32 {
    // --- [+ Add Component] button ---
    y += layout::LINE_HEIGHT;
    let button_bounds = ui::Rect::new(content_x, y, 160.0, 24.0);
    let add_button_id = FieldId::slot(component_index, editor::WidgetSlot::AddButton);
    if ctx.ui.button(add_button_id, "+ Add Component", button_bounds) {
        editor.toggle_add_component_popup();
    }
    y += 28.0;

    // --- Add Component Popup ---
    if editor.is_add_component_popup_open() {
        let available = available_components(ctx.world, entity_id);
        // Dynamic-tier (game-registered) components get their own popup
        // section.
        let available_dynamic =
            editor::stored_component::available_dynamic_components(ctx.world, entity_id);
        if available.is_empty() && available_dynamic.is_empty() {
            ctx.ui.label(
                "(all components added)",
                Vec2::new(content_x + layout::PADDING, y),
            );
            y += layout::LINE_HEIGHT;
        } else {
            // Height first, then anchor against the WINDOW (the popup lives
            // on the Floating layer, free of the panel clip): open below
            // the button, flip up when it would overflow the window bottom.
            let rows = popup_rows(&available, &available_dynamic);
            let height = popup_height(&rows);
            let popup_y0 = popup_anchor_y(y, 28.0, height, ctx.window_size.y);
            let popup_bounds = ui::Rect::new(content_x, popup_y0, 180.0, height);
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
            let mut popup_button_index: usize = 0;

            for row in &rows {
                match row {
                    PopupRow::Heading(label) => {
                        ctx.ui.label_styled(
                            label,
                            Vec2::new(content_x + layout::PADDING, popup_y),
                            editor.theme.text_muted,
                            editor.theme.fonts.small,
                        );
                        popup_y += 18.0;
                    }
                    PopupRow::Typed(kind) => {
                        let button_bounds = ui::Rect::new(content_x + 16.0, popup_y, 148.0, 22.0);
                        let button_id = FieldId::slot(
                            component_index,
                            editor::WidgetSlot::PopupRow(popup_button_index),
                        );
                        if ctx.ui.button(button_id, kind.display_name(), button_bounds) {
                            let cmd = editor::commands::AddComponentCommand::new(entity_id, *kind);
                            command_history.execute(Box::new(cmd), ctx.world);
                            editor.close_add_component_popup();
                            log::info!("Added component: {}", kind.display_name());
                        }
                        popup_y += 24.0;
                        popup_button_index += 1;
                    }
                    PopupRow::Game(name) => {
                        let button_bounds = ui::Rect::new(content_x + 16.0, popup_y, 148.0, 22.0);
                        let button_id = FieldId::slot(
                            component_index,
                            editor::WidgetSlot::PopupRow(popup_button_index),
                        );
                        if ctx.ui.button(button_id, name, button_bounds) {
                            let cmd = editor::commands::AddComponentCommand::dynamic(
                                entity_id,
                                name.clone(),
                            );
                            command_history.execute(Box::new(cmd), ctx.world);
                            editor.close_add_component_popup();
                            log::info!("Added dynamic component: {}", name);
                        }
                        popup_y += 24.0;
                        popup_button_index += 1;
                    }
                }
            }
            ctx.ui.end_overlay();
        }
    }

    y
}

#[cfg(test)]
mod tests {
    use super::{popup_anchor_y, popup_height, popup_rows, PopupRow};
    use editor::ComponentKind;

    #[test]
    fn test_popup_flips_up_when_it_would_overflow_window() {
        // Fits below: opens at the anchor.
        assert_eq!(popup_anchor_y(100.0, 28.0, 200.0, 720.0), 100.0);
        // Would overflow the window bottom: flips above the button.
        assert_eq!(popup_anchor_y(650.0, 28.0, 200.0, 720.0), 650.0 - 28.0 - 200.0);
        // Taller than everything: clamps to the window top, never negative.
        assert_eq!(popup_anchor_y(100.0, 28.0, 900.0, 720.0), 0.0);
    }

    #[test]
    fn test_popup_rows_omits_empty_categories_and_includes_game_heading_only_with_dynamic_components() {
        // When no components are available, no headings or rows are generated.
        let empty_rows = popup_rows(&[], &[]);
        assert!(empty_rows.is_empty());

        // When only a dynamic component is available, only the Game heading and dynamic row are generated.
        let dynamic_only = popup_rows(&[], &["Health".to_string()]);
        assert_eq!(
            dynamic_only,
            vec![
                PopupRow::Heading("Game"),
                PopupRow::Game("Health".to_string()),
            ]
        );

        // When one typed component is available, only its category heading is emitted, and no Game heading.
        let typed_only = popup_rows(&[ComponentKind::Sprite], &[]);
        assert_eq!(
            typed_only,
            vec![
                PopupRow::Heading("Rendering"),
                PopupRow::Typed(ComponentKind::Sprite),
            ]
        );
    }

    #[test]
    fn test_popup_height_equals_padding_plus_row_contributions() {
        let rows = vec![
            PopupRow::Heading("Rendering"),
            PopupRow::Typed(ComponentKind::Sprite),
            PopupRow::Heading("Physics"),
            PopupRow::Typed(ComponentKind::RigidBody),
            PopupRow::Heading("Game"),
            PopupRow::Game("Health".to_string()),
        ];
        let headings = 3.0;
        let buttons = 3.0;
        let expected = 8.0 + 18.0 * headings + 24.0 * buttons;
        assert_eq!(popup_height(&rows), expected);
    }
}
