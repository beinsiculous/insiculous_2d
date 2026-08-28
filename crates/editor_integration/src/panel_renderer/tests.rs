use editor::apply_component_edit;
use ecs::World;
use editor::ComponentEdit;
use glam::Vec2;

/// Verify Transform2D writeback applies the new value and records an undo entry.
#[test]
fn test_transform_writeback_applies_and_records_undo() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let old = *world.get::<common::Transform2D>(entity).unwrap();
    let mut new = old;
    new.position = Vec2::new(100.0, 200.0);

    let mut history = editor::CommandHistory::new();
    apply_component_edit(
        &mut world, entity, &old,
        Some(ComponentEdit { new_value: new, field_hint: "position" }),
        &mut history,
        |e, old, new, hint| Box::new(editor::commands::SetTransformCommand::new(e, old, new, hint)),
    );

    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::new(100.0, 200.0));

    // Undo restores the original value.
    history.undo(&mut world);
    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::ZERO);
}

/// Verify writeback is a no-op (world untouched, no undo entry) when nothing changed.
#[test]
fn test_writeback_none_edit_is_noop() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::new(7.0, 8.0))).ok();
    let old = *world.get::<common::Transform2D>(entity).unwrap();

    let mut history = editor::CommandHistory::new();
    apply_component_edit(
        &mut world, entity, &old,
        None,
        &mut history,
        |e, old, new, hint| Box::new(editor::commands::SetTransformCommand::new(e, old, new, hint)),
    );

    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::new(7.0, 8.0));
    assert!(!history.can_undo());
}

/// Verify Sprite writeback applies the full new value.
#[test]
fn test_sprite_writeback() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, ecs::sprite_components::Sprite::new(42)).ok();

    let old = world.get::<ecs::sprite_components::Sprite>(entity).unwrap().clone();
    let mut new = old.clone();
    new.color = glam::Vec4::new(1.0, 0.0, 0.0, 1.0);

    let mut history = editor::CommandHistory::new();
    apply_component_edit(
        &mut world, entity, &old,
        Some(ComponentEdit { new_value: new, field_hint: "color" }),
        &mut history,
        |e, old, new, hint| Box::new(editor::commands::SetSpriteCommand::new(e, old, new, hint)),
    );

    let s = world.get::<ecs::sprite_components::Sprite>(entity).unwrap();
    assert_eq!(s.color, glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(s.texture_handle, 42); // unchanged
    assert!(history.can_undo());
}

/// Verify RigidBody writeback applies physics properties.
#[test]
fn test_rigid_body_writeback() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, physics::components::RigidBody::default()).ok();

    let old = world.get::<physics::components::RigidBody>(entity).unwrap().clone();
    let mut new = old.clone();
    new.gravity_scale = 0.5;

    let mut history = editor::CommandHistory::new();
    apply_component_edit(
        &mut world, entity, &old,
        Some(ComponentEdit { new_value: new, field_hint: "gravity_scale" }),
        &mut history,
        |e, old, new, hint| Box::new(editor::commands::SetRigidBodyCommand::new(e, old, new, hint)),
    );

    assert_eq!(world.get::<physics::components::RigidBody>(entity).unwrap().gravity_scale, 0.5);

    history.undo(&mut world);
    assert_eq!(world.get::<physics::components::RigidBody>(entity).unwrap().gravity_scale, 1.0);
}

/// Verify Collider writeback applies material properties.
#[test]
fn test_collider_writeback() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, physics::components::Collider::default()).ok();

    let old = world.get::<physics::components::Collider>(entity).unwrap().clone();
    let mut new = old.clone();
    new.friction = 0.9;
    new.is_sensor = true;

    let mut history = editor::CommandHistory::new();
    apply_component_edit(
        &mut world, entity, &old,
        Some(ComponentEdit { new_value: new, field_hint: "friction" }),
        &mut history,
        |e, old, new, hint| Box::new(editor::commands::SetColliderCommand::new(e, old, new, hint)),
    );

    let c = world.get::<physics::components::Collider>(entity).unwrap();
    assert_eq!(c.friction, 0.9);
    assert!(c.is_sensor);
}

/// Verify AudioSource writeback applies audio properties.
#[test]
fn test_audio_source_writeback() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, ecs::audio_components::AudioSource::default()).ok();

    let old = world.get::<ecs::audio_components::AudioSource>(entity).unwrap().clone();
    let mut new = old.clone();
    new.volume = 0.5;
    new.spatial = true;

    let mut history = editor::CommandHistory::new();
    apply_component_edit(
        &mut world, entity, &old,
        Some(ComponentEdit { new_value: new, field_hint: "volume" }),
        &mut history,
        |e, old, new, hint| Box::new(editor::commands::SetAudioSourceCommand::new(e, old, new, hint)),
    );

    let a = world.get::<ecs::audio_components::AudioSource>(entity).unwrap();
    assert_eq!(a.volume, 0.5);
    assert!(a.spatial);
}

/// Verify continuous edits applied via apply_component_edit merge into one undo entry.
#[test]
fn test_apply_component_edit_merges_continuous_edits() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let mut history = editor::CommandHistory::new();

    // Simulate 3 frames of dragging the position slider.
    for i in 1..=3 {
        let old = *world.get::<common::Transform2D>(entity).unwrap();
        let mut new = old;
        new.position = Vec2::new(i as f32 * 10.0, 0.0);
        apply_component_edit(
            &mut world, entity, &old,
            Some(ComponentEdit { new_value: new, field_hint: "position" }),
            &mut history,
            |e, old, new, hint| Box::new(editor::commands::SetTransformCommand::new(e, old, new, hint)),
        );
    }

    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::new(30.0, 0.0));

    // Single undo goes all the way back to the original.
    history.undo(&mut world);
    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::ZERO);
    assert!(!history.can_undo());
}

/// Verify writeback on non-existent entity is safe (no panic).
#[test]
fn test_writeback_missing_entity_is_safe() {
    let mut world = World::new();
    let fake_entity = ecs::EntityId::with_generation(999, 0);

    // Attempting writeback on a non-existent entity should return None, not panic
    let result = world.get_mut::<common::Transform2D>(fake_entity);
    assert!(result.is_none());
}

/// Verify writeback when component not present on entity is safe.
#[test]
fn test_writeback_missing_component_is_safe() {
    let mut world = World::new();
    let entity = world.create_entity();
    // Entity exists but has no Transform2D

    let result = world.get_mut::<common::Transform2D>(entity);
    assert!(result.is_none());
}

// Gizmo drag application/commit/cancel behavior is covered by
// editor_game/gizmo_drag_tests.rs (against the real drag state), and the
// interaction math by the editor crate's gizmo tests.

// ==================== Command-based writeback tests ====================

/// Verify SetTransformCommand applies and undoes correctly via command history.
#[test]
fn test_set_transform_via_command_and_undo() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let old = *world.get::<common::Transform2D>(entity).unwrap();
    let new = common::Transform2D::new(Vec2::new(100.0, 200.0));

    // Apply manually (like the inspector does)
    if let Some(t) = world.get_mut::<common::Transform2D>(entity) {
        *t = new;
    }

    // Push to history without executing
    let mut history = editor::CommandHistory::new();
    let cmd = editor::commands::SetTransformCommand::new(entity, old, new, "position");
    history.try_merge_or_push(Box::new(cmd));

    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::new(100.0, 200.0));

    // Undo should restore original
    history.undo(&mut world);
    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::ZERO);

    // Redo should reapply
    history.redo(&mut world);
    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::new(100.0, 200.0));
}

/// Verify continuous slider drags merge into a single undo entry.
#[test]
fn test_transform_slider_merges_into_single_undo() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let mut history = editor::CommandHistory::new();

    // Simulate 3 frames of dragging the position slider
    let t0 = *world.get::<common::Transform2D>(entity).unwrap();
    let t1 = common::Transform2D::new(Vec2::new(10.0, 0.0));
    let t2 = common::Transform2D::new(Vec2::new(20.0, 0.0));
    let t3 = common::Transform2D::new(Vec2::new(30.0, 0.0));

    // Frame 1
    world.get_mut::<common::Transform2D>(entity).unwrap().position = t1.position;
    history.try_merge_or_push(Box::new(
        editor::commands::SetTransformCommand::new(entity, t0, t1, "position"),
    ));

    // Frame 2 (merges)
    world.get_mut::<common::Transform2D>(entity).unwrap().position = t2.position;
    history.try_merge_or_push(Box::new(
        editor::commands::SetTransformCommand::new(entity, t1, t2, "position"),
    ));

    // Frame 3 (merges)
    world.get_mut::<common::Transform2D>(entity).unwrap().position = t3.position;
    history.try_merge_or_push(Box::new(
        editor::commands::SetTransformCommand::new(entity, t2, t3, "position"),
    ));

    // Single undo should go back to original
    history.undo(&mut world);
    assert_eq!(world.get::<common::Transform2D>(entity).unwrap().position, Vec2::ZERO);
    assert!(!history.can_undo());
}

/// Verify AddComponentCommand works via execute and can be undone.
#[test]
fn test_add_component_via_command_and_undo() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();

    let mut history = editor::CommandHistory::new();
    let cmd = editor::commands::AddComponentCommand::new(
        entity, editor::commands::ComponentKind::Sprite,
    );
    history.execute(Box::new(cmd), &mut world);

    assert!(world.get::<ecs::sprite_components::Sprite>(entity).is_some());

    history.undo(&mut world);
    assert!(world.get::<ecs::sprite_components::Sprite>(entity).is_none());
}

/// Verify RemoveComponentCommand works via execute and can be undone.
#[test]
fn test_remove_component_via_command_and_undo() {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(Vec2::ZERO)).ok();
    world.add_component(&entity, ecs::sprite_components::Sprite::new(42)).ok();

    let mut history = editor::CommandHistory::new();
    let cmd = editor::commands::RemoveComponentCommand::new(
        entity, editor::commands::ComponentKind::Sprite,
    );
    history.execute(Box::new(cmd), &mut world);

    assert!(world.get::<ecs::sprite_components::Sprite>(entity).is_none());

    history.undo(&mut world);
    assert!(world.get::<ecs::sprite_components::Sprite>(entity).is_some());
    assert_eq!(world.get::<ecs::sprite_components::Sprite>(entity).unwrap().texture_handle, 42);
}
