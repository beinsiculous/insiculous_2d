//! The asset browser's keyboard at render time: the grid's columns recorded for Up and
//! Down, the selected tile scrolled wholly into view after a step and its path put on the
//! status bar as a click puts it, the focus ring on it while the browser holds the
//! keyboard, and the context menu a key asked for, opened at the tile. What each key does
//! is the host's (`editor_game/asset_browser_keys.rs`).

use glam::Vec2;

use editor::EditorContext;

use super::{TILE_GAP, TILE_LABEL_HEIGHT, TILE_SIZE};

/// How far the focus ring stands outside the tile, as the text fields' ring does.
const FOCUS_RING_OFFSET: f32 = 2.0;

/// Record the grid's shape and, after a step, scroll the selected tile wholly into the
/// `viewport_height` the grid shows and select it as a click does, its path on the status
/// bar. Runs between the scroll's `end_frame` and its `begin_frame`, which clamps the
/// offset set here.
pub(super) fn follow_grid(editor: &mut EditorContext, columns: usize, viewport_height: f32) {
    let state = &mut editor.asset_browser;
    state.columns = columns;
    if !std::mem::take(&mut state.reveal_selected) {
        return;
    }
    if let Some(index) = state.selected {
        let row_top = (index / columns.max(1)) as f32 * (TILE_SIZE + TILE_LABEL_HEIGHT + TILE_GAP);
        let row_bottom = row_top + TILE_SIZE + TILE_LABEL_HEIGHT;
        let offset = reveal_offset(row_top, row_bottom, state.scroll.offset(), viewport_height);
        state.scroll.scroll_to(offset);
        super::select_tile(editor, index);
    }
}

/// The scroll offset that shows the span `top..bottom` wholly, moving as little as it can.
fn reveal_offset(top: f32, bottom: f32, offset: f32, viewport_height: f32) -> f32 {
    if top < offset {
        top
    } else if bottom > offset + viewport_height {
        bottom - viewport_height
    } else {
        offset
    }
}

/// While the browser holds the keyboard, ring the selected tile — `slot_of` is where a
/// tile is drawn — and open the context menu a key asked for, under the tile.
pub(super) fn mark_selected_tile(
    editor: &mut EditorContext,
    ui: &mut ui::UIContext,
    slot_of: impl Fn(usize) -> common::Rect,
) {
    let state = &mut editor.asset_browser;
    let Some(index) = state.selected.filter(|_| state.holds_keyboard) else {
        state.menu_requested = false;
        return;
    };
    let slot = slot_of(index);
    ui.rect_border(slot.expand(FOCUS_RING_OFFSET), editor.theme.focus_ring, 2.0, 4.0 + FOCUS_RING_OFFSET);
    if std::mem::take(&mut editor.asset_browser.menu_requested) {
        super::tile::open_tile_menu(editor, index, Vec2::new(slot.x, slot.y + slot.height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use editor::{AssetEntry, AssetKind};

    const SLOT: common::Rect = common::Rect::new(40.0, 40.0, TILE_SIZE, TILE_SIZE);

    /// The focus rings drawn by one frame of [`mark_selected_tile`].
    fn rings_drawn(editor: &mut EditorContext) -> Vec<common::Rect> {
        let mut ui = ui::UIContext::new();
        ui.begin_frame(&input::InputHandler::new(), Vec2::new(800.0, 600.0));
        mark_selected_tile(editor, &mut ui, |_| SLOT);
        ui.end_frame();
        let ring = editor.theme.focus_ring;
        ui.draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                ui::DrawCommand::RectBorder { bounds, color, .. } if *color == ring => Some(*bounds),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_the_selected_tile_wears_the_focus_ring_only_while_the_browser_holds_the_keyboard() {
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(vec![AssetEntry {
            name: "hero.png".to_string(),
            relative_path: "sprites/hero.png".to_string(),
            kind: AssetKind::Image,
            texture_handle: None,
            load_failed: false,
        }]);
        editor.asset_browser.selected = Some(0);
        assert!(rings_drawn(&mut editor).is_empty(), "a selected tile without the keyboard has no ring");

        editor.asset_browser.holds_keyboard = true;
        editor.asset_browser.menu_requested = true;
        assert_eq!(rings_drawn(&mut editor), vec![SLOT.expand(FOCUS_RING_OFFSET)]);
        assert_eq!(editor.context_menu.labels(), vec!["Open with the system default", "Open containing folder"]);
        assert!(!editor.asset_browser.menu_requested, "the request is answered once");
    }

    #[test]
    fn test_a_step_puts_the_selected_tiles_path_on_the_status_bar_as_a_click_does() {
        let mut editor = EditorContext::new();
        let tile = |name: &str| AssetEntry {
            name: name.to_string(),
            relative_path: format!("sprites/{name}"),
            kind: AssetKind::Image,
            texture_handle: None,
            load_failed: false,
        };
        editor.asset_browser.apply_scan(vec![tile("hero.png"), tile("enemy.png")]);
        editor.asset_browser.selected = Some(0);
        editor.asset_browser.step_selection(editor::TileStep::Right);

        follow_grid(&mut editor, 2, 200.0);

        assert_eq!(editor.status_bar.message(), Some("sprites/enemy.png"));
    }

    #[test]
    fn test_a_step_scrolls_the_selected_row_wholly_into_view_and_no_further() {
        let row_height = TILE_SIZE + TILE_LABEL_HEIGHT;
        assert_eq!(reveal_offset(0.0, row_height, 50.0, 200.0), 0.0, "a row above the view scrolls up to its top");
        assert_eq!(reveal_offset(300.0, 300.0 + row_height, 0.0, 200.0), 300.0 + row_height - 200.0, "a row below scrolls up just enough");
        assert_eq!(reveal_offset(100.0, 100.0 + row_height, 50.0, 200.0), 50.0, "a row in view moves nothing");
    }
}
