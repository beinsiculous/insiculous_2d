//! Tests for `entity_ops` (split out to keep the module under 600 lines).

use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Create Tests ====================

    #[test]
    fn test_create_empty_has_transform_and_name() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        assert!(world.get::<common::Transform2D>(entity).is_some());
        assert!(world.get::<GlobalTransform2D>(entity).is_some());
        assert!(world.get::<Name>(entity).is_some());
    }

    #[test]
    fn test_create_empty_at_correct_position() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let pos = Vec2::new(100.0, -50.0);
        let entity = create_empty_entity(&mut world, &mut sel, pos, &mut counter);

        let t = world.get::<common::Transform2D>(entity).unwrap();
        assert_eq!(t.position, pos);
    }

    #[test]
    fn test_create_empty_auto_selects() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        assert_eq!(sel.primary(), Some(entity));
    }

    #[test]
    fn test_create_sprite_has_sprite_component() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_sprite_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        assert!(world.get::<Sprite>(entity).is_some());
        assert!(world.get::<common::Transform2D>(entity).is_some());
    }

    #[test]
    fn test_create_camera_has_camera_component() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_camera_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        assert!(world.get::<common::Camera>(entity).is_some());
        assert!(world.get::<common::Transform2D>(entity).is_some());
    }

    #[test]
    fn test_create_static_body_has_physics() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_physics_body(
            &mut world, &mut sel, Vec2::ZERO, RigidBodyType::Static, &mut counter,
        );

        let rb = world.get::<RigidBody>(entity).unwrap();
        assert_eq!(rb.body_type, RigidBodyType::Static);
        assert!(world.get::<Collider>(entity).is_some());
        assert!(world.get::<Sprite>(entity).is_some());
    }

    #[test]
    fn test_create_dynamic_body_has_physics() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_physics_body(
            &mut world, &mut sel, Vec2::ZERO, RigidBodyType::Dynamic, &mut counter,
        );

        let rb = world.get::<RigidBody>(entity).unwrap();
        assert_eq!(rb.body_type, RigidBodyType::Dynamic);
    }

    #[test]
    fn test_create_increments_counter() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        assert_eq!(counter, 1);
        create_sprite_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        assert_eq!(counter, 2);
        create_camera_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        assert_eq!(counter, 3);
    }

    #[test]
    fn test_create_names_are_unique() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let e1 = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let e2 = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        let n1 = world.get::<Name>(e1).unwrap().as_str().to_string();
        let n2 = world.get::<Name>(e2).unwrap().as_str().to_string();
        assert_ne!(n1, n2);
    }

    // ==================== Delete Tests ====================

    #[test]
    fn test_delete_removes_entity() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let entity = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        delete_selected_entities(&mut world, &mut sel);

        assert!(world.get::<common::Transform2D>(entity).is_none());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_delete_clears_selection() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        delete_selected_entities(&mut world, &mut sel);

        assert!(sel.is_empty());
    }

    #[test]
    fn test_delete_reparents_children_to_grandparent() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let grandparent = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let parent = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let child = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        world.set_parent(parent, grandparent).unwrap();
        world.set_parent(child, parent).unwrap();

        // Select only the parent for deletion
        sel.select(parent);
        delete_selected_entities(&mut world, &mut sel);

        // Child should now be parented to grandparent
        let child_parent = world.get_parent(child);
        assert_eq!(child_parent, Some(grandparent));
    }

    #[test]
    fn test_delete_orphans_children_when_root() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let parent = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let child = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        world.set_parent(child, parent).unwrap();

        // Select only the parent for deletion
        sel.select(parent);
        delete_selected_entities(&mut world, &mut sel);

        // Child should be a root entity now
        assert!(world.get_parent(child).is_none());
        assert!(world.get::<common::Transform2D>(child).is_some());
    }

    #[test]
    fn test_delete_empty_selection_is_noop() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        sel.clear();

        let count_before = world.entity_count();
        delete_selected_entities(&mut world, &mut sel);
        assert_eq!(world.entity_count(), count_before);
    }

    #[test]
    fn test_delete_multiple_selected() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let e1 = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let e2 = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let _e3 = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        sel.select(e1);
        sel.add(e2);
        delete_selected_entities(&mut world, &mut sel);

        // Only e3 should remain
        assert_eq!(world.entity_count(), 1);
    }

    // ==================== Duplicate Tests ====================

    #[test]
    fn test_duplicate_copies_components() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let original = create_sprite_entity(&mut world, &mut sel, Vec2::new(10.0, 20.0), &mut counter);

        sel.select(original);
        duplicate_selected_entities(&mut world, &mut sel);

        let dup = sel.primary().unwrap();
        assert_ne!(dup, original);
        assert!(world.get::<Sprite>(dup).is_some());
        assert!(world.get::<common::Transform2D>(dup).is_some());
        assert!(world.get::<Name>(dup).is_some());
    }

    #[test]
    fn test_duplicate_offsets_position() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let original = create_empty_entity(&mut world, &mut sel, Vec2::new(100.0, 200.0), &mut counter);

        sel.select(original);
        duplicate_selected_entities(&mut world, &mut sel);

        let dup = sel.primary().unwrap();
        let t = world.get::<common::Transform2D>(dup).unwrap();
        assert_eq!(t.position, Vec2::new(120.0, 180.0));
    }

    #[test]
    fn test_duplicate_selects_new_entity() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let original = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        sel.select(original);
        duplicate_selected_entities(&mut world, &mut sel);

        let dup = sel.primary().unwrap();
        assert_ne!(dup, original);
    }

    #[test]
    fn test_duplicate_preserves_original() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let original = create_empty_entity(&mut world, &mut sel, Vec2::new(50.0, 50.0), &mut counter);

        sel.select(original);
        duplicate_selected_entities(&mut world, &mut sel);

        // Original should be untouched
        let t = world.get::<common::Transform2D>(original).unwrap();
        assert_eq!(t.position, Vec2::new(50.0, 50.0));
    }

    #[test]
    fn test_duplicate_recursive_copies_children() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let parent = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let child = create_empty_entity(&mut world, &mut sel, Vec2::new(10.0, 10.0), &mut counter);
        world.set_parent(child, parent).unwrap();

        sel.select(parent);
        let count_before = world.entity_count();
        duplicate_selected_entities(&mut world, &mut sel);

        // Should have 2 new entities (parent dup + child dup)
        assert_eq!(world.entity_count(), count_before + 2);
    }

    #[test]
    fn test_duplicate_children_have_correct_parent() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let parent = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        let _child = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        world.set_parent(_child, parent).unwrap();

        sel.select(parent);
        duplicate_selected_entities(&mut world, &mut sel);

        let dup_parent = sel.primary().unwrap();
        let dup_children = world.get_children(dup_parent);
        assert!(dup_children.is_some());
        assert_eq!(dup_children.unwrap().len(), 1);

        // The duplicated child's parent should be the duplicated parent
        let dup_child = dup_children.unwrap()[0];
        assert_eq!(world.get_parent(dup_child), Some(dup_parent));
    }

    #[test]
    fn test_duplicate_name_appends_copy() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        let original = create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);

        sel.select(original);
        duplicate_selected_entities(&mut world, &mut sel);

        let dup = sel.primary().unwrap();
        let name = world.get::<Name>(dup).unwrap();
        assert!(name.as_str().ends_with("(Copy)"), "Name was: {}", name.as_str());
    }

    #[test]
    fn test_duplicate_empty_selection_is_noop() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;
        create_empty_entity(&mut world, &mut sel, Vec2::ZERO, &mut counter);
        sel.clear();

        let count_before = world.entity_count();
        duplicate_selected_entities(&mut world, &mut sel);
        assert_eq!(world.entity_count(), count_before);
    }

    // Component add/remove dispatch tests live with the registry in
    // editor/src/stored_component.rs (and the RigidBody → Collider cascade
    // in editor/src/commands.rs).
}

#[cfg(test)]
mod asset_assignment_tests {
    use super::*;
    use editor::CommandHistory;

    #[test]
    fn test_assign_sprite_texture_sets_handle_and_undoes() {
        let mut world = World::new();
        let mut selection = Selection::new();
        let mut counter = 0;
        let entity = create_sprite_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let mut history = CommandHistory::new();

        assert!(assign_sprite_texture(&mut world, entity, 7, &mut history));
        assert_eq!(world.get::<Sprite>(entity).unwrap().texture_handle, 7);

        assert!(history.undo(&mut world));
        assert_eq!(world.get::<Sprite>(entity).unwrap().texture_handle, 0, "undo restores the old texture");
    }

    #[test]
    fn test_assign_same_texture_or_no_sprite_is_noop() {
        let mut world = World::new();
        let mut selection = Selection::new();
        let mut counter = 0;
        let sprite_entity = create_sprite_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let bare_entity = create_empty_entity(&mut world, &mut selection, Vec2::ZERO, &mut counter);
        let mut history = CommandHistory::new();

        assert!(!assign_sprite_texture(&mut world, sprite_entity, 0, &mut history), "same handle");
        assert!(!assign_sprite_texture(&mut world, bare_entity, 7, &mut history), "no Sprite component");
        assert!(!history.can_undo(), "no-ops must not record undo entries");
    }

    #[test]
    fn test_create_sprite_entity_with_texture_undo_removes_it() {
        let mut world = World::new();
        let mut selection = Selection::new();
        let mut counter = 0;
        let mut history = CommandHistory::new();

        let entity = create_sprite_entity_with_texture(
            &mut world, &mut selection, Vec2::new(50.0, -20.0), 9, "crate", &mut counter, &mut history,
        );

        assert_eq!(world.get::<Sprite>(entity).unwrap().texture_handle, 9);
        assert_eq!(
            world.get::<common::Transform2D>(entity).unwrap().position,
            Vec2::new(50.0, -20.0)
        );
        assert!(world.get::<Name>(entity).unwrap().as_str().starts_with("crate"));
        assert_eq!(selection.primary(), Some(entity));

        assert!(history.undo(&mut world));
        assert!(world.get::<Sprite>(entity).is_none(), "undo deletes the spawned entity");
    }

    // ==================== UI element creation ====================

    #[test]
    fn test_create_ui_entities_have_name_but_no_transform() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;

        let label = create_ui_label(&mut world, &mut sel, &mut counter);
        let panel = create_ui_panel(&mut world, &mut sel, &mut counter);
        let button = create_ui_button(&mut world, &mut sel, &mut counter);

        for &e in &[label, panel, button] {
            assert!(world.get::<Name>(e).is_some(), "UI entities are named");
            assert!(
                world.get::<common::Transform2D>(e).is_none(),
                "UI entities are screen-space: anchor+offset is their placement model"
            );
            assert!(world.get::<GlobalTransform2D>(e).is_none());
        }
        assert!(world.get::<UiLabel>(label).is_some());
        assert!(world.get::<UiPanel>(panel).is_some());
        assert!(world.get::<UiButton>(button).is_some());
        assert_eq!(sel.primary(), Some(button), "last created is selected");
    }

    #[test]
    fn test_handle_create_action_dispatches_ui_labels() {
        let (mut world, mut sel) = (World::new(), Selection::new());
        let mut counter = 0;

        for action in ["Create UI Label", "Create UI Panel", "Create UI Button"] {
            let created = handle_create_action(action, &mut world, &mut sel, Vec2::ZERO, &mut counter);
            assert!(created.is_some(), "{action} must create an entity");
        }
        assert_eq!(counter, 3);
    }
}
