//! The Conductor panel's keys, as one table: what each key does while the Conductor panel
//! holds the keyboard. The host consults it before the editor's own table, so while the
//! Conductor panel is focused these keys never reach the scene or the game; a key the table
//! lacks that carries Ctrl (Ctrl+S, Ctrl+C …) and F5 go on to the editor as before, and every
//! other key is swallowed — the single-key editor toggles (G, C, S, Q/W/E/R) are unreachable
//! while the Conductor panel holds the keyboard, deliberately, so a key meant for the
//! Conductor panel never moves the scene.

use winit::keyboard::KeyCode;

use crate::editor_input::Modifiers;

/// A direction an arrow key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// What a Conductor panel key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorKeyAction {
    /// Drop the keyboard.
    Escape,
    NextArea,
    PreviousArea,
    /// Move the focused area's cursor: the transport's or the sidebar's control.
    MoveCursor(Direction),
    /// Act on the control at the cursor.
    Activate,
    Undo,
    Redo,
    /// Start or stop the song's preview.
    PreviewToggle,
}

/// One row of the table: a key with its Ctrl and Shift state, exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConductorKeyBinding {
    pub key: KeyCode,
    pub ctrl: bool,
    pub shift: bool,
    pub action: ConductorKeyAction,
}

const fn bind(key: KeyCode, ctrl: bool, shift: bool, action: ConductorKeyAction) -> ConductorKeyBinding {
    ConductorKeyBinding { key, ctrl, shift, action }
}

use ConductorKeyAction as Action;
use Direction::{Down, Left, Right, Up};

/// See the module docs.
pub const CONDUCTOR_KEYS: &[ConductorKeyBinding] = &[
    bind(KeyCode::Escape, false, false, Action::Escape),
    bind(KeyCode::Tab, false, false, Action::NextArea),
    bind(KeyCode::Tab, false, true, Action::PreviousArea),
    bind(KeyCode::ArrowLeft, false, false, Action::MoveCursor(Left)),
    bind(KeyCode::ArrowRight, false, false, Action::MoveCursor(Right)),
    bind(KeyCode::ArrowUp, false, false, Action::MoveCursor(Up)),
    bind(KeyCode::ArrowDown, false, false, Action::MoveCursor(Down)),
    bind(KeyCode::Enter, false, false, Action::Activate),
    bind(KeyCode::NumpadEnter, false, false, Action::Activate),
    bind(KeyCode::Insert, false, false, Action::Activate),
    bind(KeyCode::KeyZ, true, false, Action::Undo),
    bind(KeyCode::KeyZ, true, true, Action::Redo),
    bind(KeyCode::KeyY, true, false, Action::Redo),
    bind(KeyCode::Space, false, false, Action::PreviewToggle),
];

/// What the Conductor panel does with `key`, when it holds the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductorKeyRoute {
    /// A row of the table.
    Conductor(ConductorKeyAction),
    /// Not the Conductor panel's: the editor's own table resolves it (a Ctrl chord, F5).
    Editor,
    /// Not the Conductor panel's and not let through: nothing happens.
    Swallowed,
}

/// Look `key` up under `modifiers`, exactly.
pub fn resolve(key: KeyCode, modifiers: Modifiers) -> ConductorKeyRoute {
    let row = CONDUCTOR_KEYS.iter().find(|binding| {
        binding.key == key && binding.ctrl == modifiers.ctrl && binding.shift == modifiers.shift
    });
    match row {
        Some(binding) => ConductorKeyRoute::Conductor(binding.action),
        None if modifiers.ctrl || key == KeyCode::F5 => ConductorKeyRoute::Editor,
        None => ConductorKeyRoute::Swallowed,
    }
}
