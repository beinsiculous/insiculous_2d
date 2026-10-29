//! A recorded sound edited on disk, through the real `EditorGame`: what the scene and the
//! sound tier put in the watched set, a failed path's arrival caught at the first poll, and
//! `ReloadSample` — the same handle serving the new file, a failed path retried, a bad file
//! keeping the old sound, and the same in a play session.

use std::collections::BTreeSet;

use ecs::{SoundEmitter, SoundRef, World};
use editor::console::Severity;
use editor::external_tools::{FileChange, FileWatch};

use super::hot_reload::WatchOwner;
use super::hot_reload_tests::Session;
use super::test_support::press_play;

/// A silent 16-bit WAV of `frames` frames at 44.1 kHz with `channels` channels.
fn wav(channels: u16, frames: u32) -> Vec<u8> {
    let data_bytes = frames * u32::from(channels) * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&44_100u32.to_le_bytes());
    bytes.extend_from_slice(&(44_100 * u32::from(channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    bytes.resize(bytes.len() + data_bytes as usize, 0);
    bytes
}

fn write_wav(session: &Session, relative: &str, bytes: &[u8]) {
    common::vfs::write(&session.path(relative), bytes).expect("the sound is written");
}

/// A session whose sound tier has loaded `sounds/hum.wav`, the watcher's set taken.
fn humming() -> (Session, audio::SoundHandle) {
    let mut session = Session::new();
    write_wav(&session, "sounds/hum.wav", &wav(1, 1));
    let handle = session.assets.resolve_sound("sounds/hum.wav", &mut session.audio).expect("the sound loads");
    session.watch_now();
    (session, handle)
}

#[test]
fn an_emitters_sound_and_the_sound_tiers_paths_are_watched_and_a_wav_nobody_names_is_not() {
    let (mut session, _) = humming();
    assert!(session.assets.resolve_sound("sounds/late.wav", &mut session.audio).is_err(), "not there yet");
    write_wav(&session, "sounds/unnamed.wav", &wav(1, 1));
    let mut world = World::new();
    let beeper = world.create_entity();
    let beep = SoundEmitter { sound: SoundRef::Path("sounds/beep.wav".into()), ..SoundEmitter::default() };
    world.add_component(&beeper, beep).ok();
    let silent = world.create_entity();
    world.add_component(&silent, SoundEmitter { sound: SoundRef::Path(String::new()), ..SoundEmitter::default() }).ok();

    let watched = session.editor.watched_paths(&world, &session.assets);

    let owner = |sound: &str| watched.get(&session.path(sound)).map(|owners| owners.iter().cloned().collect::<Vec<_>>());
    for sound in ["sounds/beep.wav", "sounds/hum.wav", "sounds/late.wav"] {
        assert_eq!(owner(sound), Some(vec![WatchOwner::Sound(sound.to_string())]), "{sound}");
    }
    assert_eq!(watched.len(), 3, "an empty path and a file nothing names are not watched: {watched:?}");
    assert_eq!(session.editor.expected_absent_paths(&session.assets), BTreeSet::from([session.path("sounds/late.wav")]));
}

#[test]
fn a_failed_sounds_file_present_at_the_first_poll_is_reported_changed() {
    let (mut session, _) = humming();
    assert!(session.assets.resolve_sound("sounds/late.wav", &mut session.audio).is_err());
    write_wav(&session, "sounds/late.wav", &wav(1, 1));
    let mut watch = FileWatch::new();
    let watched = session.editor.watched_paths(&session.world, &session.assets);
    watch.set_registered(watched.into_keys().collect(), &session.editor.expected_absent_paths(&session.assets));

    assert!(watch.poll().is_empty(), "held one poll, as every change is");
    assert_eq!(
        watch.poll(),
        [FileChange::Changed(session.path("sounds/late.wav"))],
        "the failed path's file is a change; the loaded one's first reading is its baseline"
    );
}

#[test]
fn a_changed_wav_is_served_from_the_same_handle_after_the_dispatch_and_a_new_format_is_named() {
    let (mut session, handle) = humming();
    write_wav(&session, "sounds/hum.wav", &wav(1, 441));
    session.changed("sounds/hum.wav");
    assert_eq!(
        session.console(),
        [
            (Severity::Info, "sounds/hum.wav changed on disk (reload sample)".to_string()),
            (Severity::Info, "sounds/hum.wav reloaded".to_string()),
        ]
    );
    assert_eq!(session.assets.resolve_sound("sounds/hum.wav", &mut session.audio).expect("loaded"), handle);
    assert!(session.audio.play(handle).is_ok());

    write_wav(&session, "sounds/hum.wav", &wav(2, 441));
    session.changed("sounds/hum.wav");
    assert!(
        session.console_mentions("its format changed (mono at 44100 Hz → stereo at 44100 Hz)"),
        "{:?}",
        session.console()
    );
}

#[test]
fn a_failed_path_whose_file_appears_serves_after_the_dispatch() {
    let (mut session, _) = humming();
    assert!(session.assets.resolve_sound("sounds/late.wav", &mut session.audio).is_err());
    session.watch_now();
    write_wav(&session, "sounds/late.wav", &wav(1, 1));

    session.changed("sounds/late.wav");

    assert!(session.console_mentions("sounds/late.wav loads now"), "{:?}", session.console());
    assert!(session.assets.resolve_sound("sounds/late.wav", &mut session.audio).is_ok(), "the failure is forgotten");
}

#[test]
fn a_file_that_does_not_decode_keeps_the_old_sound_and_reports_at_error_severity() {
    let (mut session, handle) = humming();
    write_wav(&session, "sounds/hum.wav", b"RIFF and nothing after it");

    session.changed("sounds/hum.wav");

    let (severity, line) = session.console().last().cloned().expect("a line");
    assert_eq!(severity, Severity::Error);
    assert!(line.starts_with("sounds/hum.wav did not reload, the sound before it is kept — "), "{line}");
    assert!(session.editor.editor.status_bar.is_showing_error());
    assert_eq!(session.assets.resolve_sound("sounds/hum.wav", &mut session.audio).expect("kept"), handle);
}

#[test]
fn the_same_reload_lands_in_a_play_session() {
    let (mut session, handle) = humming();
    press_play(&mut session.editor, &mut session.world);
    assert!(session.editor.editor.in_play_session());
    write_wav(&session, "sounds/hum.wav", &wav(1, 441));

    session.changed("sounds/hum.wav");

    assert!(session.console_mentions("sounds/hum.wav reloaded"), "{:?}", session.console());
    assert_eq!(session.assets.resolve_sound("sounds/hum.wav", &mut session.audio).expect("loaded"), handle);
}

#[test]
fn a_sound_that_fails_after_its_path_was_registered_is_marked_at_the_next_registration_and_its_arrival_is_a_change() {
    let (mut session, _) = humming();
    let mut watch = FileWatch::new();
    let register = |watch: &mut FileWatch, session: &Session| {
        let watched = session.editor.watched_paths(&session.world, &session.assets);
        watch.set_registered(watched.into_keys().collect(), &session.editor.expected_absent_paths(&session.assets));
    };
    register(&mut watch, &session);
    assert!(session.editor.expected_absent_paths(&session.assets).is_empty(), "nothing has failed yet");

    assert!(session.assets.resolve_sound("sounds/late.wav", &mut session.audio).is_err(), "the load fails while the file is missing");
    write_wav(&session, "sounds/late.wav", &wav(1, 1));
    assert!(watch.poll().is_empty(), "the file arrived before the first poll: its silent baseline");
    assert!(watch.poll().is_empty());

    let marks = session.editor.expected_absent_paths(&session.assets);
    assert_eq!(marks, BTreeSet::from([session.path("sounds/late.wav")]), "the failure is a mark the host now sends");
    register(&mut watch, &session);
    assert!(watch.poll().is_empty(), "newly marked, the present file is held one poll");
    assert_eq!(watch.poll(), [FileChange::Changed(session.path("sounds/late.wav"))], "and then it is the arrival");
}
