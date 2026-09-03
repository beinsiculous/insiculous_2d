//! Bounded cache of rasterized glyph bitmaps.
//!
//! `GlyphCache` owns the (font, character, size) → bitmap mapping and fills
//! misses by rasterizing with fontdue. Bitmaps are shared via `Arc` so a
//! cache hit never copies pixel data.

use std::collections::HashMap;
use std::sync::Arc;
use fontdue::Font;

use super::FontError;

/// Rasterized glyph data ready for rendering.
///
/// The bitmap is shared via `Arc` so cloning a glyph (cache → layout → draw
/// command) never copies pixel data.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Glyph bitmap data (grayscale, one byte per pixel)
    pub bitmap: Arc<[u8]>,
    /// Width of the bitmap in pixels
    pub width: u32,
    /// Height of the bitmap in pixels
    pub height: u32,
    /// Horizontal offset from cursor position to glyph origin
    pub offset_x: f32,
    /// Vertical offset from baseline to top of glyph
    pub offset_y: f32,
    /// How much to advance the cursor after this glyph
    pub advance: f32,
}

/// Cached glyph information for a specific font size.
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// Rasterized bitmap
    pub rasterized: RasterizedGlyph,
    /// Character this glyph represents
    pub character: char,
    /// Font size this was rasterized at
    pub font_size: f32,
}

/// Key for glyph cache lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    font_id: u32,
    character: char,
    /// Font size in tenths of a pixel (to allow float sizes with integer key)
    size_tenths: u32,
}

impl GlyphKey {
    fn new(font_id: u32, character: char, font_size: f32) -> Self {
        Self {
            font_id,
            character,
            size_tenths: (font_size * 10.0) as u32,
        }
    }
}

/// Maximum number of cached rasterized glyphs.
///
/// Dynamic text (scores, timers, animated sizes) keeps adding new
/// (font, char, size) combinations forever; without a bound the cache grows
/// for the lifetime of the process. When the limit is hit the cache is
/// cleared and rebuilt from live text — a one-frame re-rasterization cost.
const MAX_CACHED_GLYPHS: usize = 4096;

/// Bounded storage for rasterized glyphs, keyed by (font, character, size).
pub(super) struct GlyphCache {
    glyphs: HashMap<GlyphKey, GlyphInfo>,
}

impl GlyphCache {
    /// Create an empty glyph cache.
    pub(super) fn new() -> Self {
        Self {
            glyphs: HashMap::new(),
        }
    }

    /// Get a cached glyph, rasterizing and caching it on a miss.
    pub(super) fn get_or_rasterize(
        &mut self,
        font: &Font,
        font_id: u32,
        character: char,
        font_size: f32,
    ) -> Result<&GlyphInfo, FontError> {
        let key = GlyphKey::new(font_id, character, font_size);

        // Cache miss: rasterize before inserting (eviction may clear the map,
        // so the entry API can't be combined with the bounds check).
        if !self.glyphs.contains_key(&key) {
            let (metrics, bitmap) = font.rasterize(character, font_size);

            let glyph_info = GlyphInfo {
                rasterized: RasterizedGlyph {
                    bitmap: bitmap.into(),
                    width: metrics.width as u32,
                    height: metrics.height as u32,
                    offset_x: metrics.xmin as f32,
                    // Convert from fontdue coords (ymin = bottom of glyph, +Y = up)
                    // to UI coords (offset_y = top of glyph relative to baseline, +Y = down)
                    // Top in fontdue = ymin + height, flip sign for UI = -(ymin + height)
                    offset_y: -(metrics.ymin as f32 + metrics.height as f32),
                    advance: metrics.advance_width,
                },
                character,
                font_size,
            };

            self.evict_if_full();
            self.glyphs.insert(key, glyph_info);
        }

        self.glyphs.get(&key)
            .ok_or_else(|| FontError::RasterizeError(format!("Glyph cache lookup failed for '{}'", character)))
    }

    /// Bound the glyph cache: evict everything once full rather than growing
    /// without limit. Live text repopulates it within a frame.
    fn evict_if_full(&mut self) {
        if self.glyphs.len() >= MAX_CACHED_GLYPHS {
            log::debug!(
                "Glyph cache reached {} entries; clearing",
                self.glyphs.len()
            );
            self.glyphs.clear();
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.glyphs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_glyph() -> GlyphInfo {
        GlyphInfo {
            rasterized: RasterizedGlyph {
                bitmap: Arc::from([]),
                width: 0,
                height: 0,
                offset_x: 0.0,
                offset_y: 0.0,
                advance: 0.0,
            },
            character: 'a',
            font_size: 16.0,
        }
    }

    #[test]
    fn test_glyph_key_quantizes_font_size_to_tenths() {
        // Deliberate: sizes within a tenth of a pixel share a bitmap, so
        // an animated size does not rasterize a new glyph every frame.
        assert_eq!(GlyphKey::new(1, 'A', 16.0), GlyphKey::new(1, 'A', 16.04));
        assert_ne!(GlyphKey::new(1, 'A', 16.0), GlyphKey::new(1, 'A', 16.1));
        assert_ne!(GlyphKey::new(1, 'A', 16.0), GlyphKey::new(2, 'A', 16.0));
        assert_ne!(GlyphKey::new(1, 'A', 16.0), GlyphKey::new(1, 'B', 16.0));
    }

    #[test]
    fn test_glyph_cache_clears_at_the_limit_and_keeps_entries_below_it() {
        let mut cache = GlyphCache::new();
        cache.glyphs.insert(GlyphKey::new(1, 'a', 16.0), dummy_glyph());
        cache.evict_if_full();
        assert_eq!(cache.len(), 1, "below the limit nothing is evicted");

        // Fill to the limit with unique keys (size_tenths varies per entry).
        for i in 0..MAX_CACHED_GLYPHS {
            cache.glyphs.insert(GlyphKey::new(1, 'a', i as f32 * 0.1), dummy_glyph());
        }
        assert_eq!(cache.len(), MAX_CACHED_GLYPHS);
        cache.evict_if_full();
        assert_eq!(cache.len(), 0, "a cache at the limit is cleared, not grown further");
    }
}
