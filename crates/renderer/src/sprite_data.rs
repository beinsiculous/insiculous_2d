//! Sprite data structures and management

use glam::{Vec2, Vec3, Vec4};
use std::sync::Arc;
use wgpu::{Device, Queue, Texture, TextureView, Sampler, Buffer};
use crate::texture::SamplerConfig;

// Re-export Camera2D and CameraUniform from common crate
pub use common::{Camera, camera::CameraUniform};

/// Vertex data for a sprite
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    /// Position in world space
    pub position: [f32; 3],
    /// Texture coordinates
    pub tex_coords: [f32; 2],
    /// Color tint (RGBA as 4 floats)
    pub color: [f32; 4],
}

impl SpriteVertex {
    /// Create a new sprite vertex
    pub fn new(position: Vec3, tex_coords: Vec2, color: Vec4) -> Self {
        Self {
            position: position.to_array(),
            tex_coords: tex_coords.to_array(),
            color: color.to_array(),
        }
    }

    /// Get the vertex buffer layout
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Texture coordinates
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Sprite instance data for batching
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    /// World position
    pub position: [f32; 2],
    /// Rotation in radians
    pub rotation: f32,
    /// Scale
    pub scale: [f32; 2],
    /// Texture region (x, y, width, height) in texture coordinates [0, 1]
    pub tex_region: [f32; 4],
    /// Color tint
    pub color: [f32; 4],
    /// Layer depth for sorting (and depth-test once HDR pipeline is enabled)
    pub depth: f32,
    /// Emissive intensity — 0.0 = no glow, >0.0 amplifies RGB above 1.0 so bloom picks it up
    pub emissive: f32,
    /// SDF shape parameters: `[kind, corner_radius, border_width, reserved]`.
    /// kind 0 = plain textured quad (default, zeroed == legacy behavior),
    /// 1 = rounded rect, 2 = circle. Radius/border are in local pixels.
    pub shape: [f32; 4],
}

impl SpriteInstance {
    /// Create a new sprite instance with no emission
    pub fn new(
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        tex_region: [f32; 4],
        color: Vec4,
        depth: f32,
    ) -> Self {
        Self::with_emissive(position, rotation, scale, tex_region, color, depth, 0.0)
    }

    /// Create a new sprite instance with explicit emissive intensity
    pub fn with_emissive(
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        tex_region: [f32; 4],
        color: Vec4,
        depth: f32,
        emissive: f32,
    ) -> Self {
        Self {
            position: position.to_array(),
            rotation,
            scale: scale.to_array(),
            tex_region,
            color: color.to_array(),
            depth,
            emissive,
            shape: [0.0; 4],
        }
    }

    /// Set the SDF shape parameters (builder-style).
    pub fn with_shape(mut self, shape: [f32; 4]) -> Self {
        self.shape = shape;
        self
    }

    /// Get the instance buffer layout
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Rotation
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                // Scale
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Texture region
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Depth
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 13]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32,
                },
                // Emissive intensity
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 14]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32,
                },
                // SDF shape params [kind, corner_radius, border_width, reserved]
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 15]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// Note: Camera2D and CameraUniform are now re-exported from common crate
// This eliminates ~100 lines of duplicated code

/// A texture with its view and sampler
#[derive(Debug, Clone)]
pub struct TextureResource {
    pub texture: Arc<Texture>,
    pub view: TextureView,
    pub sampler: Sampler,
    pub width: u32,
    pub height: u32,
}

impl TextureResource {
    /// Create a new texture resource from existing texture
    pub fn new(device: &Device, texture: Arc<Texture>) -> Self {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = SamplerConfig::default().create_sampler(device, Some("Texture Sampler"));

        let size = texture.size();
        
        Self {
            texture,
            view,
            sampler,
            width: size.width,
            height: size.height,
        }
    }

}

/// Dynamic buffer for sprite data.
///
/// Grows on demand: when [`update`](Self::update) receives more elements than
/// the current capacity, the GPU buffer is recreated at the next power of two
/// and the capacity updated. It never shrinks.
pub struct DynamicBuffer<T> {
    buffer: Buffer,
    capacity: usize,
    usage: wgpu::BufferUsages,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod> DynamicBuffer<T> {
    /// Create a new dynamic buffer with the given initial capacity (in elements)
    pub fn new(device: &Device, capacity: usize, usage: wgpu::BufferUsages) -> Self {
        Self {
            buffer: Self::create_buffer(device, capacity, usage),
            capacity,
            usage,
            _phantom: std::marker::PhantomData,
        }
    }

    fn create_buffer(device: &Device, capacity: usize, usage: wgpu::BufferUsages) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Dynamic Buffer<{}>", std::any::type_name::<T>())),
            size: (capacity * std::mem::size_of::<T>()) as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Update buffer data, growing the GPU buffer if `data` exceeds capacity
    pub fn update(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        if data.len() > self.capacity {
            let new_capacity = data.len().next_power_of_two();
            log::debug!(
                "Growing Dynamic Buffer<{}> from {} to {} elements",
                std::any::type_name::<T>(),
                self.capacity,
                new_capacity
            );
            self.buffer = Self::create_buffer(device, new_capacity, self.usage);
            self.capacity = new_capacity;
        }

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    /// Get buffer slice
    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }

    /// Get buffer
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::vertex_attribute;
    use std::mem::{offset_of, size_of};
    use wgpu::{VertexFormat, VertexStepMode};

    /// `shaders/sprite_instanced.wgsl` reads these buffers by stride and the
    /// camera uniform by size; a struct that grows a field without the shader
    /// following it renders garbage with no error.
    #[test]
    fn test_gpu_structs_match_shader_layout() {
        let vertex = SpriteVertex::desc();
        let instance = SpriteInstance::desc();

        assert_eq!(size_of::<SpriteVertex>(), 36, "3 + 2 + 4 floats");
        assert_eq!(vertex.array_stride, 36);
        assert_eq!(vertex.step_mode, VertexStepMode::Vertex);

        assert_eq!(size_of::<SpriteInstance>(), 76, "2 + 1 + 2 + 4 + 4 + 1 + 1 + 4 floats");
        assert_eq!(instance.array_stride, 76);
        assert_eq!(instance.step_mode, VertexStepMode::Instance);

        assert_eq!(size_of::<CameraUniform>(), 80, "mat4x4 + vec2 position + vec2 padding");
    }

    /// Guard: the eleven attribute offsets are hand-written as
    /// `size_of::<[f32; N]>()`. Swapping two keeps count and stride intact,
    /// compiles, and renders sprites at the wrong depth with the wrong glow.
    /// Every `(location, offset, format)` triple is pinned to the struct
    /// field that feeds it and the `@location` the shader declares.
    #[test]
    fn test_sprite_attributes_match_shader_locations() {
        let expected_vertex = [
            vertex_attribute(0, offset_of!(SpriteVertex, position), VertexFormat::Float32x3),
            vertex_attribute(1, offset_of!(SpriteVertex, tex_coords), VertexFormat::Float32x2),
            vertex_attribute(2, offset_of!(SpriteVertex, color), VertexFormat::Float32x4),
        ];
        let expected_instance = [
            vertex_attribute(3, offset_of!(SpriteInstance, position), VertexFormat::Float32x2),
            vertex_attribute(4, offset_of!(SpriteInstance, rotation), VertexFormat::Float32),
            vertex_attribute(5, offset_of!(SpriteInstance, scale), VertexFormat::Float32x2),
            vertex_attribute(6, offset_of!(SpriteInstance, tex_region), VertexFormat::Float32x4),
            vertex_attribute(7, offset_of!(SpriteInstance, color), VertexFormat::Float32x4),
            vertex_attribute(8, offset_of!(SpriteInstance, depth), VertexFormat::Float32),
            vertex_attribute(9, offset_of!(SpriteInstance, emissive), VertexFormat::Float32),
            vertex_attribute(10, offset_of!(SpriteInstance, shape), VertexFormat::Float32x4),
        ];

        assert_eq!(SpriteVertex::desc().attributes, &expected_vertex[..]);
        assert_eq!(SpriteInstance::desc().attributes, &expected_instance[..]);
    }
}
