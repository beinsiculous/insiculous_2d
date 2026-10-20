//! Criterion 3: a user WAV decoded outside rendering, pitched from a declared root,
//! mixed with SoundFont output; duration follows pitch; one-shot and gated note-offs.

use super::{fixture, root_mean_square, sine_bank, sine_slot, SAMPLE_RATE};
use crate::mixer::{Mixer, SlotTarget, SpikeCommand, BLOCK_FRAMES};
use crate::sample_slot::{decode_sample, SampleError, SampleLimits, SampleSlot, TriggerMode, FADE_SECONDS, PROPOSED_LIMITS};

const ROOT: i32 = 60;

fn stereo_sample() -> crate::sample_slot::OwnedPcm {
    decode_sample(fixture("sine_48000_stereo.wav"), PROPOSED_LIMITS).expect("the stereo fixture decodes")
}

/// Frames a single voice at `key` renders before it ends, and its rendered left channel.
fn play(slot: &mut SampleSlot, key: i32, note_off_after_frames: Option<usize>) -> (usize, Vec<f32>) {
    slot.note_on(1, key, 127);
    let mut rendered = Vec::new();
    while slot.voice_count() > 0 {
        if note_off_after_frames == Some(rendered.len()) {
            slot.note_off(1, key);
        }
        let mut left = [0.0; BLOCK_FRAMES];
        let mut right = [0.0; BLOCK_FRAMES];
        slot.render_add(&mut left, &mut right);
        rendered.extend_from_slice(&left);
    }
    let last_sound = rendered.iter().rposition(|sample| *sample != 0.0).map_or(0, |index| index + 1);
    (last_sound, rendered)
}

#[test]
fn test_a_wav_decodes_once_into_owned_interleaved_pcm() {
    let pcm = stereo_sample();
    assert_eq!((pcm.sample_rate, pcm.channels, pcm.frame_count()), (48_000, 2, 48_000));
    let left_peak = pcm.frames.iter().step_by(2).fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let right_peak = pcm.frames.iter().skip(1).step_by(2).fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    assert!((left_peak / right_peak - 2.0).abs() < 0.01, "channels kept apart: {left_peak} vs {right_peak}");
}

#[test]
fn test_sample_limits_bind_before_anything_large_is_built() {
    let bytes = fixture("sine_48000_mono.wav");
    let tight_bytes = SampleLimits { maximum_bytes: bytes.len() - 1, ..PROPOSED_LIMITS };
    assert_eq!(decode_sample(bytes.clone(), tight_bytes).err(), Some(SampleError::TooManyBytes(bytes.len())));
    let short = SampleLimits { maximum_seconds: 0.5, ..PROPOSED_LIMITS };
    assert_eq!(decode_sample(bytes.clone(), short).err(), Some(SampleError::TooLong { limit_seconds: 0.5 }));
    let high_floor = SampleLimits { minimum_sample_rate: 96_000, ..PROPOSED_LIMITS };
    assert_eq!(decode_sample(bytes, high_floor).err(), Some(SampleError::UnsupportedSampleRate(48_000)));
    assert!(matches!(decode_sample(vec![0; 64], PROPOSED_LIMITS), Err(SampleError::Undecodable(_))));
}

#[test]
fn test_pitching_by_resampling_changes_duration_by_the_semitone_ratio() {
    let mut slot = SampleSlot::new(stereo_sample(), ROOT, TriggerMode::OneShot, SAMPLE_RATE, 16);
    let (root_frames, _) = play(&mut slot, ROOT, None);
    let expected_root_frames = slot.duration_frames_for_key(ROOT);
    assert!((root_frames as f64 - expected_root_frames).abs() <= 2.0, "{root_frames} vs {expected_root_frames}");
    for (key, ratio) in [(67, 2f64.powf(-7.0 / 12.0)), (48, 2.0), (ROOT + 30, 0.25)] {
        let (frames, _) = play(&mut slot, key, None);
        let measured = frames as f64 / root_frames as f64;
        assert!((measured - ratio).abs() < 0.002, "key {key}: {measured} times the root's length, expected {ratio}");
    }
}

#[test]
fn test_a_pitched_sample_plays_at_the_pitched_frequency() {
    let mut slot = SampleSlot::new(stereo_sample(), ROOT, TriggerMode::OneShot, SAMPLE_RATE, 16);
    let (frames, rendered) = play(&mut slot, ROOT + 12, None);
    let crossings = rendered[..frames].windows(2).filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0).count();
    let hertz = crossings as f64 / (frames as f64 / f64::from(SAMPLE_RATE));
    assert!((hertz - 523.25).abs() < 3.0, "an octave above middle C: {hertz} Hz");
}

#[test]
fn test_a_one_shot_ignores_note_off_and_a_gated_voice_fades_out_in_five_milliseconds() {
    let pcm = stereo_sample();
    let note_off_at = 69 * BLOCK_FRAMES;
    let mut one_shot = SampleSlot::new(pcm.clone(), ROOT, TriggerMode::OneShot, SAMPLE_RATE, 16);
    let (one_shot_frames, _) = play(&mut one_shot, ROOT, Some(note_off_at));
    assert!(one_shot_frames > SAMPLE_RATE as usize / 2, "the one-shot played on: {one_shot_frames}");

    let mut gated = SampleSlot::new(pcm, ROOT, TriggerMode::Gated, SAMPLE_RATE, 16);
    let (gated_frames, _) = play(&mut gated, ROOT, Some(note_off_at));
    let fade_frames = (FADE_SECONDS * SAMPLE_RATE as f32) as usize;
    assert!(gated_frames <= note_off_at + fade_frames + 1, "the gated voice ended within the fade: {gated_frames}");
    assert!(gated_frames > note_off_at, "and it faded rather than cutting dead");
}

#[test]
fn test_the_live_voice_cap_fades_the_oldest_voice_out() {
    let mut slot = SampleSlot::new(stereo_sample(), ROOT, TriggerMode::OneShot, SAMPLE_RATE, 2);
    for key in [60, 62, 64] {
        slot.note_on(1, key, 100);
    }
    assert_eq!((slot.live_voice_count(), slot.voice_count()), (2, 3));
    let mut left = vec![0.0; 1024];
    let mut right = vec![0.0; 1024];
    slot.render_add(&mut left, &mut right);
    assert_eq!(slot.voice_count(), 2, "the faded voice is gone after five milliseconds");
    assert_eq!(slot.counters().steals, 1);
}

#[test]
fn test_sample_voices_mix_with_soundfont_output() {
    let bank = sine_bank();
    let pcm = stereo_sample();
    let note = |mixer: &mut Mixer, target| mixer.apply(SpikeCommand::NoteOn { target, source: 1, key: 64, velocity: 100 });
    let mut both = Mixer::new(None, vec![sine_slot(&bank, 24, 128)], Some(SampleSlot::new(pcm.clone(), ROOT, TriggerMode::OneShot, SAMPLE_RATE, 16)));
    let mut synth_only = Mixer::new(None, vec![sine_slot(&bank, 24, 128)], None);
    let mut sample_only = Mixer::new(None, Vec::new(), Some(SampleSlot::new(pcm, ROOT, TriggerMode::OneShot, SAMPLE_RATE, 16)));
    for target in [SlotTarget::Instrument(0), SlotTarget::Sample] {
        note(&mut both, target);
    }
    note(&mut synth_only, SlotTarget::Instrument(0));
    note(&mut sample_only, SlotTarget::Sample);
    let mut largest_error = 0.0_f32;
    let mut mixed_level = Vec::new();
    for _ in 0..100 {
        let mut blocks = [[0.0; BLOCK_FRAMES]; 6];
        let [both_left, both_right, synth_left, synth_right, sample_left, sample_right] = &mut blocks;
        both.render_block(both_left, both_right);
        synth_only.render_block(synth_left, synth_right);
        sample_only.render_block(sample_left, sample_right);
        for frame in 0..BLOCK_FRAMES {
            largest_error = largest_error.max((both_left[frame] - synth_left[frame] - sample_left[frame]).abs());
        }
        mixed_level.extend_from_slice(both_left);
    }
    assert!(largest_error < 1e-5, "the mix is the sum of its parts: {largest_error}");
    assert!(root_mean_square(&mixed_level) > 0.01);
}
