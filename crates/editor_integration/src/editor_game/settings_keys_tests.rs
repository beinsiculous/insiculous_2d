//! The External Tools panel's keyboard through the real `EditorGame`: a press in the panel
//! takes it, its keys decide before the editor's so an arrow never nudges an entity behind
//! it, Enter and Right act on the control at the cursor at the next render, and a press
//! elsewhere, Escape or entering Play lets go.

use ecs::World;
use editor::external_tools::{PresetId, ToolCategory, ToolChoice};
use editor::{EditorAction, Modifiers, PanelId};
use glam::Vec2;
use winit::keyboard::KeyCode;

use super::settings_keys::SettingsKey;
use super::shortcuts::KeyRoute;
use super::test_support::{editor_game, press_mouse, press_play, spawn_at, DummyGame};
use super::EditorGame;

const PLAIN: Modifiers = Modifiers { ctrl: false, shift: false };
const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

/// An editor laid out at the window's size with the panel shown beside the Inspector and
/// one entity selected in the scene.
fn settings_session(world: &mut World) -> (EditorGame<DummyGame>, ecs::EntityId) {
    let mut editor = editor_game();
    editor.editor.dock_area.set_bounds(ui::Rect::new(0.0, 24.0, WINDOW.x, WINDOW.y - 48.0));
    editor.editor.dock_area.show_panel(PanelId::SETTINGS);
    let entity = spawn_at(world, Vec2::ZERO);
    editor.editor.selection.select(entity);
    (editor, entity)
}

/// A frame whose only input is a left press in `panel`, settled as frame start does.
fn press_in(editor: &mut EditorGame<DummyGame>, panel: PanelId) {
    let position = editor.editor.dock_area.get_panel(panel).map(|panel| panel.content_bounds().center());
    let Some(position) = position else {
        panic!("{panel:?} is laid out");
    };
    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    press_mouse(&mut input, position);
    ui.begin_frame(&input, WINDOW);
    editor.settle_conductor(&ui);
    editor.settle_asset_browser(&ui);
    editor.settle_console(&ui);
    editor.settle_settings(&ui);
}

/// Route `key` and run it when it is the panel's, as `handle_editor_key` does.
fn press_key(editor: &mut EditorGame<DummyGame>, key: KeyCode) -> KeyRoute {
    let route = editor.route_editor_key(key, false, PLAIN);
    if let KeyRoute::Settings(settings_key) = route {
        editor.run_settings_key(settings_key);
    }
    route
}

/// One frame of the panel with no input, as the dock draws it.
fn render(editor: &mut EditorGame<DummyGame>) {
    let Some(bounds) = editor.editor.dock_area.get_panel(PanelId::SETTINGS).map(|panel| panel.content_bounds()) else {
        panic!("the panel is laid out");
    };
    let mut ui = ui::UIContext::new();
    ui.begin_frame(&input::InputHandler::new(), WINDOW);
    let context = &mut editor.editor;
    editor::settings_panel::render_settings_panel(&mut ui, &context.theme, &mut context.settings, &mut context.external_tools, bounds, true);
    ui.end_frame();
}

fn position_of(world: &World, entity: ecs::EntityId) -> Option<Vec2> {
    world.get::<common::Transform2D>(entity).map(|transform| transform.position)
}

#[test]
fn while_the_panel_holds_the_keyboard_an_arrow_moves_its_cursor_and_nudges_no_entity_until_a_press_elsewhere() {
    let mut world = World::new();
    let (mut editor, entity) = settings_session(&mut world);
    render(&mut editor);
    press_in(&mut editor, PanelId::SETTINGS);
    assert!(editor.editor.settings.holds_keyboard, "a press in the panel takes the keyboard");

    assert_eq!(press_key(&mut editor, KeyCode::ArrowDown), KeyRoute::Settings(SettingsKey::Step(false)));
    assert_eq!(editor.editor.settings.cursor, 1);
    assert_eq!(position_of(&world, entity), Some(Vec2::ZERO), "no arrow nudged the selected entity");
    assert!(!matches!(press_key(&mut editor, KeyCode::KeyG), KeyRoute::Settings(_)), "a key it does not use goes on");

    press_in(&mut editor, PanelId::HIERARCHY);
    assert!(!editor.editor.settings.holds_keyboard, "a press in the hierarchy lets go");
    assert_eq!(
        press_key(&mut editor, KeyCode::ArrowDown),
        KeyRoute::Editor { action: EditorAction::NudgeDown, shift: false },
        "the arrow nudges again"
    );
}

#[test]
fn escape_and_entering_play_let_the_keyboard_go() {
    let mut world = World::new();
    let (mut editor, _) = settings_session(&mut world);
    press_in(&mut editor, PanelId::SETTINGS);
    assert_eq!(press_key(&mut editor, KeyCode::Escape), KeyRoute::Settings(SettingsKey::Release));
    assert!(!editor.editor.settings.holds_keyboard);

    press_in(&mut editor, PanelId::SETTINGS);
    press_play(&mut editor, &mut world);
    assert!(!editor.editor.settings.holds_keyboard, "a play session's keys are the game's");
}

#[test]
fn down_then_enter_presses_the_code_test_button() {
    let mut world = World::new();
    let (mut editor, _) = settings_session(&mut world);
    render(&mut editor);
    press_in(&mut editor, PanelId::SETTINGS);

    press_key(&mut editor, KeyCode::ArrowDown);
    assert_eq!(press_key(&mut editor, KeyCode::Enter), KeyRoute::Settings(SettingsKey::Activate));
    render(&mut editor);

    assert_eq!(editor.editor.settings.tool_tests, [ToolCategory::Code], "the system default has no field, so Test follows the Tool row");
}

#[test]
fn right_on_the_code_row_cycles_to_the_next_tool() {
    let mut world = World::new();
    let (mut editor, _) = settings_session(&mut world);
    render(&mut editor);
    press_in(&mut editor, PanelId::SETTINGS);

    assert_eq!(press_key(&mut editor, KeyCode::ArrowRight), KeyRoute::Settings(SettingsKey::Cycle(true)));
    render(&mut editor);
    assert_eq!(editor.editor.external_tools.code, ToolChoice::Preset(PresetId::RustRover), "the system default steps to the first preset");

    press_key(&mut editor, KeyCode::ArrowLeft);
    render(&mut editor);
    assert_eq!(editor.editor.external_tools.code, ToolChoice::OsDefault, "and back");
}
