//! A sound's bytes replaced behind its handle: the loop source's boundary, pulled sample by
//! sample with no device, and the manager's reload — the handle kept, a bad stream refused
//! with the old bytes kept, a format change told apart from a same-format reload.

use std::sync::{Arc, Mutex};

use rodio::buffer::SamplesBuffer;
use rodio::Source;

use super::*;
use crate::loop_source::{LoopSlot, LoopSource};
use crate::sound::SoundFormatChange;

/// A 16-bit PCM WAV of `frames` frames, each sample holding `level`; one silent mono frame
/// at 44.1 kHz is the manager tests' `tiny_wav()` byte for byte.
fn wav(channels: u16, sample_rate: u32, frames: u32, level: i16) -> Vec<u8> {
    let data_bytes = frames * u32::from(channels) * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for _ in 0..frames * u32::from(channels) {
        bytes.extend_from_slice(&level.to_le_bytes());
    }
    bytes
}

fn mono(samples: &[f32]) -> SamplesBuffer<f32> {
    SamplesBuffer::new(1, 44_100, samples.to_vec())
}

fn looping(samples: &[f32]) -> (LoopSource<SamplesBuffer<f32>>, LoopSlot<SamplesBuffer<f32>>) {
    let slot: LoopSlot<SamplesBuffer<f32>> = Arc::new(Mutex::new(None));
    (LoopSource::new(mono(samples), Arc::clone(&slot)), slot)
}

fn pull(source: &mut LoopSource<SamplesBuffer<f32>>, count: usize) -> Vec<f32> {
    source.by_ref().take(count).collect()
}

fn stored_bytes(manager: &AudioManager, handle: SoundHandle) -> Arc<[u8]> {
    Arc::clone(&manager.sounds[&handle.id].bytes)
}

#[test]
fn a_filled_slot_lets_the_old_pass_finish_whole_and_every_later_pass_is_the_new_sound() {
    let (mut source, slot) = looping(&[1.0, 2.0, 3.0]);
    assert_eq!(pull(&mut source, 1), [1.0]);
    *slot.lock().expect("the slot") = Some(mono(&[7.0, 8.0]).buffered());

    assert_eq!(pull(&mut source, 2), [2.0, 3.0], "the pass playing is never cut");
    assert_eq!(pull(&mut source, 2), [7.0, 8.0], "the boundary takes the new sound");
    assert_eq!(pull(&mut source, 4), [7.0, 8.0, 7.0, 8.0], "and keeps it on every pass after");
    assert!(slot.lock().expect("the slot").is_none(), "the slot was emptied");
}

#[test]
fn an_empty_slot_repeats_the_old_sound() {
    let (mut source, _slot) = looping(&[1.0, 2.0, 3.0]);
    assert_eq!(pull(&mut source, 9), [1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
}

#[test]
fn a_slot_held_at_the_boundary_plays_the_old_pass_again_and_is_taken_at_the_next() {
    let (mut source, slot) = looping(&[1.0, 2.0, 3.0]);
    let mut held = slot.lock().expect("the slot");
    *held = Some(mono(&[7.0, 8.0]).buffered());

    assert_eq!(
        pull(&mut source, 6),
        [1.0, 2.0, 3.0, 1.0, 2.0, 3.0],
        "the frame thread holds the slot: the loop plays on, never waiting for it"
    );
    drop(held);
    assert_eq!(pull(&mut source, 2), [7.0, 8.0], "taken at the next boundary");
}

#[test]
fn a_reload_keeps_the_handle_playable_and_an_unknown_handle_is_refused() {
    let mut manager = AudioManager::disabled();
    let handle = manager.load_sound_from_bytes(wav(1, 44_100, 1, 0)).expect("loads");
    let longer = wav(1, 44_100, 64, 100);

    assert_eq!(manager.reload_sound(handle, longer.clone()).expect("reloads"), None, "the same format");
    assert_eq!(&*stored_bytes(&manager, handle), longer.as_slice(), "the handle now serves the new bytes");
    assert!(manager.play(handle).is_ok(), "the handle every caller holds still plays");

    let unknown = SoundHandle::from_id(999);
    assert!(matches!(manager.reload_sound(unknown, longer), Err(AudioError::InvalidHandle(999))));
}

#[test]
fn a_header_with_no_body_or_bytes_that_do_not_decode_are_refused_and_the_old_bytes_kept() {
    let mut manager = AudioManager::disabled();
    let original = wav(1, 44_100, 8, 100);
    let handle = manager.load_sound_from_bytes(original.clone()).expect("loads");
    let header_only = wav(1, 44_100, 1, 0)[..44].to_vec();
    assert!(
        Decoder::new(Cursor::new(header_only.clone())).is_ok(),
        "the header alone opens a decoder, so a check that stops there would pass it"
    );

    for (bytes, case) in [(header_only, "a header with no body"), (b"not audio at all".to_vec(), "garbage")] {
        assert!(matches!(manager.reload_sound(handle, bytes), Err(AudioError::DecodeError(_))), "{case}");
        assert_eq!(&*stored_bytes(&manager, handle), original.as_slice(), "{case} keeps the old bytes");
    }
}

#[test]
fn a_stereo_or_forty_eight_kilohertz_replacement_is_told_apart_from_a_same_format_one() {
    let mut manager = AudioManager::disabled();
    let handle = manager.load_sound_from_bytes(wav(1, 44_100, 4, 0)).expect("loads");
    let mono_44 = SoundFormat { channels: 1, sample_rate: 44_100 };
    let stereo_44 = SoundFormat { channels: 2, sample_rate: 44_100 };
    let stereo_48 = SoundFormat { channels: 2, sample_rate: 48_000 };

    assert_eq!(manager.reload_sound(handle, wav(1, 44_100, 9, 50)).expect("reloads"), None);
    assert_eq!(
        manager.reload_sound(handle, wav(2, 44_100, 4, 50)).expect("reloads"),
        Some(SoundFormatChange { before: mono_44, after: stereo_44 })
    );
    let change = manager.reload_sound(handle, wav(2, 48_000, 4, 50)).expect("reloads");
    assert_eq!(change, Some(SoundFormatChange { before: stereo_44, after: stereo_48 }));
    assert_eq!(change.map(|change| change.to_string()).as_deref(), Some("stereo at 44100 Hz → stereo at 48000 Hz"));
}

#[test]
fn a_playing_loop_keeps_its_id_through_a_same_format_reload_and_a_format_change() {
    let mut manager = AudioManager::disabled();
    let handle = manager.load_sound_from_bytes(wav(1, 44_100, 4_410, 0)).expect("loads");
    let looped = manager
        .play_with_settings(&handle, SoundSettings { looping: true, volume: 0.0, ..SoundSettings::default() })
        .expect("held while disabled");
    if manager.enable_output().is_err() {
        // No device: the held loop decodes the current bytes when one arrives, so a reload
        // only replaces them.
        assert!(manager.reload_sound(handle, wav(2, 48_000, 4_410, 0)).is_ok());
        assert!(manager.is_playing(looped));
        return;
    }
    let format_of = |manager: &AudioManager| {
        let instances: Vec<_> = manager.active_sounds.iter().filter(|active| active.playback == looped).collect();
        assert_eq!(instances.len(), 1, "one instance under the loop's id");
        assert!(instances[0].slot.is_some(), "a loop plays through the loop source");
        instances[0].format
    };

    assert_eq!(manager.reload_sound(handle, wav(1, 44_100, 2_205, 0)).expect("reloads"), None);
    assert_eq!(format_of(&manager), SoundFormat { channels: 1, sample_rate: 44_100 }, "taken at its boundary");

    assert!(manager.reload_sound(handle, wav(2, 48_000, 2_205, 0)).expect("reloads").is_some());
    assert_eq!(format_of(&manager), SoundFormat { channels: 2, sample_rate: 48_000 }, "restarted on the new bytes");
    assert!(manager.is_playing(looped), "under the id its caller holds");
    manager.stop_playback(looped);
}
