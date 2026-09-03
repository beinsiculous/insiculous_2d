//! The component editors as an author experiences them, headless: what a
//! click on a cycle row does to the value, what typing into a string field
//! commits and in which frame, when a typed number earns a status-bar
//! warning, and how an edit reaches the undo stack through
//! `apply_component_edit`.

use ecs::behavior::{Behavior, EntityTag};
use glam::Vec2;
use input::prelude::KeyCode;
use physics::components::{Collider, ColliderShape, RigidBody, RigidBodyType};

use super::{apply_component_edit, edit_collider, edit_entity_tag, edit_rigid_body, ComponentEdit};
use crate::commands::{CommandHistory, SetTransformCommand};
use crate::edit_behavior;
use crate::test_support::{click_through, extras, frame, setup_entity};
use crate::{DragDropState, EditResult, EditableInspector, FieldId};

const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);

/// Geometry of the first field row under a component header, mirroring the
/// inspector's layout math (origin 10, default style: header advance 28,
/// indent 16, label_width 120, row_height 24, default width 300).
struct FirstRow {
    prev_btn_center: Vec2,
    next_btn_center: Vec2,
    row_y: f32,
}

fn first_row() -> FirstRow {
    let style = crate::EditableFieldStyle::default();
    let row_y = ORIGIN.y + style.row_height + 4.0; // after the header
    let pos_x = ORIGIN.x + style.indent;
    let control_x = pos_x + style.label_width;
    let btn_size = style.row_height - 6.0;
    let btn_y = row_y + (style.row_height - btn_size) / 2.0;
    let right = ORIGIN.x + 300.0; // DEFAULT_INSPECTOR_WIDTH
    let value_width = (right - control_x - 2.0 * btn_size - 6.0).clamp(60.0, 120.0);
    let next_x = (control_x + btn_size + value_width).min(right - btn_size);
    FirstRow {
        prev_btn_center: Vec2::new(control_x + btn_size / 2.0, btn_y + btn_size / 2.0),
        next_btn_center: Vec2::new(next_x + btn_size / 2.0, btn_y + btn_size / 2.0),
        row_y,
    }
}

/// Seed the text field `field` with `text` (as if the author had typed it)
/// and press Enter for the coming frame.
fn type_and_enter(ui: &mut ui::UIContext, input: &mut input::InputHandler, field: FieldId, text: &str) {
    let widget: ui::WidgetId = field.into();
    ui.focus_text_input(widget, text);
    input.keyboard_mut().handle_key_press(KeyCode::Enter);
}

fn platformer(tag: &str) -> Behavior {
    Behavior::PlayerPlatformer { move_speed: 100.0, jump_impulse: 50.0, jump_cooldown: 0.2, tag: tag.into() }
}

#[test]
fn test_string_fields_commit_on_enter_with_their_field_hint() {
    // EntityTag's tag is field 0; Behavior's tag is field 4 (cycle = 0,
    // three f32 fields = 1..3) — this test breaks if that order drifts,
    // which is the point: a drifted id would commit into the wrong field.
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();

    type_and_enter(&mut ui, &mut input, FieldId::new(0, 0, 0), "enemy");
    let tag_edit = frame(&mut ui, &input, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_entity_tag(&mut inspector, &EntityTag("player".into()), &mut extras(&mut drag_drop))
    })
    .expect("Enter commits the tag");
    assert_eq!((tag_edit.new_value.0.as_str(), tag_edit.field_hint), ("enemy", "tag"));

    type_and_enter(&mut ui, &mut input, FieldId::new(0, 4, 0), "p1");
    let behavior_edit = frame(&mut ui, &input, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_behavior(&mut inspector, &platformer("old"), &mut extras(&mut drag_drop))
    })
    .expect("Enter commits the behavior tag");
    assert_eq!(behavior_edit.field_hint, "tag");
    assert_eq!(platformer_tag(&behavior_edit.new_value), Some("p1"), "only the tag changes, never the variant");
}

/// The tag of a `PlayerPlatformer`, `None` for any other variant.
fn platformer_tag(behavior: &Behavior) -> Option<&str> {
    match behavior {
        Behavior::PlayerPlatformer { tag, .. } => Some(tag.as_str()),
        _ => None,
    }
}

#[test]
fn test_cycle_rows_step_the_variant_and_carry_collider_dimensions() {
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let row = first_row();

    // RigidBody: the "next" arrow steps Dynamic → Static on the RELEASE frame.
    let body = RigidBody::default();
    let (press, release) = click_through(&mut ui, &mut input, row.next_btn_center, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_rigid_body(&mut inspector, &body, &mut extras(&mut drag_drop))
    });
    assert!(press.is_none(), "press frame must not fire the cycle");
    let edit = release.expect("release frame cycles the body type");
    assert_eq!((edit.field_hint, edit.new_value.body_type), ("body_type", RigidBodyType::Static));

    // Collider: the "prev" arrow wraps Box (0) → CapsuleX (3) and carries
    // the box's 20×10 half-extents into the capsule instead of resetting it.
    let collider = Collider::new(ColliderShape::Box { half_extents: Vec2::new(20.0, 10.0) });
    let mut input = input::InputHandler::new();
    let (_, release) = click_through(&mut ui, &mut input, row.prev_btn_center, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_collider(&mut inspector, &collider, &mut extras(&mut drag_drop))
    });
    let edit = release.expect("release frame cycles the shape");
    assert_eq!(edit.field_hint, "shape");
    assert_eq!(
        edit.new_value.shape,
        ColliderShape::CapsuleX { half_height: 10.0, radius: 10.0 },
        "the new variant keeps the old shape's extent"
    );
}

#[test]
fn test_pending_string_edit_commits_before_variant_cycle_applies() {
    // With the tag field focused, clicking a cycle arrow commits the
    // pending edit on the PRESS frame (click-away fires on
    // mouse_just_pressed and clears focus) and the cycle applies on the
    // RELEASE frame — nothing typed is silently discarded.
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let behavior = platformer("old");
    let field: ui::WidgetId = FieldId::new(0, 4, 0).into();
    ui.focus_text_input(field, "goblin");
    let row = first_row();
    assert!(row.row_y > ORIGIN.y, "cycle row sits below the header");

    let (press, release) = click_through(&mut ui, &mut input, row.next_btn_center, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_behavior(&mut inspector, &behavior, &mut extras(&mut drag_drop))
    });

    let committed = press.expect("press frame commits the pending tag edit");
    assert_eq!(committed.field_hint, "tag");
    assert_eq!(platformer_tag(&committed.new_value), Some("goblin"), "the commit must not change the variant");
    let cycled = release.expect("release frame applies the variant cycle");
    assert_eq!(cycled.field_hint, "variant");
    assert_eq!(cycled.new_value.variant_index(), behavior.variant_index() + 1, "next arrow = next variant");
}

#[test]
fn test_typed_value_outside_soft_range_is_accepted_and_warned() {
    // Soft ranges accept the typed value; the inspector reports it so
    // the host can warn on the status bar instead of silently clamping. A
    // value inside the range, or an angle that WRAPS into range (270° →
    // −90°), earns no warning.
    type FieldEdit = fn(&mut EditableInspector<'_>) -> EditResult<f32>;
    let cases: [(&str, FieldEdit, f32, usize); 3] = [
        ("9999", |inspector| inspector.f32("Stiffness", 60.0, 0.0..=200.0), 9999.0, 1),
        ("80", |inspector| inspector.f32("Stiffness", 60.0, 0.0..=200.0), 80.0, 0),
        ("270", |inspector| inspector.angle("Rotation", 0.0), (-90.0_f32).to_radians(), 0),
    ];
    for (typed, edit_field, expected, warning_count) in cases {
        let mut ui = ui::UIContext::new();
        let mut input = input::InputHandler::new();
        type_and_enter(&mut ui, &mut input, FieldId::new(0, 0, 0), typed);

        let (edit, warnings) = frame(&mut ui, &input, |ui| {
            let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
            let edit = edit_field(&mut inspector);
            (edit, inspector.take_warnings())
        });

        let EditResult::Changed(value) = edit else { panic!("typing {typed} must commit, got {edit:?}") };
        assert!((value - expected).abs() < 1e-4, "typing {typed} committed {value}, expected {expected}");
        assert_eq!(warnings.len(), warning_count, "typing {typed}: {warnings:?}");
        if warning_count > 0 {
            assert!(warnings[0].contains("Stiffness") && warnings[0].contains("0..200"), "{}", warnings[0]);
        }
    }
}

#[test]
fn test_apply_component_edit_records_one_entry_that_merges_by_field_hint() {
    let mut world = ecs::World::new();
    let entity = setup_entity(&mut world);
    let mut history = CommandHistory::new();
    let at = |x: f32| common::Transform2D::new(Vec2::new(x, 0.0));
    let make_cmd = |e, old, new, hint| -> Box<dyn crate::EditorCommand> {
        Box::new(SetTransformCommand::new(e, old, new, hint))
    };
    let position_of = |world: &ecs::World| world.get::<common::Transform2D>(entity).map(|t| t.position);

    // No edit this frame: nothing is written and nothing is recorded.
    apply_component_edit(&mut world, entity, &at(0.0), None, &mut history, make_cmd);
    assert!(!history.can_undo(), "a frame without an edit records nothing");

    // Two frames of one gesture (same field_hint) collapse into ONE entry
    // whose undo restores the FIRST before-image.
    let first = Some(ComponentEdit { new_value: at(5.0), field_hint: "position" });
    apply_component_edit(&mut world, entity, &at(0.0), first, &mut history, make_cmd);
    assert_eq!(position_of(&world), Some(Vec2::new(5.0, 0.0)), "the world updates immediately");
    let second = Some(ComponentEdit { new_value: at(9.0), field_hint: "position" });
    apply_component_edit(&mut world, entity, &at(5.0), second, &mut history, make_cmd);
    assert_eq!(history.undo_name(), Some("Set Transform"));

    // A different field starts a NEW entry.
    let mut rotated = at(9.0);
    rotated.rotation = 1.0;
    let third = Some(ComponentEdit { new_value: rotated, field_hint: "rotation" });
    apply_component_edit(&mut world, entity, &at(9.0), third, &mut history, make_cmd);

    assert!(history.undo(&mut world), "the rotation entry undoes first");
    assert_eq!(world.get::<common::Transform2D>(entity), Some(&at(9.0)));
    assert!(history.undo(&mut world), "the merged position gesture is one entry");
    assert_eq!(position_of(&world), Some(Vec2::ZERO), "undo restores the pre-gesture value");
    assert!(!history.undo(&mut world), "three edits recorded exactly two entries");
}
