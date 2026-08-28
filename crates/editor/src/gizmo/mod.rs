//! Transform gizmos for the editor.
//!
//! Gizmos are visual handles that allow manipulating entity transforms
//! (position, rotation, scale) directly in the scene view. Interaction is
//! reported cumulatively where possible — translation and scale are measured
//! from the drag start, so the caller applies `start + delta` idempotently
//! instead of accumulating per-frame deltas (which is what made snapping eat
//! drag residuals).

use glam::Vec2;
use ui::{Color, Rect, UIContext};

use crate::theme::EditorTheme;

#[cfg(test)]
mod tests;

/// Half-width of the rotate ring's interactive band, in screen pixels.
/// Clicks outside the band (including the ring's dead center) claim no
/// widget, so they fall through to picking.
const RING_BAND: f32 = 12.0;

/// The type of gizmo operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    /// No gizmo visible
    None,
    /// Move/translate gizmo with XY axes
    #[default]
    Translate,
    /// Rotation gizmo with circular handle
    Rotate,
    /// Scale gizmo with corner handles
    Scale,
}

impl GizmoMode {
    /// Get the display name for this mode.
    pub fn name(&self) -> &'static str {
        match self {
            GizmoMode::None => "None",
            GizmoMode::Translate => "Translate",
            GizmoMode::Rotate => "Rotate",
            GizmoMode::Scale => "Scale",
        }
    }

    /// Whether `handle` is one this mode's renderer manages (and can
    /// therefore release at the end of a drag).
    fn owns_handle(&self, handle: GizmoHandle) -> bool {
        match self {
            GizmoMode::None => false,
            GizmoMode::Translate => matches!(
                handle,
                GizmoHandle::AxisX | GizmoHandle::AxisY | GizmoHandle::Center
            ),
            GizmoMode::Rotate => matches!(handle, GizmoHandle::Ring),
            GizmoMode::Scale => matches!(handle, GizmoHandle::ScaleCorner(_)),
        }
    }
}

/// Which part of the gizmo is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoHandle {
    /// X-axis handle (red)
    AxisX,
    /// Y-axis handle (green)
    AxisY,
    /// Both axes (center/free movement)
    Center,
    /// Rotation ring
    Ring,
    /// Scale corner handle
    ScaleCorner(Corner),
}

/// Corner positions for scale handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Result of gizmo interaction.
#[derive(Debug, Clone, Copy)]
pub struct GizmoInteraction {
    /// Which handle is being dragged (None on the release frame)
    pub handle: Option<GizmoHandle>,
    /// Cumulative screen-space translation since drag start (axis-projected)
    pub translation: Vec2,
    /// Rotation delta in radians since last frame (the caller accumulates —
    /// a cumulative angle would wrap at ±π)
    pub rotation_delta: f32,
    /// Cumulative scale factor since drag start (`Vec2::ONE` = unchanged)
    pub scale_factor: Vec2,
    /// True on the frame an active drag released — the caller commits then
    pub released: bool,
}

impl Default for GizmoInteraction {
    fn default() -> Self {
        Self {
            handle: None,
            translation: Vec2::ZERO,
            rotation_delta: 0.0,
            scale_factor: Vec2::ONE,
            released: false,
        }
    }
}

/// Complete color set for gizmo rendering.
///
/// Defaults match the editor's dark theme; `EditorTheme::gizmo_palette()`
/// produces a themed instance.
#[derive(Debug, Clone)]
pub struct GizmoPalette {
    /// X axis line and handle
    pub x: Color,
    /// Y axis line and handle
    pub y: Color,
    /// Center/free-move handle
    pub center: Color,
    /// X handle while hovered/dragged
    pub x_hover: Color,
    /// Y handle while hovered/dragged
    pub y_hover: Color,
    /// Center handle while hovered/dragged
    pub center_hover: Color,
    /// Rotation ring
    pub ring: Color,
    /// Current-rotation indicator line
    pub rotation_indicator: Color,
    /// Scale box outline
    pub scale_outline: Color,
    /// Scale corner handles
    pub scale_handle: Color,
    /// Scale corner handles while hovered/dragged
    pub scale_handle_hover: Color,
}

impl Default for GizmoPalette {
    fn default() -> Self {
        Self {
            x: Color::new(0.9, 0.2, 0.2, 1.0),      // Red
            y: Color::new(0.2, 0.9, 0.2, 1.0),      // Green
            center: Color::new(0.9, 0.9, 0.2, 1.0), // Yellow
            x_hover: Color::new(1.0, 0.4, 0.4, 1.0),
            y_hover: Color::new(0.4, 1.0, 0.4, 1.0),
            center_hover: Color::new(1.0, 1.0, 0.4, 1.0),
            ring: Color::new(0.3, 0.3, 0.9, 1.0),
            rotation_indicator: Color::new(0.9, 0.9, 0.9, 1.0),
            scale_outline: Color::new(0.6, 0.6, 0.6, 1.0),
            scale_handle: Color::new(0.7, 0.7, 0.7, 1.0),
            scale_handle_hover: Color::new(0.9, 0.9, 0.4, 1.0),
        }
    }
}

/// Transform gizmo for manipulating entity transforms.
#[derive(Debug, Clone)]
pub struct Gizmo {
    /// Current gizmo mode
    mode: GizmoMode,
    /// Position of the gizmo center (world space)
    position: Vec2,
    /// Current rotation of the entity (for rotation gizmo display)
    rotation: f32,
    /// Current scale of the entity (for scale gizmo display)
    scale: Vec2,
    /// Size of the gizmo handles
    handle_size: f32,
    /// Length of the axis lines
    axis_length: f32,
    /// Active handle being dragged
    active_handle: Option<GizmoHandle>,
    /// Last mouse position for per-frame deltas (rotation)
    last_mouse_pos: Vec2,
    /// Mouse position when the active drag began (translation/scale reference)
    drag_start_mouse: Vec2,
    /// Set by `cancel()`: ignore the rest of the current mouse gesture.
    /// Cleared by POLLED mouse-up state, never by a release event — a
    /// release delivered while the window is unfocused must not wedge us.
    suppressed_until_release: bool,
    /// Colors for every gizmo element
    palette: GizmoPalette,
}

impl Default for Gizmo {
    fn default() -> Self {
        Self::new()
    }
}

impl Gizmo {
    /// Create a new gizmo.
    pub fn new() -> Self {
        Self {
            mode: GizmoMode::Translate,
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            handle_size: 12.0,
            axis_length: 80.0,
            active_handle: None,
            last_mouse_pos: Vec2::ZERO,
            drag_start_mouse: Vec2::ZERO,
            suppressed_until_release: false,
            palette: GizmoPalette::default(),
        }
    }

    /// Set the gizmo mode.
    pub fn set_mode(&mut self, mode: GizmoMode) {
        self.mode = mode;
    }

    /// Get the current gizmo mode.
    pub fn mode(&self) -> GizmoMode {
        self.mode
    }

    /// Set the gizmo position (world space).
    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    /// Set the length of the gizmo axis arms in screen pixels (minimum 10).
    pub fn set_axis_length(&mut self, length: f32) {
        self.axis_length = length.max(10.0);
    }

    /// Get the length of the gizmo axis arms in screen pixels.
    pub fn axis_length(&self) -> f32 {
        self.axis_length
    }

    /// Get the gizmo position.
    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// Apply colors from the editor theme.
    pub fn apply_theme(&mut self, theme: &EditorTheme) {
        self.palette = theme.gizmo_palette();
    }

    /// Set the entity rotation (for rotation gizmo display).
    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    /// Set the entity scale (for scale gizmo display).
    pub fn set_scale(&mut self, scale: Vec2) {
        self.scale = scale;
    }

    /// Check if the gizmo is currently being dragged.
    pub fn is_active(&self) -> bool {
        self.active_handle.is_some()
    }

    /// Get the active handle being dragged.
    pub fn active_handle(&self) -> Option<GizmoHandle> {
        self.active_handle
    }

    /// Cancel any active gizmo operation and ignore the rest of the current
    /// mouse gesture (Escape mid-drag). The caller restores the dragged
    /// transforms; the gizmo just stops reporting the gesture.
    pub fn cancel(&mut self) {
        self.active_handle = None;
        self.suppressed_until_release = true;
    }

    /// Create a square rect centered at the given position, sized to handle_size.
    fn centered_handle_rect(&self, center: Vec2) -> Rect {
        Rect::new(
            center.x - self.handle_size / 2.0,
            center.y - self.handle_size / 2.0,
            self.handle_size,
            self.handle_size,
        )
    }

    /// Draw one translate axis (line + arrow handle) and return the handle's
    /// interactive bounds. The handle brightens while hovered or dragged.
    fn render_axis_handle(
        &self,
        ui: &mut UIContext,
        origin: Vec2,
        end: Vec2,
        base: Color,
        hover: Color,
        handle: GizmoHandle,
    ) -> Rect {
        ui.line(origin, end, base, 2.0);

        let bounds = self.centered_handle_rect(end);
        let hovered = bounds.contains(ui.mouse_pos());
        let color = if hovered || self.active_handle == Some(handle) {
            hover
        } else {
            base
        };
        ui.rect(bounds, color);
        bounds
    }

    /// Start drag bookkeeping for `handle` if `dragging` and no drag is active.
    fn begin_drag_if(&mut self, dragging: bool, handle: GizmoHandle, mouse_pos: Vec2) {
        if dragging && self.active_handle.is_none() {
            self.active_handle = Some(handle);
            self.last_mouse_pos = mouse_pos;
            self.drag_start_mouse = mouse_pos;
        }
    }

    /// End the active drag and mark the interaction released.
    fn end_drag(&mut self, interaction: &mut GizmoInteraction) {
        self.active_handle = None;
        interaction.handle = None;
        interaction.released = true;
    }

    /// Render the gizmo and handle interactions.
    ///
    /// # Arguments
    /// * `ui` - UI context for rendering
    /// * `screen_pos` - Screen position of the gizmo center
    /// * `interactive` - Whether NEW drags may start (mouse inside the scene
    ///   panel). A drag already in flight stays live even when the cursor
    ///   leaves the panel.
    ///
    /// Returns the gizmo interaction result.
    pub fn render(
        &mut self,
        ui: &mut UIContext,
        screen_pos: Vec2,
        interactive: bool,
    ) -> GizmoInteraction {
        // Cancel latch: cleared by polled mouse state so a release missed
        // while unfocused can't wedge the gizmo.
        if self.suppressed_until_release && !ui.mouse_down() {
            self.suppressed_until_release = false;
        }
        // A tool switch mid-drag (W→E while holding the mouse) leaves a
        // handle from another mode; release it so the caller commits the
        // drag instead of the gizmo wedging active forever.
        if let Some(handle) = self.active_handle {
            if !self.mode.owns_handle(handle) {
                self.active_handle = None;
            }
        }
        let hit_enabled = !self.suppressed_until_release && (interactive || self.is_active());

        match self.mode {
            GizmoMode::None => GizmoInteraction::default(),
            GizmoMode::Translate => self.render_translate(ui, screen_pos, hit_enabled),
            GizmoMode::Rotate => self.render_rotate(ui, screen_pos, hit_enabled),
            GizmoMode::Scale => self.render_scale(ui, screen_pos, hit_enabled),
        }
    }

    /// Render and handle translation gizmo.
    fn render_translate(
        &mut self,
        ui: &mut UIContext,
        screen_pos: Vec2,
        hit_enabled: bool,
    ) -> GizmoInteraction {
        let mut interaction = GizmoInteraction::default();
        let mouse_pos = ui.mouse_pos();

        // X axis (right), Y axis (up in screen space = negative Y)
        let x_end = screen_pos + Vec2::new(self.axis_length, 0.0);
        let y_end = screen_pos + Vec2::new(0.0, -self.axis_length);
        let x_arrow_bounds = self.render_axis_handle(
            ui, screen_pos, x_end, self.palette.x, self.palette.x_hover, GizmoHandle::AxisX,
        );
        let y_arrow_bounds = self.render_axis_handle(
            ui, screen_pos, y_end, self.palette.y, self.palette.y_hover, GizmoHandle::AxisY,
        );

        // Center handle (free movement, no axis line)
        let center_bounds = self.centered_handle_rect(screen_pos);
        let center_hovered = center_bounds.contains(mouse_pos);
        let center_color = if center_hovered || self.active_handle == Some(GizmoHandle::Center) {
            self.palette.center_hover
        } else {
            self.palette.center
        };
        ui.rect(center_bounds, center_color);

        // Handle interaction
        let result_x = ui.interact("gizmo_x", x_arrow_bounds, hit_enabled);
        let result_y = ui.interact("gizmo_y", y_arrow_bounds, hit_enabled);
        let result_center = ui.interact("gizmo_center", center_bounds, hit_enabled);

        // Start dragging (first dragging handle wins; later calls no-op)
        self.begin_drag_if(result_x.dragging, GizmoHandle::AxisX, mouse_pos);
        self.begin_drag_if(result_y.dragging, GizmoHandle::AxisY, mouse_pos);
        self.begin_drag_if(result_center.dragging, GizmoHandle::Center, mouse_pos);

        // Continue dragging — translation is cumulative from the drag start,
        // projected onto the grabbed axis
        if let Some(handle) = self.active_handle {
            let total = mouse_pos - self.drag_start_mouse;
            self.last_mouse_pos = mouse_pos;

            interaction.handle = Some(handle);
            interaction.translation = match handle {
                GizmoHandle::AxisX => Vec2::new(total.x, 0.0),
                GizmoHandle::AxisY => Vec2::new(0.0, total.y),
                GizmoHandle::Center => total,
                _ => Vec2::ZERO,
            };

            // Stop dragging when mouse released
            if !result_x.dragging && !result_y.dragging && !result_center.dragging {
                self.end_drag(&mut interaction);
            }
        }

        interaction
    }

    /// Render and handle rotation gizmo.
    fn render_rotate(
        &mut self,
        ui: &mut UIContext,
        screen_pos: Vec2,
        hit_enabled: bool,
    ) -> GizmoInteraction {
        let mut interaction = GizmoInteraction::default();
        let mouse_pos = ui.mouse_pos();

        // Draw rotation ring (approximated with line segments)
        let ring_radius = self.axis_length * 0.8;
        let segments = 32;
        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

            let p1 = screen_pos + Vec2::new(angle1.cos(), angle1.sin()) * ring_radius;
            let p2 = screen_pos + Vec2::new(angle2.cos(), angle2.sin()) * ring_radius;

            ui.line(p1, p2, self.palette.ring, 2.0);
        }

        // Draw current rotation indicator (negated sin: world rotation is
        // CCW-positive but screen Y grows downward)
        let indicator_end = screen_pos + Vec2::new(
            self.rotation.cos() * ring_radius,
            -self.rotation.sin() * ring_radius,
        );
        ui.line(screen_pos, indicator_end, self.palette.rotation_indicator, 3.0);

        // The ring hit-tests as an annulus band, not the filled square: the
        // widget is only registered while the mouse is on the band (or a
        // ring drag is live), so a click in the dead center claims nothing
        // and falls through to entity picking.
        let dist = (mouse_pos - screen_pos).length();
        let in_band = (dist - ring_radius).abs() <= RING_BAND;
        let ring_active = self.active_handle == Some(GizmoHandle::Ring);
        if in_band || ring_active {
            let ring_bounds = Rect::new(
                screen_pos.x - ring_radius - RING_BAND,
                screen_pos.y - ring_radius - RING_BAND,
                (ring_radius + RING_BAND) * 2.0,
                (ring_radius + RING_BAND) * 2.0,
            );
            let result = ui.interact("gizmo_ring", ring_bounds, hit_enabled);

            if result.dragging {
                self.begin_drag_if(true, GizmoHandle::Ring, mouse_pos);

                interaction.handle = Some(GizmoHandle::Ring);
                interaction.rotation_delta = crate::gizmo_math::world_rotation_delta(
                    screen_pos,
                    self.last_mouse_pos,
                    mouse_pos,
                );
                self.last_mouse_pos = mouse_pos;
            } else if ring_active {
                self.end_drag(&mut interaction);
            }
        }

        interaction
    }

    /// Render and handle scale gizmo.
    fn render_scale(
        &mut self,
        ui: &mut UIContext,
        screen_pos: Vec2,
        hit_enabled: bool,
    ) -> GizmoInteraction {
        let mut interaction = GizmoInteraction::default();
        let mouse_pos = ui.mouse_pos();

        // Draw scale box outline
        let box_size = self.axis_length * 0.6;
        let half_size = box_size / 2.0;
        let box_bounds = Rect::new(
            screen_pos.x - half_size,
            screen_pos.y - half_size,
            box_size,
            box_size,
        );

        let box_corners = [
            Vec2::new(box_bounds.x, box_bounds.y),
            Vec2::new(box_bounds.x + box_bounds.width, box_bounds.y),
            Vec2::new(box_bounds.x + box_bounds.width, box_bounds.y + box_bounds.height),
            Vec2::new(box_bounds.x, box_bounds.y + box_bounds.height),
        ];
        for i in 0..4 {
            ui.line(box_corners[i], box_corners[(i + 1) % 4], self.palette.scale_outline, 1.0);
        }

        // Draw corner handles and interact ONCE per corner per frame —
        // a second interact with the same id desyncs the gesture (the old
        // still_dragging check did exactly that, with the wrong rect).
        let corners = [
            (Corner::TopLeft, box_corners[0]),
            (Corner::TopRight, box_corners[1]),
            (Corner::BottomRight, box_corners[2]),
            (Corner::BottomLeft, box_corners[3]),
        ];

        let mut any_dragging = false;
        for (corner, pos) in corners {
            let handle_bounds = self.centered_handle_rect(pos);

            let hovered = handle_bounds.contains(mouse_pos);
            let active = self.active_handle == Some(GizmoHandle::ScaleCorner(corner));
            let color = if hovered || active {
                self.palette.scale_handle_hover
            } else {
                self.palette.scale_handle
            };
            ui.rect(handle_bounds, color);

            let id = format!("gizmo_scale_{:?}", corner);
            let result = ui.interact(id.as_str(), handle_bounds, hit_enabled);
            any_dragging |= result.dragging;

            self.begin_drag_if(result.dragging, GizmoHandle::ScaleCorner(corner), mouse_pos);
        }

        // Process active scale drag — the factor is the per-axis ratio of
        // the mouse's current offset from the gizmo center to its offset at
        // drag start: multiplicative, zoom-independent (both offsets live in
        // the same screen space), and sign-free via abs(). The .max(1.0) on
        // the reference is a degenerate-zero epsilon only — drags can only
        // start on a corner handle, so the real reference is the corner's
        // distance from center.
        if let Some(GizmoHandle::ScaleCorner(corner)) = self.active_handle {
            let start_offset = (self.drag_start_mouse - screen_pos).abs().max(Vec2::splat(1.0));
            let current_offset = (mouse_pos - screen_pos).abs();
            interaction.scale_factor =
                (current_offset / start_offset).max(Vec2::splat(0.01));
            interaction.handle = Some(GizmoHandle::ScaleCorner(corner));
            self.last_mouse_pos = mouse_pos;

            if !any_dragging {
                self.end_drag(&mut interaction);
            }
        }

        interaction
    }
}
