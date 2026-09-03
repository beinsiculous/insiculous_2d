//! Pipeline builder helper for render pipelines.

use wgpu::{
    ColorTargetState, DepthStencilState, Device, PipelineLayout, PrimitiveTopology, RenderPipeline,
    ShaderModule, VertexBufferLayout,
};

use crate::render_targets::DEPTH_FORMAT;

pub(crate) struct PipelineSpec<'a> {
    pub label: &'a str,
    pub layout: &'a PipelineLayout,
    pub shader: &'a ShaderModule,
    pub vertex_entry: &'a str,
    pub fragment_entry: &'a str,
    pub buffers: &'a [VertexBufferLayout<'a>],
    pub topology: PrimitiveTopology,
    pub target: ColorTargetState,
    pub depth: Option<DepthStencilState>,
}

pub(crate) fn build_render_pipeline(device: &Device, spec: &PipelineSpec<'_>) -> RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(spec.layout),
        vertex: wgpu::VertexState {
            module: spec.shader,
            entry_point: Some(spec.vertex_entry),
            buffers: spec.buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: spec.shader,
            entry_point: Some(spec.fragment_entry),
            targets: &[Some(spec.target.clone())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: spec.topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: spec.depth.clone(),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        cache: None,
        multiview_mask: None,
    })
}

/// Depth test against the shared buffer; sprites write, lines only test.
pub(crate) fn depth_state(write: bool) -> DepthStencilState {
    DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: write,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}
