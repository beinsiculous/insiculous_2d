//! Widget interaction and state management.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use glam::Vec2;
use input::prelude::InputHandler;

use crate::input_state::{InputState, KeyRepeat};
use crate::text_edit::TextEditState;
use crate::Rect;

/// Fallback frame delta for [`InteractionManager::begin_frame`] callers that
/// don't thread a real dt (key repeat paces off this).
const DEFAULT_FRAME_DT: f32 = 1.0 / 60.0;

/// Unique identifier for a widget.
/// Can be created from strings, integers, or tuples for hierarchical IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    /// Create a widget ID from a hash value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Create a widget ID from a string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Create a widget ID from a string and index (for lists).
    pub fn from_str_index(s: &str, index: usize) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        index.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Get the raw ID value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl From<&str> for WidgetId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<(&str, usize)> for WidgetId {
    fn from((s, index): (&str, usize)) -> Self {
        Self::from_str_index(s, index)
    }
}

/// State of a widget in the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    /// Widget is not interacted with
    Normal,
    /// Mouse is hovering over the widget
    Hovered,
    /// Widget is being pressed/dragged
    Active,
    /// Widget is disabled and cannot be interacted with
    Disabled,
}

/// Result of a widget interaction.
#[derive(Debug, Clone, Copy)]
pub struct InteractionResult {
    /// Current state of the widget
    pub state: WidgetState,
    /// True if the widget was clicked (mouse released over it while active)
    pub clicked: bool,
    /// True if the widget is currently being dragged
    pub dragging: bool,
}

impl Default for InteractionResult {
    fn default() -> Self {
        Self {
            state: WidgetState::Normal,
            clicked: false,
            dragging: false,
        }
    }
}

/// Persistent state for widgets that need to track data across frames.
#[derive(Debug, Clone, Default)]
pub struct WidgetPersistentState {
    /// Whether the widget was seen this frame (for garbage collection)
    pub seen_this_frame: bool,
    /// Text-editing state (buffer, cursor, selection) for input widgets
    pub edit: TextEditState,
    /// In-flight drag-scrub gesture on a numeric input, if any
    pub scrub: Option<ScrubState>,
}

/// A drag-scrub gesture on a numeric input: armed on press, activated once
/// the pointer travels past the click threshold, cleared on release. Arming
/// re-seeds `press_x`/`start_value`, so stale state can never leak into a
/// later gesture (even across widgets that share an id).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrubState {
    /// Pointer x at the press that armed the gesture.
    pub press_x: f32,
    /// Value at the press — scrub output is `start + dx * step`.
    pub start_value: f32,
    /// Whether the pointer has crossed the click/scrub threshold.
    pub active: bool,
}

/// Tracks interaction state for all widgets in the UI.
pub struct InteractionManager {
    /// Currently active widget (being pressed/dragged)
    active_widget: Option<WidgetId>,
    /// Input state snapshot for this frame
    input: InputState,
    /// Persistent state storage for widgets
    persistent_state: HashMap<WidgetId, WidgetPersistentState>,
    /// Widget that had keyboard focus
    focus_widget: Option<WidgetId>,
    /// Regions (e.g. open dropdowns) that swallow mouse input for all
    /// widgets outside the overlay scope. Cleared each frame.
    blocking_rects: Vec<Rect>,
    /// Whether interact() calls are currently inside an overlay (exempt
    /// from blocking rects). Cleared each frame.
    overlay_scope: bool,
    /// Hold timers for key repeat (arrows, Backspace, Delete)
    key_repeat: KeyRepeat,
}

impl Default for InteractionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionManager {
    /// Create a new interaction manager.
    pub fn new() -> Self {
        Self {
            active_widget: None,
            input: InputState::default(),
            persistent_state: HashMap::new(),
            focus_widget: None,
            blocking_rects: Vec::new(),
            overlay_scope: false,
            key_repeat: KeyRepeat::default(),
        }
    }

    /// Begin a new frame with a default frame delta for key repeat.
    /// Prefer [`Self::begin_frame_dt`] when a real delta time is available.
    pub fn begin_frame(&mut self, input: &InputHandler) {
        self.begin_frame_dt(input, DEFAULT_FRAME_DT);
    }

    /// Begin a new frame, updating input state. `dt` (seconds since the last
    /// frame) paces held-key repeat for text inputs.
    pub fn begin_frame_dt(&mut self, input: &InputHandler, dt: f32) {
        self.input = InputState::from_input_handler_with_repeat(input, &mut self.key_repeat, dt);

        // Blocking regions are re-registered each frame by whatever overlay is open
        self.blocking_rects.clear();
        self.overlay_scope = false;

        // Don't clear active_widget here - let widgets check for clicks first
        // The active_widget will be cleared in end_frame() after click detection

        // ...unless the release event was missed entirely (window lost focus
        // mid-press): not held, not releasing — the gesture is over, and a
        // stuck active widget would block all other widgets and wants_mouse()
        // consumers until the next click.
        if !self.input.mouse_down && !self.input.mouse_just_released {
            self.active_widget = None;
        }

        // Mark all persistent state as not seen
        for state in self.persistent_state.values_mut() {
            state.seen_this_frame = false;
        }
    }

    /// End a frame, cleaning up stale state.
    pub fn end_frame(&mut self) {
        // Clear active widget if mouse was just released (after click detection)
        if self.input.mouse_just_released {
            self.active_widget = None;
        }

        // Garbage collect persistent state for widgets not submitted this frame.
        // The focused widget's state is kept even when unseen so a text input
        // doesn't lose its edit buffer if its panel skips a frame.
        let focus = self.focus_widget;
        self.persistent_state
            .retain(|id, state| state.seen_this_frame || focus == Some(*id));
    }

    /// Get the current input state.
    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Get the current mouse position.
    pub fn mouse_pos(&self) -> Vec2 {
        self.input.mouse_pos
    }

    /// Check if a widget has keyboard focus.
    pub fn is_focused(&self, id: WidgetId) -> bool {
        self.focus_widget == Some(id)
    }

    /// Check if any widget has keyboard focus (e.g. a text input being edited).
    pub(crate) fn has_focus(&self) -> bool {
        self.focus_widget.is_some()
    }

    /// Whether a widget owns the current mouse gesture: true from the press
    /// that landed on a widget through the release frame (`active_widget` is
    /// cleared in [`Self::end_frame`], after click detection). Raw-input
    /// consumers (e.g. viewport picking) should not treat the mouse as theirs
    /// while this returns `true`.
    pub fn wants_mouse(&self) -> bool {
        self.active_widget.is_some()
    }

    /// Register a region that swallows mouse input for all widgets outside
    /// the overlay scope (used by dropdown menus and popups). Cleared each frame.
    pub fn push_blocking_rect(&mut self, rect: Rect) {
        self.blocking_rects.push(rect);
    }

    /// Set whether subsequent interact() calls belong to an overlay and are
    /// therefore exempt from blocking rects.
    pub fn set_overlay_scope(&mut self, overlay: bool) {
        self.overlay_scope = overlay;
    }

    /// Check if mouse input at the given position is swallowed by a blocking
    /// region (an open dropdown or popup).
    pub fn is_blocked_at(&self, pos: Vec2) -> bool {
        self.blocking_rects.iter().any(|r| r.contains(pos))
    }

    /// Set keyboard focus to a widget.
    pub fn set_focus(&mut self, id: WidgetId) {
        self.focus_widget = Some(id);
    }

    /// Clear keyboard focus.
    pub fn clear_focus(&mut self) {
        self.focus_widget = None;
    }

    /// Get persistent state for a widget, creating default if not present.
    pub fn get_state(&mut self, id: WidgetId) -> &mut WidgetPersistentState {
        let state = self.persistent_state.entry(id).or_default();
        state.seen_this_frame = true;
        state
    }

    /// Get persistent state for a widget if it exists.
    pub fn get_state_if_exists(&self, id: WidgetId) -> Option<&WidgetPersistentState> {
        self.persistent_state.get(&id)
    }

    /// Process interaction for a widget.
    pub fn interact(&mut self, id: WidgetId, bounds: Rect, enabled: bool) -> InteractionResult {
        // Mark state as seen
        self.get_state(id).seen_this_frame = true;

        if !enabled {
            return InteractionResult {
                state: WidgetState::Disabled,
                ..Default::default()
            };
        }

        // Widgets outside an overlay are inert while the mouse is over a
        // blocking region (open dropdown/popup): no hover, no click, no
        // activation. An already-active widget keeps its slot — end_frame
        // clears it on mouse release.
        if !self.overlay_scope && self.is_blocked_at(self.input.mouse_pos) {
            return InteractionResult::default();
        }

        let mouse_in_bounds = bounds.contains(self.input.mouse_pos);

        // Check if this widget should become active
        if mouse_in_bounds && self.input.mouse_just_pressed && self.active_widget.is_none() {
            self.active_widget = Some(id);
        }

        // Determine state and interactions
        let is_active = self.active_widget == Some(id);
        let is_hot = mouse_in_bounds;

        // Click happens when mouse is released while active AND still over the widget
        let clicked = is_active && self.input.mouse_just_released && mouse_in_bounds;

        let state = if is_active && !self.input.mouse_just_released {
            WidgetState::Active
        } else if is_hot {
            WidgetState::Hovered
        } else {
            WidgetState::Normal
        };

        InteractionResult {
            state,
            clicked,
            dragging: is_active && self.input.mouse_down,
        }
    }

}

#[cfg(test)]
mod tests;
