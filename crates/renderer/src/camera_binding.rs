//! Camera uniform buffer and bind group management.

use common::camera::{Camera, CameraUniform};
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device, Queue};

/// One camera uniform: buffer + bind-group layout + bind group, uploaded with `update`.
pub struct CameraBinding {
    buffer: Buffer,
    layout: BindGroupLayout,
    bind_group: BindGroup,
}

impl CameraBinding {
    /// Create a new camera uniform buffer and bind group.
    pub fn new(device: &Device, label: &str) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label} Camera Uniform Buffer")),
            size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Camera Bind Group Layout")),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label} Camera Bind Group")),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            layout,
            bind_group,
        }
    }

    /// The bind group layout for pipeline creation.
    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// The bind group for binding in a render pass.
    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    /// Update the camera uniform buffer on the GPU.
    pub fn update(&self, queue: &Queue, camera: &Camera) {
        let uniform = CameraUniform::from_camera(camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Bind the camera uniform group to the render pass at `index`.
    pub fn bind<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, index: u32) {
        pass.set_bind_group(index, &self.bind_group, &[]);
    }
}
