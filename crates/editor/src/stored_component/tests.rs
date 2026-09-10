//! The `editor_component_registry!` macro's outputs agree with each other
//! and with the world: one registry line buys capture, restore, the Add
//! Component popup and the inspector — and a forgotten type breaks here.

use std::collections::HashSet;

use ecs::tilemap::Tilemap;
use glam::Vec2;

use super::*;
use crate::test_support::extras;
use crate::inspector::InspectorStyle;
use crate::EditableFieldStyle;
use ui::UIContext;

/// The registry's hidden entries: captured for snapshots but never
/// surfaced as inspector blocks or API component values.
const HIDDEN_ENTRIES: usize = 2; // GlobalTransform2D, BehaviorState

/// An entity carrying one of every registry type.
fn entity_with_every_registry_type(world: &mut World) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0))).ok();
    world.add_component(&entity, GlobalTransform2D::default()).ok();
    world.add_component(&entity, Name::new("All")).ok();
    world.add_component(&entity, common::Camera::default()).ok();
    world.add_component(&entity, Sprite::new(7)).ok();
    world.add_component(&entity, SpriteAnimation::default()).ok();
    world.add_component(&entity, Tilemap::default()).ok();
    world.add_component(&entity, RigidBody::default()).ok();
    world.add_component(&entity, Collider::default()).ok();
    world.add_component(&entity, AudioSource::default()).ok();
    world.add_component(&entity, AudioListener::default()).ok();
    world.add_component(&entity, Behavior::default()).ok();
    world.add_component(&entity, BehaviorState::default()).ok();
    world.add_component(&entity, EntityTag::default()).ok();
    world.add_component(&entity, UiLabel::default()).ok();
    world.add_component(&entity, UiPanel::default()).ok();
    world.add_component(&entity, UiButton::default()).ok();
    world.add_component(&entity, ecs::script::Scripts::default()).ok();
    world.add_component(&entity, ecs::GridBackdrop::default()).ok();
    entity
}

#[test]
fn test_registry_and_world_type_enumeration_agree_over_every_registry_line() {
    let mut world = World::new();
    let entity = entity_with_every_registry_type(&mut world);

    // The world's type-erased view and the registry's known set agree: an
    // entity carrying one of every registry type diffs to nothing, so the
    // snapshot's unknown-type detection never false-positives on a registry
    // type — and a new ecs component missing its registry line breaks here.
    let known: HashSet<std::any::TypeId> = registered_component_type_ids().into_iter().collect();
    let unknown: Vec<&'static str> = world
        .component_types(entity)
        .into_iter()
        .filter(|(type_id, _)| !known.contains(type_id))
        .map(|(_, name)| name)
        .collect();
    assert_eq!(unknown, Vec::<&str>::new(), "registry misses component types");

    // Capture walks the same registry (the TYPED count — the dynamic tier's
    // global contents vary across tests in this process).
    let captured = capture_all_components(&world, entity);
    assert_eq!(captured.len(), registered_typed_component_type_ids().len());
    assert_eq!(captured.len(), world.component_types(entity).len());

    // The command API's value capture skips the hidden entries and carries
    // serde fields; editable Name IS captured (describe lifts it upstream).
    let values = capture_all_values(&world, entity);
    assert_eq!(values.len(), captured.len() - HIDDEN_ENTRIES);
    let names: Vec<&str> = values.iter().map(|(name, _)| name.as_ref()).collect();
    assert!(names.contains(&"Name"), "editable Name is captured as a value");
    assert!(!names.contains(&"GlobalTransform2D"), "hidden entries are not emitted");
    assert!(!names.contains(&"BehaviorState"), "hidden entries are not emitted");
    let (_, transform) = values.iter().find(|(name, _)| name == "Transform2D").expect("Transform2D");
    assert_eq!(transform["position"][0], 1.0, "serde fields come through");

    // Restoring the capture onto a fresh entity reproduces the values —
    // the delete-undo and clipboard path.
    let fresh = world.create_entity();
    restore_components(&mut world, fresh, &captured);
    assert_eq!(world.component_types(fresh).len(), captured.len());
    assert_eq!(world.get::<common::Transform2D>(fresh).expect("transform").position, Vec2::new(1.0, 2.0));
    assert_eq!(world.get::<Sprite>(fresh).expect("sprite").texture_handle, 7);
    assert_eq!(world.get::<Name>(fresh).map(Name::as_str), Some("All"));
}

#[test]
fn test_inspector_renders_one_block_per_present_component_and_records_no_edit() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0))).ok();
    world.add_component(&entity, Sprite::new(0)).ok();
    world.add_component(&entity, EntityTag::new("player")).ok();
    let bare = world.create_entity();
    let mut ui = UIContext::new();
    let mut history = CommandHistory::new();
    let inspect_style = InspectorStyle::default();
    let field_style = EditableFieldStyle::default();
    let mut drag_drop = crate::DragDropState::new();
    let mut inspector_state = crate::InspectorState::default();
    let mut extras = extras(&mut drag_drop, &mut inspector_state);
    let start_y = 40.0;

    let mut frame = InspectorFrame {
        ui: &mut ui,
        inspect_style: &inspect_style,
        field_style: &field_style,
        x: 10.0,
        width: 400.0,
        section_gap: 10.0,
        read_only: false,
    };
    let (y, count) = edit_all_components(
        &mut frame, &mut world, entity, &mut history,
        start_y, &mut extras,
    );
    let (_, none_count) = edit_all_components(
        &mut frame, &mut world, bare, &mut history,
        start_y, &mut extras,
    );

    assert_eq!(count, 3, "one block per present registry component");
    assert!(y > start_y, "rendering advances the layout cursor");
    assert_eq!(none_count, 0, "an entity with no components renders no blocks");
    assert!(!history.can_undo(), "rendering without input records no edit");
}

#[test]
fn test_component_kind_dispatch_adds_captures_removes_and_lists_every_kind() {
    let mut world = World::new();
    let entity = world.create_entity();

    // The Add Component popup groups by category: every kind sits in
    // exactly one group, under the category it reports.
    let grouped: Vec<(ComponentCategory, ComponentKind)> = categorized_components()
        .into_iter()
        .flat_map(|(category, kinds)| kinds.into_iter().map(move |kind| (category, kind)))
        .collect();
    assert_eq!(grouped.len(), ComponentKind::ALL.len(), "each kind listed exactly once");
    for &kind in ComponentKind::ALL {
        assert!(grouped.contains(&(kind.category(), kind)), "{kind:?} under its own category");
    }

    // Choosing a kind adds its default, capture sees it, and the popup no
    // longer offers it; removing it puts it back on offer.
    for &kind in ComponentKind::ALL {
        kind.add_default(&mut world, entity);
        assert!(kind.is_present(&world, entity), "add_default did not add {kind:?}");
        assert!(kind.capture(&world, entity).is_some(), "capture misses {kind:?}");
        assert!(!available_components(&world, entity).contains(&kind), "{kind:?} still offered");
        kind.remove(&mut world, entity);
        assert!(!kind.is_present(&world, entity), "remove did not delete {kind:?}");
        assert!(kind.capture(&world, entity).is_none(), "capture of absent {kind:?}");
    }
    assert_eq!(available_components(&world, entity), ComponentKind::ALL.to_vec());
    assert_eq!(
        capture_all_components(&world, entity).len(),
        0,
        "a fully stripped entity captures nothing"
    );
}

#[test]
fn test_stored_component_from_json_round_trips_all_settable_types() {
    // The write path mirrors the read path for EVERY settable registry
    // entry: capture a live value → serde value → from_json → same type
    // name. A new registry line is covered automatically or this breaks.
    let mut world = World::new();
    let entity = entity_with_every_registry_type(&mut world);

    let values = capture_all_values(&world, entity);
    for name in settable_component_names() {
        // Dynamic-tier names (e.g. PlaySoundEffect) are settable but not on
        // this entity — the typed set is what the fixture attaches.
        let Some((_, value)) = values.iter().find(|(n, _)| *n == name) else {
            assert!(
                crate::stored_component::dynamic::is_dynamic_component(&name),
                "typed settable {name} missing from capture_all_values"
            );
            continue;
        };
        let stored = stored_component_from_json(&name, value.clone())
            .unwrap_or_else(|e| panic!("{name} round-trip failed: {e}"));
        assert_eq!(stored.type_name(), name);
        assert!(
            capture_component_by_name(&world, entity, &name)
                .expect("known name")
                .is_some(),
            "{name} capturable by name"
        );
    }
    assert!(
        !settable_component_names().iter().any(|n| n == "Name"),
        "Name is set through `rename`, never `set`"
    );
    assert!(stored_component_from_json("Bogus", serde_json::Value::Null).is_err());
}

/// Every text drawn in the header or label column of one inspector pass, in
/// draw order — the panel's rows, whatever control each of them carries.
fn row_labels(ui: &UIContext, origin_x: f32, indent: f32) -> Vec<String> {
    ui.draw_list()
        .commands()
        .iter()
        .filter_map(|command| match command {
            ui::DrawCommand::TextPlaceholder { text, position, .. } => Some((text, position.x)),
            ui::DrawCommand::Text { data, .. } => Some((&data.text, data.position.x)),
            _ => None,
        })
        .filter(|(_, x)| (x - origin_x).abs() < 0.01 || (x - origin_x - indent).abs() < 0.01)
        .map(|(text, _)| text.clone())
        .collect()
}

/// One `edit_all_components` pass over `entity`, read-only or not: the rows
/// it drew, the Y it ended at, and how many blocks it rendered.
fn inspector_pass(world: &mut World, entity: EntityId, read_only: bool) -> (Vec<String>, f32, usize) {
    inspector_pass_with(world, entity, read_only, &mut crate::InspectorState::default())
}

/// [`inspector_pass`] over a given view state, so a test can collapse a
/// section before the walk and read the toggles back after it.
fn inspector_pass_with(
    world: &mut World,
    entity: EntityId,
    read_only: bool,
    state: &mut crate::InspectorState,
) -> (Vec<String>, f32, usize) {
    const ORIGIN_X: f32 = 10.0;
    const START_Y: f32 = 40.0;
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    ui.begin_frame(&input, glam::Vec2::new(800.0, 600.0));
    let mut history = CommandHistory::new();
    let inspect_style = InspectorStyle::default();
    let field_style = EditableFieldStyle::default();
    let mut drag_drop = crate::DragDropState::new();
    let mut inspector_extras = extras(&mut drag_drop, state);
    let mut frame = InspectorFrame {
        ui: &mut ui,
        inspect_style: &inspect_style,
        field_style: &field_style,
        x: ORIGIN_X,
        width: 400.0,
        section_gap: 10.0,
        read_only,
    };
    let (y, count) = edit_all_components(
        &mut frame, world, entity, &mut history, START_Y, &mut inspector_extras,
    );
    ui.end_frame();
    (row_labels(&ui, ORIGIN_X, field_style.indent), y - START_Y, count)
}

#[test]
fn test_the_inspector_draws_the_same_rows_at_the_same_height_playing_as_editing() {
    // A play session swaps every control for its value, and nothing else:
    // a different row set would make the panel jump, and a shorter content
    // height would clamp the scroll offset away.
    let mut world = World::new();
    let entity = entity_with_every_registry_type(&mut world);
    world.add_component(&entity, Sprite::new(3)).ok();

    let (_, editing_height, editing_count) = inspector_pass(&mut world, entity, false);
    let (_, playing_height, playing_count) = inspector_pass(&mut world, entity, true);
    assert_eq!(playing_height, editing_height, "the content height must not change");
    assert_eq!(playing_count, editing_count, "the block count must not change");

    // An action button's label rides centred inside its button while
    // editing and sits in the label column while playing, so the row
    // sequence is read off an entity without one; the height above covers
    // the rows those buttons occupy.
    world.remove_component::<ecs::script::Scripts>(&entity).ok();
    let (editing_rows, _, _) = inspector_pass(&mut world, entity, false);
    let (playing_rows, _, _) = inspector_pass(&mut world, entity, true);
    assert!(editing_rows.contains(&"Color".to_string()), "the fixture carries a colour row");
    assert_eq!(playing_rows, editing_rows, "the row set and order must not change");
}

#[test]
fn test_a_collapsed_section_keeps_its_header_row_and_drops_every_field_row() {
    // Collapsing happens inside the inspector, not by skipping the editor
    // function: most editors draw their own header, so a skipped call would
    // take the header — and its toggle — with it.
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(1.0, 2.0))).ok();
    world.add_component(&entity, Sprite::new(0)).ok();

    let mut state = crate::InspectorState::default();
    let (open_rows, open_height, open_count) = inspector_pass_with(&mut world, entity, false, &mut state);
    assert!(open_rows.iter().any(|row| row.contains("Sprite")), "the section drew its header");
    assert!(open_rows.contains(&"Offset".to_string()), "and its fields");

    state.toggle_collapsed("Sprite");
    let (collapsed_rows, collapsed_height, collapsed_count) =
        inspector_pass_with(&mut world, entity, false, &mut state);

    assert!(
        collapsed_rows.iter().any(|row| row.contains("Sprite")),
        "a collapsed section still draws its header, which is its toggle"
    );
    assert!(!collapsed_rows.contains(&"Offset".to_string()), "its field rows are gone");
    assert!(
        collapsed_rows.iter().any(|row| row.contains("Transform2D")),
        "collapsing one section leaves the others alone"
    );
    assert!(collapsed_rows.contains(&"Position".to_string()));
    assert_eq!(collapsed_count, open_count, "the block count is unchanged");
    assert!(collapsed_height < open_height, "the panel is shorter by the skipped rows");
}
