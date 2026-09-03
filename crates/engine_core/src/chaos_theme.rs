//! Per-chaos-mode presentation tokens shared by every game.
//!
//! The engine owns the *structure* (which tokens exist) and a default
//! palette; games own the *meaning* — remap tokens to their objects
//! (`accent_color` = ball, player ship, cursor …) and override colors where
//! their look differs, via struct-update syntax:
//!
//! ```
//! use engine_core::prelude::*;
//!
//! let base = ChaosTheme::for_mode(ChaosMode::Normal);
//! let theme = ChaosTheme {
//!     bg_color: Vec4::new(0.0, 0.01, 0.04, 1.0), // navy-tinted background
//!     ..base
//! };
//! assert_eq!(theme.particle_count_mult, 1.0);
//! ```

use glam::Vec4;

use crate::chaos_mode::ChaosMode;

/// Per-mode presentation tokens: background tint, structure/accent colors,
/// grid color, an optional HUD banner, and a particle-density multiplier.
///
/// Gameplay branches read [`ChaosMode`] directly — this is only the *look*
/// of each mode, so art tweaks stay in one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaosTheme {
    /// Background clear tint.
    pub bg_color: Vec4,
    /// Static structure color (walls, barriers, frames).
    pub structure_color: Vec4,
    /// Hero-object accent color (ball, player ship, cursor).
    pub accent_color: Vec4,
    /// HUD banner shown during gameplay (None for Normal).
    pub banner_text: Option<&'static str>,
    /// Banner tint.
    pub banner_color: Vec4,
    /// Color of the deforming grid background.
    pub grid_color: Vec4,
    /// Multiplier on particle counts — higher chaos modes spawn denser bursts.
    pub particle_count_mult: f32,
}

impl ChaosTheme {
    /// The default palette for a mode (the shared neon look). Games override
    /// individual fields with struct-update syntax where their art differs.
    pub fn for_mode(mode: ChaosMode) -> Self {
        match mode {
            ChaosMode::Normal => Self {
                bg_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
                structure_color: Vec4::new(0.35, 0.35, 0.42, 1.0),
                accent_color: Vec4::ONE,
                banner_text: None,
                banner_color: Vec4::ONE,
                grid_color: Vec4::new(0.15, 0.3, 0.7, 0.5),
                particle_count_mult: 1.0,
            },
            ChaosMode::Insane => Self {
                bg_color: Vec4::new(0.18, 0.02, 0.02, 1.0),
                structure_color: Vec4::new(1.0, 0.4, 0.2, 1.0),
                accent_color: Vec4::new(1.0, 0.82, 0.6, 1.0),
                banner_text: Some("!! INSANE !!"),
                banner_color: Vec4::new(1.0, 0.5, 0.3, 1.0),
                grid_color: Vec4::new(0.9, 0.2, 0.1, 0.55),
                particle_count_mult: 1.6,
            },
            ChaosMode::Ridiculous => Self {
                bg_color: Vec4::new(0.08, 0.02, 0.15, 1.0),
                structure_color: Vec4::new(0.9, 0.3, 1.0, 1.0),
                accent_color: Vec4::new(1.0, 0.75, 1.0, 1.0),
                banner_text: Some("~~ RIDICULOUS ~~"),
                banner_color: Vec4::new(0.95, 0.4, 1.0, 1.0),
                grid_color: Vec4::new(0.8, 0.2, 1.0, 0.55),
                particle_count_mult: 1.8,
            },
            ChaosMode::Insiculous => Self {
                bg_color: Vec4::new(0.04, 0.08, 0.04, 1.0),
                structure_color: Vec4::new(0.5, 1.0, 0.3, 1.0),
                accent_color: Vec4::new(0.85, 1.0, 0.55, 1.0),
                banner_text: Some(">>> INSICULOUS <<<"),
                banner_color: Vec4::new(0.7, 1.0, 0.4, 1.0),
                grid_color: Vec4::new(0.4, 1.0, 0.3, 0.6),
                particle_count_mult: 2.4,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_normal_mode_has_no_banner() {
        for mode in ChaosMode::ALL {
            let theme = ChaosTheme::for_mode(mode);
            assert_eq!(
                theme.banner_text.is_none(),
                mode == ChaosMode::Normal,
                "banner presence wrong for {mode:?}"
            );
        }
    }

    #[test]
    fn test_particle_density_rises_with_chaos() {
        let normal = ChaosTheme::for_mode(ChaosMode::Normal).particle_count_mult;
        let insiculous = ChaosTheme::for_mode(ChaosMode::Insiculous).particle_count_mult;
        assert!(normal < insiculous, "Insiculous must out-particle Normal");
    }
}
