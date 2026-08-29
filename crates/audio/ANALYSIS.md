# Audio Crate Analysis

## Update (June 11, 2026 — remediation pass)

- Sound bytes now cached as `Arc<[u8]>`; playback decodes from `Cursor<Arc<[u8]>>` with no per-play buffer clone.
- `set_master_volume` / `set_sfx_volume` / `set_music_volume` all re-apply `base * bus * master` to live sinks (SFX and music).
- `PlaybackState` deleted (was exported but never produced/accepted).
- `stop(handle)` implemented (stops all active instances of a sound); `#[allow(dead_code)]` removed.
- `play_music_once(path, volume)` added for one-shot music; `play_music*` keep looping semantics.
- File-read failures now surface as `AudioError::IoError` (via `#[from] io::Error`).
- `play`/`unload` take `SoundHandle` by value; `AudioResult` re-exported; `#[must_use]` on builders/getters.
- Full remediation history: `log_archive.md` § audio. Open debt (currently none for this crate) lives on the Studio Board (`tech-debt` label). Sections below updated to match.

## Audit (April 15, 2026)

**Removed:**
- Stale ECS component path — `audio_components.rs` no longer lives in this crate; it moved to `crates/ecs/src/audio_components.rs`. Path references updated throughout.
- Stale file count ("5 source files") — corrected to 4 (`lib.rs`, `manager.rs`, `sound.rs`, `error.rs`).
- Incorrect `symphonia` listed as a direct dependency — it is pulled in via rodio's `symphonia-all` feature.

**Kept:**
- All architectural sections (AudioManager design, SoundHandle/SoundSettings rationale, ECS component descriptions, attenuation formula).
- Known limitations (by-design tradeoffs) and future enhancement ideas — these remain valuable roadmap references.
- Strengths/Risks review, expanded with current implementation notes.

## Review Summary

- Provides `AudioManager`-centric playback over `rodio` with cached sound data and simple playback controls.
- ECS-facing audio components (`AudioSource`, `AudioListener`, `PlaySoundEffect`) live in the `ecs` crate (not this one) to avoid a circular dep; spatial attenuation math is implemented on `AudioSource`.
- Dependencies are minimal (`rodio` 0.20 with `symphonia-all` feature, `log`, `thiserror`) and keep the surface area focused.

### Strengths
- Straightforward API for SFX and music with `SoundHandle` + `SoundSettings` builder patterns.
- Clean separation between asset loading/caching (`AudioManager::load_sound`) and playback (`play_with_settings`).
- ECS components use builder pattern and serialize via serde, integrating with the scene serializer and editor inspector.
- Cached raw bytes (`Arc<[u8]>`) allow a sound to be replayed any number of times without re-reading from disk or copying the buffer.

### Risks & Follow-ups
- All audio is eagerly loaded into memory; consider optional streaming for large music tracks.
- Spatial audio is limited to attenuation math (no real 3D processing, no stereo panning); API docs should document this limitation clearly.
- Track whether multiple music tracks or mixing buses are needed as the project grows.

## Overview

The `audio` crate provides audio playback functionality for the insiculous_2d game engine. It wraps the `rodio` audio library to provide a simple, game-oriented API for sound effects and background music.

## Architecture

### Core Components

#### AudioManager (`manager.rs`)
The main audio system that handles:
- Loading and caching sound files
- Playing sounds with configurable settings
- Managing active sound instances
- Background music with pause/resume
- Volume control (master, SFX, music)

**Design Decisions:**
- Uses `rodio` for cross-platform audio playback
- Sounds are cached in memory as raw bytes (`Arc<[u8]>`) for efficient replay
- Each play constructs a fresh `Decoder` from a `Cursor<Arc<[u8]>>` (cheap Arc clone, no buffer copy) so multiple instances of the same sound can overlap
- Active sounds are tracked in a `Vec<ActiveSound>` and reaped on `update()` when their sinks empty
- Non-blocking audio playback via rodio's sink system
- `OutputStream` is kept alive via a `_stream` field (dropping it would kill all audio)

#### SoundHandle (`sound.rs`)
Unique identifier for loaded sounds:
- Auto-incrementing IDs via `AtomicU32` (thread-safe counter starting at 1)
- `id` field is `pub(crate)`; external access via `id()` accessor
- Used to reference cached sounds for playback

#### SoundSettings (`sound.rs`)
Configuration for individual sound playback:
- Volume (0.0 to 1.0, clamped — re-clamped at point of use since fields are public)
- Speed/pitch (minimum 0.1, no upper clamp — also re-clamped at point of use)
- Looping flag
- Builder pattern for fluent configuration

### ECS Integration (`crates/ecs/src/audio_components.rs`)

ECS-facing audio components live in the `ecs` crate (not this one) to avoid a circular dep: `audio` is a leaf crate that the engine wires together, while `ecs` owns component storage.

#### AudioSource
Component for entities that emit sounds:
- Sound ID reference (matches `SoundHandle::id()`)
- Volume, pitch, looping settings
- `play_on_spawn` / `playing` flags for state-driven playback
- Spatial audio support with:
  - Max distance (silence beyond)
  - Reference distance (full volume within)
  - Rolloff factor (inverse-distance model)
- `calculate_attenuation(distance)` — inverse distance attenuation:
  `attenuation = reference_distance / (reference_distance + rolloff_factor * (distance - reference_distance))`

#### AudioListener
Component for the entity that "hears" sounds:
- Active flag (only one listener should be active per scene)
- Listener-specific volume multiplier

#### PlaySoundEffect
One-shot sound effect request component:
- Designed for event-based sounds (jump, explosion)
- Expected to be consumed/removed after processing by an audio system

## Dependencies

- `rodio` 0.20 with `symphonia-all` feature — audio playback + format decoding
- `log` - Logging
- `thiserror` - Error handling

## Supported Formats

Via `rodio`'s `symphonia-all` feature:
- WAV
- MP3
- OGG/Vorbis
- FLAC
- AAC

## Usage Example

```rust
// Load a sound
let jump_sound = ctx.audio.load_sound("assets/jump.wav")?;

// Play with default settings (handle is a 4-byte Copy type, passed by value)
ctx.audio.play(jump_sound)?;

// Play with custom settings
let settings = SoundSettings::new()
    .with_volume(0.8)
    .with_speed(1.2);
ctx.audio.play_with_settings(&jump_sound, settings)?;

// Background music (looping); play_music_once for one-shot tracks
ctx.audio.play_music("assets/music.ogg")?;
ctx.audio.play_music_once("assets/stinger.ogg", 0.8)?;
ctx.audio.pause_music();
ctx.audio.resume_music();

// Stop all instances of one sound, or everything
ctx.audio.stop(jump_sound);
ctx.audio.stop_all();

// Volume control
ctx.audio.set_master_volume(0.5);
ctx.audio.set_sfx_volume(0.8);
ctx.audio.set_music_volume(0.6);
```

## Test Coverage

- 20 unit tests in `manager.rs` — all headless (disabled mode + bytes/temp-file APIs): settings builder/clamping, handle uniqueness, load from file/bytes (happy + IoError + DecodeError paths), unload/unload_all, stop/stop_all, volume setter clamping, disabled-mode music semantics
- 7 unit tests in `crates/ecs/src/audio_components.rs` (AudioSource builder/spatial/attenuation, AudioListener, PlaySoundEffect)
- Run with `cargo test -p audio` for audio-crate tests only; ECS component tests run under `cargo test -p ecs`

## Known Limitations

These are intentional design decisions, not technical debt:

1. **No streaming for large files** - All sounds loaded into memory for instant playback
2. **No 3D/spatial audio processing** - Only attenuation calculation provided; no stereo panning or HRTF (would require 3rd party library)
3. **No audio effects** - No reverb, echo, or other DSP effects (use external audio tools)
4. **Single music track** - Only one music track at a time (sufficient for most 2D games)
5. **Disabled mode never reports music as playing** - `play_music*` returns `Ok` (the file is still validated) but `is_music_playing()` stays `false`; documented on the API.

## Future Enhancements

1. Add streaming for large music files
2. Implement actual spatial audio positioning (stereo panning based on listener orientation)
3. Add crossfade for music transitions
4. Add audio effects processing (reverb, low-pass filter for occlusion)
5. Support for generic audio buses/groups (currently fixed master/sfx/music)
6. Audio occlusion for environments
