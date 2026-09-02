//! Public-API contracts of `World`: entity identity and generations, the
//! hierarchy's cleanup on removal, typed queries, the entity builder, the
//! per-frame event bus, and the concrete-name guard for boxed components.

use ecs::prelude::*;
use ecs::{Pair, Single, Sprite, Transform2D};
use glam::Vec2;

#[derive(Debug, PartialEq)]
struct Health(i32);

#[test]
fn test_stale_entity_id_rejected_by_component_ops() -> Result<(), EcsError> {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Health(10))?;
    assert_eq!(world.get::<Health>(entity), Some(&Health(10)));

    world.remove_entity(&entity)?;

    // The retained id is now stale: every component operation must refuse it.
    assert!(world.add_component(&entity, Health(5)).is_err());
    assert!(world.remove_component::<Health>(&entity).is_err());
    assert!(world.has_component::<Health>(&entity).is_err());
    assert_eq!(world.get::<Health>(entity), None);
    assert!(world.get_mut::<Health>(entity).is_none());
    assert!(world.validate_entity(&entity).is_err());
    Ok(())
}

#[test]
fn test_snapshot_restore_revives_entity_id() -> Result<(), EcsError> {
    let mut world = World::new();
    let entity = world.create_entity();
    world.add_component(&entity, Health(10))?;
    let other = world.create_entity();
    world.add_component(&other, Transform2D::default())?;

    // The WorldSnapshot restore contract: clear, then re-create by id.
    world.clear();
    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.get::<Health>(entity), None, "clear drops components too");
    assert_eq!(world.get::<Transform2D>(other), None);

    let restored = world.create_entity_with_id(entity);

    assert_eq!(restored, entity, "the same (id, generation) is live again");
    assert!(world.validate_entity(&entity).is_ok());
    world.add_component(&entity, Health(3))?;
    assert_eq!(world.get::<Health>(entity), Some(&Health(3)));

    // A reference to the same slot from another generation stays refused.
    let wrong_generation = EntityId::with_generation(entity.value(), entity.generation() + 1);
    assert!(world.validate_entity(&wrong_generation).is_err());
    assert_eq!(world.get::<Health>(wrong_generation), None);
    Ok(())
}

#[test]
fn test_set_parent_rejects_cycles_and_names_the_cycle() -> Result<(), EcsError> {
    let mut world = World::new();
    let grandparent = world.create_entity();
    let parent = world.create_entity();
    let child = world.create_entity();
    world.set_parent(parent, grandparent)?;
    world.set_parent(child, parent)?;

    // grandparent -> child would close child -> parent -> grandparent -> child.
    let cycle = world.set_parent(grandparent, child);

    let message = cycle.expect_err("a cycle must be refused").to_string();
    assert!(message.contains("cycle"), "the error names the cycle: {message}");
    assert!(world.set_parent(child, child).is_err(), "self-parenting is the trivial cycle");
    assert_eq!(world.get_parent(grandparent), None, "a refused link changes nothing");
    Ok(())
}

#[test]
fn test_remove_parent_entity_orphans_children_to_root() -> Result<(), EcsError> {
    let mut world = World::new();
    let root = world.create_entity();
    let parent = world.create_entity();
    let child_a = world.create_entity();
    let child_b = world.create_entity();
    world.set_parent(parent, root)?;
    world.set_parent(child_a, parent)?;
    world.set_parent(child_b, parent)?;

    world.remove_entity(&parent)?;

    assert_eq!(world.get_parent(child_a), None, "no dangling Parent");
    assert_eq!(world.get_parent(child_b), None, "no dangling Parent");
    assert_eq!(world.get_children(root).unwrap_or(&[]), [], "the removed entity leaves its parent's list");
    let roots = world.get_root_entities();
    assert!(roots.contains(&root));
    assert!(roots.contains(&child_a));
    assert!(roots.contains(&child_b));

    // Removing a leaf prunes it from the parent's list the same way.
    world.set_parent(child_a, root)?;
    world.remove_entity(&child_a)?;
    assert_eq!(world.get_children(root).unwrap_or(&[]), []);
    Ok(())
}

#[test]
fn test_remove_entity_hierarchy_deep_chain_leaves_no_residue() -> Result<(), EcsError> {
    let mut world = World::new();
    let root = world.create_entity();
    let mut current = root;
    let mut all = vec![root];
    for _ in 0..100 {
        let child = world.create_entity();
        world.set_parent(child, current)?;
        all.push(child);
        current = child;
    }
    let leaf = current;
    assert!(world.is_ancestor_of(root, leaf));
    assert!(world.is_descendant_of(leaf, root));
    assert!(!world.is_ancestor_of(leaf, root));
    assert_eq!(world.get_ancestors(leaf).len(), 100);
    assert_eq!(world.get_descendants(root).len(), 100);

    world.remove_entity_hierarchy(&root)?;

    assert_eq!(world.entity_count(), 0);
    for id in &all {
        assert!(world.validate_entity(id).is_err(), "entity {} should be dead", id.value());
    }
    Ok(())
}

#[test]
fn test_component_types_reports_concrete_type_names() -> Result<(), EcsError> {
    struct EnemyAi;

    let mut world = World::new();
    let entity = world.create_entity();
    assert!(world.component_types(entity).is_empty(), "a bare entity reports nothing");
    world.add_component(&entity, Transform2D::default())?;
    world.add_component(&entity, EnemyAi)?;

    let types = world.component_types(entity);

    // Names must be the concrete component types, never the Box's own name
    // (the blanket Component impl on Box<dyn Component> would report that).
    assert_eq!(types.len(), 2);
    assert!(types.iter().any(|(_, name)| name.contains("Transform2D")));
    assert!(types.iter().any(|(_, name)| name.contains("EnemyAi")));
    assert!(
        types.iter().all(|(_, name)| !name.contains("Box<")),
        "type names must come from the concrete component, got {types:?}"
    );

    // The report follows removal, and a dead entity reports nothing.
    world.remove_component::<EnemyAi>(&entity)?;
    let types = world.component_types(entity);
    assert_eq!(types.len(), 1);
    assert!(types[0].1.contains("Transform2D"));
    world.remove_entity(&entity)?;
    assert!(world.component_types(entity).is_empty());
    Ok(())
}

#[test]
fn test_typed_queries_select_exactly_the_entities_with_every_listed_component() -> Result<(), EcsError> {
    let mut world = World::new();
    let with_transform = world.create_entity();
    world.add_component(&with_transform, Transform2D::default())?;
    let with_sprite = world.create_entity();
    world.add_component(&with_sprite, Sprite::new(0))?;
    let with_both = world.create_entity();
    world.add_component(&with_both, Transform2D::default())?;
    world.add_component(&with_both, Sprite::new(0))?;
    let _with_nothing = world.create_entity();

    let mut transforms = world.query_entities::<Single<Transform2D>>();
    let mut sprites = world.query_entities::<Single<Sprite>>();
    let pairs = world.query_entities::<Pair<Transform2D, Sprite>>();

    let by_id = |id: &EntityId| id.value();
    transforms.sort_by_key(by_id);
    sprites.sort_by_key(by_id);
    let mut expected_transforms = [with_transform, with_both];
    expected_transforms.sort_by_key(by_id);
    let mut expected_sprites = [with_sprite, with_both];
    expected_sprites.sort_by_key(by_id);
    assert_eq!(transforms, expected_transforms);
    assert_eq!(sprites, expected_sprites);
    assert_eq!(pairs, [with_both]);
    Ok(())
}

#[test]
fn test_spawn_attaches_every_with_component_and_keeps_entities_independent() -> Result<(), EcsError> {
    let mut world = World::new();

    let first = world.spawn().with(Transform2D::new(Vec2::new(10.0, 20.0))).with(Sprite::new(42)).id();
    let second = world.spawn().with(Sprite::new(7)).id();

    assert_ne!(first, second);
    assert_eq!(world.entity_count(), 2);
    assert_eq!(world.get::<Transform2D>(first).map(|t| t.position), Some(Vec2::new(10.0, 20.0)));
    assert_eq!(world.get::<Sprite>(first).map(|s| s.texture_handle), Some(42));
    assert!(!world.has_component::<Transform2D>(&second)?);
    assert_eq!(world.get::<Sprite>(second).map(|s| s.texture_handle), Some(7));
    Ok(())
}

#[test]
fn test_events_stay_readable_until_flush_then_the_next_frame_starts_empty() {
    #[derive(Debug, PartialEq)]
    struct Collision(u32);

    let mut world = World::new();
    world.emit_event(Collision(1));
    world.emit_event(Collision(2));

    // Every consumer in the frame reads the same list: reading never drains.
    assert_eq!(world.read_events::<Collision>(), [Collision(1), Collision(2)]);
    assert_eq!(world.read_events::<Collision>(), [Collision(1), Collision(2)]);

    // The engine flushes at the frame boundary; nothing leaks into the next frame.
    world.flush_events();
    assert!(world.read_events::<Collision>().is_empty());
    world.emit_event(Collision(3));
    assert_eq!(world.read_events::<Collision>(), [Collision(3)]);
}
