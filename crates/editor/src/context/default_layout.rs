//! The editor's panels as a fresh editor lays them out.

use crate::{DockArea, DockPanel, DockPosition, PanelId};

/// The editor's default panel layout: Hierarchy left, Inspector right,
/// Scene center, Asset browser bottom.
pub(super) fn default_dock_area() -> DockArea {
    let mut dock_area = DockArea::new();
    dock_area.add_panel(
        DockPanel::new(PanelId::HIERARCHY, "Hierarchy", DockPosition::Left)
            .with_size(200.0)
            .with_min_size(150.0)
            .with_hint("The scene's entities; click one to select it."),
    );
    dock_area.add_panel(
        DockPanel::new(PanelId::INSPECTOR, "Inspector", DockPosition::Right)
            .with_size(280.0)
            .with_min_size(200.0)
            .with_hint("The fields of the selected entity; every edit is undoable."),
    );
    dock_area.add_panel(
        DockPanel::new(PanelId::SCENE_VIEW, "Scene", DockPosition::Center)
            .with_hint("The world you are building; pick and drag its entities here."),
    );
    dock_area.add_panel(
        DockPanel::new(PanelId::ASSET_BROWSER, "Assets", DockPosition::Bottom)
            .with_size(180.0)
            .with_min_size(100.0)
            .with_hint("This project's files; drag one into the scene to use it."),
    );
    // Hidden tabs join Assets without displacing its initial selection.
    let mut conductor = DockPanel::new(PanelId::CONDUCTOR, "Conductor", DockPosition::Bottom)
        .with_size(260.0)
        .with_min_size(200.0)
        .with_hint("The song's map: its transport, and its parts, instruments and bindings.");
    conductor.visible = false;
    dock_area.add_panel(conductor);
    let mut console = DockPanel::new(PanelId::CONSOLE, "Console", DockPosition::Bottom)
        .with_size(160.0)
        .with_min_size(80.0)
        .with_hint("What the editor reported: script errors and launch results; click a file:line to open it.");
    console.visible = false;
    dock_area.add_panel(console);
    let mut settings = DockPanel::new(PanelId::SETTINGS, "External Tools", DockPosition::Right)
        .with_size(320.0)
        .with_min_size(240.0)
        .with_hint("Which program opens which file: a tool per category, overrides per file type, and a Test for each.");
    settings.visible = false;
    dock_area.add_panel(settings);
    dock_area
}
