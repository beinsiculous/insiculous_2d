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
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Playing);
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
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Editing);
    harness.step(FRAME, &[]);
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Playing);
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
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Editing);
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

/// A game that asks for its song at `init`, as Tong does, and again on its first frame in a
/// session, as Tong's match start does.
struct SingsItsOwnSong {
    song: std::path::PathBuf,
    sang_in_session: bool,
}

impl engine_core::Game for SingsItsOwnSong {
    fn init(&mut self, ctx: &mut engine_core::prelude::GameContext) {
        ctx.audio.play_song(&self.song).expect("the game's song loads");
    }

    fn update(&mut self, ctx: &mut engine_core::prelude::GameContext) {
        if !self.sang_in_session {
            self.sang_in_session = true;
            ctx.audio.play_song(&self.song).expect("the game's song loads");
        }
    }
}

#[test]
fn the_editor_starts_silent_the_games_own_song_plays_in_a_session_and_stop_ends_it() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    drop(harness_with_a_saved_song(&folder));
    let song = folder.path().join("music/theme.mid");
    let mut editor = EditorGame::new(SingsItsOwnSong { song: song.clone(), sang_in_session: false });
    editor.prefs_slot = std::env::temp_dir()
        .join(format!("insiculous_test_editor_prefs_{}_silent_boot.json", std::process::id()));
    let config = GameConfig::new("Silent boot").with_asset_base_path(folder.path().to_string_lossy().to_string());
    let mut harness = GameHarness::new(editor, config);
    harness.step(FRAME, &[]);
    assert_eq!(harness.audio().song_label(), None, "the game's init-time song is stopped: editing is silent");

    harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Play, ctx.world));
    for _ in 0..3 {
        harness.step(FRAME, &[]);
    }
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Playing);
    let label = song.display().to_string();
    assert_eq!(harness.audio().song_label(), Some(label.as_str()), "the game's own music plays in its session");

    harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Stop, ctx.world));
    harness.step(FRAME, &[]);
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Editing);
    assert_eq!(harness.audio().song_label(), None, "and Stop ends it");
}

/// A game that asks for its song at `init` and never again: the editor demo's shape.
struct SingsAtInitOnly {
    song: std::path::PathBuf,
}

impl engine_core::Game for SingsAtInitOnly {
    fn init(&mut self, ctx: &mut engine_core::prelude::GameContext) {
        ctx.audio.play_song(&self.song).expect("the game's song loads");
    }

    fn update(&mut self, _ctx: &mut engine_core::prelude::GameContext) {}
}

fn silent_editor_over<G: engine_core::Game>(game: G, folder: &std::path::Path, slot: &str) -> GameHarness<EditorGame<G>> {
    let mut editor = EditorGame::new(game);
    editor.prefs_slot = std::env::temp_dir().join(format!("insiculous_test_editor_prefs_{}_{slot}.json", std::process::id()));
    let config = GameConfig::new("Init music").with_asset_base_path(folder.to_string_lossy().to_string());
    let mut harness = GameHarness::new(editor, config);
    harness.step(FRAME, &[]);
    harness
}

#[test]
fn a_games_init_time_song_is_silent_while_editing_plays_through_each_session_and_stop_ends_it() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    drop(harness_with_a_saved_song(&folder));
    let song = folder.path().join("music/theme.mid");
    let mut harness = silent_editor_over(SingsAtInitOnly { song: song.clone() }, folder.path(), "init_song");
    assert_eq!(harness.audio().song_label(), None, "editing is silent");
    assert_eq!(harness.game().init_music, super::init_music::InitMusic::Song(song.clone()), "but the request is remembered");

    let label = song.display().to_string();
    for session in 1..=2 {
        harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Play, ctx.world));
        harness.steps(3, FRAME);
        assert_eq!(harness.game().editor.play_state(), EditorPlayState::Playing);
        assert_eq!(harness.audio().song_label(), Some(label.as_str()), "session {session} asks for the game's song again");
        harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Stop, ctx.world));
        harness.step(FRAME, &[]);
        assert_eq!(harness.game().editor.play_state(), EditorPlayState::Editing);
        assert_eq!(harness.audio().song_label(), None, "and Stop ends it");
    }
}

/// A game that asks for a recorded track at `init` and never again.
struct PlaysATrackAtInit {
    track: std::path::PathBuf,
}

impl engine_core::Game for PlaysATrackAtInit {
    fn init(&mut self, ctx: &mut engine_core::prelude::GameContext) {
        ctx.audio.play_music(&self.track).expect("the track loads");
    }

    fn update(&mut self, _ctx: &mut engine_core::prelude::GameContext) {}
}

#[test]
fn a_games_init_time_track_is_asked_for_again_unless_a_composer_song_takes_the_slot() {
    const SINE_WAV: &[u8] = include_bytes!("../../../music_midi/fixtures/sine_48000_mono.wav");
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    stock_sound_font(folder.path());
    let track = folder.path().join("sounds/track.wav");
    common::vfs::write(&track, SINE_WAV).expect("written");
    let mut harness = silent_editor_over(PlaysATrackAtInit { track: track.clone() }, folder.path(), "init_track");
    assert!(harness.audio().music_request().is_none(), "editing is silent");
    assert!(
        matches!(&harness.game().init_music, super::init_music::InitMusic::Track(request) if request.path == track),
        "but the request is remembered: {:?}",
        harness.game().init_music
    );

    harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Play, ctx.world));
    harness.steps(3, FRAME);
    assert!(harness.audio().music_request().is_some_and(|request| request.path == track), "the session asks for the track again");
    harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Stop, ctx.world));
    harness.step(FRAME, &[]);
    assert!(harness.audio().music_request().is_none(), "Stop ends it");

    let saved = folder.path().join("music/theme.mid");
    harness.context(|editor, _| {
        editor.song = Some(melody_song());
        editor.follow_song();
        editor.save_song_to(saved.clone()).expect("the composer's song saves");
    });
    harness.context(|editor, ctx| editor.handle_play_action(PlayControlAction::Play, ctx.world));
    harness.steps(3, FRAME);
    assert_eq!(harness.game().editor.play_state(), EditorPlayState::Playing);
    let label = saved.display().to_string();
    assert_eq!(harness.audio().song_label(), Some(label.as_str()), "the saved song is the session's music");
    assert!(harness.audio().music_request().is_none(), "and the game's track is not asked for");
}
