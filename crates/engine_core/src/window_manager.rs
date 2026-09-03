//! Window manager for handling window creation and lifecycle.
//!
//! This module provides a focused manager for window-related concerns,
//! following the Single Responsibility Principle.

use std::sync::Arc;

use winit::{
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

use renderer::RendererError;

/// Configuration for window creation.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
    /// Whether the window is resizable
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Insiculous 2D".to_string(),
            width: 800,
            height: 600,
            resizable: true,
        }
    }
}

impl WindowConfig {
    /// Create a new window configuration with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set the window size.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set whether the window is resizable.
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

impl From<&crate::game_config::GameConfig> for WindowConfig {
    fn from(config: &crate::game_config::GameConfig) -> Self {
        Self {
            title: config.title.clone(),
            width: config.width,
            height: config.height,
            resizable: config.resizable,
        }
    }
}

/// Manages window creation and lifecycle.
///
/// This struct encapsulates all window-related responsibilities:
/// - Window creation
/// - Window size tracking
/// - DPI scale factor tracking
/// - Window access
pub struct WindowManager {
    /// The window instance
    window: Option<Arc<Window>>,
    /// Current window configuration
    config: WindowConfig,
    /// DPI scale factor (1.0 = standard, 2.0 = HiDPI/Retina)
    scale_factor: f64,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new(WindowConfig::default())
    }
}

impl WindowManager {
    /// Create a new window manager with the given configuration.
    pub fn new(config: WindowConfig) -> Self {
        Self {
            window: None,
            config,
            scale_factor: 1.0,
        }
    }

    /// Create the window using the active event loop.
    ///
    /// # Arguments
    /// * `event_loop` - The active event loop from winit
    ///
    /// # Returns
    /// * `Ok(Arc<Window>)` on successful creation
    /// * `Err(RendererError)` if creation fails
    pub fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<Arc<Window>, RendererError> {
        let window_attributes = WindowAttributes::default()
            .with_title(&self.config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ))
            .with_resizable(self.config.resizable);

        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                // winit never inserts its canvas into the DOM; a detached
                // canvas renders silently into nothing.
                #[cfg(target_arch = "wasm32")]
                renderer::insert_canvas_into_dom(&window);
                self.scale_factor = window.scale_factor();
                // Track the PHYSICAL size from frame 0: the requested size
                // above is logical, and on HiDPI the surface (configured
                // from inner_size) is scale× larger. Without this, every
                // window_size consumer — UI layout, the render camera, the
                // viewport scissor — runs one frame (or more) at the
                // wrong scale until the first Resized event.
                // Skip a 0×0 report (web canvas before its first layout —
                // the real size arrives via resize()).
                let physical = window.inner_size();
                if physical.width > 0 && physical.height > 0 {
                    self.config.width = physical.width;
                    self.config.height = physical.height;
                }
                self.window = Some(window.clone());
                log::info!("Window created: {} (scale: {})", self.config.title, self.scale_factor);
                Ok(window)
            }
            Err(e) => {
                log::error!("Failed to create window: {}", e);
                Err(RendererError::WindowCreationError(e.to_string()))
            }
        }
    }

    /// Check if the window has been created.
    pub fn is_created(&self) -> bool {
        self.window.is_some()
    }

    /// Get a reference to the window if it exists.
    pub fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }

    /// Get a clone of the window Arc if it exists.
    pub fn window_clone(&self) -> Option<Arc<Window>> {
        self.window.clone()
    }

    /// Update the tracked window size.
    ///
    /// Call this when receiving resize events.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
    }

    /// Get the current window size.
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Get the current DPI scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Update the scale factor (call on ScaleFactorChanged event).
    pub fn set_scale_factor(&mut self, scale: f64) {
        self.scale_factor = scale;
    }

    /// Get logical size (for UI layout).
    pub fn logical_size(&self) -> (f32, f32) {
        (self.config.width as f32, self.config.height as f32)
    }

    /// Get physical size (for wgpu surface).
    pub fn physical_size(&self) -> (u32, u32) {
        (
            (self.config.width as f64 * self.scale_factor) as u32,
            (self.config.height as f64 * self.scale_factor) as u32,
        )
    }

    /// Get the current window width.
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Get the current window height.
    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// Get the window title.
    pub fn title(&self) -> &str {
        &self.config.title
    }

    /// Set the OS window title. Before the window exists the title is
    /// stored on the config, so window creation picks it up; headless
    /// (no window ever) this only updates the stored title — never panics.
    pub fn set_title(&mut self, title: &str) {
        self.config.title = title.to_string();
        if let Some(window) = &self.window {
            window.set_title(title);
        }
    }

    /// Request a redraw of the window.
    pub fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Get the window ID if the window exists.
    pub fn window_id(&self) -> Option<winit::window::WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    /// Check if an event belongs to this window.
    pub fn is_our_window(&self, window_id: winit::window::WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == window_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_size_scales_the_tracked_logical_size_by_the_scale_factor() {
        let mut manager = WindowManager::new(WindowConfig::new("Test").with_size(800, 600));
        manager.set_scale_factor(2.0);

        // A resize event updates the logical size the UI lays out against ...
        manager.resize(1280, 720);

        assert_eq!(manager.size(), (1280, 720));
        assert_eq!(manager.logical_size(), (1280.0, 720.0));
        // ... and the physical size the surface is configured with is DPI-scaled.
        assert_eq!(manager.physical_size(), (2560, 1440));
    }

    #[test]
    fn set_title_without_window_updates_config_for_creation() {
        // Headless (window never created) this must not panic, and a title
        // set before the window exists must be the one creation uses.
        let mut manager = WindowManager::new(WindowConfig::new("Initial"));

        manager.set_title("Scene* - Insiculous Editor");

        assert_eq!(manager.title(), "Scene* - Insiculous Editor");
    }
}
