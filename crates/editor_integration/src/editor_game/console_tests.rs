//! The console through the real `EditorGame`: what lands in it — each script error, linked
//! to its `.rhai` file at its line, and each launch result — on the status bar too, and
//! the keyboard it holds on the asset browser's model, so an arrow walks its entries and
//! never nudges an entity behind them.

use std::path::PathBuf;

use ecs::script::{ScriptRef, Scripts};
use ecs::World;
use editor::console::{ConsoleEntry, ConsoleStep, Severity};
use editor::external_tools::{Location, OpenRequest};
use editor::{EditorAction, Modifiers, PanelId, PlayControlAction};
use engine_core::scripting::{ScriptError, ScriptErrorKind, ScriptRunner};
use glam::Vec2;
use winit::keyboard::KeyCode;

use super::console::{script_error_location, ConsoleKey};
use super::shortcuts::KeyRoute;
use super::test_support::{editor_game, press_mouse, press_play, release_mouse, spawn_at, DummyGame};
use super::EditorGame;

const PLAIN: Modifiers = Modifiers { ctrl: false, shift: false };
const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

/// A project whose `scripts/paddle.rhai` fails to compile on line 12.
fn broken_paddle_project() -> tempfile::TempDir {
    let project = tempfile::tempdir().expect("a temporary project");
    let scripts = project.path().join("scripts");
    std::fs::create_dir_all(&scripts).expect("the scripts folder");
    let source = format!("{}fn update(me, view, params, cmd, dt) {{\n    let x = ;\n}}\n", "// a comment\n".repeat(10));
    std::fs::write(scripts.join("paddle.rhai"), source).expect("the broken script");
    project
}

/// One session's frame: the runner reset for the session, stepped once over `world`,
/// and its errors tracked as the editor does each playing frame.
fn run_session(editor: &mut EditorGame<DummyGame>, world: &mut World, project: &std::path::Path) {
    press_play(editor, world);
    let mut runner = ScriptRunner::new();
    runner.reset(world, &project.to_string_lossy());
    let input = input::InputHandler::new();
    let players = input::InputSettings::default_two_player();
    runner.early_update(world, &input, &players, 0.016, None);
    editor.track_script_status(world, &runner);
}

fn script_entity(world: &mut World, script: ScriptRef) {
    let entity = world.create_entity();
    world.add_component(&entity, Scripts(vec![script])).ok();
}

fn rhai(source_path: &str) -> ScriptRef {
    let mut script = ScriptRef::new("paddle");
    script.source_path = source_path.to_string();
    script
}

fn last_entry(editor: &EditorGame<DummyGame>) -> ConsoleEntry {
    editor.editor.console.log.entries().back().cloned().expect("a console entry")
}

#[test]
fn a_syntax_error_lands_linked_at_its_line_on_the_console_and_the_status_bar() {
    let project = broken_paddle_project();
    let mut editor = editor_game();
    let mut world = World::new();
    script_entity(&mut world, rhai("scripts/paddle.rhai"));

    run_session(&mut editor, &mut world, project.path());

    let paddle = OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line: 12, column: None } };
    let entry = last_entry(&editor);
    assert_eq!((entry.severity, entry.location.as_ref()), (Severity::Error, Some(&paddle)), "{}", entry.text);
    assert!(entry.text.starts_with("scripts/paddle.rhai:12: [Syntax]"), "{}", entry.text);
    assert_eq!(editor.editor.status_bar.message(), Some(entry.text.as_str()));
    assert_eq!(editor.editor.status_bar.link(), Some(&paddle), "the bar's error opens it too");
}

#[test]
fn a_line_of_zero_lands_linked_to_the_file_alone_and_an_unresolved_script_unlinked() {
    let project = tempfile::tempdir().expect("a temporary project");
    let mut editor = editor_game();
    let mut world = World::new();
    script_entity(&mut world, rhai("scripts/missing.rhai"));
    run_session(&mut editor, &mut world, project.path());
    assert_eq!(
        last_entry(&editor).location,
        Some(OpenRequest { path: PathBuf::from("scripts/missing.rhai"), location: Location::None }),
        "a file that could not be read has no line"
    );

    let mut editor = editor_game();
    let mut world = World::new();
    script_entity(&mut world, ScriptRef::new("nobody::registered_this"));
    run_session(&mut editor, &mut world, project.path());
    let entry = last_entry(&editor);
    assert!(entry.text.contains("[UnresolvedScript]"), "{}", entry.text);
    assert_eq!(entry.location, None, "a script id is no file");
    assert_eq!(editor.editor.status_bar.link(), None);
}

#[test]
fn the_same_script_error_in_two_sessions_in_a_row_collapses_to_one_entry_counting_two() {
    let project = broken_paddle_project();
    let mut editor = editor_game();
    let mut world = World::new();
    script_entity(&mut world, rhai("scripts/paddle.rhai"));

    run_session(&mut editor, &mut world, project.path());
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    run_session(&mut editor, &mut world, project.path());

    assert_eq!(editor.editor.console.log.len(), 1);
    assert_eq!(last_entry(&editor).count, 2);
}

#[test]
fn a_target_or_a_script_id_named_like_a_script_opens_nothing() {
    let named_like_a_script = |kind: ScriptErrorKind| ScriptError {
        file: "paddle.rhai".to_string(),
        line: 0,
        kind,
        message: "Target entity named 'paddle.rhai' not found".to_string(),
    };
    for kind in [ScriptErrorKind::MissingTarget, ScriptErrorKind::AmbiguousTarget, ScriptErrorKind::UnresolvedScript] {
        assert_eq!(script_error_location(&named_like_a_script(kind.clone())), None, "{kind:?} names no file");
    }
    assert_eq!(
        script_error_location(&named_like_a_script(ScriptErrorKind::Runtime)),
        Some(OpenRequest { path: PathBuf::from("paddle.rhai"), location: Location::None }),
        "a runtime error's file is its script"
    );
}

#[test]
fn a_launch_failure_lands_as_an_unlinked_error() {
    let mut editor = editor_game();
    editor.editor.open_requests.push(editor::open_file_request("notes/plan.unknown_kind"));

    editor.open_requested_files();

    let entry = last_entry(&editor);
    assert_eq!((entry.severity, entry.location), (Severity::Error, None));
    assert!(entry.text.contains("its type is unknown"), "{}", entry.text);
    assert!(editor.editor.status_bar.is_showing_error());
    assert_eq!(editor.editor.status_bar.link(), None);
}

#[test]
fn a_click_on_the_bars_linked_error_queues_its_request_for_the_next_frames_drain() {
    let project = broken_paddle_project();
    let mut editor = editor_game();
    let mut world = World::new();
    script_entity(&mut world, rhai("scripts/paddle.rhai"));
    run_session(&mut editor, &mut world, project.path());
    editor.editor.console.clear();

    let on_message = Vec2::new(12.0, WINDOW.y - editor::status_bar::STATUS_BAR_HEIGHT / 2.0);
    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    press_mouse(&mut input, on_message);
    ui.begin_frame(&input, WINDOW);
    editor.draw_status_bar(&mut ui, WINDOW);
    ui.end_frame();
    release_mouse(&mut input);
    ui.begin_frame(&input, WINDOW);
    editor.draw_status_bar(&mut ui, WINDOW);
    ui.end_frame();

    let paddle = OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line: 12, column: None } };
    assert_eq!(editor.editor.open_requests, vec![paddle]);
}

/// An editor laid out at the window's size with the console and the asset browser shown,
/// two linked console entries, and one entity selected in the scene.
fn console_session(world: &mut World) -> (EditorGame<DummyGame>, ecs::EntityId) {
    let mut editor = editor_game();
    editor.editor.dock_area.set_bounds(ui::Rect::new(0.0, 24.0, WINDOW.x, WINDOW.y - 48.0));
    editor.editor.dock_area.set_panel_visible(PanelId::ASSET_BROWSER, true);
    editor.editor.dock_area.set_panel_visible(PanelId::CONSOLE, true);
    for line in [3, 7] {
        let request = OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line, column: None } };
        editor.report(Severity::Error, format!("scripts/paddle.rhai:{line}: [Runtime] boom"), Some(request));
    }
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
    editor.settle_composer(&ui);
    editor.settle_asset_browser(&ui);
    editor.settle_console(&ui);
}

/// Route `key` and run it when it is the console's, as `handle_editor_key` does.
fn press_key(editor: &mut EditorGame<DummyGame>, key: KeyCode) -> KeyRoute {
    let route = editor.route_editor_key(key, false, PLAIN);
    if let KeyRoute::Console(console_key) = route {
        editor.run_console_key(console_key);
    }
    route
}

fn position_of(world: &World, entity: ecs::EntityId) -> Option<Vec2> {
    world.get::<common::Transform2D>(entity).map(|transform| transform.position)
}

#[test]
fn while_the_console_holds_the_keyboard_down_walks_the_entries_and_nudges_no_entity_until_a_press_elsewhere() {
    let mut world = World::new();
    let (mut editor, entity) = console_session(&mut world);
    press_in(&mut editor, PanelId::CONSOLE);
    assert!(editor.editor.console.holds_keyboard, "a press in the panel takes the keyboard");

    for _ in 0..3 {
        assert_eq!(press_key(&mut editor, KeyCode::ArrowDown), KeyRoute::Console(ConsoleKey::Step(ConsoleStep::Down)));
    }
    assert_eq!(editor.editor.console.focused, Some(1), "the first step focuses the oldest, then down, held at the last");
    assert_eq!(position_of(&world, entity), Some(Vec2::ZERO), "no arrow nudged the selected entity");
    assert!(!matches!(press_key(&mut editor, KeyCode::KeyG), KeyRoute::Console(_)), "a key it does not use goes on");

    press_in(&mut editor, PanelId::HIERARCHY);
    assert!(!editor.editor.console.holds_keyboard, "a press in the hierarchy lets go");
    assert_eq!(
        press_key(&mut editor, KeyCode::ArrowDown),
        KeyRoute::Editor { action: EditorAction::NudgeDown, shift: false },
        "the arrow nudges again"
    );
}

#[test]
fn enter_files_the_focused_entrys_request_and_escape_lets_go() {
    let mut world = World::new();
    let (mut editor, _) = console_session(&mut world);
    press_in(&mut editor, PanelId::CONSOLE);
    press_key(&mut editor, KeyCode::ArrowUp);

    assert_eq!(press_key(&mut editor, KeyCode::Enter), KeyRoute::Console(ConsoleKey::Open));
    assert_eq!(
        editor.editor.open_requests,
        vec![OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line: 7, column: None } }],
        "Up first focuses the newest entry"
    );
    press_key(&mut editor, KeyCode::Escape);
    assert!(!editor.editor.console.holds_keyboard);
}

#[test]
fn entering_play_lets_the_console_go_and_no_press_takes_it_during_the_session() {
    let mut world = World::new();
    let (mut editor, _) = console_session(&mut world);
    press_in(&mut editor, PanelId::CONSOLE);

    press_play(&mut editor, &mut world);

    assert!(!editor.editor.console.holds_keyboard);
    press_in(&mut editor, PanelId::CONSOLE);
    assert!(!editor.editor.console.holds_keyboard, "it never holds while a session runs");
}

#[test]
fn a_press_in_the_console_takes_the_keyboard_from_the_browser_and_a_press_in_the_browser_takes_it_back() {
    let mut world = World::new();
    let (mut editor, _) = console_session(&mut world);
    press_in(&mut editor, PanelId::ASSET_BROWSER);
    assert!(editor.editor.asset_browser.holds_keyboard);

    press_in(&mut editor, PanelId::CONSOLE);
    assert!(editor.editor.console.holds_keyboard && !editor.editor.asset_browser.holds_keyboard);

    press_in(&mut editor, PanelId::ASSET_BROWSER);
    assert!(editor.editor.asset_browser.holds_keyboard && !editor.editor.console.holds_keyboard, "the two never hold at once");
}
