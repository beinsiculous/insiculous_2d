//! The song beside the scene, through the real `EditorGame`: Play and the song actions
//! park behind the song's prompt (Play's arms by whether a saved song exists), a scene
//! action never prompts for the song nor a song action for the scene, the API refuses a
//! dirty song without `discard`, and scene and song saves leave each other's dirtiness
//! alone. The preview verbs are `song_preview_tests.rs`'s.

use ecs::World;
use editor::{ConfirmChoice, EditorPlayState, PlayControlAction, SongDocument};

use super::scene_confirm::PendingSceneAction;
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
