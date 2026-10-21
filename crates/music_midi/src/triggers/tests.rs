use std::sync::Arc;

use super::*;
use crate::test_support::*;
use crate::BLOCK_FRAMES;

mod bench;

use bench::*;

#[test]
fn alternating_two_instruments_walks_one_part_in_their_two_voices() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Off"));

    for (action, id) in [("slime", 1), ("skeleton", 2), ("slime", 1), ("skeleton", 2)] {
        bench.trigger(action, source(id), 0);
        bench.run(20);
    }

    assert_eq!(bench.note_ons(), vec![(0, 60), (1, 62), (0, 64), (1, 65)]);
    assert_eq!(bench.engine.counters().accepted, 4);
}

#[test]
fn triggers_drained_in_one_block_take_successive_units_in_command_order() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Off"));

    bench.trigger("skeleton", source(9), 0);
    bench.trigger("slime", source(3), 0);
    bench.trigger("slime", source(5), 0);

    assert_eq!(bench.note_ons(), vec![(1, 60), (0, 62), (0, 64)]);
    assert!(
        bench.recorder.heard.iter().all(|heard| heard.sample == 0),
        "all three sound on the block they were drained in"
    );
}

#[test]
fn a_loop_part_wraps_at_its_end_and_never_exhausts() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Off"));

    for _ in 0..10 {
        bench.trigger("slime", source(1), 0);
        bench.run(1);
    }

    let keys: Vec<u8> = bench.note_ons().iter().map(|(_, key)| *key).collect();
    assert_eq!(keys, vec![60, 62, 64, 65, 60, 62, 64, 65, 60, 62]);
    assert_eq!(bench.engine.counters().silent, 0);
}

#[test]
fn a_sequence_part_skips_units_the_song_passed_exhausts_and_resets_when_the_song_loops() {
    let mut bench = Bench::new(song_of(four_notes(), "Sequence", "Off"));
    let past_d = bench.sample_at_tick(u64::from(QUARTER + QUARTER / 4));

    bench.trigger("slime", source(1), 0);
    bench.run_to(past_d);
    bench.trigger("slime", source(1), 0);
    bench.trigger("slime", source(1), 0);
    bench.trigger("slime", source(1), 0);
    let exhausted_cursor = bench.engine.cursor_of(0);
    bench.wrap();
    bench.trigger("slime", source(1), 0);

    let keys: Vec<u8> = bench.note_ons().iter().map(|(_, key)| *key).collect();
    assert_eq!(
        keys,
        vec![60, 64, 65, 60],
        "D started before the song's position and is skipped; the loop starts over at C"
    );
    assert_eq!(
        bench.engine.counters().silent,
        1,
        "the exhausted trigger is silent"
    );
    assert_eq!(exhausted_cursor, Some(4));
}

#[test]
fn at_song_position_plays_the_unit_sounding_through_a_tempo_change_and_nothing_in_a_rest() {
    // C for a quarter, a quarter's rest, then the tempo halves: D and E a quarter each.
    let melody = vec![
        (0, note_on(0, 60, 100)),
        (QUARTER, note_off(0, 60)),
        (QUARTER, tempo(1_000_000)),
        (0, note_on(0, 62, 90)),
        (QUARTER, note_off(0, 62)),
        (0, note_on(0, 64, 80)),
        (QUARTER, note_off(0, 64)),
    ];
    let mut bench = Bench::new(song_of(melody, "AtSongPosition", "Off"));
    let quarter = u64::from(QUARTER);
    let in_the_rest = bench.sample_at_tick(quarter + quarter / 2);
    let late_in_d = bench.sample_at_tick(2 * quarter + 3 * quarter / 4);
    let e_starts = bench.sample_at_tick(3 * quarter);

    bench.trigger("slime", source(1), 0);
    bench.run_to(in_the_rest);
    bench.trigger("slime", source(1), 0);
    bench.run_to(late_in_d);
    bench.trigger("slime", source(1), 0);
    bench.trigger("skeleton", source(2), 0);
    bench.run_to(e_starts + BLOCK_FRAMES as u64);
    bench.trigger("slime", source(1), 0);

    assert_eq!(
        late_in_d - bench.sample_at_tick(2 * quarter),
        33_075,
        "D's quarter lasts a second at 60 BPM"
    );
    assert_eq!(bench.note_ons(), vec![(0, 60), (0, 62), (1, 62), (0, 64)]);
    assert_eq!(bench.engine.counters().silent, 1, "the rest plays nothing");
    let d_off = bench
        .recorder
        .heard
        .iter()
        .find(|heard| heard.key == 62 && heard.velocity.is_none())
        .map(|heard| heard.sample);
    assert_eq!(
        d_off,
        Some(block_of(e_starts)),
        "a trigger late in D plays what remains of it, through the slower tempo"
    );
}

#[test]
fn a_chord_keeps_each_notes_own_length_and_velocity() {
    let chord = vec![
        (0, note_on(0, 60, 90)),
        (0, note_on(0, 64, 70)),
        (0, note_on(0, 67, 50)),
        (QUARTER / 4, note_off(0, 60)),
        (QUARTER / 4, note_off(0, 64)),
        (QUARTER / 2, note_off(0, 67)),
    ];
    let mut bench = Bench::new(song_of(chord, "Loop", "Off"));
    let started_at = 640;
    bench.run_to(started_at);

    bench.trigger("slime", source(1), 0);
    bench.run(400);

    let heard = &bench.recorder.heard;
    let velocities: Vec<_> = heard.iter().filter_map(|heard| heard.velocity).collect();
    assert_eq!(velocities, vec![90, 70, 50]);
    let ends: Vec<_> = heard
        .iter()
        .filter(|heard| heard.velocity.is_none())
        .map(|heard| (heard.key, heard.sample))
        .collect();
    let end_of = |ticks: u64| block_of(started_at + bench.sample_at_tick(ticks));
    assert_eq!(
        ends,
        vec![(60, end_of(96)), (64, end_of(192)), (67, end_of(384)),],
        "each note ends on the block holding its own authored end"
    );
}

#[test]
fn a_quantized_trigger_on_time_plays_now_and_one_stamped_later_waits_for_the_boundary() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Beat"));
    let beat = bench.sample_at_tick(u64::from(QUARTER));
    let next_beat = bench.sample_at_tick(2 * u64::from(QUARTER));
    bench.run_to(beat + 3 * BLOCK_FRAMES as u64);

    bench.trigger("slime", source(1), 2 * BLOCK_FRAMES as u64);
    let on_time = bench.note_ons().len();
    bench.trigger("slime", source(1), 0);
    let scheduled = bench.note_ons().len();
    bench.run_to(next_beat);
    let before_boundary = bench.note_ons().len();
    bench.next_block();

    assert_eq!(
        on_time, 1,
        "stamped within a block of the beat: it plays at once"
    );
    assert_eq!(scheduled, 1, "stamped three blocks late: it waits");
    assert_eq!(
        before_boundary, 1,
        "and nothing sounds before the next beat"
    );
    let attack = bench
        .recorder
        .heard
        .iter()
        .rfind(|heard| heard.velocity.is_some())
        .copied()
        .expect("the scheduled note sounded");
    assert_eq!((attack.key, attack.sample), (62, block_of(next_beat)));
}

#[test]
fn a_trigger_older_than_the_tolerance_is_dropped_and_its_cursor_kept() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Off"));
    bench.run(100);
    let tolerance_samples = (DEVICE_PERIOD_ALLOWANCE.as_secs_f64() * f64::from(SAMPLE_RATE)) as u64
        + BLOCK_FRAMES as u64;

    bench.trigger("slime", source(1), tolerance_samples + 50);
    bench.trigger("slime", source(1), tolerance_samples - 50);

    assert_eq!(bench.engine.counters().late, 1);
    assert_eq!(
        bench.note_ons(),
        vec![(0, 60)],
        "the first unit is still the next"
    );
}

#[test]
fn past_the_instruments_cap_the_newest_scheduled_trigger_is_refused_and_its_cursor_kept() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Beat"));
    bench.run(10);

    for _ in 0..8 {
        bench.trigger("slime", source(1), 0);
    }
    let cursor_at_the_cap = bench.engine.cursor_of(0);
    bench.trigger("slime", source(1), 0);
    bench.trigger("skeleton", source(2), 0);

    assert_eq!(
        bench.engine.counters().accepted,
        9,
        "the other instrument has room"
    );
    assert_eq!(bench.engine.counters().rejected, 1);
    assert_eq!(
        cursor_at_the_cap,
        Some(0),
        "eight units consumed: the cursor wrapped twice"
    );
    assert_eq!(
        bench.engine.cursor_of(0),
        Some(1),
        "the refused trigger consumed nothing"
    );
}

#[test]
fn a_full_scheduler_refuses_the_newest_trigger_and_keeps_its_cursor() {
    let mut bench = Bench::with_pending_capacity(song_of(four_notes(), "Loop", "Beat"), 3);
    bench.run(10);

    for (action, id) in [("slime", 1), ("skeleton", 2), ("slime", 3), ("skeleton", 4)] {
        bench.trigger(action, source(id), 0);
    }

    assert_eq!(bench.engine.counters().accepted, 3);
    assert_eq!(bench.engine.counters().rejected, 1);
    assert_eq!(bench.engine.pending_len(), 3);
    assert_eq!(
        bench.engine.cursor_of(0),
        Some(3),
        "the refused fourth consumed nothing"
    );
    bench.run_to(bench.sample_at_tick(u64::from(QUARTER)) + BLOCK_FRAMES as u64);
    assert_eq!(bench.note_ons(), vec![(0, 60), (1, 62), (0, 64)]);
}

#[test]
fn a_deactivated_source_never_starts_its_scheduled_notes_and_its_activation_stays_dead() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Beat"));
    bench.run(10);
    let enemy = source(4);

    bench.trigger("slime", enemy, 0);
    bench.engine.deactivate(enemy, &mut bench.recorder);
    bench.run(400);
    bench.trigger("slime", enemy, 0);

    assert!(
        bench.recorder.heard.is_empty(),
        "nothing it scheduled sounds"
    );
    assert_eq!(bench.engine.counters().stale, 1);
    assert_eq!(bench.engine.pending_len(), 0);
}

#[test]
fn a_recycled_or_reentered_source_plays_and_its_old_identity_cannot() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Off"));
    let first_life = source(4);
    let recycled = SourceId {
        generation: 1,
        epoch: 2,
        ..first_life
    };
    let reentered = SourceId {
        epoch: 3,
        ..recycled
    };

    bench.trigger("slime", first_life, 0);
    bench.engine.deactivate(first_life, &mut bench.recorder);
    bench.trigger("slime", recycled, 0);
    bench.trigger("slime", first_life, 0);
    bench.engine.deactivate(recycled, &mut bench.recorder);
    bench.trigger("slime", reentered, 0);
    bench.trigger("slime", recycled, 0);

    assert_eq!(bench.note_ons(), vec![(0, 60), (0, 62), (0, 64)]);
    assert_eq!(bench.engine.counters().stale, 2);
}

#[test]
fn same_pitch_notes_on_two_sources_hold_two_lanes_and_end_independently() {
    let mut steps = Vec::new();
    for length in [QUARTER * 4, QUARTER / 4] {
        steps.push((0, note_on(0, 60, 100)));
        steps.push((length, note_off(0, 60)));
    }
    let mut bench = Bench::new(song_of(steps, "Loop", "Off"));

    bench.trigger("slime", source(1), 0);
    bench.trigger("slime", source(2), 0);
    let lanes = (
        bench.recorder.ledgers[0].lane_of(source(1)),
        bench.recorder.ledgers[0].lane_of(source(2)),
    );
    bench.run_to(bench.sample_at_tick(u64::from(QUARTER)));

    assert_ne!(lanes.0, lanes.1);
    assert_eq!(
        bench.recorder.ledgers[0].sounding_count(),
        1,
        "the short note ended on its own lane"
    );
    assert!(
        bench.recorder.ledgers[0].lane_of(source(1)).is_some(),
        "the long one sounds on"
    );
}

#[test]
fn a_restart_drops_every_scheduled_event_and_resets_the_cursor_to_the_new_position() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Beat"));
    bench.run(10);
    bench.trigger("slime", source(1), 0);
    bench.trigger("slime", source(1), 0);

    let song = Arc::clone(&bench.song);
    bench.engine.restart_at(u64::from(QUARTER) * 2, &song);
    bench.run(2000);

    assert!(
        bench.recorder.heard.is_empty(),
        "the scheduled attacks were dropped"
    );
    assert_eq!(
        bench.engine.cursor_of(0),
        Some(2),
        "the cursor stands at the first unit at or after the new position"
    );
}

#[test]
fn a_song_not_playing_refuses_triggers_without_moving_the_cursor() {
    let mut bench = Bench::new(song_of(four_notes(), "Loop", "Off"));
    bench.playing = false;

    bench.trigger("slime", source(1), 0);

    assert_eq!(bench.engine.counters().rejected, 1);
    assert_eq!(bench.engine.cursor_of(0), Some(0));
    assert!(bench.recorder.heard.is_empty());
}

#[test]
fn a_refused_late_or_paused_trigger_never_evicts_a_live_source_from_a_full_table() {
    let mut bench = Bench::with_shape(
        song_of(four_notes(), "Loop", "Off"),
        EngineShape {
            sources: 4,
            pending_events: 4,
            sounding_cap: 8,
            sample_voices: 8,
        },
    );
    for id in 1..=4 {
        bench.trigger("slime", source(id), 0);
    }
    let tolerance_samples = (DEVICE_PERIOD_ALLOWANCE.as_secs_f64() * f64::from(SAMPLE_RATE)) as u64
        + BLOCK_FRAMES as u64;

    bench.trigger("slime", source(5), 0);
    bench.trigger("slime", source(6), tolerance_samples + 50);
    bench.playing = false;
    bench.trigger("slime", source(7), 0);

    let counters = bench.engine.counters();
    assert_eq!(
        (counters.accepted, counters.rejected, counters.late),
        (4, 2, 1),
        "the scheduler was full, then a trigger was late, then the song was paused"
    );
    assert!(
        (1..=4).all(|id| bench.engine.sources.is_live(source(id))),
        "every source that played is still live"
    );
    assert!(
        (5..=7).all(|id| !bench.engine.sources.is_live(source(id))),
        "no refused trigger activated its source"
    );
    assert!(
        bench
            .recorder
            .heard
            .iter()
            .all(|heard| heard.velocity.is_some()),
        "no live source lost a note"
    );
}
