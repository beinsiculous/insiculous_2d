//! The song seam through the manager's public calls, headless: a disabled manager plays a
//! song on its virtual clock, a gesture-gated one holds it pending, and `update_at`
//! drives the clock so every timing below is exact.

use std::path::PathBuf;
use std::time::Duration;

use common::clock::Instant;
use music_midi::TransportState;

use super::*;
use crate::test_support::*;

mod loading;

/// A headless manager whose sidecar paths resolve in `assets`, playing its song `song`.
fn playing(assets: &AssetDirectory) -> (AudioManager, Instant) {
    let mut manager = AudioManager::disabled();
    manager.set_asset_base(assets.path());
    let song = assets.write_song("song", &song_midi(true), &sidecar(None));
    manager.play_song(&song).expect("the song loads");
    (manager, Instant::now())
}

fn snapshot(manager: &AudioManager) -> PositionSnapshot {
    manager.song_snapshot().expect("a song is loaded").conductor
}

fn seconds(value: f64) -> Duration {
    Duration::from_secs_f64(value)
}

/// The wall time one block lasts, the grain the published position moves by.
fn one_block_seconds() -> f64 {
    music_midi::BLOCK_FRAMES as f64 / f64::from(SONG_SAMPLE_RATE)
}

fn write_recorded_track(assets: &AssetDirectory) -> PathBuf {
    assets.write("recorded.wav", SINE_WAV)
}

#[test]
fn a_song_and_a_recorded_track_share_one_pending_slot_the_last_request_winning_through_the_gesture()
{
    let assets = AssetDirectory::new();
    let song = assets.write_song("song", &song_midi(true), &sidecar(None));
    let recorded = write_recorded_track(&assets);

    let mut recorded_then_song = AudioManager::awaiting_gesture();
    recorded_then_song.set_asset_base(assets.path());
    recorded_then_song
        .play_music(&recorded)
        .expect("the track decodes");
    recorded_then_song.play_song(&song).expect("the song loads");
    assert!(matches!(
        recorded_then_song.pending_music,
        Some(PendingMusic::Song(_))
    ));
    assert!(
        !recorded_then_song.is_song_playing(),
        "a pending song is not playing"
    );
    assert_eq!(recorded_then_song.song_position(), None, "and has no clock");
    assert!(recorded_then_song.trigger("tone", keyboard()).is_ok());
    assert!(matches!(
        recorded_then_song.trigger("unbound", keyboard()),
        Err(AudioError::UnknownAction { action, .. }) if action == "unbound"
    ));
    match recorded_then_song.enable_output() {
        Ok(()) => {
            assert!(recorded_then_song.pending_music.is_none());
            assert!(
                recorded_then_song.is_song_playing(),
                "the song starts on the gesture"
            );
            assert!(
                !recorded_then_song.is_music_playing(),
                "the track lost the slot"
            );
        }
        Err(_) => assert!(matches!(
            recorded_then_song.pending_music,
            Some(PendingMusic::Song(_))
        )),
    }

    let mut song_then_recorded = AudioManager::awaiting_gesture();
    song_then_recorded.set_asset_base(assets.path());
    song_then_recorded.play_song(&song).expect("the song loads");
    song_then_recorded
        .play_music(&recorded)
        .expect("the track decodes");
    assert!(matches!(
        song_then_recorded.pending_music,
        Some(PendingMusic::Recorded { .. })
    ));
    match song_then_recorded.enable_output() {
        Ok(()) => {
            assert!(
                !song_then_recorded.is_song_playing(),
                "the song lost the slot"
            );
            assert!(song_then_recorded.songs.playback.is_none());
        }
        Err(_) => assert!(matches!(
            song_then_recorded.pending_music,
            Some(PendingMusic::Recorded { .. })
        )),
    }
    song_then_recorded.stop_music();
    assert!(song_then_recorded.pending_music.is_none());
}

#[test]
fn without_a_device_a_playing_song_advances_by_the_wall_time_between_updates() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    assert!(manager.is_song_playing());

    manager.update_at(start + seconds(0.25));
    let position = manager
        .song_position()
        .expect("a playing song has a position");

    assert!(
        (position.seconds - 0.25).abs() <= 2.0 * one_block_seconds(),
        "a quarter second of song: {position:?}"
    );
    assert_eq!(position.beat, 2, "ten beats a second: {position:?}");
    assert_eq!(position.bar, 0);
    assert_eq!(position.beat_in_bar, 2);
    assert!(
        position.latency_samples < music_midi::BLOCK_FRAMES as u64,
        "at most the rest of one block runs ahead: {position:?}"
    );

    manager.pause_song();
    manager.update_at(start + seconds(0.35));
    let held = manager
        .song_position()
        .expect("a paused song keeps its position");
    manager.update_at(start + seconds(0.95));
    assert!(!manager.is_song_playing());
    let still = manager.song_position().expect("paused");
    assert_eq!(
        (still.seconds, still.beat),
        (held.seconds, held.beat),
        "a paused song's clock holds"
    );

    manager.resume_song();
    manager.update_at(start + seconds(1.05));
    assert!(manager.song_position().expect("playing").seconds > held.seconds);
}

#[test]
fn beat_edges_count_every_boundary_once_and_a_ten_beat_gap_reports_one_crossing() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    let step = 0.01;
    let mut beats = 0;
    let mut bars = 0;
    let mut sixteenths = 0;
    let mut now = start;
    manager.update_at(now);
    for _ in 0..40 {
        now += seconds(step);
        manager.update_at(now);
        beats += usize::from(manager.beat_crossed(Subdivision::Beat));
        bars += usize::from(manager.beat_crossed(Subdivision::Bar));
        sixteenths += usize::from(manager.beat_crossed(Subdivision::Sixteenth));
    }
    let played_beats = manager.song_position().expect("playing").beat as usize;
    assert_eq!(
        beats,
        played_beats + 1,
        "one crossing a beat by integer counting, the downbeat it starts on included"
    );
    assert_eq!(bars, played_beats / 4 + 1, "a bar every four beats of 4/4");
    assert!(
        sixteenths >= 3 * played_beats,
        "a 10 ms step sees most of each beat's four 25 ms sixteenths: {sixteenths}"
    );

    let before_gap = manager.song_position().expect("playing").beat;
    now += seconds(1.0);
    manager.update_at(now);
    let after_gap = manager.song_position().expect("playing").beat;
    assert_eq!(after_gap - before_gap, 10, "the gap held ten beats");
    assert!(manager.beat_crossed(Subdivision::Beat), "reported once");
    manager.update_at(now);
    assert!(
        !manager.beat_crossed(Subdivision::Beat),
        "and not again: the gap is not replayed as a flood"
    );
}

#[test]
fn a_loop_wrap_reports_its_downbeat_and_a_seek_back_reports_nothing() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    let song_seconds = f64::from(SONG_BEATS) / 10.0;
    let mut now = start;
    manager.update_at(now);
    for _ in 0..15 {
        now += seconds(song_seconds / 16.0);
        manager.update_at(now);
    }
    assert_eq!(snapshot(&manager).loop_wraps, 0);
    now += seconds(0.2);
    manager.update_at(now);
    assert_eq!(snapshot(&manager).loop_wraps, 1);
    assert!(
        manager.beat_crossed(Subdivision::Bar),
        "the wrap lands on a downbeat"
    );

    manager.seek_song(1);
    now += seconds(0.01);
    manager.update_at(now);
    assert!(
        !manager.beat_crossed(Subdivision::Beat),
        "a seek back crosses nothing"
    );
}

#[test]
fn transport_a_full_queue_refuses_waits_and_is_sent_in_order_before_any_trigger() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.update_at(start);
    let mut refused = 0;
    for _ in 0..music_midi::COMMAND_QUEUE_CAPACITY {
        if manager.trigger("tone", keyboard()).is_err() {
            refused += 1;
        }
    }
    for id in 0..music_midi::LIFECYCLE_RESERVE as u64 {
        manager.deactivate_source(source(1_000 + id, 1));
    }
    manager.pause_song();
    let playback = manager.songs.playback.as_ref().expect("playing");
    assert_eq!(
        refused,
        music_midi::LIFECYCLE_RESERVE + 1,
        "triggers stop at the reserve, the song's own Play still holding a place"
    );
    assert_eq!(
        playback.pending_lifecycle.back(),
        Some(&ConductorCommand::Pause),
        "the pause waits behind a full queue"
    );
    assert!(
        matches!(
            manager.trigger("tone", keyboard()),
            Err(AudioError::CommandRefused(CommandRejected::QueueFull))
        ),
        "a trigger never overtakes waiting transport"
    );

    let mut now = start;
    for _ in 0..4 {
        now += seconds(0.01);
        manager.update_at(now);
    }

    let snapshot = snapshot(&manager);
    assert_eq!(snapshot.transport, TransportState::Paused, "{snapshot:?}");
    assert!(manager
        .songs
        .playback
        .as_ref()
        .is_some_and(|playback| playback.pending_lifecycle.is_empty()));
    assert!(!manager.is_song_playing());
}

#[test]
fn ten_thousand_triggers_never_lose_a_deactivation_a_flush_or_a_pause() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.update_at(start);
    let mut accepted = 0u64;
    let mut now = start;
    for round in 0..10_000u64 {
        if manager.trigger("blip", source(round % 8, 1)).is_ok() {
            accepted += 1;
        }
        if round == 5_000 {
            manager.deactivate_source(source(3, 1));
            manager.stop_all_sources(2);
            manager.pause_song();
        }
        if round % 1_000 == 999 {
            now += seconds(0.005);
            manager.update_at(now);
        }
    }
    for _ in 0..4 {
        now += seconds(0.01);
        manager.update_at(now);
    }

    let snapshot = snapshot(&manager);
    assert!(accepted < 10_000, "a storm fills the queue");
    assert_eq!(
        snapshot.transport,
        TransportState::Paused,
        "the pause arrived: {snapshot:?}"
    );
    assert_eq!(
        snapshot.live_sources, 0,
        "the flush ended every epoch-1 source"
    );
    assert_eq!(manager.songs.flush_floor, Some(2));
}

#[test]
fn a_song_change_carries_the_flush_and_the_next_trigger_reactivates_under_a_new_epoch() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.trigger("tone", source(7, 1)).expect("queued");
    manager.update_at(start + seconds(0.02));
    assert_eq!(snapshot(&manager).live_sources, 1);

    manager.stop_all_sources(2);
    let next = assets.write_song("next", &song_midi(false), &sidecar(None));
    manager.play_song(&next).expect("the next song loads");
    let started = Instant::now();
    manager.trigger("tone", source(7, 1)).expect("queued");
    manager.trigger("tone", source(7, 2)).expect("queued");
    manager.update_at(started + seconds(0.02));

    let snapshot = snapshot(&manager);
    assert_eq!(
        snapshot.stale_triggers, 1,
        "the flushed epoch stays refused: {snapshot:?}"
    );
    assert_eq!(
        snapshot.accepted_triggers, 1,
        "a new epoch of the same source plays"
    );
    assert_eq!(snapshot.live_sources, 1);
}

#[test]
fn a_dead_output_stops_the_song_with_one_warning_and_every_later_call_sees_it_stopped() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.update_at(start + seconds(0.05));
    let playback = manager.songs.playback.as_mut().expect("playing");
    let output = std::mem::replace(&mut playback.output, SongOutput::Gone);
    let SongOutput::Virtual { source, .. } = output else {
        panic!("a headless song plays on the virtual clock");
    };
    playback.output = SongOutput::Detached;
    drop(source);

    manager.update_at(start + seconds(0.1));
    manager.update_at(start + seconds(0.15));

    assert_eq!(manager.songs.dead_output_warnings, 1, "warned once");
    assert!(!manager.is_song_playing());
    assert_eq!(manager.song_position(), None);
    assert!(!manager.beat_crossed(Subdivision::Beat));
    assert!(matches!(
        manager.trigger("tone", keyboard()),
        Err(AudioError::SongNotPlaying)
    ));
    manager.pause_song();
    assert_eq!(manager.songs.dead_output_warnings, 1, "and never again");
}

#[test]
fn an_unknown_action_is_an_error_naming_it_and_no_song_is_its_own() {
    let assets = AssetDirectory::new();
    let mut manager = AudioManager::disabled();
    assert!(matches!(
        manager.trigger("tone", keyboard()),
        Err(AudioError::SongNotPlaying)
    ));
    manager.set_asset_base(assets.path());
    let song = assets.write_song("song", &song_midi(true), &sidecar(None));
    manager.play_song(&song).expect("the song loads");

    let error = manager
        .trigger("jump", keyboard())
        .expect_err("jump is not bound");

    assert!(error.to_string().contains("'jump'"), "{error}");
    assert!(error.to_string().contains("song.mid"), "{error}");
}
