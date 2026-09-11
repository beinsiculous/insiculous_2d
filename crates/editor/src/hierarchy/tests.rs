//! Hierarchy panel contracts: names round-trip to entities, F2 rename
//! commits and releases the keyboard, rows follow draw order, Shift-click
//! ranges run anchor-first, and the primary row wears its affordances.

use super::*;
use crate::test_support::{entity, frame, type_key};
use ecs::sprite_components::Sprite;
use input::prelude::{KeyCode, MouseButton};
use physics::components::RigidBody;

const BOUNDS: common::Rect = common::Rect::new(0.0, 0.0, 220.0, 120.0);

/// One panel frame at [`BOUNDS`] with the default theme.
fn render_frame(
    panel: &mut HierarchyPanel,
    ui: &mut ui::UIContext,
    input: &input::InputHandler,
    world: &World,
    selection: &mut Selection,
    drag_drop: &mut DragDropState,
) -> HierarchyResponse {
    let theme = crate::theme::EditorTheme::default();
    frame(ui, input, |ui| panel.render(ui, world, selection, BOUNDS, &theme, drag_drop))
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
        let display = crate::entity_names::entity_display_name(&world, entity);
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
    let mut drag_drop = DragDropState::new();

    // Events are processed at frame START and the per-frame wheel delta is
    // cleared by `end_frame` AFTER the UI consumed it.
    input.queue_event(InputEvent::MouseMoved(50.0, 50.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), 0.0, "the measuring frame does not scroll");

    input.queue_event(InputEvent::MouseWheelScrolled(-2.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    input.end_frame();
    let scrolled = panel.scroll.offset();
    assert!(scrolled > 0.0, "wheel in bounds must scroll the panel");

    input.queue_event(InputEvent::MouseMoved(500.0, 500.0));
    input.queue_event(InputEvent::MouseWheelScrolled(-2.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), scrolled, "wheel outside the panel is not ours");

    // Scrolling far past the end clamps at content_height - bounds.height,
    // and far back up clamps at 0.0 — the rows never leave the panel.
    let content_height = 40.0 * crate::layout::LINE_HEIGHT + crate::layout::PADDING;
    let max_scroll = content_height - BOUNDS.height;
    input.queue_event(InputEvent::MouseMoved(50.0, 50.0));
    input.queue_event(InputEvent::MouseWheelScrolled(-1000.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    input.end_frame();
    assert_eq!(panel.scroll.offset(), max_scroll, "the last row stops at the panel bottom");

    input.queue_event(InputEvent::MouseWheelScrolled(1000.0));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
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
    let mut drag_drop = DragDropState::new();

    // The host arms rename mode and pre-focuses the field.
    panel.begin_rename(named);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(named).as_str(), "Old");
    let response = render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(response.rename_committed, None);
    assert_eq!(panel.renaming(), Some(named));
    assert!(ui.wants_keyboard(), "the rename field owns the keyboard");

    let theme = crate::theme::EditorTheme::default();
    let response = type_key(&mut ui, &mut input, KeyCode::KeyZ, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme, &mut drag_drop)
    });
    assert_eq!(response.rename_committed, None, "typing does not commit");
    let response = type_key(&mut ui, &mut input, KeyCode::Enter, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme, &mut drag_drop)
    });
    assert_eq!(response.rename_committed, Some((named, "z".to_string())));
    assert_eq!(panel.renaming(), None, "commit exits rename mode");
    assert!(!ui.wants_keyboard(), "commit releases the keyboard");

    // Escape on an unnamed entity: no commit, no Name materialized.
    panel.begin_rename(unnamed);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(unnamed).as_str(), "");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    let response = type_key(&mut ui, &mut input, KeyCode::Escape, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme, &mut drag_drop)
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
    let mut drag_drop = DragDropState::new();

    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(panel.visible_order(), &[a, a_child, b, b_child]);

    panel.toggle_expanded(a);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(panel.visible_order(), &[a, b, b_child], "a collapsed subtree has no rows");
    assert!(panel.is_expanded(b), "collapsing one subtree leaves the others alone");

    panel.toggle_expanded(a);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
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
    let mut drag_drop = DragDropState::new();
    selection.select(a_child);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);

    assert_eq!(panel.shift_click_range(&selection, b_child), Some(vec![a_child, b, b_child]), "downwards");
    assert_eq!(panel.shift_click_range(&selection, a), Some(vec![a_child, a]), "upwards, anchor still first");

    selection.add(b);
    panel.toggle_expanded(a); // hides the primary
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(
        panel.shift_click_range(&selection, b_child),
        Some(vec![b, b_child]),
        "the range anchors on the last visible selected row"
    );

    selection.select(a_child);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(panel.shift_click_range(&selection, b_child), None, "no visible anchor: the host adds instead");
    Ok(())
}

/// Exactly the primary row gets the primary fill and the accent bar;
/// the other selected rows get the secondary fill.
#[test]
fn test_primary_row_fill_differs_from_secondary_rows_and_carries_an_accent() -> Result<(), ecs::EcsError> {
    let (world, [a, a_child, b, _b_child]) = four_row_world()?;
    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut selection = Selection::new();
    let mut drag_drop = DragDropState::new();
    selection.select(a);
    selection.add(a_child);
    selection.add(b);
    let theme = crate::theme::EditorTheme::default();
    let fills = theme.selection_row_fills();

    let rects: Vec<(common::Rect, ui::Color)> = frame(&mut ui, &input, |ui| {
        panel.render(ui, &world, &mut selection, BOUNDS, &theme, &mut drag_drop);
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

#[test]
fn test_entity_with_scripts_renders_pseudo_rows_and_click_reports_script() {
    let mut world = World::new();
    let entity = world.create_entity();
    world
        .add_component(
            &entity,
            ecs::Scripts(vec![
                ecs::ScriptRef::new("paddle"),
                ecs::ScriptRef::new("scoring"),
            ]),
        )
        .ok();

    let mut panel = HierarchyPanel::new();
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let mut selection = Selection::new();
    let mut drag_drop = DragDropState::new();

    let response = render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(
        panel.visible_order(),
        &[entity],
        "visible_order holds the entity once (pseudo-rows are not entities)"
    );
    assert!(response.clicked.is_empty());

    // Entity row: y = 0.0 + PADDING(8.0) = 8.0 .. 28.0
    // Script 0 (paddle): y = 28.0 .. 48.0
    // Script 1 (scoring): y = 48.0 .. 68.0
    // Click inside the second script pseudo-row at (50.0, 50.0)
    let click_pos = glam::Vec2::new(50.0, 50.0);
    input.queue_event(input::InputEvent::MouseMoved(click_pos.x, click_pos.y));
    input.queue_event(input::InputEvent::MouseButtonPressed(MouseButton::Left));
    input.process_queued_events();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    input.end_frame();

    input.queue_event(input::InputEvent::MouseButtonReleased(MouseButton::Left));
    input.process_queued_events();
    let response = render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    input.end_frame();

    assert_eq!(
        response.clicked,
        vec![HierarchyClick::Script { entity, index: 1 }],
        "clicking second script reports Script {{ entity, index: 1 }}"
    );
    assert_eq!(
        panel.visible_order(),
        &[entity],
        "visible_order still holds the entity once"
    );
}

/// A rename cancelled from outside the panel — entering Playing does it —
/// leaves a field that is no longer drawn holding the keyboard. Every key
/// then stops at the editor's text-focus guard, and the game never sees the
/// input the user is pressing.
#[test]
fn test_a_rename_cancelled_from_outside_releases_the_keyboard_on_the_next_frame() {
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("Old")).ok();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();

    panel.begin_rename(named);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(named).as_str(), "Old");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert!(ui.wants_keyboard(), "the rename field owns the keyboard");

    panel.cancel_rename();
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);

    assert_eq!(panel.renaming(), None);
    assert!(!ui.wants_keyboard(), "the cancelled field released the keyboard");
}

#[test]
fn test_a_rename_scrolled_off_the_panel_ends_and_releases_the_keyboard() {
    // Only a drawn field can take the Escape that would release it, so a
    // rename whose row has scrolled out of the panel must not keep the
    // keyboard: the game would never see a key again.
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("First")).ok();
    for _ in 0..12 {
        world.create_entity();
    }
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();

    panel.begin_rename(named);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(named).as_str(), "First");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert!(ui.wants_keyboard(), "the drawn rename field owns the keyboard");

    panel.scroll.scroll_to(f32::MAX);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert!(panel.scroll.offset() > 0.0, "the fixture scrolls the first row out of the panel");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);

    assert_eq!(panel.renaming(), None, "the undrawn rename ended");
    assert!(!ui.wants_keyboard(), "and released the keyboard");
}

#[test]
fn test_a_rename_hidden_behind_its_own_script_rows_ends_and_releases_the_keyboard() {
    // Script rows are rows: a panel showing only them has drawn content
    // without the rename field, so the field is gone, not merely unseen.
    let mut world = World::new();
    let scripted = world.create_entity();
    world.add_component(&scripted, Name::new("Scripted")).ok();
    let scripts = (0..10).map(|index| ecs::ScriptRef::new(format!("script_{index}"))).collect();
    world.add_component(&scripted, ecs::Scripts(scripts)).ok();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();

    panel.begin_rename(scripted);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(scripted).as_str(), "Scripted");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert!(ui.wants_keyboard(), "the drawn rename field owns the keyboard");

    panel.scroll.scroll_to(ROW_HEIGHT * 2.0);
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert!(panel.scroll.offset() >= ROW_HEIGHT, "the entity row is above the panel, its script rows fill it");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);

    assert_eq!(panel.renaming(), None, "the undrawn rename ended");
    assert!(!ui.wants_keyboard(), "and released the keyboard");
}

#[test]
fn test_a_rename_survives_a_splitter_through_zero_but_not_a_panel_left_there() {
    // A pass that drew no rows says nothing about the field, for a few
    // frames; a panel that stays empty would otherwise hold the keyboard
    // with no field left to take the Escape.
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("Old")).ok();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();
    let theme = crate::theme::EditorTheme::default();
    let zero = common::Rect::new(0.0, 0.0, 220.0, 0.0);

    panel.begin_rename(named);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(named).as_str(), "Old");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);

    frame(&mut ui, &input, |ui| panel.render(ui, &world, &mut selection, zero, &theme, &mut drag_drop));
    frame(&mut ui, &input, |ui| panel.render(ui, &world, &mut selection, zero, &theme, &mut drag_drop));
    assert_eq!(panel.renaming(), Some(named), "two empty passes are a splitter passing through");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert_eq!(panel.renaming(), Some(named), "and the rename is still there when the rows return");
    assert!(ui.wants_keyboard());

    for _ in 0..40 {
        frame(&mut ui, &input, |ui| panel.render(ui, &world, &mut selection, zero, &theme, &mut drag_drop));
    }
    assert_eq!(panel.renaming(), None, "a panel left empty ends the rename");
    assert!(!ui.wants_keyboard(), "and releases the keyboard");
}

#[test]
fn test_a_rename_in_a_panel_the_dock_does_not_render_ends_after_the_grace() {
    // A narrow-mode tab or a hidden panel never renders, so the frame
    // settles the rename on the panel's behalf; after the grace the
    // undrawn field ends and the keyboard is free.
    let mut world = World::new();
    let named = world.create_entity();
    world.add_component(&named, Name::new("Old")).ok();
    let mut panel = HierarchyPanel::new();
    let mut selection = Selection::new();
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    let mut drag_drop = DragDropState::new();

    panel.begin_rename(named);
    ui.focus_text_input(HierarchyPanel::rename_widget_id(named).as_str(), "Old");
    render_frame(&mut panel, &mut ui, &input, &world, &mut selection, &mut drag_drop);
    assert!(ui.wants_keyboard());

    for _ in 0..2 {
        panel.settle_rename_focus(&mut ui);
    }
    assert_eq!(panel.renaming(), Some(named), "a frame or two without the panel is the grace");
    for _ in 0..40 {
        panel.settle_rename_focus(&mut ui);
    }
    assert_eq!(panel.renaming(), None, "a panel the dock keeps leaving out ends the rename");
    assert!(!ui.wants_keyboard(), "and releases the keyboard");
}
