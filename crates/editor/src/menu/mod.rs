//! Menu bar and dropdown menu system for the editor.
//!
//! Provides a standard menu bar with File, Edit, View menus and
//! support for keyboard shortcuts.

use ui::{Rect, UIContext};

use crate::theme::EditorTheme;

mod actions;
mod dropdown;
mod keyboard;
mod scale;
pub use actions::{action_for_menu_label, open_project_item, EXTERNAL_TOOLS_LABEL, OPEN_PROJECT_PREFIX};

/// Menu dropdown layout constants. The row height and the item padding are
/// the shared layout tokens; only the width is the menu's own, and the
/// narrowest it gets — an item whose label and shortcut need more widens it.
const DROPDOWN_WIDTH: f32 = 200.0;

/// A single menu item (can be an action or separator).
#[derive(Debug, Clone)]
pub enum MenuItem {
    /// A clickable action
    Action {
        /// Display label
        label: String,
        /// Keyboard shortcut hint (e.g., "Ctrl+S")
        shortcut: Option<String>,
        /// Whether the item is enabled
        enabled: bool,
        /// Whether the item shows a check indicator (toggle state)
        checked: bool,
    },
    /// A separator line
    Separator,
    /// A submenu
    Submenu {
        /// Display label
        label: String,
        /// Submenu items
        items: Vec<MenuItem>,
    },
}

impl MenuItem {
    /// Create a new action item.
    pub fn action(label: impl Into<String>) -> Self {
        MenuItem::Action {
            label: label.into(),
            shortcut: None,
            enabled: true,
            checked: false,
        }
    }

    /// Create a new action item with a shortcut.
    pub fn action_with_shortcut(label: impl Into<String>, shortcut: impl Into<String>) -> Self {
        MenuItem::Action {
            label: label.into(),
            shortcut: Some(shortcut.into()),
            enabled: true,
            checked: false,
        }
    }

    /// Create a separator.
    pub fn separator() -> Self {
        MenuItem::Separator
    }

    /// Create a submenu.
    pub fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        MenuItem::Submenu {
            label: label.into(),
            items,
        }
    }

    /// Set whether the item is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        if let MenuItem::Action { enabled: e, .. } = &mut self {
            *e = enabled;
        }
        self
    }

    /// Set whether the item shows a check indicator.
    pub fn with_checked(mut self, checked: bool) -> Self {
        if let MenuItem::Action { checked: c, .. } = &mut self {
            *c = checked;
        }
        self
    }

    /// Get the label for this item (if any).
    pub fn label(&self) -> Option<&str> {
        match self {
            MenuItem::Action { label, .. } => Some(label),
            MenuItem::Submenu { label, .. } => Some(label),
            MenuItem::Separator => None,
        }
    }
}

/// A dropdown menu containing menu items.
#[derive(Debug, Clone)]
pub struct Menu {
    /// Menu title
    pub title: String,
    /// Menu items
    pub items: Vec<MenuItem>,
    /// Whether the menu is currently open
    pub open: bool,
    /// Menu bounds (set during render)
    pub bounds: Rect,
    /// The dropdown's width, measured from its items when it opens.
    pub dropdown_width: f32,
}

impl Menu {
    /// Create a new menu.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            open: false,
            bounds: Rect::default(),
            dropdown_width: DROPDOWN_WIDTH,
        }
    }

    /// Add an item to the menu.
    pub fn add_item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items to the menu.
    pub fn with_items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self
    }
}

/// The main menu bar at the top of the editor.
#[derive(Debug, Clone)]
pub struct MenuBar {
    /// Menus in the menu bar
    menus: Vec<Menu>,
    /// Menu bar bounds
    bounds: Rect,
    /// Menu item spacing
    item_spacing: f32,
    /// Menu item padding
    item_padding: f32,
    /// Currently open menu index
    open_menu: Option<usize>,
    submenu_path: Vec<usize>,
    keyboard_cursor: Option<usize>,
    last_pointer: Option<glam::Vec2>,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuBar {
    /// Create a new empty menu bar.
    pub fn new() -> Self {
        Self {
            menus: Vec::new(),
            bounds: Rect::default(),
            item_spacing: 16.0,
            item_padding: 8.0,
            open_menu: None,
            submenu_path: Vec::new(),
            keyboard_cursor: None,
            last_pointer: None,
        }
    }

    /// Create a default editor menu bar with File, Edit, View menus.
    pub fn editor_default() -> Self {
        let mut bar = Self::new();

        // File menu
        bar.add_menu(
            Menu::new("File").with_items(vec![
                MenuItem::action_with_shortcut("New Scene", "Ctrl+N"),
                MenuItem::action_with_shortcut("Open Scene...", "Ctrl+O"),
                MenuItem::separator(),
                MenuItem::action_with_shortcut("Save", "Ctrl+S"),
                MenuItem::action_with_shortcut("Save As...", "Ctrl+Shift+S"),
                MenuItem::separator(),
                MenuItem::action("Open Song..."),
                MenuItem::action("Save Song"),
                MenuItem::action("Save Song As..."),
                MenuItem::action("Close Song"),
                MenuItem::separator(),
                MenuItem::action("Exit"),
            ]),
        );

        // Edit menu
        bar.add_menu(
            Menu::new("Edit").with_items(vec![
                MenuItem::action_with_shortcut("Undo", "Ctrl+Z"),
                MenuItem::action_with_shortcut("Redo", "Ctrl+Y"),
                MenuItem::separator(),
                MenuItem::action_with_shortcut("Cut", "Ctrl+X"),
                MenuItem::action_with_shortcut("Copy", "Ctrl+C"),
                MenuItem::action_with_shortcut("Paste", "Ctrl+V"),
                MenuItem::separator(),
                MenuItem::action_with_shortcut("Delete", "Del"),
                MenuItem::action_with_shortcut("Duplicate", "Ctrl+D"),
                MenuItem::separator(),
                MenuItem::action(open_project_item(crate::external_tools::ToolName::SystemDefault, true).0).with_enabled(false),
                MenuItem::action(EXTERNAL_TOOLS_LABEL),
            ]),
        );

        // View menu — Scene View can't be hidden, so it stays disabled.
        bar.add_menu(
            Menu::new("View").with_items(vec![
                MenuItem::action("Scene View").with_enabled(false),
                MenuItem::action("Inspector"),
                MenuItem::action("Hierarchy"),
                MenuItem::action("Asset Browser"),
                MenuItem::action("Conductor"),
                MenuItem::action("Console"),
                MenuItem::separator(),
                MenuItem::action_with_shortcut("Toggle Grid", "G"),
                MenuItem::action_with_shortcut("Toggle Colliders", "C"),
                MenuItem::action("Toggle Game Frame"),
                MenuItem::action_with_shortcut("Snap to Grid", "S"),
                MenuItem::separator(),
                MenuItem::action("Cycle Game Locale"),
                MenuItem::action("Reset Layout"),
            ]),
        );

        // Entity menu
        let mut entity_items = Vec::new();
        for archetype in crate::archetype::Archetype::ALL {
            if matches!(
                archetype,
                crate::archetype::Archetype::Sprite
                    | crate::archetype::Archetype::StaticBody
                    | crate::archetype::Archetype::UiLabel
            ) {
                entity_items.push(MenuItem::separator());
            }
            entity_items.push(MenuItem::action(archetype.menu_label()));
        }
        bar.add_menu(Menu::new("Entity").with_items(entity_items));

        bar.sync_ui_scale(crate::editor_preferences::UiScale::FollowDisplay, 1.0);
        bar
    }

    /// Whether a dropdown is open: a click on it is chrome, which leaves the Conductor panel's
    /// keyboard focus where it was.
    pub fn is_open(&self) -> bool {
        self.open_menu.is_some()
    }

    /// Add a menu to the menu bar.
    pub fn add_menu(&mut self, menu: Menu) {
        self.menus.push(menu);
    }

    /// Access all menus in the menu bar.
    pub fn menus(&self) -> &[Menu] {
        &self.menus
    }

    /// Set the check indicator on an action item, addressed by menu title
    /// and item label. Unknown titles/labels are ignored.
    pub fn set_checked(&mut self, menu_title: &str, label: &str, checked: bool) {
        let Some(menu) = self.menus.iter_mut().find(|m| m.title == menu_title) else {
            return;
        };
        for item in &mut menu.items {
            if let MenuItem::Action { label: l, checked: c, .. } = item {
                if l == label {
                    *c = checked;
                }
            }
        }
    }

    /// Relabel the action item whose label starts with `prefix` and set whether it is
    /// enabled: an item named after a setting, which a label lookup could not find again.
    pub fn set_action_by_prefix(&mut self, menu_title: &str, prefix: &str, label: String, enabled: bool) {
        let Some(menu) = self.menus.iter_mut().find(|menu| menu.title == menu_title) else {
            return;
        };
        for item in &mut menu.items {
            if let MenuItem::Action { label: current, enabled: current_enabled, .. } = item {
                if current.starts_with(prefix) {
                    *current = label;
                    *current_enabled = enabled;
                    return;
                }
            }
        }
    }

    /// Get the check indicator state of an action item, if it exists.
    pub fn is_checked(&self, menu_title: &str, label: &str) -> Option<bool> {
        let menu = self.menus.iter().find(|m| m.title == menu_title)?;
        menu.items.iter().find_map(|item| match item {
            MenuItem::Action { label: l, checked, .. } if l == label => Some(*checked),
            _ => None,
        })
    }

    /// Set the bounds for the menu bar.
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    /// Get the menu bar height.
    pub fn height(&self) -> f32 {
        24.0
    }

    /// Render the menu bar and handle interactions.
    ///
    /// Runs the three phases in order: layout (pure geometry), title bar
    /// (drawing + click detection), and the open dropdown.
    ///
    /// Returns the label of the clicked menu item, if any.
    pub fn render(&mut self, ui: &mut UIContext, window_width: f32, theme: &EditorTheme) -> Option<String> {
        self.layout_titles(window_width);
        let toggled = self.render_title_bar(ui, theme);
        self.apply_toggle(toggled);
        self.render_open_dropdown(ui, theme)
    }

    /// Phase 1 — compute the bar bounds and each menu title's bounds.
    /// Pure geometry: no drawing, no state transitions.
    fn layout_titles(&mut self, window_width: f32) {
        let height = self.height();
        self.bounds = Rect::new(0.0, 0.0, window_width, height);

        let mut x = self.item_padding;
        for menu in &mut self.menus {
            // Wider than the text for better readability
            let title_width = menu.title.len() as f32 * 10.0 + self.item_padding * 3.0;
            menu.bounds = Rect::new(x, 0.0, title_width, height);
            x += title_width + self.item_spacing;
        }
    }

    /// Phase 2 — draw the bar background and title buttons.
    /// Returns the index of a title that was clicked this frame, if any.
    fn render_title_bar(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> Option<usize> {
        ui.rect(self.bounds, theme.surface_2);

        let mut toggled = None;
        for (index, menu) in self.menus.iter().enumerate() {
            if self.open_menu == Some(index) {
                ui.rect(menu.bounds, theme.menu_open_highlight);
            }

            let id = format!("menu_{}", menu.title);
            if ui.button(id.as_str(), &menu.title, menu.bounds) {
                toggled = Some(index);
            }
        }
        toggled
    }

    /// Phase 3 — apply a title click: open the clicked menu, or close it if
    /// it was already open.
    fn apply_toggle(&mut self, toggled: Option<usize>) {
        if let Some(index) = toggled {
            self.submenu_path.clear();
            self.keyboard_cursor = None;
            self.open_menu = if self.open_menu == Some(index) {
                None
            } else {
                Some(index)
            };
        }
    }


}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod nested_tests;
