//! Mapping from menu item labels to editor actions.

use crate::archetype::Archetype;
use crate::dock::panel_id_for_menu_label;
use crate::editor_input::EditorAction;
use crate::external_tools::ToolName;

/// What every form of Edit → Open Project's label starts with: the rest names the code
/// tool, which changes with the settings, so the label is matched by this prefix.
pub const OPEN_PROJECT_PREFIX: &str = "Open Project ";
/// Edit → External Tools…, which shows the settings panel.
pub const EXTERNAL_TOOLS_LABEL: &str = "External Tools…";

/// Edit → Open Project's label for the code tool, and whether it is enabled: the system
/// default opens files, never a project, and the web launches nothing.
pub fn open_project_item(tool: ToolName, desktop: bool) -> (String, bool) {
    let label = match tool {
        ToolName::Preset(name) => format!("{OPEN_PROJECT_PREFIX}in {name}"),
        ToolName::CustomCommand => format!("{OPEN_PROJECT_PREFIX}with the custom command"),
        ToolName::SystemDefault => format!("{OPEN_PROJECT_PREFIX}with the system default"),
    };
    (label, desktop && tool != ToolName::SystemDefault)
}

/// Menu label → editor action. The menu bar speaks labels; everything after it speaks actions.
pub fn action_for_menu_label(label: &str) -> Option<EditorAction> {
    if let Some(panel) = panel_id_for_menu_label(label) {
        return Some(EditorAction::TogglePanel(panel));
    }
    if label.starts_with(OPEN_PROJECT_PREFIX) {
        return Some(EditorAction::OpenProject);
    }
    if let Some(archetype) = Archetype::ALL.into_iter().find(|a| a.menu_label() == label) {
        return Some(EditorAction::CreateEntity(archetype));
    }
    Some(match label {
        "New Scene" => EditorAction::NewScene,
        "Open Scene..." => EditorAction::OpenScene,
        "Save" => EditorAction::Save,
        "Save As..." => EditorAction::SaveAs,
        "New Song" => EditorAction::NewSong,
        "Open Song..." => EditorAction::OpenSong,
        "Save Song" => EditorAction::SaveSong,
        "Save Song As..." => EditorAction::SaveSongAs,
        "Close Song" => EditorAction::CloseSong,
        "Exit" => EditorAction::Exit,
        "Undo" => EditorAction::Undo,
        "Redo" => EditorAction::Redo,
        "Cut" => EditorAction::Cut,
        "Copy" => EditorAction::Copy,
        "Paste" => EditorAction::Paste,
        "Delete" => EditorAction::Delete,
        "Duplicate" => EditorAction::Duplicate,
        "Toggle Grid" => EditorAction::ToggleGrid,
        "Toggle Colliders" => EditorAction::ToggleColliders,
        "Toggle Game Frame" => EditorAction::ToggleGameFrame,
        "Snap to Grid" => EditorAction::ToggleSnap,
        "Cycle Game Locale" => EditorAction::CycleGameLocale,
        "Reset Layout" => EditorAction::ResetLayout,
        EXTERNAL_TOOLS_LABEL => EditorAction::ShowExternalTools,
        _ => return None,
    })
}
