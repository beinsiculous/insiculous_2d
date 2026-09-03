//! Immediate-mode UI framework for the insiculous_2d game engine.
//!
//! Provides an immediate-mode UI system for creating user interfaces,
//! producing draw commands that the engine flattens into sprites.
//!
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
//! ui.panel(Rect::new(10.0, 10.0, 200.0, 100.0));
//! ui.label("Score: 100", Vec2::new(20.0, 30.0));
//! if ui.button("play_btn", "Play", Rect::new(20.0, 60.0, 80.0, 30.0)) {
//!     // Handle button click
//! }
//! ui.end_frame();
//! ```

mod context;
mod draw;
mod font;
mod input_state;
mod interaction;
mod style;
#[cfg(test)]
mod test_support;
mod text_edit;

// Re-export main types
pub use context::{FloatFieldOpts, FloatInputResult, TextAlign, UIContext};
pub use draw::{DrawCommand, DrawList, GlyphDrawData, SliderVisual, TextDrawData, UiLayer};
pub use font::{FontError, FontHandle, FontManager, FontMetrics, GlyphInfo, LayoutGlyph, RasterizedGlyph, TextLayout};
pub use input_state::{InputState, KeyRepeat, REPEAT_DELAY, REPEAT_INTERVAL};
pub use interaction::{
    InteractionManager, InteractionResult, ScrubState, WidgetId, WidgetPersistentState,
    WidgetState,
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
