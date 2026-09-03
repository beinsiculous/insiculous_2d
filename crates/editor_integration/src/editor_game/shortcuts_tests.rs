//! Unified-shortcut behavior (#40): arrow nudge (merge + seal), the Escape
//! cancel cascade, selection roots, and the production Delete/Duplicate
//! paths — driven headlessly against the real `EditorGame` state.

use ecs::{World, WorldHierarchyExt};
use glam::Vec2;

use super::test_support::{drag_state_for, editor_game, position, spawn_at};
use crate::entity_ops::selection_roots;

#[test]
fn test_held_arrow_merges_into_one_undo_entry_sealed_on_release() {
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);

    // A held key repeat: three presses before the release. One world unit
    // per press, ten with Shift.
    for _ in 0..3 {
        game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    }
    game.nudge_selection(&mut world, Vec2::new(0.0, 1.0), true);
    assert_eq!(position(&world, a), Vec2::new(3.0, 10.0));

    // Key release seals the entry (what `on_key_released` calls).
    game.command_history.break_merge();

    // A second hold is a separate entry.
    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    assert_eq!(position(&world, a), Vec2::new(4.0, 10.0));

    // Undo #1: only the second hold reverts.
    assert!(game.command_history.undo(&mut world));
    assert_eq!(position(&world, a), Vec2::new(3.0, 10.0));
    // Undo #2: the whole first hold reverts in one step.
    assert!(game.command_history.undo(&mut world));
    assert_eq!(position(&world, a), Vec2::ZERO);
    assert!(!game.command_history.can_undo(), "exactly two entries for two holds");
}

#[test]
fn test_selection_roots_put_the_primary_first_and_skip_selected_children() {
    // Roots feed every multi-entity move: the primary anchors grid
    // snapping (so it must come first), and a selected child of a selected
    // parent already moves through propagation (so it must be skipped).
    let mut game = editor_game();
    let mut world = World::new();
    let parent = spawn_at(&mut world, Vec2::ZERO);
    let child = spawn_at(&mut world, Vec2::new(5.0, 0.0));
    let lone = spawn_at(&mut world, Vec2::new(50.0, 0.0));
    world.set_parent(child, parent).ok();
    game.editor.selection.add(parent);
    game.editor.selection.add(child);
    game.editor.selection.add(lone);
    game.editor.selection.set_primary(lone);

    let roots = selection_roots(&world, &game.editor.selection);
    assert_eq!(roots, vec![lone, parent], "primary first, then insertion order, no child");

    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);
    assert_eq!(position(&world, parent), Vec2::new(1.0, 0.0));
    assert_eq!(position(&world, lone), Vec2::new(51.0, 0.0));
    assert_eq!(
        position(&world, child),
        Vec2::new(5.0, 0.0),
        "the child's LOCAL transform must not double-move"
    );
}

#[test]
fn test_nudge_is_suppressed_during_a_gizmo_drag() {
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);
    game.gizmo_drag = Some(drag_state_for(&world, &[a]));

    game.nudge_selection(&mut world, Vec2::new(1.0, 0.0), false);

    assert_eq!(
        position(&world, a),
        Vec2::ZERO,
        "a mid-drag nudge would be swallowed by the drag's release commit"
    );
    assert!(!game.command_history.can_undo());
}

#[test]
fn test_escape_cancels_one_live_thing_per_press() {
    // Most specific first: a gizmo drag, else a pending marquee, else the
    // selection — never two of them on one press.
    let mut game = editor_game();
    let mut world = World::new();
    let a = spawn_at(&mut world, Vec2::ZERO);
    game.editor.selection.select(a);
    game.editor
        .viewport
        .set_viewport_bounds(common::Rect::new(0.0, 0.0, 800.0, 600.0));

    // A live gizmo drag: Escape cancels THAT and leaves the selection alone.
    game.gizmo_drag = Some(drag_state_for(&world, &[a]));
    game.cancel_cascade(&mut world);
    assert!(game.gizmo_drag.is_none(), "the drag cancels first");
    assert!(!game.editor.selection.is_empty(), "selection survives the first press");

    // A pending press inside the viewport (a sub-threshold marquee): the
    // next Escape cancels it so the release cannot become a click.
    let mut input = input::InputHandler::new();
    input.mouse_mut().update_position(400.0, 300.0);
    input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
    game.editor.viewport_input.handle_input_simple(
        &mut game.editor.viewport,
        &game.editor.input_mapping,
        &input,
    );
    assert!(game.editor.viewport_input.has_pending_marquee());
    game.cancel_cascade(&mut world);
    assert!(!game.editor.viewport_input.has_pending_marquee(), "the marquee cancels second");
    assert!(!game.editor.selection.is_empty(), "selection survives the second press");

    // Nothing live: the third Escape clears the selection.
    game.cancel_cascade(&mut world);
    assert!(game.editor.selection.is_empty());
}

#[test]
fn test_deleting_one_entity_hands_its_children_to_the_grandparent_or_roots_them() {
    // The single-selection branch of the Delete shortcut and menu item:
    // one DeleteEntityCommand, children promoted rather than lost, and
    // undo puts them back under it.
    let mut game = editor_game();
    let mut world = World::new();
    let grandparent = spawn_at(&mut world, Vec2::ZERO);
    let parent = spawn_at(&mut world, Vec2::new(1.0, 0.0));
    let child = spawn_at(&mut world, Vec2::new(2.0, 0.0));
    world.set_parent(parent, grandparent).ok();
    world.set_parent(child, parent).ok();
    game.editor.selection.select(parent);

    game.delete_selected_entities(&mut world);

    assert!(world.get::<common::Transform2D>(parent).is_none(), "the entity is gone");
    assert_eq!(world.get_parent(child), Some(grandparent), "the child moves up one level");
    assert!(game.editor.selection.is_empty(), "nothing selected after a delete");
    assert_eq!(game.command_history.undo_name(), Some("Delete Entity"), "one plain entry, no macro");

    assert!(game.command_history.undo(&mut world));
    assert_eq!(world.get_parent(parent), Some(grandparent), "undo restores the parent link");
    assert_eq!(world.get_parent(child), Some(parent), "undo restores the child link");

    // Deleting a root promotes its children to roots.
    let root = spawn_at(&mut world, Vec2::ZERO);
    let orphan = spawn_at(&mut world, Vec2::new(3.0, 0.0));
    world.set_parent(orphan, root).ok();
    game.editor.selection.select(root);
    game.delete_selected_entities(&mut world);
    assert_eq!(world.get_parent(orphan), None, "a root's children become roots");
    assert_eq!(position(&world, orphan), Vec2::new(3.0, 0.0), "and keep their components");
}

#[test]
fn test_delete_of_a_multi_selection_is_one_undo_entry_that_restores_every_entity() {
    // The production path (Delete key and menu item): one DeleteEntityCommand
    // per selected entity wrapped in one MacroCommand, children promoted
    // rather than lost, the selection cleared — and ONE undo brings every
    // entity and every hierarchy link back.
    let mut game = editor_game();
    let mut world = World::new();
    let grandparent = spawn_at(&mut world, Vec2::ZERO);
    let parent = spawn_at(&mut world, Vec2::new(1.0, 0.0));
    let child = spawn_at(&mut world, Vec2::new(2.0, 0.0));
    let lone = spawn_at(&mut world, Vec2::new(9.0, 0.0));
    world.set_parent(parent, grandparent).ok();
    world.set_parent(child, parent).ok();
    game.editor.selection.select(parent);
    game.editor.selection.add(lone);

    game.delete_selected_entities(&mut world);

    assert!(world.get::<common::Transform2D>(parent).is_none());
    assert!(world.get::<common::Transform2D>(lone).is_none());
    assert_eq!(world.get_parent(child), Some(grandparent), "the child moves up one level");
    assert!(game.editor.selection.is_empty(), "nothing selected after a delete");
    assert_eq!(game.command_history.undo_name(), Some("Delete Entities"), "one macro entry");

    assert!(game.command_history.undo(&mut world));
    assert_eq!(position(&world, parent), Vec2::new(1.0, 0.0));
    assert_eq!(position(&world, lone), Vec2::new(9.0, 0.0));
    assert_eq!(world.get_parent(parent), Some(grandparent), "undo restores the parent link");
    assert_eq!(world.get_parent(child), Some(parent), "undo restores the child link");
    assert!(!game.command_history.can_undo(), "the multi-delete was exactly one entry");

    // An empty selection deletes nothing and records nothing.
    game.delete_selected_entities(&mut world);
    assert_eq!(world.entity_count(), 4);
    assert!(!game.command_history.can_undo());
}

#[test]
fn test_duplicate_copies_the_subtree_at_an_offset_selects_the_copy_and_undoes_whole() {
    use crate::constants::DUPLICATE_OFFSET;
    let mut game = editor_game();
    let mut world = World::new();
    let original = spawn_at(&mut world, Vec2::new(100.0, 200.0));
    world.add_component(&original, ecs::Name::new("Hero")).ok();
    world.add_component(&original, ecs::sprite_components::Sprite::new(3)).ok();
    let child = spawn_at(&mut world, Vec2::new(10.0, 10.0));
    world.set_parent(child, original).ok();
    game.editor.selection.select(original);

    game.duplicate_selected_entities(&mut world);

    let copy = game.editor.selection.primary().expect("the selection follows the copy");
    assert_ne!(copy, original);
    assert_eq!(game.editor.selection.len(), 1);
    assert_eq!(position(&world, copy), Vec2::new(100.0, 200.0) + DUPLICATE_OFFSET);
    assert_eq!(world.get::<ecs::sprite_components::Sprite>(copy).map(|s| s.texture_handle), Some(3));
    assert!(
        world.get::<ecs::Name>(copy).is_some_and(|n| n.as_str().ends_with("(Copy)")),
        "the copy is named after the original"
    );
    let copied_children = world.get_children(copy).map(|c| c.to_vec()).unwrap_or_default();
    assert_eq!(copied_children.len(), 1, "the whole subtree is copied");
    assert_eq!(position(&world, copied_children[0]), Vec2::new(10.0, 10.0), "children keep local offsets");
    assert_eq!(position(&world, original), Vec2::new(100.0, 200.0), "the original is untouched");
    assert_eq!(world.entity_count(), 4);

    // One undo removes the WHOLE duplicated subtree (the old per-root
    // CreateEntityCommand orphaned the copied children).
    assert!(game.command_history.undo(&mut world));
    assert_eq!(world.entity_count(), 2, "copy and its child are both gone");
    assert!(!game.command_history.can_undo(), "a duplicate is exactly one entry");

    // Nothing selected: nothing duplicated, nothing recorded.
    game.editor.selection.clear();
    game.duplicate_selected_entities(&mut world);
    assert_eq!(world.entity_count(), 2);
    assert!(!game.command_history.can_undo());
}

#[test]
fn test_copy_paste_cut_cycle_offsets_selects_removes_subtrees_and_undoes() {
    use crate::constants::DUPLICATE_OFFSET;
    let mut game = editor_game();
    let mut world = World::new();
    let root = spawn_at(&mut world, Vec2::new(100.0, 0.0));
    let child = spawn_at(&mut world, Vec2::new(10.0, 10.0));
    world.set_parent(child, root).ok();
    game.editor.selection.select(root);

    // Copy changes nothing in the world.
    game.copy_selection(&mut world);
    assert_eq!(world.entity_count(), 2);
    assert!(!game.command_history.can_undo(), "copy is not an edit");

    // Paste spawns the subtree at the duplicate offset and selects the copy.
    game.paste_clipboard(&mut world);
    assert_eq!(world.entity_count(), 4, "root and child pasted");
    let pasted = game.editor.selection.primary().expect("paste selects the new root");
    assert_ne!(pasted, root);
    assert_eq!(position(&world, pasted), Vec2::new(100.0, 0.0) + DUPLICATE_OFFSET);
    assert_eq!(world.get_children(pasted).map(|c| c.len()), Some(1), "the child came along");

    // Cut removes the WHOLE selected subtree (not Delete's child promotion)
    // and leaves it on the clipboard.
    game.cut_selection(&mut world);
    assert_eq!(world.entity_count(), 2, "cut removes the pasted root and its child");
    assert!(game.editor.selection.is_empty());
    assert_eq!(game.clipboard.len(), 1, "the cut subtree is on the clipboard");

    // Undo: cut first, then paste — each was exactly one entry.
    assert!(game.command_history.undo(&mut world));
    assert_eq!(world.entity_count(), 4, "undo cut restores the subtree");
    assert_eq!(position(&world, pasted), Vec2::new(100.0, 0.0) + DUPLICATE_OFFSET);
    assert!(game.command_history.undo(&mut world));
    assert_eq!(world.entity_count(), 2, "undo paste removes the whole pasted subtree");
    assert!(!game.command_history.can_undo(), "copy → paste → cut was two entries");
}
