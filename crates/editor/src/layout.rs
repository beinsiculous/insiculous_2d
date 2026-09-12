//! Editor layout constants for consistent spacing.
//!
//! Use these constants instead of magic numbers throughout the editor.
//! Changing a value here updates the entire editor's layout.

/// Standard padding inside panels and containers
pub const PADDING: f32 = 8.0;

/// Height of the scene view's toolbar strip: the band at the top of the
/// scene panel that holds the tools and the play controls.
pub const TOOLBAR_STRIP_HEIGHT: f32 = 40.0;

/// Panel header height
pub const HEADER_HEIGHT: f32 = 24.0;

/// Default line height (fallback when no font metrics available)
pub const LINE_HEIGHT: f32 = 20.0;

/// Height of one row of a menu, a list or a popup — and the inspector's
/// field-row height, which is the same measure. Anything that stacks rows
/// uses this or states in its own words why it is taller or tighter.
pub const ROW_HEIGHT: f32 = 24.0;

/// Height of a field inside a row: the row less the breathing space above
/// and below it. Only the popups that carry no `EditableFieldStyle` read it
/// directly; an inspector row's field takes its height from the style,
/// because a second style may size its rows differently.
pub const FIELD_HEIGHT: f32 = ROW_HEIGHT - 4.0;

/// Height of a button in a dialog or a popup. The strip's buttons are band
/// buttons and size themselves.
pub const BUTTON_HEIGHT: f32 = 26.0;

/// Gap between adjacent controls.
pub const GAP: f32 = 4.0;

/// Resize handle hit area size
pub const RESIZE_HANDLE_SIZE: f32 = 4.0;

/// Minimum panel size when resizing
pub const MIN_PANEL_SIZE: f32 = 100.0;

/// Default panel width for left/right docked panels
pub const DEFAULT_PANEL_WIDTH: f32 = 250.0;

// Compile-time sanity checks on layout constants.
const _: () = {
    assert!(PADDING > 0.0);
    assert!(TOOLBAR_STRIP_HEIGHT > HEADER_HEIGHT);
    assert!(HEADER_HEIGHT > 0.0);
    assert!(LINE_HEIGHT > 0.0);
    assert!(MIN_PANEL_SIZE > 0.0);
    assert!(FIELD_HEIGHT < ROW_HEIGHT);
    assert!(GAP > 0.0);
};
