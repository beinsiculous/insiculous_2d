@AGENTS.md
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
- Trust `AGENTS.md` / memory for current test counts, not stale numbers inside older
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
