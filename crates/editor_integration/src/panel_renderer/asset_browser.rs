//! Asset browser panel: thumbnail grid of the project's assets with
//! click-to-assign and drag-and-drop onto the scene/inspector.
//!
//! The pure parts (fs scan, entry state, aspect fit) live in
//! `editor::asset_browser`; this file owns the AssetManager interaction
//! (lazy thumbnail loads) and the panel drawing.

use std::path::Path;

use glam::Vec2;

use editor::{fit_rect, scan_assets, AssetKind, CommandHistory, DragPayload, EditorContext};
use engine_core::contexts::GameContext;
use renderer::texture::TextureHandle;

use crate::entity_ops;

/// Thumbnail tile edge in pixels.
const TILE_SIZE: f32 = 72.0;
/// Vertical space under each tile for the filename label.
const TILE_LABEL_HEIGHT: f32 = 16.0;
/// Gap between tiles.
const TILE_GAP: f32 = 10.0;
/// Panel content padding.
const PADDING: f32 = editor::layout::PADDING;
/// Header row height (Rescan button + counts).
const HEADER_HEIGHT: f32 = 26.0;
/// Cap on texture loads per frame so a big folder doesn't hitch one frame.
const MAX_THUMBNAIL_LOADS_PER_FRAME: usize = 4;

/// Grid slot rect for tile `index` in a grid `columns` wide starting at
/// `origin`, scrolled up by `scroll` pixels.
pub(crate) fn tile_rect(index: usize, columns: usize, origin: Vec2, scroll: f32) -> common::Rect {
    let col = index % columns.max(1);
    let row = index / columns.max(1);
    common::Rect::new(
        origin.x + col as f32 * (TILE_SIZE + TILE_GAP),
        origin.y + row as f32 * (TILE_SIZE + TILE_LABEL_HEIGHT + TILE_GAP) - scroll,
        TILE_SIZE,
        TILE_SIZE,
    )
}

/// Render the asset browser panel content.
pub(super) fn render_asset_browser(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    // ── Scan (first open or Rescan click) ───────────────────────────
    let rescan_bounds = ui::Rect::new(bounds.x + PADDING, bounds.y + 2.0, 70.0, 20.0);
    let rescan_clicked = ctx.ui.button("asset_rescan", "Rescan", rescan_bounds);
    if !editor.asset_browser.scanned || rescan_clicked {
        let entries = scan_assets(Path::new(ctx.assets.base_path()));
        editor.asset_browser.apply_scan(entries);
    }

    let count_label = format!("{} assets", editor.asset_browser.entries.len());
    ctx.ui.label_styled(
        &count_label,
        Vec2::new(rescan_bounds.x + rescan_bounds.width + 10.0, bounds.y + 16.0),
        editor.theme.text_muted,
        editor.theme.fonts.small,
    );

    // ── Lazy thumbnail loading (bounded per frame) ──────────────────
    let mut loads = 0;
    for entry in editor.asset_browser.entries.iter_mut() {
        if loads >= MAX_THUMBNAIL_LOADS_PER_FRAME {
            break;
        }
        if entry.kind == AssetKind::Image && entry.texture_handle.is_none() && !entry.load_failed {
            match ctx.assets.load_texture(&entry.relative_path) {
                Ok(handle) => entry.texture_handle = Some(handle.id),
                Err(e) => {
                    entry.load_failed = true;
                    log::warn!("Asset browser: failed to load '{}': {e}", entry.relative_path);
                }
            }
            loads += 1;
        }
    }

    // ── Scroll (shared ScrollState) ────────────────────────────────
    let grid_origin = Vec2::new(bounds.x + PADDING, bounds.y + HEADER_HEIGHT + PADDING);
    let columns = (((bounds.width - PADDING * 2.0) / (TILE_SIZE + TILE_GAP)) as usize).max(1);
    let rows = editor.asset_browser.entries.len().div_ceil(columns);
    let content_height = rows as f32 * (TILE_SIZE + TILE_LABEL_HEIGHT + TILE_GAP);
    let viewport_height = bounds.height - HEADER_HEIGHT - PADDING;
    // The grid's height is known up front (entry count), so record it
    // BEFORE consuming the wheel — this panel clamps lag-free.
    editor.asset_browser.scroll.end_frame(content_height, viewport_height);
    let scroll = editor.asset_browser.scroll.begin_frame(
        bounds,
        ctx.ui.mouse_pos(),
        ctx.ui.scroll_delta(),
        viewport_height,
    );

    // ── Tiles ───────────────────────────────────────────────────────
    let is_playing = editor.is_playing();
    let mouse_pos = ctx.ui.mouse_pos();
    let mut assign: Option<(u32, String)> = None;

    for i in 0..editor.asset_browser.entries.len() {
        let slot = tile_rect(i, columns, grid_origin, scroll);
        // Cull tiles fully outside the panel (the clip rect trims partials)
        if slot.y + TILE_SIZE + TILE_LABEL_HEIGHT < bounds.y || slot.y > bounds.y + bounds.height {
            continue;
        }

        let entry = &editor.asset_browser.entries[i];
        let slot_ui = ui::Rect::new(slot.x, slot.y, slot.width, slot.height);

        // Tile background + content
        ctx.ui.rect_rounded(slot_ui, editor.theme.surface_3, 4.0);
        match (entry.kind, entry.texture_handle) {
            (AssetKind::Image, Some(handle)) => {
                let (w, h) = ctx
                    .assets
                    .get_texture(TextureHandle { id: handle })
                    .map(|t| (t.width, t.height))
                    .unwrap_or((1, 1));
                let img = fit_rect(w, h, common::Rect::new(slot.x + 3.0, slot.y + 3.0, slot.width - 6.0, slot.height - 6.0));
                ctx.ui.image(
                    ui::Rect::new(img.x, img.y, img.width, img.height),
                    handle,
                    ui::Color::WHITE,
                );
            }
            (AssetKind::Image, None) => {
                let color = if entry.load_failed { editor.theme.error_red } else { editor.theme.text_muted };
                ctx.ui.label_in_bounds_styled(
                    if entry.load_failed { "!" } else { "…" },
                    slot_ui,
                    ui::TextAlign::Center,
                    color,
                    editor.theme.fonts.heading,
                    0.0,
                );
            }
            (AssetKind::Scene, _) => {
                // Simple scene glyph: an accent page outline with a fold tick
                let page = ui::Rect::new(slot.x + 20.0, slot.y + 12.0, slot.width - 40.0, slot.height - 24.0);
                ctx.ui.rect_border(page, editor.theme.accent_cyan, 1.5, 2.0);
                ctx.ui.rect(
                    ui::Rect::new(page.x + page.width - 12.0, page.y, 12.0, 2.0),
                    editor.theme.accent_cyan,
                );
            }
        }

        // Filename label under the tile (clipped by the panel rect)
        ctx.ui.label_in_bounds_styled(
            &entry.name,
            ui::Rect::new(slot.x, slot.y + TILE_SIZE, TILE_SIZE, TILE_LABEL_HEIGHT),
            ui::TextAlign::Center,
            editor.theme.text_secondary,
            editor.theme.fonts.small,
            0.0,
        );

        if is_playing {
            continue;
        }

        // Interaction: press arms a drag (images only), plain click assigns
        let result = ctx.ui.interact(ui::WidgetId::from_str_index("asset_tile", i), slot_ui, true);
        let hovered = slot_ui.contains(mouse_pos);
        if hovered {
            ctx.ui.rect_border(slot_ui, editor.theme.hover_fill, 1.5, 4.0);
        }

        if let (AssetKind::Image, Some(handle)) = (entry.kind, entry.texture_handle) {
            if result.state == ui::WidgetState::Active && ctx.ui.mouse_just_pressed() {
                editor.drag_drop.arm(
                    DragPayload::Texture { handle, path: entry.relative_path.clone() },
                    mouse_pos,
                );
            }
            if result.clicked && !editor.drag_drop.suppresses_click() {
                assign = Some((handle, entry.relative_path.clone()));
            }
        }
    }

    // Click-to-assign: set the selected entity's sprite texture
    if let Some((handle, path)) = assign {
        match editor.selection.primary() {
            Some(entity) if entity_ops::assign_sprite_texture(ctx.world, entity, handle, command_history) => {
                editor.status_bar.show_message(format!("Assigned {path}"));
            }
            Some(_) => {
                editor.status_bar.show_message("Select an entity with a Sprite to assign textures");
            }
            None => {
                editor.status_bar.show_message("Select an entity first (or drag onto the scene)");
            }
        }
    }
}

/// Draw the drag ghost (a translucent thumbnail following the cursor) while
/// a texture drag is in flight. The overlay's blocking rect also makes
/// widgets and viewport picking under the cursor inert for the frame.
pub(crate) fn render_drag_ghost(editor: &mut EditorContext, ctx: &mut GameContext) {
    let Some(DragPayload::Texture { handle, .. }) = editor.drag_drop.dragging_payload() else {
        return;
    };
    let handle = *handle;
    let mouse = ctx.ui.mouse_pos();
    let ghost = ui::Rect::new(mouse.x - 24.0, mouse.y - 24.0, 48.0, 48.0);
    // DragGhost band: the ghost rides above even an open dropdown.
    ctx.ui.begin_overlay_in(ui::UiLayer::DragGhost, ghost);
    ctx.ui.image(ghost, handle, ui::Color::new(1.0, 1.0, 1.0, 0.8));
    ctx.ui.end_overlay();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_rect_grid_layout() {
        let origin = Vec2::new(10.0, 40.0);
        // 3 columns: index 0 top-left, index 3 wraps to row 1
        let first = tile_rect(0, 3, origin, 0.0);
        assert_eq!((first.x, first.y), (10.0, 40.0));

        let fourth = tile_rect(3, 3, origin, 0.0);
        assert_eq!(fourth.x, 10.0, "wraps to column 0");
        assert!(fourth.y > first.y, "second row is below the first");

        // Scroll moves tiles up
        let scrolled = tile_rect(0, 3, origin, 25.0);
        assert_eq!(scrolled.y, 15.0);

        // Zero columns must not divide by zero
        let degenerate = tile_rect(2, 0, origin, 0.0);
        assert!(degenerate.y > origin.y);
    }
}
