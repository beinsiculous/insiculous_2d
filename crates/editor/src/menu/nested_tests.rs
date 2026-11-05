use glam::Vec2;
use input::{InputHandler, prelude::MouseButton};
use winit::keyboard::KeyCode;
use ui::{DrawCommand, Rect, UIContext};
use super::{Menu, MenuBar, MenuItem};
use crate::theme::EditorTheme;

fn menu() -> MenuBar {
    let mut bar = MenuBar::new();
    bar.add_menu(Menu::new("View").with_items(vec![
        MenuItem::submenu("Nested", vec![MenuItem::action("Choose child")]),
        MenuItem::action("Parent action"),
    ]));
    bar.open_menu = Some(0);
    bar
}

fn frame(bar: &mut MenuBar, ui: &mut UIContext, input: &InputHandler, window: Vec2) -> Option<String> {
    ui.begin_frame(input, window);
    let selected = bar.render(ui, ui.window_size().x, &EditorTheme::default());
    ui.end_frame();
    selected
}

fn popup_bounds(ui: &UIContext) -> Vec<Rect> {
    ui.draw_list().commands().iter().filter_map(|command| match command {
        DrawCommand::RectBorder { bounds, color, .. } if *color == EditorTheme::default().popup_border => Some(*bounds),
        _ => None,
    }).collect()
}

#[test]
fn a_hovered_submenu_fires_its_child_and_closes_the_whole_menu() {
    let mut bar = menu();
    let mut ui = UIContext::new();
    let mut input = InputHandler::new();
    let window = Vec2::new(800.0, 600.0);
    input.mouse_mut().update_position(80.0, 40.0);
    frame(&mut bar, &mut ui, &input, window);
    assert_eq!(bar.submenu_path, vec![0]);
    let child = *popup_bounds(&ui).last().unwrap();
    input.update();
    input.mouse_mut().update_position(child.center().x, child.center().y);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    assert_eq!(frame(&mut bar, &mut ui, &input, window), None);
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    assert_eq!(frame(&mut bar, &mut ui, &input, window).as_deref(), Some("Choose child"));
    assert!(!bar.is_open());
    assert!(bar.submenu_path.is_empty());
}

#[test]
fn arrows_walk_children_left_returns_and_escape_closes_every_level() {
    let mut bar = menu();
    bar.handle_key(KeyCode::ArrowDown);
    bar.handle_key(KeyCode::ArrowRight);
    assert_eq!(bar.handle_key(KeyCode::Enter).as_deref(), Some("Choose child"));
    assert!(!bar.is_open());
    bar.open_menu = Some(0);
    bar.handle_key(KeyCode::ArrowRight);
    bar.handle_key(KeyCode::ArrowLeft);
    assert!(bar.submenu_path.is_empty());
    bar.handle_key(KeyCode::ArrowRight);
    bar.handle_key(KeyCode::Escape);
    assert!(!bar.is_open());
    assert!(bar.submenu_path.is_empty());
}

#[test]
fn narrow_submenus_stay_in_the_window_and_the_child_blocks_covered_parent_rows() {
    for (window, scale) in [(Vec2::new(390.0, 844.0), 1.5), (Vec2::new(1280.0, 800.0), 2.0)] {
        let mut bar = menu();
        let mut ui = UIContext::new();
        ui.set_pixels_per_point(scale);
        let mut input = InputHandler::new();
        input.mouse_mut().update_position(30.0 * scale, 40.0 * scale);
        frame(&mut bar, &mut ui, &input, window);
        let panels = popup_bounds(&ui);
        assert_eq!(panels.len(), 2);
        for panel in &panels {
            assert!(panel.x >= 0.0 && panel.y >= 0.0 && panel.right() <= window.x && panel.bottom() <= window.y);
        }
        let child = panels[1];
        input.update();
        input.mouse_mut().update_position(child.center().x, child.center().y);
        input.mouse_mut().handle_button_press(MouseButton::Left);
        frame(&mut bar, &mut ui, &input, window);
        input.update();
        input.mouse_mut().handle_button_release(MouseButton::Left);
        assert_eq!(frame(&mut bar, &mut ui, &input, window).as_deref(), Some("Choose child"));
    }
}

#[test]
fn a_move_and_press_in_one_frame_on_a_narrow_submenu_opens_it_on_release_and_selects_nothing() {
    let window = Vec2::new(390.0, 844.0);
    let mut bar = menu();
    let mut ui = UIContext::new();
    ui.set_pixels_per_point(1.5);
    let mut input = InputHandler::new();
    input.mouse_mut().update_position(30.0 * 1.5, 40.0 * 1.5);
    input.mouse_mut().handle_button_press(MouseButton::Left);
    assert_eq!(frame(&mut bar, &mut ui, &input, window), None);
    assert!(bar.submenu_path.is_empty(), "the press frame opens no child under the pointer");
    input.update();
    input.mouse_mut().handle_button_release(MouseButton::Left);
    assert_eq!(frame(&mut bar, &mut ui, &input, window), None);
    assert_eq!(bar.submenu_path, vec![0], "the release opens the submenu");
    assert!(bar.is_open());
}

#[test]
fn enter_with_nothing_highlighted_fires_nothing() {
    let mut bar = menu();
    assert_eq!(bar.handle_key(KeyCode::Enter), None);
    assert!(bar.is_open());
    bar.handle_key(KeyCode::ArrowDown);
    bar.handle_key(KeyCode::ArrowDown);
    assert_eq!(bar.handle_key(KeyCode::Enter).as_deref(), Some("Parent action"));
}

#[test]
fn scale_menu_checks_the_preference_and_names_the_live_display() {
    use crate::editor_preferences::UiScale;
    let mut bar = MenuBar::editor_default();
    bar.sync_ui_scale(UiScale::Percent(150), 1.25);
    let scale_items = |bar: &MenuBar| {
        bar.menus.iter().flat_map(|menu| &menu.items).find_map(|item| match item {
            MenuItem::Submenu { label, items, .. } if label == "UI Scale" => Some(items.clone()),
            _ => None,
        }).unwrap()
    };
    let items = scale_items(&bar);
    assert_eq!(items[0].label(), Some("Follow the display (125 %)"));
    let checked: Vec<_> = items.iter().filter_map(|item| match item {
        MenuItem::Action { label, checked: true, .. } => Some(label.as_str()),
        _ => None,
    }).collect();
    assert_eq!(checked, ["150 %"]);
    for item in &items {
        if let Some(label) = item.label() { assert!(super::action_for_menu_label(label).is_some()); }
    }
    for (label, shortcut, action) in [
        ("Follow the display (125 %)", "Ctrl+0", crate::EditorAction::SetUiScale(UiScale::FollowDisplay)),
        ("Larger text", "Ctrl++", crate::EditorAction::UiScaleUp),
        ("Smaller text", "Ctrl+-", crate::EditorAction::UiScaleDown),
    ] {
        assert!(items.iter().any(|item| matches!(item,
            MenuItem::Action { label: text, shortcut: Some(hint), .. } if text == label && hint == shortcut)));
        assert_eq!(super::action_for_menu_label(label), Some(action));
    }
    bar.sync_ui_scale(UiScale::FollowDisplay, 1.0);
    assert!(matches!(&scale_items(&bar)[0], MenuItem::Action { label, checked: true, .. } if label == "Follow the display (100 %)"));
}
