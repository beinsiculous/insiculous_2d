//! Input mapping system for binding input sources to game-defined actions.
//!
//! [`InputMapping`] maps arbitrary `Copy + Eq + Hash` action types to bound
//! [`InputSource`]s, evaluated against an [`InputHandler`]'s device state.
//! Games define their own action enums; a new mapping is empty, and the
//! engine's [`GameAction`] preset comes from [`InputMapping::with_default_bindings`].
//!
//! ```
//! use input::{InputMapping, InputSource, InputHandler};
//! use winit::keyboard::KeyCode;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! enum MyAction {
//!     Jump,
//!     Shoot,
//! }
//!
//! let mut actions = InputMapping::new();
//! actions.bind(MyAction::Jump, InputSource::Keyboard(KeyCode::Space));
//! actions.bind(MyAction::Jump, InputSource::Keyboard(KeyCode::KeyW));
//!
//! let input = InputHandler::new();
//! assert!(!actions.is_active(MyAction::Jump, &input));
//! ```

use crate::gamepad::{AxisDirection, GamepadAxis, GamepadButton};
use crate::input_handler::InputHandler;
use crate::pad_layout::STANDARD_PAD_LAYOUT;
use std::collections::HashMap;
use std::hash::Hash;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// How far an analog axis must travel (after dead-zone normalization) before
/// an axis-bound action counts as "pressed". Fixed engine-wide; per-binding
/// thresholds are future work.
pub const AXIS_ACTIVATION_THRESHOLD: f32 = 0.5;

/// Represents different types of input sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum InputSource {
    /// Keyboard key
    Keyboard(KeyCode),
    /// Mouse button
    Mouse(MouseButton),
    /// Gamepad button (with gamepad ID)
    Gamepad(u32, GamepadButton),
    /// Gamepad analog axis used as a digital input (with gamepad ID).
    /// Active while the axis is past [`AXIS_ACTIVATION_THRESHOLD`] in the
    /// given direction.
    GamepadAxis(u32, GamepadAxis, AxisDirection),
}

/// Built-in action preset for the engine's data-driven behaviors and demos.
///
/// This is **optional** — games are encouraged to define their own action
/// enums and use them with [`InputMapping`] directly. The engine uses this
/// preset for scene-defined behaviors (e.g. `PlayerControlled` movement),
/// bound via [`InputMapping::with_default_bindings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GameAction {
    /// Movement actions
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    /// Action buttons
    Action1, // Typically primary action (e.g., A button, Space key)
    Action2, // Typically secondary action (e.g., B button, Enter key)
    Action3, // Typically tertiary action (e.g., X button, Shift key)
    Action4, // Typically quaternary action (e.g., Y button, Ctrl key)
    /// UI actions
    Menu,
    Cancel,
    Select,
    /// Custom action with ID
    Custom(u32),
}

/// Maps game-defined actions to the input sources that trigger them.
///
/// See the [module documentation](self) for the binding model and semantics.
#[derive(Debug, Clone)]
pub struct InputMapping<A: Copy + Eq + Hash, S: Copy + Eq + Hash = InputSource> {
    /// Action → bound input sources (single source of truth)
    bindings: HashMap<A, Vec<S>>,
}

impl<A: Copy + Eq + Hash, S: Copy + Eq + Hash> Default for InputMapping<A, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Copy + Eq + Hash, S: Copy + Eq + Hash> InputMapping<A, S> {
    /// Create a new, empty input mapping (no implicit default bindings)
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Bind an input source to an action.
    ///
    /// An action can have multiple sources, and a source can be bound to
    /// multiple actions. Binding the same (action, source) pair twice is a no-op.
    /// Returns true when a new binding was inserted.
    pub fn bind(&mut self, action: A, source: S) -> bool {
        let sources = self.bindings.entry(action).or_default();
        if !sources.contains(&source) {
            sources.push(source);
            true
        } else {
            false
        }
    }

    /// Remove one source from an action's bindings.
    ///
    /// Returns true when an existing binding was removed.
    pub fn unbind(&mut self, action: A, source: &S) -> bool {
        if let Some(sources) = self.bindings.get_mut(&action) {
            let before = sources.len();
            sources.retain(|s| s != source);
            let removed = sources.len() != before;
            if sources.is_empty() {
                self.bindings.remove(&action);
            }
            removed
        } else {
            false
        }
    }

    /// Get all input sources bound to an action
    pub fn bindings(&self, action: A) -> &[S] {
        self.bindings
            .get(&action)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if the mapping has no bindings at all
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Iterate over all (action, sources) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (A, &[S])> {
        self.bindings.iter().map(|(a, s)| (*a, s.as_slice()))
    }
}

impl<A: Copy + Eq + Hash> InputMapping<A, InputSource> {
    // ================== Action State Evaluation ==================

    /// Check if an action is currently active (any bound source is pressed)
    pub fn is_active(&self, action: A, input: &InputHandler) -> bool {
        self.bindings(action)
            .iter()
            .any(|source| input.is_source_pressed(source))
    }

    /// Check if an action became active this frame.
    ///
    /// Returns `false` if the action was already active last frame (e.g.
    /// pressing W while ArrowUp is held does not re-trigger MoveUp).
    pub fn just_activated(&self, action: A, input: &InputHandler) -> bool {
        self.bindings(action)
            .iter()
            .any(|source| input.is_source_just_pressed(source))
            && !self.was_active(action, input)
    }

    /// Whether the action was active on the previous frame.
    fn was_active(&self, action: A, input: &InputHandler) -> bool {
        self.bindings(action)
            .iter()
            .any(|source| source_was_pressed(source, input))
    }
}

/// Whether a source was pressed on the previous frame. Shared by [`InputMapping`]
/// and the player-aware settings layer so edge semantics never diverge.
pub(crate) fn source_was_pressed(source: &InputSource, input: &InputHandler) -> bool {
    input.was_source_pressed(source)
}

impl InputMapping<GameAction, InputSource> {
    /// Create a mapping pre-populated with the engine's default [`GameAction`]
    /// bindings (WASD + arrows movement, Space/Enter/Shift/Ctrl actions,
    /// Escape menu, Tab select, gamepad 0 equivalents).
    pub fn with_default_bindings() -> Self {
        let mut mapping = Self::new();

        // Movement (keyboard)
        mapping.bind(GameAction::MoveUp, InputSource::Keyboard(KeyCode::KeyW));
        mapping.bind(GameAction::MoveUp, InputSource::Keyboard(KeyCode::ArrowUp));
        mapping.bind(GameAction::MoveDown, InputSource::Keyboard(KeyCode::KeyS));
        mapping.bind(GameAction::MoveDown, InputSource::Keyboard(KeyCode::ArrowDown));
        mapping.bind(GameAction::MoveLeft, InputSource::Keyboard(KeyCode::KeyA));
        mapping.bind(GameAction::MoveLeft, InputSource::Keyboard(KeyCode::ArrowLeft));
        mapping.bind(GameAction::MoveRight, InputSource::Keyboard(KeyCode::KeyD));
        mapping.bind(GameAction::MoveRight, InputSource::Keyboard(KeyCode::ArrowRight));

        // Actions (keyboard / mouse)
        mapping.bind(GameAction::Action1, InputSource::Keyboard(KeyCode::Space));
        mapping.bind(GameAction::Action1, InputSource::Mouse(MouseButton::Left));

        mapping.bind(GameAction::Action2, InputSource::Keyboard(KeyCode::Enter));
        mapping.bind(GameAction::Action2, InputSource::Mouse(MouseButton::Right));

        mapping.bind(GameAction::Action3, InputSource::Keyboard(KeyCode::ShiftLeft));

        mapping.bind(GameAction::Action4, InputSource::Keyboard(KeyCode::ControlLeft));

        // UI (keyboard)
        mapping.bind(GameAction::Menu, InputSource::Keyboard(KeyCode::Escape));

        mapping.bind(GameAction::Select, InputSource::Keyboard(KeyCode::Tab));

        // Gamepad 0 movement and actions via the standard pad layout
        for &(action, source) in STANDARD_PAD_LAYOUT {
            mapping.bind(action, source.on_pad(0));
        }

        mapping
    }
}
