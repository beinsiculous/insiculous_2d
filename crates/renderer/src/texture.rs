//! Texture loading and management system
//!
//! Provides comprehensive texture loading, caching, and management for the renderer.
//! Supports loading from files (PNG, JPEG, BMP, GIF) and raw RGBA data.

use std::sync::Arc;
use std::path::Path;
use std::collections::HashMap;
use wgpu::{Device, Queue, Sampler, TextureFormat, Extent3d};
use thiserror::Error;
use image::GenericImageView;

use crate::sprite_data::TextureResource;

/// Texture loading errors
#[derive(Debug, Error)]
pub enum TextureError {
    #[error("Failed to load image: {0}")]
    ImageLoadError(String),
    #[error("Texture not found: {0}")]
    TextureNotFound(String),
    #[error("Invalid texture format")]
    InvalidFormat,
    #[error("Texture too large: {width}x{height}, max: {max_dimension}")]
    TextureTooLarge { width: u32, height: u32, max_dimension: u32 },
}

/// Handle to a loaded texture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
pub struct TextureHandle {
    pub id: u32,
}

impl TextureHandle {
    /// Reserved handle for the renderer's built-in 1x1 white texture.
    /// Sprites that want a flat color multiply their tint against it.
    /// [`TextureManager`] starts allocating real handles at 1 to keep this free.
    pub const WHITE: Self = Self { id: 0 };

    /// Create a new texture handle
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}


/// Texture loading configuration
///
/// Note: mipmap generation is intentionally not offered. An earlier
/// `generate_mipmaps` flag allocated a mip chain but never filled it, so
/// minified sprites sampled uninitialized levels. Add it back only together
/// with actual mip generation.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct TextureLoadConfig {
    /// Texture format (None to auto-detect)
    pub format: Option<TextureFormat>,
    /// Sampler configuration
    pub sampler_config: SamplerConfig,
}


/// Sampler configuration
#[derive(Debug, Clone)]
pub struct SamplerConfig {
    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::MipmapFilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<wgpu::CompareFunction>,
    pub anisotropy_clamp: u16,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp: 1,
        }
    }
}

impl SamplerConfig {
    /// Create a WGPU sampler from this configuration.
    ///
    /// This is the single place where samplers are created from config,
    /// eliminating duplicate sampler creation code across the crate.
    pub fn create_sampler(&self, device: &Device, label: Option<&str>) -> Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare,
            anisotropy_clamp: self.anisotropy_clamp,
            ..Default::default()
        })
    }
}

/// Texture manager for loading and caching textures
pub struct TextureManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    textures: HashMap<TextureHandle, TextureResource>,
    next_handle: u32,
    max_texture_dimension: u32,
}

impl TextureManager {
    /// Create a new texture manager
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        
        Self {
            device,
            queue,
            textures: HashMap::new(),
            next_handle: TextureHandle::WHITE.id + 1, // 0 is reserved for the white texture
            max_texture_dimension,
        }
    }

    /// Load a texture from a file path
    ///
    /// Supports PNG, JPEG, BMP, and GIF formats. The image is automatically
    /// converted to RGBA format for GPU upload.
    ///
    /// # Example
    /// ```no_run
    /// # use renderer::{TextureManager, TextureLoadConfig, TextureError};
    /// # fn load(texture_manager: &mut TextureManager) -> Result<(), TextureError> {
    /// let handle = texture_manager.load_texture("assets/player.png", TextureLoadConfig::default())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_texture<P: AsRef<Path>>(
        &mut self,
        path: P,
        config: TextureLoadConfig,
    ) -> Result<TextureHandle, TextureError> {
        let path = path.as_ref();

        log::info!("Loading texture from path: {:?}", path);

        // Load the image through the VFS seam so wasm can serve it from the
        // boot-populated map with the same synchronous call shape.
        let bytes = common::vfs::read(path).map_err(|e| {
            TextureError::ImageLoadError(format!("Failed to load {:?}: {}", path, e))
        })?;
        let img = image::load_from_memory(&bytes).map_err(|e| {
            TextureError::ImageLoadError(format!("Failed to load {:?}: {}", path, e))
        })?;

        // Get image dimensions
        let (width, height) = img.dimensions();

        // Validate dimensions
        if width > self.max_texture_dimension || height > self.max_texture_dimension {
            return Err(TextureError::TextureTooLarge {
                width,
                height,
                max_dimension: self.max_texture_dimension,
            });
        }

        // Convert to RGBA8
        let rgba = img.to_rgba8();
        let data = rgba.as_raw();

        // Create handle and texture
        let handle = TextureHandle::new(self.next_handle);
        self.next_handle += 1;

        let texture = self.create_texture_from_rgba(width, height, data, config)?;
        self.textures.insert(handle, texture);

        log::info!("Loaded texture {:?}: {}x{} (handle {})", path, width, height, handle.id);

        Ok(handle)
    }

    /// Load a texture from raw bytes (file contents)
    ///
    /// Useful for loading textures from embedded assets or network resources.
    pub fn load_texture_from_bytes(
        &mut self,
        bytes: &[u8],
        config: TextureLoadConfig,
    ) -> Result<TextureHandle, TextureError> {
        let img = image::load_from_memory(bytes).map_err(|e| {
            TextureError::ImageLoadError(format!("Failed to decode image: {}", e))
        })?;

        let (width, height) = img.dimensions();

        if width > self.max_texture_dimension || height > self.max_texture_dimension {
            return Err(TextureError::TextureTooLarge {
                width,
                height,
                max_dimension: self.max_texture_dimension,
            });
        }

        let rgba = img.to_rgba8();
        let data = rgba.as_raw();

        let handle = TextureHandle::new(self.next_handle);
        self.next_handle += 1;

        let texture = self.create_texture_from_rgba(width, height, data, config)?;
        self.textures.insert(handle, texture);

        log::info!("Loaded texture from bytes: {}x{} (handle {})", width, height, handle.id);

        Ok(handle)
    }

    /// Load a texture from raw RGBA data
    pub fn load_texture_from_rgba(
        &mut self,
        width: u32,
        height: u32,
        data: &[u8],
        config: TextureLoadConfig,
    ) -> Result<TextureHandle, TextureError> {
        if width == 0 || height == 0 {
            return Err(TextureError::InvalidFormat);
        }

        if width > self.max_texture_dimension || height > self.max_texture_dimension {
            return Err(TextureError::TextureTooLarge {
                width,
                height,
                max_dimension: self.max_texture_dimension,
            });
        }

        if data.len() != (width * height * 4) as usize {
            return Err(TextureError::InvalidFormat);
        }

        let handle = TextureHandle::new(self.next_handle);
        self.next_handle += 1;

        let texture = self.create_texture_from_rgba(width, height, data, config)?;
        self.textures.insert(handle, texture);

        Ok(handle)
    }

    /// Create a solid color texture
    pub fn create_solid_color(
        &mut self,
        width: u32,
        height: u32,
        color: [u8; 4],
    ) -> Result<TextureHandle, TextureError> {
        let data = vec![color; (width * height) as usize];
        self.load_texture_from_rgba(width, height, bytemuck::cast_slice(&data), TextureLoadConfig::default())
    }

    /// Create a checkerboard pattern texture
    pub fn create_checkerboard(
        &mut self,
        width: u32,
        height: u32,
        color1: [u8; 4],
        color2: [u8; 4],
        check_size: u32,
    ) -> Result<TextureHandle, TextureError> {
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let check_x = (x / check_size) % 2;
                let check_y = (y / check_size) % 2;
                let color = if (check_x + check_y).is_multiple_of(2) { color1 } else { color2 };
                data.extend_from_slice(&color);
            }
        }

        self.load_texture_from_rgba(width, height, &data, TextureLoadConfig::default())
    }

    /// Get a texture by handle
    pub fn get_texture(&self, handle: TextureHandle) -> Option<&TextureResource> {
        self.textures.get(&handle)
    }

    /// Remove a texture
    pub fn remove_texture(&mut self, handle: TextureHandle) -> Option<TextureResource> {
        self.textures.remove(&handle)
    }

    /// Check if a texture exists
    pub fn has_texture(&self, handle: TextureHandle) -> bool {
        self.textures.contains_key(&handle)
    }

    /// Get all texture handles
    pub fn texture_handles(&self) -> Vec<TextureHandle> {
        self.textures.keys().copied().collect()
    }

    /// Get texture count
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Get all textures as a reference to the internal HashMap
    ///
    /// Useful for passing texture resources to the renderer for sprite rendering.
    pub fn textures(&self) -> &HashMap<TextureHandle, TextureResource> {
        &self.textures
    }

    /// Create texture from RGBA data using write_texture directly with simplified layout
    fn create_texture_from_rgba(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        config: TextureLoadConfig,
    ) -> Result<TextureResource, TextureError> {
        let format = config.format.unwrap_or(TextureFormat::Rgba8UnormSrgb);
        
        let texture = Arc::new(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dynamic Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }));

        // Upload texture data using write_texture directly with simplified layout
        self.queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            texture.size(),
        );

        // Create view
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler
        let sampler = self.create_sampler(&config.sampler_config);

        Ok(TextureResource {
            texture: Arc::clone(&texture),
            view,
            sampler,
            width,
            height,
        })
    }

    /// Create sampler from config (delegates to SamplerConfig::create_sampler)
    fn create_sampler(&self, config: &SamplerConfig) -> Sampler {
        config.create_sampler(&self.device, Some("Texture Sampler"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Sprite::default()` carries `TextureHandle::default()`; it must be the
    /// built-in white so a flat-colour sprite renders its tint instead of
    /// sampling nothing. (`TextureManager` allocating from `WHITE.id + 1` is
    /// device-bound and not provable here.)
    #[test]
    fn test_default_handle_is_the_reserved_white_texture() {
        assert_eq!(TextureHandle::default(), TextureHandle::WHITE);
    }

    /// The error a game author sees names the file, the size and the limit,
    /// not just the variant.
    #[test]
    fn test_texture_errors_carry_their_operands_in_the_message() {
        let load = TextureError::ImageLoadError("Failed to load \"missing.png\": no such file".into());
        let too_large = TextureError::TextureTooLarge { width: 10000, height: 4096, max_dimension: 8192 };

        assert_eq!(
            load.to_string(),
            "Failed to load image: Failed to load \"missing.png\": no such file"
        );
        assert_eq!(too_large.to_string(), "Texture too large: 10000x4096, max: 8192");
        assert_eq!(TextureError::InvalidFormat.to_string(), "Invalid texture format");
    }
}
