//! Text methods for [`UIContext`]: the label/measure family and the shared
//! text-drawing helpers every text-bearing widget routes through.

use glam::Vec2;

use crate::{Color, FontHandle, GlyphDrawData, Rect, TextDrawData, TextLayout};

use super::{TextAlign, UIContext};

impl UIContext {
    // ================== Text Helpers ==================

    /// Estimate text dimensions when no font is loaded.
    ///
    /// Single home for the character-count heuristic used as a fallback by
    /// all measurement and placement methods.
    fn estimate_text_size(text: &str, font_size: f32) -> Vec2 {
        /// Average glyph width as a fraction of font size.
        const CHAR_WIDTH_FACTOR: f32 = 0.6;
        /// Line height as a fraction of font size.
        const LINE_HEIGHT_FACTOR: f32 = 1.2;
        Vec2::new(
            text.chars().count() as f32 * font_size * CHAR_WIDTH_FACTOR,
            font_size * LINE_HEIGHT_FACTOR,
        )
    }

    /// Calculate the baseline Y position for vertically centered text.
    fn baseline_y(&self, text_top: f32, font_size: f32, font_handle: Option<FontHandle>) -> f32 {
        /// Ascent as a fraction of font size when metrics are unavailable.
        const ASCENT_FACTOR: f32 = 0.8;
        let ascent = font_handle
            .and_then(|fh| self.font_manager.metrics(fh, font_size))
            .map(|m| m.ascent)
            .unwrap_or(font_size * ASCENT_FACTOR);
        text_top + ascent
    }

    /// Compute the baseline position for text placed inside `bounds`:
    /// horizontally aligned per `align` (inset by `padding` for edge
    /// alignments) and vertically centered.
    ///
    /// Shared by every widget that draws text in a box (buttons, bounded
    /// labels, input fields).
    pub(super) fn text_pos_in_bounds(
        &self,
        text: &str,
        bounds: Rect,
        align: TextAlign,
        font_size: f32,
        padding: f32,
    ) -> Vec2 {
        self.text_pos_in_bounds_with_font(
            text,
            bounds,
            align,
            font_size,
            padding,
            self.font_manager.default_font(),
        )
    }

    /// [`Self::text_pos_in_bounds`] measured and baselined with an explicit
    /// font (`None` = the placeholder estimate), so a widget drawing in a
    /// non-default face places its text where that face measures.
    pub(super) fn text_pos_in_bounds_with_font(
        &self,
        text: &str,
        bounds: Rect,
        align: TextAlign,
        font_size: f32,
        padding: f32,
        font: Option<FontHandle>,
    ) -> Vec2 {
        let text_size = self.measure_text_with_font(text, font_size, font);
        let x = match align {
            TextAlign::Left => bounds.x + padding,
            TextAlign::Center => bounds.x + (bounds.width - text_size.x) / 2.0,
            TextAlign::Right => bounds.x + bounds.width - text_size.x - padding,
        };
        let text_top = bounds.y + (bounds.height - text_size.y) / 2.0;
        let baseline_y = self.baseline_y(text_top, font_size, font);
        Vec2::new(x, baseline_y)
    }

    /// Draw text at a baseline position with an explicit font, falling back
    /// to a placeholder when the font is missing or layout fails.
    ///
    /// Single home for the layout-or-placeholder tail shared by all
    /// text-drawing widgets.
    pub(super) fn draw_text_with_font(
        &mut self,
        font: Option<FontHandle>,
        text: &str,
        position: Vec2,
        color: Color,
        font_size: f32,
    ) {
        if let Some(font) = font {
            match self.font_manager.layout_text(font, text, font_size) {
                Ok(layout) => {
                    let text_data = Self::layout_to_draw_data(&layout, text, position, color, font_size, font);
                    self.draw_list.text(text_data);
                    return;
                }
                Err(e) => log::warn!("Font layout failed: {}", e),
            }
        }
        self.draw_list.text_placeholder(text, position, color, font_size);
    }

    /// Draw text at a baseline position using the default font (or a
    /// placeholder when no font is loaded).
    pub(super) fn draw_text_at_baseline(&mut self, text: &str, position: Vec2, color: Color, font_size: f32) {
        self.draw_text_with_font(self.font_manager.default_font(), text, position, color, font_size);
    }

    /// Convert a TextLayout to TextDrawData for rendering.
    ///
    /// This helper extracts the common pattern of converting font layout information
    /// into the draw data structure used by the rendering system.
    fn layout_to_draw_data(
        layout: &TextLayout,
        text: &str,
        position: Vec2,
        color: Color,
        font_size: f32,
        font: FontHandle,
    ) -> TextDrawData {
        let glyphs: Vec<GlyphDrawData> = layout.glyphs.iter().map(|g| {
            GlyphDrawData {
                bitmap: g.info.rasterized.bitmap.clone(),
                width: g.info.rasterized.width,
                height: g.info.rasterized.height,
                x: g.x,
                y: g.y,
                character: g.character,
            }
        }).collect();

        TextDrawData {
            text: text.to_string(),
            position,
            color,
            font_size,
            font_id: font.id,
            width: layout.width,
            height: layout.height,
            glyphs,
        }
    }

    // ================== Label Methods ==================

    /// Create a text label.
    ///
    /// If a font has been loaded, renders with actual glyphs. Otherwise falls back to placeholder.
    /// The position specifies where the baseline of the text should be placed.
    pub fn label(&mut self, text: &str, position: Vec2) {
        self.label_styled(text, position, self.theme.text.color, self.theme.text.font_size);
    }

    /// Create a text label with custom styling.
    ///
    /// If a font has been loaded, renders with actual glyphs. Otherwise falls back to placeholder.
    /// The position specifies where the baseline of the text should be placed.
    pub fn label_styled(&mut self, text: &str, position: Vec2, color: Color, font_size: f32) {
        self.draw_text_at_baseline(text, position, color, font_size);
    }

    /// Create a text label with a specific font handle.
    pub fn label_with_font(&mut self, text: &str, position: Vec2, font: FontHandle, font_size: f32) {
        let color = self.theme.text.color;
        self.draw_text_with_font(Some(font), text, position, color, font_size);
    }

    /// Draw a label centered within bounds.
    ///
    /// This method handles vertical centering automatically using font metrics
    /// when available, falling back to approximate centering otherwise.
    /// Use this for text in buttons, headers, and other bounded containers.
    pub fn label_in_bounds(&mut self, text: &str, bounds: Rect, align: TextAlign) {
        let color = self.theme.text.color;
        let font_size = self.theme.text.font_size;
        let padding = self.theme.panel.padding;
        self.label_in_bounds_styled(text, bounds, align, color, font_size, padding);
    }

    /// Draw a styled label vertically centered within bounds.
    ///
    /// Like [`label_in_bounds`](Self::label_in_bounds) but with explicit color,
    /// font size, and edge padding. Use this instead of [`label_styled`](Self::label_styled)
    /// whenever the text sits inside a box (panel headers, status bars) so the
    /// glyphs never straddle the box's border.
    pub fn label_in_bounds_styled(
        &mut self,
        text: &str,
        bounds: Rect,
        align: TextAlign,
        color: Color,
        font_size: f32,
        padding: f32,
    ) {
        let position = self.text_pos_in_bounds(text, bounds, align, font_size, padding);
        self.draw_text_at_baseline(text, position, color, font_size);
    }

    /// Create a text label centered horizontally at a position.
    ///
    /// Measures the text width and offsets so the text appears centered
    /// on `center.x`. The `center.y` specifies the vertical baseline position.
    pub fn label_centered(&mut self, text: &str, center: Vec2) {
        let color = self.theme.text.color;
        let font_size = self.theme.text.font_size;
        self.label_centered_styled(text, center, color, font_size);
    }

    /// Create a centered text label with custom styling.
    pub fn label_centered_styled(
        &mut self,
        text: &str,
        center: Vec2,
        color: Color,
        font_size: f32,
    ) {
        let half_width = self.measure_text_styled(text, font_size).x / 2.0;
        self.label_styled(text, Vec2::new(center.x - half_width, center.y), color, font_size);
    }

    // ================== Measurement ==================

    /// Measure text dimensions using the default font and font size.
    ///
    /// Returns the width and height of the text bounding box. If no font
    /// is loaded, returns an estimate based on character count.
    pub fn measure_text(&self, text: &str) -> Vec2 {
        self.measure_text_styled(text, self.theme.text.font_size)
    }

    /// Measure text dimensions with a custom font size.
    pub fn measure_text_styled(&self, text: &str, font_size: f32) -> Vec2 {
        self.measure_text_with_font(text, font_size, self.font_manager.default_font())
    }

    /// Measure text in an explicit font; `None` (or a handle that no longer
    /// resolves) falls back to the character-count estimate the placeholder
    /// draw path uses, so measurement and drawing stay in step.
    pub fn measure_text_with_font(&self, text: &str, font_size: f32, font: Option<FontHandle>) -> Vec2 {
        font.and_then(|fh| self.font_manager.measure_text(fh, text, font_size).ok())
            .unwrap_or_else(|| Self::estimate_text_size(text, font_size))
    }
}
