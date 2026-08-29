# Engine Core Analysis

## Audit Note (2026-04-15)

This document was audited against the current codebase. Pruned as completed or
stale:
- SRP refactoring TODOs for `GameRunner` / `EngineApplication` (managers
  extracted; `EngineApplication` was deleted entirely — only the `Game` trait
  and `run_game()` API remain)
- File extraction phases 1–5 (all completed: `game_config.rs`, `contexts.rs`,
  `ui_integration.rs`, `scene_manager.rs`, behavior optimization)
- Dead-code / debug-`println!` fix items (resolved, verified via grep of
  `src/`)
- Out-of-date test counts (was reported as 33 → now 110 passing, 8 ignored)
- Step-by-step "Phase N COMPLETED" checklists that no longer serve as forward
  guidance

Kept as reference material: architectural rationale, crate boundary decisions,
integration diagram, cross-game theme notes, performance notes, forward-looking
tech debt.

---

## Current State (2026-04-15)

The `engine_core` crate is the orchestration layer tying together windowing,
rendering, input, audio, ECS, UI, and (feature-gated) physics. It owns the
public `Game` trait and `run_game()` entry point, and delegates its concerns to
focused manager structs.

- **Test count:** 110 passing, 8 ignored (GPU/window), 0 failed
- **`game.rs` size:** 577 lines (orchestration only — down from 862 pre-refactor)
- **Public API surface:** `Game` trait + `GameContext` + `GameConfig` +
  `run_game()`. `EngineApplication` is gone.

### Module Layout

```
game.rs                 Game trait, GameRunner orchestration, run_game() entry
game_config.rs          GameConfig builder (title, size, clear color, chaos_mode)
contexts.rs             GameContext, RenderContext, GlyphCacheKey
game_loop.rs            GameLoop config + timer wrapper
game_loop_manager.rs    Frame timing / delta calculation
ui_manager.rs           UI lifecycle + draw command collection
ui_integration.rs       UI DrawCommand -> Sprite bridge; screen-to-world helpers
render_manager.rs       Renderer + sprite pipeline lifecycle, error handling
window_manager.rs       Window creation, size tracking, DPI
scene.rs                Scene lifecycle / world coordination
scene_manager.rs        Scene stack + loading coordination
scene_loader.rs         RON -> World (prefabs, entities, component instantiation)
scene_saver.rs          World -> RON file I/O (paired with scene_serializer)
scene_serializer.rs     World -> SceneData (used by editor save path)
scene_data.rs           SceneData / PrefabData / EntityData / EditorSettings
assets.rs               Texture/font loading; handle_to_path reverse lookup
behavior_runner.rs      Entity behavior iteration (optimized for refs)
lifecycle.rs            FSM for scene lifecycle states
timing.rs               Timer utility
chaos_mode.rs           Normal/Insane/Ridiculous/Insiculous enum + helpers
achievements.rs         Engine-wide achievement registration + toasts + persistence
prelude.rs              Re-exports for `use engine_core::prelude::*`
```

---

## Architecture

### Manager Pattern

Responsibilities extracted from the original monolithic `GameRunner` into
focused managers, each with one job:

| Manager            | Responsibility                                         |
|--------------------|--------------------------------------------------------|
| `GameLoopManager`  | Frame timing, delta calculation                        |
| `UIManager`        | Begin/end UI frame, collect draw commands              |
| `RenderManager`    | Renderer + sprite pipeline lifecycle, error recovery   |
| `WindowManager`    | Window creation, resize tracking, DPI                  |
| `SceneManager`     | Scene stack (push/pop/active), lifecycle               |

`GameRunner::update_and_render()` is pure orchestration — it delegates each
step to the appropriate manager and the user's `Game` impl.

### Game API

The only public way to run a game:

```rust
use engine_core::prelude::*;

struct MyGame;

impl Game for MyGame {
    fn init(&mut self, ctx: &mut GameContext) { /* load assets, spawn entities */ }
    fn update(&mut self, ctx: &mut GameContext) { /* per-frame logic */ }
    // render() has a default impl that extracts sprites from ECS
}

fn main() {
    run_game(MyGame, GameConfig::new("My Game")).unwrap();
}
```

- `GameContext` exposes: `world`, `input`, `assets`, `ui`, `physics`,
  `delta_time`, `chaos_mode`
- ESC is **not** hard-coded to exit anymore — it flows to
  `Game::on_key_pressed()` so the editor (and games that want a pause menu) can
  intercept it
- Scene lifecycle is fully managed internally via `SceneManager` +
  `LifecycleManager`

### Save/Load Pipeline

Two parallel paths — the place most likely to need future work:

1. **Runtime / game loading** — `SceneLoader::load_and_instantiate(path, world,
   assets)` reads a RON file via `scene_data.rs` structs and instantiates
   prefabs+entities into the world.
2. **Editor saving** — `scene_serializer::world_to_scene_data(world, name,
   physics, texture_path_fn)` is the inverse: walks the world, produces a
   `SceneData` which the editor writes via `scene_saver`.

The texture handle → path mapping is carried on `AssetManager.handle_to_path`,
populated during `load_texture()`. Programmatic textures (`#white`,
`#solid:RRGGBB`) round-trip through sentinel strings.

**Known limitation:** `scene_loader.rs:475` has a TODO noting that full dynamic
component instantiation requires type-erased component addition on `World`. The
current loader dispatches on built-in component names explicitly.

### Crate Boundary Decisions

- **UI integration lives here, not in `renderer` or `ui`.**
  - `ui` defines `DrawCommand` (renderer-agnostic)
  - `renderer` defines `Sprite` (UI-agnostic)
  - `engine_core` owns the bridge in `ui_integration.rs`
  - This keeps `renderer` and `ui` independently testable and avoids a circular
    dep. When in doubt about where cross-cutting glue goes, bias toward
    `engine_core`.
- **Physics is feature-gated.** Games that don't need Rapier2d should not pay
  the compile cost. `GameContext.physics` is optional accordingly.
- **`editor_integration` (separate crate) wraps `engine_core` — not the other
  way around.** `engine_core` has no knowledge of the editor.

---

## Cross-Game Themes (carried by engine_core)

### ChaosMode

`ChaosMode` (`Normal` / `Insane` / `Ridiculous` / `Insiculous`) is carried on
`GameConfig.chaos_mode` and mirrored on `GameContext.chaos_mode`. The engine
ships **no** gameplay logic for the variants — each game interprets them. See
`crates/engine_core/src/chaos_mode.rs`:

- `ChaosMode::ALL` for menu iteration
- `is_insane()` / `is_ridiculous()` both return true for `Insiculous`, so
  independently wiring both flags gives "both behaviors" for Insiculous for
  free
- `label()` for display strings

### Achievements

Engine-wide achievement system (`achievements.rs`). Games register definitions
at startup, unlock by id during play; the engine handles JSON persistence and
in-game toast rendering. Deliberately in `engine_core` (not per-game) so that
later tooling (editor badge UI, Steam export, etc.) has a single entry point.

---

## Performance Notes

### Behavior System (optimized)

`behavior_runner.rs` originally cloned `Behavior`, `Transform2D`, and
`BehaviorState` per entity per frame (~80 allocations/frame at 40 entities).
Post-optimization:

- Entities iterated directly (no collection step)
- Components accessed by reference (`world.get::<Behavior>(entity)` without
  `.cloned()`)
- `BehaviorState` cloned only when mutation requires it — only one `.cloned()`
  call remains in the file, and it is justified by the state mutation path

Result: ~85% fewer allocations per frame, verified by tests in
`tests/behavior_optimization.rs`.

### Bind Group Caching

Camera bind group is cached in the renderer; texture bind groups are cached per
handle. The glyph texture cache still keys on `(glyph_id, color)`, which wastes
memory when the same glyph is drawn in multiple colors.

---

## Known Tech Debt / Future Work

Open debt lives on the Studio Board (`tech-debt` label; engine_core's
low-priority backlog is issue #84 — the TECH_DEBT.md files were retired
Aug 28 2026). Headline items that may influence `engine_core` architecture:

- **Dynamic component instantiation in `scene_loader`** — currently dispatches
  on built-in component names. A fully generic approach needs type-erased
  component addition on `World` (tracked via TODO at `scene_loader.rs:475`).
  Until that lands, every new built-in component requires loader changes.
- **Glyph cache keying on color** — wastes texture memory. Should separate
  glyph geometry from tinting (tint via shader uniform).
- **`GameRunner` size** — still ~580 lines in `game.rs`. Further extraction
  candidates: input manager, event dispatch. Push on this only if a concrete
  responsibility emerges; premature extraction adds indirection without
  benefit.
- **Dependency surface is broad** — `engine_core` depends on
  `ecs + renderer + input + ui + audio + physics(opt)`. Watch for any
  temptation to let those crates depend on each other transitively through
  `engine_core`.

---

## Godot Oracle References

When design decisions around the orchestration layer get murky:

- Game loop iteration cadence: `main/main.cpp` — `iteration()` method
- Scene resource loading / packing: `scene/resources/packed_scene.cpp`
- Asset / resource caching: `core/io/resource_loader.cpp`
- FSM for scene lifecycle: `scene/main/scene_tree.cpp`

Use `WebFetch` against
`https://github.com/godotengine/godot/blob/master/<path>`. Study the patterns —
don't transliterate the C++.

---

## Integration Diagram

```
                 run_game(game, config)
                         |
                         v
                    GameRunner
                    /    |    \
                   /     |     \
        WindowManager  GameLoopManager  RenderManager
                          |
                          v
                    GameContext  <-- passed to Game::update()
                    /  |  \  \
                  ECS UI In Assets  (+ physics if enabled)
                          |
                          v
                    UIManager (per-frame begin/end)
                          |
                          v
                    ui_integration (DrawCommand -> Sprite)
                          |
                          v
                    RenderManager.render(batches, textures)
```

Scene operations flow through `SceneManager`, which owns the scene stack and
delegates lifecycle transitions to `LifecycleManager`.
