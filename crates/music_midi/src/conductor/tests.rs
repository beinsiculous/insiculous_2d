use std::sync::Arc;

use super::*;
use crate::song_data::SongData;
use crate::song_file::parse_song_file;
use crate::test_support::*;
use crate::{BLOCK_FRAMES, SOUNDING_NOTE_CAP};

mod saturation;
mod stage;
mod triggering;

const SAMPLE_RATE: u32 = 44_100;
const SINE_FONT: &[u8] = include_bytes!("../../fixtures/sine.sf2");
const SINE_PATH: &str = "soundfonts/sine.sf2";
/// The sine bank's one preset.
const SINE_INSTRUMENT: &str =
    r#"(name: "sine", source: SoundFont(path: "soundfonts/sine.sf2", bank: 0, preset: 0))"#;
/// MIDI key 84, two octaves above middle C's A: the held note of the loop test.
const HELD_KEY_HERTZ: f32 = 1046.502;

fn song(midi: &[u8], looping: bool, instruments: &[&str]) -> Arc<Song> {
    let file = parse_song_file(
        "test.mid.ron",
        &format!(
            r#"SongFile(version: 1, soundfont: "{SINE_PATH}", looping: {looping}, instruments: [{}])"#,
            instruments.join(", ")
        ),
    )
    .expect("the sidecar parses");
    let (data, _) = SongData::from_smf(midi).expect("the MIDI reads");
    Arc::new(Song::new(file, data, &[(SINE_PATH, SINE_FONT)]).expect("the song builds"))
}

fn loaded(song: Arc<Song>) -> (Conductor, ConductorHandle) {
    let (mut conductor, handle) =
        Conductor::new(ConductorConfig::new(SAMPLE_RATE)).expect("the frozen config builds");
    conductor.load(song).expect("the song loads");
    (conductor, handle)
}

/// Render `frames` frames in the conductor's own blocks.
fn render(conductor: &mut Conductor, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    for (left_block, right_block) in left
        .chunks_mut(BLOCK_FRAMES)
        .zip(right.chunks_mut(BLOCK_FRAMES))
    {
        conductor.render(left_block, right_block, BusGains::UNITY);
    }
    (left, right)
}

fn seconds(seconds: f64) -> usize {
    (seconds * f64::from(SAMPLE_RATE)) as usize
}

/// About `seconds` of frames, rounded down to whole blocks so the next command is
/// drained on the first frame of the next render.
fn whole_blocks(seconds: f64) -> usize {
    self::seconds(seconds) / BLOCK_FRAMES * BLOCK_FRAMES
}

/// Equal sample for sample, or the first frame that differs.
fn assert_same_samples(actual: &[f32], expected: &[f32], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: lengths");
    if let Some((frame, (got, wanted))) = actual
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (got, wanted))| got != wanted)
    {
        panic!("{what}: frame {frame} is {got}, expected {wanted}");
    }
}

fn root_mean_square(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

/// The power of one frequency in `samples`, normalized by their count.
fn goertzel_power(samples: &[f32], frequency_hertz: f32) -> f32 {
    let omega = std::f32::consts::TAU * frequency_hertz / SAMPLE_RATE as f32;
    let coefficient = 2.0 * omega.cos();
    let (mut previous, mut before_previous) = (0.0_f32, 0.0_f32);
    for sample in samples {
        let current = sample + coefficient * previous - before_previous;
        before_previous = previous;
        previous = current;
    }
    let power = previous * previous + before_previous * before_previous
        - coefficient * previous * before_previous;
    power / (samples.len() as f32).powi(2)
}

#[test]
fn a_conductor_can_move_to_the_audio_thread() {
    fn assert_send<T: Send>() {}
    assert_send::<Conductor>();
    assert_send::<ConductorHandle>();
}

#[test]
fn capacities_the_synthesizer_cannot_honour_are_refused_at_construction() {
    let with = |change: fn(&mut ConductorConfig)| {
        let mut config = ConductorConfig::new(SAMPLE_RATE);
        change(&mut config);
        Conductor::new(config).map(|_| ()).expect_err("refused")
    };

    assert_eq!(
        with(|config| config.capacities.sounding_cap = 0),
        ConductorError::ZeroCapacity {
            capacity: "sounding cap"
        }
    );
    assert_eq!(
        with(|config| config.capacities.command_queue = 0),
        ConductorError::ZeroCapacity {
            capacity: "command queue"
        }
    );
    assert_eq!(
        with(|config| {
            config.capacities.polyphony = 16;
            config.capacities.sounding_cap = 24;
        }),
        ConductorError::PoolBelowSoundingCap {
            polyphony: 16,
            sounding_cap: 24
        }
    );
    assert_eq!(
        with(|config| config.capacities.polyphony = 257),
        ConductorError::PolyphonyOutOfRange { polyphony: 257 }
    );
    assert_eq!(
        with(|config| config.block_size = 7),
        ConductorError::BlockSizeOutOfRange { block_size: 7 }
    );
    assert_eq!(
        with(|config| config.block_size = 1025),
        ConductorError::BlockSizeOutOfRange { block_size: 1025 }
    );
    assert_eq!(
        with(|config| config.capacities.lanes_per_slot = 16),
        ConductorError::TooManyLanes {
            lanes: 16,
            maximum: 15
        }
    );
    assert_eq!(
        with(|config| config.sample_rate = 8_000),
        ConductorError::SampleRateOutOfRange { sample_rate: 8_000 }
    );
}

#[test]
fn a_song_too_big_for_the_slots_or_naming_a_missing_preset_is_refused_at_load() {
    let midi = smf(&[vec![(0, note_on(0, 60, 100)), (96, note_off(0, 60))]]);
    let missing =
        r#"(name: "organ", source: SoundFont(path: "soundfonts/sine.sf2", bank: 0, preset: 19))"#;
    let second =
        r#"(name: "second", source: SoundFont(path: "soundfonts/sine.sf2", bank: 0, preset: 0))"#;
    let mut config = ConductorConfig::new(SAMPLE_RATE);
    config.capacities.instrument_slots = 1;
    let (mut conductor, _) = Conductor::new(config).expect("builds");

    assert_eq!(
        conductor.load(song(&midi, true, &[missing])).map(|_| ()),
        Err(ConductorError::PresetMissing {
            instrument: "organ".to_string(),
            soundfont: SINE_PATH.to_string(),
            bank: 0,
            preset: 19
        })
    );
    assert_eq!(
        conductor
            .load(song(&midi, true, &[SINE_INSTRUMENT, second]))
            .map(|_| ()),
        Err(ConductorError::TooManyInstruments {
            count: 2,
            capacity: 1
        })
    );
    assert!(
        conductor.song().is_none(),
        "a refused load leaves nothing loaded"
    );
}

#[test]
fn a_background_score_denser_than_its_pool_loads_with_a_warning() {
    let mut config = ConductorConfig::new(SAMPLE_RATE);
    config.capacities.polyphony = 8;
    config.capacities.sounding_cap = 8;
    let (mut conductor, _) = Conductor::new(config).expect("builds");

    let dense = conductor
        .load(song(EGMONT, true, &[]))
        .expect("loads anyway");
    let sparse = conductor
        .load(song(BACH_INVENTION, true, &[]))
        .expect("loads");

    assert!(matches!(
        dense.warnings.as_slice(),
        [LoadWarning::BackgroundMayOverflowPool { peak_voices, polyphony: 8, .. }] if *peak_voices > 8
    ));
    assert_eq!(sparse.warnings, vec![], "two voices never overflow eight");
}

#[test]
fn the_background_score_renders_sound_once_played_and_silence_before() {
    let (mut conductor, handle) = loaded(song(NAPOLITAN_DANCE, true, &[]));

    let (before_left, _) = render(&mut conductor, seconds(0.5));
    handle.send(ConductorCommand::Play).expect("queued");
    let (playing_left, playing_right) = render(&mut conductor, seconds(2.0));

    assert_eq!(root_mean_square(&before_left), 0.0);
    assert!(root_mean_square(&playing_left) > 0.01);
    assert!(root_mean_square(&playing_right) > 0.01);
    assert_eq!(handle.position().transport(), TransportState::Playing);
}

#[test]
fn the_loop_seam_releases_every_held_note_and_the_downbeat_attacks_on_its_exact_sample() {
    // The downbeat plays hard right on channel 0 and the held note hard left on channel
    // 1, so each side of the output carries one of them alone.
    let midi = smf(&[vec![
        (0, controller(0, 10, 127)),
        (0, controller(0, 42, 127)),
        (0, controller(1, 10, 0)),
        (0, note_on(0, 48, 100)),
        (96, note_off(0, 48)),
        (288, note_on(1, 84, 100)),
        (391, note_off(1, 84)),
    ]]);
    let (mut conductor, handle) = loaded(song(&midi, true, &[]));
    let seam_sample = conductor
        .song()
        .map(|song| {
            song.data()
                .tempo_map()
                .sample_at_tick(song.background().end_tick(), SAMPLE_RATE)
        })
        .expect("loaded") as usize;
    assert_ne!(
        seam_sample % BLOCK_FRAMES,
        0,
        "the seam falls inside a block"
    );
    handle.send(ConductorCommand::Play).expect("queued");

    let (left, right) = render(&mut conductor, seam_sample + seconds(0.5));

    let downbeat_frames = seconds(0.3);
    assert!(root_mean_square(&right[..downbeat_frames]) > 0.01);
    assert!(right[seam_sample - BLOCK_FRAMES..seam_sample]
        .iter()
        .all(|sample| sample.abs() < 1e-6));
    assert_same_samples(
        &right[seam_sample..seam_sample + downbeat_frames],
        &right[..downbeat_frames],
        "the second pass's downbeat is the first's, sample for sample from the seam",
    );
    let window = seconds(0.1);
    let held = goertzel_power(&left[seam_sample - window..seam_sample], HELD_KEY_HERTZ);
    let after_release = goertzel_power(
        &left[seam_sample + seconds(0.2)..seam_sample + seconds(0.2) + window],
        HELD_KEY_HERTZ,
    );
    assert!(held > 1e-4, "the held note sounds up to the seam: {held}");
    assert!(
        after_release < held * 1e-6,
        "the seam released it and its tail has rung out: {after_release} against {held}"
    );
    let position = handle.position().snapshot();
    assert_eq!((position.loop_wraps, position.reconstructions), (1, 2));
}

#[test]
fn a_seek_to_tick_zero_restores_its_tempo_and_controllers_exactly() {
    let midi = smf(&[vec![
        (0, tempo(250_000)),
        (0, controller(0, 7, 40)),
        (192, note_on(0, 69, 100)),
        (192, note_off(0, 69)),
        (384, controller(0, 7, 127)),
        (384, note_on(0, 60, 100)),
        (96, note_off(0, 60)),
    ]]);
    let (mut conductor, handle) = loaded(song(&midi, false, &[]));
    handle.send(ConductorCommand::Play).expect("queued");
    let (first_pass, _) = render(&mut conductor, whole_blocks(0.6));

    handle.send(ConductorCommand::Seek(0)).expect("queued");
    let (after_seek, _) = render(&mut conductor, whole_blocks(0.5));

    let note_frames = seconds(0.4);
    assert!(root_mean_square(&first_pass[..note_frames]) > 0.001);
    assert_same_samples(
        &after_seek[..note_frames],
        &first_pass[..note_frames],
        "the note at 240 BPM and volume 40 sounds as it did from the top, not at the \
         default 120 BPM or the later volume",
    );
}

#[test]
fn pause_holds_the_clock_and_resume_continues_from_the_same_sample() {
    let (mut conductor, handle) = loaded(song(NAPOLITAN_DANCE, true, &[]));
    handle.send(ConductorCommand::Play).expect("queued");
    render(&mut conductor, BLOCK_FRAMES * 100);
    handle.send(ConductorCommand::Pause).expect("queued");
    render(&mut conductor, BLOCK_FRAMES);
    let paused_at = handle.position().sample_position();

    let (held_left, _) = render(&mut conductor, whole_blocks(1.0));
    let still_paused_at = handle.position().sample_position();
    handle.send(ConductorCommand::Resume).expect("queued");
    render(&mut conductor, BLOCK_FRAMES * 10);

    assert_eq!(paused_at, (BLOCK_FRAMES * 100) as u64);
    assert_eq!(still_paused_at, paused_at, "the clock holds while paused");
    assert_eq!(handle.position().transport(), TransportState::Playing);
    assert_eq!(
        handle.position().sample_position(),
        paused_at + (BLOCK_FRAMES * 10) as u64,
        "resume continues from the paused sample"
    );
    assert!(
        root_mean_square(&held_left[seconds(0.5)..]) < 1e-6,
        "pause released every note and the tails rang out"
    );
}

#[test]
fn a_storm_of_random_notes_and_seeks_never_writes_a_non_finite_sample() {
    let (mut conductor, handle) = loaded(song(EGMONT, true, &[SINE_INSTRUMENT]));
    handle.send(ConductorCommand::Play).expect("queued");
    let mut seed = 0x9e37_79b9_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };

    let mut tickets = Vec::new();
    for block in 0..seconds(10.0) / BLOCK_FRAMES {
        let slot = conductor
            .slot_mut(0)
            .expect("the sine instrument has a slot");
        for _ in 0..next() % 4 {
            let source = SourceId {
                id: next() % 40,
                generation: 0,
                epoch: 0,
            };
            let key = (next() % 128) as u8;
            if next() % 3 == 0 && !tickets.is_empty() {
                slot.note_off(tickets.swap_remove(next() as usize % tickets.len()));
            } else {
                tickets.push(slot.note_on(source, key, 1 + (next() % 127) as u8).ticket);
            }
        }
        assert!(slot.sounding_count() <= SOUNDING_NOTE_CAP);
        if block % 500 == 0 {
            conductor.apply(ConductorCommand::Seek(next() % 100_000));
        }
        render(&mut conductor, BLOCK_FRAMES);
    }

    let position = handle.position().snapshot();
    assert_eq!(position.non_finite_samples, 0);
    assert!(conductor.slot_mut(0).expect("slot").counters().steals > 0);
}

#[test]
fn a_mix_that_is_not_finite_is_written_as_silence_and_counted() {
    let (mut conductor, handle) = loaded(song(NAPOLITAN_DANCE, true, &[]));
    handle.send(ConductorCommand::Play).expect("queued");
    let mut left = vec![1.0; BLOCK_FRAMES];
    let mut right = vec![1.0; BLOCK_FRAMES];
    let poisoned = BusGains {
        music: f32::NAN,
        sfx: 1.0,
    };

    conductor.render(&mut left, &mut right, poisoned);

    assert!(left.iter().chain(&right).all(|sample| *sample == 0.0));
    assert_eq!(
        handle.position().snapshot().non_finite_samples,
        2 * BLOCK_FRAMES as u64
    );
}
