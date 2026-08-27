//! Input handling for the scene viewport.
//!
//! Handles pan, zoom, and navigation controls for the scene viewport camera.
//! Uses the editor input mapping system for configurable key bindings.

use glam::Vec2;

use crate::editor_input::{EditorAction, EditorInputMapping, EditorInputState};
use crate::viewport::SceneViewport;

/// Configuration for viewport input handling.
#[derive(Debug, Clone)]
pub struct ViewportInputConfig {
    /// Zoom factor per scroll notch
    pub zoom_factor: f32,
    /// Whether to invert scroll direction for zoom
    pub invert_zoom: bool,
    /// Minimum zoom level
    pub min_zoom: f32,
    /// Maximum zoom level
    pub max_zoom: f32,
    /// Pan sensitivity multiplier
    pub pan_sensitivity: f32,
    /// Drag threshold in pixels (movement needed to start drag vs click)
    pub drag_threshold: f32,
}

impl Default for ViewportInputConfig {
    fn default() -> Self {
        Self {
            zoom_factor: 1.1,
            invert_zoom: false,
            min_zoom: 0.1,
            max_zoom: 10.0,
            pan_sensitivity: 1.0,
            drag_threshold: 5.0,
        }
    }
}

/// Input state for viewport interaction.
#[derive(Debug, Clone, Default)]
struct ViewportInputInternalState {
    /// Whether panning is currently active
    panning: bool,
    /// Last mouse position during pan
    last_pan_position: Vec2,
    /// Selection rectangle start position (screen coords)
    selection_start: Option<Vec2>,
    /// Whether selection rectangle is active (dragged past threshold)
    selection_active: bool,
}

/// Result of viewport input handling.
#[derive(Debug, Clone, Default)]
pub struct ViewportInputResult {
    /// Whether the viewport consumed this input (should not pass to other systems)
    pub consumed: bool,
    /// Whether a click occurred (for entity picking)
    pub clicked: bool,
    /// Click position in screen coordinates
    pub click_position: Vec2,
    /// Whether add-to-selection modifier is held (Shift)
    pub shift_held: bool,
    /// Whether toggle-selection modifier is held (Ctrl)
    pub ctrl_held: bool,
    /// Whether a selection rectangle drag is active
    pub selection_drag_active: bool,
    /// Selection rectangle start position (screen coords)
    pub selection_start: Vec2,
    /// Selection rectangle end position (screen coords)
    pub selection_end: Vec2,
    /// Whether focus on selection was requested (F)
    pub focus_requested: bool,
    /// Whether framing every entity was requested (Shift+F)
    pub frame_all_requested: bool,
    /// Whether camera reset was requested (Home)
    pub reset_requested: bool,
}

/// Handles input for viewport navigation (pan, zoom, focus).
#[derive(Debug, Clone)]
pub struct ViewportInputHandler {
    /// Configuration
    pub config: ViewportInputConfig,
    /// Current input state
    state: ViewportInputInternalState,
}

impl Default for ViewportInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewportInputHandler {
    /// Create a new viewport input handler.
    pub fn new() -> Self {
        Self {
            config: ViewportInputConfig::default(),
            state: ViewportInputInternalState::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: ViewportInputConfig) -> Self {
        Self {
            config,
            state: ViewportInputInternalState::default(),
        }
    }

    /// Handle input and update viewport camera.
    ///
    /// Returns information about the input for other systems (picking, etc).
    ///
    /// # Arguments
    /// * `viewport` - The scene viewport to update
    /// * `input_state` - Current input state from EditorInputMapping
    /// * `input_mapping` - Editor input mapping for checking actions
    /// * `input_handler` - Raw input handler for action checks
    /// * `viewport_contains_mouse` - Whether mouse is over the viewport
    pub fn handle_input(
        &mut self,
        viewport: &mut SceneViewport,
        input_state: &EditorInputState,
        input_mapping: &EditorInputMapping,
        input_handler: &input::InputHandler,
        viewport_contains_mouse: bool,
    ) -> ViewportInputResult {
        // Track modifier keys from input state
        let mut result = ViewportInputResult {
            shift_held: input_state.add_modifier,
            ctrl_held: input_state.toggle_modifier,
            ..Default::default()
        };

        // Only handle input if mouse is over viewport
        if !viewport_contains_mouse {
            // End any active interactions
            if self.state.panning {
                self.state.panning = false;
            }
            if self.state.selection_active {
                self.state.selection_active = false;
                self.state.selection_start = None;
            }
            return result;
        }

        let mouse_pos = input_state.mouse_position;

        // Keyboard shortcuts only REQUEST camera moves here — the caller
        // consumes them (it holds the ui-focus and gizmo context this
        // handler cannot see), so they deliberately do not set `consumed`.
        // Shift+F = frame all, F = frame selection.
        if input_mapping.is_action_just_pressed(EditorAction::FocusSelection, input_handler) {
            if input_state.add_modifier {
                result.frame_all_requested = true;
            } else {
                result.focus_requested = true;
            }
        }

        if input_mapping.is_action_just_pressed(EditorAction::ResetCamera, input_handler) {
            result.reset_requested = true;
        }

        // Handle pan input
        // Pan is active when: middle mouse button held, OR pan modifier (Space) + primary button
        let pan_via_middle = input_state.middle_button.pressed;
        let pan_via_space = input_state.pan_modifier && input_state.primary_button.pressed;
        let pan_active = pan_via_middle || pan_via_space;

        if pan_active {
            if !self.state.panning {
                // Start panning
                self.state.panning = true;
                self.state.last_pan_position = mouse_pos;
            } else {
                // Continue panning - convert screen delta to world delta
                let screen_delta = mouse_pos - self.state.last_pan_position;
                let world_delta = Vec2::new(
                    -screen_delta.x / viewport.camera_zoom() * self.config.pan_sensitivity,
                    screen_delta.y / viewport.camera_zoom() * self.config.pan_sensitivity,
                );
                viewport.pan_immediate(world_delta);
                self.state.last_pan_position = mouse_pos;
            }
            result.consumed = true;
        } else {
            self.state.panning = false;
        }

        // Handle zoom input (scroll wheel)
        if input_state.scroll_delta.abs() > 0.001 {
            let factor = if self.config.invert_zoom {
                if input_state.scroll_delta > 0.0 {
                    1.0 / self.config.zoom_factor
                } else {
                    self.config.zoom_factor
                }
            } else if input_state.scroll_delta > 0.0 {
                self.config.zoom_factor
            } else {
                1.0 / self.config.zoom_factor
            };

            viewport.zoom_at(factor, mouse_pos);
            result.consumed = true;
        }

        // Handle selection rectangle (primary button drag without pan modifier)
        let can_select = input_state.primary_button.pressed && !input_state.pan_modifier && !self.state.panning;

        if can_select {
            if self.state.selection_start.is_none() {
                // Start selection drag
                self.state.selection_start = Some(mouse_pos);
                self.state.selection_active = false; // Not yet active until dragged past threshold
            } else if let Some(start) = self.state.selection_start {
                // Check if we've dragged enough to start selection rect
                let drag_dist = (mouse_pos - start).length();
                if drag_dist > self.config.drag_threshold {
                    self.state.selection_active = true;
                }
            }

            if self.state.selection_active {
                result.selection_drag_active = true;
                result.selection_start = self.state.selection_start.unwrap_or(mouse_pos);
                result.selection_end = mouse_pos;
            }
        } else {
            // Primary button released or pan started
            if let Some(start) = self.state.selection_start {
                if self.state.selection_active {
                    // Complete selection rectangle
                    result.selection_drag_active = false;
                    result.selection_start = start;
                    result.selection_end = mouse_pos;
                    // The caller will handle the actual selection
                } else {
                    // Was a click, not a drag
                    result.clicked = true;
                    result.click_position = start;
                }
            }
            self.state.selection_start = None;
            self.state.selection_active = false;
        }

        result
    }

    /// Simplified input handling that creates the input state internally.
    pub fn handle_input_simple(
        &mut self,
        viewport: &mut SceneViewport,
        input_mapping: &EditorInputMapping,
        input_handler: &input::InputHandler,
    ) -> ViewportInputResult {
        let input_state = input_mapping.update_state(input_handler);
        let viewport_contains_mouse = viewport.contains_screen_point(input_state.mouse_position);

        self.handle_input(
            viewport,
            &input_state,
            input_mapping,
            input_handler,
            viewport_contains_mouse,
        )
    }

    /// Check if panning is currently active.
    pub fn is_panning(&self) -> bool {
        self.state.panning
    }

    /// Check if selection rectangle is active.
    pub fn is_selecting(&self) -> bool {
        self.state.selection_active
    }

    /// Reset all input state.
    pub fn reset(&mut self) {
        self.state = ViewportInputInternalState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calculate zoom factor for a scroll delta (mirrors the logic in `handle_input`).
    fn calculate_zoom_factor(scroll_delta: f32, base_factor: f32, invert: bool) -> f32 {
        let factor = if scroll_delta > 0.0 {
            base_factor
        } else {
            1.0 / base_factor
        };

        if invert {
            1.0 / factor
        } else {
            factor
        }
    }

    /// Convert screen delta to world delta for panning (mirrors the logic in `handle_input`).
    fn screen_to_world_delta(screen_delta: Vec2, camera_zoom: f32) -> Vec2 {
        Vec2::new(
            -screen_delta.x / camera_zoom,
            screen_delta.y / camera_zoom, // Flip Y
        )
    }

    #[test]
    fn test_viewport_input_handler_new() {
        let handler = ViewportInputHandler::new();
        assert!(!handler.is_panning());
        assert!(!handler.is_selecting());
    }

    #[test]
    fn test_zoom_factor_calculation() {
        let factor = calculate_zoom_factor(1.0, 1.1, false);
        assert!((factor - 1.1).abs() < 0.001);

        let factor = calculate_zoom_factor(-1.0, 1.1, false);
        assert!((factor - 1.0 / 1.1).abs() < 0.001);
    }

    #[test]
    fn test_zoom_factor_inverted() {
        let factor = calculate_zoom_factor(1.0, 1.1, true);
        assert!((factor - 1.0 / 1.1).abs() < 0.001);
    }

    #[test]
    fn test_screen_to_world_delta() {
        let screen_delta = Vec2::new(100.0, 50.0);
        let world_delta = screen_to_world_delta(screen_delta, 1.0);

        // X should be negated, Y should be flipped
        assert_eq!(world_delta.x, -100.0);
        assert_eq!(world_delta.y, 50.0);
    }

    #[test]
    fn test_screen_to_world_delta_with_zoom() {
        let screen_delta = Vec2::new(100.0, 50.0);
        let world_delta = screen_to_world_delta(screen_delta, 2.0);

        // At 2x zoom, world deltas are halved
        assert_eq!(world_delta.x, -50.0);
        assert_eq!(world_delta.y, 25.0);
    }

    #[test]
    fn test_viewport_input_config_default() {
        let config = ViewportInputConfig::default();
        assert!((config.zoom_factor - 1.1).abs() < 0.001);
        assert!(!config.invert_zoom);
        assert_eq!(config.min_zoom, 0.1);
        assert_eq!(config.max_zoom, 10.0);
    }

    // ---- Camera shortcut requests (issue #21) ----

    /// A viewport with bounds, mouse hovering its center, and a raw input
    /// handler with the given key just pressed.
    fn shortcut_rig(
        key: winit::keyboard::KeyCode,
    ) -> (SceneViewport, EditorInputState, EditorInputMapping, input::InputHandler) {
        let mut viewport = SceneViewport::new();
        viewport.set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));
        let state = EditorInputState {
            mouse_position: Vec2::new(400.0, 300.0),
            ..Default::default()
        };
        let mut input = input::InputHandler::new();
        input.keyboard_mut().handle_key_press(key);
        (viewport, state, EditorInputMapping::new(), input)
    }

    #[test]
    fn test_f_requests_focus_on_selection() {
        let (mut viewport, state, mapping, input) =
            shortcut_rig(winit::keyboard::KeyCode::KeyF);
        let mut handler = ViewportInputHandler::new();

        let result = handler.handle_input(&mut viewport, &state, &mapping, &input, true);

        assert!(result.focus_requested);
        assert!(!result.frame_all_requested);
        // Requests don't claim the input — the caller decides whether to act.
        assert!(!result.consumed);
    }

    #[test]
    fn test_shift_f_requests_frame_all_not_focus() {
        let (mut viewport, mut state, mapping, input) =
            shortcut_rig(winit::keyboard::KeyCode::KeyF);
        state.add_modifier = true; // Shift held
        let mut handler = ViewportInputHandler::new();

        let result = handler.handle_input(&mut viewport, &state, &mapping, &input, true);

        assert!(result.frame_all_requested);
        assert!(!result.focus_requested);
    }

    #[test]
    fn test_home_requests_reset_and_leaves_the_camera_to_the_caller() {
        let (mut viewport, state, mapping, input) =
            shortcut_rig(winit::keyboard::KeyCode::Home);
        viewport.set_target_camera_position(Vec2::new(50.0, 60.0));
        let mut handler = ViewportInputHandler::new();

        let result = handler.handle_input(&mut viewport, &state, &mapping, &input, true);

        // The handler only reports the request — the caller consumes it
        // (it knows whether a text field owns the keyboard).
        assert!(result.reset_requested);
        assert_eq!(viewport.target_camera_position(), Vec2::new(50.0, 60.0));
    }

    #[test]
    fn test_shortcuts_ignored_while_mouse_outside_viewport() {
        let (mut viewport, state, mapping, input) =
            shortcut_rig(winit::keyboard::KeyCode::KeyF);
        let mut handler = ViewportInputHandler::new();

        let result = handler.handle_input(&mut viewport, &state, &mapping, &input, false);

        assert!(!result.focus_requested);
        assert!(!result.frame_all_requested);
    }
}
