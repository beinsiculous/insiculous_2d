//! View toggles: the four authoring overlays and aids an editor user turns
//! on and off while working (Grid, Colliders, Game Frame, Snap to Grid).
//!
//! Defined together so the View menu and the toolbar strip read and toggle
//! the same state, and the user's choices persist across editor sessions
//! through [`crate::EditorPreferences`].

use glam::Vec2;
use serde::{Deserialize, Serialize};
use ui::{Color, Rect, UIContext, WidgetState};

use crate::menu::MenuBar;
use crate::theme::EditorTheme;

/// The four view toggles as an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewToggle {
    Grid,
    Colliders,
    GameFrame,
    Snap,
}

impl ViewToggle {
    /// Every view toggle, in menu/group order.
    pub const ALL: [Self; 4] = [
        Self::Grid,
        Self::Colliders,
        Self::GameFrame,
        Self::Snap,
    ];

    /// Label as shown in the View menu.
    pub fn menu_label(self) -> &'static str {
        match self {
            Self::Grid => "Toggle Grid",
            Self::Colliders => "Toggle Colliders",
            Self::GameFrame => "Toggle Game Frame",
            Self::Snap => "Snap to Grid",
        }
    }

    /// Short name for tooltips and accessibility.
    pub fn name(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Colliders => "Colliders",
            Self::GameFrame => "Game Frame",
            Self::Snap => "Snap",
        }
    }

    /// Single character glyph drawn in the toggle button.
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Grid => "#",
            Self::Colliders => "O",
            Self::GameFrame => "[]",
            Self::Snap => "%",
        }
    }

    /// Default keyboard shortcut, if bound.
    pub fn shortcut(self) -> Option<&'static str> {
        match self {
            Self::Grid => Some("G"),
            Self::Colliders => Some("C"),
            Self::GameFrame => None,
            Self::Snap => Some("S"),
        }
    }

    /// Corresponding editor action.
    pub fn action(self) -> crate::EditorAction {
        match self {
            Self::Grid => crate::EditorAction::ToggleGrid,
            Self::Colliders => crate::EditorAction::ToggleColliders,
            Self::GameFrame => crate::EditorAction::ToggleGameFrame,
            Self::Snap => crate::EditorAction::ToggleSnap,
        }
    }
}

/// The state of each view toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewToggles {
    /// Grid overlay visibility
    #[serde(rename = "grid_visible")]
    pub grid: bool,

    /// Collider outline overlay visibility
    #[serde(rename = "colliders_visible")]
    pub colliders: bool,

    /// Game-frame camera rect overlay visibility
    #[serde(rename = "game_frame_visible")]
    pub game_frame: bool,

    /// Grid snapping active
    #[serde(rename = "snap_to_grid")]
    pub snap: bool,
}

impl Default for ViewToggles {
    fn default() -> Self {
        Self {
            grid: true,
            colliders: true,
            game_frame: true,
            snap: false,
        }
    }
}

impl ViewToggles {
    /// Check whether a given toggle is on.
    pub fn is_on(&self, toggle: ViewToggle) -> bool {
        match toggle {
            ViewToggle::Grid => self.grid,
            ViewToggle::Colliders => self.colliders,
            ViewToggle::GameFrame => self.game_frame,
            ViewToggle::Snap => self.snap,
        }
    }

    /// Set a toggle's state explicitly.
    pub fn set(&mut self, toggle: ViewToggle, on: bool) {
        match toggle {
            ViewToggle::Grid => self.grid = on,
            ViewToggle::Colliders => self.colliders = on,
            ViewToggle::GameFrame => self.game_frame = on,
            ViewToggle::Snap => self.snap = on,
        }
    }

    /// Invert a toggle's state and return the new value.
    pub fn toggle(&mut self, toggle: ViewToggle) -> bool {
        let new_val = !self.is_on(toggle);
        self.set(toggle, new_val);
        new_val
    }

    /// Synchronize the View menu's check marks with these toggles.
    pub fn sync_menu(&self, menu_bar: &mut MenuBar) {
        for toggle in ViewToggle::ALL {
            menu_bar.set_checked("View", toggle.menu_label(), self.is_on(toggle));
        }
    }
}

/// Actions produced by the right-hand strip controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewGroupAction {
    /// A view toggle was clicked.
    Toggle(ViewToggle),
    /// Reset layout was clicked.
    ResetLayout,
}

/// Width of each toggle button in the group.
const TOGGLE_BUTTON_SIZE: f32 = 24.0;
/// Gap between items in the group.
const ITEM_GAP: f32 = 4.0;
/// Extra gap before the Reset Layout button.
const RESET_GAP: f32 = 8.0;

/// Total width of the strip's right view-controls group:
/// 4 toggles (24px) at 4px gaps (3 * 4 = 12px), an 8px gap, and Reset Layout (24px).
/// 4 * 24 + 12 + 8 + 24 = 140px.
pub const VIEW_GROUP_WIDTH: f32 = 4.0 * TOGGLE_BUTTON_SIZE + 3.0 * ITEM_GAP + RESET_GAP + TOGGLE_BUTTON_SIZE;

/// Render the view toggles group (the 4 toggles + Reset Layout button) at `origin`.
///
/// Must be called inside the strip's scope (`toolbar_strip::begin`/`end`).
pub fn render_group(
    ui: &mut UIContext,
    theme: &EditorTheme,
    origin: Vec2,
    toggles: &ViewToggles,
) -> Option<ViewGroupAction> {
    let mut picked = None;
    let mut current_x = origin.x;

    for toggle in ViewToggle::ALL {
        let bounds = Rect::new(current_x, origin.y, TOGGLE_BUTTON_SIZE, TOGGLE_BUTTON_SIZE);
        let id = format!("view_toggle_{}", toggle.name());
        let active = toggles.is_on(toggle);

        let result = ui.interact(id.as_str(), bounds, true);
        if active {
            ui.rect_rounded(bounds, theme.toolbar_active, 3.0);
            ui.panel_styled(bounds, Color::TRANSPARENT, theme.accent_blue, 1.0);
        } else if result.state == WidgetState::Hovered || result.dragging {
            ui.rect_rounded(bounds, theme.hover_fill, 3.0);
        }

        let glyph = toggle.glyph();
        let text_color = if active { theme.text_primary } else { theme.text_secondary };
        ui.label_in_bounds_styled(
            glyph,
            bounds,
            ui::TextAlign::Center,
            text_color,
            theme.fonts.small,
            0.0,
        );

        if result.clicked {
            picked = Some(ViewGroupAction::Toggle(toggle));
        }

        current_x += TOGGLE_BUTTON_SIZE + ITEM_GAP;
    }

    current_x += RESET_GAP - ITEM_GAP;

    // Reset Layout button
    let reset_bounds = Rect::new(current_x, origin.y, TOGGLE_BUTTON_SIZE, TOGGLE_BUTTON_SIZE);
    let reset_result = ui.interact("strip_reset_layout", reset_bounds, true);
    if reset_result.state == WidgetState::Hovered || reset_result.dragging {
        ui.rect_rounded(reset_bounds, theme.hover_fill, 3.0);
    }
    ui.label_in_bounds_styled(
        "R",
        reset_bounds,
        ui::TextAlign::Center,
        theme.text_secondary,
        theme.fonts.small,
        0.0,
    );
    if reset_result.clicked {
        picked = Some(ViewGroupAction::ResetLayout);
    }

    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_toggles_defaults() {
        let toggles = ViewToggles::default();
        assert!(toggles.grid);
        assert!(toggles.colliders);
        assert!(toggles.game_frame);
        assert!(!toggles.snap);
    }

    #[test]
    fn test_view_toggles_toggle_and_set() {
        let mut toggles = ViewToggles::default();
        assert!(toggles.is_on(ViewToggle::Grid));
        assert!(!toggles.toggle(ViewToggle::Grid));
        assert!(!toggles.grid);
        assert!(!toggles.is_on(ViewToggle::Grid));

        toggles.set(ViewToggle::Snap, true);
        assert!(toggles.snap);
        assert!(toggles.is_on(ViewToggle::Snap));
    }
}
