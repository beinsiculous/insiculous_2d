//! Menu bar and dropdown menu system for the editor.
//!
//! Provides a standard menu bar with File, Edit, View menus and
//! support for keyboard shortcuts.

use glam::Vec2;
use ui::{Rect, UIContext};

use crate::theme::EditorTheme;

mod actions;
pub use actions::action_for_menu_label;

/// Menu dropdown layout constants
const DROPDOWN_ITEM_HEIGHT: f32 = 24.0;
const DROPDOWN_ITEM_PADDING: f32 = 8.0;
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
}

impl Menu {
    /// Create a new menu.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            open: false,
            bounds: Rect::default(),
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
            ]),
        );

        // View menu — Scene View can't be hidden and Console has no panel
        // implementation yet, so both stay disabled.
        bar.add_menu(
            Menu::new("View").with_items(vec![
                MenuItem::action("Scene View").with_enabled(false),
                MenuItem::action("Inspector"),
                MenuItem::action("Hierarchy"),
                MenuItem::action("Asset Browser"),
                MenuItem::action("Console").with_enabled(false),
                MenuItem::separator(),
                MenuItem::action_with_shortcut("Toggle Grid", "G"),
                MenuItem::action_with_shortcut("Toggle Colliders", "C"),
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

        bar
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
        ui.rect(self.bounds, theme.bg_header);

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
            self.open_menu = if self.open_menu == Some(index) {
                None
            } else {
                Some(index)
            };
        }
    }

    /// Phase 4 — render the dropdown for the open menu (if any) and handle
    /// item clicks. A click on an item — or a press outside the dropdown —
    /// closes the menu; an item click returns its label.
    fn render_open_dropdown(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> Option<String> {
        let open_index = self.open_menu?;
        let menu = &self.menus[open_index];
        let dropdown_bounds = Self::dropdown_bounds(menu, menu.bounds);

        if ui.mouse_just_pressed()
            && Self::should_close_on_press(ui.mouse_pos(), dropdown_bounds, menu.bounds)
        {
            self.open_menu = None;
            return None;
        }

        // Overlay: render on top of panels/toolbar and swallow clicks so
        // they don't pass through to widgets underneath.
        ui.begin_overlay(dropdown_bounds);
        let clicked = Self::render_dropdown_static(ui, menu, dropdown_bounds, theme);
        ui.end_overlay();

        if clicked.is_some() {
            self.open_menu = None;
        }
        clicked
    }

    /// Compute the dropdown bounds for a menu anchored below its title.
    fn dropdown_bounds(menu: &Menu, anchor: Rect) -> Rect {
        let dropdown_height = menu.items.len() as f32 * DROPDOWN_ITEM_HEIGHT + 8.0;
        Rect::new(
            anchor.x,
            anchor.y + anchor.height,
            DROPDOWN_WIDTH,
            dropdown_height,
        )
    }

    /// Whether a mouse press at `mouse` should close the open dropdown.
    ///
    /// Presses inside the dropdown are item interactions; presses on the open
    /// menu's own title must NOT close here because the title's click fires on
    /// mouse *release* — closing on press would make the release re-toggle the
    /// menu open (close/reopen flicker). Pressing a different title closes
    /// here, then that title's release opens its menu.
    fn should_close_on_press(mouse: Vec2, dropdown: Rect, title: Rect) -> bool {
        !dropdown.contains(mouse) && !title.contains(mouse)
    }

    /// Render a dropdown menu (static method to avoid borrow issues).
    fn render_dropdown_static(ui: &mut UIContext, menu: &Menu, dropdown_bounds: Rect, theme: &EditorTheme) -> Option<String> {
        // Floating surface + bounded border: a dropdown must
        // read as an object ABOVE the panel, not blend into it.
        ui.panel_styled(dropdown_bounds, theme.surface_4, theme.popup_border, 1.0);

        // Draw items
        let mut y = dropdown_bounds.y + 4.0;
        for (i, item) in menu.items.iter().enumerate() {
            match item {
                MenuItem::Action { label, shortcut, enabled, checked } => {
                    let item_bounds = Rect::new(
                        dropdown_bounds.x + 4.0,
                        y,
                        dropdown_bounds.width - 8.0,
                        DROPDOWN_ITEM_HEIGHT,
                    );

                    let id = format!("menu_item_{}_{}", menu.title, i);

                    if ui.button_styled(id.as_str(), label, item_bounds, *enabled) {
                        return Some(label.clone());
                    }

                    // Check indicator: a small accent square on the item's
                    // left edge (a primitive, so no font-coverage risk).
                    if *checked {
                        let check_size = 6.0;
                        ui.rect(
                            Rect::new(
                                item_bounds.x + DROPDOWN_ITEM_PADDING / 2.0,
                                item_bounds.center().y - check_size / 2.0,
                                check_size,
                                check_size,
                            ),
                            theme.accent_cyan,
                        );
                    }

                    // Draw shortcut if present
                    if let Some(shortcut) = shortcut {
                        let shortcut_pos = Vec2::new(
                            item_bounds.x + item_bounds.width - DROPDOWN_ITEM_PADDING - shortcut.len() as f32 * 6.0,
                            item_bounds.center().y,
                        );
                        ui.label_styled(shortcut, shortcut_pos, theme.shortcut_hint, theme.fonts.small);
                    }

                    y += DROPDOWN_ITEM_HEIGHT;
                }
                MenuItem::Separator => {
                    let sep_y = y + DROPDOWN_ITEM_HEIGHT / 2.0;
                    ui.line(
                        Vec2::new(dropdown_bounds.x + 8.0, sep_y),
                        Vec2::new(dropdown_bounds.x + dropdown_bounds.width - 8.0, sep_y),
                        theme.menu_separator,
                        1.0,
                    );
                    y += DROPDOWN_ITEM_HEIGHT;
                }
                MenuItem::Submenu { label, .. } => {
                    // For now, just render the label with an arrow indicator
                    let item_bounds = Rect::new(
                        dropdown_bounds.x + 4.0,
                        y,
                        dropdown_bounds.width - 8.0,
                        DROPDOWN_ITEM_HEIGHT,
                    );

                    let id = format!("menu_submenu_{}_{}", menu.title, i);
                    ui.button(id.as_str(), &format!("{} >", label), item_bounds);
                    y += DROPDOWN_ITEM_HEIGHT;
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests;
