use super::*;
use crate::test_support::*;
use crate::{SAMPLE_FADE_BUDGET, SAMPLE_VOICE_CAP};

const OUTPUT_RATE: u32 = 44_100;
const LOUDEST: u8 = 127;
/// The largest step a full-scale voice of the sine fixtures takes at its root: the sine's
/// own slope, `2π × 261.63 Hz / 44 100 Hz × 0.5`, is 0.0186. A forced end must stay under
/// this; a hard cut at the sine's peak would step 0.5.
const CLICK_FREE_STEP: f32 = 0.02;

fn sample_slot(wav: &[u8], playback: SamplePlayback, live_voice_cap: usize) -> SampleSlot {
    SampleSlot::new(
        SampleInstrument {
            pcm: fixture_pcm(wav),
            root_note: SINE_WAV_ROOT_NOTE,
            gain: 1.0,
            playback,
        },
        SampleShape {
            output_rate: OUTPUT_RATE,
            live_voice_cap,
            fade_budget: SAMPLE_FADE_BUDGET,
        },
    )
}

fn source(id: u64) -> SourceId {
    SourceId {
        id,
        generation: 0,
        epoch: 1,
    }
}

fn render(slot: &mut SampleSlot, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    slot.render_add(&mut left, &mut right);
    (left, right)
}

/// Frames rendered, one at a time, until the slot holds no voice.
fn frames_until_silent(slot: &mut SampleSlot) -> usize {
    let mut frames = 0;
    while slot.voice_count() > 0 {
        render(slot, 1);
        frames += 1;
    }
    frames
}

/// The frequency of a signal by its rising zero crossings, each placed between frames by
/// linear interpolation.
fn zero_crossing_hertz(samples: &[f32]) -> f64 {
    let crossings: Vec<f64> = samples
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(frame, pair)| frame as f64 + f64::from(-pair[0] / (pair[1] - pair[0])))
        .collect();
    let (first, last) = (crossings[0], crossings[crossings.len() - 1]);
    (crossings.len() - 1) as f64 * f64::from(OUTPUT_RATE) / (last - first)
}

fn largest_step(samples: &[f32]) -> f32 {
    samples
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max)
}

fn root_mean_square(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

#[test]
fn a_sample_from_either_source_rate_plays_its_written_pitch_at_the_root_a_fifth_up_and_an_octave_down(
) {
    for wav in [SINE_48000_MONO, SINE_22050_MONO] {
        for key in [60, 67, 48] {
            let mut slot = sample_slot(wav, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);
            slot.note_on(source(1), key, LOUDEST);
            let length = slot.duration_frames(key) as usize;
            let (output, _) = render(&mut slot, length);

            let measured = zero_crossing_hertz(&output);
            let expected = SINE_WAV_HERTZ * 2f64.powf(f64::from(i32::from(key) - 60) / 12.0);
            assert!(
                (measured / expected - 1.0).abs() < 0.01,
                "a {} Hz source at key {key}: {measured:.2} Hz, expected {expected:.2}",
                slot.pcm().sample_rate()
            );
        }
    }
}

#[test]
fn a_voice_lasts_its_pitch_scaled_length_two_thirds_a_fifth_up_and_double_an_octave_down() {
    for wav in [SINE_48000_MONO, SINE_22050_MONO] {
        let mut lengths = Vec::new();
        for key in [60, 67, 48] {
            let mut slot = sample_slot(wav, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);
            slot.note_on(source(1), key, LOUDEST);
            let frames = frames_until_silent(&mut slot);
            assert!(
                (frames as f64 - slot.duration_frames(key)).abs() <= 1.0,
                "key {key}: {frames} frames, expected {:.1}",
                slot.duration_frames(key)
            );
            lengths.push(frames as f64);
        }
        let root = lengths[0];
        for (length, ratio) in lengths.iter().zip([1.0, 2f64.powf(-7.0 / 12.0), 2.0]) {
            assert!(
                (length - root * ratio).abs() <= 1.0,
                "{length} frames against {root} at the root: the ratio is {:.4}, expected \
                 {ratio:.4}",
                length / root
            );
        }
    }
}

#[test]
fn a_mono_sample_plays_alike_on_both_channels_and_a_stereo_one_keeps_its_two() {
    for wav in [SINE_48000_MONO, SINE_22050_MONO] {
        let mut mono = sample_slot(wav, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);
        mono.note_on(source(1), 60, LOUDEST);
        let (left, right) = render(&mut mono, 22_050);
        assert_eq!(left, right);
    }
    for wav in [SINE_48000_STEREO, SINE_22050_STEREO] {
        let mut stereo = sample_slot(wav, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);
        stereo.note_on(source(1), 60, LOUDEST);
        let (left, right) = render(&mut stereo, 22_050);
        let balance = root_mean_square(&right) / root_mean_square(&left);
        assert!(
            (balance - 0.5).abs() < 0.01,
            "the fixture's right channel is half its left: {balance}"
        );
    }
}

#[test]
fn a_one_shot_plays_through_its_note_off_and_a_gated_voice_fades_out_from_it() {
    let mut one_shot = sample_slot(SINE_48000_MONO, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);
    let ticket = one_shot.note_on(source(1), 60, LOUDEST);
    render(&mut one_shot, 4_410);
    one_shot.note_off(ticket);
    assert!(
        one_shot.is_sounding(ticket),
        "a one-shot ignores its note-off"
    );
    let rest = frames_until_silent(&mut one_shot);
    assert!((4_410.0 + rest as f64 - one_shot.duration_frames(60)).abs() <= 1.0);

    let mut gated = sample_slot(SINE_48000_MONO, SamplePlayback::Gated, SAMPLE_VOICE_CAP);
    let ticket = gated.note_on(source(1), 60, LOUDEST);
    render(&mut gated, 4_410);
    gated.note_off(ticket);
    assert!(!gated.is_sounding(ticket));
    let fade_frames = gated.fade_frames() as usize;
    assert_eq!(fade_frames, 221, "5 ms at 44.1 kHz");
    let (fade, _) = render(&mut gated, fade_frames - 1);
    assert_eq!(
        gated.voice_count(),
        1,
        "still fading one frame before the end"
    );
    render(&mut gated, 1);
    assert_eq!(gated.voice_count(), 0);
    assert!(
        root_mean_square(&fade[fade_frames / 2..]) < root_mean_square(&fade[..fade_frames / 2])
    );
}

#[test]
fn two_sources_on_one_sample_hold_voices_with_their_own_positions_and_note_offs() {
    let mut alone = sample_slot(SINE_48000_MONO, SamplePlayback::Gated, SAMPLE_VOICE_CAP);
    alone.note_on(source(2), 64, LOUDEST);
    let (reference, _) = render(&mut alone, 20_000);

    let mut slot = sample_slot(SINE_48000_MONO, SamplePlayback::Gated, SAMPLE_VOICE_CAP);
    let first = slot.note_on(source(1), 64, LOUDEST);
    render(&mut slot, 1_000);
    let second = slot.note_on(source(2), 64, LOUDEST);
    slot.note_off(first);
    let (output, _) = render(&mut slot, 20_000);

    assert!(!slot.is_sounding(first));
    assert!(
        slot.is_sounding(second),
        "one source's note-off ends only its voice"
    );
    let fade_frames = slot.fade_frames() as usize;
    assert_eq!(
        output[fade_frames..],
        reference[fade_frames..],
        "once the first voice has faded, the second plays exactly as it would alone"
    );
}

#[test]
fn every_forced_end_fades_without_a_step_steeper_than_the_sine_itself() {
    type Force = fn(&mut SampleSlot);
    let forces: [(&str, Force); 3] = [
        ("release every voice", |slot| slot.release_all()),
        ("release its source", |slot| slot.release_source(source(1))),
        ("steal it for a new note", |slot| {
            slot.note_on(source(2), 60, LOUDEST);
        }),
    ];
    for (force_name, force) in forces {
        let mut slot = sample_slot(SINE_48000_MONO, SamplePlayback::OneShot, 1);
        slot.note_on(source(1), 60, LOUDEST);
        let (before, _) = render(&mut slot, 13_230);
        force(&mut slot);
        let (after, _) = render(&mut slot, 2_205);

        let whole: Vec<f32> = before.into_iter().chain(after).collect();
        let step = largest_step(&whole);
        assert!(
            step < CLICK_FREE_STEP,
            "{force_name}: the largest step is {step}"
        );
        assert!(
            slot.counters().cuts == 0,
            "{force_name}: nothing was cut outright"
        );
    }
}

#[test]
fn a_burst_past_the_live_cap_fades_the_oldest_and_never_holds_more_than_the_ceiling() {
    let mut slot = sample_slot(SINE_48000_MONO, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);

    for id in 0..64 {
        slot.note_on(source(id), 60, LOUDEST);
        assert!(slot.voice_count() <= SAMPLE_VOICE_CAP + SAMPLE_FADE_BUDGET);
    }

    let counters = slot.counters();
    assert_eq!(slot.live_voice_count(), SAMPLE_VOICE_CAP);
    assert_eq!(slot.voice_ceiling(), 32);
    assert_eq!(counters.peak_voices, 32);
    assert_eq!(
        counters.steals, 48,
        "every note past the sixteenth fades one out"
    );
    assert_eq!(
        counters.cuts, 32,
        "every note past the thirty-second cuts a fading one"
    );
}

#[test]
fn a_key_more_than_two_octaves_from_the_root_plays_at_two_octaves() {
    let slot = sample_slot(SINE_48000_MONO, SamplePlayback::OneShot, SAMPLE_VOICE_CAP);

    assert_eq!(slot.duration_frames(90), slot.duration_frames(84));
    assert_eq!(slot.duration_frames(10), slot.duration_frames(36));
    assert!((slot.duration_frames(84) * 4.0 - slot.duration_frames(60)).abs() < 1e-6);
}
