//! Shared Window and tab-menu actions, including overlap with the real narrow dock.

use super::*;
use crate::{DockAction, DockPosition, EditorContext, PanelId, context_menu::{ContextMenu, ContextMenuKey, ContextMenuAction}};
use crate::test_support::{frame, press_at, release};
use glam::Vec2;
use winit::keyboard::KeyCode;

#[test]
fn window_checks_follow_a_hide_chosen_from_the_tabs_menu() {
    let mut context = EditorContext::new();
    context.menu_bar.sync_window(&context.dock_area);
    let mut menu = ContextMenu::default();
    menu.open_for_tab(Vec2::ZERO, PanelId::INSPECTOR, &context.dock_area);
    assert_eq!(menu.labels(), ["Move to", "Group with", "Hide"]);
    assert_eq!(menu.handle_key(ContextMenuKey::Choose), None, "Enter without a highlighted row fires nothing");
    menu.handle_key(ContextMenuKey::Up);
    let action = menu.handle_key(ContextMenuKey::Choose);
    assert_eq!(action, Some(ContextMenuAction::Dock(DockAction::Hide(PanelId::INSPECTOR))));
    context.dock_area.set_panel_visible(PanelId::INSPECTOR, false);
    context.menu_bar.sync_window(&context.dock_area);
    let window = context.menu_bar.menus().iter().find(|menu| menu.title == "Window").unwrap();
    assert!(window.items.iter().any(|item| matches!(item,
        MenuItem::Submenu { label, checked: false, .. } if label == "Inspector")));
}

#[test]
fn the_tab_menu_walks_move_and_group_submenus_and_returns_typed_actions() {
    let context = EditorContext::new();
    let mut menu = ContextMenu::default();
    menu.open_for_tab(Vec2::ZERO, PanelId::INSPECTOR, &context.dock_area);
    menu.handle_key(ContextMenuKey::Down);
    menu.handle_key(ContextMenuKey::Right);
    assert_eq!(menu.handle_key(ContextMenuKey::Choose), Some(ContextMenuAction::Dock(DockAction::MoveTo(PanelId::INSPECTOR, DockPosition::Left))));
    menu.open_for_tab(Vec2::ZERO, PanelId::INSPECTOR, &context.dock_area);
    menu.handle_key(ContextMenuKey::Down);
    menu.handle_key(ContextMenuKey::Down);
    menu.handle_key(ContextMenuKey::Right);
    assert_eq!(menu.handle_key(ContextMenuKey::Choose), Some(ContextMenuAction::Dock(DockAction::GroupWith(PanelId::INSPECTOR, PanelId::HIERARCHY))));
}

#[test]
fn a_file_dropdown_draws_above_the_narrow_overlay_and_its_row_alone_receives_the_click() {
    let mut context = EditorContext::new();
    context.dock_area.set_bounds(Rect::new(0.0, 24.0, 390.0, 600.0));
    context.dock_area.layout();
    context.dock_area.show_panel(PanelId::HIERARCHY);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    context.menu_bar.open_menu = Some(0);
    context.menu_bar.layout_titles(390.0);
    let mut underlying_clicked = false;
    let row = Rect::new(20.0, 104.0, 100.0, 20.0);
    let mut draw = |ui: &mut UIContext| {
        let chosen = context.menu_bar.render(ui, 390.0, &context.theme);
        context.dock_area.render(ui, &context.theme);
        let content = context.dock_area.get_panel(PanelId::HIERARCHY).unwrap().content_bounds();
        ui.begin_overlay_in(ui::UiLayer::Floating, content);
        underlying_clicked |= ui.button("under_menu", "Under menu", row);
        ui.end_overlay();
        chosen
    };
    frame(&mut ui, &input, &mut draw);
    let deepest = ui.draw_list().commands().iter().map(|command| command.depth()).fold(f32::MIN, f32::max);
    assert!(deepest >= ui::UiLayer::Modal.depth_base());
    press_at(&mut ui, &mut input, row.center(), &mut draw);
    assert_eq!(release(&mut ui, &mut input, &mut draw), Some("Save".into()));
    assert!(!underlying_clicked);
    assert_eq!(context.dock_area.narrow_overlay(), Some(PanelId::HIERARCHY));
}

#[test]
fn the_window_keyboard_returns_the_chosen_edge_and_opening_a_row_changes_nothing() {
    let mut context = EditorContext::new();
    context.menu_bar.sync_window(&context.dock_area);
    context.menu_bar.open_menu = context.menu_bar.menus.iter().position(|menu| menu.title == "Window");
    context.menu_bar.handle_key(KeyCode::ArrowDown);
    context.menu_bar.handle_key(KeyCode::ArrowRight);
    assert_eq!(context.menu_bar.take_dock_action(), None, "opening Hierarchy's submenu asks for nothing");
    context.menu_bar.handle_key(KeyCode::ArrowRight);
    context.menu_bar.handle_key(KeyCode::ArrowDown);
    assert_eq!(context.menu_bar.handle_key(KeyCode::Enter), Some("Right".into()));
    assert_eq!(context.menu_bar.take_dock_action(), Some(DockAction::MoveTo(PanelId::HIERARCHY, DockPosition::Right)));
}

#[test]
fn a_hidden_panels_window_row_ends_in_show_and_choosing_it_asks_to_show_the_panel() {
    let mut context = EditorContext::new();
    assert!(!context.dock_area.get_panel(PanelId::CONSOLE).unwrap().visible, "fixture: the Console starts hidden");
    context.menu_bar.sync_window(&context.dock_area);
    context.menu_bar.open_menu = context.menu_bar.menus.iter().position(|menu| menu.title == "Window");
    let console_row = context.menu_bar.menus[context.menu_bar.open_menu.unwrap()].items.iter()
        .position(|item| matches!(item, MenuItem::Submenu { label, .. } if label == "Console")).unwrap();
    for _ in 0..=console_row { context.menu_bar.handle_key(KeyCode::ArrowDown); }
    context.menu_bar.handle_key(KeyCode::ArrowRight);
    assert!(!context.dock_area.get_panel(PanelId::CONSOLE).unwrap().visible, "browsing the row shows nothing");
    let labels: Vec<String> = context.menu_bar.active_items().iter().filter_map(MenuItem::label).map(str::to_string).collect();
    assert_eq!(labels, ["Move to", "Group with", "Show"]);
    context.menu_bar.handle_key(KeyCode::ArrowDown);
    context.menu_bar.handle_key(KeyCode::ArrowDown);
    assert_eq!(context.menu_bar.handle_key(KeyCode::Enter), Some("Show".into()));
    assert_eq!(context.menu_bar.take_dock_action(), Some(DockAction::Show(PanelId::CONSOLE)));
    let mut menu = ContextMenu::default();
    menu.open_for_tab(Vec2::ZERO, PanelId::INSPECTOR, &context.dock_area);
    assert_eq!(menu.labels(), ["Move to", "Group with", "Hide"], "a visible panel's row ends in Hide");
}

#[test]
fn the_toolbar_overflow_takes_its_click_above_the_real_narrow_inspector_overlay() {
    let mut context = EditorContext::new();
    context.update_layout(Vec2::new(311.0, 720.0));
    context.dock_area.show_panel(PanelId::INSPECTOR);
    let strip = context.toolbar_strip_bounds().unwrap();
    let layout = crate::toolbar_strip::layout(strip, &context.toolbar, &context.play_controls, context.play_state());
    let button = layout.overflow_button.unwrap();
    let menu = crate::overflow_menu::overflow_menu_bounds(button,
        5.0 * crate::layout::ROW_HEIGHT + 8.0, Vec2::new(311.0, 720.0), strip.y);
    let shed_tools = crate::EditorTool::all().len() - layout.visible_tools;
    let separator = if shed_tools > 0 { 6.0 } else { 0.0 };
    let row = Rect::new(menu.x + 4.0, menu.y + 4.0 + separator + (shed_tools + 1) as f32 * crate::layout::ROW_HEIGHT,
        menu.width - 8.0, crate::layout::ROW_HEIGHT);
    context.toolbar.toggle_overflow();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let mut beneath = false;
    let mut draw = |ui: &mut UIContext| {
        let picked = crate::overflow_menu::render_overflow_menu(&mut context.toolbar, ui, &context.theme, &layout, &context.view);
        context.dock_area.render(ui, &context.theme);
        let content = context.dock_area.get_panel(PanelId::INSPECTOR).unwrap().content_bounds();
        assert!(content.contains(row.center()), "the menu row covers the live overlay");
        ui.begin_overlay_in(ui::UiLayer::Floating, content);
        beneath |= ui.button("under_overflow", "Under overflow", row);
        ui.end_overlay();
        picked
    };
    crate::test_support::press_mouse(&mut input, row.center());
    ui.begin_frame(&input, Vec2::new(311.0, 720.0));
    draw(&mut ui);
    ui.end_frame();
    crate::test_support::release_mouse(&mut input);
    ui.begin_frame(&input, Vec2::new(311.0, 720.0));
    let picked = draw(&mut ui);
    ui.end_frame();
    assert_eq!(picked, Some(crate::overflow_menu::OverflowPick::Toggle(crate::ViewToggle::Colliders)));
    assert!(!beneath);
}
