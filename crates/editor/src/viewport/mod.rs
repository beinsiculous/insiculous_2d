//! Scene viewport rendering for the editor.
//!
//! The SceneViewport handles rendering game entities within the scene view panel,
//! managing camera transforms, and converting between screen and world coordinates.

use common::{Camera, Rect};
use glam::Vec2;

/// Manages rendering the game world within the scene view panel.
///
/// The SceneViewport coordinates:
/// - Camera viewport calculation from panel bounds
/// - World-to-screen coordinate transformations
/// - Entity sprite generation within viewport region
#[derive(Debug, Clone)]
pub struct SceneViewport {
    /// Current viewport bounds in screen space (from DockPanel)
    viewport_bounds: Rect,
    /// Camera position in world space (pan offset)
    camera_position: Vec2,
    /// Camera zoom level (1.0 = normal, 2.0 = zoomed in 2x)
    camera_zoom: f32,
    /// Target camera position for smooth interpolation
    target_camera_position: Vec2,
    /// Target zoom for smooth interpolation
    target_camera_zoom: f32,
    /// Interpolation speed (0.0-1.0, higher = snappier)
    interpolation_speed: f32,
}

impl Default for SceneViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneViewport {
    /// Create a new scene viewport with default settings.
    pub fn new() -> Self {
        Self {
            viewport_bounds: Rect::default(),
            camera_position: Vec2::ZERO,
            camera_zoom: 1.0,
            target_camera_position: Vec2::ZERO,
            target_camera_zoom: 1.0,
            interpolation_speed: 0.15,
        }
    }

    /// Set the viewport bounds (from the scene view panel content bounds).
    pub fn set_viewport_bounds(&mut self, bounds: Rect) {
        self.viewport_bounds = bounds;
    }

    /// Get the current viewport bounds.
    pub fn viewport_bounds(&self) -> Rect {
        self.viewport_bounds
    }

    /// Get the viewport center in screen coordinates.
    pub fn viewport_center(&self) -> Vec2 {
        Vec2::new(
            self.viewport_bounds.x + self.viewport_bounds.width * 0.5,
            self.viewport_bounds.y + self.viewport_bounds.height * 0.5,
        )
    }

    /// Get the viewport size.
    pub fn viewport_size(&self) -> Vec2 {
        Vec2::new(self.viewport_bounds.width, self.viewport_bounds.height)
    }

    // ================== Camera Methods ==================

    /// Get the current camera position.
    pub fn camera_position(&self) -> Vec2 {
        self.camera_position
    }

    /// Set the camera position directly (no interpolation).
    pub fn set_camera_position(&mut self, position: Vec2) {
        self.camera_position = position;
        self.target_camera_position = position;
    }

    /// Set the target camera position (will interpolate).
    pub fn set_target_camera_position(&mut self, position: Vec2) {
        self.target_camera_position = position;
    }

    /// Camera position the viewport is interpolating toward.
    pub fn target_camera_position(&self) -> Vec2 {
        self.target_camera_position
    }

    /// Camera zoom the viewport is interpolating toward.
    pub fn target_camera_zoom(&self) -> f32 {
        self.target_camera_zoom
    }

    /// Pan the camera by a delta amount (in world space).
    pub fn pan(&mut self, delta: Vec2) {
        self.target_camera_position += delta;
    }

    /// Pan the camera immediately (no interpolation).
    pub fn pan_immediate(&mut self, delta: Vec2) {
        self.camera_position += delta;
        self.target_camera_position = self.camera_position;
    }

    /// Get the current camera zoom level.
    pub fn camera_zoom(&self) -> f32 {
        self.camera_zoom
    }

    /// Set the camera zoom directly (no interpolation).
    pub fn set_camera_zoom(&mut self, zoom: f32) {
        let clamped = zoom.clamp(0.1, 10.0);
        self.camera_zoom = clamped;
        self.target_camera_zoom = clamped;
    }

    /// Adopt a zoom from the GAME's main camera without the interactive
    /// [0.1, 10.0] UX clamp: the follow view must match
    /// the shipped game exactly, even at extreme authored zooms. Non-finite
    /// or non-positive values fall back to 1.0 — the viewport's world↔screen
    /// math divides by zoom and must never see 0 or NaN.
    pub fn adopt_camera_zoom(&mut self, zoom: f32) {
        let safe = if zoom.is_finite() && zoom > 0.0 { zoom } else { 1.0 };
        self.camera_zoom = safe;
        self.target_camera_zoom = safe;
    }

    /// Set the target zoom level (will interpolate).
    pub fn set_target_zoom(&mut self, zoom: f32) {
        self.target_camera_zoom = zoom.clamp(0.1, 10.0);
    }

    /// Zoom the camera by a factor centered on a screen position.
    ///
    /// The zoom is centered on the given screen position so the world point
    /// under the cursor stays fixed.
    pub fn zoom_at(&mut self, factor: f32, screen_pos: Vec2) {
        let old_zoom = self.target_camera_zoom;
        let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);

        // Calculate world position under cursor before zoom
        let world_before = self.screen_to_world(screen_pos);

        // Apply new zoom
        self.target_camera_zoom = new_zoom;

        // Calculate world position under cursor after zoom (with new zoom but old camera pos)
        let temp_zoom = self.camera_zoom;
        self.camera_zoom = new_zoom;
        let world_after = self.screen_to_world(screen_pos);
        self.camera_zoom = temp_zoom;

        // Adjust camera position to keep world_before at the same screen position
        self.target_camera_position += world_before - world_after;
    }

    /// Reset the camera to default view.
    pub fn reset_camera(&mut self) {
        self.target_camera_position = Vec2::ZERO;
        self.target_camera_zoom = 1.0;
    }

    /// Reset camera immediately (no interpolation).
    pub fn reset_camera_immediate(&mut self) {
        self.camera_position = Vec2::ZERO;
        self.camera_zoom = 1.0;
        self.target_camera_position = Vec2::ZERO;
        self.target_camera_zoom = 1.0;
    }

    /// Update camera interpolation. Call each frame.
    pub fn update(&mut self, delta_time: f32) {
        // Exponential decay toward the target: `interpolation_speed` is the
        // per-frame factor AT 60 FPS, converted so any frame rate covers the
        // same distance over the same wall-clock time (two 1/120s steps
        // compose to exactly one 1/60s step).
        let t = 1.0 - (1.0 - self.interpolation_speed).powf(delta_time * 60.0);
        self.camera_position = self.camera_position.lerp(self.target_camera_position, t);
        self.camera_zoom = self.camera_zoom + (self.target_camera_zoom - self.camera_zoom) * t;
    }

    /// Set interpolation speed (0.0 = no movement, 1.0 = instant).
    pub fn set_interpolation_speed(&mut self, speed: f32) {
        self.interpolation_speed = speed.clamp(0.0, 1.0);
    }

    // ================== Coordinate Conversion ==================

    /// Convert screen coordinates to world coordinates.
    ///
    /// Screen coordinates have origin at top-left of window.
    /// World coordinates have origin at camera position.
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let viewport_center = self.viewport_center();

        // Convert screen position relative to viewport center
        let relative = Vec2::new(
            screen_pos.x - viewport_center.x,
            viewport_center.y - screen_pos.y, // Flip Y for world coords
        );

        // Scale by zoom and add camera offset
        relative / self.camera_zoom + self.camera_position
    }

    /// Convert world coordinates to screen coordinates.
    ///
    /// Returns screen position with origin at top-left of window.
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        let viewport_center = self.viewport_center();

        // Convert world position relative to camera
        let relative = (world_pos - self.camera_position) * self.camera_zoom;

        // Convert to screen coordinates (flip Y)
        Vec2::new(
            viewport_center.x + relative.x,
            viewport_center.y - relative.y,
        )
    }

    /// Check if a screen position is within the viewport bounds.
    pub fn contains_screen_point(&self, screen_pos: Vec2) -> bool {
        self.viewport_bounds.contains(screen_pos)
    }

    /// Get the visible world bounds (min_x, min_y, max_x, max_y).
    pub fn visible_world_bounds(&self) -> (f32, f32, f32, f32) {
        let half_w = self.viewport_bounds.width * 0.5 / self.camera_zoom;
        let half_h = self.viewport_bounds.height * 0.5 / self.camera_zoom;

        (
            self.camera_position.x - half_w,
            self.camera_position.y - half_h,
            self.camera_position.x + half_w,
            self.camera_position.y + half_h,
        )
    }

    /// Render camera that reproduces this viewport's world→screen mapping
    /// when the GPU renders to a full-window surface of `window_size`.
    ///
    /// The GPU projection centers the world on the window; this viewport
    /// centers it on the scene panel. Offsetting the camera position by the
    /// panel-center-to-window-center delta (Y flipped for the Y-up world,
    /// zoom-scaled) makes both transforms produce identical screen
    /// coordinates, so sprites land exactly where the editor overlay
    /// (gizmo, picking, grid) expects them.
    pub fn to_window_render_camera(&self, window_size: Vec2) -> Camera {
        let viewport_center = self.viewport_center();
        let offset = Vec2::new(
            (window_size.x * 0.5 - viewport_center.x) / self.camera_zoom,
            (viewport_center.y - window_size.y * 0.5) / self.camera_zoom,
        );
        Camera::new(self.camera_position + offset, window_size).with_zoom(self.camera_zoom)
    }

    /// Focus the camera on multiple positions (center of bounding box).
    pub fn focus_on_bounds(&mut self, positions: &[Vec2]) {
        if positions.is_empty() {
            return;
        }

        let mut min = positions[0];
        let mut max = positions[0];

        for pos in positions {
            min = min.min(*pos);
            max = max.max(*pos);
        }

        let center = (min + max) * 0.5;
        self.target_camera_position = center;

        // Optionally adjust zoom to fit bounds
        let bounds_size = max - min;
        let viewport_size = self.viewport_size();
        if bounds_size.x > 0.0 && bounds_size.y > 0.0 {
            let zoom_x = viewport_size.x / (bounds_size.x + 100.0); // Add padding
            let zoom_y = viewport_size.y / (bounds_size.y + 100.0);
            self.target_camera_zoom = zoom_x.min(zoom_y).clamp(0.1, 10.0);
        }
    }
}


#[cfg(test)]
mod tests;
