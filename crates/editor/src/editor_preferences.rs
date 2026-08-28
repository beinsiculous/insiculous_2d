//! Editor preferences for persisting editor state across sessions.
//!
//! Stores camera position, zoom level, last opened scene, grid settings,
//! and per-panel layout (visibility, collapse state, size).

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::dock::{DockArea, DockPosition, PanelId};

/// Persisted layout state for one dock panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Load preferences from a JSON file.
    ///
    /// Returns default preferences if the file doesn't exist or can't be parsed.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save preferences to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize preferences: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write preferences file: {}", e))
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

    #[test]
    fn test_editor_preferences_defaults() {
        let prefs = EditorPreferences::default();
        assert_eq!(prefs.camera_position, (0.0, 0.0));
        assert_eq!(prefs.camera_zoom, 1.0);
        assert!(prefs.last_scene_path.is_none());
        assert!(!prefs.snap_to_grid);
        assert_eq!(prefs.grid_size, 32.0);
        assert!(prefs.panels.is_empty());
    }

    #[test]
    fn test_editor_preferences_roundtrip() {
        let prefs = EditorPreferences {
            camera_position: (100.0, 200.0),
            camera_zoom: 2.5,
            last_scene_path: Some("scenes/test.ron".to_string()),
            snap_to_grid: true,
            grid_size: 64.0,
            grid_visible: false,
            panels: vec![PanelPrefs { id: 1, visible: false, collapsed: true, size: 320.0 }],
        };

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_editor_prefs.json");

        prefs.save(&path).expect("Failed to save");
        let loaded = EditorPreferences::load(&path);

        assert_eq!(loaded.camera_position, (100.0, 200.0));
        assert_eq!(loaded.camera_zoom, 2.5);
        assert_eq!(loaded.last_scene_path, Some("scenes/test.ron".to_string()));
        assert!(loaded.snap_to_grid);
        assert_eq!(loaded.grid_size, 64.0);
        assert!(!loaded.grid_visible);
        assert_eq!(loaded.panels.len(), 1);
        assert_eq!(loaded.panels[0].id, 1);
        assert!(!loaded.panels[0].visible);
        assert!(loaded.panels[0].collapsed);
        assert_eq!(loaded.panels[0].size, 320.0);

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_editor_preferences_load_missing_file() {
        let prefs = EditorPreferences::load(Path::new("/nonexistent/path.json"));
        assert_eq!(prefs.camera_zoom, 1.0); // Should return defaults
    }

    #[test]
    fn test_legacy_prefs_without_panels_field_still_load() {
        let legacy = r#"{
            "camera_position": [10.0, 20.0],
            "camera_zoom": 1.5,
            "last_scene_path": null,
            "snap_to_grid": false,
            "grid_size": 32.0
        }"#;
        let prefs: EditorPreferences = serde_json::from_str(legacy).expect("legacy JSON parses");
        assert_eq!(prefs.camera_position, (10.0, 20.0));
        assert!(prefs.panels.is_empty());
        // Prefs files predating the drawn grid default to visible.
        assert!(prefs.grid_visible);
    }

    /// Dock area with the editor's default panel set (minus theme concerns).
    fn test_dock() -> DockArea {
        let mut dock = DockArea::new();
        dock.set_bounds(ui::Rect::new(0.0, 0.0, 1000.0, 800.0));
        dock.add_panel(
            DockPanel::new(PanelId::HIERARCHY, "Hierarchy", DockPosition::Left)
                .with_size(200.0)
                .with_min_size(150.0),
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
    fn test_capture_apply_panels_roundtrip() {
        let mut dock = test_dock();
        dock.set_panel_collapsed(PanelId::HIERARCHY, true);
        dock.set_panel_visible(PanelId::INSPECTOR, false);
        dock.get_panel_mut(PanelId::INSPECTOR).unwrap().size = 333.0;

        let mut prefs = EditorPreferences::default();
        prefs.capture_panels(&dock);
        // Center panel is never captured
        assert_eq!(prefs.panels.len(), 2);
        assert!(prefs.panels.iter().all(|p| p.id != PanelId::SCENE_VIEW.0));

        let mut fresh = test_dock();
        prefs.apply_panels(&mut fresh);
        assert!(fresh.get_panel(PanelId::HIERARCHY).unwrap().collapsed);
        assert!(!fresh.get_panel(PanelId::INSPECTOR).unwrap().visible);
        assert_eq!(fresh.get_panel(PanelId::INSPECTOR).unwrap().size, 333.0);
    }

    #[test]
    fn test_apply_panels_clamps_size_and_skips_unknown() {
        let prefs = EditorPreferences {
            panels: vec![
                PanelPrefs { id: PanelId::HIERARCHY.0, visible: true, collapsed: false, size: 1.0 },
                PanelPrefs { id: 999, visible: false, collapsed: true, size: 50.0 },
            ],
            ..Default::default()
        };

        let mut dock = test_dock();
        prefs.apply_panels(&mut dock);
        // Size clamped up to the panel's min_size (150)
        assert_eq!(dock.get_panel(PanelId::HIERARCHY).unwrap().size, 150.0);
    }
}
