//! Component category definitions and categorization for the editor.

use super::ComponentKind;

/// Category grouping for the add-component popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentCategory {
    Core,
    Rendering,
    Physics,
    Audio,
    Gameplay,
    Ui,
}

impl ComponentCategory {
    /// All categories in display order.
    pub const ALL: [ComponentCategory; 6] = [
        ComponentCategory::Core,
        ComponentCategory::Rendering,
        ComponentCategory::Physics,
        ComponentCategory::Audio,
        ComponentCategory::Gameplay,
        ComponentCategory::Ui,
    ];

    /// Display name for the category header.
    pub fn label(self) -> &'static str {
        match self {
            ComponentCategory::Core => "Core",
            ComponentCategory::Rendering => "Rendering",
            ComponentCategory::Physics => "Physics",
            ComponentCategory::Audio => "Audio",
            ComponentCategory::Gameplay => "Gameplay",
            ComponentCategory::Ui => "UI",
        }
    }
}

/// Returns all component kinds grouped by category, in display order.
/// Categories with no components are omitted.
pub fn categorized_components() -> Vec<(ComponentCategory, Vec<ComponentKind>)> {
    ComponentCategory::ALL
        .iter()
        .map(|&category| {
            let kinds: Vec<ComponentKind> = ComponentKind::ALL
                .iter()
                .copied()
                .filter(|kind| kind.category() == category)
                .collect();
            (category, kinds)
        })
        .filter(|(_, kinds)| !kinds.is_empty())
        .collect()
}
