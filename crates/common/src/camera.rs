//! Unified 2D Camera type.

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// 2D Camera with orthographic projection.
///
/// This is the canonical camera type used across the engine.
/// It combines features from both rendering and ECS requirements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    /// Camera position in world space
    pub position: Vec2,
    /// Camera rotation in radians
    pub rotation: f32,
    /// Zoom level (1.0 = normal, 2.0 = 2x zoom in, 0.5 = 2x zoom out)
    pub zoom: f32,
    /// Viewport dimensions in pixels
    pub viewport_size: Vec2,
    /// Near clipping plane (typically negative for 2D)
    pub near: f32,
    /// Far clipping plane (typically positive for 2D)
    pub far: f32,
    /// Whether this is the main/active camera for rendering
    pub is_main_camera: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            zoom: 1.0,
            viewport_size: Vec2::new(800.0, 600.0),
            near: -1000.0,
            far: 1000.0,
            is_main_camera: false,
        }
    }
}

impl Camera {
    /// Create a new camera at the given position with viewport size.
    #[inline]
    pub fn new(position: Vec2, viewport_size: Vec2) -> Self {
        Self {
            position,
            viewport_size,
            ..Default::default()
        }
    }

    /// Set as the main camera (builder pattern).
    #[inline]
    pub fn as_main_camera(mut self) -> Self {
        self.is_main_camera = true;
        self
    }

    /// Set rotation (builder pattern).
    #[inline]
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set zoom level (builder pattern).
    #[inline]
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Set clipping planes (builder pattern).
    #[inline]
    pub fn with_clipping(mut self, near: f32, far: f32) -> Self {
        self.near = near;
        self.far = far;
        self
    }

    /// Build the view matrix.
    ///
    /// The view matrix transforms world coordinates to view/camera space.
    pub fn view_matrix(&self) -> Mat4 {
        let mut view = Mat4::IDENTITY;

        // Apply zoom
        view *= Mat4::from_scale(Vec3::new(self.zoom, self.zoom, 1.0));

        // Apply rotation
        view *= Mat4::from_rotation_z(self.rotation);

        // Apply translation (negated for view matrix)
        view *= Mat4::from_translation(Vec3::new(-self.position.x, -self.position.y, 0.0));

        view
    }

    /// Build the orthographic projection matrix.
    pub fn projection_matrix(&self) -> Mat4 {
        let half_width = self.viewport_size.x * 0.5;
        let half_height = self.viewport_size.y * 0.5;

        Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            self.near,
            self.far,
        )
    }

    /// Build the combined view-projection matrix.
    #[inline]
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Convert screen coordinates to world coordinates.
    ///
    /// Screen coordinates have origin at top-left, Y increasing downward.
    /// World coordinates have Y increasing upward.
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // Convert to normalized device coordinates (-1 to 1)
        let ndc = Vec2::new(
            (screen_pos.x - self.viewport_size.x * 0.5) / (self.viewport_size.x * 0.5),
            (self.viewport_size.y * 0.5 - screen_pos.y) / (self.viewport_size.y * 0.5),
        );

        // Inverse of world_to_screen: unproject NDC through view-projection.
        let world_pos = self.view_projection_matrix().inverse() * Vec4::new(ndc.x, ndc.y, 0.0, 1.0);

        Vec2::new(world_pos.x, world_pos.y)
    }

    /// Convert world coordinates to screen coordinates.
    ///
    /// Returns screen position with origin at top-left.
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        // Transform by view-projection matrix
        let clip_pos = self.view_projection_matrix() * Vec4::new(world_pos.x, world_pos.y, 0.0, 1.0);

        // Convert to screen coordinates
        Vec2::new(
            (clip_pos.x + 1.0) * 0.5 * self.viewport_size.x,
            (1.0 - clip_pos.y) * 0.5 * self.viewport_size.y,
        )
    }

    /// Get the visible world bounds (min_x, min_y, max_x, max_y).
    pub fn world_bounds(&self) -> (f32, f32, f32, f32) {
        let half_w = self.viewport_size.x * 0.5 / self.zoom;
        let half_h = self.viewport_size.y * 0.5 / self.zoom;

        (
            self.position.x - half_w,
            self.position.y - half_h,
            self.position.x + half_w,
            self.position.y + half_h,
        )
    }

    /// Check if a point is visible in the camera's view.
    #[inline]
    pub fn contains_point(&self, point: Vec2) -> bool {
        let (min_x, min_y, max_x, max_y) = self.world_bounds();
        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
    }

    /// Update viewport size (call on window resize).
    #[inline]
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_size = Vec2::new(width, height);
    }
}

/// Camera uniform data for GPU upload.
///
/// This struct is designed to be directly uploadable to GPU buffers.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// The view-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Camera position for shader calculations
    pub position: [f32; 2],
    /// Padding to align to 16 bytes
    pub _padding: [f32; 2],
}

impl CameraUniform {
    /// Create camera uniform from a Camera2D.
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_projection: camera.view_projection_matrix().to_cols_array_2d(),
            position: camera.position.to_array(),
            _padding: [0.0, 0.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_to_screen_round_trips_screen_to_world() {
        let camera = Camera::new(Vec2::new(80.0, -20.0), Vec2::new(800.0, 600.0));
        let screen_center = Vec2::new(400.0, 300.0);
        let screen_point = Vec2::new(120.0, 90.0);

        let world_at_center = camera.screen_to_world(screen_center);
        let round_tripped = camera.world_to_screen(camera.screen_to_world(screen_point));

        assert!(
            (world_at_center - camera.position).length() < 0.001,
            "screen center must be the camera position, got {world_at_center:?}"
        );
        assert!(
            (round_tripped - screen_point).length() < 0.05,
            "expected {screen_point:?}, got {round_tripped:?}"
        );
    }

    #[test]
    fn test_screen_y_down_maps_to_world_y_up() {
        let camera = Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0));

        let above_center = camera.screen_to_world(Vec2::new(400.0, 100.0));
        let below_center = camera.screen_to_world(Vec2::new(400.0, 500.0));

        assert!(
            above_center.y > 0.0,
            "a point above screen center must be +Y in world, got {above_center:?}"
        );
        assert!(
            below_center.y < 0.0,
            "a point below screen center must be -Y in world, got {below_center:?}"
        );
    }

    /// The orthographic projection maps the viewport's half extents to the
    /// NDC corners; get this wrong and every sprite is scaled or off-centre.
    #[test]
    fn test_projection_maps_viewport_half_extents_to_ndc_corners() {
        let camera = Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0));
        let projection = camera.projection_matrix();

        let top_right = projection * Vec4::new(400.0, 300.0, 0.0, 1.0);
        let bottom_left = projection * Vec4::new(-400.0, -300.0, 0.0, 1.0);

        assert!(
            (Vec2::new(top_right.x, top_right.y) - Vec2::new(1.0, 1.0)).length() < 1e-4,
            "(+half_w, +half_h) must be NDC (1, 1), got {top_right:?}"
        );
        assert!(
            (Vec2::new(bottom_left.x, bottom_left.y) - Vec2::new(-1.0, -1.0)).length() < 1e-4,
            "(-half_w, -half_h) must be NDC (-1, -1), got {bottom_left:?}"
        );
    }

    /// Both GPU pipelines upload this uniform every frame; the shader reads
    /// the view-projection as column-major columns and the position after it.
    #[test]
    fn test_camera_uniform_carries_the_view_projection_and_position() {
        let camera = Camera::new(Vec2::new(50.0, 100.0), Vec2::new(800.0, 600.0)).with_zoom(2.0);

        let uniform = CameraUniform::from_camera(&camera);

        assert_eq!(uniform.view_projection, camera.view_projection_matrix().to_cols_array_2d());
        assert_eq!(uniform.position, [50.0, 100.0]);
        assert_eq!(uniform._padding, [0.0, 0.0]);
    }

    /// Zoom scales the view (2x zoom moves world (10, 10) to view (20, 20))
    /// and `view_projection_matrix` composes view first, then projection.
    #[test]
    fn test_zoom_scales_the_view_and_view_projection_composes_view_then_projection() {
        let camera = Camera::new(Vec2::ZERO, Vec2::new(800.0, 600.0)).with_zoom(2.0);
        let world_point = Vec4::new(10.0, 10.0, 0.0, 1.0);

        let in_view = camera.view_matrix() * world_point;
        let via_view_projection = camera.view_projection_matrix() * world_point;
        let via_composition = camera.projection_matrix() * (camera.view_matrix() * world_point);

        assert!(
            (Vec2::new(in_view.x, in_view.y) - Vec2::new(20.0, 20.0)).length() < 1e-4,
            "2x zoom must double view-space coordinates, got {in_view:?}"
        );
        assert!(
            (via_view_projection - via_composition).length() < 1e-5,
            "view_projection must equal projection * view, got {via_view_projection:?} vs {via_composition:?}"
        );
    }
}
