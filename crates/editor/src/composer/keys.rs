//! The composer's keys, as one table: what each key does while the composer holds the
//! keyboard. The host consults it before the editor's own table, so while the composer is
//! focused these keys never reach the scene or the game; a key the table lacks that carries
//! Ctrl (Ctrl+S, Ctrl+C …) and F5 go on to the editor as before, and every other key is
//! swallowed — the single-key editor toggles (G, C, S, Q/W/E/R) are unreachable while the
//! composer holds the keyboard, deliberately, since the composer's own keys reuse them.

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

/// What a composer key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerKeyAction {
    /// Drop a drag in flight, else drop the keyboard.
    Escape,
    NextArea,
    PreviousArea,
    /// Move the focused area's cursor: the roll's position, the list's row, the transport's
    /// or the sidebar's control.
    MoveCursor(Direction),
    /// Move the selected notes a sixteenth in time or a semitone in pitch; a held key is one
    /// undo entry.
    Nudge(Direction),
    /// Add a note at the roll's cursor on the active track; in the transport or the
    /// sidebar, act on the control at the cursor.
    Activate,
    DeleteNotes,
    Shorten,
    Lengthen,
    VelocityUp,
    VelocityDown,
    Undo,
    Redo,
    SelectAll,
    /// Start or stop the song's preview.
    PreviewToggle,
}

/// One row of the table: a key with its Ctrl and Shift state, exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposerKeyBinding {
    pub key: KeyCode,
    pub ctrl: bool,
    pub shift: bool,
    pub action: ComposerKeyAction,
}

const fn bind(key: KeyCode, ctrl: bool, shift: bool, action: ComposerKeyAction) -> ComposerKeyBinding {
    ComposerKeyBinding { key, ctrl, shift, action }
}

use ComposerKeyAction as Action;
use Direction::{Down, Left, Right, Up};

/// See the module docs. `+` is Shift+`=` on most layouts, so both states of `=` raise the
/// velocity; Backspace deletes as Delete does, as in the scene.
pub const COMPOSER_KEYS: &[ComposerKeyBinding] = &[
    bind(KeyCode::Escape, false, false, Action::Escape),
    bind(KeyCode::Tab, false, false, Action::NextArea),
    bind(KeyCode::Tab, false, true, Action::PreviousArea),
    bind(KeyCode::ArrowLeft, false, false, Action::MoveCursor(Left)),
    bind(KeyCode::ArrowRight, false, false, Action::MoveCursor(Right)),
    bind(KeyCode::ArrowUp, false, false, Action::MoveCursor(Up)),
    bind(KeyCode::ArrowDown, false, false, Action::MoveCursor(Down)),
    bind(KeyCode::ArrowLeft, false, true, Action::Nudge(Left)),
    bind(KeyCode::ArrowRight, false, true, Action::Nudge(Right)),
    bind(KeyCode::ArrowUp, false, true, Action::Nudge(Up)),
    bind(KeyCode::ArrowDown, false, true, Action::Nudge(Down)),
    bind(KeyCode::Enter, false, false, Action::Activate),
    bind(KeyCode::NumpadEnter, false, false, Action::Activate),
    bind(KeyCode::Insert, false, false, Action::Activate),
    bind(KeyCode::Delete, false, false, Action::DeleteNotes),
    bind(KeyCode::Backspace, false, false, Action::DeleteNotes),
    bind(KeyCode::BracketLeft, false, false, Action::Shorten),
    bind(KeyCode::BracketRight, false, false, Action::Lengthen),
    bind(KeyCode::Equal, false, false, Action::VelocityUp),
    bind(KeyCode::Equal, false, true, Action::VelocityUp),
    bind(KeyCode::NumpadAdd, false, false, Action::VelocityUp),
    bind(KeyCode::Minus, false, false, Action::VelocityDown),
    bind(KeyCode::NumpadSubtract, false, false, Action::VelocityDown),
    bind(KeyCode::KeyZ, true, false, Action::Undo),
    bind(KeyCode::KeyZ, true, true, Action::Redo),
    bind(KeyCode::KeyY, true, false, Action::Redo),
    bind(KeyCode::KeyA, true, false, Action::SelectAll),
    bind(KeyCode::Space, false, false, Action::PreviewToggle),
];

/// What the composer does with `key`, when it holds the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerKeyRoute {
    /// A row of the table.
    Composer(ComposerKeyAction),
    /// Not the composer's: the editor's own table resolves it (a Ctrl chord, F5).
    Editor,
    /// Not the composer's and not let through: nothing happens.
    Swallowed,
}

/// Look `key` up under `modifiers`, exactly.
pub fn resolve(key: KeyCode, modifiers: Modifiers) -> ComposerKeyRoute {
    let row = COMPOSER_KEYS.iter().find(|binding| {
        binding.key == key && binding.ctrl == modifiers.ctrl && binding.shift == modifiers.shift
    });
    match row {
        Some(binding) => ComposerKeyRoute::Composer(binding.action),
        None if modifiers.ctrl || key == KeyCode::F5 => ComposerKeyRoute::Editor,
        None => ComposerKeyRoute::Swallowed,
    }
}
