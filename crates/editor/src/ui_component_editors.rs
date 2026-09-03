//! Per-component editors for the data-driven UI components
//! (`UiLabel`/`UiPanel`/`UiButton`): text/id via string inputs, anchor via a
//! cycle selector, everything else via the shared field editors. Same
//! contract as `component_editors.rs` — return `Some(ComponentEdit)` when a
//! field changed this frame.

use ecs::ui_components::{UiAnchor, UiButton, UiLabel, UiPanel};

use crate::component_editors::ComponentEdit;
use crate::field_style::EditResult;
use crate::EditableInspector;

/// Field ranges for UI element editing.
mod ranges {
    use std::ops::RangeInclusive;

    /// Anchor offsets can cross the whole window in either direction.
    pub const UI_OFFSET: RangeInclusive<f32> = -4000.0..=4000.0;
    /// Element sizes in pixels.
    pub const UI_SIZE: RangeInclusive<f32> = 1.0..=4000.0;
    /// Readable font sizes.
    pub const UI_FONT_SIZE: RangeInclusive<f32> = 6.0..=128.0;
    /// Border thickness; 0 disables the border.
    pub const UI_BORDER_WIDTH: RangeInclusive<f32> = 0.0..=20.0;
}

/// Render the anchor cycle row shared by all three UI components.
fn edit_anchor(inspector: &mut EditableInspector<'_>, anchor: UiAnchor) -> Option<UiAnchor> {
    match inspector.cycle("Anchor", anchor.label(), anchor.index(), UiAnchor::ALL.len()) {
        EditResult::Changed(index) => Some(UiAnchor::ALL[index]),
        EditResult::Unchanged => None,
    }
}

/// Edit a UiLabel component.
pub fn edit_ui_label(
    inspector: &mut EditableInspector<'_>,
    label: &UiLabel,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<UiLabel>> {
    let mut new = label.clone();
    let mut hint = None;

    inspector.header("UiLabel");

    inspector.string_edit("Text", &label.text).assign(&mut new.text, &mut hint, "text");
    if let Some(anchor) = edit_anchor(inspector, label.anchor) {
        new.anchor = anchor;
        hint = Some("anchor");
    }
    inspector.vec2("Offset", label.offset, ranges::UI_OFFSET).assign(&mut new.offset, &mut hint, "offset");
    inspector.f32("Font Size", label.font_size, ranges::UI_FONT_SIZE).assign(&mut new.font_size, &mut hint, "font_size");
    inspector.color("Color", label.color).assign(&mut new.color, &mut hint, "color");
    inspector.bool("Visible", label.visible).assign(&mut new.visible, &mut hint, "visible");

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Edit a UiPanel component.
pub fn edit_ui_panel(
    inspector: &mut EditableInspector<'_>,
    panel: &UiPanel,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<UiPanel>> {
    let mut new = panel.clone();
    let mut hint = None;

    inspector.header("UiPanel");

    if let Some(anchor) = edit_anchor(inspector, panel.anchor) {
        new.anchor = anchor;
        hint = Some("anchor");
    }
    inspector.vec2("Offset", panel.offset, ranges::UI_OFFSET).assign(&mut new.offset, &mut hint, "offset");
    inspector.vec2("Size", panel.size, ranges::UI_SIZE).assign(&mut new.size, &mut hint, "size");
    inspector.color("Background", panel.background).assign(&mut new.background, &mut hint, "background");
    inspector.color("Border", panel.border).assign(&mut new.border, &mut hint, "border");
    inspector.f32("Border Width", panel.border_width, ranges::UI_BORDER_WIDTH).assign(&mut new.border_width, &mut hint, "border_width");
    inspector.bool("Visible", panel.visible).assign(&mut new.visible, &mut hint, "visible");

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// Edit a UiButton component.
pub fn edit_ui_button(
    inspector: &mut EditableInspector<'_>,
    button: &UiButton,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<UiButton>> {
    let mut new = button.clone();
    let mut hint = None;

    inspector.header("UiButton");

    inspector.string_edit("Text", &button.text).assign(&mut new.text, &mut hint, "text");
    inspector.string_edit("Event Id", &button.id).assign(&mut new.id, &mut hint, "id");
    if let Some(anchor) = edit_anchor(inspector, button.anchor) {
        new.anchor = anchor;
        hint = Some("anchor");
    }
    inspector.vec2("Offset", button.offset, ranges::UI_OFFSET).assign(&mut new.offset, &mut hint, "offset");
    inspector.vec2("Size", button.size, ranges::UI_SIZE).assign(&mut new.size, &mut hint, "size");
    inspector.bool("Visible", button.visible).assign(&mut new.visible, &mut hint, "visible");

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{extras, frame};
    use crate::{EditableFieldStyle, FieldId};
    use input::prelude::KeyCode;
    use ui::UIContext;

    #[test]
    fn test_ui_label_text_commits_on_enter_with_the_text_hint() {
        // Text is the first field under the UiLabel header (field 0): a
        // committed edit carries the new text — `@key` strings included,
        // that is how labels localize — and the "text" hint that lets
        // consecutive keystrokes merge into one undo entry.
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();
        let mut drag_drop = crate::DragDropState::new();
        let field: ui::WidgetId = FieldId::new(0, 0, 0).into();
        ui.focus_text_input(field, "@hud.score");
        input.keyboard_mut().handle_key_press(KeyCode::Enter);

        let style = EditableFieldStyle::default();
        let edit = frame(&mut ui, &input, |ui| {
            let mut inspector = EditableInspector::new(ui, &style, 10.0, 10.0);
            edit_ui_label(&mut inspector, &UiLabel::default(), &mut extras(&mut drag_drop))
        })
        .expect("Enter commits the label text");

        assert_eq!((edit.new_value.text.as_str(), edit.field_hint), ("@hud.score", "text"));
    }
}
