//! Sample instruments through the conductor: beside a SoundFont instrument under one
//! transport, under a burst of triggers, and a song whose sample was never supplied.

use super::stage::*;
use super::*;
use crate::pcm::OwnedPcm;
use crate::{SAMPLE_FADE_BUDGET, SAMPLE_VOICE_CAP};

const QUARTER: u32 = 384;
const SAMPLE_PATH: &str = "music/samples/sine_48000_mono.wav";

fn sine_sample() -> OwnedPcm {
    fixture_pcm(SINE_48000_MONO)
}

/// One part of a half-note bound to the sine preset (`tone`) and to the sine sample,
/// gated (`blip`), on a song that loops after one second.
fn sample_and_preset_song() -> Arc<Song> {
    trigger_song_with_samples(
        vec![("melody", melody(&[(64, 100, QUARTER * 2)]))],
        QUARTER * 2,
        true,
        &format!(
            r#"parts: [(name: "melody", track: Name("melody"))],
               instruments: [{}, {}],
               bindings: [(action: "tone", part: "melody", instrument: "sine"),
                          (action: "blip", part: "melody", instrument: "wav")]"#,
            sine_instrument("sine"),
            sample_instrument("wav", SAMPLE_PATH, "Gated")
        ),
        &[(SAMPLE_PATH, sine_sample())],
    )
}

/// Notes sounding on the preset's slot, and sample voices playing on the sample's.
fn sounding(stage: &mut Stage) -> (usize, usize) {
    let synthesized = stage
        .conductor
        .slot_mut(0)
        .map_or(0, |slot| slot.sounding_count());
    let sampled = stage
        .conductor
        .sample_slot_mut(1)
        .map_or(0, |slot| slot.voice_count());
    (synthesized, sampled)
}

#[test]
fn a_sample_and_a_preset_triggered_in_one_block_both_end_on_a_wrap_a_seek_a_pause_and_a_stop() {
    let stop_by: [(&str, Option<ConductorCommand>); 4] = [
        ("the loop wraps", None),
        ("a seek", Some(ConductorCommand::Seek(0))),
        ("a pause", Some(ConductorCommand::Pause)),
        ("a stop", Some(ConductorCommand::Stop)),
    ];
    for (transport, command) in stop_by {
        let mut stage = Stage::playing(sample_and_preset_song());
        let loop_end = stage.sample_at_tick(u64::from(QUARTER * 2)) as usize;
        if command.is_none() {
            stage.render(whole_blocks(0.8));
        }
        stage.trigger("tone", source(1)).expect("queued");
        stage.trigger("blip", source(2)).expect("queued");
        stage.render(BLOCK_FRAMES);
        assert_eq!(sounding(&mut stage), (1, 1), "both start in the same block");

        stage.render(whole_blocks(0.1));
        match command {
            Some(command) => stage.send(command),
            None => {
                stage.render(loop_end - stage.frames as usize);
            }
        }
        stage.render(BLOCK_FRAMES * 5);

        assert_eq!(sounding(&mut stage), (0, 0), "{transport} ends both kinds");
        assert_eq!(stage.snapshot().sample_voices, 0);
    }
}

#[test]
fn a_song_whose_sample_was_not_supplied_is_refused_and_the_loaded_song_plays_on() {
    let playing = sample_and_preset_song();
    let mut stage = Stage::playing(Arc::clone(&playing));
    stage.trigger("blip", source(1)).expect("queued");
    stage.render(whole_blocks(0.1));

    let refused = [
        OwnedPcm::new(48_000, 1, vec![f32::NAN; 4_800]),
        OwnedPcm::new(8_000, 1, vec![0.0; 88_000]),
    ];
    let unsupplied = trigger_song(
        vec![("melody", melody(&[(64, 100, QUARTER)]))],
        QUARTER,
        true,
        &format!(
            r#"parts: [(name: "melody", track: Name("melody"))],
               instruments: [{}],
               bindings: [(action: "blip", part: "melody", instrument: "wav")]"#,
            sample_instrument("wav", SAMPLE_PATH, "OneShot")
        ),
    );
    let load = stage.conductor.load(unsupplied);

    assert!(
        refused.iter().all(Result::is_err),
        "audio that is not finite, or longer than 10 s, never becomes a sample"
    );
    assert_eq!(
        load.map(|_| ()),
        Err(ConductorError::SampleNotSupplied {
            instrument: "wav".to_string(),
            path: SAMPLE_PATH.to_string(),
        })
    );
    assert!(stage
        .conductor
        .song()
        .is_some_and(|song| Arc::ptr_eq(song, &playing)));
    let (still_playing, _) = stage.render(whole_blocks(0.1));
    assert!(root_mean_square(&still_playing) > 0.01);
    assert_eq!(stage.snapshot().sample_voices, 1);
}

#[test]
fn instruments_naming_one_sample_share_one_decoded_buffer() {
    let song = trigger_song_with_samples(
        vec![("melody", melody(&[(64, 100, QUARTER)]))],
        QUARTER,
        true,
        &format!(
            r#"parts: [(name: "melody", track: Name("melody"))],
               instruments: [{}, {}],
               bindings: [(action: "low", part: "melody", instrument: "one-shot"),
                          (action: "high", part: "melody", instrument: "gated")]"#,
            sample_instrument("one-shot", SAMPLE_PATH, "OneShot"),
            sample_instrument("gated", SAMPLE_PATH, "Gated")
        ),
        &[
            (SAMPLE_PATH, sine_sample()),
            ("music/samples/unused.wav", sine_sample()),
        ],
    );
    let mut stage = Stage::playing(Arc::clone(&song));

    assert_eq!(
        song.samples().len(),
        1,
        "a sample the song never names is not kept"
    );
    assert_eq!(
        song.samples().seconds().get(SAMPLE_PATH).copied(),
        Some(1.0)
    );
    let first = stage
        .conductor
        .sample_slot_mut(0)
        .map(|slot| slot.pcm().clone())
        .expect("a sample slot");
    let second = stage
        .conductor
        .sample_slot_mut(1)
        .map(|slot| slot.pcm().clone())
        .expect("a sample slot");
    assert!(first.shares_samples_with(&second));
}

/// The largest step the burst below may make, frozen from its measurement: 2.979, where
/// sixteen voices mixing before any burst step 0.098. A burst past the live cap plus the
/// fade budget cuts fading voices that have not yet faded at all — the one step a sample
/// slot takes on purpose, bounding the voices instead of the click. The margin is the
/// synthesizer storm's, half again the measurement.
const BURST_STEP_THRESHOLD: f32 = 4.5;

#[test]
fn a_burst_of_sixty_four_triggers_in_one_block_never_passes_the_ceiling_and_stays_finite() {
    let keys = [48, 55, 60, 67, 72].map(|key| (key, 100, QUARTER / 2));
    let song = trigger_song_with_samples(
        vec![("melody", melody(&keys))],
        QUARTER * 64,
        true,
        &format!(
            r#"parts: [(name: "melody", track: Name("melody"))],
               instruments: [{}],
               bindings: [(action: "blip", part: "melody", instrument: "wav")]"#,
            sample_instrument("wav", SAMPLE_PATH, "OneShot")
        ),
        &[(SAMPLE_PATH, sine_sample())],
    );
    let mut stage = Stage::playing(song);
    let mut output = Vec::new();
    let mut most_voices = 0;
    for id in 0..SAMPLE_VOICE_CAP as u64 {
        stage.trigger("blip", source(id)).expect("queued");
        output.extend(stage.render(whole_blocks(0.02)).0);
    }
    for burst in 0..3u64 {
        for id in 0..64 {
            stage
                .trigger("blip", source(1_000 + burst * 64 + id))
                .expect("queued");
        }
        for _ in 0..whole_blocks(0.1) / BLOCK_FRAMES {
            output.extend(stage.render(BLOCK_FRAMES).0);
            most_voices = most_voices.max(stage.snapshot().sample_voices);
        }
    }

    let counters = stage
        .conductor
        .sample_slot_mut(0)
        .map(|slot| slot.counters())
        .expect("a sample slot");
    let ceiling = SAMPLE_VOICE_CAP + SAMPLE_FADE_BUDGET;
    assert!(counters.peak_voices <= ceiling, "{counters:?}");
    assert!(most_voices <= ceiling as u64);
    assert!(counters.cuts > 0, "the bursts reached the ceiling");
    assert_eq!(stage.snapshot().accepted_triggers, 16 + 3 * 64);
    assert_eq!(stage.snapshot().non_finite_samples, 0);
    assert!(output.iter().all(|sample| sample.is_finite()));
    let step = output
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max);
    assert!(step <= BURST_STEP_THRESHOLD, "largest step {step}");
}
