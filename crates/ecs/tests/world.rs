use ecs::prelude::*;

#[test]
fn test_world_creation() {
    // Test creating a new world
    let world = World::new();

    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.system_count(), 0);
}

#[test]
fn test_entity_creation_and_removal() {
    // Test creating and removing entities
    let mut world = World::new();

    // Create an entity
    let entity_id = world.create_entity();

    assert_eq!(world.entity_count(), 1);

    // Remove the entity
    let result = world.remove_entity(&entity_id);

    assert!(result.is_ok());
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn test_component_management() {
    // Test adding, getting, and removing components
    let mut world = World::new();
    let entity_id = world.create_entity();

    // Define a simple test component
    #[derive(Debug)]
    struct TestComponent {
        #[allow(dead_code)]
        value: i32,
    }

    // Add the component to the entity
    let result = world.add_component(&entity_id, TestComponent { value: 42 });

    assert!(result.is_ok());

    // Check if the entity has the component
    let has_component = world.has_component::<TestComponent>(&entity_id);

    assert!(has_component.is_ok());
    assert!(has_component.unwrap());

    // Remove the component
    let result = world.remove_component::<TestComponent>(&entity_id);

    assert!(result.is_ok());

    // Check if the entity still has the component
    let has_component = world.has_component::<TestComponent>(&entity_id);

    assert!(has_component.is_ok());
    assert!(!has_component.unwrap());
}

#[test]
fn test_system_management() {
    // Test adding and updating systems
    let mut world = World::new();

    // Create a simple system
    struct TestSystem {
        update_count: usize,
    }

    impl System for TestSystem {
        fn update(&mut self, _world: &mut World, _delta_time: f32) {
            self.update_count += 1;
        }

        fn name(&self) -> &str {
            "TestSystem"
        }
    }

    // Add the system to the world
    world.add_system(TestSystem { update_count: 0 });

    assert_eq!(world.system_count(), 1);

    // Update the systems
    let _ = world.update(0.016);

    // Note: We can't directly check the update_count since we don't have access to the system after it's added
}

#[test]
fn test_world_initialization() {
    // Test initializing the world through the init function
    let result = init();

    assert!(result.is_ok());
    let world = result.unwrap();
    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.system_count(), 0);
}

#[test]
fn test_hierarchy_cycle_detection() {
    // Test that setting a parent that would create a cycle is rejected
    let mut world = World::new();

    // Create a chain: grandparent -> parent -> child
    let grandparent = world.create_entity();
    let parent = world.create_entity();
    let child = world.create_entity();

    // Set up the hierarchy
    assert!(world.set_parent(parent, grandparent).is_ok());
    assert!(world.set_parent(child, parent).is_ok());

    // Attempt to create a cycle: grandparent -> child (child is an ancestor of grandparent via parent)
    // This would create: child -> parent -> grandparent -> child (cycle!)
    let result = world.set_parent(grandparent, child);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cycle"));
}

#[test]
fn test_hierarchy_self_parent_rejected() {
    // Test that an entity cannot be its own parent
    let mut world = World::new();

    let entity = world.create_entity();

    // Attempt to set entity as its own parent
    let result = world.set_parent(entity, entity);

    // This should be rejected - self-parenting creates a trivial cycle
    assert!(result.is_err());
}

#[test]
fn test_query_entities() {
    use ecs::{Single, Pair};
    use ecs::sprite_components::{Transform2D, Sprite};

    let mut world = World::new();

    // Create entities with different component combinations
    let entity_with_transform = world.create_entity();
    world.add_component(&entity_with_transform, Transform2D::default()).unwrap();

    let entity_with_sprite = world.create_entity();
    world.add_component(&entity_with_sprite, Sprite::new(0)).unwrap();

    let entity_with_both = world.create_entity();
    world.add_component(&entity_with_both, Transform2D::default()).unwrap();
    world.add_component(&entity_with_both, Sprite::new(0)).unwrap();

    let entity_with_nothing = world.create_entity();

    // Query for entities with Transform2D
    let with_transform = world.query_entities::<Single<Transform2D>>();
    assert_eq!(with_transform.len(), 2);
    assert!(with_transform.contains(&entity_with_transform));
    assert!(with_transform.contains(&entity_with_both));
    assert!(!with_transform.contains(&entity_with_sprite));
    assert!(!with_transform.contains(&entity_with_nothing));

    // Query for entities with Sprite
    let with_sprite = world.query_entities::<Single<Sprite>>();
    assert_eq!(with_sprite.len(), 2);
    assert!(with_sprite.contains(&entity_with_sprite));
    assert!(with_sprite.contains(&entity_with_both));

    // Query for entities with both Transform2D and Sprite
    let with_both = world.query_entities::<Pair<Transform2D, Sprite>>();
    assert_eq!(with_both.len(), 1);
    assert!(with_both.contains(&entity_with_both));
}

#[test]
fn test_world_clear_removes_entities_and_components() {
    use ecs::sprite_components::Transform2D;

    let mut world = World::new();
    let e1 = world.create_entity();
    let e2 = world.create_entity();
    world.add_component(&e1, Transform2D::default()).unwrap();
    world.add_component(&e2, Transform2D::default()).unwrap();

    assert_eq!(world.entity_count(), 2);

    world.clear();

    assert_eq!(world.entity_count(), 0);
    assert!(world.get::<Transform2D>(e1).is_none());
    assert!(world.get::<Transform2D>(e2).is_none());
}

#[test]
fn test_create_entity_with_id_preserves_id() {
    let mut world = World::new();
    let id = EntityId::with_generation(42, 3);
    let created = world.create_entity_with_id(id);

    assert_eq!(created, id);
    assert_eq!(created.value(), 42);
    assert_eq!(created.generation(), 3);
    assert_eq!(world.entity_count(), 1);
    assert!(world.get_entity(&id).is_ok());
}

#[test]
fn test_clear_then_create_entity_with_id() {
    use ecs::sprite_components::Transform2D;

    let mut world = World::new();
    let original = world.create_entity();
    world.add_component(&original, Transform2D::new(glam::Vec2::new(10.0, 20.0))).unwrap();

    world.clear();

    // Recreate with same ID
    let restored = world.create_entity_with_id(original);
    world.add_component(&restored, Transform2D::new(glam::Vec2::new(30.0, 40.0))).unwrap();

    assert_eq!(world.entity_count(), 1);
    let t = world.get::<Transform2D>(restored).unwrap();
    assert_eq!(t.position, glam::Vec2::new(30.0, 40.0));
}

// === EntityBuilder (world.spawn()) tests ===

#[test]
fn test_spawn_creates_entity() {
    let mut world = World::new();
    let entity = world.spawn().id();

    assert_eq!(world.entity_count(), 1);
    assert!(world.get_entity(&entity).is_ok());
}

#[test]
fn test_spawn_with_single_component() {
    use ecs::sprite_components::Transform2D;

    let mut world = World::new();
    let entity = world.spawn()
        .with(Transform2D::new(glam::Vec2::new(10.0, 20.0)))
        .id();

    assert!(world.has_component::<Transform2D>(&entity).unwrap());
    let t = world.get::<Transform2D>(entity).unwrap();
    assert_eq!(t.position, glam::Vec2::new(10.0, 20.0));
}

#[test]
fn test_spawn_with_multiple_components() {
    use ecs::sprite_components::{Transform2D, Sprite};

    let mut world = World::new();
    let entity = world.spawn()
        .with(Transform2D::new(glam::Vec2::new(5.0, 5.0)))
        .with(Sprite::new(42))
        .id();

    assert!(world.has_component::<Transform2D>(&entity).unwrap());
    assert!(world.has_component::<Sprite>(&entity).unwrap());
    assert_eq!(world.get::<Sprite>(entity).unwrap().texture_handle, 42);
}

#[test]
fn test_spawn_returns_correct_entity_id() {
    use ecs::sprite_components::Transform2D;

    let mut world = World::new();
    let e1 = world.spawn()
        .with(Transform2D::new(glam::Vec2::new(1.0, 0.0)))
        .id();
    let e2 = world.spawn()
        .with(Transform2D::new(glam::Vec2::new(2.0, 0.0)))
        .id();

    assert_ne!(e1, e2);
    assert_eq!(world.get::<Transform2D>(e1).unwrap().position.x, 1.0);
    assert_eq!(world.get::<Transform2D>(e2).unwrap().position.x, 2.0);
}

#[test]
fn test_spawn_multiple_entities_independent() {
    use ecs::sprite_components::{Transform2D, Sprite};

    let mut world = World::new();
    let e1 = world.spawn()
        .with(Transform2D::new(glam::Vec2::ZERO))
        .id();
    let e2 = world.spawn()
        .with(Sprite::new(7))
        .id();

    assert_eq!(world.entity_count(), 2);
    assert!(world.has_component::<Transform2D>(&e1).unwrap());
    assert!(!world.has_component::<Sprite>(&e1).unwrap());
    assert!(!world.has_component::<Transform2D>(&e2).unwrap());
    assert!(world.has_component::<Sprite>(&e2).unwrap());
}

// --- Stale entity ID rejection (generation validation in component ops) ---

#[test]
fn test_stale_entity_id_rejected_by_component_ops() {
    #[derive(Debug)]
    struct Health(i32);

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Health(10)).unwrap();
    assert_eq!(world.get::<Health>(entity).map(|h| h.0), Some(10));
    world.remove_entity(&entity).unwrap();

    // The retained ID is now stale: every component operation must refuse it
    assert!(world.add_component(&entity, Health(5)).is_err());
    assert!(world.remove_component::<Health>(&entity).is_err());
    assert!(world.has_component::<Health>(&entity).is_err());
    assert!(world.get::<Health>(entity).is_none());
    assert!(world.get_mut::<Health>(entity).is_none());
}

#[test]
fn test_snapshot_restore_revives_entity_id() {
    #[derive(Debug)]
    struct Marker;

    let mut world = World::new();
    let entity = world.create_entity();
    world.remove_entity(&entity).unwrap();

    // Snapshot-restore contract: clear, then re-create with the same ID
    world.clear();
    let restored = world.create_entity_with_id(entity);

    assert_eq!(restored, entity);
    assert!(world.validate_entity(&entity).is_ok());
    assert!(world.add_component(&entity, Marker).is_ok());
    assert!(world.get::<Marker>(entity).is_some());
}

// --- Hierarchy cleanup on entity removal ---

#[test]
fn test_remove_entity_unlinks_from_parent_children() {
    let mut world = World::new();
    let parent = world.create_entity();
    let child = world.create_entity();
    world.set_parent(child, parent).unwrap();

    world.remove_entity(&child).unwrap();

    let children = world.get_children(parent).unwrap_or(&[]);
    assert!(
        !children.contains(&child),
        "removed child must not linger in parent's Children list"
    );
}

#[test]
fn test_remove_parent_entity_orphans_children_to_root() {
    let mut world = World::new();
    let parent = world.create_entity();
    let child_a = world.create_entity();
    let child_b = world.create_entity();
    world.set_parent(child_a, parent).unwrap();
    world.set_parent(child_b, parent).unwrap();

    world.remove_entity(&parent).unwrap();

    assert_eq!(world.get_parent(child_a), None, "child must not keep a dangling Parent");
    assert_eq!(world.get_parent(child_b), None, "child must not keep a dangling Parent");
    let roots = world.get_root_entities();
    assert!(roots.contains(&child_a));
    assert!(roots.contains(&child_b));
}

#[test]
fn test_remove_middle_of_chain_orphans_grandchild() {
    let mut world = World::new();
    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    world.set_parent(b, a).unwrap();
    world.set_parent(c, b).unwrap();

    world.remove_entity(&b).unwrap();

    assert_eq!(world.get_parent(c), None, "grandchild becomes a root");
    assert!(world.get_children(a).unwrap_or(&[]).is_empty(), "a's children list is empty");
    let roots = world.get_root_entities();
    assert!(roots.contains(&a));
    assert!(roots.contains(&c));
}

#[test]
fn test_remove_entity_hierarchy_deep_chain_leaves_no_residue() {
    let mut world = World::new();
    let root = world.create_entity();
    let mut current = root;
    let mut all = vec![root];
    for _ in 0..100 {
        let child = world.create_entity();
        world.set_parent(child, current).unwrap();
        all.push(child);
        current = child;
    }

    world.remove_entity_hierarchy(&root).unwrap();

    assert_eq!(world.entity_count(), 0);
    for id in &all {
        assert!(world.validate_entity(id).is_err(), "entity {} should be dead", id.value());
    }
}

#[test]
fn test_deep_hierarchy_ancestor_descendant_queries() {
    let mut world = World::new();
    let root = world.create_entity();
    let mut current = root;
    for _ in 0..50 {
        let child = world.create_entity();
        world.set_parent(child, current).unwrap();
        current = child;
    }
    let leaf = current;

    assert!(world.is_ancestor_of(root, leaf));
    assert!(world.is_descendant_of(leaf, root));
    assert!(!world.is_ancestor_of(leaf, root));
    assert!(!world.is_descendant_of(root, leaf));
    assert_eq!(world.get_ancestors(leaf).len(), 50);
    assert_eq!(world.get_descendants(root).len(), 50);
}

#[test]
fn test_component_types_reports_concrete_type_names() {
    use ecs::Transform2D;

    struct EnemyAi;

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::default()).unwrap();
    world.add_component(&entity, EnemyAi).unwrap();

    let types = world.component_types(entity);
    assert_eq!(types.len(), 2);
    // Names must be the concrete component types, never the Box's own name
    // (the blanket Component impl on Box<dyn Component> would report that).
    assert!(types.iter().any(|(_, name)| name.contains("Transform2D")));
    assert!(types.iter().any(|(_, name)| name.contains("EnemyAi")));
    assert!(
        types.iter().all(|(_, name)| !name.contains("Box<")),
        "type names must come from the concrete component, got {:?}",
        types
    );
}

#[test]
fn test_component_types_empty_for_bare_entity() {
    let mut world = World::new();
    let entity = world.create_entity();
    assert!(world.component_types(entity).is_empty());

    world.remove_entity(&entity).unwrap();
    assert!(world.component_types(entity).is_empty(), "dead entity reports no components");
}

#[test]
fn test_component_types_reflects_removal() {
    use ecs::{Sprite, Transform2D};

    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::default()).unwrap();
    world.add_component(&entity, Sprite::default()).unwrap();
    assert_eq!(world.component_types(entity).len(), 2);

    world.remove_component::<Sprite>(&entity).unwrap();
    let types = world.component_types(entity);
    assert_eq!(types.len(), 1);
    assert!(types[0].1.contains("Transform2D"));
}
