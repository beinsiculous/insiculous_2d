//! Inspector editor for `GridBackdrop`: topology cycle, dimensions
//! that snap through the same rule the engine builds with (an odd hex
//! column count shows as the even count that renders), and the simulation
//! tunables as soft-range rows.

use std::ops::RangeInclusive;

use ecs::{GridBackdrop, GridTopology};

use crate::component_editors::ComponentEdit;
use crate::{EditResult, EditableInspector};

/// Field ranges (soft unless noted).
mod ranges {
    use super::RangeInclusive;
    use ecs::GridBackdrop;

    /// Hard: node counts per axis.
    pub const DIMENSION: RangeInclusive<f32> = 2.0..=(GridBackdrop::MAX_DIMENSION as f32);
    /// Hard: hexagon side / lattice pitch in world units.
    pub const SPACING: RangeInclusive<f32> = 1.0..=200.0;
    pub const EMISSIVE: RangeInclusive<f32> = 0.0..=4.0;
    pub const STIFFNESS: RangeInclusive<f32> = 0.0..=200.0;
    pub const DAMPING: RangeInclusive<f32> = 0.0..=1.0;
    pub const REST_PULL: RangeInclusive<f32> = 0.0..=20.0;
    pub const REST_ALPHA: RangeInclusive<f32> = 0.0..=1.0;
    pub const ACTIVITY_SECONDS: RangeInclusive<f32> = 0.0..=5.0;
    pub const DISPLACEMENT_REF: RangeInclusive<f32> = 0.0..=100.0;
    pub const VELOCITY_REF: RangeInclusive<f32> = 0.0..=1000.0;
}

/// Edit a GridBackdrop component.
pub fn edit_grid_backdrop(
    inspector: &mut EditableInspector<'_>,
    backdrop: &GridBackdrop,
    _extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<GridBackdrop>> {
    let mut new = backdrop.clone();
    let mut hint = None;

    inspector.header("GridBackdrop");

    if let EditResult::Changed(index) = inspector.cycle(
        "Topology",
        backdrop.topology.label(),
        backdrop.topology.index(),
        GridTopology::ALL.len(),
    ) {
        new.topology = GridTopology::ALL[index];
        // A switch to Hex snaps the column count the way the engine will.
        new.cols = GridBackdrop::normalized_cols(new.cols, new.topology);
        hint = Some("topology");
    }
    if let EditResult::Changed(v) = inspector.f32_hard("Columns", backdrop.cols as f32, ranges::DIMENSION) {
        new.cols = GridBackdrop::normalized_cols(v.round() as u32, new.topology);
        hint = Some("cols");
    }
    if let EditResult::Changed(v) = inspector.f32_hard("Rows", backdrop.rows as f32, ranges::DIMENSION) {
        new.rows = (v.round() as u32).clamp(2, GridBackdrop::MAX_DIMENSION);
        hint = Some("rows");
    }
    inspector.f32_hard("Spacing", backdrop.spacing, ranges::SPACING).assign(&mut new.spacing, &mut hint, "spacing");
    inspector.color("Color", backdrop.color).assign(&mut new.color, &mut hint, "color");
    inspector.bool("Visible", backdrop.visible).assign(&mut new.visible, &mut hint, "visible");
    inspector.f32("Emissive", backdrop.emissive, ranges::EMISSIVE).assign(&mut new.emissive, &mut hint, "emissive");
    inspector.f32("Stiffness", backdrop.stiffness, ranges::STIFFNESS).assign(&mut new.stiffness, &mut hint, "stiffness");
    inspector.f32("Damping", backdrop.damping, ranges::DAMPING).assign(&mut new.damping, &mut hint, "damping");
    inspector.f32("Rest Pull", backdrop.rest_pull, ranges::REST_PULL).assign(&mut new.rest_pull, &mut hint, "rest_pull");
    inspector.f32("Rest Alpha", backdrop.rest_alpha_fraction, ranges::REST_ALPHA).assign(&mut new.rest_alpha_fraction, &mut hint, "rest_alpha_fraction");
    inspector.f32("Attack (s)", backdrop.activity_attack, ranges::ACTIVITY_SECONDS).assign(&mut new.activity_attack, &mut hint, "activity_attack");
    inspector.f32("Release (s)", backdrop.activity_release, ranges::ACTIVITY_SECONDS).assign(&mut new.activity_release, &mut hint, "activity_release");
    inspector.f32(
        "Displacement Ref",
        backdrop.activity_displacement_ref,
        ranges::DISPLACEMENT_REF,
    ).assign(&mut new.activity_displacement_ref, &mut hint, "activity_displacement_ref");
    inspector.f32("Velocity Ref", backdrop.activity_velocity_ref, ranges::VELOCITY_REF).assign(&mut new.activity_velocity_ref, &mut hint, "activity_velocity_ref");

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{extras, frame};
    use crate::{DragDropState, EditableFieldStyle, FieldId};
    use input::prelude::KeyCode;

    #[test]
    fn test_typed_odd_hex_column_count_commits_as_the_even_count_that_renders() {
        // Field 1 = Columns (field 0 is the Topology cycle row). A hex grid
        // only renders even column counts, so what the inspector stores is
        // what the engine builds: a typed 45 commits as 46.
        let mut ui = ui::UIContext::new();
        let mut drag_drop = DragDropState::new();
        let mut input = input::InputHandler::new();
        let field: ui::WidgetId = FieldId::new(0, 1, 0).into();
        ui.focus_text_input(field, "45");
        input.keyboard_mut().handle_key_press(KeyCode::Enter);

        let style = EditableFieldStyle::default();
        let edit = frame(&mut ui, &input, |ui| {
            let mut inspector = EditableInspector::new(ui, &style, 10.0, 10.0);
            edit_grid_backdrop(&mut inspector, &GridBackdrop::default(), &mut extras(&mut drag_drop))
        })
        .expect("Enter commits");

        assert_eq!((edit.field_hint, edit.new_value.cols), ("cols", 46));
    }
}
