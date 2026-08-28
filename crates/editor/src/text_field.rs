//! String field widgets for the editable inspector: an editable text input
//! plus the read-only string/u32 displays (moved from `editable_inspector.rs`
//! for file size).

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::editable_inspector::draw_field_label;
use crate::field_style::{EditResult, EditableFieldStyle, FieldId};
use crate::row_layout::RowLayout;

/// Render an editable string field (label + free-form text input).
///
/// Commits on Enter/Tab/click-away, cancels on Escape — the semantics of
/// `UIContext::text_input`. Returns `Changed` only when the committed text
/// differs from the current value.
pub fn edit_string(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: &str,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<String> {
    draw_field_label(ui, label, &layout, style);

    // Text input bounds — wider than numeric inputs; strings are longer.
    let input_height = style.row_height - 4.0;
    let input_bounds = Rect::new(
        layout.control_x,
        layout.pos.y + (style.row_height - input_height) / 2.0,
        layout.clamp_width(style.input_width * 1.6),
        input_height,
    );

    match ui.text_input(id, value, input_bounds) {
        Some(new_value) if new_value != value => EditResult::Changed(new_value),
        _ => EditResult::Unchanged,
    }
}

/// Render a read-only u32 value (for asset handles, etc.).
pub fn display_u32(
    ui: &mut UIContext,
    label: &str,
    value: u32,
    layout: RowLayout,
    style: &EditableFieldStyle,
) {
    display_string(ui, label, &format!("{}", value), layout, style);
}

/// Render a read-only string value (for tags, target names, etc.), the value
/// ellipsized at the panel's right edge.
pub fn display_string(
    ui: &mut UIContext,
    label: &str,
    value: &str,
    layout: RowLayout,
    style: &EditableFieldStyle,
) {
    draw_field_label(ui, label, &layout, style);
    let value_budget = layout.available();
    let shown = crate::row_layout::ellipsize(value, value_budget, |s| {
        ui.measure_text_styled(s, style.label_font).x
    });
    ui.label_styled(
        &shown,
        Vec2::new(layout.control_x, layout.pos.y + 4.0),
        style.value_color,
        style.label_font,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_string_without_interaction_is_unchanged() {
        let mut ui = UIContext::new();
        ui.begin_frame(&input::InputHandler::new(), Vec2::new(800.0, 600.0));
        let style = EditableFieldStyle::default();
        let layout = crate::row_layout::field_row(Vec2::new(10.0, 10.0), 10.0, 400.0, &style);
        let result = edit_string(
            &mut ui,
            FieldId::new(0, 0, 0),
            "Text",
            "hello",
            layout,
            &style,
        );
        ui.end_frame();
        assert!(!result.is_changed());
    }
}
