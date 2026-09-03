//! Text layout and measurement.
//!
//! Lays out a string into positioned glyphs (filling the glyph cache on the
//! way) and measures text dimensions without rasterizing.

use fontdue::Font;
use glam::Vec2;

use super::glyph_cache::GlyphCache;
use super::{FontError, GlyphInfo};

/// Text layout information for a string of text.
#[derive(Debug, Clone)]
pub struct TextLayout {
    /// Total width of the text in pixels
    pub width: f32,
    /// Total height of the text in pixels
    pub height: f32,
    /// Individual glyph positions and info
    pub glyphs: Vec<LayoutGlyph>,
}

/// A single glyph in a text layout.
#[derive(Debug, Clone)]
pub struct LayoutGlyph {
    /// Character this glyph represents
    pub character: char,
    /// X position relative to text origin
    pub x: f32,
    /// Y position relative to text origin (baseline)
    pub y: f32,
    /// Glyph info with bitmap data
    pub info: GlyphInfo,
}

/// The line height a font reports at `font_size` (`new_line_size`), with the one
/// shared fallback when the font has no metrics. Both layout and measurement use it.
pub(super) fn text_height(font: &Font, font_size: f32) -> f32 {
    font.horizontal_line_metrics(font_size)
        .map(|m| m.new_line_size)
        .unwrap_or(font_size * 1.2)
}

/// Layout a string of text, returning positions and glyph info for each character.
///
/// Glyphs are pulled from (and inserted into) `cache` so repeated layout of
/// the same font/size combination never re-rasterizes.
///
/// Coordinate system:
/// - The text origin (position.y) is at the BASELINE
/// - glyph.y is the offset from baseline to glyph top (negative = above baseline)
/// - The rendering code subtracts glyph.y from position.y to place the glyph correctly
pub(super) fn layout_text(
    font: &Font,
    font_id: u32,
    cache: &mut GlyphCache,
    text: &str,
    font_size: f32,
) -> Result<TextLayout, FontError> {
    let mut glyphs = Vec::new();
    let mut cursor_x = 0.0f32;

    for character in text.chars() {
        // Handle special characters
        if character == '\n' {
            // Newlines not fully supported yet, just skip
            continue;
        }

        // Use the glyph cache, which rasterizes on miss (including spaces)
        let glyph_info = cache.get_or_rasterize(font, font_id, character, font_size)?;

        // Skip rendering for zero-width glyphs but still advance cursor
        let advance = glyph_info.rasterized.advance;

        if character != ' ' && glyph_info.rasterized.width > 0 {
            // glyph.y is the offset from baseline to glyph top
            // offset_y (ymin) from fontdue is already this: negative = above baseline
            let glyph_y = glyph_info.rasterized.offset_y;

            glyphs.push(LayoutGlyph {
                character,
                x: cursor_x + glyph_info.rasterized.offset_x,
                y: glyph_y,  // Offset from baseline (negative = above baseline)
                info: glyph_info.clone(),
            });
        }

        cursor_x += advance;
    }

    Ok(TextLayout {
        width: cursor_x,
        height: text_height(font, font_size),
        glyphs,
    })
}

/// Measure the size of a text string without rasterizing.
///
/// Uses `font.metrics()` instead of `font.rasterize()` to get advance widths
/// without the expensive bitmap generation step.
pub(super) fn measure_text(font: &Font, text: &str, font_size: f32) -> Vec2 {
    let mut width = 0.0f32;
    let height = text_height(font, font_size);

    for character in text.chars() {
        let metrics = font.metrics(character, font_size);
        width += metrics.advance_width;
    }

    Vec2::new(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FIXTURE_FONT;

    const SIZE: f32 = 32.0;

    fn fixture() -> Result<(Font, GlyphCache), FontError> {
        let font = Font::from_bytes(FIXTURE_FONT, fontdue::FontSettings::default())
            .map_err(|e| FontError::LoadError(e.to_string()))?;
        Ok((font, GlyphCache::new()))
    }

    #[test]
    fn test_layout_glyph_y_is_the_top_relative_to_the_baseline_with_down_positive() -> Result<(), FontError> {
        // The documented convention behind the "UI text y = baseline"
        // footgun: the origin is the baseline, glyph.y is the top of the
        // glyph relative to it, +Y down — so a capital's top is negative
        // and its bottom (y + height) sits on the baseline, while a
        // descender's bottom is positive.
        let (font, mut cache) = fixture()?;

        let layout = layout_text(&font, 1, &mut cache, "Hg", SIZE)?;

        let capital = &layout.glyphs[0];
        let capital_bottom = capital.y + capital.info.rasterized.height as f32;
        assert!(capital.y < -SIZE * 0.5, "'H' top is well above the baseline: {}", capital.y);
        assert!(capital_bottom.abs() <= 1.5, "'H' sits on the baseline, bottom at {capital_bottom}");
        let descender = &layout.glyphs[1];
        let descender_bottom = descender.y + descender.info.rasterized.height as f32;
        assert!(descender_bottom > 2.0, "'g' descends below the baseline, bottom at {descender_bottom}");
        Ok(())
    }

    #[test]
    fn test_layout_skips_spaces_but_advances_the_cursor_past_them() -> Result<(), FontError> {
        let (font, mut cache) = fixture()?;

        let layout = layout_text(&font, 1, &mut cache, "a b", SIZE)?;

        let characters: Vec<char> = layout.glyphs.iter().map(|g| g.character).collect();
        assert_eq!(characters, vec!['a', 'b'], "a space draws nothing");
        assert!(
            layout.glyphs[1].x > measure_text(&font, "a", SIZE).x,
            "'b' starts past the space's advance: {} vs {}",
            layout.glyphs[1].x,
            measure_text(&font, "a", SIZE).x
        );
        assert_eq!(layout.width, measure_text(&font, "a b", SIZE).x, "the space still counts toward the width");
        Ok(())
    }

    #[test]
    fn test_measure_text_sums_advances_and_matches_the_laid_out_width() -> Result<(), FontError> {
        let (font, mut cache) = fixture()?;

        let word = measure_text(&font, "Hello", SIZE);
        let per_char: f32 = "Hello".chars().map(|c| measure_text(&font, &c.to_string(), SIZE).x).sum();
        assert!((word.x - per_char).abs() < 1e-3, "width is the sum of advances: {} vs {per_char}", word.x);
        assert_eq!(layout_text(&font, 1, &mut cache, "Hello", SIZE)?.width, word.x, "layout and measure agree");
        assert_eq!(measure_text(&font, "", SIZE), Vec2::new(0.0, word.y), "empty text has no width, one line of height");
        Ok(())
    }

    #[test]
    fn test_measured_height_matches_laid_out_height_within_a_pixel_for_descenders() -> Result<(), FontError> {
        // Both layout and measurement share `text_height`, so laid-out height
        // equals measured height exactly, even for descender-tall glyphs.
        let (font, mut cache) = fixture()?;

        let laid_out = layout_text(&font, 1, &mut cache, "gjpqy", SIZE)?;
        let measured = measure_text(&font, "gjpqy", SIZE);

        assert_eq!(
            laid_out.height,
            measured.y,
            "laid out {} vs measured {}",
            laid_out.height,
            measured.y
        );
        assert_eq!(layout_text(&font, 1, &mut cache, "", SIZE)?.height, measured.y, "empty text is still one line tall");
        Ok(())
    }

}
