//! Dock layout contracts: edge panels carve the window, the center gets the
//! rest, collapsing and hiding reflow it, and splitter drags stay in range.

use glam::Vec2;
use ui::{FloatFieldOpts, Rect, UIContext};

use crate::layout::HEADER_HEIGHT;
use crate::test_support::{move_to, press_at, release};
use crate::theme::EditorTheme;

use super::render::{chevron_bounds, resized_size};
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

/// The editor's real default side panels at a page width: the hierarchy 200
/// wide, the inspector 280, a centre between them. `width` is the dock's,
/// which at these page widths is the window's.
fn phone_dock(width: f32) -> DockArea {
    let mut area = DockArea::new();
    area.set_bounds(Rect::new(0.0, 0.0, width, 560.0));
    area.add_panel(
        DockPanel::new(PanelId::HIERARCHY, "Hierarchy", DockPosition::Left)
            .with_size(200.0)
            .with_min_size(150.0),
    );
    area.add_panel(
        DockPanel::new(PanelId::INSPECTOR, "Inspector", DockPosition::Right)
            .with_size(280.0)
            .with_min_size(200.0),
    );
    area.add_panel(DockPanel::new(PanelId::SCENE_VIEW, "Scene", DockPosition::Center));
    area.layout();
    area
}

/// At the width a 390px page produces, both default side panels docked would
/// leave the centre nothing at all — 480px of panels in 390px of window. The
/// dock goes narrow instead: the centre takes the whole width and keeps at
/// least the strip's minimum, and the side panels become tabs at the edges.
#[test]
fn test_a_390px_page_gives_the_centre_the_whole_width_instead_of_nothing() {
    let area = phone_dock(390.0);

    assert!(area.is_narrow(), "480px of side panels in 390px must trigger narrow mode");
    let centre = bounds_of(&area, PanelId::SCENE_VIEW);
    assert_eq!(centre.width, 390.0, "the centre takes the width the panels left");
    assert!(
        centre.width >= MIN_CENTER_WIDTH,
        "the centre must hold the toolbar strip ({} < {MIN_CENTER_WIDTH})",
        centre.width
    );
    assert_eq!(area.narrow_overlay(), None, "narrow mode opens with both panels closed");

    for id in [PanelId::HIERARCHY, PanelId::INSPECTOR] {
        let tab = bounds_of(&area, id);
        assert_eq!(tab.width, HEADER_HEIGHT, "{id:?} shows as an edge tab, not a panel");
        assert!(
            tab.y >= strip_bottom_of(&area),
            "{id:?}'s tab must sit below the strip so the play controls stay clickable"
        );
    }
    assert_eq!(bounds_of(&area, PanelId::HIERARCHY).x, 0.0, "the hierarchy tab is at the left");
    assert_eq!(
        bounds_of(&area, PanelId::INSPECTOR).right(),
        390.0,
        "the inspector tab is at the right"
    );
}

/// A comfortable width keeps the ordinary edge allocation — narrow mode is
/// what a squeezed centre triggers, not a width the editor is stuck in.
#[test]
fn test_a_comfortable_width_keeps_both_panels_docked_at_the_edges() {
    let area = phone_dock(1280.0);

    assert!(!area.is_narrow());
    assert_eq!(bounds_of(&area, PanelId::HIERARCHY).width, 200.0);
    assert_eq!(bounds_of(&area, PanelId::INSPECTOR).width, 280.0);
    assert_eq!(bounds_of(&area, PanelId::SCENE_VIEW).width, 1280.0 - 480.0);
}

/// Opening the inspector from its tab, editing a field in it, and then
/// reaching the hierarchy — the acceptance path, at both phone widths.
///
/// The field is live because the panel's content renders inside the
/// overlay's own scope; a field at the same place in the viewport underneath
/// is inert, which is what stops a click reaching the scene through the
/// panel. Opening the hierarchy closes the inspector: two overlays at once
/// would leave no viewport between them.
#[test]
fn test_the_inspector_opens_edits_and_hands_over_to_the_hierarchy_at_390_and_320() {
    for width in [390.0_f32, 320.0] {
        let mut area = phone_dock(width);
        let theme = EditorTheme::default();
        let mut ui = UIContext::new();
        let mut input = input::InputHandler::new();

        let inspector_tab = bounds_of(&area, PanelId::INSPECTOR).center();
        press_at(&mut ui, &mut input, inspector_tab, |ui| area.render(ui, &theme));
        release(&mut ui, &mut input, |ui| area.render(ui, &theme));
        assert_eq!(
            area.narrow_overlay(),
            Some(PanelId::INSPECTOR),
            "{width}px: the tab must open the inspector"
        );
        assert!(
            bounds_of(&area, PanelId::INSPECTOR).y >= strip_bottom_of(&area),
            "{width}px: the overlay must start below the strip, not over the play controls"
        );

        // A field inside the overlay's scope and, at the same rect, a widget
        // rendered outside it standing in for the scene underneath. The
        // gesture lands on both, so only the overlay can separate them.
        let content = content_of(&area, PanelId::INSPECTOR);
        let field = Rect::new(content.x + 8.0, content.y + 8.0, 80.0, 20.0);
        let under = field;
        let scrub_from = field.center();

        let edited = scrub_a_field_inside_the_overlay(&mut area, &theme, &mut ui, &mut input, field, under, scrub_from);
        assert!(edited.0, "{width}px: the inspector's field must be editable through the overlay");
        assert!(!edited.1, "{width}px: a widget under the overlay must stay inert");
        assert!(
            edited.2,
            "{width}px: the overlay must swallow input at the pointer so picking never sees it"
        );

        let hierarchy_tab = bounds_of(&area, PanelId::HIERARCHY).center();
        press_at(&mut ui, &mut input, hierarchy_tab, |ui| area.render(ui, &theme));
        release(&mut ui, &mut input, |ui| area.render(ui, &theme));
        assert_eq!(
            area.narrow_overlay(),
            Some(PanelId::HIERARCHY),
            "{width}px: the hierarchy must be reachable with the inspector open"
        );
    }
}

/// Drag-scrub a float field inside the narrow overlay's scope, and click a
/// widget at the same rect rendered outside it. Returns whether the field
/// changed, whether the widget underneath took the gesture, and whether the
/// overlay swallowed input at the pointer.
fn scrub_a_field_inside_the_overlay(
    area: &mut DockArea,
    theme: &EditorTheme,
    ui: &mut UIContext,
    input: &mut input::InputHandler,
    inside: Rect,
    underneath: Rect,
    scrub_from: Vec2,
) -> (bool, bool, bool) {
    let mut changed_inside = false;
    let mut clicked_underneath = false;
    let mut blocked_at_pointer = false;
    let mut render = |ui: &mut UIContext, area: &mut DockArea| {
        let overlay = area.narrow_overlay().map(|id| bounds_of(area, id));
        area.render(ui, theme);
        if let Some(overlay) = overlay {
            ui.begin_overlay_in(ui::UiLayer::Floating, overlay);
            changed_inside |= ui.float_input("inside", 1.0, FloatFieldOpts::range(-100.0, 100.0), inside).changed;
            ui.end_overlay();
        }
        // The scene underneath is reached after the panels, the order the
        // editor's own frame uses for picking and the gizmo.
        clicked_underneath |= ui.button("under", "Scene", underneath);
        blocked_at_pointer |= ui.is_input_blocked_at(scrub_from);
    };

    press_at(ui, input, scrub_from, |ui| render(ui, area));
    move_to(ui, input, scrub_from + Vec2::new(12.0, 0.0), |ui| render(ui, area));
    release(ui, input, |ui| render(ui, area));
    (changed_inside, clicked_underneath, blocked_at_pointer)
}

/// A panel's content area after the current layout.
fn content_of(area: &DockArea, id: PanelId) -> Rect {
    area.get_panel(id).expect("panel in the fixture").content_bounds()
}

/// Where the scene panel's toolbar strip ends, measured the way the strip is
/// laid out: the panel has a header like every other, and the strip is the
/// top of the content below it. Measuring from the dock's own top passed
/// while the tabs and the overlay sat over the play controls.
fn strip_bottom_of(area: &DockArea) -> f32 {
    crate::toolbar_strip::split(content_of(area, PanelId::SCENE_VIEW)).0.bottom()
}

/// A panel collapsed at desktop width and opened as the narrow overlay must
/// have content to render into, and must keep its collapse flag: the flag is
/// the persisted desktop layout, autosaved on change, and a narrow-mode visit
/// must not rewrite it.
#[test]
fn test_a_collapsed_panel_opens_as_a_full_overlay_without_losing_its_collapse() {
    let mut area = phone_dock(1280.0);
    area.set_panel_collapsed(PanelId::INSPECTOR, true);
    area.set_bounds(Rect::new(0.0, 0.0, 390.0, 560.0));
    area.layout();
    assert!(area.is_narrow(), "200px of hierarchy plus a collapsed inspector still squeeze 390px");
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let input = input::InputHandler::new();

    area.open_narrow_overlay(PanelId::INSPECTOR);
    ui.begin_frame(&input, Vec2::new(390.0, 560.0));
    let content_areas = area.render(&mut ui, &theme);
    ui.end_frame();

    let (_, content) = content_areas
        .iter()
        .find(|(id, _)| *id == PanelId::INSPECTOR)
        .copied()
        .expect("the overlay renders its content");
    assert!(
        content.width > 0.0 && content.height > 0.0,
        "the overlay must have a content rect, got {content:?}"
    );
    assert!(
        area.get_panel(PanelId::INSPECTOR).expect("panel in the fixture").collapsed,
        "the desktop collapse preference survives the visit"
    );
}

/// A short window with the assets panel up leaves the centre a few rows
/// tall. The overlay runs from the strip's bottom to the dock's bottom, over
/// the assets panel, so a field is still editable there.
#[test]
fn test_the_narrow_overlay_runs_to_the_docks_bottom_over_a_bottom_panel() {
    let mut area = phone_dock(390.0);
    area.set_bounds(Rect::new(0.0, 0.0, 390.0, 274.0));
    area.add_panel(
        DockPanel::new(PanelId::ASSET_BROWSER, "Assets", DockPosition::Bottom)
            .with_size(180.0)
            .with_min_size(100.0),
    );
    area.layout();
    assert!(bounds_of(&area, PanelId::SCENE_VIEW).height < 100.0, "fixture: the centre is short");

    area.open_narrow_overlay(PanelId::INSPECTOR);

    let overlay = bounds_of(&area, PanelId::INSPECTOR);
    assert_eq!(overlay.bottom(), 274.0, "the overlay reaches the dock's bottom");
    assert!(overlay.y >= strip_bottom_of(&area), "and still starts below the strip");
    let content = content_of(&area, PanelId::INSPECTOR);
    assert!(content.height >= 150.0, "room to edit a field in, got {}", content.height);
}

/// The open overlay's chevron closes it. Its tab is gone while it is open,
/// so without the chevron the only way back to a bare viewport was to open
/// the other panel.
#[test]
fn test_the_overlays_chevron_closes_it_and_brings_its_tab_back() {
    let mut area = phone_dock(390.0);
    let theme = EditorTheme::default();
    let mut ui = UIContext::new();
    let mut input = input::InputHandler::new();
    area.open_narrow_overlay(PanelId::INSPECTOR);

    let chevron = chevron_bounds(area.get_panel(PanelId::INSPECTOR).expect("panel in the fixture")).center();
    press_at(&mut ui, &mut input, chevron, |ui| area.render(ui, &theme));
    release(&mut ui, &mut input, |ui| area.render(ui, &theme));

    assert_eq!(area.narrow_overlay(), None, "the chevron closes the overlay");
    assert_eq!(bounds_of(&area, PanelId::INSPECTOR).width, HEADER_HEIGHT, "the tab is back at the edge");
}
