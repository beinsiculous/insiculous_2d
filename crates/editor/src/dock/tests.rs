//! Dock layout contracts: edge panels carve the window, the center gets the
//! rest, collapsing and hiding reflow it, and splitter drags stay in range.

use glam::Vec2;
use ui::Rect;

use crate::layout::HEADER_HEIGHT;

use super::render::resized_size;
use super::*;

const DOCK: Rect = Rect::new(0.0, 0.0, 1000.0, 800.0);

/// What a panel renderer may draw into: the panel minus its header strip,
/// and nothing at all while the panel is collapsed.
#[test]
fn test_content_bounds_sit_below_the_header_and_vanish_when_collapsed() {
    let mut panel = DockPanel::new(PanelId::INSPECTOR, "Test", DockPosition::Right);
    panel.bounds = Rect::new(100.0, 50.0, 200.0, 400.0);

    let content = panel.content_bounds();
    assert_eq!(content, Rect::new(100.0, 50.0 + HEADER_HEIGHT, 200.0, 400.0 - HEADER_HEIGHT));

    panel.collapsed = true;
    let collapsed = panel.content_bounds();
    assert_eq!((collapsed.width, collapsed.height), (0.0, 0.0), "a collapsed panel has no content area");
}

/// A dock with a 200px left panel, a 250px right panel and a center view.
fn three_panel_dock() -> DockArea {
    let mut area = DockArea::new();
    area.set_bounds(DOCK);
    area.add_panel(DockPanel::new(PanelId::HIERARCHY, "Hierarchy", DockPosition::Left).with_size(200.0));
    area.add_panel(DockPanel::new(PanelId::INSPECTOR, "Inspector", DockPosition::Right).with_size(250.0));
    area.add_panel(DockPanel::new(PanelId::SCENE_VIEW, "Scene", DockPosition::Center));
    area.layout();
    area
}

fn bounds_of(area: &DockArea, id: PanelId) -> Rect {
    area.get_panel(id).expect("panel exists").bounds
}

/// Edge panels take their size from their own edge, full height; the
/// center is whatever remains between them.
#[test]
fn test_edge_panels_carve_the_dock_and_the_center_gets_the_remainder() {
    let area = three_panel_dock();
    let table = [
        (PanelId::HIERARCHY, Rect::new(0.0, 0.0, 200.0, 800.0)),
        (PanelId::INSPECTOR, Rect::new(750.0, 0.0, 250.0, 800.0)),
        (PanelId::SCENE_VIEW, Rect::new(200.0, 0.0, 550.0, 800.0)),
    ];
    for (id, expected) in table {
        assert_eq!(bounds_of(&area, id), expected, "{id:?}");
    }
}

/// Collapsing an edge panel leaves a header-wide strip and the center
/// reclaims the space; expanding restores the remembered size; hiding a
/// panel hands the center its full width and re-showing relayouts; the
/// center itself never collapses.
#[test]
fn test_dock_area_layout_collapsed_left_is_slim_strip_and_center_reclaims() {
    let mut area = three_panel_dock();

    area.set_panel_collapsed(PanelId::HIERARCHY, true);
    assert_eq!(bounds_of(&area, PanelId::HIERARCHY).width, HEADER_HEIGHT, "a collapsed panel is a strip");
    assert_eq!(bounds_of(&area, PanelId::SCENE_VIEW).x, HEADER_HEIGHT);
    assert_eq!(bounds_of(&area, PanelId::SCENE_VIEW).width, 1000.0 - HEADER_HEIGHT - 250.0);

    area.toggle_panel_collapsed(PanelId::HIERARCHY);
    let hierarchy = area.get_panel(PanelId::HIERARCHY).expect("panel exists");
    assert!(!hierarchy.collapsed);
    assert_eq!(hierarchy.size, 200.0, "expanding restores the remembered size");
    assert_eq!(hierarchy.bounds.width, 200.0);

    area.toggle_panel_visible(PanelId::HIERARCHY);
    area.toggle_panel_visible(PanelId::INSPECTOR);
    assert_eq!(bounds_of(&area, PanelId::SCENE_VIEW), DOCK, "with both edges hidden the center is the dock");

    area.toggle_panel_visible(PanelId::HIERARCHY);
    assert_eq!(bounds_of(&area, PanelId::SCENE_VIEW).x, 200.0, "re-showing relayouts immediately");

    area.set_panel_collapsed(PanelId::SCENE_VIEW, true);
    assert!(!area.get_panel(PanelId::SCENE_VIEW).expect("panel exists").collapsed, "the center never collapses");
}

/// A splitter drag clamps to the panel's min size and to half the dock,
/// and right/bottom panels measure their size from the far edge.
#[test]
fn test_resized_size_clamps_to_min_and_half_dock() {
    let left = Rect::new(0.0, 0.0, 200.0, 800.0);
    let right = Rect::new(750.0, 0.0, 250.0, 800.0);
    let bottom = Rect::new(0.0, 620.0, 1000.0, 180.0);
    let table = [
        ("left below min", DockPosition::Left, Vec2::new(10.0, 400.0), left, 100.0),
        ("left beyond half", DockPosition::Left, Vec2::new(900.0, 400.0), left, 500.0),
        ("left in range", DockPosition::Left, Vec2::new(300.0, 400.0), left, 300.0),
        ("right from far edge", DockPosition::Right, Vec2::new(700.0, 400.0), right, 300.0),
        ("bottom from far edge", DockPosition::Bottom, Vec2::new(500.0, 600.0), bottom, 200.0),
    ];
    for (name, position, mouse, panel, expected) in table {
        assert_eq!(resized_size(position, mouse, panel, 100.0, DOCK), expected, "{name}");
    }
}

/// The View menu's labels and the panel ids they toggle are two tables;
/// a renamed label must fail here, not silently do nothing when clicked.
#[test]
fn test_panel_id_for_menu_label_map() {
    let table = [
        ("Inspector", Some(PanelId::INSPECTOR)),
        ("Hierarchy", Some(PanelId::HIERARCHY)),
        ("Asset Browser", Some(PanelId::ASSET_BROWSER)),
        // The scene view cannot be hidden; Console has no panel yet.
        ("Scene View", None),
        ("Console", None),
        ("Toggle Grid", None),
    ];
    for (label, expected) in table {
        assert_eq!(panel_id_for_menu_label(label), expected, "{label}");
    }
}
