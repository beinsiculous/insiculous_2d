//! The colour editor's frame pass.
//!
//! It runs before any panel renders, because a blocking rect is consulted
//! at each widget's own `interact` call: a popup pushed part-way through
//! the inspector's walk cannot make the rows already drawn above it inert,
//! and a drag on one of its channels would scrub whatever row lay
//! underneath. On the Modal band, drawn first, its rect covers every widget
//! of the frame — including a panel that is itself a Floating overlay in
//! the dock's narrow mode.

use editor::EditorContext;
use ui::UIContext;

/// Whether one of the popup's own fields — four channels and the hex
/// field — holds the keyboard.
fn popup_owns_focus(ui: &UIContext) -> bool {
    (0..=4).any(|channel| ui.is_focused(editor::color_editor_channel_id(channel)))
}

/// Draw the colour editor when a colour row has one open, and carry that
/// frame's edit back to the row, which writes it as its own later in the
/// same frame.
///
/// A row that has not drawn since the last pass — its entity deselected,
/// its section collapsed, its component undone away, its panel hidden —
/// takes the editor with it.
pub(crate) fn render_color_editor_pass(
    editor: &mut EditorContext,
    ui: &mut UIContext,
    dialog_up: bool,
) {
    let Some((anchor, value)) = editor.inspector_state.color_editor_frame() else {
        // Closed elsewhere — an entity switch, Play, the Escape cascade —
        // with a field of the popup still focused and no longer drawn.
        if popup_owns_focus(ui) {
            ui.clear_text_focus();
        }
        return;
    };
    // A confirm dialog owns the frame: the popup would draw over its scrim
    // on the same band and take the presses meant for its buttons. It is
    // discarded, not suspended — every channel edit it made is already in
    // the world, and only unentered hex text goes with it.
    if dialog_up
        || editor.inspector_state.color_editor_closing()
        || !editor.inspector_state.color_editor_row_was_seen()
    {
        close(editor, ui);
        return;
    }

    let style = editor
        .theme
        .editable_field_style()
        .with_numeric_font(editor.fonts.mono);
    let frame = editor::render_color_editor(ui, value, anchor, &style);

    if let Some(typed) = frame.invalid_hex {
        editor
            .status_bar
            .show_message(format!("Not a colour: {typed} — use #rrggbb or #rrggbbaa"));
    }
    let edited = matches!(frame.result, editor::EditResult::Changed(_));
    if let editor::EditResult::Changed(edited) = frame.result {
        editor.inspector_state.set_color_edit(edited);
    }
    match (frame.close, edited) {
        // A click outside commits a focused field on that same press; the
        // row takes the edit in the walk that follows, and the next pass
        // closes.
        (true, true) => editor.inspector_state.close_color_editor_after_edit(),
        (true, false) => close(editor, ui),
        (false, _) => editor.inspector_state.clear_color_editor_seen(),
    }
}

/// Close the editor and release the keyboard when one of the popup's own
/// fields holds it: undrawn, that field would keep every key with nothing
/// left to take the Escape. Any other field's focus — the one the
/// click-away landed on, a rename in progress — is left alone.
fn close(editor: &mut EditorContext, ui: &mut UIContext) {
    editor.inspector_state.close_color_editor();
    if popup_owns_focus(ui) {
        ui.clear_text_focus();
    }
}
