//! Source-frame and border editing for sprites that repeat without scaling their pixels.

use ecs::SpriteTiling;

use super::ComponentEdit;
use crate::{EditableInspector, EditResult};

/// Edit the source dimensions and fixed margins; the Transform2D gizmo owns the destination size.
pub fn edit_sprite_tiling(
    inspector: &mut EditableInspector<'_>,
    tiling: &SpriteTiling,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<SpriteTiling>> {
    let mut new = *tiling;
    let mut hint = None;
    inspector.header("SpriteTiling");
    inspector.vec2("Source Pixels", tiling.source_size, 1.0..=1024.0)
        .assign(&mut new.source_size, &mut hint, "source_size");
    for (index, label) in ["Left Pixels", "Top Pixels", "Right Pixels", "Bottom Pixels"].into_iter().enumerate() {
        if let EditResult::Changed(value) = inspector.f32(label, new.borders[index], 0.0..=1024.0) {
            let maximum = (new.source_size[index % 2] - new.borders[(index + 2) % 4] - 1.0).max(0.0);
            new.borders[index] = value.clamp(0.0, maximum);
            hint = Some("borders");
        }
    }
    new.normalize();
    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{extras, frame};
    use crate::{DragDropState, EditableFieldStyle, FieldId};
    use input::prelude::KeyCode;

    #[test]
    fn typed_border_pixels_keep_a_repeatable_interior() {
        let mut ui = ui::UIContext::new();
        let mut drag_drop = DragDropState::new();
        let mut inspector_state = crate::InspectorState::default();
        let mut input = input::InputHandler::new();
        let field: ui::WidgetId = FieldId::new(0, 1, 0).into();
        ui.focus_text_input(field, "1000");
        input.keyboard_mut().handle_key_press(KeyCode::Enter);
        let style = EditableFieldStyle::default();
        let source = SpriteTiling {
            source_size: glam::Vec2::new(96.0, 192.0),
            borders: glam::Vec4::new(48.0, 80.0, 16.0, 80.0),
        };
        let edit = frame(&mut ui, &input, |ui| {
            let mut inspector = EditableInspector::new(ui, &style, 10.0, 10.0);
            edit_sprite_tiling(&mut inspector, &source, &mut extras(&mut drag_drop, &mut inspector_state))
        }).expect("Enter commits");
        assert_eq!(edit.field_hint, "borders");
        assert!(edit.new_value.is_valid());
        assert_eq!(edit.new_value.borders.x, 79.0);
        assert_eq!(edit.new_value.borders.z, 16.0, "editing left must preserve the right border");
    }
}
