//! Bloom post-processing pipeline.
//!
//! Runs three full-screen passes after the main scene has been drawn into the
//! HDR target:
//!
//! 1. **Extract** — read HDR, keep only pixels brighter than `threshold`,
//!    write to `bloom_ping` (half-resolution).
//! 2. **Blur** — separable Gaussian, ping-ponging between `bloom_ping` and
//!    `bloom_pong`. Repeats `blur_iterations` times for a wider glow.
//! 3. **Composite** — read HDR + final bloom, tonemap, write to the
//!    swapchain in sRGB.
//!
//! The output of the chain is the sRGB swapchain texture.

use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, FilterMode,
    Queue, RenderPipeline, Sampler, ShaderModule, TextureFormat, TextureView,
};

use crate::render_targets::{RenderTargets, HDR_FORMAT};
use crate::scissor::PassScissor;

/// Tunables for the bloom pipeline. Mutate at runtime via
/// [`Renderer::bloom_config_mut`](crate::Renderer::bloom_config_mut).
#[derive(Debug, Clone, Copy)]
pub struct BloomConfig {
    /// Master switch — when false, the composite pass still runs (to do the
    /// tonemap), but bloom contribution is zeroed.
    pub enabled: bool,
    /// Luminance threshold for the bright pass. Pixels below this don't bloom.
    pub threshold: f32,
    /// Soft-knee width around the threshold to avoid banding.
    pub knee: f32,
    /// Multiplier applied to the bloom buffer during composite.
    pub intensity: f32,
    /// Number of horizontal+vertical blur iterations. Each iteration roughly
    /// doubles the effective blur radius. 2–4 is typical.
    pub blur_iterations: u32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 1.0,
            knee: 0.5,
            intensity: 0.8,
            blur_iterations: 3,
        }
    }
}

/// GPU layout for the extract/composite uniform buffer.
/// Padding makes this match the 16-byte alignment required for uniform buffers.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomParams {
    threshold: f32,
    knee: f32,
    intensity: f32,
    /// Exponent applied to the composite output. 1.0 on an sRGB swapchain
    /// (the hardware converts); 1.0/2.2 on non-sRGB swapchains (the web —
    /// WebGPU canvases expose no sRGB formats) so brightness matches native.
    inv_gamma: f32,
}

/// The composite pass's destination: the swapchain view plus whether its
/// format is sRGB (non-sRGB targets — WebGPU canvases — get gamma-encoded
/// in the shader instead of by the hardware), plus an optional scissor
/// bounding the composite draw (the tonemapped game image and
/// its glow stay inside the editor's scene panel; `None` = fullscreen).
pub struct SwapchainTarget<'a> {
    pub view: &'a TextureView,
    pub is_srgb: bool,
    pub composite_scissor: Option<[u32; 4]>,
}

/// GPU layout for the blur uniform buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    texel_size: [f32; 2],
    direction: [f32; 2],
}

/// Bind groups for all bloom passes, valid for one render-target size.
/// Rebuilt whenever the surface (and therefore the targets) resizes.
struct CachedBindGroups {
    size: (u32, u32),
    extract: BindGroup,
    blur_h: BindGroup,
    blur_v: BindGroup,
    composite: BindGroup,
}

/// Owns the bloom render pipelines, samplers, and bind-group layouts.
pub struct BloomPipeline {
    extract_pipeline: RenderPipeline,
    blur_pipeline: RenderPipeline,
    composite_pipeline: RenderPipeline,

    extract_layout: BindGroupLayout,
    blur_layout: BindGroupLayout,
    composite_layout: BindGroupLayout,

    sampler: Sampler,

    /// Uniform buffer reused for the extract and composite passes.
    bloom_params_buffer: Buffer,
    /// Per-direction blur uniform buffers. `queue.write_buffer` flushes all
    /// writes at submit time — before any encoded pass executes — so a single
    /// shared buffer rewritten between passes would make every pass read the
    /// last write. One buffer per direction keeps each pass's params stable.
    blur_params_h_buffer: Buffer,
    blur_params_v_buffer: Buffer,

    /// Bind groups cached per render-target size — creating them every frame
    /// is the exact churn the sprite pipeline's texture cache avoids.
    cached: Option<CachedBindGroups>,
}

impl BloomPipeline {
    /// Build all three render pipelines. Format must match the swapchain.
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let extract_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom Extract Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/bloom_extract.wgsl"))),
        });
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/bloom_blur.wgsl"))),
        });
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom Composite Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shaders/bloom_composite.wgsl"))),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Bloom Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            // Bloom textures are single-mip so this is academic, but keep it
            // explicit to match the field's type in wgpu 28.
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let extract_layout = create_single_tex_layout(device, "Bloom Extract Layout");
        let blur_layout = create_single_tex_layout(device, "Bloom Blur Layout");
        let composite_layout = create_composite_layout(device);

        let extract_pipeline = build_fullscreen_pipeline(
            device,
            &extract_layout,
            &extract_shader,
            HDR_FORMAT,
            "Bloom Extract Pipeline",
        );
        let blur_pipeline = build_fullscreen_pipeline(
            device,
            &blur_layout,
            &blur_shader,
            HDR_FORMAT,
            "Bloom Blur Pipeline",
        );
        let composite_pipeline = build_fullscreen_pipeline(
            device,
            &composite_layout,
            &composite_shader,
            surface_format,
            "Bloom Composite Pipeline",
        );

        let bloom_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bloom Params Buffer"),
            contents: bytemuck::bytes_of(&BloomParams { threshold: 1.0, knee: 0.5, intensity: 0.8, inv_gamma: 1.0 }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let blur_params_h_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blur Params Buffer (Horizontal)"),
            contents: bytemuck::bytes_of(&BlurParams { texel_size: [0.0, 0.0], direction: [1.0, 0.0] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let blur_params_v_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blur Params Buffer (Vertical)"),
            contents: bytemuck::bytes_of(&BlurParams { texel_size: [0.0, 0.0], direction: [0.0, 1.0] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            extract_pipeline,
            blur_pipeline,
            composite_pipeline,
            extract_layout,
            blur_layout,
            composite_layout,
            sampler,
            bloom_params_buffer,
            blur_params_h_buffer,
            blur_params_v_buffer,
            cached: None,
        }
    }

    /// Run the full extract → blur → composite chain.
    ///
    /// `targets` holds the HDR + bloom ping-pong textures.
    /// `swapchain` is the destination view (sRGB).
    ///
    /// `swapchain.composite_scissor` bounds the composite draw only
    /// (extract/blur are offscreen and untouched): the tonemapped game image
    /// and its glow stay inside the editor's scene panel. The
    /// composite pass itself always runs — its `LoadOp::Clear` blanks the
    /// whole swapchain, which is what keeps regions outside the scissor
    /// defined.
    pub fn run(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        targets: &RenderTargets,
        swapchain: SwapchainTarget<'_>,
        config: &BloomConfig,
    ) {
        // 1. Update the extract/composite params (tunable at runtime).
        let intensity = if config.enabled { config.intensity } else { 0.0 };
        queue.write_buffer(
            &self.bloom_params_buffer,
            0,
            bytemuck::bytes_of(&BloomParams {
                threshold: config.threshold,
                knee: config.knee,
                intensity,
                inv_gamma: if swapchain.is_srgb { 1.0 } else { 1.0 / 2.2 },
            }),
        );

        // 2. (Re)build bind groups if the targets resized. This also rewrites
        // the per-direction blur params, whose texel size only changes with
        // the target size.
        let size = (targets.width(), targets.height());
        if self.cached.as_ref().map(|c| c.size) != Some(size) {
            self.cached = Some(self.build_bind_groups(device, queue, targets));
        }
        let Some(cached) = self.cached.as_ref() else {
            return;
        };

        // 3. Extract bright pass: HDR -> bloom_ping (half-res).
        self.run_fullscreen_pass(
            encoder,
            &self.extract_pipeline,
            &cached.extract,
            &targets.bloom_ping_view,
            "Bloom Extract",
        );

        // 4. Blur iterations — ping-pong horizontal then vertical. The
        // bloom_ping texture holds the live state at the start and end of
        // every iteration, so it's the source the composite pass reads.
        //
        // Each direction has its own uniform buffer. Sharing one buffer and
        // rewriting it between passes would not work: write_buffer flushes at
        // submit, before any pass runs, so every pass would see only the
        // final write.
        let iterations = config.blur_iterations.max(1);
        for _ in 0..iterations {
            // Horizontal: bloom_ping -> bloom_pong.
            self.run_fullscreen_pass(encoder, &self.blur_pipeline, &cached.blur_h, &targets.bloom_pong_view, "Bloom Blur H");
            // Vertical: bloom_pong -> bloom_ping.
            self.run_fullscreen_pass(encoder, &self.blur_pipeline, &cached.blur_v, &targets.bloom_ping_view, "Bloom Blur V");
        }

        // 5. Composite HDR + bloom -> swapchain. The render targets are
        // surface-sized, so their size is the clamp bound. An empty
        // effective scissor still runs the pass (the Clear must blank the
        // swapchain) but skips the fullscreen draw.
        let composite_scissor = PassScissor::resolve(
            swapchain.composite_scissor,
            (targets.width(), targets.height()),
        );
        self.run_composite_pass(encoder, &cached.composite, swapchain.view, composite_scissor);
    }

    /// The composite pass with optional scissoring.
    fn run_composite_pass(
        &self,
        encoder: &mut CommandEncoder,
        bind_group: &BindGroup,
        target: &TextureView,
        scissor: PassScissor,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Bloom Composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        match scissor {
            PassScissor::Empty => return, // empty scissor: clear only, no draw
            PassScissor::Rect([x, y, w, h]) => pass.set_scissor_rect(x, y, w, h),
            PassScissor::Fullscreen => {}
        }
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Build the bind groups for the current render targets and write the
    /// blur params (texel size + direction) for both directions.
    fn build_bind_groups(
        &self,
        device: &Device,
        queue: &Queue,
        targets: &RenderTargets,
    ) -> CachedBindGroups {
        let texel = [1.0 / targets.bloom_width() as f32, 1.0 / targets.bloom_height() as f32];
        queue.write_buffer(
            &self.blur_params_h_buffer,
            0,
            bytemuck::bytes_of(&BlurParams { texel_size: texel, direction: [1.0, 0.0] }),
        );
        queue.write_buffer(
            &self.blur_params_v_buffer,
            0,
            bytemuck::bytes_of(&BlurParams { texel_size: texel, direction: [0.0, 1.0] }),
        );

        let extract = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom Extract Bind Group"),
            layout: &self.extract_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.bloom_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&targets.hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let blur_h = self.blur_bind_group(device, &self.blur_params_h_buffer, &targets.bloom_ping_view);
        let blur_v = self.blur_bind_group(device, &self.blur_params_v_buffer, &targets.bloom_pong_view);
        let composite = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom Composite Bind Group"),
            layout: &self.composite_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.bloom_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&targets.hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&targets.bloom_ping_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });

        CachedBindGroups {
            size: (targets.width(), targets.height()),
            extract,
            blur_h,
            blur_v,
            composite,
        }
    }

    fn blur_bind_group(&self, device: &Device, params: &Buffer, src_view: &TextureView) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom Blur Bind Group"),
            layout: &self.blur_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(src_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        })
    }

    fn run_fullscreen_pass(
        &self,
        encoder: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        bind_group: &BindGroup,
        target: &TextureView,
        label: &str,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn create_single_tex_layout(device: &Device, label: &str) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_composite_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bloom Composite Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn build_fullscreen_pipeline(
    device: &Device,
    bind_layout: &BindGroupLayout,
    shader: &ShaderModule,
    color_format: TextureFormat,
    label: &str,
) -> RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[bind_layout],
        ..Default::default()
    });

    crate::pipeline_builder::build_render_pipeline(
        device,
        &crate::pipeline_builder::PipelineSpec {
            label,
            layout: &pipeline_layout,
            shader,
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
            buffers: &[],
            topology: wgpu::PrimitiveTopology::TriangleList,
            target: wgpu::ColorTargetState {
                format: color_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            },
            depth: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// Uniform buffers are 16-byte aligned and the WGSL structs read fields
    /// by position: `BloomParams` feeds `bloom_extract.wgsl` and
    /// `bloom_composite.wgsl`, `BlurParams` feeds `bloom_blur.wgsl`.
    #[test]
    fn test_uniform_structs_match_shader_layout() {
        assert_eq!(size_of::<BloomParams>(), 16);
        assert_eq!(
            [
                offset_of!(BloomParams, threshold),
                offset_of!(BloomParams, knee),
                offset_of!(BloomParams, intensity),
                offset_of!(BloomParams, inv_gamma),
            ],
            [0, 4, 8, 12]
        );

        assert_eq!(size_of::<BlurParams>(), 16);
        assert_eq!(
            [offset_of!(BlurParams, texel_size), offset_of!(BlurParams, direction)],
            [0, 8]
        );
    }

    /// The shipped look: every game's glow is tuned against these values.
    #[test]
    fn test_default_bloom_config_is_the_shipped_tuning() {
        let config = BloomConfig::default();

        assert!(config.enabled, "bloom is on by default");
        assert_eq!(config.threshold, 1.0);
        assert_eq!(config.knee, 0.5);
        assert_eq!(config.intensity, 0.8);
        assert_eq!(config.blur_iterations, 3);
    }
}
