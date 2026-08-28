//! Authoring grid overlay for the scene viewport.
//!
//! Produces world-space line segments (pure, headless-testable) that the
//! scene view draws as screen-space `ui.line` calls clipped to the panel —
//! the same pattern as the collider overlay. The authoring grid is a square,
//! axis-aligned, zoom-adaptive ruler with distinguished origin axes; it is
//! deliberately separate from the game's deforming spring-grid effect.

use glam::Vec2;
use ui::{Color, Rect, UIContext};

use crate::viewport::SceneViewport;

/// Colors for grid rendering.
#[derive(Debug, Clone)]
pub struct GridColors {
    /// Primary grid line color
    pub primary: Color,
    /// Secondary (subdivision) grid line color
    pub secondary: Color,
    /// X axis color (typically red)
    pub axis_x: Color,
    /// Y axis color (typically green)
    pub axis_y: Color,
}

impl Default for GridColors {
    fn default() -> Self {
        Self {
            primary: Color::new(0.3, 0.3, 0.3, 0.5),
            secondary: Color::new(0.25, 0.25, 0.25, 0.3),
            axis_x: Color::new(0.8, 0.2, 0.2, 0.8),
            axis_y: Color::new(0.2, 0.8, 0.2, 0.8),
        }
    }
}

impl GridColors {
    /// The color for a grid line of the given kind.
    pub fn color_for(&self, kind: GridLineKind) -> Color {
        match kind {
            GridLineKind::Primary => self.primary,
            GridLineKind::Secondary => self.secondary,
            GridLineKind::AxisX => self.axis_x,
            GridLineKind::AxisY => self.axis_y,
        }
    }
}

/// Configuration for grid rendering.
#[derive(Debug, Clone)]
pub struct GridConfig {
    /// Size of primary grid cells in world units
    pub primary_size: f32,
    /// Number of subdivisions per primary cell (0 = no subdivisions)
    pub subdivisions: u32,
    /// Line thickness in screen pixels
    pub line_thickness: f32,
    /// Axis line thickness in screen pixels
    pub axis_thickness: f32,
    /// Maximum number of grid lines to render (LOD limit)
    pub max_lines: usize,
    /// Minimum zoom level to show subdivisions
    pub subdivision_min_zoom: f32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            primary_size: 32.0,
            subdivisions: 4,
            line_thickness: 1.0,
            axis_thickness: 2.0,
            max_lines: 200,
            subdivision_min_zoom: 0.5,
        }
    }
}

impl GridConfig {
    /// The screen-pixel line width for a grid line of the given kind.
    pub fn width_for(&self, kind: GridLineKind) -> f32 {
        match kind {
            GridLineKind::Primary | GridLineKind::Secondary => self.line_thickness,
            GridLineKind::AxisX | GridLineKind::AxisY => self.axis_thickness,
        }
    }
}

/// Which visual tier a grid segment belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridLineKind {
    /// Primary grid line (every LOD-adjusted cell)
    Primary,
    /// Subdivision line between primary lines
    Secondary,
    /// The world X axis (y = 0)
    AxisX,
    /// The world Y axis (x = 0)
    AxisY,
}

/// One world-space grid line spanning the visible bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSegment {
    /// World-space start point
    pub start: Vec2,
    /// World-space end point
    pub end: Vec2,
    /// Visual tier of this line
    pub kind: GridLineKind,
}

/// Computes the authoring grid for the scene viewport.
///
/// The grid consists of:
/// - Primary grid lines at regular intervals (LOD-merged when zoomed out)
/// - Secondary subdivision lines (visible at higher zoom)
/// - X and Y axis lines through the origin
#[derive(Debug, Clone)]
pub struct GridRenderer {
    /// Grid configuration
    pub config: GridConfig,
    /// Grid colors
    pub colors: GridColors,
    /// Whether the grid is visible
    visible: bool,
    /// Whether axes are visible
    axes_visible: bool,
}

impl Default for GridRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GridRenderer {
    /// Create a new grid renderer with default settings.
    pub fn new() -> Self {
        Self {
            config: GridConfig::default(),
            colors: GridColors::default(),
            visible: true,
            axes_visible: true,
        }
    }

    /// Set grid visibility.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Check if the grid is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Toggle grid visibility.
    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }

    /// Set axes visibility.
    pub fn set_axes_visible(&mut self, visible: bool) {
        self.axes_visible = visible;
    }

    /// Set primary grid size.
    pub fn set_grid_size(&mut self, size: f32) {
        self.config.primary_size = size.max(1.0);
    }

    /// Get primary grid size.
    pub fn grid_size(&self) -> f32 {
        self.config.primary_size
    }

    /// World-space grid segments for the visible bounds. Empty when hidden.
    ///
    /// `visible_bounds` is `(min_x, min_y, max_x, max_y)` in world units
    /// (`SceneViewport::visible_world_bounds`). Primary/subdivision lines
    /// respect the `max_lines` LOD cap; origin axes are always included
    /// while `axes_visible`.
    pub fn grid_segments(
        &self,
        visible_bounds: (f32, f32, f32, f32),
        camera_zoom: f32,
    ) -> Vec<GridSegment> {
        if !self.visible {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let (min_x, min_y, max_x, max_y) = visible_bounds;

        // Calculate effective grid size based on zoom (LOD)
        let effective_grid_size = self.calculate_lod_grid_size(camera_zoom);

        let (h_lines, v_lines) =
            self.calculate_grid_lines(min_x, min_y, max_x, max_y, effective_grid_size);

        // Check LOD limit
        let total_lines = h_lines.len() + v_lines.len();
        if total_lines <= self.config.max_lines {
            // Subdivision lines when zoomed in far enough
            if camera_zoom >= self.config.subdivision_min_zoom && self.config.subdivisions > 0 {
                let sub_size = effective_grid_size / self.config.subdivisions as f32;
                let (h_sub, v_sub) =
                    self.calculate_grid_lines(min_x, min_y, max_x, max_y, sub_size);

                // Skip subdivision lines that coincide with primary lines
                for y in h_sub {
                    if !is_on_grid(y, effective_grid_size) {
                        segments.push(horizontal_segment(
                            (min_x, max_x),
                            y,
                            GridLineKind::Secondary,
                        ));
                    }
                }
                for x in v_sub {
                    if !is_on_grid(x, effective_grid_size) {
                        segments.push(vertical_segment(
                            (min_y, max_y),
                            x,
                            GridLineKind::Secondary,
                        ));
                    }
                }
            }

            // Primary grid lines (the axes are drawn separately below)
            for y in h_lines {
                if y.abs() < 0.001 {
                    continue;
                }
                segments.push(horizontal_segment((min_x, max_x), y, GridLineKind::Primary));
            }
            for x in v_lines {
                if x.abs() < 0.001 {
                    continue;
                }
                segments.push(vertical_segment((min_y, max_y), x, GridLineKind::Primary));
            }
        }

        // Always render axes if visible
        if self.axes_visible {
            segments.push(horizontal_segment((min_x, max_x), 0.0, GridLineKind::AxisX));
            segments.push(vertical_segment((min_y, max_y), 0.0, GridLineKind::AxisY));
        }

        segments
    }

    /// Calculate grid size with LOD (level of detail) based on zoom.
    ///
    /// At lower zoom levels, grid cells are merged to maintain readable density.
    fn calculate_lod_grid_size(&self, camera_zoom: f32) -> f32 {
        let base_size = self.config.primary_size;

        // Scale grid size inversely with zoom to maintain visual density
        // At zoom 0.5, double the grid size; at zoom 0.25, quadruple it
        if camera_zoom < 1.0 {
            let multiplier = (1.0 / camera_zoom).log2().ceil().exp2();
            base_size * multiplier
        } else {
            base_size
        }
    }

    /// Calculate grid line positions for the given bounds.
    fn calculate_grid_lines(
        &self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        grid_size: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        // Horizontal lines (varying Y)
        let start_y = (min_y / grid_size).floor() * grid_size;
        let end_y = (max_y / grid_size).ceil() * grid_size;
        let h_lines: Vec<f32> = (0..)
            .map(|i| start_y + i as f32 * grid_size)
            .take_while(|&y| y <= end_y)
            .collect();

        // Vertical lines (varying X)
        let start_x = (min_x / grid_size).floor() * grid_size;
        let end_x = (max_x / grid_size).ceil() * grid_size;
        let v_lines: Vec<f32> = (0..)
            .map(|i| start_x + i as f32 * grid_size)
            .take_while(|&x| x <= end_x)
            .collect();

        (h_lines, v_lines)
    }
}

/// A horizontal grid line at `y` spanning `(min_x, max_x)`.
fn horizontal_segment(span_x: (f32, f32), y: f32, kind: GridLineKind) -> GridSegment {
    GridSegment {
        start: Vec2::new(span_x.0, y),
        end: Vec2::new(span_x.1, y),
        kind,
    }
}

/// A vertical grid line at `x` spanning `(min_y, max_y)`.
fn vertical_segment(span_y: (f32, f32), x: f32, kind: GridLineKind) -> GridSegment {
    GridSegment {
        start: Vec2::new(x, span_y.0),
        end: Vec2::new(x, span_y.1),
        kind,
    }
}

/// Check if a value falls on the grid (within floating point tolerance).
fn is_on_grid(value: f32, grid_size: f32) -> bool {
    let remainder = (value / grid_size).fract().abs();
    !(0.001..=0.999).contains(&remainder)
}

/// Draw the authoring grid for the current view, clipped to the scene-view
/// `bounds`. World geometry comes from `grid_segments`; this maps it through
/// `viewport.world_to_screen` like the collider overlay does.
pub fn render_grid_overlay(
    ui: &mut UIContext,
    grid: &GridRenderer,
    viewport: &SceneViewport,
    colors: &GridColors,
    bounds: Rect,
) {
    if !grid.is_visible() {
        return;
    }
    ui.push_clip_rect(bounds);
    let visible_bounds = viewport.visible_world_bounds();
    for segment in grid.grid_segments(visible_bounds, viewport.camera_zoom()) {
        let start = viewport.world_to_screen(segment.start);
        let end = viewport.world_to_screen(segment.end);
        // An extreme camera state can produce non-finite screen coordinates;
        // never feed those into the draw list.
        if !start.is_finite() || !end.is_finite() {
            continue;
        }
        ui.line(
            start,
            end,
            colors.color_for(segment.kind),
            grid.config.width_for(segment.kind),
        );
    }
    ui.pop_clip_rect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds_200() -> (f32, f32, f32, f32) {
        (-100.0, -100.0, 100.0, 100.0)
    }

    #[test]
    fn test_grid_renderer_new() {
        let grid = GridRenderer::new();
        assert!(grid.is_visible());
        assert_eq!(grid.grid_size(), 32.0);
    }

    #[test]
    fn test_grid_visibility_toggle() {
        let mut grid = GridRenderer::new();
        assert!(grid.is_visible());

        grid.toggle_visible();
        assert!(!grid.is_visible());

        grid.toggle_visible();
        assert!(grid.is_visible());
    }

    #[test]
    fn test_grid_size_setting() {
        let mut grid = GridRenderer::new();
        grid.set_grid_size(64.0);
        assert_eq!(grid.grid_size(), 64.0);

        // Minimum size enforcement
        grid.set_grid_size(0.5);
        assert_eq!(grid.grid_size(), 1.0);
    }

    #[test]
    fn test_hidden_grid_produces_no_segments() {
        let mut grid = GridRenderer::new();
        grid.set_visible(false);
        assert!(grid.grid_segments(bounds_200(), 1.0).is_empty());
    }

    #[test]
    fn test_segments_include_axes_spanning_the_bounds() {
        let grid = GridRenderer::new();
        let segments = grid.grid_segments(bounds_200(), 1.0);

        let x_axis = segments
            .iter()
            .find(|s| s.kind == GridLineKind::AxisX)
            .expect("X axis rendered");
        assert_eq!(x_axis.start, Vec2::new(-100.0, 0.0));
        assert_eq!(x_axis.end, Vec2::new(100.0, 0.0));

        let y_axis = segments
            .iter()
            .find(|s| s.kind == GridLineKind::AxisY)
            .expect("Y axis rendered");
        assert_eq!(y_axis.start, Vec2::new(0.0, -100.0));
        assert_eq!(y_axis.end, Vec2::new(0.0, 100.0));
    }

    #[test]
    fn test_primary_lines_land_on_grid_size_multiples() {
        let grid = GridRenderer::new();
        let segments = grid.grid_segments(bounds_200(), 1.0);

        let primaries: Vec<_> = segments
            .iter()
            .filter(|s| s.kind == GridLineKind::Primary)
            .collect();
        assert!(!primaries.is_empty());
        for segment in primaries {
            // A primary line is axis-aligned; its constant coordinate sits
            // on a multiple of the grid size and never on the origin axes
            // (those are drawn as axis segments).
            let coord = if segment.start.y == segment.end.y {
                segment.start.y
            } else {
                segment.start.x
            };
            assert!(is_on_grid(coord, 32.0), "line at {coord} off-grid");
            assert!(coord.abs() > 0.001, "axis line duplicated as primary");
        }
    }

    #[test]
    fn test_lod_grid_size() {
        let grid = GridRenderer::new();

        // At zoom 1.0, should use base size
        assert_eq!(grid.calculate_lod_grid_size(1.0), 32.0);
        // At zoom 0.5, should double
        assert_eq!(grid.calculate_lod_grid_size(0.5), 64.0);
        // At zoom 2.0, should use base size (no reduction)
        assert_eq!(grid.calculate_lod_grid_size(2.0), 32.0);
    }

    #[test]
    fn test_lod_doubles_primary_spacing_when_zoomed_out() {
        let grid = GridRenderer::new();
        let segments = grid.grid_segments(bounds_200(), 0.5);
        for segment in segments.iter().filter(|s| s.kind == GridLineKind::Primary) {
            let coord = if segment.start.y == segment.end.y {
                segment.start.y
            } else {
                segment.start.x
            };
            assert!(is_on_grid(coord, 64.0), "line at {coord} off the LOD grid");
        }
    }

    #[test]
    fn test_subdivisions_gated_by_zoom_and_never_on_primary_lines() {
        let grid = GridRenderer::new();

        // Below subdivision_min_zoom (0.5): no secondary lines.
        let zoomed_out = grid.grid_segments(bounds_200(), 0.4);
        assert!(
            !zoomed_out.iter().any(|s| s.kind == GridLineKind::Secondary),
            "subdivisions must hide below the zoom threshold"
        );

        // At zoom 1.0: secondary lines exist between primaries, never on them.
        let zoomed_in = grid.grid_segments(bounds_200(), 1.0);
        let secondaries: Vec<_> = zoomed_in
            .iter()
            .filter(|s| s.kind == GridLineKind::Secondary)
            .collect();
        assert!(!secondaries.is_empty());
        for segment in secondaries {
            let coord = if segment.start.y == segment.end.y {
                segment.start.y
            } else {
                segment.start.x
            };
            assert!(
                !is_on_grid(coord, 32.0),
                "subdivision at {coord} coincides with a primary line"
            );
        }
    }

    #[test]
    fn test_calculate_grid_lines() {
        let grid = GridRenderer::new();
        let (h_lines, v_lines) = grid.calculate_grid_lines(-64.0, -64.0, 64.0, 64.0, 32.0);

        // Should have lines at -64, -32, 0, 32, 64
        assert!(h_lines.len() >= 5);
        assert!(v_lines.len() >= 5);

        // Check that 0 is included
        assert!(h_lines.iter().any(|&y| y.abs() < 0.001));
        assert!(v_lines.iter().any(|&x| x.abs() < 0.001));
    }

    #[test]
    fn test_is_on_grid() {
        assert!(is_on_grid(0.0, 32.0));
        assert!(is_on_grid(32.0, 32.0));
        assert!(is_on_grid(64.0, 32.0));
        assert!(is_on_grid(-32.0, 32.0));

        assert!(!is_on_grid(16.0, 32.0));
        assert!(!is_on_grid(8.0, 32.0));
    }

    #[test]
    fn test_max_lines_exceeded_leaves_only_axes() {
        let mut grid = GridRenderer::new();
        grid.config.max_lines = 10;

        let segments = grid.grid_segments((-10000.0, -10000.0, 10000.0, 10000.0), 4.0);
        assert_eq!(segments.len(), 2);
        assert!(segments.iter().any(|s| s.kind == GridLineKind::AxisX));
        assert!(segments.iter().any(|s| s.kind == GridLineKind::AxisY));
    }

    #[test]
    fn test_render_overlay_hidden_grid_draws_nothing() {
        let mut grid = GridRenderer::new();
        grid.set_visible(false);
        let mut ui = UIContext::new();
        let viewport = SceneViewport::new();

        render_grid_overlay(
            &mut ui,
            &grid,
            &viewport,
            &GridColors::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );

        assert!(ui.draw_list().commands().is_empty());
    }

    #[test]
    fn test_render_overlay_emits_clipped_lines() {
        let grid = GridRenderer::new();
        let mut ui = UIContext::new();
        let mut viewport = SceneViewport::new();
        viewport.set_viewport_bounds(Rect::new(0.0, 0.0, 800.0, 600.0));

        render_grid_overlay(
            &mut ui,
            &grid,
            &viewport,
            &GridColors::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );

        let commands = ui.draw_list().commands();
        let lines = commands
            .iter()
            .filter(|c| matches!(c, ui::DrawCommand::Line { .. }))
            .count();
        assert!(lines > 2, "grid plus axes expected, got {lines} lines");
        assert!(commands
            .iter()
            .any(|c| matches!(c, ui::DrawCommand::PushClipRect { .. })));
        assert!(commands
            .iter()
            .any(|c| matches!(c, ui::DrawCommand::PopClipRect)));
    }
}
