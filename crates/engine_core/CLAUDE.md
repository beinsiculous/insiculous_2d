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
  **exit_requested** (write true → clean engine shutdown, same path as window close)
- `ChaosMode` — cross-game Normal/Insane/Ridiculous/Insiculous theme (engine carries the selection, games define the meaning)
- Managers: `GameLoopManager`, `UIManager`, `RenderManager`, `WindowManager`, `SceneManager`

## File Map
- `game.rs` — Game trait, run_game(), GameRunner orchestration (~530 lines; the render
  tail lives in the child module `game/render.rs` — new render passes go in their own
  module like `tilemap_render.rs`)
- `game/app_handler.rs` — the winit `ApplicationHandler` impl + `shutdown` +
  `drive_frame` (split out of game.rs, Aug 2026). **Frame driving is
  cfg-split**: native = `about_to_wait` (unchanged behavior, throttle+redraw);
  wasm = the `RedrawRequested` arm (maps to requestAnimationFrame). Never
  unify them — an occluded/minimized native window stops receiving redraws on
  some compositors (adversarial finding F2)
- `game/web.rs` (wasm-only) — async renderer bring-up: `spawn_renderer_init`
  (spawn_local fills `pending_renderer`), `drain_pending_renderer` (adopts via
  `complete_init` + `finish_renderer_setup`, pushes the canvas's pixel size
  through resize — the adopted surface starts 1×1). Also H7 gesture-gated
  audio: `upgrade_audio_on_gesture` calls `AudioManager::enable_output` on
  the first activation gesture (keydown/mousedown/touchstart, auto-repeat
  excluded), retries capped at 5 failures; hooked pre-match in
  `app_handler::window_event` so audio is live before `on_key_pressed`
- `web/mod.rs` (wasm-only, `engine_core::web`) — the web boot phase:
  `preload_assets(base)` fetches `{base}/manifest.json` + every entry into
  `common::vfs` under `{base}/{entry}` keys BEFORE `run_game`;
  `init_web_logging()`, `set_boot_status()` (writes `#game-loading`).
  Game wasm entries call these from `#[wasm_bindgen(start)]`
- `game/render.rs` — GameRunner's frame-render tail (`render_frame`, batch-ref sorting,
  particle append); child module of `game` so no field visibility changes were needed
- `game/frame_tail.rs` — GameRunner's post-update tail (particles, **sprite animations**,
  lines, UI-element pass, toasts, base-font capture); both particles and
  `ecs::SpriteAnimationSystem` step on `delta_time * time_scale`, which is what makes a
  paused game freeze both; `game/locale_font.rs` — locale-font application
- `localization.rs` — `Strings`: RON locale tables (`assets/locales/*.ron`, `LocaleFile` v1
  with display_name/optional font/strings map), `tr()` with current→en→key fallback
  (log-once), `resolve()` for `@key` text, `cycle_locale`/`available_locales`/`locale_keys`,
  per-locale font tracking (`font_dirty`/`active_font`). Exposed as `ctx.strings`;
  `GameConfig.locale` + `locales_dir` configure it
- `ui_element_system.rs` — draws ecs `UiLabel`/`UiPanel`/`UiButton` each frame (panels →
  buttons → labels, anchor-placed, `@key` localized); returns `UiButtonPressed` presses that
  the runner buffers and emits on the event bus after the NEXT frame's flush (one-frame
  latency — the bus flushes before update). `UiElementsHidden` world resource suppresses
  the pass (editor inserts it while Editing)
- `gamepad_backend.rs` — gilrs hardware poll (`GamepadBackend::new_or_disabled()`,
  `pump()` drained right before `process_queued_events()`); pure translation fns
  (button/axis tables, 0.15 dead-zone rescale, hat-switch dpad synthesis on ±0.5
  crossings). gilrs stick +Y = up; needs `libudev-dev` on Linux at build time
- `input_settings_io.rs` — JSON load/save for player input bindings (versioned
  Vec-of-entries DTO; missing file → defaults written for hand-editing; corrupt/wrong
  version → warn + defaults, never panics). Wired to `GameConfig::input_settings_path`
  (load at startup, save on change via `InputSettings` dirty tracking — polled each
  `drive_frame`, failed saves retried with a once-per-streak warn — plus the
  CloseRequested save). Routes through `save_store`
- `save_store.rs` — **the player-save persistence seam** (roadmap H6, #6/#17):
  `read(slot)`/`write(slot, contents)` where a slot is a filesystem path natively
  (atomic tmp+rename, parents created) and a localStorage key on wasm (each persist
  dispatches the `insiculous-save` CustomEvent; storage-blocked browsers degrade to a
  session-local `MemoryStore` with one warn). Contract with the website:
  `docs/WEB_SAVES.md` — keys `beinsiculous.games.<slug>.{achievements,scores,input}`,
  values byte-identical to the native JSON save files
- `scores.rs` — `Scores` high-score lists (`ctx.scores`): top-10 per game-defined mode
  string, `submit(mode, score) -> bool` write-through persist, `best`/`top`; wired to
  `GameConfig::score_save_path`. Achievements + scores both merge-on-save (multi-tab
  web safety); their `reset()` overwrites (an explicit clear must clear)
- `glyph_texture_cache.rs` — GlyphTextureCache: UI glyph bitmap → GPU texture cache (extracted from GameRunner)
- `game_config.rs` — GameConfig struct (incl. `input_settings_path`)
- `game_loop_manager.rs` — Frame timing and delta
- `ui_manager.rs` — UI lifecycle and draw commands
- `render_manager.rs` — Renderer lifecycle; `sync_main_camera(world)` copies the main-camera entity's Transform2D position onto the render camera each frame (position only; no-op without a `Camera { is_main_camera: true }` entity). Device-loss fail-stop: `note_render_error` state machine (surface-error streak, `MAX_SURFACE_ERROR_STREAK = 10` → fatal; `DeviceLost` → fatal immediately), `is_fatal()` makes `render()` refuse GPU work; `GameRunner.render_fatal` + `app_handler::handle_render_fatal` stop the frame loop (web: "reload the page" boot status, rAF simply not re-armed, key dispatch gated; native: clean shutdown) — never submit to a dead queue (the Firefox parent-process WebGPU crash, Aug 2026)
- `tilemap_render.rs` — expands `Tilemap` + `Transform2D` entities into the game sprite batcher (called at the top of the default `Game::render`; one batch per tileset)
- `window_manager.rs` — Window creation
- `scene.rs` — Scene lifecycle / world coordination
- `scene_manager.rs` — Scene loading and entity instantiation
- `scene_loader.rs` — RON → World deserialization (`ComponentData` construction lives in `scene_loader_components.rs`); `SceneInstance` retains the prefab table and offers runtime `spawn_prefab(world, assets, name, overrides)` (Prototype pattern, override semantics; failed spawns leave no debris)
- `scene_serializer.rs` — World → SceneData (inverse of scene_loader, used by editor save; tests in `scene_serializer_tests.rs`). NEW COMPONENT TYPES need arms in BOTH scene_loader_components.rs and scene_serializer.rs
- `scene_data.rs` — SceneData / PrefabData / EntityData structs (schema incl. `ComponentData::EntityTag`, Sprite `emissive`/`tex_region`/`visible` — the latter two with NAMED serde defaults (full region / true); a plain `#[serde(default)]` would render nothing / hide every old sprite)
- `behavior_data.rs` — `BehaviorData` + the `Behavior`↔`BehaviorData` From impl pair (re-exported via `scene_data`)
- `texture_ref.rs` — scene texture reference resolution (`#white`, `#solid:RRGGBB`, file paths); `solid_color_path(color)` is the canonical `#solid:` writer (inverse of `parse_hex_color`, alpha byte only when translucent — what `create_solid_color` records so solids survive save/load); `TextureResolver` trait is the GPU + filesystem seam (AssetManager = production impl, tests stub it). Beyond `resolve_texture` it carries `sheet_for()` (a PNG's `.sheet.ron` → `SheetData` of grid + clips) and `clear_sidecar_cache()`, so the scene loader can re-resolve animations against their sidecar while staying headless-testable. File-path resolution consults the sidecar's `filter`, which is how scene-referenced pixel-art sheets get Nearest with no per-game code
- `sheet_file.rs` — **THE `.sheet.ron` schema** (`SheetFile` v1: `version`, pixel `cell`, `filter` defaulting to Nearest, `clips`). `parse_sheet_file` validates version/cell/fps/frames; `into_parts(png_w, png_h)` derives the `SheetGrid` and rejects frame indices past the last cell. `sidecar_path_for` is the one place the stem + `.sheet.ron` rule lives
- `texture_filter_serde.rs` — shared `TextureFilter` serde bridge (the renderer crate stays serde-free); used by BOTH `GameConfig.texture_filter` and `SheetFile.filter`
- `assets/sprite_sheet.rs` — `AssetManager::load_sprite_sheet(png)` → `SpriteSheet { texture, grid, clips, path }` with `.animation()` / `.sprite()` conveniences. Order is read sidecar → parse → probe PNG dims → validate → **then** load the texture, so a bad sheet leaves no handle behind. `SidecarCache` backs the implicit path (warn-and-fall-back, one read per path per scene load, cleared at the top of `SceneLoader::instantiate`); the explicit API stays fail-loud
- `assets.rs` — Asset loading (textures, fonts); tracks `handle_to_path` for save; `AssetConfig.default_filter` (from `GameConfig::with_texture_filter` via `impl From<&GameConfig>` — the one place `game.rs` builds it) applies to `load_texture`/`load_texture_from_bytes`, per-call override with `load_texture_filtered`, while `create_solid_color`/`create_checkerboard`/`create_glyph_texture` stay Linear; `create_solid_color` records the reconstructible `#solid:RRGGBB` path (E5 — reload rebuilds as 1×1); `create_texture_from_rgba` (raw RGBA8 → always-nearest texture for tileset strips; validates before device, `"#rgba"` sentinel path — does NOT survive save/load); `game_root_from()` + the `game_root!()` macro (asset/save anchoring — macro so the game crate's manifest dir is baked in)
- `behavior_runner/` — Entity behavior system: `mod.rs` (runner, dispatch loop, command
  application), `handlers.rs` (player/AI/collectible handlers), `camera.rs` (`CameraFollow`
  incl. input-driven look-ahead)
- `lifecycle.rs` — FSM for scene lifecycle
- `timing.rs` — Timer utilities
- `contexts.rs` — GameContext, RenderContext (incl. `viewport_scissor` writeback: an editor-style host bounds the game-world passes to a sub-rect in physical pixels; plain games leave it `None`)
- `chaos_mode.rs` — `ChaosMode` enum + helpers (`ALL`, `is_insane`, `is_ridiculous`, `label`)
- `chaos_theme.rs` — `ChaosTheme` per-mode presentation tokens (bg/structure/accent/grid colors, banner, particle mult); engine owns structure + default palette, games override via struct-update syntax
- `pause.rs` — `PauseMenu`/`PauseAction`/`PauseMenuLabels`: shared pause mechanism (Menu/Esc/Start
  toggles, Resume/Restart/Quit-to-Title/Exit-Game items — localizable via `draw_labeled`;
  games map actions onto their
  own start_game/reset_to_title/`ctx.exit_requested` and skip their whole gameplay
  update while active;
  `time_scale()` feeds `ctx.time_scale` so engine particles freeze too). Takes
  `&InputSettings + &InputHandler + window_size: Vec2` (NOT GameContext) so it's
  headless-testable; `window_size` locates the panel for mouse hit-testing
  (hover moves the highlight, click executes a row) — mouse reads live inside
  the paused branch only, so gameplay never sees the clicks
- `menu_panel.rs` — `MenuPanel`/`MenuStyle`: shared menu window chrome (opaque
  themed panel, border, accent separator + corner ticks, ▶-cursor highlight
  rows, hint footer, input-blocking overlay variant). Flair is rect-based;
  the ▶ cursor is verified in the games' shared font.ttf. Rows are
  mouse-clickable (Aug 2026): `row_rect`/`row_at` are the pure hit-test
  geometry, `mouse_select(input) -> MenuMouse` reads hover + left-click from
  `InputHandler` (headless). Convention: hover moves the shared selection
  (only on frames the mouse moved — a resting cursor never fights keyboard
  nav), click = select + confirm that row
- `menu_input.rs` — `MenuInput` shared menu-screen input (W/S+arrows up/down,
  Space/Enter/NumpadEnter confirm, Esc back — plus EVERY connected gamepad: dpad/left-stick
  edge up/down, A/Start confirm, B back) + wraparound `navigate`; used by every game's
  title/select screens
- `spawn_helpers.rs` — shared entity recipes (`spawn_background` full-window backdrop); `RENDER_UNIT = 80.0` (pixels per world unit) lives at the crate root and is used by the render path in `game.rs`
- `pickups.rs` — generic pickup/collectible tracking (`Pickups<K>` keyed by a game-defined kind, `EffectTimer` for timed effects); collection = started-collision events vs a collector set, once per pickup. Used by BOTH Pong (floating power-ups, balls collect) and Breakout (falling drops, paddle collects) — engine owns the mechanism, games own the meaning
- `ui_integration/{mod,tests}.rs` — UI-to-renderer bridge. **Camera-relative**: UI sprites are positioned/scaled against the render camera so UI stays at fixed screen pixels when the camera moves/zooms (camera-follow games, editor). Emits SDF shapes: rounded rects, single-sprite borders, true circles, and `DrawCommand::Image` textured quads. Clip rects are real since #41: `PushClipRect`/`PopClipRect` drive `SpriteBatcher::set_clip`, so clipped commands land in clip-tagged batches the GPU scissors (the CPU fully-outside cull stays as a perf win). UI coords are already physical pixels — never DPI-convert a clip rect
- `prelude.rs` — Re-exports for `use engine_core::prelude::*`

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
- 381 passing (incl. doc tests; GPU/window-bound ones compile-only `no_run`), 0 ignored — `cargo test -p engine_core`

## Godot Oracle
- Game loop: `main/main.cpp` — `iteration()` method
- Scene loading: `scene/resources/packed_scene.cpp`
- Asset management: `core/io/resource_loader.cpp`
