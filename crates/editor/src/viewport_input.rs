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
    /// Set by `cancel_marquee` (Escape): ignore the rest of the current
    /// mouse gesture so the cancelled rect can't re-arm at the cursor.
    suppressed_until_release: bool,
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
    /// A marquee drag live THIS frame: `(start, current)` screen coords.
    /// The caller renders the rubber-band rect from this.
    pub marquee_active: Option<(Vec2, Vec2)>,
    /// A marquee drag that completed THIS frame: `(start, end)` screen
    /// coords. The caller applies the rectangle selection. Options, not a
    /// `Vec2::ZERO` sentinel — a drag starting exactly at (0,0) is real.
    pub marquee_released: Option<(Vec2, Vec2)>,
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
            if self.state.suppressed_until_release && !input_state.primary_button.pressed {
                self.state.suppressed_until_release = false;
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
        if self.state.suppressed_until_release {
            // A cancelled gesture stays dead until the button comes up —
            // cleared from POLLED state so a release missed while unfocused
            // can't wedge the marquee.
            if !input_state.primary_button.pressed {
                self.state.suppressed_until_release = false;
            }
            return result;
        }

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
                if let Some(start) = self.state.selection_start {
                    result.marquee_active = Some((start, mouse_pos));
                }
            }
        } else {
            // Primary button released or pan started
            if let Some(start) = self.state.selection_start {
                if self.state.selection_active {
                    // Complete selection rectangle — the caller applies it
                    result.marquee_released = Some((start, mouse_pos));
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

    /// Cancel a marquee in progress (Escape): the rect disappears, nothing
    /// is selected on release, and the rest of the mouse gesture is ignored
    /// so a fresh rect can't re-arm at the cursor.
    pub fn cancel_marquee(&mut self) {
        if self.state.selection_start.is_some() {
            self.state.suppressed_until_release = true;
        }
        self.state.selection_start = None;
        self.state.selection_active = false;
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

    /// Whether ANY marquee gesture is in flight — an active rect OR a
    /// pressed-but-under-threshold press. Escape uses this: cancelling a
    /// pending press must suppress the click it would otherwise become.
    pub fn has_pending_marquee(&self) -> bool {
        self.state.selection_start.is_some()
    }
}

#[cfg(test)]
mod tests {
    //! Driven through `handle_input` / `handle_input_simple` so the asserts
    //! pin what the viewport actually does with a pan, a wheel notch, a
    //! marquee gesture and a camera shortcut — never a test-local copy of
    //! the math.

    use super::*;
    use crate::editor_input::ButtonState;
    use crate::test_support::{move_mouse, next_frame, press_button, release_button, test_viewport};
    use input::prelude::{InputHandler, KeyCode, MouseButton};

    const PANEL_CENTER: Vec2 = Vec2::new(400.0, 300.0);

    fn rig() -> (SceneViewport, EditorInputMapping, InputHandler, ViewportInputHandler) {
        (test_viewport(), EditorInputMapping::new(), InputHandler::new(), ViewportInputHandler::new())
    }

    /// Input state with the primary button held (or not) at `pos`.
    fn mouse_state(pos: Vec2, pressed: bool) -> EditorInputState {
        EditorInputState {
            mouse_position: pos,
            primary_button: ButtonState { pressed, ..Default::default() },
            ..Default::default()
        }
    }

    // ---- Pan and zoom through the real input path (E1) ----

    #[test]
    fn test_middle_button_drag_pans_the_camera_by_screen_delta_over_zoom() {
        let (mut viewport, mapping, mut input, mut handler) = rig();
        viewport.set_camera_zoom(2.0);

        press_button(&mut input, MouseButton::Middle, PANEL_CENTER);
        let pressed = handler.handle_input_simple(&mut viewport, &mapping, &input);
        assert!(pressed.consumed, "a pan claims the input");
        assert!(handler.is_panning());
        assert_eq!(viewport.camera_position(), Vec2::ZERO, "the press frame only anchors the drag");

        move_mouse(&mut input, PANEL_CENTER + Vec2::new(100.0, 40.0));
        handler.handle_input_simple(&mut viewport, &mapping, &input);
        // Screen right/down at zoom 2 drags the world with the cursor, so the
        // camera moves left/up by half the pixels: (-dx/zoom, +dy/zoom).
        assert_eq!(viewport.camera_position(), Vec2::new(-50.0, 20.0));

        release_button(&mut input, MouseButton::Middle);
        let released = handler.handle_input_simple(&mut viewport, &mapping, &input);
        assert!(!handler.is_panning());
        assert!(!released.clicked, "a pan is never a pick");
        assert_eq!(viewport.camera_position(), Vec2::new(-50.0, 20.0), "release moves nothing");
    }

    #[test]
    fn test_wheel_notch_multiplies_zoom_by_the_factor_around_the_cursor_and_clamps() {
        let (mut viewport, mapping, mut input, mut handler) = rig();
        viewport.set_interpolation_speed(1.0);
        let cursor = Vec2::new(600.0, 200.0);
        let under_cursor = viewport.screen_to_world(cursor);
        input.mouse_mut().update_position(cursor.x, cursor.y);

        input.mouse_mut().update_wheel_delta(1.0);
        let zoomed = handler.handle_input_simple(&mut viewport, &mapping, &input);
        assert!(zoomed.consumed, "a wheel notch claims the input");
        assert!((viewport.target_camera_zoom() - 1.1).abs() < 1e-6, "scroll up zooms in by zoom_factor");
        viewport.update(1.0 / 60.0);
        assert!(
            (viewport.screen_to_world(cursor) - under_cursor).length() < 1e-3,
            "the world point under the cursor stays put"
        );

        next_frame(&mut input);
        input.mouse_mut().update_wheel_delta(-1.0);
        handler.handle_input_simple(&mut viewport, &mapping, &input);
        assert!((viewport.target_camera_zoom() - 1.0).abs() < 1e-6, "scroll down divides by the same factor");

        for (start, notch, clamped) in [(10.0, 1.0, 10.0), (0.1, -1.0, 0.1)] {
            viewport.set_camera_zoom(start);
            next_frame(&mut input);
            input.mouse_mut().update_wheel_delta(notch);
            handler.handle_input_simple(&mut viewport, &mapping, &input);
            assert_eq!(viewport.target_camera_zoom(), clamped, "zoom clamps at the {clamped} end");
        }

        handler.config.invert_zoom = true;
        viewport.set_camera_zoom(1.0);
        next_frame(&mut input);
        input.mouse_mut().update_wheel_delta(1.0);
        handler.handle_input_simple(&mut viewport, &mapping, &input);
        assert!((viewport.target_camera_zoom() - 1.0 / 1.1).abs() < 1e-6, "invert_zoom flips the direction");
    }

    // ---- Marquee state machine ----

    #[test]
    fn test_marquee_from_screen_origin_is_real_and_a_sub_threshold_press_is_a_click() {
        let (mut viewport, mapping, input, mut handler) = rig();

        // Press exactly at (0,0) — the old Vec2::ZERO sentinel silently
        // dropped this drag on release.
        handler.handle_input(&mut viewport, &mouse_state(Vec2::ZERO, true), &mapping, &input, true);
        let live = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(100.0, 80.0), true), &mapping, &input, true);
        assert_eq!(live.marquee_active, Some((Vec2::ZERO, Vec2::new(100.0, 80.0))));
        let released = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(100.0, 80.0), false), &mapping, &input, true);
        assert_eq!(released.marquee_released, Some((Vec2::ZERO, Vec2::new(100.0, 80.0))));
        assert!(!released.clicked, "a drag is not a click");

        // 2px of jitter stays under the 5px drag threshold: a click at the
        // PRESS position, no marquee.
        handler.handle_input(&mut viewport, &mouse_state(Vec2::new(50.0, 50.0), true), &mapping, &input, true);
        let jitter = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(52.0, 51.0), true), &mapping, &input, true);
        assert_eq!(jitter.marquee_active, None, "jitter must not start a marquee");
        let clicked = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(52.0, 51.0), false), &mapping, &input, true);
        assert!(clicked.clicked, "a sub-threshold gesture stays a click");
        assert_eq!(clicked.click_position, Vec2::new(50.0, 50.0));
        assert_eq!(clicked.marquee_released, None);
    }

    #[test]
    fn test_cancel_marquee_kills_the_gesture_until_release() {
        let (mut viewport, mapping, input, mut handler) = rig();

        handler.handle_input(&mut viewport, &mouse_state(Vec2::ZERO, true), &mapping, &input, true);
        handler.handle_input(&mut viewport, &mouse_state(Vec2::new(100.0, 100.0), true), &mapping, &input, true);

        // Escape
        handler.cancel_marquee();

        // Still holding: the cancelled gesture must not re-arm a fresh rect
        let held = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(150.0, 150.0), true), &mapping, &input, true);
        assert_eq!(held.marquee_active, None);

        // Release: nothing selected, nothing clicked
        let released = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(150.0, 150.0), false), &mapping, &input, true);
        assert_eq!(released.marquee_released, None);
        assert!(!released.clicked);

        // A fresh press afterwards drags normally again
        handler.handle_input(&mut viewport, &mouse_state(Vec2::new(10.0, 10.0), true), &mapping, &input, true);
        let fresh = handler.handle_input(&mut viewport, &mouse_state(Vec2::new(60.0, 60.0), true), &mapping, &input, true);
        assert_eq!(fresh.marquee_active, Some((Vec2::new(10.0, 10.0), Vec2::new(60.0, 60.0))), "latch must clear on release");
    }

    // ---- Camera shortcut requests ----

    #[test]
    fn test_camera_shortcuts_are_requests_the_caller_consumes() {
        // (key, shift held, mouse over the viewport) → (focus, frame all, reset)
        let table = [
            (KeyCode::KeyF, false, true, (true, false, false)),
            (KeyCode::KeyF, true, true, (false, true, false)),
            (KeyCode::Home, false, true, (false, false, true)),
            (KeyCode::KeyF, false, false, (false, false, false)),
        ];
        for (key, shift, over_viewport, expected) in table {
            let (mut viewport, mapping, mut input, mut handler) = rig();
            viewport.set_target_camera_position(Vec2::new(50.0, 60.0));
            input.keyboard_mut().handle_key_press(key);
            let state = EditorInputState { mouse_position: PANEL_CENTER, add_modifier: shift, ..Default::default() };

            let result = handler.handle_input(&mut viewport, &state, &mapping, &input, over_viewport);

            assert_eq!(
                (result.focus_requested, result.frame_all_requested, result.reset_requested),
                expected,
                "{key:?} shift={shift} over_viewport={over_viewport}"
            );
            // The handler cannot see ui focus or the gizmo: it reports and
            // leaves both the input claim and the camera to the caller.
            assert!(!result.consumed, "requests never claim the input");
            assert_eq!(viewport.target_camera_position(), Vec2::new(50.0, 60.0));
        }
    }
}
