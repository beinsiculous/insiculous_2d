//! Play with a song open hears the saved song: scene Play arms, the request frame's drain
//! plays the saved file, and the next frame starts the session — through Play and through
//! the dirty prompt's "Play saved"; Stop while armed drops the play; Pause holds the song;
//! Stop flushes the sources, stops the slot and hands back a stopped preview.

use ecs::World;
use editor::{ConfirmChoice, EditorPlayState, PlayControlAction};
use engine_core::prelude::{AudioManager, GameConfig, SoundSources};
use engine_core::test_support::GameHarness;

use super::scene_confirm::PendingSceneAction;
use super::test_support::{api_line, composer_session, editor_game, melody_song, stock_sound_font, DummyGame};
use super::EditorGame;

const FRAME: f32 = 1.0 / 60.0;

fn json(response: &str) -> serde_json::Value {
    serde_json::from_str(response).expect("one line of JSON")
}

/// A harness over a fresh session whose asset folder holds the bank and a saved melody at
/// `music/theme.mid`, its first frame run.
fn harness_with_a_saved_song(folder: &tempfile::TempDir) -> GameHarness<EditorGame<DummyGame>> {
    stock_sound_font(folder.path());
    let config = GameConfig::new("Song preview").with_asset_base_path(folder.path().to_string_lossy().to_string());
    let mut harness = GameHarness::new(editor_game(), config);
    harness.step(FRAME, &[]);
    harness.context(|editor, ctx| {
        editor.song = Some(melody_song());
        editor.follow_song();
        let saved = json(&api_line(editor, ctx.world, "song save music/theme.mid"));
        assert_eq!(saved["ok"], true, "{saved}");
    });
    harness
}

fn play_state(harness: &mut GameHarness<EditorGame<DummyGame>>) -> EditorPlayState {
    harness.game().editor.play_state()
}

#[test]
fn a_preview_plays_then_play_parks_it_and_the_session_hears_the_saved_song_the_frame_after() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut harness = harness_with_a_saved_song(&folder);
    harness.context(|editor, _| editor.preview_play(0).expect("plays"));
    harness.step(FRAME, &[]);
    assert!(harness.audio().is_song_playing(), "the transport's Play is heard");
    assert!(harness.game().song_preview.is_playing());

    harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Play, ctx.world));
    assert!(!harness.game().song_preview.is_playing(), "Play parks the preview");
    let refused = harness.context(|editor, _| editor.preview_play(0));
    assert!(refused.is_err(), "a preview pushed while Play is armed is dropped");
    harness.step(FRAME, &[]);
    assert_eq!(play_state(&mut harness), EditorPlayState::Editing, "the request frame plays the song, not the game");
    assert!(harness.audio().is_song_playing(), "the saved song took the slot");
    harness.step(FRAME, &[]);
    assert_eq!(play_state(&mut harness), EditorPlayState::Playing);
    assert!(harness.audio().is_song_playing());
    assert!(!harness.game().song_preview.is_playing(), "the preview stays parked through the session");
}

#[test]
fn play_saved_from_the_dirty_prompt_takes_the_same_two_frames() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut harness = harness_with_a_saved_song(&folder);
    harness.context(|editor, ctx| {
        let edited = json(&api_line(editor, ctx.world, r#"song edit {"SetLoop": {"looping": false}}"#));
        assert_eq!(edited["ok"], true);
        editor.handle_play_action(PlayControlAction::Play, ctx.world);
        assert_eq!(editor.scene_confirm.pending_action, Some(PendingSceneAction::StartPlay));
        editor.resolve_song_choice(PendingSceneAction::StartPlay, ConfirmChoice::Alt);
    });
    harness.step(FRAME, &[]);
    assert_eq!(play_state(&mut harness), EditorPlayState::Editing);
    harness.step(FRAME, &[]);
    assert_eq!(play_state(&mut harness), EditorPlayState::Playing);
    assert!(harness.audio().is_song_playing(), "the saved song, the edits kept unsaved");
    assert!(harness.game().song.as_ref().is_some_and(editor::SongDocument::is_dirty));
}

#[test]
fn stop_while_play_is_armed_drops_it_and_starts_nothing() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut harness = harness_with_a_saved_song(&folder);
    harness.context(|editor, ctx| {
        editor.handle_play_action(PlayControlAction::Play, ctx.world);
        assert!(editor.song_preview.is_armed());
        editor.handle_play_action(PlayControlAction::Pause, ctx.world);
        assert!(editor.song_preview.is_armed(), "Pause while armed is nothing");
        assert!(!editor.handle_play_action(PlayControlAction::Stop, ctx.world));
        assert!(!editor.song_preview.is_armed(), "Stop drops it");
    });
    harness.steps(2, FRAME);
    assert_eq!(play_state(&mut harness), EditorPlayState::Editing);
    assert!(!harness.audio().is_song_playing(), "no song was started");
}

#[test]
fn pause_holds_the_song_and_stop_flushes_the_sources_and_returns_a_stopped_preview() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let mut world = World::new();
    assert_eq!(json(&api_line(&mut editor, &mut world, "song save music/theme.mid"))["ok"], true);
    let mut audio = AudioManager::disabled();
    audio.set_asset_base(folder.path());
    let mut sources = SoundSources::default();
    let drain = |editor: &mut EditorGame<DummyGame>, audio: &mut AudioManager, sources: &mut SoundSources| {
        std::thread::sleep(std::time::Duration::from_millis(15));
        audio.update();
        editor.apply_song_intents(audio, sources);
    };

    editor.preview_play(0).expect("plays");
    drain(&mut editor, &mut audio, &mut sources);
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    let report = editor.apply_song_intents(&mut audio, &mut sources);
    assert_eq!(report.session_play, Some(Ok(())));
    editor.start_armed_play(&mut world);
    assert!(editor.editor.is_playing());
    let menu = sources.menu();

    drain(&mut editor, &mut audio, &mut sources);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    drain(&mut editor, &mut audio, &mut sources);
    let held = audio.song_position().expect("a paused song keeps its place");
    drain(&mut editor, &mut audio, &mut sources);
    assert_eq!(audio.song_position().map(|position| position.seconds), Some(held.seconds), "Pause holds the song");
    assert!(!audio.is_song_playing());

    editor.handle_play_action(PlayControlAction::Play, &mut world);
    drain(&mut editor, &mut audio, &mut sources);
    assert!(audio.is_song_playing(), "Resume continues it");

    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    editor.handle_play_action(PlayControlAction::Stop, &mut world);
    drain(&mut editor, &mut audio, &mut sources);
    assert!(editor.editor.is_editing());
    assert_eq!(audio.song_position(), None, "Stop stopped the slot it started");
    assert_ne!(sources.menu(), menu, "and flushed every source: the menu activates anew");
    assert!(!editor.song_preview.is_playing());
    assert_eq!(editor.editor.composer.playhead_tick, None);

    editor.preview_play(0).expect("the transport's Play hears the document again");
    drain(&mut editor, &mut audio, &mut sources);
    assert!(audio.is_song_playing());
}

#[test]
fn pause_and_resume_are_refused_while_the_sessions_song_waits_for_the_first_click() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut editor = composer_session(folder.path(), melody_song());
    let mut world = World::new();
    assert_eq!(json(&api_line(&mut editor, &mut world, "song save music/theme.mid"))["ok"], true);
    let mut audio = AudioManager::disabled();
    audio.set_asset_base(folder.path());
    let mut sources = SoundSources::default();
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    // The drain with no follow: the song has published nothing yet, as one waiting for the
    // web's gesture never does.
    editor.drain_song_intents(&mut audio, &mut sources);
    editor.start_armed_play(&mut world);
    assert!(editor.editor.is_playing());

    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.editor.is_playing(), "the session would part from its sound");
    let status = editor.editor.status_bar.message().unwrap_or_default().to_string();
    assert!(status.contains("waiting for the first click"), "{status}");

    editor.apply_song_intents(&mut audio, &mut sources);
    editor.handle_play_action(PlayControlAction::Pause, &mut world);
    assert!(editor.editor.is_paused(), "heard once, it pauses");
}

#[test]
fn a_save_refused_for_the_songs_provenance_from_the_play_prompt_opens_its_author_field() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut editor = composer_session(folder.path(), editor::SongDocument::new_song().expect("a blank song"));
    let mut world = World::new();
    editor.leave_composer();
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert_eq!(editor.scene_confirm.pending_action, Some(PendingSceneAction::StartPlay));

    editor.resolve_song_choice(PendingSceneAction::StartPlay, ConfirmChoice::Confirm);
    let status = editor.editor.status_bar.message().unwrap_or_default().to_string();
    assert!(status.contains("author and license"), "{status}");
    assert_eq!(editor.scene_confirm.pending_action, None, "the dialog gives the fields the screen");
    assert!(!editor.song_preview.is_armed(), "and Play waits for a save that holds");
    let composer = &editor.editor.composer;
    assert_eq!(composer.focus, editor::composer::ComposerFocus::Sidebar);
    assert_eq!(composer.sidebar_section, editor::composer::SidebarSection::Song);
    assert_eq!(composer.request, Some(editor::composer::ControlRequest::FocusSongAuthor));
}
