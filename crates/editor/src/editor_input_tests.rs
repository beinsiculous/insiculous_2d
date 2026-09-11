use super::*;
use EditorAction as A;

/// The whole default shortcut table, event path: every chord an editor
/// user can type resolves to its action. The KeyF rows pin the two
/// framing shortcuts here so editor_integration need not keep its own
/// copy; Ctrl+F falls through to the any-mods F binding.
#[test]
fn test_every_default_chord_resolves_to_its_action() {
    let mapping = EditorInputMapping::new();
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
        (KeyCode::KeyF, false, false, A::FocusSelection),
        (KeyCode::KeyF, true, false, A::FocusSelection),
        (KeyCode::KeyF, true, true, A::ToggleCameraFollow),
        (KeyCode::Equal, false, false, A::ZoomIn),
        (KeyCode::Minus, false, false, A::ZoomOut),
        (KeyCode::Digit0, false, false, A::ResetZoom),
        (KeyCode::F2, false, false, A::RenameSelected),
        (KeyCode::F5, false, false, A::PlayResume),
        (KeyCode::KeyP, true, false, A::TogglePlayPause),
        (KeyCode::KeyP, true, true, A::StopPlay),
        // Any-mods bindings fire whatever is held.
        (KeyCode::Escape, false, false, A::Cancel),
        (KeyCode::Escape, true, true, A::Cancel),
        (KeyCode::ArrowLeft, false, true, A::NudgeLeft),
        (KeyCode::ArrowUp, false, false, A::NudgeUp),
        (KeyCode::Delete, true, false, A::Delete),
    ];
    for (key, ctrl, shift, expected) in table {
        assert_eq!(
            mapping.resolve(key, ctrl, shift),
            Some(expected),
            "chord {key:?} ctrl={ctrl} shift={shift}"
        );
    }
}

/// Exact chord wins: the same key under three modifier states is three
/// distinct chords on BOTH the event path and the poll path, and a bare
/// key bound only with Ctrl resolves to nothing.
#[test]
fn test_chord_specificity_same_key_three_ways() {
    let mapping = EditorInputMapping::new();
    assert_eq!(mapping.resolve(KeyCode::KeyZ, false, false), None, "bare Z is unbound");
    assert_eq!(mapping.resolve(KeyCode::KeyZ, true, false), Some(A::Undo));
    assert_eq!(mapping.resolve(KeyCode::KeyZ, true, true), Some(A::Redo));
    assert_eq!(mapping.resolve(KeyCode::KeyD, false, false), None, "bare D must not duplicate");

    // Poll path: bare S is ToggleSnap; with Ctrl held the same key is Save.
    let mut input = InputHandler::new();
    input.keyboard_mut().handle_key_press(KeyCode::KeyS);
    assert!(mapping.is_action_just_pressed(A::ToggleSnap, &input));
    assert!(!mapping.is_action_just_pressed(A::Save, &input));
    input.keyboard_mut().handle_key_press(KeyCode::ControlLeft);
    assert!(mapping.is_action_just_pressed(A::Save, &input));
    assert!(!mapping.is_action_just_pressed(A::ToggleSnap, &input));
}

/// Rebinding evicts only the exact (key, ctrl, shift) tuple, and
/// unbinding an action clears only its own chords.
#[test]
fn test_rebind_evicts_only_the_exact_chord() {
    let mut mapping = EditorInputMapping::new();

    // Steal bare S for SelectAll: ToggleSnap loses it...
    mapping.bind(A::SelectAll, EditorBinding::key(KeyCode::KeyS));
    assert_eq!(mapping.resolve(KeyCode::KeyS, false, false), Some(A::SelectAll));
    assert_eq!(mapping.get_bindings(A::ToggleSnap), &[], "the evicted action has no chord left");
    // ...but Ctrl+S (Save) and Ctrl+Shift+S (SaveAs) survive untouched.
    assert_eq!(mapping.resolve(KeyCode::KeyS, true, false), Some(A::Save));
    assert_eq!(mapping.resolve(KeyCode::KeyS, true, true), Some(A::SaveAs));

    // Unbinding Undo leaves Redo's Ctrl+Shift+Z entry alone.
    mapping.unbind(A::Undo);
    assert_eq!(mapping.get_bindings(A::Undo), &[]);
    assert_eq!(mapping.resolve(KeyCode::KeyZ, true, false), None);
    assert_eq!(mapping.resolve(KeyCode::KeyZ, true, true), Some(A::Redo));
}

/// Pan is reachable two ways — Space-hold and the middle mouse button —
/// and nothing else.
#[test]
fn test_pan_is_bound_to_space_and_the_middle_mouse_button() {
    let mapping = EditorInputMapping::new();
    assert_eq!(
        mapping.get_bindings(A::Pan),
        &[EditorBinding::KeyAnyMods(KeyCode::Space), EditorBinding::Mouse(MouseButton::Middle)]
    );
}

#[test]
fn test_modifiers_read_detects_left_and_right_ctrl_and_shift() {
    let mut input = InputHandler::new();
    assert_eq!(Modifiers::read(&input), Modifiers { ctrl: false, shift: false });

    input.keyboard_mut().handle_key_press(KeyCode::ControlLeft);
    assert_eq!(Modifiers::read(&input), Modifiers { ctrl: true, shift: false });

    input.keyboard_mut().handle_key_release(KeyCode::ControlLeft);
    input.keyboard_mut().handle_key_press(KeyCode::ControlRight);
    assert_eq!(Modifiers::read(&input), Modifiers { ctrl: true, shift: false });

    input.keyboard_mut().handle_key_press(KeyCode::ShiftLeft);
    assert_eq!(Modifiers::read(&input), Modifiers { ctrl: true, shift: true });

    input.keyboard_mut().handle_key_release(KeyCode::ControlRight);
    assert_eq!(Modifiers::read(&input), Modifiers { ctrl: false, shift: true });

    input.keyboard_mut().handle_key_release(KeyCode::ShiftLeft);
    input.keyboard_mut().handle_key_press(KeyCode::ShiftRight);
    assert_eq!(Modifiers::read(&input), Modifiers { ctrl: false, shift: true });
}
