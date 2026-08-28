//! Headless tests for [`AudioManager`] — both seams (device/SFX and
//! music/buses) share the WAV fixtures below, so they live in one file.

use super::*;

#[test]
fn test_sound_settings_volume_clamping() {
    let settings = SoundSettings::new().with_volume(2.0);
    assert!((settings.volume - 1.0).abs() < f32::EPSILON);

    let settings = SoundSettings::new().with_volume(-1.0);
    assert!(settings.volume.abs() < f32::EPSILON);
}

#[test]
fn test_sound_settings_speed_floored_at_point_one() {
    let settings = SoundSettings::new().with_speed(0.01);
    assert!((settings.speed - 0.1).abs() < f32::EPSILON);

    let settings = SoundSettings::new().with_speed(-3.0);
    assert!((settings.speed - 0.1).abs() < f32::EPSILON);
}

#[test]
fn test_clamp_helpers_enforce_valid_ranges() {
    assert!((clamp_volume(2.0) - 1.0).abs() < f32::EPSILON);
    assert!(clamp_volume(-0.5).abs() < f32::EPSILON);
    assert!((clamp_volume(0.7) - 0.7).abs() < f32::EPSILON);

    assert!((clamp_speed(0.0) - 0.1).abs() < f32::EPSILON);
    assert!((clamp_speed(-1.0) - 0.1).abs() < f32::EPSILON);
    assert!((clamp_speed(2.5) - 2.5).abs() < f32::EPSILON);
}

#[test]
fn test_sound_ids_are_manager_local_and_deterministic() {
    // Ids come from an instance-local counter, so two independent managers
    // hand out the same first id (no process-global drift across managers).
    let mut first = AudioManager::disabled();
    let mut second = AudioManager::disabled();

    let a = first.load_sound_from_bytes(tiny_wav()).unwrap();
    let b = second.load_sound_from_bytes(tiny_wav()).unwrap();
    assert_eq!(a.id(), b.id(), "fresh managers must start from the same id");

    // Ids still climb within a single manager.
    let a2 = first.load_sound_from_bytes(tiny_wav()).unwrap();
    assert_ne!(a.id(), a2.id(), "ids within one manager must be unique");
}

/// Minimal valid WAV file (44-byte header + one silent 16-bit sample).
fn tiny_wav() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&38u32.to_le_bytes()); // chunk size
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
    bytes.extend_from_slice(&88200u32.to_le_bytes()); // byte rate
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2u32.to_le_bytes()); // data size
    bytes.extend_from_slice(&0i16.to_le_bytes()); // one silent sample
    bytes
}

/// Write `tiny_wav` to a unique temp file and return its path.
fn write_temp_wav(tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "insiculous_audio_test_{}_{}.wav",
        tag,
        std::process::id()
    ));
    std::fs::write(&path, tiny_wav()).expect("temp dir must be writable");
    path
}

#[test]
fn test_disabled_manager_loads_and_plays_as_noop() {
    let mut manager = AudioManager::disabled();
    assert!(!manager.is_enabled());
    let handle = manager.load_sound_from_bytes(tiny_wav()).unwrap();
    assert!(manager.play(handle).is_ok());
    assert_eq!(manager.active_sound_count(), 0, "no-op playback must not track sinks");
}

#[test]
fn test_disabled_manager_still_rejects_invalid_handles() {
    let mut manager = AudioManager::disabled();
    let bogus = SoundHandle::from_id(9999);
    assert!(manager.play(bogus).is_err());
}

#[test]
fn test_disabled_manager_music_controls_are_safe() {
    let mut manager = AudioManager::disabled();
    manager.stop_music();
    manager.pause_music();
    manager.resume_music();
    assert!(!manager.is_music_playing());
    manager.update();
}

#[test]
fn test_new_or_disabled_never_fails() {
    // With or without an audio device, construction must succeed and be usable.
    let mut manager = AudioManager::new_or_disabled();
    let handle = manager.load_sound_from_bytes(tiny_wav()).unwrap();
    assert!(manager.play(handle).is_ok());
}

#[test]
fn test_load_sound_from_file_succeeds() {
    let path = write_temp_wav("load_ok");
    let mut manager = AudioManager::disabled();
    let result = manager.load_sound(&path);
    std::fs::remove_file(&path).ok();

    let handle = result.expect("valid wav file must load");
    assert!(manager.play(handle).is_ok());
}

#[test]
fn test_load_sound_missing_file_returns_io_error() {
    let mut manager = AudioManager::disabled();
    let missing = std::env::temp_dir().join("insiculous_audio_test_definitely_missing.wav");
    let err = manager.load_sound(&missing).expect_err("missing file must fail");
    assert!(
        matches!(err, AudioError::IoError(_)),
        "expected IoError, got: {err:?}"
    );
}

#[test]
fn test_load_sound_from_invalid_bytes_returns_decode_error() {
    let mut manager = AudioManager::disabled();
    let err = manager
        .load_sound_from_bytes(vec![0xDE, 0xAD, 0xBE, 0xEF])
        .expect_err("garbage bytes must fail to decode");
    assert!(
        matches!(err, AudioError::DecodeError(_)),
        "expected DecodeError, got: {err:?}"
    );
}

#[test]
fn test_unloaded_sound_can_no_longer_be_played() {
    let mut manager = AudioManager::disabled();
    let handle = manager.load_sound_from_bytes(tiny_wav()).unwrap();
    assert!(manager.play(handle).is_ok());

    manager.unload(handle);
    let err = manager.play(handle).expect_err("unloaded handle must be rejected");
    assert!(matches!(err, AudioError::InvalidHandle(_)));
}

#[test]
fn test_unload_all_invalidates_every_handle() {
    let mut manager = AudioManager::disabled();
    let first = manager.load_sound_from_bytes(tiny_wav()).unwrap();
    let second = manager.load_sound_from_bytes(tiny_wav()).unwrap();

    manager.unload_all();

    assert!(manager.play(first).is_err());
    assert!(manager.play(second).is_err());
}

#[test]
fn test_stop_on_unknown_handle_is_noop() {
    let mut manager = AudioManager::disabled();
    let bogus = SoundHandle::from_id(9999);
    manager.stop(bogus);
    assert_eq!(manager.active_sound_count(), 0);
}

#[test]
fn test_stop_and_stop_all_are_safe_when_nothing_plays() {
    let mut manager = AudioManager::disabled();
    let handle = manager.load_sound_from_bytes(tiny_wav()).unwrap();
    manager.play(handle).unwrap();

    manager.stop(handle);
    manager.stop_all();
    assert_eq!(manager.active_sound_count(), 0);
}

#[test]
fn test_volume_setters_clamp_out_of_range_values() {
    let mut manager = AudioManager::disabled();

    manager.set_master_volume(2.0);
    assert!((manager.master_volume() - 1.0).abs() < f32::EPSILON);
    manager.set_master_volume(-1.0);
    assert!(manager.master_volume().abs() < f32::EPSILON);

    manager.set_sfx_volume(5.0);
    assert!((manager.sfx_volume() - 1.0).abs() < f32::EPSILON);
    manager.set_sfx_volume(-0.2);
    assert!(manager.sfx_volume().abs() < f32::EPSILON);

    manager.set_music_volume(1.5);
    assert!((manager.music_volume() - 1.0).abs() < f32::EPSILON);
    manager.set_music_volume(-0.5);
    assert!(manager.music_volume().abs() < f32::EPSILON);
}

#[test]
fn test_disabled_manager_music_loads_but_reports_not_playing() {
    let path = write_temp_wav("music_once");
    let mut manager = AudioManager::disabled();

    let looping = manager.play_music(&path);
    let once = manager.play_music_once(&path, 0.5);
    std::fs::remove_file(&path).ok();

    assert!(looping.is_ok());
    assert!(once.is_ok());
    // Documented behavior: disabled mode validates the file but never
    // reports music as playing.
    assert!(!manager.is_music_playing());
}

#[test]
fn test_play_music_missing_file_returns_io_error() {
    let mut manager = AudioManager::disabled();
    let missing = std::env::temp_dir().join("insiculous_audio_test_no_such_music.ogg");
    let err = manager
        .play_music_once(&missing, 1.0)
        .expect_err("missing music file must fail");
    assert!(matches!(err, AudioError::IoError(_)));
}
