//! Keyboard navigation for the open dropdown path.

use winit::keyboard::KeyCode;
use super::{MenuBar, MenuItem};

impl MenuBar {
    /// Close the menu and every child dropdown.
    pub fn close(&mut self) {
        self.open_menu = None;
        self.submenu_path.clear();
        self.keyboard_cursor = None;
    }

    fn active_items(&self) -> &[MenuItem] {
        let Some(index) = self.open_menu else { return &[]; };
        let mut items = self.menus[index].items.as_slice();
        for &index in &self.submenu_path {
            let Some(MenuItem::Submenu { items: children, .. }) = items.get(index) else { break; };
            items = children;
        }
        items
    }

    fn step_cursor(&self, forward: bool) -> Option<usize> {
        let items = self.active_items();
        let count = items.len();
        let start = self.keyboard_cursor.unwrap_or(if forward { count.saturating_sub(1) } else { 0 });
        (1..=count).map(|offset| {
            if forward { (start + offset) % count } else { (start + count - offset) % count }
        }).find(|&index| selectable(&items[index]))
    }

    /// Walk the deepest dropdown with Up/Down, enter children with Right/Enter,
    /// return with Left, and close the whole path with Escape. The host consumes
    /// every key while a menu is open; an action returns the same label as a click.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<String> {
        let open_index = self.open_menu?;
        match key {
            KeyCode::Escape => self.close(),
            KeyCode::ArrowDown => self.keyboard_cursor = self.step_cursor(true),
            KeyCode::ArrowUp => self.keyboard_cursor = self.step_cursor(false),
            KeyCode::ArrowLeft => {
                if let Some(index) = self.submenu_path.pop() {
                    self.keyboard_cursor = Some(index);
                } else {
                    self.open_menu = Some((open_index + self.menus.len() - 1) % self.menus.len());
                    self.keyboard_cursor = None;
                }
            }
            KeyCode::ArrowRight | KeyCode::Enter | KeyCode::NumpadEnter => {
                // Only a highlighted row answers: a menu opened by the mouse has none, and
                // Enter there must fire nothing (File's first row is New Scene).
                let selected = self.keyboard_cursor;
                let item = selected.and_then(|index| self.active_items().get(index)).cloned();
                match (selected, item) {
                    (Some(index), Some(MenuItem::Submenu { items, .. })) => {
                        self.submenu_path.push(index);
                        self.keyboard_cursor = items.iter().position(selectable);
                    }
                    (_, Some(MenuItem::Action { label, enabled: true, .. })) if key != KeyCode::ArrowRight => {
                        self.close();
                        return Some(label);
                    }
                    _ if key == KeyCode::ArrowRight && self.submenu_path.is_empty() => {
                        self.open_menu = Some((open_index + 1) % self.menus.len());
                        self.keyboard_cursor = None;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        None
    }
}

fn selectable(item: &MenuItem) -> bool {
    matches!(item, MenuItem::Action { enabled: true, .. } | MenuItem::Submenu { .. })
}
