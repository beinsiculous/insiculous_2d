//! The song preview through the real `EditorGame` and a headless audio manager: the
//! transport, Space and the API push intents the frame's drain plays; the preview follows
//! its song — waiting for the web's first click, never reaching a device, stopping early or
//! at its end — and stops when its document or its song changes; a binding's Trigger and
//! the idle throttle wait for it to be heard.

use std::path::Path;

use ecs::World;
use editor::composer::transport::status_text;
use editor::{Gesture, SongEdit};
use engine_core::prelude::{AudioManager, SoundSources};

use super::song_preview::{SlotReadings, SongAudioIntent};
use super::test_support::{api_line, composer_session, melody_song, second_track_holds_the_notes, DummyGame};
use super::EditorGame;

/// A manager with no device that plays a song on its virtual clock, its asset base the
/// session's.
fn headless_audio(editor: &EditorGame<DummyGame>) -> AudioManager {
    let mut audio = AudioManager::disabled();
    audio.set_asset_base(&editor.asset_base);
    audio
}

/// One frame's drain and follow, after the virtual clock has rendered a few blocks.
fn frame(editor: &mut EditorGame<DummyGame>, audio: &mut AudioManager, sources: &mut SoundSources) {
    editor.apply_song_intents(audio, sources);
    std::thread::sleep(std::time::Duration::from_millis(15));
    audio.update();
    editor.follow_song_slot(SlotReadings::read(audio));
}

fn status(editor: &EditorGame<DummyGame>) -> String {
    editor.editor.status_bar.message().unwrap_or_default().to_string()
}

fn json(response: &str) -> serde_json::Value {
    serde_json::from_str(response).expect("one line of JSON")
}

/// What a song waiting for the web's first click reads: no position, no device.
const WAITING_FOR_THE_GESTURE: SlotReadings = SlotReadings {
    position: None,
    published: false,
    playing: false,
    ended: false,
    enabled: false,
};

fn session() -> (tempfile::TempDir, EditorGame<DummyGame>) {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let editor = composer_session(folder.path(), melody_song());
    (folder, editor)
}

#[test]
fn the_transport_plays_the_document_from_the_rolls_cursor_and_stop_ends_it() {
    let (_folder, mut editor) = session();
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    editor.editor.composer.cursor.tick = 960;

    editor.run_composer_actions(vec![editor::composer::ComposerAction::PreviewPlayStop]);
    assert!(editor.editor.composer.preview_playing, "the transport shows it at once");
    frame(&mut editor, &mut audio, &mut sources);
    assert!(audio.is_song_playing(), "the drain played the document");
    let playhead = editor.editor.composer.playhead_tick.expect("a sounding preview has a playhead");
    assert!(playhead >= 960, "it starts at the cursor's tick, never before: {playhead}");
    let song = editor.song.as_ref().expect("a song");
    assert!(status_text(song, &editor.editor.composer).starts_with("Playing · bar 1 · beat 3 · 120 BPM · 4/4"));
    assert!(editor.song_preview.is_audible());
    assert!(!editor.idle_throttle_allowed(), "a moving playhead keeps the page awake");

    editor.run_composer_actions(vec![editor::composer::ComposerAction::PreviewPlayStop]);
    frame(&mut editor, &mut audio, &mut sources);
    assert!(!audio.is_song_playing());
    assert_eq!(editor.editor.composer.playhead_tick, None);
    let song = editor.song.as_ref().expect("a song");
    assert!(status_text(song, &editor.editor.composer).starts_with("Playhead bar 1 · beat 1 · tick 0"));
    assert!(editor.idle_throttle_allowed());
}

#[test]
fn space_toggles_the_preview_and_a_stop_while_stopped_sends_nothing() {
    let (_folder, mut editor) = session();
    editor.preview_stop();
    assert!(editor.song_preview.push(SongAudioIntent::Seek { beat: 0 }), "the queue is open");
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    frame(&mut editor, &mut audio, &mut sources);
    assert!(!audio.is_song_playing(), "a seek with no preview reaches nothing");

    editor.run_composer_key(editor::composer::ComposerKeyAction::PreviewToggle);
    frame(&mut editor, &mut audio, &mut sources);
    assert!(audio.is_song_playing(), "Space plays");
    editor.dispatch_song_action(editor::EditorAction::SongPlayStop);
    frame(&mut editor, &mut audio, &mut sources);
    assert!(!audio.is_song_playing(), "and stops");
}

#[test]
fn a_song_waiting_for_the_first_click_stays_playing_unheard_and_a_seek_plays_it_again_from_there() {
    let (_folder, mut editor) = session();
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    editor.preview_play(0).expect("plays");
    editor.drain_song_intents(&mut audio, &mut sources);
    editor.follow_song_slot(WAITING_FOR_THE_GESTURE);

    assert!(editor.song_preview.is_playing(), "the gesture-pending window is not an end");
    assert!(!editor.song_preview.is_audible());
    assert!(editor.idle_throttle_allowed(), "a song nobody hears holds no frames");
    let song = editor.song.as_ref().expect("a song");
    assert_eq!(status_text(song, &editor.editor.composer), "Playing · waiting for the first click");
    let refused = editor.preview_trigger("jump").expect_err("a trigger would be a silent Ok");
    assert!(refused.contains("press Play"), "{refused}");

    editor.preview_seek(4).expect("a seek while waiting");
    let tick = editor.song.as_ref().map(|song| song.data().tempo_map().tick_at_beat(4)).expect("a song");
    let (mut second_audio, mut second_sources) = (headless_audio(&editor), SoundSources::default());
    let report = editor.drain_song_intents(&mut second_audio, &mut second_sources);
    assert_eq!(report.session_play, None);
    assert!(second_audio.is_song_playing(), "the seek issued the play again, from beat 4 (tick {tick})");
}

#[test]
fn a_song_that_never_reaches_a_connected_device_ends_the_preview_saying_so() {
    let (_folder, mut editor) = session();
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    editor.preview_play(0).expect("plays");
    editor.drain_song_intents(&mut audio, &mut sources);
    editor.follow_song_slot(SlotReadings { enabled: true, ..WAITING_FOR_THE_GESTURE });

    assert!(!editor.song_preview.is_playing());
    assert!(!editor.editor.composer.preview_playing);
    assert!(status(&editor).contains("could not reach the device"), "{}", status(&editor));
}

#[test]
fn once_heard_a_song_that_stops_ends_the_preview_silently_at_its_end_and_early_otherwise() {
    for (ended, word) in [(true, None), (false, Some("stopped early"))] {
        let (_folder, mut editor) = session();
        let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
        editor.preview_play(0).expect("plays");
        frame(&mut editor, &mut audio, &mut sources);
        assert!(editor.song_preview.is_audible());
        editor.editor.status_bar.show_message("before");

        let stopped = SlotReadings { playing: false, ended, ..SlotReadings::read(&audio) };
        editor.follow_song_slot(stopped);
        assert!(!editor.song_preview.is_playing(), "ended: {ended}");
        assert_eq!(editor.editor.composer.playhead_tick, None);
        match word {
            Some(word) => assert!(status(&editor).contains(word), "{}", status(&editor)),
            None => assert_eq!(status(&editor), "before", "the score's end needs no word"),
        }
    }
}

#[test]
fn an_edit_and_an_undo_under_a_playing_preview_each_stop_it_and_say_so() {
    let (_folder, mut editor) = session();
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    let mut world = World::new();
    for change in ["edit", "undo"] {
        editor.preview_play(0).expect("plays");
        frame(&mut editor, &mut audio, &mut sources);
        assert!(audio.is_song_playing(), "{change}: playing first");
        match change {
            "edit" => {
                editor.apply_composer_edit(SongEdit::SetLoop { looping: false }, Gesture::Discrete);
            }
            _ => {
                let response = json(&api_line(&mut editor, &mut world, "song undo"));
                assert_eq!(response["ok"], true, "{response}");
            }
        }
        frame(&mut editor, &mut audio, &mut sources);
        assert!(!audio.is_song_playing(), "{change}: the prepared score is the old document");
        assert!(!editor.song_preview.is_playing());
        assert!(status(&editor).contains("stopped by the edit"), "{change}: {}", status(&editor));
    }
}

#[test]
fn a_song_replaced_under_a_playing_preview_stops_it() {
    let (_folder, mut editor) = session();
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    editor.preview_play(0).expect("plays");
    frame(&mut editor, &mut audio, &mut sources);
    editor.new_song();
    frame(&mut editor, &mut audio, &mut sources);
    assert!(!audio.is_song_playing(), "the old song does not play on under the new one");
    assert!(!editor.song_preview.is_playing());
    assert_eq!(status(&editor), "New song", "a replacement says what replaced it, not an edit");
}

#[test]
fn the_api_plays_from_a_whole_beat_triggers_only_a_bound_action_while_heard_and_seeks_only_a_playing_preview() {
    let folder = tempfile::tempdir().expect("a temporary asset folder");
    let mut editor = composer_session(folder.path(), second_track_holds_the_notes());
    let mut world = World::new();
    // The lead plays the binding's part; the pad's long note is the background the song lasts by.
    let edit = r#"song edit {"Batch": [
        {"AddNote": {"track": 1, "tick": 0, "key": 48, "velocity": 90, "duration_ticks": 1920}},
        {"AddPart": {"part": {"name": "lead", "track": {"Index": 2}}}},
        {"AddInstrument": {"instrument": {"name": "sine", "source": {"SoundFont": {"path": "soundfonts/cast.sf2", "bank": 0, "preset": 0}}}}},
        {"SetBinding": {"binding": {"action": "jump", "part": "lead", "instrument": "sine"}}}]}"#;
    let line = edit.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(json(&api_line(&mut editor, &mut world, &line))["ok"], true);

    let seek = json(&api_line(&mut editor, &mut world, "song seek 2"));
    assert_eq!(seek["error"]["kind"], "refused", "nothing to seek: {seek}");
    let early = json(&api_line(&mut editor, &mut world, "song trigger jump"));
    assert_eq!(early["error"]["kind"], "refused", "{early}");
    let played = json(&api_line(&mut editor, &mut world, "song play 2"));
    assert_eq!(played["data"]["from_tick"], 960, "beat 2 at 480 ticks a quarter: {played}");

    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    frame(&mut editor, &mut audio, &mut sources);
    assert!(editor.song_preview.is_audible(), "{}", status(&editor));
    let unknown = json(&api_line(&mut editor, &mut world, "song trigger fall"));
    assert_eq!(unknown["error"]["kind"], "not_found", "{unknown}");
    let triggered = json(&api_line(&mut editor, &mut world, "song trigger jump"));
    assert_eq!(triggered["data"]["triggered"], "jump", "{triggered}");
    assert_eq!(json(&api_line(&mut editor, &mut world, "song seek 1"))["ok"], true);
    frame(&mut editor, &mut audio, &mut sources);
    let heard = audio.song_snapshot().map(|snapshot| snapshot.conductor.accepted_triggers);
    assert_eq!(heard, Some(1), "the trigger reached the conductor: {}", status(&editor));

    assert_eq!(json(&api_line(&mut editor, &mut world, "song stop"))["data"]["preview"], "stopped");
    frame(&mut editor, &mut audio, &mut sources);
    assert!(!audio.is_song_playing());
}

#[test]
fn a_trigger_button_is_refused_on_the_status_bar_until_the_preview_is_heard() {
    let (_folder, mut editor) = session();
    editor.preview_trigger_binding(0);
    assert_eq!(status(&editor), "", "a song with no binding has no button to press");
    let mut world = World::new();
    let line = r#"song edit {"Batch": [{"AddPart": {"part": {"name": "lead", "track": {"Index": 1}}}}, {"AddInstrument": {"instrument": {"name": "sine", "source": {"SoundFont": {"path": "soundfonts/cast.sf2", "bank": 0, "preset": 0}}}}}, {"SetBinding": {"binding": {"action": "jump", "part": "lead", "instrument": "sine"}}}]}"#;
    assert_eq!(json(&api_line(&mut editor, &mut world, line))["ok"], true);
    editor.preview_trigger_binding(0);
    assert!(status(&editor).contains("Trigger jump"), "{}", status(&editor));
}

/// The asset folder the session saves into, as the drain resolves it.
fn saved_song(editor: &EditorGame<DummyGame>) -> Option<&Path> {
    editor.song.as_ref().and_then(|song| song.path())
}

#[test]
fn a_saved_song_missing_a_sample_keeps_the_preview_shows_the_error_and_starts_no_session() {
    let (folder, mut editor) = session();
    let mut world = World::new();
    const SINE_WAV: &[u8] = include_bytes!("../../../music_midi/fixtures/sine_48000_mono.wav");
    common::vfs::write(&folder.path().join("sounds/blip.wav"), SINE_WAV).expect("written");
    let add = r#"song edit {"AddInstrument": {"instrument": {"name": "blip", "source": {"Sample": {"path": "sounds/blip.wav", "root_note": 60}}, "provenance": {"author": "2026 Jesse", "license": "MIT"}}}}"#;
    assert_eq!(json(&api_line(&mut editor, &mut world, add))["ok"], true);
    assert_eq!(json(&api_line(&mut editor, &mut world, "song save music/theme.mid"))["ok"], true);
    assert!(saved_song(&editor).is_some());
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    editor.preview_play(0).expect("plays");
    frame(&mut editor, &mut audio, &mut sources);
    assert!(editor.song_preview.is_audible());

    common::vfs::remove_prefix(&folder.path().join("music/samples/theme/blip.wav")).expect("removed");
    editor.handle_play_action(editor::PlayControlAction::Play, &mut world);
    assert!(!editor.song_preview.is_playing(), "Play parks the preview");
    let report = editor.apply_song_intents(&mut audio, &mut sources);
    assert!(matches!(report.session_play, Some(Err(_))), "{report:?}");
    editor.start_armed_play(&mut world);

    assert!(editor.editor.is_editing(), "no session started");
    assert!(editor.song_preview.is_playing(), "the preview it parked plays on");
    assert!(audio.is_song_playing());
    assert!(status(&editor).contains("Play could not start the song"), "{}", status(&editor));
}

#[test]
fn play_from_a_cursor_past_the_last_note_starts_from_the_top() {
    let (_folder, mut editor) = session();
    // Not looping, so a play from the song's end would end at once instead of wrapping.
    editor.apply_composer_edit(SongEdit::SetLoop { looping: false }, Gesture::Discrete);
    let (mut audio, mut sources) = (headless_audio(&editor), SoundSources::default());
    editor.editor.composer.cursor.tick = 1200;
    editor.preview_play_stop();
    frame(&mut editor, &mut audio, &mut sources);
    assert!(editor.song_preview.is_playing(), "{}", status(&editor));
    let playhead = editor.editor.composer.playhead_tick.expect("sounding");
    assert!(playhead < 1200, "the last note ends at 1200, so Play starts at the top: {playhead}");
}
