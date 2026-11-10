//! Asset operations through the document host, guard and watcher seams.

use std::cell::RefCell;
use std::path::PathBuf;
use common::vfs;
use editor::{AssetKind, TileId};
use crate::panel_renderer::asset_browser::rename::{self, RenameRequest};
use super::{FileMutation, MutationPhase};
use super::scene_confirm::PendingSceneAction;
use super::test_support::{dirty_editor, editor_game, signed_song, stock_sound_font};
use super::hot_reload_tests::Session;
use ecs::script::{ScriptRef, Scripts};
use engine_core::test_support::test_texture_path;

fn request(path: &str, name: &str, kind: AssetKind) -> RenameRequest {
    RenameRequest { tile: TileId::file(path), name: name.into(), kind }
}

#[test]
fn deleting_the_open_scene_or_its_folder_is_refused() {
    let mut session = Session::new();
    session.save_scene("scenes/level.scene.ron");
    for tile in [TileId::file("scenes/level.scene.ron"), TileId::folder("scenes")] {
        let error = session.editor.prepare_asset_delete(&tile, &[]).unwrap_err();
        assert!(error.contains("scene") && error.contains("level.scene.ron") && error.contains("is open"));
    }
    assert!(session.path("scenes/level.scene.ron").exists());
}

#[test]
fn renaming_an_open_scene_moves_its_next_save_target() {
    let mut session = Session::new();
    session.save_scene("scenes/level.scene.ron");
    session.editor.request_asset_rename(request("scenes/level.scene.ron", "next", AssetKind::Scene), false, Vec::new());
    assert_eq!(session.editor.editor.scene_path(), Some(session.path("scenes/next.scene.ron").as_path()));
    session.editor.save_scene(&mut session.world, &session.assets).unwrap();
    assert!(!session.path("scenes/level.scene.ron").exists());
    assert!(engine_core::SceneLoader::load_from_file(session.path("scenes/next.scene.ron")).is_ok());
}

#[test]
fn a_folder_rename_moves_the_open_scene_and_refuses_while_a_reload_waits() {
    let mut session = Session::new();
    session.save_scene("scenes/levels/level.scene.ron");
    let request = RenameRequest { tile: TileId::folder("scenes/levels"), name: "chapters".into(), kind: AssetKind::Folder };
    session.editor.hot_reload.waiting_scene = Some(session.path("scenes/levels/level.scene.ron"));
    session.editor.request_asset_rename(request.clone(), false, Vec::new());
    assert!(session.editor.editor.asset_prompt.error().unwrap().contains("pending confirm or reload"));
    assert!(session.path("scenes/levels/level.scene.ron").exists());
    session.editor.hot_reload.waiting_scene = None;
    session.editor.request_asset_rename(request, false, Vec::new());
    assert_eq!(session.editor.editor.scene_path(), Some(session.path("scenes/chapters/level.scene.ron").as_path()));
}

#[test]
fn renaming_an_asset_the_unsaved_scene_names_parks_the_reference_confirm() {
    let mut world = ecs::World::new();
    let editor = dirty_editor(&mut world);
    let mut session = Session::with_editor(editor);
    session.world = world;
    session.write("scripts/paddle.rhai", "fn update(me, view, params, cmd, dt) {}");
    let mut script = ScriptRef::new("mover");
    script.source_path = "scripts/paddle.rhai".into();
    let entity = session.world.create_entity();
    session.world.add_component(&entity, Scripts(vec![script])).ok();
    let live = session.editor.unsaved_document_texts(&session.world, &test_texture_path);
    session.editor.request_asset_rename(request("scripts/paddle.rhai", "player", AssetKind::Script), false, live);
    match &session.editor.scene_confirm.pending_action {
        Some(PendingSceneAction::RenameAsset(_, hits)) => assert!(hits.iter().any(|hit| hit.contains("open scene")), "{hits:?}"),
        other => panic!("the unsaved scene's reference must park the confirm, not {other:?}"),
    }
    assert!(session.path("scripts/paddle.rhai").exists(), "nothing moved before the answer");
}

#[test]
fn deleting_an_asset_the_unsaved_scene_names_tells_the_confirm_and_an_unrendered_scene_counts() {
    let mut world = ecs::World::new();
    let editor = dirty_editor(&mut world);
    let mut session = Session::with_editor(editor);
    session.world = world;
    session.write("scripts/paddle.rhai", "fn update(me, view, params, cmd, dt) {}");
    let mut script = ScriptRef::new("mover");
    script.source_path = "scripts/paddle.rhai".into();
    let entity = session.world.create_entity();
    session.world.add_component(&entity, Scripts(vec![script])).ok();
    let tile = TileId::file("scripts/paddle.rhai");
    let parked = session.editor.prepare_asset_delete(&tile, &[]).unwrap();
    assert!(parked.references.is_empty() && !parked.live_scanned, "the menu's entry point has no world");
    let live = session.editor.unsaved_document_texts(&session.world, &test_texture_path);
    let rescanned = session.editor.scan_delete_plan_live(parked, &live).unwrap();
    assert!(rescanned.live_scanned && rescanned.references.iter().any(|hit| hit.contains("open scene")), "{:?}", rescanned.references);
    let unrendered = [("the open scene (unsaved, not scanned: no snapshot)".to_string(), None)];
    let conservative = session.editor.prepare_asset_delete(&tile, &unrendered).unwrap();
    assert_eq!(conservative.references, vec!["the open scene (unsaved, not scanned: no snapshot)"]);
    assert!(session.path("scripts/paddle.rhai").exists());
}

#[test]
fn a_recovered_song_open_in_the_conductor_refuses_its_primarys_delete_and_follows_its_rename() {
    let mut session = Session::new();
    session.write("music/theme.mid", "score");
    session.write("music/theme.recovered-1.mid", "score");
    session.editor.song = Some(signed_song());
    let recovered = session.path("music/theme.recovered-1.mid");
    session.editor.song.as_mut().unwrap().follow_rename(recovered);
    let error = session.editor.prepare_asset_delete(&TileId::file("music/theme.mid"), &[]).unwrap_err();
    assert!(error.contains("song") && error.contains("theme.recovered-1.mid") && error.contains("is open"), "{error}");
    assert!(session.path("music/theme.recovered-1.mid").exists());
    let rename_request = request("music/theme.mid", "anthem", AssetKind::Song);
    session.editor.request_asset_rename(rename_request.clone(), false, Vec::new());
    assert!(matches!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::SaveAndRenameAsset(..))), "the dirty recovered song parks the rename behind Save and rename, as the primary would");
    assert!(session.path("music/theme.recovered-1.mid").exists());
    session.editor.scene_confirm.pending_action = None;
    let plan = rename::prepare(session.project.path(), &rename_request).unwrap();
    session.editor.perform_asset_rename(&rename_request, plan);
    assert_eq!(session.editor.song.as_ref().unwrap().path(), Some(session.path("music/anthem.recovered-1.mid").as_path()), "the open recovered song follows the family");
    assert!(session.path("music/anthem.recovered-1.mid").exists() && !session.path("music/theme.recovered-1.mid").exists());
}

#[test]
fn a_recovered_scene_open_with_a_reload_waiting_refuses_its_primarys_rename() {
    let mut session = Session::new();
    session.save_scene("scenes/level.scene.recovered-1.ron");
    session.write("scenes/level.scene.ron", "scene");
    session.editor.hot_reload.waiting_scene = Some(session.path("scenes/level.scene.recovered-1.ron"));
    session.editor.request_asset_rename(request("scenes/level.scene.ron", "next", AssetKind::Scene), false, Vec::new());
    assert!(session.editor.editor.asset_prompt.error().unwrap().contains("pending confirm or reload"));
    assert!(session.path("scenes/level.scene.ron").exists() && session.path("scenes/level.scene.recovered-1.ron").exists());
}

#[test]
fn the_open_songs_orphan_sidecar_refuses_its_delete() {
    let mut session = Session::new();
    session.write("music/theme.mid.ron", "map");
    session.editor.song = Some(signed_song());
    let song_path = session.path("music/theme.mid");
    session.editor.song.as_mut().unwrap().follow_rename(song_path);
    let error = session.editor.prepare_asset_delete(&TileId::file("music/theme.mid.ron"), &[]).unwrap_err();
    assert!(error.contains("song") && error.contains("is open"), "{error}");
    assert!(session.path("music/theme.mid.ron").exists());
}

#[test]
fn an_open_scene_changed_on_disk_since_its_save_refuses_its_rename_until_reloaded() {
    let mut session = Session::new();
    session.save_scene("scenes/level.scene.ron");
    let mut external = vfs::read_to_string(&session.path("scenes/level.scene.ron")).unwrap();
    external.push_str("\n// another program's edit\n");
    std::thread::sleep(std::time::Duration::from_millis(20));
    vfs::write(&session.path("scenes/level.scene.ron"), external.as_bytes()).unwrap();
    session.editor.request_asset_rename(request("scenes/level.scene.ron", "next", AssetKind::Scene), false, Vec::new());
    let error = session.editor.editor.asset_prompt.error().unwrap().to_string();
    assert!(error.contains("changed on disk"), "{error}");
    assert!(session.path("scenes/level.scene.ron").exists() && !session.path("scenes/next.scene.ron").exists());
    session.save_scene("scenes/level.scene.ron");
    session.editor.request_asset_rename(request("scenes/level.scene.ron", "next", AssetKind::Scene), false, Vec::new());
    assert_eq!(session.editor.editor.scene_path(), Some(session.path("scenes/next.scene.ron").as_path()), "a save takes the stamp again");
    let mut external = vfs::read_to_string(&session.path("scenes/next.scene.ron")).unwrap();
    external.push_str("\n// edited after the rename\n");
    std::thread::sleep(std::time::Duration::from_millis(20));
    vfs::write(&session.path("scenes/next.scene.ron"), external.as_bytes()).unwrap();
    session.editor.request_asset_rename(request("scenes/next.scene.ron", "third", AssetKind::Scene), false, Vec::new());
    assert!(session.editor.editor.asset_prompt.error().unwrap().contains("changed on disk"), "the baseline followed the rename");
    assert!(session.path("scenes/next.scene.ron").exists());
    // The refused prompt is a dialog the reload waits out; closed, the host's next tick
    // watches the renamed path and the worker's report reloads it.
    session.editor.editor.asset_prompt.close();
    session.editor.editor.asset_creation = None;
    session.watch_now();
    session.changed("scenes/next.scene.ron");
    assert!(session.console_mentions("reloaded from disk"), "{:?}", session.console());
    session.editor.request_asset_rename(request("scenes/next.scene.ron", "third", AssetKind::Scene), false, Vec::new());
    assert_eq!(session.editor.editor.scene_path(), Some(session.path("scenes/third.scene.ron").as_path()), "a reload from disk takes the stamp again");
}

#[test]
fn a_dirty_scene_changed_on_disk_is_held_before_save_and_rename_could_overwrite_the_edit() {
    let mut world = ecs::World::new();
    let editor = dirty_editor(&mut world);
    let mut session = Session::with_editor(editor);
    session.world = world;
    session.save_scene("scenes/level.scene.ron");
    let entity = session.world.create_entity();
    let command = Box::new(editor::commands::CreateEntityCommand::already_created(&session.world, entity));
    session.editor.command_history.execute(command, &mut session.world);
    assert!(session.editor.command_history.is_dirty());
    let mut external = vfs::read_to_string(&session.path("scenes/level.scene.ron")).unwrap();
    external.push_str("\n// another program's edit\n");
    std::thread::sleep(std::time::Duration::from_millis(20));
    vfs::write(&session.path("scenes/level.scene.ron"), external.as_bytes()).unwrap();
    session.editor.request_asset_rename(request("scenes/level.scene.ron", "next", AssetKind::Scene), false, Vec::new());
    assert!(session.editor.editor.asset_prompt.error().unwrap().contains("changed on disk"), "a dirty scene's save would overwrite the edit, so it is held too");
    assert!(session.editor.scene_confirm.pending_action.is_none());
    assert_eq!(vfs::read_to_string(&session.path("scenes/level.scene.ron")).unwrap(), external, "nothing was written over the external edit");
}

#[test]
fn an_edit_landing_between_the_check_and_the_move_is_caught_at_the_move_and_a_failed_move_adopts_no_baseline() {
    let mut session = Session::new();
    session.save_scene("scenes/level.scene.ron");
    let old = session.path("scenes/level.scene.ron");
    let rename_request = request("scenes/level.scene.ron", "next", AssetKind::Scene);
    let plan = rename::prepare(session.project.path(), &rename_request).unwrap();
    let mut external = vfs::read_to_string(&old).unwrap();
    external.push_str("\n// landed after the first check\n");
    std::thread::sleep(std::time::Duration::from_millis(20));
    vfs::write(&old, external.as_bytes()).unwrap();
    PHASES.with(|phases| phases.borrow_mut().clear());
    session.editor.file_mutation_guard = Some(collision_guard);
    session.editor.perform_asset_rename(&rename_request, plan);
    assert!(session.editor.editor.asset_prompt.error().unwrap().contains("changed on disk"), "the move checks again");
    PHASES.with(|phases| assert!(phases.borrow().is_empty(), "a refusal at the move never asks the page to prepare: {:?}", phases.borrow()));
    assert!(old.exists() && !session.path("scenes/next.scene.ron").exists());
    assert_eq!(session.editor.hot_reload.awaiting_note.get(&old), None, "no removal note went out for a move that never started");
    session.save_scene("scenes/level.scene.ron");
    let plan = rename::prepare(session.project.path(), &rename_request).unwrap();
    COLLISION.with(|path| *path.borrow_mut() = Some(session.path("scenes/next.scene.ron")));
    session.editor.perform_asset_rename(&rename_request, plan);
    PHASES.with(|phases| assert_eq!(*phases.borrow(), [MutationPhase::Prepare, MutationPhase::Failed], "a move that fails after Prepare answers Failed"));
    assert!(old.exists(), "the collision failed the move and the original stays");
    assert!(!session.editor.document_changed_on_disk(&old), "an unchanged survivor is the baseline again");
}

#[test]
fn a_watcher_note_carries_the_registered_spelling_under_a_relative_asset_base() {
    let mut game = editor_game();
    game.asset_base = PathBuf::from("assets");
    assert_eq!(game.watched_spelling("scenes/level.scene.ron"), PathBuf::from("assets/scenes/level.scene.ron"), "the note is spelled as the watcher registered the path, not canonicalised");
}

#[test]
fn save_and_rename_drops_the_rename_when_the_song_save_parks_overwrite() {
    let mut session = Session::new();
    stock_sound_font(session.project.path());
    session.editor.song = Some(signed_song());
    let old = session.path("music/theme.mid");
    session.editor.save_song(Some(old.clone())).unwrap().unwrap();
    session.editor.song.as_mut().unwrap().apply(editor::SongEdit::SetLoop { looping: false }, editor::Gesture::Discrete).unwrap();
    let sidecar = session.path("music/theme.mid.ron");
    let mut text = vfs::read_to_string(&sidecar).unwrap();
    text.push_str("\n// an external edit\n");
    vfs::write(&sidecar, text.as_bytes()).unwrap();
    let request = request("music/theme.mid", "anthem", AssetKind::Song);
    session.editor.request_asset_rename(request.clone(), false, Vec::new());
    assert!(matches!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::SaveAndRenameAsset(..))));
    session.editor.scene_confirm.pending_action = None;
    session.editor.save_before_asset_rename(request, &mut session.world, &session.assets, &test_texture_path);
    assert!(matches!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::OverwriteSong(..))));
    assert_eq!(session.editor.editor.status_bar.message(), Some("the save is waiting on Overwrite — rename after it"));
    assert!(old.exists());
    assert!(!session.path("music/anthem.mid").exists());
}

thread_local! {
    static PHASES: RefCell<Vec<MutationPhase>> = const { RefCell::new(Vec::new()) };
    static COLLISION: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

fn refusing_guard(mutation: &FileMutation) -> Result<(), String> {
    PHASES.with(|phases| phases.borrow_mut().push(mutation.phase));
    Err("the open script has unsaved edits".into())
}

fn collision_guard(mutation: &FileMutation) -> Result<(), String> {
    PHASES.with(|phases| phases.borrow_mut().push(mutation.phase));
    if mutation.phase == MutationPhase::Prepare {
        COLLISION.with(|path| vfs::write(path.borrow().as_ref().unwrap(), b"external winner").unwrap());
    }
    Ok(())
}

#[test]
fn a_prepare_refusal_leaves_files_and_the_prompt_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let mut game = editor_game();
    game.asset_base = directory.path().to_path_buf();
    vfs::write(&directory.path().join("scripts/paddle.rhai"), b"original").unwrap();
    game.file_mutation_guard = Some(refusing_guard);
    PHASES.with(|phases| phases.borrow_mut().clear());
    game.request_asset_rename(request("scripts/paddle.rhai", "player", AssetKind::Script), false, Vec::new());
    assert_eq!(vfs::read(&directory.path().join("scripts/paddle.rhai")).unwrap(), b"original");
    assert!(!directory.path().join("scripts/player.rhai").exists());
    assert!(game.editor.asset_prompt.error().unwrap().contains("unsaved edits"));
    PHASES.with(|phases| assert_eq!(*phases.borrow(), [MutationPhase::Prepare]));
}

#[test]
fn an_approved_rename_that_fails_reports_failed_and_keeps_the_original() {
    let directory = tempfile::tempdir().unwrap();
    let mut game = editor_game();
    game.asset_base = directory.path().to_path_buf();
    vfs::write(&directory.path().join("scripts/paddle.rhai"), b"original").unwrap();
    COLLISION.with(|path| *path.borrow_mut() = Some(directory.path().join("scripts/player.rhai")));
    PHASES.with(|phases| phases.borrow_mut().clear());
    game.file_mutation_guard = Some(collision_guard);
    game.request_asset_rename(request("scripts/paddle.rhai", "player", AssetKind::Script), false, Vec::new());
    PHASES.with(|phases| assert_eq!(*phases.borrow(), [MutationPhase::Prepare, MutationPhase::Failed]));
    assert_eq!(vfs::read(&directory.path().join("scripts/paddle.rhai")).unwrap(), b"original");
    assert_eq!(vfs::read(&directory.path().join("scripts/player.rhai")).unwrap(), b"external winner");
}

#[test]
#[cfg(unix)]
fn a_survivor_edited_during_a_failed_delete_is_not_made_the_baseline() {
    use std::os::unix::fs::PermissionsExt;
    let mut session = Session::new();
    session.write("sprites/hero.png", "image");
    session.write("sprites/hero.sheet.ron", "sheet");
    let (image, sheet) = (session.path("sprites/hero.png"), session.path("sprites/hero.sheet.ron"));
    let plan = session.editor.prepare_asset_delete(&TileId::file("sprites/hero.png"), &[]).unwrap();
    // The folder goes read-only after the plan, so every unlink fails and both files survive;
    // the page's Prepare stands in for another program editing the sidecar during the
    // attempt, after the stamps and before the unlinks, which must come back as its change.
    let folder = session.path("sprites");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    COLLISION.with(|path| *path.borrow_mut() = Some(sheet.clone()));
    PHASES.with(|phases| phases.borrow_mut().clear());
    session.editor.file_mutation_guard = Some(collision_guard);
    let stamps_before = session.editor.hot_reload.document_stamps.len();
    session.editor.perform_asset_delete(plan, &[]);
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(image.exists() && sheet.exists(), "the read-only folder failed the delete and both files stay");
    assert_eq!(session.editor.hot_reload.awaiting_note.get(&image), Some(&2), "the unchanged survivor is noted removed, then written: the baseline again");
    assert_eq!(session.editor.hot_reload.awaiting_note.get(&sheet), Some(&1), "the survivor edited during the attempt is noted removed only; its edit is the watcher's to report");
    assert_eq!(session.editor.hot_reload.document_stamps.len(), stamps_before, "the stamps taken for the attempt are forgotten after it");
}

#[test]
fn an_open_scene_rename_with_two_old_path_polls_before_the_host_tick_reports_no_deletion() {
    let mut session = Session::new();
    session.save_scene("scenes/level.scene.ron");
    let old = session.path("scenes/level.scene.ron");
    session.editor.hot_reload.watched = session.editor.watched_paths(&session.world, &session.assets);
    let request = request("scenes/level.scene.ron", "next", AssetKind::Scene);
    let plan = rename::prepare(session.project.path(), &request).unwrap();
    let events = super::hot_reload::poll_after_removal(old.clone(), || {
        session.editor.perform_asset_rename(&request, plan);
    });
    assert!(events.iter().all(|event| !matches!(event, editor::external_tools::WatchEvent::Change(_))));
    session.dispatch_events(events, false);
    assert!(!session.editor.editor.console.log.entries().iter().any(|entry| entry.text.contains("deleted on disk")));
}
