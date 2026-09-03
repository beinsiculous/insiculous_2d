//! Field identity and styling shared by the editable inspector widgets.
//!
//! `FieldId` maps inspector fields to stable widget IDs, `EditableFieldStyle`
//! centralizes all layout dimensions and colors (themed via
//! `EditorTheme::editable_field_style()`), and `EditResult<T>` reports whether
//! a single field changed this frame.

use ui::Color;

/// Widget-ID stride between components (a component may use up to this many field IDs).
const COMPONENT_ID_STRIDE: u64 = 10_000;

/// Widget-ID stride between fields (a field may use up to this many subfield IDs).
const FIELD_ID_STRIDE: u64 = 100;

/// Unique identifier for inspector fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId {
    pub(crate) component_index: usize,
    pub(crate) field_index: usize,
    pub(crate) subfield_index: usize,
}

impl FieldId {
    /// Create a new field ID.
    pub fn new(component_index: usize, field_index: usize, subfield_index: usize) -> Self {
        Self {
            component_index,
            field_index,
            subfield_index,
        }
    }
}

impl From<FieldId> for ui::WidgetId {
    fn from(id: FieldId) -> Self {
        // Create a unique widget ID from the field indices
        let id_value = (id.component_index as u64) * COMPONENT_ID_STRIDE
            + (id.field_index as u64) * FIELD_ID_STRIDE
            + id.subfield_index as u64;
        ui::WidgetId::new(id_value)
    }
}

/// Configuration for editable field display.
#[derive(Debug, Clone)]
pub struct EditableFieldStyle {
    /// Height of each field row
    pub row_height: f32,
    /// Width of the label column
    pub label_width: f32,
    /// Padding between elements
    pub padding: f32,
    /// Checkbox size
    pub checkbox_size: f32,
    /// Color preview size
    pub color_preview_size: f32,
    /// Indentation for nested fields
    pub indent: f32,
    /// Width of a single-value text input (f32 fields)
    pub input_width: f32,
    /// Maximum width of each text input in a Vec2 pair (pairs shrink to fit
    /// the panel; this caps how wide they grow)
    pub vec2_input_width: f32,
    /// Horizontal gap between adjacent inputs in a multi-input row
    pub input_gap: f32,
    /// Maximum width of each RGBA channel text input (channels shrink to fit)
    pub color_input_width: f32,
    /// Height of each RGBA channel text input
    pub color_input_height: f32,
    /// Horizontal gap between adjacent RGBA channel inputs
    pub color_input_gap: f32,
    /// Label color
    pub label_color: Color,
    /// Value color
    pub value_color: Color,
    /// Header color for component names
    pub header_color: Color,
    /// "X" axis label color in Vec2 fields
    pub axis_x_label: Color,
    /// "Y" axis label color in Vec2 fields
    pub axis_y_label: Color,
    /// "R", "G", "B", "A" channel label colors in color fields
    pub channel_labels: [Color; 4],
    /// Background of asset slot fields (texture references)
    pub slot_bg: Color,
    /// Border highlight while a compatible drag hovers a drop target
    pub drop_highlight: Color,
    /// Font size for field name labels and values
    pub label_font: f32,
    /// Font size for component headers/section titles
    pub header_font: f32,
    /// Font size for "X"/"Y" axis labels in Vec2 fields
    pub axis_font: f32,
    /// Font size for "R"/"G"/"B"/"A" channel labels in color fields
    pub channel_font: f32,
    /// Face for numeric inputs (f32, Vec2 axes, RGBA channels) — the editor
    /// hands in its monospace handle so digits line up and a scrub never
    /// jitters the caret. `None` = the default font.
    pub numeric_font: Option<ui::FontHandle>,
}

impl EditableFieldStyle {
    /// Draw numeric inputs in `font` (`None` keeps the default font).
    pub fn with_numeric_font(mut self, font: Option<ui::FontHandle>) -> Self {
        self.numeric_font = font;
        self
    }
}

impl Default for EditableFieldStyle {
    fn default() -> Self {
        Self {
            row_height: 24.0,
            label_width: 120.0,
            padding: 8.0,
            checkbox_size: 16.0,
            color_preview_size: 20.0,
            indent: 16.0,
            input_width: 100.0,
            vec2_input_width: 70.0,
            input_gap: 8.0,
            color_input_width: 48.0,
            color_input_height: 16.0,
            color_input_gap: 4.0,
            label_color: Color::new(0.7, 0.7, 0.7, 1.0),
            value_color: Color::new(1.0, 1.0, 1.0, 1.0),
            header_color: Color::new(0.9, 0.9, 0.5, 1.0),
            axis_x_label: Color::new(0.8, 0.4, 0.4, 1.0),
            axis_y_label: Color::new(0.4, 0.8, 0.4, 1.0),
            channel_labels: [
                Color::new(0.9, 0.4, 0.4, 1.0), // R
                Color::new(0.4, 0.9, 0.4, 1.0), // G
                Color::new(0.4, 0.4, 0.9, 1.0), // B
                Color::new(0.7, 0.7, 0.7, 1.0), // A
            ],
            slot_bg: Color::new(0.18, 0.18, 0.18, 1.0),
            drop_highlight: Color::new(0.0, 0.47, 0.83, 1.0),
            label_font: 14.0,
            header_font: 16.0,
            axis_font: 12.0,
            channel_font: 12.0,
            numeric_font: None,
        }
    }
}

/// A field renderer's report: the edit result plus any warnings the host
/// should surface (a typed value outside its soft range). Renderers
/// stay pure widget code; [`crate::EditableInspector`] routes the warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEdit<T> {
    pub result: EditResult<T>,
    pub warnings: Vec<String>,
}

/// Result of editing a field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditResult<T> {
    /// Value was not changed.
    Unchanged,
    /// Value was modified to a new value.
    Changed(T),
}

impl<T> EditResult<T> {
    /// Check if the value was changed.
    pub fn is_changed(&self) -> bool {
        matches!(self, EditResult::Changed(_))
    }

    /// Get the new value if changed, otherwise None.
    pub fn new_value(&self) -> Option<&T> {
        match self {
            EditResult::Changed(v) => Some(v),
            EditResult::Unchanged => None,
        }
    }

    /// Unwrap the value, returning the new value if changed, or the original.
    pub fn unwrap_or(self, original: T) -> T {
        match self {
            EditResult::Changed(v) => v,
            EditResult::Unchanged => original,
        }
    }
}
