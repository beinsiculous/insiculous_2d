//! Preferences preserve visibility independently of tab membership and active selection.

use crate::{EditorContext, EditorPreferences, PanelId, PanelPrefs};
use ui::Rect;

fn context() -> EditorContext {
    let mut context = EditorContext::new();
    context.dock_area.set_bounds(Rect::new(0.0, 0.0, 1280.0, 800.0));
    context.dock_area.layout();
    context
}

#[test]
fn legacy_visible_assets_win_over_a_hidden_collapsed_conductor_and_clamp_to_the_group_minimum() {
    let mut context = context();
    let preferences = EditorPreferences { panels: vec![
        PanelPrefs { id: 5, visible: false, collapsed: true, size: 260.0 },
        PanelPrefs { id: 3, visible: true, collapsed: false, size: 180.0 },
    ], ..Default::default() };
    preferences.apply_panels(&mut context.dock_area);
    let content = context.dock_area.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds();
    assert_eq!(content.height, 200.0 - crate::layout::HEADER_HEIGHT);
    assert!(!context.dock_area.get_panel(PanelId::CONDUCTOR).unwrap().visible);
    let mut saved = EditorPreferences::default();
    saved.capture_panels(&context.dock_area);
    let json = saved.to_json().unwrap();
    assert!(!serde_json::from_str::<serde_json::Value>(&json).unwrap().as_object().unwrap().contains_key("panels"));
    assert_eq!(EditorPreferences::from_json(&json).unwrap(), saved);
}

#[test]
fn an_all_hidden_legacy_group_restores_default_size_and_expands_when_shown() {
    let mut context = context();
    let preferences = EditorPreferences { panels: vec![
        PanelPrefs { id: 3, visible: false, collapsed: true, size: 400.0 },
        PanelPrefs { id: 5, visible: false, collapsed: true, size: 500.0 },
        PanelPrefs { id: 4, visible: false, collapsed: true, size: 600.0 },
    ], ..Default::default() };
    preferences.apply_panels(&mut context.dock_area);
    for id in [PanelId::ASSET_BROWSER, PanelId::CONDUCTOR, PanelId::CONSOLE] {
        assert_eq!(context.dock_area.get_panel(id).unwrap().content_bounds(), Rect::default());
    }
    context.dock_area.set_panel_visible(PanelId::ASSET_BROWSER, true);
    assert_eq!(context.dock_area.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds().height,
        200.0 - crate::layout::HEADER_HEIGHT);
}

#[test]
fn new_groups_round_trip_hidden_default_visible_and_shown_default_hidden_panels() {
    let mut original = context();
    original.dock_area.set_panel_visible(PanelId::INSPECTOR, false);
    original.dock_area.show_panel(PanelId::SETTINGS);
    original.dock_area.show_panel(PanelId::CONSOLE);
    let mut saved = EditorPreferences::default();
    saved.capture_panels(&original.dock_area);
    let loaded = EditorPreferences::from_json(&saved.to_json().unwrap()).unwrap();
    let mut restored = context();
    loaded.apply_panels(&mut restored.dock_area);
    assert!(!restored.dock_area.get_panel(PanelId::INSPECTOR).unwrap().visible);
    for id in [PanelId::SETTINGS, PanelId::CONSOLE] {
        assert!(restored.dock_area.get_panel(id).unwrap().content_bounds().height > 0.0);
    }
    assert_eq!(restored.dock_area.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds(), Rect::default());
}

#[test]
fn a_hidden_or_unknown_saved_active_member_falls_back_to_the_first_visible_tab() {
    for active in [PanelId::CONDUCTOR.0, 999] {
        let mut original = context();
        let mut saved = EditorPreferences::default();
        saved.capture_panels(&original.dock_area);
        let bottom = saved.groups.iter_mut().find(|group| group.panels.contains(&PanelId::ASSET_BROWSER.0)).unwrap();
        bottom.active = active;
        saved.apply_panels(&mut original.dock_area);
        assert!(original.dock_area.get_panel(PanelId::ASSET_BROWSER).unwrap().content_bounds().height > 0.0);
        assert_eq!(original.dock_area.get_panel(PanelId::CONDUCTOR).unwrap().content_bounds(), Rect::default());
    }
}
