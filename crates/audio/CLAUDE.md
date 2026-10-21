# Audio Crate — Agent Context

Audio playback via rodio, in three seams: **the device** (the output connection, its
web gesture gate and the master/sfx/music buses), **recorded sound and music** (sound
effects and one recorded music track, decoded from files), and **the song** (a MIDI file
and its `.mid.ron` sidecar played by `music_midi`'s conductor, with triggers and beat
queries). This is the only crate that names rodio. **No spatial audio lives in this
crate, and no audio component lives anywhere**: a game plays sound from its own code
through `ctx.audio`. Scene-authored audio (a sound component, spatial playback) is the
Conductor milestone's feature.

## Files
- `manager/mod.rs` — **the device**: `AudioManager`, its output (`enable_output`, the
  disabled and gesture-gated modes), sound loading and SFX playback, `play_source` (any
  rodio `Source` on a bus), `update`/`update_at`, and the `PendingMusic` slot both kinds
  of music share.
- `manager/music.rs` — **recorded music** (play/stop/pause/resume) and the volume buses
  (`update_all_volumes`, which also hands the music and sfx gains to a playing song).
- `manager/song.rs` — **the song**: `play_song`, `prepare_song`, `play_song_in_memory`,
  `stop_song`, `pause_song`, `resume_song`, `seek_song(beat)`, `set_song_looping`,
  `is_song_playing`, `song_position`, `song_snapshot`, `beat_crossed`, `trigger`,
  `deactivate_source`, `stop_all_sources`, `set_asset_base`; `SongPosition`,
  `SongSnapshot`; the playback state, the commands waiting for room, the virtual clock,
  the dead-output path.
- `manager/song/load.rs` — a song's dependency closure through `common::vfs` (the `.mid`,
  the `.mid.ron`, every SoundFont and sample, each refused by size before it is read),
  `ResourceMetadata` filled in, validation, the conductor built; `DependencyCache`
  (decoded SoundFonts and samples by content fingerprint); `SongProblem`,
  `SONG_FILE_BYTE_LIMIT` (256 KiB), `SOUND_FONT_BYTE_LIMIT` (48 MiB).
- `manager/song/edges.rs` — `Subdivision` and the beat edges latched each update.
- `conductor_source.rs` — `ConductorSource`, the conductor as a rodio `Source`: one
  64-frame block rendered per pull at its own instant, the bus gains read from two atomics
  per block, `SourceReadings` (limited samples, the frames not yet handed out, render
  cost, the drain gap).
- `conductor_source/limiter.rs` — the peak limiter: a gain lowered ahead of a peak (the
  rendered block is its look-ahead) and let back up after, so nothing passes
  `LIMITER_THRESHOLD` (0.9); `LIMITER_ATTACK_SECONDS` (1 ms), `LIMITER_RELEASE_SECONDS`
  (80 ms).
- `sound.rs` — `SoundHandle` (Copy id), `SoundSettings` (builder: volume/speed/looping), `Bus`.
- `sample_loader.rs` — `load_sample_pcm(path)` / `decode_sample_pcm(bytes)`: a sample
  instrument's WAV, refused by its size before it is read, read through `common::vfs` and
  decoded with rodio's WAV decoder into `music_midi::OwnedPcm`; `SampleProblem`,
  `SAMPLE_BYTE_LIMIT` (4 MiB). Tests in `sample_loader/tests.rs`.
- `test_support.rs` — test-only: the sine fixtures, a hand-built song in a temporary
  asset directory.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| `_stream: OutputStream` on `AudioManager` must be kept alive; dropping it kills all audio playback | — none |
| `AudioManager` on wasm32 must start disabled because browsers gate audio behind a user gesture, enabling on first user interaction | `src/manager/tests.rs test_enable_output_keeps_handles_ids_buses_and_pending_music_whatever_the_outcome` |
| Clamping volume and speed at point of use: `SoundSettings` fields are public, so out-of-range volume or speed must be clamped before reaching sinks | `src/manager/tests.rs test_sound_settings_clamp_volume_to_unit_range_and_floor_speed_at_a_tenth` |
| Unloaded sound handle must be rejected and its ID never recycled | `src/manager/tests.rs test_unloaded_handle_is_rejected_and_its_id_is_never_recycled` |
| Missing audio files produce IO errors while corrupt data produces decode errors naming the file | `src/manager/tests.rs test_missing_files_are_io_errors_and_undecodable_data_is_a_decode_error_naming_the_file` |
| A sample file's byte limit is checked by its size before it is read, the decoder's rate and channels before its samples, and the decode stops one frame past 10 s, so nothing oversized is read or built only to be refused | `src/sample_loader/tests.rs a_file_over_four_mebibytes_is_refused_by_its_size_before_it_is_read` `src/sample_loader/tests.rs a_file_over_four_mebibytes_is_refused_before_it_is_decoded` `src/sample_loader/tests.rs a_wav_outside_the_sample_limits_is_refused_naming_the_limit` |
| WAV is the only pitched-sample import, told by its RIFF header and never by its name: a file called `.wav` may hold anything, and rodio decodes more formats than the import supports | `src/sample_loader/tests.rs a_stream_that_is_not_wav_is_refused_naming_wav_whatever_the_file_is_called` |
| rodio 0.20's decoder yields 16-bit integers, so a float WAV's NaN arrives finite; the non-finite refusal is `OwnedPcm::new`'s, for any other producer | `src/sample_loader/tests.rs a_float_wav_carrying_nan_decodes_finite_because_the_decoder_yields_sixteen_bit_integers` |
| Every sample-loader error names the path, an unreadable file included (unlike `IoError`) | `src/sample_loader/tests.rs a_file_that_cannot_be_read_is_an_error_naming_its_path` |
| A song and a recorded track share one music slot: playing either stops the other, and before the web's gesture the last request of either kind is the one pending; a pending song is not playing and has no position | `src/manager/song/tests.rs a_song_and_a_recorded_track_share_one_pending_slot_the_last_request_winning_through_the_gesture` |
| A disabled manager is one of two things: gesture-gated (the web) holds a song pending, headless (tests, `GameHarness`, no device) plays it on a virtual clock advanced by `update()`'s wall time, capped at one second an update | `src/manager/song/tests.rs without_a_device_a_playing_song_advances_by_the_wall_time_between_updates` |
| Beat edges are counted on the published sample by integer boundaries and latched once an update: a long gap reports one crossing, not a flood; a wrap reports its downbeat, a seek back nothing | `src/manager/song/tests.rs beat_edges_count_every_boundary_once_and_a_ten_beat_gap_reports_one_crossing` `src/manager/song/tests.rs a_loop_wrap_reports_its_downbeat_and_a_seek_back_reports_nothing` |
| Transport, deactivations and flushes a full queue refuses wait in the manager and go first, in order, from `update()`; a trigger is refused while one waits, so it never overtakes them | `src/manager/song/tests.rs transport_a_full_queue_refuses_waits_and_is_sent_in_order_before_any_trigger` `src/manager/song/tests.rs ten_thousand_triggers_never_lose_a_deactivation_a_flush_or_a_pause` |
| Each song gets a conductor of its own, built on the game thread, so a song change neither allocates on the audio thread nor lets an old binding index reach the new song; the flush floor is the one thing carried, sent after `Play` | `src/manager/song/tests.rs a_song_change_carries_the_flush_and_the_next_trigger_reactivates_under_a_new_epoch` |
| A song's whole closure is loaded and checked before the playing song is touched, in disabled mode too; every refusal names the song, the sidecar entry and the path | `src/manager/song/tests/loading.rs a_missing_sample_a_missing_bank_or_an_oversized_file_is_refused_and_the_song_plays_on` `src/manager/song/tests/loading.rs a_missing_sidecar_is_refused_naming_it_without_touching_recorded_music` |
| The recorded path keeps the same rule: `play_music` reads and decodes before it stops anything, so a missing or undecodable track leaves a playing or pending song (and the playing track) alone | `src/manager/song/tests/loading.rs a_recorded_track_that_fails_to_load_leaves_a_playing_or_pending_song_alone` |
| Decoded SoundFonts and samples are cached by the file's content fingerprint, not its path or the song: an edit at the same path decodes again, an unchanged file never does, and a song built in memory shares them | `src/manager/song/tests/loading.rs unchanged_files_are_decoded_once_and_a_file_changed_at_its_path_is_decoded_again` `src/manager/song/tests/loading.rs a_song_built_in_memory_shares_the_decoded_files_of_the_song_on_disk` |
| The song's output is dead when its conductor is gone (the device let the source go, or a send finds no receiver): the song stops with one warning, and every later call sees it stopped | `src/manager/song/tests.rs a_dead_output_stops_the_song_with_one_warning_and_every_later_call_sees_it_stopped` |
| The background is mixed at the music gain and the instruments at the sfx gain inside the source, master on the sink: music at 0 keeps triggers audible, sfx at 0 mutes them, and the clock runs at both | `src/conductor_source/tests.rs music_at_zero_keeps_triggers_audible_and_sfx_at_zero_mutes_them_while_the_clock_runs` |
| The limiter turns the gain down, it never bends the waveform: a stateless soft knee clipped a held key's four-times-full-scale mix into distortion. It falls at the attack rate ahead of a peak it has seen in the block, holds every sample at 0.9, and lets the gain back up to exactly unity | `src/conductor_source/tests.rs a_loud_passage_is_turned_down_ahead_of_it_never_past_the_threshold_and_let_back_up` `src/conductor_source/tests.rs the_limiter_holds_the_worst_case_storm_at_its_threshold_with_every_sample_finite` `src/conductor_source/tests.rs a_quiet_stream_passes_the_limiter_untouched` |
| The limiter's gain is kept as its reduction: stepped towards one in f32 the gain stalls short of unity and the limiter never lets go | `src/conductor_source/tests.rs a_loud_passage_is_turned_down_ahead_of_it_never_past_the_threshold_and_let_back_up` |
| A sample voice plays at `SAMPLE_VOICE_LEVEL` (a fifth of the recording) before its gain, so a key held on a sample instrument sits near one held on a preset instead of at twelve times it | `src/conductor_source/tests.rs a_key_held_on_a_sample_instrument_plays_near_the_level_of_one_held_on_a_preset` |
| `latency_samples` counts only what this engine holds — the frames of the current block not yet handed out; the device's buffer lies beyond and is not known | `src/conductor_source/tests.rs the_frames_of_a_block_not_yet_handed_out_are_what_the_latency_counts` |

## Key Types & Behavior
- `AudioManager::new_or_disabled()` — never fails; *disabled* mode (no audio
  device) still loads/validates sounds; sound and recorded-music playback no-ops. In
  disabled mode `play_music*` returns `Ok` but `is_music_playing()` stays `false`
  (documented). **A song is the exception:** a headless disabled manager plays it on
  a virtual clock advanced by `update()` (capped at one second an update), so
  `is_song_playing()` is `true` and `song_position()` moves; a gesture-gated one holds
  it pending. Neither query is a probe for a device — `is_enabled()` is.
  On wasm32 it ALWAYS starts disabled and gesture-gated (browsers gate audio
  behind a user gesture).
- `enable_output()` (H7, Aug 2026) — upgrades a disabled manager to a real
  `OutputStream`; no-op `Ok` when already enabled, `Err` leaves it disabled
  and fully functional. Sounds loaded while disabled keep their handles/ids;
  bus volumes carry over; music issued while disabled — a recorded track or a
  song — is remembered (`pending_music`, one slot, last request of either kind
  wins, cleared by `stop_music` or `stop_song` for its own kind) and starts on a
  successful enable; a song playing on the virtual clock moves to the device.
  The engine calls this from the first user activation gesture on the web
  (`engine_core/src/game/web.rs`); `new()` is `disabled()` + `enable_output()?`.
- Path-based loads (`load_sound`, music, songs) read through `common::vfs` —
  std::fs natively, the prefetched in-memory map on the web — so the same path
  API works on both targets. Recorded music decodes from an in-memory `Cursor`
  (eager, like all audio here).
- Sound bytes cached as `Arc<[u8]>`; each play decodes from
  `Cursor<Arc<[u8]>>` — no buffer copy per play.
- Volume model: sink volume = `base * bus * master`, re-applied to all live
  sinks (music + SFX + sources) by `set_master_volume` / `set_sfx_volume` / `set_music_volume`.
  A song's sink carries master alone; its music and sfx gains reach the source
  as f32 bit patterns in two atomics, read once a block.
- `play_source(source, Bus::Music | Bus::Sfx)` plays any caller-built
  `Source<Item = f32>` until it ends, its sink kept so bus and master changes
  reach it (base volume 1.0). Disabled mode drops the source unpulled and returns `Ok`.
- Clamping happens at point of use (`clamp_volume` 0..=1, `clamp_speed` floor
  0.1) because `SoundSettings` fields are public.
- `play(handle)` / `unload(handle)` take `SoundHandle` by value (Copy);
  `play_with_settings(&handle, settings)` keeps a reference (external callers).
- `stop(handle)` stops all active instances of one sound. Unknown handles are a no-op.
- Music: `play_music` / `play_music_with_volume` loop forever. No crossfade.
- **Songs.** `play_song(path)` reads `path` and `<path>.ron`; the sidecar's
  SoundFont and sample paths are relative to `set_asset_base` (the game's asset
  directory, which `engine_core`'s runner sets from `GameConfig::asset_base` when it
  builds the manager; the working directory until set). Every song renders at 44.1 kHz
  in a conductor of its own (the device's mixer resamples). `trigger(action,
  source)` resolves the action through a table built at load and stamps the
  trigger with `common::clock::Instant::now()`; an unbound action is
  `AudioError::UnknownAction` naming it. `song_position()` reports the start of
  the next block — where a scheduled trigger fires, up to 63 frames ahead of the
  frame playing — and `latency_samples` beyond it. `beat_crossed(Subdivision)`
  reads the edges the last `update()` latched. `stop_song` stops the sink, which
  drops the source and its conductor.
- Errors: file-read failures are `AudioError::IoError` (`#[from] io::Error`);
  undecodable data is `DecodeError`; `LoadError` reserved for non-IO load problems;
  a sample the loader refuses is `AudioError::Sample { path, problem }`; a song
  is `AudioError::Song { song, entry, path, problem }`; `SongNotPlaying` and
  `CommandRefused` answer song calls that cannot be made.
- Sample instruments: `load_sample_pcm(path)` decodes a WAV once, before anything
  renders, into the conductor's `OwnedPcm` (8–96 kHz, mono or stereo, 1 ms–10 s,
  ≤ 4 MiB of file). Only WAV, by its RIFF header; recorded MP3/OGG still play through
  `load_sound`. The crate depends on `music_midi` with its `synth` feature: the
  conductor plays here, and its crate never does I/O.
- `unload` does not cut off already-playing instances (each holds its own Arc).

## Known Limitations (by design — current constraints, not open work)
- **No streaming for large files** — all audio loaded eagerly into memory for
  instant playback
- **No audio effects** — no reverb, echo, or other DSP; **single music track**
  at a time (no crossfade); fixed master/sfx/music buses. The song's limiter is a
  safety net, not a mastering stage: it ducks the whole mix, so one loud instrument
  turns every other one down with it.
- **`IoError` carries no file path** (`#[from] io::Error` loses it; decode
  errors include the path)
- **No AudioContext resume after tab-switch suspension (web)** — a browser may
  suspend the context in a background tab; rodio/cpal expose no resume hook,
  so audio can stay silent until reload. Accepted #8 scope cut.
- **A song's position runs ahead of the sound by the device's buffer**, which
  rodio does not report; `latency_samples` counts only the engine's own block.
- Future-enhancement ideas (streaming, ECS spatial bridge, crossfade, DSP,
  generic buses, occlusion) are roadmap material, not debt.

Open debt lives on the Studio Board (`tech-debt` label); this crate currently
has none.

## Testing
- `cargo test -p audio` — headless (disabled mode + bytes/temp-file APIs +
  enable_output/pending music + the song seam on the virtual clock, driven by
  `update_at`). No audio device needed — enable_output tests match on the result
  and assert the invariants of whichever branch ran.

## Godot Oracle
- Audio architecture: `servers/audio_server.cpp`, `scene/audio/audio_stream_player.cpp`
