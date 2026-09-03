//! UI draw-command collection: per-layer command lists with depth bands.
//!
//! Commands are recorded into the layer on top of the layer stack
//! ([`UiLayer`], default [`UiLayer::Content`]) and flushed into ONE
//! submission-ordered stream at `end_frame` — elevated layers are
//! appended after Content in enum order, so a Floating popup recorded
//! inside a panel's `PushClipRect`/`PopClipRect` pair physically escapes
//! the clip (clip commands carry no depth; they are never reordered, and
//! whole-layer concatenation can never tear a pair). Depth bands mirror
//! the flush order so the depth buffer agrees with it.

mod command;

pub use command::{DrawCommand, GlyphDrawData, TextDrawData};

use glam::Vec2;

use crate::{Color, Rect};

/// Z-band a draw command belongs to. Flushed (and depth-banded) in enum
/// order: `Content` is always lowest, `DragGhost` always topmost.
///
/// `PanelChrome` intentionally draws ABOVE `Content`: it is for chrome
/// that must stay grabbable/visible over panel content (dock resize
/// grabbers, tab strips), not for panel backgrounds — backgrounds are
/// Content like the widgets on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum UiLayer {
    /// Panel bodies, widgets, the regular UI (band base 900).
    #[default]
    Content,
    /// Chrome above content: dock grabbers, tab strips (915).
    PanelChrome,
    /// Dropdowns, popups — `begin_overlay` records here (930).
    Floating,
    /// Modal dialogs + their scrims (945).
    Modal,
    /// Tooltips ride above modals (960).
    Tooltip,
    /// Drag ghosts follow the cursor above everything (975).
    DragGhost,
}

impl UiLayer {
    /// Every layer, in flush order.
    pub const ALL: [UiLayer; 6] = [
        UiLayer::Content,
        UiLayer::PanelChrome,
        UiLayer::Floating,
        UiLayer::Modal,
        UiLayer::Tooltip,
        UiLayer::DragGhost,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// The depth this layer's band starts at (each band spans 15.0).
    /// Consumers assert band membership with it; never hardcode band values.
    pub fn depth_base(self) -> f32 {
        UI_BASE_DEPTH + self.index() as f32 * LAYER_BAND
    }
}

/// Base depth for UI draw commands: UI renders on top of game content
/// (< camera far=1000).
const UI_BASE_DEPTH: f32 = 900.0;

/// Depth span of one [`UiLayer`] band. Six bands cover 900..990; at the
/// 0.001 per-command step a band holds 15,000 commands before colliding
/// with the next.
const LAYER_BAND: f32 = 15.0;

/// A draw list that collects all UI draw commands for a frame.
#[derive(Debug, Clone, Default)]
pub struct DrawList {
    /// The [`UiLayer::Content`] commands — and, after
    /// [`flush_layers`](Self::flush_layers), the complete ordered stream.
    commands: Vec<DrawCommand>,
    /// Commands for the five elevated layers (index = `UiLayer::index()-1`).
    elevated: [Vec<DrawCommand>; 5],
    /// Nested layer scopes; the top is where commands record.
    /// Empty = Content.
    layer_stack: Vec<UiLayer>,
    /// Debug bookkeeping: open clip rects per layer, so a clip pair torn
    /// across layers (push in Content, pop in Floating) is caught at the
    /// pop instead of corrupting the renderer's clip stack.
    clip_depth: [u32; 6],
}

impl DrawList {
    /// Create a new empty draw list.
    pub fn new() -> Self {
        Self::default()
    }

    /// The layer commands currently record into.
    pub fn current_layer(&self) -> UiLayer {
        self.layer_stack.last().copied().unwrap_or(UiLayer::Content)
    }

    /// Record subsequent commands into `layer` until the matching
    /// [`pop_layer`](Self::pop_layer). A clip rect pushed inside a layer
    /// scope must be popped inside it.
    pub fn push_layer(&mut self, layer: UiLayer) {
        self.layer_stack.push(layer);
    }

    /// End the innermost layer scope (no-op when already at Content).
    pub fn pop_layer(&mut self) {
        self.layer_stack.pop();
    }

    /// Record subsequent commands in the Floating band so they render on
    /// top of all content UI (panels, toolbars) regardless of submission
    /// order. Must be paired with [`end_overlay`](Self::end_overlay).
    /// Sugar for `push_layer(UiLayer::Floating)`.
    pub fn begin_overlay(&mut self) {
        self.push_layer(UiLayer::Floating);
    }

    /// Return to the previous layer.
    pub fn end_overlay(&mut self) {
        self.pop_layer();
    }

    /// Append the elevated layers to the Content stream, in [`UiLayer`]
    /// order. Called by `UIContext::end_frame`; idempotent. After this,
    /// [`commands`](Self::commands) is the complete frame in the exact
    /// order the renderer must consume it.
    pub fn flush_layers(&mut self) {
        for layer_commands in &mut self.elevated {
            self.commands.append(layer_commands);
        }
    }

    /// Calculate the depth for the next draw command: the current layer's
    /// band base plus a small per-command step to maintain draw order.
    #[inline]
    fn next_depth(&self) -> f32 {
        let layer = self.current_layer();
        let step = self.target(layer).len() as f32 * 0.001;
        debug_assert!(
            step < LAYER_BAND,
            "{layer:?} band exhausted ({} commands) — depths now collide with the next band",
            self.target(layer).len()
        );
        layer.depth_base() + step
    }

    fn target(&self, layer: UiLayer) -> &Vec<DrawCommand> {
        match layer.index() {
            0 => &self.commands,
            i => &self.elevated[i - 1],
        }
    }

    fn push(&mut self, cmd: DrawCommand) {
        let layer = self.current_layer();
        match layer.index() {
            0 => self.commands.push(cmd),
            i => self.elevated[i - 1].push(cmd),
        }
    }

    /// Clear all draw commands and layer scopes.
    pub fn clear(&mut self) {
        self.commands.clear();
        for layer_commands in &mut self.elevated {
            layer_commands.clear();
        }
        self.layer_stack.clear();
        self.clip_depth = [0; 6];
    }

    /// Get all draw commands.
    ///
    /// **Lifecycle contract:** before [`flush_layers`](Self::flush_layers)
    /// (which `UIContext::end_frame` calls) this is the CONTENT layer
    /// only — elevated commands (overlays, modals, drag ghosts) are not
    /// in it yet, and [`len`](Self::len) (which counts every layer) can
    /// exceed `commands().len()`. Renderers and assertions that need the
    /// complete ordered frame must read after `end_frame`.
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Total number of draw commands recorded across ALL layers — see the
    /// lifecycle note on [`commands`](Self::commands): before flush this
    /// can exceed `commands().len()`.
    pub fn len(&self) -> usize {
        self.commands.len() + self.elevated.iter().map(Vec::len).sum::<usize>()
    }

    /// Check if the draw list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Add a filled rectangle.
    pub fn rect(&mut self, bounds: Rect, color: Color) {
        self.rect_rounded(bounds, color, 0.0);
    }

    /// Add a filled rectangle with rounded corners.
    pub fn rect_rounded(&mut self, bounds: Rect, color: Color, corner_radius: f32) {
        let depth = self.next_depth();
        self.push(DrawCommand::Rect { bounds, color, corner_radius, depth });
    }

    /// Add a rectangle border.
    pub fn rect_border(&mut self, bounds: Rect, color: Color, width: f32) {
        self.rect_border_rounded(bounds, color, width, 0.0);
    }

    /// Add a rectangle border with rounded corners.
    pub fn rect_border_rounded(&mut self, bounds: Rect, color: Color, width: f32, corner_radius: f32) {
        let depth = self.next_depth();
        self.push(DrawCommand::RectBorder { bounds, color, width, corner_radius, depth });
    }

    /// Add a textured image by renderer texture id.
    pub fn image(&mut self, bounds: Rect, texture_id: u32, tint: Color) {
        self.image_rounded(bounds, texture_id, tint, 0.0);
    }

    /// Add a textured image with rounded corners.
    pub(crate) fn image_rounded(&mut self, bounds: Rect, texture_id: u32, tint: Color, corner_radius: f32) {
        let depth = self.next_depth();
        self.push(DrawCommand::Image { bounds, texture_id, tint, corner_radius, depth });
    }

    /// Add text placeholder (renders as approximate rectangle without font).
    pub(crate) fn text_placeholder(&mut self, text: impl Into<String>, position: Vec2, color: Color, font_size: f32) {
        let depth = self.next_depth();
        self.push(DrawCommand::TextPlaceholder {
            text: text.into(),
            position,
            color,
            font_size,
            depth,
        });
    }

    /// Add text with rasterized glyph data.
    pub fn text(&mut self, data: TextDrawData) {
        let depth = self.next_depth();
        self.push(DrawCommand::Text { data, depth });
    }

    /// Add a filled circle.
    pub fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        let depth = self.next_depth();
        self.push(DrawCommand::Circle { center, radius, color, depth });
    }

    /// Add a line.
    pub fn line(&mut self, start: Vec2, end: Vec2, color: Color, width: f32) {
        let depth = self.next_depth();
        self.push(DrawCommand::Line { start, end, color, width, depth });
    }

    /// Begin clipping all subsequent draws to the given bounds.
    /// Must be paired with `pop_clip_rect()` in the SAME layer scope —
    /// layers flush as whole blocks, so a pair torn across layers would
    /// unbalance the renderer's clip stack.
    pub fn push_clip_rect(&mut self, bounds: Rect) {
        self.clip_depth[self.current_layer().index()] += 1;
        self.push(DrawCommand::PushClipRect { bounds });
    }

    /// End the current clip region, restoring the previous clip state.
    pub fn pop_clip_rect(&mut self) {
        let layer = self.current_layer().index();
        debug_assert!(
            self.clip_depth[layer] > 0,
            "pop_clip_rect in a layer with no open clip — the matching push \
             is in another layer; keep clip pairs inside one layer scope"
        );
        self.clip_depth[layer] = self.clip_depth[layer].saturating_sub(1);
        self.push(DrawCommand::PopClipRect);
    }

    /// Draw a panel background with border.
    pub fn panel(&mut self, bounds: Rect, background: Color, border: Color, border_width: f32, corner_radius: f32) {
        // Background first
        self.rect_rounded(bounds, background, corner_radius);
        // Then border on top
        if border_width > 0.0 {
            self.rect_border_rounded(bounds, border, border_width, corner_radius);
        }
    }

    /// Draw a slider track and thumb.
    pub fn slider(&mut self, visual: SliderVisual) {
        // Draw track background
        self.rect_rounded(visual.track_bounds, visual.track_background, visual.track_bounds.height / 2.0);

        // Draw filled portion
        if visual.fill_amount > 0.0 {
            let fill_width = visual.track_bounds.width * visual.fill_amount;
            let fill_bounds = Rect::new(visual.track_bounds.x, visual.track_bounds.y, fill_width, visual.track_bounds.height);
            self.rect_rounded(fill_bounds, visual.track_fill, visual.track_bounds.height / 2.0);
        }

        // Draw thumb
        self.circle(visual.thumb_center, visual.thumb_radius, visual.thumb_color);
    }
}

/// Visual parameters for drawing a slider track and thumb.
#[derive(Debug, Clone, Copy)]
pub struct SliderVisual {
    pub track_bounds: Rect,
    pub thumb_center: Vec2,
    pub thumb_radius: f32,
    pub track_background: Color,
    pub track_fill: Color,
    pub thumb_color: Color,
    pub fill_amount: f32,
}

#[cfg(test)]
mod tests;
