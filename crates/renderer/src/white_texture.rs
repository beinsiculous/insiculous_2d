//! The built-in 1x1 white texture used for flat-colored sprites
//! ([`crate::texture::TextureHandle::WHITE`]).

use wgpu::{Device, Queue};

use crate::sprite_data::TextureResource;

/// Create the white texture resource for colored sprites (multiply by white
/// instead of transparent black).
pub(crate) fn create_white_texture_resource(device: &Device, queue: &Queue) -> TextureResource {
    use std::sync::Arc;

    log::info!("Creating white texture resource for colored sprites");

    let texture = Arc::new(device.create_texture(&wgpu::TextureDescriptor {
        label: Some("White Texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    }));

    let white_pixel: [u8; 4] = [255, 255, 255, 255];

    queue.write_texture(
        texture.as_image_copy(),
        &white_pixel,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    TextureResource::new(device, texture)
}
