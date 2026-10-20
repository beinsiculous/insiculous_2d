//! Triggers under load: storms of triggers and of lifecycle commands, a consumer that
//! stops draining, a flush through a full channel, and the synthesizer's own stealer.

use std::time::Duration;

use super::stage::*;
use super::*;
use crate::commands::CommandRejected;
use crate::{COMMAND_QUEUE_CAPACITY, LIFECYCLE_RESERVE};

const QUARTER: u32 = 384;
const A4: u8 = 69;
/// The spike's mix gain for the instruments, under which its threshold was measured.
const SPIKE_SFX_GAIN: BusGains = BusGains {
    music: 1.0,
    sfx: 0.3,
};

fn looping_part(notes: &[(u8, u8, u32)]) -> Arc<Song> {
    trigger_song(
        vec![("melody", melody(notes))],
        QUARTER * 64,
        true,
        &format!(
            r#"parts: [(name: "melody", track: Name("melody"))],
               instruments: [{}],
               bindings: [(action: "hit", part: "melody", instrument: "sine")]"#,
            sine_instrument("sine")
        ),
    )
}

fn five_eighths() -> Arc<Song> {
    looping_part(&[
        (60, 100, QUARTER / 2),
        (62, 100, QUARTER / 2),
        (64, 100, QUARTER / 2),
        (65, 100, QUARTER / 2),
        (67, 100, QUARTER / 2),
    ])
}

fn source_at(id: u64, epoch: u32) -> SourceId {
    SourceId {
        id,
        generation: 0,
        epoch,
    }
}

fn handled(snapshot: &PositionSnapshot) -> u64 {
    snapshot.accepted_triggers
        + snapshot.rejected_triggers
        + snapshot.late_triggers
        + snapshot.stale_triggers
        + snapshot.silent_triggers
}

/// The largest step from one output sample to the next.
fn largest_step(samples: &[f32]) -> f32 {
    samples
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max)
}

#[test]
fn a_ten_thousand_trigger_storm_never_crowds_out_a_lifecycle_command_and_keeps_the_cursor() {
    let mut stage = Stage::playing(five_eighths());
    let mut admitted = 0_u64;

    for frame in 0..100_u64 {
        let epoch = frame as u32 + 1;
        for trigger in 0..100 {
            if stage.trigger("hit", source_at(trigger % 40, epoch)).is_ok() {
                admitted += 1;
            }
        }
        stage
            .handle
            .send(ConductorCommand::DeactivateSource {
                source: source_at(frame % 40, epoch),
            })
            .expect("a deactivation always fits behind the triggers");
        if frame % 10 == 9 {
            stage
                .handle
                .send(ConductorCommand::StopAllSources { flush_epoch: epoch })
                .expect("a flush always fits behind the triggers");
        }
        stage.render(BLOCK_FRAMES);
    }
    while stage.handle.in_flight() > 0 {
        stage.render(BLOCK_FRAMES);
    }

    let snapshot = stage.snapshot();
    assert!(
        admitted < 10_000,
        "the storm filled the queue: {admitted} admitted"
    );
    assert_eq!(
        snapshot.commands_rejected,
        10_000 - admitted,
        "only triggers were refused"
    );
    assert_eq!(
        handled(&snapshot),
        admitted,
        "every admitted trigger was drained and counted"
    );
    assert_eq!(
        stage.conductor.part_cursor(0),
        Some((snapshot.accepted_triggers % 5) as usize),
        "only accepted triggers moved the cursor"
    );
    assert_eq!(snapshot.non_finite_samples, 0);
}

#[test]
fn a_lifecycle_storm_past_the_queue_is_lost_by_none_once_the_host_retries() {
    let mut stage = Stage::playing(five_eighths());
    let mut waiting: Vec<u64> = (0..300).collect();
    let mut refused_triggers = 0;

    let mut sent_while_stalled = 0;
    waiting.retain(|&id| {
        let sent = stage
            .handle
            .send(ConductorCommand::DeactivateSource {
                source: source_at(id, 1),
            })
            .is_ok();
        sent_while_stalled += u64::from(sent);
        !sent
    });
    if stage.trigger("hit", source(1)) == Err(CommandRejected::QueueFull) {
        refused_triggers += 1;
    }
    while !waiting.is_empty() {
        stage.render(BLOCK_FRAMES);
        waiting.retain(|&id| {
            stage
                .handle
                .send(ConductorCommand::DeactivateSource {
                    source: source_at(id, 1),
                })
                .is_err()
        });
    }
    while stage.handle.in_flight() > 0 {
        stage.render(BLOCK_FRAMES);
    }

    assert_eq!(
        sent_while_stalled,
        (COMMAND_QUEUE_CAPACITY - 1) as u64,
        "lifecycle commands fill the whole queue behind Play, reserve included"
    );
    assert_eq!(
        refused_triggers, 1,
        "a trigger is refused while the reserve is in use"
    );
    assert_eq!(
        stage.snapshot().commands_applied,
        301,
        "Play and all 300 deactivations"
    );
}

#[test]
fn a_consumer_stalled_ten_blocks_drains_triggers_before_the_deactivation_that_followed_them() {
    let mut stage = Stage::playing(looping_part(&[
        (60, 100, QUARTER * 8),
        (64, 100, QUARTER * 8),
        (67, 100, QUARTER * 8),
    ]));
    stage.render(BLOCK_FRAMES);
    let enemy = source(7);
    let mut queued = 0;
    while stage.trigger("hit", enemy).is_ok() {
        queued += 1;
    }
    stage
        .handle
        .send(ConductorCommand::DeactivateSource { source: enemy })
        .expect("the reserve holds a place for it");
    let refused_after = stage.trigger("hit", enemy);

    stage.stall(Duration::from_secs_f64(
        10.0 * BLOCK_FRAMES as f64 / f64::from(SAMPLE_RATE),
    ));
    stage.render(BLOCK_FRAMES * 8);
    let drained = stage.snapshot();
    stage.trigger("hit", enemy).expect("queued");
    stage.render(BLOCK_FRAMES);

    assert_eq!(queued, COMMAND_QUEUE_CAPACITY - LIFECYCLE_RESERVE);
    assert_eq!(refused_after, Err(CommandRejected::QueueFull));
    assert_eq!(
        drained.accepted_triggers, queued as u64,
        "every trigger sent before the deactivation played before it"
    );
    assert_eq!(
        drained.late_triggers, 0,
        "a stall of ten blocks is within the tolerance"
    );
    assert_eq!((drained.live_sources, drained.pending_events), (0, 0));
    assert_eq!(
        stage
            .conductor
            .slot_mut(0)
            .map(|slot| slot.sounding_count()),
        Some(0)
    );
    assert_eq!(
        stage.snapshot().stale_triggers,
        1,
        "the ended source cannot trigger again"
    );
    assert_eq!(
        stage.conductor.part_cursor(0),
        Some(queued % 3),
        "the cursor holds what the accepted triggers consumed"
    );
}

#[test]
fn a_flush_through_a_full_channel_spares_the_epoch_queued_behind_it_and_the_next_song_plays() {
    let mut stage = Stage::playing(five_eighths());
    stage.render(BLOCK_FRAMES);
    let before_the_flush = source_at(1, 1);
    let returned = source_at(1, 2);
    let newcomer = source_at(2, 3);
    while stage.trigger("hit", before_the_flush).is_ok() {}
    stage
        .handle
        .send(ConductorCommand::StopAllSources { flush_epoch: 2 })
        .expect("the reserve holds a place for the flush");
    let refused_while_full = stage.trigger("hit", returned);
    stage.render(BLOCK_FRAMES);
    stage
        .trigger("hit", returned)
        .expect("room again, behind the flush");
    stage.trigger("hit", newcomer).expect("queued");
    stage.render(BLOCK_FRAMES * 4);
    let after_the_flush = stage.snapshot();

    stage.send(ConductorCommand::Stop);
    stage.render(BLOCK_FRAMES);
    stage
        .conductor
        .load(five_eighths())
        .expect("the next song loads");
    stage.send(ConductorCommand::Play);
    stage.trigger("hit", returned).expect("queued");
    stage.trigger("hit", newcomer).expect("queued");
    stage.trigger("hit", before_the_flush).expect("queued");
    let (next_song, _) = stage.render(whole_blocks(0.1));

    assert_eq!(refused_while_full, Err(CommandRejected::QueueFull));
    assert_eq!(
        after_the_flush.live_sources, 2,
        "the flush ended the old epoch only; both queued behind it are live"
    );
    let next = stage.snapshot();
    assert_eq!(
        next.accepted_triggers, 2,
        "the new song's first triggers play"
    );
    assert_eq!(
        next.stale_triggers, 1,
        "the flushed epoch stays refused across the load"
    );
    assert!(root_mean_square(&next_song) > 0.01);
}

#[test]
fn two_hundred_triggers_queued_before_a_two_second_stall_never_sound() {
    let mut stage = Stage::playing(five_eighths());
    stage.render(BLOCK_FRAMES);
    for id in 0..200 {
        stage.trigger("hit", source(id)).expect("queued");
    }

    stage.stall(Duration::from_secs(2));
    let (resumed, _) = stage.render(BLOCK_FRAMES * 8);

    assert_eq!(stage.snapshot().late_triggers, 200);
    assert_eq!(stage.snapshot().accepted_triggers, 0);
    assert!(
        resumed.iter().all(|sample| *sample == 0.0),
        "no burst on resume"
    );
    assert_eq!(stage.conductor.part_cursor(0), Some(0));
}

/// Three notes held on one source while another strikes one key every block, so each
/// strike leaves a tail and the tails overflow a pool of `polyphony` voices.
fn hammer_over_held_notes(polyphony: usize) -> Vec<f32> {
    let song = trigger_song(
        vec![
            ("held", {
                let mut chord = Vec::new();
                for key in [57, 64, 72] {
                    chord.push((0, note_on(0, key, 100)));
                }
                chord.push((QUARTER * 16, note_off(0, 57)));
                chord.push((0, note_off(0, 64)));
                chord.push((0, note_off(0, 72)));
                chord
            }),
            ("hammer", melody(&[(81, 100, QUARTER / 16)])),
        ],
        QUARTER * 16,
        true,
        &format!(
            r#"parts: [(name: "held", track: Name("held")), (name: "hammer", track: Name("hammer"))],
               instruments: [{}],
               bindings: [(action: "hold", part: "held", instrument: "sine"),
                          (action: "hit", part: "hammer", instrument: "sine")]"#,
            sine_instrument("sine")
        ),
    );
    let mut config = ConductorConfig::new(SAMPLE_RATE);
    config.capacities.polyphony = polyphony;
    config.capacities.sounding_cap = 4;
    let mut stage = Stage::new(song, config);
    stage.send(ConductorCommand::Play);
    stage.trigger("hold", source(1)).expect("queued");
    let mut output = Vec::new();
    for _ in 0..seconds(2.0) / BLOCK_FRAMES {
        stage.trigger("hit", source(2)).expect("queued");
        output.extend(stage.render_with(BLOCK_FRAMES, SPIKE_SFX_GAIN).0);
    }
    output
}

/// The largest sample-to-sample step the storms below may make, frozen from their
/// measurement on the sine bank at the spike's 0.3 gain: 0.0131 with the pool
/// overflowing (0.0042 with room for every tail, 0.0008 for one lane hammering one key),
/// where K1's own storm measured 0.0126. The margin over it is K1's.
const STORM_STEP_THRESHOLD: f32 = 0.02;

#[test]
fn a_storm_overflowing_the_pool_steps_under_the_frozen_threshold_and_cuts_no_sounding_note() {
    let overflowing = hammer_over_held_notes(8);
    let roomy = hammer_over_held_notes(256);

    let step = largest_step(&overflowing);
    assert!(
        step <= STORM_STEP_THRESHOLD,
        "largest step {step} over {STORM_STEP_THRESHOLD}"
    );
    assert_ne!(overflowing, roomy, "the synthesizer's stealer ran");
    let difference: Vec<f32> = overflowing
        .iter()
        .zip(&roomy)
        .map(|(overflowing, roomy)| overflowing - roomy)
        .collect();
    for (key, hertz) in [(57, 220.0), (64, 329.628), (72, 523.251)] {
        let held = goertzel_power(&roomy, hertz);
        let lost = goertzel_power(&difference, hertz);
        assert!(
            lost < held * 1e-3,
            "key {key}: the stolen voices held {lost} of its {held}; only tails were cut"
        );
    }
}

#[test]
fn one_lane_hammering_one_key_steps_under_the_threshold_and_hangs_nothing() {
    let mut stage = Stage::playing(looping_part(&[(A4, 100, QUARTER)]));
    let mut output = Vec::new();

    for _ in 0..seconds(1.0) / BLOCK_FRAMES {
        stage.trigger("hit", source(1)).expect("queued");
        output.extend(stage.render_with(BLOCK_FRAMES, SPIKE_SFX_GAIN).0);
    }
    let (after, _) = stage.render_with(whole_blocks(1.0), SPIKE_SFX_GAIN);

    let retriggers = stage
        .conductor
        .slot_mut(0)
        .map(|slot| slot.counters().retriggers);
    assert!(
        retriggers > Some(600),
        "every strike after the first retriggered: {retriggers:?}"
    );
    let step = largest_step(&output);
    assert!(step <= STORM_STEP_THRESHOLD, "largest step {step}");
    assert!(
        root_mean_square(&after[seconds(0.7)..]) < 1e-6,
        "the last strike ended and its tail rang out"
    );
    assert_eq!(
        stage
            .conductor
            .slot_mut(0)
            .map(|slot| slot.sounding_count()),
        Some(0)
    );
}
