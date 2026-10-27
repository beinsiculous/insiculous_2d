//! The one rule the panels that hold the keyboard — the composer, the asset browser and
//! the console — settle by at frame start: where this frame's press landed for a panel,
//! read from raw mouse state before any chrome handles it.
//!
//! A press of either button inside the panel, its header included, is the panel's. A
//! press elsewhere in the dock is not — and a narrow-mode overlay is elsewhere even where
//! it covers the panel, since the overlay's own blocking rect makes the panel's widgets
//! inert there but cannot reach a rule that reads the pointer. A press on the menu bar's
//! dropdown, the context menu or under a dialog is chrome's, and moves nothing.

use editor::PanelId;
use engine_core::Game;

use super::EditorGame;

/// Where this frame's press landed for one panel that can hold the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelPress {
    /// A press of either button inside the panel, its header included
    Inside,
    /// A press elsewhere in the dock, a narrow-mode overlay over the panel included
    Elsewhere,
    /// No press, a press outside the dock, or one chrome owns
    Neither,
}

impl<G: Game> EditorGame<G> {
    /// Whether a confirm dialog is up: it owns the frame's presses.
    pub(super) fn dialog_up(&self) -> bool {
        self.scene_confirm.pending_action.is_some() || self.stop_confirm.pending
    }

    /// Where this frame's press landed for `panel`; `None` while the panel is hidden or
    /// collapsed.
    pub(super) fn panel_press(&self, ui: &ui::UIContext, panel: PanelId) -> Option<PanelPress> {
        let dock = &self.editor.dock_area;
        let content = dock.get_panel(panel).filter(|shown| shown.visible && !shown.collapsed)?.content_bounds();
        let chrome_owns_press = self.editor.menu_bar.is_open() || self.editor.context_menu.is_open() || self.dialog_up();
        if !(ui.mouse_just_pressed() || ui.mouse_right_just_pressed()) || chrome_owns_press {
            return Some(PanelPress::Neither);
        }
        let mouse = ui.mouse_pos();
        // The dock keeps naming a panel hidden while it was the overlay, as long as it stays
        // narrow, and draws nothing there: only a shown overlay covers anything.
        let on_overlay = dock
            .narrow_overlay()
            .filter(|&overlay| overlay != panel)
            .and_then(|overlay| dock.get_panel(overlay))
            .filter(|overlay| overlay.visible)
            .is_some_and(|overlay| with_header(overlay.expanded_content_bounds()).contains(mouse));
        Some(if with_header(content).contains(mouse) && !on_overlay {
            PanelPress::Inside
        } else if dock.bounds().contains(mouse) {
            PanelPress::Elsewhere
        } else {
            PanelPress::Neither
        })
    }
}

/// A panel's whole rect from its content's: the header sits above the content.
fn with_header(content: ui::Rect) -> ui::Rect {
    let header = editor::layout::HEADER_HEIGHT;
    ui::Rect::new(content.x, content.y - header, content.width, content.height + header)
}
