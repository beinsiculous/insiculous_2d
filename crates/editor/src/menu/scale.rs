//! The UI Scale submenu, populated from the current preference and display.

use crate::editor_preferences::{resolve_ui_scale, UiScale, UI_SCALE_PERCENTAGES};
use super::{MenuBar, MenuItem};

impl MenuBar {
    /// Refresh the scale choices without replacing the open dropdown path. The display
    /// percentage stays live even while an explicit percentage overrides it.
    pub fn sync_ui_scale(&mut self, preference: UiScale, display_scale: f32) {
        let Some(view) = self.menus.iter_mut().find(|menu| menu.title == "View") else { return; };
        let display_percent = resolve_ui_scale(&UiScale::FollowDisplay, display_scale) * 100.0;
        let mut items = vec![MenuItem::action_with_shortcut(format!("Follow the display ({display_percent:.0} %)"), "Ctrl+0")
            .with_checked(preference == UiScale::FollowDisplay)];
        items.extend(UI_SCALE_PERCENTAGES.into_iter().map(|percent| {
            MenuItem::action(format!("{percent} %")).with_checked(preference == UiScale::Percent(percent))
        }));
        items.extend([
            MenuItem::separator(),
            MenuItem::action_with_shortcut("Larger text", "Ctrl++"),
            MenuItem::action_with_shortcut("Smaller text", "Ctrl+-"),
        ]);
        if let Some(MenuItem::Submenu { items: current, .. }) = view.items.iter_mut()
            .find(|item| matches!(item, MenuItem::Submenu { label, .. } if label == "UI Scale"))
        {
            *current = items;
        } else {
            view.items.push(MenuItem::submenu("UI Scale", items));
        }
    }
}

pub(super) fn preference_for_label(label: &str) -> Option<UiScale> {
    if label.starts_with("Follow the display (") { return Some(UiScale::FollowDisplay); }
    UI_SCALE_PERCENTAGES.into_iter().find(|percent| label == format!("{percent} %")).map(UiScale::Percent)
}
