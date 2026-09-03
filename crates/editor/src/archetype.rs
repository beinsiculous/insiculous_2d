//! Archetypes for entity creation: shared between the Entity menu and the command API.

/// The fixed set of entity factories the Entity menu and the command API's `create` share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Archetype {
    Empty,
    Sprite,
    Camera,
    StaticBody,
    DynamicBody,
    KinematicBody,
    UiLabel,
    UiPanel,
    UiButton,
}

impl Archetype {
    pub const ALL: [Archetype; 9] = [
        Archetype::Empty,
        Archetype::Sprite,
        Archetype::Camera,
        Archetype::StaticBody,
        Archetype::DynamicBody,
        Archetype::KinematicBody,
        Archetype::UiLabel,
        Archetype::UiPanel,
        Archetype::UiButton,
    ];

    /// Command-API name ("static-body"). `const fn` so `ARCHETYPES` can stay a const.
    pub const fn kebab(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Sprite => "sprite",
            Self::Camera => "camera",
            Self::StaticBody => "static-body",
            Self::DynamicBody => "dynamic-body",
            Self::KinematicBody => "kinematic-body",
            Self::UiLabel => "ui-label",
            Self::UiPanel => "ui-panel",
            Self::UiButton => "ui-button",
        }
    }

    pub fn from_kebab(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.kebab() == name)
    }

    /// Entity-menu label ("Create Static Body").
    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::Empty => "Create Empty",
            Self::Sprite => "Create Sprite",
            Self::Camera => "Create Camera",
            Self::StaticBody => "Create Static Body",
            Self::DynamicBody => "Create Dynamic Body",
            Self::KinematicBody => "Create Kinematic Body",
            Self::UiLabel => "Create UI Label",
            Self::UiPanel => "Create UI Panel",
            Self::UiButton => "Create UI Button",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetype_from_kebab_round_trips_all_variants() {
        for archetype in Archetype::ALL {
            assert_eq!(Archetype::from_kebab(archetype.kebab()), Some(archetype));
        }
    }
}
