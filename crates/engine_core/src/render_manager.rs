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
        let sprite_pipeline = SpritePipeline::new(renderer.device_ref(), 1000);
        // UI draws post-tonemap straight to the swapchain (issue #26).
        let ui_pipeline =
            SpritePipeline::new_ui(renderer.device_ref(), 1000, renderer.surface_format());

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

    /// Handle a render error, attempting surface recreation on surface loss.
    fn handle_render_error(renderer: &mut Renderer, error: RendererError) -> Result<(), RendererError> {
        match error {
            RendererError::SurfaceError(_) => {
                if let Err(e) = renderer.recreate_surface() {
                    log::error!("Failed to recreate surface: {}", e);
                    return Err(e);
                }
                log::debug!("Surface recreated after loss");
                Ok(())
            }
            e => {
                log::error!("Render error: {}", e);
                Err(e)
            }
        }
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
            Ok(_) => Ok(()),
            Err(e) => Self::handle_render_error(renderer, e),
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
        });
        self.render(&batch_refs, &[], textures)
    }


    /// Get the GPU device if the renderer is initialized.
    pub fn device(&self) -> Option<Arc<Device>> {
        self.renderer.as_ref().map(|r| r.device())
    }

    /// Get the GPU queue if the renderer is initialized.
    pub fn queue(&self) -> Option<Arc<Queue>> {
        self.renderer.as_ref().map(|r| r.queue())
    }

    /// Get a reference to the device (for pipeline creation).
    pub fn device_ref(&self) -> Option<&Device> {
        self.renderer.as_ref().map(|r| r.device_ref())
    }

    /// Get a reference to the camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Copy the main camera entity's position onto the render camera.
    ///
    /// Only `position` is synced — zoom, rotation, and viewport_size stay
    /// render-managed (viewport_size tracks window resizes). Worlds without
    /// a main-camera entity are untouched, and games can still override
    /// `ctx.camera` in `render()` afterwards.
    pub fn sync_main_camera(&mut self, world: &World) {
        if let Some(position) = main_camera_position(world) {
            self.camera.position = position;
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
}

/// Position of the first entity with `Camera { is_main_camera: true }` and a
/// `Transform2D` — the world entity that drives the render camera, if any.
///
/// Public so the editor integration can mirror the game's camera onto the
/// editor viewport while a play session runs.
pub fn main_camera_position(world: &World) -> Option<Vec2> {
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
        .and_then(|e| world.get::<Transform2D>(e).map(|t| t.position))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_manager_new() {
        let manager = RenderManager::new();
        assert!(!manager.is_initialized());
        assert!(manager.device().is_none());
        assert!(manager.queue().is_none());
    }

    #[test]
    fn test_render_manager_default() {
        let manager = RenderManager::default();
        assert!(!manager.is_initialized());
    }

    #[test]
    fn test_camera_access() {
        let mut manager = RenderManager::new();

        // Test initial camera state - Camera2D::default() sets viewport to 800x600
        assert_eq!(manager.camera().viewport_size, Vec2::new(800.0, 600.0));

        // Test camera mutation
        manager.set_viewport_size(1024.0, 768.0);
        assert_eq!(manager.camera().viewport_size, Vec2::new(1024.0, 768.0));

        // Test mutable access
        manager.camera_mut().position = Vec2::new(100.0, 50.0);
        assert_eq!(manager.camera().position, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn test_sync_main_camera_copies_main_camera_entity_position() {
        let mut manager = RenderManager::new();
        let mut world = World::new();

        let cam = world.create_entity();
        world
            .add_component(&cam, Transform2D::new(Vec2::new(320.0, -40.0)))
            .unwrap();
        world
            .add_component(&cam, Camera::default().as_main_camera())
            .unwrap();

        let viewport_before = manager.camera().viewport_size;
        manager.sync_main_camera(&world);

        assert_eq!(manager.camera().position, Vec2::new(320.0, -40.0));
        // Only position syncs — viewport stays render-managed.
        assert_eq!(manager.camera().viewport_size, viewport_before);
    }

    #[test]
    fn test_sync_main_camera_is_noop_without_main_camera_entity() {
        let mut manager = RenderManager::new();
        manager.camera_mut().position = Vec2::new(7.0, 9.0);

        let mut world = World::new();
        // Non-main camera entities must not drive the render camera.
        let cam = world.create_entity();
        world
            .add_component(&cam, Transform2D::new(Vec2::new(1.0, 2.0)))
            .unwrap();
        world.add_component(&cam, Camera::default()).unwrap();

        manager.sync_main_camera(&world);
        assert_eq!(manager.camera().position, Vec2::new(7.0, 9.0));
    }

    #[test]
    fn test_resize_without_renderer() {
        let mut manager = RenderManager::new();
        // Should not panic, just updates camera
        manager.resize(1024, 768);
        assert_eq!(manager.camera().viewport_size, Vec2::new(1024.0, 768.0));
    }
}
