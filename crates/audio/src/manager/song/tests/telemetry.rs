//! What a game reads to decide about the slot and to report on it: the song's label, the
//! maxima taken window by window, and the one log line.

use super::*;

#[test]
fn the_label_names_a_pending_playing_or_paused_song_and_nothing_once_it_ended_died_or_stopped()
{
    let assets = AssetDirectory::new();
    let song = assets.write_song("song", &song_midi(true), &sidecar(None));
    let label = song.display().to_string();

    let mut gated = AudioManager::awaiting_gesture();
    gated.set_asset_base(assets.path());
    assert_eq!(gated.song_label(), None, "no song");
    gated.play_song(&song).expect("the song loads");
    assert_eq!(gated.song_label(), Some(label.as_str()), "pending the gesture");

    let (mut manager, start) = playing(&assets);
    assert_eq!(manager.song_label(), Some(label.as_str()), "playing");
    manager.pause_song();
    manager.update_at(start + seconds(0.05));
    assert_eq!(manager.song_label(), Some(label.as_str()), "a paused song is a live song");
    manager.stop_song();
    assert_eq!(manager.song_label(), None, "stopped");

    let (mut dead, start) = playing(&assets);
    let playback = dead.songs.playback.as_mut().expect("playing");
    playback.output = SongOutput::Detached;
    dead.update_at(start + seconds(0.05));
    assert_eq!(dead.song_label(), None, "an output that died is no song");

    let (mut ended, song) = song_in_memory(&assets, false);
    ended.play_song_in_memory(song).expect("the conductor loads it");
    let start = Instant::now();
    let beyond_the_end = f64::from(SONG_BEATS) / 10.0 + 0.5;
    for step in 1..=4 {
        ended.update_at(start + seconds(beyond_the_end * f64::from(step) / 4.0));
    }
    assert!(ended.song_ended());
    assert_eq!(ended.song_label(), None, "a song played to its end is no song");
}

#[test]
fn taking_the_maxima_returns_what_they_held_and_leaves_zeros_for_the_next_window() {
    let assets = AssetDirectory::new();
    assert_eq!(AudioManager::disabled().take_song_maxima(), SongMaxima::default());
    let (mut manager, start) = playing(&assets);
    for step in 1..=5 {
        manager.update_at(start + seconds(0.02 * f64::from(step)));
    }
    let held = manager.song_snapshot().expect("playing").source;
    assert!(held.render_nanoseconds_longest > 0, "{held:?}");

    let taken = manager.take_song_maxima();
    assert!(taken.render_nanoseconds_longest >= held.render_nanoseconds_longest);
    assert!(taken.drain_gap_nanoseconds_longest >= held.drain_gap_nanoseconds_longest);
    let after = manager.song_snapshot().expect("playing").source;
    assert_eq!(after.render_nanoseconds_longest, 0, "the next window starts empty");
    assert_eq!(after.drain_gap_nanoseconds_longest, 0);
    assert_eq!(after.render_nanoseconds_total, held.render_nanoseconds_total, "totals are kept");
}

#[test]
fn the_log_line_carries_every_counter_of_the_snapshot_it_was_made_from() {
    let assets = AssetDirectory::new();
    let (mut manager, start) = playing(&assets);
    manager.update_at(start + seconds(0.05));
    manager.trigger("tone", keyboard()).expect("the action is bound");
    manager.update_at(start + seconds(0.1));
    let snapshot = manager.song_snapshot().expect("playing");
    let line = snapshot.log_line();

    assert!(line.starts_with("conductor=on "), "{line}");
    let conductor = snapshot.conductor;
    for expected in [
        format!("blocks={}", conductor.blocks_rendered),
        format!("triggers_accepted={}", conductor.accepted_triggers),
        format!("triggers_rejected={}", conductor.rejected_triggers),
        format!("live_sources={}", conductor.live_sources),
        format!("wraps={}", conductor.loop_wraps),
    ] {
        assert!(line.contains(&expected), "{expected} missing from {line}");
    }
    assert_eq!(conductor.accepted_triggers, 1, "{line}");
}
