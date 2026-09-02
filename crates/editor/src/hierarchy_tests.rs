//! Tests for the hierarchy panel (moved from `hierarchy.rs` for file size).

use crate::hierarchy::*;
use crate::Selection;
use ecs::sprite_components::{Name, Sprite};
use ecs::{EntityId, World, WorldHierarchyExt};
use physics::components::RigidBody;

fn entity(id: u64) -> EntityId {
    EntityId::with_generation(id, 1)
}

// ==================== Name resolution ====================

#[test]
fn test_resolve_by_name_inverse_of_display_name() {
    use ecs::sprite_components::Name;
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("Player")).ok();
    let other = world.create_entity();
    world.add_component(&other, Name::new("Player")).ok();
    let unnamed = world.create_entity();

    // A unique name round-trips: display -> resolve.
    world.remove_component::<Name>(&other).ok();
    let display = HierarchyPanel::entity_display_name(&world, named);
    assert_eq!(HierarchyPanel::resolve_by_name(&world, &display), NameResolution::One(named));

    // Synthesized display names are NOT addresses.
    let synthesized = HierarchyPanel::entity_display_name(&world, unnamed);
    assert_eq!(HierarchyPanel::resolve_by_name(&world, &synthesized), NameResolution::None);

    // Duplicates report ambiguity instead of first-match.
    world.add_component(&other, Name::new("Player")).ok();
    match HierarchyPanel::resolve_by_name(&world, "Player") {
        NameResolution::Ambiguous(matches) => assert_eq!(matches.len(), 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

// ==================== Scrolling ====================

#[test]
fn test_hierarchy_scrolls_rows_with_wheel_in_bounds() {
    // Audit §3.3: past ~30 entities rows were invisible AND the panel
    // ignored the wheel. Wheel input inside the panel bounds must move
    // the scroll offset so later rows lay out inside the panel.
    use input::InputEvent;

    let mut world = World::new();
    for _ in 0..40 {
        world.create_entity();
    }
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let bounds = common::Rect::new(0.0, 0.0, 200.0, 100.0);
    let theme = crate::theme::EditorTheme::default();

    // Frame 1 measures content height (no wheel yet). NOTE the input
    // lifecycle: process events at frame START, end_frame (which clears
    // the per-frame wheel delta) after the UI consumed it.
    let mut input = input::InputHandler::new();
    input.queue_event(InputEvent::MouseMoved(50.0, 50.0));
    input.process_queued_events();
    let mut ui = ui::UIContext::new();
    ui.begin_frame(&input, glam::Vec2::new(800.0, 600.0));
    panel.render(&mut ui, &world, &mut selection, bounds, &theme);
    ui.end_frame();
    input.end_frame();
    assert_eq!(panel.scroll.offset(), 0.0);

    // Frame 2: wheel down inside the bounds scrolls.
    input.queue_event(InputEvent::MouseWheelScrolled(-2.0));
    input.process_queued_events();
    ui.begin_frame(&input, glam::Vec2::new(800.0, 600.0));
    panel.render(&mut ui, &world, &mut selection, bounds, &theme);
    ui.end_frame();
    input.end_frame();
    assert!(panel.scroll.offset() > 0.0, "wheel in bounds must scroll the panel");

    // Frame 3: wheel outside the bounds is ignored.
    let before = panel.scroll.offset();
    input.queue_event(InputEvent::MouseMoved(500.0, 500.0));
    input.queue_event(InputEvent::MouseWheelScrolled(-2.0));
    input.process_queued_events();
    ui.begin_frame(&input, glam::Vec2::new(800.0, 600.0));
    panel.render(&mut ui, &world, &mut selection, bounds, &theme);
    ui.end_frame();
    input.end_frame();
    assert_eq!(panel.scroll.offset(), before, "wheel outside the panel is not ours");
}

// ==================== Expand/Collapse State Tests ====================

#[test]
fn test_default_expanded() {
    let panel = HierarchyPanel::new();
    let e1 = entity(1);

    // Entities are expanded by default
    assert!(panel.is_expanded(e1));
}

#[test]
fn test_toggle_collapse() {
    let mut panel = HierarchyPanel::new();
    let e1 = entity(1);

    // Initially expanded
    assert!(panel.is_expanded(e1));

    // Toggle to collapse
    panel.toggle_expanded(e1);
    assert!(!panel.is_expanded(e1));

    // Toggle to expand again
    panel.toggle_expanded(e1);
    assert!(panel.is_expanded(e1));
}

#[test]
fn test_collapse_persists() {
    let mut panel = HierarchyPanel::new();
    let e1 = entity(1);
    let e2 = entity(2);

    // Collapse e1
    panel.collapse(e1);
    assert!(!panel.is_expanded(e1));
    assert!(panel.is_expanded(e2)); // e2 still expanded

    // Expand e1
    panel.expand(e1);
    assert!(panel.is_expanded(e1));
}

#[test]
fn test_multiple_entities_independent_state() {
    let mut panel = HierarchyPanel::new();
    let e1 = entity(1);
    let e2 = entity(2);
    let e3 = entity(3);

    panel.collapse(e1);
    panel.collapse(e3);

    assert!(!panel.is_expanded(e1));
    assert!(panel.is_expanded(e2));
    assert!(!panel.is_expanded(e3));
}

// ==================== Name Resolution Tests ====================

#[test]
fn test_name_from_name_component() {
    let mut world = World::new();
    let e = world.create_entity();
    world.add_component(&e, Name::new("Player")).ok();
    world.add_component(&e, Sprite::default()).ok(); // Also has sprite

    // Name component takes priority
    let name = HierarchyPanel::entity_display_name(&world, e);
    assert_eq!(name, "Player");
}

#[test]
fn test_name_fallback_sprite() {
    let mut world = World::new();
    let e = world.create_entity();
    world.add_component(&e, Sprite::default()).ok();

    let name = HierarchyPanel::entity_display_name(&world, e);
    assert!(name.starts_with("Sprite (Entity"));
}

#[test]
fn test_name_fallback_rigidbody() {
    let mut world = World::new();
    let e = world.create_entity();
    world.add_component(&e, RigidBody::default()).ok();

    let name = HierarchyPanel::entity_display_name(&world, e);
    assert!(name.starts_with("RigidBody (Entity"));
}

#[test]
fn test_name_fallback_entity_id() {
    let mut world = World::new();
    let e = world.create_entity();

    let name = HierarchyPanel::entity_display_name(&world, e);
    assert!(name.starts_with("Entity"));
}

// ==================== Tree Structure Tests ====================

#[test]
fn test_hierarchy_panel_new() {
    let panel = HierarchyPanel::new();
    assert!(panel.is_expanded(entity(1)), "nothing starts collapsed");
}

#[test]
fn test_root_entities_rendering_order() {
    // This test verifies the logic without actual UI rendering
    let mut world = World::new();
    let root1 = world.create_entity();
    let root2 = world.create_entity();
    let child = world.create_entity();

    world.set_parent(child, root1).unwrap();

    let roots = world.get_root_entities();

    // Should have 2 root entities
    assert_eq!(roots.len(), 2);
    assert!(roots.contains(&root1));
    assert!(roots.contains(&root2));
    assert!(!roots.contains(&child));
}

#[test]
fn test_collapsed_hides_children() {
    let mut panel = HierarchyPanel::new();
    let mut world = World::new();

    let parent = world.create_entity();
    let child = world.create_entity();
    world.set_parent(child, parent).unwrap();

    // When parent is expanded, children are visible (is_expanded returns true)
    assert!(panel.is_expanded(parent));

    // When parent is collapsed, children are hidden
    panel.collapse(parent);
    assert!(!panel.is_expanded(parent));
}

#[test]
fn test_deep_hierarchy_structure() {
    let mut world = World::new();

    let grandparent = world.create_entity();
    let parent = world.create_entity();
    let child = world.create_entity();

    world.set_parent(parent, grandparent).unwrap();
    world.set_parent(child, parent).unwrap();

    // Verify hierarchy structure
    let roots = world.get_root_entities();
    assert_eq!(roots.len(), 1);
    assert!(roots.contains(&grandparent));

    let descendants = world.get_descendants(grandparent);
    assert_eq!(descendants.len(), 2);
    assert!(descendants.contains(&parent));
    assert!(descendants.contains(&child));
}

// ==================== Inline rename (F2) ====================

#[test]
fn test_normalized_rename_rejects_empty_and_unchanged() {
    assert_eq!(normalized_rename(Some("Old"), "  New  "), Some("New".to_string()));
    assert_eq!(normalized_rename(None, "Fresh"), Some("Fresh".to_string()));
    assert_eq!(normalized_rename(Some("Old"), "   "), None, "blank commit is a no-op");
    assert_eq!(normalized_rename(None, ""), None);
    assert_eq!(normalized_rename(Some("Old"), " Old "), None, "unchanged commit records nothing");
}

fn rename_frame(
    panel: &mut HierarchyPanel,
    ui: &mut ui::UIContext,
    input: &input::InputHandler,
    world: &World,
    selection: &mut Selection,
) -> HierarchyResponse {
    let bounds = common::Rect::new(0.0, 0.0, 220.0, 120.0);
    let theme = crate::theme::EditorTheme::default();
    ui.begin_frame(input, glam::Vec2::new(800.0, 600.0));
    let response = panel.render(ui, world, selection, bounds, &theme);
    ui.end_frame();
    response
}

#[test]
fn test_rename_commit_reports_new_name_and_exits_mode() {
    let mut world = World::new();
    let e = world.create_entity();
    world.add_component(&e, Name::new("Old")).ok();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();

    // F2: the host arms rename mode and pre-focuses the field.
    panel.begin_rename(e);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(e).as_str(), "Old");

    // Frame 1: field renders in edit mode, nothing committed yet.
    let r = rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert!(r.rename_committed.is_none());
    assert_eq!(panel.renaming(), Some(e));
    assert!(ui.wants_keyboard(), "rename field owns the keyboard");

    // Frame 2: typing replaces the fully-selected seed text.
    input.update();
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::KeyZ);
    let r = rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert!(r.rename_committed.is_none());
    input.keyboard_mut().handle_key_release(input::prelude::KeyCode::KeyZ);

    // Frame 3: Enter commits the new text and exits rename mode.
    input.update();
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::Enter);
    let r = rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(r.rename_committed, Some((e, "z".to_string())));
    assert_eq!(panel.renaming(), None);
    assert!(!ui.wants_keyboard(), "commit releases the keyboard");
}

#[test]
fn test_rename_escape_cancels_without_commit() {
    let mut world = World::new();
    let e = world.create_entity();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();

    // F2 on an UNNAMED entity opens an empty field (kimi F6: no Name is
    // materialized unless a non-empty commit lands).
    panel.begin_rename(e);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(e).as_str(), "");
    let r = rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert!(r.rename_committed.is_none());
    assert_eq!(panel.renaming(), Some(e));

    // Escape: no commit, mode exits, the entity still has no Name.
    input.update();
    input.keyboard_mut().handle_key_press(input::prelude::KeyCode::Escape);
    let r = rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert!(r.rename_committed.is_none(), "escape must never commit");
    assert_eq!(panel.renaming(), None, "escape exits rename mode");
    assert!(world.get::<Name>(e).is_none());
}

// ==================== Primary affordances + Shift range (#51) ====================

/// A tree of four rows in draw order: `a`, `a_child`, `b`, `b_child`.
fn four_row_world() -> (World, [EntityId; 4]) {
    let mut world = World::new();
    let a = world.create_entity();
    let a_child = world.create_entity();
    let b = world.create_entity();
    let b_child = world.create_entity();
    world.set_parent(a_child, a).unwrap();
    world.set_parent(b_child, b).unwrap();
    (world, [a, a_child, b, b_child])
}

#[test]
fn test_visible_order_follows_draw_order_and_skips_collapsed_subtrees() {
    let (world, [a, a_child, b, b_child]) = four_row_world();
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();

    rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(panel.visible_order(), &[a, a_child, b, b_child]);

    panel.collapse(a);
    rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(panel.visible_order(), &[a, b, b_child], "a collapsed subtree has no rows");
}

#[test]
fn test_shift_click_range_runs_anchor_first_in_either_direction() {
    let (world, [a, a_child, b, b_child]) = four_row_world();
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    selection.select(a_child);
    rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);

    // Downwards: anchor, then the rows below it.
    assert_eq!(panel.shift_click_range(&selection, b_child), Some(vec![a_child, b, b_child]));
    // Upwards: the same rows, anchor still first — select_multiple keeps it primary.
    assert_eq!(panel.shift_click_range(&selection, a), Some(vec![a_child, a]));
}

#[test]
fn test_shift_click_range_anchors_on_last_visible_selected_row_when_primary_hidden() {
    let (world, [a, a_child, b, b_child]) = four_row_world();
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    selection.select(a_child); // primary
    selection.add(b);
    panel.collapse(a); // hides the primary
    rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);

    assert_eq!(
        panel.shift_click_range(&selection, b_child),
        Some(vec![b, b_child]),
        "the range anchors on the last visible selected row, not on nothing"
    );
}

#[test]
fn test_shift_click_range_is_none_when_no_selected_row_is_visible() {
    let (world, [a, a_child, _b, b_child]) = four_row_world();
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    selection.select(a_child);
    panel.collapse(a);
    rename_frame(&mut panel, &mut ui, &input, &world, &mut selection);

    assert_eq!(panel.shift_click_range(&selection, b_child), None, "the host adds instead");
}

#[test]
fn test_primary_row_fill_differs_from_secondary_rows_and_carries_an_accent() {
    let (world, [a, a_child, b, _b_child]) = four_row_world();
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    selection.select(a); // primary
    selection.add(a_child);
    selection.add(b);

    let bounds = common::Rect::new(0.0, 0.0, 220.0, 120.0);
    let theme = crate::theme::EditorTheme::default();
    let fills = theme.selection_row_fills();
    ui.begin_frame(&input, glam::Vec2::new(800.0, 600.0));
    panel.render(&mut ui, &world, &mut selection, bounds, &theme);
    let rects: Vec<(common::Rect, ui::Color)> = ui
        .draw_list()
        .commands()
        .iter()
        .filter_map(|c| match c {
            ui::DrawCommand::Rect { bounds, color, .. } => Some((*bounds, *color)),
            _ => None,
        })
        .collect();
    ui.end_frame();

    let primary_fills = rects.iter().filter(|(r, c)| r.width == bounds.width && *c == fills.primary).count();
    let secondary_fills = rects.iter().filter(|(r, c)| r.width == bounds.width && *c == fills.secondary).count();
    let accents = rects.iter().filter(|(r, c)| r.width == PRIMARY_ACCENT_WIDTH && *c == fills.accent).count();
    assert_eq!(primary_fills, 1, "exactly the primary row gets the primary fill");
    assert_eq!(secondary_fills, 2, "the other selected rows get the secondary fill");
    assert_eq!(accents, 1, "one accent bar, on the primary row");
}
