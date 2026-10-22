//! Triggers through the real synthesizers: placement on the block grid, isolation between
//! sources and instruments, and the transport releasing what triggers started.

use super::stage::*;
use super::*;

const QUARTER: u32 = 384;
/// The sine bank's sample is A4; this key plays it unpitched.
const A4: u8 = 69;

fn one_part_song(
    notes: &[(u8, u8, u32)],
    quantize: &str,
    looping: bool,
    end_tick: u32,
) -> Arc<Song> {
    trigger_song(
        vec![("melody", melody(notes))],
        end_tick,
        looping,
        &format!(
            r#"parts: [(name: "melody", track: Name("melody"), quantize: {quantize})],
               instruments: [{}],
               bindings: [(action: "hit", part: "melody", instrument: "sine")]"#,
            sine_instrument("sine")
        ),
    )
}

fn energy(samples: &[f32]) -> f32 {
    samples.iter().map(|sample| sample * sample).sum()
}

#[test]
fn a_quantized_trigger_attacks_on_the_first_frame_of_the_block_holding_its_beat() {
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER)],
        "Beat",
        true,
        QUARTER * 8,
    ));
    let beat = stage.sample_at_tick(u64::from(QUARTER)) as usize;
    stage.render(whole_blocks(0.3));

    stage.trigger("hit", source(1)).expect("queued");
    let (output, _) = stage.render(beat + 2 * BLOCK_FRAMES - stage.frames as usize);

    let onset_block = beat / BLOCK_FRAMES * BLOCK_FRAMES;
    let played_from = whole_blocks(0.3);
    assert_eq!(beat, 22_050);
    assert_eq!(
        energy(&output[..onset_block - played_from]),
        0.0,
        "nothing sounds before the beat's block"
    );
    assert!(energy(&output[onset_block - played_from..beat - played_from]) > 0.0);
    assert_eq!(
        onset_block % BLOCK_FRAMES,
        0,
        "placement is on the 64-frame grid"
    );
}

#[test]
fn a_long_pull_places_a_trigger_drained_in_its_last_block_as_block_by_block_rendering_does() {
    const PULL_FRAMES: usize = 2048;
    let song = one_part_song(&[(A4, 100, QUARTER)], "Beat", true, QUARTER * 8);
    let config = ConductorConfig {
        capacities: Capacities {
            commands_per_block: 1,
            ..Capacities::default()
        },
        ..ConductorConfig::new(SAMPLE_RATE)
    };
    let mut outputs = Vec::new();
    for in_one_call in [true, false] {
        let mut stage = Stage::new(Arc::clone(&song), config);
        stage.send(ConductorCommand::Play);
        let beat = stage.sample_at_tick(u64::from(QUARTER)) as usize;
        let pull_starts_at = (beat / BLOCK_FRAMES + 1) * BLOCK_FRAMES;
        stage.render(pull_starts_at);
        // Struck on the beat, just before the pull; the commands ahead of it hold it in
        // the queue until the pull's last block.
        let stamp = stage.now()
            - std::time::Duration::from_secs_f64(
                (pull_starts_at - beat) as f64 / f64::from(SAMPLE_RATE),
            );
        for _ in 1..PULL_FRAMES / BLOCK_FRAMES {
            stage.send(ConductorCommand::SetLooping(true));
        }
        stage
            .trigger_stamped("hit", source(1), stamp)
            .expect("queued");
        let (output, _) = if in_one_call {
            stage.render_in_one_call(PULL_FRAMES)
        } else {
            stage.render(PULL_FRAMES)
        };
        assert_eq!(stage.snapshot().late_triggers, 0);
        outputs.push(output);
    }

    let last_block = PULL_FRAMES - BLOCK_FRAMES;
    assert_eq!(energy(&outputs[0][..last_block]), 0.0);
    assert!(
        energy(&outputs[0][last_block..]) > 0.0,
        "on time for its beat, it plays in the block that drained it"
    );
    assert_same_samples(&outputs[0], &outputs[1], "one pull against block by block");
}

#[test]
fn an_unquantized_trigger_attacks_on_the_block_it_is_drained_in() {
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER)],
        "Off",
        true,
        QUARTER * 8,
    ));
    stage.render(BLOCK_FRAMES * 10);

    stage.trigger("hit", source(1)).expect("queued");
    let (output, _) = stage.render(BLOCK_FRAMES * 2);

    assert!(energy(&output[..BLOCK_FRAMES]) > 0.0);
    assert_eq!(stage.snapshot().accepted_triggers, 1);
}

#[test]
fn same_pitch_notes_on_two_sources_end_independently_on_the_synthesizer() {
    let song = one_part_song(
        &[(A4, 100, QUARTER * 4), (A4, 100, QUARTER / 4)],
        "Off",
        true,
        QUARTER * 16,
    );
    let mut both = Stage::playing(Arc::clone(&song));
    let mut alone = Stage::playing(song);
    both.trigger("hit", source(1)).expect("queued");
    both.trigger("hit", source(2)).expect("queued");
    alone.trigger("hit", source(1)).expect("queued");

    let (with_short_note, _) = both.render(whole_blocks(1.5));
    let (long_note_alone, _) = alone.render(whole_blocks(1.5));

    let after_short_tail = seconds(0.5);
    let worst = with_short_note[after_short_tail..]
        .iter()
        .zip(&long_note_alone[after_short_tail..])
        .map(|(both, alone)| (both - alone).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        root_mean_square(&long_note_alone[after_short_tail..]) > 0.01,
        "the long note still sounds"
    );
    assert!(
        worst < 1e-5,
        "the second source's note-off left the first's note as it sounds alone: {worst}"
    );
}

#[test]
fn a_same_pitch_note_off_on_one_instrument_never_reaches_another() {
    let song = trigger_song(
        vec![
            (
                "lead",
                melody(&[(A4, 100, QUARTER * 4), (A4, 90, QUARTER / 4)]),
            ),
            ("echo", melody(&[(A4, 100, QUARTER * 4)])),
        ],
        QUARTER * 16,
        true,
        &format!(
            r#"parts: [(name: "lead", track: Name("lead")), (name: "echo", track: Name("echo"))],
               instruments: [{}, {}],
               bindings: [(action: "lead", part: "lead", instrument: "lead"),
                          (action: "echo", part: "echo", instrument: "echo")]"#,
            sine_instrument("lead"),
            sine_instrument("echo")
        ),
    );
    let mut stages = [
        Stage::playing(Arc::clone(&song)),
        Stage::playing(Arc::clone(&song)),
        Stage::playing(song),
    ];
    let [both, lead_alone, echo_alone] = &mut stages;
    for stage in [&mut *both, &mut *lead_alone] {
        stage.trigger("lead", source(1)).expect("queued");
    }
    for stage in [&mut *both, &mut *echo_alone] {
        stage.trigger("echo", source(1)).expect("queued");
    }
    for stage in [&mut *both, &mut *lead_alone, &mut *echo_alone] {
        stage.render(whole_blocks(0.3));
    }
    for stage in [&mut *both, &mut *lead_alone] {
        stage.trigger("lead", source(1)).expect("queued");
    }

    let (mixed, _) = both.render(whole_blocks(1.0));
    let (lead, _) = lead_alone.render(whole_blocks(1.0));
    let (echo, _) = echo_alone.render(whole_blocks(1.0));

    let worst = mixed
        .iter()
        .zip(lead.iter().zip(&echo))
        .map(|(mixed, (lead, echo))| (mixed - lead - echo).abs())
        .fold(0.0_f32, f32::max);
    assert!(root_mean_square(&echo) > 0.01, "the echo sounds throughout");
    assert!(
        worst < 1e-6,
        "the mix minus the lead is the echo alone, so the lead's retrigger and note-off \
         never reached it: {worst}"
    );
}

#[test]
fn a_sixteenth_source_takes_the_oldest_sources_lane_and_sounds() {
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER * 8)],
        "Off",
        true,
        QUARTER * 16,
    ));
    for id in 1..=15 {
        stage.trigger("hit", source(id)).expect("queued");
    }
    stage.render(BLOCK_FRAMES);

    stage.trigger("hit", source(16)).expect("queued");
    stage.render(BLOCK_FRAMES);

    let slot = stage.conductor.slot_mut(0).expect("the sine slot");
    assert!(slot.lane_of(source(16)).is_some(), "the newcomer sounds");
    assert_eq!(
        slot.lane_of(source(1)),
        None,
        "the oldest source lost its lane"
    );
    assert_eq!(slot.sounding_count(), 15);
    assert_eq!(slot.counters().lane_steals, 1);
}

#[test]
fn play_releases_every_triggered_note() {
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER * 16)],
        "Off",
        true,
        QUARTER * 32,
    ));
    stage.trigger("hit", source(1)).expect("queued");
    let (before, _) = stage.render(whole_blocks(0.5));

    stage.send(ConductorCommand::Play);
    let (after, _) = stage.render(whole_blocks(0.5));

    assert!(root_mean_square(&before) > 0.01);
    assert!(
        root_mean_square(&after[seconds(0.3)..]) < 1e-6,
        "the note was released at Play and its tail has rung out"
    );
    assert_eq!(
        stage
            .conductor
            .slot_mut(0)
            .map(|slot| slot.sounding_count()),
        Some(0)
    );
}

#[test]
fn a_loop_wrap_and_a_pause_never_hang_a_triggered_note() {
    // The loop is one second long, the note too, and it starts halfway through.
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER * 2)],
        "Off",
        true,
        QUARTER * 2,
    ));
    stage.render(whole_blocks(0.5));
    stage.trigger("hit", source(1)).expect("queued");

    let (across_the_wrap, _) = stage.render(whole_blocks(0.9));
    let wrapped = stage.snapshot();
    stage.trigger("hit", source(1)).expect("queued");
    stage.render(whole_blocks(0.2));
    stage.send(ConductorCommand::Pause);
    let (paused, _) = stage.render(whole_blocks(0.4));

    assert_eq!(wrapped.loop_wraps, 1);
    assert!(root_mean_square(&across_the_wrap[..seconds(0.3)]) > 0.01);
    assert!(
        root_mean_square(&across_the_wrap[seconds(0.75)..]) < 1e-6,
        "the wrap released the note it cut short, and its tail rang out"
    );
    assert!(root_mean_square(&paused[seconds(0.25)..]) < 1e-6);
    assert_eq!(
        stage
            .conductor
            .slot_mut(0)
            .map(|slot| slot.sounding_count()),
        Some(0)
    );
    assert_eq!(stage.snapshot().pending_events, 0);
}

#[test]
fn a_paused_song_refuses_triggers_and_resumes_without_a_burst() {
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER)],
        "Off",
        true,
        QUARTER * 8,
    ));
    stage.render(BLOCK_FRAMES * 4);
    stage.send(ConductorCommand::Pause);
    stage.render(BLOCK_FRAMES);
    for _ in 0..20 {
        stage.trigger("hit", source(1)).expect("queued");
    }
    stage.render(BLOCK_FRAMES);

    stage.send(ConductorCommand::Resume);
    let (resumed, _) = stage.render(whole_blocks(0.3));

    assert_eq!(stage.snapshot().rejected_triggers, 20);
    assert_eq!(stage.conductor.part_cursor(0), Some(0));
    assert_eq!(
        energy(&resumed),
        0.0,
        "nothing refused while paused plays on resume"
    );
}

#[test]
fn a_seek_while_paused_is_the_position_a_stop_then_publishes() {
    let mut stage = Stage::playing(one_part_song(
        &[(A4, 100, QUARTER)],
        "Off",
        true,
        QUARTER * 32,
    ));
    stage.render(BLOCK_FRAMES * 100);
    stage.send(ConductorCommand::Pause);
    stage.send(ConductorCommand::Seek(3_000));
    stage.render(BLOCK_FRAMES);
    let paused = stage.snapshot().sample_position;

    stage.send(ConductorCommand::Stop);
    stage.render(BLOCK_FRAMES);

    let destination = stage.sample_at_tick(3_000);
    assert_eq!(paused, destination);
    assert_eq!(stage.snapshot().sample_position, destination);
    assert_eq!(stage.snapshot().transport, TransportState::Stopped);
}

#[test]
fn resuming_on_a_sample_carrying_a_state_change_sounds_as_a_seek_to_it_does() {
    let midi = smf(&[vec![
        (0, controller(0, 7, 20)),
        (QUARTER * 2, controller(0, 7, 127)),
        (0, note_on(0, A4, 100)),
        (QUARTER, note_off(0, A4)),
    ]]);
    let background = song(&midi, true, &[]);
    let destination = u64::from(QUARTER) * 2;
    let mut resumed = Stage::playing(Arc::clone(&background));
    let mut seeked = Stage::playing(background);
    for stage in [&mut resumed, &mut seeked] {
        stage.render(BLOCK_FRAMES * 10);
    }
    resumed.send(ConductorCommand::Pause);
    resumed.send(ConductorCommand::Seek(destination));
    resumed.send(ConductorCommand::Resume);
    seeked.send(ConductorCommand::Seek(destination));

    let (after_resume, _) = resumed.render(whole_blocks(0.4));
    let (after_seek, _) = seeked.render(whole_blocks(0.4));

    assert!(root_mean_square(&after_seek) > 0.01);
    assert_same_samples(
        &after_resume,
        &after_seek,
        "the resume folds the volume change on its own sample, as the seek does",
    );
}

#[test]
fn a_seek_while_stopped_is_where_the_next_play_starts_and_only_that_play() {
    let mut stage = Stage::new(
        one_part_song(&[(A4, 100, QUARTER)], "Off", true, QUARTER * 32),
        ConductorConfig::new(SAMPLE_RATE),
    );
    stage.send(ConductorCommand::Seek(3_000));
    stage.render(BLOCK_FRAMES);
    assert_eq!(stage.snapshot().transport, TransportState::Stopped);

    stage.send(ConductorCommand::Play);
    stage.render(BLOCK_FRAMES);
    let destination = stage.sample_at_tick(3_000);
    assert_eq!(
        stage.snapshot().sample_position,
        destination + BLOCK_FRAMES as u64,
        "the first block played from the recorded tick"
    );

    stage.send(ConductorCommand::Stop);
    stage.send(ConductorCommand::Play);
    stage.render(BLOCK_FRAMES);
    assert_eq!(
        stage.snapshot().sample_position,
        BLOCK_FRAMES as u64,
        "the start tick is spent by the play it started"
    );
}
