//! Replacing the playing song at a bar line, through the public calls on the virtual clock:
//! where the new song lands, when the swap is due, which preparation wins, and what drops
//! one. Every wait is a bounded number of `update_at` steps.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;

mod queries;
mod seeks;
mod timing;

/// The wall time one step advances the clock: a tenth of a beat of the test songs.
const STEP_SECONDS: f64 = 0.01;
/// Steps in one bar of the test songs: four beats at ten beats a second.
const STEPS_A_BAR: usize = 40;

/// A song of `shape`, looping or not, with [`sidecar`]'s parts and `extra_instrument`.
fn write_shaped(
    assets: &AssetDirectory,
    stem: &str,
    shape: SongShape,
    looping: bool,
    extra_instrument: Option<(&str, &str)>,
) -> PathBuf {
    let text =
        sidecar(extra_instrument).replacen("parts:", &format!("looping: {looping},\nparts:"), 1);
    assets.write_song(stem, &shaped_song_midi(shape, true), &text)
}

/// A headless manager playing a song of `shape` from `assets`, and the clock's start.
fn playing_shaped(
    assets: &AssetDirectory,
    shape: SongShape,
    looping: bool,
) -> (AudioManager, Instant) {
    let mut manager = AudioManager::disabled();
    manager.set_asset_base(assets.path());
    let song = write_shaped(assets, "playing", shape, looping, None);
    manager.play_song(&song).expect("the song loads");
    (manager, Instant::now())
}

/// One binding more than [`sidecar`]'s: `action` on the melody, played by a sine preset.
fn added_action(action: &str) -> (String, String) {
    (
        format!(
            r#"(name: "{action}", source: SoundFont(path: "{FONT_PATH}", bank: 0, preset: 0)),"#
        ),
        format!(r#"(action: "{action}", part: "melody", instrument: "{action}"),"#),
    )
}

/// Ask for `path` at the next bar line and wait for the worker; the next update takes it in.
fn request(manager: &mut AudioManager, path: &Path) {
    manager.replace_song_at_seam(path).expect("the files read");
    manager.wait_for_preparations_newest_first();
}

/// Advance the clock `steps` steps.
fn run(manager: &mut AudioManager, now: &mut Instant, steps: usize) {
    for _ in 0..steps {
        *now += seconds(STEP_SECONDS);
        manager.update_at(*now);
    }
}

/// Step until a replacement is swapped in, at most `limit` steps: the bar it took over in,
/// where the new song plays from, and the steps taken; every other outcome on the way is
/// returned with them.
fn run_until_swapped(
    manager: &mut AudioManager,
    now: &mut Instant,
    limit: usize,
) -> (u64, SongPosition, usize, Vec<SongReplacement>) {
    let mut others = Vec::new();
    for step in 1..=limit {
        *now += seconds(STEP_SECONDS);
        manager.update_at(*now);
        for outcome in manager.take_song_replacements() {
            match outcome {
                SongReplacement::Swapped { bar, position } => return (bar, position, step, others),
                other => others.push(other),
            }
        }
    }
    panic!("no swap within {limit} steps; on the way: {others:?}");
}

fn shape(beats_per_bar: u8, beats: u32) -> SongShape {
    SongShape {
        beats_per_bar,
        beats,
        ..SONG_SHAPE
    }
}

#[test]
fn a_reordered_binding_is_resolved_against_the_new_song_after_the_swap() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    let reordered = sidecar(None).replace(
        r#"(action: "tone", part: "melody", instrument: "sine"),
                (action: "blip", part: "melody", instrument: "wav"),"#,
        r#"(action: "blip", part: "melody", instrument: "wav"),
                (action: "tone", part: "melody", instrument: "sine"),"#,
    );
    assert_ne!(reordered, sidecar(None), "the bindings changed places");
    request(
        &mut manager,
        &assets.write_song("reordered", &song_midi(true), &reordered),
    );
    run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

    manager.trigger("tone", keyboard()).expect("tone is bound");
    let mut most_sample_voices = 0;
    for _ in 0..20 {
        run(&mut manager, &mut now, 1);
        most_sample_voices = most_sample_voices.max(snapshot(&manager).sample_voices);
    }
    assert_eq!(snapshot(&manager).accepted_triggers, 1);
    assert_eq!(
        most_sample_voices, 0,
        "tone plays the preset, not the sample now first"
    );

    manager.trigger("blip", keyboard()).expect("blip is bound");
    for _ in 0..20 {
        run(&mut manager, &mut now, 1);
        most_sample_voices = most_sample_voices.max(snapshot(&manager).sample_voices);
    }
    assert!(most_sample_voices > 0, "blip plays the sample");
}

#[test]
fn an_added_action_is_unknown_before_the_swap_and_plays_after_it() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    let (instrument, binding) = added_action("extra");
    let added = assets.write_song(
        "added",
        &song_midi(true),
        &sidecar(Some((&instrument, &binding))),
    );
    request(&mut manager, &added);
    run(&mut manager, &mut now, 1);
    assert!(
        manager.parked_replacement().is_some(),
        "prepared and parked"
    );
    assert!(matches!(
        manager.trigger("extra", keyboard()),
        Err(AudioError::UnknownAction { action, .. }) if action == "extra"
    ));

    run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

    manager
        .trigger("extra", keyboard())
        .expect("the new song binds it");
    run(&mut manager, &mut now, 20);
    assert_eq!(snapshot(&manager).accepted_triggers, 1);
}

#[test]
fn a_tempo_change_lands_in_the_same_bar_and_the_next_bar_comes_at_the_new_tempo() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    let half_speed = SongShape {
        microseconds_per_quarter: 2 * TEN_BEATS_A_SECOND,
        ..SONG_SHAPE
    };
    request(
        &mut manager,
        &write_shaped(&assets, "slower", half_speed, true, None),
    );

    let (bar, position, steps, others) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

    assert!(others.is_empty(), "{others:?}");
    assert_eq!(bar, 1, "due at the first bar line after it arrived");
    assert_eq!(
        (position.bar, position.beat, position.beat_in_bar),
        (1, 4, 0)
    );
    assert!(
        steps <= STEPS_A_BAR - 4,
        "swapped within a step of the bar line: {steps}"
    );
    assert!(
        manager.beat_crossed(Subdivision::Bar),
        "the game sees the bar it swapped at"
    );
    let mut bars_crossed = Vec::new();
    for step in 1..=2 * STEPS_A_BAR + 5 {
        run(&mut manager, &mut now, 1);
        if manager.beat_crossed(Subdivision::Bar) {
            bars_crossed.push(step);
        }
    }
    assert_eq!(
        bars_crossed.len(),
        1,
        "one bar at five beats a second: {bars_crossed:?}"
    );
    assert!(
        bars_crossed[0].abs_diff(2 * STEPS_A_BAR) <= 2,
        "eight tenths of a second later: {bars_crossed:?}"
    );
}

/// Pause the playing song exactly half way through bar 1 (beat 6), then ask for `path`:
/// a paused song takes it at once.
fn swap_paused_at_beat_six(
    path_for: impl FnOnce(&AssetDirectory) -> PathBuf,
) -> (u64, SongPosition, PositionSnapshot) {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    manager.pause_song();
    manager.seek_song(6);
    run(&mut manager, &mut now, 3);
    assert_eq!(
        snapshot(&manager).tick,
        u64::from(6 * QUARTER),
        "held on beat 6"
    );
    request(&mut manager, &path_for(&assets));

    let (bar, position, steps, _) = run_until_swapped(&mut manager, &mut now, 1);
    assert_eq!(
        steps, 1,
        "a paused song swaps on the update the preparation arrives"
    );
    run(&mut manager, &mut now, 3);
    let held = snapshot(&manager);
    assert_eq!(held.transport, TransportState::Paused);
    (bar, position, held)
}

#[test]
fn four_four_to_three_four_keeps_the_bar_and_its_proportion() {
    let (bar, position, held) =
        swap_paused_at_beat_six(|assets| write_shaped(assets, "waltz", shape(3, 12), true, None));

    assert_eq!(bar, 1);
    assert_eq!(
        held.tick,
        u64::from(3 * QUARTER + 3 * QUARTER / 2),
        "half way through the 3/4 bar 1, as the 4/4 one was"
    );
    assert_eq!(
        (position.bar, position.beat, position.beat_in_bar),
        (1, 4, 1)
    );
}

#[test]
fn a_partial_last_bar_takes_the_fraction_of_its_own_length() {
    let (bar, position, held) =
        swap_paused_at_beat_six(|assets| write_shaped(assets, "partial", shape(4, 6), true, None));

    let half_of_its_two_beats = u64::from(5 * QUARTER);
    assert_eq!((bar, held.tick), (1, half_of_its_two_beats));
    assert_eq!((position.bar, position.beat_in_bar), (1, 1));
}

#[test]
fn a_running_four_four_to_three_four_swap_counts_its_next_bar_after_three_beats() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    request(
        &mut manager,
        &write_shaped(&assets, "waltz", shape(3, 12), true, None),
    );

    let (bar, position, ..) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

    assert_eq!((bar, position.bar, position.beat), (1, 1, 3));
    let mut bars_crossed = Vec::new();
    for step in 1..=STEPS_A_BAR {
        run(&mut manager, &mut now, 1);
        if manager.beat_crossed(Subdivision::Bar) {
            bars_crossed.push(step);
        }
    }
    assert_eq!(bars_crossed.len(), 1, "{bars_crossed:?}");
    assert!(
        bars_crossed[0].abs_diff(30) <= 2,
        "three beats on: {bars_crossed:?}"
    );
}

#[test]
fn a_ticks_per_quarter_change_maps_by_the_bar_not_by_the_tick() {
    let coarse = SongShape {
        ticks_per_quarter: 96,
        ..SONG_SHAPE
    };
    let (bar, position, held) =
        swap_paused_at_beat_six(|assets| write_shaped(assets, "coarse", coarse, true, None));

    assert_eq!(bar, 1);
    assert_eq!(held.tick, 6 * 96, "beat 6 at 96 ticks a quarter");
    assert_eq!(
        (position.bar, position.beat, position.beat_in_bar),
        (1, 6, 2)
    );
}

#[test]
fn a_seek_sent_before_the_preparation_arrives_moves_the_swap_to_the_bar_after_it_lands() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 3);
    manager.seek_song(6);
    run(&mut manager, &mut now, 2);
    request(
        &mut manager,
        &write_shaped(&assets, "next", SONG_SHAPE, true, None),
    );

    let (bar, _, steps, _) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

    assert_eq!(bar, 2, "the bar after beat 6");
    assert!(steps.abs_diff(18) <= 2, "at beat 8, from beat 6.2: {steps}");
}

#[test]
fn a_seek_sent_after_the_preparation_arrived_is_no_crossing_forward_or_back() {
    for (from_steps, to_beat, expected_bar, expected_steps) in [(3, 6, 2, 20), (90, 1, 1, 30)] {
        let assets = AssetDirectory::new();
        let (mut manager, mut now) = playing(&assets);
        run(&mut manager, &mut now, from_steps);
        request(
            &mut manager,
            &write_shaped(&assets, "next", SONG_SHAPE, true, None),
        );
        run(&mut manager, &mut now, 1);
        assert!(manager.parked_replacement().is_some());

        manager.seek_song(to_beat);
        let (bar, _, steps, _) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

        assert_eq!(bar, expected_bar, "seek to beat {to_beat}");
        assert!(
            steps.abs_diff(expected_steps) <= 2,
            "the next bar line after beat {to_beat}: {steps} steps"
        );
    }
}

#[test]
fn a_preparation_for_a_song_stopped_before_it_arrives_is_dropped() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    manager
        .replace_song_at_seam(write_shaped(&assets, "next", SONG_SHAPE, true, None))
        .expect("the files read");
    manager.stop_song();
    manager.wait_for_preparations_newest_first();
    run(&mut manager, &mut now, 1);

    assert!(matches!(
        manager.take_song_replacements()[..],
        [SongReplacement::Dropped]
    ));
    assert!(
        manager.songs.playback.is_none(),
        "nothing came back to life"
    );
    assert!(matches!(
        manager.replace_song_at_seam(write_shaped(&assets, "again", SONG_SHAPE, true, None)),
        Err(AudioError::SongNotPlaying)
    ));
}

#[test]
fn stop_song_drops_a_parked_preparation() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    request(
        &mut manager,
        &write_shaped(&assets, "next", SONG_SHAPE, true, None),
    );
    run(&mut manager, &mut now, 1);
    assert!(manager.parked_replacement().is_some());

    manager.stop_song();

    assert!(matches!(
        manager.take_song_replacements()[..],
        [SongReplacement::Dropped]
    ));
    assert_eq!(manager.parked_replacement(), None);
}

#[test]
fn a_song_stopped_at_its_end_takes_the_replacement_stopped_at_the_new_end() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing_shaped(&assets, SONG_SHAPE, false);
    run(&mut manager, &mut now, 4 * STEPS_A_BAR + 10);
    assert!(manager.song_ended());
    let longer = shape(4, 24);
    request(
        &mut manager,
        &write_shaped(&assets, "longer", longer, false, None),
    );

    let (bar, position, steps, _) = run_until_swapped(&mut manager, &mut now, 1);
    run(&mut manager, &mut now, 3);

    assert_eq!(steps, 1, "at once");
    assert_eq!(bar, 4, "the old end is the bar line after its last bar");
    assert_eq!(position.beat, 24, "the new end");
    let stopped = snapshot(&manager);
    assert!(stopped.ended && manager.song_ended() && !manager.is_song_playing());
    assert_eq!(stopped.tick, u64::from(24 * QUARTER));
}

#[test]
fn the_newer_request_wins_when_it_finishes_first_and_the_older_is_dropped() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    let (instrument_a, binding_a) = added_action("from_a");
    let (instrument_b, binding_b) = added_action("from_b");
    let first = assets.write_song(
        "first",
        &song_midi(true),
        &sidecar(Some((&instrument_a, &binding_a))),
    );
    let second = assets.write_song(
        "second",
        &song_midi(true),
        &sidecar(Some((&instrument_b, &binding_b))),
    );
    manager.replace_song_at_seam(&first).expect("A reads");
    manager.replace_song_at_seam(&second).expect("B reads");
    manager.wait_for_preparations_newest_first();
    run(&mut manager, &mut now, 1);

    assert!(
        matches!(
            manager.take_song_replacements()[..],
            [SongReplacement::Dropped]
        ),
        "A, older, arrived after B"
    );
    assert_eq!(
        manager.parked_replacement().map(|(request, _)| request),
        Some(2)
    );
    run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);
    assert!(manager.trigger("from_b", keyboard()).is_ok());
    assert!(matches!(
        manager.trigger("from_a", keyboard()),
        Err(AudioError::UnknownAction { .. })
    ));
}

#[test]
fn a_shorter_looping_song_takes_the_bar_modulo_its_own_bar_count() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 2 * STEPS_A_BAR + 5);
    request(
        &mut manager,
        &write_shaped(&assets, "shorter", shape(4, 8), true, None),
    );

    let (bar, position, ..) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

    assert_eq!(bar, 3);
    assert_eq!(
        (position.bar, position.beat),
        (1, 4),
        "bar 3 of a two-bar loop is its bar 1"
    );
    run(&mut manager, &mut now, STEPS_A_BAR + 5);
    assert_eq!(
        snapshot(&manager).loop_wraps,
        1,
        "and it wraps after that bar"
    );
}

#[test]
fn a_bank_only_edit_decodes_exactly_the_bank_again() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    let decoded_before = manager.songs.cache.decodes();
    let bank = assets.path().join(FONT_PATH);
    std::fs::File::options()
        .write(true)
        .open(&bank)
        .and_then(|file| file.set_modified(SystemTime::now() + Duration::from_secs(3_600)))
        .expect("the bank's modified time moves");

    request(&mut manager, &assets.path().join("song.mid"));
    run(&mut manager, &mut now, 1);

    assert_eq!(
        manager.songs.cache.decodes(),
        decoded_before + 1,
        "the bank, not the sample"
    );
    run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);
}

#[test]
fn a_loop_of_one_bar_or_less_swaps_at_its_wrap() {
    for beats in [4, 2] {
        let assets = AssetDirectory::new();
        let (mut manager, mut now) = playing_shaped(&assets, shape(4, beats), true);
        run(&mut manager, &mut now, 5);
        request(
            &mut manager,
            &write_shaped(&assets, "next", SONG_SHAPE, true, None),
        );

        let (bar, position, steps, _) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);

        let loop_steps = beats as usize * 10;
        assert_eq!(
            (bar, position.bar),
            (0, 0),
            "{beats} beats: the wrap lands on bar 0"
        );
        assert!(
            (5 + steps).abs_diff(loop_steps) <= 2,
            "{beats} beats: at the wrap, step {steps}"
        );
    }
}

#[test]
fn a_running_song_past_the_new_songs_end_refuses_and_its_end_takes_the_staged_preparation() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing_shaped(&assets, SONG_SHAPE, false);
    run(&mut manager, &mut now, STEPS_A_BAR + 5);
    request(
        &mut manager,
        &write_shaped(&assets, "shorter", shape(4, 8), false, None),
    );

    let (bar, position, steps, others) = run_until_swapped(&mut manager, &mut now, 4 * STEPS_A_BAR);

    assert!(
        matches!(&others[..], [SongReplacement::Refused { reason }] if reason.contains("bar 2")),
        "refused once, at bar 2, and staged: {others:?}"
    );
    assert!(
        steps >= 3 * STEPS_A_BAR - 10,
        "the old song played on to its end: {steps}"
    );
    assert_eq!((bar, position.beat), (4, 8));
    run(&mut manager, &mut now, 3);
    let stopped = snapshot(&manager);
    assert!(stopped.ended, "stopped at the new end: {stopped:?}");
    assert_eq!(stopped.tick, u64::from(8 * QUARTER));
}

#[test]
fn a_preparation_that_fails_names_the_file_and_leaves_the_song_playing() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    let broken = sidecar(None).replace(FONT_PATH, "absent.sf2");
    request(
        &mut manager,
        &assets.write_song("broken", &song_midi(true), &broken),
    );
    run(&mut manager, &mut now, 1);

    let outcomes = manager.take_song_replacements();
    assert!(
        matches!(&outcomes[..], [SongReplacement::Failed(error)] if error.to_string().contains("absent.sf2")),
        "{outcomes:?}"
    );
    assert_eq!(manager.parked_replacement(), None);
    run(&mut manager, &mut now, STEPS_A_BAR);
    assert!(manager.is_song_playing());
    assert!(manager.take_song_replacements().is_empty());
}
