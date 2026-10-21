//! Inspector sound field: shows a `SoundEmitter`'s recorded sound by its path and takes a
//! sound dropped from the asset browser — the twin of the texture field.

use ui::UIContext;

use crate::drag_drop::{DragDropState, DragPayload};
use crate::editable_inspector::draw_field_label;
use crate::field_style::{EditResult, EditableFieldStyle};
use crate::row_layout::RowLayout;

/// What the slot shows for an emitter with no sound yet.
const NO_SOUND: &str = "(drop a sound)";

/// Render a sound slot: label + a boxed value showing `path`, acting as a drop target for
/// [`DragPayload::Sound`]. Returns `Changed(path)` when a drop of another sound lands.
pub fn edit_sound_field(
    ui: &mut UIContext,
    label: &str,
    path: &str,
    drag_drop: &mut DragDropState,
    layout: RowLayout,
    style: &EditableFieldStyle,
) -> EditResult<String> {
    draw_field_label(ui, label, &layout, style);

    let slot_bounds = ui::Rect::new(
        layout.control_x,
        layout.pos.y + 2.0,
        layout.clamp_width(style.input_width + 40.0),
        style.field_height(),
    );

    let dragging_sound = matches!(drag_drop.dragging_payload(), Some(DragPayload::Sound { .. }));
    let hovered = slot_bounds.contains(ui.mouse_pos());
    ui.rect_rounded(slot_bounds, style.slot_bg, 2.0);
    if dragging_sound && hovered {
        ui.rect_border(slot_bounds, style.drop_highlight, 2.0, 2.0);
    }

    let text = if path.is_empty() { NO_SOUND } else { path };
    ui.label_in_bounds_styled(
        text,
        slot_bounds,
        ui::TextAlign::Left,
        style.value_color,
        style.label_font,
        4.0,
    );

    let drop_bounds = common::Rect::new(slot_bounds.x, slot_bounds.y, slot_bounds.width, slot_bounds.height);
    if let Some((DragPayload::Sound { path: dropped }, _)) = drag_drop.take_drop_in(drop_bounds) {
        if dropped != path {
            return EditResult::Changed(dropped);
        }
    }
    EditResult::Unchanged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row_layout::field_row;
    use crate::test_support::frame;
    use glam::Vec2;

    fn drop_at(drag_drop: &mut DragDropState, payload: DragPayload, pos: Vec2) {
        drag_drop.arm(payload, Vec2::ZERO);
        drag_drop.begin_frame(Vec2::new(50.0, 50.0), true, false);
        drag_drop.begin_frame(pos, false, true);
    }

    #[test]
    fn a_sound_dropped_in_the_slot_assigns_its_path_and_a_texture_or_the_same_sound_does_not() {
        let style = EditableFieldStyle::default();
        let slot_center = Vec2::new(10.0 + style.label_width + 20.0, 12.0 + 8.0);
        let sound = |path: &str| DragPayload::Sound { path: path.into() };
        let cases = [
            (sound("sounds/jump.wav"), slot_center, EditResult::Changed("sounds/jump.wav".to_string())),
            (sound("sounds/jump.wav"), Vec2::new(700.0, 500.0), EditResult::Unchanged),
            (sound("sounds/beep.wav"), slot_center, EditResult::Unchanged),
            (DragPayload::Texture { handle: 3, path: "tex.png".into() }, slot_center, EditResult::Unchanged),
        ];
        for (payload, at, expected) in cases {
            let mut ui = UIContext::new();
            let input = input::InputHandler::new();
            let mut drag_drop = DragDropState::new();
            drop_at(&mut drag_drop, payload.clone(), at);

            let result = frame(&mut ui, &input, |ui| {
                let layout = field_row(Vec2::new(10.0, 10.0), 10.0, 400.0, &style);
                edit_sound_field(ui, "Sound", "sounds/beep.wav", &mut drag_drop, layout, &style)
            });

            assert_eq!(result, expected, "dropping {payload:?} at {at}");
        }
    }
}
