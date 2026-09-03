//! Menu bar contracts: an open dropdown floats above everything and swallows
//! the clicks under it, an item click returns its label, and the View
//! menu's check marks and disabled entries are what the host set.

use super::*;
use crate::test_support::{frame, press_at, release, WINDOW};
use crate::theme::EditorTheme;

/// A bar with one open "File" menu: an enabled item, a separator and a
/// disabled item, laid out for the harness window.
fn open_file_menu() -> MenuBar {
    let mut bar = MenuBar::new();
    bar.add_menu(Menu::new("File").with_items(vec![
        MenuItem::action("New"),
        MenuItem::separator(),
        MenuItem::action("Locked").with_enabled(false),
    ]));
    bar.layout_titles(WINDOW.x);
    bar.open_menu = Some(0);
    bar
}

/// Center of the dropdown row at `index` (separators occupy a row too).
fn item_center(bar: &MenuBar, index: usize) -> Vec2 {
    let menu = &bar.menus[0];
    let dropdown = MenuBar::dropdown_bounds(menu, menu.bounds);
    Vec2::new(
        dropdown.x + dropdown.width / 2.0,
        dropdown.y + 4.0 + DROPDOWN_ITEM_HEIGHT * (index as f32 + 0.5),
    )
}

/// An open dropdown renders in the Floating band above panels and the
/// toolbar, and the widgets underneath it are input-blocked.
#[test]
fn test_open_dropdown_renders_in_overlay_band_and_blocks_input() {
    let mut bar = open_file_menu();
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();

    ui.begin_frame(&input, WINDOW);
    bar.render(&mut ui, WINDOW.x, &theme);
    let dropdown = MenuBar::dropdown_bounds(&bar.menus[0], bar.menus[0].bounds);
    assert!(ui.is_input_blocked_at(dropdown.center()), "mouse input under the dropdown is swallowed");
    assert_eq!(ui.draw_list().current_layer(), ui::UiLayer::Content, "the overlay scope was closed");
    ui.end_frame();

    let max_depth = ui.draw_list().commands().iter().map(|c| c.depth()).fold(f32::MIN, f32::max);
    assert!(
        max_depth >= ui::UiLayer::Floating.depth_base(),
        "dropdown must render in the Floating band, got {max_depth}"
    );
}

/// A click on an item fires on the release frame, returns the item's label
/// and closes the menu; a disabled item returns nothing and leaves the
/// menu open.
#[test]
fn test_item_click_returns_its_label_and_a_disabled_item_returns_nothing() {
    let mut bar = open_file_menu();
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    let pressed = press_at(&mut ui, &mut input, item_center(&bar, 0), |ui| bar.render(ui, WINDOW.x, &theme));
    assert_eq!(pressed, None, "clicks fire on release");
    assert_eq!(bar.open_menu, Some(0), "a press inside the dropdown keeps it open");
    let clicked = release(&mut ui, &mut input, |ui| bar.render(ui, WINDOW.x, &theme));
    assert_eq!(clicked.as_deref(), Some("New"));
    assert_eq!(bar.open_menu, None, "an item click closes the menu");

    bar.open_menu = Some(0);
    press_at(&mut ui, &mut input, item_center(&bar, 2), |ui| bar.render(ui, WINDOW.x, &theme));
    let clicked = release(&mut ui, &mut input, |ui| bar.render(ui, WINDOW.x, &theme));
    assert_eq!(clicked, None, "a disabled item is not clickable");
    assert_eq!(bar.open_menu, Some(0), "the menu stays open");
}

/// A press outside the dropdown closes it; a press on the open menu's own
/// title does not (the title's release toggles it — closing on press too
/// would flicker it closed and open again).
#[test]
fn test_outside_press_closes_the_menu_but_the_open_title_waits_for_release() {
    let mut bar = open_file_menu();
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();

    let title_center = bar.menus[0].bounds.center();
    press_at(&mut ui, &mut input, title_center, |ui| bar.render(ui, WINDOW.x, &theme));
    assert_eq!(bar.open_menu, Some(0), "a press on the open title keeps the menu open");
    release(&mut ui, &mut input, |ui| bar.render(ui, WINDOW.x, &theme));
    assert_eq!(bar.open_menu, None, "the title's release toggles it closed");

    bar.open_menu = Some(0);
    let clicked = press_at(&mut ui, &mut input, Vec2::new(700.0, 500.0), |ui| bar.render(ui, WINDOW.x, &theme));
    assert_eq!(clicked, None);
    assert_eq!(bar.open_menu, None, "a press outside must close the dropdown");
}

/// The View menu carries a check mark per toggle the host sets, and the
/// panels that do not exist yet are disabled rather than silently inert.
#[test]
fn test_view_menu_check_marks_follow_the_host_and_missing_panels_are_disabled() {
    let mut bar = MenuBar::editor_default();
    assert_eq!(bar.is_checked("View", "Inspector"), Some(false));
    bar.set_checked("View", "Inspector", true);
    assert_eq!(bar.is_checked("View", "Inspector"), Some(true));
    bar.set_checked("Nope", "Inspector", true);
    assert_eq!(bar.is_checked("Nope", "Inspector"), None, "an unknown menu is ignored");
    assert_eq!(bar.is_checked("View", "Nope"), None, "an unknown label is absent");

    let view = bar.menus.iter().find(|menu| menu.title == "View").expect("a View menu");
    for item in &view.items {
        if let MenuItem::Action { label, enabled, .. } = item {
            let expected = !matches!(label.as_str(), "Scene View" | "Console");
            assert_eq!(*enabled, expected, "{label}");
        }
    }

    // Titles are laid out left to right without overlapping.
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    frame(&mut ui, &input, |ui| bar.render(ui, WINDOW.x, &theme));
    for pair in bar.menus.windows(2) {
        let (left, right) = (pair[0].bounds, pair[1].bounds);
        assert!(left.x + left.width <= right.x, "menu '{}' overlaps '{}'", pair[0].title, pair[1].title);
    }
}
