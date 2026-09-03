//! Render manager for handling renderer lifecycle and sprite rendering.
//!
//! This module provides a focused manager for all rendering concerns,
//! following the Single Responsibility Principle.

use std::collections::HashMap;
use std::sync::Arc;

use ecs::sprite_components::Transform2D;
use ecs::World;
use glam::Vec2;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Window;

use renderer::{
    bloom::BloomConfig,
    line_pipeline::LineVertex,
    sprite::{SpriteBatch, SpriteBatcher, SpritePipeline},
    sprite_data::TextureResource,
    texture::TextureHandle,
    wgpu::{Device, Queue},
    Camera, Renderer, RendererError,
};

/// Manages the renderer lifecycle and sprite rendering pipeline.
///
/// This struct encapsulates all rendering-related responsibilities:
/// - Renderer initialization and lifecycle
/// - Sprite pipeline management
/// - Camera configuration
/// - Surface management
pub struct RenderManager {
    /// The WGPU renderer
    renderer: Option<Renderer>,
    /// The sprite rendering pipeline
    sprite_pipeline: Option<SpritePipeline>,
    /// Post-tonemap UI pipeline: targets the swapchain format directly so
    /// authored UI colors display exactly (issue #26).
    ui_pipeline: Option<SpritePipeline>,
    /// The 2D camera for orthographic projection
    camera: Camera,
    /// Consecutive frames that ended in a surface error. Reset on any
    /// successful render; at [`MAX_SURFACE_ERROR_STREAK`] the manager goes
    /// fatal instead of reconfiguring a possibly-dead device forever.
    surface_error_streak: u32,
    /// Latched on device loss (or a surface-error streak). Once set, render
    /// calls fail fast with `DeviceLost` and never touch the GPU again.
    fatal: bool,
}

/// Consecutive surface-error frames tolerated before escalating to fatal.
///
/// This is the backstop for the case where the device died but wgpu's
/// device-lost callback never fired (a browser-bug scenario — the exact one
/// that crashed Firefox). Healthy transients never get here: `Outdated` and
/// `Timeout` skip the frame without erroring, so only `SurfaceError::Lost`
/// feeds the streak, and one successful frame resets it. Ten straight lost
/// frames (~160ms at 60fps) means reconfiguring is not helping.
const MAX_SURFACE_ERROR_STREAK: u32 = 10;

/// What to do about a render error, given how many consecutive
/// surface-error frames preceded it.
#[derive(Debug, PartialEq, Eq)]
enum RenderErrorAction {
    /// Transient surface loss: reconfigure and keep going.
    RecreateSurface,
    /// The device is gone (or reconfiguring stopped helping): stop rendering
    /// for good.
    Fatal,
    /// Not a surface problem — hand the error to the caller unchanged.
    Propagate,
}

/// Classify a render error against the current surface-error streak.
/// `streak` counts the errors BEFORE this one.
fn classify_render_error(error: &RendererError, streak: u32) -> RenderErrorAction {
    match error {
        RendererError::DeviceLost => RenderErrorAction::Fatal,
        RendererError::SurfaceError(_) if streak + 1 >= MAX_SURFACE_ERROR_STREAK => {
            RenderErrorAction::Fatal
        }
        RendererError::SurfaceError(_) => RenderErrorAction::RecreateSurface,
        _ => RenderErrorAction::Propagate,
    }
}

impl Default for RenderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderManager {
    /// Create a new render manager with default settings.
    pub fn new() -> Self {
        Self {
            renderer: None,
            sprite_pipeline: None,
            ui_pipeline: None,
            camera: Camera::default(),
            surface_error_streak: 0,
            fatal: false,
        }
    }

    /// Initialize the renderer with a window (native only).
    ///
    /// Blocks on the async wgpu setup via `pollster` — the one place the
    /// engine may block, at the native outer edge. On the web, renderer
    /// creation is spawned as a browser task and its result is handed to
    /// [`Self::complete_init`] instead.
    ///
    /// # Arguments
    /// * `window` - The window to render to
    /// * `clear_color` - RGBA clear color for the background
    ///
    /// # Returns
    /// * `Ok(())` on successful initialization
    /// * `Err(RendererError)` if initialization fails
    #[cfg(not(target_arch = "wasm32"))]
    pub fn init(
        &mut self,
        window: Arc<Window>,
        clear_color: [f32; 4],
        renderer_config: renderer::RendererConfig,
    ) -> Result<(), RendererError> {
        let renderer = pollster::block_on(renderer::init_with_config(window, renderer_config))?;
        self.complete_init(renderer, clear_color);
        Ok(())
    }

    /// Adopt an already-created renderer: set the clear color, build the
    /// sprite pipeline, and mark the manager initialized. Shared tail of
    /// native (blocking) and web (async task) renderer bring-up.
    pub fn complete_init(&mut self, mut renderer: Renderer, clear_color: [f32; 4]) {
        renderer.set_clear_color(
            clear_color[0] as f64,
            clear_color[1] as f64,
            clear_color[2] as f64,
            clear_color[3] as f64,
        );

        // Create sprite pipeline with max 1000 sprites per batch
        let sprite_pipeline = SpritePipeline::new(renderer.device(), 1000);
        // UI draws post-tonemap straight to the swapchain (issue #26).
        let ui_pipeline =
            SpritePipeline::new_ui(renderer.device(), 1000, renderer.surface_format());

        self.renderer = Some(renderer);
        self.sprite_pipeline = Some(sprite_pipeline);
        self.ui_pipeline = Some(ui_pipeline);

        log::info!("RenderManager initialized");
    }

    /// Check if the renderer is initialized.
    pub fn is_initialized(&self) -> bool {
        self.renderer.is_some()
    }

    /// Resize the renderer surface.
    ///
    /// Updates both the renderer surface and camera viewport.
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(width, height);
        }
        self.camera.viewport_size = Vec2::new(width as f32, height as f32);
    }

    /// Update the camera viewport size based on current window dimensions.
    pub fn update_viewport_from_renderer(&mut self) {
        if let Some(renderer) = &self.renderer {
            let width = renderer.surface_width() as f32;
            let height = renderer.surface_height() as f32;
            if height > 0.0 {
                self.camera.viewport_size = Vec2::new(width, height);
            }
        }
    }

    /// Has rendering failed fatally (device lost, or surface errors that
    /// reconfiguring could not fix)? Once true, [`Self::render`] refuses to
    /// touch the GPU and the frame loop should stop.
    pub fn is_fatal(&self) -> bool {
        self.fatal
            || self
                .renderer
                .as_ref()
                .is_some_and(|r| r.is_device_lost())
    }

    /// A frame rendered: surface errors are no longer consecutive.
    fn note_render_success(&mut self) {
        self.surface_error_streak = 0;
    }

    /// Record a render error: advance the surface-error streak, classify,
    /// and latch fatal when warranted. Returns the action the caller must
    /// perform (the actual `recreate_surface` call stays at the call site so
    /// this state machine tests headless).
    fn note_render_error(&mut self, error: &RendererError) -> RenderErrorAction {
        let action = classify_render_error(error, self.surface_error_streak);
        if matches!(error, RendererError::SurfaceError(_)) {
            self.surface_error_streak += 1;
        }
        if action == RenderErrorAction::Fatal {
            self.fatal = true;
            log::error!("Fatal render failure ({error}) — rendering stopped");
        }
        action
    }

    /// Render sprites using the provided batcher and textures.
    ///
    /// # Arguments
    /// * `batches` - Sprite batches to render
    /// * `textures` - Texture resources for rendering
    ///
    /// # Returns
    /// * `Ok(())` on successful render
    /// * `Err(RendererError)` if rendering fails
    pub fn render(
        &mut self,
        batches: &[&SpriteBatch],
        ui_batches: &[&SpriteBatch],
        textures: &HashMap<TextureHandle, TextureResource>,
    ) -> Result<(), RendererError> {
        if self.is_fatal() {
            return Err(RendererError::DeviceLost);
        }
        let renderer = self.renderer.as_mut().ok_or_else(|| {
            RendererError::WindowCreationError("Renderer not initialized".to_string())
        })?;
        let pipeline = self.sprite_pipeline.as_mut().ok_or_else(|| {
            RendererError::WindowCreationError("Sprite pipeline not initialized".to_string())
        })?;
        let ui_pipeline = self.ui_pipeline.as_mut().ok_or_else(|| {
            RendererError::WindowCreationError("UI pipeline not initialized".to_string())
        })?;

        match renderer.render_with_sprites(
            pipeline,
            ui_pipeline,
            &self.camera,
            textures,
            batches,
            ui_batches,
        ) {
            Ok(_) => {
                self.note_render_success();
                Ok(())
            }
            Err(e) => match self.note_render_error(&e) {
                RenderErrorAction::RecreateSurface => {
                    // recreate_surface itself fails with DeviceLost on a dead
                    // device; note that too so fatal latches without waiting
                    // for the streak.
                    let recreate = self
                        .renderer
                        .as_mut()
                        .map(|r| r.recreate_surface())
                        .unwrap_or(Ok(()));
                    match recreate {
                        Ok(()) => {
                            log::debug!("Surface recreated after loss");
                            Ok(())
                        }
                        Err(recreate_err) => {
                            log::error!("Failed to recreate surface: {recreate_err}");
                            if self.note_render_error(&recreate_err)
                                == RenderErrorAction::Fatal
                            {
                                Err(RendererError::DeviceLost)
                            } else {
                                Err(recreate_err)
                            }
                        }
                    }
                }
                RenderErrorAction::Fatal => Err(RendererError::DeviceLost),
                RenderErrorAction::Propagate => {
                    log::error!("Render error: {e}");
                    Err(e)
                }
            },
        }
    }

    /// Render a frame using a SpriteBatcher.
    ///
    /// This is a convenience method that extracts batches from the batcher.
    /// Batches are submitted in deterministic order (min depth, then texture
    /// handle) — HashMap iteration order would make cross-batch draw order
    /// vary between runs.
    pub fn render_batcher(
        &mut self,
        batcher: &SpriteBatcher,
        textures: &HashMap<TextureHandle, TextureResource>,
    ) -> Result<(), RendererError> {
        let mut batch_refs: Vec<&SpriteBatch> = batcher.batches().values().collect();
        batch_refs.sort_by(|a, b| {
            let min_depth = |batch: &SpriteBatch| {
                batch
                    .instances
                    .iter()
                    .map(|i| i.depth)
                    .min_by(f32::total_cmp)
                    .unwrap_or(0.0)
            };
            min_depth(a)
                .total_cmp(&min_depth(b))
                .then_with(|| a.texture_handle.id.cmp(&b.texture_handle.id))
                // Same-texture batches can differ only by clip (issue #41).
                .then_with(|| a.clip.cmp(&b.clip))
        });
        self.render(&batch_refs, &[], textures)
    }


    /// Get the GPU device if the renderer is initialized.
    pub fn device(&self) -> Option<Arc<Device>> {
        self.renderer.as_ref().map(|r| Arc::clone(r.device()))
    }

    /// Get the GPU queue if the renderer is initialized.
    pub fn queue(&self) -> Option<Arc<Queue>> {
        self.renderer.as_ref().map(|r| Arc::clone(r.queue()))
    }

    /// Get a reference to the camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Copy the main camera entity's position and zoom onto the render camera.
    ///
    /// Rotation and viewport_size stay render-managed (viewport_size tracks
    /// window resizes; the editor viewport's world↔screen math has no
    /// rotation term, so syncing rotation would break its overlay/GPU
    /// agreement — issue #42's documented limitation). Worlds without a
    /// main-camera entity are untouched, and games can still override
    /// `ctx.camera` in `render()` afterwards.
    pub fn sync_main_camera(&mut self, world: &World) {
        if let Some((position, zoom)) = main_camera_pose(world) {
            self.camera.position = position;
            self.camera.zoom = zoom;
        }
    }

    /// Set the camera viewport size.
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.camera.viewport_size = Vec2::new(width, height);
    }

    /// Get the current surface width.
    pub fn surface_width(&self) -> Option<u32> {
        self.renderer.as_ref().map(|r| r.surface_width())
    }

    /// Get the current surface height.
    pub fn surface_height(&self) -> Option<u32> {
        self.renderer.as_ref().map(|r| r.surface_height())
    }

    /// Get adapter info string for debugging.
    pub fn adapter_info(&self) -> Option<String> {
        self.renderer.as_ref().map(|r| r.adapter_info())
    }

    /// Read-only view of the bloom configuration.
    pub fn bloom_config(&self) -> Option<&BloomConfig> {
        self.renderer.as_ref().map(|r| r.bloom_config())
    }

    /// Mutable access to the bloom configuration — tune threshold, intensity,
    /// iteration count, or disable bloom entirely.
    pub fn bloom_config_mut(&mut self) -> Option<&mut BloomConfig> {
        self.renderer.as_mut().map(|r| r.bloom_config_mut())
    }

    /// Upload line vertices for the next frame. Pairs of vertices form line
    /// segments. Empty slice (or no call) draws no lines this frame.
    pub fn set_lines(&mut self, vertices: &[LineVertex]) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_lines(vertices);
        }
    }

    /// Bound the game-world passes to a scissor rect in physical surface
    /// pixels (`None` = full window). Per-frame state, forwarded to the
    /// renderer like `set_lines` (issue #41).
    pub fn set_viewport_scissor(&mut self, scissor: Option<[u32; 4]>) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_viewport_scissor(scissor);
        }
    }
}

/// Position of the first entity with `Camera { is_main_camera: true }` and a
/// `Transform2D` — the world entity that drives the render camera, if any.
///
/// Public so the editor integration can mirror the game's camera onto the
/// editor viewport while a play session runs.
pub fn main_camera_position(world: &World) -> Option<Vec2> {
    main_camera_pose(world).map(|(position, _)| position)
}

/// Position AND zoom of the main-camera entity (the full pose the render
/// path honors — rotation is deliberately excluded, see
/// [`RenderManager::sync_main_camera`]).
///
/// A non-finite or non-positive authored zoom is replaced with `1.0` rather
/// than propagated — a `zoom: 0.0` in a scene file must never divide the
/// projection (or the editor viewport) by zero.
pub fn main_camera_pose(world: &World) -> Option<(Vec2, f32)> {
    world
        .entities()
        .into_iter()
        .find(|e| {
            world
                .get::<Camera>(*e)
                .map(|c| c.is_main_camera)
                .unwrap_or(false)
                && world.get::<Transform2D>(*e).is_some()
        })
        .and_then(|e| {
            let position = world.get::<Transform2D>(e).map(|t| t.position)?;
            let zoom = world
                .get::<Camera>(e)
                .map(|c| c.zoom)
                .filter(|z| z.is_finite() && *z > 0.0)
                .unwrap_or(1.0);
            Some((position, zoom))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_device_lost_is_fatal_immediately_regardless_of_streak() {
        let surface = RendererError::SurfaceError("lost".to_string());
        for (error, streak, action) in [
            (&RendererError::DeviceLost, 0, RenderErrorAction::Fatal),
            (&surface, 0, RenderErrorAction::RecreateSurface),
            (&surface, MAX_SURFACE_ERROR_STREAK - 1, RenderErrorAction::Fatal),
            (&RendererError::RenderingError("oom".to_string()), 0, RenderErrorAction::Propagate),
        ] {
            assert_eq!(classify_render_error(error, streak), action, "{error:?} at streak {streak}");
        }
    }

    #[test]
    fn fatal_render_manager_refuses_to_render() {
        let mut manager = RenderManager::new();
        assert!(!manager.is_fatal());

        assert_eq!(manager.note_render_error(&RendererError::DeviceLost), RenderErrorAction::Fatal);

        assert!(manager.is_fatal());
        // Never submit to a dead queue: the render call fails before any GPU work.
        let result = manager.render(&[], &[], &HashMap::new());
        assert!(matches!(result, Err(RendererError::DeviceLost)));
    }

    #[test]
    fn surface_error_streak_latches_fatal_without_device_lost_callback() {
        // The browser-bug backstop: the device died but wgpu's lost callback
        // never fired — repeated surface errors alone must stop rendering.
        let mut manager = RenderManager::new();
        let err = RendererError::SurfaceError("lost".to_string());
        for _ in 0..MAX_SURFACE_ERROR_STREAK - 1 {
            assert_eq!(manager.note_render_error(&err), RenderErrorAction::RecreateSurface);
            assert!(!manager.is_fatal());
        }
        assert_eq!(manager.note_render_error(&err), RenderErrorAction::Fatal);
        assert!(manager.is_fatal());
    }

    #[test]
    fn surface_error_streak_resets_on_successful_frame() {
        let mut manager = RenderManager::new();
        let err = RendererError::SurfaceError("lost".to_string());
        for _ in 0..MAX_SURFACE_ERROR_STREAK - 1 {
            manager.note_render_error(&err);
        }
        manager.note_render_success();
        // The next error starts a fresh streak — recreate, not fatal.
        assert_eq!(manager.note_render_error(&err), RenderErrorAction::RecreateSurface);
        assert!(!manager.is_fatal());
    }

    fn world_with_main_camera(zoom: f32) -> World {
        let mut world = World::new();
        let cam = world.create_entity();
        world.add_component(&cam, Transform2D::new(Vec2::new(320.0, -40.0))).ok();
        let mut camera = Camera::default().as_main_camera();
        camera.zoom = zoom;
        world.add_component(&cam, camera).ok();
        world
    }

    #[test]
    fn sync_main_camera_copies_position_and_sanitized_zoom_only() {
        let mut manager = RenderManager::new();
        // Resizing before a renderer exists only updates the camera viewport,
        // and the game loop's own two writes are observable on the camera.
        manager.resize(1024, 768);
        assert_eq!(manager.camera().viewport_size, Vec2::new(1024.0, 768.0));
        manager.set_viewport_size(640.0, 480.0);
        let viewport_before = manager.camera().viewport_size;
        assert_eq!(viewport_before, Vec2::new(640.0, 480.0));
        manager.camera_mut().position = Vec2::new(7.0, 9.0);
        assert_eq!(manager.camera().position, Vec2::new(7.0, 9.0));

        manager.sync_main_camera(&world_with_main_camera(2.0));

        assert_eq!(manager.camera().position, Vec2::new(320.0, -40.0));
        assert_eq!(manager.camera().zoom, 2.0);
        assert_eq!(manager.camera().viewport_size, viewport_before, "viewport stays render-managed");

        // A `zoom: 0.0` (or NaN, or negative) in a scene file must never
        // divide the projection or the editor viewport by zero.
        for bad_zoom in [0.0, f32::NAN, -1.5, f32::INFINITY] {
            let pose = main_camera_pose(&world_with_main_camera(bad_zoom));
            assert_eq!(pose, Some((Vec2::new(320.0, -40.0), 1.0)), "zoom {bad_zoom}");
        }

        // Non-main camera entities never drive the render camera.
        let mut world = World::new();
        let cam = world.create_entity();
        world.add_component(&cam, Transform2D::new(Vec2::new(1.0, 2.0))).ok();
        world.add_component(&cam, Camera::default()).ok();
        manager.sync_main_camera(&world);
        assert_eq!(manager.camera().position, Vec2::new(320.0, -40.0));
        assert_eq!(main_camera_pose(&world), None);
    }
}
