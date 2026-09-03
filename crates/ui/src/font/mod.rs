//! Font loading and text rendering with fontdue.
//!
//! This module provides font loading, glyph rasterization, and text measurement
//! using the fontdue library for CPU-side font rendering.
//!
//! Split by responsibility:
//! - `mod.rs` — `FontManager` facade (font loading/storage) and shared types
//! - `glyph_cache.rs` — `GlyphCache` (rasterized glyph storage with bounded eviction)
//! - `layout.rs` — text layout and measurement

mod glyph_cache;
mod layout;

pub use glyph_cache::{GlyphInfo, RasterizedGlyph};
pub use layout::{LayoutGlyph, TextLayout};

use std::collections::HashMap;
use fontdue::{Font, FontSettings};
use glam::Vec2;

use self::glyph_cache::GlyphCache;

/// Error type for font operations.
#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("Failed to load font: {0}")]
    LoadError(String),
    #[error("Font not found: {0}")]
    NotFound(String),
    #[error("Failed to rasterize glyph: {0}")]
    RasterizeError(String),
}

/// A handle to a loaded font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
pub struct FontHandle {
    pub id: u32,
}

/// Font measurement information for layout calculations.
///
/// These metrics are essential for proper text positioning:
/// - `ascent`: Use to know how much space text needs above the baseline
/// - `descent`: Use to know how much space text needs below the baseline
/// - `line_height`: Use for spacing between lines of text
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    /// Distance from baseline to top of tallest glyph (positive value)
    pub ascent: f32,
    /// Distance from baseline to bottom of descenders (negative value)
    pub descent: f32,
    /// Recommended distance between baselines for line spacing
    pub line_height: f32,
}

/// Font manager for loading fonts and rasterizing glyphs.
///
/// Acts as the public facade over font storage, composing a [`GlyphCache`]
/// for rasterized bitmaps and delegating layout/measurement to the `layout`
/// module.
pub struct FontManager {
    /// Loaded fonts by handle
    fonts: HashMap<u32, Font>,
    /// Glyph cache
    glyph_cache: GlyphCache,
    /// Next font handle ID
    next_id: u32,
    /// Default font handle (if loaded)
    default_font: Option<FontHandle>,
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FontManager {
    /// Create a new font manager.
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            glyph_cache: GlyphCache::new(),
            next_id: 1,
            default_font: None,
        }
    }

    /// Load a font from file bytes.
    pub fn load_font(&mut self, font_data: &[u8]) -> Result<FontHandle, FontError> {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| FontError::LoadError(e.to_string()))?;

        let handle = FontHandle { id: self.next_id };
        self.fonts.insert(self.next_id, font);
        self.next_id += 1;

        // Set as default if this is the first font
        if self.default_font.is_none() {
            self.default_font = Some(handle);
        }

        log::info!("Loaded font with handle {}", handle.id);
        Ok(handle)
    }

    /// Load a font from a file path.
    pub fn load_font_file(&mut self, path: &str) -> Result<FontHandle, FontError> {
        let font_data = common::vfs::read(std::path::Path::new(path))
            .map_err(|e| FontError::LoadError(format!("Failed to read file {}: {}", path, e)))?;
        self.load_font(&font_data)
    }

    /// Get the default font handle.
    pub fn default_font(&self) -> Option<FontHandle> {
        self.default_font
    }

    /// Set the default font.
    pub fn set_default_font(&mut self, handle: FontHandle) {
        self.default_font = Some(handle);
    }

    /// Get a font by handle.
    pub fn get_font(&self, handle: FontHandle) -> Option<&Font> {
        self.fonts.get(&handle.id)
    }

    /// Get font metrics for layout calculations.
    ///
    /// Returns None if the font handle is invalid or metrics unavailable.
    pub fn metrics(&self, handle: FontHandle, font_size: f32) -> Option<FontMetrics> {
        let font = self.fonts.get(&handle.id)?;
        let line_metrics = font.horizontal_line_metrics(font_size)?;

        Some(FontMetrics {
            ascent: line_metrics.ascent,
            descent: line_metrics.descent,
            line_height: line_metrics.ascent - line_metrics.descent + line_metrics.line_gap,
        })
    }

    /// Layout a string of text, returning positions and glyph info for each character.
    ///
    /// This method uses the internal glyph cache to avoid re-rasterizing glyphs that have
    /// already been rendered at the same font/size combination. The CPU bitmap cache here
    /// works in tandem with engine_core's GPU texture cache - this caches the rasterized
    /// bitmaps, while engine_core caches the GPU textures created from those bitmaps.
    ///
    /// Coordinate system:
    /// - The text origin (position.y) is at the BASELINE
    /// - glyph.y is the offset from baseline to glyph top (negative = above baseline)
    /// - The rendering code subtracts glyph.y from position.y to place the glyph correctly
    pub fn layout_text(
        &mut self,
        handle: FontHandle,
        text: &str,
        font_size: f32,
    ) -> Result<TextLayout, FontError> {
        let font = self.fonts.get(&handle.id)
            .ok_or_else(|| FontError::NotFound(format!("Font {} not found", handle.id)))?;
        layout::layout_text(font, handle.id, &mut self.glyph_cache, text, font_size)
    }

    /// Measure the size of a text string without rasterizing.
    ///
    /// Uses `font.metrics()` instead of `font.rasterize()` to get advance widths
    /// without the expensive bitmap generation step.
    pub fn measure_text(
        &self,
        handle: FontHandle,
        text: &str,
        font_size: f32,
    ) -> Result<Vec2, FontError> {
        let font = self.fonts.get(&handle.id)
            .ok_or_else(|| FontError::NotFound(format!("Font {} not found", handle.id)))?;
        Ok(layout::measure_text(font, text, font_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_font_handles_are_not_found_and_garbage_bytes_fail_to_load() {
        // The typed error paths the placeholder fallback in `UIContext`
        // keys on: a stale handle is `NotFound`, bad bytes are `LoadError`.
        let mut manager = FontManager::new();
        let stale = FontHandle { id: 999 };

        assert!(matches!(manager.load_font(b"not a font"), Err(FontError::LoadError(_))));
        assert!(manager.default_font().is_none(), "a failed load registers nothing");
        assert!(matches!(manager.measure_text(stale, "x", 16.0), Err(FontError::NotFound(_))));
        assert!(matches!(manager.layout_text(stale, "x", 16.0), Err(FontError::NotFound(_))));
        assert_eq!(manager.metrics(stale, 16.0), None);
    }
}
