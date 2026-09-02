//! Texture filtering mode — the pixel-art knob.
//!
//! Split out of [`crate::texture`] so the filter policy (a tiny, serde-free,
//! GPU-free enum that engine_core plumbs from `GameConfig` and `.sheet.ron`
//! sidecars) is not buried in the texture manager.

use crate::texture::SamplerConfig;

/// How a texture is filtered when magnified or minified.
///
/// `Linear` blends between texels — the right choice for photographic art and
/// UI glyphs. `Nearest` keeps texel edges hard, which pixel art and tileset
/// strips require: linear filtering bleeds neighbouring cells across tile
/// borders.
///
/// Convert to a [`SamplerConfig`] to apply it:
/// ```
/// use renderer::{SamplerConfig, TextureFilter};
///
/// let config: SamplerConfig = TextureFilter::Nearest.into();
/// assert_eq!(config.mag_filter, renderer::wgpu::FilterMode::Nearest);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureFilter {
    /// Smooth blending between texels (default).
    #[default]
    Linear,
    /// Hard texel edges — pixel art, tilesets, palettes.
    Nearest,
}

impl From<TextureFilter> for SamplerConfig {
    /// Build a sampler config whose magnification, minification and mipmap
    /// filters all agree. Every other field keeps its [`SamplerConfig`]
    /// default.
    fn from(filter: TextureFilter) -> Self {
        let (filter_mode, mipmap_filter) = match filter {
            TextureFilter::Linear => {
                (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Linear)
            }
            TextureFilter::Nearest => {
                (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest)
            }
        };
        Self {
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nearest is the pixel-art knob: mag, min AND mipmap must all agree, or
    /// a tileset strip bleeds across cell borders on whichever one is still
    /// linear. Every other sampler field stays at its default.
    #[test]
    fn test_filter_sets_every_sampler_filter_and_nothing_else() {
        let cases = [
            (TextureFilter::Linear, wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Linear),
            (TextureFilter::Nearest, wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest),
        ];
        let defaults = SamplerConfig::default();

        for (filter, expected_filter, expected_mipmap) in cases {
            let config: SamplerConfig = filter.into();

            assert_eq!(config.mag_filter, expected_filter, "{filter:?} mag");
            assert_eq!(config.min_filter, expected_filter, "{filter:?} min");
            assert_eq!(config.mipmap_filter, expected_mipmap, "{filter:?} mipmap");
            assert_eq!(config.address_mode_u, defaults.address_mode_u, "{filter:?}");
            assert_eq!(config.address_mode_v, defaults.address_mode_v, "{filter:?}");
            assert_eq!(config.address_mode_w, defaults.address_mode_w, "{filter:?}");
            assert_eq!(config.lod_min_clamp, defaults.lod_min_clamp, "{filter:?}");
            assert_eq!(config.lod_max_clamp, defaults.lod_max_clamp, "{filter:?}");
            assert_eq!(config.compare, defaults.compare, "{filter:?}");
            assert_eq!(config.anisotropy_clamp, defaults.anisotropy_clamp, "{filter:?}");
        }
    }
}
