//! Line render pipeline.
//!
//! Used by the engine's spring-mass grid to draw glowing lines on top of the
//! HDR target. Bloom picks up emissive lines automatically — that's the
//! signature Geometry Wars look.

use wgpu::{CommandEncoder, Device, Queue, RenderPipeline};

use crate::camera_binding::CameraBinding;
use crate::render_targets::{HDR_FORMAT, RenderTargets};
use crate::scissor::PassScissor;
use crate::sprite_data::{Camera, DynamicBuffer};

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
        const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x4,
            2 => Float32,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

/// Render pipeline + buffers for drawing dynamic line geometry.
pub struct LinePipeline {
    pipeline: RenderPipeline,
    vertex_buffer: DynamicBuffer<LineVertex>,
    camera: CameraBinding,
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
        let camera = CameraBinding::new(device, "Line");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[camera.layout()],
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/line.wgsl"))),
        });

        let vertex_buffer = DynamicBuffer::new(device, capacity, wgpu::BufferUsages::VERTEX);

        let pipeline = crate::pipeline_builder::build_render_pipeline(
            device,
            &crate::pipeline_builder::PipelineSpec {
                label: "Line Pipeline",
                layout: &pipeline_layout,
                shader: &shader,
                vertex_entry: "vs_main",
                fragment_entry: "fs_main",
                buffers: &[LineVertex::desc()],
                topology: wgpu::PrimitiveTopology::LineList,
                target: wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                },
                depth: Some(crate::pipeline_builder::depth_state(false)),
            },
        );

        Self {
            pipeline,
            vertex_buffer,
            camera,
            device: device.clone(),
        }
    }

    /// Push the camera uniform to the GPU. Call once per frame.
    pub fn update_camera(&self, queue: &Queue, camera: &Camera) {
        self.camera.update(queue, camera);
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
    /// editor clips them to the scene panel); an empty effective
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
        let scissor = PassScissor::resolve(viewport_scissor, (targets.width(), targets.height()));
        let scissor_rect = match scissor {
            PassScissor::Empty => return,
            PassScissor::Fullscreen => None,
            PassScissor::Rect(rect) => Some(rect),
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

        if let Some(rect) = scissor_rect {
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
        }
        pass.set_pipeline(&self.pipeline);
        self.camera.bind(&mut pass, 0);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice());
        pass.draw(0..vertex_count, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::vertex_attribute;
    use std::mem::{offset_of, size_of};
    use wgpu::{VertexFormat, VertexStepMode};

    /// `shaders/line.wgsl` reads a 28-byte vertex at locations 0–2; the
    /// offsets are pinned to the struct field that feeds it.
    #[test]
    fn test_line_vertex_matches_shader_layout() {
        let desc = LineVertex::desc();
        let expected = [
            vertex_attribute(0, offset_of!(LineVertex, position), VertexFormat::Float32x2),
            vertex_attribute(1, offset_of!(LineVertex, color), VertexFormat::Float32x4),
            vertex_attribute(2, offset_of!(LineVertex, emissive), VertexFormat::Float32),
        ];

        assert_eq!(size_of::<LineVertex>(), 28, "2 + 4 + 1 floats");
        assert_eq!(desc.array_stride, 28);
        assert_eq!(desc.step_mode, VertexStepMode::Vertex);
        assert_eq!(desc.attributes, &expected[..]);
    }
}
