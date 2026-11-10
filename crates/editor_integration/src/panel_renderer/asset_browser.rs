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
use editor::{scan_assets, AssetKind, CommandHistory, EditorContext, TileId, TileKind};
use engine_core::contexts::GameContext;
use engine_core::AssetManager;


use crate::entity_ops;

pub(super) mod keyboard;
mod tile;
mod header;
pub(crate) mod mutate;
pub(crate) mod rename;
pub(crate) mod remove;
use header::render_header;
#[cfg(test)]
use header::can_assign_selection;
#[cfg(test)]
use tile::tile_label;
use tile::{tile_interaction, TilePresses};

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
const HEADER_HEIGHT: f32 = 42.0;
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
    let entries = scan_assets(Path::new(assets.base_path()), &editor.external_tools.daw_project_extensions);
    editor.asset_browser.apply_scan(entries);
    // The Conductor panel's Songs list is a view of the same folders, so Rescan refreshes it too.
    editor.conductor.songs_scanned = false;
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
    let tiles = editor.asset_browser.visible_tiles();
    let rows = tiles.len().div_ceil(columns);
    let content_height = rows as f32 * (TILE_SIZE + TILE_LABEL_HEIGHT + TILE_GAP);
    let viewport_height = bounds.height - HEADER_HEIGHT - PADDING;
    // The grid's height is known up front (entry count), so record it
    // BEFORE consuming the wheel — this panel clamps lag-free.
    editor.asset_browser.scroll.end_frame(content_height, viewport_height);
    keyboard::follow_grid(editor, columns, viewport_height);
    let scroll = editor.asset_browser.scroll.begin_frame(
        bounds,
        ctx.ui.mouse_pos(),
        ctx.ui.scroll_delta(),
        viewport_height,
    );

    let is_playing = editor.is_playing();
    let mouse_pos = ctx.ui.mouse_pos();
    let mut presses = TilePresses::default();

    for (index, visible) in tiles.iter().enumerate() {
        let slot = tile_rect(index, columns, grid_origin, scroll);
        if slot.y + TILE_SIZE + TILE_LABEL_HEIGHT < grid_origin.y || slot.y > bounds.bottom() { continue; }
        let selected = editor.asset_browser.selected.as_ref() == Some(&visible.id);
        let entry = editor.asset_browser.entries.iter().find(|entry| entry.relative_path == visible.id.path);
        if visible.id.kind == TileKind::Folder {
            tile::render_folder(ctx.ui, &editor.theme, visible, slot, selected);
            if !is_playing {
                let interaction = tile::folder_interaction(ctx.ui, &editor.theme, visible, slot);
                presses.record(&visible.id, &interaction);
            }
        } else if let Some(entry) = entry {
            tile::render_tile(ctx.ui, &editor.theme, ctx.assets, entry, slot, selected);
            if !is_playing {
                let interaction = tile_interaction(ctx.ui, &mut editor.drag_drop, &editor.theme, entry, &visible.id, slot, mouse_pos);
                presses.record(&visible.id, &interaction);
            }
        }
    }

    if let Some(selected) = &presses.clicked { select_tile(editor, selected); }
    tile::open_from_presses(editor, &presses, mouse_pos);
    let grid = common::Rect::new(bounds.x, grid_origin.y, bounds.width, viewport_height);
    if !is_playing && grid.contains(mouse_pos) && ctx.ui.mouse_right_just_pressed()
        && !ctx.ui.is_input_blocked_at(mouse_pos) && !presses.has_right_press() {
        header::open_new_menu(editor, mouse_pos, false);
    }
    keyboard::mark_selected_tile(editor, ctx.ui, |index| tile_rect(index, columns, grid_origin, scroll));

    if assign_clicked {
        assign_selected_texture(editor, ctx.world, command_history);
    }
}

/// Select a tile and put its full relative path on the status bar — the
/// label under the tile is ellipsized, so the path is only readable in full
/// on the click that selects it and on the tile's tooltip. An error on the
/// bar stays: selecting a tile is not clearing a failed save.
fn select_tile(editor: &mut EditorContext, tile: &TileId) {
    // The release after a double press that entered a folder clicks the folder's own tile,
    // which is no longer in the grid: selecting it would point past the visible tiles.
    if !editor.asset_browser.visible_tiles().iter().any(|visible| visible.id == *tile) { return; }
    editor.asset_browser.selected = Some(tile.clone());
    if !editor.status_bar.is_showing_error() {
        editor.status_bar.show_message(tile.path.clone());
    }
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
                Ok(handle) => {
                    entry.texture_handle = Some(handle.id);
                    entry.image_size = assets.get_texture(handle).map(|texture| (texture.width, texture.height));
                },
                Err(error) => {
                    entry.load_failed = true;
                    log::warn!("Asset browser: failed to load '{}': {error}", entry.relative_path);
                }
            }
            loads += 1;
        }
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
    use editor::{AssetEntry, DragDropState, DragPayload, EditorTheme};

    /// One image entry with a loaded thumbnail, ready to be selected.
    fn image_entry(name: &str, handle: u32) -> AssetEntry {
        AssetEntry {
            name: name.to_string(),
            relative_path: format!("sprites/{name}"),
            kind: AssetKind::Image,
            texture_handle: Some(handle),
            load_failed: false,
            has_sidecar: false, image_size: None,
        }
    }

    /// An editor with one selectable image and a sprite entity selected.
    fn editor_with_sprite_and_asset() -> (EditorContext, World, ecs::EntityId) {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(&entity, Sprite::new(0)).ok();

        let mut editor = EditorContext::new();
        editor.asset_browser.current_folder = "sprites".into();
        editor.asset_browser.apply_scan(editor::AssetScan { files: vec![image_entry("hero.png", 7)], folders: vec![editor::AssetFolder { name: "sprites".into(), relative_path: "sprites".into() }] });
        editor.selection.select(entity);
        (editor, world, entity)
    }

    #[test]
    fn test_a_click_selects_the_tile_without_touching_the_entity() {
        let (mut editor, world, entity) = editor_with_sprite_and_asset();

        select_tile(&mut editor, &editor::TileId::file("sprites/hero.png"));

        assert_eq!(editor.asset_browser.selected, Some(editor::TileId::file("sprites/hero.png")));
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

        select_tile(&mut editor, &editor::TileId::file("sprites/hero.png"));
        assert_eq!(
            editor.status_bar.message(),
            Some("sprites/hero.png"),
            "the click is what writes the full relative path the tile label cannot show"
        );

        editor.status_bar.show_error("Failed to save");
        select_tile(&mut editor, &editor::TileId::file("sprites/hero.png"));
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
            tile_interaction(&mut ui, &mut drag_drop, &theme, &entry, &editor::TileId::file(&entry.relative_path), slot, pointer);
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
    fn test_a_press_on_a_sound_a_song_or_a_soundfont_tile_arms_a_drag_of_its_path() {
        let theme = EditorTheme::default();
        let cases = [
            (AssetKind::Sound, "sounds/jump.wav", DragPayload::Sound { path: "sounds/jump.wav".to_string() }),
            (AssetKind::Song, "music/theme.mid", DragPayload::Song { path: "music/theme.mid".to_string() }),
            (AssetKind::SoundFont, "soundfonts/cast.sf2", DragPayload::SoundFont { path: "soundfonts/cast.sf2".to_string() }),
        ];
        for (kind, path, payload) in cases {
            let entry = AssetEntry {
                name: path.rsplit('/').next().unwrap_or(path).to_string(),
                relative_path: path.to_string(),
                kind,
                texture_handle: None,
                load_failed: false,
            has_sidecar: false, image_size: None,
            };
            let slot = common::Rect::new(40.0, 40.0, TILE_SIZE, TILE_SIZE + TILE_LABEL_HEIGHT);
            let pointer = Vec2::new(slot.x + TILE_SIZE * 0.5, slot.y + TILE_SIZE * 0.5);
            let mut ui = ui::UIContext::new();
            let mut input = input::InputHandler::new();
            let mut drag_drop = DragDropState::new();
            input.mouse_mut().update_position(pointer.x, pointer.y);
            input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);

            ui.begin_frame_dt(&input, Vec2::new(800.0, 600.0), 0.016);
            tile_interaction(&mut ui, &mut drag_drop, &theme, &entry, &editor::TileId::file(&entry.relative_path), slot, pointer);
            ui.end_frame();
            drag_drop.begin_frame(pointer + Vec2::new(20.0, 0.0), true, false);

            assert_eq!(drag_drop.dragging_payload(), Some(&payload), "{kind:?}");
        }
    }

    #[test]
    fn test_assign_sets_the_texture_and_undo_puts_the_old_one_back() {
        let (mut editor, mut world, entity) = editor_with_sprite_and_asset();
        let mut history = CommandHistory::new();
        select_tile(&mut editor, &editor::TileId::file("sprites/hero.png"));

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

        select_tile(&mut editor, &editor::TileId::file("sprites/hero.png"));
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
        editor.asset_browser.current_folder = "sprites".into();
        editor.asset_browser.apply_scan(editor::AssetScan { files: vec![image_entry("hero.png", 7)], folders: vec![editor::AssetFolder { name: "sprites".into(), relative_path: "sprites".into() }] });
        select_tile(&mut editor, &editor::TileId::file("sprites/hero.png"));

        // A new file sorts ahead of the selected one, moving its index.
        editor
            .asset_browser
            .apply_scan(editor::AssetScan { files: vec![image_entry("armour.png", 8), image_entry("hero.png", 7)], folders: vec![editor::AssetFolder { name: "sprites".into(), relative_path: "sprites".into() }] });
        assert_eq!(editor.asset_browser.selected, Some(editor::TileId::file("sprites/hero.png")));

        editor.asset_browser.current_folder = "sprites".into();
        editor.asset_browser.apply_scan(editor::AssetScan { files: vec![image_entry("armour.png", 8)], folders: vec![editor::AssetFolder { name: "sprites".into(), relative_path: "sprites".into() }] });
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
