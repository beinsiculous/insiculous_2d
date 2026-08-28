use super::*;

#[test]
fn test_menu_item_action() {
    let item = MenuItem::action("Test");
    assert_eq!(item.label(), Some("Test"));
}

#[test]
fn test_menu_item_with_shortcut() {
    let item = MenuItem::action_with_shortcut("Save", "Ctrl+S");
    if let MenuItem::Action { label, shortcut, enabled, checked } = item {
        assert_eq!(label, "Save");
        assert_eq!(shortcut, Some("Ctrl+S".to_string()));
        assert!(enabled);
        assert!(!checked);
    } else {
        panic!("Expected Action variant");
    }
}

#[test]
fn test_menu_bar_set_checked_and_is_checked() {
    let mut bar = MenuBar::editor_default();
    assert_eq!(bar.is_checked("View", "Inspector"), Some(false));

    bar.set_checked("View", "Inspector", true);
    assert_eq!(bar.is_checked("View", "Inspector"), Some(true));

    bar.set_checked("View", "Inspector", false);
    assert_eq!(bar.is_checked("View", "Inspector"), Some(false));

    // Unknown menu/label are ignored / absent
    bar.set_checked("Nope", "Inspector", true);
    assert_eq!(bar.is_checked("Nope", "Inspector"), None);
    assert_eq!(bar.is_checked("View", "Nope"), None);
}

#[test]
fn test_menu_item_with_checked_builder() {
    let item = MenuItem::action("Toggle Grid").with_checked(true);
    if let MenuItem::Action { checked, .. } = item {
        assert!(checked);
    } else {
        panic!("Expected Action variant");
    }
}

#[test]
fn test_view_menu_disables_unimplemented_panels() {
    let bar = MenuBar::editor_default();
    let view = bar.menus.iter().find(|m| m.title == "View").unwrap();
    for item in &view.items {
        if let MenuItem::Action { label, enabled, .. } = item {
            match label.as_str() {
                "Scene View" | "Console" => assert!(!enabled, "{label} must be disabled"),
                _ => assert!(enabled, "{label} must be enabled"),
            }
        }
    }
}

#[test]
fn test_menu_item_separator() {
    let item = MenuItem::separator();
    assert!(matches!(item, MenuItem::Separator));
    assert!(item.label().is_none());
}

#[test]
fn test_menu_item_submenu() {
    let item = MenuItem::submenu("Create", vec![MenuItem::action("Sprite")]);
    if let MenuItem::Submenu { label, items } = item {
        assert_eq!(label, "Create");
        assert_eq!(items.len(), 1);
    } else {
        panic!("Expected Submenu variant");
    }
}

#[test]
fn test_menu_item_with_enabled() {
    let item = MenuItem::action("Test").with_enabled(false);
    if let MenuItem::Action { enabled, .. } = item {
        assert!(!enabled);
    } else {
        panic!("Expected Action variant");
    }
}

#[test]
fn test_menu_new() {
    let menu = Menu::new("File");
    assert_eq!(menu.title, "File");
    assert!(menu.items.is_empty());
    assert!(!menu.open);
}

#[test]
fn test_menu_add_item() {
    let menu = Menu::new("File")
        .add_item(MenuItem::action("New"))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::action("Exit"));

    assert_eq!(menu.items.len(), 3);
}

#[test]
fn test_menu_with_items() {
    let menu = Menu::new("Edit").with_items(vec![
        MenuItem::action("Undo"),
        MenuItem::action("Redo"),
    ]);

    assert_eq!(menu.items.len(), 2);
}

#[test]
fn test_menu_visible_item_count() {
    let menu = Menu::new("File").with_items(vec![
        MenuItem::action("New"),
        MenuItem::separator(),
        MenuItem::action("Exit"),
    ]);

    // Separators don't count
    assert_eq!(menu.visible_item_count(), 2);
}

#[test]
fn test_menu_bar_new() {
    let bar = MenuBar::new();
    assert!(bar.menus.is_empty());
    assert!(bar.open_menu.is_none());
}

#[test]
fn test_menu_bar_add_menu() {
    let mut bar = MenuBar::new();
    bar.add_menu(Menu::new("File"));
    bar.add_menu(Menu::new("Edit"));

    assert_eq!(bar.menus.len(), 2);
}

#[test]
fn test_menu_bar_editor_default() {
    let bar = MenuBar::editor_default();

    // Should have File, Edit, View, Entity menus
    assert_eq!(bar.menus.len(), 4);
    assert_eq!(bar.menus[0].title, "File");
    assert_eq!(bar.menus[1].title, "Edit");
    assert_eq!(bar.menus[2].title, "View");
    assert_eq!(bar.menus[3].title, "Entity");
}

#[test]
fn test_menu_bar_height() {
    let bar = MenuBar::new();
    assert_eq!(bar.height(), 24.0);
}

#[test]
fn test_menu_bar_close_all() {
    let mut bar = MenuBar::new();
    bar.add_menu(Menu::new("File"));
    bar.open_menu = Some(0);

    bar.close_all();
    assert!(bar.open_menu.is_none());
}

#[test]
fn test_layout_titles_sets_bar_bounds_to_window_width() {
    let mut bar = MenuBar::editor_default();
    bar.layout_titles(1280.0);
    assert_eq!(bar.bounds.width, 1280.0);
    assert_eq!(bar.bounds.height, bar.height());
}

#[test]
fn test_layout_titles_do_not_overlap() {
    let mut bar = MenuBar::editor_default();
    bar.layout_titles(1280.0);

    for pair in bar.menus.windows(2) {
        let left = pair[0].bounds;
        let right = pair[1].bounds;
        assert!(
            left.x + left.width <= right.x,
            "menu '{}' overlaps '{}'",
            pair[0].title,
            pair[1].title
        );
    }
}

#[test]
fn test_layout_title_width_scales_with_title_length() {
    let mut bar = MenuBar::new();
    bar.add_menu(Menu::new("File"));
    bar.add_menu(Menu::new("MuchLongerTitle"));
    bar.layout_titles(800.0);

    assert!(bar.menus[1].bounds.width > bar.menus[0].bounds.width);
}

#[test]
fn test_should_close_on_press_geometry() {
    let dropdown = Rect::new(8.0, 24.0, 200.0, 100.0);
    let title = Rect::new(8.0, 0.0, 60.0, 24.0);

    // Press far away → close
    assert!(MenuBar::should_close_on_press(Vec2::new(500.0, 300.0), dropdown, title));
    // Press inside the dropdown → keep open (item interaction)
    assert!(!MenuBar::should_close_on_press(Vec2::new(50.0, 60.0), dropdown, title));
    // Press on the open menu's own title → keep open (release toggles it)
    assert!(!MenuBar::should_close_on_press(Vec2::new(30.0, 10.0), dropdown, title));
}

/// Build an InputHandler with the mouse at the given position, pressed.
fn pressed_mouse_at(x: f32, y: f32) -> input::InputHandler {
    let mut input = input::InputHandler::new();
    input.mouse_mut().update_position(x, y);
    input.mouse_mut().handle_button_press(winit::event::MouseButton::Left);
    input
}

#[test]
fn test_outside_press_closes_open_menu() {
    let mut bar = MenuBar::editor_default();
    bar.layout_titles(1280.0);
    bar.open_menu = Some(0);

    let mut ui = UIContext::new();
    let input = pressed_mouse_at(900.0, 400.0); // far from menu + dropdown
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    let clicked = bar.render(&mut ui, 1280.0, &crate::theme::EditorTheme::default());
    ui.end_frame();

    assert!(clicked.is_none());
    assert!(bar.open_menu.is_none(), "press outside must close the dropdown");
}

#[test]
fn test_press_on_open_title_keeps_menu_open_until_release() {
    let mut bar = MenuBar::editor_default();
    bar.layout_titles(1280.0);
    bar.open_menu = Some(0);
    let title_center = bar.menus[0].bounds.center();

    let mut ui = UIContext::new();
    let input = pressed_mouse_at(title_center.x, title_center.y);
    ui.begin_frame(&input, Vec2::new(1280.0, 720.0));
    bar.render(&mut ui, 1280.0, &crate::theme::EditorTheme::default());
    ui.end_frame();

    // The close happens via the title's click (on release), not on press —
    // closing here too would cause a close/reopen flicker.
    assert_eq!(bar.open_menu, Some(0));
}

#[test]
fn test_open_dropdown_renders_in_overlay_band_and_blocks_input() {
    let mut bar = MenuBar::editor_default();
    bar.layout_titles(1280.0);
    bar.open_menu = Some(0);

    let mut ui = UIContext::new();
    ui.begin_frame(&input::InputHandler::new(), Vec2::new(1280.0, 720.0));
    bar.render(&mut ui, 1280.0, &crate::theme::EditorTheme::default());

    // Mouse input under the dropdown is swallowed for later widgets
    let dropdown = MenuBar::dropdown_bounds(&bar.menus[0], bar.menus[0].bounds);
    assert!(ui.is_input_blocked_at(dropdown.center()));

    // Overlay mode was properly closed: subsequent draws are content band
    assert_eq!(ui.draw_list().current_layer(), ui::UiLayer::Content);

    // Dropdown draws above the content band (Floating layer)
    ui.end_frame();
    let max_depth = ui
        .draw_list()
        .commands()
        .iter()
        .map(|c| c.depth())
        .fold(f32::MIN, f32::max);
    assert!(
        max_depth >= ui::UiLayer::Floating.depth_base(),
        "dropdown must render in the Floating band, got {max_depth}"
    );
}

#[test]
fn test_apply_toggle_opens_and_closes() {
    let mut bar = MenuBar::editor_default();

    bar.apply_toggle(Some(1));
    assert_eq!(bar.open_menu, Some(1));

    // Clicking the same title again closes it
    bar.apply_toggle(Some(1));
    assert!(bar.open_menu.is_none());

    // No click leaves state unchanged
    bar.apply_toggle(Some(2));
    bar.apply_toggle(None);
    assert_eq!(bar.open_menu, Some(2));
}
