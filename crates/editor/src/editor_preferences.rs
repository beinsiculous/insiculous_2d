//! Editor preferences for persisting editor state across sessions.
//!
//! Stores camera position, zoom level, last opened scene, grid settings,
//! and per-panel layout (visibility, collapse state, size).

use serde::{Deserialize, Serialize};

use crate::dock::{DockArea, DockPosition, PanelId};

/// Persisted layout state for one dock panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelPrefs {
    /// Panel identifier (`PanelId.0`)
    pub id: u32,
    /// Whether the panel is visible
    pub visible: bool,
    /// Whether the panel is collapsed to a slim strip
    pub collapsed: bool,
    /// Panel size (width for Left/Right, height for Top/Bottom)
    pub size: f32,
}

/// Persistent editor preferences saved between sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorPreferences {
    /// Last camera position (x, y)
    pub camera_position: (f32, f32),
    /// Last camera zoom level
    pub camera_zoom: f32,
    /// Path to the last opened scene file
    pub last_scene_path: Option<String>,
    /// Whether snap-to-grid was enabled
    pub snap_to_grid: bool,
    /// Grid cell size
    pub grid_size: f32,
    /// Whether the authoring grid overlay was visible (absent in prefs
    /// files from older versions, which predate the drawn grid)
    #[serde(default = "default_grid_visible")]
    pub grid_visible: bool,
    /// Per-panel layout state (absent in prefs files from older versions)
    #[serde(default)]
    pub panels: Vec<PanelPrefs>,
}

fn default_grid_visible() -> bool {
    true
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            camera_position: (0.0, 0.0),
            camera_zoom: 1.0,
            last_scene_path: None,
            snap_to_grid: false,
            grid_size: 32.0,
            grid_visible: true,
            panels: Vec::new(),
        }
    }
}

impl EditorPreferences {
    /// Deserialize preferences from JSON text.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to parse preferences: {e}"))
    }

    /// Serialize preferences to pretty-printed JSON text.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize preferences: {e}"))
    }

    /// Capture the current panel layout from a dock area.
    ///
    /// The Center panel (scene view) is layout-derived and never persisted.
    pub fn capture_panels(&mut self, dock: &DockArea) {
        self.panels = dock
            .panels()
            .iter()
            .filter(|p| p.position != DockPosition::Center)
            .map(|p| PanelPrefs {
                id: p.id.0,
                visible: p.visible,
                collapsed: p.collapsed,
                size: p.size,
            })
            .collect();
    }

    /// Apply saved panel layout onto a dock area.
    ///
    /// Unknown panel ids and Center panels are skipped; sizes are clamped
    /// to each panel's minimum so a corrupt file can't zero out a panel.
    pub fn apply_panels(&self, dock: &mut DockArea) {
        for pref in &self.panels {
            let Some(panel) = dock.get_panel_mut(PanelId(pref.id)) else {
                continue;
            };
            if panel.position == DockPosition::Center {
                continue;
            }
            panel.visible = pref.visible;
            panel.collapsed = pref.collapsed && panel.is_collapsible();
            panel.size = pref.size.max(panel.min_size);
        }
        dock.layout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dock::DockPanel;

    const HIERARCHY_MIN_WIDTH: f32 = 150.0;

    /// Dock area with the editor's default panel set (minus theme concerns).
    fn test_dock() -> DockArea {
        let mut dock = DockArea::new();
        dock.set_bounds(ui::Rect::new(0.0, 0.0, 1000.0, 800.0));
        dock.add_panel(
            DockPanel::new(PanelId::HIERARCHY, "Hierarchy", DockPosition::Left)
                .with_size(200.0)
                .with_min_size(HIERARCHY_MIN_WIDTH),
        );
        dock.add_panel(
            DockPanel::new(PanelId::INSPECTOR, "Inspector", DockPosition::Right)
                .with_size(280.0)
                .with_min_size(200.0),
        );
        dock.add_panel(DockPanel::new(PanelId::SCENE_VIEW, "Scene", DockPosition::Center));
        dock.layout();
        dock
    }

    #[test]
    fn test_prefs_round_trip_through_json_including_the_panel_layout() -> Result<(), String> {
        let mut dock = test_dock();
        dock.set_panel_collapsed(PanelId::HIERARCHY, true);
        dock.set_panel_visible(PanelId::INSPECTOR, false);
        dock.get_panel_mut(PanelId::INSPECTOR).expect("inspector").size = 333.0;
        let mut prefs = EditorPreferences {
            camera_position: (100.0, 200.0),
            camera_zoom: 2.5,
            last_scene_path: Some("scenes/test.ron".to_string()),
            snap_to_grid: true,
            grid_size: 64.0,
            grid_visible: false,
            panels: Vec::new(),
        };
        prefs.capture_panels(&dock);

        let json = prefs.to_json()?;
        let loaded = EditorPreferences::from_json(&json)?;
        let mut fresh = test_dock();
        loaded.apply_panels(&mut fresh);

        assert_eq!(loaded.camera_position, (100.0, 200.0));
        assert_eq!(loaded.camera_zoom, 2.5);
        assert_eq!(loaded.last_scene_path.as_deref(), Some("scenes/test.ron"));
        assert!(loaded.snap_to_grid);
        assert_eq!(loaded.grid_size, 64.0);
        assert!(!loaded.grid_visible);
        // The Center panel is layout-derived and never persisted.
        assert_eq!(loaded.panels.len(), 2);
        assert!(loaded.panels.iter().all(|p| p.id != PanelId::SCENE_VIEW.0));
        assert!(fresh.get_panel(PanelId::HIERARCHY).expect("hierarchy").collapsed);
        assert!(!fresh.get_panel(PanelId::INSPECTOR).expect("inspector").visible);
        assert_eq!(fresh.get_panel(PanelId::INSPECTOR).expect("inspector").size, 333.0);
        Ok(())
    }

    #[test]
    fn test_apply_panels_clamps_sizes_to_the_panel_minimum_and_skips_unknown_ids() {
        let prefs = EditorPreferences {
            panels: vec![
                PanelPrefs { id: PanelId::HIERARCHY.0, visible: true, collapsed: false, size: 1.0 },
                PanelPrefs { id: 999, visible: false, collapsed: true, size: 50.0 },
            ],
            ..Default::default()
        };
        let mut dock = test_dock();

        prefs.apply_panels(&mut dock);

        assert_eq!(
            dock.get_panel(PanelId::HIERARCHY).expect("hierarchy").size,
            HIERARCHY_MIN_WIDTH,
            "a corrupt file cannot zero out a panel"
        );
        assert_eq!(dock.panels().len(), 3, "an unknown id adds nothing");
    }

    #[test]
    fn test_legacy_prefs_without_panels_or_grid_fields_still_load() {
        let legacy = r#"{
            "camera_position": [10.0, 20.0],
            "camera_zoom": 1.5,
            "last_scene_path": null,
            "snap_to_grid": false,
            "grid_size": 32.0
        }"#;

        let prefs: EditorPreferences =
            EditorPreferences::from_json(legacy).expect("legacy JSON parses");

        assert_eq!(prefs.camera_position, (10.0, 20.0));
        assert!(prefs.panels.is_empty());
        assert!(prefs.grid_visible, "prefs files predating the drawn grid default to visible");
    }

    #[test]
    fn test_from_json_fails_on_truncated_json_and_caller_falls_back_to_defaults() {
        let truncated = r#"{"camera_position": [100.0, 200.0], "camera_zo"#;
        let result = EditorPreferences::from_json(truncated);
        assert!(result.is_err());

        let fallback = result.unwrap_or_default();
        assert_eq!(fallback.camera_zoom, 1.0, "a corrupt file falls back to defaults");
        assert_eq!(fallback.camera_position, (0.0, 0.0), "no partial values leak");
    }
}
