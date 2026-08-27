use super::*;

#[test]
fn test_aabb_contains_point() {
    let aabb = AABB::new(Vec2::new(-10.0, -10.0), Vec2::new(10.0, 10.0));

    assert!(aabb.contains_point(Vec2::ZERO));
    assert!(aabb.contains_point(Vec2::new(5.0, 5.0)));
    assert!(aabb.contains_point(Vec2::new(-10.0, -10.0))); // On edge
    assert!(!aabb.contains_point(Vec2::new(15.0, 0.0)));
    assert!(!aabb.contains_point(Vec2::new(0.0, 15.0)));
}

#[test]
fn test_aabb_intersects() {
    let aabb1 = AABB::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
    let aabb2 = AABB::new(Vec2::new(5.0, 5.0), Vec2::new(15.0, 15.0));
    let aabb3 = AABB::new(Vec2::new(20.0, 20.0), Vec2::new(30.0, 30.0));

    assert!(aabb1.intersects(&aabb2)); // Overlapping
    assert!(!aabb1.intersects(&aabb3)); // Not overlapping
}

#[test]
fn test_aabb_from_position_size() {
    let aabb = AABB::from_position_size(Vec2::new(10.0, 20.0), Vec2::new(6.0, 4.0));

    assert_eq!(aabb.center(), Vec2::new(10.0, 20.0));
    assert_eq!(aabb.size(), Vec2::new(6.0, 4.0));
    assert_eq!(aabb.min, Vec2::new(7.0, 18.0));
    assert_eq!(aabb.max, Vec2::new(13.0, 22.0));
}

#[test]
fn test_aabb_expand() {
    let aabb = AABB::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
    let expanded = aabb.expand(5.0);

    assert_eq!(expanded.min, Vec2::new(-5.0, -5.0));
    assert_eq!(expanded.max, Vec2::new(15.0, 15.0));
}

#[test]
fn test_pick_single_entity() {
    let mut picker = EntityPicker::new();
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    let entities = vec![PickableEntity::new(
        EntityId::with_generation(1, 1),
        Vec2::new(0.0, 0.0),
        Vec2::new(50.0, 50.0),
        0.0,
    )];

    // Click at viewport center (world origin)
    let result = picker.pick_at_screen_pos(&viewport, Vec2::new(400.0, 300.0), &entities);

    assert_eq!(result.len(), 1);
    assert_eq!(result.topmost(), Some(EntityId::with_generation(1, 1)));
}

#[test]
fn test_flip_scaled_sprite_is_picked_at_its_visual_bounds() {
    // A sprite flipped via negative scale has a negative pickable size;
    // the AABB must use the absolute size or min > max and every click
    // misses (regression: flipped sprites were unclickable).
    let mut picker = EntityPicker::new();
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    let entities = vec![PickableEntity::new(
        EntityId::with_generation(1, 1),
        Vec2::new(0.0, 0.0),
        Vec2::new(-50.0, 50.0),
        0.0,
    )];

    let result = picker.pick_at_screen_pos(&viewport, Vec2::new(400.0, 300.0), &entities);
    assert_eq!(result.topmost(), Some(EntityId::with_generation(1, 1)));
}

#[test]
fn test_pick_miss() {
    let mut picker = EntityPicker::new();
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    let entities = vec![PickableEntity::new(
        EntityId::with_generation(1, 1),
        Vec2::new(100.0, 100.0), // Entity at (100, 100)
        Vec2::new(10.0, 10.0),
        0.0,
    )];

    // Click at viewport center (world origin) - should miss
    let result = picker.pick_at_screen_pos(&viewport, Vec2::new(400.0, 300.0), &entities);

    assert!(result.is_empty());
}

#[test]
fn test_pick_depth_sorting() {
    let mut picker = EntityPicker::new();
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    let entities = vec![
        PickableEntity::new(EntityId::with_generation(1, 1), Vec2::ZERO, Vec2::new(50.0, 50.0), 0.0),
        PickableEntity::new(EntityId::with_generation(2, 1), Vec2::ZERO, Vec2::new(50.0, 50.0), 10.0), // Higher depth
        PickableEntity::new(EntityId::with_generation(3, 1), Vec2::ZERO, Vec2::new(50.0, 50.0), 5.0),
    ];

    let result = picker.pick_at_screen_pos(&viewport, Vec2::new(400.0, 300.0), &entities);

    assert_eq!(result.len(), 3);
    // Should be sorted by depth, highest first
    assert_eq!(result.hits[0], EntityId::with_generation(2, 1)); // depth 10
    assert_eq!(result.hits[1], EntityId::with_generation(3, 1)); // depth 5
    assert_eq!(result.hits[2], EntityId::with_generation(1, 1)); // depth 0
}

#[test]
fn test_pick_in_rect() {
    let picker = EntityPicker::new();
    let mut viewport = SceneViewport::new();
    viewport.set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    let entities = vec![
        PickableEntity::new(
            EntityId::with_generation(1, 1),
            Vec2::new(-50.0, 50.0),
            Vec2::new(10.0, 10.0),
            0.0,
        ),
        PickableEntity::new(
            EntityId::with_generation(2, 1),
            Vec2::new(50.0, -50.0),
            Vec2::new(10.0, 10.0),
            0.0,
        ),
        PickableEntity::new(
            EntityId::with_generation(3, 1),
            Vec2::new(200.0, 200.0), // Outside rect
            Vec2::new(10.0, 10.0),
            0.0,
        ),
    ];

    // Select rectangle around entities 1 and 2 (but not 3)
    // Screen coords: top-left to bottom-right
    let result = picker.pick_in_screen_rect(
        &viewport,
        Vec2::new(300.0, 200.0), // Screen top-left
        Vec2::new(500.0, 400.0), // Screen bottom-right
        &entities,
    );

    assert_eq!(result.len(), 2);
    assert!(result.hits.contains(&EntityId::with_generation(1, 1)));
    assert!(result.hits.contains(&EntityId::with_generation(2, 1)));
    assert!(!result.hits.contains(&EntityId::with_generation(3, 1)));
}

#[test]
fn test_selection_rect_normalized() {
    let mut rect = SelectionRect::new();
    rect.begin(Vec2::new(100.0, 200.0));
    rect.update(Vec2::new(50.0, 150.0)); // End is before start

    let (min, max) = rect.normalized();
    assert_eq!(min, Vec2::new(50.0, 150.0));
    assert_eq!(max, Vec2::new(100.0, 200.0));
}

#[test]
fn test_selection_rect_is_drag() {
    let mut rect = SelectionRect::new();
    rect.begin(Vec2::new(100.0, 100.0));

    // Small movement - not a drag
    rect.update(Vec2::new(102.0, 102.0));
    assert!(!rect.is_drag(5.0));

    // Large movement - is a drag
    rect.update(Vec2::new(150.0, 150.0));
    assert!(rect.is_drag(5.0));
}
