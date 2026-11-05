//! Dropdown geometry and rendering for menu bars and context menus.

use glam::Vec2;
use ui::{Rect, UIContext};
use crate::{theme::EditorTheme, layout::{PADDING, ROW_HEIGHT}};
use super::{Menu, MenuBar, MenuItem, DROPDOWN_WIDTH};

impl MenuBar {
    pub(super) fn render_open_dropdown(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> Option<String> {
        let open_index = self.open_menu?;
        self.menus[open_index].dropdown_width = Self::measured_width(ui, &self.menus[open_index], theme);
        let mut levels = self.dropdown_levels(ui, theme);
        let mouse = ui.mouse_pos();
        let pressed = ui.mouse_just_pressed() || ui.mouse_right_just_pressed();
        if pressed && Self::should_close_on_press(mouse, levels[0].1, self.menus[open_index].bounds)
            && !levels.iter().skip(1).any(|(_, bounds)| bounds.contains(mouse))
        {
            self.close();
            return None;
        }

        // The topmost containing menu owns the hover. In a narrow window a child overlaps
        // its parent: walking the parent's covered rows would close that child immediately.
        // A press frame never opens a child here: a child laid over its parent would take
        // this same press on its first row, so a press on a submenu row opens it on release.
        if let Some(depth) = levels.iter().rposition(|(_, bounds)| bounds.contains(mouse)) {
            let (menu, bounds) = &levels[depth];
            if let Some(index) = menu.items.iter().enumerate().find_map(|(index, item)| {
                (Self::dropdown_item_bounds(*bounds, index).contains(mouse)
                    && matches!(item, MenuItem::Submenu { .. }))
                    .then_some(index)
            }) {
                if self.submenu_path.get(depth) != Some(&index) && self.last_pointer != Some(mouse) && !pressed {
                    self.submenu_path.truncate(depth);
                    self.submenu_path.push(index);
                    self.keyboard_cursor = None;
                    levels = self.dropdown_levels(ui, theme);
                }
            } else if let Some(index) = menu.items.iter().enumerate().find_map(|(index, item)| {
                (Self::dropdown_item_bounds(*bounds, index).contains(mouse)
                    && matches!(item, MenuItem::Action { enabled: true, .. }))
                    .then_some(index)
            }) {
                // Keyboard navigation must work with a stationary pointer left over the
                // parent. Only a pointer move or press returns navigation to that menu.
                if self.last_pointer != Some(mouse) || pressed {
                    self.submenu_path.truncate(depth);
                    self.keyboard_cursor = Some(index);
                    levels.truncate(depth + 1);
                }
            }
        }
        self.last_pointer = Some(mouse);

        let mut clicked = None;
        for (depth, (menu, bounds)) in levels.iter().enumerate() {
            let covered = levels.iter().skip(depth + 1).any(|(_, child)| child.contains(mouse));
            ui.begin_overlay(*bounds);
            ui.push_clip_rect(*bounds);
            ui.panel_styled(*bounds, theme.surface_4, theme.popup_border, 1.0);
            for (index, item) in menu.items.iter().enumerate() {
                let row = Self::dropdown_item_bounds(*bounds, index);
                if let Some(label) = Self::render_item(ui, menu, index, row, theme, !covered) {
                    clicked = Some(label);
                }
                if depth + 1 == levels.len() && self.keyboard_cursor == Some(index) {
                    ui.rect_border(row, theme.focus_ring, 2.0, 2.0);
                }
                if !covered && matches!(item, MenuItem::Submenu { .. })
                    && row.contains(mouse) && ui.mouse_just_released()
                {
                    self.submenu_path.truncate(depth);
                    self.submenu_path.push(index);
                    self.keyboard_cursor = None;
                }
            }
            ui.pop_clip_rect();
            ui.end_overlay();
        }
        if clicked.is_some() { self.close(); }
        clicked
    }

    fn dropdown_levels(&self, ui: &UIContext, theme: &EditorTheme) -> Vec<(Menu, Rect)> {
        let Some(open_index) = self.open_menu else { return Vec::new(); };
        let root = self.menus[open_index].clone();
        let root_bounds = inside_window(Self::dropdown_bounds(&root, root.bounds), ui.window_size());
        let mut levels = vec![(root, root_bounds)];
        for &index in &self.submenu_path {
            let Some((parent, bounds)) = levels.last() else { break; };
            let Some(MenuItem::Submenu { items, .. }) = parent.items.get(index) else { break; };
            let mut child = Menu::new(format!("{}/{}", parent.title, index)).with_items(items.clone());
            child.dropdown_width = Self::measured_width(ui, &child, theme);
            let row = Self::dropdown_item_bounds(*bounds, index);
            let child_bounds = submenu_bounds(*bounds, row, &child, ui.window_size());
            levels.push((child, child_bounds));
        }
        levels
    }

    /// Compute the dropdown bounds for a menu anchored below its title — or below any
    /// rect: the context menu anchors at a point.
    pub(crate) fn dropdown_bounds(menu: &Menu, anchor: Rect) -> Rect {
        let dropdown_height = menu.items.len() as f32 * ROW_HEIGHT + 8.0;
        Rect::new(
            anchor.x,
            anchor.y + anchor.height,
            menu.dropdown_width,
            dropdown_height,
        )
    }

    /// The width at which every item's centred label clears its shortcut, drawn at the
    /// row's right edge: twice the shortcut and its gaps, either side of the label; a label
    /// with no shortcut also clears the checked marker at the row's left edge.
    pub(crate) fn measured_width(ui: &UIContext, menu: &Menu, theme: &EditorTheme) -> f32 {
        menu.items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { label, shortcut: Some(shortcut), .. } => {
                    let label_width = ui.measure_text(label).x;
                    let shortcut_width = ui.measure_text_styled(shortcut, theme.fonts.small).x;
                    Some(label_width + 2.0 * (shortcut_width + 2.0 * PADDING) + 8.0)
                }
                MenuItem::Action { label, shortcut: None, .. } => Some(ui.measure_text(label).x + 4.0 * PADDING + 8.0),
                MenuItem::Submenu { label, .. } => Some(ui.measure_text(&format!("{label} >")).x + 2.0 * PADDING),
                _ => None,
            })
            .fold(DROPDOWN_WIDTH, f32::max)
    }

    /// Whether a mouse press at `mouse` should close the open dropdown.
    ///
    /// Presses inside the dropdown are item interactions; presses on the open
    /// menu's own title must NOT close here because the title's click fires on
    /// mouse *release* — closing on press would make the release re-toggle the
    /// menu open (close/reopen flicker). Pressing a different title closes
    /// here, then that title's release opens its menu.
    pub(super) fn should_close_on_press(mouse: Vec2, dropdown: Rect, title: Rect) -> bool {
        !dropdown.contains(mouse) && !title.contains(mouse)
    }

    /// The row a dropdown draws item `index` in; a separator takes a row too.
    pub(crate) fn dropdown_item_bounds(dropdown_bounds: Rect, index: usize) -> Rect {
        Rect::new(
            dropdown_bounds.x + 4.0,
            dropdown_bounds.y + 4.0 + index as f32 * ROW_HEIGHT,
            dropdown_bounds.width - 8.0,
            ROW_HEIGHT,
        )
    }

    /// Render one flat dropdown for the context-menu caller, inside its existing scope.
    pub(crate) fn render_dropdown_static(ui: &mut UIContext, menu: &Menu, dropdown_bounds: Rect, theme: &EditorTheme) -> Option<String> {
        ui.panel_styled(dropdown_bounds, theme.surface_4, theme.popup_border, 1.0);
        for index in 0..menu.items.len() {
            let bounds = Self::dropdown_item_bounds(dropdown_bounds, index);
            if let Some(label) = Self::render_item(ui, menu, index, bounds, theme, true) {
                return Some(label);
            }
        }
        None
    }

    fn render_item(ui: &mut UIContext, menu: &Menu, index: usize, bounds: Rect, theme: &EditorTheme, interactive: bool) -> Option<String> {
        match &menu.items[index] {
            MenuItem::Action { label, shortcut, enabled, checked } => {
                let id = format!("menu_item_{}_{}", menu.title, index);
                let clicked = ui.button_styled(id.as_str(), label, bounds, *enabled && interactive);
                if *checked {
                    ui.rect(Rect::new(bounds.x + PADDING / 2.0, bounds.center().y - 3.0, 6.0, 6.0), theme.accent_cyan);
                }
                if let Some(shortcut) = shortcut {
                    let shortcut_bounds = Rect::new(bounds.x, bounds.y, bounds.width - PADDING, bounds.height);
                    ui.label_in_bounds_styled(shortcut, shortcut_bounds, ui::TextAlign::Right, theme.shortcut_hint, theme.fonts.small, 0.0);
                }
                if clicked { return Some(label.clone()); }
            }
            MenuItem::Separator => {
                let middle = bounds.center().y;
                ui.line(Vec2::new(bounds.x + 4.0, middle), Vec2::new(bounds.right() - 4.0, middle), theme.menu_separator, 1.0);
            }
            MenuItem::Submenu { label, .. } => {
                let id = format!("menu_submenu_{}_{}", menu.title, index);
                ui.button_styled(id.as_str(), &format!("{label} >"), bounds, interactive);
            }
        }
        None
    }
}

fn inside_window(bounds: Rect, window: Vec2) -> Rect {
    let width = bounds.width.min(window.x).max(0.0);
    let height = bounds.height.min(window.y).max(0.0);
    Rect::new(bounds.x.clamp(0.0, (window.x - width).max(0.0)),
        bounds.y.clamp(0.0, (window.y - height).max(0.0)), width, height)
}

fn submenu_bounds(parent: Rect, row: Rect, child: &Menu, window: Vec2) -> Rect {
    let width = child.dropdown_width.min(window.x);
    let x = if parent.right() + width <= window.x {
        parent.right()
    } else if parent.x >= width {
        parent.x - width
    } else {
        window.x - width
    };
    inside_window(Rect::new(x, row.y - 4.0, width, child.items.len() as f32 * ROW_HEIGHT + 8.0), window)
}
