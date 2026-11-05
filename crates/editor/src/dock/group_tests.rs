//! Group movement, effective budgets and the reachable overflow tabs.

use super::*;
use crate::{EditorContext, EditorPreferences, GroupPrefs, EditorTheme};
use crate::test_support::{press_at, release};
use glam::Vec2;

fn dock() -> DockArea {
    let mut dock = EditorContext::new().dock_area;
    dock.set_bounds(Rect::new(0.0, 0.0, 1280.0, 800.0));
    dock.layout();
    dock
}

#[test]
fn shared_edge_panels_use_tabs_and_hiding_the_active_tab_reveals_the_next_visible_one() {
    let mut dock = dock();
    let before = dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds();
    dock.show_panel(PanelId::SETTINGS);
    assert_eq!(dock.get_panel(PanelId::SETTINGS).unwrap().content_bounds(), before);
    assert_eq!(dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds(), Rect::default());
    dock.set_panel_visible(PanelId::SETTINGS, false);
    assert_eq!(dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds(), before);
    assert_eq!(dock.groups().len(), 3);
}

#[test]
fn moving_the_last_panel_off_an_edge_drops_its_group_and_widens_the_scene() {
    let mut dock = dock();
    let before = dock.get_panel(PanelId::SCENE_VIEW).unwrap().content_bounds();
    dock.move_panel_to_edge(PanelId::HIERARCHY, DockPosition::Bottom);
    assert!(dock.groups().iter().all(|group| group.position != DockPosition::Left));
    let after = dock.get_panel(PanelId::SCENE_VIEW).unwrap().content_bounds();
    assert!(after.width > before.width);
    assert_eq!(after.x, 0.0);
}

#[test]
fn grouping_keeps_the_target_allocation_and_activates_the_inserted_tab() {
    let mut dock = dock();
    let target = dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds();
    dock.group_panel_with(PanelId::HIERARCHY, PanelId::INSPECTOR);
    let moved = dock.get_panel(PanelId::HIERARCHY).unwrap().content_bounds();
    assert_eq!(moved.width, target.width);
    assert_eq!(dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds(), Rect::default());
    let group = &dock.groups()[dock.group_of(PanelId::INSPECTOR).unwrap()];
    assert_eq!(group.panels, [PanelId::INSPECTOR, PanelId::HIERARCHY, PanelId::SETTINGS]);
}

#[test]
fn three_bottom_groups_at_double_scale_keep_the_centre_and_remember_the_users_sizes() {
    let mut dock = dock();
    dock.move_panel_to_edge(PanelId::CONDUCTOR, DockPosition::Bottom);
    dock.move_panel_to_edge(PanelId::CONSOLE, DockPosition::Bottom);
    for group in &mut dock.groups {
        if group.position == DockPosition::Bottom { group.size = 200.0; }
    }
    dock.set_bounds(Rect::new(0.0, 0.0, 640.0, 400.0));
    dock.layout();
    assert!(dock.get_panel(PanelId::SCENE_VIEW).unwrap().bounds.height >= MIN_CENTER_HEIGHT);
    assert!(dock.get_panel(PanelId::SCENE_VIEW).unwrap().content_bounds().height >= crate::layout::TOOLBAR_STRIP_HEIGHT + 120.0, "the budget leaves the toolbar and 120 points of viewport under the scene header");
    assert!(dock.groups.iter().filter(|group| group.position == DockPosition::Bottom).all(|group| group.size == 200.0 && !group.collapsed));
    assert_eq!(dock.budget_collapsed.first().copied(), dock.group_of(PanelId::ASSET_BROWSER), "collapse the furthest group first");
    dock.set_bounds(Rect::new(0.0, 0.0, 1280.0, 1000.0));
    dock.layout();
    for id in [PanelId::ASSET_BROWSER, PanelId::CONDUCTOR, PanelId::CONSOLE] {
        assert_eq!(dock.get_panel(id).unwrap().bounds.height, 200.0, "restored on enlargement");
    }
}

#[test]
fn an_oversized_saved_layout_yields_effectively_without_rewriting_its_saved_sizes() {
    let mut dock = dock();
    let mut preferences = EditorPreferences::default();
    preferences.capture_panels(&dock);
    for group in &mut preferences.groups { group.size = 1200.0; }
    preferences.apply_panels(&mut dock);
    dock.set_bounds(Rect::new(0.0, 0.0, 640.0, 400.0));
    dock.layout();
    assert!(dock.get_panel(PanelId::SCENE_VIEW).unwrap().bounds.height >= MIN_CENTER_HEIGHT);
    let mut saved = EditorPreferences::default();
    saved.capture_panels(&dock);
    assert_eq!(saved.groups, preferences.groups);
}

#[test]
fn a_move_that_cannot_fit_collapsed_strips_is_refused_without_losing_the_panel() {
    let mut dock = dock();
    dock.set_bounds(Rect::new(0.0, 0.0, 320.0, 200.0));
    dock.layout();
    let original = dock.group_of(PanelId::INSPECTOR);
    assert!(!dock.can_move_panel_to_edge(PanelId::INSPECTOR, DockPosition::Bottom));
    dock.move_panel_to_edge(PanelId::INSPECTOR, DockPosition::Bottom);
    assert_eq!(dock.group_of(PanelId::INSPECTOR), original);
}

#[test]
fn every_panel_in_a_minimum_width_right_group_is_reachable_at_double_scale() {
    let mut dock = dock();
    for id in [PanelId::HIERARCHY, PanelId::ASSET_BROWSER, PanelId::CONDUCTOR, PanelId::CONSOLE, PanelId::SETTINGS] {
        dock.group_panel_with(id, PanelId::INSPECTOR);
    }
    dock.show_panel(PanelId::INSPECTOR);
    let index = dock.group_of(PanelId::INSPECTOR).unwrap();
    dock.groups[index].size = dock.groups[index].min_size;
    dock.set_bounds(Rect::new(0.0, 0.0, 640.0, 400.0));
    dock.layout();
    let mut ui = ui::UIContext::new();
    ui.set_pixels_per_point(2.0);
    let mut input = input::InputHandler::new();
    let theme = EditorTheme::default();
    ui.begin_frame(&input, Vec2::new(1280.0, 800.0));
    dock.render(&mut ui, &theme);
    ui.end_frame();
    assert!(ui.draw_list().commands().iter().any(|command| matches!(command,
        ui::DrawCommand::TextPlaceholder { text, .. } if text == "»")));
    dock.cycle_tab_focus();
    for _ in 0..6 {
        let id = dock.focused_tab().unwrap();
        let tab = dock.tab_bounds(id).unwrap();
        assert!(tab.width >= 48.0);
        assert!(tab.right() <= dock.groups[index].bounds.right() - HEADER_HEIGHT);
        dock.activate(id);
        assert!(dock.get_panel(id).unwrap().content_bounds().height > 0.0);
        dock.step_tab_focus(true);
    }
    // The pointer menu reaches the tabs omitted by the same layout.
    ui.set_pixels_per_point(1.0);
    let button = dock.groups[index].bounds;
    let point = Vec2::new(button.right() - HEADER_HEIGHT * 1.5, button.y + HEADER_HEIGHT / 2.0);
    press_at(&mut ui, &mut input, point, |ui| dock.render(ui, &theme));
    release(&mut ui, &mut input, |ui| dock.render(ui, &theme));
    assert!(dock.overflow_group.is_some());
    let menu = crate::overflow_menu::overflow_menu_bounds(Rect::new(point.x - 12.0, button.y, 24.0, 24.0),
        3.0 * crate::layout::ROW_HEIGHT + 8.0, Vec2::new(1280.0, 720.0), 24.0);
    let row = Vec2::new(menu.x + 20.0, menu.y + 16.0);
    press_at(&mut ui, &mut input, row, |ui| dock.render(ui, &theme));
    release(&mut ui, &mut input, |ui| dock.render(ui, &theme));
    assert!(dock.overflow_group.is_none(), "a menu row activates its tab and closes");
}

#[test]
fn saved_groups_take_precedence_and_unknown_members_do_not_hide_new_default_tabs() {
    let mut dock = dock();
    let preferences = EditorPreferences { groups: vec![GroupPrefs {
        position: DockPosition::Right, size: 1.0, collapsed: false,
        panels: vec![1, 999], active: 999, hidden: vec![],
    }], ..Default::default() };
    preferences.apply_panels(&mut dock);
    assert!(dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds().width >= 240.0);
    assert!(dock.get_panel(PanelId::HIERARCHY).unwrap().content_bounds().width > 0.0);
    assert!(!dock.get_panel(PanelId::SETTINGS).unwrap().visible);
    dock.show_panel(PanelId::SETTINGS);
    assert!(dock.get_panel(PanelId::SETTINGS).unwrap().content_bounds().width >= 240.0);
}

fn drawn(dock: &mut DockArea) {
    let mut ui = ui::UIContext::new();
    let input = input::InputHandler::new();
    ui.begin_frame(&input, Vec2::new(1280.0, 800.0));
    dock.render(&mut ui, &EditorTheme::default());
    ui.end_frame();
}

#[test]
fn tabs_take_their_titles_width_and_leave_the_rest_of_the_strip_empty() {
    let mut dock = dock();
    dock.show_panel(PanelId::CONSOLE);
    dock.activate(PanelId::ASSET_BROWSER);
    drawn(&mut dock);
    let assets = dock.tab_bounds(PanelId::ASSET_BROWSER).unwrap();
    let console = dock.tab_bounds(PanelId::CONSOLE).unwrap();
    assert!(assets.width >= render::MIN_TAB_WIDTH && assets.width <= render::MAX_TAB_WIDTH, "{assets:?}");
    assert_eq!(console.x, assets.right(), "the second tab follows the first");
    let strip = dock.groups()[dock.group_of(PanelId::ASSET_BROWSER).unwrap()].bounds();
    assert!(console.right() < strip.center().x, "two tabs never split a wide strip between them: {console:?}");
    assert_eq!(dock.tab_at(assets.center()), Some(PanelId::ASSET_BROWSER));
    assert_eq!(dock.tab_at(console.center()), Some(PanelId::CONSOLE));
    assert_eq!(dock.tab_at(Vec2::new(strip.right() - 60.0, assets.center().y)), None, "the empty strip is no tab");
}

#[test]
fn a_tab_click_on_a_budget_collapsed_group_opens_it_as_the_overlay() {
    let mut dock = dock();
    dock.set_bounds(Rect::new(0.0, 0.0, 640.0, 300.0));
    dock.layout();
    assert_eq!(dock.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds(), Rect::default(), "fixture: the budget collapsed the bottom group");
    let mut ui = ui::UIContext::new();
    let mut input = input::InputHandler::new();
    let theme = EditorTheme::default();
    drawn(&mut dock);
    let tab = dock.tab_bounds(PanelId::ASSET_BROWSER).unwrap().center();
    press_at(&mut ui, &mut input, tab, |ui| dock.render(ui, &theme));
    release(&mut ui, &mut input, |ui| dock.render(ui, &theme));
    assert_eq!(dock.narrow_overlay(), Some(PanelId::ASSET_BROWSER));
    assert!(dock.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds().height > 0.0);
    assert!(!dock.groups()[dock.group_of(PanelId::ASSET_BROWSER).unwrap()].collapsed, "the click never writes a collapse");
}

#[test]
fn an_open_overflow_menu_closes_when_its_group_loses_its_strip() {
    let mut dock = dock();
    for id in [PanelId::HIERARCHY, PanelId::ASSET_BROWSER, PanelId::CONDUCTOR, PanelId::CONSOLE, PanelId::SETTINGS] {
        dock.group_panel_with(id, PanelId::INSPECTOR);
    }
    let index = dock.group_of(PanelId::INSPECTOR).unwrap();
    dock.groups[index].size = dock.groups[index].min_size;
    dock.set_bounds(Rect::new(0.0, 0.0, 640.0, 400.0));
    dock.layout();
    dock.overflow_group = Some(index);
    dock.set_bounds(Rect::new(0.0, 0.0, 320.0, 400.0));
    dock.layout();
    assert!(dock.is_narrow(), "fixture: the group is now one vertical tab");
    assert_eq!(dock.overflow_group, None, "no menu hangs beside a tab the group no longer has");
}

#[test]
fn a_right_press_on_a_narrow_tab_or_a_collapsed_strip_asks_for_its_tab_menu() {
    use crate::test_support::{frame, press_button};
    use input::prelude::MouseButton;
    let theme = EditorTheme::default();
    let mut dock = dock();
    dock.set_bounds(Rect::new(0.0, 0.0, 320.0, 400.0));
    dock.layout();
    assert!(dock.is_narrow(), "fixture: side groups are narrow tabs");
    let tab = dock.tab_bounds(PanelId::INSPECTOR).unwrap().center();
    let (mut ui, mut input) = (ui::UIContext::new(), input::InputHandler::new());
    press_button(&mut input, MouseButton::Right, tab);
    frame(&mut ui, &input, |ui| dock.render(ui, &theme));
    assert_eq!(dock.take_tab_context_request().map(|(id, _)| id), Some(PanelId::INSPECTOR));

    let mut dock = self::dock();
    dock.set_panel_collapsed(PanelId::HIERARCHY, true);
    let strip = dock.tab_bounds(PanelId::HIERARCHY).unwrap();
    assert_eq!(strip.width, HEADER_HEIGHT, "fixture: a collapsed side group is a vertical strip");
    let (mut ui, mut input) = (ui::UIContext::new(), input::InputHandler::new());
    press_button(&mut input, MouseButton::Right, strip.center());
    frame(&mut ui, &input, |ui| dock.render(ui, &theme));
    assert_eq!(dock.take_tab_context_request().map(|(id, _)| id), Some(PanelId::HIERARCHY));
}

#[test]
fn the_budget_collapses_the_furthest_group_on_either_edge_before_a_centre_adjacent_one() {
    let mut dock = dock();
    dock.move_panel_to_edge(PanelId::HIERARCHY, DockPosition::Top);
    dock.move_panel_to_edge(PanelId::INSPECTOR, DockPosition::Top);
    dock.show_panel(PanelId::CONSOLE);
    dock.move_panel_to_edge(PanelId::CONSOLE, DockPosition::Bottom);
    let group = |dock: &DockArea, id: PanelId| dock.group_of(id).unwrap();
    let far = [group(&dock, PanelId::HIERARCHY), group(&dock, PanelId::ASSET_BROWSER)];
    let near = [group(&dock, PanelId::INSPECTOR), group(&dock, PanelId::CONSOLE)];
    dock.set_bounds(Rect::new(0.0, 0.0, 1280.0, 100.0));
    dock.layout();
    let collapsed = dock.budget_collapsed.clone();
    assert_eq!(collapsed.len(), 4, "fixture: every group collapses at this height");
    assert!(far.iter().all(|index| collapsed[..2].contains(index)), "the two groups at the edges go first: {collapsed:?}");
    assert!(near.iter().all(|index| collapsed[2..].contains(index)), "the two beside the centre go last: {collapsed:?}");
}

#[test]
fn a_tab_click_inside_the_narrow_overlay_switches_the_tab_without_writing_the_saved_collapse() {
    let mut dock = dock();
    dock.set_panel_visible(PanelId::SETTINGS, true);
    dock.activate(PanelId::INSPECTOR);
    dock.set_panel_collapsed(PanelId::INSPECTOR, true);
    dock.set_bounds(Rect::new(0.0, 0.0, 320.0, 600.0));
    dock.layout();
    assert!(dock.is_narrow(), "fixture: the right group is a narrow tab");
    dock.open_narrow_overlay(PanelId::INSPECTOR);
    drawn(&mut dock);
    let settings_tab = dock.tab_bounds(PanelId::SETTINGS).unwrap().center();
    let theme = EditorTheme::default();
    let (mut ui, mut input) = (ui::UIContext::new(), input::InputHandler::new());
    press_at(&mut ui, &mut input, settings_tab, |ui| dock.render(ui, &theme));
    release(&mut ui, &mut input, |ui| dock.render(ui, &theme));
    assert_eq!(dock.narrow_overlay(), Some(PanelId::SETTINGS), "the overlay now shows the clicked tab");
    assert!(dock.groups()[dock.group_of(PanelId::INSPECTOR).unwrap()].collapsed, "the desktop collapse preference survives the visit");
    let mut saved = EditorPreferences::default();
    saved.capture_panels(&dock);
    assert!(saved.groups.iter().any(|group| group.panels.contains(&PanelId::INSPECTOR.0) && group.collapsed));
}

#[test]
fn no_tab_answers_under_an_open_overlay_and_a_drag_rests_before_a_tab_comes_forward() {
    let mut dock = dock();
    dock.set_bounds(Rect::new(0.0, 0.0, 320.0, 600.0));
    dock.layout();
    dock.open_narrow_overlay(PanelId::INSPECTOR);
    let overlay = dock.groups()[dock.group_of(PanelId::INSPECTOR).unwrap()].bounds();
    let assets_tab = dock.tab_bounds(PanelId::ASSET_BROWSER).unwrap();
    let covered = Vec2::new(overlay.x + 8.0, assets_tab.center().y);
    assert!(assets_tab.contains(covered) && overlay.contains(covered), "fixture: the Assets tab runs under the overlay");
    assert_eq!(dock.tab_at(covered), None, "the overlay floats above the strip it covers");
    let clear = Vec2::new(overlay.x - 8.0, assets_tab.center().y);
    assert_eq!(dock.tab_at(clear), Some(PanelId::ASSET_BROWSER));
    for frame in 1..DRAG_DWELL_FRAMES {
        assert_eq!(dock.tab_rested_on(clear), None, "frame {frame} is not a rest yet");
    }
    assert_eq!(dock.tab_rested_on(clear), Some(PanelId::ASSET_BROWSER), "the dwell frame answers once");
    assert_eq!(dock.tab_rested_on(clear), None, "and only once");
    dock.end_drag();
    assert_eq!(dock.tab_rested_on(clear), None, "a new drag starts its rest from nothing");
}
