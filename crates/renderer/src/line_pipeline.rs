//! Line render pipeline.
//!
//! Used by the engine's spring-mass grid to draw glowing lines on top of the
//! HDR target. Bloom picks up emissive lines automatically — that's the
//! signature Geometry Wars look.
//!
//! The pipeline declares its own camera bind-group layout (identical in shape
//! to the sprite pipeline's) and owns its own camera uniform buffer, so the
//! camera is uploaded once per pipeline per frame. Extracting a shared
//! `CameraBinding` is tracked in TECH_DEBT.md.


use wgpu::{
    util::DeviceExt, BindGroup, Buffer, CommandEncoder, Device, Queue, RenderPipeline,
};

use crate::render_targets::{DEPTH_FORMAT, HDR_FORMAT, RenderTargets};
use crate::sprite_data::{Camera, CameraUniform, DynamicBuffer};

/// One vertex of a line segment.
///
/// Two adjacent vertices in the buffer form a `LineList` primitive.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub emissive: f32,
}

impl LineVertex {
    pub fn new(position: glam::Vec2, color: glam::Vec4, emissive: f32) -> Self {
        Self {
            position: position.to_array(),
            color: color.to_array(),
            emissive,
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

/// Render pipeline + buffers for drawing dynamic line geometry.
pub struct LinePipeline {
    pipeline: RenderPipeline,
    vertex_buffer: DynamicBuffer<LineVertex>,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    /// Used to grow the vertex buffer when an upload exceeds its capacity.
    /// A plain clone — wgpu's `Device` is internally reference-counted, so
    /// wrapping it in an `Arc` was a redundant refcount (and `Arc<Device>`
    /// trips clippy's `arc_with_non_send_sync` on wasm, where `Device` is
    /// not `Send`).
    device: Device,
}

impl LinePipeline {
    /// Initial number of vertices (not segments) the dynamic vertex buffer
    /// holds; it grows on demand. Default is generous — a 60×40 grid with
    /// horizontal + vertical springs is only ~9k vertices.
    pub const DEFAULT_CAPACITY: usize = 16_384;

    pub fn new(device: &Device, capacity: usize) -> Self {

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Line Camera Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/line.wgsl"))),
        });

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Camera Buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::from_camera(&Camera::default())]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Line Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = DynamicBuffer::new(device, capacity, wgpu::BufferUsages::VERTEX);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            // Test against the depth buffer the sprite pipeline wrote into,
            // but don't write — lines are 2D background and shouldn't occlude
            // each other.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
            multiview_mask: None,
        });

        Self {
            pipeline,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
            device: device.clone(),
        }
    }

    /// Push the camera uniform to the GPU. Call once per frame.
    pub fn update_camera(&self, queue: &Queue, camera: &Camera) {
        let uniform = CameraUniform::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Upload a fresh vertex set. Pairs of vertices form line segments.
    /// The vertex buffer grows automatically if the set exceeds its capacity.
    pub fn upload_vertices(&mut self, queue: &Queue, vertices: &[LineVertex]) {
        if vertices.is_empty() {
            return;
        }
        self.vertex_buffer.update(&self.device, queue, vertices);
    }

    /// Draw the uploaded vertices into the HDR target, with depth-test
    /// against the existing depth buffer.
    ///
    /// `load_color = false` clears the HDR color before drawing; `true`
    /// preserves whatever the sprite pipeline drew (typical case — lines
    /// composite on top of the sprite frame).
    ///
    /// `viewport_scissor` bounds the pass (lines are game geometry — the
    /// editor clips them to the scene panel, issue #41); an empty effective
    /// scissor skips the pass entirely (both attachments are `Load`, so
    /// skipping changes nothing).
    pub fn draw(
        &self,
        encoder: &mut CommandEncoder,
        targets: &RenderTargets,
        vertex_count: u32,
        viewport_scissor: Option<[u32; 4]>,
    ) {
        if vertex_count == 0 {
            return;
        }
        let surface = (targets.width(), targets.height());
        let scissor = match viewport_scissor {
            None => None,
            Some(rect) => match crate::scissor::clamp_scissor(rect, surface.0, surface.1) {
                Some(clamped) => Some(clamped),
                // Empty scissor: nothing visible, skip the whole pass.
                None => return,
            },
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Line Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.hdr_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    // Lines are drawn after sprites and must NOT clear.
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        if let Some((x, y, w, h)) = scissor {
            pass.set_scissor_rect(x, y, w, h);
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice());
        pass.draw(0..vertex_count, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec4};

    #[test]
    fn line_vertex_layout_size() {
        // 2*4 + 4*4 + 1*4 = 28 bytes.
        assert_eq!(std::mem::size_of::<LineVertex>(), 28);
        let desc = LineVertex::desc();
        assert_eq!(desc.step_mode, wgpu::VertexStepMode::Vertex);
        assert_eq!(desc.attributes.len(), 3);
        assert_eq!(desc.array_stride, 28);
    }

    #[test]
    fn line_vertex_new_populates_fields() {
        let v = LineVertex::new(Vec2::new(1.0, 2.0), Vec4::new(0.5, 0.5, 0.5, 1.0), 1.2);
        assert_eq!(v.position, [1.0, 2.0]);
        assert_eq!(v.color, [0.5, 0.5, 0.5, 1.0]);
        assert!((v.emissive - 1.2).abs() < 1e-5);
    }
}
