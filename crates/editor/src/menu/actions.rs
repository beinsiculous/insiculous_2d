//! Mapping from menu item labels to editor actions.

use crate::archetype::Archetype;
use crate::dock::panel_id_for_menu_label;
use crate::editor_input::EditorAction;

/// Menu label → editor action. The menu bar speaks labels; everything after it speaks actions.
pub fn action_for_menu_label(label: &str) -> Option<EditorAction> {
    if let Some(panel) = panel_id_for_menu_label(label) {
        return Some(EditorAction::TogglePanel(panel));
    }
    if let Some(archetype) = Archetype::ALL.into_iter().find(|a| a.menu_label() == label) {
        return Some(EditorAction::CreateEntity(archetype));
    }
    Some(match label {
        "New Scene" => EditorAction::NewScene,
        "Open Scene..." => EditorAction::OpenScene,
        "Save" => EditorAction::Save,
        "Save As..." => EditorAction::SaveAs,
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
        "Snap to Grid" => EditorAction::ToggleSnap,
        "Cycle Game Locale" => EditorAction::CycleGameLocale,
        "Reset Layout" => EditorAction::ResetLayout,
        _ => return None,
    })
}
