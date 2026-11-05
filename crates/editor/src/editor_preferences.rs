//! Editor preferences for persisting editor state across sessions.
//!
//! Stores camera position, zoom level, last opened scene, grid settings,
//! per-panel layout (visibility, collapse state, size) and the external tools.

use serde::{Deserialize, Serialize};

use crate::dock::{DockArea, DockPosition, PanelId};

/// The editor's scale preference; old documents follow the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UiScale {
    #[default]
    FollowDisplay,
    Percent(u16),
}

/// The explicit scale choices in the menu and keyboard step sequence.
pub const UI_SCALE_PERCENTAGES: [u16; 5] = [100, 125, 150, 175, 200];

impl Serialize for UiScale {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::FollowDisplay => serializer.serialize_str("follow"),
            Self::Percent(percent) => serializer.serialize_u16(*percent),
        }
    }
}

impl<'de> Deserialize<'de> for UiScale {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value { Text(String), Percent(u16) }
        match Value::deserialize(deserializer)? {
            Value::Text(text) if text == "follow" => Ok(Self::FollowDisplay),
            Value::Percent(percent) => Ok(Self::Percent(percent)),
            Value::Text(_) => Err(serde::de::Error::custom("UI scale must be 'follow' or a percentage")),
        }
    }
}

/// Resolve one finite pixels-per-point value for all editor coordinate conversions.
/// Zero and non-finite values fall back to 1.0; other values clamp to 0.5 through 4.0.
pub fn resolve_ui_scale(preference: &UiScale, display_scale: f32) -> f32 {
    let scale = match preference {
        UiScale::FollowDisplay => display_scale,
        UiScale::Percent(percent) => f32::from(*percent) / 100.0,
    };
    if !scale.is_finite() || scale == 0.0 { 1.0 } else { scale.clamp(0.5, 4.0) }
}

impl UiScale {
    /// Choose the next explicit menu step above or below the currently resolved scale; past
    /// the last step the preference is returned unchanged.
    pub fn stepped(self, display_scale: f32, increase: bool) -> Self {
        let current_percent = resolve_ui_scale(&self, display_scale) * 100.0;
        // Past the last step in either direction the preference stays: a display at 250 %
        // stepped up must not drop to 200 %, nor one at 75 % stepped down rise to 100 %.
        let percent = if increase {
            UI_SCALE_PERCENTAGES.into_iter().find(|percent| f32::from(*percent) > current_percent)
        } else {
            UI_SCALE_PERCENTAGES.into_iter().rev().find(|percent| f32::from(*percent) < current_percent)
        };
        percent.map(Self::Percent).unwrap_or(self)
    }
}

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
    /// Scale for the editor chrome and viewport, independent of the game HUD.
    #[serde(default)]
    pub ui_scale: UiScale,
    /// Last camera position (x, y)
    pub camera_position: (f32, f32),
    /// Last camera zoom level
    pub camera_zoom: f32,
    /// Path to the last opened scene file
    pub last_scene_path: Option<String>,
    /// Grid cell size
    pub grid_size: f32,
    /// View toggles (grid, colliders, game frame, snap) flattened into top-level keys
    #[serde(flatten)]
    pub view: crate::view_toggles::ViewToggles,
    /// Per-panel layout state (absent in prefs files from older versions)
    #[serde(default)]
    pub panels: Vec<PanelPrefs>,
    /// Which program opens which project file. A file from before this field
    /// carried an `ide_command` string instead, read into the code tool on load.
    #[serde(default)]
    pub external_tools: crate::external_tools::ExternalToolsConfig,
    /// Registry type names whose inspector section is collapsed (absent in
    /// prefs files from older versions)
    #[serde(default)]
    pub collapsed_components: Vec<String>,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            ui_scale: UiScale::FollowDisplay,
            camera_position: (0.0, 0.0),
            camera_zoom: 1.0,
            last_scene_path: None,
            grid_size: 32.0,
            view: crate::view_toggles::ViewToggles::default(),
            panels: Vec::new(),
            external_tools: crate::external_tools::ExternalToolsConfig::default(),
            collapsed_components: Vec::new(),
        }
    }
}

impl EditorPreferences {
    /// Deserialize preferences from JSON text. A legacy `ide_command` becomes the
    /// custom code tool and is never written back. An `external_tools` section that
    /// does not parse — a hand edit gone wrong, or a tool only a newer editor knows —
    /// is warned about and left at its defaults, so the rest of the preferences survive.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse preferences: {e}"))?;
        let (legacy_ide_command, external_tools) = match value.as_object_mut() {
            Some(fields) => (fields.remove("ide_command"), fields.remove("external_tools")),
            None => (None, None),
        };
        let mut prefs: Self =
            serde_json::from_value(value).map_err(|e| format!("Failed to parse preferences: {e}"))?;
        if let Some(external_tools) = external_tools {
            match serde_json::from_value(external_tools) {
                Ok(config) => prefs.external_tools = config,
                Err(error) => log::warn!(
                    "the editor preferences' external_tools section was ignored and the next save replaces it with the defaults: {error}"
                ),
            }
        }
        if let Some(serde_json::Value::String(ide_command)) = legacy_ide_command {
            prefs.external_tools.adopt_legacy_ide_command(&ide_command);
        }
        Ok(prefs)
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

    #[test]
    fn scale_preferences_round_trip_and_old_files_follow_the_display() {
        for preference in [UiScale::FollowDisplay, UiScale::Percent(150), UiScale::Percent(0), UiScale::Percent(65535)] {
            let preferences = EditorPreferences { ui_scale: preference, ..Default::default() };
            let json = preferences.to_json().unwrap();
            assert_eq!(EditorPreferences::from_json(&json).unwrap().ui_scale, preference);
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["ui_scale"], match preference {
                UiScale::FollowDisplay => serde_json::json!("follow"),
                UiScale::Percent(percent) => serde_json::json!(percent),
            });
        }
        let mut old = serde_json::to_value(EditorPreferences::default()).unwrap();
        old.as_object_mut().unwrap().remove("ui_scale");
        assert_eq!(EditorPreferences::from_json(&old.to_string()).unwrap().ui_scale, UiScale::FollowDisplay);
    }

    #[test]
    fn malformed_scale_values_cannot_zero_or_poison_layout_and_steps_use_the_live_scale() {
        for (preference, display, expected) in [
            (UiScale::Percent(0), 2.0, 1.0), (UiScale::Percent(1), 2.0, 0.5),
            (UiScale::Percent(65535), 1.0, 4.0), (UiScale::FollowDisplay, f32::NAN, 1.0),
            (UiScale::FollowDisplay, 0.0, 1.0), (UiScale::FollowDisplay, f32::INFINITY, 1.0),
            (UiScale::FollowDisplay, 1.5, 1.5),
        ] {
            assert_eq!(resolve_ui_scale(&preference, display), expected);
        }
        assert_eq!(UiScale::FollowDisplay.stepped(1.5, true), UiScale::Percent(175));
        assert_eq!(UiScale::FollowDisplay.stepped(1.5, false), UiScale::Percent(125));
        assert_eq!(UiScale::FollowDisplay.stepped(2.5, true), UiScale::FollowDisplay);
        assert_eq!(UiScale::FollowDisplay.stepped(0.75, false), UiScale::FollowDisplay);
        assert_eq!(UiScale::Percent(200).stepped(1.0, true), UiScale::Percent(200));
        assert_eq!(UiScale::Percent(100).stepped(1.0, false), UiScale::Percent(100));
    }

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
            ui_scale: UiScale::Percent(150),
            camera_position: (100.0, 200.0),
            camera_zoom: 2.5,
            last_scene_path: Some("scenes/test.ron".to_string()),
            grid_size: 64.0,
            view: crate::view_toggles::ViewToggles {
                grid: false,
                colliders: true,
                game_frame: false,
                snap: true,
            },
            panels: Vec::new(),
            external_tools: crate::external_tools::ExternalToolsConfig::default(),
            collapsed_components: vec!["Sprite".to_string()],
        };
        prefs.capture_panels(&dock);

        let json = prefs.to_json()?;
        let loaded = EditorPreferences::from_json(&json)?;
        let mut fresh = test_dock();
        loaded.apply_panels(&mut fresh);

        assert_eq!(loaded.camera_position, (100.0, 200.0));
        assert_eq!(loaded.camera_zoom, 2.5);
        assert_eq!(loaded.last_scene_path.as_deref(), Some("scenes/test.ron"));
        assert_eq!(loaded.grid_size, 64.0);
        assert!(!loaded.view.grid);
        assert!(loaded.view.colliders);
        assert!(!loaded.view.game_frame);
        assert!(loaded.view.snap);
        assert_eq!(loaded.collapsed_components, vec!["Sprite".to_string()], "a collapsed section survives a restart");
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
        assert!(!prefs.view.snap, "legacy snap_to_grid false lands in view.snap");
        assert!(prefs.view.grid, "prefs files predating the drawn grid default to visible");
        assert!(prefs.view.colliders, "colliders default to visible");
        assert!(prefs.view.game_frame, "game frame defaults to visible");
        assert_eq!(prefs.external_tools, crate::external_tools::ExternalToolsConfig::default());
        assert!(prefs.collapsed_components.is_empty(), "prefs files predating collapsible sections open every section");
    }

    #[test]
    fn test_prefs_without_any_view_toggles_loads_defaults() {
        let no_toggles = r#"{
            "camera_position": [0.0, 0.0],
            "camera_zoom": 1.0,
            "last_scene_path": null,
            "grid_size": 32.0,
            "panels": []
        }"#;

        let prefs: EditorPreferences =
            EditorPreferences::from_json(no_toggles).expect("valid JSON without view toggles parses");

        assert_eq!(prefs.view, crate::view_toggles::ViewToggles::default());
    }

    #[test]
    fn test_an_old_ide_command_loads_as_the_custom_code_tool_and_is_never_written_again() -> Result<(), String> {
        use crate::external_tools::{CommandTemplate, CustomTool, ToolChoice};
        let old = r#"{
            "camera_position": [0.0, 0.0],
            "camera_zoom": 1.0,
            "last_scene_path": null,
            "grid_size": 32.0,
            "panels": [],
            "ide_command": "code --wait"
        }"#;

        let prefs = EditorPreferences::from_json(old)?;
        assert_eq!(
            prefs.external_tools.code,
            ToolChoice::Custom(CustomTool::file_only(CommandTemplate("code --wait {file}".to_string()))),
            "the old command opened the file it was given, so the template names it"
        );
        let json = prefs.to_json()?;
        assert!(!json.contains("ide_command"), "the old field is read once and never written");
        assert_eq!(EditorPreferences::from_json(&json)?, prefs);
        Ok(())
    }

    #[test]
    fn test_an_unreadable_external_tools_section_keeps_every_other_preference() -> Result<(), String> {
        let hand_edited = r#"{
            "camera_position": [0.0, 0.0],
            "camera_zoom": 2.5,
            "last_scene_path": null,
            "grid_size": 32.0,
            "panels": [],
            "external_tools": { "code": "RustRover" }
        }"#;

        let prefs = EditorPreferences::from_json(hand_edited)?;

        assert_eq!(prefs.camera_zoom, 2.5, "a mistake in the tools section costs the camera nothing");
        assert_eq!(prefs.external_tools, crate::external_tools::ExternalToolsConfig::default());
        Ok(())
    }

    #[test]
    fn test_a_new_prefs_file_round_trips_its_external_tools() -> Result<(), String> {
        use crate::external_tools::{FileType, PresetId, ToolChoice};
        let mut prefs = EditorPreferences::default();
        prefs.external_tools.code = ToolChoice::Preset(PresetId::RustRover);
        prefs.external_tools.music = ToolChoice::Preset(PresetId::MidiEvil);
        prefs.external_tools.overrides.insert(FileType::SoundFont, ToolChoice::OsDefault);
        prefs.external_tools.daw_project_extensions = vec![".evil".to_string(), ".als".to_string()];

        let loaded = EditorPreferences::from_json(&prefs.to_json()?)?;

        assert_eq!(loaded.external_tools, prefs.external_tools);
        Ok(())
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
