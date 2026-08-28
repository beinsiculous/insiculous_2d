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

    #[test]
    fn test_texture_filter_defaults_to_linear() {
        assert_eq!(TextureFilter::default(), TextureFilter::Linear);
    }

    #[test]
    fn test_linear_filter_maps_every_sampler_filter_to_linear() {
        let config: SamplerConfig = TextureFilter::Linear.into();
        assert_eq!(config.mag_filter, wgpu::FilterMode::Linear);
        assert_eq!(config.min_filter, wgpu::FilterMode::Linear);
        assert_eq!(config.mipmap_filter, wgpu::MipmapFilterMode::Linear);
    }

    #[test]
    fn test_nearest_filter_maps_every_sampler_filter_to_nearest() {
        let config: SamplerConfig = TextureFilter::Nearest.into();
        assert_eq!(config.mag_filter, wgpu::FilterMode::Nearest);
        assert_eq!(config.min_filter, wgpu::FilterMode::Nearest);
        assert_eq!(config.mipmap_filter, wgpu::MipmapFilterMode::Nearest);
    }

    #[test]
    fn test_texture_filter_leaves_other_sampler_fields_at_default() {
        let config: SamplerConfig = TextureFilter::Nearest.into();
        let defaults = SamplerConfig::default();
        assert_eq!(config.address_mode_u, defaults.address_mode_u);
        assert_eq!(config.lod_max_clamp, defaults.lod_max_clamp);
        assert_eq!(config.anisotropy_clamp, defaults.anisotropy_clamp);
    }
}
