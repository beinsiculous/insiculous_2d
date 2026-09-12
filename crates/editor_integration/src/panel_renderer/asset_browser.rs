//! Asset browser panel: thumbnail grid of the project's assets. A click
//! selects a tile; assigning is a drag onto the scene or the inspector's
//! texture field, or the header's Assign button.
//!
//! The pure parts (fs scan, entry state, aspect fit) live in
//! `editor::asset_browser`; this file owns the AssetManager interaction
//! (lazy thumbnail loads) and the panel drawing.

use std::path::Path;

use glam::Vec2;

use editor::layout::PADDING;
use editor::{
    fit_rect, scan_assets, AssetKind, CommandHistory, DragDropState, DragPayload, EditorContext,
    EditorTheme,
};
use engine_core::contexts::GameContext;
use engine_core::AssetManager;
use renderer::texture::TextureHandle;

use crate::entity_ops;

/// Thumbnail tile edge in pixels.
const TILE_SIZE: f32 = 72.0;
/// Vertical space under each tile for the filename label.
const TILE_LABEL_HEIGHT: f32 = 16.0;
/// Gap between tiles: wider than [`editor::layout::GAP`], because pictures
/// need air around them to read as separate tiles rather than one sheet.
const TILE_GAP: f32 = 10.0;
/// Height of the panel's own header row: not the dock's
/// [`editor::layout::HEADER_HEIGHT`], because this row holds buttons and
/// counts rather than a title.
const HEADER_HEIGHT: f32 = 26.0;
/// Header button size, shared by Rescan and Assign.
const HEADER_BUTTON_SIZE: Vec2 = Vec2::new(70.0, 20.0);
/// Gap between the header's button and the label beside it: wider than the
/// shared [`editor::layout::GAP`] so a count reads as a caption, not as part
/// of the button.
const HEADER_GAP: f32 = 10.0;
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

/// Ensure assets have been scanned at least once; if not, refreshes assets and builds script catalog.
pub(crate) fn ensure_scanned(
    editor: &mut EditorContext,
    assets: &AssetManager,
    registry: &engine_core::scripting::ScriptRegistry,
) {
    if !editor.asset_browser.scanned {
        refresh_assets(editor, assets, registry);
    }
}

/// Rescan assets from filesystem, update asset browser entries, and rebuild script catalog.
pub(crate) fn refresh_assets(
    editor: &mut EditorContext,
    assets: &AssetManager,
    registry: &engine_core::scripting::ScriptRegistry,
) {
    let entries = scan_assets(Path::new(assets.base_path()));
    editor.asset_browser.apply_scan(entries);
    editor.script_catalog = super::script_catalog::build_script_catalog(
        &editor.asset_browser.entries,
        registry,
        assets.base_path(),
    );
}

/// Render the asset browser panel content.
pub(super) fn render_asset_browser(
    editor: &mut EditorContext,
    ctx: &mut GameContext,
    bounds: common::Rect,
    command_history: &mut CommandHistory,
) {
    let assign_clicked = render_header(
        editor,
        ctx.ui,
        ctx.assets,
        ctx.scripts.registry(),
        ctx.world,
        bounds,
    );
    load_pending_thumbnails(&mut editor.asset_browser.entries, ctx.assets);

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

    let is_playing = editor.is_playing();
    let mouse_pos = ctx.ui.mouse_pos();
    let mut clicked_tile: Option<usize> = None;

    for index in 0..editor.asset_browser.entries.len() {
        let slot = tile_rect(index, columns, grid_origin, scroll);
        // Cull tiles fully outside the panel (the clip rect trims partials)
        if slot.y + TILE_SIZE + TILE_LABEL_HEIGHT < bounds.y || slot.y > bounds.y + bounds.height {
            continue;
        }

        let selected = editor.asset_browser.selected == Some(index);
        let entry = &editor.asset_browser.entries[index];
        render_tile(ctx.ui, &editor.theme, ctx.assets, entry, slot, selected);

        if !is_playing {
            let tile = tile_interaction(
                ctx.ui, &mut editor.drag_drop, &editor.theme, entry, index, slot, mouse_pos,
            );
            if tile.clicked {
                clicked_tile = Some(index);
            }
        }
    }

    if let Some(index) = clicked_tile {
        select_tile(editor, index);
    }

    if assign_clicked {
        assign_selected_texture(editor, ctx.world, command_history);
    }
}

/// Select a tile and put its full relative path on the status bar — the
/// label under the tile is ellipsized, so the path is only readable in full
/// on the click that selects it and on the tile's tooltip. An error on the
/// bar stays: selecting a tile is not clearing a failed save.
fn select_tile(editor: &mut EditorContext, index: usize) {
    editor.asset_browser.selected = Some(index);
    if editor.status_bar.is_showing_error() {
        return;
    }
    if let Some(entry) = editor.asset_browser.entries.get(index) {
        let path = entry.relative_path.clone();
        editor.status_bar.show_message(path);
    }
}

/// Draw the header row and report whether Assign was clicked this frame.
fn render_header(
    editor: &mut EditorContext,
    ui: &mut ui::UIContext,
    assets: &AssetManager,
    registry: &engine_core::scripting::ScriptRegistry,
    world: &ecs::World,
    bounds: common::Rect,
) -> bool {
    let rescan_bounds = ui::Rect::new(
        bounds.x + PADDING,
        bounds.y + 2.0,
        HEADER_BUTTON_SIZE.x,
        HEADER_BUTTON_SIZE.y,
    );
    let rescan_clicked = ui.button("asset_rescan", "Rescan", rescan_bounds);
    if !editor.asset_browser.scanned {
        ensure_scanned(editor, assets, registry);
    } else if rescan_clicked {
        refresh_assets(editor, assets, registry);
    }

    let assign_bounds = ui::Rect::new(
        rescan_bounds.x + rescan_bounds.width + HEADER_GAP,
        rescan_bounds.y,
        HEADER_BUTTON_SIZE.x,
        HEADER_BUTTON_SIZE.y,
    );
    let assign_clicked = ui.button_styled(
        "asset_assign",
        "Assign",
        assign_bounds,
        !editor.is_playing() && can_assign_selection(editor, world),
    );

    let count_label = format!("{} assets", editor.asset_browser.entries.len());
    ui.label_styled(
        &count_label,
        Vec2::new(assign_bounds.x + assign_bounds.width + HEADER_GAP, bounds.y + 16.0),
        editor.theme.text_muted,
        editor.theme.fonts.small,
    );

    assign_clicked
}

/// Whether Assign has something to do: a loaded image tile is selected and
/// the primary selection is an entity that carries a Sprite.
fn can_assign_selection(editor: &EditorContext, world: &ecs::World) -> bool {
    let has_texture = editor
        .asset_browser
        .selected_entry()
        .is_some_and(|entry| entry.kind == AssetKind::Image && entry.texture_handle.is_some());
    has_texture
        && editor
            .selection
            .primary()
            .is_some_and(|entity| world.get::<ecs::Sprite>(entity).is_some())
}

fn load_pending_thumbnails(
    entries: &mut [editor::AssetEntry],
    assets: &mut AssetManager,
) {
    let mut loads = 0;
    for entry in entries.iter_mut() {
        if loads >= MAX_THUMBNAIL_LOADS_PER_FRAME {
            break;
        }
        if entry.kind == AssetKind::Image && entry.texture_handle.is_none() && !entry.load_failed {
            match assets.load_texture(&entry.relative_path) {
                Ok(handle) => entry.texture_handle = Some(handle.id),
                Err(error) => {
                    entry.load_failed = true;
                    log::warn!("Asset browser: failed to load '{}': {error}", entry.relative_path);
                }
            }
            loads += 1;
        }
    }
}

fn render_tile(
    ui: &mut ui::UIContext,
    theme: &editor::EditorTheme,
    assets: &AssetManager,
    entry: &editor::AssetEntry,
    slot: common::Rect,
    selected: bool,
) {
    let slot_ui = ui::Rect::new(slot.x, slot.y, slot.width, slot.height);

    // Tile background + content
    let background = if selected { theme.selection_fill } else { theme.surface_3 };
    ui.rect_rounded(slot_ui, background, 4.0);
    match (entry.kind, entry.texture_handle) {
        (AssetKind::Image, Some(handle)) => {
            let (width, height) = assets
                .get_texture(TextureHandle { id: handle })
                .map(|t| (t.width, t.height))
                .unwrap_or((1, 1));
            let img = fit_rect(width, height, common::Rect::new(slot.x + 3.0, slot.y + 3.0, slot.width - 6.0, slot.height - 6.0));
            ui.image(
                ui::Rect::new(img.x, img.y, img.width, img.height),
                handle,
                ui::Color::WHITE,
            );
        }
        (AssetKind::Image, None) => {
            let color = if entry.load_failed { theme.error_red } else { theme.text_muted };
            ui.label_in_bounds_styled(
                if entry.load_failed { "!" } else { "…" },
                slot_ui,
                ui::TextAlign::Center,
                color,
                theme.fonts.heading,
                0.0,
            );
        }
        (AssetKind::Scene, _) => {
            // Simple scene glyph: an accent page outline with a fold tick
            let page = ui::Rect::new(slot.x + 20.0, slot.y + 12.0, slot.width - 40.0, slot.height - 24.0);
            ui.rect_border(page, theme.accent_cyan, 1.5, 2.0);
            ui.rect(
                ui::Rect::new(page.x + page.width - 12.0, page.y, 12.0, 2.0),
                theme.accent_cyan,
            );
        }
        (AssetKind::Script, _) => {
            let extension = std::path::Path::new(&entry.name)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_uppercase();
            ui.label_in_bounds_styled(
                &extension,
                slot_ui,
                ui::TextAlign::Center,
                theme.accent_cyan,
                theme.fonts.heading,
                0.0,
            );
        }
    }

    // A name wider than the tile would bleed into its neighbours, so it is
    // truncated here; the tile's tooltip and the status bar carry the full
    // path.
    let label = tile_label(&entry.name, |text| {
        ui.measure_text_styled(text, theme.fonts.small).x
    });
    ui.label_in_bounds_styled(
        &label,
        ui::Rect::new(slot.x, slot.y + TILE_SIZE, TILE_SIZE, TILE_LABEL_HEIGHT),
        ui::TextAlign::Center,
        theme.text_secondary,
        theme.fonts.small,
        0.0,
    );
}

/// The filename as it is drawn under a tile: truncated with an ellipsis to
/// the tile's own width according to the caller's measurement.
fn tile_label(name: &str, measure: impl Fn(&str) -> f32) -> String {
    editor::ellipsize(name, TILE_SIZE, measure)
}

/// What one tile did this frame.
struct TileInteraction {
    /// A press-and-release that did not turn into a drag.
    clicked: bool,
}

/// Press arms a drag (images and .rhai scripts); a plain click selects the tile.
fn tile_interaction(
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
    }

    TileInteraction {
        clicked: result.clicked && !drag_drop.suppresses_click(),
    }
}

/// Assign the selected tile's texture to the primary selection, as one undo
/// entry. The header's button is disabled unless this can succeed, so the
/// refusals below are the safety net, not the usual path.
fn assign_selected_texture(
    editor: &mut EditorContext,
    world: &mut ecs::World,
    command_history: &mut CommandHistory,
) {
    let Some((handle, path)) = editor
        .asset_browser
        .selected_entry()
        .and_then(|entry| entry.texture_handle.map(|handle| (handle, entry.relative_path.clone())))
    else {
        editor.status_bar.show_message("Select a texture in the asset browser first");
        return;
    };

    match editor.selection.primary() {
        Some(entity) if entity_ops::assign_sprite_texture(world, entity, handle, command_history) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    use ecs::{Sprite, World};
    use editor::AssetEntry;

    /// One image entry with a loaded thumbnail, ready to be selected.
    fn image_entry(name: &str, handle: u32) -> AssetEntry {
        AssetEntry {
            name: name.to_string(),
            relative_path: format!("sprites/{name}"),
            kind: AssetKind::Image,
            texture_handle: Some(handle),
            load_failed: false,
        }
    }

    /// An editor with one selectable image and a sprite entity selected.
    fn editor_with_sprite_and_asset() -> (EditorContext, World, ecs::EntityId) {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(&entity, Sprite::new(0)).ok();

        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(vec![image_entry("hero.png", 7)]);
        editor.selection.select(entity);
        (editor, world, entity)
    }

    #[test]
    fn test_a_click_selects_the_tile_without_touching_the_entity() {
        let (mut editor, world, entity) = editor_with_sprite_and_asset();

        select_tile(&mut editor, 0);

        assert_eq!(editor.asset_browser.selected, Some(0));
        assert_eq!(
            world.get::<Sprite>(entity).map(|sprite| sprite.texture_handle),
            Some(0),
            "selecting a tile must never assign its texture"
        );
        assert_eq!(
            editor.status_bar.message(),
            Some("sprites/hero.png"),
            "the status bar carries the full relative path the tile label cannot show"
        );
    }

    /// A click puts the full relative path on the status bar and never
    /// writes over a persistent error — a failed save must stay readable,
    /// and the selection is not a way to clear it.
    #[test]
    fn test_a_click_puts_the_path_on_the_bar_and_leaves_an_error_alone() {
        let (mut editor, _, _) = editor_with_sprite_and_asset();

        select_tile(&mut editor, 0);
        assert_eq!(
            editor.status_bar.message(),
            Some("sprites/hero.png"),
            "the click is what writes the full relative path the tile label cannot show"
        );

        editor.status_bar.show_error("Failed to save");
        select_tile(&mut editor, 0);
        assert_eq!(editor.status_bar.message(), Some("Failed to save"), "a click never erases an error");
        assert!(editor.status_bar.is_showing_error());
    }

    /// The label under a tile is ellipsized to the tile's width, so resting
    /// on the tile is how its full relative path is read: that is the
    /// tooltip's text, and the status bar stops carrying it on hover.
    #[test]
    fn test_resting_on_a_tile_offers_its_full_path_as_the_tooltips_text() {
        let theme = EditorTheme::default();
        let entry = image_entry("hero.png", 7);
        let slot = common::Rect::new(40.0, 40.0, TILE_SIZE, TILE_SIZE + TILE_LABEL_HEIGHT);
        let pointer = Vec2::new(slot.x + TILE_SIZE * 0.5, slot.y + TILE_SIZE * 0.5);

        let mut ui = ui::UIContext::new();
        let mut input = input::InputHandler::new();
        let mut drag_drop = DragDropState::new();
        input.mouse_mut().update_position(pointer.x, pointer.y);

        // Three frames of half a second: the first takes the anchor up, the
        // rest is past the tooltip's rest-to-show delay.
        for _ in 0..3 {
            ui.begin_frame_dt(&input, Vec2::new(800.0, 600.0), 0.5);
            tile_interaction(&mut ui, &mut drag_drop, &theme, &entry, 0, slot, pointer);
            ui.end_frame();
        }

        let band = ui::UiLayer::Tooltip.depth_base()..ui::UiLayer::DragGhost.depth_base();
        let words: Vec<String> = ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| band.contains(&command.depth()))
            .filter_map(|command| match command {
                ui::DrawCommand::Text { data, .. } => Some(data.text.clone()),
                ui::DrawCommand::TextPlaceholder { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(words, vec!["sprites/hero.png".to_string()]);
    }

    #[test]
    fn test_assign_sets_the_texture_and_undo_puts_the_old_one_back() {
        let (mut editor, mut world, entity) = editor_with_sprite_and_asset();
        let mut history = CommandHistory::new();
        select_tile(&mut editor, 0);

        assign_selected_texture(&mut editor, &mut world, &mut history);
        assert_eq!(
            world.get::<Sprite>(entity).map(|sprite| sprite.texture_handle),
            Some(7)
        );

        assert!(history.undo(&mut world), "the assignment is one undo entry");
        assert_eq!(
            world.get::<Sprite>(entity).map(|sprite| sprite.texture_handle),
            Some(0)
        );
    }

    #[test]
    fn test_assign_is_offered_only_for_a_loaded_image_over_a_sprite_entity() {
        let (mut editor, world, _) = editor_with_sprite_and_asset();
        assert!(!can_assign_selection(&editor, &world), "nothing is selected yet");

        select_tile(&mut editor, 0);
        assert!(can_assign_selection(&editor, &world));

        editor.selection.clear();
        assert!(!can_assign_selection(&editor, &world), "no entity to assign to");
    }

    #[test]
    fn test_a_long_name_is_ellipsized_to_fit_under_its_tile() {
        let measure = |text: &str| text.chars().count() as f32 * 7.0;

        let short = tile_label("hero.png", measure);
        assert_eq!(short, "hero.png", "a name that fits is drawn whole");

        let long = tile_label("a_very_long_deion_sprite_name.png", measure);
        assert!(measure(&long) <= TILE_SIZE, "the drawn label fits the tile");
        assert!(long.ends_with('…'), "truncation is visible to the reader");
    }

    #[test]
    fn test_a_rescan_keeps_the_selection_on_the_file_it_pointed_at() {
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(vec![image_entry("hero.png", 7)]);
        select_tile(&mut editor, 0);

        // A new file sorts ahead of the selected one, moving its index.
        editor
            .asset_browser
            .apply_scan(vec![image_entry("armour.png", 8), image_entry("hero.png", 7)]);
        assert_eq!(editor.asset_browser.selected, Some(1));

        editor.asset_browser.apply_scan(vec![image_entry("armour.png", 8)]);
        assert_eq!(editor.asset_browser.selected, None, "a deleted file drops the selection");
    }

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
