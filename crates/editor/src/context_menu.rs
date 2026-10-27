//! The editor's context menu: a short list of actions over one thing, opened at a
//! point by a right press or from the keyboard, drawn by the menu bar's own dropdown on
//! [`ui::UiLayer::Modal`], the colour editor's band: a narrow-mode panel shown as an
//! overlay draws on `Floating` after the menu, and would cover it and take its presses.
//! While it is open it blocks what is beneath it, and a press outside it closes it and
//! reaches nothing.
//!
//! Its keys reach it through the host's key route: Up and Down walk the enabled items,
//! Enter chooses, Escape closes, and every other key is swallowed while it is open.

use std::path::Path;

use glam::Vec2;
use ui::{Rect, UIContext};
use winit::keyboard::KeyCode;

use crate::asset_browser::{open_file_request, open_folder_request};
use crate::external_tools::{ExternalToolsConfig, FileType, OpenRequest, DESKTOP_ONLY_HINT};
use crate::menu::{Menu, MenuBar, MenuItem};
use crate::theme::EditorTheme;

/// The stroke of the ring on the keyboard's item.
const CURSOR_RING_WIDTH: f32 = 2.0;
/// What the menu adds to the menu bar's width on each side, so a label clears the ring:
/// that rule leaves a label only the few pixels the ring's stroke covers.
const CURSOR_RING_ROOM: f32 = 2.0 * CURSOR_RING_WIDTH;

/// What choosing an item does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuAction {
    /// File the request onto the editor's open requests
    Open(OpenRequest),
}

/// One row of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuItem {
    pub label: String,
    pub enabled: bool,
    pub action: ContextMenuAction,
}

/// A key the open menu answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuKey {
    Up,
    Down,
    Choose,
    Close,
}

impl ContextMenuKey {
    /// The menu's key for `key`, if it has one; every other key goes on.
    pub fn of(key: KeyCode) -> Option<Self> {
        match key {
            KeyCode::ArrowUp => Some(Self::Up),
            KeyCode::ArrowDown => Some(Self::Down),
            KeyCode::Enter | KeyCode::NumpadEnter => Some(Self::Choose),
            KeyCode::Escape => Some(Self::Close),
            _ => None,
        }
    }
}

/// The context menu's state: closed, or open at a point over its items.
#[derive(Debug, Default)]
pub struct ContextMenu {
    open: Option<OpenMenu>,
}

#[derive(Debug)]
struct OpenMenu {
    anchor: Vec2,
    items: Vec<ContextMenuItem>,
    /// A line under the items saying why they are disabled
    note: Option<String>,
    /// The item Enter chooses: the first enabled one when the menu opens
    cursor: Option<usize>,
}

impl ContextMenu {
    /// Open the menu with its top-left corner at `anchor`, replacing any menu open.
    pub fn open(&mut self, anchor: Vec2, items: Vec<ContextMenuItem>, note: Option<String>) {
        let cursor = items.iter().position(|item| item.enabled);
        self.open = Some(OpenMenu { anchor, items, note, cursor });
    }

    /// Open the menu over one project file, by its path relative to the asset base:
    /// Open in the tool the file resolves to, and Open containing folder. On the web no
    /// program can be launched, so both draw disabled over the desktop hint.
    pub fn open_for_file(&mut self, anchor: Vec2, relative_path: &str, config: &ExternalToolsConfig, desktop: bool) {
        let file_type = FileType::of(Path::new(relative_path), &config.daw_project_extensions);
        let open_label = file_type.map_or_else(
            || "Open (no tool opens this type)".to_string(),
            |file_type| config.tool_name(file_type).open_item(),
        );
        let items = vec![
            ContextMenuItem {
                label: open_label,
                enabled: desktop && file_type.is_some(),
                action: ContextMenuAction::Open(open_file_request(relative_path)),
            },
            ContextMenuItem {
                label: "Open containing folder".to_string(),
                enabled: desktop,
                action: ContextMenuAction::Open(open_folder_request(relative_path)),
            },
        ];
        self.open(anchor, items, (!desktop).then(|| sentence(DESKTOP_ONLY_HINT)));
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn close(&mut self) {
        self.open = None;
    }

    /// The labels of the open menu's items, in order.
    pub fn labels(&self) -> Vec<&str> {
        self.open.iter().flat_map(|open| open.items.iter().map(|item| item.label.as_str())).collect()
    }

    /// Answer one of the menu's keys; a chosen item closes the menu and returns its action.
    pub fn handle_key(&mut self, key: ContextMenuKey) -> Option<ContextMenuAction> {
        let open = self.open.as_mut()?;
        match key {
            ContextMenuKey::Up => open.cursor = open.step(-1),
            ContextMenuKey::Down => open.cursor = open.step(1),
            ContextMenuKey::Choose => {
                let action = open.cursor.map(|index| open.items[index].action.clone());
                self.open = None;
                return action;
            }
            ContextMenuKey::Close => self.open = None,
        }
        None
    }

    /// Draw the open menu, before every widget it may cover so its blocking rect reaches
    /// them, and return the action of an item clicked this frame. A press outside closes
    /// the menu and is swallowed whole.
    pub fn render(&mut self, ui: &mut UIContext, theme: &EditorTheme) -> Option<ContextMenuAction> {
        let (menu, bounds) = self.layout(ui, theme)?;

        let pressed = ui.mouse_just_pressed() || ui.mouse_right_just_pressed();
        if pressed && !bounds.contains(ui.mouse_pos()) {
            self.open = None;
            let window = ui.window_size();
            ui.begin_overlay_in(ui::UiLayer::Modal, Rect::new(0.0, 0.0, window.x, window.y));
            ui.end_overlay();
            return None;
        }

        let open = self.open.as_ref()?;
        ui.begin_overlay_in(ui::UiLayer::Modal, bounds);
        let clicked = MenuBar::render_dropdown_static(ui, &menu, bounds, theme);
        if let Some(cursor) = open.cursor {
            let ring = MenuBar::dropdown_item_bounds(bounds, cursor);
            ui.rect_border(ring, theme.focus_ring, CURSOR_RING_WIDTH, 2.0);
        }
        ui.end_overlay();

        let action = clicked.and_then(|label| {
            open.items.iter().find(|item| item.enabled && item.label == label).map(|item| item.action.clone())
        });
        if action.is_some() {
            self.open = None;
        }
        action
    }

    /// Where the open menu draws: at its anchor, moved inside the window where it would
    /// run past an edge.
    pub fn bounds(&self, ui: &UIContext, theme: &EditorTheme) -> Option<Rect> {
        self.layout(ui, theme).map(|(_, bounds)| bounds)
    }

    fn layout(&self, ui: &UIContext, theme: &EditorTheme) -> Option<(Menu, Rect)> {
        let open = self.open.as_ref()?;
        let mut menu = open.as_menu();
        menu.dropdown_width = MenuBar::measured_width(ui, &menu, theme) + 2.0 * CURSOR_RING_ROOM;
        let anchor = Rect::new(open.anchor.x, open.anchor.y, 0.0, 0.0);
        let bounds = inside_window(MenuBar::dropdown_bounds(&menu, anchor), ui.window_size());
        Some((menu, bounds))
    }
}

impl OpenMenu {
    /// The menu bar's model of these items; the note follows a separator as a disabled row.
    fn as_menu(&self) -> Menu {
        let mut items: Vec<MenuItem> = self
            .items
            .iter()
            .map(|item| MenuItem::action(item.label.clone()).with_enabled(item.enabled))
            .collect();
        if let Some(note) = &self.note {
            items.push(MenuItem::separator());
            items.push(MenuItem::action(note.clone()).with_enabled(false));
        }
        Menu::new("context").with_items(items)
    }

    /// The next enabled item `direction` away from the cursor, wrapping.
    fn step(&self, direction: isize) -> Option<usize> {
        let count = self.items.len() as isize;
        let start = self.cursor.map_or(if direction > 0 { -1 } else { count }, |index| index as isize);
        (1..=count)
            .map(|distance| (start + direction * distance).rem_euclid(count) as usize)
            .find(|&index| self.items[index].enabled)
    }
}

/// `bounds` moved up and left until it lies inside the window, where it fits.
fn inside_window(bounds: Rect, window_size: Vec2) -> Rect {
    let x = bounds.x.min(window_size.x - bounds.width).max(0.0);
    let y = bounds.y.min(window_size.y - bounds.height).max(0.0);
    Rect::new(x, y, bounds.width, bounds.height)
}

/// `text` with its first letter capitalized, as a line of its own.
fn sentence(text: &str) -> String {
    let mut characters = text.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;
