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
#[derive(Debug, Clone, PartialEq)]
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
        crate::theme::EditorTheme::default().grid_colors()
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
    config: GridConfig,
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
        crate::world_lines::draw_world_line(
            ui,
            viewport,
            segment.start,
            segment.end,
            colors.color_for(segment.kind),
            grid.config.width_for(segment.kind),
        );
    }
    ui.pop_clip_rect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_viewport;
    use ui::DrawCommand;

    const BOUNDS: (f32, f32, f32, f32) = (-100.0, -100.0, 100.0, 100.0);

    /// A sub-pixel grid size would flood the viewport with lines (and
    /// divide snapping by ~0); the setter floors it at one pixel.
    #[test]
    fn test_grid_size_floors_at_one_pixel() {
        let mut grid = GridRenderer::new();

        grid.set_grid_size(64.0);
        assert_eq!(grid.grid_size(), 64.0);
        grid.set_grid_size(0.5);
        assert_eq!(grid.grid_size(), 1.0, "sizes below a pixel clamp to 1.0");
        grid.set_grid_size(f32::NAN);
        assert_eq!(grid.grid_size(), 1.0, "NaN clamps to 1.0 — snapping divides by this");
    }

    /// The constant coordinate of an axis-aligned segment.
    fn line_coordinate(segment: &GridSegment) -> f32 {
        if segment.start.y == segment.end.y { segment.start.y } else { segment.start.x }
    }

    fn coordinates(segments: &[GridSegment], kind: GridLineKind) -> Vec<f32> {
        segments.iter().filter(|s| s.kind == kind).map(line_coordinate).collect()
    }

    #[test]
    fn test_subdivisions_appear_only_zoomed_in_and_never_on_a_primary_line() {
        let grid = GridRenderer::new();

        let zoomed_out = grid.grid_segments(BOUNDS, 0.4);
        assert_eq!(
            coordinates(&zoomed_out, GridLineKind::Secondary),
            Vec::<f32>::new(),
            "subdivisions hide below subdivision_min_zoom"
        );

        let zoomed_in = grid.grid_segments(BOUNDS, 1.0);
        let secondaries = coordinates(&zoomed_in, GridLineKind::Secondary);
        assert!(!secondaries.is_empty());
        for coord in secondaries {
            assert!(is_on_grid(coord, 8.0), "subdivision at {coord} is off the quarter-cell grid");
            assert!(!is_on_grid(coord, 32.0), "subdivision at {coord} coincides with a primary line");
        }
    }

    #[test]
    fn test_lod_doubles_primary_spacing_each_time_zoom_halves() {
        let grid = GridRenderer::new();
        for (zoom, spacing) in [(2.0, 32.0), (1.0, 32.0), (0.5, 64.0), (0.25, 128.0)] {
            let primaries = coordinates(&grid.grid_segments(BOUNDS, zoom), GridLineKind::Primary);
            assert!(
                primaries.iter().any(|coord| (coord - spacing).abs() < 0.001),
                "zoom {zoom}: expected a primary line at {spacing}, got {primaries:?}"
            );
            for coord in &primaries {
                assert!(is_on_grid(*coord, spacing), "zoom {zoom}: primary at {coord} is off the {spacing} grid");
                assert!(coord.abs() > 0.001, "the axes are never duplicated as primaries");
            }
        }
    }

    #[test]
    fn test_too_many_lines_leaves_only_the_two_axes() {
        let mut grid = GridRenderer::new();
        grid.config.max_lines = 10;

        let segments = grid.grid_segments((-10000.0, -10000.0, 10000.0, 10000.0), 4.0);

        let kinds: Vec<GridLineKind> = segments.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, [GridLineKind::AxisX, GridLineKind::AxisY]);
        assert_eq!((segments[0].start, segments[0].end), (Vec2::new(-10000.0, 0.0), Vec2::new(10000.0, 0.0)));
        assert_eq!((segments[1].start, segments[1].end), (Vec2::new(0.0, -10000.0), Vec2::new(0.0, 10000.0)));
    }

    #[test]
    fn test_overlay_draws_axes_and_cells_through_the_viewport_mapping_inside_a_clip() {
        let mut grid = GridRenderer::new();
        let viewport = test_viewport();
        let colors = GridColors::default();
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ui = UIContext::new();

        render_grid_overlay(&mut ui, &grid, &viewport, &colors, bounds);

        let commands = ui.draw_list().commands();
        assert!(commands.iter().any(|c| matches!(c, DrawCommand::PushClipRect { .. })));
        assert!(commands.iter().any(|c| matches!(c, DrawCommand::PopClipRect)));
        let lines: Vec<(Vec2, Vec2, Color)> = commands
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Line { start, end, color, .. } => Some((*start, *end, *color)),
                _ => None,
            })
            .collect();
        // World (-400,0)→(400,0) is screen (0,300)→(800,300); world
        // (0,-300)→(0,300) is screen (400,600)→(400,0) — Y flips.
        assert!(lines.contains(&(Vec2::new(0.0, 300.0), Vec2::new(800.0, 300.0), colors.axis_x)), "X axis");
        assert!(lines.contains(&(Vec2::new(400.0, 600.0), Vec2::new(400.0, 0.0), colors.axis_y)), "Y axis");
        assert!(
            lines.contains(&(Vec2::new(0.0, 268.0), Vec2::new(800.0, 268.0), colors.primary)),
            "the primary line one cell above the X axis sits 32px up on screen"
        );

        grid.set_visible(false);
        let mut hidden = UIContext::new();
        render_grid_overlay(&mut hidden, &grid, &viewport, &colors, bounds);
        assert!(hidden.draw_list().commands().is_empty(), "a hidden grid draws nothing, not even its clip");
    }

    #[test]
    fn test_grid_colors_default_matches_editor_theme_grid_colors() {
        assert_eq!(GridColors::default(), crate::theme::EditorTheme::default().grid_colors());
    }
}
