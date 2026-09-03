//! Offscreen render targets for the HDR + bloom pipeline.
//!
//! The renderer no longer writes sprites directly to the swapchain. Instead it
//! draws into an HDR color buffer (paired with a depth buffer) and then a
//! bloom pass downsamples the bright regions, blurs them, and composites the
//! result to the swapchain.
//!
//! This module owns the lifetime of those intermediate textures and rebuilds
//! them on window resize.

use wgpu::{Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor};

/// Format used for the HDR color buffer. 16-bit float per channel keeps
/// bright values above 1.0 so the bloom pipeline can find them.
pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

/// Format for the depth buffer attached to the HDR target.
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// Half-resolution scale for the bloom textures — saves bandwidth and gives
/// a softer, wider glow than full-res blur.
const BLOOM_DOWNSAMPLE: u32 = 2;

/// Owns the HDR color, depth, and bloom ping-pong textures.
///
/// The textures are recreated whenever the surface resizes — see
/// [`resize`](Self::resize).
pub struct RenderTargets {
    /// Full-resolution HDR color target. Sprites + lines write here.
    pub hdr_color: Texture,
    pub hdr_view: TextureView,

    /// Depth buffer paired with the HDR color target.
    pub depth: Texture,
    pub depth_view: TextureView,

    /// Half-resolution bloom textures used in ping-pong blur.
    /// `bloom_ping` is also the final source consumed by the composite pass.
    pub bloom_ping: Texture,
    pub bloom_ping_view: TextureView,
    pub bloom_pong: Texture,
    pub bloom_pong_view: TextureView,

    width: u32,
    height: u32,
}

/// Compute the half-resolution bloom texture dimensions from surface dimensions,
/// clamped to at least 1x1.
pub const fn bloom_dims(width: u32, height: u32) -> (u32, u32) {
    let w = width / BLOOM_DOWNSAMPLE;
    let h = height / BLOOM_DOWNSAMPLE;
    let w = if w > 0 { w } else { 1 };
    let h = if h > 0 { h } else { 1 };
    (w, h)
}

impl RenderTargets {
    /// Build a fresh set of render targets sized for the given surface.
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let (hdr_color, hdr_view) = create_hdr_color(device, width, height);
        let (depth, depth_view) = create_depth(device, width, height);
        let (bloom_w, bloom_h) = bloom_dims(width, height);
        let (bloom_ping, bloom_ping_view) = create_bloom_tex(device, bloom_w, bloom_h, "Bloom Ping");
        let (bloom_pong, bloom_pong_view) = create_bloom_tex(device, bloom_w, bloom_h, "Bloom Pong");

        Self {
            hdr_color,
            hdr_view,
            depth,
            depth_view,
            bloom_ping,
            bloom_ping_view,
            bloom_pong,
            bloom_pong_view,
            width,
            height,
        }
    }

    /// Recreate every texture at the new surface size. Cheap if the size
    /// hasn't actually changed.
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        *self = Self::new(device, width, height);
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    /// Width of the bloom textures (half of the surface width).
    pub fn bloom_width(&self) -> u32 { bloom_dims(self.width, self.height).0 }
    /// Height of the bloom textures (half of the surface height).
    pub fn bloom_height(&self) -> u32 { bloom_dims(self.width, self.height).1 }
}

fn create_hdr_color(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("HDR Color"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: HDR_FORMAT,
        // RENDER_ATTACHMENT to draw into, TEXTURE_BINDING so bloom can sample it.
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

fn create_depth(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("Depth"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

fn create_bloom_tex(device: &Device, width: u32, height: u32, label: &str) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: HDR_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_dims_halves_and_clamps_to_one() {
        assert_eq!(bloom_dims(800, 600), (400, 300));
        assert_eq!(bloom_dims(1920, 1080), (960, 540));
        assert_eq!(bloom_dims(1, 1), (1, 1));
        assert_eq!(bloom_dims(0, 0), (1, 1));
    }
}
