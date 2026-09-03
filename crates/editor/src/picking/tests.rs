//! Picking contracts: the order hits come back in (it becomes the
//! selection's primary), repeat-click cycling through a stack, marquee
//! selection through the screen→world mapping, and flip-scaled sprites
//! staying clickable at their visual bounds.

use super::*;
use crate::test_support::{entity, pickable, test_viewport};

const SIZE: Vec2 = Vec2::new(50.0, 50.0);

/// The box math every pick and marquee test above relies on: a box built
/// from a center and size has the expected edges, contains its edges but
/// not the outside, intersects on overlap only, and expands symmetrically
/// (the pick margin).
#[test]
fn test_aabb_edges_contain_intersect_and_expand_as_marquee_picking_assumes() {
    let box_ = AABB::from_position_size(Vec2::new(10.0, 20.0), Vec2::new(6.0, 4.0));
    assert_eq!((box_.min, box_.max), (Vec2::new(7.0, 18.0), Vec2::new(13.0, 22.0)));
    assert_eq!((box_.center(), box_.size()), (Vec2::new(10.0, 20.0), Vec2::new(6.0, 4.0)));

    let unit = AABB::new(Vec2::new(-10.0, -10.0), Vec2::new(10.0, 10.0));
    assert!(unit.contains_point(Vec2::ZERO));
    assert!(unit.contains_point(Vec2::new(-10.0, -10.0)), "an edge point is inside");
    assert!(!unit.contains_point(Vec2::new(15.0, 0.0)), "outside on x");
    assert!(!unit.contains_point(Vec2::new(0.0, 15.0)), "outside on y");

    let origin = AABB::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
    assert!(origin.intersects(&AABB::new(Vec2::new(5.0, 5.0), Vec2::new(15.0, 15.0))), "overlap");
    assert!(!origin.intersects(&AABB::new(Vec2::new(20.0, 20.0), Vec2::new(30.0, 30.0))), "disjoint");

    let expanded = origin.expand(5.0);
    assert_eq!((expanded.min, expanded.max), (Vec2::new(-5.0, -5.0), Vec2::new(15.0, 15.0)));
}
/// The screen point over the world origin in [`test_viewport`].
const OVER_ORIGIN: Vec2 = Vec2::new(400.0, 300.0);

#[test]
fn test_click_hits_sort_front_to_back_then_by_id_and_repeat_clicks_cycle_the_stack() {
    let mut picker = EntityPicker::new();
    let viewport = test_viewport();
    let stack = [
        pickable(1, Vec2::ZERO, SIZE, 0.0),
        pickable(9, Vec2::ZERO, SIZE, 5.0),
        pickable(2, Vec2::ZERO, SIZE, 10.0),
        pickable(4, Vec2::ZERO, SIZE, 5.0),
    ];

    let first = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN, &stack);

    // Highest depth first; equal depths order by id so the primary is never
    // left to an unstable sort.
    assert_eq!(first.hits, vec![entity(2), entity(4), entity(9), entity(1)]);

    // Clicking the same spot again walks down the stack.
    let second = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN, &stack);
    assert_eq!(second.topmost(), Some(entity(4)), "a repeat click cycles to the next entity");
    let third = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN, &stack);
    assert_eq!(third.topmost(), Some(entity(9)));
    picker.reset_cycle();
    let reset = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN, &stack);
    assert_eq!(reset.topmost(), Some(entity(2)), "a selection change restarts from the front");
}

#[test]
fn test_marquee_selects_what_it_overlaps_in_the_same_order_as_a_click() {
    let picker = EntityPicker::new();
    let viewport = test_viewport();
    let entities = [
        pickable(9, Vec2::new(-50.0, 50.0), SIZE, 5.0),
        pickable(2, Vec2::new(50.0, -50.0), SIZE, 5.0),
        pickable(4, Vec2::ZERO, SIZE, 8.0),
        pickable(3, Vec2::new(200.0, 200.0), Vec2::new(10.0, 10.0), 9.0),
    ];

    // Screen top-left → bottom-right is world (-100, 100) → (100, -100).
    let result = picker.pick_in_screen_rect(&viewport, Vec2::new(300.0, 200.0), Vec2::new(500.0, 400.0), &entities);

    assert_eq!(result.hits, vec![entity(4), entity(2), entity(9)], "inside entities front to back, ties by id; the far one is out");
}

#[test]
fn test_flip_scaled_sprite_is_picked_at_its_visual_bounds() {
    // A sprite flipped via negative scale has a negative pickable size;
    // the AABB must use the absolute size or min > max and every click
    // misses (regression: flipped sprites were unclickable).
    let mut picker = EntityPicker::new();
    let viewport = test_viewport();
    let flipped = [pickable(1, Vec2::ZERO, Vec2::new(-50.0, 50.0), 0.0)];

    let hit = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN, &flipped);
    assert_eq!(hit.topmost(), Some(entity(1)));

    let edge = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN + Vec2::new(25.0, 0.0), &flipped);
    assert_eq!(edge.topmost(), Some(entity(1)), "the visual edge is inside the pick margin");
    let miss = picker.pick_at_screen_pos(&viewport, OVER_ORIGIN + Vec2::new(100.0, 0.0), &flipped);
    assert!(miss.is_empty(), "a click well outside the sprite misses");
}
