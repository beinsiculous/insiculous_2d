//! The draw primitives the UI system generates — converted to sprites by
//! the renderer integration layer. Collection and layering live in the
//! parent module ([`DrawList`](super::DrawList)).

use std::sync::Arc;
use glam::Vec2;
use crate::{Color, Rect};

/// Data for rendering a single glyph.
///
/// The bitmap is shared with the font glyph cache via `Arc` — cloning draw
/// data never copies pixel data.
#[derive(Debug, Clone)]
pub struct GlyphDrawData {
    /// Glyph bitmap data (grayscale, one byte per pixel)
    pub bitmap: Arc<[u8]>,
    /// Width of the glyph bitmap
    pub width: u32,
    /// Height of the glyph bitmap
    pub height: u32,
    /// X position relative to text origin
    pub x: f32,
    /// Y position relative to text origin
    pub y: f32,
    /// The character this glyph represents
    pub character: char,
}

/// Data for rendering text with rasterized glyphs.
#[derive(Debug, Clone)]
pub struct TextDrawData {
    /// Text string (for reference)
    pub text: String,
    /// Position of the text origin (top-left)
    pub position: Vec2,
    /// Text color
    pub color: Color,
    /// Font size used
    pub font_size: f32,
    /// Id of the font the glyphs were rasterized from (`FontHandle.id`).
    /// Downstream glyph-texture caches must include this in their keys —
    /// different fonts rasterize the same character at the same size to
    /// different bitmaps.
    pub font_id: u32,
    /// Total width of the laid out text
    pub width: f32,
    /// Total height of the laid out text
    pub height: f32,
    /// Individual glyphs with positions and bitmaps
    pub glyphs: Vec<GlyphDrawData>,
}

/// A UI draw command representing a visual primitive to render.
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// Draw a filled rectangle
    Rect {
        bounds: Rect,
        color: Color,
        corner_radius: f32,
        depth: f32,
    },
    /// Draw a rectangle border (outline)
    RectBorder {
        bounds: Rect,
        color: Color,
        width: f32,
        corner_radius: f32,
        depth: f32,
    },
    /// Draw text with rasterized glyph data
    Text {
        data: TextDrawData,
        depth: f32,
    },
    /// Draw text without font data (fallback/placeholder)
    TextPlaceholder {
        text: String,
        position: Vec2,
        color: Color,
        font_size: f32,
        depth: f32,
    },
    /// Draw a circle
    Circle {
        center: Vec2,
        radius: f32,
        color: Color,
        depth: f32,
    },
    /// Draw a line
    Line {
        start: Vec2,
        end: Vec2,
        color: Color,
        width: f32,
        depth: f32,
    },
    /// Draw a textured image (thumbnails, previews). `texture_id` is the
    /// renderer texture handle id — the ui crate stays renderer-agnostic.
    Image {
        bounds: Rect,
        texture_id: u32,
        tint: Color,
        corner_radius: f32,
        depth: f32,
    },
    /// Begin clipping to a rectangular region.
    /// All subsequent draws are clipped to this bounds until PopClipRect.
    PushClipRect {
        bounds: Rect,
    },
    /// End the current clipping region, restore previous clip state.
    PopClipRect,
}

impl DrawCommand {
    /// Get the depth assigned to this draw command.
    ///
    /// Depth increases with submission order and is intended for the renderer's
    /// depth buffer, not for reordering. Commands MUST be consumed in submission
    /// order: clip commands (`PushClipRect`/`PopClipRect`) carry no depth and
    /// sorting by depth would tear clip pairs apart.
    pub fn depth(&self) -> f32 {
        match self {
            DrawCommand::Rect { depth, .. } => *depth,
            DrawCommand::RectBorder { depth, .. } => *depth,
            DrawCommand::Text { depth, .. } => *depth,
            DrawCommand::TextPlaceholder { depth, .. } => *depth,
            DrawCommand::Circle { depth, .. } => *depth,
            DrawCommand::Line { depth, .. } => *depth,
            DrawCommand::Image { depth, .. } => *depth,
            DrawCommand::PushClipRect { .. } => 0.0, // Clip commands don't have depth
            DrawCommand::PopClipRect => 0.0,
        }
    }
}
