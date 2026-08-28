@training.md


# Solo Session Guardrails — READ FIRST (applies to every model)

This project is worked on by different Claude models across sessions. These rules
encode lessons already learned here — following them is cheaper than re-learning them.

## Non-Negotiable Workflow
1. **Never invent an API.** Before calling any engine function, `grep` for its
   definition or an existing call site. If you can't find it, it doesn't exist —
   check `training.md` or the crate's `CLAUDE.md` instead of guessing.
2. **Small verified steps.** After each edit: `cargo check --workspace`. After each
   feature: `cargo test -p <crate>`. Before claiming done: `cargo test --workspace`
   AND `cargo clippy --workspace --all-targets` (both must be fully clean —
   0 failed, 0 ignored, 0 warnings). Use the `/finish-task` skill for the full checklist.
3. **Never claim tests pass without running them.** Never delete/weaken a failing
   test to make it pass, never add `#[ignore]` or ` ```ignore ` doc examples
   (GPU/window-bound doc examples use ` ```no_run `).
4. **Do only what was asked.** No opportunistic refactors, no new dependencies, no
   `#[allow(...)]`, no `unwrap()` outside tests. Files stay under 600 lines — split
   instead of growing.
5. **When stuck after 2 attempts, stop thrashing.** Consult the Godot oracle (below),
   write findings to `coordination/BLOCKERS.md`, and report to the user with what you
   tried. A clear blocker report beats a wrong "fix".
6. **Use the project skills** for recurring tasks: `/add-component` (wire a new ECS
   component through registry + editor), `/new-game` (20-games-challenge scaffold),
   `/finish-task` (definition-of-done verification).

## Single Sources of Truth (edit HERE, nowhere else)
| Concern | The one place |
|---------|---------------|
| Editor-visible components | `crates/editor/src/stored_component.rs` — one line in `editor_component_registry!` |
| Dynamic component creation by name | `crates/ecs/src/component_registry.rs` — `registry.register::<T>()` in the global-registry fn |
| Scene RON schema (load) | `crates/engine_core/src/scene_data.rs` — `ComponentData` enum + `scene_loader.rs` |
| `.sheet.ron` sidecar schema | `crates/engine_core/src/sheet_file.rs` — `SheetFile` + `parse_sheet_file` + `into_parts` (validation lives here too) |
| Animation clip wire format | `ClipData` in `scene_data.rs` — ONE DTO shared by scene RON and `.sheet.ron` |
| World → RON save | `crates/engine_core/src/scene_serializer.rs` — `extract_components()` (the ONLY save pipeline) |
| Inspector writeback / undo merge | `apply_component_edit()` in `crates/editor/src/component_editors.rs` (called by the registry-generated `edit_all_components`); `impl_set_component_command!` in `crates/editor/src/commands/set_commands.rs` |
| Frame timing | `GameLoopManager` (`game_loop_manager.rs`) — there is no other frame timer |
| Editor colors | `crates/editor/src/theme.rs` `EditorTheme` tokens — never hardcode colors in panels |
| Behavior ↔ BehaviorData | The `From` impl pair in `scene_data.rs` |

## Known Footguns (silent bugs already paid for once)
- **Physics ignores `Transform2D.scale`.** Colliders are absolute-pixel sized; sprites
  scaled via `scale` will visually drift from their collider. Games use
  `RENDER_UNIT = 80` (scale × 80 = pixel size). Check the collider overlay (C key in
  editor) when sprites and physics disagree.
- **Live physics edits now apply (GPP-09):** editing `Transform2D` on a live physics
  entity teleports the body (velocity preserved) and editing `Collider` rebuilds its
  rapier collider — detected by value-compare against a last-pushed baseline.
  Exception: `RigidBody` config edits (body_type, damping, gravity_scale) still
  require the body to be recreated.
- **`Box<dyn Component>` + blanket `Any` impl:** call `.as_ref().as_any()` /
  `.as_mut().as_any_mut()` before downcasting. Bare `.as_any()` on the Box resolves to
  the Box's own TypeId and every downcast fails (bit us in `ecs/component.rs`).
- **wgpu `queue.write_buffer` flushes at `submit()`, not encode time.** Rewriting one
  uniform buffer between passes in a single submit means all passes read the LAST
  write (broke bloom). One buffer per distinct per-frame value.
- **UI text y = baseline** in `label_styled`. For text inside a box use
  `label_in_bounds_styled` (vertically centers via font metrics) or glyphs straddle
  the box border.
- **Editor keyboard shortcuts must gate on `ctx.ui.wants_keyboard()`**, and raw mouse
  consumers (viewport picking) on `ctx.ui.is_input_blocked_at(mouse)` AND
  `ctx.ui.wants_mouse()` (a widget owns the gesture press→release — note the release
  frame, when click handlers fire, is NOT `WidgetState::Active`) — otherwise typing in
  an inspector field triggers Delete-entity/tool shortcuts, clicks pass through open
  dropdowns, and toolbar clicks silently reselect the sprite underneath.
- **Same-frame spawns:** `PhysicsSystem::set_velocity` and `reset_body` are buffered
  and apply once the body syncs — use them, don't reach for rapier directly.
- **Collision events:** drain once per frame with `physics.take_collision_events()`
  (owned `Vec`, no borrow held) and share the Vec among all consumers (gameplay,
  pickups). A second take in the same frame returns empty — never take twice.
- **Destroying a body on contact-start cancels rapier's impulse.** An entity
  destroyed the frame its collision event fires may never push the other body
  back (corner/gap contacts especially) — the mover sails straight through.
  If the response matters (breakout bricks), apply it in game code; see
  `brick_bounce_velocity` in `../games/breakout/src/gameplay.rs`.
- **`ctx.chaos_mode` is read-write** — the engine persists writes made during
  update/key handlers.
- **ECS access:** `world.get::<T>(entity)` / `get_mut` take `EntityId` by value and
  return `Option`. To update component B from component A, read A first, then
  `get_mut` B sequentially (no simultaneous borrows).
- Trust this file / memory for current test counts, not stale numbers inside older
  docs — when in doubt, `cargo test --workspace` is the truth.

# Agent Teams — Parallel Development System

When dispatching parallel work, use the Task tool to spawn subagents that work on
independent crates simultaneously. Each crate has its own `CLAUDE.md` with domain
expertise and Godot oracle references.

## How to Dispatch Work

**Task source of truth (Aug 19 2026): the org taskboard**
https://github.com/orgs/beinsiculous/projects/1 — query it with
`gh issue list -R beinsiculous/insiculous_2d` (games: their own repos).
Claim by assigning/commenting on the issue; close via "fixes …#N" commits.
`coordination/TODO.md` is a pointer; `coordination/PROGRESS.md` stays the
narrative log. Dispatch subagents by crate:

```
Task(subagent_type="general-purpose", prompt="
  Read crates/ecs/CLAUDE.md for domain context, then work on TASK-XXX:
  [paste task spec from TODO.md]
  ...
  When done, run cargo test -p ecs && cargo test --workspace.
")
```

Launch independent tasks in parallel (single message, multiple Task calls).
Wait for all to complete, then verify with `cargo test --workspace`.

## Coordination Protocol

### Task Lifecycle
1. **Claim**: Comment on / assign yourself the GitHub issue (replaces the old
   `coordination/current_tasks/` lock files)
2. **Work**: Subagent implements the task, writes tests, verifies
3. **Verify**: `cargo test --workspace` must pass (0 failures, 0 ignored)
4. **Log**: Append to `coordination/PROGRESS.md` with timestamp and summary
5. **Close**: Commit message references the issue ("fixes beinsiculous/insiculous_2d#N")

### Parallel Safety Rules
- Dispatch agents to **different crates** to avoid merge conflicts
- Never have two agents editing the same file
- Cross-crate tasks (editor_integration touches editor + ecs) should be single-agent
- After all subagents finish, run `cargo test --workspace` from the coordinator

### Crate → Agent Mapping
| Crate | Domain Focus | Test Command |
|-------|-------------|--------------|
| `ecs` | Components, queries, hierarchy, world ops | `cargo test -p ecs` |
| `renderer` | WGPU pipeline, sprites, textures, shaders | `cargo test -p renderer` |
| `physics` | Rapier2d, colliders, presets, spatial | `cargo test -p physics` |
| `editor` | Panels, inspector, picking, gizmos | `cargo test -p editor` |
| `editor_integration` | Wiring editor to engine, play/pause | `cargo test -p editor_integration` |
| `engine_core` | Game API, managers, scene loading | `cargo test -p engine_core` |
| `ui` | Immediate-mode widgets, fonts | `cargo test -p ui` |
| `input` | Keyboard, mouse, gamepad, actions | `cargo test -p input` |
| `audio` | Rodio playback, spatial audio | `cargo test -p audio` |
| `common` | Math, shared types | `cargo test -p common` |

## Quality Review Role

After subagents push changes, review their work:
- Run `cargo clippy --workspace` — no new warnings
- Check that file sizes stay under 600 lines
- Verify test names describe behavior, not implementation
- No `unwrap()` outside tests, no `#[allow(dead_code)]` additions
- Cross-reference against Godot oracle if architectural decisions look questionable

## Godot Oracle — Global Reference

When any agent (or you as coordinator) is stuck on design decisions, consult Godot:
Use `WebFetch` on `https://github.com/godotengine/godot/blob/master/<path>`

**Quick lookup by feature area:**
- Editor architecture: `editor/editor_node.cpp`
- Entity CRUD: `editor/scene_tree_dock.cpp` — `_tool_selected`
- Inspector: `editor/editor_inspector.cpp` — `_property_changed`
- Viewport picking: `editor/plugins/canvas_item_editor_plugin.cpp` — `_gui_input_viewport`
- Undo/redo: `core/object/undo_redo.cpp`, `editor/editor_undo_redo_manager.cpp`
- Scene save/load: `scene/resources/packed_scene.cpp`
- 2D rendering: `servers/rendering/renderer_canvas_cull.cpp`
- 2D physics: `servers/physics_2d/godot_step_2d.cpp`
- Node hierarchy: `scene/main/node.cpp`

**Rule:** Study Godot's *design patterns*. Adapt to our Rust ECS architecture. Don't copy C++.

## When Stuck — Escalation
1. Agent consults its crate's `CLAUDE.md` Godot oracle table
2. Agent uses `WebFetch` to read relevant Godot source
3. If still stuck, agent writes findings to `coordination/BLOCKERS.md`
4. Coordinator reviews blockers, may reassign or break down the task

## Coordination Files
- `coordination/TODO.md` — Task queue (highest priority at top)
- `coordination/PROGRESS.md` — Completed work log
- `coordination/BLOCKERS.md` — Issues with what-was-tried documentation
- `coordination/current_tasks/` — Lock files for active work


# Insiculous 2D - AI Agent Notes

**Reference:** Use `training.md` for detailed API, patterns, and examples
**This section:** Project status, architecture, and high-level guidance
(this whole file is the single agent ruleset — `AGENTS.md` is a symlink to it)

## Project Status (July 2026)

### Core Systems Complete
- **ECS**: HashMap-based per-type storage, 211 tests, type-safe queries, data-driven UI components (UiLabel/UiPanel/UiButton + UiAnchor), named-clip `SpriteAnimation` over `SheetGrid` (`play`/`ensure_playing` by clip name; `SpriteAnimationSystem` writes `Sprite.tex_region`)
- **Renderer**: WGPU 28.0.0, instanced sprites, SDF shapes (rounded rects/circles/borders in the sprite shader), nearest/linear `TextureFilter`, device-loss fail-stop (`DeviceLossLatch` in `device_status.rs` — wgpu lost/uncaptured-error callbacks, every render-path entry guards; same-size/zero-size surface reconfigures deduped), 73 tests
- **Physics**: Rapier2d integration, 62 tests, presets
- **UI**: Immediate-mode, 107 tests, `UiLayer` z-bands (per-layer draw collection flushed in enum order at end_frame — Content/PanelChrome/Floating/Modal/Tooltip/DragGhost; `begin_overlay` = Floating sugar), fontdue integration, real text editing (cursor/selection/key-repeat; numeric `float_input` + free-form `text_input`, full A–Z/space typing), Image draw command
- **Input**: Event-based, 79 tests, generic action mapping (`InputMapping<A>`) + player-aware `InputSettings` layer (P1/P2 device routing, axis-as-button, serde bindings) + gilrs hardware backend in engine_core (GAP-001 closed Jul 2026)
- **Audio**: Rodio backend, 27 tests, `manager/{mod,music,tests}.rs` split (device/SFX vs music/buses), gesture-gated web upgrade (`enable_output()` + pending music; the engine calls it on the first activation gesture — H7 closed Aug 27 2026) (spatial audio components exist in ecs but have no runtime system yet)
- **Engine Core**: Game API, managers, scene serializer, generic pickups, shared arcade scaffolding (`MenuInput`, `spawn_background`, `default_playfield_grid`, `RENDER_UNIT`), tilemap render pass, main-camera sync, input-settings JSON persistence (save-on-change dirty tracking), runtime window retitling (`ctx.window_title` writeback + `WindowManager::set_title`), post-tonemap UI render pass (UI draws to the swapchain AFTER bloom/tonemap — authored UI colors display exactly; game/UI batches stay separate end-to-end), `save_store` persistence seam (native atomic files / wasm localStorage + `insiculous-save` event — `docs/WEB_SAVES.md` is the site contract), `Scores` high-score facility (`ctx.scores`), gilrs gamepad backend, PauseMenu + MenuPanel chrome (localizable via `PauseMenuLabels`), localization (`Strings`, RON locale files, per-locale fonts), data-driven UI element pass (`UiButtonPressed` events), input-driven camera look-ahead (`CameraFollow`), texture-filter config knob, sprite-sheet pipeline (`load_sprite_sheet`, `.sheet.ron` sidecar schema in `sheet_file.rs`, sidecar-as-SSOT scene reload, sidecar-declared filter on scene texture refs, E5 tex_region/visible/#solid:RRGGBB scene round-trip), GPU device-loss fail-stop (RenderManager surface-error streak escalation + `GameRunner.render_fatal` — the frame loop STOPS on device loss instead of submitting to a dead queue, which crashed Firefox's in-process WebGPU; web shows "reload the page", native takes clean shutdown), 381 tests
- **Editor**: Dockable panels (hide/collapse/resize, View-menu toggles with check marks, persisted layout), viewport, inspector (incl. string fields + UI components), hierarchy, asset browser + drag-drop state, typography/theme tokens (WCAG surface ladder surface_0..4 + popup_border with luminance guard tests), command API Stage A (`command_api/` — list/describe/selection/scene queries, name-first EntityRef, transport-agnostic dispatch), CommandHistory dirty watermark (id-of-top; merges reassign ids + clear redo), crate-shipped DejaVu chrome fonts (regular/bold/mono via include_bytes), shared `ScrollState` (inspector/hierarchy/assets), 362 tests
- **Editor Integration**: `run_game_with_editor()` wrapper + inspector writeback + play/pause/stop + scene save/load + viewport↔render camera sync + asset browser panel + editor prefs persistence + editor-font scoping (locale fonts apply to the game view only) + engine-time freeze outside Play (particles/animations hold still while Editing) + play-session data-loss guards (save/new/open refused mid-simulation, snapshot loss warnings on Play and Stop; load_scene parses + dry-runs into a scratch World before touching the live one), --api stdin/stdout query transport, OS-title dirty indicator, 97 tests

### Key Metrics
- **Total Tests**: 1447/1447 passing (100% success rate), 0 ignored
- **Code Quality**: every doc example compiles and runs (window/GPU-bound ones are compile-only `no_run`); 1 tracked TODO in production code (`scene_loader.rs` — the ARCH-006/GPP-06 dynamic-component gap, deliberate)
- Games (in `../games/`): breakout 47 tests, pong 11, space_invaders 36, snake 38, asteroids 42, frogger 45 — all clippy-clean, all 2-player; Pong and Frogger are fully localized (en + pirate, locale-driven font); Frogger is the first Tilemap consumer (Jul 2026)

### Current Priority
**The Deion Pivot** drives the roadmap (see `PROJECT_ROADMAP.md`, reworked Jul 28 2026 via adversarial review): the project's identity is **Deion the Insiculous** (SNES-styled hero — ball of DEIONized water with an icicle mohawk — in a food-coded world); the geometry-wars neon look becomes an FX/accent layer. The 20 Games Challenge is **paused at game 7 (Tetris)** while Phases E–I land: **E** engine asset pipeline (nearest filtering, `SheetGrid`, `SpriteAnimation` rework into named clips that actually write `Sprite.tex_region` — today it's disconnected from rendering, `.sheet.ron` sheet import, scene round-trip fixes), **F** `../games/deion_assets/DEION_STYLE.md` + asset production (Jesse draws heroes, agents generate tiles/placeholders; 16px cells × 5 = RENDER_UNIT), **G** re-skin all 6 games (Pong → Frogger → Breakout → Snake → Invaders → Asteroids), **H** WASM port (H1 rodio-audio spike first; fetch-by-default assets; WebGPU-only), **I** deploy (Cloudflare Workers site `../insiculous_web/` — owned by Mily's GitHub account `milyramic` since Aug 2026 — then free itch.io via butler; Steam checklist only), **J** Insiculous Arcade (outline only: the challenge games compiled into one Deion-skinned MARKETPLACE package — Steam/iOS/Android, hand-drawn art only). **Studio premise (Aug 19 2026): Be Insiculous is an AI dev studio** — AI-assisted development is the product story; the **tiered AI-art rule** replaces "AI art never ships": free releases (website, free itch.io) may ship AI art as workflow showcase, paid/marketplace releases never do (`check_no_ai_assets.sh` gates paid paths only; DEION_STYLE.md §6 is SSOT). E1 (`TextureFilter` knob), E2 (`common::SheetGrid`), E6 (atlas.rs deleted) shipped Jul 30 2026 via adversarial-reviewed batch; H1 WASM spike PASSED same day (stay on rodio; see `coordination/H1_SPIKE.md`, listen test PASSED by Jesse — rodio decision FINAL). E3 (named-clip `SpriteAnimation`, wired to rendering + pause + editor freeze) and E4 (`load_sprite_sheet` + `.sheet.ron` v1 schema in `sheet_file.rs`) shipped Jul 30 2026 via the settled `review/plan-v4.md` — **SCHEMA FREEZE REACHED**. **Aug 19 2026: the Pong web vertical slice SHIPPED** — engine runs on wasm32/WebGPU (`common::clock` + `common::vfs` + web boot fetch, cfg-split frame loop with the native path byte-identical, async renderer init, `renderer::insert_canvas_into_dom`), Pong builds at 2.5 MiB via `scripts/build_wasm.sh`, browser-verified interactive, and the site deploy is staged in `../insiculous_web/` awaiting Jesse's push (I1). **Phase H is COMPLETE (Aug 27 2026)**: H6 save_store + web-saves wave (#6/#17), H7 gesture-gated audio (#8 — `enable_output` + first-gesture hook, pong paddle beep is the demo consumer, browser-verified), H8 wasm CI guard (#7 — `scripts/check_wasm.sh` + `wasm-check.yml`, workspace-scope, arc-lint decided), H5 include_bytes remainder closed won't-do (#9). **Editor Sprint 2 COMPLETE (Aug 27 2026)** — #50 (scene-load data-loss guard incl. AssetManager (path,filter) texture-load dedupe), #24 (CommandHistory dirty watermark + OS-title indicator; confirm dialog = follow-up on the new Modal layer), #25 Stage A (query-only command API, `docs/EDITOR_COMMAND_API.md`, Stage B–D open), #29 (UiLayer z-bands), #28 (shared ScrollState + window-anchored add-component popup on Floating), #27 (crate-shipped DejaVu chrome fonts), #26 (post-tonemap UI pass — the audit's "gamma bug" was the Reinhard tonemap over UI, NOT an sRGB double-encode; WCAG ≥1.35:1 surface ladder + ≥3:1 popup borders, guard-tested). All seven kimi-reviewed; screenshot pass deferred (screen locked during the session). Next actionable: **I1 site deploy push (Jesse) + Editor Sprint 3** (#30–#35); F/G asset production continues as the parallel art track (F1 style guide exists — `../games/deion_assets/DEION_STYLE.md`), plus engine leftovers E5 (gated on F3 for the `#rgba` error) and E7 (alpha-cutoff). Editor: Phase 1 complete; the old Phase-2 (Ideal Editor UI) lettering is retired — editor work now follows the UX-audit sprint order (Aug 27 2026): `PROJECT_ROADMAP.md` § "Editor — UX Audit & Work Order" + `docs/EDITOR_UX_AUDIT.md` (live items = Studio Board issues, Phase = Editor).
**History**: Phase A (games 1–5) ☑, Phase B (CameraFollow/Lifetime/Tilemap) ☑, game 6 Frogger ☑ (`../games/frogger/`, first Tilemap consumer, 43 tests).
**2-Player + universal input (Jul 16 2026)**: every game is now 2-player (Pong 2P human/AI, Breakout co-op top/bottom paddles + dedicated `*_2p` level scenes, Space Invaders & Asteroids co-op, Snake versus) on the engine's player-aware `InputSettings` layer (`ctx.players`, `GameContext`) with JSON-persisted bindings and gamepad-ready menus. Controller hardware works end-to-end via the gilrs backend (GAP-001 closed same day). Also same-day: universal pause (engine `PauseMenu` — Menu/Esc/Start toggles, Resume/Restart/Quit, world+particles freeze via `ctx.time_scale`) and menu window chrome (engine `MenuPanel`/`MenuStyle` — opaque themed panels, ▶-cursor rows) adopted by every game's menus, game-over screens, and pause overlay.
**Panels + data-driven UI + localization (Jul 16 2026)**: editor panels hide/collapse/resize with the layout persisted to `editor_prefs.json` (View-menu toggles with check marks, Reset Layout, toolbar follows the scene view); localization is engine-native (`ctx.strings` — `assets/locales/*.ron` tables, `tr()` with en fallback, per-locale fonts, font-aware glyph caches; editor chrome keeps its own font while the game view localizes); game UI is data-driven via `UiLabel`/`UiPanel`/`UiButton` components (anchor+offset placement, `@key` localized text, `UiButtonPressed` events, full scene round-trip + inspector editing, hidden in the editor until Play). hello_world/editor_demo demo both (L key / View→Cycle Game Locale); Pong is the first fully localized game (en + Pirate with BlackSamsGold font, localized menus/HUD/pause/achievements with a Language title item).

### Technical Debt (live docs — open work only)
- Root `TECH_DEBT.md` — workspace rollup with per-crate open counts; detail in `crates/*/TECH_DEBT.md` and `../games/TECH_DEBT.md`
- `log_archive.md` — resolved/completed history (incl. the closed Jul 2026 Game Programming Patterns audit); when you resolve an item, MOVE it there (never leave ✅/strikethrough entries in the live docs)

## AI-Friendly Development Principles

This engine is designed to be developed collaboratively with AI agents. Follow these principles:

### Everything Must Be CLI-Testable
- **All logic must be testable without a GPU or window.** Every test runs headless — including doc tests. Doc examples that genuinely need a window/GPU/device use ` ```no_run ` (compile-checked, not executed); never ` ```ignore `.
- **`cargo test --workspace`** is the single command to validate the entire engine. It must always pass.
- **`cargo test -p <crate>`** tests individual crates in isolation. Use this for faster iteration on a single system.
- **No manual testing required.** If a feature can't be verified by `cargo test`, it needs a test. AI agents can't click buttons or look at screens.
- **Prefer unit tests over integration tests.** Unit tests are faster and give better error localization. Integration tests are for cross-crate interactions.
- **Test names describe behavior**, not implementation: `test_selection_toggle_adds_and_removes` not `test_toggle_method`.

### Code Must Be Readable by AI
- **Explicit over implicit.** No hidden side effects, magic numbers, or clever tricks. AI agents read code linearly.
- **Small, focused files.** Files over 600 lines should be split. AI context windows are limited.
- **Consistent patterns.** Use the established patterns (Manager pattern, Component pattern, etc.) so AI can predict structure.
- **Strong typing.** Enums over strings, newtypes over primitives. Let the compiler catch errors AI might miss.
- **Doc comments on public APIs.** AI agents use these to understand intent without reading implementation.

### Verification Before Claims
- **Run `cargo test --workspace` before claiming any work is done.**
- **Run `cargo check --workspace` to catch compile errors fast** (faster than full test suite).
- **Check for warnings with `cargo clippy --workspace`** when doing cleanup work.
- **Never claim "tests pass" without actually running them.**

### Workflow for AI Agents
1. **Read `PROJECT_ROADMAP.md`** for current priorities and task breakdown
2. **Read `training.md`** for API patterns and coding guidelines
3. **Read the relevant `TECH_DEBT.md`** in the crate you're working on
4. **Write tests first** when implementing new features
5. **Run `cargo test -p <crate>`** after each change to catch regressions fast
6. **Run `cargo test --workspace`** before considering work complete

## Recurring Themes

### ChaosMode (engine_core)
Project-wide "Normal / Insane / Ridiculous / Insiculous" intensity selector
carried on `GameConfig.chaos_mode` and mirrored on `GameContext.chaos_mode`.
The engine ships *no* gameplay logic for the variants — each game interprets
them per its own mechanics (Pong: Insane doubles ball speed per paddle hit,
Ridiculous starts with 2 balls, Insiculous = both). Helpers: `ChaosMode::ALL`,
`is_insane()`, `is_ridiculous()`, `label()`. Games that let the player pick at
runtime keep their own field as the source of truth; the engine field is for
games that set the mode once at startup via `GameConfig::with_chaos_mode()`.

## Architecture

### Manager Pattern (engine_core)
`GameRunner` is a thin orchestrator (`game.rs`, ~594 lines) over five focused managers:
- `GameLoopManager` - Frame timing (the ONLY frame timer)
- `UIManager` - UI lifecycle and draw-command collection
- `RenderManager` - Renderer/sprite pipeline lifecycle
- `WindowManager` - Window creation and size tracking
- `SceneManager` - Scene loading and stack management

Supporting modules: `game_config.rs`, `contexts.rs` (GameContext/RenderContext), `ui_integration.rs` (UI→renderer bridge), `glyph_texture_cache.rs`, `behavior_runner.rs`. (Refactoring history: `log_archive.md`.)

### Editor Integration
The `editor_integration` crate bridges `engine_core` and `editor` without circular deps:
- `EditorGame<G: Game>` — transparent wrapper that implements `Game`, intercepts all methods to add editor chrome
- `run_game_with_editor(game, config)` — public entry point, wraps game and enforces min window size (1024x720)
- `panel_renderer/` — panel content rendering (scene view, hierarchy, inspector)
- `EditorPlayState` (`Editing`/`Playing`/`Paused`): game logic runs only during Playing; `WorldSnapshot` typed-clone capture on Play, restore on Stop; inspector read-only while Playing

**Dependency graph:**
```
engine_core ──→ ecs, renderer, input, physics, audio, ui
editor ──→ ecs, ui, input, renderer, physics, common      (NO engine_core dep)
editor_integration ──→ editor, engine_core, ecs, ui, input, renderer, common
insiculous_2d (root) ──→ editor_integration (optional, behind "editor" feature)
```

Notes: Escape is NOT a hard-coded exit — it flows to `Game::on_key_pressed()`. `editor_demo.rs` wraps the full PlatformerGame (synced with hello_world.rs). A standalone editor binary exists: `cargo run --bin editor --features editor -- /path/to/project`.

## Quick Reference

**Commands:**
```bash
cargo check --workspace              # Fast compile check (no tests)
cargo test --workspace               # Run all 1447 tests
cargo test -p editor                 # Run editor tests only
cargo test -p editor_integration     # Run editor integration tests
cargo test -p ecs                    # Run ECS tests only
cargo clippy --workspace             # Lint check
cargo run --example hello_world      # Run platformer demo
cargo run --example editor_demo --features editor  # Run editor demo
cargo run --bin editor --features editor -- ../games/pong  # Standalone editor on a project
```

**Key Files:**
- `CLAUDE.md` - This file: guardrails + status + architecture (`AGENTS.md` is a symlink to it)
- `training.md` - Detailed API, patterns, examples
- `PROJECT_ROADMAP.md` - LIVE: tasks, priorities, engine gaps
- `TECH_DEBT.md` + `crates/*/TECH_DEBT.md` + `../games/TECH_DEBT.md` - LIVE: open debt only
- `log_archive.md` - Resolved/completed history (move finished items here)
- `examples/hello_world.rs` - Working game demonstration
- `examples/editor_demo.rs` - Editor demo (requires `--features editor`)
- `src/bin/editor.rs` - Standalone editor binary
- `../games/` - Sibling dir: one cargo project per game (pong, breakout, space_invaders)

**Test Status:**
```
$ cargo test --workspace
passed: 1447/1447 (100%)
ignored: 0
failed: 0
```
