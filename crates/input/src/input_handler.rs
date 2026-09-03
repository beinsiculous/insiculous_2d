//! Unified input handling.
//!
//! [`InputHandler`] owns raw device state: keyboard, mouse, and gamepads.
//! Semantic action mapping lives in [`crate::InputMapping`], which games own
//! and evaluate against this handler's state.
//!
//! # Frame Lifecycle
//!
//! The input system follows a specific frame lifecycle:
//!
//! 1. **Event Collection** (automatic): Window events are queued via `handle_window_event()`
//! 2. **Event Processing** (start of frame): Call `process_queued_events()` to update input state
//! 3. **Game Logic**: Read input state via `is_key_pressed()`, `InputMapping::is_active()`, etc.
//! 4. **State Reset** (end of frame): Call `end_frame()` to clear per-frame state
//!    (just_pressed/just_released flags, mouse movement and wheel deltas)
//!
//! ```
//! use input::{InputHandler, InputEvent};
//! use winit::keyboard::KeyCode;
//!
//! let mut input = InputHandler::new();
//! # input.queue_event(InputEvent::KeyPressed(KeyCode::Space));
//!
//! // Typical frame loop:
//! input.process_queued_events();  // Process this frame's input
//! // ... game logic reads input state ...
//! assert!(input.is_key_just_pressed(KeyCode::Space));
//! input.end_frame();              // Clear per-frame state for next frame
//! assert!(!input.is_key_just_pressed(KeyCode::Space));
//! ```
//!
//! For simple use cases, `update()` combines steps 2 and 4 into one call.

use crate::gamepad::GamepadManager;
use crate::input_mapping::{InputSource, AXIS_ACTIVATION_THRESHOLD};
use crate::keyboard::{convert_physical_key, KeyboardState};
use crate::mouse::MouseState;
use std::collections::VecDeque;
use winit::event::{ElementState, WindowEvent};

/// Approximate pixels per scroll "line", used to normalize trackpad/pixel
/// scroll deltas to the same scale as mouse wheel line deltas.
const SCROLL_PIXELS_PER_LINE: f32 = 16.0;

/// Input events that can be queued for processing.
///
/// Uses winit's `KeyCode` and `MouseButton` directly, by decision: no
/// conversion layer. If another windowing backend ever arrives, the mapping
/// goes at the boundary in [`InputHandler::handle_window_event`].
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Keyboard key pressed
    KeyPressed(winit::keyboard::KeyCode),
    /// Keyboard key released
    KeyReleased(winit::keyboard::KeyCode),
    /// Mouse button pressed
    MouseButtonPressed(winit::event::MouseButton),
    /// Mouse button released
    MouseButtonReleased(winit::event::MouseButton),
    /// Mouse moved to position
    MouseMoved(f32, f32),
    /// Mouse wheel scrolled
    MouseWheelScrolled(f32),
    /// Gamepad button pressed
    GamepadButtonPressed(u32, crate::gamepad::GamepadButton),
    /// Gamepad button released
    GamepadButtonReleased(u32, crate::gamepad::GamepadButton),
    /// Gamepad axis updated
    GamepadAxisUpdated(u32, crate::gamepad::GamepadAxis, f32),
    /// Gamepad connected (backends emit this; pads also auto-register on
    /// their first button/axis event)
    GamepadConnected(u32),
    /// Gamepad disconnected. Drops the pad's state, so all of its sources
    /// read released afterwards — no just-released edge is emitted.
    GamepadDisconnected(u32),
}

/// A unified handler for all input device state
#[derive(Debug, Default)]
pub struct InputHandler {
    /// Keyboard state
    keyboard: KeyboardState,
    /// Mouse state
    mouse: MouseState,
    /// Gamepad manager
    gamepads: GamepadManager,
    /// Event queue for buffering input events
    event_queue: VecDeque<InputEvent>,
}

impl InputHandler {
    /// Create a new input handler
    pub fn new() -> Self {
        Self::default()
    }

    // ================== Input Source Checks ==================

    /// Check if an input source is currently pressed
    pub fn is_source_pressed(&self, source: &InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keyboard.is_key_pressed(*key),
            InputSource::Mouse(button) => self.mouse.is_button_pressed(*button),
            InputSource::Gamepad(id, button) => self
                .gamepads
                .get_gamepad(*id)
                .is_some_and(|g| g.is_button_pressed(*button)),
            InputSource::GamepadAxis(id, axis, direction) => self
                .gamepads
                .get_gamepad(*id)
                .is_some_and(|g| g.axis_active(*axis, *direction, AXIS_ACTIVATION_THRESHOLD)),
        }
    }

    /// Check if an input source was just pressed this frame
    pub fn is_source_just_pressed(&self, source: &InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keyboard.is_key_just_pressed(*key),
            InputSource::Mouse(button) => self.mouse.is_button_just_pressed(*button),
            InputSource::Gamepad(id, button) => self
                .gamepads
                .get_gamepad(*id)
                .is_some_and(|g| g.is_button_just_pressed(*button)),
            InputSource::GamepadAxis(id, axis, direction) => {
                self.gamepads.get_gamepad(*id).is_some_and(|g| {
                    g.axis_just_activated(*axis, *direction, AXIS_ACTIVATION_THRESHOLD)
                })
            }
        }
    }

    /// Check if an input source was just released this frame
    pub fn is_source_just_released(&self, source: &InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keyboard.is_key_just_released(*key),
            InputSource::Mouse(button) => self.mouse.is_button_just_released(*button),
            InputSource::Gamepad(id, button) => self
                .gamepads
                .get_gamepad(*id)
                .is_some_and(|g| g.is_button_just_released(*button)),
            InputSource::GamepadAxis(id, axis, direction) => {
                self.gamepads.get_gamepad(*id).is_some_and(|g| {
                    g.axis_just_deactivated(*axis, *direction, AXIS_ACTIVATION_THRESHOLD)
                })
            }
        }
    }

    /// Check if an input source was pressed at the end of the previous frame
    pub fn was_source_pressed(&self, source: &InputSource) -> bool {
        match source {
            InputSource::Keyboard(key) => self.keyboard.was_key_pressed(*key),
            InputSource::Mouse(button) => self.mouse.was_button_pressed(*button),
            InputSource::Gamepad(id, button) => self
                .gamepads
                .get_gamepad(*id)
                .is_some_and(|g| g.was_button_pressed(*button)),
            InputSource::GamepadAxis(id, axis, direction) => self
                .gamepads
                .get_gamepad(*id)
                .is_some_and(|g| g.axis_was_active(*axis, *direction, AXIS_ACTIVATION_THRESHOLD)),
        }
    }

    // ================== Event Queue ==================

    /// Queue an input event for later processing
    pub fn queue_event(&mut self, event: InputEvent) {
        self.event_queue.push_back(event);
    }

    /// Process all queued input events, updating input state.
    ///
    /// Call this at the **start** of each frame, before reading input state.
    /// This processes all events that were queued via `handle_window_event()`
    /// since the last call, updating keyboard, mouse, and gamepad state.
    ///
    /// After calling this, you can read current input state via methods like
    /// `is_key_pressed()`, `is_key_just_pressed()`, or evaluate actions via
    /// [`crate::InputMapping`].
    ///
    /// At the end of the frame, call `end_frame()` to reset per-frame state.
    pub fn process_queued_events(&mut self) {
        while let Some(event) = self.event_queue.pop_front() {
            self.process_event(event);
        }
    }

    /// Process a single input event
    fn process_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::KeyPressed(key) => {
                self.keyboard.handle_key_press(key);
            }
            InputEvent::KeyReleased(key) => {
                self.keyboard.handle_key_release(key);
            }
            InputEvent::MouseButtonPressed(button) => {
                self.mouse.handle_button_press(button);
            }
            InputEvent::MouseButtonReleased(button) => {
                self.mouse.handle_button_release(button);
            }
            InputEvent::MouseMoved(x, y) => {
                self.mouse.update_position(x, y);
            }
            InputEvent::MouseWheelScrolled(delta) => {
                self.mouse.update_wheel_delta(delta);
            }
            InputEvent::GamepadButtonPressed(id, button) => {
                self.gamepads.get_or_register(id).handle_button_press(button);
            }
            InputEvent::GamepadButtonReleased(id, button) => {
                self.gamepads.get_or_register(id).handle_button_release(button);
            }
            InputEvent::GamepadAxisUpdated(id, axis, value) => {
                self.gamepads.get_or_register(id).update_axis(axis, value);
            }
            InputEvent::GamepadConnected(id) => {
                self.gamepads.register_gamepad(id);
            }
            InputEvent::GamepadDisconnected(id) => {
                self.gamepads.unregister_gamepad(id);
            }
        }
    }

    /// Handle a window event by queuing it for later processing
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(key_code) = convert_physical_key(event.physical_key) {
                    let input_event = match event.state {
                        ElementState::Pressed => InputEvent::KeyPressed(key_code),
                        ElementState::Released => InputEvent::KeyReleased(key_code),
                    };
                    self.queue_event(input_event);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let input_event = match state {
                    ElementState::Pressed => InputEvent::MouseButtonPressed(*button),
                    ElementState::Released => InputEvent::MouseButtonReleased(*button),
                };
                self.queue_event(input_event);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.queue_event(InputEvent::MouseMoved(position.x as f32, position.y as f32));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Normalize both variants to "lines" so scroll speed is
                // consistent across mice (LineDelta) and trackpads (PixelDelta)
                let scroll_delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    winit::event::MouseScrollDelta::PixelDelta(position) => {
                        position.y as f32 / SCROLL_PIXELS_PER_LINE
                    }
                };
                self.queue_event(InputEvent::MouseWheelScrolled(scroll_delta));
            }
            _ => {
                // Other events can be ignored for now
            }
        }
    }

    // ================== Frame Lifecycle ==================

    /// Convenience method that processes events and resets state in one call.
    ///
    /// This is equivalent to calling `process_queued_events()` followed by `end_frame()`.
    /// Use this for simple applications where you don't need fine-grained control over
    /// when events are processed vs when state is reset.
    ///
    /// For most game loops, prefer the two-step approach:
    /// ```
    /// # use input::InputHandler;
    /// # let mut input = InputHandler::new();
    /// input.process_queued_events();  // At start of frame
    /// // ... game logic ...
    /// input.end_frame();              // At end of frame
    /// ```
    pub fn update(&mut self) {
        self.process_queued_events();
        self.end_frame();
    }

    /// Reset per-frame input state for the next frame.
    ///
    /// Clears "just pressed" / "just released" flags and per-frame mouse
    /// deltas (movement, wheel). Call this at the **end** of each frame,
    /// after all game logic has had a chance to check input state.
    ///
    /// ```
    /// # use input::{InputHandler, InputEvent};
    /// # use winit::keyboard::KeyCode;
    /// # let mut input = InputHandler::new();
    /// # let mut player_jumped = false;
    /// // Start of frame
    /// input.process_queued_events();
    ///
    /// // Game logic - can check is_key_just_pressed(), etc.
    /// if input.is_key_just_pressed(KeyCode::Space) {
    ///     player_jumped = true;
    /// }
    ///
    /// // End of frame - reset for next frame
    /// input.end_frame();
    /// # let _ = player_jumped;
    /// ```
    pub fn end_frame(&mut self) {
        self.keyboard.clear_frame_state();
        self.mouse.clear_frame_state();
        self.gamepads.clear_frame_state();
    }

    // ================== Device Accessors ==================

    /// Get a reference to the keyboard state
    pub fn keyboard(&self) -> &KeyboardState {
        &self.keyboard
    }

    /// Get a mutable reference to the keyboard state
    pub fn keyboard_mut(&mut self) -> &mut KeyboardState {
        &mut self.keyboard
    }

    /// Get a reference to the mouse state
    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    /// Get a mutable reference to the mouse state
    pub fn mouse_mut(&mut self) -> &mut MouseState {
        &mut self.mouse
    }

    /// Get a reference to the gamepad manager
    pub fn gamepads(&self) -> &GamepadManager {
        &self.gamepads
    }

    // ================== Convenience Queries ==================

    /// Check if a specific key is currently pressed
    pub fn is_key_pressed(&self, key: winit::keyboard::KeyCode) -> bool {
        self.keyboard.is_key_pressed(key)
    }

    /// Check if a specific key was just pressed this frame
    pub fn is_key_just_pressed(&self, key: winit::keyboard::KeyCode) -> bool {
        self.keyboard.is_key_just_pressed(key)
    }

    /// Check if a specific key was just released this frame
    pub fn is_key_just_released(&self, key: winit::keyboard::KeyCode) -> bool {
        self.keyboard.is_key_just_released(key)
    }

    /// Check if a mouse button is currently pressed
    pub fn is_mouse_button_pressed(&self, button: winit::event::MouseButton) -> bool {
        self.mouse.is_button_pressed(button)
    }

    /// Check if a mouse button was just pressed this frame
    pub fn is_mouse_button_just_pressed(&self, button: winit::event::MouseButton) -> bool {
        self.mouse.is_button_just_pressed(button)
    }

    /// Get current mouse position
    pub fn mouse_position(&self) -> crate::mouse::MousePosition {
        self.mouse.position()
    }

    /// Get mouse movement delta accumulated this frame
    pub fn mouse_movement_delta(&self) -> (f32, f32) {
        self.mouse.movement_delta()
    }

    /// Get mouse wheel scroll delta accumulated this frame
    pub fn mouse_wheel_delta(&self) -> f32 {
        self.mouse.wheel_delta()
    }
}
