//! The press rule every keyboard holder settles by, where a narrow-mode overlay covers a
//! bottom panel: a press on the overlay is the overlay's panel's, never the one beneath
//! it, so it takes no keyboard there and lets a holder beneath it go.

use editor::conductor::ConductorFocus;
use editor::PanelId;
use glam::Vec2;

use super::test_support::{conductor_session, editor_game, melody_song, press_mouse, DummyGame};
use super::EditorGame;

/// Narrow with both side panels shown, and still narrow with the Inspector alone: its
/// default width leaves the centre under the toolbar strip's minimum.
const WINDOW: Vec2 = Vec2::new(
    editor::toolbar_strip::TOOLBAR_STRIP_MIN_WIDTH + editor::layout::DEFAULT_PANEL_WIDTH - 40.0,
    720.0,
);

/// A dock too narrow for its side panels, with `bottom` shown and the Hierarchy open as
/// the narrow-mode overlay over it.
fn under_the_hierarchy_overlay(editor: &mut EditorGame<DummyGame>, bottom: &[PanelId]) {
    editor.editor.dock_area.set_bounds(ui::Rect::new(0.0, 24.0, WINDOW.x, WINDOW.y - 48.0));
    for &panel in bottom {
        editor.editor.dock_area.set_panel_visible(panel, true);
    }
    editor.editor.dock_area.open_narrow_overlay(PanelId::HIERARCHY);
    assert_eq!(editor.editor.dock_area.narrow_overlay(), Some(PanelId::HIERARCHY), "the dock is narrow");
}

/// A point in `panel`'s content on its middle row: on the overlay when `on_overlay`, and
/// clear of it otherwise.
fn point_in(editor: &EditorGame<DummyGame>, panel: PanelId, on_overlay: bool) -> Vec2 {
    let dock = &editor.editor.dock_area;
    let overlay = dock.get_panel(PanelId::HIERARCHY).map(|hierarchy| hierarchy.expanded_content_bounds());
    let content = dock.get_panel(panel).map(|shown| shown.content_bounds());
    let (Some(overlay), Some(content)) = (overlay, content) else {
        panic!("{panel:?} and the overlay are laid out");
    };
    let x = if on_overlay { overlay.x + overlay.width / 2.0 } else { overlay.right() + 40.0 };
    let point = Vec2::new(x, content.center().y);
    assert!(content.contains(point) && overlay.contains(point) == on_overlay, "{panel:?} at {point}");
    point
}

/// A frame whose only input is a left press at `point`, settled as frame start does.
fn press_at(editor: &mut EditorGame<DummyGame>, point: Vec2) {
    let mut input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    press_mouse(&mut input, point);
    ui.begin_frame(&input, WINDOW);
    editor.settle_conductor(&ui);
    editor.settle_asset_browser(&ui);
    editor.settle_console(&ui);
}

#[test]
fn a_press_on_the_overlay_over_the_console_or_the_assets_takes_neither_and_lets_each_go() {
    let mut editor = editor_game();
    under_the_hierarchy_overlay(&mut editor, &[PanelId::ASSET_BROWSER, PanelId::CONSOLE]);

    let console_clear = point_in(&editor, PanelId::CONSOLE, false);
    press_at(&mut editor, console_clear);
    assert!(editor.editor.console.holds_keyboard, "a press in the console's own part takes the keyboard");
    let console_covered = point_in(&editor, PanelId::CONSOLE, true);
    press_at(&mut editor, console_covered);
    assert!(!editor.editor.console.holds_keyboard, "a press on the overlay over it lets it go");

    let assets_clear = point_in(&editor, PanelId::ASSET_BROWSER, false);
    press_at(&mut editor, assets_clear);
    assert!(editor.editor.asset_browser.holds_keyboard);
    let assets_covered = point_in(&editor, PanelId::ASSET_BROWSER, true);
    press_at(&mut editor, assets_covered);
    assert!(!editor.editor.asset_browser.holds_keyboard, "nor does the browser keep it");
    assert!(!editor.editor.console.holds_keyboard);
}

#[test]
fn an_overlay_hidden_while_the_dock_stays_narrow_covers_nothing() {
    let mut editor = editor_game();
    under_the_hierarchy_overlay(&mut editor, &[PanelId::CONSOLE]);
    let formerly_covered = point_in(&editor, PanelId::CONSOLE, true);
    editor.editor.dock_area.set_panel_visible(PanelId::HIERARCHY, false);
    assert!(editor.editor.dock_area.is_narrow(), "the Inspector alone still squeezes the centre");

    press_at(&mut editor, formerly_covered);

    assert!(editor.editor.console.holds_keyboard, "the press is the console's, which nothing covers now");
}

#[test]
fn a_press_on_the_overlay_over_the_conductor_panel_lets_its_keyboard_go() {
    let folder = tempfile::tempdir().expect("an asset folder");
    let mut editor = conductor_session(folder.path(), melody_song());
    under_the_hierarchy_overlay(&mut editor, &[PanelId::CONDUCTOR]);
    assert_eq!(editor.editor.conductor.focus, ConductorFocus::Sidebar);

    let covered = point_in(&editor, PanelId::CONDUCTOR, true);
    press_at(&mut editor, covered);

    assert_eq!(editor.editor.conductor.focus, ConductorFocus::None, "the overlay's press is elsewhere in the dock");
}
