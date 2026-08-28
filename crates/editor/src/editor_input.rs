//! Editor-specific input: the ONE table of editor shortcuts.
//!
//! The editor owns its chord model (`EditorBinding`) instead of extending
//! the engine's `InputSource`: gameplay input has no chord semantics, and
//! `InputSource` is serialized into player-facing bindings JSON — a chord
//! variant there would be a save-format migration bought for zero gameplay
//! benefit. Modifier model is Ctrl+Shift only; Alt/Super are deliberately
//! outside it (left to the OS/window manager — widen `Chord` if that ever
//! changes).
//!
//! Two consumption paths share this table:
//! - the EVENT path: `resolve(key, ctrl, shift)` from `on_key_pressed`
//!   (an exact chord beats an any-mods binding for the same key)
//! - the POLL path: `is_action_pressed`/`is_action_just_pressed` for
//!   held-state actions (Pan, modifiers, F/Home camera requests)

use std::collections::HashMap;

use input::prelude::MouseButton;
use input::InputHandler;
use winit::keyboard::KeyCode;

/// All editor actions that can be bound to input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorAction {
    // ---- Viewport navigation ----
    /// Pan the viewport camera (held)
    Pan,
    /// Zoom in
    ZoomIn,
    /// Zoom out
    ZoomOut,
    /// Reset zoom to 1:1
    ResetZoom,
    /// Focus camera on selection (Shift = frame all)
    FocusSelection,
    /// Reset camera to origin
    ResetCamera,

    // ---- Selection ----
    /// Select entity under cursor
    Select,
    /// Add to selection (held modifier)
    AddToSelection,
    /// Toggle selection (held modifier)
    ToggleSelection,
    /// Select all entities
    SelectAll,
    /// Cancel the most specific live thing: gizmo drag → marquee → selection
    Cancel,

    // ---- Tools ----
    /// Switch to select tool
    ToolSelect,
    /// Switch to move tool
    ToolMove,
    /// Switch to rotate tool
    ToolRotate,
    /// Switch to scale tool
    ToolScale,

    // ---- Edit operations ----
    /// Delete selected entities
    Delete,
    /// Duplicate selected entities
    Duplicate,
    /// Undo last operation
    Undo,
    /// Redo last undone operation
    Redo,
    /// Copy selection to the entity clipboard
    Copy,
    /// Paste the entity clipboard
    Paste,
    /// Cut selection to the entity clipboard
    Cut,
    /// Rename the primary selection (inline, in the hierarchy)
    RenameSelected,
    /// Nudge the selection left by one unit (Shift = 10)
    NudgeLeft,
    /// Nudge the selection right by one unit (Shift = 10)
    NudgeRight,
    /// Nudge the selection up by one unit (Shift = 10)
    NudgeUp,
    /// Nudge the selection down by one unit (Shift = 10)
    NudgeDown,

    // ---- Scene file ----
    /// Save the scene
    Save,
    /// Save the scene under a new path
    SaveAs,
    /// New empty scene
    NewScene,
    /// Open a scene
    OpenScene,

    // ---- View / play ----
    /// Toggle grid display
    ToggleGrid,
    /// Toggle collider overlay
    ToggleColliders,
    /// Toggle snap to grid
    ToggleSnap,
    /// Start or resume a play session
    PlayResume,
    /// Toggle between Play and Pause
    TogglePlayPause,
    /// Stop the play session
    StopPlay,
    /// Toggle the play-session camera follow (viewport mirrors the game
    /// camera vs. free pan/zoom — issue #42)
    ToggleCameraFollow,
}

/// One input binding in the editor's chord model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorBinding {
    /// Fires only when the Ctrl/Shift state matches EXACTLY — Ctrl+Shift+Z,
    /// Ctrl+Z and bare Z are three distinct chords.
    Chord {
        key: KeyCode,
        ctrl: bool,
        shift: bool,
    },
    /// Fires on the key under ANY modifier state (Space-pan, arrows, F,
    /// Home, Escape — keys whose handler reads the modifiers itself).
    KeyAnyMods(KeyCode),
    /// A mouse button (poll path only).
    Mouse(MouseButton),
}

impl EditorBinding {
    /// A no-modifier chord.
    pub fn key(key: KeyCode) -> Self {
        Self::Chord { key, ctrl: false, shift: false }
    }

    /// A Ctrl+key chord.
    pub fn ctrl(key: KeyCode) -> Self {
        Self::Chord { key, ctrl: true, shift: false }
    }

    /// A Ctrl+Shift+key chord.
    pub fn ctrl_shift(key: KeyCode) -> Self {
        Self::Chord { key, ctrl: true, shift: true }
    }
}

/// Current editor input state snapshot.
#[derive(Debug, Clone, Default)]
pub struct EditorInputState {
    /// Whether pan modifier is active (Space key held)
    pub pan_modifier: bool,
    /// Whether add-to-selection modifier is active (Shift held)
    pub add_modifier: bool,
    /// Whether toggle-selection modifier is active (Ctrl held)
    pub toggle_modifier: bool,
    /// Current mouse position (screen coords)
    pub mouse_position: glam::Vec2,
    /// Mouse movement delta
    pub mouse_delta: glam::Vec2,
    /// Mouse scroll delta
    pub scroll_delta: f32,
    /// Primary mouse button (left) state
    pub primary_button: ButtonState,
    /// Secondary mouse button (right) state
    pub secondary_button: ButtonState,
    /// Middle mouse button state
    pub middle_button: ButtonState,
}

/// State of a mouse button.
#[derive(Debug, Clone, Copy, Default)]
pub struct ButtonState {
    /// Currently held down
    pub pressed: bool,
    /// Just pressed this frame
    pub just_pressed: bool,
    /// Just released this frame
    pub just_released: bool,
}

/// The editor's shortcut table: rebindable chord-aware bindings with an
/// event-path `resolve` and poll-path action queries.
#[derive(Debug)]
pub struct EditorInputMapping {
    /// Bindings per action (the authoritative table)
    bindings: HashMap<EditorAction, Vec<EditorBinding>>,
    /// Exact-chord lookup, rebuilt on bind/unbind. Keyed by the FULL
    /// `(key, ctrl, shift)` tuple, so rebinding bare S never evicts Ctrl+S.
    chord_index: HashMap<(KeyCode, bool, bool), EditorAction>,
    /// Any-modifier lookup (consulted after the chord index).
    anymods_index: HashMap<KeyCode, EditorAction>,
}

impl Default for EditorInputMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorInputMapping {
    /// Create a new editor input mapping with default bindings.
    pub fn new() -> Self {
        let mut mapping = Self {
            bindings: HashMap::new(),
            chord_index: HashMap::new(),
            anymods_index: HashMap::new(),
        };
        mapping.set_default_bindings();
        mapping
    }

    /// Set default key bindings — the single source of truth for what the
    /// hardcoded shortcut table used to be.
    fn set_default_bindings(&mut self) {
        use EditorAction::*;
        use EditorBinding as B;

        // Viewport navigation (poll path; F/Home read Shift themselves)
        self.bind(Pan, B::KeyAnyMods(KeyCode::Space));
        self.bind(Pan, B::Mouse(MouseButton::Middle));
        self.bind(FocusSelection, B::KeyAnyMods(KeyCode::KeyF));
        self.bind(ResetCamera, B::KeyAnyMods(KeyCode::Home));
        self.bind(ZoomIn, B::key(KeyCode::Equal));
        self.bind(ZoomOut, B::key(KeyCode::Minus));
        self.bind(ResetZoom, B::key(KeyCode::Digit0));

        // Selection modifiers (poll path)
        self.bind(AddToSelection, B::KeyAnyMods(KeyCode::ShiftLeft));
        self.bind(AddToSelection, B::KeyAnyMods(KeyCode::ShiftRight));
        self.bind(ToggleSelection, B::KeyAnyMods(KeyCode::ControlLeft));
        self.bind(ToggleSelection, B::KeyAnyMods(KeyCode::ControlRight));
        self.bind(Select, B::Mouse(MouseButton::Left));
        self.bind(SelectAll, B::ctrl(KeyCode::KeyA));
        self.bind(Cancel, B::KeyAnyMods(KeyCode::Escape));

        // Tools (Q, W, E, R like most editors)
        self.bind(ToolSelect, B::key(KeyCode::KeyQ));
        self.bind(ToolMove, B::key(KeyCode::KeyW));
        self.bind(ToolRotate, B::key(KeyCode::KeyE));
        self.bind(ToolScale, B::key(KeyCode::KeyR));

        // Edit operations
        self.bind(Delete, B::KeyAnyMods(KeyCode::Delete));
        self.bind(Delete, B::KeyAnyMods(KeyCode::Backspace));
        self.bind(Duplicate, B::ctrl(KeyCode::KeyD));
        self.bind(Undo, B::ctrl(KeyCode::KeyZ));
        self.bind(Redo, B::ctrl_shift(KeyCode::KeyZ));
        self.bind(Redo, B::ctrl(KeyCode::KeyY));
        self.bind(Copy, B::ctrl(KeyCode::KeyC));
        self.bind(Paste, B::ctrl(KeyCode::KeyV));
        self.bind(Cut, B::ctrl(KeyCode::KeyX));
        self.bind(RenameSelected, B::key(KeyCode::F2));
        // Arrow nudge reads Shift itself for the ×10 step
        self.bind(NudgeLeft, B::KeyAnyMods(KeyCode::ArrowLeft));
        self.bind(NudgeRight, B::KeyAnyMods(KeyCode::ArrowRight));
        self.bind(NudgeUp, B::KeyAnyMods(KeyCode::ArrowUp));
        self.bind(NudgeDown, B::KeyAnyMods(KeyCode::ArrowDown));

        // Scene file
        self.bind(Save, B::ctrl(KeyCode::KeyS));
        self.bind(SaveAs, B::ctrl_shift(KeyCode::KeyS));
        self.bind(NewScene, B::ctrl(KeyCode::KeyN));
        self.bind(OpenScene, B::ctrl(KeyCode::KeyO));

        // View / play
        self.bind(ToggleGrid, B::key(KeyCode::KeyG));
        self.bind(ToggleColliders, B::key(KeyCode::KeyC));
        self.bind(ToggleSnap, B::key(KeyCode::KeyS));
        self.bind(PlayResume, B::key(KeyCode::F5));
        self.bind(TogglePlayPause, B::ctrl(KeyCode::KeyP));
        self.bind(StopPlay, B::ctrl_shift(KeyCode::KeyP));
        // Exact chord wins over the KeyAnyMods(F) focus binding on the
        // event path — F alone still frames the selection.
        self.bind(ToggleCameraFollow, B::ctrl_shift(KeyCode::KeyF));
    }

    /// Bind an input to an action. Rebinding EVICTS only the exact same
    /// binding from its previous owner — binding bare `S` elsewhere leaves
    /// `Ctrl+S` untouched.
    pub fn bind(&mut self, action: EditorAction, binding: EditorBinding) {
        // Evict the binding from its previous owner, if any
        let previous = match binding {
            EditorBinding::Chord { key, ctrl, shift } => {
                self.chord_index.insert((key, ctrl, shift), action)
            }
            EditorBinding::KeyAnyMods(key) => self.anymods_index.insert(key, action),
            EditorBinding::Mouse(_) => None, // mouse bindings are poll-only, no index
        };
        if let Some(previous_action) = previous {
            if previous_action != action {
                if let Some(list) = self.bindings.get_mut(&previous_action) {
                    list.retain(|b| *b != binding);
                }
            }
        }

        let list = self.bindings.entry(action).or_default();
        if !list.contains(&binding) {
            list.push(binding);
        }
    }

    /// Remove all bindings for an action.
    pub fn unbind(&mut self, action: EditorAction) {
        if let Some(list) = self.bindings.remove(&action) {
            for binding in list {
                match binding {
                    EditorBinding::Chord { key, ctrl, shift } => {
                        self.chord_index.remove(&(key, ctrl, shift));
                    }
                    EditorBinding::KeyAnyMods(key) => {
                        self.anymods_index.remove(&key);
                    }
                    EditorBinding::Mouse(_) => {}
                }
            }
        }
    }

    /// Get all bindings for an action.
    pub fn get_bindings(&self, action: EditorAction) -> &[EditorBinding] {
        self.bindings.get(&action).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Event-path lookup for a key press: the EXACT chord wins over an
    /// any-mods binding for the same key (Ctrl+Shift+Z beats Ctrl+Z beats Z
    /// by construction — each is its own index entry).
    pub fn resolve(&self, key: KeyCode, ctrl: bool, shift: bool) -> Option<EditorAction> {
        self.chord_index
            .get(&(key, ctrl, shift))
            .or_else(|| self.anymods_index.get(&key))
            .copied()
    }

    /// Whether a binding is currently satisfied by the device state.
    fn binding_pressed(binding: &EditorBinding, input: &InputHandler, just: bool) -> bool {
        let kb = input.keyboard();
        let ctrl_now = kb.is_key_pressed(KeyCode::ControlLeft) || kb.is_key_pressed(KeyCode::ControlRight);
        let shift_now = kb.is_key_pressed(KeyCode::ShiftLeft) || kb.is_key_pressed(KeyCode::ShiftRight);
        match *binding {
            EditorBinding::Chord { key, ctrl, shift } => {
                let key_state = if just { kb.is_key_just_pressed(key) } else { kb.is_key_pressed(key) };
                key_state && ctrl_now == ctrl && shift_now == shift
            }
            EditorBinding::KeyAnyMods(key) => {
                if just { kb.is_key_just_pressed(key) } else { kb.is_key_pressed(key) }
            }
            EditorBinding::Mouse(button) => {
                if just {
                    input.is_mouse_button_just_pressed(button)
                } else {
                    input.is_mouse_button_pressed(button)
                }
            }
        }
    }

    /// Check if an action is currently active (any bound input satisfied).
    pub fn is_action_pressed(&self, action: EditorAction, input: &InputHandler) -> bool {
        self.get_bindings(action)
            .iter()
            .any(|b| Self::binding_pressed(b, input, false))
    }

    /// Check if an action became active this frame.
    pub fn is_action_just_pressed(&self, action: EditorAction, input: &InputHandler) -> bool {
        self.get_bindings(action)
            .iter()
            .any(|b| Self::binding_pressed(b, input, true))
    }

    /// Update input state from InputHandler.
    pub fn update_state(&self, input: &InputHandler) -> EditorInputState {
        let mouse_pos = input.mouse_position();
        let mouse_delta = input.mouse_movement_delta();

        EditorInputState {
            pan_modifier: self.is_action_pressed(EditorAction::Pan, input),
            add_modifier: self.is_action_pressed(EditorAction::AddToSelection, input),
            toggle_modifier: self.is_action_pressed(EditorAction::ToggleSelection, input),
            mouse_position: glam::Vec2::new(mouse_pos.x, mouse_pos.y),
            mouse_delta: glam::Vec2::new(mouse_delta.0, mouse_delta.1),
            scroll_delta: input.mouse_wheel_delta(),
            primary_button: ButtonState {
                pressed: input.is_mouse_button_pressed(MouseButton::Left),
                just_pressed: input.is_mouse_button_just_pressed(MouseButton::Left),
                just_released: input.mouse().is_button_just_released(MouseButton::Left),
            },
            secondary_button: ButtonState {
                pressed: input.is_mouse_button_pressed(MouseButton::Right),
                just_pressed: input.is_mouse_button_just_pressed(MouseButton::Right),
                just_released: input.mouse().is_button_just_released(MouseButton::Right),
            },
            middle_button: ButtonState {
                pressed: input.is_mouse_button_pressed(MouseButton::Middle),
                just_pressed: input.is_mouse_button_just_pressed(MouseButton::Middle),
                just_released: input.mouse().is_button_just_released(MouseButton::Middle),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use EditorAction as A;

    #[test]
    fn test_every_default_chord_resolves_to_its_action() {
        let m = EditorInputMapping::new();
        let table = [
            (KeyCode::KeyQ, false, false, A::ToolSelect),
            (KeyCode::KeyW, false, false, A::ToolMove),
            (KeyCode::KeyE, false, false, A::ToolRotate),
            (KeyCode::KeyR, false, false, A::ToolScale),
            (KeyCode::KeyZ, true, false, A::Undo),
            (KeyCode::KeyZ, true, true, A::Redo),
            (KeyCode::KeyY, true, false, A::Redo),
            (KeyCode::KeyS, true, false, A::Save),
            (KeyCode::KeyS, true, true, A::SaveAs),
            (KeyCode::KeyS, false, false, A::ToggleSnap),
            (KeyCode::KeyN, true, false, A::NewScene),
            (KeyCode::KeyO, true, false, A::OpenScene),
            (KeyCode::KeyD, true, false, A::Duplicate),
            (KeyCode::KeyC, true, false, A::Copy),
            (KeyCode::KeyC, false, false, A::ToggleColliders),
            (KeyCode::KeyV, true, false, A::Paste),
            (KeyCode::KeyX, true, false, A::Cut),
            (KeyCode::KeyA, true, false, A::SelectAll),
            (KeyCode::KeyG, false, false, A::ToggleGrid),
            (KeyCode::Equal, false, false, A::ZoomIn),
            (KeyCode::Minus, false, false, A::ZoomOut),
            (KeyCode::Digit0, false, false, A::ResetZoom),
            (KeyCode::F2, false, false, A::RenameSelected),
            (KeyCode::F5, false, false, A::PlayResume),
            (KeyCode::KeyP, true, false, A::TogglePlayPause),
            (KeyCode::KeyP, true, true, A::StopPlay),
        ];
        for (key, ctrl, shift, expected) in table {
            assert_eq!(
                m.resolve(key, ctrl, shift),
                Some(expected),
                "chord {key:?} ctrl={ctrl} shift={shift}"
            );
        }
    }

    #[test]
    fn test_chord_specificity_same_key_three_ways() {
        let m = EditorInputMapping::new();
        // Z: bare unbound, Ctrl+Z undo, Ctrl+Shift+Z redo — all distinct
        assert_eq!(m.resolve(KeyCode::KeyZ, false, false), None);
        assert_eq!(m.resolve(KeyCode::KeyZ, true, false), Some(A::Undo));
        assert_eq!(m.resolve(KeyCode::KeyZ, true, true), Some(A::Redo));
        // The old table's classic: bare D must NOT duplicate
        assert_eq!(m.resolve(KeyCode::KeyD, false, false), None);
    }

    #[test]
    fn test_anymods_bindings_fire_under_any_modifiers() {
        let m = EditorInputMapping::new();
        // Escape and the arrows resolve regardless of held modifiers
        assert_eq!(m.resolve(KeyCode::Escape, false, false), Some(A::Cancel));
        assert_eq!(m.resolve(KeyCode::Escape, true, true), Some(A::Cancel));
        assert_eq!(m.resolve(KeyCode::ArrowLeft, false, true), Some(A::NudgeLeft));
        assert_eq!(m.resolve(KeyCode::ArrowUp, false, false), Some(A::NudgeUp));
        assert_eq!(m.resolve(KeyCode::Delete, true, false), Some(A::Delete));
    }

    #[test]
    fn test_rebind_evicts_only_the_exact_chord() {
        let mut m = EditorInputMapping::new();
        // Steal bare S for SelectAll: ToggleSnap loses it...
        m.bind(A::SelectAll, EditorBinding::key(KeyCode::KeyS));
        assert_eq!(m.resolve(KeyCode::KeyS, false, false), Some(A::SelectAll));
        assert!(m.get_bindings(A::ToggleSnap).is_empty());
        // ...but Ctrl+S (Save) and Ctrl+Shift+S (SaveAs) survive untouched
        assert_eq!(m.resolve(KeyCode::KeyS, true, false), Some(A::Save));
        assert_eq!(m.resolve(KeyCode::KeyS, true, true), Some(A::SaveAs));
    }

    #[test]
    fn test_unbind_action_clears_its_index_entries() {
        let mut m = EditorInputMapping::new();
        m.unbind(A::Undo);
        assert!(m.get_bindings(A::Undo).is_empty());
        assert_eq!(m.resolve(KeyCode::KeyZ, true, false), None);
        // Redo's Ctrl+Shift+Z entry is untouched
        assert_eq!(m.resolve(KeyCode::KeyZ, true, true), Some(A::Redo));
    }

    #[test]
    fn test_poll_methods_honor_chord_modifiers() {
        let m = EditorInputMapping::new();
        let mut input = InputHandler::new();

        // Bare S pressed: ToggleSnap active, Save not
        input.keyboard_mut().handle_key_press(KeyCode::KeyS);
        assert!(m.is_action_just_pressed(A::ToggleSnap, &input));
        assert!(!m.is_action_just_pressed(A::Save, &input));

        // Ctrl held: the same key press now satisfies Save, not ToggleSnap
        input.keyboard_mut().handle_key_press(KeyCode::ControlLeft);
        assert!(m.is_action_just_pressed(A::Save, &input));
        assert!(!m.is_action_just_pressed(A::ToggleSnap, &input));
    }

    #[test]
    fn test_pan_has_multiple_bindings() {
        let mapping = EditorInputMapping::new();
        let bindings = mapping.get_bindings(A::Pan);
        // Both Space and middle mouse
        assert!(bindings.len() >= 2);
    }
}
