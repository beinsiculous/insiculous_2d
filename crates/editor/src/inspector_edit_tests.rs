//! Headless coverage for the Sprint-3 editable inspector paths (#34/#35):
//! EntityTag and Behavior string fields committing through `string_edit`,
//! the RigidBody/Collider cycle rows, and the commit-before-cycle ordering
//! that makes the cycle early-return safe (kimi batch-3 F1/F4).

use ecs::behavior::{Behavior, EntityTag};
use glam::Vec2;
use physics::components::{Collider, RigidBody, RigidBodyType};

use crate::component_editors::{edit_collider, edit_entity_tag, edit_rigid_body};
use crate::edit_behavior;
use crate::{DragDropState, EditableInspector, FieldId, InspectorExtras};

const ORIGIN: Vec2 = Vec2::new(10.0, 10.0);

fn extras(drag_drop: &mut DragDropState) -> InspectorExtras<'_> {
    InspectorExtras { drag_drop, texture_display: None }
}

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

/// Press frame + release frame at `point`, running `edit` each frame.
/// Returns the per-frame outputs (press frame, release frame).
fn click_through<T>(
    ui: &mut ui::UIContext,
    input: &mut input::InputHandler,
    point: Vec2,
    mut edit: impl FnMut(&mut ui::UIContext) -> T,
) -> (T, T) {
    use input::prelude::MouseButton;
    input.mouse_mut().update_position(point.x, point.y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    ui.begin_frame(&*input, Vec2::new(800.0, 600.0));
    let press = edit(ui);
    ui.end_frame();

    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    ui.begin_frame(&*input, Vec2::new(800.0, 600.0));
    let release = edit(ui);
    ui.end_frame();
    (press, release)
}

#[test]
fn test_edit_entity_tag_commits_via_string_edit() {
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let tag = EntityTag("player".into());

    // Arm the tag field (field 0 of component 0) as if clicked, seeded with
    // the replacement text, then commit with Enter.
    let field: ui::WidgetId = FieldId::new(0, 0, 0).into();
    ui.focus_text_input(field, "enemy");
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);

    ui.begin_frame(&input, Vec2::new(800.0, 600.0));
    let mut inspector = EditableInspector::new(&mut ui, ORIGIN.x, ORIGIN.y);
    let edit = edit_entity_tag(&mut inspector, &tag, &mut extras(&mut drag_drop));
    ui.end_frame();

    let edit = edit.expect("Enter commit must produce an edit");
    assert_eq!(edit.new_value.0, "enemy");
    assert_eq!(edit.field_hint, "tag");
}

#[test]
fn test_behavior_tag_commits_via_string_edit() {
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let behavior = Behavior::PlayerPlatformer {
        move_speed: 100.0,
        jump_impulse: 50.0,
        jump_cooldown: 0.2,
        tag: "old".into(),
    };

    // Field order inside edit_behavior: cycle=0, three f32 fields=1..3,
    // tag=4 — this test breaks if that order drifts, which is the point.
    let field: ui::WidgetId = FieldId::new(0, 4, 0).into();
    ui.focus_text_input(field, "p1");
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);

    ui.begin_frame(&input, Vec2::new(800.0, 600.0));
    let mut inspector = EditableInspector::new(&mut ui, ORIGIN.x, ORIGIN.y);
    let edit = edit_behavior(&mut inspector, &behavior, &mut extras(&mut drag_drop));
    ui.end_frame();

    let edit = edit.expect("Enter commit must produce an edit");
    assert_eq!(edit.field_hint, "tag");
    match edit.new_value {
        Behavior::PlayerPlatformer { tag, .. } => assert_eq!(tag, "p1"),
        other => panic!("variant must be unchanged, got {other:?}"),
    }
}

#[test]
fn test_rigid_body_type_cycle_row_changes_type() {
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let body = RigidBody::default(); // Dynamic

    let row = first_row();
    let (press, release) = click_through(&mut ui, &mut input, row.next_btn_center, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_rigid_body(&mut inspector, &body, &mut extras(&mut drag_drop))
    });

    assert!(press.is_none(), "press frame must not fire the cycle");
    let edit = release.expect("release frame cycles the body type");
    assert_eq!(edit.field_hint, "body_type");
    assert_eq!(edit.new_value.body_type, RigidBodyType::Static);
}

#[test]
fn test_collider_shape_cycle_carries_size_into_new_variant() {
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let collider = Collider::new(physics::components::ColliderShape::Box {
        half_extents: Vec2::new(20.0, 10.0),
    });

    let row = first_row();
    let (_, release) = click_through(&mut ui, &mut input, row.prev_btn_center, |ui| {
        let mut inspector = EditableInspector::new(ui, ORIGIN.x, ORIGIN.y);
        edit_collider(&mut inspector, &collider, &mut extras(&mut drag_drop))
    });

    let edit = release.expect("release frame cycles the shape");
    assert_eq!(edit.field_hint, "shape");
    // Box is variant 0; prev wraps to CapsuleX (3), carrying dimensions.
    assert_eq!(edit.new_value.shape.variant_index(), 3);
    assert_eq!(
        edit.new_value.shape,
        collider.shape.variant_with_carried_dimensions(3)
    );
}

#[test]
fn test_pending_string_edit_commits_before_variant_cycle_applies() {
    // Kimi batch-3 F1 regression lock: with the tag field focused, clicking
    // a cycle arrow commits the pending edit on the PRESS frame
    // (click-away fires on mouse_just_pressed and clears focus) and the
    // cycle applies on the RELEASE frame — nothing is silently discarded.
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let behavior = Behavior::PlayerPlatformer {
        move_speed: 100.0,
        jump_impulse: 50.0,
        jump_cooldown: 0.2,
        tag: "old".into(),
    };

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
    match committed.new_value {
        Behavior::PlayerPlatformer { tag, .. } => assert_eq!(tag, "goblin"),
        other => panic!("tag commit must not change the variant, got {other:?}"),
    }

    let cycled = release.expect("release frame applies the variant cycle");
    assert_eq!(cycled.field_hint, "variant");
    assert_ne!(cycled.new_value.variant_index(), behavior.variant_index());
}
