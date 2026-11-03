//! The asset browser's keyboard through the real `EditorGame`: a press in the panel takes
//! it, its keys decide before the editor's so an arrow never nudges an entity behind it,
//! Enter opens the selected tile, and a press in the hierarchy or entering Play lets go.

use ecs::World;
use editor::context_menu::ContextMenuKey;
use editor::{AssetEntry, AssetKind, EditorAction, Modifiers, PanelId, TileStep};
use glam::Vec2;
use input::prelude::MouseButton;
use winit::keyboard::KeyCode;

use super::asset_browser_keys::BrowserKey;
use super::shortcuts::KeyRoute;
use super::test_support::{editor_game, press_mouse, press_play, spawn_at, DummyGame};
use super::EditorGame;

const PLAIN: Modifiers = Modifiers { ctrl: false, shift: false };
const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

fn image(name: &str) -> AssetEntry {
    AssetEntry {
        name: name.to_string(),
        relative_path: format!("sprites/{name}"),
        kind: AssetKind::Image,
        texture_handle: None,
        load_failed: false,
    }
}

/// An editor laid out at the window's size with three image tiles, two to a row, and
/// one entity selected in the scene.
fn browser_session(world: &mut World) -> (EditorGame<DummyGame>, ecs::EntityId) {
    let mut editor = editor_game();
    editor.editor.dock_area.set_bounds(ui::Rect::new(0.0, 24.0, WINDOW.x, WINDOW.y - 48.0));
    editor.editor.dock_area.set_panel_visible(PanelId::ASSET_BROWSER, true);
    editor.editor.asset_browser.apply_scan(vec![image("a.png"), image("b.png"), image("c.png")]);
    editor.editor.asset_browser.columns = 2;
    let entity = spawn_at(world, Vec2::ZERO);
    editor.editor.selection.select(entity);
    (editor, entity)
}

/// A frame whose only input is a left press in `panel`, settled as frame start does.
fn press_in(editor: &mut EditorGame<DummyGame>, panel: PanelId) {
    press_in_with(editor, panel, MouseButton::Left);
}

/// A frame whose only input is a press of `button` in `panel`, the Conductor panel settled
/// before the browser as frame start does.
fn press_in_with(editor: &mut EditorGame<DummyGame>, panel: PanelId, button: MouseButton) {
    let position = editor.editor.dock_area.get_panel(panel).map(|panel| panel.content_bounds().center());
    let Some(position) = position else {
        panic!("{panel:?} is laid out");
    };
    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    if button == MouseButton::Left {
        press_mouse(&mut input, position);
    } else {
        input.update();
        input.mouse_mut().update_position(position.x, position.y);
        input.mouse_mut().handle_button_press(button);
    }
    ui.begin_frame(&input, WINDOW);
    editor.settle_conductor(&ui);
    editor.settle_asset_browser(&ui);
}

/// Route `key` and run it when it is the browser's, as `handle_editor_key` does.
fn press_key(editor: &mut EditorGame<DummyGame>, key: KeyCode, modifiers: Modifiers) -> KeyRoute {
    let route = editor.route_editor_key(key, false, modifiers);
    if let KeyRoute::AssetBrowser(browser_key) = route {
        editor.run_asset_browser_key(browser_key);
    }
    route
}

fn position_of(world: &World, entity: ecs::EntityId) -> Option<Vec2> {
    world.get::<common::Transform2D>(entity).map(|transform| transform.position)
}

#[test]
fn while_the_browser_holds_the_keyboard_an_arrow_steps_the_tile_and_nudges_no_entity_until_a_press_elsewhere() {
    let mut world = World::new();
    let (mut editor, entity) = browser_session(&mut world);
    press_in(&mut editor, PanelId::ASSET_BROWSER);
    assert!(editor.editor.asset_browser.holds_keyboard, "a press in the panel takes the keyboard");

    let steps = [KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowDown];
    for key in steps {
        assert!(matches!(press_key(&mut editor, key, PLAIN), KeyRoute::AssetBrowser(BrowserKey::Step(_))));
    }
    assert_eq!(editor.editor.asset_browser.selected, Some(1), "the first step selects, then right; down has no tile under it");
    assert_eq!(position_of(&world, entity), Some(Vec2::ZERO), "no arrow nudged the selected entity");
    assert!(!matches!(press_key(&mut editor, KeyCode::KeyG, PLAIN), KeyRoute::AssetBrowser(_)), "a key it does not use goes on");

    press_in(&mut editor, PanelId::HIERARCHY);
    assert!(!editor.editor.asset_browser.holds_keyboard, "a press in the hierarchy lets go");
    assert_eq!(
        press_key(&mut editor, KeyCode::ArrowRight, PLAIN),
        KeyRoute::Editor { action: EditorAction::NudgeRight, shift: false },
        "the arrow nudges again"
    );
}

#[test]
fn enter_files_the_selected_tiles_request_and_the_menu_key_or_shift_f10_asks_for_its_menu() {
    let mut world = World::new();
    let (mut editor, _) = browser_session(&mut world);
    press_in(&mut editor, PanelId::ASSET_BROWSER);
    editor.editor.asset_browser.step_selection(TileStep::Down);

    assert_eq!(press_key(&mut editor, KeyCode::Enter, PLAIN), KeyRoute::AssetBrowser(BrowserKey::Open));
    assert_eq!(editor.editor.open_requests, vec![editor::open_file_request("sprites/a.png")]);

    let shift = Modifiers { ctrl: false, shift: true };
    for (key, modifiers) in [(KeyCode::ContextMenu, PLAIN), (KeyCode::F10, shift)] {
        editor.editor.asset_browser.menu_requested = false;
        assert_eq!(press_key(&mut editor, key, modifiers), KeyRoute::AssetBrowser(BrowserKey::OpenMenu));
        assert!(editor.editor.asset_browser.menu_requested, "{key:?} asks the render to open the menu at the tile");
    }
    assert!(!matches!(press_key(&mut editor, KeyCode::F10, PLAIN), KeyRoute::AssetBrowser(_)), "F10 alone goes on");

    press_key(&mut editor, KeyCode::Escape, PLAIN);
    assert!(!editor.editor.asset_browser.holds_keyboard, "Escape lets go");
}

#[test]
fn an_open_context_menu_takes_every_key_and_down_then_enter_files_its_second_item() {
    let mut world = World::new();
    let (mut editor, entity) = browser_session(&mut world);
    let config = editor.editor.external_tools.clone();
    editor.editor.context_menu.open_for_file(Vec2::new(100.0, 400.0), "sprites/a.png", &config, true);

    for key in [KeyCode::ArrowRight, KeyCode::ArrowLeft, KeyCode::Delete] {
        assert_eq!(press_key(&mut editor, key, PLAIN), KeyRoute::Consumed, "the open menu swallows {key:?}");
    }
    assert!(world.get::<common::Transform2D>(entity).is_some(), "Delete deleted nothing behind the menu");
    assert_eq!(
        press_key(&mut editor, KeyCode::ArrowDown, PLAIN),
        KeyRoute::AssetBrowser(BrowserKey::Menu(ContextMenuKey::Down))
    );
    press_key(&mut editor, KeyCode::Enter, PLAIN);
    assert_eq!(editor.editor.open_requests, vec![editor::open_folder_request("sprites/a.png")]);
    assert!(!editor.editor.context_menu.is_open());
    assert_eq!(position_of(&world, entity), Some(Vec2::ZERO));
}

#[test]
fn entering_play_lets_the_keyboard_go_and_closes_the_context_menu() {
    let mut world = World::new();
    let (mut editor, _) = browser_session(&mut world);
    press_in(&mut editor, PanelId::ASSET_BROWSER);
    let config = editor.editor.external_tools.clone();
    editor.editor.context_menu.open_for_file(Vec2::new(100.0, 400.0), "sprites/a.png", &config, true);

    press_play(&mut editor, &mut world);

    assert!(!editor.editor.asset_browser.holds_keyboard);
    assert!(!editor.editor.context_menu.is_open());
    press_in(&mut editor, PanelId::ASSET_BROWSER);
    assert!(!editor.editor.asset_browser.holds_keyboard, "no press takes it during a play session");
}

#[test]
fn a_right_press_in_the_browser_takes_the_keyboard_from_the_conductor_panel() {
    let mut world = World::new();
    let (mut editor, _) = browser_session(&mut world);
    // The Conductor panel's first follow resets its state, focus included; settle it first.
    editor.follow_song();
    editor.show_conductor(editor::conductor::ConductorFocus::Sidebar);
    assert!(editor.conductor_holds_focus(), "the Conductor panel starts with the keyboard");

    press_in_with(&mut editor, PanelId::ASSET_BROWSER, MouseButton::Right);

    assert!(!editor.conductor_holds_focus(), "a right press elsewhere in the dock takes it from the Conductor panel");
    assert!(editor.editor.asset_browser.holds_keyboard, "and gives it to the browser it landed in");
    assert!(matches!(press_key(&mut editor, KeyCode::ArrowRight, PLAIN), KeyRoute::AssetBrowser(BrowserKey::Step(_))));
}

#[test]
fn a_confirm_dialog_coming_up_closes_the_context_menu() {
    let mut world = World::new();
    let (mut editor, _) = browser_session(&mut world);
    let config = editor.editor.external_tools.clone();
    editor.editor.context_menu.open_for_file(Vec2::new(100.0, 400.0), "sprites/a.png", &config, true);
    editor.stop_confirm.pending = true;
    let dialog_up = editor.dialog_up();

    let mut ui = ui::UIContext::new();
    ui.begin_frame(&input::InputHandler::new(), WINDOW);
    editor.render_context_menu(&mut ui, dialog_up);
    ui.end_frame();

    assert!(!editor.editor.context_menu.is_open(), "the menu shares the dialog's band and closes under it");
}
