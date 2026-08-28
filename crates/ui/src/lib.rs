//! Immediate-mode UI framework for the insiculous_2d game engine.
//!
//! This crate provides a lightweight, immediate-mode UI system for creating
//! in-game user interfaces. It follows the immediate-mode paradigm where you
//! describe the UI every frame rather than retaining UI state.
//!
//! # Features
//! - Immediate-mode API for simplicity
//! - Common widgets: buttons, labels, sliders, checkboxes, progress bars
//! - Customizable themes (dark and light included)
//! - Efficient draw command batching
//! - Mouse interaction with hover, click, and drag support
//!
//! # Example
//! ```
//! use ui::{UIContext, Rect};
//! use glam::Vec2;
//! # use input::InputHandler;
//!
//! let mut ui = UIContext::new();
//! # let input = InputHandler::new();
//!
//! // Each frame (the engine passes its InputHandler and window size):
//! ui.begin_frame(&input, Vec2::new(800.0, 600.0));
//!
//! // Create UI elements
//! ui.panel(Rect::new(10.0, 10.0, 200.0, 100.0));
//! ui.label("Score: 100", Vec2::new(20.0, 30.0));
//!
//! if ui.button("play_btn", "Play", Rect::new(20.0, 60.0, 80.0, 30.0)) {
//!     // Handle button click
//! }
//!
//! ui.end_frame();
//! ```
//!
//! # Rendering Integration
//! The UI system generates draw commands that need to be converted to sprites
//! for rendering. See the engine_core integration for how this is done.

mod context;
mod draw;
mod font;
mod input_state;
mod interaction;
mod style;
mod text_edit;

// Re-export main types
pub use context::{TextAlign, UIContext};
pub use draw::{DrawCommand, DrawList, GlyphDrawData, TextDrawData, UiLayer};
pub use font::{FontError, FontHandle, FontManager, FontMetrics, GlyphInfo, LayoutGlyph, RasterizedGlyph, TextLayout};
pub use input_state::{InputState, KeyRepeat, REPEAT_DELAY, REPEAT_INTERVAL};
pub use interaction::{
    InteractionManager, InteractionResult, WidgetId, WidgetPersistentState, WidgetState,
};
pub use text_edit::TextEditState;
pub use common::Rect;
pub use style::{ButtonStyle, Color, PanelStyle, SliderStyle, TextInputStyle, TextStyle, Theme};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{
        Color, DrawCommand, DrawList, FontHandle, FontManager, Rect, Theme, UIContext, WidgetId, WidgetState,
    };
}
