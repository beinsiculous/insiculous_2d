//! Editable component inspector with field modification support.
//!
//! This module provides editable UI widgets for modifying component
//! properties directly in the editor. Supports all common component
//! field types used in Transform2D, Sprite, RigidBody, Collider, and AudioSource.
//! Field identity and styling types live in [`crate::field_style`]; horizontal
//! placement is computed by the pure [`crate::row_layout`] module so controls
//! track the panel's actual width; Vec2/color composite rows live in
//! [`crate::composite_rows`].

use std::ops::RangeInclusive;

use glam::{Vec2, Vec4};
use ui::{Rect, UIContext};

pub use crate::composite_rows::{edit_color, edit_vec2};
pub use crate::field_style::{EditResult, EditableFieldStyle, FieldEdit, FieldId, WidgetSlot};
use crate::row_layout::{color_block_height, field_row, remove_button_x, scrub_step, RowLayout};

/// Fallback content width for inspectors constructed without an explicit
/// panel width (tests, standalone widget demos).
const DEFAULT_INSPECTOR_WIDTH: f32 = 300.0;

/// Gap kept between the end of a label and its control column.
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

    let input_height = style.row_height - 4.0;
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

// Read-only string/u32 displays live in `text_field.rs` (moved for file
// size), re-exported from the crate root as before.
use crate::text_field::{display_string, display_u32};

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

/// A builder for constructing editable component inspectors.
///
/// This provides a fluent API for building inspectors for specific component types.
pub struct EditableInspector<'a> {
    ui: &'a mut UIContext,
    style: EditableFieldStyle,
    component_index: usize,
    field_index: usize,
    current_y: f32,
    x: f32,
    width: f32,
    /// Soft-range warnings raised by this component's fields this frame;
    /// drained by the registry block into `InspectorExtras`.
    warnings: Vec<String>,
}

impl<'a> EditableInspector<'a> {
    /// Create a new editable inspector builder.
    pub fn new(ui: &'a mut UIContext, x: f32, y: f32) -> Self {
        Self {
            ui,
            style: EditableFieldStyle::default(),
            component_index: 0,
            field_index: 0,
            current_y: y,
            x,
            width: DEFAULT_INSPECTOR_WIDTH,
            warnings: Vec::new(),
        }
    }

    /// Take the soft-range warnings raised so far this frame.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Set the component index for field IDs.
    pub fn with_component_index(mut self, index: usize) -> Self {
        self.component_index = index;
        self
    }

    /// Set the style.
    pub fn with_style(mut self, style: EditableFieldStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the content width the inspector may occupy (controls clamp to
    /// `x + width` and the remove [X] button right-aligns to it).
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Get the current Y position.
    pub fn y(&self) -> f32 {
        self.current_y
    }

    /// Add a component header.
    pub fn header(&mut self, type_name: &str) {
        self.current_y = component_header(self.ui, type_name, self.x, self.current_y, &self.style);
        self.field_index = 0;
    }

    /// Add a component header with an optional [X] remove button.
    ///
    /// Returns `true` if the remove button was clicked.
    /// When `removable` is `false`, behaves identically to `header()`.
    pub fn header_with_remove(&mut self, type_name: &str, removable: bool) -> bool {
        // Draw the header label
        self.ui.label_styled(
            type_name,
            glam::Vec2::new(self.x, self.current_y),
            self.style.header_color,
            self.style.header_font,
        );

        let mut clicked = false;

        if removable {
            // Right-align a small [X] button to the panel's content edge.
            let button_size = 18.0;
            let button_x = remove_button_x(self.x, self.width, button_size);
            let button_bounds = Rect::new(button_x, self.current_y, button_size, button_size);

            let button_id = FieldId::slot(self.component_index, WidgetSlot::Remove);
            clicked = self.ui.button(button_id, "X", button_bounds);
        }

        self.current_y += self.style.row_height + 4.0;
        self.field_index = 0;
        clicked
    }

    /// Position of the next field, indented from the inspector origin.
    fn field_pos(&self) -> Vec2 {
        Vec2::new(self.x + self.style.indent, self.current_y)
    }

    /// Layout of the next field row (indented position + panel-bounded span).
    fn row(&self) -> RowLayout {
        field_row(self.field_pos(), self.x, self.width, &self.style)
    }

    /// Add a texture slot field: shows the texture's display name and acts
    /// as a drag-and-drop target for asset-browser textures.
    pub fn texture(
        &mut self,
        label: &str,
        handle: u32,
        extras: &mut crate::InspectorExtras<'_>,
    ) -> EditResult<u32> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let display = extras.texture_display.clone();
        let result = crate::edit_texture_field(
            self.ui,
            id,
            label,
            handle,
            extras.drag_drop,
            display.as_deref(),
            layout,
            &self.style,
        );
        self.field_index += 1;
        self.current_y += self.style.row_height;
        result
    }

    /// Add an editable f32 field (soft range: scrub/arrows clamp, typing
    /// may exceed).
    pub fn f32(&mut self, label: &str, value: f32, range: RangeInclusive<f32>) -> EditResult<f32> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let edit = edit_f32(self.ui, id, label, value, range, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        self.route(edit)
    }

    /// Keep a field's warnings for the registry block to drain, hand back
    /// its edit result.
    fn route<T>(&mut self, edit: FieldEdit<T>) -> EditResult<T> {
        self.warnings.extend(edit.warnings);
        edit.result
    }

    /// Add an editable f32 field with a HARD range: typed commits clamp
    /// too. For values where the range is a runtime contract (audio volume
    /// 0..=1, pitch floor), not a convenience.
    pub fn f32_hard(&mut self, label: &str, value: f32, range: RangeInclusive<f32>) -> EditResult<f32> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let opts = ui::FloatFieldOpts::hard(*range.start(), *range.end())
            .with_step(scrub_step(&range));
        let edit = edit_f32_opts(self.ui, id, label, value, opts, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        self.route(edit)
    }

    /// Add an editable angle field: stored in radians, displayed and edited
    /// in degrees with a `°` suffix, wrapped to ±180° on commit — the field
    /// can express any rotation (the old ±π hard clamp is gone).
    pub fn angle(&mut self, label: &str, radians: f32) -> EditResult<f32> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let opts = ui::FloatFieldOpts::range(-180.0, 180.0)
            .with_step(scrub_step(&(-180.0..=180.0)))
            .with_suffix("°");
        // Display wraps too, so the field always operates in the canonical
        // ±180° space — a rotation stored as 270° shows (and scrubs) as
        // −90° instead of sitting outside its own range.
        let edit = edit_f32_opts(
            self.ui,
            id,
            label,
            wrap_degrees(radians.to_degrees()),
            opts,
            layout,
            &self.style,
        );
        self.field_index += 1;
        self.current_y += self.style.row_height;
        // The wrap makes the soft range meaningless for typed commits (270°
        // lands at −90°), so this row raises no out-of-range warning.
        match edit.result {
            EditResult::Changed(deg) => EditResult::Changed(wrap_degrees(deg).to_radians()),
            EditResult::Unchanged => EditResult::Unchanged,
        }
    }

    /// Add an editable boolean field.
    pub fn bool(&mut self, label: &str, value: bool) -> EditResult<bool> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let result = edit_bool(self.ui, id, label, value, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        result
    }

    /// Add an editable Vec2 field.
    pub fn vec2(&mut self, label: &str, value: Vec2, range: RangeInclusive<f32>) -> EditResult<Vec2> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let edit = edit_vec2(self.ui, id, label, value, range, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        self.route(edit)
    }

    /// Add a read-only u32 display.
    pub fn u32(&mut self, label: &str, value: u32) {
        let layout = self.row();
        display_u32(self.ui, label, value, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
    }

    /// Add a read-only string display.
    pub fn string(&mut self, label: &str, value: &str) {
        let layout = self.row();
        display_string(self.ui, label, value, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
    }

    /// A compact action button in the control column (e.g. "+ Add Script",
    /// "− Remove param" in the scripts editor). Returns whether it was
    /// clicked this frame.
    pub fn action_button(&mut self, label: &str) -> bool {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let height = self.style.row_height - 4.0;
        let rect = Rect::new(
            layout.control_x,
            layout.pos.y + 2.0,
            (layout.right - layout.control_x).max(60.0),
            height,
        );
        let clicked = self.ui.button(id, label, rect);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        clicked
    }

    /// Add an editable string field (free-form text input; commits on
    /// Enter/Tab/click-away, cancels on Escape).
    pub fn string_edit(&mut self, label: &str, value: &str) -> EditResult<String> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let result = crate::text_field::edit_string(self.ui, id, label, value, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        result
    }

    /// Add a cycle selector row: `label  [<] value [>]` for choosing among
    /// `count` named values (e.g. enum variants, where a dropdown is not
    /// available).
    ///
    /// Returns `Changed(new_index)` when an arrow button is clicked,
    /// wrapping within `count`.
    pub fn cycle(
        &mut self,
        label: &str,
        value_name: &str,
        index: usize,
        count: usize,
    ) -> EditResult<usize> {
        let layout = self.row();
        let pos = layout.pos;
        let value_color = self.style.value_color;
        let row_height = self.style.row_height;
        let label_font = self.style.label_font;
        let style = self.style.clone();
        draw_field_label(self.ui, label, &layout, &style);

        let btn_size = row_height - 6.0;
        let btn_y = pos.y + (row_height - btn_size) / 2.0;
        let prev_x = layout.control_x;
        // Value span between the arrows, bounded by the panel's right edge.
        let value_width =
            (layout.right - prev_x - 2.0 * btn_size - LABEL_GAP).clamp(60.0, 120.0);

        let prev_bounds = Rect::new(prev_x, btn_y, btn_size, btn_size);
        let prev_clicked = self.ui.button(
            FieldId::new(self.component_index, self.field_index, 0),
            "<",
            prev_bounds,
        );

        let shown_value = crate::row_layout::ellipsize(value_name, value_width, |s| {
            self.ui.measure_text_styled(s, label_font).x
        });
        self.ui.label_styled(
            &shown_value,
            glam::Vec2::new(prev_x + btn_size + LABEL_GAP, pos.y + 4.0),
            value_color,
            label_font,
        );

        let next_x = (prev_x + btn_size + value_width).min(layout.right - btn_size);
        let next_bounds = Rect::new(next_x, btn_y, btn_size, btn_size);
        let next_clicked = self.ui.button(
            FieldId::new(self.component_index, self.field_index, 1),
            ">",
            next_bounds,
        );

        self.field_index += 1;
        self.current_y += self.style.row_height;

        if prev_clicked || next_clicked {
            EditResult::Changed(cycle_step(index, count, next_clicked))
        } else {
            EditResult::Unchanged
        }
    }

    /// Add an editable color (Vec4) field.
    pub fn color(&mut self, label: &str, value: Vec4) -> EditResult<Vec4> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let result = edit_color(self.ui, id, label, value, layout, &self.style);
        self.field_index += 1;
        self.current_y += color_block_height(&self.style);
        result
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
