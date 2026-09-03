//! Glyph texture cache for UI text rendering.
//!
//! Scans UI draw commands for text glyphs and creates one GPU texture per
//! unique glyph bitmap, caching handles across frames so each glyph is only
//! uploaded once.

use std::collections::HashMap;

use renderer::texture::TextureHandle;
use ui::{DrawCommand, GlyphDrawData};

use crate::assets::AssetManager;
use crate::contexts::GlyphCacheKey;

/// Caches one texture per unique glyph so text rendering reuses GPU
/// textures across frames.
///
/// Cache keys are color-agnostic: glyph textures are grayscale alpha masks
/// and the color is applied at render time (see [`GlyphCacheKey`]).
#[derive(Default)]
pub struct GlyphTextureCache {
    textures: HashMap<GlyphCacheKey, TextureHandle>,
}

impl GlyphTextureCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// The cached glyph textures, keyed for lookup during UI rendering.
    pub fn textures(&self) -> &HashMap<GlyphCacheKey, TextureHandle> {
        &self.textures
    }

    /// Create textures for any glyphs in `commands` that are not cached yet.
    ///
    /// Called once per frame before rendering. Glyphs already in the cache
    /// (including duplicates within the same command list) are skipped.
    pub fn prepare(&mut self, commands: &[DrawCommand], assets: &mut AssetManager) {
        let missing = self.uncached_glyphs(commands);
        for (key, glyph) in missing {
            // Re-check: the same glyph can appear more than once per frame,
            // and the first occurrence has already created the texture.
            if self.textures.contains_key(&key) {
                continue;
            }

            // Create glyph texture (grayscale alpha mask)
            match assets.create_glyph_texture(glyph.width, glyph.height, &glyph.bitmap) {
                Ok(handle) => {
                    self.textures.insert(key, handle);
                }
                Err(e) => {
                    log::warn!("Failed to create glyph texture for '{}': {}", glyph.character, e);
                }
            }
        }
    }

    /// Collect glyphs from `commands` that have no cached texture yet,
    /// in command order. Duplicates are not removed here; `prepare` skips
    /// them once the first occurrence has been created.
    fn uncached_glyphs<'a>(
        &self,
        commands: &'a [DrawCommand],
    ) -> Vec<(GlyphCacheKey, &'a GlyphDrawData)> {
        Self::renderable_glyphs(commands)
            .filter(|(key, _)| !self.textures.contains_key(key))
            .collect()
    }

    /// Iterate all glyphs in Text commands that need a texture to render
    /// (skips empty glyphs such as spaces).
    fn renderable_glyphs(
        commands: &[DrawCommand],
    ) -> impl Iterator<Item = (GlyphCacheKey, &GlyphDrawData)> {
        commands
            .iter()
            .filter_map(|cmd| match cmd {
                DrawCommand::Text { data, .. } => {
                    Some(data.glyphs.iter().map(|g| (data.font_id, g)))
                }
                _ => None,
            })
            .flatten()
            .filter(|(_, glyph)| glyph.width > 0 && glyph.height > 0 && !glyph.bitmap.is_empty())
            .map(|(font_id, glyph)| {
                (
                    GlyphCacheKey::new(glyph.character, glyph.width, glyph.height, font_id),
                    glyph,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{Color, Rect};
    use glam::Vec2;
    use std::sync::Arc;
    use ui::TextDrawData;

    fn glyph(character: char, width: u32, height: u32, bitmap: &[u8]) -> GlyphDrawData {
        GlyphDrawData {
            bitmap: Arc::from(bitmap),
            width,
            height,
            x: 0.0,
            y: 0.0,
            character,
        }
    }

    fn text_command(glyphs: Vec<GlyphDrawData>, font_id: u32) -> DrawCommand {
        DrawCommand::Text {
            data: TextDrawData {
                text: String::new(),
                position: Vec2::ZERO,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
                font_size: 14.0,
                font_id,
                width: 0.0,
                height: 0.0,
                glyphs,
            },
            depth: 0.0,
        }
    }

    #[test]
    fn same_glyph_same_size_different_fonts_needs_separate_textures() {
        let mut cache = GlyphTextureCache::new();
        // 'a' at 4x4 in font 1 already has a texture.
        cache
            .textures
            .insert(GlyphCacheKey::new('a', 4, 4, 1), TextureHandle { id: 7 });
        let commands = vec![
            DrawCommand::Rect {
                bounds: Rect::new(0.0, 0.0, 10.0, 10.0),
                color: Color::new(1.0, 1.0, 1.0, 1.0),
                corner_radius: 0.0,
                depth: 0.0,
            },
            text_command(
                vec![
                    glyph('a', 4, 4, &[255; 16]), // cached: (char, size, font) hit
                    glyph('a', 8, 8, &[255; 64]), // same char, other size
                    glyph(' ', 0, 0, &[]),        // space: zero size, no bitmap
                    glyph('b', 4, 4, &[]),        // empty bitmap
                ],
                1,
            ),
            text_command(vec![glyph('a', 4, 4, &[128; 16])], 2), // same char+size, other font
        ];

        let missing = cache.uncached_glyphs(&commands);

        // The key is (char, size, font): the cached hit and the two
        // unrenderable glyphs are skipped, the other size and the other
        // font each need their own texture.
        let keys: Vec<(char, u32, u32)> = missing
            .iter()
            .map(|(_, glyph)| (glyph.character, glyph.width, glyph.height))
            .collect();
        assert_eq!(keys, [('a', 8, 8), ('a', 4, 4)]);
        assert_eq!(cache.textures().len(), 1, "reporting does not create textures");
    }
}
