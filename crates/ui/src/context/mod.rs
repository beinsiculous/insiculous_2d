//! Immediate-mode UI context.
//!
//! The UIContext is the main entry point for creating UI elements.
//! It follows an immediate-mode paradigm where you describe the UI every frame.
//!
//! Split by responsibility:
//! - `mod.rs` — UIContext struct, construction, frame lifecycle, fonts, core state
//! - `text.rs` — label/measure family and shared text-drawing helpers
//! - `widgets.rs` — interactive widgets (button, slider, checkbox)
//!   and container/shape drawing
//! - `text_input.rs` — the float/text input widget (cursor, selection, caret)

mod text;
mod text_input;
mod widgets;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod overlay_tests;
#[cfg(test)]
mod focus_tests;

use glam::Vec2;
use input::InputHandler;

/// Text alignment within a bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Align text to the left edge
    #[default]
    Left,
    /// Center text horizontally
    Center,
    /// Align text to the right edge
    Right,
}

use crate::{
    Color, DrawList, FontError, FontHandle, FontManager, InteractionManager, InteractionResult,
    Rect, Theme, UiLayer, WidgetId,
};

/// The main UI context for immediate-mode UI rendering.
///
/// # Example
/// ```
/// # use ui::{UIContext, Rect};
/// # use glam::Vec2;
/// # use input::InputHandler;
/// let mut ui = UIContext::new();
/// # let input_handler = InputHandler::new();
///
/// // In your update loop:
/// ui.begin_frame(&input_handler, Vec2::new(800.0, 600.0));
///
/// if ui.button("my_button", "Click Me!", Rect::new(10.0, 10.0, 100.0, 30.0)) {
///     println!("Button clicked!");
/// }
///
/// ui.end_frame();
///
/// // Get draw commands for rendering
/// let commands = ui.draw_list().commands();
/// # assert!(!commands.is_empty());
/// ```
pub struct UIContext {
    /// Interaction manager for widget state tracking
    interaction: InteractionManager,
    /// Draw list for collecting render commands
    draw_list: DrawList,
    /// Current theme
    theme: Theme,
    /// Window size for layout calculations
    window_size: Vec2,
    /// Font manager for text rendering
    font_manager: FontManager,
}

impl Default for UIContext {
    fn default() -> Self {
        Self::new()
    }
}

impl UIContext {
    /// Create a new UI context with default theme.
    pub fn new() -> Self {
        Self {
            interaction: InteractionManager::new(),
            draw_list: DrawList::new(),
            theme: Theme::default(),
            window_size: Vec2::new(800.0, 600.0),
            font_manager: FontManager::new(),
        }
    }

    /// Create a new UI context with a custom theme.
    pub fn with_theme(theme: Theme) -> Self {
        let mut ctx = Self::new();
        ctx.theme = theme;
        ctx
    }

    // ================== Font Methods ==================

    /// Load a font from file bytes.
    pub fn load_font(&mut self, font_data: &[u8]) -> Result<FontHandle, FontError> {
        self.font_manager.load_font(font_data)
    }

    /// Load a font from a file path.
    pub fn load_font_file(&mut self, path: &str) -> Result<FontHandle, FontError> {
        self.font_manager.load_font_file(path)
    }

    /// Get the default font handle.
    pub fn default_font(&self) -> Option<FontHandle> {
        self.font_manager.default_font()
    }

    /// Set the default font.
    pub fn set_default_font(&mut self, handle: FontHandle) {
        self.font_manager.set_default_font(handle);
    }

    /// Get the font manager for advanced operations.
    pub fn font_manager(&self) -> &FontManager {
        &self.font_manager
    }

    /// Get the font manager mutably.
    pub fn font_manager_mut(&mut self) -> &mut FontManager {
        &mut self.font_manager
    }

    /// Get font metrics for the default font at the given size.
    ///
    /// Returns None if no font is loaded or metrics are unavailable.
    /// Use this for calculating text positions with baseline alignment.
    pub fn font_metrics(&self, font_size: f32) -> Option<crate::FontMetrics> {
        let font = self.font_manager.default_font()?;
        self.font_manager.metrics(font, font_size)
    }

    // ================== Frame Lifecycle ==================

    /// Begin a new frame. Call this at the start of each frame.
    ///
    /// Key repeat paces off an assumed 60 FPS frame delta; prefer
    /// [`Self::begin_frame_dt`] when a real delta time is available.
    pub fn begin_frame(&mut self, input: &InputHandler, window_size: Vec2) {
        self.interaction.begin_frame(input);
        self.draw_list.clear();
        self.window_size = window_size;
    }

    /// Begin a new frame with an explicit frame delta (seconds). The delta
    /// paces held-key repeat in text inputs.
    pub fn begin_frame_dt(&mut self, input: &InputHandler, window_size: Vec2, dt: f32) {
        self.interaction.begin_frame_dt(input, dt);
        self.draw_list.clear();
        self.window_size = window_size;
    }

    /// End the frame. Call this after all UI elements have been created.
    pub fn end_frame(&mut self) {
        // Elevated layers (popups, modals, drag ghosts) flush after the
        // content stream so they physically escape any panel clip pairs.
        self.draw_list.flush_layers();
        self.interaction.end_frame();
    }

    /// Get the draw list containing all render commands.
    pub fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Set a new theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Get the window size.
    pub fn window_size(&self) -> Vec2 {
        self.window_size
    }

    /// Get the current mouse position.
    pub fn mouse_pos(&self) -> Vec2 {
        self.interaction.mouse_pos()
    }

    /// Whether the left mouse button was pressed this frame.
    pub fn mouse_just_pressed(&self) -> bool {
        self.interaction.input().mouse_just_pressed
    }

    /// Whether the left mouse button is currently held down.
    pub fn mouse_down(&self) -> bool {
        self.interaction.input().mouse_down
    }

    /// Whether the left mouse button was released this frame.
    pub fn mouse_just_released(&self) -> bool {
        self.interaction.input().mouse_just_released
    }

    /// Mouse wheel scroll delta for this frame.
    pub fn scroll_delta(&self) -> f32 {
        self.interaction.input().scroll_delta
    }

    /// Whether a widget (e.g. a text input being edited) currently has
    /// keyboard focus. Hosts should suppress their own keyboard shortcuts
    /// while this returns `true`.
    pub fn wants_keyboard(&self) -> bool {
        self.interaction.has_focus()
    }

    /// Whether a widget owns the current mouse gesture — true from the press
    /// that landed on a widget through the release frame. Raw-input consumers
    /// (e.g. viewport picking) should not act on mouse clicks while this
    /// returns `true`, or a click on chrome falls through to what's beneath.
    pub fn wants_mouse(&self) -> bool {
        self.interaction.wants_mouse()
    }

    /// Whether the given widget currently has keyboard focus.
    pub fn is_focused(&self, id: impl Into<WidgetId>) -> bool {
        self.interaction.is_focused(id.into())
    }

    /// Programmatically focus a text input before it is next rendered,
    /// seeding its edit buffer with `initial` fully selected — typing
    /// replaces it, exactly as if the user had clicked the field. Lets hosts
    /// open an inline edit from a shortcut (e.g. F2 rename) instead of
    /// requiring a click.
    pub fn focus_text_input(&mut self, id: impl Into<WidgetId>, initial: &str) {
        let id = id.into();
        self.interaction.set_focus(id);
        self.interaction.get_state(id).edit.set_text_select_all(initial);
    }

    // ================== Overlays ==================

    /// Begin an overlay (dropdown menu, popup) over `blocking_rect`.
    ///
    /// Until [`end_overlay`](Self::end_overlay):
    /// - draw commands go to a higher depth band so they render on top of
    ///   all base UI regardless of submission order, and
    /// - `interact()` calls stay live while widgets *outside* the overlay
    ///   become inert whenever the mouse is inside `blocking_rect` (for the
    ///   rest of the frame), so clicks don't pass through the overlay.
    pub fn begin_overlay(&mut self, blocking_rect: Rect) {
        self.begin_overlay_in(UiLayer::Floating, blocking_rect);
    }

    /// [`begin_overlay`](Self::begin_overlay) into an explicit [`UiLayer`]
    /// band — a drag ghost belongs in [`UiLayer::DragGhost`] so it rides
    /// above an open dropdown, a modal in [`UiLayer::Modal`].
    ///
    /// Do not NEST overlay scopes: blocking rects accumulate for the
    /// whole frame (never popped, by design) and the interaction scope is
    /// a flag, not a stack — an inner `end_overlay` would already mark
    /// widgets between the scopes as outside-the-overlay. Draw sequential
    /// overlays back-to-back instead.
    pub fn begin_overlay_in(&mut self, layer: UiLayer, blocking_rect: Rect) {
        self.draw_list.push_layer(layer);
        self.interaction.push_blocking_rect(blocking_rect);
        self.interaction.set_overlay_scope(true);
    }

    /// End the current overlay, returning to the base depth band and
    /// re-enabling input blocking for subsequent widgets.
    pub fn end_overlay(&mut self) {
        self.draw_list.pop_layer();
        self.interaction.set_overlay_scope(false);
    }

    /// Whether mouse input at `pos` is swallowed by an open overlay
    /// (dropdown/popup). Raw-input consumers (e.g. viewport picking) should
    /// check this before acting on mouse events.
    pub fn is_input_blocked_at(&self, pos: Vec2) -> bool {
        self.interaction.is_blocked_at(pos)
    }

    // ================== Drawing Primitives ==================

    /// Draw a colored rectangle.
    pub fn rect(&mut self, bounds: Rect, color: Color) {
        self.draw_list.rect(bounds, color);
    }

    /// Draw a colored rectangle with rounded corners.
    pub fn rect_rounded(&mut self, bounds: Rect, color: Color, corner_radius: f32) {
        self.draw_list.rect_rounded(bounds, color, corner_radius);
    }

    /// Draw a rectangle outline with the given stroke width and corner radius.
    pub fn rect_border(&mut self, bounds: Rect, color: Color, width: f32, corner_radius: f32) {
        self.draw_list.rect_border_rounded(bounds, color, width, corner_radius);
    }

    /// Draw a textured image by renderer texture id (thumbnails, previews).
    pub fn image(&mut self, bounds: Rect, texture_id: u32, tint: Color) {
        self.draw_list.image(bounds, texture_id, tint);
    }

    /// Draw a circle.
    pub fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.draw_list.circle(center, radius, color);
    }

    /// Draw a line.
    pub fn line(&mut self, start: Vec2, end: Vec2, color: Color, width: f32) {
        self.draw_list.line(start, end, color, width);
    }

    /// Begin clipping all subsequent draws to the given bounds.
    ///
    /// Use this to prevent content from rendering outside panel boundaries.
    /// Must be paired with `pop_clip_rect()`. Clip regions can be nested.
    pub fn push_clip_rect(&mut self, bounds: Rect) {
        self.draw_list.push_clip_rect(bounds);
    }

    /// End the current clip region, restoring the previous clip state.
    ///
    /// If no clip region is active, this resets to the full viewport.
    pub fn pop_clip_rect(&mut self) {
        self.draw_list.pop_clip_rect();
    }

    // ================== Custom Widget Hooks ==================

    /// Check if a point is inside a rectangle (for custom hit testing).
    pub fn hit_test(&self, point: Vec2, bounds: Rect) -> bool {
        bounds.contains(point)
    }

    /// Get raw interaction result for custom widgets.
    pub fn interact(&mut self, id: impl Into<WidgetId>, bounds: Rect, enabled: bool) -> InteractionResult {
        self.interaction.interact(id.into(), bounds, enabled)
    }
}
