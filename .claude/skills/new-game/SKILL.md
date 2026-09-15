---
name: new-game
description: Scaffold a new game for the 20 Games Challenge following the established Pong/Breakout conventions (module layout, chaos modes, achievements, neon look, headless tests). Use when starting any new game in ../games/.
---

# New Game — 20 Games Challenge Scaffold

Games are **standalone cargo projects in `../games/<name>/`** (sibling of the
engine repo) — NOT workspace members; each is its OWN git repo (`git init -b
main`, copy breakout's `.gitignore`, commit when the game is green). The
GitHub home is the **`beinsiculous` org** (Aug 2026): create the remote with
`gh repo create beinsiculous/<name> --private --source . --push`, copy
`.github/workflows/add-to-project.yml` from any existing game (auto-adds new
issues to the org Studio Board, https://github.com/orgs/beinsiculous/projects/1),
and set its secret: `gh secret set ADD_TO_PROJECT_PAT -R beinsiculous/<name>`
(a token with `project` scope — ask Jesse, never commit it). Task tracking
for the new game happens as issues on that board, not in md files. Before
writing anything,
open `../games/space_invaders/` (most recent) and `../games/breakout/` and
mirror their structure. When conventions here conflict with what those games
actually do, the newest game wins — read the code.

## Cargo setup

- Depend on `engine_core` by path (`../../insiculous_2d/crates/engine_core`).
- Optional `editor_integration` dep behind an `editor` feature (copy breakout's
  `Cargo.toml`).

## Required module layout

`main.rs` (Game impl; asset anchoring via `engine_core::game_root!()` — a
macro so YOUR crate's manifest dir is baked in), `constants.rs`, `types.rs`,
`spawning.rs`, `gameplay.rs` (or a `gameplay/` split like pong/space_invaders
once it grows), `menu.rs`, `drawing.rs`, `achievements.rs`, `effects.rs`.
A `chaos_theme.rs` is only needed if you override the engine palette (see
below) — pong and space_invaders use the engine defaults directly.

## Non-negotiable conventions

1. **Chaos modes**: all four variants (Normal / Insane / Ridiculous /
   Insiculous) with a per-game meaning for each. The LOOK comes from the
   engine: `ChaosTheme::for_mode(mode)` (prelude) gives the shared palette
   (`bg_color`, `structure_color`, `accent_color`, `grid_color`, banner,
   `particle_count_mult`); override individual fields with struct-update
   syntax if your game's art differs (see breakout's thin `theme_for()`).
   Mirror the menu selection into `ctx.chaos_mode` (it's read-write and
   persisted by the engine). Insiculous = insane + ridiculous simultaneously —
   `is_insane()`/`is_ridiculous()` both return true for it, so branch on the
   two predicates independently.
2. **Achievements**: engine `AchievementManager`, persisted to
   `saves/<game>_achievements.json`, plus a `DISPLAY_SECTIONS` coverage test
   (every registered achievement appears in a display section).
3. **Neon look**: `self.grid = Some(default_playfield_grid(&theme))` (prelude)
   builds the deforming background; drive it per frame with
   `step_and_emit_grid(self.grid.as_mut(), ctx.world, ctx.lines,
   ctx.delta_time, self.debug_colliders)` (also prelude — gives you the F1
   collider overlay). Backdrop sprite via the engine's
   `spawn_background(world, tex, theme.bg_color, Vec2::new(WIN_W, WIN_H))`.
   Emissive sprites, particle bursts via `ParticleConfig`.
4. **Sizing**: `RENDER_UNIT` (prelude, = 80) — `Transform2D.scale × RENDER_UNIT`
   = pixel size. Do NOT redefine it locally.
   **Colliders use absolute pixels; physics IGNORES Transform2D.scale.** Size
   them from the same constants or sprites and colliders drift apart.
5. **Determinism**: pseudo-random via `hash_f32(frame_count)` /
   `hash_u32(...)` from the prelude. No `rand` crate, no hand-rolled hashing.
6. **Headless tests** for all pure logic: layout math, bounce/steering
   direction, achievement registration, and physics-pair sims for any
   collision the game depends on (see space_invaders `gameplay_tests.rs`).
   Every test must run without GPU/window.
7. **Menus**: `MenuInput::read(ctx.input)` + `input.navigate(selection, count)`
   (prelude) — never hand-roll the up/down/confirm/back reads.

## Proven gameplay patterns (from breakout + space_invaders — reuse, don't reinvent)

- Offset-based paddle bounce (`paddle_bounce_direction`).
- Minimum-vertical-velocity enforcement (anti-stalemate).
- Serve-glue via per-frame `reset_body` (buffered, safe on same-frame spawns).
- Ball/entity loss = sensor hit OR out-of-bounds safety net (both).
- Collision events: drain ONCE per frame with
  `self.physics.take_collision_events()` and share the Vec with every
  consumer (gameplay + `Pickups::collect`). A second take returns empty.
- Movement: `PhysicsSystem::set_velocity(entity, linear, angular)` for every
  launch/move case — never reach into rapier unless you need mass-aware impulses.
- Bullets/effects/debris: attach `Lifetime::new(seconds)`; the engine's frame
  tail runs `LifetimeSystem` for every game — never own one (it would halve
  every lifetime). No per-entity timer bookkeeping.
- Pickups/drops: `engine_core::pickups::Pickups<K>` + `EffectTimer`.
- Menu-state sprite hiding: `set_sprites_visible(ctx.world, entities, visible)`
  (prelude) — you supply the entity list and the state match.
- Scene-defined templates: `instance.spawn_prefab(world, assets, "Name", &overrides)`
  stamps runtime copies of RON prefabs (breakout levels are scene-driven).
- Formation movement: kinematic bodies + per-frame `set_kinematic_target`
  toward home-slot + one shared offset (space_invaders fleet).
- **Rapier does NOT report kinematic-vs-static or kinematic-vs-kinematic
  pairs.** For those, use a game-side AABB test (`rects_overlap` in
  space_invaders) — and prove every physics pair the game depends on with a
  headless sim test before building on it.

## Rule-of-three check

The GPP-03 audit closed with game 3, but the standing directive remains:
any mechanism this game copies from TWO existing games gets promoted to the
engine as part of this task (engine tests + refactor all games onto it +
doc updates); anything genre-specific stays game-side.

## If the engine is missing something

Log it (and fix it in the engine, with tests) rather than working around it in
the game — e.g. MouseButton was added to the engine_core prelude this way.
Engine changes still require the full engine checklist:
`cargo test --workspace` + clippy clean in the engine repo.

## Verify

The game builds and its own `cargo test` passes headless. If you touched the
engine, run the engine's full verification too (`/finish-task`).
