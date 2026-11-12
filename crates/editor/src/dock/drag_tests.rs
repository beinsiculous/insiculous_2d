//! The tab drag: the pure target resolution, then the gesture through `DockArea` with a
//! scripted pointer.

use super::drag::{edge_name, resolve_target, DropFrame, DropTarget, GroupView};
use super::*;
use crate::test_support::{move_mouse, press_mouse, release_mouse};
use crate::{EditorContext, EditorPreferences, EditorTheme, GroupPrefs, MenuItem, DRAG_THRESHOLD};
use glam::Vec2;
use input::InputHandler;
use ui::UIContext;

const FIRST: PanelId = PanelId(21);
const SECOND: PanelId = PanelId(22);
const STRIPPED: PanelId = PanelId(23);
const RIGHT_TAB: PanelId = PanelId(24);
const DRAGGED: PanelId = PanelId(25);

/// A dock of 1000 × 600: a bottom group's strip with two tabs, a collapsed left strip, a
/// right group with one tab, and every edge open but the top.
fn drop_frame() -> DropFrame {
    DropFrame {
        dock_bounds: Rect::new(0.0, 0.0, 1000.0, 600.0),
        groups: vec![
            GroupView {
                bounds: Rect::new(200.0, 400.0, 600.0, 200.0),
                tabs: vec![(FIRST, Rect::new(200.0, 400.0, 80.0, 24.0)), (SECOND, Rect::new(280.0, 400.0, 80.0, 24.0))],
                collapsed_strip: None,
                overlay: false,
            },
            GroupView { bounds: Rect::new(0.0, 0.0, 24.0, 600.0), tabs: Vec::new(), collapsed_strip: Some(STRIPPED), overlay: false },
            GroupView {
                bounds: Rect::new(800.0, 0.0, 200.0, 600.0),
                tabs: vec![(RIGHT_TAB, Rect::new(800.0, 0.0, 90.0, 24.0))],
                collapsed_strip: None,
                overlay: false,
            },
        ],
        open_edges: vec![DockPosition::Left, DockPosition::Right, DockPosition::Bottom],
    }
}

#[test]
fn a_tab_or_a_strips_empty_run_is_a_join_and_the_dragged_panels_own_tab_is_nothing() {
    let frame = drop_frame();
    let at = |x: f32, y: f32, dragged: PanelId| resolve_target(Vec2::new(x, y), dragged, &frame);
    assert_eq!(at(240.0, 410.0, DRAGGED), Some(DropTarget::AfterTab(FIRST)));
    assert_eq!(at(320.0, 410.0, DRAGGED), Some(DropTarget::AfterTab(SECOND)));
    assert_eq!(at(600.0, 410.0, DRAGGED), Some(DropTarget::StripEnd(SECOND)), "the empty run is after the last tab");
    assert_eq!(at(240.0, 410.0, FIRST), None, "on its own tab, nothing");
    assert_eq!(at(600.0, 410.0, SECOND), None, "after itself, nothing");
    assert_eq!(at(600.0, 410.0, FIRST), Some(DropTarget::StripEnd(SECOND)), "within its own group the run reorders");
    assert_eq!(DropTarget::AfterTab(SECOND).action(DRAGGED), DockAction::GroupWith(DRAGGED, SECOND));
}

#[test]
fn a_collapsed_side_strip_joins_its_active_panel_and_wins_over_the_edge_zone_it_sits_in() {
    let frame = drop_frame();
    let target = resolve_target(Vec2::new(10.0, 300.0), DRAGGED, &frame);
    assert_eq!(target, Some(DropTarget::CollapsedStrip(STRIPPED)), "the strip lies inside the left zone and wins");
    assert_eq!(target.map(|target| target.action(DRAGGED)), Some(DockAction::GroupWith(DRAGGED, STRIPPED)));
    let strip_in_corner = resolve_target(Vec2::new(960.0, 10.0), DRAGGED, &frame);
    assert_eq!(strip_in_corner, Some(DropTarget::StripEnd(RIGHT_TAB)), "a strip in the corner zone wins too");
}

#[test]
fn each_edge_zone_makes_a_new_group_a_corner_goes_to_the_side_and_a_refusing_edge_takes_nothing() {
    let frame = drop_frame();
    let at = |x: f32, y: f32| resolve_target(Vec2::new(x, y), DRAGGED, &frame);
    assert_eq!(at(980.0, 300.0), Some(DropTarget::Edge(DockPosition::Right)), "over a group's content near the edge");
    assert_eq!(at(500.0, 590.0), Some(DropTarget::Edge(DockPosition::Bottom)));
    assert_eq!(at(30.0, 300.0), Some(DropTarget::Edge(DockPosition::Left)), "beside the collapsed strip, still in the zone");
    assert_eq!(at(980.0, 590.0), Some(DropTarget::Edge(DockPosition::Right)), "a corner goes to the nearer side edge");
    assert_eq!(at(500.0, 10.0), None, "the top edge refuses: its zone takes no drop");
    assert_eq!(at(500.0, 300.0), None, "nowhere near a strip or an edge");
    assert_eq!(at(500.0, 700.0), None, "outside the dock");
    assert_eq!(DropTarget::Edge(DockPosition::Right).action(DRAGGED), DockAction::MoveTo(DRAGGED, DockPosition::Right));

    let mut side_refuses = drop_frame();
    side_refuses.open_edges.retain(|edge| *edge != DockPosition::Right);
    let corner = resolve_target(Vec2::new(980.0, 590.0), DRAGGED, &side_refuses);
    assert_eq!(corner, Some(DropTarget::Edge(DockPosition::Bottom)), "where the side refuses, the corner is the bottom's");
}

#[test]
fn only_a_panel_alone_in_the_docks_last_group_is_offered_no_drop_on_its_own_edge() {
    let dock = dock();
    let last = dock.groups().last().expect("a docked group");
    let (resting, own_edge) = (last.active, last.position);
    assert!(dock.can_move_panel_to_edge(resting, own_edge), "fixture: the guard alone would offer it");
    let offered = dock.drop_frame(resting).open_edges;
    assert!(!offered.contains(&own_edge), "the move would detach the last group and push it back last");
    assert!(offered.iter().any(|edge| *edge != own_edge), "the other edges are still offered");

    // An earlier group re-pushed last hands its corners to the groups after it: a real move.
    let earlier = &dock.groups()[0];
    assert!(dock.groups().len() > 1 && earlier.panels.len() == 1, "fixture: a lone panel in a group that is not the last");
    assert!(dock.drop_frame(earlier.active).open_edges.contains(&earlier.position), "its own edge reorders the dock, so it is offered");
}

#[test]
fn under_an_open_overlay_only_its_own_strip_is_a_target() {
    let mut frame = drop_frame();
    frame.groups.push(GroupView {
        bounds: Rect::new(150.0, 380.0, 300.0, 220.0),
        tabs: vec![(DRAGGED, Rect::new(150.0, 380.0, 90.0, 24.0))],
        collapsed_strip: None,
        overlay: true,
    });
    assert_eq!(resolve_target(Vec2::new(240.0, 410.0), FIRST, &frame), None, "the bottom strip's tab under the overlay's content is covered");
    assert_eq!(resolve_target(Vec2::new(400.0, 390.0), FIRST, &frame), Some(DropTarget::StripEnd(DRAGGED)));
    assert_eq!(resolve_target(Vec2::new(500.0, 410.0), DRAGGED, &frame), Some(DropTarget::StripEnd(SECOND)), "beside the overlay the strip answers");
}

#[test]
fn an_edge_bands_name_is_the_window_menus_move_to_item() {
    let dock = dock();
    let popup = crate::MenuBar::dock_popup(Vec2::ZERO, PanelId::HIERARCHY, &dock);
    let Some(MenuItem::Submenu { items, .. }) = popup.menus()[0].items.first() else { panic!("the first item is Move to") };
    let labels: Vec<&str> = items.iter().filter_map(|item| match item {
        MenuItem::Action { label, .. } => Some(label.as_str()),
        _ => None,
    }).collect();
    let edges = [DockPosition::Left, DockPosition::Right, DockPosition::Top, DockPosition::Bottom];
    assert_eq!(labels, edges.map(edge_name));
}

fn dock() -> DockArea {
    let mut dock = EditorContext::new().dock_area;
    dock.set_bounds(Rect::new(0.0, 0.0, 1280.0, 800.0));
    dock.layout();
    dock
}

/// A pointer scripted through the ui's own input path, one frame per call, each frame laid out
/// as the host lays it: the drop taken and its action run before the dock draws.
struct Pointer {
    ui: UIContext,
    input: InputHandler,
    theme: EditorTheme,
}

impl Pointer {
    fn new() -> Self {
        Self { ui: UIContext::new(), input: InputHandler::new(), theme: EditorTheme::default() }
    }

    fn frame(&mut self, dock: &mut DockArea) {
        self.ui.begin_frame(&self.input, Vec2::new(1280.0, 800.0));
        match dock.take_tab_drop(&self.ui) {
            Some(DockAction::GroupWith(id, other)) => dock.group_panel_with(id, other),
            Some(DockAction::MoveTo(id, edge)) => dock.move_panel_to_edge(id, edge),
            Some(other) => panic!("a drop is a join or a move, not {other:?}"),
            None => {}
        }
        dock.render(&mut self.ui, &self.theme);
        self.ui.end_frame();
    }

    fn press(&mut self, dock: &mut DockArea, at: Vec2) {
        press_mouse(&mut self.input, at);
        self.frame(dock);
    }

    fn move_to(&mut self, dock: &mut DockArea, at: Vec2) {
        move_mouse(&mut self.input, at);
        self.frame(dock);
    }

    fn release(&mut self, dock: &mut DockArea) {
        release_mouse(&mut self.input);
        self.frame(dock);
    }

    /// Press `id`'s tab and carry it to `to`, past the threshold; the button stays down.
    fn drag(&mut self, dock: &mut DockArea, id: PanelId, to: Vec2) {
        let from = dock.tab_bounds(id).expect("a drawn tab").center();
        self.press(dock, from);
        self.move_to(dock, to);
    }
}

/// Every field the drag must leave alone until its drop, and the saved groups.
#[derive(Debug, PartialEq)]
struct DockLayout {
    groups: Vec<(DockPosition, Vec<PanelId>, PanelId, f32, bool)>,
    visible: Vec<bool>,
    saved: Vec<GroupPrefs>,
}

fn layout_of(dock: &DockArea) -> DockLayout {
    let groups = dock.groups().iter()
        .map(|group| (group.position, group.panels.clone(), group.active, group.size, group.collapsed)).collect();
    let visible = dock.panels().iter().map(|panel| panel.visible).collect();
    let mut saved = EditorPreferences::default();
    saved.capture_panels(dock);
    DockLayout { groups, visible, saved: saved.groups }
}

/// The default dock drawn once, with the bottom group's Console shown behind Assets: an
/// inactive tab, which a stray click would bring forward.
fn drawn_dock(pointer: &mut Pointer) -> DockArea {
    let mut dock = dock();
    dock.show_panel(PanelId::CONSOLE);
    dock.activate(PanelId::ASSET_BROWSER);
    pointer.frame(&mut dock);
    dock
}

fn group_panels(dock: &DockArea, id: PanelId) -> Vec<PanelId> {
    dock.groups()[dock.group_of(id).expect("a member")].panels.clone()
}

#[test]
fn a_press_released_under_the_threshold_is_the_tabs_click_and_moves_nothing() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    let groups_before = dock.groups().iter().map(|group| group.panels.clone()).collect::<Vec<_>>();
    let tab = dock.tab_bounds(PanelId::CONSOLE).unwrap().center();
    pointer.press(&mut dock, tab);
    pointer.move_to(&mut dock, tab + Vec2::new(DRAG_THRESHOLD - 1.0, 0.0));
    assert_eq!(dock.dragged_tab(), None, "under the threshold there is no drag");
    pointer.release(&mut dock);
    assert!(dock.is_active(PanelId::CONSOLE), "the release is today's click, which reveals");
    assert_eq!(dock.groups().iter().map(|group| group.panels.clone()).collect::<Vec<_>>(), groups_before);
}

#[test]
fn a_drag_dropped_on_another_groups_tab_joins_it_after_that_tab() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    let inspector = dock.tab_bounds(PanelId::INSPECTOR).unwrap().center();
    pointer.drag(&mut dock, PanelId::HIERARCHY, inspector);
    assert_eq!(dock.dragged_tab(), Some(PanelId::HIERARCHY));
    pointer.release(&mut dock);
    assert_eq!(group_panels(&dock, PanelId::INSPECTOR), [PanelId::INSPECTOR, PanelId::HIERARCHY, PanelId::SETTINGS]);
    assert!(dock.is_active(PanelId::HIERARCHY), "the dropped panel is shown where it landed");
    assert!(dock.groups().iter().all(|group| group.position != DockPosition::Left), "the emptied group is dropped");
    assert_eq!(dock.dragged_tab(), None);
}

#[test]
fn a_drag_within_one_group_reorders_it() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    assert_eq!(group_panels(&dock, PanelId::CONSOLE), [PanelId::ASSET_BROWSER, PanelId::CONDUCTOR, PanelId::CONSOLE]);
    let console = dock.tab_bounds(PanelId::CONSOLE).unwrap().center();
    pointer.drag(&mut dock, PanelId::ASSET_BROWSER, console);
    pointer.release(&mut dock);
    assert_eq!(group_panels(&dock, PanelId::CONSOLE), [PanelId::CONDUCTOR, PanelId::CONSOLE, PanelId::ASSET_BROWSER]);
    assert!(dock.is_active(PanelId::ASSET_BROWSER));
}

#[test]
fn a_drop_on_a_collapsed_bottom_groups_tab_joins_it_and_makes_no_new_group() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    dock.set_panel_collapsed(PanelId::ASSET_BROWSER, true);
    pointer.frame(&mut dock);
    let groups_before = dock.groups().len();
    let assets = dock.tab_bounds(PanelId::ASSET_BROWSER).unwrap();
    assert!(dock.bounds().bottom() - assets.center().y < drag::EDGE_ZONE_DEPTH, "fixture: the collapsed strip lies in the bottom zone");
    pointer.drag(&mut dock, PanelId::HIERARCHY, assets.center());
    pointer.release(&mut dock);
    assert_eq!(dock.group_of(PanelId::HIERARCHY), dock.group_of(PanelId::ASSET_BROWSER), "the tab wins over the zone it sits in");
    assert_eq!(group_panels(&dock, PanelId::HIERARCHY)[..2], [PanelId::ASSET_BROWSER, PanelId::HIERARCHY]);
    assert_eq!(dock.groups().len(), groups_before - 1, "no new group; only the emptied left one goes");
}

#[test]
fn a_drop_on_an_edge_makes_the_new_group_and_drops_the_emptied_one() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    let inspector_content = dock.get_panel(PanelId::INSPECTOR).unwrap().content_bounds();
    let edge = Vec2::new(dock.bounds().right() - 10.0, inspector_content.center().y);
    assert!(inspector_content.contains(edge), "fixture: the zone lies over the Inspector's content, not a strip");
    pointer.drag(&mut dock, PanelId::HIERARCHY, edge);
    pointer.release(&mut dock);
    let index = dock.group_of(PanelId::HIERARCHY).unwrap();
    assert_eq!(dock.groups()[index].position, DockPosition::Right);
    assert_eq!(dock.groups()[index].panels, [PanelId::HIERARCHY], "a group of its own");
    assert!(dock.groups().iter().all(|group| group.position != DockPosition::Left), "the emptied left group is dropped");
    assert!(dock.get_panel(PanelId::HIERARCHY).unwrap().content_bounds().height > 0.0);
}

#[test]
fn escape_its_own_tab_a_release_over_nothing_and_play_each_leave_the_layout_and_the_saved_groups_identical() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    let before = layout_of(&dock);
    let own_tab = dock.tab_bounds(PanelId::CONSOLE).unwrap().center();
    let inspector = dock.tab_bounds(PanelId::INSPECTOR).unwrap().center();
    let scene = dock.get_panel(PanelId::SCENE_VIEW).unwrap().content_bounds().center();

    // Escape (the host's cancel), then the release back on the panel's own tab.
    pointer.drag(&mut dock, PanelId::CONSOLE, inspector);
    dock.cancel_tab_drag();
    assert_eq!(dock.dragged_tab(), None, "the cancel ends the drag");
    pointer.move_to(&mut dock, own_tab);
    pointer.release(&mut dock);
    assert_eq!(layout_of(&dock), before, "a cancelled drag's release is no click: Console stays behind Assets");

    // Carried away and released back on its own tab.
    pointer.drag(&mut dock, PanelId::CONSOLE, scene);
    pointer.move_to(&mut dock, own_tab);
    pointer.release(&mut dock);
    assert_eq!(layout_of(&dock), before, "a drag released on its own tab reveals nothing");

    // Released over nothing.
    pointer.drag(&mut dock, PanelId::CONSOLE, scene);
    pointer.release(&mut dock);
    assert_eq!(layout_of(&dock), before, "a release over no target moves nothing");

    // Play starting is the same cancel; the release over a target then takes nothing.
    pointer.drag(&mut dock, PanelId::CONSOLE, inspector);
    dock.cancel_tab_drag();
    pointer.release(&mut dock);
    assert_eq!(layout_of(&dock), before, "a drag ended by Play drops nowhere");
}

#[test]
fn no_layout_field_changes_on_any_frame_of_a_drag_before_its_drop() {
    let mut pointer = Pointer::new();
    let mut dock = drawn_dock(&mut pointer);
    let before = layout_of(&dock);
    let inspector = dock.tab_bounds(PanelId::INSPECTOR).unwrap().center();
    let assets = dock.tab_bounds(PanelId::ASSET_BROWSER).unwrap().center();
    let right_edge = Vec2::new(dock.bounds().right() - 10.0, 400.0);
    let bottom_edge = Vec2::new(640.0, dock.bounds().bottom() - 10.0);
    pointer.drag(&mut dock, PanelId::HIERARCHY, inspector);
    assert_eq!(layout_of(&dock), before, "the frame the drag starts");
    for at in [assets, right_edge, bottom_edge, inspector] {
        pointer.move_to(&mut dock, at);
        assert_eq!(layout_of(&dock), before, "a frame over {at:?}");
    }
    pointer.release(&mut dock);
    assert_ne!(layout_of(&dock), before, "the drop is the first frame that moves anything");
}

#[test]
fn no_drag_starts_while_the_narrow_overlay_is_open_and_its_tabs_still_click() {
    let mut pointer = Pointer::new();
    let mut dock = dock();
    dock.set_panel_visible(PanelId::SETTINGS, true);
    dock.activate(PanelId::INSPECTOR);
    dock.set_bounds(Rect::new(0.0, 0.0, 320.0, 600.0));
    dock.layout();
    dock.open_narrow_overlay(PanelId::INSPECTOR);
    pointer.frame(&mut dock);
    let settings = dock.tab_bounds(PanelId::SETTINGS).unwrap();
    pointer.press(&mut dock, settings.center());
    pointer.move_to(&mut dock, Vec2::new(settings.right() - 2.0, settings.center().y));
    assert!(settings.center().distance(Vec2::new(settings.right() - 2.0, settings.center().y)) > DRAG_THRESHOLD, "fixture: past the threshold");
    assert_eq!(dock.dragged_tab(), None, "no drag starts over the overlay");
    pointer.release(&mut dock);
    assert!(dock.is_active(PanelId::SETTINGS), "a release inside the tab is still its click");
}
