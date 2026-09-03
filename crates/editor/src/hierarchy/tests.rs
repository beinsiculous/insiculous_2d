//! Hierarchy panel contracts: names round-trip to entities, F2 rename
//! commits and releases the keyboard, rows follow draw order, Shift-click
//! ranges run anchor-first, and the primary row wears its affordances.

use super::*;
use crate::test_support::{entity, frame, type_key};
use input::prelude::KeyCode;

const BOUNDS: common::Rect = common::Rect::new(0.0, 0.0, 220.0, 120.0);

/// One panel frame at [`BOUNDS`] with the default theme.
fn render_frame(
    panel: &mut HierarchyPanel,
    ui: &mut ui::UIContext,
    input: &input::InputHandler,
    world: &World,
    selection: &mut Selection,
) -> HierarchyResponse {
    let theme = crate::theme::EditorTheme::default();
    frame(ui, input, |ui| panel.render(ui, world, selection, BOUNDS, &theme))
}

/// A tree of four rows in draw order: `a`, `a_child`, `b`, `b_child`.
fn four_row_world() -> Result<(World, [EntityId; 4]), ecs::EcsError> {
    let mut world = World::new();
    let a = world.create_entity();
    let a_child = world.create_entity();
    let b = world.create_entity();
    let b_child = world.create_entity();
    world.set_parent(a_child, a)?;
    world.set_parent(b_child, b)?;
    Ok((world, [a, a_child, b, b_child]))
}

/// A display name is an address only when it came from a `Name`: a unique
/// name round-trips through `resolve_by_name`, a synthesized fallback
/// ("Sprite (Entity N)", "Entity N") resolves to nothing, and a duplicate
/// name is ambiguous instead of first-match.
#[test]
fn test_resolve_by_name_inverse_of_display_name() {
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("Player")).ok();
    world.add_component(&named, Sprite::default()).ok();
    let sprite_only = world.create_entity();
    world.add_component(&sprite_only, Sprite::default()).ok();
    let body_only = world.create_entity();
    world.add_component(&body_only, RigidBody::default()).ok();
    let bare = world.create_entity();

    let fallbacks = [
        (named, "Player", "a Name wins over every fallback"),
        (sprite_only, "Sprite (Entity", "a sprite falls back to its kind"),
        (body_only, "RigidBody (Entity", "a body falls back to its kind"),
        (bare, "Entity", "an empty entity is just its id"),
    ];
    for (entity, prefix, why) in fallbacks {
        let display = HierarchyPanel::entity_display_name(&world, entity);
        assert!(display.starts_with(prefix), "{why}: {display:?}");
        let expected = if entity == named { NameResolution::One(named) } else { NameResolution::None };
        assert_eq!(HierarchyPanel::resolve_by_name(&world, &display), expected, "{display:?}");
    }

    let twin = world.create_entity();
    world.add_component(&twin, Name::new("Player")).ok();
    match HierarchyPanel::resolve_by_name(&world, "Player") {
        NameResolution::Ambiguous(matches) => assert_eq!(matches, vec![named, twin]),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

/// Past ~30 entities the rows were invisible and the panel ignored the
/// wheel. Wheel input inside the panel bounds scrolls the rows; wheel
/// input outside is not the panel's.
#[test]
fn test_hierarchy_scrolls_rows_with_wheel_in_bounds() {
    use input::InputEvent;
    let mut world = World::new();
    for _ in 0..40 {
        world.create_entity();
    }
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();

    // Events are processed at frame START and the per-frame wheel delta is
    // cleared by `end_frame` AFTER the UI consumed it.
    input.queue_event(InputEvent::MouseMoved(50.0, 50.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), 0.0, "the measuring frame does not scroll");

    input.queue_event(InputEvent::MouseWheelScrolled(-2.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    input.end_frame();
    let scrolled = panel.scroll.offset();
    assert!(scrolled > 0.0, "wheel in bounds must scroll the panel");

    input.queue_event(InputEvent::MouseMoved(500.0, 500.0));
    input.queue_event(InputEvent::MouseWheelScrolled(-2.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), scrolled, "wheel outside the panel is not ours");

    // Scrolling far past the end clamps at content_height - bounds.height,
    // and far back up clamps at 0.0 — the rows never leave the panel.
    let content_height = 40.0 * crate::layout::LINE_HEIGHT + crate::layout::PADDING;
    let max_scroll = content_height - BOUNDS.height;
    input.queue_event(InputEvent::MouseMoved(50.0, 50.0));
    input.queue_event(InputEvent::MouseWheelScrolled(-1000.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), max_scroll, "the last row stops at the panel bottom");

    input.queue_event(InputEvent::MouseWheelScrolled(1000.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), 0.0, "scrolling back up stops at the first row");
}

/// F2 rename: the field owns the keyboard, typing replaces the selected
/// seed text, Enter reports the raw new text and exits rename mode
/// releasing the keyboard; Escape exits without a commit and an unnamed
/// entity gains no `Name`. The host then normalizes: trimmed, non-empty,
/// changed — or nothing is recorded.
#[test]
fn test_rename_commit_reports_new_name_and_exits_mode() {
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("Old")).ok();
    let unnamed = world.create_entity();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();

    // The host arms rename mode and pre-focuses the field.
    panel.begin_rename(named);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(named).as_str(), "Old");
    let response = render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(response.rename_committed, None);
    assert_eq!(panel.renaming(), Some(named));
    assert!(ui.wants_keyboard(), "the rename field owns the keyboard");

    let theme = crate::theme::EditorTheme::default();
    let response = type_key(&mut ui, &mut input, KeyCode::KeyZ, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme)
    });
    assert_eq!(response.rename_committed, None, "typing does not commit");
    let response = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme)
    });
    assert_eq!(response.rename_committed, Some((named, "z".to_string())));
    assert_eq!(panel.renaming(), None, "commit exits rename mode");
    assert!(!ui.wants_keyboard(), "commit releases the keyboard");

    // Escape on an unnamed entity: no commit, no Name materialized.
    panel.begin_rename(unnamed);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(unnamed).as_str(), "");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    let response = type_key(&mut ui, &mut input, KeyCode::Escape, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme)
    });
    assert_eq!(response.rename_committed, None, "escape must never commit");
    assert_eq!(panel.renaming(), None, "escape exits rename mode");
    assert!(world.get::<Name>(unnamed).is_none());

    let normalized = [
        (Some("Old"), "  New  ", Some("New".to_string())),
        (None, "Fresh", Some("Fresh".to_string())),
        (Some("Old"), "   ", None),
        (None, "", None),
        (Some("Old"), " Old ", None),
    ];
    for (current, raw, expected) in normalized {
        assert_eq!(normalized_rename(current, raw), expected, "current={current:?} raw={raw:?}");
    }
}

/// Rows are laid out in draw order (roots by id, children under their
/// parent); collapsing a subtree removes its rows and toggling restores them.
#[test]
fn test_visible_order_follows_draw_order_and_skips_collapsed_subtrees() -> Result<(), ecs::EcsError> {
    let (world, [a, a_child, b, b_child]) = four_row_world()?;
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();

    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(panel.visible_order(), &[a, a_child, b, b_child]);

    panel.toggle_expanded(a);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(panel.visible_order(), &[a, b, b_child], "a collapsed subtree has no rows");
    assert!(panel.is_expanded(b), "collapsing one subtree leaves the others alone");

    panel.toggle_expanded(a);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(panel.visible_order(), &[a, a_child, b, b_child], "toggling re-expands");
    assert!(panel.is_expanded(entity(99)), "an entity never touched starts expanded");
    Ok(())
}

/// Shift-click selects the visible rows between the anchor and the target,
/// anchor first in either direction so `select_multiple` keeps it primary.
/// A hidden primary falls back to the last visible selected row; with no
/// selected row visible there is no range and the host adds instead.
#[test]
fn test_shift_click_range_runs_anchor_first_in_either_direction() -> Result<(), ecs::EcsError> {
    let (world, [a, a_child, b, b_child]) = four_row_world()?;
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    selection.select(a_child);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);

    assert_eq!(panel.shift_click_range(&selection, b_child), Some(vec![a_child, b, b_child]), "downwards");
    assert_eq!(panel.shift_click_range(&selection, a), Some(vec![a_child, a]), "upwards, anchor still first");

    selection.add(b);
    panel.toggle_expanded(a); // hides the primary
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(
        panel.shift_click_range(&selection, b_child),
        Some(vec![b, b_child]),
        "the range anchors on the last visible selected row"
    );

    selection.select(a_child);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection);
    assert_eq!(panel.shift_click_range(&selection, b_child), None, "no visible anchor: the host adds instead");
    Ok(())
}

/// #51: exactly the primary row gets the primary fill and the accent bar;
/// the other selected rows get the secondary fill.
#[test]
fn test_primary_row_fill_differs_from_secondary_rows_and_carries_an_accent() -> Result<(), ecs::EcsError> {
    let (world, [a, a_child, b, _b_child]) = four_row_world()?;
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    selection.select(a);
    selection.add(a_child);
    selection.add(b);
    let theme = crate::theme::EditorTheme::default();
    let fills = theme.selection_row_fills();

    let rects: Vec<(common::Rect, ui::Color)> = frame(&mut ui, &input, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme);
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::Rect { bounds, color, .. } => Some((*bounds, *color)),
                _ => None,
            })
            .collect()
    });

    let count = |width: f32, fill: ui::Color| rects.iter().filter(|(r, c)| r.width == width && *c == fill).count();
    assert_eq!(count(BOUNDS.width, fills.primary), 1, "exactly the primary row gets the primary fill");
    assert_eq!(count(BOUNDS.width, fills.secondary), 2, "the other selected rows get the secondary fill");
    assert_eq!(count(PRIMARY_ACCENT_WIDTH, fills.accent), 1, "one accent bar, on the primary row");
    Ok(())
}
