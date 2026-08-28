# Insiculous 2D — Project Roadmap

## Vision: Deion the Insiculous

**The world of Deion is the project's identity.** Deion the Insiculous — a
SNES-styled hero, a ball of DEIONized water with an icicle mohawk — lives in a
food-coded world. Every game we ship is a window into that world: unique
"Deion Style" pixel-art characters and assets, not stand-in shapes. The
geometry-wars neon look that carried games 1–6 is demoted to an FX/accent
layer; SNES-era sprite art becomes the primary style.

**The 20 Games Challenge is the vehicle**, not the destination: arcade classics
teach the engine and expose gaps, building toward original Deion titles. The
challenge is **paused at game 7 (Tetris)** while the Deion Pivot (Phases E–I
below) lands; it resumes with the new asset style from day one.

**Studio premise (Jesse, Aug 19 2026): Be Insiculous is an AI dev studio**, not
a game dev studio that happens to use AI. AI-assisted development is the primary
workflow and part of the product story — the studio umbrella also covers Mily's
ongoing non-game AI work. Consequences: **free web releases showcase the AI
workflow, AI art included**; **marketplace releases (Steam, iOS, Android —
anything charging money) ship hand-drawn art only** (see the tiered AI-art rule
in the settled decisions below). The first marketplace release target is
**Insiculous Arcade** (Phase J): the non-original challenge games compiled into
one Deion-skinned package.

**Engine Status (July 2026):** Core systems complete. 1253 tests passing
(100%), 0 ignored — every doc example compiles and runs (window/GPU-bound ones
are `no_run`). Full DRY/SRP/KISS audit + Game Programming Patterns audit closed
(history in `log_archive.md`); see `TECH_DEBT.md` for the live rollup.

**The Deion Pivot (adversarially reviewed Jul 28 2026):** plan drafted, reviewed
by kimi (15 findings adjudicated — 13 accepted, 2 rebutted), settled. Phases E–I
below are the reworked roadmap.

---

## Current Engine Capabilities

| System | Status | Notes |
|--------|--------|-------|
| ECS | ✅ Complete | HashMap-based per-type storage, type-safe queries, hierarchy |
| Physics | ✅ Complete | Rapier2d, platformer + top-down presets, collision events (bus + `take_collision_events()`) |
| Rendering | ✅ Complete | WGPU 28, instanced sprites, batching |
| Sprite Animation | ✅ Complete (E3, Jul 30 2026) | Named clips over `SheetGrid`; `SpriteAnimationSystem` writes `Sprite.tex_region` every frame from `frame_tail.rs` (time-scaled — pause freezes it); render path passes the region through |
| Pixel-Art Pipeline | 🔧 In progress (Phase E) | E1 ☑ `TextureFilter` knob, E2 ☑ `common::SheetGrid`, E6 ☑ `atlas.rs` deleted, E3 ☑ named-clip animation, E4 ☑ `.sheet.ron` loader + `load_sprite_sheet` (schema freeze REACHED — all Jul 30 2026). Remaining: `#solid`/`#rgba` scene round-trip (E5), alpha-cutoff (E7) |
| Audio | ✅ Complete | Rodio backend, SFX/music/master buses (spatial audio components are editor-only data — no runtime system) |
| Input | ✅ Complete | Keyboard/mouse/gamepads (gilrs backend), `InputMapping<A>`, player-aware `InputSettings` (`ctx.players`, JSON-persisted bindings) |
| Local 2-Player | ✅ Complete | All games 2-player (Jul 2026) |
| Pause + Menu Chrome | ✅ Complete | Engine `PauseMenu` + `MenuPanel` window chrome — all games |
| UI | ✅ Complete | Immediate-mode, text editing, data-driven UiLabel/UiPanel/UiButton components |
| Localization | ✅ Complete | `ctx.strings`, RON locale files, per-locale fonts (Pong + Frogger localized) |
| Scene Serialization | ✅ Complete | RON format, prefabs, hierarchy; `tex_region`/`visible` + `#solid:RRGGBB` round-trip (E5, Jul 30 2026 — only the F3-gated `#rgba` save error remains) |
| Behaviors | ✅ Complete | `PlayerPlatformer`, `PlayerTopDown`, `Patrol`, `FollowEntity`, `FollowTagged`, `Collectible`, `CameraFollow` |
| Scene Editor | ✅ Complete | Entity CRUD, inspector, gizmos, play/pause/stop, undo/redo, save/load, asset browser + drag-to-assign |
| Standalone Editor | ✅ Complete | `cargo run --bin editor -- /path/to/project` |
| Tilemap | ✅ Complete | `Tilemap` component, batched through the sprite pipeline (Frogger is first consumer) |
| Web/WASM Export | 🔧 In progress (Phase H) | H1 spike ☑ PASSED Jul 30 2026 (`coordination/H1_SPIKE.md`): 14/14 deps compile for wasm32, **audio decision: stay on rodio** (works in live browser), WebGPU demo screenshot-verified, gilrs has a real web backend (no no-op gating needed). Remaining (H2–H6, known refactor): `pollster::block_on` init → async, `thread::sleep` throttle, `Instant`/`SystemTime` → `web-time`, `std::fs` sites → bytes-primary API + localStorage persistence |

---

## Phase A: Games 1–5 — ☑ COMPLETE (July 2026)

Pong, Breakout, Space Invaders, Snake, Asteroids — details in `log_archive.md`.
Each lives in `../games/<name>/` as a standalone cargo project.

## Phase B: Engine Gap Work — ☑ COMPLETE (July 2026)

`CameraFollow`, `Lifetime`, `Tilemap` — details in `log_archive.md`.

## Game 6: Frogger — ☑ COMPLETE (July 2026)

First Tilemap consumer, 43 tests — details in `log_archive.md`.

---

# The Deion Pivot — Phases E–I

Settled decisions (Jul 28 2026, adversarial review round 1; artifacts in
`review/` during the session, verdict adjudicated with Jesse):
- **Art source: mix** — Jesse hand-draws hero assets (Aseprite → PNG); simple
  tiles/props are code-generated **offline into PNGs** (never runtime rgba).
- **All 6 games get full Deion-world theming**; ChaosTheme neon becomes the
  FX/accent layer.
- **Deployment web-first in the CURRENT look (Jesse, Aug 19 2026):** the six
  games ship to the website as they are today (neon look, AI stand-ins where
  they exist) — **Phase H + I1/I2 are the open front, ahead of F/G**. Deion
  re-skins roll out to the site as updates; **Insiculous Arcade (Phase J)** is
  the marketplace milestone. Free itch.io follows the site; Steam/iOS/Android
  wait for Phase J.
- **Tiered AI-art rule (Jesse, Aug 19 2026 — supersedes "AI art never ships"):**
  the money line is the boundary. AI-generated assets **may ship in free
  releases** (the studio website, free itch.io games) as part of the AI-workflow
  showcase; they **never ship in paid/marketplace releases** — the purge gate
  (`check_no_ai_assets.sh`) applies to paid publish paths only. Quarantine
  mechanics (`ai/` dir + `ai_` prefix + inspection gate) are unchanged and are
  what keep the paid-tier purge greppable. SSOT: DEION_STYLE.md §6.
- **Web assets fetch-by-default** (boot-phase manifest fetch into a bytes map;
  loaders stay sync); **WebGPU-only at launch**; **games stay standalone**.

## Phase E — Asset Pipeline (engine)

Make pixel art actually work, end-to-end, headless-tested.

| # | Task | Key decisions |
|---|------|---------------|
| E1 ☑ | `TextureFilter` knob | DONE Jul 30 2026. Config default (`GameConfig::with_texture_filter` → `AssetConfig.default_filter`) + per-call `load_texture_filtered`; `Linear` default for plain loads (back-compat) |
| E2 ☑ | `common::SheetGrid` | DONE Jul 30 2026. Tilemap delegates (behavior-identical, test-locked incl. out-of-range passthrough); `uv_rect_checked` for E3/E4 consumers. E4 note: `Deserialize` needs explicit `cell_uv` on the wire |
| E3 ☑ | `SpriteAnimation` rework | DONE Jul 30 2026 (plan-v4, 3 adversarial rounds). Named clips over `SheetGrid` (`play`/`ensure_playing`/`resume`, arithmetic frame advance, fps/empty-clip guards); `SpriteAnimationSystem` driven from `frame_tail.rs` on the time-scaled delta; render path passes `tex_region`; scene chain uses `GridData` + shared `ClipData` DTOs, `autoplay` written only while playing; sidecar-as-SSOT on reload via `TextureResolver::sheet_for`; editor freezes engine time outside Play (`editor_time_scale`) |
| E4 ☑ | `load_sprite_sheet()` + `.sheet.ron` | DONE Jul 30 2026. `SheetFile` v1 schema in `sheet_file.rs` (pixel `cell`, filter defaults Nearest, looping defaults true, fail-loud validation incl. inf/NaN fps + index-past-grid); validate-before-GPU ordering (no handle leak); `SidecarCache` (one read per path per load, cleared at every scene load); scene texture refs take the sidecar's filter automatically |
| E5 🔧 | Scene serialization fixes | **Round-trip half DONE Jul 30 2026** (adversarially reviewed): `create_solid_color` records canonical `#solid:RRGGBB` (`solid_color_path` in `texture_ref.rs`, alpha byte when translucent); `tex_region` + `visible` on the `Sprite` wire with named serde defaults (old scenes load unchanged; autoplaying clips still overwrite the saved region snapshot on load — test-locked). Remaining: `#rgba` becomes a per-sprite save-time error naming the entity — **enforced only after F3 migrates Frogger's tileset** |
| E6 ☑ | Delete `renderer/src/atlas.rs` | DONE Jul 30 2026 (incl. orphaned `TextureError::TextureCreationError`) |
| E7 | Sprite-shader alpha-cutoff | Configurable threshold, conservative default; closes the renderer TECH_DEBT alpha/depth item |
| E8 | Inspector wiring | Via `/add-component` only; [Animation] timeline tab stays backlog |
| E9 ☑ | Docs | DONE Jul 30 2026 with E3/E4: training.md Sprite Sheet Pattern section + directory map, ecs/engine_core CLAUDE.md, root CLAUDE.md SSOT rows (`.sheet.ron` schema, `ClipData` wire format) |

**Checkpoint: E2 + E4 merged = schema freeze — REACHED Jul 30 2026.** Asset
production (F2 onward) is unblocked.

## Phase F — Deion Style Guide + Asset Production

**Parallel art track since Aug 19 2026** — no longer the front (that's Phase
H + I1/I2, per the web-first-in-current-look decision above). F/G continue
alongside web work and land on the site as updates.

| # | Task | Notes |
|---|------|-------|
| F1 | `../games/deion_assets/DEION_STYLE.md` + castings proposal | World bible (Deion, food-coded world), palette, metrics, per-game castings table, naming, export rules (**no anti-aliased edges** in pixel exports), clips-are-the-API convention. May start before schema freeze |
| F2 | `../games/deion_assets/` + sync script | Canonical asset source; sync copies into each game's `assets/sprites/`; **`--check` hash-compare mode** wired into build + definition of done. No symlinks |
| F3 | `scripts/gen_tiles` offline generator | image-crate bin producing PNGs; first consumers: Frogger lanes (migrates its in-code rgba tileset — unblocks E5's `#rgba` error), Breakout bricks |
| F4 | Placeholder sheets for all 6 games | Agent-made: correct cell size, blocked-out colors, **final clip names** — Phase G never blocks on art |
| F5 | First animated Deion on screen | Validation milestone; needs Phase E complete |

**Metrics:** 16px base cell, nearest filter, 5× integer scale to
`RENDER_UNIT = 80` — one art cell = one world unit = one collider unit.

**Split:** Jesse draws hero sheets (idle/walk/jump/hurt), per-game variants,
key characters, palette sign-off. Agents do everything else.

**AI baseline stand-ins (Aug 2 2026, pixellab trial — 37/40 generations
used, treat the remaining 3 as reserve):** full cast now has restyled
side-view baselines AND 8-frame walk-cycle sheets + GIF previews
(deion_assets commit e2ea5be; Funguy de-armed, Maxwell got his face on
attempt 3, Cubert is side-profile with Deion-matching icicle mohawk — canon
in DEION_STYLE.md). Walk frames carry the known mid-sequence drift —
curation to 4–6 keepers per clip is the next step (Jesse or agent, in
Aseprite). full-cast baselines quarantined in `../games/deion_assets/ai/`
(`ai_<name>_64_side.png`, 8 PNGs: Deion, Cubert, Bananakin, ham **Captham
Michael**, angry cream-pie Master Pi, prune Aleister Prunely, mushroom Funguy,
Dr. Maxwell — 64×64 single side-view iconic poses, shape-with-a-face style,
food identities settled by Jesse same day). Policy (tiered since Aug 19 2026):
AI art may ship in FREE releases (website, free itch.io) but never in
paid/marketplace releases —
`../games/deion_assets/scripts/check_no_ai_assets.sh <assets-dir>` must pass on
any paid release's asset tree (DEION_STYLE.md §6).
- **Tooling lesson (paid for once):** pixellab `create_character` forces a
  humanoid/quadruped SKELETON — it produced 5 little humans, all rejected by
  Jesse (first batch, deleted). For this project's geometric shape-with-a-face
  cast use **`create_image_pixflux`** (freeform, 1 generation, `view: side`,
  `no_background`, ~8 concurrent jobs max) — landed on-style first try.
- **Style transfer (validated Aug 2):** img2img (`init_image`, strength 160)
  re-renders an existing concept into Jesse's flat hand-drawn style while
  keeping identity — whole cast restyled this way (deion_assets commits
  0820e4f → 09bd21d hold before/after). Prompts for the batch went through
  the deion_assets **prompt-mode adversarial review** (kimi, 7 findings, all
  accepted — the mode's first live run).
- **Animation workflow (validated Aug 2):** `animate_image` on
  Jesse's HAND-DRAWN sprite (loose PNG, no rig; 64×64×8 frames = 1 generation)
  preserves his style — palette/mohawk/face carry through because frames derive
  from his pixels. Results in `ai/`: `ai_deion_walk_sheet_64.png` (9f) +
  `ai_deion_idle_sheet_64.png` (5f) + GIF previews + loose frames. Caveat:
  mid-sequence frames drift off-model (faces mutate) — workflow is generate 8,
  curate the best 4–6, hand-fix stragglers in Aseprite; still far faster than
  animating from scratch. `seed` param allows re-rolls.
- Remaining tweaks: single poses only (no walk/attack frames, no `.sheet.ron`
  sidecars — F4 placeholder sheets remain the real deliverable); 64×64 is off
  the 16px grid (don't bake into colliders/cells); palette is AI-picked, not
  the §4 ramps (conformance happens in Jesse's hand-drawn replacements);
  top-down variants not generated yet (make them per-game when a top-down
  game needs the character — iconic look is side-view first per Jesse).

## Phase G — Re-skin Games 1–6

**Parallel art track since Aug 19 2026** — re-skins no longer gate anything
going live on the web; each finished re-skin ships to the site as an update.

Order (each independently shippable): **Pong → Frogger → Breakout → Snake →
Space Invaders → Asteroids.** Pong validates the pipeline (smallest, has PNGs);
Frogger validates tile sheets; Breakout validates the scene-RON fixes;
Asteroids last (rotation/animation heavy).

**Per-game identities (settled with Jesse, Aug 9 2026 — detail lives in each
game's README "Deion Pivot" section; castings SSOT is DEION_STYLE.md §5):**

| Game | New identity |
|------|-------------|
| Pong | **Tong** — paddles are living tong characters (rounded grip = rounded paddle surface); Deion stays the ball. Tong art shared with Breakout |
| Frogger | **Chicken Coop** — chicken player(s) crossing food traffic; co-op = 2 chickens (the pun); home slots = coop nest boxes |
| Breakout | **The Food Pyramid** — level select is a 1992-USDA-style pyramid: Fruits & Veggies base (L1/L2) → Bread (L3) → Dairy (L4) + Meats (L5) side by side → Sweets & Fats finale (L6). Gates: L1+L2 → L3; L3 → choose L4/L5 → L6. Per-level brick themes + power-up re-themes; paddle = tong character. Level-select screen + unlock persistence are new scope — **design TBD at re-skin time** (incl. the L4/L5 both-or-either gate) |
| Snake | **Hot Dog!** — the snake is a wiener dog (working name "Frank", Jesse signs off); body growth introduces Tilemap logic (2nd engine Tilemap consumer); angry-meatball hazard |
| Space Invaders | **Burger Invaders** — Deion fires mohawk icicles up; levels build a burger bottom-up (patty → +cheese → +lettuce → …), enemy ranks match the layer (buns = In-Bread Yokels, patty = angry meatballs, cheese = wedge guys). Per-level enemy rosters are new scope vs today's single formation — design TBD at re-skin time |
| Asteroids | **Meatieroids** — asteroids are roided-out flexing meatballs (3 sizes, flex-burst splits); Flying Funk / icicle shots / Maxwell UFO kept |

Cross-game: the **angry meatball** family recurs (Meatieroids rocks, Hot Dog!
hazard, Burger Invaders patty rank). New characters needing Jesse's design +
castings sign-off: tong characters, the chicken, the wiener dog, angry
meatballs, cheese-wedge guys.

Per game:
- Gameplay entities get sheet sprites; ChaosTheme/bloom/grid kept as accent;
  wireframes debug-only.
- **Collider/velocity audit** against new sprite dimensions (collider overlay,
  C key) — physics ignores `Transform2D.scale`; 1:1 cell/unit is a target for
  new art, never assumed for existing tuning.
- Audit headless tests asserting on `#white`/color tints.
- `deion_assets` sync `--check` in the definition of done.
- README + castings note.

Cross-cutting: **G0** update `/new-game` skill ("Neon look" → "Deion look") ·
**G7** rule-of-2+ promotion sweep after the 3rd re-skin.

## Phase H — WASM Port — **THE OPEN FRONT (Aug 19 2026)**

Web deployment in the current look is the top priority; H2–H8 are
parallelizable across crates now and feed straight into H9 + I1/I2.

**H1 spike ☑ DONE Jul 30 2026** (`coordination/H1_SPIKE.md`, demo in
`../spikes/h1_wasm/`): browser demo renders a fetched texture (wgpu 28 +
winit 0.30, WebGPU, screenshot-verified); 14/14 dependency pass/fail table
with exact feature sets; **audio decision: STAY ON RODIO** (OutputStream +
symphonia decode confirmed in a live browser via an AudioManager-surface
mirror; kira / web-sys shim not needed). Listen test PASSED Jul 30 2026 —
Jesse confirmed all 4 audio checks by ear; the rodio decision is FINAL. Corrections to this phase's
assumptions: gilrs ships a web backend (no no-op gating needed); wgpu builds
for wasm with unchanged Cargo.toml. Forced H2 API change: `load_sound` /
`play_music` become bytes-primary (path versions native-only convenience).

| # | Task | Key decisions |
|---|------|---------------|
| H2 ☑ | `web-time` swap — **DONE Aug 19 2026** | `common::clock` re-exports `std::time` natively / `web_time` on wasm; swapped in game_loop_manager, timing, lifecycle, achievements |
| H3 ☑ | Redraw-driven loop — **DONE Aug 19 2026, revised** | Adversarial review (F2) killed "one model native+web": on some compositors (Wayland/macOS occlusion) `RedrawRequested` stops for hidden windows, freezing native games. Shipped as a cfg-split instead: **native keeps the `about_to_wait` driver byte-for-byte**; wasm drives frames from `RedrawRequested` → `request_redraw` (rAF). Shared `drive_frame()` in `game/app_handler.rs`; `thread::sleep` throttle cfg'd out on wasm; `target_fps` documented native-only |
| H4 ☑ | Async renderer init — **DONE Aug 19 2026** | `RenderManager::init` (native, pollster at the outer edge) + shared `complete_init`; wasm `spawn_local` fills `pending_renderer`, drained by the frame driver (`game/web.rs`). Init surface clamps 0→1px (adopted canvas reports 0 pre-layout); real size pushed from canvas attrs after adoption |
| H5 ◐ | Asset manifest + fetch boot phase — **CORE DONE Aug 19 2026** | `common::vfs` (native = std::fs passthrough, wasm = in-memory map; canonical key = `{asset_base}/{entry}`); `engine_core::web::preload_assets` fetches `manifest.json` + all entries pre-`GameRunner`, so existing sync loaders work unchanged. Locale dir-scan solved by VFS prefix-scan (no manifest list needed). Converted: textures, fonts, locales, scenes, sheet sidecars, **and audio** (`load_sound`/`start_music` read via vfs — code-review F1; the bytes-primary API redesign is no longer forced). `include_bytes!` bootstrap: **decided WON'T-DO Aug 27 2026 (#9 closed)** — the loading screen is site-owned DOM text consuming zero engine assets, and fonts load inside `Game::init`, strictly after `preload_assets`; no pre-VFS draw window exists |
| H6 ☑ | `save_store` persistence seam — **DONE Aug 27 2026 (#6/#17)** | `engine_core::save_store` (vfs-pattern free functions, not a trait — backend fixed per target): native = atomic JSON file writes, wasm = localStorage under the `beinsiculous.games.<slug>.*` keys + `insiculous-save` CustomEvent per persist. Achievements + input settings routed through it; new `Scores` facility (`ctx.scores`, top-10 per mode); achievements/scores merge-on-save (multi-tab). Contract: `docs/WEB_SAVES.md`. IndexedDB rejected (KB-scale blobs) |
| H7 ☑ | Audio backend — **DONE Aug 27 2026 (#8)** | ☑ DECIDED (H1 spike): stay on rodio. Gesture-gated upgrade shipped: `AudioManager::enable_output()` (preserves loaded handles/volumes; pending-music memory replays a pre-gesture `play_music`) + first-activation-gesture hook in `game/app_handler.rs`→`game/web.rs` (keydown/mousedown/touchstart, key auto-repeat excluded, retries capped at 5 — `try_default()` Ok does NOT prove the context is running, so it is never used as a startup health check). Demo consumer: pong's paddle beep. Known limitation: no AudioContext resume after tab-switch suspension (audio TECH_DEBT) |
| H8 ☑ | wasm CI guard — **DONE Aug 27 2026 (#7)** | The whole workspace already compiled for wasm32, so the guard landed at workspace scope in one step: `scripts/check_wasm.sh` (check + clippy `-D warnings`, deliberately no `--all-targets` — dev-deps don't build on wasm) run verbatim by `.github/workflows/wasm-check.yml` (push/PR/dispatch, dtolnay toolchain + rust-cache). The 3 `arc_with_non_send_sync` warnings: 2 fixed for real (pipelines stored `Arc::new(device.clone())` around an already-refcounted `Device` — now plain `Device`), 1 narrow line-level allow at `renderer.rs` device Arc (public API spanning crates; web is single-threaded) |
| H9 ☑ | Port all 6 games — **ALL 6 DONE Aug 19 2026** | `scripts/build_wasm.sh <game_dir> <slug> [--serve]` (generic: CLI-version assert with remediation, manifest gen, guarded local test page); `[profile.wasm-release]` opt-level="s" + lto in the game's Cargo.toml; pong split into lib.rs + thin main.rs + `web_entry.rs` (`#[wasm_bindgen(start)]` → preload → `run_game`). All six ported same-day via 5 parallel agents on the pong recipe: pong 2.5, snake 1.4, frogger 1.4 (720x768, localized), breakout 3.0 (scene RON levels load through the VFS — verified by designed-layout bricks in-browser), invaders 2.1, asteroids 2.1 MiB. Every game browser-verified on WebGPU. Breakout threads `ctx.assets.base_path()` into `level_scene_path` (owned copy before the mutable assets borrow) |
| H10 | Web editor — **the north-star stretch goal ("super-ultra-win")** | The editor runs on wasm32 like the games (audit §8, adopted Aug 27 2026): scene authoring, component/param tuning, script attachment, and play/pause all work in-browser; scenes via OPFS / import-export `.ron`; "Open in IDE" and Build & Reload stay **native-only by design** (no local FS / cargo on web — remote build service explicitly deferred). The §9 command layer is designed transport-ready (stdin/stdout now, WebSocket later) so the web editor's remote-control channel falls out for free. Standing constraint on all editor work: stay wasm32-compatible |

WebGPU-only at launch; WebGL2 fallback revisited at the post-I2 launch review.
gilrs needed NO gating (H1 finding confirmed — compiles unchanged). Lessons
paid for during the pong port (Aug 19 2026), for the next 5 ports:
- **winit never inserts its canvas into the DOM.** A detached canvas renders
  silently into nothing — every pass valid, page black, zero errors. The fix
  lives in `renderer::insert_canvas_into_dom` (swaps winit's canvas in place
  of the page's `#game-canvas` placeholder, copying id/size/a11y attrs),
  called from `WindowManager::create`. Adopting an existing canvas via
  `with_canvas` was abandoned — the winit-owns-the-canvas path is what the
  spike verified. Don't re-litigate.
- WebGPU canvases expose **no sRGB surface formats**; the bloom composite
  shader gamma-encodes (`inv_gamma` in `BloomParams`) when the swapchain
  isn't sRGB, so web brightness matches native.
- Surfaces must never configure at 0×0 (validation error): init clamps to
  1×1 and the adopted size is pushed through `resize()` after renderer
  adoption (`game/web.rs`).
- Headless-shell/swiftshader Chromium runs the full engine but composites
  nothing visible — use Playwright's **full Chromium, headed** for pixel
  verification (screenshots are ground truth; canvas `drawImage` readback of
  a WebGPU canvas lies). Headless Firefox has no `navigator.gpu` at all.
- Pong's whole bundle is 2.5 MiB wasm — the 25 MiB Cloudflare budget is a
  non-issue at current scope; `wasm-opt` still uninstalled and unneeded.

## Phase I — Deployment

The site already exists and deploys: `../insiculous_web/` is an **Astro 5 site
shipping as a Cloudflare Workers static-assets Worker** (NOT GitHub Pages —
corrected Aug 19 2026; wrangler `[assets]` on `dist/`, every push to `main`
deploys via Workers Builds). **The repo belongs to Mily's GitHub account
(`milyramic`) as of Aug 2026**; Cloudflare relink + final URL are tracked in the
handoff notes below. The site has the WASM drop-in convention ready: put builds
at `public/games/<slug>/v1/{game.js, game_bg.wasm}`, set frontmatter
`wasm: '/games/<slug>/v1/game.js'`, and flip that game's `status:` to
`playable` — **the flip happens per game, only when its H9 build actually
lands** (a `postbuild-check.mjs` enforces existing paths + Cloudflare's 25 MiB
per-file limit). All six game pages exist at `status: alpha` today.

| # | Task | Notes |
|---|------|-------|
| I1 ☑ | First game live on the site — **LIVE Aug 19 2026** | Pong build dropped at `public/games/pong/v1/` (v1 = immutable once live; rebuilds bump v2), `pong.md` flipped to `playable` + `wasm:`, GameEmbed activated (WebGPU gate BEFORE the module import, `#game-canvas` placeholder the engine swaps into, controls note), `npm run verify` fully green (data + check + build + postbuild + a11y 43 pages). Merged jesse→main, Actions deployed, **verified rendering live at beinsiculous.com/games/pong/** (screenshot-checked; wasm served as application/wasm). Launch-window hotfix same-day: 1x1-canvas resize-observer deadlock (engine sizes the surface from GameConfig; canvas readback removed) — deliberate one-time same-v1 exception, immutability applies from here on |
| I2 ☑ | Remaining games on the site — **ALL 6 LIVE Aug 19 2026** | snake/frogger/breakout/invaders/asteroids bundles synced to `public/games/<slug>/v1/`, pages flipped playable (frogger with width/height 720x768 frontmatter — schema + [slug].astro pass-through added), copy updated, `npm run verify` green (43 pages). Merged jesse→main (over Mily's Astro-7/RSS upgrade), deployed, and every game live-verified in-browser at beinsiculous.com (pages 200, wasm exact sizes, WebGPU canvases boot, 0 page errors). First real devlog post shipped in the same deploy |
| I3 | Free itch.io via butler | `scripts/publish_itch.sh`, HTML5 project per game from the same dist zips; page copy/screenshots Jesse-side. **Free tier = AI art okay** (same builds as the site); a *paid* itch.io release would count as marketplace and take the I0 gate |
| I0 | AI-asset purge gate — **paid/marketplace paths only** (retiered Aug 19 2026) | Any paid publish path (Phase J store builds, paid itch.io) runs `../games/deion_assets/scripts/check_no_ai_assets.sh` against the dist's assets and FAILS on any `ai_*` file. Free web deploys (site, free itch.io) skip the purge by design — AI stand-ins there showcase the workflow (DEION_STYLE.md §6) |
| I4 | `docs/STEAM_CHECKLIST.md` | Doc only — Steam = native packaging + Steamworks, deferred as Phase J groundwork (Steam doesn't host HTML5) |

**insiculous_web handoff (Aug 19 2026):** milestone (a) ☑ local clone's origin
→ `https://github.com/milyramic/insiculous_web.git` (repo transferred — old URL
redirects to the same head). Milestone (b) is largely done Mily-side already:
deploys run via GitHub Actions on every push to `main` (`wrangler deploy` with
`CLOUDFLARE_API_TOKEN`/`ACCOUNT_ID` repo secrets; the dashboard's Workers
Builds git integration is deliberately disconnected), and the production URL is
the custom domain **`beinsiculous.com`** (wrangler route + astro `site`) —
remaining manual step: the beinsiculous.com zone must exist in the target
Cloudflare account with the registrar pointing at Cloudflare's nameservers
(deploys succeed on workers.dev until then). The site also now hosts Mily's
FortKnight/ForkKnife app alongside the games/engine/devlog surfaces — the
AI-dev-studio umbrella in practice.

## Phase J — Insiculous Arcade (marketplace compilation) — OUTLINE ONLY

First marketplace release (Jesse, Aug 19 2026): **all non-original
20-games-challenge games compiled into one Deion-skinned package** for paid
storefronts (Steam, iOS, Android). Do not plan in detail yet — this section
exists so the target is named and its gates are on record.

Hard gates (all must hold before any store submission):
- Phase G complete — every included game fully Deion re-skinned.
- Hand-drawn art swap complete — no AI stand-ins anywhere in the package;
  `check_no_ai_assets.sh` passes on the shipping asset tree (I0 gate).
- Phase H/I stable — the games have shipped and soaked on the free web tier.

Open questions (deliberately unanswered until Phase J planning starts):
launcher/wrapper design (one binary hosting six games vs a hub scene),
per-store native packaging (Steamworks; iOS/Android toolchains are entirely
new scope), input/UX for storefront cert requirements, pricing.

Note: "arcade scaffolding" in engine_core docs (`MenuInput`, `spawn_background`,
etc.) is unrelated engine vocabulary that predates this product name — leave it.

## Phase K — Conductor: adaptive MIDI music (parallel engine track, Aug 28 2026)

Jesse's founding audio wish, sharpened (Aug 28): music ships as a MIDI file
synthesized at runtime, and trigger notes **replace sound effects** — every
player/enemy/world action fires its own instrument track from the song's
score, so the player's playstyle IS the arrangement: music unique to each run
that always fits, because every producible sound was authored into the score.
The level song is the band that never stops; the world's actors are the
soloists, playing only when they act. Pause mutes the soloists for free
(frozen world = no events) while the menu cursor picks up an instrument of
its own — the Banjo-Kazooie / Wii Shop pause-jam, over the still-playing
song. Second north star: **NecroDancer-style movement-to-the-beat** — the
beat-query API (`beat_crossed`, `music_position`) drives the reverse
direction (song rhythm → world visuals/input-timing judgment). Opt-in per
game; standard ogg/wav music + SFX stay the untouched default. Runs parallel
to Editor Sprint 5 and F/G — the K1 spike goes first and **gates K2–K6**.

**Libraries (settled, FOSS-clean, pure Rust, wasm-friendly):** `midly` (MIT,
MIDI parsing, load-time only) + `rustysynth` (MIT, zero deps, SoundFont synth
with real-time `note_on`/`render`). rustysynth's bundled sequencer *consumes*
the synth, so we write our own — which is exactly what enables live triggering.

**Architecture (kimi-reviewed plan: `review/plan-conductor.md`, round 1
adjudicated):** new crate `crates/music_midi` (no rodio, no I/O — bytes in,
samples out, fully headless); `audio → music_midi` keeps rodio contained both
directions. `.mid` + `.mid.ron` sidecar (schema SSOT mirrors `sheet_file.rs`;
declares SoundFont path, looping, trigger tracks + quantize). `Conductor`
(pure `SequencerCore` + Synthesizer) moves into a custom rodio
`Source<Item=f32>` — no Mutex: `std::sync::mpsc` commands in, atomic
position mirror out, 64-frame blocks. Song position advances by samples
rendered (audio thread), never frame delta — so `ctx.time_scale = 0` freezes
the world while music and triggers keep going: **pause-menu jamming works by
construction**. `AudioManager` gains `play_midi_music`, `midi_trigger(id)`,
`music_position()`, `beat_crossed(subdivision)` via a new `manager/midi.rs`
seam; games reach it all through `ctx.audio`, zero engine_core dep changes.

Issues (Studio Board): **#60 K1** spike (.mid→rustysynth→rodio native+wasm;
verifies rustysynth `Send`/ctor FIRST, documents a wasm fallback, picks the
small CC0 SoundFont — never a 30 MB GM bank) → **#61 K2** `music_midi` core
(sidecar SSOT incl. per-track `mode: Sequence|Loop`, tempo map, sequencer,
edge-case validation: 120 BPM/4-4 defaults, SMPTE rejected, duplicate track
names rejected) → **#62 K3** trigger tracks (chord units, quantize +
one-block late tolerance, loop-wrap all-notes-off + cursor reset, commands
drained before advancing; `Loop` mode wraps a track's cursor so an
SFX-replacement instrument NEVER goes silent — default, since silence =
broken feedback when the note IS the sound effect; `Sequence` = the chime
feel) → **#63 K4** audio integration (pending/gesture parity with
`play_music`, one music slot, paused = triggers dropped not queued,
dead-audio-thread detection, integer-math `beat_crossed`) → **#64 K5** demo
(pong: paddle hits + scoring AS instruments, pause-menu jam, beat-pulsed
visual), wasm listen test, `training.md` pattern → **#65 K6** the
**all-games wave** (like 2-player/pause/localization before it): a composed
song per game with per-game instrument casting (breakout brick-breaks,
snake's rising eat-melody, frogger hops — the NecroDancer-est fit), pause
jam everywhere, browser-verified; songs are a new authored-asset type
parallel to the F art track (**SETTLED, Jesse, Aug 28: music follows the
same tiered AI rules as pixel art** — DEION_STYLE.md §6 amended: AI-assisted
compositions may ship free-tier, never paid; `ai/` + `ai_` quarantine and
the purge script apply to audio files as-is). v2 ideas on record in K5: harmony-aware
position-keyed triggers, intensity layers, song position save/restore for
dedicated pause songs, editor integration.

---

## Phase C (paused): Games 7–15 — Classic + Action

Resumes after Phase G with Deion styling from day one.

| # | Game | Requires | Deion casting | Key New Patterns |
|---|------|----------|---------------|-----------------|
| 7 | Tetris | Tilemap | TBD (DEION_STYLE.md castings table) | Grid logic, piece rotation, line clearing |
| 8 | Galaga | Lifetime, SpriteAnimation | TBD | Enemy formation paths, multi-bullet patterns |
| 9 | Pac-Man | Tilemap, SpriteAnimation | TBD | Pathfinding AI (BFS), power-up state, ghost modes |
| 10 | Simple Platformer | CameraFollow, SpriteAnimation | **Deion himself — first playable Deion** | Multi-level progression, camera smoothing |
| 11 | Run & Gun | CameraFollow, Tilemap, Lifetime | TBD | Horizontal scroll, level checkpoints |
| 12 | Zelda-style Top-Down | CameraFollow, Tilemap, SpriteAnimation | TBD | Room transitions, NPC dialog, item system |
| 13 | Tower Defense | Tilemap, SpriteAnimation | TBD | Wave spawning, tower placement, pathfinding |
| 14 | Sokoban / Puzzle | Tilemap | TBD | Move history / undo, level editor-compatible format |
| 15 | Metroidvania | CameraFollow, Tilemap, SpriteAnimation | TBD | Ability gating, persistent world state, map system |

## Phase D (paused): Games 16–20 — Complex + Original

| # | Game | Focus |
|---|------|-------|
| 16 | Bullet Hell Shoot-em-up | High entity counts, Lifetime at scale, pattern scripting |
| 17 | Roguelike Dungeon | Procedural generation, fog of war, persistent save state |
| 18 | Fighting Game (simple) | Frame-precise animation, hitboxes, combo system |
| 19 | Strategy / Mini-RTS | Unit selection, pathfinding at scale, fog of war |
| 20 | Original Concept — **Deion the Insiculous platformer** | The capstone: the founding concept, built with the full engine + editor + Deion asset pipeline |

---

## Supporting Infrastructure

Done when they unblock a specific game or pivot phase, not on a fixed schedule.

### Editor — UX Audit & Work Order (Aug 27 2026)

**Two north stars (Jesse, Aug 27 2026) — weigh every editor decision against
these:**
1. **AI-first.** An AI agent must be able to author games through the editor's
   **command layer** (audit §9: query API → write commands → headless `--api`,
   designed transport-ready from day one). Per audit §8.3 this is the stronger
   answer to "AI friendly" than any GUI work — driving a UI by screenshots and
   pixel coordinates is the worst interface an agent can have.
2. **The editor running in the browser is the super-ultra-win.** The engine
   already runs on wasm32 (Pong slice shipped Aug 19 2026), so the editor
   staying lightweight and **wasm32-compatible is a standing constraint** on
   every editor decision — it is why dylib scripting is dropped (wasm32 has no
   `dlopen`). Concretely: no *architectural* choice may foreclose the web port;
   inherently native-only subsystems (file dialogs/`std::fs` scans, process
   spawn, `cargo` builds, "Open in IDE") are fine but get designed as
   cfg-gated native features with the web replacement named up front (OPFS /
   VFS fetch / deferred remote build — the audit §8.2 capability split). See
   the Phase H web-editor row.

The full file:line-anchored audit lives at `docs/EDITOR_UX_AUDIT.md`
(2026-08-27): P0 = broken or built-but-never-wired (§1, §2), P1 =
inspector/visual/interaction quality (§3–§5), end state = authoring games in
the editor via the ScriptRef seam (§6) plus the AI command layer (§9). The
audit's §7 work order is adopted as five sprints; **the live items are issues
on the org Studio Board (Phase = Editor)** — the board is the SSOT for open
items, this section carries only the why.

| Sprint | Theme | Audit sections | Priority |
|--------|-------|----------------|----------|
| 1 | "the editor works" | §1.1, §4.4, §2.2, §2.3, §1.3+§1.2, §4.11 | P0 |
| 2 | "the editor is honest" | §1.4 (+§9.3a), §9 Stage A, §5.1→§5.2+§5.3, §5.6, §3.3, §5.4 | P1 |
| 3 | "the inspector is a tool" | §3.5, §3.1/§3.2, §4.10, §9 Stage B, §3.6, §3.4 | P1 |
| 4 | "direct manipulation" | §2.1, §4.6, §4.5, §4.7, §4.9 | P2 |
| 5 | "architecture" | §4.2, §4.3, §6.7, §6.5 Stage 1, §9 Stage C | P2 |

The old "Phase 2 (Ideal Editor UI)" lettering (2A–2G) is superseded by this
sprint order; its history lives in `log_archive.md`.

**Reference:** `crates/editor/IdealEditor.png` for the target mockup — but the
Design System Reference palette (bottom of this file) is **pending the audit
§5.1 gamma verification**: screenshot colors measure ~2× brighter than the
declared tokens (suspected linear→sRGB double-encode); settle the sRGB
question before deriving or re-picking any colors.

### Web Playground — the learn-to-code front (Aug 27 2026)

Adopted Aug 27 2026 (Jesse): the open-source goal is that people **learn to
code and build their own games with this engine**, so beinsiculous.com gets a
playground where visitors use the real editor in the browser and keep their
work. This is north star #2 (browser editor) given a shipping destination, and
the AI-dev-studio premise made touchable. Live items are Studio Board issues;
this section carries the shape:

1. **Editor-on-wasm milestone** (beinsiculous/insiculous_2d#48) — the engine
   already runs on wasm32/WebGPU and the editor is wasm-constrained by
   standing rule, so this is a port: wasm32 check spike folded into the H8 CI
   guard, the audit §8.2 capability split (fs scans → VFS fetch, prefs/scene
   IO → KvStore #6/OPFS), boot on a bundled sample scene, embed on the site
   behind the existing WebGPU gate. Sequenced after the H9 game ports.
2. **Export/import + template repo** (beinsiculous/insiculous_2d#49) — the
   zero-backend save path: client-side project zip export, an Import button
   that round-trips it, and an org `game-template` repo ("Use this template")
   matching the export layout so a browser experiment graduates to real local
   development.
3. **Deferred until the playground proves engagement:** GitHub App/OAuth
   committing to the user's own repo + an Actions template that deploys their
   game. Deliberately not designed yet.

### Scripting — the ScriptRef seam

Adopted from audit §6.3/§6.5/§6.6(4), Aug 27 2026. The seam is a stable,
serializable identity for a chunk of game logic: `ScriptRef { script_id,
source_path, params }` plus `Scripts(Vec<ScriptRef>)` — one component holding
N scripts, string-keyed, because every closed enum (`Behavior`,
`ComponentData`, `ComponentKind`, `EntitySnapshot`) lives upstream of the game
crates and cannot be extended downstream.

- **Stage 1 first, and it is the load-bearing stage:** `Scripts` as inert,
  editor-editable data — attach a named script with tunable params, edit,
  save, reload, undo, duplicate, and it executes nothing. The scene file then
  carries game-logic bindings; everything after is decoration or execution.
  **Hard prerequisite:** the `WorldSnapshot` completeness fix (audit §1.2,
  Sprint 1) — today Play→Stop destroys any component outside its hardcoded
  18-type list, which would silently delete `Scripts` bindings; add `Scripts`
  to the snapshot the same day the component lands.
- Crate placement (audit §6.4): the data types (`ScriptRef`/`ScriptValue`/
  `Scripts`/`ParamSpec`) live in `ecs` — the editor depends on ecs, never on
  engine_core; the runtime registry/runner live in `engine_core`, and the
  script catalog reaches the editor through `InspectorExtras` (the existing
  `texture_display` precedent).
- Execution later via a runtime `ScriptRegistry` + `ScriptBehavior` trait
  (`Game::register_scripts`, defaulted so no existing game breaks).
- **The editor owns build-and-relaunch** (audit §6.5 Stage 5): edit Rust in a
  real IDE → Build & Reload → editor rebuilds, respawns, restores the same
  scene in ~seconds.
- **dylib hot-reload is dropped** — `TypeId` is not stable across dylib
  reloads (the ECS is TypeId-keyed), panics unwinding across FFI are UB, and
  wasm32 has no `dlopen`, so Phase H forecloses it. True hot reload is
  deferred indefinitely; revisit only if build-and-relaunch proves to be the
  actual bottleneck.

---

## Technical Debt (High + Medium Only)

Workspace rollup with per-crate counts: root `TECH_DEBT.md`. LOW priority items
are tracked in `crates/*/TECH_DEBT.md`. Resolved items live in `log_archive.md`.

### Medium Priority

**engine_core (1 item):**
- [ ] **ARCH-006: Closed-world component registries block game-defined components** (widened Aug 27 2026 per audit §6.2/§6.7) — not just scene serialization: **four** parallel closed lists exist for one concept — editor `ComponentKind`/`StoredComponent`, `ComponentData` (`scene_data.rs`/`scene_loader.rs`/`scene_serializer.rs`), `Behavior`, and `EntitySnapshot`. A new type must be added to all four across three crates, and none can be extended by a downstream game crate — which is why a game component cannot appear in Add Component and why `WorldSnapshot` destroys unknown components on Stop. Route through a registry/`Custom` wire variant and collapse the four lists **before** the scripting phase, not paired with it. Note: `ecs/src/component_registry.rs` (dynamic, name-keyed) and `#[derive(ComponentMeta)]` already exist — the editor currently uses neither.

**common (2 items):**
- [ ] **ARCH-001: `CameraUniform` duplicated in renderer crate** — Use `common::CameraUniform` everywhere, remove renderer copy.
- [ ] **DRY-002: Volume clamping duplicated cross-crate** (`audio`, `ecs`) — add `clamp_volume()` utility in common.

**ecs_macros (1 item):**
- [ ] **KISS-001: Over-specified `syn` features** — `["full", "parsing"]` where `["derive"]` suffices; compile-time win. **Re-decide with audit §6.6(2):** a future `#[derive(Script)]` attribute parser would need `syn` "full" — close or reclassify this item before the scripting phase's Stage 3.

**renderer (1 item):**
- [ ] Alpha-blended sprites vs `depth_write_enabled: true` can punch holes across batches — becomes visible with real alpha-edged art; Phase E7 (alpha-cutoff) closes it.

---

## Development Guidelines

### For Every Game
1. Each game is a standalone cargo project in `../games/<name>/` (sibling to this repo)
2. Depends on `engine_core` (includes physics by default) + `ecs` if needed directly — no editor dep
3. Has a `README.md` with: controls, how to run, what patterns it demonstrates
4. `cargo run` from the game directory launches it
5. **Deion Style**: sprites from `.sheet.ron` sheets per `../games/deion_assets/DEION_STYLE.md` (post-Phase F); ChaosTheme neon is the accent layer

### AI-Friendly Development
1. **CLI-testable** — All logic testable without GPU/window. `cargo test --workspace` validates everything.
2. **No manual testing** — If a feature can't be verified by `cargo test`, it needs a test.
3. **Small, focused files** — Files over 600 lines should be split.
4. **Explicit over implicit** — No magic numbers, hidden side effects, or clever tricks.
5. **Strong typing** — Enums over strings, newtypes over primitives.
6. **Verify before claiming** — Always run `cargo test --workspace` before claiming work is done.

### Editor Architecture
1. **Feature-gated** — Editor code compiles out without `--features editor`
2. **Design system** — All colors/spacing from `EditorTheme`, never hardcoded
3. **Command pattern** — All operations undoable
4. **Live editing** — Property changes visible immediately

---

## Quick Reference

```bash
# Run all engine tests
cargo test --workspace

# Run engine example
cargo run --example hello_world

# Run editor on a game project (games/ is a sibling directory)
cargo run --bin editor --features editor -- ../games/pong

# Run a game directly
cd ../games/pong && cargo run
```

**Key Files:**
- `AGENTS.md` — AI agent guidance (high-level)
- `training.md` — API patterns and examples
- `PROJECT_ROADMAP.md` — This file
- `../games/deion_assets/DEION_STYLE.md` — Deion style guide (lands in Phase F)
- `../games/` — Sibling directory with all game projects
- `src/bin/editor.rs` — Standalone editor binary
- `crates/editor/IdealEditor.png` — Target mockup for editor UI
- `examples/hello_world.rs` — Reference implementation
- `examples/editor_demo.rs` — Editor demo (requires `--features editor`)

---

## Design System Reference

Derived from `crates/editor/IdealEditor.png`.

> **Pending the audit §5.1 gamma verification (Aug 27 2026):** on-screen editor
> colors measure ~2× brighter than these declared tokens (suspected
> linear→sRGB double-encode — verify the swapchain format and whether
> `Color::from_hex` linearizes). Until that is settled, do not derive or
> re-pick colors from this palette.

### Color Palette
| Token | Hex | Usage |
|-------|-----|-------|
| `bg-primary` | `#1e1e1e` | Main panel backgrounds |
| `bg-viewport` | `#000000` | Viewport / canvas area |
| `bg-input` | `#2d2d2d` | Input fields, dropdowns |
| `accent-blue` | `#0078d4` | Selection highlights, active buttons |
| `accent-cyan` | `#00d9ff` | Panel headers, interactive highlights |
| `border-panel` | `#007acc` | Panel borders |
| `border-subtle` | `#333333` | Grid lines, separators |
| `text-primary` | `#ffffff` | Primary text |
| `text-secondary` | `#cccccc` | Secondary text, labels |
| `text-muted` | `#888888` | Disabled text, placeholders |
| `gizmo-x` | `#00ff00` | X-axis (green, horizontal) |
| `gizmo-y` | `#ff0000` | Y-axis (red, vertical) |
| `play-green` | `#00cc44` | Play button, playing border tint |
| `pause-yellow` | `#ffcc00` | Pause border tint |
| `stop-red` | `#cc3333` | Stop button |
| `error-red` | `#ff4444` | Error logs, validation |
| `warn-yellow` | `#ffcc00` | Warning logs |

### Spacing
| Element | Value |
|---------|-------|
| Panel padding | 8px |
| Component section spacing | 12px |
| Input field height | 24px |
| Panel header height | 28px |
| Toolbar height | 36px |
| Status bar height | 22px |

### Layout
```
+-------------------------------------------------------------------+
| TOOLBAR (36px)                                                     |
+----------+----------------------------------------+---------------+
| SCENE    | 2D VIEWPORT                            | INSPECTOR     |
| TREE     |   (flexible center)                    |   (280px)     |
| (200px)  |                                        |               |
+----------+----+--------+--------+--------+--------+---------------+
| Bottom Panel Tabs: [Project] [Animation] [Tilemap] [Profiler]     |
+-------------------------------------------------------------------+
| STATUS BAR (22px): Ready | Objects: 42 | FPS: 60 | v2.0.1        |
+-------------------------------------------------------------------+
```

---

Completed milestones (Engine Core 2025, Editor Phase 1, Phase 2A standalone
infrastructure, Games 1–6, Phase B engine gaps) live in `log_archive.md`.
