//! One asset tile's interaction: the press that arms a drag of what the tile holds, the
//! click that selects it, the hover border and the full-path tooltip.

use glam::Vec2;

use editor::{AssetKind, DragDropState, DragPayload, EditorTheme};

/// What one tile did this frame.
pub(super) struct TileInteraction {
    /// A press-and-release that did not turn into a drag.
    pub(super) clicked: bool,
}

/// Press arms a drag (images, .rhai scripts and sounds); a plain click selects the tile.
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
    }

    TileInteraction {
        clicked: result.clicked && !drag_drop.suppresses_click(),
    }
}
