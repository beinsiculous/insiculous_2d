# Engine Core Crate — Agent Context

Core engine: Game trait, run_game(), managers, scene loading/saving, asset management.

## Key Types
- `Game` trait — `init()`, `update()`, `on_key_pressed()` — the public API for games
- `GameConfig` — window title, size, clear color, **`chaos_mode`**, **`texture_filter`** (default sampling for loaded textures; `TextureFilter::Nearest` for pixel art)
- `run_game(game, config)` — entry point, creates window + event loop
- `GameContext` — passed to Game methods: world, input, **players** (per-player
  `InputSettings`: `ctx.players.is_active(PlayerId::P1, GameAction::Action1, ctx.input)`,
  `move_x/move_y`), assets, ui, physics, delta_time, **chaos_mode**, **time_scale**
  (read-write; scales engine-side particle stepping only — set 0.0 while paused),
  `request_exit()` (clean engine shutdown, same path as window close)
- `ChaosMode` — cross-game Normal/Insane/Ridiculous/Insiculous theme (engine carries the selection, games define the meaning)
- Managers: `GameLoopManager`, `RenderManager`, `WindowManager`

## Crate Boundaries
Cross-cutting glue biases toward `engine_core`: `ui` defines `DrawCommand` (renderer-agnostic), `renderer` defines `Sprite` (UI-agnostic), and `engine_core` owns the bridge in `ui_integration`. This keeps `renderer` and `ui` independently testable and prevents either crate from depending on the other transitively through `engine_core`. The dual glyph cache is intentional: `ui` caches rasterized bitmaps (`font/glyph_cache.rs`) to avoid re-rasterization, while `engine_core` caches GPU textures (`glyph_texture_cache.rs`) to avoid re-uploads.

## File Map
- `game.rs` — Game trait, `run_game()`, and `GameRunner` orchestration; new render passes go in their own module like `tilemap_render.rs`.
- `game/app_handler.rs` — winit `ApplicationHandler`: native frame driving uses `about_to_wait` while wasm uses `RedrawRequested`; never unify them (an occluded native window stops receiving redraws).
- `game/web.rs` (wasm-only) — async renderer bring-up (adopted surface starts 1×1) and gesture-gated audio enable (retries capped at 5 failures, hooked pre-match so audio is live before `on_key_pressed`).
- `web/mod.rs` (wasm-only) — `preload_assets` fetches manifest and entries into `common::vfs` under `{base}/{entry}` keys before `run_game`.
- `game/frame_tail.rs` — post-update tail: particles and `ecs::SpriteAnimationSystem` step on `delta_time * time_scale` so pausing freezes both.
- `localization.rs` — `Strings`: RON locale tables with `current→en→key` fallback, per-locale font tracking, and `@key` resolution.
- `ui_element_system.rs` — draws `UiLabel`/`UiPanel`/`UiButton` and buffers `UiButtonPressed` on the event bus after the next frame's flush; suppressed by `UiElementsHidden`.
- `gamepad_backend.rs` — gilrs hardware poll (0.15 dead zone rescale, stick +Y = up, pumped before `process_queued_events`).
- `input_settings_io.rs` — JSON persistence for player bindings with missing/corrupt fallback to defaults.
- `save_store/` — player save persistence seam (filesystem path natively, localStorage on wasm with fallback to `MemoryStore`; multi-tab merge-on-save via `JsonSaveSlot`).
- `scores.rs` — high scores (top-10 per mode) with merge-on-save for multi-tab safety, while `reset()` overwrites.
- `glyph_texture_cache.rs` — GPU glyph texture cache (dual cache with ui crate's rasterized bitmap cache).
- `render_manager.rs` — `sync_main_camera` copies pose (position and zoom, rotation excluded) onto render camera; device loss fail-stop stops frame loop after `MAX_SURFACE_ERROR_STREAK = 10` or `DeviceLost`.
- `tilemap_render.rs` — expands `Tilemap` and `Transform2D` into the game sprite batcher (one batch per tileset).
- `scene_loader.rs` — RON to World deserialization; `SceneInstance` retains prefabs for runtime `spawn_prefab`.
- `scene_serializer.rs` — World to `SceneData` serialization (inverse of `scene_loader`, requires loader match arm and serializer table row for new components).
- `scene_data.rs` — `SceneData` schema; Sprite `emissive`/`tex_region`/`visible` require named serde defaults to avoid blanking old sprites.
- `script_data.rs` — script wire schema: entity params persist by name, auto-named at save and resolved post-instantiate.
- `texture_ref.rs` — scene texture reference resolution (`#white`, `#solid:RRGGBB`, file paths); `TextureResolver` trait is the GPU/filesystem seam.
- `sheet_file.rs` — `.sheet.ron` schema and validation (`parse_sheet_file` and `into_parts`); `sidecar_path_for` holds the naming rule.
- `assets/sprite_sheet.rs` — `AssetManager::load_sprite_sheet` order: read sidecar, parse, probe PNG dims, validate, then load texture so bad sheets leave no handle.
- `assets.rs` — asset loading; `create_texture_from_rgba` validates before device with `"#rgba"` sentinel (does not survive save/load).
- `behavior_runner/` — entity behavior system: runner dispatch loop, handlers, and `CameraFollow` with look-ahead.
- `pause.rs` — shared `PauseMenu`; headless-testable via `&InputSettings + &InputHandler + window_size` (mouse reads live inside paused branch only).
- `menu_panel.rs` — `MenuPanel` chrome; resting cursor never fights keyboard nav, click selects and confirms.
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

## Godot Oracle
- Game loop: `main/main.cpp` — `iteration()` method
- Scene loading: `scene/resources/packed_scene.cpp`
- Asset management: `core/io/resource_loader.cpp`
