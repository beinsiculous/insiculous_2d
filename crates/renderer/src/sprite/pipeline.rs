//! GPU sprite pipeline: instanced quad rendering into the HDR target.

use std::collections::HashMap;
use wgpu::{Device, Queue, RenderPipeline, BindGroupLayout, Buffer, CommandEncoder};
use wgpu::util::DeviceExt;

use crate::render_targets::{DEPTH_FORMAT, HDR_FORMAT, RenderTargets};
use crate::sprite::SpriteBatch;
use crate::sprite_data::{Camera, CameraUniform, DynamicBuffer, SpriteInstance, SpriteVertex, TextureResource};
use crate::texture::TextureHandle;

/// Enhanced sprite pipeline with camera support and proper batching
pub struct SpritePipeline {
    /// The render pipeline
    pipeline: RenderPipeline,
    /// Vertex buffer for quad geometry
    vertex_buffer: Buffer,
    /// Instance buffer for sprite data (grows on demand)
    instance_buffer: DynamicBuffer<SpriteInstance>,
    /// Index buffer for quad geometry
    index_buffer: Buffer,
    /// Camera uniform buffer
    camera_buffer: Buffer,
    /// Texture bind group layout
    texture_bind_group_layout: BindGroupLayout,
    /// Camera bind group layout
    camera_bind_group_layout: BindGroupLayout,
    /// Cached camera bind group (created once, updated via buffer writes)
    camera_bind_group: wgpu::BindGroup,
    /// Cached texture bind groups (keyed by TextureHandle)
    texture_bind_group_cache: HashMap<TextureHandle, wgpu::BindGroup>,
    /// Change detector + staging buffer: skips the instance upload when
    /// nothing on screen changed (GPP-15)
    instance_cache: super::InstanceCache,
    /// Device reference for creating bind groups and growing buffers.
    /// A plain clone — wgpu's `Device` is internally reference-counted, so
    /// wrapping it in an `Arc` was a redundant refcount (and `Arc<Device>`
    /// trips clippy's `arc_with_non_send_sync` on wasm, where `Device` is
    /// not `Send`).
    device: Device,
}

impl SpritePipeline {
    /// Create a new sprite pipeline.
    ///
    /// `initial_sprite_capacity` sizes the instance buffer; it grows
    /// automatically if more sprites are submitted in a frame.
    pub fn new(device: &Device, initial_sprite_capacity: usize) -> Self {

        // Create texture bind group layout
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Texture Bind Group Layout"),
            entries: &[
                // Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Camera Bind Group Layout"),
            entries: &[
                // Camera uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            ..Default::default()
        });

        // Create quad vertices (full texture coordinates)
        let vertices = [
            // Top-left
            SpriteVertex::new(glam::Vec3::new(-0.5, 0.5, 0.0), glam::Vec2::new(0.0, 0.0), glam::Vec4::ONE),
            // Top-right
            SpriteVertex::new(glam::Vec3::new(0.5, 0.5, 0.0), glam::Vec2::new(1.0, 0.0), glam::Vec4::ONE),
            // Bottom-right
            SpriteVertex::new(glam::Vec3::new(0.5, -0.5, 0.0), glam::Vec2::new(1.0, 1.0), glam::Vec4::ONE),
            // Bottom-left
            SpriteVertex::new(glam::Vec3::new(-0.5, -0.5, 0.0), glam::Vec2::new(0.0, 1.0), glam::Vec4::ONE),
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer for quad (2 triangles, 6 indices)
        let indices: [u16; 6] = [
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create instance buffer
        let instance_buffer = DynamicBuffer::new(
            device,
            initial_sprite_capacity,
            wgpu::BufferUsages::VERTEX,
        );

        // Create camera uniform buffer
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Camera Buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::from_camera(&Camera::default())]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create camera bind group once (will be reused, buffer updated via write_buffer)
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("../shaders/sprite_instanced.wgsl"))),
        });

        // Create the render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    SpriteVertex::desc(),     // Vertex buffer
                    SpriteInstance::desc(),   // Instance buffer
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // Sprites render to the HDR offscreen target. The bloom
                    // composite is what writes the final sRGB swapchain.
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Don't cull sprites
                ..Default::default()
            },
            // Real depth buffer. depth_write_enabled=true plus alpha blending
            // means the user still needs to sort transparent sprites
            // back-to-front, but opaque depth ordering Just Works once any
            // 3D-ish features arrive.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
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
            instance_buffer,
            index_buffer,
            camera_buffer,
            texture_bind_group_layout,
            camera_bind_group_layout,
            camera_bind_group,
            texture_bind_group_cache: HashMap::new(),
            instance_cache: super::InstanceCache::new(),
            device: device.clone(),
        }
    }

    /// Update camera uniform
    pub fn update_camera(&self, queue: &Queue, camera: &Camera) {
        let uniform = CameraUniform::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Update instance buffer with sprite data and return the number of instances.
    ///
    /// The instance buffer grows automatically when the sprite count exceeds
    /// its current capacity.
    pub fn update_instance_buffer(&mut self, queue: &Queue, instances: &[SpriteInstance]) -> usize {
        if instances.is_empty() {
            return 0;
        }

        self.instance_buffer.update(&self.device, queue, instances);

        instances.len()
    }

    /// Update texture bind group cache for new textures
    ///
    /// Call this when new textures are loaded to create and cache their bind groups.
    /// This avoids creating bind groups during the render loop.
    pub fn cache_texture_bind_groups(&mut self, texture_resources: &HashMap<TextureHandle, TextureResource>) {
        for (handle, resource) in texture_resources {
            self.cache_texture_bind_group(*handle, resource);
        }
    }

    /// Cache the bind group for a single texture. No-op if already cached.
    ///
    /// Used per-frame for the renderer's built-in white texture
    /// ([`TextureHandle::WHITE`]), which lives outside the asset manager's
    /// texture map.
    pub fn cache_texture_bind_group(&mut self, handle: TextureHandle, resource: &TextureResource) {
        if self.texture_bind_group_cache.contains_key(&handle) {
            return;
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Cached Texture Bind Group {:?}", handle)),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&resource.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&resource.sampler),
                },
            ],
        });

        self.texture_bind_group_cache.insert(handle, bind_group);
        log::debug!("Cached bind group for texture {:?}", handle);
    }

    /// Clear a texture from the bind group cache (e.g., when texture is unloaded)
    pub fn invalidate_texture_cache(&mut self, handle: &TextureHandle) {
        self.texture_bind_group_cache.remove(handle);
    }

    /// Clear all cached texture bind groups
    pub fn clear_texture_cache(&mut self) {
        self.texture_bind_group_cache.clear();
    }

    /// Draw sprites into the HDR target.
    ///
    /// `targets` provides the HDR color view (Rgba16Float) and matching depth
    /// view. `clear_color` is applied to the HDR target — it's treated as a
    /// linear RGB color, so values >1.0 are valid and bloom.
    ///
    /// The camera uniform is uploaded separately via
    /// [`update_camera`](Self::update_camera) before this call; the caller is
    /// responsible for that.
    ///
    /// Uses cached bind groups for improved performance. Call
    /// [`cache_texture_bind_groups`](Self::cache_texture_bind_groups) before
    /// drawing if new textures have been loaded.
    pub fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        texture_resources: &HashMap<TextureHandle, TextureResource>,
        batches: &[&SpriteBatch],
        targets: &RenderTargets,
        clear_color: wgpu::Color,
    ) {
        log::debug!("SPRITE DRAW: batches={}, clear_color={:?}", batches.len(), clear_color);

        // Ensure all textures have cached bind groups
        self.cache_texture_bind_groups(texture_resources);

        // Warn about missing textures. The bind group cache is the source of
        // truth for drawability — it also covers the built-in white texture,
        // which isn't part of the asset manager's texture map.
        for batch in batches {
            if !batch.is_empty() && !self.texture_bind_group_cache.contains_key(&batch.texture_handle) {
                log::warn!(
                    "Missing texture: handle {:?} referenced by {} sprite(s) has no cached bind group. \
                     Sprites will not render. Ensure the texture is loaded via AssetManager.",
                    batch.texture_handle,
                    batch.len()
                );
            }
        }

        // Begin render pass: clear HDR color + depth, draw sprites with depth-test.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sprite Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.hdr_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Set pipeline
        render_pass.set_pipeline(&self.pipeline);

        // Set vertex buffers
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice());

        // Use cached camera bind group (set 0)
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        // Draw each batch
        let mut instance_offset = 0u32;
        for batch in batches {
            if batch.is_empty() {
                continue;
            }

            // Get cached texture bind group or skip the batch
            let texture_bind_group = if let Some(cached) = self.texture_bind_group_cache.get(&batch.texture_handle) {
                cached
            } else {
                // This should not happen since we called cache_texture_bind_groups above
                log::warn!("No cached bind group for texture handle {:?}, sprite may not render correctly", batch.texture_handle);
                continue;
            };

            // Set texture bind group (set 1)
            render_pass.set_bind_group(1, texture_bind_group, &[]);

            // Draw the instances for this batch
            let instance_count = batch.len() as u32;

            // Draw indexed triangles with instancing
            // 6 indices per quad (2 triangles), draw instances from instance_offset to instance_offset + instance_count
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, instance_offset..(instance_offset + instance_count));

            // Update offset for next batch
            instance_offset += instance_count;
        }
    }

    /// Get the render pipeline
    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    /// Get the camera bind group layout
    pub fn camera_bind_group_layout(&self) -> &BindGroupLayout {
        &self.camera_bind_group_layout
    }

    /// Get the texture bind group layout
    pub fn texture_bind_group_layout(&self) -> &BindGroupLayout {
        &self.texture_bind_group_layout
    }

    /// Prepare sprite data for rendering by updating the instance buffer.
    ///
    /// The upload happens only when the flattened instances or batch layout
    /// actually changed since the last upload (GPP-15) — a static scene
    /// re-renders from the buffer already on the GPU.
    pub fn prepare_sprites(&mut self, queue: &Queue, batches: &[&SpriteBatch]) {
        if self.instance_cache.stage(batches) {
            let staged = self.instance_cache.staged();
            if !staged.is_empty() {
                log::debug!("Uploading {} sprite instances to GPU", staged.len());
                self.instance_buffer.update(&self.device, queue, staged);
            }
        }
    }
}
