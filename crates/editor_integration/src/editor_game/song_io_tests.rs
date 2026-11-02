//! The song beside the scene, through the real `EditorGame`: Play and the song actions
//! park behind the song's prompt (Play's arms by whether a saved song exists), a scene
//! action never prompts for the song nor a song action for the scene, the API refuses a
//! dirty song without `discard`, scene and song saves leave each other's dirtiness alone, a
//! sidecar changed on disk parks the prompt or answers the API with both files, `song open`
//! rereads the open song, and `song init` maps a `.mid`. The preview verbs are
//! `song_preview_tests.rs`'s.

use ecs::World;
use editor::{ConfirmChoice, EditorPlayState, PlayControlAction, SongDocument};

use super::scene_confirm::{OverwriteCause, PendingSceneAction};
use super::test_support::{api_line, assert_ok, complete_play, dirty_editor, editor_game, signed_song, DummyGame};
use super::EditorGame;
use super::test_support::press_play;

/// A session whose asset base is a temporary folder, so saves land somewhere real.
fn session_in(folder: &tempfile::TempDir) -> EditorGame<DummyGame> {
    let mut editor = editor_game();
    editor.asset_base = folder.path().to_path_buf();
    super::test_support::stock_sound_font(folder.path());
    editor
}

fn folder() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary asset folder")
}

fn json(response: &str) -> serde_json::Value {
    serde_json::from_str(response).expect("one line of JSON")
}

/// Give the session's song the author and license Save requires, as the API does.
fn sign_song(editor: &mut EditorGame<DummyGame>, world: &mut World) {
    assert_ok(&api_line(
        editor,
        world,
        r#"song edit {"SetProvenance": {"provenance": {"author": "2026 Jesse", "license": "MIT"}}}"#,
    ));
}

/// Give the session's song an edit its last save does not hold.
fn edit_song(editor: &mut EditorGame<DummyGame>, world: &mut World) {
    assert_ok(&api_line(
        editor,
        world,
        r#"song edit {"SetLoop": {"looping": false}}"#,
    ));
}

#[test]
fn play_over_a_never_saved_song_parks_and_offers_save_or_cancel_only() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    editor.song = Some(signed_song());

    press_play(&mut editor, &mut world);
    assert_eq!(
        editor.scene_confirm.pending_action,
        Some(PendingSceneAction::StartPlay)
    );
    assert_eq!(
        editor.editor.play_state(),
        EditorPlayState::Editing,
        "Play waits for the answer"
    );
    assert!(
        !editor.song_saved_exists(),
        "so the prompt draws no \"Play saved\""
    );

    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Alt
    );
    assert_eq!(
        editor.scene_confirm.pending_action,
        Some(PendingSceneAction::StartPlay),
        "no such arm"
    );

    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Confirm
    );
    assert_eq!(editor.editor.play_state(), EditorPlayState::Editing, "armed until the frame after");
    complete_play(&mut editor, &mut world);
    assert_eq!(editor.editor.play_state(), EditorPlayState::Playing);
    let saved = folder.path().join("music/untitled.mid");
    assert_eq!(
        editor.song.as_ref().and_then(SongDocument::path),
        Some(saved.as_path()),
        "Save wrote the default path"
    );
    assert!(!editor.song.as_ref().is_some_and(SongDocument::is_dirty));
}

#[test]
fn play_over_a_saved_but_dirty_song_offers_play_saved_which_keeps_the_edits() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    sign_song(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        "song save music/theme.mid",
    ));
    edit_song(&mut editor, &mut world);

    press_play(&mut editor, &mut world);
    assert_eq!(
        editor.scene_confirm.pending_action,
        Some(PendingSceneAction::StartPlay)
    );
    assert!(editor.song_saved_exists());

    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Cancel
    );
    assert_eq!(
        editor.editor.play_state(),
        EditorPlayState::Editing,
        "Cancel stays put"
    );
    assert_eq!(editor.scene_confirm.pending_action, None);

    press_play(&mut editor, &mut world);
    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Alt
    );
    complete_play(&mut editor, &mut world);
    assert_eq!(
        editor.editor.play_state(),
        EditorPlayState::Playing,
        "Play saved plays"
    );
    assert!(
        editor.song.as_ref().is_some_and(SongDocument::is_dirty),
        "and the edits are kept, never discarded"
    );
}

#[test]
fn a_scene_action_never_prompts_for_the_song_nor_a_song_action_for_the_scene() {
    let folder = folder();
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.asset_base = folder.path().to_path_buf();
    assert!(
        !editor.request_document_action(PendingSceneAction::NewScene),
        "fixture: the scene is dirty"
    );
    editor.scene_confirm.pending_action = None;

    assert!(
        editor.request_document_action(PendingSceneAction::NewSong),
        "a clean song is not asked about the dirty scene"
    );
    editor.perform_song_action(PendingSceneAction::NewSong);
    assert!(
        editor.song.as_ref().is_some_and(SongDocument::is_dirty),
        "a new song is dirty"
    );

    let mut clean_scene = session_in(&folder);
    clean_scene.song = Some(signed_song());
    assert!(
        clean_scene.request_document_action(PendingSceneAction::NewScene),
        "opening another scene loses no song edit"
    );
    assert!(
        !clean_scene.request_document_action(PendingSceneAction::CloseSong),
        "the dirty song parks"
    );
    clean_scene.resolve_song_choice(
        PendingSceneAction::CloseSong,
        ConfirmChoice::Alt
    );
    assert!(clean_scene.song.is_none(), "Discard closes it");
}

#[test]
fn a_song_action_is_refused_during_a_play_session_with_no_dialog() {
    let mut editor = editor_game();
    let mut world = World::new();
    press_play(&mut editor, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    editor.song = Some(signed_song());

    assert!(!editor.request_document_action(PendingSceneAction::NewSong));
    assert_eq!(editor.scene_confirm.pending_action, None);
    let response = json(&api_line(
        &mut editor,
        &mut world,
        "song save music/theme.mid",
    ));
    assert_eq!(
        response["error"]["kind"], "refused",
        "Paused refuses song writes: {response}"
    );
}

#[test]
fn new_open_and_close_refuse_a_dirty_song_over_the_api_unless_the_line_says_discard() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    sign_song(&mut editor, &mut world);
    for line in ["song new", "song open music/theme.mid", "song close"] {
        let response = json(&api_line(&mut editor, &mut world, line));
        assert_eq!(response["error"]["kind"], "refused", "{line}: {response}");
        assert!(editor.song.is_some(), "{line}: the song is still there");
    }
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        "song save music/theme.mid",
    ));
    edit_song(&mut editor, &mut world);
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        "song open music/theme.mid discard",
    ));
    assert!(
        editor.song.as_ref().is_some_and(|song| song.file().looping),
        "the edit was dropped, the saved song opened"
    );
    assert_ok(&api_line(&mut editor, &mut world, "song close"));
    assert!(editor.song.is_none(), "a clean song closes without discard");
}

#[test]
fn scene_and_song_saves_leave_each_others_dirtiness_alone_and_scene_reports_both() {
    let folder = folder();
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);
    editor.asset_base = folder.path().to_path_buf();
    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    sign_song(&mut editor, &mut world);

    assert_ok(&api_line(&mut editor, &mut world, "save scenes/level.ron"));
    let scene = json(&api_line(&mut editor, &mut world, "scene"));
    assert_eq!(scene["data"]["dirty"], false, "the scene saved");
    assert_eq!(
        scene["data"]["song_dirty"], true,
        "a scene save never marks the song clean"
    );
    assert_eq!(scene["data"]["any_dirty"], true);

    assert_ok(&api_line(&mut editor, &mut world, "create sprite Crate"));
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        "song save music/theme.mid",
    ));
    let scene = json(&api_line(&mut editor, &mut world, "scene"));
    assert_eq!(
        (
            scene["data"]["dirty"].clone(),
            scene["data"]["song_dirty"].clone()
        ),
        (true.into(), false.into())
    );

    editor.sync_dirty_mirror();
    assert_eq!(
        editor.editor.title_bar_text(),
        "level.ron* · theme.mid - Insiculous Editor"
    );
}

#[test]
fn song_undo_leaves_the_scene_history_and_scene_undo_leaves_the_song() {
    let mut editor = editor_game();
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "create sprite Crate"));
    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    sign_song(&mut editor, &mut world);
    edit_song(&mut editor, &mut world);

    let undone = json(&api_line(&mut editor, &mut world, "song undo"));
    assert_eq!(undone["data"]["undid"], "SetLoop");
    assert_eq!(world.entities().len(), 1, "the scene's create stands");
    edit_song(&mut editor, &mut world);
    let undone = json(&api_line(&mut editor, &mut world, "undo"));
    assert!(
        undone["data"]["undid"]
            .as_str()
            .is_some_and(|name| name.contains("Create")),
        "{undone}"
    );
    assert!(
        editor
            .song
            .as_ref()
            .is_some_and(|song| !song.file().looping),
        "the song's edit stands"
    );
}

#[test]
fn a_new_song_plays_on_the_projects_first_bank_and_on_the_default_when_it_has_none() {
    let banked = folder();
    let mut editor = editor_game();
    editor.asset_base = banked.path().to_path_buf();
    for bank in ["soundfonts/b.sf2", "soundfonts/a.sf2"] {
        common::vfs::write(&banked.path().join(bank), b"bank").expect("written");
    }
    editor.new_song();
    let song = editor.song.as_ref().expect("a new song");
    assert_eq!(song.file().soundfont, "soundfonts/a.sf2", "the first in sorted order");

    let bare = folder();
    let mut editor = editor_game();
    editor.asset_base = bare.path().to_path_buf();
    editor.new_song();
    let song = editor.song.as_ref().expect("a new song");
    assert_eq!(song.file().soundfont, editor::song::DEFAULT_SOUND_FONT_PATH);
    assert!(!editor::composer::sound_font_exists(bare.path(), &song.file().soundfont), "which the composer warns about");
}

/// A song saved at `music/theme.mid`, its sidecar then rewritten by another program, and an
/// edit of its own unsaved.
fn song_whose_map_changed_on_disk(editor: &mut EditorGame<DummyGame>, world: &mut World, folder: &tempfile::TempDir) -> std::path::PathBuf {
    assert_ok(&api_line(editor, world, "song new"));
    sign_song(editor, world);
    assert_ok(&api_line(editor, world, "song save music/theme.mid"));
    let sidecar = folder.path().join("music/theme.mid.ron");
    let mut text = common::vfs::read_to_string(&sidecar).expect("the sidecar");
    text.push_str("\n// a part MIDI Evil added\n");
    common::vfs::write_string(&sidecar, &text).expect("another program's write");
    edit_song(editor, world);
    sidecar
}

#[test]
fn a_sidecar_changed_on_disk_parks_the_overwrite_prompt_whose_overwrite_saves() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    let sidecar = song_whose_map_changed_on_disk(&mut editor, &mut world, &folder);
    let theirs = common::vfs::read(&sidecar).expect("theirs");

    assert_eq!(editor.save_song(None), Ok(None), "parked, not saved");
    let target = folder.path().join("music/theme.mid");
    let parked = PendingSceneAction::OverwriteSong(target.clone(), OverwriteCause::MapChanged);
    assert_eq!(editor.scene_confirm.pending_action, Some(parked.clone()));
    let dialog = editor::ConfirmDialog::map_changed_on_disk(&sidecar);
    assert!(dialog.message.contains("map changed on disk since you loaded it"), "{}", dialog.message);

    editor.resolve_song_choice(parked.clone(), ConfirmChoice::Cancel);
    assert_eq!(common::vfs::read(&sidecar).ok(), Some(theirs), "Cancel writes nothing");
    assert!(editor.song.as_ref().is_some_and(SongDocument::is_dirty));

    editor.save_song(None).expect("parks again");
    editor.resolve_song_choice(parked, ConfirmChoice::Confirm);
    assert_eq!(editor.scene_confirm.pending_action, None, "one click closes the prompt");
    let written = common::vfs::read_to_string(&sidecar).expect("ours");
    assert!(written.contains("looping: false") && !written.contains("MIDI Evil"), "{written}");
    assert!(editor.song.as_ref().is_some_and(|song| !song.is_dirty()));
}

#[test]
fn the_api_refuses_a_changed_sidecar_naming_both_files_and_overwrite_saves_over_it() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    let sidecar = song_whose_map_changed_on_disk(&mut editor, &mut world, &folder);

    let refused = json(&api_line(&mut editor, &mut world, "song save"));
    assert_eq!(refused["error"]["kind"], "refused", "{refused}");
    let files = refused["error"]["files"].as_array().expect("the files, structured");
    assert_eq!(files.len(), 2, "{refused}");
    assert!(files[0]["path"].as_str().is_some_and(|path| path.ends_with("theme.mid")));
    assert_eq!(files[0]["loaded"], files[0]["on_disk"], "the score did not change");
    assert!(files[1]["path"].as_str().is_some_and(|path| path.ends_with("theme.mid.ron")));
    assert_ne!(files[1]["loaded"], files[1]["on_disk"], "the map did");
    assert_eq!(files[1]["on_disk"].as_str().map(str::len), Some(16), "a fingerprint in hex");
    assert!(editor.scene_confirm.pending_action.is_none(), "the API parks no prompt");

    assert_ok(&api_line(&mut editor, &mut world, "song save music/theme.mid overwrite"));
    let written = common::vfs::read_to_string(&sidecar).expect("ours");
    assert!(written.contains("looping: false") && !written.contains("MIDI Evil"), "{written}");
}

#[test]
fn song_open_of_the_open_song_rereads_its_map_from_disk() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    sign_song(&mut editor, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, "song save music/theme.mid"));
    let sidecar = folder.path().join("music/theme.mid.ron");
    let text = common::vfs::read_to_string(&sidecar).expect("the sidecar");
    common::vfs::write_string(&sidecar, &text.replace("looping: true", "looping: false")).expect("rewritten");

    assert_ok(&api_line(&mut editor, &mut world, "song open music/theme.mid"));
    assert!(editor.song.as_ref().is_some_and(|song| !song.file().looping), "the reload read the map");
}

#[test]
fn song_init_maps_an_unmapped_midi_and_refuses_a_mapped_one_and_an_unmapped_open_is_in_place_and_dirty() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    let score = SongDocument::new_song().expect("a blank song").data().write_smf().expect("exports");
    for name in ["lonely", "bare"] {
        common::vfs::write(&folder.path().join(format!("music/{name}.mid")), &score).expect("a .mid");
    }

    let initialized = json(&api_line(&mut editor, &mut world, "song init music/lonely.mid"));
    assert!(
        initialized["data"]["initialized"].as_str().is_some_and(|path| path.ends_with("music/lonely.mid.ron")),
        "{initialized}"
    );
    let again = json(&api_line(&mut editor, &mut world, "song init music/lonely.mid"));
    assert_eq!(again["error"]["kind"], "refused", "{again}");
    assert!(again["error"]["message"].as_str().is_some_and(|message| message.contains("already has its sidecar")));
    let opened = json(&api_line(&mut editor, &mut world, "song open music/lonely.mid"));
    assert_eq!(opened["data"]["dirty"], false, "{opened}");

    let bare = json(&api_line(&mut editor, &mut world, "song open music/bare.mid"));
    assert!(bare["data"]["path"].as_str().is_some_and(|path| path.ends_with("music/bare.mid")), "{bare}");
    assert_eq!(bare["data"]["dirty"], true, "the map is not on disk yet: {bare}");
    assert!(common::vfs::read(&folder.path().join("music/bare.mid.ron")).is_err(), "the open writes nothing");
}
