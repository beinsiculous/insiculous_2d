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
use crate::inspector::InspectorStyle;
use crate::row_layout::{color_block_height, field_row, scrub_step, RowLayout};

/// One inspector render pass: the UI context, the two styles and the
/// content column every component block shares. The host builds it once
/// per frame; the registry-generated editors thread it through.
pub struct InspectorFrame<'a> {
    pub ui: &'a mut UIContext,
    pub inspect_style: &'a InspectorStyle,
    pub field_style: &'a EditableFieldStyle,
    /// Left edge of the content column.
    pub x: f32,
    /// Width of the content column.
    pub width: f32,
    /// Vertical gap before each component block.
    pub section_gap: f32,
    /// Draw every row in its read-only form: a play session is running and
    /// an edit made now would mutate the live world outside the history.
    pub read_only: bool,
}

/// The disclosure markers a header and an Advanced row carry: a section
/// that is open points down, a collapsed one points at its own name.
const OPEN_MARKER: &str = "\u{25be}";
const CLOSED_MARKER: &str = "\u{25b8}";

/// Horizontal space at a header's right edge left to the remove [X], so a
/// click meant for it never lands on the collapse toggle instead.
const HEADER_REMOVE_ZONE: f32 = 24.0;

/// Fallback content width for inspectors constructed without an explicit
/// panel width (tests, standalone widget demos).
const DEFAULT_INSPECTOR_WIDTH: f32 = 300.0;

pub use crate::field_widgets::{
    component_header, cycle_step, edit_bool, edit_cycle, edit_f32, edit_f32_opts, wrap_degrees,
};
pub(crate) use crate::field_widgets::{draw_field_label, out_of_range_warning};

// Read-only string/u32 displays live in `text_field.rs` (moved for file
// size), re-exported from the crate root as before.
use crate::text_field::{display_string, display_u32};

/// A builder for constructing editable component inspectors.
///
/// This provides a fluent API for building inspectors for specific component types.
pub struct EditableInspector<'a> {
    ui: &'a mut UIContext,
    style: &'a EditableFieldStyle,
    component_index: usize,
    field_index: usize,
    current_y: f32,
    x: f32,
    width: f32,
    /// Soft-range warnings raised by this component's fields this frame;
    /// drained by the registry block into `InspectorExtras`.
    warnings: Vec<String>,
    scroll_target: Option<&'static str>,
    scroll_target_y: Option<f32>,
    read_only: bool,
    collapsed: bool,
    advanced_open: bool,
    /// The field index this component's colour editor is open on.
    color_editor_field: Option<usize>,
    /// The colour the popup edited to, for that row to write out.
    color_editor_pending: Option<Vec4>,
    toggles: InspectorToggles,
}

/// One colour row as it drew this frame: which field of its component it
/// is, where its swatch landed, and the colour it showed. The colour editor
/// is drawn by a pass of its own, before the panels, so this is how it
/// learns where to hang and what to show.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRowFrame {
    pub field_index: usize,
    pub anchor: Rect,
    pub value: Vec4,
}

/// What this component's header, disclosure and swatch asked for this
/// frame. The registry block drains it into the inspector's view state,
/// which the editor functions themselves know nothing about.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct InspectorToggles {
    /// The header row was clicked: collapse an open section, open a
    /// collapsed one.
    pub header: bool,
    /// The Advanced disclosure row was clicked.
    pub advanced: bool,
    /// A colour row's swatch was clicked: open the editor on it.
    pub open_color_editor: Option<ColorRowFrame>,
    /// The colour row the editor is already open on drew this frame.
    pub color_row_seen: Option<ColorRowFrame>,
}

impl<'a> EditableInspector<'a> {
    /// Create a new editable inspector builder.
    pub fn new(ui: &'a mut UIContext, style: &'a EditableFieldStyle, x: f32, y: f32) -> Self {
        Self {
            ui,
            style,
            component_index: 0,
            field_index: 0,
            current_y: y,
            x,
            width: DEFAULT_INSPECTOR_WIDTH,
            warnings: Vec::new(),
            scroll_target: None,
            scroll_target_y: None,
            read_only: false,
            collapsed: false,
            advanced_open: false,
            color_editor_field: None,
            color_editor_pending: None,
            toggles: InspectorToggles::default(),
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

    /// Set the content width the inspector may occupy (controls clamp to
    /// `x + width` and the remove [X] button right-aligns to it).
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Draw every row in its read-only form (no widget ids, muted colours,
    /// unchanged row heights) — the whole inspector while a play session
    /// runs.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Draw this component's fields collapsed to its header row.
    pub fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Draw this component's Advanced disclosure open.
    pub fn with_advanced_open(mut self, open: bool) -> Self {
        self.advanced_open = open;
        self
    }

    /// Name the colour row the colour editor is open on, and hand it the
    /// edit the popup made before this frame for it to write out.
    pub fn with_color_editor(mut self, field_index: Option<usize>, pending: Option<Vec4>) -> Self {
        self.color_editor_field = field_index;
        self.color_editor_pending = pending;
        self
    }

    /// Take what this component's header, disclosure and swatch asked for.
    pub fn take_toggles(&mut self) -> InspectorToggles {
        std::mem::take(&mut self.toggles)
    }

    /// Whether a row draws nothing of its own: a collapsed section skips
    /// its fields entirely, and a read-only pass replaces each control with
    /// the value it holds.
    fn is_quiet(&self) -> bool {
        self.collapsed || self.read_only
    }

    /// Draw a row's quiet form: nothing at all while collapsed, otherwise
    /// the label and its value, advancing by the height the editable form
    /// would have taken so the panel measures the same either way.
    fn quiet_row(&mut self, label: &str, value: &str, height: f32) {
        if self.collapsed {
            return;
        }
        let layout = self.row();
        crate::read_only_rows::value_row(self.ui, label, value, layout, self.style);
        self.advance(height);
    }

    /// A disclosure for the fields most entities never touch. The row
    /// itself always draws; the closure runs only while it is open.
    pub fn advanced(&mut self, fields: impl FnOnce(&mut Self)) {
        if self.collapsed {
            return;
        }
        let (id, layout) = self.next_field();
        let marker = if self.advanced_open { OPEN_MARKER } else { CLOSED_MARKER };
        crate::read_only_rows::value_row(self.ui, &format!("{marker} Advanced"), "", layout, self.style);
        let row = Rect::new(layout.pos.x, layout.pos.y, self.width, self.style.row_height);
        if self.ui.interact(id, row, true).clicked {
            self.toggles.advanced = true;
        }
        self.advance(self.style.row_height);
        if self.advanced_open {
            fields(self);
        }
    }

    /// Set the target component header to look for during rendering.
    pub fn with_scroll_target(mut self, target: Option<&'static str>) -> Self {
        self.scroll_target = target;
        self
    }

    /// Get the recorded Y position of the target header, if it was rendered.
    pub fn scroll_target_y(&self) -> Option<f32> {
        self.scroll_target_y
    }

    /// Get the current Y position.
    pub fn y(&self) -> f32 {
        self.current_y
    }

    /// Add a component header. The header row is the section's collapse
    /// toggle, in both states — collapsing is a view choice, not an edit.
    pub fn header(&mut self, type_name: &str) {
        if self.scroll_target == Some(type_name) && self.scroll_target_y.is_none() {
            self.scroll_target_y = Some(self.current_y);
        }
        let marker = if self.collapsed { CLOSED_MARKER } else { OPEN_MARKER };
        let header_y = self.current_y;
        self.current_y = component_header(
            self.ui,
            &format!("{marker} {type_name}"),
            self.x,
            header_y,
            self.style,
        );
        let toggle = Rect::new(
            self.x,
            header_y,
            (self.width - HEADER_REMOVE_ZONE).max(0.0),
            self.style.row_height,
        );
        let toggle_id = FieldId::slot(self.component_index, WidgetSlot::Header);
        if self.ui.interact(toggle_id, toggle, true).clicked {
            self.toggles.header = true;
        }
        self.field_index = 0;
    }

    /// Add a component header with an [X] remove button.
    ///
    /// Returns `true` if the remove button was clicked.
    pub fn header_with_remove(&mut self, type_name: &str) -> bool {
        let header_y = self.current_y;
        self.header(type_name);
        if self.read_only {
            return false;
        }
        crate::component_editors::remove_button(
            self.ui,
            self.component_index,
            self.x,
            header_y,
            self.width,
        )
    }

    /// Position of the next field, indented from the inspector origin.
    fn field_pos(&self) -> Vec2 {
        Vec2::new(self.x + self.style.indent, self.current_y)
    }

    /// Layout of the next field row (indented position + panel-bounded span).
    fn row(&self) -> RowLayout {
        field_row(self.field_pos(), self.x, self.width, self.style)
    }

    /// Widget ID and row layout for the next field (subfield 0).
    fn next_field(&mut self) -> (FieldId, RowLayout) {
        (
            FieldId::new(self.component_index, self.field_index, 0),
            self.row(),
        )
    }

    /// Advance the field index and Y coordinate.
    fn advance(&mut self, height: f32) {
        self.field_index += 1;
        self.current_y += height;
    }

    /// Add a texture slot field: shows the texture's display name and acts
    /// as a drag-and-drop target for asset-browser textures.
    pub fn texture(
        &mut self,
        label: &str,
        handle: u32,
        extras: &mut crate::InspectorExtras<'_>,
    ) -> EditResult<u32> {
        if self.is_quiet() {
            let shown = extras
                .texture_display
                .clone()
                .unwrap_or_else(|| format!("#{handle}"));
            self.quiet_row(label, &shown, self.style.row_height);
            return EditResult::Unchanged;
        }
        let (_, layout) = self.next_field();
        let display = extras.texture_display.clone();
        let result = crate::edit_texture_field(
            self.ui,
            label,
            handle,
            extras.drag_drop,
            display.as_deref(),
            layout,
            self.style,
        );
        self.advance(self.style.row_height);
        result
    }

    /// Add an editable f32 field (soft range: scrub/arrows clamp, typing
    /// may exceed).
    pub fn f32(&mut self, label: &str, value: f32, range: RangeInclusive<f32>) -> EditResult<f32> {
        if self.is_quiet() {
            self.quiet_row(label, &crate::read_only_rows::float_text(value, ""), self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let edit = edit_f32(self.ui, id, label, value, range, layout, self.style);
        self.advance(self.style.row_height);
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
        if self.is_quiet() {
            self.quiet_row(label, &crate::read_only_rows::float_text(value, ""), self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let opts = ui::FloatFieldOpts::hard(*range.start(), *range.end())
            .with_step(scrub_step(&range));
        let edit = edit_f32_opts(self.ui, id, label, value, opts, layout, self.style);
        self.advance(self.style.row_height);
        self.route(edit)
    }

    /// Add an editable angle field: stored in radians, displayed and edited
    /// in degrees with a `°` suffix, wrapped to ±180° on commit — the field
    /// can express any rotation (the old ±π hard clamp is gone).
    pub fn angle(&mut self, label: &str, radians: f32) -> EditResult<f32> {
        if self.is_quiet() {
            let degrees = wrap_degrees(radians.to_degrees());
            self.quiet_row(label, &crate::read_only_rows::float_text(degrees, "\u{b0}"), self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
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
            self.style,
        );
        self.advance(self.style.row_height);
        // The wrap makes the soft range meaningless for typed commits (270°
        // lands at −90°), so this row raises no out-of-range warning.
        match edit.result {
            EditResult::Changed(deg) => EditResult::Changed(wrap_degrees(deg).to_radians()),
            EditResult::Unchanged => EditResult::Unchanged,
        }
    }

    /// Add an editable boolean field.
    pub fn bool(&mut self, label: &str, value: bool) -> EditResult<bool> {
        if self.is_quiet() {
            self.quiet_row(label, crate::read_only_rows::bool_text(value), self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let result = edit_bool(self.ui, id, label, value, layout, self.style);
        self.advance(self.style.row_height);
        result
    }

    /// Add an editable Vec2 field.
    pub fn vec2(&mut self, label: &str, value: Vec2, range: RangeInclusive<f32>) -> EditResult<Vec2> {
        if self.is_quiet() {
            self.quiet_row(label, &crate::read_only_rows::vec2_text(value), self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let edit = edit_vec2(self.ui, id, label, value, range, layout, self.style);
        self.advance(self.style.row_height);
        self.route(edit)
    }

    /// Add a read-only u32 display.
    pub fn u32(&mut self, label: &str, value: u32) {
        if self.is_quiet() {
            self.quiet_row(label, &value.to_string(), self.style.row_height);
            return;
        }
        let layout = self.row();
        display_u32(self.ui, label, value, layout, self.style);
        self.advance(self.style.row_height);
    }

    /// Add a read-only string display.
    pub fn string(&mut self, label: &str, value: &str) {
        if self.is_quiet() {
            self.quiet_row(label, value, self.style.row_height);
            return;
        }
        let layout = self.row();
        display_string(self.ui, label, value, layout, self.style);
        self.advance(self.style.row_height);
    }

    /// A compact action button in the control column (e.g. "+ Add Script",
    /// "− Remove param" in the scripts editor). Returns whether it was
    /// clicked this frame.
    pub fn action_button(&mut self, label: &str) -> bool {
        // The row keeps its height so the panel measures the same, but the
        // button is gone: `edit_scripts` is built on this, and a live one
        // would be a mutation path into a running world.
        if self.is_quiet() {
            self.quiet_row(label, "", self.style.row_height);
            return false;
        }
        let (id, layout) = self.next_field();
        let height = self.style.row_height - 4.0;
        let rect = Rect::new(
            layout.control_x,
            layout.pos.y + 2.0,
            (layout.right - layout.control_x).max(60.0),
            height,
        );
        let clicked = self.ui.button(id, label, rect);
        self.advance(self.style.row_height);
        clicked
    }

    /// Whether this inspector draws read-only rows (a play session runs).
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// The field style this inspector draws with (the theme-derived colours).
    pub fn style(&self) -> &EditableFieldStyle {
        self.style
    }

    /// Add an editable string field (free-form text input; commits on
    /// Enter/Tab/click-away, cancels on Escape).
    pub fn string_edit(&mut self, label: &str, value: &str) -> EditResult<String> {
        self.string_edit_colored(label, value, None)
    }

    /// Add an editable string field with an optional label/value color override.
    pub fn string_edit_colored(
        &mut self,
        label: &str,
        value: &str,
        color: Option<ui::Color>,
    ) -> EditResult<String> {
        if self.is_quiet() {
            self.quiet_row(label, value, self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let result = if let Some(c) = color {
            let mut custom_style = self.style.clone();
            custom_style.label_color = c;
            custom_style.value_color = c;
            crate::text_field::edit_string(self.ui, id, label, value, layout, &custom_style)
        } else {
            crate::text_field::edit_string(self.ui, id, label, value, layout, self.style)
        };
        self.advance(self.style.row_height);
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
        if self.is_quiet() {
            self.quiet_row(label, value_name, self.style.row_height);
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let row = crate::field_widgets::CycleRow { label, value_name, index, count };
        let result = edit_cycle(self.ui, id, row, layout, self.style);
        self.advance(self.style.row_height);
        result
    }


    /// Add an editable color (Vec4) field.
    pub fn color(&mut self, label: &str, value: Vec4) -> EditResult<Vec4> {
        if self.is_quiet() {
            self.quiet_row(label, &crate::read_only_rows::color_text(value), color_block_height(self.style));
            return EditResult::Unchanged;
        }
        let (id, layout) = self.next_field();
        let row = edit_color(self.ui, id, label, value, layout, self.style);
        let frame = ColorRowFrame { field_index: self.field_index, anchor: row.swatch, value };
        if row.swatch_clicked {
            self.toggles.open_color_editor = Some(frame);
        }
        // The popup is drawn by a pass of its own, before any panel: it
        // reports where the row is and takes the edit back through it, so
        // the write is this row's and lands in one undo entry per gesture.
        let is_target = self.color_editor_field == Some(self.field_index);
        if is_target {
            self.toggles.color_row_seen = Some(frame);
        }
        self.advance(color_block_height(self.style));
        match self.color_editor_pending.filter(|_| is_target) {
            Some(edited) => EditResult::Changed(edited),
            None => row.result,
        }
    }
}

