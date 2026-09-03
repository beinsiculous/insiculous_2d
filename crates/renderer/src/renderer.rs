//! WGPU renderer implementation managing surface configuration, passes, and device lifecycle.

use std::sync::Arc;
use wgpu::{
    Adapter, Device, Queue, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
};
use winit::window::Window;

use crate::bloom::{BloomConfig, BloomPipeline};
use crate::error::RendererError;
use crate::line_pipeline::{LinePipeline, LineVertex};
use crate::render_targets::RenderTargets;

/// Configuration for creating a [`Renderer`].
///
/// Games normally set these through `GameConfig` in `engine_core`; this
/// struct is the renderer-level surface for embedders that drive the
/// renderer directly.
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// Present frames with vsync (`PresentMode::Fifo` — never tears, capped
    /// at the display refresh rate). `false` selects `AutoNoVsync` for the
    /// lowest latency the platform offers.
    pub vsync: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self { vsync: true }
    }
}

/// The main renderer struct - now with proper lifetime management
pub struct Renderer {
    window: Arc<Window>,
    /// Kept alive for the surface's whole life: on the WebGPU backend
    /// (Chromium's Dawn) dropping the instance invalidates presentation
    /// silently — frames submit without errors but never reach the canvas.
    _instance: wgpu::Instance,
    surface: Surface<'static>, // 'static is safe because we control the lifetime
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    clear_color: wgpu::Color,
    /// White texture resource for colored sprites (multiply by white instead of transparent black)
    white_texture: Option<crate::sprite_data::TextureResource>,
    /// HDR color + depth + bloom ping-pong textures.
    render_targets: RenderTargets,
    /// Bloom post-processing pipeline (extract -> blur -> composite).
    bloom_pipeline: BloomPipeline,
    /// Runtime-tunable bloom knobs.
    bloom_config: BloomConfig,
    /// Pipeline + buffer for line-list geometry (e.g. the spring-mass grid).
    line_pipeline: LinePipeline,
    /// Number of line vertices uploaded by the most recent `set_lines` call.
    /// Reset to 0 when no lines are drawn this frame.
    line_vertex_count: u32,
    /// Per-frame scissor bounding the game-world passes (sprites, lines,
    /// bloom composite) in physical surface pixels. `None` = full surface
    /// (the default; shipped games never set it). A zero-size rect draws no
    /// game world at all (the editor's "scene panel hidden" case). Set every
    /// frame via [`set_viewport_scissor`](Self::set_viewport_scissor);
    /// the UI pass is never affected.
    viewport_scissor: Option<[u32; 4]>,
    /// One-way flag set by wgpu's device-lost callback. Every queue/surface
    /// touchpoint checks it first — submitting to a dead queue is what
    /// crashed Firefox's parent-process WebGPU (its wgpu-core panics on the
    /// reclaimed queue id).
    device_lost: crate::device_status::DeviceLossLatch,
    /// Set when a zero-size resize request was skipped: the next non-zero
    /// resize must reconfigure even at an unchanged size (hidden-canvas
    /// round trip on the web).
    pending_reconfigure: bool,
}

impl Renderer {
    /// Create a new renderer with an existing window and default configuration
    pub async fn new(window: Arc<Window>) -> Result<Self, RendererError> {
        Self::with_config(window, RendererConfig::default()).await
    }

    /// Create a new renderer with an existing window
    ///
    /// This method properly manages the surface lifetime by:
    /// 1. Creating the instance and surface first
    /// 2. The surface gets `'static` lifetime because `Arc<Window>` is `'static`
    /// 3. WGPU 28.0.0 supports `Arc<Window>` -> `Surface<'static>` conversion
    pub async fn with_config(window: Arc<Window>, renderer_config: RendererConfig) -> Result<Self, RendererError> {
        // Create a WGPU instance
        let instance = wgpu::Instance::default();

        // Create a surface with 'static lifetime
        // Arc<Window> implements Into<SurfaceTarget<'static>> because Arc<T> is 'static when T: 'static
        // This is safe and doesn't require unsafe code - WGPU 28.0.0 handles this correctly
        let surface: Surface<'static> = instance
            .create_surface(window.clone())
            .map_err(|e| RendererError::SurfaceCreationError(e.to_string()))?;

        // Get adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(|_e| RendererError::AdapterCreationError("No suitable adapter found".to_string()))?;

        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    experimental_features: Default::default(),
                    memory_hints: Default::default(),
                    trace: Default::default(),
                },
            )
            .await
            .map_err(|e| RendererError::DeviceCreationError(e.to_string()))?;

        // Device-loss fail-stop: the callback (real on both the native
        // wgpu-core and browser webgpu backends — on the web it hooks the JS
        // `device.lost` promise) sets a one-way latch that every render-path
        // entry point checks before touching the queue or surface. Paths
        // outside the render loop (glyph-cache uploads, asset loads) stay
        // unguarded on purpose: the frame loop halts one frame after the
        // latch sets, so they can run at most once against a dead device —
        // accepted over threading the latch through AssetManager.
        let device_lost = crate::device_status::DeviceLossLatch::new();
        let loss_latch = device_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            log::error!("GPU device lost ({reason:?}): {message}");
            loss_latch.mark_lost();
        });
        // Log uncaptured errors instead of wgpu's native panic-by-default: a
        // dying device emits a storm of errors between loss and fail-stop,
        // and panicking on the first would preempt the clean shutdown path.
        // Validation errors are NOT device loss — never set the latch here.
        // Debug native builds keep the fail-fast signal for real bugs.
        device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| {
            log::error!("Uncaptured wgpu error: {e}");
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            if matches!(e, wgpu::Error::Validation { .. }) {
                panic!("wgpu validation error (debug build): {e}");
            }
        }));

        // Configure surface. The bloom composite pass writes the final tonemapped
        // color and relies on the GPU's automatic linear -> sRGB conversion when
        // writing to an sRGB swapchain, so we prefer an sRGB surface format.
        // Clamp to 1x1: a zero-size surface is a wgpu validation error. On
        // the web the adopted canvas can report 0 until its first layout —
        // the real size arrives via resize() right after adoption.
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: if renderer_config.vsync {
                wgpu::PresentMode::Fifo
            } else {
                wgpu::PresentMode::AutoNoVsync
            },
            // Prefer an opaque surface: on the web, `Auto` can resolve to
            // premultiplied alpha and any pass writing alpha < 1 turns the
            // canvas transparent (a "black" page with zero errors). Native
            // swapchains ignore alpha, so this changes nothing there.
            alpha_mode: if surface_caps
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::Opaque)
            {
                wgpu::CompositeAlphaMode::Opaque
            } else {
                wgpu::CompositeAlphaMode::Auto
            },
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        log::info!(
            "surface config: {}x{} format {:?} alpha {:?} (caps: formats {:?}, alpha {:?})",
            config.width, config.height, config.format, config.alpha_mode,
            surface_caps.formats, surface_caps.alpha_modes
        );

        // Configure the surface before moving it
        surface.configure(&device, &config);

        // Now we can safely create the renderer with 'static surface
        // This is safe because:
        // 1. The surface is tied to the window (Arc<Window>)
        // 2. The window outlives the renderer
        // 3. We control the renderer's lifetime through the game runner
        
        // Wrap device and queue in Arc for sharing. On wasm `Device` is not
        // `Send`, which trips `arc_with_non_send_sync` — but these Arcs are
        // public API (`device()`/`queue()` feed engine_core's RenderManager
        // and AssetManager across 4+ signatures), the web build is
        // single-threaded, and unwinding the Arc there isn't worth one
        // wasm-only lint.
        #[allow(clippy::arc_with_non_send_sync)]
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Create white texture for colored sprites
        let white_texture = crate::white_texture::create_white_texture_resource(&device, &queue);

        // Build offscreen targets + post-processing pipelines sized to the initial window.
        let render_targets = RenderTargets::new(&device, size.width, size.height);
        let bloom_pipeline = BloomPipeline::new(&device, format);
        let bloom_config = BloomConfig::default();
        let line_pipeline = LinePipeline::new(&device, LinePipeline::DEFAULT_CAPACITY);

        Ok(Self {
            window,
            _instance: instance,
            surface,
            adapter,
            device,
            queue,
            config,
            clear_color: wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            white_texture: Some(white_texture),
            render_targets,
            bloom_pipeline,
            bloom_config,
            line_pipeline,
            line_vertex_count: 0,
            viewport_scissor: None,
            device_lost,
            pending_reconfigure: false,
        })
    }

    /// Read-only view of the bloom tunables.
    pub fn bloom_config(&self) -> &BloomConfig {
        &self.bloom_config
    }

    /// Mutable access to the bloom tunables (threshold, intensity, etc.).
    pub fn bloom_config_mut(&mut self) -> &mut BloomConfig {
        &mut self.bloom_config
    }

    /// Upload line vertices for the next render. Pairs of vertices form line
    /// segments. The line pipeline draws these into the HDR target after
    /// sprites and before bloom, so emissive lines bloom.
    ///
    /// Call every frame — vertices are not retained across frames; an empty
    /// slice (or no call at all) means no lines render this frame.
    pub fn set_lines(&mut self, vertices: &[LineVertex]) {
        if self.device_lost.is_lost() {
            self.line_vertex_count = 0;
            return;
        }
        self.line_vertex_count = vertices.len() as u32;
        self.line_pipeline.upload_vertices(&self.queue, vertices);
    }

    /// Set the clear color
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.clear_color = wgpu::Color { r, g, b, a };
    }

    /// Bound the game-world passes (sprites, lines, bloom composite) to a
    /// scissor rect in physical surface pixels. `None` restores full-surface
    /// rendering; a zero-size rect draws no game world. Call every frame —
    /// like [`set_lines`](Self::set_lines), the value is per-frame state.
    pub fn set_viewport_scissor(&mut self, scissor: Option<[u32; 4]>) {
        self.viewport_scissor = scissor;
    }

    /// Acquire the current surface texture for rendering.
    ///
    /// Returns:
    /// - `Ok(Some(frame))` - Successfully acquired frame, proceed with rendering
    /// - `Ok(None)` - Transient error, skip this frame
    /// - `Err(_)` - Fatal or recoverable error that needs handling
    fn acquire_frame(&self) -> Result<Option<wgpu::SurfaceTexture>, RendererError> {
        if self.device_lost.is_lost() {
            return Err(RendererError::DeviceLost);
        }
        match self.surface.get_current_texture() {
            Ok(frame) => Ok(Some(frame)),
            Err(wgpu::SurfaceError::Lost) => {
                // Surface was lost, return error so caller can recreate it
                Err(RendererError::SurfaceError("Surface lost".to_string()))
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                // Fatal error, we can't recover
                Err(RendererError::RenderingError("Out of memory".to_string()))
            }
            Err(e) => {
                // Other errors (Timeout, Outdated) can be logged and skipped
                log::warn!("Surface error: {:?}, skipping frame", e);
                Ok(None)
            }
        }
    }

    /// Render a frame with a sprite pipeline
    pub fn render_with_sprites(
        &mut self,
        sprite_pipeline: &mut crate::sprite::SpritePipeline,
        ui_pipeline: &mut crate::sprite::SpritePipeline,
        camera: &crate::sprite_data::Camera,
        texture_resources: &std::collections::HashMap<crate::texture::TextureHandle, crate::sprite_data::TextureResource>,
        sprite_batches: &[&crate::sprite::SpriteBatch],
        ui_batches: &[&crate::sprite::SpriteBatch],
    ) -> Result<(), RendererError> {
        // A lost device must fail before prepare_sprites' write_buffer calls
        // — those are exactly the queue traffic that panics Firefox's
        // parent-process wgpu once its queue id is reclaimed.
        if self.device_lost.is_lost() {
            return Err(RendererError::DeviceLost);
        }

        // Make sure the built-in white texture (for flat-colored sprites) has
        // a cached bind group. Cheap no-op after the first frame — no need to
        // clone the caller's texture map just to splice it in.
        if let Some(white_texture) = &self.white_texture {
            sprite_pipeline.cache_texture_bind_group(crate::texture::TextureHandle::WHITE, white_texture);
            ui_pipeline.cache_texture_bind_group(crate::texture::TextureHandle::WHITE, white_texture);
        }

        // Prepare sprites - update instance buffers with sprite data
        sprite_pipeline.prepare_sprites(&self.queue, sprite_batches);
        ui_pipeline.prepare_sprites(&self.queue, ui_batches);

        // Get a frame (returns None if we should skip this frame)
        let frame = match self.acquire_frame()? {
            Some(frame) => frame,
            None => return Ok(()),
        };

        // Swapchain view: final destination for the composite pass.
        let swapchain_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        sprite_pipeline.update_camera(&self.queue, camera);
        ui_pipeline.update_camera(&self.queue, camera);
        self.line_pipeline.update_camera(&self.queue, camera);

        // Viewport scissor bounds every game-world pass; the UI pass is
        // exempt (its chrome must fill the window) but honors per-batch
        // clip rects.
        let viewport_scissor = self.viewport_scissor;

        // Pass 1: sprites -> HDR color (+ depth).
        sprite_pipeline.draw(
            &mut encoder,
            texture_resources,
            sprite_batches,
            &self.render_targets,
            self.clear_color,
            viewport_scissor,
        );

        // Pass 2: lines (e.g. the spring-mass grid) on top of sprites in HDR.
        // No-op when `set_lines` wasn't called this frame.
        self.line_pipeline.draw(
            &mut encoder,
            &self.render_targets,
            self.line_vertex_count,
            viewport_scissor,
        );

        // Pass 3..N: bloom (extract -> blur -> composite to swapchain).
        self.bloom_pipeline.run(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.render_targets,
            crate::bloom::SwapchainTarget {
                view: &swapchain_view,
                is_srgb: self.config.format.is_srgb(),
                composite_scissor: viewport_scissor,
            },
            &self.bloom_config,
        );

        // Final pass: UI straight to the swapchain, after (and exempt from)
        // the tonemap — authored UI colors display exactly.
        ui_pipeline.draw_ui(
            &mut encoder,
            texture_resources,
            ui_batches,
            &swapchain_view,
            &self.render_targets.depth_view,
            (self.config.width, self.config.height),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    /// Get a reference to the window
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get a reference to the GPU device.
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    /// Get a reference to the GPU queue.
    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    /// Get adapter information
    pub fn adapter_info(&self) -> String {
        self.adapter.get_info().name
    }

    /// Get surface format
    pub fn surface_format(&self) -> TextureFormat {
        self.config.format
    }

    /// Get surface width
    pub fn surface_width(&self) -> u32 {
        self.config.width
    }

    /// Get surface height
    pub fn surface_height(&self) -> u32 {
        self.config.height
    }

    /// Get a reference to the surface (for diagnostic purposes)
    pub fn surface(&self) -> &Surface<'_> {
        &self.surface
    }

    /// Resize the surface and recreate the offscreen HDR / depth / bloom targets.
    ///
    /// Same-size requests are skipped — `surface.configure` tears down and
    /// recreates the swapchain, and on the web ResizeObserver echoes arrive
    /// every frame. Zero-size requests are skipped too (wgpu validation
    /// error) but arm a forced reconfigure for the next non-zero size, so a
    /// hidden canvas coming back at its old size still gets a fresh surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.device_lost.is_lost() {
            return;
        }
        if width == 0 || height == 0 {
            self.pending_reconfigure = true;
            return;
        }
        let current = (self.config.width, self.config.height);
        let Some((w, h)) = crate::device_status::resize_action(
            current,
            (width, height),
            self.pending_reconfigure,
        ) else {
            return;
        };
        self.pending_reconfigure = false;
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.render_targets.resize(&self.device, w, h);
    }

    /// Handle surface lost error by recreating the surface
    pub fn recreate_surface(&mut self) -> Result<(), RendererError> {
        // A lost device turns "recreate" into an immediate fatal error —
        // reconfiguring a dead device is more traffic at a dead queue.
        if self.device_lost.is_lost() {
            return Err(RendererError::DeviceLost);
        }
        // Reconfigure the surface
        self.surface.configure(&self.device, &self.config);
        log::debug!("Surface recreated after loss");
        Ok(())
    }

    /// Has wgpu reported the GPU device lost? Once true, every render-path
    /// method fails fast and the frame loop should stop.
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.is_lost()
    }

    /// Get the white texture resource for colored sprites
    pub fn white_texture(&self) -> Option<&crate::sprite_data::TextureResource> {
        self.white_texture.as_ref()
    }
}