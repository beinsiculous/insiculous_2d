//! Widget interaction and state management.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use glam::Vec2;
use input::prelude::InputHandler;

use crate::input_state::{InputState, KeyRepeat};
use crate::text_edit::TextEditState;
use crate::{Rect, UiLayer};

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

    /// Create a widget ID by hashing a string.
    pub fn hashed(s: &str) -> Self {
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
        Self::hashed(s)
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

/// A region an open overlay claims for the rest of the frame, and the layer
/// it claims it on.
#[derive(Debug, Clone, Copy)]
struct BlockingRegion {
    rect: Rect,
    layer: UiLayer,
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
    /// Regions (an open dropdown, a modal's scrim) that swallow mouse input.
    /// A widget outside any overlay scope is inert under every one of them;
    /// a widget inside a scope is inert only under a region from a higher
    /// layer. Cleared each frame.
    blocking_regions: Vec<BlockingRegion>,
    /// The layer of the overlay scope subsequent interact() calls belong
    /// to, `None` outside any scope. Cleared each frame.
    overlay_scope: Option<UiLayer>,
    /// Hold timers for key repeat (arrows, Backspace, Delete)
    key_repeat: KeyRepeat,
    /// Frame delta of the current frame (seconds). Kept so anything
    /// pacing off wall-clock time — key repeat today, the tooltip's
    /// rest-to-show delay — reads one number.
    frame_dt: f32,
    /// The editable fields drawn so far this frame, in draw order. Tab and
    /// Shift-Tab walk this list.
    current_frame_order: Vec<WidgetId>,
    /// The previous frame's registration order. A Tab commit looks its
    /// neighbour up here rather than in `current_frame_order`, which at
    /// that moment is missing every field drawn after the committing one.
    previous_frame_order: Vec<WidgetId>,
    /// The field a Tab or Shift-Tab commit handed focus to, waiting for a
    /// later frame to claim it (see `UIContext::edit_field_click`, which
    /// also says why the committing frame can never claim it), paired with
    /// how many consecutive frames it has gone without drawing at all. A
    /// mouse press discards it immediately. A target still drawing every
    /// frame — even one it cannot claim because an overlay blocks it, its
    /// one deliberately indefinite wait — keeps its clock at zero; only a
    /// target that stops drawing entirely (scrolled off, its panel closed,
    /// the selection changed) ages, and is forgotten past one grace frame.
    /// `WidgetId` is a bare string hash with no notion of "this entity" — a
    /// different entity's field can hash to the very same id — so a target
    /// that goes quiet must not sit armed indefinitely, or it can steal
    /// focus from an unrelated field that later reuses its id.
    pending_focus_target: Option<(WidgetId, u8)>,
    /// Whether the pending target's own registration found it blocked by an
    /// overlay THIS frame. Reset to `false` at the top of every frame, so it
    /// answers "as of the last time anyone checked" — which is every frame
    /// the target draws, blocked or not, and stays at its reset value on a
    /// frame the target does not draw at all. Read by `wants_keyboard()`: a
    /// target only shields the keyboard while it is actually reachable soon,
    /// never for as long as some overlay happens to sit over it (see that
    /// method for why an unconditional "pending = keyboard owned" is wrong).
    pending_target_blocked: bool,
}

/// How many consecutive frames a pending traversal target may go without
/// drawing at all before it is forgotten. A frame where it draws but cannot
/// be claimed (an overlay blocks it) resets this to zero rather than
/// counting against it — only genuinely going quiet risks a stale match.
/// Kept at exactly 1 (the ordinary one-frame resolution) deliberately: a
/// wider window was tried and reverted (review-38 F1/F2) — inspector field
/// ids are pure layout indices with no entity component
/// (`crates/editor/src/field_style.rs`), so the same id names, say,
/// position.x for every entity, and a selection change with no mouse press
/// (Ctrl+A, hierarchy keyboard navigation, undo/redo restoring a prior
/// selection) can swap in a same-shaped different entity within a few
/// frames — a wider grace widens exactly the window this age exists to
/// close, for a speculative benefit (an unconfirmed zero-field inspector
/// pass) that was never worth that trade.
const PENDING_FOCUS_MAX_AGE: u8 = 1;

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
            blocking_regions: Vec::new(),
            overlay_scope: None,
            key_repeat: KeyRepeat::default(),
            frame_dt: DEFAULT_FRAME_DT,
            current_frame_order: Vec::new(),
            previous_frame_order: Vec::new(),
            pending_focus_target: None,
            pending_target_blocked: false,
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
        self.frame_dt = dt;

        // The frame that just ended becomes the order a Tab commit looks its
        // neighbour up in; this frame starts collecting its own. Two buffers
        // swapped rather than two rebuilt — this runs on every frame.
        self.previous_frame_order.clear();
        std::mem::swap(&mut self.previous_frame_order, &mut self.current_frame_order);

        // A click always wins over a traversal still waiting to resolve: a
        // fresh press is newer intent than the Tab that scheduled it, and
        // leaving it standing would focus a field the user just clicked away
        // from on a later frame.
        if self.input.mouse_just_pressed {
            self.pending_focus_target = None;
        }
        // Reset for this frame; the target's own registration (if it draws)
        // sets it back to `true` when it finds itself blocked.
        self.pending_target_blocked = false;
        // A target still drawing every frame (even blocked by an overlay, its
        // one deliberately indefinite wait) resets its grace clock — it is
        // still legitimately the same target. Only a target that has
        // stopped drawing at all — scrolled off, its panel closed, the
        // selection changed to a different entity whose fields can hash to
        // the very same ids — ages, and is forgotten past one grace frame
        // rather than left armed to match whatever unrelated field draws
        // with that id next.
        if let Some((id, age)) = &mut self.pending_focus_target {
            if self.previous_frame_order.contains(id) {
                *age = 0;
            } else {
                *age += 1;
                if *age > PENDING_FOCUS_MAX_AGE {
                    self.pending_focus_target = None;
                }
            }
        }

        // Blocking regions are re-registered each frame by whatever overlay is open
        self.blocking_regions.clear();
        self.overlay_scope = None;

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

    /// Seconds since the previous frame, as passed to [`Self::begin_frame_dt`].
    pub(crate) fn frame_dt(&self) -> f32 {
        self.frame_dt
    }

    /// The layer of the overlay scope `interact()` calls currently belong
    /// to, `None` outside any overlay.
    pub(crate) fn overlay_scope(&self) -> Option<UiLayer> {
        self.overlay_scope
    }

    /// Check if a widget has keyboard focus.
    pub fn is_focused(&self, id: WidgetId) -> bool {
        self.focus_widget == Some(id)
    }

    /// Check if any widget has keyboard focus (e.g. a text input being edited).
    pub(crate) fn has_focus(&self) -> bool {
        self.focus_widget.is_some()
    }

    /// Whether a host should suppress its own keyboard shortcuts: a widget
    /// has focus, OR a Tab/Shift-Tab commit is mid-traversal to one and it is
    /// currently reachable (not sitting behind an overlay). A target an
    /// overlay blocks is deliberately excluded — it can wait indefinitely
    /// for that overlay to close, and treating "waiting, possibly forever"
    /// the same as "about to land next frame" would swallow every editor
    /// shortcut, play controls included, for as long as the overlay stays
    /// open.
    pub(crate) fn wants_keyboard(&self) -> bool {
        self.has_focus() || (self.pending_focus_target.is_some() && !self.pending_target_blocked)
    }

    /// Whether a widget owns the current mouse gesture: true from the press
    /// that landed on a widget through the release frame (`active_widget` is
    /// cleared in [`Self::end_frame`], after click detection). Raw-input
    /// consumers (e.g. viewport picking) should not treat the mouse as theirs
    /// while this returns `true`.
    pub fn wants_mouse(&self) -> bool {
        self.active_widget.is_some()
    }

    /// Register a region that swallows mouse input, claimed by an overlay on
    /// `layer` (a dropdown on Floating, a modal's scrim on Modal). Widgets
    /// outside any overlay scope are inert under it; widgets inside a scope
    /// are inert under it only when their scope's layer is lower. Cleared
    /// each frame.
    pub fn push_blocking_rect(&mut self, rect: Rect, layer: UiLayer) {
        self.blocking_regions.push(BlockingRegion { rect, layer });
    }

    /// Set the overlay scope subsequent interact() calls belong to: the
    /// layer of the open overlay, or `None` outside any overlay.
    pub fn set_overlay_scope(&mut self, scope: Option<UiLayer>) {
        self.overlay_scope = scope;
    }

    /// Check if mouse input at the given position is swallowed by any
    /// blocking region — the question a raw-input consumer (viewport
    /// picking) asks, which lives outside every scope.
    pub fn is_blocked_at(&self, pos: Vec2) -> bool {
        self.is_blocked_for_scope(None, pos)
    }

    /// Whether a widget in `scope` is inert at `pos`: some region contains
    /// the point, and the widget is outside every scope or the region came
    /// from a higher layer. A scope on its own layer or above a region is
    /// exempt — a dropdown hanging over the toolbar strip stays live — but
    /// a modal's scrim reaches every scope below it, or a Play button drawn
    /// after the dialog would change the session while the dialog still asks.
    ///
    /// The tooltip asks this too, with [`Self::overlay_scope`]: an
    /// affordance that is inert must not raise one.
    pub(crate) fn is_blocked_for_scope(&self, scope: Option<UiLayer>, pos: Vec2) -> bool {
        self.blocking_regions.iter().any(|region| {
            region.rect.contains(pos)
                && match scope {
                    None => true,
                    Some(scope_layer) => region.layer > scope_layer,
                }
        })
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

    /// Record an editable field drawn this frame. Call it once per field per
    /// frame, in draw order — that order is the one Tab and Shift-Tab walk.
    pub(crate) fn register_editable_field(&mut self, id: WidgetId) {
        self.current_frame_order.push(id);
    }

    /// The field a Tab (`backward == false`) or Shift-Tab commit from `id`
    /// hands focus to: its neighbour in the previous frame's order, wrapping
    /// at either end. `None` when `id` is not in that order — the first
    /// frame ever, or a field set that changed since — or when it is the
    /// only field there is.
    pub(crate) fn traversal_neighbour(&self, id: WidgetId, backward: bool) -> Option<WidgetId> {
        let order = &self.previous_frame_order;
        if order.len() < 2 {
            return None;
        }
        let position = order.iter().position(|candidate| *candidate == id)?;
        let offset = if backward { order.len() - 1 } else { 1 };
        Some(order[(position + offset) % order.len()])
    }

    /// The field a pending traversal is waiting for.
    pub(crate) fn pending_focus_target(&self) -> Option<WidgetId> {
        self.pending_focus_target.map(|(id, _)| id)
    }

    /// Schedule `id` as the field a later frame focuses (starting its grace
    /// period over), or forget the pending one with `None`.
    pub(crate) fn set_pending_focus_target(&mut self, target: Option<WidgetId>) {
        self.pending_focus_target = target.map(|id| (id, 0));
    }

    /// Record that the pending target's own registration found it blocked
    /// by an overlay this frame — so `wants_keyboard()` stops shielding it.
    pub(crate) fn note_pending_target_blocked(&mut self) {
        self.pending_target_blocked = true;
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

        // A widget under a blocking region it is not exempt from is inert:
        // no hover, no click, no activation. An already-active widget keeps
        // its slot — end_frame clears it on mouse release.
        if self.is_blocked_for_scope(self.overlay_scope, self.input.mouse_pos) {
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
