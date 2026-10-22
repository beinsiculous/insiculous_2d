//! The composer's key table, read as data: every row resolves to its own action under
//! exactly its modifiers, no two rows share a chord, and what the table lacks either goes
//! on to the editor (a Ctrl chord, F5) or is swallowed.

use std::collections::HashSet;

use winit::keyboard::KeyCode;

use super::keys::{resolve, ComposerKeyAction as Action, ComposerKeyRoute as Route, Direction, COMPOSER_KEYS};
use crate::editor_input::Modifiers;

fn with(ctrl: bool, shift: bool) -> Modifiers {
    Modifiers { ctrl, shift }
}

#[test]
fn every_row_resolves_to_its_action_and_no_chord_is_bound_twice() {
    let mut chords = HashSet::new();
    for row in COMPOSER_KEYS {
        assert!(chords.insert((row.key, row.ctrl, row.shift)), "{:?} is bound twice", row.key);
        assert_eq!(resolve(row.key, with(row.ctrl, row.shift)), Route::Composer(row.action));
    }
}

#[test]
fn the_table_says_what_each_key_does_while_the_composer_holds_the_keyboard() {
    let plain = with(false, false);
    let shift = with(false, true);
    let ctrl = with(true, false);
    let cases = [
        (KeyCode::Escape, plain, Action::Escape),
        (KeyCode::Tab, plain, Action::NextArea),
        (KeyCode::Tab, shift, Action::PreviousArea),
        (KeyCode::ArrowLeft, plain, Action::MoveCursor(Direction::Left)),
        (KeyCode::ArrowUp, shift, Action::Nudge(Direction::Up)),
        (KeyCode::Enter, plain, Action::Activate),
        (KeyCode::Insert, plain, Action::Activate),
        (KeyCode::Delete, plain, Action::DeleteNotes),
        (KeyCode::BracketLeft, plain, Action::Shorten),
        (KeyCode::BracketRight, plain, Action::Lengthen),
        (KeyCode::Equal, shift, Action::VelocityUp),
        (KeyCode::Minus, plain, Action::VelocityDown),
        (KeyCode::KeyZ, ctrl, Action::Undo),
        (KeyCode::KeyZ, with(true, true), Action::Redo),
        (KeyCode::KeyY, ctrl, Action::Redo),
        (KeyCode::KeyA, ctrl, Action::SelectAll),
        (KeyCode::Space, plain, Action::PreviewToggle),
    ];
    for (key, modifiers, action) in cases {
        assert_eq!(resolve(key, modifiers), Route::Composer(action), "{key:?} {modifiers:?}");
    }
}

#[test]
fn a_key_the_table_lacks_goes_to_the_editor_with_ctrl_or_as_f5_and_is_swallowed_otherwise() {
    assert_eq!(resolve(KeyCode::KeyS, with(true, false)), Route::Editor, "Ctrl+S saves the scene");
    assert_eq!(resolve(KeyCode::KeyM, with(true, true)), Route::Editor);
    assert_eq!(resolve(KeyCode::F5, with(false, false)), Route::Editor, "F5 still plays");
    for toggle in [KeyCode::KeyG, KeyCode::KeyC, KeyCode::KeyS, KeyCode::KeyQ, KeyCode::KeyW] {
        assert_eq!(
            resolve(toggle, with(false, false)),
            Route::Swallowed,
            "{toggle:?}: a single-key editor toggle is unreachable while the composer is focused"
        );
    }
}
