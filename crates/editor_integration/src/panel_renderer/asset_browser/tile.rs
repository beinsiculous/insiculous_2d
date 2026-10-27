//! One asset tile's interaction: the press that arms a drag of what the tile holds, the
//! click that selects it, the double-click that opens it, the right press that opens its
//! context menu, the hover border and the full-path tooltip.

use common::clock::Instant;
use glam::Vec2;

use editor::{AssetKind, DragDropState, DragPayload, EditorContext, EditorTheme};

/// What one tile did this frame.
pub(super) struct TileInteraction {
    /// A press-and-release that did not turn into a drag.
    pub(super) clicked: bool,
    /// A left press landed on the tile this frame.
    pub(super) pressed: bool,
    /// A right press landed on the tile this frame.
    pub(super) right_pressed: bool,
}

/// What the grid's tiles did this frame, by tile index.
#[derive(Debug, Default)]
pub(super) struct TilePresses {
    pub(super) clicked: Option<usize>,
    pressed: Option<usize>,
    right_pressed: Option<usize>,
}

impl TilePresses {
    pub(super) fn record(&mut self, index: usize, tile: &TileInteraction) {
        if tile.clicked {
            self.clicked = Some(index);
        }
        if tile.pressed {
            self.pressed = Some(index);
        }
        if tile.right_pressed {
            self.right_pressed = Some(index);
        }
    }
}

/// Act on this frame's presses: a second left press on the same tile within the
/// double-click's bounds files its file's request, whatever the tile's kind; a right
/// press selects the tile, takes the keyboard and opens the context menu at the pointer.
/// A drag started since the last press forgets it.
pub(super) fn open_from_presses(editor: &mut EditorContext, presses: &TilePresses, mouse_pos: Vec2) {
    if editor.drag_drop.suppresses_click() {
        editor.asset_browser.last_press.forget();
    }
    if let Some(index) = presses.pressed {
        if let Some(request) = editor.asset_browser.press_tile(index, Instant::now(), mouse_pos) {
            editor.open_requests.push(request);
        }
    }
    if let Some(index) = presses.right_pressed {
        super::select_tile(editor, index);
        editor.asset_browser.holds_keyboard = true;
        open_tile_menu(editor, index, mouse_pos);
    }
}

/// Open the context menu over tile `index` with its corner at `anchor`; on the web its
/// items are disabled, since no program can be launched there.
pub(super) fn open_tile_menu(editor: &mut EditorContext, index: usize, anchor: Vec2) {
    if let Some(entry) = editor.asset_browser.entries.get(index) {
        let desktop = cfg!(not(target_arch = "wasm32"));
        editor.context_menu.open_for_file(anchor, &entry.relative_path, &editor.external_tools, desktop);
    }
}

/// Press arms a drag (images, .rhai scripts, sounds, songs and SoundFonts); a plain click
/// selects the tile.
pub(super) fn tile_interaction(
    ui: &mut ui::UIContext,
    drag_drop: &mut DragDropState,
    theme: &EditorTheme,
    entry: &editor::AssetEntry,
    index: usize,
    slot: common::Rect,
    mouse_pos: Vec2,
) -> TileInteraction {
    let slot_ui = ui::Rect::new(slot.x, slot.y, slot.width, slot.height);
    let result = ui.interact(ui::WidgetId::from_str_index("asset_tile", index), slot_ui, true);
    let hovered = slot_ui.contains(mouse_pos);
    if hovered {
        ui.rect_border(slot_ui, theme.hover_fill, 1.5, 4.0);
    }
    // The label under the tile is ellipsized to the tile's width, so resting
    // on it is how the full relative path is read.
    ui.tooltip(slot_ui, &entry.relative_path);

    if let (AssetKind::Image, Some(handle)) = (entry.kind, entry.texture_handle) {
        if result.state == ui::WidgetState::Active && ui.mouse_just_pressed() {
            drag_drop.arm(
                DragPayload::Texture { handle, path: entry.relative_path.clone() },
                mouse_pos,
            );
        }
    } else if entry.kind == AssetKind::Script
        && std::path::Path::new(&entry.relative_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rhai"))
        && result.state == ui::WidgetState::Active
        && ui.mouse_just_pressed()
    {
        drag_drop.arm(
            DragPayload::Script { path: entry.relative_path.clone() },
            mouse_pos,
        );
    } else if entry.kind == AssetKind::Sound
        && result.state == ui::WidgetState::Active
        && ui.mouse_just_pressed()
    {
        drag_drop.arm(DragPayload::Sound { path: entry.relative_path.clone() }, mouse_pos);
    } else if matches!(entry.kind, AssetKind::Song | AssetKind::SoundFont)
        && result.state == ui::WidgetState::Active
        && ui.mouse_just_pressed()
    {
        let path = entry.relative_path.clone();
        let payload = match entry.kind {
            AssetKind::Song => DragPayload::Song { path },
            _ => DragPayload::SoundFont { path },
        };
        drag_drop.arm(payload, mouse_pos);
    }

    TileInteraction {
        clicked: result.clicked && !drag_drop.suppresses_click(),
        pressed: result.state == ui::WidgetState::Active && ui.mouse_just_pressed(),
        right_pressed: result.state == ui::WidgetState::Hovered && ui.mouse_right_just_pressed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use editor::AssetEntry;
    use input::prelude::MouseButton;

    const SLOT: common::Rect = common::Rect::new(40.0, 40.0, 72.0, 72.0);

    fn editor_with_a_song_tile() -> EditorContext {
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(vec![AssetEntry {
            name: "theme.mid".to_string(),
            relative_path: "music/theme.mid".to_string(),
            kind: AssetKind::Song,
            texture_handle: None,
            load_failed: false,
        }]);
        editor
    }

    /// One frame of the grid's one tile, its presses acted on as the panel does.
    fn tile_frame(editor: &mut EditorContext, ui: &mut ui::UIContext, input: &input::InputHandler) {
        let pointer = input.mouse().position();
        let pointer = Vec2::new(pointer.x, pointer.y);
        ui.begin_frame_dt(input, Vec2::new(800.0, 600.0), 0.016);
        editor.drag_drop.begin_frame(pointer, ui.mouse_down(), ui.mouse_just_released());
        let entry = editor.asset_browser.entries[0].clone();
        let tile = tile_interaction(ui, &mut editor.drag_drop, &editor.theme, &entry, 0, SLOT, pointer);
        let mut presses = TilePresses::default();
        presses.record(0, &tile);
        open_from_presses(editor, &presses, pointer);
        ui.end_frame();
    }

    fn click(editor: &mut EditorContext, ui: &mut ui::UIContext, input: &mut input::InputHandler, button: MouseButton) {
        input.update();
        input.mouse_mut().update_position(SLOT.center().x, SLOT.center().y);
        input.mouse_mut().handle_button_press(button);
        tile_frame(editor, ui, input);
        input.update();
        input.mouse_mut().handle_button_release(button);
        tile_frame(editor, ui, input);
    }

    #[test]
    fn test_a_double_click_on_a_tile_files_one_request_for_its_path_whatever_its_kind() {
        let mut editor = editor_with_a_song_tile();
        let (mut ui, mut input) = (ui::UIContext::new(), input::InputHandler::new());
        click(&mut editor, &mut ui, &mut input, MouseButton::Left);
        assert!(editor.open_requests.is_empty(), "one click selects and opens nothing");
        click(&mut editor, &mut ui, &mut input, MouseButton::Left);
        assert_eq!(editor.open_requests, vec![editor::open_file_request("music/theme.mid")]);
    }

    #[test]
    fn test_a_right_press_on_a_tile_selects_it_takes_the_keyboard_and_opens_the_menu_there() {
        let mut editor = editor_with_a_song_tile();
        let (mut ui, mut input) = (ui::UIContext::new(), input::InputHandler::new());
        click(&mut editor, &mut ui, &mut input, MouseButton::Right);

        assert_eq!(editor.asset_browser.selected, Some(0));
        assert!(editor.asset_browser.holds_keyboard);
        assert_eq!(editor.context_menu.labels(), vec!["Open with the system default", "Open containing folder"]);
        let bounds = editor.context_menu.bounds(&ui, &editor.theme).map(|bounds| (bounds.x, bounds.y));
        assert_eq!(bounds, Some((SLOT.center().x, SLOT.center().y)), "the menu opens at the press");
        assert!(editor.open_requests.is_empty() && editor.drag_drop.dragging_payload().is_none(), "a right press arms no drag");
    }
}
