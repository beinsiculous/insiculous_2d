//! Breadcrumb navigation and the asset creation menu shared by pointer and keyboard.

use super::*;
use editor::context_menu::{AssetAction, ContextMenuAction, ContextMenuItem};
use editor::external_tools::FileType;

const BUTTON_WIDTH: f32 = 70.0;
const BUTTON_HEIGHT: f32 = 20.0;
const HEADER_GAP: f32 = 8.0;

pub(super) fn render_header(
    editor: &mut EditorContext, ui: &mut ui::UIContext, assets: &AssetManager,
    registry: &engine_core::scripting::ScriptRegistry, world: &ecs::World, bounds: common::Rect,
) -> bool {
    ensure_scanned(editor, assets, registry);
    let mut left = bounds.x + PADDING;
    let right = bounds.right() - PADDING;
    let controls_width = BUTTON_WIDTH * 3.0 + HEADER_GAP * 3.0;
    let breadcrumb_right = (right - controls_width).max(left);
    let mut segments = vec![("Assets".to_string(), String::new())];
    let mut path = String::new();
    for segment in editor.asset_browser.current_folder.split('/').filter(|segment| !segment.is_empty()) {
        if !path.is_empty() { path.push('/'); }
        path.push_str(segment);
        segments.push((segment.to_string(), path.clone()));
    }
    for (index, (name, folder)) in segments.iter().enumerate() {
        let remaining = (breadcrumb_right - left).max(0.0);
        if remaining < 20.0 { break; }
        let width = (ui.measure_text_styled(name, editor.theme.fonts.body).x + 18.0).min(remaining);
        let button = common::Rect::new(left, bounds.y + 3.0, width, BUTTON_HEIGHT);
        let label = editor::ellipsize(name, width - 10.0, |text| ui.measure_text_styled(text, editor.theme.fonts.body).x);
        if ui.button_styled(ui::WidgetId::from_str_index("asset_breadcrumb", index), &label, button, !editor.in_play_session()) { editor.asset_browser.enter(folder); }
        ui.tooltip(button, if folder.is_empty() { "Assets" } else { folder });
        left += width + HEADER_GAP;
        if index + 1 < segments.len() {
            ui.label_in_bounds_styled("▸", common::Rect::new(left - 8.0, button.y, 8.0, button.height), ui::TextAlign::Center, editor.theme.text_muted, editor.theme.fonts.small, 0.0);
        }
    }
    let new_bounds = common::Rect::new(breadcrumb_right + HEADER_GAP, bounds.y + 3.0, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui.button_styled("asset_new", "New ▾", new_bounds, !editor.in_play_session()) { open_new_menu(editor, Vec2::new(new_bounds.x, new_bounds.bottom())); }
    let assign_bounds = common::Rect::new(new_bounds.right() + HEADER_GAP, new_bounds.y, BUTTON_WIDTH, BUTTON_HEIGHT);
    // Assign is a paused edit like a drop: only Playing itself refuses it.
    let assign = ui.button_styled("asset_assign", "Assign", assign_bounds, !editor.is_playing() && can_assign_selection(editor, world));
    let rescan_bounds = common::Rect::new(assign_bounds.right() + HEADER_GAP, new_bounds.y, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui.button("asset_rescan", "Rescan", rescan_bounds) { refresh_assets(editor, assets, registry); }
    let files = editor.asset_browser.visible_tiles().iter().filter(|tile| tile.id.kind == TileKind::File).count();
    let count = format!("{files} {}", if files == 1 { "asset" } else { "assets" });
    ui.label_in_bounds_styled(&count, common::Rect::new(bounds.x + PADDING, bounds.y + 25.0, bounds.width - PADDING * 2.0, 16.0), ui::TextAlign::Right, editor.theme.text_muted, editor.theme.fonts.small, 0.0);
    assign
}

pub(super) fn can_assign_selection(editor: &EditorContext, world: &ecs::World) -> bool {
    editor.asset_browser.selected_entry().is_some_and(|entry| entry.kind == AssetKind::Image && entry.texture_handle.is_some())
        && editor.selection.primary().is_some_and(|entity| world.get::<ecs::Sprite>(entity).is_some())
}

/// The New button, empty grid and tile menus all expose this same action list.
pub(super) fn open_new_menu(editor: &mut EditorContext, anchor: Vec2) {
    let selected = editor.asset_browser.selected_entry();
    let primary = selected.map_or("", |entry| entry.relative_path.as_str());
    let png = Path::new(primary).extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png"));
    let no_sidecar = selected.is_some_and(|entry| !entry.has_sidecar);
    let loaded = selected.is_some_and(|entry| entry.image_size.is_some());
    let editable = !editor.in_play_session();
    let item = |label: &str, enabled: bool, action| ContextMenuItem { label: label.into(), enabled: enabled && editable, action: ContextMenuAction::Asset(action) };
    let mut items = vec![
        item("New Folder…", true, AssetAction::NewFolder),
        item("New Scene…", true, AssetAction::NewScene),
        item("New Script…", true, AssetAction::NewScript),
        item(if png && no_sidecar && !loaded { "New Sheet sidecar (loading…)" } else { "New Sheet sidecar" }, png && no_sidecar && loaded, AssetAction::NewSheetSidecar(primary.into())),
        item("New Song sidecar", editor::external_tools::is_midi_path(Path::new(primary)) && no_sidecar, AssetAction::NewSongSidecar(primary.into())),
        item("", false, AssetAction::NewFolder),
    ];
    let desktop = cfg!(not(target_arch = "wasm32"));
    let file_type = FileType::of(Path::new(primary), &editor.external_tools.daw_project_extensions);
    let open_label = file_type.map_or_else(|| "Open (select a file)".into(), |kind| editor.external_tools.tool_name(kind).open_item());
    items.push(ContextMenuItem { label: open_label, enabled: desktop && selected.is_some() && file_type.is_some(), action: ContextMenuAction::Open(editor::open_file_request(primary)) });
    items.push(ContextMenuItem { label: "Open containing folder".into(), enabled: desktop && selected.is_some(), action: ContextMenuAction::Open(editor::open_folder_request(primary)) });
    if selected.is_some_and(|entry| entry.has_sidecar) {
        if let Some(sidecar) = editor::companion_path(primary) {
            let label = editor.external_tools.tool_name(FileType::Ron).open_item().replacen("Open", "Open sidecar", 1);
            items.push(ContextMenuItem { label, enabled: desktop && editable, action: ContextMenuAction::Asset(AssetAction::OpenSidecar(sidecar)) });
        }
    }
    // On the web no program can be launched: the Open items draw disabled over the reason,
    // as the tile's menu did before the New items joined it.
    let note = (!desktop).then(|| editor::context_menu::sentence(editor::external_tools::DESKTOP_ONLY_HINT));
    editor.context_menu.open(anchor, items, note);
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor::context_menu::ContextMenuKey;

    #[test]
    fn sheet_creation_waits_for_thumbnail_dimensions_and_a_badge_offers_its_sidecar() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("hero.png"), b"image").unwrap();
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(editor::scan_assets(directory.path(), &[]));
        editor.asset_browser.selected = Some(TileId::file("hero.png"));
        open_new_menu(&mut editor, Vec2::ZERO);
        assert!(editor.context_menu.labels().contains(&"New Sheet sidecar (loading…)"));
        for _ in 0..3 { editor.context_menu.handle_key(ContextMenuKey::Down); }
        assert!(matches!(editor.context_menu.handle_key(ContextMenuKey::Choose), Some(ContextMenuAction::Open(_))), "the unloaded image's sidecar action is skipped");
        editor.asset_browser.entries[0].image_size = Some((16, 16));
        open_new_menu(&mut editor, Vec2::ZERO);
        for _ in 0..3 { editor.context_menu.handle_key(ContextMenuKey::Down); }
        assert_eq!(editor.context_menu.handle_key(ContextMenuKey::Choose), Some(ContextMenuAction::Asset(AssetAction::NewSheetSidecar("hero.png".into()))));
        std::fs::write(directory.path().join("hero.sheet.ron"), b"sidecar").unwrap();
        editor.asset_browser.apply_scan(editor::scan_assets(directory.path(), &[]));
        open_new_menu(&mut editor, Vec2::ZERO);
        assert!(editor.context_menu.labels().iter().any(|label| label.starts_with("Open sidecar")));
    }
}
