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
pub use crate::field_style::{EditResult, EditableFieldStyle, FieldId};
use crate::row_layout::{color_block_height, field_row, remove_button_x, RowLayout};

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

/// Render an editable f32 value with a text input box.
pub fn edit_f32(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: f32,
    range: RangeInclusive<f32>,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<f32> {
    draw_field_label(ui, label, &layout, style);

    let input_height = style.row_height - 4.0;
    let input_bounds = Rect::new(
        layout.control_x,
        layout.pos.y + (style.row_height - input_height) / 2.0,
        layout.clamp_width(style.input_width),
        input_height,
    );

    let new_value = ui.float_input(id, value, *range.start(), *range.end(), input_bounds);

    if (new_value - value).abs() > f32::EPSILON {
        EditResult::Changed(new_value)
    } else {
        EditResult::Unchanged
    }
}

/// Render an editable f32 value clamped to a 0-1 range.
/// Useful for normalized values like volume, friction, restitution.
pub fn edit_normalized_f32(
    ui: &mut UIContext,
    id: FieldId,
    label: &str,
    value: f32,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<f32> {
    let clamped = value.clamp(0.0, 1.0);
    edit_f32(ui, id, label, clamped, 0.0..=1.0, layout, style)
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
        }
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
            let btn_size = 18.0;
            let btn_x = remove_button_x(self.x, self.width, btn_size);
            let btn_bounds = Rect::new(btn_x, self.current_y, btn_size, btn_size);

            // Use component_index + 99 to avoid ID collisions with field inputs
            let btn_id = FieldId::new(self.component_index, 99, 0);
            clicked = self.ui.button(btn_id, "X", btn_bounds);
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

    /// Add an editable f32 field.
    pub fn f32(&mut self, label: &str, value: f32, range: RangeInclusive<f32>) -> EditResult<f32> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let result = edit_f32(self.ui, id, label, value, range, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        result
    }

    /// Add an editable normalized f32 field (0-1 range).
    pub fn normalized_f32(&mut self, label: &str, value: f32) -> EditResult<f32> {
        let id = FieldId::new(self.component_index, self.field_index, 0);
        let layout = self.row();
        let result = edit_normalized_f32(self.ui, id, label, value, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        result
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
        let result = edit_vec2(self.ui, id, label, value, range, layout, &self.style);
        self.field_index += 1;
        self.current_y += self.style.row_height;
        result
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
    fn test_edit_result_unchanged() {
        let result: EditResult<f32> = EditResult::Unchanged;
        assert!(!result.is_changed());
        assert_eq!(result.new_value(), None);
        assert_eq!(result.unwrap_or(5.0), 5.0);
    }

    #[test]
    fn test_edit_result_changed() {
        let result = EditResult::Changed(10.0);
        assert!(result.is_changed());
        assert_eq!(result.new_value(), Some(&10.0));
        assert_eq!(result.unwrap_or(5.0), 10.0);
    }

    #[test]
    fn test_field_id_creation() {
        let id = FieldId::new(1, 2, 3);
        let _widget_id: ui::WidgetId = id.into();
        // WidgetId is created successfully (can't verify internal value without accessor)
    }

    #[test]
    fn test_editable_field_style_default() {
        let style = EditableFieldStyle::default();
        assert_eq!(style.row_height, 24.0);
        assert_eq!(style.label_width, 120.0);
        assert_eq!(style.padding, 8.0);
    }

    #[test]
    fn test_cycle_step_wraps_both_directions() {
        assert_eq!(cycle_step(0, 7, true), 1);
        assert_eq!(cycle_step(6, 7, true), 0); // wraps forward
        assert_eq!(cycle_step(0, 7, false), 6); // wraps backward
        assert_eq!(cycle_step(3, 7, false), 2);
    }

    #[test]
    fn test_cycle_step_zero_count_is_safe() {
        assert_eq!(cycle_step(5, 0, true), 0);
        assert_eq!(cycle_step(5, 0, false), 0);
    }

    #[test]
    fn test_editable_inspector_builder() {
        // Just verify the builder pattern compiles and initializes correctly
        // Actual rendering requires a UIContext which needs rendering infrastructure
        let style = EditableFieldStyle::default();
        assert_eq!(style.row_height, 24.0);
    }
}
