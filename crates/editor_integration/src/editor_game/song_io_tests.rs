//! The song beside the scene, through the real `EditorGame`: Play and the song actions
//! park behind the song's prompt (Play's arms by whether a saved song exists), a scene
//! action never prompts for the song nor a song action for the scene, the API refuses a
//! dirty song without `discard`, scene and song saves leave each other's dirtiness alone,
//! and the preview verbs are refused until the preview exists.

use ecs::World;
use editor::{ConfirmChoice, EditorPlayState, PlayControlAction, SongDocument};

use super::scene_confirm::PendingSceneAction;
use super::test_support::{api_line, assert_ok, dirty_editor, editor_game, DummyGame};
use super::EditorGame;

/// A session whose asset base is a temporary folder, so saves land somewhere real.
fn session_in(folder: &tempfile::TempDir) -> EditorGame<DummyGame> {
    let mut editor = editor_game();
    editor.asset_base = folder.path().to_path_buf();
    editor
}

fn folder() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary asset folder")
}

fn json(response: &str) -> serde_json::Value {
    serde_json::from_str(response).expect("one line of JSON")
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
    editor.song = Some(SongDocument::new_song().expect("a blank song"));

    editor.handle_play_action(PlayControlAction::Play, &mut world);
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
        ConfirmChoice::Alt,
        &mut world,
    );
    assert_eq!(
        editor.scene_confirm.pending_action,
        Some(PendingSceneAction::StartPlay),
        "no such arm"
    );

    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Confirm,
        &mut world,
    );
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
    assert_ok(&api_line(
        &mut editor,
        &mut world,
        "song save music/theme.mid",
    ));
    edit_song(&mut editor, &mut world);

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(
        editor.scene_confirm.pending_action,
        Some(PendingSceneAction::StartPlay)
    );
    assert!(editor.song_saved_exists());

    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Cancel,
        &mut world,
    );
    assert_eq!(
        editor.editor.play_state(),
        EditorPlayState::Editing,
        "Cancel stays put"
    );
    assert_eq!(editor.scene_confirm.pending_action, None);

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.resolve_song_choice(
        PendingSceneAction::StartPlay,
        ConfirmChoice::Alt,
        &mut world,
    );
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
    editor.perform_song_action(PendingSceneAction::NewSong, &mut world);
    assert!(
        editor.song.as_ref().is_some_and(SongDocument::is_dirty),
        "a new song is dirty"
    );

    let mut clean_scene = session_in(&folder);
    clean_scene.song = Some(SongDocument::new_song().expect("a blank song"));
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
        ConfirmChoice::Alt,
        &mut world,
    );
    assert!(clean_scene.song.is_none(), "Discard closes it");
}

#[test]
fn a_song_action_is_refused_during_a_play_session_with_no_dialog() {
    let mut editor = editor_game();
    let mut world = World::new();
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    editor.song = Some(SongDocument::new_song().expect("a blank song"));

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
fn open_song_looks_for_a_file_that_exists_where_save_looks_for_a_free_name() {
    let folder = folder();
    let mut editor = session_in(&folder);
    let mut world = World::new();
    let untitled = folder.path().join("music/untitled.mid");
    assert_eq!(
        editor.song_path_to_open(),
        untitled,
        "no song: the fixed default"
    );

    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    assert_ok(&api_line(&mut editor, &mut world, "song save"));
    assert_eq!(
        editor.song_path_to_open(),
        untitled,
        "beside the open song's file: the same name here, since the song is untitled.mid"
    );
    assert_eq!(
        editor.default_song_path(),
        folder.path().join("music/untitled_2.mid"),
        "Save's default skips the taken name; Open must not"
    );
    assert_ok(&api_line(&mut editor, &mut world, "song save music/levels/boss.mid"));
    assert_eq!(
        editor.song_path_to_open(),
        folder.path().join("music/levels/untitled.mid"),
        "beside the open song's file, never the file itself"
    );

    editor.close_song();
    editor.perform_song_action(
        PendingSceneAction::OpenSong(editor.song_path_to_open()),
        &mut world,
    );
    assert_eq!(
        editor.song.as_ref().and_then(SongDocument::path),
        Some(untitled.as_path()),
        "Open Song opens the file it names"
    );

    edit_song(&mut editor, &mut world);
    assert_ok(&api_line(&mut editor, &mut world, "song undo"));
    assert!(editor.song.as_ref().is_some_and(|song| song.history().can_redo()));
    editor.perform_song_action(PendingSceneAction::OpenSong(untitled.clone()), &mut world);
    assert!(
        editor.song.as_ref().is_some_and(|song| song.history().can_redo()),
        "the open song's own file is not reloaded under its history"
    );
}

#[test]
fn the_preview_verbs_are_refused_naming_the_batch_that_builds_them() {
    let mut editor = editor_game();
    let mut world = World::new();
    assert_ok(&api_line(&mut editor, &mut world, "song new"));
    for line in ["song play", "song stop", "song seek 4", "song trigger jump"] {
        let response = json(&api_line(&mut editor, &mut world, line));
        assert_eq!(response["error"]["kind"], "refused", "{line}");
        assert!(
            response["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("batch 11")),
            "{response}"
        );
    }
}
