//! View toggles: the four authoring overlays and aids an editor user turns
//! on and off while working (Grid, Colliders, Game Frame, Snap to Grid).
//!
//! Defined together so the View menu and the toolbar strip read and toggle
//! the same state, and the user's choices persist across editor sessions
//! through [`crate::EditorPreferences`].

use glam::Vec2;
use serde::{Deserialize, Serialize};
use ui::{Rect, UIContext};

use crate::icons::Icon;
use crate::menu::MenuBar;
use crate::theme::EditorTheme;
use crate::toolbar::{draw_segment, segment_button, tooltip_text, ButtonLook, SEGMENT_DIVIDER_GAP, TOOL_BUTTON_SIZE};

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

    /// The icon the toggle's button carries.
    pub fn icon(self) -> Icon {
        match self {
            Self::Grid => Icon::Grid,
            Self::Colliders => Icon::Colliders,
            Self::GameFrame => Icon::GameFrame,
            Self::Snap => Icon::Snap,
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

    /// One sentence on what the toggle shows or does — the tail of its
    /// [`tooltip`](Self::tooltip), after the name and the key.
    ///
    /// The game frame's names no size. The rect is the world the game shows
    /// at the size its config declares, so a resized window or a preview
    /// box of another size shows more or less of the world than it does.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Grid => "Draw the authoring grid over the scene.",
            Self::Colliders => "Outline every collider in the scene.",
            Self::GameFrame => {
                "Outline the game's configured size; a resized window shows more or less."
            }
            Self::Snap => "Snap gizmo drags to the grid; holding Ctrl snaps one drag.",
        }
    }

    /// The toggle button's tooltip: name, key where one is bound, and hint,
    /// in [`tooltip_text`]'s shape — the button itself carries only its icon.
    pub fn tooltip(self) -> String {
        tooltip_text(self.name(), self.shortcut(), self.hint())
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

/// Gap before the Reset Layout button, inside the segment — wider than the
/// segment's divider gap so Reset Layout reads as an action beside the
/// toggles rather than one of them. Its divider sits at its centre.
const RESET_GAP: f32 = 8.0;

/// Total width of the strip's right view-controls group: one segment of
/// four toggles at the divider gap, the wider gap, and Reset Layout.
pub const VIEW_GROUP_WIDTH: f32 =
    5.0 * TOOL_BUTTON_SIZE + 3.0 * SEGMENT_DIVIDER_GAP + RESET_GAP;

/// Render the view toggles group (the 4 toggles + Reset Layout button) at `origin`,
/// as one segment with the strip's separator before it. A toggle that is on
/// is filled and ringed.
///
/// Must be called inside the strip's scope (`toolbar_strip::begin`/`end`).
pub fn render_group(
    ui: &mut UIContext,
    theme: &EditorTheme,
    origin: Vec2,
    toggles: &ViewToggles,
) -> Option<ViewGroupAction> {
    crate::toolbar_strip::draw_separator_before(ui, theme, origin);
    let stride = TOOL_BUTTON_SIZE + SEGMENT_DIVIDER_GAP;
    let toggle_bounds = |index: usize| {
        Rect::new(origin.x + index as f32 * stride, origin.y, TOOL_BUTTON_SIZE, TOOL_BUTTON_SIZE)
    };
    let reset_bounds = Rect::new(
        origin.x + VIEW_GROUP_WIDTH - TOOL_BUTTON_SIZE,
        origin.y,
        TOOL_BUTTON_SIZE,
        TOOL_BUTTON_SIZE,
    );
    let mut buttons: Vec<Rect> = (0..ViewToggle::ALL.len()).map(toggle_bounds).collect();
    buttons.push(reset_bounds);
    draw_segment(ui, theme, &buttons);

    let mut picked = None;
    for (index, toggle) in ViewToggle::ALL.into_iter().enumerate() {
        let id = format!("view_toggle_{}", toggle.name());
        let look = if toggles.is_on(toggle) { ButtonLook::On } else { ButtonLook::Plain };
        if segment_button(ui, theme, &id, toggle.icon(), toggle_bounds(index), look, &toggle.tooltip()) {
            picked = Some(ViewGroupAction::Toggle(toggle));
        }
    }

    let reset_tooltip = tooltip_text("Reset Layout", None, "Put every panel back where it started.");
    if segment_button(
        ui,
        theme,
        "strip_reset_layout",
        Icon::ResetLayout,
        reset_bounds,
        ButtonLook::Plain,
        &reset_tooltip,
    ) {
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

    /// A toggle's button carries an icon and no caption, so its tooltip is
    /// where the key that flips it is read. Held to `shortcut()`: a bound
    /// key is named in parentheses after the name, and a toggle without
    /// one advertises none.
    #[test]
    fn test_a_toggles_hint_advertises_the_key_that_flips_it() {
        for toggle in ViewToggle::ALL {
            let tooltip = toggle.tooltip();
            assert!(tooltip.starts_with(toggle.name()), "{toggle:?}'s tooltip leads with its name");
            match toggle.shortcut() {
                Some(key) => assert!(
                    tooltip.contains(&format!("({key})")),
                    "{toggle:?}'s tooltip does not name {key}"
                ),
                None => assert!(
                    !tooltip.contains('('),
                    "{toggle:?} has no bound key but its tooltip advertises one"
                ),
            }
        }
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
