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

    if let EditResult::Changed(v) = inspector.string_edit("Text", &label.text) {
        new.text = v;
        hint = Some("text");
    }
    if let Some(anchor) = edit_anchor(inspector, label.anchor) {
        new.anchor = anchor;
        hint = Some("anchor");
    }
    if let EditResult::Changed(v) = inspector.vec2("Offset", label.offset, ranges::UI_OFFSET) {
        new.offset = v;
        hint = Some("offset");
    }
    if let EditResult::Changed(v) = inspector.f32("Font Size", label.font_size, ranges::UI_FONT_SIZE)
    {
        new.font_size = v;
        hint = Some("font_size");
    }
    if let EditResult::Changed(v) = inspector.color("Color", label.color) {
        new.color = v;
        hint = Some("color");
    }
    if let EditResult::Changed(v) = inspector.bool("Visible", label.visible) {
        new.visible = v;
        hint = Some("visible");
    }

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
    if let EditResult::Changed(v) = inspector.vec2("Offset", panel.offset, ranges::UI_OFFSET) {
        new.offset = v;
        hint = Some("offset");
    }
    if let EditResult::Changed(v) = inspector.vec2("Size", panel.size, ranges::UI_SIZE) {
        new.size = v;
        hint = Some("size");
    }
    if let EditResult::Changed(v) = inspector.color("Background", panel.background) {
        new.background = v;
        hint = Some("background");
    }
    if let EditResult::Changed(v) = inspector.color("Border", panel.border) {
        new.border = v;
        hint = Some("border");
    }
    if let EditResult::Changed(v) =
        inspector.f32("Border Width", panel.border_width, ranges::UI_BORDER_WIDTH)
    {
        new.border_width = v;
        hint = Some("border_width");
    }
    if let EditResult::Changed(v) = inspector.bool("Visible", panel.visible) {
        new.visible = v;
        hint = Some("visible");
    }

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

    if let EditResult::Changed(v) = inspector.string_edit("Text", &button.text) {
        new.text = v;
        hint = Some("text");
    }
    if let EditResult::Changed(v) = inspector.string_edit("Event Id", &button.id) {
        new.id = v;
        hint = Some("id");
    }
    if let Some(anchor) = edit_anchor(inspector, button.anchor) {
        new.anchor = anchor;
        hint = Some("anchor");
    }
    if let EditResult::Changed(v) = inspector.vec2("Offset", button.offset, ranges::UI_OFFSET) {
        new.offset = v;
        hint = Some("offset");
    }
    if let EditResult::Changed(v) = inspector.vec2("Size", button.size, ranges::UI_SIZE) {
        new.size = v;
        hint = Some("size");
    }
    if let EditResult::Changed(v) = inspector.bool("Visible", button.visible) {
        new.visible = v;
        hint = Some("visible");
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui::UIContext;

    #[test]
    fn test_ui_editors_render_without_input_and_report_no_edit() {
        let mut ui = UIContext::new();
        ui.begin_frame(&input::InputHandler::new(), glam::Vec2::new(800.0, 600.0));

        let mut drag_drop = crate::DragDropState::new();
        let mut extras =
            crate::InspectorExtras { drag_drop: &mut drag_drop, texture_display: None, warnings: Vec::new() };

        let mut inspector = EditableInspector::new(&mut ui, 10.0, 10.0);
        assert!(edit_ui_label(&mut inspector, &UiLabel::default(), &mut extras).is_none());
        let y_after_label = inspector.y();
        assert!(y_after_label > 10.0, "label editor must advance the cursor");

        let mut inspector = EditableInspector::new(&mut ui, 10.0, y_after_label);
        assert!(edit_ui_panel(&mut inspector, &UiPanel::default(), &mut extras).is_none());

        let mut inspector = EditableInspector::new(&mut ui, 10.0, 400.0);
        assert!(edit_ui_button(&mut inspector, &UiButton::default(), &mut extras).is_none());

        ui.end_frame();
    }
}
