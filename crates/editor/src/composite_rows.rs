//! Composite inspector rows: Vec2 (X/Y pair) and color (RGBA grid).
//!
//! Moved from `editable_inspector.rs` (file size) and rebuilt as single
//! composite widgets: axis/channel badges are measured, slot positions come
//! from [`crate::row_layout::pair_slots`], and every control stays inside
//! the panel's right edge instead of trusting fixed gaps.

use std::ops::RangeInclusive;

use glam::{Vec2, Vec4};
use ui::{Color, Rect, UIContext};

use crate::field_style::{EditResult, EditableFieldStyle, FieldId};
use crate::row_layout::{pair_slots, PairSlot, RowLayout};

/// Render an editable Vec2 value as one composite row:
/// `label | X [input] Y [input]`, right-bounded by the panel edge.
pub fn edit_vec2(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: Vec2,
    range: RangeInclusive<f32>,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<Vec2> {
    let (min, max) = (*range.start(), *range.end());
    let pos = layout.pos;
    crate::editable_inspector::draw_field_label(ui, label, &layout, style);

    let badge_w = [
        ui.measure_text_styled("X", style.axis_font).x,
        ui.measure_text_styled("Y", style.axis_font).x,
    ];
    let slots = pair_slots(&layout, badge_w, style.input_gap, style.vec2_input_width);

    let input_height = style.row_height - 4.0;
    let input_y = pos.y + (style.row_height - input_height) / 2.0;

    let mut new_value = value;
    for (axis, (slot, badge, color)) in [
        (slots[0], "X", style.axis_x_label),
        (slots[1], "Y", style.axis_y_label),
    ]
    .into_iter()
    .enumerate()
    {
        ui.label_styled(badge, Vec2::new(slot.badge_x, pos.y + 4.0), color, style.axis_font);
        let bounds = Rect::new(slot.input_x, input_y, slot.input_width, input_height);
        let axis_value = if axis == 0 { value.x } else { value.y };
        let edited = ui.float_input(
            FieldId::new(id.component_index, id.field_index, axis),
            axis_value,
            min,
            max,
            bounds,
        );
        if axis == 0 {
            new_value.x = edited;
        } else {
            new_value.y = edited;
        }
    }

    if new_value != value {
        EditResult::Changed(new_value)
    } else {
        EditResult::Unchanged
    }
}

/// Render an editable color (Vec4) as a preview swatch plus a 2×2 channel
/// grid (R/G over B/A) whose columns share x positions so the grid aligns.
pub fn edit_color(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: Vec4,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<Vec4> {
    let y = layout.pos.y;
    crate::editable_inspector::draw_field_label(ui, label, &layout, style);

    // Color preview swatch at the control column.
    let preview_bounds = Rect::new(
        layout.control_x,
        y + (style.row_height - style.color_preview_size) / 2.0,
        style.color_preview_size,
        style.color_preview_size,
    );
    ui.rect_rounded(preview_bounds, Color::new(value.x, value.y, value.z, value.w), 2.0);

    // The channel grid occupies the span right of the preview. Column badge
    // widths are the max of the two badges sharing that column, so R/B and
    // G/A line up.
    let grid = RowLayout {
        pos: layout.pos,
        control_x: layout.control_x + style.color_preview_size + style.input_gap,
        right: layout.right,
    };
    let col_badge_w = [
        ui.measure_text_styled("R", style.channel_font)
            .x
            .max(ui.measure_text_styled("B", style.channel_font).x),
        ui.measure_text_styled("G", style.channel_font)
            .x
            .max(ui.measure_text_styled("A", style.channel_font).x),
    ];
    let cols = pair_slots(&grid, col_badge_w, style.color_input_gap, style.color_input_width);

    let row_ys = [y + 2.0, y + 2.0 + style.color_input_height + 4.0];
    let channels: [(&str, f32, usize, PairSlot, f32); 4] = [
        ("R", value.x, 0, cols[0], row_ys[0]),
        ("G", value.y, 1, cols[1], row_ys[0]),
        ("B", value.z, 2, cols[0], row_ys[1]),
        ("A", value.w, 3, cols[1], row_ys[1]),
    ];

    let mut new_value = value;
    let mut changed = false;
    for (badge, channel_value, subfield, slot, row_y) in channels {
        ui.label_styled(
            badge,
            Vec2::new(slot.badge_x, row_y),
            style.channel_labels[subfield],
            style.channel_font,
        );
        let bounds = Rect::new(slot.input_x, row_y, slot.input_width, style.color_input_height);
        let edited = ui.float_input(
            FieldId::new(id.component_index, id.field_index, subfield),
            channel_value,
            0.0,
            1.0,
            bounds,
        );
        if (edited - channel_value).abs() > f32::EPSILON {
            match subfield {
                0 => new_value.x = edited,
                1 => new_value.y = edited,
                2 => new_value.z = edited,
                _ => new_value.w = edited,
            }
            changed = true;
        }
    }

    if changed {
        EditResult::Changed(new_value)
    } else {
        EditResult::Unchanged
    }
}
