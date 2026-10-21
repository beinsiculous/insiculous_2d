# Audio Crate — Agent Context

Audio playback via rodio (sound effects + background music). **No spatial audio
lives in this crate, and no audio component lives anywhere**: a game plays sound from
its own code through `ctx.audio`. Scene-authored audio (a sound component, spatial
playback) is the Conductor milestone's feature.

## Files
- `manager/mod.rs` — `AudioManager` device/output lifecycle, load/cache, SFX playback, and active sound tracking.
- `manager/music.rs` — music playback (play/stop/pause/resume) and volume buses (`update_all_volumes`).
- `sound.rs` — `SoundHandle` (Copy id) and `SoundSettings` (builder: volume/speed/looping).
- `sample_loader.rs` — `load_sample_pcm(path)` / `decode_sample_pcm(bytes)`: a sample
  instrument's WAV, read through `common::vfs` and decoded with rodio's WAV decoder into
  `music_midi::OwnedPcm`; `SampleProblem`, `SAMPLE_BYTE_LIMIT` (4 MiB). Tests in
  `sample_loader/tests.rs`.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| `_stream: OutputStream` on `AudioManager` must be kept alive; dropping it kills all audio playback | — none |
| `AudioManager` on wasm32 must start disabled because browsers gate audio behind a user gesture, enabling on first user interaction | `src/manager/tests.rs test_enable_output_keeps_handles_ids_buses_and_pending_music_whatever_the_outcome` |
| Clamping volume and speed at point of use: `SoundSettings` fields are public, so out-of-range volume or speed must be clamped before reaching sinks | `src/manager/tests.rs test_sound_settings_clamp_volume_to_unit_range_and_floor_speed_at_a_tenth` |
| Unloaded sound handle must be rejected and its ID never recycled | `src/manager/tests.rs test_unloaded_handle_is_rejected_and_its_id_is_never_recycled` |
| Missing audio files produce IO errors while corrupt data produces decode errors naming the file | `src/manager/tests.rs test_missing_files_are_io_errors_and_undecodable_data_is_a_decode_error_naming_the_file` |
| A sample file's byte limit is checked before decoding, the decoder's rate and channels before its samples, and the decode stops one frame past 10 s, so nothing oversized is built only to be refused | `src/sample_loader/tests.rs a_file_over_four_mebibytes_is_refused_before_it_is_decoded` `src/sample_loader/tests.rs a_wav_outside_the_sample_limits_is_refused_naming_the_limit` |
| WAV is the only pitched-sample import, told by its RIFF header and never by its name: a file called `.wav` may hold anything, and rodio decodes more formats than the import supports | `src/sample_loader/tests.rs a_stream_that_is_not_wav_is_refused_naming_wav_whatever_the_file_is_called` |
| rodio 0.20's decoder yields 16-bit integers, so a float WAV's NaN arrives finite; the non-finite refusal is `OwnedPcm::new`'s, for any other producer | `src/sample_loader/tests.rs a_float_wav_carrying_nan_decodes_finite_because_the_decoder_yields_sixteen_bit_integers` |
| Every sample-loader error names the path, an unreadable file included (unlike `IoError`) | `src/sample_loader/tests.rs a_file_that_cannot_be_read_is_an_error_naming_its_path` |

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
  sinks (music + SFX + sources) by `set_master_volume` / `set_sfx_volume` / `set_music_volume`.
- `play_source(source, Bus::Music | Bus::Sfx)` plays any caller-built
  `Source<Item = f32>` (a synthesizer, a sequencer) until it ends, its sink kept
  so bus and master changes reach it (base volume 1.0). Disabled mode drops the
  source unpulled and returns `Ok`.
- Clamping happens at point of use (`clamp_volume` 0..=1, `clamp_speed` floor
  0.1) because `SoundSettings` fields are public.
- `play(handle)` / `unload(handle)` take `SoundHandle` by value (Copy);
  `play_with_settings(&handle, settings)` keeps a reference (external callers).
- `stop(handle)` stops all active instances of one sound. Unknown handles are a no-op.
- Music: `play_music` / `play_music_with_volume` loop forever. No crossfade.
- Errors: file-read failures are `AudioError::IoError` (`#[from] io::Error`);
  undecodable data is `DecodeError`; `LoadError` reserved for non-IO load problems;
  a sample the loader refuses is `AudioError::Sample { path, problem }`.
- Sample instruments: `load_sample_pcm(path)` decodes a WAV once, before anything
  renders, into the conductor's `OwnedPcm` (8–96 kHz, mono or stereo, 1 ms–10 s,
  ≤ 4 MiB of file). Only WAV, by its RIFF header; recorded MP3/OGG still play through
  `load_sound`. The crate depends on `music_midi` with default features off: the PCM type
  and its limits, nothing of the synthesizer.
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
