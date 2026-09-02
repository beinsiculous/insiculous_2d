//! Data-driven game-UI components: screen-space labels, panels, and buttons
//! that live in the world (and therefore in scene files and the editor).
//!
//! These are pure data — the engine's UI-element pass reads them each frame
//! and emits immediate-mode UI. Placement is anchor-based: the element's
//! anchor point coincides with the same point of the window, then `offset`
//! (in pixels, +y down like all screen space) shifts it. UI entities need
//! no `Transform2D`; anchor + offset IS their placement model.
//!
//! `text` fields starting with `@` are localization keys, resolved through
//! `engine_core`'s `Strings` at draw time.

use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};

use crate::component_registry::ComponentMeta;
use crate::DeriveComponentMeta;

/// The nine screen anchor points a UI element can attach to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UiAnchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl UiAnchor {
    /// All anchors in reading order (used by editor cycle selectors).
    pub const ALL: [UiAnchor; 9] = [
        UiAnchor::TopLeft,
        UiAnchor::TopCenter,
        UiAnchor::TopRight,
        UiAnchor::CenterLeft,
        UiAnchor::Center,
        UiAnchor::CenterRight,
        UiAnchor::BottomLeft,
        UiAnchor::BottomCenter,
        UiAnchor::BottomRight,
    ];

    /// Human-readable name (also the RON spelling).
    pub fn label(&self) -> &'static str {
        match self {
            UiAnchor::TopLeft => "TopLeft",
            UiAnchor::TopCenter => "TopCenter",
            UiAnchor::TopRight => "TopRight",
            UiAnchor::CenterLeft => "CenterLeft",
            UiAnchor::Center => "Center",
            UiAnchor::CenterRight => "CenterRight",
            UiAnchor::BottomLeft => "BottomLeft",
            UiAnchor::BottomCenter => "BottomCenter",
            UiAnchor::BottomRight => "BottomRight",
        }
    }

    /// Index into [`Self::ALL`].
    pub fn index(&self) -> usize {
        Self::ALL.iter().position(|a| a == self).unwrap_or(0)
    }

    /// The anchor point as a fraction of a rect: `(0,0)` top-left,
    /// `(0.5,0.5)` center, `(1,1)` bottom-right.
    pub fn fraction(&self) -> Vec2 {
        let x = match self {
            UiAnchor::TopLeft | UiAnchor::CenterLeft | UiAnchor::BottomLeft => 0.0,
            UiAnchor::TopCenter | UiAnchor::Center | UiAnchor::BottomCenter => 0.5,
            UiAnchor::TopRight | UiAnchor::CenterRight | UiAnchor::BottomRight => 1.0,
        };
        let y = match self {
            UiAnchor::TopLeft | UiAnchor::TopCenter | UiAnchor::TopRight => 0.0,
            UiAnchor::CenterLeft | UiAnchor::Center | UiAnchor::CenterRight => 0.5,
            UiAnchor::BottomLeft | UiAnchor::BottomCenter | UiAnchor::BottomRight => 1.0,
        };
        Vec2::new(x, y)
    }
}

/// Top-left screen position of a `size`-sized rect anchored to `anchor` of
/// a `window`-sized screen, shifted by `offset` pixels: the rect's anchor
/// point coincides with the window's anchor point, plus the offset.
pub fn resolve_anchored_pos(anchor: UiAnchor, offset: Vec2, size: Vec2, window: Vec2) -> Vec2 {
    let f = anchor.fraction();
    window * f - size * f + offset
}

fn default_true() -> bool {
    true
}

fn default_font_size() -> f32 {
    16.0
}

fn default_color() -> Vec4 {
    Vec4::ONE
}

fn default_panel_background() -> Vec4 {
    Vec4::new(0.1, 0.1, 0.15, 0.85)
}

fn default_border_width() -> f32 {
    1.0
}

fn default_button_size() -> Vec2 {
    Vec2::new(120.0, 32.0)
}

fn default_panel_size() -> Vec2 {
    Vec2::new(200.0, 120.0)
}

/// A screen-space text label. `text` starting with `@` is a localization key.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct UiLabel {
    /// Literal text, or `@key` to look up in the locale tables.
    #[serde(default)]
    pub text: String,
    /// Which window point the label attaches to.
    #[serde(default)]
    pub anchor: UiAnchor,
    /// Pixel offset from the anchor point (+y down).
    #[serde(default)]
    pub offset: Vec2,
    /// Font size in pixels.
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// Text color (RGBA 0..=1).
    #[serde(default = "default_color")]
    pub color: Vec4,
    /// Hidden labels are skipped entirely.
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl Default for UiLabel {
    fn default() -> Self {
        Self {
            text: String::new(),
            anchor: UiAnchor::default(),
            offset: Vec2::ZERO,
            font_size: default_font_size(),
            color: default_color(),
            visible: true,
        }
    }
}

/// A screen-space colored panel (background box with optional border).
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct UiPanel {
    /// Which window point the panel attaches to.
    #[serde(default)]
    pub anchor: UiAnchor,
    /// Pixel offset from the anchor point (+y down).
    #[serde(default)]
    pub offset: Vec2,
    /// Panel size in pixels.
    #[serde(default = "default_panel_size")]
    pub size: Vec2,
    /// Fill color (RGBA 0..=1).
    #[serde(default = "default_panel_background")]
    pub background: Vec4,
    /// Border color (RGBA 0..=1).
    #[serde(default = "default_color")]
    pub border: Vec4,
    /// Border thickness in pixels; `0.0` = no border.
    #[serde(default = "default_border_width")]
    pub border_width: f32,
    /// Hidden panels are skipped entirely.
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl Default for UiPanel {
    fn default() -> Self {
        Self {
            anchor: UiAnchor::default(),
            offset: Vec2::ZERO,
            size: default_panel_size(),
            background: default_panel_background(),
            border: default_color(),
            border_width: default_border_width(),
            visible: true,
        }
    }
}

/// A screen-space clickable button. Presses surface as `UiButtonPressed`
/// events (engine_core) carrying `id`.
#[derive(Debug, Clone, Serialize, Deserialize, DeriveComponentMeta)]
pub struct UiButton {
    /// Button label; `@key` localizes like `UiLabel.text`.
    #[serde(default)]
    pub text: String,
    /// Event id games match on when the button is pressed.
    #[serde(default)]
    pub id: String,
    /// Which window point the button attaches to.
    #[serde(default)]
    pub anchor: UiAnchor,
    /// Pixel offset from the anchor point (+y down).
    #[serde(default)]
    pub offset: Vec2,
    /// Button size in pixels.
    #[serde(default = "default_button_size")]
    pub size: Vec2,
    /// Hidden buttons are skipped entirely (and can't be clicked).
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl Default for UiButton {
    fn default() -> Self {
        Self {
            text: String::new(),
            id: String::new(),
            anchor: UiAnchor::default(),
            offset: Vec2::ZERO,
            size: default_button_size(),
            visible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_anchored_pos_matrix() {
        let window = Vec2::new(800.0, 600.0);
        let size = Vec2::new(100.0, 50.0);
        let at = |anchor, offset| resolve_anchored_pos(anchor, offset, size, window);

        // The rect's matching corner/edge lands on the window's.
        assert_eq!(at(UiAnchor::TopLeft, Vec2::ZERO), Vec2::new(0.0, 0.0));
        assert_eq!(at(UiAnchor::TopCenter, Vec2::ZERO), Vec2::new(350.0, 0.0));
        assert_eq!(at(UiAnchor::TopRight, Vec2::ZERO), Vec2::new(700.0, 0.0));
        assert_eq!(at(UiAnchor::CenterLeft, Vec2::ZERO), Vec2::new(0.0, 275.0));
        assert_eq!(at(UiAnchor::Center, Vec2::ZERO), Vec2::new(350.0, 275.0));
        assert_eq!(at(UiAnchor::CenterRight, Vec2::ZERO), Vec2::new(700.0, 275.0));
        assert_eq!(at(UiAnchor::BottomLeft, Vec2::ZERO), Vec2::new(0.0, 550.0));
        assert_eq!(at(UiAnchor::BottomCenter, Vec2::ZERO), Vec2::new(350.0, 550.0));
        assert_eq!(at(UiAnchor::BottomRight, Vec2::ZERO), Vec2::new(700.0, 550.0));
        // The offset shifts the result (+y down).
        assert_eq!(at(UiAnchor::TopRight, Vec2::new(-10.0, 20.0)), Vec2::new(690.0, 20.0));
    }

    #[test]
    fn test_anchor_cycle_order_round_trips_through_index_and_labels() {
        // The editor's cycle row steps through ALL and shows label(); the
        // order and spelling are what an author sees.
        let labels: Vec<&str> = UiAnchor::ALL.iter().map(UiAnchor::label).collect();
        assert_eq!(
            labels,
            [
                "TopLeft", "TopCenter", "TopRight", "CenterLeft", "Center", "CenterRight",
                "BottomLeft", "BottomCenter", "BottomRight",
            ]
        );
        for (index, anchor) in UiAnchor::ALL.iter().enumerate() {
            assert_eq!(anchor.index(), index, "{} must map back to its slot", anchor.label());
        }
    }

    #[test]
    fn test_serde_defaults_fill_missing_fields() -> Result<(), serde_json::Error> {
        // Old/hand-written scene data with only some fields must deserialize,
        // and an omitted `visible` must yield elements that RENDER: a false
        // default would silently hide every hand-written label and button.
        let label: UiLabel = serde_json::from_str(r#"{"text": "@hud.score"}"#)?;
        assert_eq!(label.text, "@hud.score");
        assert_eq!(label.anchor, UiAnchor::TopLeft);
        assert_eq!(label.font_size, 16.0);
        assert!(label.visible);

        let button: UiButton = serde_json::from_str(r#"{"id": "play"}"#)?;
        assert_eq!(button.id, "play");
        assert_eq!(button.size, Vec2::new(120.0, 32.0));
        assert!(button.visible);

        let panel: UiPanel = serde_json::from_str("{}")?;
        assert!(panel.border_width > 0.0);
        assert!(panel.visible);
        Ok(())
    }
}
