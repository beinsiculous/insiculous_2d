# Audio Crate — Agent Context

Audio playback via rodio (sound effects + background music). **No spatial audio
lives in this crate**: the ECS components `AudioSource`, `AudioListener`, and
`PlaySoundEffect` are in `crates/ecs/src/audio_components.rs` and are currently
editor-inspectable data only — no runtime system consumes them (a bridging
audio system is future work).

## Files
- `lib.rs` — crate docs + re-exports (`AudioManager`, `SoundHandle`, `SoundSettings`, `AudioError`, `AudioResult`)
- `manager/mod.rs` — `AudioManager` struct + device/output lifecycle, load/cache, SFX playback, stop-by-handle
- `manager/music.rs` — music playback (play/stop/pause/resume) + the volume buses (`update_all_volumes`)
- `manager/tests.rs` — headless tests for both seams (shared WAV fixtures)
- `sound.rs` — `SoundHandle` (Copy id), `SoundSettings` (builder: volume/speed/looping)
- `error.rs` — `AudioError` (thiserror) + `AudioResult<T>` alias

## Key Types & Behavior
- `AudioManager::new_or_disabled()` — never fails; *disabled* mode (no audio
  device) still loads/validates sounds, playback no-ops. In disabled mode
  `play_music*` returns `Ok` but `is_music_playing()` stays `false` (documented).
  On wasm32 it ALWAYS starts disabled (browsers gate audio behind a user
  gesture).
- `enable_output()` (H7, Aug 2026) — upgrades a disabled manager to a real
  `OutputStream`; no-op `Ok` when already enabled, `Err` leaves it disabled
  and fully functional. Sounds loaded while disabled keep their handles/ids;
  bus volumes carry over; a `play_music` issued while disabled is remembered
  (`pending_music`, last request wins, cleared by `stop_music`) and starts on
  a successful enable. The engine calls this from the first user activation
  gesture on the web (`engine_core/src/game/web.rs`); `new()` is now
  `disabled()` + `enable_output()?` — one construction path.
- Path-based loads (`load_sound`, music) read through `common::vfs` (Aug 2026)
  — std::fs natively, the prefetched in-memory map on the web — so the same
  path API works on both targets. Music decodes from an in-memory `Cursor`
  (eager, like all audio here).
- Sound bytes cached as `Arc<[u8]>`; each play decodes from
  `Cursor<Arc<[u8]>>` — no buffer copy per play.
- Volume model: sink volume = `base * bus * master`, re-applied to all live
  sinks (music + SFX) by `set_master_volume` / `set_sfx_volume` / `set_music_volume`.
- Clamping happens at point of use (`clamp_volume` 0..=1, `clamp_speed` floor
  0.1) because `SoundSettings` fields are public.
- `play(handle)` / `unload(handle)` take `SoundHandle` by value (Copy);
  `play_with_settings(&handle, settings)` keeps a reference (external callers).
- `stop(handle)` stops all active instances of one sound; `stop_all()` stops
  every SFX (music unaffected). Unknown handles are a no-op.
- Music: `play_music` / `play_music_with_volume` loop forever;
  `play_music_once(path, volume)` plays one-shot. No crossfade.
- Errors: file-read failures are `AudioError::IoError` (`#[from] io::Error`);
  undecodable data is `DecodeError`; `LoadError` reserved for non-IO load problems.
- `unload` does not cut off already-playing instances (each holds its own Arc).

## Known Limitations (by design — current constraints, not open work)
- **No streaming for large files** — all audio loaded eagerly into memory for
  instant playback
- **No audio effects** — no reverb, echo, or other DSP; **single music track**
  at a time (no crossfade); fixed master/sfx/music buses
- **`IoError` carries no file path** (`#[from] io::Error` loses it; decode
  errors include the path)
- **No AudioContext resume after tab-switch suspension (web)** — a browser may
  suspend the context in a background tab; rodio/cpal expose no resume hook,
  so audio can stay silent until reload. Accepted #8 scope cut.
- Future-enhancement ideas (streaming, ECS spatial bridge, crossfade, DSP,
  generic buses, occlusion) are roadmap material, not debt.

Open debt lives on the Studio Board (`tech-debt` label); this crate currently
has none.

## Testing
- `cargo test -p audio` — headless (disabled mode + bytes/temp-file APIs +
  enable_output/pending-music). No audio
  device needed — enable_output tests match on the result and assert the
  invariants of whichever branch ran.

## Godot Oracle
- Audio architecture: `servers/audio_server.cpp`, `scene/audio/audio_stream_player.cpp`
