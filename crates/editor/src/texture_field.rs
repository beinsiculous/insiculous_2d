//! Inspector texture field: shows the sprite's texture by name and accepts
//! drag-and-drop assignment from the asset browser.

use ui::UIContext;

use crate::drag_drop::{DragDropState, DragPayload};
use crate::editable_inspector::draw_field_label;
use crate::field_style::{EditResult, EditableFieldStyle, FieldId};
use crate::row_layout::RowLayout;

/// Context the integration layer threads into the editable inspector for
/// fields that reach beyond one component's data: the drag-drop coordinator
/// and pre-resolved display strings (the editor crate cannot see the
/// engine's `AssetManager`, so path lookups happen upstream).
#[derive(Debug)]
pub struct InspectorExtras<'a> {
    /// Cross-panel drag-and-drop state (drop target queries).
    pub drag_drop: &'a mut DragDropState,
    /// Display path for the selected entity's sprite texture, if resolvable
    /// (e.g. `"player.png"` or `"#white"`).
    pub texture_display: Option<String>,
}

/// Render a texture slot: label + a boxed value showing the texture's path
/// (falling back to the raw handle), acting as a drop target for
/// [`DragPayload::Texture`]. Returns `Changed(handle)` when a drop lands.
#[allow(clippy::too_many_arguments)]
pub fn edit_texture_field(
    ui: &mut UIContext,
    _id: FieldId,
    label: &str,
    handle: u32,
    drag_drop: &mut DragDropState,
    display: Option<&str>,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<u32> {
    draw_field_label(ui, label, &layout, style);

    let slot_bounds = ui::Rect::new(
        layout.control_x,
        layout.pos.y + 2.0,
        layout.clamp_width(style.input_width + 40.0),
        style.row_height - 4.0,
    );

    // Slot box; highlight while a texture drag hovers it
    let dragging_texture = matches!(drag_drop.dragging_payload(), Some(DragPayload::Texture { .. }));
    let hovered = slot_bounds.contains(ui.mouse_pos());
    ui.rect_rounded(slot_bounds, style.slot_bg, 2.0);
    if dragging_texture && hovered {
        ui.rect_border(slot_bounds, style.drop_highlight, 2.0, 2.0);
    }

    let fallback = if handle == 0 { "#white".to_string() } else { format!("handle {handle}") };
    let text = display.unwrap_or(&fallback);
    ui.label_in_bounds_styled(
        text,
        slot_bounds,
        ui::TextAlign::Left,
        style.value_color,
        style.label_font,
        4.0,
    );

    let drop_bounds = common::Rect::new(
        slot_bounds.x,
        slot_bounds.y,
        slot_bounds.width,
        slot_bounds.height,
    );
    if let Some((DragPayload::Texture { handle: new_handle, .. }, _)) =
        drag_drop.take_drop_in(drop_bounds)
    {
        if new_handle != handle {
            return EditResult::Changed(new_handle);
        }
    }
    EditResult::Unchanged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row_layout::field_row;
    use glam::Vec2;

    fn drop_texture_at(drag_drop: &mut DragDropState, handle: u32, pos: Vec2) {
        drag_drop.arm(
            DragPayload::Texture { handle, path: "tex.png".into() },
            Vec2::new(0.0, 0.0),
        );
        drag_drop.begin_frame(Vec2::new(50.0, 50.0), true, false); // past threshold
        drag_drop.begin_frame(pos, false, true); // release = drop
    }

    #[test]
    fn test_drop_inside_slot_changes_handle() {
        let mut ui = UIContext::new();
        ui.begin_frame(&input::InputHandler::new(), Vec2::new(800.0, 600.0));
        let mut drag_drop = DragDropState::new();
        let style = EditableFieldStyle::default();
        let pos = Vec2::new(10.0, 10.0);

        // Slot spans x: 10+label_width .. +input_width+40, y: 12 .. 12+row_height-4
        let slot_center = Vec2::new(10.0 + style.label_width + 20.0, 12.0 + 8.0);
        drop_texture_at(&mut drag_drop, 7, slot_center);

        let layout = field_row(pos, pos.x, 400.0, &style);
        let result = edit_texture_field(
            &mut ui, FieldId::new(0, 0, 0), "Texture", 1,
            &mut drag_drop, Some("old.png"), layout, &style,
        );
        assert_eq!(result, EditResult::Changed(7));
    }

    #[test]
    fn test_drop_outside_slot_is_ignored() {
        let mut ui = UIContext::new();
        ui.begin_frame(&input::InputHandler::new(), Vec2::new(800.0, 600.0));
        let mut drag_drop = DragDropState::new();
        let style = EditableFieldStyle::default();

        drop_texture_at(&mut drag_drop, 7, Vec2::new(700.0, 500.0));

        let layout = field_row(Vec2::new(10.0, 10.0), 10.0, 400.0, &style);
        let result = edit_texture_field(
            &mut ui, FieldId::new(0, 0, 0), "Texture", 1,
            &mut drag_drop, None, layout, &style,
        );
        assert_eq!(result, EditResult::Unchanged);
    }

    #[test]
    fn test_dropping_same_handle_is_unchanged() {
        let mut ui = UIContext::new();
        ui.begin_frame(&input::InputHandler::new(), Vec2::new(800.0, 600.0));
        let mut drag_drop = DragDropState::new();
        let style = EditableFieldStyle::default();
        let slot_center = Vec2::new(10.0 + style.label_width + 20.0, 12.0 + 8.0);

        drop_texture_at(&mut drag_drop, 1, slot_center);

        let layout = field_row(Vec2::new(10.0, 10.0), 10.0, 400.0, &style);
        let result = edit_texture_field(
            &mut ui, FieldId::new(0, 0, 0), "Texture", 1,
            &mut drag_drop, None, layout, &style,
        );
        assert_eq!(result, EditResult::Unchanged, "no-op drop must not dirty the scene");
    }
}
