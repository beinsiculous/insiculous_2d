//! Project-wide gameplay intensity selector.
//!
//! Each game decides what each variant *means*; the engine just carries the
//! selection. A game reads [`GameContext::chaos_mode`] (or checks the field on
//! its [`GameConfig`]) and branches gameplay accordingly.
//!
//! The engine intentionally ships no gameplay logic for these variants — a
//! racing game's "Insane" will look nothing like a Pong "Insane". The enum
//! exists to keep the *vocabulary* consistent across games.

use serde::{Deserialize, Serialize};

/// Recurring gameplay intensity theme: Normal / Insane / Ridiculous / Insiculous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChaosMode {
    #[default]
    Normal,
    Insane,
    Ridiculous,
    Insiculous,
}

impl ChaosMode {
    pub const ALL: [ChaosMode; 4] = [
        ChaosMode::Normal,
        ChaosMode::Insane,
        ChaosMode::Ridiculous,
        ChaosMode::Insiculous,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ChaosMode::Normal => "Normal",
            ChaosMode::Insane => "Insane",
            ChaosMode::Ridiculous => "Ridiculous",
            ChaosMode::Insiculous => "Insiculous",
        }
    }

    pub fn is_insane(self) -> bool {
        matches!(self, ChaosMode::Insane | ChaosMode::Insiculous)
    }

    pub fn is_ridiculous(self) -> bool {
        matches!(self, ChaosMode::Ridiculous | ChaosMode::Insiculous)
    }

    /// True only for `Insiculous` — the "both at once" combined mode.
    ///
    /// `is_insane()` and `is_ridiculous()` are both true in Insiculous as
    /// well, so games usually branch on those. Use this when a behavior
    /// should fire *only* in the combined mode and not in pure Insane or
    /// pure Ridiculous.
    pub fn is_insiculous(self) -> bool {
        matches!(self, ChaosMode::Insiculous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insiculous_is_both_insane_and_ridiculous() {
        // Games branch on the two predicates independently, so Insiculous
        // gets both behaviors for free — that is the whole point of the tier.
        for (mode, insane, ridiculous) in [
            (ChaosMode::Normal, false, false),
            (ChaosMode::Insane, true, false),
            (ChaosMode::Ridiculous, false, true),
            (ChaosMode::Insiculous, true, true),
        ] {
            assert!(ChaosMode::ALL.contains(&mode));
            assert_eq!(mode.is_insane(), insane, "{mode:?}");
            assert_eq!(mode.is_ridiculous(), ridiculous, "{mode:?}");
            assert_eq!(mode.is_insiculous(), insane && ridiculous, "{mode:?}");
        }
    }
}
