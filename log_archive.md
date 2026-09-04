# Log Archive — Completed Work

**Convention (updated Aug 28 2026):** open work lives on the **org Studio Board**
(https://github.com/orgs/beinsiculous/projects/1; technical debt = the
`tech-debt` issue label). `PROJECT_ROADMAP.md` carries only vision + settled
decisions, and the per-crate `TECH_DEBT.md` files are retired. Everything
completed or resolved moves HERE, organized by area, so history is never lost
but never clutters the working docs. When you resolve a tracked item: close its
issue (commit message "fixes …#N"), and if the resolution carries reusable
lessons, append them here with the date. (The pre-Aug-2026 convention — live
TECH_DEBT/ROADMAP docs — is what the sections below refer to.)

---

## Docs retirement — TECH_DEBT.md files + roadmap task tracking → issues (Aug 28 2026)

All 13 `TECH_DEBT.md` files (root, `crates/*`, `../games`) were deleted and
`PROJECT_ROADMAP.md` was slimmed to vision/decisions; every still-valid open
item moved to the Studio Board:
- **Roadmap tasks** → #67 (E8), #68–#71 (F2–F5), #72–#79 (G0–G7 re-skins),
  #80 (I0 purge gate). Already tracked before the migration: #10 (E7), #11
  (E5), #13, #15, #16, #46, #48, #49, #60–#65.
- **Debt Mediums** → #81 (BUILD-001 my_platformer broken), #82 (DRY-002 volume
  clamp), #83 (KISS-001 syn features), beinsiculous/breakout#2 (GPP-11 shadow
  bricks).
- **Debt Lows** → per-area rollups #84 (engine_core), #85 (physics), #86
  (ecs), #87 (input), #88 (ui), #89 (renderer), #90 (editor_integration),
  #91 (common), #92 (ecs_macros), #93 (games).
- **Verified already-resolved during migration, dropped without issues:**
  games GPP-12 (`BrickSpec` + `parse_brick_tag` shipped) and GPP-L11
  (breakout `gameplay/` split done); engine_core DRY-010 (single
  `merge_components` helper) and SIZE-001 (all named files under 600 lines);
  common ARCH-001/ARCH-002 (renderer re-exports `common::CameraUniform` /
  `Camera` — no duplicates) and the KISS-001 strikethrough leftover; input
  GAP-001 (gilrs backend shipped Jul 2026); the roadmap's ARCH-006 copy
  (registry collapse closed by Sprint 5 #43). `editor` and `audio` had zero
  open items.
- Non-task content moved to crate CLAUDE.mds (audio by-design limitations,
  renderer deferred-by-design, ecs GPP-02 decision-of-record).

## Roadmap slimmed Aug 28 2026 — archived completed-phase detail

The following completed-work records moved here verbatim-ish from
`PROJECT_ROADMAP.md` when it was slimmed. (Phases A/B/game-6 were already
archived below.)

### Phase E — asset pipeline (core ☑ Jul 30 2026, adversarially reviewed)
- **E1** `TextureFilter` knob: config default (`GameConfig::with_texture_filter`
  → `AssetConfig.default_filter`) + per-call `load_texture_filtered`; Linear
  default for plain loads (back-compat).
- **E2** `common::SheetGrid`: Tilemap delegates (behavior-identical,
  test-locked); `uv_rect_checked` for E3/E4 consumers; `Deserialize` needs
  explicit `cell_uv` on the wire.
- **E3** `SpriteAnimation` rework (plan-v4, 3 adversarial rounds): named clips
  over `SheetGrid` (`play`/`ensure_playing`/`resume`, fps/empty-clip guards);
  `SpriteAnimationSystem` driven from `frame_tail.rs` on the time-scaled delta;
  scene chain uses `GridData` + shared `ClipData` DTOs, `autoplay` written only
  while playing; sidecar-as-SSOT on reload via `TextureResolver::sheet_for`;
  editor freezes engine time outside Play.
- **E4** `load_sprite_sheet()` + `.sheet.ron`: `SheetFile` v1 in
  `sheet_file.rs` (pixel `cell`, filter defaults Nearest, looping defaults
  true, fail-loud validation); validate-before-GPU ordering (no handle leak);
  `SidecarCache`; scene texture refs take the sidecar's filter automatically.
- **E5 (round-trip half)**: `create_solid_color` records canonical
  `#solid:RRGGBB`; `tex_region` + `visible` on the Sprite wire with named serde
  defaults. Remainder (the `#rgba` save-time error) = issue #11, gated on #69.
- **E6** deleted `renderer/src/atlas.rs`; **E9** docs.
- **Checkpoint: E2 + E4 merged = SCHEMA FREEZE, Jul 30 2026.**

### Phase F provenance — pixellab lessons (paid for once, Aug 2 2026)
37/40 trial generations used; full cast has restyled side-view baselines +
8-frame walk sheets (deion_assets commit e2ea5be; quarantined `ai/`,
`ai_<name>_64_side.png` — Deion, Cubert, Bananakin, Captham Michael, Master
Pi, Aleister Prunely, Funguy, Dr. Maxwell).
- pixellab `create_character` forces a humanoid/quadruped SKELETON — produced 5
  little humans, all rejected. For geometric shape-with-a-face casts use
  **`create_image_pixflux`** (freeform, `view: side`, `no_background`) —
  on-style first try.
- **img2img style transfer** (`init_image`, strength 160) re-renders a concept
  into Jesse's flat hand-drawn style keeping identity (deion_assets 0820e4f →
  09bd21d hold before/after); prompts went through the deion_assets prompt-mode
  adversarial review.
- **`animate_image` on hand-drawn PNGs** (loose PNG, no rig) preserves style —
  but mid-sequence frames drift off-model; workflow is generate 8, curate 4–6,
  hand-fix stragglers in Aseprite. `seed` allows re-rolls.
(The open follow-ups — curation, sidecars, top-down variants — live in #70.)

### Phase H — WASM port ☑ COMPLETE Aug 27 2026
H1 spike PASSED Jul 30 2026 (`coordination/H1_SPIKE.md`): 14/14 deps compile
for wasm32; **audio decision:
STAY ON RODIO — FINAL** (listen test passed by Jesse). Shipped waves: **H2**
`common::clock` (web-time) · **H3** cfg-split frame loop (native
`about_to_wait` byte-identical; wasm rAF via `RedrawRequested` — never unify,
occluded native windows stop redraws) · **H4** async renderer init
(`spawn_local` + `pending_renderer`; 0×0 surface clamp) · **H5** `common::vfs`
+ `engine_core::web::preload_assets` manifest fetch (include_bytes bootstrap
WON'T-DO, #9) · **H6** `save_store` seam + `Scores` (#6/#17,
`docs/WEB_SAVES.md`) · **H7** gesture-gated audio `enable_output()` (#8) ·
**H8** wasm CI guard `scripts/check_wasm.sh` + workflow (#7) · **H9** all 6
games ported same-day via 5 parallel agents (pong 2.5 MiB … asteroids 2.1 MiB,
every game browser-verified on WebGPU).
Lessons paid for during the pong port:
- **winit never inserts its canvas into the DOM** — detached canvas renders
  silently into nothing. Fix lives in `renderer::insert_canvas_into_dom`
  (called from `WindowManager::create`); `with_canvas` adoption was tried and
  abandoned. Don't re-litigate.
- WebGPU canvases expose **no sRGB surface formats**; the bloom composite
  gamma-encodes (`inv_gamma`) when the swapchain isn't sRGB.
- Never configure a surface at 0×0 (validation error) — clamp to 1×1, push the
  real size after adoption.
- Headless/swiftshader Chromium composites nothing visible — use Playwright's
  **full Chromium, headed** for pixel verification; headless Firefox has no
  `navigator.gpu`.
- Pong's whole bundle is 2.5 MiB wasm — Cloudflare's 25 MiB budget is a
  non-issue; `wasm-opt` uninstalled and unneeded.

### Phase I — I1/I2 ☑ LIVE Aug 19 2026
Pong live at beinsiculous.com/games/pong/ (v1 immutable-once-live convention;
launch-window hotfix: 1×1-canvas resize-observer deadlock — one-time same-v1
exception). Then all 6 games live same-day (`npm run verify` green, 43 pages;
frogger 720×768 frontmatter pass-through added; first devlog post in the same
deploy). Handoff: repo transferred to `milyramic`; deploys via GitHub Actions
(`wrangler deploy`, dashboard Workers Builds deliberately disconnected);
production URL beinsiculous.com. Remaining manual step (tracked in memory, not
an issue): the beinsiculous.com zone must exist in the target Cloudflare
account with the registrar pointing at Cloudflare's nameservers.

### Deion Pivot provenance
The pivot plan was adversarially reviewed Jul 28 2026 (kimi, 15 findings — 13
accepted, 2 rebutted) and the capability-table status detail that used to open
the roadmap is superseded by `CLAUDE.md`'s live status section.

## GPP-L7 — gizmo drags vs. commands (RESOLVED Aug 28 2026, editor Sprint 4)
The last open GPP audit low. The documented hazard — mid-drag world mutations
live outside the command system, so Ctrl+Z during a drag could undo against
half-applied state — is now structurally prevented: `dispatch_editor_action`
suppresses Undo/Redo (and every transform/existence-mutating action) while a
gizmo drag is live, drags apply idempotently from captured start state
(`GizmoDragState`), commit as ONE command on release, and Escape rolls the
whole gesture back without touching history. Live-feedback mutation between
commands remains by design; it just can't interleave with history anymore.

## Game Programming Patterns Audit (July 13, 2026) — CLOSED
A full-codebase audit against Robert Nystrom's *Game Programming Patterns* catalog ran on Jul 13 2026 (originally `PATTERNS_AUDIT.md`, retired the same day once its open items were re-homed in the live TECH_DEBT docs). Final score: **15 of 17 numbered findings resolved same-day** (GPP-01 behavior FSM, GPP-04 dirty-flag transforms, GPP-07 runtime prefabs, GPP-08/09/10 physics event+edit APIs, GPP-13 registry inspector, GPP-14 stable undo ids, GPP-15 upload gating, GPP-17 + 7 of 12 lows, GPP-03 part 1 promotions) — per-crate resolutions below. Still open, tracked in live docs: GPP-02 (data-locality decision of record + revisit trigger — ecs), GPP-03 part 2 / GPP-11 / GPP-12 (games), GPP-06/GPP-16 (behavior/registry extensibility — parked with Phase 4 scripting, engine_core ARCH-006 + ecs), GPP-05/L2/L7/L10/L12 (lows, per-crate). Standing verdict: Command/Event Queue/Component/Object Pool/Flyweight/Service Locator were textbook already; Spatial Partition correctly delegated to rapier.

---

## Completed Games (20 Games Challenge)

### Game 1: Pong ☑ (June 2026)
Two paddles, one ball, score display. The "Hello, World" of games. Lives in `../games/pong/`.

- **Taught:** Physics bounce, score tracking, UI overlay, simple AI opponent
- **Key components:** `RigidBody` (kinematic paddles, dynamic ball), `Collider`, `Sprite`, immediate-mode score UI
- **Controls:** Player 1 W/S, Player 2 Up/Down. AI mode: right paddle tracks ball with lag. Win: first to 7.
- Full chaos-mode + achievements support; 5 tests, clippy-clean.

### Game 3: Space Invaders ☑ (July 13, 2026)
Grid of invaders marching left/right and descending; player fires upward. Lives in `../games/space_invaders/` — 25 tests, clippy-clean, full chaos-mode + achievements support (10, incl. sharpshooter streak), deforming grid background, particles, destructible barriers.

- **Taught:** Formation movement (one shared offset over kinematic bodies via `set_kinematic_target`), bullets as `Lifetime`-carrying dynamic sensors (engine auto-despawn), one `take_collision_events()` drain shared by every consumer, game-side AABB fallbacks for pairs rapier never reports (kinematic-vs-static barrier chomping, kinematic-vs-kinematic invader-vs-player, fast bullet-vs-bullet cancels)
- **Key components:** Player cannon (kinematic box), 5×10 invader fleet (kinematic, per-row colors/values), barrier bunkers (grids of static sensor blocks), bullets (dynamic sensors + `Lifetime`)
- **Controls:** Mouse or arrows/A/D; hold Space/click to fire (cooldown). Win: clear the fleet. Lose: cannon out of lives, or any invader reaches the invasion line / the cannon.
- **Chaos meanings (split Jul 13 2026, Jesse's tuning):** Insane = 1.8× march speed, player compensated with a 4-bullet cap (vs the classic 1); Ridiculous = 2.4× invader fire rate, player answers with twin cannons (one 2-bullet volley in flight); Insiculous = both fleet buffs, twin cannons with 3 stacked volleys (6-bullet cap). Single sources: `player_fire_caps(mode)` + `invader_fire_rate(mode)` in `gameplay/combat.rs`, `march_speed` in `gameplay/formation.rs`.
- **GPP-03 part 2 rule-of-three (closed with this game):** third literal duplications promoted to the engine — `default_playfield_grid` (was `build_grid` verbatim in pong+breakout), `MenuInput` (pong's struct == breakout's `nav_keys`/`menu_navigate`), `spawn_background`, and the `RENDER_UNIT = 80.0` constant (now defined once in `engine_core`, used by the renderer path in `game.rs`). Pong + breakout refactored onto all four. NOT promoted (no third use, stays game-side by the rule): `spawn_paddle`/`spawn_ball`/`spawn_wall` shapes (SI has no paddle-bounce ball or bouncy walls), the particle preset semantics (SI's bursts are its own), and the `Serving` flow skeleton (SI has no serve state).

### Game 2: Breakout ☑ (June 2026)
Ball bouncing off a paddle, destroying a grid of bricks. Lives in `../games/breakout/` — 43 tests, clippy-clean, full chaos-mode + achievements support, deforming grid background, particles, scene-driven levels (`level{1-4}.scene.ron`).

- **Taught:** Dynamic entity despawning via collision events (bricks), grid layout spawning, lives system (bottom sensor + escape safety net), offset-based paddle bounce control, mouse+keyboard on one input
- **Key components:** Ball (dynamic, CCD), paddle (kinematic `capsule_x`), brick grid (static, destroyed on hit)
- **Controls:** Mouse or arrow keys/A/D; Space/Enter/click to launch. Win: clear all bricks.
- **Engine gap found:** `MouseButton` wasn't re-exported in `engine_core::prelude` — added alongside `KeyCode`.
- **July 2026 level system:** chaos = per-level flavor; brick tags via `EntityTag` (`armored{N}` + `drop_*`); pickups fall and paddle catches; `engine_core::pickups` promoted as the shared mechanism.

### Game 4: Snake ☑ (July 13, 2026)
Growing snake in four directions, eat food to grow, avoid self. Lives in `../games/snake/` — 31 tests, clippy-clean, full chaos-mode + achievements support (9), deforming grid background, particles. **First game with NO physics at all** — segments/food/walls are sprite-only entities; every rule is grid-cell math in pure functions (`step_snake`, `next_direction`, `place_food`, `tick_interval`), fully headless-testable.

- **Taught:** Grid-based movement (cells `VecDeque<IVec2>` head-first as the single source of truth, index-parallel segment sprites resynced per tick), fixed-tick accumulator (`tick_timer += interval` catch-up loop — first game with a discrete step), buffered turn input (cap-2 queue, 180° reversals rejected at apply time so two buffered turns can never combine into one), deterministic food placement (`hash_u32(frame_count)` → linear probe to first free cell)
- **Key components:** `Name` + `Transform2D` + `Sprite` only. No `RigidBody`/`Collider`/`PhysicsSystem` anywhere.
- **Controls:** Arrows/WASD to turn; Esc to title. Survive and maximize length; eating within the vacating-tail rule (stepping into the cell the tail leaves this tick is legal — unless that step eats and the tail stays).
- **Chaos meanings:** Insane = faster base tick (0.10s vs 0.14s) that shrinks 2ms per food down to a 0.05s floor; Ridiculous = wrap-around walls (dimmed wall frame — portals, not hazards) + two pellets on the board; Insiculous = both. Single sources: `tick_interval(mode, foods_eaten)`, `food_count(mode)`, `walls_wrap(mode)` in `gameplay.rs`.
- **Rule-of-three check:** nothing new to promote — the tick accumulator is Snake-first (one use), and all shared scaffolding (`MenuInput`, `spawn_background`, `default_playfield_grid` + `step_and_emit_grid`, `RENDER_UNIT`, `set_sprites_visible`, `hash_u32`, `game_root!`) was consumed from the engine as promoted. No engine changes.

### Game 5: Asteroids ☑ (July 13, 2026) — PHASE A COMPLETE
Rotating/thrusting ship in zero-g, rocks split on hit, everything wraps. Lives in `../games/asteroids/` — 37 tests, clippy-clean, full chaos-mode + achievements support (11), deforming grid background, particles. **First wireframe-vector game**: the ship (4-segment neon hull) and rocks (10-vertex hash-seeded irregular polygons, generated once at spawn) are drawn by pushing `LineVertex` pairs into `ctx.lines` after `step_and_emit_grid` — no sprites on gameplay bodies; bullets/particles/background stay sprites.

- **Taught:** Rotation-based movement (dynamic sensor ship, free rotation, ONE `set_velocity(linear, angular)` per frame from pure `integrate_ship` — no game-side heading/velocity state, rapier's Transform2D writeback is the source of truth), screen wrap via `Transform2D.position` writes on live dynamic bodies (GPP-09 teleport preserves velocity — proven by a dedicated sim test), asteroid splitting (destroy + spawn hash-fanned children offset 1.2×parent-radius so solid bodies don't spawn interpenetrated), invincibility frames (blink = skip wireframe emission; ship-vs-rock death is a game-side circle test because a rock overlapping through i-frames must kill when mercy ends — stale `started` events can't)
- **Key components:** Ship = dynamic sensor circle (rocks never deflect it); rocks = solid dynamic circles, restitution 1.0 (they elastically bounce off each other — rapier does the work); bullets = dynamic sensors + `Lifetime` (SI recipe). `PhysicsConfig::space()` for zero-g.
- **Controls:** A/D or arrows rotate, W/Up thrust, Space fire (max 4 live bullets), Esc to title. Endless waves (base 4, +1/wave, cap 9), border spawns with a deterministic opposite-side fallback (opposite perimeter points are ≥600px apart on this window — proven in test). Extra ship every 10k.
- **Chaos meanings:** Insane = rock speed ×1.6 + i-frames halved; Ridiculous = large rocks split into 3 (not 2) + 1 extra rock per wave; Insiculous = both. Single sources: `asteroid_speed`, `invincibility_secs`, `split_spec`, `wave_asteroid_count`.
- **Rule-of-three check:** screen wrap (`wrap_position`) and wireframe polygon emission are both first-use — stay game-side; promote if game 6+ wants them. All shared scaffolding consumed from the engine as promoted. No engine changes.

---

## Engine & Editor Milestones (moved from PROJECT_ROADMAP "Archive: Completed Work" and AGENTS.md)

### Roadmap Phase B, Gap 1: CameraFollow behavior ☑ (July 16, 2026) — PHASE B COMPLETE
- `Behavior::CameraFollow { target_tag, lerp_speed, offset, dead_zone }` (8th variant): dt-corrected exponential lerp toward the nearest tagged entity — `alpha = 1 - (1 - lerp_speed)^(dt·60)`, so `lerp_speed` is fraction-per-frame at 60 FPS exactly (0.5 converges within 10 frames per the roadmap acceptance test; 1.0 snaps in one frame), frame-rate independent otherwise. `dead_zone: Some((w, h))` = full-size box centered on the entity, platformer clamp-to-edge semantics (no movement while the focus point is inside; lerps the focus back to the box edge when outside). Offset/dead_zone use `(f32, f32)` tuples per the enum's serde convention.
- Applied via a new `positions` channel on `BehaviorCommands` (absolute `Transform2D.position` write after velocities — CameraFollow entities carry no RigidBody, no physics branch).
- **New engine mechanism — main-camera sync:** `RenderManager::sync_main_camera(world)` copies the first `Camera { is_main_camera: true }` + `Transform2D` entity's position onto the render camera each frame, called in `GameRunner::render_frame` before `game.render()` (games can still override `ctx.camera`; worlds without a main-camera entity are untouched — position only, zoom/rotation/viewport stay render-managed). This closed the "world Camera components are inert" gap.
- Editor: CameraFollow arm in the behavior variant editor (new `ranges::FRACTION` 0..=1 for lerp_speed; dead_zone read-only pending an Option widget). 10 new tests (6 acceptance in `engine_core/tests/camera_follow.rs`, 2 sync, 2 serde).

### Roadmap Phase B, Gap 3: Tilemap component ☑ (July 16, 2026)
- `ecs::Tilemap { width, height, tile_size, tileset, tiles, tile_uv_size, depth }` (new `crates/ecs/src/tilemap.rs`): row-major grid of tile indices, `0` = empty, value `t` = tileset cell `t-1` counted row-major across `1/tile_uv_size.x` columns. `Transform2D.position` anchors the **center of tile (0,0)**, row 0 on top, rows grow downward (world Y up); `tile_size` in pixels (positions never go through RENDER_UNIT); transform rotation/scale ignored. `depth` defaults to **-1.0** (behind default-depth sprites). `sprite_instances()` yields plain `TileInstance { offset, tex_region }` data (ecs has no renderer dep); short `tiles` vecs truncate instead of panicking.
- `engine_core/src/tilemap_render.rs` expands every Tilemap+Transform2D entity into the game batcher at the top of the default `Game::render` — whole map shares the tileset's batch (one draw call), static maps upload once via the GPP-15 InstanceCache byte-compare.
- Wired through all component touchpoints: global ECS registry, scene RON schema (`ComponentData::Tilemap` with `tileset` as a texture-ref string, load via `TextureResolver`, save via `texture_path_fn`), editor registry (`readonly`, Rendering category). 11 new tests (7 component, 2 render expansion, 1 loader round-trip, 1 serializer).
- Unblocks Frogger (game 6), Tetris, Pac-Man, Zelda, Tower Defense, Sokoban, Roguelike.

### Roadmap Phase B, Gap 2: Lifetime auto-despawn ☑ (July 13, 2026)
- `ecs::lifetime` — `Lifetime { remaining }` component + `LifetimeSystem` (both in the engine prelude): ticks every carrier down by the frame delta and `remove_entity`s on expiry (hierarchy auto-detaches; physics GC reclaims rapier state next update). 4 headless tests incl. the roadmap acceptance criterion (alive at t=0.4, gone at t=0.6 for a 0.5s lifetime). Unblocks bullets/effects for Space Invaders, Galaga, Run & Gun, Bullet Hell.

### Manager Pattern + File Refactoring (January 2026) — COMPLETE (from AGENTS.md)
- SRP refactoring: `GameRunner.update_and_render()` 110+ lines → 25 lines; 7 focused orchestration methods extracted
- 5 managers extracted: GameLoopManager, UIManager, RenderManager, WindowManager, SceneManager
- 5 modules extracted (674 lines, 12 tests): `game_config.rs`, `contexts.rs`, `ui_integration.rs`, `scene_manager.rs`; `behavior_runner.rs` optimized (-85% per-frame allocations)
- `game.rs`: 862 lines (mixed concerns) → 553 at extraction time (later ~594 after GlyphTextureCache and other additions)

### Editor Integration (February 2026) — COMPLETE (from AGENTS.md)
- New `editor_integration` crate: `EditorGame<G>` wrapper, `run_game_with_editor()`, `panel_renderer` extraction
- Phase 1C play/pause/stop: `EditorPlayState` enum with border tint, `WorldSnapshot` typed-clone capture/restore, `PlayControls` widget (Ctrl+P, Ctrl+Shift+P, F5), conditional `inner.update()`, read-only inspector during play
- Hard-coded Escape exit removed from `GameRunner` (flows to `Game::on_key_pressed()`); `editor_demo.rs` switched to full PlatformerGame

### Engine Core (2025) — COMPLETE
- Memory safety, thread-safe input, panic-safe system registry
- Sprite rendering (WGPU 28), ECS with type-safe queries, asset management
- Rapier2d physics, scene serialization (RON + prefabs), scene graph hierarchy
- Audio (Rodio, spatial), immediate-mode UI, Simple Game API (`Game` trait, `run_game()`)
- SRP refactoring: GameLoopManager, UIManager, RenderManager, WindowManager, SceneManager extracted
- Test count at the time: 724 passing, 29 ignored (GPU/window only), 0 failed

### Editor Phase 1 (January–February 2026) — COMPLETE
- Dockable panel system, scene viewport (pan/zoom, grid overlay, LOD)
- Entity picking (click, rectangle), transform gizmos (translate/rotate/scale)
- Component inspector with live writeback (Transform2D, Sprite, RigidBody, Collider, AudioSource)
- Generic serde-based read-only display for any component
- Component add/remove (categorized popup, cascade removal)
- Entity CRUD (create empty/sprite/physics, delete, duplicate Ctrl+D)
- Hierarchy panel (tree view, expand/collapse, Ctrl+click multi-select)
- Play/Pause/Stop with `WorldSnapshot` capture/restore (Ctrl+P, F5)
- Undo/redo command system (11 command types, merging for continuous edits)
- Scene save/load (Ctrl+S/Ctrl+O/Ctrl+N, RON format, dirty flag)
- Editor preferences persistence (camera, zoom, grid, last scene)
- `EditorTheme` system (30+ color tokens, converter methods)
- Status bar (entity count, FPS, status messages)
- Snap-to-grid (toggle, configurable grid size)

### Phase 2A: Standalone Infrastructure (March 2026) — COMPLETE
- Standalone editor binary (`cargo run --bin editor -- /path/to/project`)
- Standalone game project (`my_platformer`) consuming engine as external dep
- Editor font path fix, extended engine prelude

---

## PROJECT_ROADMAP — Resolved Technical-Debt Notes (moved from the roadmap)

**Resolved Aug 27 2026 (editor UX-audit Sprint 2, issue #28):** ui JUN-T2
(`scroll_delta` captured but unconsumed) — the shared `editor::ScrollState`
now consumes it in three panels (inspector, hierarchy, asset browser) under
the documented mouse-in-bounds ownership convention; a generic ui-crate
`scroll_area` widget remains a future nicety alongside JUN-T3.

**Resolved by the June 2026 audit passes** (previously listed in the roadmap's Technical Debt section):
renderer SRP-001 (`sprite.rs` split into `sprite/{batch,pipeline}.rs`) and
ARCH-003 (all `#[allow(dead_code)]` removed, ~700 lines dead code deleted);
audio ARCH-001 reclassified as a feature gap (streaming — audio crate has 0
open debt); input TEST-001 superseded by GAP-001 (dead-zone tests land
with the gamepad backend); physics TEST-001 mostly closed (sensor + collision
response tests added; remaining friction/kinematic/tunneling gaps are Low).

**Resolved June 11, 2026 (tech-debt pass):** engine_core SRP-001
(`GlyphTextureCache` extracted to `glyph_texture_cache.rs`), SRP-002
(BehaviorRunner match split into 7 handler methods), LOGIC-002 (`let-else`
replaces the asset_manager `unwrap()`), ARCH-007 (`ToastStyle` on
`AchievementManager`, `reset()` logs save errors), ARCH-003 (all `lib.rs`
glob re-exports made explicit); ui SRP-001 (`font/` split: FontManager facade
+ `GlyphCache` + `layout`), SRP-002 (`context/` split: mod/text/widgets/tests,
all files <600 lines).

---

## training.md — Retired "Known Limitations" entries (moved Jul 2026)

All items from training.md's old "Current Known Limitations (January 2026)" list were fixed:
SRP violations in GameRunner (managers extracted); bind groups created per frame (camera +
texture bind groups cached); glyph texture cache color-in-key (Feb 2026 — `GlyphKey` excludes
color, grayscale masks); first-frame UI placeholder flicker (font rendering bug); 40+
allocations per frame in behavior system (behaviors accessed by reference); component
registration requiring a separate ComponentMeta impl (`#[derive(ComponentMeta)]` macro).
The section now just points at the live debt docs.

---

## Workspace Rollup — Resolved in the 2026 Audit Passes (from root TECH_DEBT.md)

> Note: the pre-June-2026 version of the root TECH_DEBT.md was a January 2026 review of the
> editor change set. All of its items were resolved or superseded by the June 2026
> remediation passes: mouse-button release tracking now exists via the shared
> `ButtonTracker` (`is_source_just_released`), editor shortcuts use real
> modifier combinations (Ctrl+S, Ctrl+Shift+P, …), `EditorInputMapping`
> delegates to the generic `InputMapping<EditorAction>`, and panel rendering
> moved out of `examples/editor_demo.rs` into `editor_integration`.

- **ecs (Feb):** broken archetype/dual storage deleted entirely (single HashMap-based path), hierarchy cycle detection, `WorldHierarchyExt` extraction, generation-validated component ops
- **renderer (Jun):** bloom blur bug (uniform rewrite between passes), sprite overflow panic → growing `DynamicBuffer`, NaN-safe depth sort, per-frame bind-group/clone churn eliminated, `sprite.rs` split, ~700 lines dead code removed
- **ui (Jun):** glyph bitmaps shared as `Arc<[u8]>` (no per-frame copies), focused-widget state survives unseen frames, theme bypass fixed (`TextInputStyle`), dead draw/interact APIs deleted
- **input (Jun):** stale mouse-delta bug, unbind/rebind leak, strict action-edge semantics; `InputMapping<A>` made generic, `ButtonTracker<T>` deduplicates device state, `ThreadSafeInputHandler`/`init()`/`InputError` deleted (~250 lines)
- **audio (Jun):** per-play full-buffer clone removed (`Arc<[u8]>` + `Cursor`), live-sink volume re-apply, clamping at point of use, `stop(handle)` implemented, dead `PlaybackState` deleted
- **physics (Jun):** collision event clear/append contract (no stale re-emission, no sub-step loss), world-space contact points, one-update forces, raycast normalization, `PhysicsError`/`MovementConfig` deleted, directory splits under 600 lines
- **engine_core (Jun):** orphaned `scene_saver.rs`/`file_operations.rs` deleted (single save pipeline), Behavior↔BehaviorData conversion collapsed to one `From` pair, dead `game_loop.rs` deleted, clippy-clean incl. `--all-targets`
- **editor + editor_integration (Jun):** component registry macro (`stored_component.rs`) as single source of truth, `ComponentEdit<T>` writeback, 1,100-line files split into feature directories, full theme routing, duplicate `ComponentKind`/dispatch deleted
- **engine_core (Jun 11 debt pass):** `GlyphTextureCache` extracted from `GameRunner`, BehaviorRunner match split into per-variant handlers, asset_manager `unwrap()` → `let-else`, `ToastStyle` on `AchievementManager` (+ `reset()` logs save errors), all `lib.rs` glob re-exports made explicit
- **ui (Jun 11 debt pass):** `context.rs` (~990 lines) split into `context/` (mod/text/widgets/tests), `font.rs` split into `font/` (FontManager facade + `GlyphCache` + `layout`); public API unchanged, all files <600 lines

---

## ecs — Resolved Debt

### July 13, 2026 — GPP-04 + SRP-003 (Dirty Flag, transform hierarchy)
- **GPP-04**: `TransformHierarchySystem` no longer recomputes every `GlobalTransform2D` every frame. Design (Jesse-approved): **self-contained value-compare cache** — the system keeps a `HashMap<EntityId, CachedNode { local, parent, stamp }>` baseline; a node is dirty iff its local `Transform2D` or parent link differs from the baseline, its global is missing, or an ancestor was dirty. Clean frames recompute nothing (and do zero mutable World access). Chosen over store-level ECS dirty flags because value comparison catches every writer by construction (incl. editor `WorldSnapshot::restore`, which re-adds components) and identical physics writebacks stay clean; zero ECS API changes. **Decision of record:** no shared change-tracking infra — GPP-09 will keep its own last-pushed cache in physics, GPP-15 a coarse render-path counter.
- **SRP-003** folded in: the old double iteration (root loop + recursive `propagate_transforms`) replaced by a single iterative DFS on a reusable stack — also killing the per-node children `Vec` allocs and the `entities()`/`get_root_entities()` per-frame allocations.
- New API: `reset()` (drop all baselines — the `PhysicsSystem::clear()` analogue, called by the editor's Stop after snapshot restore), `recomputed_last_update()`, `visited_last_update()`, `tracked_entity_count()` for headless observability. `GlobalTransform2D` gained `Copy, PartialEq`. Cache pruning via frame stamps (removed entities fall out; stamps refreshed on every visit, including clean-under-dirty-ancestor).
- Tests: 9 new in `crates/ecs/tests/hierarchy_dirty.rs` (clean-frame zero, leaf-only, subtree-only, reparent, orphan-on-parent-delete + prune, identical-write-stays-clean, re-enable self-heal, global re-added, reset) + editor_integration `test_stop_resets_transform_propagation_cache`; the 5 pre-existing propagation tests unchanged.

### July 13, 2026 — GPP-01 (State pattern)
- **GPP-01**: `BehaviorState` boolean-flag soup (`patrol_toward_b`/`is_chasing`/`is_waiting`) replaced with `StateMachine<BehaviorPhase>` — new `BehaviorPhase { Idle, Patrolling { toward }, Waiting { then_toward }, Chasing }` + `PatrolTarget { A, B }` enums in `behavior.rs`; patrol/chase handlers in `engine_core/behavior_runner.rs` rewritten as explicit FSM transitions (wait duration now uses the machine's `elapsed()` clock instead of a countdown timer; `timer` field retained for the platformer jump cooldown). Illegal states (waiting AND chasing) now unrepresentable; `state_machine.rs` gained its first real consumer. Regression tests: patrol arrive→wait→reverse, chase enter/hysteresis/leave, no-target stays Idle (`behavior_runner.rs` tests) + updated ecs behavior tests. Editor `WorldSnapshot`/registry unaffected (hidden-component capture is Clone-based).

### February 2026 Fixes
- **PATTERN-001**: Broken archetype storage removed entirely. ECS now uses single HashMap-based per-type storage. Proper archetype storage deferred as future ground-up rewrite. The `Component` trait still requires `as_any()` for downcasting — inherent to HashMap-based storage with `Box<dyn Component>`.
- **PATTERN-002**: Dual-storage system removed. `World::new_optimized()`, `ComponentStorage` enum, `ArchetypeStorage`, and `archetype.rs` deleted. Single storage path via `ComponentRegistry` → `ComponentStore`. Documentation updated from "archetype-based" to "HashMap-based per-type storage".
- **SRP-002**: `ComponentStorage` enum removed. Single `ComponentStore` type (no delegation/match arms).
- **ARCH-003**: Archetype dead code removed (`archetype.rs` deleted, query types extracted to `query.rs`).
- **ARCH-004**: Dual storage systems removed. Single HashMap-based storage path.
- **KISS-002**: Over-engineered `ComponentColumn` raw pointer manipulation — originally resolved with safety documentation (January 2026); the entire `archetype.rs` file was then deleted in February 2026. The unsafe raw pointer code is no longer in the codebase.

### January 2026 Fixes
- **SRP-001**: Extracted 10 hierarchy methods (~150 lines) to `WorldHierarchyExt` extension trait (`set_parent`, `remove_parent`, `get_parent`, `get_children`, `get_root_entities`, `get_descendants`, `get_ancestors`, `is_ancestor_of`, `is_descendant_of`, `remove_entity_hierarchy`); 11 tests. World struct retains 6 core responsibilities (~400 lines).
- **KISS-001**: Replaced non-functional `QueryIterator` scaffolding with working `query_entities::<Q>()` method (25 lines vs 45, actually works; test added).
- **ARCH-001**: Module visibility strategy documented in `lib.rs` — private modules for core infrastructure, public modules for domain-specific concerns.
- **ARCH-002**: Cycle detection added to `set_parent()` using `is_ancestor_of()`. Tests: `test_hierarchy_cycle_detection`, `test_hierarchy_self_parent_rejected`.
- **DRY-002**: Extracted `set_global_transform()` helper in `hierarchy_system.rs` to eliminate the duplicate update-or-add pattern.

### Earlier (from ANALYSIS.md)
| Issue | Resolution |
|-------|------------|
| Deprecated PlayerTag alias | Removed, use EntityTag instead |
| Incomplete test assertions (TODO comments) | All replaced with meaningful assertions |
| Memory safety issues | Generation tracking implemented |
| System registry memory safety | catch_unwind for panic isolation |

---

## engine_core — Resolved Debt

### July 13, 2026 — GPP-07 + DRY-010 (Prototype: runtime prefab spawning)
- **GPP-07**: `SceneInstance` retains the scene's prefab table (`pub prefabs` + `has_prefab()`) and gained `spawn_prefab(world, assets, name, overrides) -> Result<EntityId>` — runtime stamping of RON-defined templates with scene-file override semantics (reuses `merge_components` + `add_component_to_entity`). A spawn that fails mid-build removes the half-built entity; spawned entities are caller-owned (not added to instance bookkeeping). New `TextureResolver` trait in `texture_ref.rs` is the GPU seam: `AssetManager` is the production impl, tests stub it — `SceneLoader` internals generified from `&mut AssetManager` to `&mut impl TextureResolver` (all call sites unchanged), making scene/prefab instantiation headless-testable for the first time. Dead private `SceneLoader::resolve_texture` wrapper deleted.
- **DRY-010** folded: `merge_components`' duplicated replace-or-append loop extracted to `apply_component_layer()`.
- Tests: 5 headless integration tests in `engine_core/tests/prefab_spawning.rs` (table retained, spawn stamps a new caller-owned entity, overrides replace, unknown prefab errors without debris, mid-build failure cleans up).

| Issue | Resolution |
|-------|------------|
| DRY-001/DRY-006/DRY-007 placeholder audio + ctx duplication | Consolidated into `initialize_and_update()` |
| DRY-002 coord transforms (ui_integration) | Helpers extracted (Feb 2026) |
| DRY-004 hex color parsing | `parse_hex_byte` helper (Feb 2026) |
| DRY-005 surface error recovery | `handle_render_error` helper (Feb 2026) |
| SRP-003/ARCH-001 EngineApplication dual API | Deleted entirely (Feb 2026) |
| KISS-001 glyph cache key color | Color removed from key; grayscale masks (Feb 2026) |
| LOGIC-001 `partial_cmp().unwrap()` depth sort | `total_cmp()` (Feb 2026) |
| LOGIC-003 lifecycle lock unwraps | All use `unwrap_or_else` poison recovery + test coverage |
| ARCH-002 Timer vs GameLoopManager | Reclassified not-an-issue: Timer is a user-facing utility, GameLoopManager is the engine frame timer |
| ARCH-004 duplicate UIManager accessors | Consolidated to `ui_context()` (Feb 2026) |
| DRY-008 two parallel World→RON save pipelines | Resolved (Jun 2026): `scene_saver.rs` and `editor/file_operations.rs` were ORPHANED files — never declared as modules. Both deleted; `scene_serializer.rs` is the single save pipeline, consumed by `editor_integration`. |
| DRY-009 Behavior ↔ BehaviorData conversion ×3 | Resolved (Jun 2026): single pair of `From` impls in `scene_data.rs` (load + save direction). Adding a variant now means: enum + the two `From` arms. |
| DEAD-001 `game_loop.rs` dead code | Resolved (Jun 2026): `game_loop.rs` + tests deleted, exports removed. `GameLoopManager` is the only frame timer. |
| QUAL-001 clippy warnings | Resolved (Jun 2026): orphaned doc comment, needless borrow, `push_arc` derives segment count from sweep angle, `!= true` assertion, trait placement. `cargo clippy -p engine_core --all-targets` clean. |
| ARCH-005 `grid/` single-game code | Decision (Jun 2026): KEEP in engine as a general-purpose deformable-grid effect (candidate for editor grid visualization). Module docs updated. Pong remains the reference consumer. |
| SRP-001 GameRunner owned glyph texture caching | Resolved (Jun 2026): extracted `GlyphTextureCache` (`glyph_texture_cache.rs`); `GameRunner` just calls `prepare()`/`textures()`. Unit-tested headless. |
| SRP-002 BehaviorRunner giant match | Resolved (Jun 2026): one private handler per variant plus `BehaviorCommands` + `apply_commands()`; no logic change. |
| LOGIC-002 `unwrap()` on asset_manager | Resolved (Jun 2026): `let Some(...) else { log::warn!(...); return; }` — production code is `unwrap()`-free. |
| ARCH-007 achievement toast styling hardcoded | Resolved (Jun 2026): `ToastStyle` struct with `Default` matching the old appearance; `set_toast_style()`/`toast_style()`; `reset()` logs its save error. |
| ARCH-003 glob re-exports in lib.rs | Resolved (Jun 2026): all 15 `pub use module::*` globs replaced with explicit re-export lists; prelude semantics unchanged. |

### What's Healthy (June 2026 audit snapshot)
- Manager pattern holds: `RenderManager`, `WindowManager`, `UIManager`, `SceneManager`, `GameLoopManager` are each single-purpose and small.
- `SceneManager` vs `SceneLoader` separation is clean (stack management vs deserialization).
- `particles/` is genuinely reusable (config builder, pooling, no game-specific tuning); the emitter-in-ECS / particles-in-manager split is intentional.
- Production code is `unwrap()`-free; errors are typed (`thiserror`) throughout the scene pipeline.
- `ChaosMode` correctly carries selection only — no gameplay logic in the engine.

---

## physics — Resolved Debt

### July 13, 2026 — GPP-09 (Dirty Flag / Observer: external-edit detection)
- The "sync only ADDS bodies" footgun is gone. `PhysicsSystem` keeps a per-entity `PushedState { position, rotation, collider }` baseline (the GPP-04 no-shared-infra pattern applied locally): during `sync_entity_to_physics`, live entities' components are value-compared against it — an edited `Transform2D` teleports the body via `set_body_transform` (velocity preserved), an edited `Collider` rebuilds the rapier collider (`add_collider` already removes-then-adds), a removed `Collider` component drops the rapier collider. The physics writeback (`sync_physics_to_ecs`, now `&mut self`) refreshes the baseline so rapier-driven motion is never mistaken for an external edit; identical-value writes stay clean. This also fixes the editor gap where inspector collider edits never reached the simulation.
- `Collider`/`ColliderShape` gained `PartialEq`; observability via `external_edits_pushed_last_update()`; baselines pruned in `destroy_entity`/`clear`/orphan GC. Verified safe against existing Transform2D writers (games write only on pre-sync freshly-spawned entities).
- Remaining (new Low, EDIT-001): `RigidBody` config edits still require body recreation (velocity fields are writeback-owned, so whole-struct compare would false-positive).
- Tests: 5 new in `physics/tests/external_edits.rs` (teleport + velocity preserved, writeback-not-an-edit over 10 falling frames, identical write pushes nothing, collider enlarge → real rapier collision, collider removal drops rapier collider).

### July 13, 2026 — GPP-08 + GPP-10 + GPP-L9 (Event Queue / Observer / Command)
- **GPP-08 (Event Queue)**: game-facing collision event API is now drain-style — `PhysicsSystem::take_collision_events() -> Vec<CollisionData>` (+ `PhysicsWorld::take_collision_events`). Consumers own the Vec, so no borrow is held while reacting (the `.to_vec()` snapshot footgun is gone) and a frame's events are consumed structurally. `PhysicsSystem::collision_events()` (borrow) deleted; `PhysicsWorld::collision_events()`/`clear_collision_events()` remain for the driver's internal clear/append contract. Migrated: engine_core pickups doc-test flow, pong + breakout gameplay (one take per frame, Vec shared with all consumers incl. `Pickups::collect`), physics integration + delivery tests. New regression test: `test_take_collision_events_drains_the_buffer` (take returns events; second take empty; buffer empty).
- **GPP-10 (Observer)**: synchronous collision callbacks deleted outright (were labeled legacy; fired under `&mut world` so they couldn't mutate anything; zero users outside physics' own tests). `CollisionCallback` type, `with_collision_callback`, `add_collision_callback`, `clear_collision_callbacks`, `collision_callback_count`, and the update-loop invocation removed. The two delivery channels are now the world event bus (`world.emit_event`/`read_events::<CollisionData>()`) and `take_collision_events()`.
- **GPP-L9 (Command)**: the two parallel deferred queues (`pending_velocities` + `pending_resets` tuple-Vecs with drain-resets-first-by-convention) replaced by a single `pending_ops: Vec<(EntityId, DeferredBodyOp)>` enum queue drained in call order — the documented "reset then launch" pattern works because the launch is queued after the reset. `destroy_entity`/`clear`/orphan-pruning updated; deferred-op ordering pinned by test.

### June 2026 Audit Remediation (all correctness findings fixed; 58 passing lib tests)
- **Stale event re-emission**: `step()` no longer clears the event buffer; `clear_collision_events()` added, `PhysicsSystem::update()` clears once before the sub-step loop. Zero-step frames emit nothing.
- **Sub-step event loss**: `step()` APPENDS events, so all catch-up sub-steps' events survive a single update.
- **Contact points were collider-local**: now transformed through collider1's world isometry (point and normal) before meters→pixels.
- **`apply_force` persisted forever**: forces reset after the step loop (`reset_forces()`); skipped on zero-step frames. Documented as one-update.
- **Dead `RigidBody::apply_impulse`/`apply_force` component methods removed** (mutated a `velocity` field only read at body creation — silent no-op on live bodies).
- **MISSING-001 (partial)**: `pixels_per_meter` validated in `with_scale` and at `PhysicsWorld::new` (non-finite or <= 0 → warn + default 100.0).
- **CollisionPair bit-packing overflow**: canonical ordering compares `(value, generation)` tuples.
- **`raycast` direction**: normalized internally; zero/non-finite returns `None`; distance always in pixels.
- **Kinematic dead config**: removed ignored `linvel`/`angvel` on `kinematic_position_based` bodies.
- **Non-test `unwrap()`**: `NonZeroUsize::new(8).unwrap()` replaced with `config.solver_iterations.max(1)` + `NonZeroUsize::MIN` fallback.
- **Iteration names matched to rapier**: `velocity_iterations`/`position_iterations` → `solver_iterations`/`friction_iterations`.
- **Unused deps removed**: `toml`, `common`, `thiserror` (after `PhysicsError` deletion).
- **Dead public API deleted**: `PhysicsError`, `MovementConfig`; `Collider::player_box()` takes `(width, height)`.
- **DRY-002/DRY-003**: `body(entity)`/`body_mut(entity)` helpers replace seven duplicated lookups; builder setup deduplicated across the three RigidBodyType arms.
- **600-line rule**: `physics_world.rs` (977) → `physics_world/{mod,bodies,stepping,queries,tests}.rs`; `physics_system.rs` (775) → `physics_system/{mod,sync,update,tests}.rs`.
- **Test gaps closed**: sensor intersection, real-collision callbacks, world-space contacts, scale validation, raycast normalization, event delivery contracts, physics+hierarchy pin (physics entities must be root entities).
- ℹ️ `CollisionCallback` keeps `Send + Sync`: required because `ecs::System` has `Any + Send + Sync` supertraits.

### January/February 2026
- **ARCH-002**: multiple collision callbacks (`with_collision_callback`, `add_collision_callback`, `clear_collision_callbacks`, `collision_callback_count`) + tests.
- **ARCH-001**: pass-through methods documented as intentional API ergonomics. Follow-up (April 2026): `set_body_velocity` + `apply_impulse` collapsed into game-facing `set_velocity` (deferred-safe on same-frame spawns); `PhysicsWorld::apply_impulse` remains for genuine mass-aware impulses.
- **KISS-001**: proper collision start/stop tracking (`CollisionPair` canonical ordering, `previous_collisions` set, `started`/`stopped` flags) + 4 tests.
- **DRY-001**: all 12+ locations use `pixels_to_meters`/`meters_to_pixels` helper methods.
- Dead-code warning on conversion methods: now public API.

### Code Quality Notes (June 2026 snapshot)
Clean rapier2d integration; excellent presets; good ECS sync; fixed timestep with accumulator; comprehensive builder API; `presets.rs` extends structs with `impl` blocks — clean pattern.

---

## renderer — Resolved Debt

### July 13, 2026 — GPP-15 + ARCH-007 (Dirty Flag: static-scene upload skip)
- **GPP-15**: sprite instance uploads are now change-gated. New `sprite/instance_cache.rs` `InstanceCache` (CPU-only, headless-tested): flattens batches into a reusable staging buffer and byte-compares (bytemuck, NaN-safe) instances + batch layout (texture, count) against the last upload; `SpritePipeline::prepare_sprites` skips `write_buffer` when nothing changed — a static scene re-renders from the buffer already on the GPU. Detection is on the *built* data (after `game.render()` runs), so procedural game render code stays fully supported; static UI frames (menus, pause screens) skip too. Per the GPP-04 decision: render-path-local detection, no ECS hooks.
- **ARCH-007** folded in: the per-frame scratch `Vec` in `prepare_sprites` is gone — the cache's staging/snapshot buffers are persistent.
- engine_core side: `GameRunner` now owns persistent `game_batcher`/`ui_batcher` fields (cleared per frame, capacity retained — no per-frame `SpriteBatcher::new()` HashMap churn) and the per-frame full clone of every batch (`.values().cloned().collect()`) is gone — `sort_batch_refs` orders `&SpriteBatch` refs (empty batches from persistent batchers filtered out; game-then-UI painter order preserved).
- Tests: 4 new `InstanceCache` unit tests (identical-skip + counters, instance-move re-uploads, same-bytes-different-layout re-uploads, empty↔content transitions).
| Issue | Resolution |
|-------|------------|
| **Bloom blur was vertical-only** | `queue.write_buffer` flushes at submit; rewriting one shared blur-params buffer between passes made every pass read the last write. Split into per-direction uniform buffers. |
| **Sprite overflow panicked at 1,001 sprites** | `DynamicBuffer::update` grows the GPU buffer (next power of two) instead of `panic!`. Line pipeline's silent truncation replaced by the same growth path. |
| **NaN depth panicked the batch sort** | `sort_by_depth` uses `f32::total_cmp` (NaN sorts last, deterministic). Regression test added. |
| **Bloom created 2 + 2×iterations bind groups per frame** | Bind groups cached per render-target size, rebuilt on resize. Blur texel params written only on resize. |
| **Texture map deep-cloned twice per frame** | White texture cached via `cache_texture_bind_group`; `game.rs` uses `assets.textures()` by reference; `textures_cloned()` removed. |
| **`render_batcher` cloned every batch per frame** | Collects `&SpriteBatch` refs; sorts deterministically (min depth, then handle). |
| **Magic white-texture handle `{ id: 0 }`** | `TextureHandle::WHITE` const. |
| **`generate_mipmaps` produced broken textures** | Flag removed (allocated mips were never filled). |
| **`render_pipeline_inspector.rs` (434 lines)** | Deleted — never declared in `lib.rs`, not even compiled. |
| **`shaders/sprite.wgsl`** | Deleted — only `sprite_instanced.wgsl` is referenced. |
| **`TextureResource::create_solid_color` stub** | Deleted — created textures without uploading pixel data, zero callers. |
| **Legacy `Renderer::render()` + `render_basic()` + `run_with_app`** | Deleted — `Game` trait / `run_game()` is the only path. |
| **4 unused `RendererError` variants** | Removed. |
| **6 `#[allow(dead_code)]` suppressions** | All removed (fields deleted or became used). |
| **Empty placeholder test** | Deleted. |
| **`sprite.rs` at 1,059 lines** | Split: `sprite.rs`, `sprite/batch.rs`, `sprite/pipeline.rs`; atlas types in `atlas.rs`. |
| **No renderer configuration** | `RendererConfig { vsync }` wired through `GameConfig::with_vsync()`. |
| **Nondeterministic cross-batch draw order** | `render_batcher` sorts same as `game.rs::sort_batches`. |
| **line_pipeline doc claimed shared camera layout** | Doc corrected (sharing tracked as DRY-006). |

### January/February 2026
| Issue | Resolution |
|-------|------------|
| Bind groups created every frame (sprites) | Camera bind group cached; texture bind groups cached per handle |
| DRY-001: duplicate surface error handling | `acquire_frame()` helper |
| DRY-002: duplicate sampler creation | `SamplerConfig::create_sampler()` |
| KISS-002: unsafe transmute for surface lifetime | WGPU 28 infers `'static` from `Arc<Window>` |
| ARCH-002: `Time` struct misplaced | Moved to `common` crate |
| ARCH-004: inconsistent error types | `From<TextureError> for RendererError` |
| SRP-002: Renderer init + rendering coupled | Documented as intentional (WGPU lifetimes) |

---

## ui — Resolved Debt

### [JUN-T1] Text input is numeric-only — RESOLVED (July 16, 2026)
General text input shipped with the panels/UI/localization plan: `keycode_to_char` + `typed_keys` grew A–Z (shift = uppercase), Space, and Shift+Minus = `_`, and a free-form `text_input` widget joined `float_input` (same editing model: select-all-on-focus, cursor/selection/repeat, Enter/Tab/click-away commits, Escape cancels). Editor string fields (`UiLabel.text`, `UiButton.id`, …) edit through it. The keyboard-layout-blindness half was re-scoped as Low JUN-T1b (physical-keycode mapping is US-layout-only; fix = winit character events through `InputHandler`).

### January–June 2026
- **DRY-001**: `layout_to_draw_data()` helper — both `label_styled()` and `label_with_font()` use it (Jan 2026).
- **DRY-002**: `widget_background_color(state)` helper for button/checkbox (Jan 2026).
- **DRY-003**: `with_theme()` delegates to `new()` (Jan 2026).
- **DRY-004**: text-drawing helpers extracted (`draw_text_with_font`, `draw_text_at_baseline`, `text_pos_in_bounds`, `estimate_text_size` — was 5 scattered magic-number copies) (Jun 2026).
- **SRP-001**: `font.rs` → `font/` directory (`mod` facade, `glyph_cache.rs`, `layout.rs`); `FontLoader` deliberately not extracted (Jun 2026).
- **SRP-002**: `context.rs` (~990 lines) → `context/{mod,text,widgets,tests}` (Jun 2026).
- **KISS-001**: `WidgetPersistentState` unused `float_value`/`bool_value` deleted (Jun 2026).
- **ARCH-001**: dual glyph caching is intentional (CPU bitmaps in ui, GPU textures in engine_core); real bug was `layout_text()` bypassing the cache — fixed (Jan 2026).
- **ARCH-002**: `rect.rs` removed; `Rect` re-exported from `common` (Jan 2026).
- **PERF-001**: `measure_text()` uses `font.metrics()` (no rasterization) (Feb 2026).
- **DRY-005**: `next_depth()` helper in DrawList (Feb 2026). **DRY-006**: `baseline_y()` helper (Feb 2026). **DRY-007**: private `palette` module for theme hex constants (Jun 2026).
- First-frame font rendering bug: static PRINTED flag removed; retries every frame.

### June 2026 Audit (JUN-001..008)
| ID | Issue | Resolution |
|----|-------|------------|
| JUN-001 | `unwrap()` outside tests in `font.rs` | Single fallible lookups with `ok_or_else` |
| JUN-002 | Per-frame double glyph-bitmap clone | `Arc<[u8]>` bitmaps; clones are O(1) refcount bumps |
| JUN-003 | Persistent-state GC dropped focused widget's state on skipped frame | `end_frame` retains focused widget's state; regression tests |
| JUN-004 | `DrawCommand::depth()` = 0.0 for clip commands would tear pairs | Documented consume-in-submission-order contract |
| JUN-005 | Dead public API | Deleted: `DrawList::button/text_simple/with_base_depth`, `interact_draggable`, `load_default_font` |
| JUN-006 | `float_input` commit logic ×3 | `commit_float_input()` + `draw_float_value()` extracted |
| JUN-007 | Theme bypass (hardcoded colors/paddings) | `TextInputStyle` on `Theme`; paddings routed through theme |
| JUN-008 | Cosmetic re-export + clippy lints in tests | Fixed |

---

## input — Resolved Debt (June 2026 restructure)

### July 16, 2026 — GAP-001 Gamepad backend (gilrs) — CLOSED
The last piece of controller support: `engine_core/src/gamepad_backend.rs`. `GamepadBackend::new_or_disabled()` (AudioManager pattern — headless/CI boxes get a no-op backend, never a crash); `pump(&mut InputHandler)` drains `gilrs.next_event()` right before `process_queued_events()` in the game loop, so gamepad events share frame semantics with window events. Pure, unit-tested translation boundary: `translate_button` (South→A …, gilrs LeftTrigger=bumper→LeftBumper, LeftTrigger2→our LeftTrigger; C/Z/Mode/Unknown dropped), `translate_axis` (sticks 1:1 +Y=up, LeftZ/RightZ→trigger axes), `apply_dead_zone` (0.15, sign-preserving rescale so full deflection still reads ±1.0), `hat_transition_events` (hat-switch pads report DPadX/DPadY as axes → synthesized dpad presses/releases on ±0.5 crossings ONLY — `ButtonTracker::release` is unconditional, so phantom releases would leak just-released edges; backend keeps per-pad prev hat values). Analog triggers arriving as `ButtonChanged(LeftTrigger2/RightTrigger2)` mirror onto the trigger axes. Connect/disconnect → `InputEvent::GamepadConnected/Disconnected`. Needed `libudev-dev` (installed Jul 16 2026) + workspace `gilrs = "0.11"`. `InputEvent` gained `PartialEq` for event-list assertions. 5 backend tests.

### July 16, 2026 — GAP-004 Binding persistence (+ the player-input layer)
Closed as part of the universal controller/2-player feature:
- **GAP-004 resolved**: `GamepadButton/GamepadAxis/AxisDirection/GameAction/InputSource/PlayerSource` derive serde (winit `serde` feature enabled workspace-wide for `KeyCode`/`MouseButton`); on-disk shape is a versioned Vec-of-entries JSON DTO in `engine_core/src/input_settings_io.rs` (`load_or_create` writes defaults when missing → hand-edit rebinding works today; corrupt/wrong-version → warn + defaults, never panics; `save` on `CloseRequested`). Per-game path via `GameConfig::input_settings_path` (all 5 games pass `saves/input_settings.json`).
- **New player-aware layer** (`input/src/player.rs`): `PlayerId` (P1/P2), device-relative `PlayerSource` (`PadButton`/`PadAxis` resolve against a single per-player `pad: Option<u32>` — re-pointing a player never rewrites bindings), `PlayerBindings`, `InputSettings` with `default_two_player()` (P1 = WASD+Space+mouse+pad0, P2 = arrows+Enter+pad1), `is_active/just_activated[_any]`, and `move_x/move_y` (digital −1/0/+1 merged with the stick, clamped; axis-threshold bindings excluded from the digital half so a half-deflected stick reads 0.5). Exposed to games as `GameContext.players`.
- **Axis-as-button**: `GamepadState.prev_axis_values` snapshot in `clear_frame_state` → `axis_just_activated/just_deactivated` edges; `InputSource::GamepadAxis(id, axis, direction)` with `AXIS_ACTIVATION_THRESHOLD = 0.5`; `InputEvent::GamepadConnected/Disconnected`; `GameAction` preset gained pad-0 dpad + left-stick movement.
- **MenuInput** (engine_core) reads every connected pad: dpad/stick-edge up/down (no held-stick repeat), A/Start confirm, B back — all 5 games' menus gained controller nav with zero game edits. BehaviorRunner platformer jump now `GameAction::Action1` (was a raw Space read).
- GAP-001 note: only the gilrs poll itself remains (blocked on `libudev-dev`).

### July 13, 2026 — GPP-L4 (Double Buffer)
- First-frame mouse warp fixed: `MouseState` gained `has_position: bool`; `update_position` only accumulates `frame_delta` once a real previous position exists, so the first move after startup no longer produces a spurious delta against the default `(0,0)` baseline. Four tests that had encoded the phantom jump were updated to the corrected semantics (incl. renamed `test_first_position_update_records_position_without_delta`); accumulation and per-frame-reset assertions preserved.

- **BUG-001 Stale mouse movement delta**: `MouseState` accumulates `frame_delta` across move events, zeroed in `clear_frame_state()`; wheel delta accumulates. Regression tests added.
- **BUG-002 Multi-action bindings leaked on unbind/rebind**: single source of truth `HashMap<A, Vec<InputSource>>`; `unbind_source()` removes from all actions.
- **BUG-003 Incorrect action edge semantics**: `just_activated`/`just_deactivated` compare against reconstructed previous-frame state — strict inactive→active / active→inactive edges.
- **ARCH-003 Engine-owned action enum**: `InputMapping<A>` made generic; `GameAction` survives as optional preset (`with_default_bindings()`); `EditorInputMapping` delegates.
- **ARCH-004 Implicit default bindings in `new()`**: `new()` is empty; defaults opt-in.
- **SRP-002 InputHandler owned the action mapping**: handler is device state + event queue only; action evaluation on `InputMapping`.
- **DRY-001 Triplicated press-state tracking**: shared `ButtonTracker<T>`; press-while-held no longer re-triggers.
- **DRY-002 / DEAD-001/2/3**: `ThreadSafeInputHandler` (158 lines) deleted (no consumer); `init()` + `InputError` deleted; redundant re-export removed.
- **KISS-002**: device-state `update()` renamed `clear_frame_state()`.
- Minor: gamepads auto-register on first event; scroll `PixelDelta` normalized to lines (÷16 px); clippy-clean; docs synced (training.md, README, CLAUDE.md).
- Historical: Jan/Feb 2026 items (DRY-002/003, SRP-001, KISS-001, ARCH-001) resolved then or made moot by the June restructure. ARCH-002 (winit type coupling) remains an intentional documented choice.

---

## games — Resolved Debt

### July 13, 2026 — GPP-03 part 1 (Flyweight/DRY: game-agnostic promotions, promotion #2 under the standing directive)
- Five pieces of pong↔breakout copy-paste moved into the engine (engine owns the mechanism, games keep semantics/tuning):
  - **`engine_core::ChaosTheme`** (new `chaos_theme.rs`, in prelude): per-mode presentation tokens (`bg_color`, `structure_color`, `accent_color`, `banner_text`/`banner_color`, `grid_color`, `particle_count_mult`) with the shared neon palette as `for_mode()` defaults; games override via struct-update syntax (breakout keeps a thin `theme_for()` with its 3 Normal-mode colors; pong uses the defaults directly). Fields de-genre'd: `ball_color`→`accent_color`, `wall_color`→`structure_color`.
  - **`engine_core::grid::step_and_emit_grid(grid, world, lines, dt, debug_colliders)`** — the per-frame grid driver + F1 magenta collider overlay, headless-testable (takes narrow params, not GameContext, since GameContext can't be built without a GPU).
  - **`ecs::sprite_components::set_sprites_visible(world, entities, visible)`** — the hide-gameplay-sprites-in-menus loop; games keep their entity lists and state matching.
  - **`common::{hash_u32, hash_f32}`** (new `hash.rs`) — deterministic frame-count hashing, re-exported via the engine prelude.
  - **`engine_core::game_root!()`** macro + `assets::game_root_from(manifest_dir)` — exe-dir-with-assets → manifest-dir fallback; a macro so the GAME crate's `CARGO_MANIFEST_DIR` is baked in, not the engine's.
- Both games refactored onto every piece (their tests stay green — the sustainability proof); ~120 lines of duplication deleted. Part 2 (genre-flavored spawners/particle semantics/flow skeleton) deferred to game 3's rule-of-three — tracked in `../games/TECH_DEBT.md`.

### July 13, 2026 — GPP-17 (magic numbers, breakout)
- 10 inline tuning literals hoisted from `breakout/src/gameplay.rs` into `constants.rs`: `LAUNCH_ANGLE_SPREAD` (0.6), `BRICK_DAMAGE_COLOR_FACTOR` (0.65), `BRICK_DAMAGE_EMISSIVE_FACTOR` (0.5), `BALL_LOST_BOUNDS_PAD` (60.0), and per-effect grid impulses `GRID_IMPULSE_{PADDLE_HIT,BRICK_DESTROY,BALL_LOST}_{STRENGTH,RADIUS}` (200/70, 260/90, 700/160). Structural literals (hash-recentering 0.5, velocity-epsilon guards) deliberately left in place. Behavior bit-identical; 43 tests green.

---

## audio — Resolved Debt (June 11, 2026 remediation)

### August 27, 2026 — manager.rs 600-line guardrail (issue #12)
- `manager.rs` (689 lines; ~674 pre-VFS + ~15 wasm-port routing) split into
  `manager/{mod,music,tests}.rs` along the documented seams: mod.rs =
  device/output + SFX playback, music.rs = music + volume buses, tests.rs =
  headless tests (shared WAV fixtures serve both seams). Pure move, no API
  or behavior change; all files under 600 lines.

### July 13, 2026 — GPP-L3 (Singleton)
- `SoundHandle::new()`'s process-global `static NEXT_ID: AtomicU32` replaced with an instance-local `next_sound_id: u32` on `AudioManager` (allocated in `load_sound_from_bytes` after decode validation, so failed loads consume no id); `SoundHandle::new()` → `pub(crate) from_id(u32)`. Ids are now manager-local and deterministic (fresh managers start at 1). Regression test: `test_sound_ids_are_manager_local_and_deterministic`.

1. **Per-play full-buffer clone** — bytes are `Arc<[u8]>`, decoder reads `Cursor<Arc<[u8]>>` directly.
2. **Master/SFX volume ignored live sounds** — `ActiveSound` stores `base_volume`; all volume setters re-apply `base * bus * master` to every live sink.
3. **Clamping bypassable** — clamped at point of use via `clamp_volume`/`clamp_speed` (tested).
4. **Dead `PlaybackState` enum** — deleted.
5. **`AudioError::IoError` never constructed** — I/O failures convert via `#[from] io::Error`.
6. **`#[allow(dead_code)]` on `ActiveSound.handle`** — fixed by implementing `stop(handle)`.
7. **`AudioResult<T>` not exported** — re-exported from `lib.rs`.
8. **Music always looped** — `play_music_once(path, volume)` added.
9. **`play`/`unload` took `&SoundHandle`** — now take by value (Copy).
10. **DRY: `load_sound` duplicated decode validation** — delegates to `load_sound_from_bytes`.
11. **DRY: `set_music_volume` re-inlined `update_all_volumes`** — shared helper.
12. **False "crossfade support" doc claims** — removed.
13. **Missing `#[must_use]`** on builders and pure getters — added.

---

## editor — Resolved Debt

### July 13, 2026 — GPP-14 (Command: stable EntityIds across undo/redo)
- `CreateEntityCommand` (redo) and `DeleteEntityCommand` (undo) no longer mint new EntityIds — they resurrect the entity via `world.create_entity_with_id(self.entity)` (ids are never recycled, so the original slot is guaranteed free). The entire staleness class dies at the root: `Selection` and any history commands referencing the id stay valid across undo/redo cycles, with no remapping machinery (the Godot stable-object-identity approach; chosen over the audit's remap fallback once `create_entity_with_id` proved safe).
- **Pre-existing bug found by the new tests and fixed:** a `CreateEntityCommand` pushed via `push_already_executed` never cleared its `captured` flag (its `execute()` is never called first), so the first redo after an undo silently recreated nothing — `undo()` now clears the flag.
- Regression tests: delete→undo resurrects the same id, create→undo→redo resurrects the same id, and a `SetTransformCommand` recorded before a delete still resolves after the delete is undone (previously no-opped against a dead id).

### July 13, 2026 — GPP-13 (Component/DRY: registry-generated editable inspector)
- The ~11 hand-written per-component blocks in `editor_integration/panel_renderer/inspector.rs` are gone. `editor_component_registry!` entries now carry an edit spec — `{ edit edit_sprite => SetSpriteCommand }` for field editing or `{ readonly }` for serde display — and the macro generates `edit_all_components()` (field editors, undo-recorded writeback via `apply_component_edit`, remove [X] buttons, removal commands, in registry order; `registry_edit_block!` helper expands the three block shapes). `inspector.rs` shrank ~357 → 160 lines and is now a thin shell (readonly branch + add-component popup). `apply_component_edit` + `remove_button` moved to `editor/src/component_editors.rs` (exported from editor; editor_integration writeback tests re-pointed to `editor::apply_component_edit`). `/add-component` skill Steps 4–5 rewritten: a new component is one registry line (+ an `edit_*` fn and `Set*Command` only if its fields should be editable). New regression test: `test_edit_all_components_covers_present_components_and_advances_y`.

### July 13, 2026 — GPP-L5 + GPP-L6 (Command)
- **GPP-L5**: `CommandHistory.undo_stack` switched `Vec` → `VecDeque`; `enforce_limit` uses O(1) `pop_front()` instead of O(n) `remove(0)` (push/pop/last sites → push_back/pop_back/back/back_mut; redo_stack stays `Vec` — it never removes from the front). Regression test: `test_max_history_drops_oldest_and_preserves_undo_order`.
- **GPP-L6**: `CommandHistory::undo()`/`redo()` now return `bool` (command actually applied); Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y shortcut handlers and the menu Undo/Redo actions in editor_integration gate `mark_dirty()` on it — a no-op undo/redo on an empty history no longer dirties a clean scene. Regression test: `test_undo_redo_on_empty_history_do_not_mark_dirty`.

### June 2026 remediation + design decisions
- **MAGIC-001**: all 13 slider ranges extracted to `mod ranges` constants; `EditableInspector::f32/vec2` take `RangeInclusive<f32>`.
- **MAGIC-002**: widget-ID multipliers promoted to `COMPONENT_ID_STRIDE`/`FIELD_ID_STRIDE` with documented collision limits.
- **MAGIC-003**: input widths/gaps/offsets are `EditableFieldStyle` fields (defaults preserve previous values).
- **SRP-002**: `MenuBar.render()` is a 4-line orchestrator over `layout_titles()` (pure geometry, unit-tested), `render_title_bar()`, `apply_toggle()`, `render_open_dropdown()`.
- **ARCH-001**: gizmo `render_translate()` split into `render_axis_handle()` + `begin_drag_if()`; colors from `GizmoPalette`.
- **ARCH-002**: five `*EditResult` structs (26-33 fields each) replaced by generic `ComponentEdit<T> { new_value, field_hint }`; the five `Set*Command` impls collapsed into `impl_set_component_command!`.
- **Design decisions of record:** (1) `stored_component.rs` holds the single `editor_component_registry!` invocation — add new editor-visible components there, one line; (2) `context.rs` state/delegation split evaluated and REJECTED (cohesive accessors; tests moved to `context/tests.rs` instead); (3) theme-driven colors — widgets take `&EditorTheme` or themed style structs, `Default` impls keep old literals.

### February 2026
- **DRY-001**: `has_exact_keys(map, keys)` helper; `is_vec_like()` 25→3 lines.
- **SRP-001/DRY-002**: `check_action_with()` generic helper for the three action-check methods.
- **DRY-003**: dropdown constants promoted to module level.
- **DRY-004**: `centered_handle_rect()` helper (5 occurrences).

---

## editor_integration — Resolved Debt (June 2026)

- **Duplicate `ComponentKind` enum + dispatch matches**: entity_ops.rs's own `ComponentKind`/`ComponentCategory`/dispatch helpers and panel_renderer's `to_command_kind()` converter all deleted. `editor::ComponentKind` (registry macro) is the single type. Adding a component type is one line in the editor crate's registry.
- **Files over 600 lines**: `editor_game.rs` (1108) → `editor_game/` directory (mod/menu_actions/scene_io/shortcuts/viewport_interaction/tests); `panel_renderer.rs` (923) → `panel_renderer/` (mod/inspector/tests); `entity_ops.rs` 901 → 563.
- **Duplicated inspector writeback (5×)**: one generic `apply_component_edit()` (world write + `try_merge_or_push`).
- **Duplicated Delete/Duplicate logic**: single `delete_selected_entities()`/`duplicate_selected_entities()` called from menu and shortcuts.
- **Magic numbers**: `src/constants.rs` holds `DEFAULT_SCENE_PATH` (was hardcoded 5×), `MIN_EDITOR_WINDOW_WIDTH/HEIGHT`, `MIN_ENTITY_SCALE`, `DUPLICATE_OFFSET`.
- **Open Scene failures only logged**: `load_scene_with_feedback()` surfaces load errors on the status bar.

## Sprint 5 #43 — Registry collapse: ARCH-006 (widened) + ecs GPP-16 CLOSED (Aug 28 2026)

- **ecs GPP-16**: `global_registry()` is now `OnceLock<RwLock<ComponentRegistry>>` with
  `ecs::register_components(f)` callable at any time (games in `main()`, physics via
  `engine_core::component_registration::register_engine_components()` from `run_game`
  AND `SceneLoader::instantiate`). Same-name/different-TypeId registration is a hard
  panic (silent scene corruption otherwise); identical re-registration is a no-op;
  lock poisoning recovers via `into_inner`; a thread-local guard panics loudly on
  re-entrant access instead of deadlocking.
- **ARCH-006 (widened per the Aug 27 roadmap note)**: the registry entries carry
  monomorphized fn pointers (create/insert/extract/remove/has/default) — **no
  `World::add_boxed` was needed** (Component is blanket-impl'd over Any+Send+Sync, so
  registration-time generics close over the ordinary typed World API).
  `ComponentData::Dynamic { type, data }` (flatten dropped — plain JSON map through
  RON, spike-tested) now actually LOADS (the tracked TODO in scene_loader is gone;
  unknown names are a hard load error — fail loud, never drop authored data) and
  `extract_components` emits every registry-persisted non-concrete type as Dynamic,
  name-sorted — **which ended the AudioSource/AudioListener silent drop on save**
  (they were editable but never persisted; no shipped scene contained them, so zero
  compat exposure). Transient types (`PlaySoundEffect`, now `register_transient`) are
  editable but never persisted. The editor gained `StoredComponent::Dynamic` +
  `stored_component/dynamic.rs`: snapshot/clipboard/undo, the add-component popup's
  "Game" section, read-only inspection, and command-API `set`/`add`/`remove`/`describe`
  all reach game-registered components by name (`Add/RemoveDynamicComponentCommand`;
  name-bearing APIs relaxed to `Cow<'static, str>`).
- Deliberately NOT registry-ized: Sprite/SpriteAnimation/RigidBody/Collider/Tilemap
  wire arms (load-time resolve logic — texture refs, sidecars, shape enums) and
  `Behavior` (frozen at 8 variants per audit §6.7(5); new logic goes through the
  Scripts seam). Dynamic components must not store raw `EntityId`s (documented).
- The standalone editor binary cannot see game-registered types; such scenes refuse
  to load there with a clear error (edit from the game's binary instead).

## Physics System Pass-Through and Velocity Semantics (recorded Sep 2026; the `apply_impulse` removal itself predates this entry and was moved here from the `physics_system` module doc)

`PhysicsSystem` provides pass-through methods (`set_velocity`, `take_collision_events`, `destroy_entity`, `reset_body`) delegating to `PhysicsWorld` for ergonomics. `set_velocity` is the single, universal launch/movement API, buffering operations on bodies spawned in the current frame until they are synchronized with Rapier. The legacy `apply_impulse` pass-through was removed because all call sites across the workspace intended "start this body at velocity V" rather than a mass-aware momentum delta, creating a footgun where impulse silently no-oped on same-frame spawns. `PhysicsWorld::apply_impulse` remains available for cases requiring mass-aware momentum changes on live bodies.

## Cleanup effort (Sep 2026)

- **Plans directory retired (Sep 2026, batch 2):** deleted `docs/plans/` (three shipped Jan–Feb 2026 plans for scene saving and a DRY/SRP pass); history lives in git.

