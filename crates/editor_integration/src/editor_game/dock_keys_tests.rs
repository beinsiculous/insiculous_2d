//! Tab-strip keys through the host, document focus and the budget overlay's live controls.

use editor::{PanelId, Modifiers, EditorAction, DockPosition};
use editor::conductor::ConductorFocus;
use engine_core::{GameConfig, test_support::GameHarness};
use glam::Vec2;
use input::{InputEvent, prelude::MouseButton};
use winit::keyboard::KeyCode;
use super::{shortcuts::KeyRoute, test_support::{editor_game, dirty_editor, melody_song, DummyGame}, EditorGame};

const PLAIN: Modifiers = Modifiers { ctrl: false, shift: false };
const STEP: f32 = 1.0 / 60.0;

fn dock_key(editor: &mut EditorGame<DummyGame>, key: KeyCode) {
    let route = editor.route_editor_key(key, false, PLAIN);
    match route {
        KeyRoute::DockTabs(key) => editor.run_dock_key(key),
        KeyRoute::AssetBrowser(key) => editor.run_asset_browser_key(key),
        other => panic!("{key:?} left the tab route: {other:?}"),
    }
}

#[test]
fn f6_walks_inspector_to_bottom_and_back_without_a_pointer_and_escape_leaves_the_menu_then_strip() {
    let mut editor = editor_game();
    editor.editor.update_layout(Vec2::new(1280.0, 800.0));
    dock_key(&mut editor, KeyCode::F6);
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::HIERARCHY));
    dock_key(&mut editor, KeyCode::F6);
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::INSPECTOR));
    dock_key(&mut editor, KeyCode::F6);
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::ASSET_BROWSER));
    dock_key(&mut editor, KeyCode::F6);
    dock_key(&mut editor, KeyCode::F6);
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::INSPECTOR));
    dock_key(&mut editor, KeyCode::ContextMenu);
    assert!(editor.editor.context_menu.is_open());
    dock_key(&mut editor, KeyCode::Escape);
    assert!(!editor.editor.context_menu.is_open());
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::INSPECTOR));
    dock_key(&mut editor, KeyCode::Escape);
    assert_eq!(editor.editor.dock_area.focused_tab(), None);
}

#[test]
fn f6_to_the_bottom_right_to_assets_enter_then_undo_changes_the_scene_and_not_the_song() {
    let mut world = ecs::World::new();
    let mut editor = dirty_editor(&mut world);
    editor.song = Some(melody_song());
    editor.follow_song();
    editor.editor.update_layout(Vec2::new(1280.0, 800.0));
    editor.show_conductor(ConductorFocus::Sidebar);
    let song_entries = editor.song.as_ref().unwrap().history().undo_count();
    assert!(song_entries > 0);
    for _ in 0..3 { dock_key(&mut editor, KeyCode::F6); }
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::CONDUCTOR));
    dock_key(&mut editor, KeyCode::ArrowRight);
    assert_eq!(editor.editor.dock_area.focused_tab(), Some(PanelId::ASSET_BROWSER));
    dock_key(&mut editor, KeyCode::Enter);
    assert_eq!(editor.route_editor_key(KeyCode::KeyZ, false, Modifiers { ctrl: true, shift: false }),
        KeyRoute::Editor { action: EditorAction::Undo, shift: false });
    editor.undo_with_feedback(&mut world);
    assert!(!editor.command_history.can_undo());
    assert_eq!(editor.song.as_ref().unwrap().history().undo_count(), song_entries);
    assert_eq!(editor.editor.conductor.focus, ConductorFocus::None);
}

#[test]
fn switching_tabs_without_a_press_releases_every_inactive_keyboard_holder() {
    let mut editor = editor_game();
    editor.editor.update_layout(Vec2::new(1280.0, 800.0));
    editor.editor.dock_area.show_panel(PanelId::SETTINGS);
    editor.editor.settings.holds_keyboard = true;
    editor.editor.dock_area.activate(PanelId::INSPECTOR);
    editor.editor.dock_area.show_panel(PanelId::CONSOLE);
    editor.editor.console.holds_keyboard = true;
    editor.editor.asset_browser.holds_keyboard = true;
    editor.editor.dock_area.activate(PanelId::CONDUCTOR);
    let mut ui = ui::UIContext::new();
    ui.begin_frame(&input::InputHandler::new(), Vec2::new(1280.0, 800.0));
    editor.settle_asset_browser(&ui);
    editor.settle_console(&ui);
    editor.settle_settings(&ui);
    assert!(!editor.editor.asset_browser.holds_keyboard && !editor.editor.console.holds_keyboard && !editor.editor.settings.holds_keyboard);
}

#[test]
fn a_budget_collapsed_conductor_opens_from_its_chevron_and_its_track_selector_still_works() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    let game = harness.game_mut();
    game.ui_scale = editor::editor_preferences::UiScale::Percent(200);
    game.song = Some(super::test_support::quick_melody_and_bass_song());
    game.follow_song();
    game.editor.dock_area.show_panel(PanelId::CONDUCTOR);
    game.editor.dock_area.move_panel_to_edge(PanelId::HIERARCHY, DockPosition::Bottom);
    game.editor.dock_area.move_panel_to_edge(PanelId::INSPECTOR, DockPosition::Bottom);
    harness.step(STEP, &[]);
    let dock = &harness.game().editor.dock_area;
    assert_eq!(dock.get_panel(PanelId::CONDUCTOR).unwrap().content_bounds(), ui::Rect::default());
    let group = &dock.groups()[dock.group_of(PanelId::CONDUCTOR).unwrap()];
    let bounds = group.bounds();
    let chevron = Vec2::new(bounds.right() - 12.0, bounds.y + 12.0) * 2.0;
    harness.step(STEP, &[InputEvent::MouseMoved(chevron.x, chevron.y), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(harness.game().editor.dock_area.narrow_overlay(), Some(PanelId::CONDUCTOR));
    let content = harness.game().editor.dock_area.get_panel(PanelId::CONDUCTOR).unwrap().content_bounds();
    let transport = editor::conductor::layout::compute(content).transport;
    let track = Vec2::new(transport.x + 80.0, transport.center().y) * 2.0;
    let before = harness.game().editor.conductor.active_track;
    harness.step(STEP, &[InputEvent::MouseMoved(track.x, track.y), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(harness.game().editor.conductor.focus, ConductorFocus::Transport);
    assert_eq!(before, Some(1));
    assert_eq!(harness.game().editor.conductor.active_track, Some(2), "the track selector changes from melody to bass through the overlay");
    assert!(harness.game().editor.dock_area.groups().iter().all(|group| !group.collapsed), "effective collapse never rewrites preferences");
}

#[test]
fn reset_play_and_a_pointer_press_release_the_strip_and_dialogs_close_menus() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    harness.step(STEP, &[InputEvent::MouseMoved(40.0, 12.0), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert!(harness.game().editor.menu_bar.is_open());
    let game = harness.game_mut();
    game.editor.toolbar.toggle_overflow();
    game.editor.context_menu.open_for_tab(Vec2::new(100.0, 100.0), PanelId::HIERARCHY, &game.editor.dock_area);
    game.stop_confirm.pending = true;
    harness.step(STEP, &[]);
    assert!(!harness.game().editor.menu_bar.is_open());
    assert!(!harness.game().editor.toolbar.is_overflow_open());
    assert!(!harness.game().editor.context_menu.is_open());
    harness.game_mut().stop_confirm.pending = false;
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::F6)]);
    assert!(harness.game().editor.dock_area.focused_tab().is_some());
    harness.step(STEP, &[InputEvent::KeyReleased(KeyCode::F6), InputEvent::MouseMoved(500.0, 200.0), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    assert_eq!(harness.game().editor.dock_area.focused_tab(), None);
    harness.game_mut().editor.dock_area.cycle_tab_focus();
    harness.game_mut().editor.reset_layout();
    assert_eq!(harness.game().editor.dock_area.focused_tab(), None);
    harness.game_mut().editor.dock_area.cycle_tab_focus();
    harness.context(|game, context| super::test_support::press_play(game, context.world));
    assert_eq!(harness.game().editor.dock_area.focused_tab(), None);
}

#[test]
fn enter_and_the_menu_key_reach_the_game_while_no_strip_is_focused() {
    let mut editor = editor_game();
    editor.editor.update_layout(Vec2::new(1280.0, 800.0));
    for key in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::ContextMenu] {
        assert_eq!(editor.route_editor_key(key, false, PLAIN), KeyRoute::ForwardToGame, "{key:?} with no strip focused");
    }
    dock_key(&mut editor, KeyCode::F6);
    assert!(matches!(editor.route_editor_key(KeyCode::Enter, false, PLAIN), KeyRoute::DockTabs(_)));
}

#[test]
fn a_drag_resting_on_the_conductor_tab_brings_it_forward_with_the_payload_still_in_hand() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    let game = harness.game_mut();
    game.song = Some(melody_song());
    game.follow_song();
    game.editor.dock_area.show_panel(PanelId::CONDUCTOR);
    game.editor.dock_area.activate(PanelId::ASSET_BROWSER);
    let origin = game.editor.dock_area.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds().center();
    game.editor.drag_drop.arm(editor::DragPayload::Sound { path: "sounds/kick.wav".to_string() }, origin);
    let tab = game.editor.dock_area.tab_bounds(PanelId::CONDUCTOR).unwrap().center();
    harness.step(STEP, &[InputEvent::MouseMoved(origin.x, origin.y), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseMoved(tab.x, tab.y)]);
    assert!(harness.game().editor.dock_area.is_active(PanelId::ASSET_BROWSER), "a drag grazing the tab switches nothing");
    for _ in 1..editor::DRAG_DWELL_FRAMES { harness.step(STEP, &[]); }
    let dock = &harness.game().editor.dock_area;
    assert!(dock.is_active(PanelId::CONDUCTOR), "the tab the drag rests on comes forward");
    assert!(dock.get_panel(PanelId::CONDUCTOR).unwrap().content_bounds().height > 0.0, "with content to drop on");
    assert!(harness.game().editor.drag_drop.dragging_payload().is_some(), "and the payload rides on");

    // A collapsed group's tabs are inert under a drag: expanding it would be saved as the
    // user's layout, and the payload may not even be droppable there.
    harness.game_mut().editor.dock_area.set_panel_collapsed(PanelId::CONDUCTOR, true);
    let assets_tab = harness.game().editor.dock_area.tab_bounds(PanelId::ASSET_BROWSER).unwrap().center();
    harness.step(STEP, &[InputEvent::MouseMoved(assets_tab.x, assets_tab.y)]);
    let dock = &harness.game().editor.dock_area;
    let bottom = &dock.groups()[dock.group_of(PanelId::CONDUCTOR).unwrap()];
    assert!(bottom.collapsed, "a drag over a collapsed strip never expands it");
    assert_eq!(bottom.active, PanelId::CONDUCTOR, "nor switches its tab");
    let mut saved = editor::EditorPreferences::default();
    saved.capture_panels(dock);
    assert!(saved.groups.iter().any(|group| group.panels.contains(&PanelId::CONDUCTOR.0) && group.collapsed), "and the saved layout keeps the collapse");
}

#[test]
fn view_menu_brings_a_visible_back_tab_forward_and_hides_it_only_from_the_front() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    harness.game_mut().editor.dock_area.show_panel(PanelId::SETTINGS);
    assert!(!harness.game().editor.dock_area.is_active(PanelId::INSPECTOR), "fixture: the Inspector is visible behind External Tools");
    harness.context(|game, context| game.dispatch_editor_action(EditorAction::TogglePanel(PanelId::INSPECTOR), false, context));
    assert!(harness.game().editor.dock_area.is_active(PanelId::INSPECTOR), "View → Inspector brings it forward");
    harness.context(|game, context| game.dispatch_editor_action(EditorAction::TogglePanel(PanelId::INSPECTOR), false, context));
    assert!(!harness.game().editor.dock_area.get_panel(PanelId::INSPECTOR).unwrap().visible, "and from the front it hides");
}

/// The dock's groups as a drag must leave them until its drop.
fn groups_of(game: &EditorGame<DummyGame>) -> Vec<(DockPosition, Vec<PanelId>, PanelId, bool)> {
    game.editor.dock_area.groups().iter()
        .map(|group| (group.position, group.panels.clone(), group.active, group.collapsed)).collect()
}

fn tab_center(harness: &GameHarness<EditorGame<DummyGame>>, id: PanelId) -> Vec2 {
    harness.game().editor.dock_area.tab_bounds(id).expect("a drawn tab").center()
}

/// Press `id`'s tab and carry it to `to`; the button stays down.
fn drag_tab(harness: &mut GameHarness<EditorGame<DummyGame>>, id: PanelId, to: Vec2) {
    let from = tab_center(harness, id);
    harness.step(STEP, &[InputEvent::MouseMoved(from.x, from.y), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::MouseMoved(to.x, to.y)]);
    assert_eq!(harness.game().editor.dock_area.dragged_tab(), Some(id), "fixture: the drag is live");
}

fn named_entity(harness: &mut GameHarness<EditorGame<DummyGame>>, name: &str) -> ecs::EntityId {
    harness.context(|_, context| {
        let entity = context.world.create_entity();
        context.world.add_component(&entity, ecs::Name::new(name)).ok();
        entity
    })
}

#[test]
fn escape_cancels_a_tab_drag_begun_while_an_inspector_text_field_held_focus() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    let hero = named_entity(&mut harness, "Hero");
    harness.game_mut().editor.selection.select(hero);
    harness.step(STEP, &[]);
    let name_field = editor::FieldId::new(0, 0, 0);
    harness.context(|_, context| context.ui.focus_text_input(name_field, "Hero"));
    harness.step(STEP, &[]);
    assert!(harness.context(|_, context| context.ui.is_focused(name_field)), "fixture: the inspector's Name field holds the keyboard");
    // A field its panel stops drawing keeps the keyboard until it is drawn again, so a press
    // elsewhere — the tab's — never reaches it to commit and let go.
    harness.game_mut().editor.selection.clear();
    harness.step(STEP, &[]);
    assert!(harness.context(|_, context| context.ui.is_focused(name_field)), "fixture: the undrawn field still holds it");
    let before = groups_of(harness.game());
    let inspector_tab = tab_center(&harness, PanelId::INSPECTOR);
    drag_tab(&mut harness, PanelId::HIERARCHY, inspector_tab);
    assert!(!harness.context(|_, context| context.ui.wants_keyboard()), "past the threshold no field keeps the keyboard");
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Escape)]);
    assert_eq!(harness.game().editor.dock_area.dragged_tab(), None, "Escape reached the drag");
    harness.step(STEP, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(groups_of(harness.game()), before, "the release over the Inspector's tab moved nothing");
}

#[test]
fn escape_in_the_same_step_as_the_threshold_crossing_move_ends_the_press_and_keeps_the_selection() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    let hero = named_entity(&mut harness, "Hero");
    harness.game_mut().editor.selection.select(hero);
    harness.step(STEP, &[]);
    let before = groups_of(harness.game());
    let from = tab_center(&harness, PanelId::HIERARCHY);
    let to = tab_center(&harness, PanelId::INSPECTOR);
    harness.step(STEP, &[InputEvent::MouseMoved(from.x, from.y), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    // The key handler runs before the frame that would promote the press to a drag.
    harness.step(STEP, &[InputEvent::MouseMoved(to.x, to.y), InputEvent::KeyPressed(KeyCode::Escape)]);
    assert!(harness.game().editor.selection.contains(hero), "Escape was the press's, not the selection's");
    assert_eq!(harness.game().editor.dock_area.dragged_tab(), None, "the press never became a drag");
    harness.step(STEP, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(groups_of(harness.game()), before, "the release over the Inspector's tab moved nothing");

    // Escape on a press that never moved: the release on that same tab is not its click.
    let dock = &mut harness.game_mut().editor.dock_area;
    dock.show_panel(PanelId::CONDUCTOR);
    dock.activate(PanelId::ASSET_BROWSER);
    harness.step(STEP, &[InputEvent::MouseMoved(1.0, 1.0)]);
    assert!(!harness.game().editor.dock_area.is_active(PanelId::CONDUCTOR), "fixture: the Conductor is a back tab");
    let conductor = tab_center(&harness, PanelId::CONDUCTOR);
    harness.step(STEP, &[InputEvent::MouseMoved(conductor.x, conductor.y), InputEvent::MouseButtonPressed(MouseButton::Left)]);
    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::Escape)]);
    harness.step(STEP, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert!(!harness.game().editor.dock_area.is_active(PanelId::CONDUCTOR), "the cancelled press did not bring the tab forward");
}

#[test]
fn the_frame_of_a_cross_group_drop_draws_the_dragged_panels_content_in_its_new_groups_rectangle() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    named_entity(&mut harness, "Hero");
    harness.step(STEP, &[]);
    let hero_row = |harness: &GameHarness<EditorGame<DummyGame>>| harness.ui_commands().iter().find_map(|command| match command {
        ui::DrawCommand::Text { data, .. } if data.text.contains("Hero") => Some(data.position),
        ui::DrawCommand::TextPlaceholder { text, position, .. } if text.contains("Hero") => Some(*position),
        _ => None,
    });
    let dock = &harness.game().editor.dock_area;
    let left = dock.groups()[dock.group_of(PanelId::HIERARCHY).unwrap()].bounds();
    let right = dock.groups()[dock.group_of(PanelId::INSPECTOR).unwrap()].bounds();
    assert!(hero_row(&harness).is_some_and(|row| left.contains(row)), "fixture: the hierarchy draws its row in the left group");
    let inspector_tab = tab_center(&harness, PanelId::INSPECTOR);
    drag_tab(&mut harness, PanelId::HIERARCHY, inspector_tab);
    harness.step(STEP, &[InputEvent::MouseButtonReleased(MouseButton::Left)]);
    let row = hero_row(&harness).expect("the release frame draws the hierarchy");
    assert!(right.contains(row), "the release frame draws the row in the right group, not the old left one: {row:?}");
    assert_eq!(harness.game().editor.dock_area.group_of(PanelId::HIERARCHY), harness.game().editor.dock_area.group_of(PanelId::INSPECTOR));
}

#[test]
fn f6_an_arrow_and_enter_during_a_tab_drag_change_nothing_and_play_starting_ends_it() {
    let mut harness = GameHarness::new(editor_game(), GameConfig { width: 1280, height: 800, ..Default::default() });
    harness.step(STEP, &[]);
    let before = groups_of(harness.game());
    let scene = harness.game().editor.dock_area.get_panel(PanelId::SCENE_VIEW).unwrap().content_bounds().center();
    drag_tab(&mut harness, PanelId::ASSET_BROWSER, scene);
    for key in [KeyCode::F6, KeyCode::ArrowRight, KeyCode::Enter] {
        assert_eq!(harness.game_mut().route_editor_key(key, false, PLAIN), KeyRoute::Consumed, "{key:?} under a drag");
        harness.step(STEP, &[InputEvent::KeyPressed(key)]);
        harness.step(STEP, &[InputEvent::KeyReleased(key)]);
    }
    let dock = &harness.game().editor.dock_area;
    assert_eq!(dock.focused_tab(), None, "F6 focused no strip");
    assert_eq!(dock.dragged_tab(), Some(PanelId::ASSET_BROWSER), "and the drag is still live");
    assert_eq!(groups_of(harness.game()), before);

    harness.step(STEP, &[InputEvent::KeyPressed(KeyCode::F5)]);
    harness.step(STEP, &[InputEvent::KeyReleased(KeyCode::F5)]);
    harness.step(STEP, &[]);
    assert!(harness.game().editor.in_play_session(), "fixture: F5 started Play");
    assert_eq!(harness.game().editor.dock_area.dragged_tab(), None, "Play's start ended the drag");
    let inspector_tab = tab_center(&harness, PanelId::INSPECTOR);
    harness.step(STEP, &[InputEvent::MouseMoved(inspector_tab.x, inspector_tab.y), InputEvent::MouseButtonReleased(MouseButton::Left)]);
    assert_eq!(groups_of(harness.game()), before, "its release over a tab moved nothing");
}
