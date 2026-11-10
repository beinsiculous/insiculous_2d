//! One asset tile's interaction: the press that arms a drag of what the tile holds, the
//! click that selects it, the double-click that opens it, the right press that opens its
//! context menu, the hover border and the full-path tooltip.

use common::clock::Instant;
use glam::Vec2;

use editor::{AssetKind, DragDropState, DragPayload, EditorContext, EditorTheme, TileId, Tile};
use editor::fit_rect;
use engine_core::AssetManager;
use renderer::texture::TextureHandle;
use super::{TILE_SIZE, TILE_LABEL_HEIGHT};

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
    pub(super) clicked: Option<TileId>,
    pressed: Option<TileId>,
    right_pressed: Option<TileId>,
}

impl TilePresses {
    pub(super) fn has_right_press(&self) -> bool { self.right_pressed.is_some() }
    pub(super) fn record(&mut self, identity: &TileId, tile: &TileInteraction) {
        if tile.clicked {
            self.clicked = Some(identity.clone());
        }
        if tile.pressed {
            self.pressed = Some(identity.clone());
        }
        if tile.right_pressed {
            self.right_pressed = Some(identity.clone());
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
    if let Some(identity) = &presses.pressed {
        if let Some(request) = editor.asset_browser.press_tile(identity, Instant::now(), mouse_pos) {
            editor.open_requests.push(request);
        }
    }
    if let Some(identity) = &presses.right_pressed {
        super::select_tile(editor, identity);
        editor.asset_browser.holds_keyboard = true;
        open_tile_menu(editor, mouse_pos);
    }
}

/// Open the context menu over tile `index` with its corner at `anchor`; on the web its
/// items are disabled, since no program can be launched there.
pub(super) fn open_tile_menu(editor: &mut EditorContext, anchor: Vec2) {
    super::header::open_new_menu(editor, anchor, true);
}

/// Press arms a drag (images, .rhai scripts, sounds, songs and SoundFonts); a plain click
/// selects the tile.
pub(super) fn tile_interaction(
    ui: &mut ui::UIContext,
    drag_drop: &mut DragDropState,
    theme: &EditorTheme,
    entry: &editor::AssetEntry,
    identity: &TileId,
    slot: common::Rect,
    mouse_pos: Vec2,
) -> TileInteraction {
    let slot_ui = ui::Rect::new(slot.x, slot.y, slot.width, slot.height);
    let result = ui.interact(ui::WidgetId::hashed(&format!("asset_file:{}", identity.path)), slot_ui, true);
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

pub(super) fn render_tile(
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
        (_, _) => {
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

    if entry.has_sidecar && entry.kind == AssetKind::Image {
        let badge = ui::Rect::new(slot.right() - 39.0, slot.y + 3.0, 36.0, 18.0);
        ui.rect_rounded(badge, theme.surface_4, 2.0);
        ui.label_in_bounds_styled("sheet", badge, ui::TextAlign::Center, theme.text_primary, theme.fonts.small, 0.0);
    }
    if entry.kind == AssetKind::Companion {
        let badge = ui::Rect::new(slot.right() - 47.0, slot.y + 3.0, 44.0, 18.0);
        ui.rect_rounded(badge, theme.surface_4, 2.0);
        ui.label_in_bounds_styled("orphan", badge, ui::TextAlign::Center, theme.text_primary, theme.fonts.small, 0.0);
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
pub(super) fn tile_label(name: &str, measure: impl Fn(&str) -> f32) -> String {
    editor::ellipsize(name, TILE_SIZE, measure)
}

pub(super) fn render_folder(ui: &mut ui::UIContext, theme: &EditorTheme, tile: &Tile, slot: common::Rect, selected: bool) {
    ui.rect_rounded(slot, if selected { theme.selection_fill } else { theme.surface_3 }, 4.0);
    let icon = if tile.name == ".." { editor::icons::Icon::Up } else { editor::icons::Icon::Folder };
    editor::icons::draw_icon(ui, icon, slot.shrink(16.0), theme.accent_cyan, 2.0);
    let label = tile_label(&tile.name, |text| ui.measure_text_styled(text, theme.fonts.small).x);
    ui.label_in_bounds_styled(&label, common::Rect::new(slot.x, slot.bottom(), TILE_SIZE, TILE_LABEL_HEIGHT), ui::TextAlign::Center, theme.text_secondary, theme.fonts.small, 0.0);
}

pub(super) fn folder_interaction(ui: &mut ui::UIContext, theme: &EditorTheme, tile: &Tile, slot: common::Rect) -> TileInteraction {
    let result = ui.interact(ui::WidgetId::hashed(&format!("asset_folder:{}", tile.id.path)), slot, true);
    if result.state == ui::WidgetState::Hovered { ui.rect_border(slot, theme.hover_fill, 1.5, 4.0); }
    ui.tooltip(slot, if tile.name == ".." { "Parent folder" } else { &tile.id.path });
    TileInteraction {
        clicked: result.clicked,
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
        editor.asset_browser.current_folder = "music".into();
        editor.asset_browser.apply_scan(editor::AssetScan { files: vec![AssetEntry {
            name: "theme.mid".to_string(),
            relative_path: "music/theme.mid".to_string(),
            kind: AssetKind::Song,
            texture_handle: None,
            load_failed: false,
            has_sidecar: false, image_size: None,
        }], folders: vec![editor::AssetFolder { name: "music".into(), relative_path: "music".into() }] });
        editor
    }

    /// One frame of the grid's one tile, its presses acted on as the panel does.
    fn tile_frame(editor: &mut EditorContext, ui: &mut ui::UIContext, input: &input::InputHandler) {
        let pointer = input.mouse().position();
        let pointer = Vec2::new(pointer.x, pointer.y);
        ui.begin_frame_dt(input, Vec2::new(800.0, 600.0), 0.016);
        editor.drag_drop.begin_frame(pointer, ui.mouse_down(), ui.mouse_just_released());
        let entry = editor.asset_browser.entries[0].clone();
        let tile = tile_interaction(ui, &mut editor.drag_drop, &editor.theme, &entry, &editor::TileId::file(&entry.relative_path), SLOT, pointer);
        let mut presses = TilePresses::default();
        presses.record(&editor::TileId::file(&entry.relative_path), &tile);
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

        assert_eq!(editor.asset_browser.selected, Some(editor::TileId::file("music/theme.mid")));
        assert!(editor.asset_browser.holds_keyboard);
        assert_eq!(editor.context_menu.labels(), vec!["New Folder…", "New Scene…", "New Script…", "New Sheet sidecar", "New Song sidecar", "Rename…", "Delete…", "Open with the system default", "Open containing folder"]);
        let bounds = editor.context_menu.bounds(&ui, &editor.theme).map(|bounds| (bounds.x, bounds.y));
        assert_eq!(bounds, Some((SLOT.center().x, SLOT.center().y)), "the menu opens at the press");
        assert!(editor.open_requests.is_empty() && editor.drag_drop.dragging_payload().is_none(), "a right press arms no drag");
    }
}

#[cfg(test)]
mod folder_tests {
    use super::*;

    #[test]
    fn a_folder_double_press_enters_without_opening_a_file_or_arming_a_drag() {
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(editor::AssetScan { files: vec![], folders: vec![editor::AssetFolder { name: "sprites".into(), relative_path: "sprites".into() }] });
        let identity = editor::TileId::folder("sprites");
        let tile = editor.asset_browser.visible_tiles()[0].clone();
        let mut ui = ui::UIContext::new();
        let mut input = input::InputHandler::new();
        let slot = common::Rect::new(40.0, 40.0, 72.0, 72.0);
        input.mouse_mut().update_position(60.0, 60.0);
        for _ in 0..2 {
            input.update();
            input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
            ui.begin_frame(&input, Vec2::new(800.0, 600.0));
            let interaction = folder_interaction(&mut ui, &editor.theme, &tile, slot);
            let mut presses = TilePresses::default();
            presses.record(&identity, &interaction);
            open_from_presses(&mut editor, &presses, Vec2::new(60.0, 60.0));
            ui.end_frame();
            input.update();
            input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
            ui.begin_frame(&input, Vec2::new(800.0, 600.0));
            folder_interaction(&mut ui, &editor.theme, &tile, slot);
            ui.end_frame();
        }
        assert_eq!(editor.asset_browser.current_folder, "sprites");
        assert!(editor.open_requests.is_empty());
        assert!(editor.drag_drop.dragging_payload().is_none());
        // The release's click names the folder just entered, which the new grid does not show.
        super::super::select_tile(&mut editor, &identity);
        assert_eq!(editor.asset_browser.selected, None, "a tile outside the grid is never selected");
    }
}
