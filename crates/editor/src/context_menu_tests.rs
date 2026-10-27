//! The context menu's contracts: the keys walk only enabled items and choose one,
//! Escape and a press outside close it choosing nothing, the outside press reaches no
//! widget beneath, and on the web the file items are disabled over the desktop hint.

use super::*;
use crate::external_tools::{Location, PresetId, ToolChoice};
use crate::test_support::{frame, press_at, release};

fn file_menu(desktop: bool) -> ContextMenu {
    let config = ExternalToolsConfig { art: ToolChoice::Preset(PresetId::Aseprite), ..Default::default() };
    let mut menu = ContextMenu::default();
    menu.open_for_file(Vec2::new(100.0, 100.0), "sprites/hero.png", &config, desktop);
    menu
}

fn open_request(path: &str) -> Option<ContextMenuAction> {
    Some(ContextMenuAction::Open(OpenRequest { path: path.into(), location: Location::None }))
}

#[test]
fn test_a_files_menu_names_its_tool_and_down_then_enter_chooses_the_second_item() {
    let mut menu = file_menu(true);
    assert_eq!(menu.labels(), vec!["Open in Aseprite", "Open containing folder"]);

    assert_eq!(menu.handle_key(ContextMenuKey::Down), None);
    assert_eq!(menu.handle_key(ContextMenuKey::Choose), open_request("sprites"));
    assert!(!menu.is_open(), "choosing closes the menu");

    let mut wrapped = file_menu(true);
    wrapped.handle_key(ContextMenuKey::Up);
    assert_eq!(wrapped.handle_key(ContextMenuKey::Choose), open_request("sprites"), "Up from the first wraps to the last");
}

#[test]
fn test_escape_closes_the_menu_choosing_nothing() {
    let mut menu = file_menu(true);
    assert_eq!(menu.handle_key(ContextMenuKey::Close), None);
    assert!(!menu.is_open());
    assert_eq!(menu.handle_key(ContextMenuKey::Choose), None, "a closed menu chooses nothing");
}

#[test]
fn test_a_press_outside_closes_the_menu_and_reaches_nothing_beneath() {
    let theme = EditorTheme::default();
    let mut menu = file_menu(true);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let beneath = Rect::new(400.0, 400.0, 80.0, 24.0);

    let blocked = press_at(&mut ui, &mut input, beneath.center(), |ui| {
        let chosen = menu.render(ui, &theme);
        let pressed = ui.button("beneath", "Beneath", beneath);
        (chosen, pressed, ui.is_input_blocked_at(beneath.center()), ui.wants_mouse())
    });
    assert_eq!(blocked, (None, false, true, false), "the press closes the menu and no widget takes it");
    assert!(!menu.is_open());

    let clicked = release(&mut ui, &mut input, |ui| ui.button("beneath", "Beneath", beneath));
    assert!(!clicked, "the release of the swallowed press is no click");
}

#[test]
fn test_the_menu_draws_above_a_floating_panel_and_its_outside_press_reaches_no_row_there() {
    let theme = EditorTheme::default();
    let mut menu = file_menu(true);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    // A narrow-mode panel overlay: drawn after the menu, in a scope of its own on the
    // Floating band, covering the menu and the press beside it.
    let panel = Rect::new(0.0, 0.0, 600.0, 500.0);
    let row = Rect::new(400.0, 400.0, 80.0, 24.0);
    let mut render = |ui: &mut UIContext| {
        menu.render(ui, &theme);
        ui.begin_overlay_in(ui::UiLayer::Floating, panel);
        let clicked = ui.button("panel_row", "Row", row);
        ui.end_overlay();
        clicked
    };

    frame(&mut ui, &input, &mut render);
    let deepest = ui.draw_list().commands().iter().map(|command| command.depth()).fold(f32::MIN, f32::max);
    assert!(deepest >= ui::UiLayer::Modal.depth_base(), "the menu draws over the panel, not under it");

    press_at(&mut ui, &mut input, row.center(), &mut render);
    let clicked = release(&mut ui, &mut input, &mut render);
    assert!(!clicked, "the press that closes the menu reaches no row of the panel");
    assert!(!menu.is_open());
}

#[test]
fn test_a_label_clears_the_keyboards_ring_by_the_room_the_menu_adds() {
    let theme = EditorTheme::default();
    let mut menu = file_menu(true);
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();
    frame(&mut ui, &input, |ui| menu.render(ui, &theme));

    let commands = ui.draw_list().commands();
    let ring = commands.iter().find_map(|command| match command {
        ui::DrawCommand::RectBorder { bounds, color, .. } if *color == theme.focus_ring => Some(*bounds),
        _ => None,
    });
    // The widest label sets the menu's width, so it is the one the width rule fits
    // tightest; every row's ring stands at the same x.
    let label = commands.iter().find_map(|command| match command {
        ui::DrawCommand::TextPlaceholder { text, position, .. } if text == "Open containing folder" => Some(*position),
        _ => None,
    });
    let (Some(ring), Some(label)) = (ring, label) else {
        panic!("the open menu rings its first item and draws its widest label");
    };
    let gap = label.x - (ring.x + CURSOR_RING_WIDTH);
    assert!(gap >= CURSOR_RING_ROOM, "the label starts {gap} px inside the ring");
}

#[test]
fn test_a_click_on_an_item_chooses_it_and_the_menu_blocks_what_it_covers() {
    let theme = EditorTheme::default();
    let mut menu = file_menu(true);
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    let bounds = frame(&mut ui, &input, |ui| {
        menu.render(ui, &theme);
        menu.bounds(ui, &theme)
    });
    let Some(bounds) = bounds else {
        panic!("an open menu has bounds");
    };
    let first_item = MenuBar::dropdown_item_bounds(bounds, 0).center();

    press_at(&mut ui, &mut input, first_item, |ui| {
        menu.render(ui, &theme);
        assert!(ui.is_input_blocked_at(first_item), "a widget under the menu is inert");
    });
    let chosen = release(&mut ui, &mut input, |ui| menu.render(ui, &theme));
    assert_eq!(chosen, open_request("sprites/hero.png"));
}

#[test]
fn test_on_the_web_both_items_are_disabled_over_the_desktop_hint() {
    let mut menu = file_menu(false);
    assert_eq!(menu.labels(), vec!["Open in Aseprite", "Open containing folder"]);
    assert_eq!(menu.handle_key(ContextMenuKey::Down), None);
    assert_eq!(menu.handle_key(ContextMenuKey::Choose), None, "no enabled item to choose");

    let open = file_menu(false);
    let rows: Vec<String> = open
        .open
        .iter()
        .flat_map(|open| open.as_menu().items)
        .filter_map(|item| item.label().map(str::to_string))
        .collect();
    assert_eq!(rows.last().map(String::as_str), Some("External tools open it from the desktop editor"));
}

#[test]
fn test_a_menu_near_the_windows_corner_is_moved_inside_it() {
    let window = Vec2::new(800.0, 600.0);
    let moved = inside_window(Rect::new(780.0, 590.0, 200.0, 60.0), window);
    assert_eq!((moved.x, moved.y), (600.0, 540.0));
}
