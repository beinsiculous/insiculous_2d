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
    if let EditResult::Changed(v) = inspector.f32_hard("Spacing", backdrop.spacing, ranges::SPACING) {
        new.spacing = v;
        hint = Some("spacing");
    }
    if let EditResult::Changed(v) = inspector.color("Color", backdrop.color) {
        new.color = v;
        hint = Some("color");
    }
    if let EditResult::Changed(v) = inspector.bool("Visible", backdrop.visible) {
        new.visible = v;
        hint = Some("visible");
    }
    if let EditResult::Changed(v) = inspector.f32("Emissive", backdrop.emissive, ranges::EMISSIVE) {
        new.emissive = v;
        hint = Some("emissive");
    }
    if let EditResult::Changed(v) = inspector.f32("Stiffness", backdrop.stiffness, ranges::STIFFNESS) {
        new.stiffness = v;
        hint = Some("stiffness");
    }
    if let EditResult::Changed(v) = inspector.f32("Damping", backdrop.damping, ranges::DAMPING) {
        new.damping = v;
        hint = Some("damping");
    }
    if let EditResult::Changed(v) = inspector.f32("Rest Pull", backdrop.rest_pull, ranges::REST_PULL) {
        new.rest_pull = v;
        hint = Some("rest_pull");
    }
    if let EditResult::Changed(v) =
        inspector.f32("Rest Alpha", backdrop.rest_alpha_fraction, ranges::REST_ALPHA)
    {
        new.rest_alpha_fraction = v;
        hint = Some("rest_alpha_fraction");
    }
    if let EditResult::Changed(v) =
        inspector.f32("Attack (s)", backdrop.activity_attack, ranges::ACTIVITY_SECONDS)
    {
        new.activity_attack = v;
        hint = Some("activity_attack");
    }
    if let EditResult::Changed(v) =
        inspector.f32("Release (s)", backdrop.activity_release, ranges::ACTIVITY_SECONDS)
    {
        new.activity_release = v;
        hint = Some("activity_release");
    }
    if let EditResult::Changed(v) = inspector.f32(
        "Displacement Ref",
        backdrop.activity_displacement_ref,
        ranges::DISPLACEMENT_REF,
    ) {
        new.activity_displacement_ref = v;
        hint = Some("activity_displacement_ref");
    }
    if let EditResult::Changed(v) =
        inspector.f32("Velocity Ref", backdrop.activity_velocity_ref, ranges::VELOCITY_REF)
    {
        new.activity_velocity_ref = v;
        hint = Some("activity_velocity_ref");
    }

    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{extras, frame};
    use crate::{DragDropState, FieldId};
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

        let edit = frame(&mut ui, &input, |ui| {
            let mut inspector = EditableInspector::new(ui, 10.0, 10.0);
            edit_grid_backdrop(&mut inspector, &GridBackdrop::default(), &mut extras(&mut drag_drop))
        })
        .expect("Enter commits");

        assert_eq!((edit.field_hint, edit.new_value.cols), ("cols", 46));
    }
}
