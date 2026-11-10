//! Asset creation writes files, never scene commands: there is no undo of file operations.
//! Browser navigation changes only after the requested filesystem operation succeeds.

use std::path::Path;
use common::vfs;
use editor::{AssetKind, EditorContext, TileId, PromptOutcome};
use editor::context_menu::AssetAction;
use engine_core::contexts::GameContext;
use engine_core::sheet_file::{SheetFile, SHEET_FILE_VERSION, write_sheet_file, parse_sheet_file};

const SCRIPT_TEMPLATE: &str = "// @param speed: f32 = 250.0\n\nfn update(me, view, params, cmd, dt) {\n}\n";

/// Prepare one operation. Sidecar names are derived from the selected primary.
pub(crate) fn begin_creation(editor: &mut EditorContext, base: &Path, action: AssetAction) {
    if editor.in_play_session() || editor.asset_prompt.is_open() { return; }
    if let AssetAction::OpenSidecar(path) = action {
        let spelled = editor::companion_on_disk(&base.join(&path).to_string_lossy())
            .and_then(|found| Path::new(&found).file_name().map(|name| name.to_string_lossy().into_owned()))
            .map(|name| path.rsplit_once('/').map_or(name.clone(), |(folder, _)| format!("{folder}/{name}")));
        editor.open_requests.push(editor::open_file_request(spelled.as_deref().unwrap_or(&path)));
        return;
    }
    let (title, initial, immediate) = match &action {
        AssetAction::NewFolder => ("New Folder", "new_folder".into(), false),
        AssetAction::NewScene => ("New Scene", "new_scene".into(), false),
        AssetAction::NewScript => ("New Script", "new_script".into(), false),
        AssetAction::NewSheetSidecar(primary) | AssetAction::NewSongSidecar(primary) => {
            let path = editor::companion_path(primary).unwrap_or_default();
            let name = Path::new(&path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            ("New sidecar", name, true)
        }
        AssetAction::OpenSidecar(_) | AssetAction::Rename(_) | AssetAction::Delete(_) => return,
    };
    editor.asset_creation = Some(action);
    editor.asset_prompt.open(title, &initial, "Create");
    editor.context_menu.close();
    if immediate { confirm_creation(editor, base, &initial); }
}

/// Draw before the chrome and commit only a confirmed, validated name.
pub(crate) fn render_prompt(editor: &mut EditorContext, ctx: &mut GameContext) -> Option<(TileId, String)> {
    match editor.asset_prompt.render(ctx.ui, &editor.theme) {
        PromptOutcome::Confirmed(name) => {
            if let Some(AssetAction::Rename(tile)) = &editor.asset_creation {
                return Some((tile.clone(), name));
            }
            if confirm_creation(editor, Path::new(ctx.assets.base_path()), &name) {
                ctx.ui.clear_text_focus();
                super::refresh_assets(editor, ctx.assets, ctx.scripts.registry());
            }
        }
        PromptOutcome::Cancelled => editor.asset_creation = None,
        PromptOutcome::Open => {},
    }
    None
}

/// Apply the pending creation. Refusals leave the draft and browser intact.
pub(crate) fn confirm_creation(editor: &mut EditorContext, base: &Path, name: &str) -> bool {
    let Some(action) = editor.asset_creation.clone() else { return false; };
    let size = editor.asset_browser.selected_entry().and_then(|entry| entry.image_size);
    let created = create(base, &editor.asset_browser.current_folder, &action, name, size);
    match created {
        Ok((path, identity)) => {
            editor.asset_browser.apply_scan(editor::scan_assets(base, &editor.external_tools.daw_project_extensions));
            let parent = Path::new(&identity.path).parent().and_then(Path::to_str).unwrap_or("");
            editor.asset_browser.enter(parent);
            editor.asset_browser.selected = Some(identity);
            editor.asset_browser.reveal_selected = true;
            editor.conductor.songs_scanned = false;
            editor.asset_prompt.close();
            editor.asset_creation = None;
            editor.status_bar.show_message(format!("Created {path}"));
            true
        }
        Err(error) => { editor.asset_prompt.show_error(error); false }
    }
}

fn create(base: &Path, current: &str, action: &AssetAction, name: &str, image_size: Option<(u32, u32)>) -> Result<(String, TileId), String> {
    if let AssetAction::NewSheetSidecar(primary) | AssetAction::NewSongSidecar(primary) = action {
        let path = editor::companion_path(primary).ok_or("Select a PNG or MIDI primary for its sidecar")?;
        if Path::new(&path).file_name().and_then(|name| name.to_str()) != Some(name) {
            return Err("A companion must use the name derived from its primary file".into());
        }
        // A menu can remain open while another program removes its primary.
        if !vfs::exists(&base.join(primary)) { return Err(format!("The primary {primary} no longer exists")); }
        if matches!(action, AssetAction::NewSongSidecar(_)) {
            editor::song::init_song(&base.join(primary), base).map_err(|error| error.to_string())?;
        } else {
            let cell = image_size.ok_or("The image is still loading; wait for its thumbnail")?;
            let sheet = SheetFile {
                version: SHEET_FILE_VERSION, cell, filter: renderer::TextureFilter::Nearest,
                clips: vec![("idle".into(), engine_core::scene_data::ClipData { frames: vec![0], fps: 6.0, looping: true })],
            };
            let text = write_sheet_file(&sheet);
            parse_sheet_file(&path, &text).map_err(|error| error.to_string())?;
            vfs::write_new(&base.join(&path), text.as_bytes()).map_err(|error| format!("Could not create {path}: {error}"))?;
        }
        return Ok((path, TileId::file(primary)));
    }
    let kind = match action {
        AssetAction::NewFolder => AssetKind::Folder,
        AssetAction::NewScene => AssetKind::Scene,
        AssetAction::NewScript => AssetKind::Script,
        _ => return Err("Select a creation action".into()),
    };
    let name = if kind == AssetKind::Folder { editor::validate_folder(current, name)? } else { editor::validate_name(name, kind)? };
    let path = editor::route_new_file(current, kind, &name);
    let destination = base.join(&path);
    if kind == AssetKind::Folder {
        if vfs::exists(&destination) { return Err(format!("{path} already exists; choose another name")); }
        vfs::create_dir_all(&destination).map_err(|error| format!("Could not create folder {path}: {error}; any parent directories already created remain on disk"))?;
        return Ok((path.clone(), TileId::folder(&path)));
    }
    let text = if kind == AssetKind::Scene {
        engine_core::scene_serializer::serialize_to_ron(&engine_core::scene_data::SceneData {
            name: name.trim_end_matches(".scene.ron").into(), ..Default::default()
        })?
    } else { SCRIPT_TEMPLATE.into() };
    let parent = destination.parent().ok_or("The new file has no parent folder")?;
    let created_parent = !vfs::exists(parent);
    if created_parent {
        vfs::create_dir_all(parent).map_err(|error| format!("Could not create {}: {error}; any parent directories already created remain on disk", parent.display()))?;
    }
    vfs::write_new(&destination, text.as_bytes()).map_err(|error| {
        let partial = if created_parent { format!("Created folder {}; ", parent.display()) } else { String::new() };
        format!("{partial}could not create {path}: {error}")
    })?;
    Ok((path.clone(), TileId::file(&path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_scene_loads_and_new_script_compiles_and_a_collision_keeps_the_draft_open() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        let mut editor = EditorContext::new();
        begin_creation(&mut editor, base, AssetAction::NewScene);
        assert!(confirm_creation(&mut editor, base, "level_two"));
        let scene = engine_core::SceneLoader::load_from_file(base.join("scenes/level_two.scene.ron")).unwrap();
        assert_eq!(scene.name, "level_two");
        assert!(scene.entities.is_empty() && scene.prefabs.is_empty());
        assert_eq!(editor.asset_browser.current_folder, "scenes");
        assert_eq!(editor.asset_browser.selected, Some(TileId::file("scenes/level_two.scene.ron")));
        begin_creation(&mut editor, base, AssetAction::NewScene);
        assert!(!confirm_creation(&mut editor, base, "Bad Name"));
        assert!(editor.asset_prompt.error().unwrap().contains("snake_case"));
        assert!(!confirm_creation(&mut editor, base, "level_two"));
        assert!(editor.asset_prompt.is_open());
        assert_eq!(editor.asset_browser.selected, Some(TileId::file("scenes/level_two.scene.ron")));
        assert_eq!(engine_core::SceneLoader::load_from_file(base.join("scenes/level_two.scene.ron")).unwrap().name, "level_two");
        editor.asset_prompt.close();
        begin_creation(&mut editor, base, AssetAction::NewScript);
        assert!(confirm_creation(&mut editor, base, "paddle"));
        let text = vfs::read_to_string(&base.join("scripts/paddle.rhai")).unwrap();
        engine_core::scripting::RhaiBackend::new().compile("scripts/paddle.rhai", &text).unwrap();
        assert_eq!(editor.asset_browser.current_folder, "scripts");
    }

    #[test]
    fn a_sheet_sidecar_is_a_single_cell_idle_clip_beside_the_nested_primary() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        let primary = "sprites/props/hero.png";
        vfs::write(&base.join(primary), b"thumbnail already loaded by host").unwrap();
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(editor::scan_assets(base, &[]));
        editor.asset_browser.enter("sprites/props");
        editor.asset_browser.selected = Some(TileId::file(primary));
        editor.asset_browser.entries.iter_mut().find(|entry| entry.relative_path == primary).unwrap().image_size = Some((32, 48));
        begin_creation(&mut editor, base, AssetAction::NewSheetSidecar(primary.into()));
        assert!(!editor.asset_prompt.is_open());
        let text = vfs::read_to_string(&base.join("sprites/props/hero.sheet.ron")).unwrap();
        let sheet = parse_sheet_file("hero.sheet.ron", &text).unwrap();
        assert_eq!(sheet, SheetFile { version: 1, cell: (32, 48), filter: renderer::TextureFilter::Nearest,
            clips: vec![("idle".into(), engine_core::scene_data::ClipData { frames: vec![0], fps: 6.0, looping: true })] });
        assert!(editor.asset_browser.selected_entry().unwrap().has_sidecar);
        assert_eq!(editor.asset_browser.visible_tiles().len(), 2, "parent and primary, never the sidecar");
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_keeps_the_prompt_open_and_the_browser_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        use std::os::unix::fs::PermissionsExt;
        let scripts = base.join("scripts");
        std::fs::create_dir(&scripts).unwrap();
        std::fs::set_permissions(&scripts, std::fs::Permissions::from_mode(0o555)).unwrap();
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(editor::scan_assets(base, &[]));
        let before = editor.asset_browser.visible_tiles();
        begin_creation(&mut editor, base, AssetAction::NewScript);
        let succeeded = confirm_creation(&mut editor, base, "paddle");
        std::fs::set_permissions(&scripts, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!succeeded);
        assert!(editor.asset_prompt.is_open());
        assert!(editor.asset_prompt.error().unwrap().contains("could not create scripts/paddle.rhai"));
        assert_eq!(editor.asset_browser.visible_tiles(), before);
        assert!(!vfs::exists(&scripts.join("paddle.rhai")));
        assert_eq!(editor.asset_browser.current_folder, "");
    }

    #[test]
    fn new_folder_stays_visible_and_refuses_nested_kind_folders() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::create_dir_all(&base.join("sprites")).unwrap();
        let mut editor = EditorContext::new();
        editor.asset_browser.apply_scan(editor::scan_assets(base, &[]));
        editor.asset_browser.enter("sprites");
        begin_creation(&mut editor, base, AssetAction::NewFolder);
        assert!(!confirm_creation(&mut editor, base, "scenes"));
        assert!(confirm_creation(&mut editor, base, "props"));
        assert_eq!(editor.asset_browser.selected, Some(TileId::folder("sprites/props")));
        editor.asset_browser.apply_scan(editor::scan_assets(base, &[]));
        assert!(editor.asset_browser.visible_tiles().iter().any(|tile| tile.id == TileId::folder("sprites/props")));
    }

    #[test]
    fn open_sidecar_asks_for_the_companion_as_the_folder_spells_it() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("sprites/hero.png"), b"image").unwrap();
        vfs::write(&base.join("sprites/hero.SHEET.RON"), b"sheet").unwrap();
        let mut editor = EditorContext::new();
        begin_creation(&mut editor, base, AssetAction::OpenSidecar("sprites/hero.sheet.ron".into()));
        assert_eq!(editor.open_requests, vec![editor::open_file_request("sprites/hero.SHEET.RON")]);
    }
}
