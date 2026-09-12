//! The inspector's standalone field widgets: the label column, the float,
//! boolean and cycle rows, and the component header.
//!
//! They are pure `UIContext` code taking a resolved [`RowLayout`], with no
//! knowledge of the walk that places them — [`crate::EditableInspector`] is
//! that walk. Composite Vec2/colour rows live in [`crate::composite_rows`],
//! and string rows in [`crate::text_field`].

use std::ops::RangeInclusive;

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::field_style::{EditResult, EditableFieldStyle, FieldEdit, FieldId};
use crate::row_layout::{scrub_step, RowLayout};

/// Gap kept between the end of a label and its control column.
/// A label sits tighter to the field it names than two controls sit to
/// each other, so this is not the shared [`crate::layout::GAP`].
const LABEL_GAP: f32 = 6.0;

/// Draw a field label at the row position, ellipsized so it can never run
/// under the control that starts at `control_x`.
pub(crate) fn draw_field_label(
    ui: &mut UIContext,
    label: &str,
    layout: &RowLayout,
    style: &EditableFieldStyle,
) {
    let pos = layout.pos;
    let budget = (layout.control_x - pos.x - LABEL_GAP).max(0.0);
    let shown =
        crate::row_layout::ellipsize(label, budget, |s| ui.measure_text_styled(s, style.label_font).x);
    ui.label_styled(&shown, Vec2::new(pos.x, pos.y + 4.0), style.label_color, style.label_font);
}

/// Render an editable f32 value with a text input box (drag-scrub,
/// Up/Down nudge, soft-range semantics — see [`ui::FloatFieldOpts`]).
pub fn edit_f32(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: f32,
    range: RangeInclusive<f32>,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> FieldEdit<f32> {
    let opts = ui::FloatFieldOpts::range(*range.start(), *range.end())
        .with_step(scrub_step(&range));
    edit_f32_opts(ui, id, label, value, opts, layout, style)
}

/// The status-bar line for a typed value outside its soft range.
pub(crate) fn out_of_range_warning(label: &str, value: f32, opts: &ui::FloatFieldOpts) -> String {
    format!(
        "{label} = {value:.2}{} is outside the usual {}..{}",
        opts.suffix, opts.min, opts.max
    )
}

/// [`edit_f32`] with explicit float-field options (hard clamp, suffix).
/// A typed commit outside a SOFT range is accepted; the returned
/// [`FieldEdit`] carries the warning for the host to surface.
pub fn edit_f32_opts(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: f32,
    opts: ui::FloatFieldOpts,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> FieldEdit<f32> {
    draw_field_label(ui, label, &layout, style);
    let opts = opts.with_font(opts.font.or(style.numeric_font));

    let input_height = style.field_height();
    let input_bounds = Rect::new(
        layout.control_x,
        layout.pos.y + (style.row_height - input_height) / 2.0,
        layout.clamp_width(style.input_width),
        input_height,
    );

    let result = ui.float_input(id, value, opts, input_bounds);
    let warnings = if result.out_of_range {
        vec![out_of_range_warning(label, result.value, &opts)]
    } else {
        Vec::new()
    };
    let result = if result.changed {
        EditResult::Changed(result.value)
    } else {
        EditResult::Unchanged
    };
    FieldEdit { result, warnings }
}

/// Wrap a degree value into `-180.0..180.0` (720° → 0°, 190° → −170°).
pub fn wrap_degrees(deg: f32) -> f32 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}

/// Render an editable boolean value with a checkbox.
pub fn edit_bool(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: bool,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<bool> {
    draw_field_label(ui, label, &layout, style);

    let checkbox_bounds = Rect::new(
        layout.control_x,
        layout.pos.y + (style.row_height - style.checkbox_size) / 2.0,
        style.checkbox_size,
        style.checkbox_size,
    );

    // Render checkbox and check if toggled
    let toggled = ui.checkbox(id, value, checkbox_bounds);

    if toggled {
        EditResult::Changed(!value)
    } else {
        EditResult::Unchanged
    }
}

/// Step an index forward or backward through `count` values, wrapping at
/// the ends. Pure helper behind [`EditableInspector::cycle`].
pub fn cycle_step(index: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        return 0;
    }
    if forward {
        (index + 1) % count
    } else {
        (index + count - 1) % count
    }
}

/// Calculate the Y position after rendering a component section header.
pub fn component_header(
    ui: &mut UIContext,
    type_name: &str,
    x: f32,
    y: f32,
    style: &EditableFieldStyle,
) -> f32 {
    ui.label_styled(type_name, glam::Vec2::new(x, y), style.header_color, style.header_font);
    y + style.row_height + 4.0
}


/// One cycle row's content: the field's label and the named value it sits
/// on, with that value's position in the variant list.
#[derive(Debug, Clone, Copy)]
pub struct CycleRow<'a> {
    pub label: &'a str,
    pub value_name: &'a str,
    pub index: usize,
    pub count: usize,
}

/// Render a cycle selector row: `label  [<] value [>]` for choosing among
/// `count` named values (an enum's variants, where no dropdown exists).
/// Returns `Changed(new_index)` when an arrow is clicked, wrapping within
/// `count`.
pub fn edit_cycle(
    ui: &mut UIContext,
    id: FieldId,
    row: CycleRow<'_>,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<usize> {
    let CycleRow { label, value_name, index, count } = row;
        let prev_id = id;
        let pos = layout.pos;
        let value_color = style.value_color;
        let row_height = style.row_height;
        let label_font = style.label_font;
        draw_field_label(ui, label, &layout, style);

        let button_size = row_height - 6.0;
        let button_y = pos.y + (row_height - button_size) / 2.0;
        let prev_x = layout.control_x;
        // Value span between the arrows, bounded by the panel's right edge.
        let value_width =
            (layout.right - prev_x - 2.0 * button_size - LABEL_GAP).clamp(60.0, 120.0);

        let prev_bounds = Rect::new(prev_x, button_y, button_size, button_size);
        let prev_clicked = ui.button(prev_id, "<", prev_bounds);

        let shown_value = crate::row_layout::ellipsize(value_name, value_width, |s| {
            ui.measure_text_styled(s, label_font).x
        });
        ui.label_styled(
            &shown_value,
            glam::Vec2::new(prev_x + button_size + LABEL_GAP, pos.y + 4.0),
            value_color,
            label_font,
        );

        let next_x = (prev_x + button_size + value_width).min(layout.right - button_size);
        let next_bounds = Rect::new(next_x, button_y, button_size, button_size);
        let next_clicked = ui.button(
            FieldId::new(id.component_index, id.field_index, 1),
            ">",
            next_bounds,
        );

    if prev_clicked || next_clicked {
        EditResult::Changed(cycle_step(index, count, next_clicked))
    } else {
        EditResult::Unchanged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_field_wraps_degrees_to_a_half_turn_and_round_trips_radians() {
        // The rotation field shows degrees in (-180, 180]: a typed 270 reads
        // back as -90, and a stored rotation survives display → edit →
        // commit within float precision.
        let wraps = [(0.0, 0.0), (190.0, -170.0), (-190.0, 170.0), (720.0, 0.0), (270.0, -90.0)];
        for (typed, shown) in wraps {
            assert_eq!(wrap_degrees(typed), shown, "{typed}° must show as {shown}°");
        }
        for degrees in [-179.0_f32, -90.0, 0.0, 45.0, 179.0] {
            let radians = degrees.to_radians();
            let round_tripped = wrap_degrees(radians.to_degrees()).to_radians();
            assert!((round_tripped - radians).abs() < 1e-5, "{degrees}° drifted to {round_tripped} rad");
        }
    }

    #[test]
    fn test_cycle_step_wraps_both_directions_and_survives_zero_variants() {
        assert_eq!(cycle_step(0, 7, true), 1);
        assert_eq!(cycle_step(6, 7, true), 0, "forward wraps to the first variant");
        assert_eq!(cycle_step(0, 7, false), 6, "backward wraps to the last variant");
        assert_eq!(cycle_step(3, 7, false), 2);
        // An empty variant list must not underflow `count - 1`.
        assert_eq!(cycle_step(5, 0, true), 0);
        assert_eq!(cycle_step(5, 0, false), 0);
    }
}
