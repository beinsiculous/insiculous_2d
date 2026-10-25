# Engine Core Crate — Agent Context

Core engine: Game trait, run_game(), managers, scene loading/saving, asset management.

## Key Types
- `Game` trait — `init()`, `update()`, `on_key_pressed()` — the public API for games
- `GameConfig` — window title, size, clear color, **`chaos_mode`**, **`texture_filter`** (default sampling for loaded textures; `TextureFilter::Nearest` for pixel art)
- `run_game(game, config)` — entry point, creates window + event loop
- `GameContext` — passed to Game methods: world, input, **players** (per-player
  `InputSettings`: `ctx.players.is_active(PlayerId::P1, GameAction::Action1, ctx.input)`,
  `move_x/move_y`), assets, audio, **sources** (`SoundSources`: the song source an
  entity, the game or the menu triggers under), **emitters** (`SoundEmitters`: play and
  stop an entity's `SoundEmitter`), ui, delta_time, **chaos_mode**, **time_scale**
  (read-write; scales the frame tail — particles, animation, lifetimes, clip
  machines, grid backdrops — set 0.0 while paused; `delta_time` is never scaled).
  and `request_exit()` (clean engine shutdown, same path as window close).
  Physics is not on the context: a game owns and steps its own `PhysicsSystem`.
- `ChaosMode` — cross-game Normal/Insane/Ridiculous/Insiculous theme (engine carries the selection, games define the meaning)
- Managers: `GameLoopManager`, `RenderManager`, `WindowManager`

## Crate Boundaries
Cross-cutting glue biases toward `engine_core`: `ui` defines `DrawCommand` (renderer-agnostic), `renderer` defines `Sprite` (UI-agnostic), and `engine_core` owns the bridge in `ui_integration`. This keeps `renderer` and `ui` independently testable and prevents either crate from depending on the other transitively through `engine_core`. The dual glyph cache is intentional: `ui` caches rasterized bitmaps (`font/glyph_cache.rs`) to avoid re-rasterization, while `engine_core` caches GPU textures (`glyph_texture_cache.rs`) to avoid re-uploads.

## File Map
- `game.rs` — `run_game()` and `GameRunner` orchestration (it re-exports `Game`); new render passes go in their own module like `tilemap_render.rs`. `step_frame` is the window-free middle of `update_and_render` (event flush through `input.end_frame()`); the window loop and `GameHarness` both call it, so a frame change lands in both.
- `game/game_trait.rs` — the `Game` trait: the contract a game implements, including the default `render` that draws tilemaps, then ECS sprites, then UI.
- `game/save_documents.rs` — opens the runner's three save documents (achievements, scores, input bindings) from `GameConfig`: persisted under a save path, folding in a renamed game's former path when the config names one, in memory otherwise.
- `game/headless.rs` (test builds only) — the runner's seams for `GameHarness`: `GameRunner::headless` (mirrors what `run_game` and `resumed` do before the first frame, with `AssetManager::headless` in place of the renderer's, the disabled gamepad backend and the disabled audio manager, whose songs run on its virtual clock), the input queue, key-handler dispatch, a resize, `with_context`, and read-back accessors (the title reads a pending request first, so a lent context's title is visible before any frame; the audio manager, the sound sources and the sound emitters).
- `test_support.rs` (`cfg(test)` or the `test-support` feature) — scene round-trip fixtures (`StubResolver`, `roundtrip`, `load_ron`), the input `frame` helper, and `GameHarness`, which drives a game's update loop headlessly through the real runner and frame (`training.md` § Writing Tests, "Driving a game's update loop").
- `game/app_handler.rs` — winit `ApplicationHandler<WakeUp>`: native frame driving uses `about_to_wait` while wasm uses `RedrawRequested`; never unify them (an occluded native window stops receiving redraws). A hidden document gets no animation frames, so on wasm `user_event` drives a frame from the `WakeUp` events `web::install_hidden_frame_pump` sends through the proxy `run_game` publishes.
- `game/web.rs` (wasm-only) — async renderer bring-up (adopted surface starts 1×1) and gesture-gated audio enable (retries capped at 5 failures, hooked pre-match so audio is live before `on_key_pressed`).
- `web/mod.rs` (wasm-only) — `preload_assets` fetches manifest and entries into `common::vfs` under `{base}/{entry}` keys before `run_game`; the page-exit latch; `set_boot_status`/`boot_status` over the page's `game-loading` element; the wake proxy and `install_hidden_frame_pump` (a 100 ms timer that drives frames while the document is hidden; one chain at a time, started by the visibility change, or by the proxy's arrival when the page was hidden all through its boot).
- `startup_splash.rs` — the startup cards a game names (`GameConfig::startup_splashes`) and the lifecycle that holds the game behind them (`Pending` → `Splashing` → `Handoff` → `Running`), their clock, skip, quiet period and drawing (a black base, the backdrop as four margins around the card, the card — nothing drawn over what fades, because the UI batcher orders whole texture batches by their lowest depth). `game/startup_frame.rs` is the runner's side: the frame the cards own, and the quiet period's edge reset. The key-event path, `render` and `GameHarness::context()` are gated on the same `holds_game()`.
- `game/frame_tail.rs` — post-update tail: particles, then `step_world_systems` — `ecs::SpriteAnimationSystem`, `ecs::LifetimeSystem`, `ecs::ClipStateMachineSystem`, in that order — on `delta_time * time_scale` so pausing freezes all of it, then the song sources settle (the scripts' queued sounds submitted — triggers, and emitter plays and stops in one order — then every gone entity's source ended; a filed `TransformHierarchyReset` flushes every source, clears every emitter and drops the frame's sounds instead, whatever the time scale — read before the hierarchy consumes it), then the runner's `TransformHierarchySystem` (it ignores the delta, so a paused game and the editor's Editing state still propagate), then the sound emitters' pass — first sight only while `time_scale` is not zero; the requests, the re-mix and the eviction every frame — then the grid backdrops, which read `GlobalTransform2D`. Every game gets lifetimes, clip machines and hierarchy propagation from here; a game must not own a `LifetimeSystem` of its own (it would halve every lifetime) and needs no hierarchy system. Code that replaces the world wholesale files `TransformHierarchyReset` through `request_transform_hierarchy_reset`, which the next tail consumes.
- `sound_sources.rs` — `SoundSources`, lent as `ctx.sources`: an entity's `SourceId` (its id and generation, and an epoch from the one host-wide activation counter, kept until the entity is gone), the game's and the menu's at the top two `u64` ids, `trigger` (a refusal counted in `refused_triggers`, logged at debug), `reconcile`, `end` (one entity's activation, a binding emitter's stop) and `flush`. Not behind `physics`: a game without scripting still reconciles.
- `game_music.rs` — `GameMusic`, a game's song: requested at `start` with the first failure logged at error, `restart` deciding by the song in the slot (none → request; the game's own path → seek to the top; another → left alone), the pause menu's jam (`follow_pause` triggers the game's `menu` action on `ctx.sources.menu()` and nothing on the menu's opening frame), `beat_pulse` (bar first, at most one an update) and the five-second `conductor=on` telemetry line with the game's counts appended. The game keeps its bindings, its moments and its visuals; tested in `tests/game_music.rs`.
- `sound_emitters.rs` — `SoundEmitters`, lent as `ctx.emitters`: the table of playing `SoundEmitter`s (the component is data; first sight, the instance and the last mix live here), `play(entity, world)`/`stop` requests (capped at `EMITTER_REQUEST_CAPACITY`; a play carries the emitter and position it saw, so a one-shot outlives an entity despawned in the same update), the pass the frame tail runs (first sight with `play_on_spawn`, the requests, the re-mix, the eviction), `clear` for a world replacement, and `spatial_mix` — pure, 2D, world units, the main camera the listener. Not behind `physics`.
- `assets/sound_library.rs` — the sound tier: `AssetManager::resolve_sound(path, audio)`, a recorded sound loaded once by its asset-relative path, a failed path warned once and remembered; both maps cleared by `set_base_path`.
- `scripting/target.rs` — `Target` (an entity id or a name) and `TargetResolver`, which reads the world's names once a phase and reports a missing or ambiguous name.
- `scripting/triggers.rs` — the frame's script sounds (`QueuedSound`: `Trigger`, an emitter's `Play` carrying the emitter and position the command saw, `Stop`) in one `TriggerQueue`, filled by `ScriptCommands::apply` in the order commands apply; `ScriptRunner::take_sounds`; and `submit`, which the frame tail calls — a trigger and a binding emitter's play or stop are served there, a recorded emitter's handed on, in order, to the emitter pass.
- `localization.rs` — `Strings`: RON locale tables with `current→en→key` fallback, per-locale font tracking, and `@key` resolution.
- `ui_element_system.rs` — draws `UiLabel`/`UiPanel`/`UiButton` and buffers `UiButtonPressed` on the event bus after the next frame's flush; suppressed by `UiElementsHidden`.
- `gamepad_backend.rs` — gilrs hardware poll (0.15 dead zone rescale, stick +Y = up, pumped before `process_queued_events`).
- `input_settings_io.rs` — JSON persistence for player bindings with missing/corrupt fallback to defaults.
- `save_store/` — player save persistence seam (filesystem path natively, localStorage on wasm with fallback to `MemoryStore`; multi-tab merge-on-save via `JsonSaveSlot`).
- `achievements/` — the registry, its `JsonSaveSlot` persistence, toasts in `toast.rs`, and the manifest export (`--achievements-manifest`, `docs/WEB_SAVES.md` § The manifest).
- `scores.rs` — high scores (top-10 per mode) with merge-on-save for multi-tab safety, while `reset()` overwrites.
- `glyph_texture_cache.rs` — GPU glyph texture cache (dual cache with ui crate's rasterized bitmap cache).
- `render_manager.rs` — `sync_main_camera` copies pose (position and zoom, rotation excluded) onto render camera; device loss fail-stop stops frame loop after `MAX_SURFACE_ERROR_STREAK = 10` or `DeviceLost`.
- `tilemap_render.rs` — expands `Tilemap` and `Transform2D` into the game sprite batcher (one batch per tileset).
- `scene_loader.rs` — RON to World deserialization; `SceneInstance` retains prefabs for runtime `spawn_prefab`.
- `scene_loader_merge.rs` — the layering every entity loads through, public as `SceneLoader::effective_components`: the prefab's components, each override merged field by field (top-level fields only; `Behavior`/`Scripts` replace whole; `Dynamic` merges `data` key by key against the registered default), then the inline `components` replacing whole. A named field the component lacks, a typed/`Dynamic` pair of one name, or a non-object `Dynamic` payload is a load error naming the entity.
- `scene_override.rs` — `OverrideData`: an override's component plus the top-level fields its RON text named (read back through `ron::value::RawValue`, so presence comes from the text, never from comparing against defaults); `OverrideData::fields` for runtime callers.
- `scene_retired_components.rs` — the component names the engine retired (the three audio components): before a load instantiates anything, a `Dynamic` entry of a retired, unregistered name is dropped from every layer with a warning in `SceneInstance.load_warnings`, so an older save still loads; every other unregistered name stays a hard error.
- `scene_serializer.rs` — World to `SceneData` serialization (inverse of `scene_loader`, requires loader match arm and serializer table row for new components).
- `scene_data.rs` — `SceneData` schema; Sprite `emissive`/`tex_region`/`visible` require named serde defaults to avoid blanking old sprites. `SoundEmitter` is typed for its load-time checks (a binding that is spatial or loops, a negative or non-finite volume — a `ComponentError` naming the entity; `max_distance` floored), never for resolution: nothing at load touches audio.
- `script_data.rs` — script wire schema: entity params persist by name, auto-named at save and resolved post-instantiate.
- `texture_ref.rs` — scene texture reference resolution (`#white`, `#solid:RRGGBB`, file paths); `TextureResolver` trait is the GPU/filesystem seam.
- `sheet_file.rs` — `.sheet.ron` schema and validation (`parse_sheet_file` and `into_parts`); `sidecar_path_for` holds the naming rule.
- `assets/sprite_sheet.rs` — `AssetManager::load_sprite_sheet` order: read sidecar, parse, probe PNG dims, validate, then load texture so bad sheets leave no handle.
- `assets.rs` — asset loading; `create_texture_from_rgba` validates before device with `"#rgba"` sentinel (does not survive save/load).
- `behavior_runner/` — entity behavior system: runner dispatch loop, handlers, and `CameraFollow` with look-ahead.
- `scripting/` — game scripting subsystem: `ScriptRegistry`, `ScriptRunner`, Rhai backend, and built-in behaviors.
- `pause.rs` — shared `PauseMenu`; headless-testable via `&InputSettings + &InputHandler + window_size` (mouse reads live inside paused branch only).
- `menu_panel.rs` — `MenuPanel` chrome; resting cursor never fights keyboard nav, click selects and confirms.
- `title_art.rs` — a title screen's art over its menu: `TitleArt::load` measures the art's last content row, `title_layout` places art and panel (or falls back to the centred panel), `draw_title_art` fills and draws; one layout serves drawing and the mouse.
- `menu_input.rs` — shared menu-screen input handling (keyboard, arrows, and gamepad navigation with wraparound).
- `spawn_helpers.rs` — shared entity recipes; `RENDER_UNIT = 80.0` pixels per world unit lives at crate root.
- `pickups.rs` — generic pickup/collectible tracking (started-collision events vs collector set once per pickup).
- `ui_integration/` — UI-to-renderer bridge: camera-relative UI sprite positioning, SDF shapes, and physical pixel clip rects driving `SpriteBatcher::set_clip`.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Never unify the native and wasm frame drivers: an occluded or minimized native window stops receiving redraws | — none |
| Wasm audio upgrade on user gesture retries capped at 5 attempts to avoid unbounded churn | — none |
| Web asset preloading must populate `common::vfs` before `run_game` is called | — none |
| Surface error streak or `DeviceLost` triggers fail-stop and halts the frame loop to prevent dead queue submission | `src/render_manager.rs surface_error_streak_latches_fatal_without_device_lost_callback` |
| Immediate fatal halt when device is lost regardless of surface error streak | `src/render_manager.rs classify_device_lost_is_fatal_immediately_regardless_of_streak` |
| `sync_main_camera` sanitizes non-finite or negative zoom to 1.0 and deliberately excludes rotation | `src/render_manager.rs sync_main_camera_copies_position_and_sanitized_zoom_only` |
| Multi-tab concurrent writes to save slots must merge changes on load rather than clobbering | `src/scores.rs test_concurrent_stores_merge_instead_of_clobbering` |
| Achievement unlocks merge across concurrent managers and atomic saves leave no temp files | `src/achievements/tests.rs concurrent_managers_merge_unlocks_instead_of_clobbering` |
| Atomic save write-then-read leaves no temp files on disk | `src/save_store/mod.rs test_write_then_read_round_trips_and_leaves_no_temp_file` |
| Sidecar validation must run before texture allocation so invalid sheet files leave no texture handle | `src/assets/sprite_sheet.rs prepare_sheet_fails_before_any_texture_is_loaded` |
| Sidecar path replaces file extension with `.sheet.ron` rather than appending | `src/sheet_file.rs sidecar_path_replaces_the_extension_never_appends` |
| Frame indices past grid bounds in `.sheet.ron` must be rejected naming the invalid clip | `src/sheet_file.rs frame_index_past_the_grid_is_rejected_naming_the_clip` |
| Sheet grid derivation from PNG dimensions truncates partial trailing cells | `src/sheet_file.rs into_parts_derives_the_grid_from_png_dimensions_excluding_a_partial_trailing_cell` |
| Solid color texture paths round-trip through `#solid:RRGGBB[AA]` syntax | `src/texture_ref.rs test_solid_color_path_round_trips_through_parse` |
| UI button clicks emit on the event bus on release after the next frame's flush | `src/ui_element_system.rs button_click_returns_press_event_on_release` |
| Resting cursor over pause/menu panels must not hover or fight keyboard navigation | `src/menu_panel.rs resting_cursor_does_not_hover_but_still_clicks` |
| Engine time scale is zero only while paused so particles and animations freeze | `src/pause.rs time_scale_is_zero_only_while_paused` |
| A startup card must hold the game — no `init`, `update`, key handler or context — until the frame after its handoff, and the press that ends the cards must never reach the title | `tests/startup_splash.rs a_press_after_a_cards_fade_in_skips_it_and_the_title_never_sees_the_press` |
| A despawned entity's song source is ended by the frame tail, after the frame's triggers: a script that triggers and despawns in one hook plays the note and has it released, with no call from the game | `tests/song_triggers.rs a_script_that_triggers_and_despawns_its_entity_plays_the_note_and_leaves_no_source` `tests/song_triggers.rs an_entity_destroyed_from_rust_has_its_source_ended_by_the_next_tail` |
| A world replaced wholesale is an explicit flush, not a scan — ids are revived by a snapshot restore and resources survive `World::clear` — and the frame's queued triggers are dropped; the flush epoch sits above every epoch minted before it, and the game and the menu re-mint once | `tests/song_triggers.rs a_world_replacement_flushes_every_source_and_the_next_trigger_activates_anew` `tests/song_triggers.rs a_trigger_queued_in_the_frame_of_a_world_replacement_never_sounds` `src/sound_sources.rs a_flush_raises_the_floor_above_this_frames_epoch_and_the_fixed_sources_re_mint_once` |
| A recorded sound that fails to load is warned once and remembered for the session, never re-read every frame; `set_base_path` forgets it | `src/assets/sound_library.rs a_failed_path_is_remembered_until_the_asset_base_changes` `tests/sound_emitters.rs a_missing_file_fails_once_and_never_panics` |
| The emitter pass runs after the transform hierarchy, so a child emitter is heard from this frame's `GlobalTransform2D`; first sight is recorded only while `time_scale` is live — the editor freezes the engine outside Play that way, so Editing records no first sight and a scene's spawn sounds fire on Play's first frame — while the requests, the re-mix and the eviction run every frame, so a loop whose entity goes while paused still stops | `tests/sound_emitters.rs a_frozen_engine_starts_nothing_and_the_spawn_sound_fires_on_the_first_live_frame` `tests/sound_emitters.rs a_loop_whose_entity_goes_while_the_engine_is_frozen_is_stopped_in_that_tail` |
| A one-shot released by a newer play stays owned until it ends, so a world replacement stops it too; a scripted stop in a binding emitter's spawn frame wins over `play_on_spawn`; a `SoundEmitter` removed and added again spawns again; a `play` from Rust in the update that despawns the entity still sounds | `tests/sound_emitters.rs a_world_replacement_stops_a_released_one_shot_too` `tests/sound_emitters.rs a_stop_in_a_binding_emitters_spawn_frame_wins_over_play_on_spawn` `tests/sound_emitters.rs re_adding_a_play_on_spawn_emitter_fires_again` `tests/sound_emitters.rs a_one_shot_played_from_rust_in_the_update_that_despawns_its_entity_still_sounds` |
| A script's `play_sound` and `stop_sound` share the triggers' one queue, so `stop_sound(me); trigger(me, …)` keeps its order; a one-shot played on an entity its script despawns that frame still sounds where it was, a loop is refused | `tests/sound_emitters.rs stop_then_trigger_on_a_binding_emitter_leaves_the_new_activation_and_the_reverse_ends_it` `tests/sound_emitters.rs a_one_shot_played_by_a_script_that_despawns_its_entity_still_sounds_and_a_loop_is_refused` |
| A looping emitter whose entity is gone, or whose component was removed, is stopped in that frame's tail — never left playing untracked | `tests/sound_emitters.rs a_looping_emitters_entity_destroyed_in_update_is_stopped_in_that_frames_tail` `tests/sound_emitters.rs removing_a_looping_emitters_component_stops_the_loop_and_leaves_the_entity` |
| A match start decides by the song in the slot, never by a flag: an empty slot is requested into, the game's own song is sought to the top, another playing song — the editor's audition — is left alone, and one that played to its end is replaced | `tests/game_music.rs a_match_start_requests_the_song_into_an_empty_slot_and_takes_its_own_back_to_the_top` `tests/game_music.rs a_match_start_leaves_another_song_alone_unless_it_played_to_its_end` |
| The pause menu's opening frame jams nothing — a move pressed with the key that opened it included — and neither does a reopen on the frame after a resume or the first frame after a Restart | `tests/game_music.rs the_pause_menus_opening_frame_jams_nothing_and_each_move_after_jams_one_unit` `tests/game_music.rs a_restart_clears_the_pause_latch_so_the_next_opening_frame_jams_nothing` |
| A song change flushes nothing: a live entity keeps its `SourceId` and activates lazily in the next song | `tests/song_triggers.rs a_song_change_keeps_each_live_source_and_the_new_song_admits_it` |
| Delta time is clamped after an engine stall to prevent physics explosions | `src/game_loop_manager.rs test_delta_time_is_clamped_after_a_stall` |
| Pickups are collected exactly once on contact start and then destroyed | `src/pickups.rs test_each_pickup_is_collected_once_on_a_started_contact_and_destroyed` |


## Save/Load Pipeline
- Editor calls `world_to_scene_data(world, name, physics, texture_path_fn)` from `scene_serializer.rs`
- Texture handle → path resolved via `AssetManager.handle_to_path` (populated by `load_texture()`)
- Inverse path: `SceneLoader::load_and_instantiate(path, world, assets)` from `scene_loader.rs`
- Loader attaches a `Name` component for named entities (in addition to `SceneInstance.named_entities`), so names survive an editor load→save round-trip

## Rendering note (Aug 2026, issue #26)
UI draws in its own **post-tonemap pass**: game sprites render into the HDR
target and get Reinhard-tonemapped by the bloom composite; UI batches (always
separate from game batches in `game/render.rs`) then draw straight to the
swapchain via `RenderManager.ui_pipeline` — authored UI colors display as
exactly that byte (white text = 255, not 188). `render()` takes game and UI
batch refs separately. Also: `GameContext.window_title` writeback retitles the
OS window in the frame tail (`WindowManager::set_title`, headless no-op, title
stored pre-creation).

## Testing
- `cargo test -p engine_core` — 0 failed, 0 ignored (GPU/window-bound doc examples are compile-only `no_run`)
- `cargo test -p engine_core --features test-support --test headless_game` — the harness's own contract: `init` once, art loads headlessly, the engine's tail runs, input edges per frame, title and exit read back.
- `cargo test -p engine_core --features test-support --test sound_emitters` — scene-authored sound: a `SoundEmitter` played on first sight and on request, re-mixed from the camera, ended with its instance, its entity or its component, scripted plays and stops in order with the triggers, the freeze, a world replacement, the load-time refusals.
- `cargo test -p engine_core --test game_music` — a game's song through `GameMusic`: the identity rule's four arms, the open-frame jam latch, the bar-first pulse, a song that will not load.
- `cargo test -p engine_core --features test-support --test song_triggers` — the host's side of a song: the asset base, scripted triggers in order, the reconcile, the flush, a song change, a storm. The song runs on the disabled manager's virtual clock, so the tests sleep a few milliseconds between frames for the conductor to drain.

## Godot Oracle
- Game loop: `main/main.cpp` — `iteration()` method
- Scene loading: `scene/resources/packed_scene.cpp`
- Asset management: `core/io/resource_loader.cpp`
