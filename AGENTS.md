@training.md

# Insiculous 2D - AI Agent Notes

**Reference:** Use `training.md` for detailed API, patterns, and examples
**This file:** Project status, architecture, and high-level guidance

## Project Status (July 2026)

### Core Systems Complete
- **ECS**: HashMap-based per-type storage, 208 tests, type-safe queries, data-driven UI components (UiLabel/UiPanel/UiButton + UiAnchor), named-clip `SpriteAnimation` over `SheetGrid` (`play`/`ensure_playing` by clip name; `SpriteAnimationSystem` writes `Sprite.tex_region`)
- **Renderer**: WGPU 28.0.0, instanced sprites, SDF shapes (rounded rects/circles/borders in the sprite shader), nearest/linear `TextureFilter`, 65 tests
- **Physics**: Rapier2d integration, 62 tests, presets
- **UI**: Immediate-mode, 101 tests, fontdue integration, real text editing (cursor/selection/key-repeat; numeric `float_input` + free-form `text_input`, full A–Z/space typing), Image draw command
- **Input**: Event-based, 75 tests, generic action mapping (`InputMapping<A>`) + player-aware `InputSettings` layer (P1/P2 device routing, axis-as-button, serde bindings) + gilrs hardware backend in engine_core (GAP-001 closed Jul 2026)
- **Audio**: Rodio backend, 19 tests (spatial audio components exist in ecs but have no runtime system yet)
- **Engine Core**: Game API, managers, scene serializer, generic pickups, shared arcade scaffolding (`MenuInput`, `spawn_background`, `default_playfield_grid`, `RENDER_UNIT`), tilemap render pass, main-camera sync, input-settings JSON persistence, gilrs gamepad backend, PauseMenu + MenuPanel chrome (localizable via `PauseMenuLabels`), localization (`Strings`, RON locale files, per-locale fonts), data-driven UI element pass (`UiButtonPressed` events), input-driven camera look-ahead (`CameraFollow`), texture-filter config knob, sprite-sheet pipeline (`load_sprite_sheet`, `.sheet.ron` sidecar schema in `sheet_file.rs`, sidecar-as-SSOT scene reload, sidecar-declared filter on scene texture refs, E5 tex_region/visible/#solid:RRGGBB scene round-trip), 332 tests
- **Editor**: Dockable panels (hide/collapse/resize, View-menu toggles with check marks, persisted layout), viewport, inspector (incl. string fields + UI components), hierarchy, asset browser + drag-drop state, typography/theme tokens, 297 tests
- **Editor Integration**: `run_game_with_editor()` wrapper + inspector writeback + play/pause/stop + scene save/load + viewport↔render camera sync + asset browser panel + editor prefs persistence + editor-font scoping (locale fonts apply to the game view only) + engine-time freeze outside Play (particles/animations hold still while Editing), 79 tests

### Key Metrics
- **Total Tests**: 1305/1305 passing (100% success rate), 0 ignored
- **Code Quality**: every doc example compiles and runs (window/GPU-bound ones are compile-only `no_run`); 1 tracked TODO in production code (`scene_loader.rs` — the ARCH-006/GPP-06 dynamic-component gap, deliberate)
- Games (in `../games/`): breakout 47 tests, pong 11, space_invaders 36, snake 38, asteroids 42, frogger 43 — all clippy-clean, all 2-player; Pong and Frogger are fully localized (en + pirate, locale-driven font); Frogger is the first Tilemap consumer (Jul 2026)

### Current Priority
**The Deion Pivot** drives the roadmap (see `PROJECT_ROADMAP.md`, reworked Jul 28 2026 via adversarial review): the project's identity is **Deion the Insiculous** (SNES-styled hero — ball of DEIONized water with an icicle mohawk — in a food-coded world); the geometry-wars neon look becomes an FX/accent layer. The 20 Games Challenge is **paused at game 7 (Tetris)** while Phases E–I land: **E** engine asset pipeline (nearest filtering, `SheetGrid`, `SpriteAnimation` rework into named clips that actually write `Sprite.tex_region` — today it's disconnected from rendering, `.sheet.ron` sheet import, scene round-trip fixes), **F** `../games/deion_assets/DEION_STYLE.md` + asset production (Jesse draws heroes, agents generate tiles/placeholders; 16px cells × 5 = RENDER_UNIT), **G** re-skin all 6 games (Pong → Frogger → Breakout → Snake → Invaders → Asteroids), **H** WASM port (H1 rodio-audio spike first; fetch-by-default assets; WebGPU-only), **I** deploy (Cloudflare Workers site `../insiculous_web/` — owned by Mily's GitHub account `milyramic` since Aug 2026 — then free itch.io via butler; Steam checklist only), **J** Insiculous Arcade (outline only: the challenge games compiled into one Deion-skinned MARKETPLACE package — Steam/iOS/Android, hand-drawn art only). **Studio premise (Aug 19 2026): Be Insiculous is an AI dev studio** — AI-assisted development is the product story; the **tiered AI-art rule** replaces "AI art never ships": free releases (website, free itch.io) may ship AI art as workflow showcase, paid/marketplace releases never do (`check_no_ai_assets.sh` gates paid paths only; DEION_STYLE.md §6 is SSOT). E1 (`TextureFilter` knob), E2 (`common::SheetGrid`), E6 (atlas.rs deleted) shipped Jul 30 2026 via adversarial-reviewed batch; H1 WASM spike PASSED same day (stay on rodio; see `coordination/H1_SPIKE.md`, listen test PASSED by Jesse — rodio decision FINAL). E3 (named-clip `SpriteAnimation`, wired to rendering + pause + editor freeze) and E4 (`load_sprite_sheet` + `.sheet.ron` v1 schema in `sheet_file.rs`) shipped Jul 30 2026 via the settled `review/plan-v4.md` — **SCHEMA FREEZE REACHED**. **Aug 19 2026: the Pong web vertical slice SHIPPED** — engine runs on wasm32/WebGPU (`common::clock` + `common::vfs` + web boot fetch, cfg-split frame loop with the native path byte-identical, async renderer init, `renderer::insert_canvas_into_dom`), Pong builds at 2.5 MiB via `scripts/build_wasm.sh`, browser-verified interactive, and the site deploy is staged in `../insiculous_web/` awaiting Jesse's push (I1). Next actionable: **H9 remaining 5 game ports + H5 audio remainder + H6/H7/H8** (see `coordination/TODO.md`); F/G asset production continues as the parallel art track (F1 style guide exists — `../games/deion_assets/DEION_STYLE.md`), plus engine leftovers E5 (gated on F3 for the `#rgba` error) and E7 (alpha-cutoff). Editor: Phase 1 complete; the old Phase-2 (Ideal Editor UI) lettering is retired — editor work now follows the UX-audit sprint order (Aug 27 2026): `PROJECT_ROADMAP.md` § "Editor — UX Audit & Work Order" + `docs/EDITOR_UX_AUDIT.md` (live items = Studio Board issues, Phase = Editor).
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
cargo test --workspace               # Run all 1305 tests
cargo test -p editor                 # Run editor tests only
cargo test -p editor_integration     # Run editor integration tests
cargo test -p ecs                    # Run ECS tests only
cargo clippy --workspace             # Lint check
cargo run --example hello_world      # Run platformer demo
cargo run --example editor_demo --features editor  # Run editor demo
cargo run --bin editor --features editor -- ../games/pong  # Standalone editor on a project
```

**Key Files:**
- `AGENTS.md` - This file (high-level guidance)
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
passed: 1305/1305 (100%)
ignored: 0
failed: 0
```
