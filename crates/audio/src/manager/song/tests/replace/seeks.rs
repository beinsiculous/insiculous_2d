//! The host's seeks against a parked replacement: a seek in flight defers the swap and
//! takes it where it lands — the update after the reading that showed it applied — a staged
//! refusal with it; back-to-back seeks count nothing between them; a seek after a swap is
//! no crossing either. Every wait is a bounded number of `update_at` steps.

use super::*;

/// Three frames: the clock sits mid-block after every step (a step is 441 frames, a block
/// 64), so this renders no block boundary and the conductor drains nothing.
const SUB_BLOCK_SECONDS: f64 = 3.0 / 44_100.0;

#[test]
fn a_seek_accepted_but_not_yet_applied_defers_a_paused_swap_and_the_landing_takes_it() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    manager.pause_song();
    run(&mut manager, &mut now, 1);
    request(
        &mut manager,
        &write_shaped(&assets, "same", SONG_SHAPE, true, None),
    );
    manager.seek_song(6);
    now += seconds(SUB_BLOCK_SECONDS);
    manager.update_at(now);

    assert!(
        manager.take_song_replacements().is_empty(),
        "nothing swaps while the seek is in flight"
    );
    assert!(manager.parked_replacement().is_some(), "the preparation waits");
    let (bar, position, steps, others) = run_until_swapped(&mut manager, &mut now, 3);
    assert!(others.is_empty(), "{others:?}");
    assert_eq!(steps, 2, "the update after the reading that showed the seek applied");
    assert_eq!(
        (bar, position.bar, position.beat_in_bar),
        (1, 1, 2),
        "beat 6, where the seek went"
    );
    run(&mut manager, &mut now, 2);
    assert_eq!(snapshot(&manager).tick, u64::from(6 * QUARTER), "held where the seek went");
}

#[test]
fn a_staged_refusal_is_taken_where_a_seek_lands() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing_shaped(&assets, SONG_SHAPE, false);
    run(&mut manager, &mut now, 2 * STEPS_A_BAR + 5);
    request(
        &mut manager,
        &write_shaped(&assets, "shorter", shape(4, 8), false, None),
    );
    run(&mut manager, &mut now, STEPS_A_BAR);
    let refused = manager.take_song_replacements();
    assert!(
        matches!(&refused[..], [SongReplacement::Refused { .. }]),
        "bar 3 is not in a two-bar song: {refused:?}"
    );
    assert_eq!(manager.parked_replacement(), Some((1, true)), "staged");

    manager.seek_song(2);
    let (bar, position, steps, others) = run_until_swapped(&mut manager, &mut now, 3);

    assert!(others.is_empty(), "{others:?}");
    assert_eq!(steps, 2, "taken where the seek landed, not at a bar line");
    assert_eq!((bar, position.bar, position.beat_in_bar), (0, 0, 2));
}

#[test]
fn a_seek_on_a_song_stopped_at_its_end_lands_the_replacement_stopped_at_the_new_end() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing_shaped(&assets, SONG_SHAPE, false);
    run(&mut manager, &mut now, 4 * STEPS_A_BAR + 10);
    assert!(manager.song_ended());
    request(
        &mut manager,
        &write_shaped(&assets, "longer", shape(4, 24), false, None),
    );
    manager.seek_song(0);
    now += seconds(SUB_BLOCK_SECONDS);
    manager.update_at(now);
    assert!(manager.take_song_replacements().is_empty(), "the seek is in flight");

    let (bar, position, _, others) = run_until_swapped(&mut manager, &mut now, 3);

    assert!(others.is_empty(), "{others:?}");
    assert_eq!((bar, position.beat), (4, 24), "stopped at the new end, as without the seek");
    run(&mut manager, &mut now, 3);
    assert!(manager.song_ended() && !manager.is_song_playing(), "a stopped song stays stopped");
}

#[test]
fn a_seek_after_a_swap_is_no_crossing_and_the_next_beat_counts() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    request(
        &mut manager,
        &write_shaped(&assets, "same", SONG_SHAPE, true, None),
    );
    run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);
    run(&mut manager, &mut now, 2);

    manager.seek_song(12);
    for _ in 0..3 {
        run(&mut manager, &mut now, 1);
        assert!(
            !manager.beat_crossed(Subdivision::Beat) && !manager.beat_crossed(Subdivision::Bar),
            "the seek's jump over two bars is no crossing"
        );
    }
    let position = manager.song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (3, 12), "landed on the seek");
    let mut steps_to_the_next_beat = None;
    for step in 1..=12 {
        run(&mut manager, &mut now, 1);
        if manager.beat_crossed(Subdivision::Beat) {
            steps_to_the_next_beat = Some(step);
            break;
        }
    }
    assert!(
        steps_to_the_next_beat.is_some_and(|step| step <= 9),
        "beat 13 counts: {steps_to_the_next_beat:?}"
    );
}

#[test]
fn back_to_back_seeks_before_a_block_renders_count_nothing_between_them() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    manager.seek_song(4);
    now += seconds(SUB_BLOCK_SECONDS);
    manager.update_at(now);
    manager.seek_song(12);

    for _ in 0..3 {
        run(&mut manager, &mut now, 1);
        assert!(
            !manager.beat_crossed(Subdivision::Beat) && !manager.beat_crossed(Subdivision::Bar),
            "the seeks' jumps are no crossing"
        );
    }
    let position = manager.song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (3, 12), "landed on the second seek");
    let mut steps_to_the_next_beat = None;
    for step in 1..=12 {
        run(&mut manager, &mut now, 1);
        if manager.beat_crossed(Subdivision::Beat) {
            steps_to_the_next_beat = Some(step);
            break;
        }
    }
    assert!(
        steps_to_the_next_beat.is_some_and(|step| step <= 9),
        "beat 13 counts, seven tenths of a beat on: {steps_to_the_next_beat:?}"
    );
}
