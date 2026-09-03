//! Contracts of the dirty-flagged transform hierarchy propagation:
//! clean frames recompute nothing, changes
//! recompute exactly the affected subtree, and the baseline cache stays
//! consistent through reparenting, deletion, disable/enable, and reset.

use ecs::{
    EcsError, EntityId, GlobalTransform2D, System, Transform2D, TransformHierarchySystem, World,
    WorldHierarchyExt,
};
use glam::Vec2;

fn spawn(world: &mut World, position: Vec2) -> Result<EntityId, EcsError> {
    let entity = world.create_entity();
    world.add_component(&entity, Transform2D::new(position))?;
    world.add_component(&entity, GlobalTransform2D::default())?;
    Ok(entity)
}

fn global_pos(world: &World, entity: EntityId) -> Vec2 {
    world.get::<GlobalTransform2D>(entity).expect("global transform").position
}

#[test]
fn test_no_change_second_frame_recomputes_zero() -> Result<(), EcsError> {
    let mut world = World::new();
    let parent = spawn(&mut world, Vec2::new(100.0, 0.0))?;
    let child = spawn(&mut world, Vec2::new(50.0, 0.0))?;
    world.set_parent(child, parent)?;
    let mut system = TransformHierarchySystem::new();

    system.update(&mut world, 0.016);
    assert_eq!(system.recomputed_last_update(), 2, "first frame computes everything");

    system.update(&mut world, 0.016);
    assert_eq!(system.recomputed_last_update(), 0, "clean frame must recompute nothing");
    assert_eq!(system.visited_last_update(), 2, "clean entities are still dirty-checked");
    assert_eq!(global_pos(&world, child), Vec2::new(150.0, 0.0), "globals stay correct");

    // reset() discards every baseline: the next frame recomputes all.
    system.reset();
    assert_eq!(system.tracked_entity_count(), 0);
    system.update(&mut world, 0.016);
    assert_eq!(system.recomputed_last_update(), 2);
    assert_eq!(global_pos(&world, child), Vec2::new(150.0, 0.0));
    Ok(())
}

#[test]
fn test_leaf_change_recomputes_one() -> Result<(), EcsError> {
    let mut world = World::new();
    let root = spawn(&mut world, Vec2::new(100.0, 0.0))?;
    let mid = spawn(&mut world, Vec2::new(10.0, 0.0))?;
    let leaf = spawn(&mut world, Vec2::new(1.0, 0.0))?;
    let sibling = spawn(&mut world, Vec2::new(2.0, 0.0))?;
    world.set_parent(mid, root)?;
    world.set_parent(leaf, mid)?;
    world.set_parent(sibling, mid)?;
    let mut system = TransformHierarchySystem::new();
    system.update(&mut world, 0.016);

    world.get_mut::<Transform2D>(leaf).expect("leaf").position = Vec2::new(5.0, 0.0);
    system.update(&mut world, 0.016);

    assert_eq!(system.recomputed_last_update(), 1, "only the mutated leaf recomputes");
    assert_eq!(global_pos(&world, leaf), Vec2::new(115.0, 0.0));
    assert_eq!(global_pos(&world, sibling), Vec2::new(112.0, 0.0), "sibling untouched and correct");
    Ok(())
}

#[test]
fn test_parent_change_recomputes_subtree_only() -> Result<(), EcsError> {
    let mut world = World::new();
    let parent = spawn(&mut world, Vec2::new(100.0, 0.0))?;
    let child_a = spawn(&mut world, Vec2::new(10.0, 0.0))?;
    let child_b = spawn(&mut world, Vec2::new(20.0, 0.0))?;
    world.set_parent(child_a, parent)?;
    world.set_parent(child_b, parent)?;
    // Unrelated tree that must stay clean.
    let other_root = spawn(&mut world, Vec2::new(-100.0, 0.0))?;
    let other_child = spawn(&mut world, Vec2::new(-10.0, 0.0))?;
    world.set_parent(other_child, other_root)?;
    let mut system = TransformHierarchySystem::new();
    system.update(&mut world, 0.016);

    world.get_mut::<Transform2D>(parent).expect("parent").position = Vec2::new(200.0, 0.0);
    system.update(&mut world, 0.016);

    assert_eq!(
        system.recomputed_last_update(),
        3,
        "parent + its two children recompute; the unrelated tree does not"
    );
    assert_eq!(global_pos(&world, child_a), Vec2::new(210.0, 0.0));
    assert_eq!(global_pos(&world, child_b), Vec2::new(220.0, 0.0));
    assert_eq!(global_pos(&world, other_child), Vec2::new(-110.0, 0.0));

    // Reparenting is a parent-link change: the moved child recomputes.
    world.set_parent(child_a, other_root)?;
    system.update(&mut world, 0.016);
    assert_eq!(system.recomputed_last_update(), 1, "only the reparented child is dirty");
    assert_eq!(global_pos(&world, child_a), Vec2::new(-90.0, 0.0));

    // A removed GlobalTransform2D is restored on the next frame.
    world.remove_component::<GlobalTransform2D>(&child_b)?;
    system.update(&mut world, 0.016);
    assert_eq!(system.recomputed_last_update(), 1, "missing global must be restored");
    assert_eq!(global_pos(&world, child_b), Vec2::new(220.0, 0.0));
    Ok(())
}

#[test]
fn test_parent_deletion_orphans_recompute_and_cache_prunes() -> Result<(), EcsError> {
    let mut world = World::new();
    let parent = spawn(&mut world, Vec2::new(100.0, 0.0))?;
    let child = spawn(&mut world, Vec2::new(10.0, 0.0))?;
    world.set_parent(child, parent)?;
    let mut system = TransformHierarchySystem::new();
    system.update(&mut world, 0.016);
    assert_eq!(system.tracked_entity_count(), 2);
    assert_eq!(global_pos(&world, child), Vec2::new(110.0, 0.0));

    // remove_entity auto-detaches hierarchy links (child orphans to root).
    world.remove_entity(&parent)?;
    system.update(&mut world, 0.016);

    assert_eq!(
        system.recomputed_last_update(),
        1,
        "orphan recomputes as a root (its parent link changed)"
    );
    assert_eq!(global_pos(&world, child), Vec2::new(10.0, 0.0), "orphan global = its local");
    assert_eq!(system.tracked_entity_count(), 1, "removed entity pruned from cache");
    Ok(())
}

#[test]
fn test_identical_write_stays_clean() -> Result<(), EcsError> {
    let mut world = World::new();
    let entity = spawn(&mut world, Vec2::new(100.0, 50.0))?;
    let mut system = TransformHierarchySystem::new();
    system.update(&mut world, 0.016);

    // The physics-writeback pattern: get_mut + write the same values (a
    // sleeping body). Value comparison must keep the entity clean.
    let transform = world.get_mut::<Transform2D>(entity).expect("transform");
    transform.position = Vec2::new(100.0, 50.0);
    transform.rotation = 0.0;
    system.update(&mut world, 0.016);

    assert_eq!(
        system.recomputed_last_update(),
        0,
        "writing identical values must not dirty the entity"
    );
    Ok(())
}

#[test]
fn test_reenable_after_disable_catches_stale() -> Result<(), EcsError> {
    let mut world = World::new();
    let entity = spawn(&mut world, Vec2::new(100.0, 0.0))?;
    let mut system = TransformHierarchySystem::new();
    system.update(&mut world, 0.016);

    system.set_enabled(false);
    world.get_mut::<Transform2D>(entity).expect("transform").position = Vec2::new(300.0, 0.0);
    system.update(&mut world, 0.016); // no-op while disabled
    assert_eq!(global_pos(&world, entity), Vec2::new(100.0, 0.0), "disabled = stale global");

    system.set_enabled(true);
    system.update(&mut world, 0.016);

    assert_eq!(system.recomputed_last_update(), 1, "drift detected on re-enable");
    assert_eq!(global_pos(&world, entity), Vec2::new(300.0, 0.0));
    Ok(())
}

#[test]
fn test_hand_written_global_transform_is_discarded_once_the_entity_goes_dirty() -> Result<(), EcsError> {
    // GlobalTransform2D is system-owned: Transform2D is the edit surface.
    // An author who writes the global directly sees it vanish on the next
    // recompute, position, rotation and scale alike.
    let mut world = World::new();
    let entity = spawn(&mut world, Vec2::new(100.0, 0.0))?;
    let mut system = TransformHierarchySystem::new();
    system.update(&mut world, 0.016);

    *world.get_mut::<GlobalTransform2D>(entity).expect("global") =
        GlobalTransform2D::new(Vec2::new(999.0, 999.0), 1.0, Vec2::new(5.0, 5.0));
    world.get_mut::<Transform2D>(entity).expect("transform").position = Vec2::new(150.0, 0.0);
    system.update(&mut world, 0.016);

    let global = world.get::<GlobalTransform2D>(entity).expect("global");
    assert_eq!(global.position, Vec2::new(150.0, 0.0), "the local edit wins");
    assert_eq!(global.rotation, 0.0, "the hand-written rotation is gone");
    assert_eq!(global.scale, Vec2::ONE, "the hand-written scale is gone");
    Ok(())
}
