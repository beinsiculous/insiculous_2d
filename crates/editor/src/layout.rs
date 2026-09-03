//! Editor layout constants for consistent spacing.
//!
//! Use these constants instead of magic numbers throughout the editor.
//! Changing a value here updates the entire editor's layout.

/// Standard padding inside panels and containers
pub const PADDING: f32 = 8.0;

/// Panel header height
pub const HEADER_HEIGHT: f32 = 24.0;

/// Default line height (fallback when no font metrics available)
pub const LINE_HEIGHT: f32 = 20.0;

/// Resize handle hit area size
pub const RESIZE_HANDLE_SIZE: f32 = 4.0;

/// Minimum panel size when resizing
pub const MIN_PANEL_SIZE: f32 = 100.0;

/// Default panel width for left/right docked panels
pub const DEFAULT_PANEL_WIDTH: f32 = 250.0;

// Compile-time sanity checks on layout constants.
const _: () = {
    assert!(PADDING > 0.0);
    assert!(HEADER_HEIGHT > 0.0);
    assert!(LINE_HEIGHT > 0.0);
    assert!(MIN_PANEL_SIZE > 0.0);
};
