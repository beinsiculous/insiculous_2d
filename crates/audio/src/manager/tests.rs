//! Tests for [`AudioManager`]: the disabled no-op mode, the gesture-gated
//! `enable_output` upgrade with its pending music, typed load errors, handle
//! lifetime, and the volume model (`base × bus × master`). Both seams
//! (device/SFX and music/buses) share the WAV fixtures below, so they live in
//! one file.
//!
//! Tests run on machines WITH an audio device (dev) and without one (CI), so
//! anything that needs a live sink matches on `enable_output`'s result and
//! asserts the invariants of whichever branch ran.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

// === Fixtures ===

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

/// A file in the temp dir, removed when dropped (also on a failed assert).
struct TempFile(PathBuf);

impl TempFile {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
    }
}

/// Write `bytes` to a temp file whose name is unique per process AND per
/// call, so parallel tests (and one test writing twice) never collide.
fn write_temp_file(bytes: &[u8]) -> TempFile {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "insiculous_audio_test_{}_{}.wav",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes).expect("temp dir must be writable");
    TempFile(path)
}

/// Write `tiny_wav` to a unique temp file.
fn write_temp_wav() -> TempFile {
    write_temp_file(&tiny_wav())
}

/// A path that no test ever creates.
fn missing_path() -> PathBuf {
    std::env::temp_dir().join("insiculous_audio_test_definitely_missing.wav")
}

fn sfx_sink_volume(manager: &AudioManager) -> f32 {
    manager.active_sounds[0].sink.volume()
}

fn music_sink_volume(manager: &AudioManager) -> f32 {
    manager
        .music_sink
        .as_ref()
        .expect("music sink exists")
        .volume()
}

// === SoundSettings ===

#[test]
fn test_sound_settings_clamp_volume_to_unit_range_and_floor_speed_at_a_tenth() {
    let loud = SoundSettings::new().with_volume(2.0);
    let negative = SoundSettings::new().with_volume(-1.0);
    let crawl = SoundSettings::new().with_speed(0.01);
    let backwards = SoundSettings::new().with_speed(-3.0);

    assert_eq!(loud.volume, 1.0, "volume clamps at full");
    assert_eq!(negative.volume, 0.0, "volume clamps at silent");
    assert_eq!(
        crawl.speed, 0.1,
        "speed floors at 0.1 (rodio misbehaves at zero)"
    );
    assert_eq!(backwards.speed, 0.1, "negative speed floors at 0.1");
}

// === Disabled mode ===

#[test]
fn test_disabled_manager_validates_everything_and_plays_nothing() -> AudioResult<()> {
    let mut manager = AudioManager::disabled();
    let music = write_temp_wav();
    assert!(!manager.is_enabled());

    // Sounds load and every playback call succeeds as a no-op.
    let handle = manager.load_sound_from_bytes(tiny_wav())?;
    manager.play(handle)?;
    manager.play_with_settings(&handle, SoundSettings::new().with_looping(true))?;
    manager.stop(handle);
    manager.update();

    // Handles are still validated: an unknown one is rejected, not ignored.
    let bogus = SoundHandle::from_id(9999);
    assert!(
        matches!(manager.play(bogus), Err(AudioError::InvalidHandle(9999))),
        "a disabled manager still rejects unknown handles"
    );

    // Music validates the file and returns Ok, but never claims to be audible.
    manager.play_music(music.path())?;
    manager.pause_music();
    manager.resume_music();
    assert!(
        !manager.is_music_playing(),
        "disabled play_music returns Ok but is_music_playing stays false"
    );

    // The engine's constructor never fails, device or not.
    let mut fallback = AudioManager::new_or_disabled();
    let handle = fallback.load_sound_from_bytes(tiny_wav())?;
    fallback.play(handle)?;
    Ok(())
}

// === enable_output ===

#[test]
fn test_enable_output_keeps_handles_ids_buses_and_pending_music_whatever_the_outcome(
) -> AudioResult<()> {
    let mut manager = AudioManager::disabled();
    manager.set_master_volume(0.5);
    manager.set_sfx_volume(0.25);
    manager.set_music_volume(0.75);
    let first = manager.load_sound_from_bytes(tiny_wav())?;
    let second = manager.load_sound_from_bytes(tiny_wav())?;
    let music = write_temp_wav();
    manager.play_music(music.path())?;
    // Ids come from an instance-local counter: a sibling manager hands out
    // the same first id, so nothing process-global drifts between managers.
    let sibling = AudioManager::disabled().load_sound_from_bytes(tiny_wav())?;
    assert_eq!(sibling.id(), first.id(), "ids are manager-local");

    let enabled = manager.enable_output().is_ok();

    assert_eq!(
        manager.is_enabled(),
        enabled,
        "Ok means a live device, Err means still disabled"
    );
    let second_call = manager.enable_output();
    assert_eq!(
        manager.is_enabled(),
        enabled,
        "a second call never changes the state"
    );
    if enabled {
        assert!(second_call.is_ok(), "already enabled is an Ok no-op");
        assert!(
            manager.pending_music.is_none(),
            "success consumes the pending request"
        );
        assert!(
            manager.is_music_playing(),
            "the pending track actually started"
        );
    } else {
        assert!(
            manager.pending_music.is_some(),
            "failure keeps the request for a later try"
        );
        assert!(!manager.is_music_playing());
    }
    assert_eq!(manager.master_volume(), 0.5, "bus volumes carry over");
    assert_eq!(manager.sfx_volume(), 0.25);
    assert_eq!(manager.music_volume(), 0.75);
    manager.play(first)?;
    manager.play(second)?;
    let third = manager.load_sound_from_bytes(tiny_wav())?;
    assert_eq!(
        third.id(),
        second.id() + 1,
        "the id sequence continues across the upgrade"
    );
    Ok(())
}

// === Pending music ===

#[test]
fn test_pending_music_keeps_only_the_last_request_until_stop_or_a_failed_load() -> AudioResult<()> {
    let first = write_temp_wav();
    let second = write_temp_wav();
    let mut manager = AudioManager::disabled();

    manager.play_music(first.path())?;
    let pending = manager
        .pending_music
        .as_ref()
        .expect("a disabled play_music records the request");
    assert_eq!(pending.path, first.path());
    assert_eq!(pending.volume, 1.0);
    assert!(pending.looping, "play_music records a looping request");

    manager.play_music_with_volume(second.path(), 0.5)?;
    let pending = manager
        .pending_music
        .as_ref()
        .expect("the newer request is pending");
    assert_eq!(pending.path, second.path(), "last request wins");
    assert_eq!(pending.volume, 0.5);

    manager.stop_music();
    assert!(
        manager.pending_music.is_none(),
        "stop_music clears the request so a stopped track cannot resurrect on enable"
    );

    manager.play_music(first.path())?;
    assert!(manager.play_music(missing_path()).is_err());
    assert!(
        manager.pending_music.is_none(),
        "a failed request leaves no doomed entry; like any new request it stopped the previous one"
    );
    Ok(())
}

// === Typed load errors ===

#[test]
fn test_missing_files_are_io_errors_and_undecodable_data_is_a_decode_error_naming_the_file() {
    let mut manager = AudioManager::disabled();
    let garbage = write_temp_file(&[0xDE, 0xAD, 0xBE, 0xEF]);
    let garbage_name = garbage.path().display().to_string();

    assert!(matches!(
        manager.load_sound(missing_path()),
        Err(AudioError::IoError(_))
    ));
    assert!(matches!(
        manager.play_music(missing_path()),
        Err(AudioError::IoError(_))
    ));
    assert!(matches!(
        manager.load_sound_from_bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        Err(AudioError::DecodeError(_))
    ));
    match manager.load_sound(garbage.path()) {
        Err(AudioError::DecodeError(message)) => {
            assert!(
                message.contains(&garbage_name),
                "load_sound names the file: {message}"
            );
        }
        other => panic!("expected DecodeError, got {other:?}"),
    }
    match manager.play_music_with_volume(garbage.path(), 1.0) {
        Err(AudioError::DecodeError(message)) => {
            assert!(
                message.contains(&garbage_name),
                "play_music names the file: {message}"
            );
        }
        other => panic!("expected DecodeError, got {other:?}"),
    }
}

// === Handle lifetime ===

#[test]
fn test_unloaded_handle_is_rejected_and_its_id_is_never_recycled() -> AudioResult<()> {
    let mut manager = AudioManager::disabled();
    let wav = write_temp_wav();
    // The path-based load goes through the VFS seam; a valid file plays.
    let handle = manager.load_sound(wav.path())?;
    manager.play(handle)?;

    manager.unload(handle);

    assert!(
        matches!(manager.play(handle), Err(AudioError::InvalidHandle(id)) if id == handle.id()),
        "an unloaded handle is InvalidHandle carrying its own id"
    );
    let next = manager.load_sound_from_bytes(tiny_wav())?;
    assert_ne!(
        next, handle,
        "a stale Copy of the handle can never play a newer sound"
    );
    Ok(())
}

// === Volume buses ===

#[test]
fn test_bus_volumes_clamp_to_the_unit_range() {
    type Setter = fn(&mut AudioManager, f32);
    type Getter = fn(&AudioManager) -> f32;
    let buses: [(&str, Setter, Getter); 3] = [
        (
            "master",
            AudioManager::set_master_volume,
            AudioManager::master_volume,
        ),
        (
            "sfx",
            AudioManager::set_sfx_volume,
            AudioManager::sfx_volume,
        ),
        (
            "music",
            AudioManager::set_music_volume,
            AudioManager::music_volume,
        ),
    ];
    let mut manager = AudioManager::disabled();

    for (bus, set, get) in buses {
        set(&mut manager, 2.0);
        assert_eq!(get(&manager), 1.0, "{bus} bus clamps at full");
        set(&mut manager, -1.0);
        assert_eq!(get(&manager), 0.0, "{bus} bus clamps at silent");
        set(&mut manager, 0.7);
        assert_eq!(get(&manager), 0.7, "{bus} bus keeps an in-range value");
    }
}

/// Needs a live device: a disabled manager owns no sinks, so the product is
/// unobservable there and the test passes vacuously on CI.
#[test]
fn test_bus_volumes_multiply_into_every_live_sink_and_reapply_on_change() -> AudioResult<()> {
    let mut manager = AudioManager::disabled();
    if manager.enable_output().is_err() {
        assert!(
            !manager.is_enabled(),
            "a failed enable leaves the manager disabled"
        );
        return Ok(());
    }
    let handle = manager.load_sound_from_bytes(tiny_wav())?;
    let music = write_temp_wav();
    manager.set_master_volume(0.5);
    manager.set_sfx_volume(0.5);
    manager.set_music_volume(0.25);

    // Public fields bypass the builder clamps, so clamping happens at play.
    let settings = SoundSettings {
        volume: 2.0,
        speed: 0.0,
        looping: false,
    };
    manager.play_with_settings(&handle, settings)?;
    manager.play_music_with_volume(music.path(), 0.5)?;

    assert_eq!(
        sfx_sink_volume(&manager),
        0.25,
        "sfx sink = clamp(base) × sfx × master"
    );
    assert_eq!(
        manager.active_sounds[0].sink.speed(),
        0.1,
        "speed floors at play time"
    );
    assert_eq!(
        music_sink_volume(&manager),
        0.0625,
        "music sink = base × music × master"
    );

    manager.set_master_volume(1.0);
    assert_eq!(sfx_sink_volume(&manager), 0.5, "master re-derives live sfx");
    assert_eq!(
        music_sink_volume(&manager),
        0.125,
        "master re-derives live music"
    );
    manager.set_sfx_volume(0.2);
    assert_eq!(sfx_sink_volume(&manager), 0.2);
    assert_eq!(
        music_sink_volume(&manager),
        0.125,
        "the sfx bus leaves music alone"
    );
    manager.set_music_volume(1.0);
    assert_eq!(music_sink_volume(&manager), 0.5);
    assert_eq!(
        sfx_sink_volume(&manager),
        0.2,
        "the music bus leaves sfx alone"
    );
    Ok(())
}

#[test]
fn test_effective_volume_combines_base_bus_and_master() {
    assert_eq!(effective_volume(1.0, 1.0, 1.0), 1.0);
    assert_eq!(effective_volume(0.5, 0.5, 0.5), 0.125);
    assert_eq!(effective_volume(0.0, 1.0, 1.0), 0.0);
    assert_eq!(effective_volume(0.8, 0.5, 0.25), 0.1);
}
