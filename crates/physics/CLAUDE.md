# Physics Crate — Agent Context

You are working in the physics crate. Rapier2d integration with ECS components and presets.

## Architecture
```
PhysicsSystem
├── PhysicsWorld (rapier2d wrapper)
│   ├── RigidBodySet, ColliderSet
│   ├── IntegrationParameters
│   └── PhysicsPipeline
└── ECS sync: one-way per direction (see Update Flow)
    Collision events: take_collision_events() drain
```

## Update Flow
1. `PhysicsSystem::update(world, delta_time)`
2. Garbage-collect rapier state for entities removed from the ECS directly
3. Sync ECS → physics: adds missing bodies/colliders AND pushes **external
   ECS-side edits** (GPP-09, value-compare vs a last-pushed baseline):
   editing `Transform2D` teleports the live body (velocity preserved),
   editing `Collider` rebuilds its rapier collider, removing `Collider`
   drops it. `set_velocity` / `reset_body` remain the
   explicit APIs. `RigidBody` config edits still require body recreation.
4. Flush deferred resets/velocities (for entities spawned the same frame)
5. Clear the collision event buffer, then run 0..=8 fixed-timestep sub-steps
   (each `step()` APPENDS its events)
6. Sync rapier body positions/velocities → ECS components (Dynamic/Kinematic);
   game code drains collisions via `take_collision_events()`

## Collision Event Contract
- Game-facing API: **`PhysicsSystem::take_collision_events()`** — drain once
  per frame after `update()`, share the owned `Vec` among all consumers
  (gameplay, pickups). No borrow is held, so handlers can freely mutate
  physics/world. A second take in the same frame returns empty.
- `PhysicsWorld::step()` APPENDS events; it never clears the buffer.
- `PhysicsWorld::clear_collision_events()` must be called once per frame
  before the first step (`PhysicsSystem::update` does this).
- A frame with zero sub-steps therefore emits NO events (no stale
  re-delivery of last step's `started` events), and a frame with multiple
  catch-up sub-steps delivers the events of every sub-step.
- Contact points/normals are in world space (pixels).

## Physics Entities Must Be Root Entities
Physics ignores the ECS parent-child hierarchy entirely: an entity's
`Transform2D` is read as a WORLD-space position when the body is created, and
rapier results are written back into that same (local) transform every frame.
Parenting an entity that has a `RigidBody` gives nonsense — the parent offset
is never applied and hierarchy propagation will fight the physics writeback.
Pinned by `test_parented_entity_with_rigid_body_is_treated_as_world_space`.

## File Map
- `physics_world/` — Rapier2d wrapper: `PhysicsConfig` (unit conversion, validated scale), bodies, stepping, and collision extraction.
- `physics_system/` — ECS driver: fixed-timestep sub-stepping, ECS↔rapier sync, external-edit propagation, and deferred op queue.
- `components.rs` — `RigidBody` and `Collider` components, collision events, and editor variant helpers with carried dimensions.
- `presets.rs` — pre-configured physics body and collider archetypes.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Colliders are absolute-pixel sized and ignore `Transform2D.scale`; scaled sprites will visually drift from colliders | `src/physics_system/tests.rs test_collider_size_is_absolute_pixels_and_ignores_transform_scale` |
| Live physics edits: editing `Transform2D` teleports the body and editing `Collider` rebuilds its collider, but `RigidBody` config edits require recreating the body | `src/physics_system/tests.rs test_live_rigid_body_config_edit_needs_the_body_rebuilt` |
| Physics entities must be root entities: parenting an entity that has a `RigidBody` is treated as world space and hierarchy propagation fights physics writeback | `src/physics_system/tests.rs test_parented_entity_with_rigid_body_is_treated_as_world_space` |
| Draining collision events via `take_collision_events()` more than once in the same frame returns empty | `src/physics_system/tests.rs test_second_take_collision_events_in_a_frame_returns_empty` |
| Destroying a body on contact-start cancels Rapier's impulse; if collision response matters, apply in game code | — none |
| `PhysicsWorld::apply_impulse` silently no-ops on same-frame spawns without a synced body; `PhysicsSystem::set_velocity` defers safely | `src/physics_system/tests.rs test_reset_body_and_set_velocity_apply_in_call_order_live_or_deferred` |
| External `Transform2D` edits teleport live bodies while preserving velocity | `tests/external_edits.rs test_external_transform_edit_teleports_live_body_and_keeps_its_velocity` |
| External `Collider` edits rebuild Rapier colliders and removal drops them | `tests/external_edits.rs test_collider_edit_rebuilds_and_collider_removal_drops_the_rapier_collider` |


## Key Patterns
- All rapier types stay inside `PhysicsWorld` — ECS components are our own types
- Body handles stored in RigidBody component for rapier lookup
- Presets: `player_platformer()`, `platform(w, h)`, `player_box(w, h)`
- `PhysicsSystem::set_velocity` is the universal "launch this body" API
  (deferred-safe for same-frame spawns); `PhysicsWorld::apply_impulse` exists
  for genuine mass-aware impulses (used by engine_core's behavior_runner)
- `PhysicsConfig.solver_iterations` / `.friction_iterations` map to rapier's
  `num_solver_iterations` / `num_additional_friction_iterations`

## Known Tech Debt
Tracked on the Studio Board: issue #85 — all Low: RigidBody config edits on
live bodies not pushed (rebuild required), SRP-001 (PhysicsWorld manages many
rapier types), API-001 (timing getters), partial MISSING-001
(gravity/collider-dim validation), GPP-L10 (per-step contact Vec alloc),
SRP-002 (collider clamping vs builder).

## Testing
- `cargo test -p physics` — 0 failed, 0 ignored
- Pure math/simulation — no GPU needed

## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

| Our Concept | Godot Equivalent | File |
|-------------|-----------------|------|
| PhysicsSystem::update | Physics step | `servers/physics_2d/godot_step_2d.cpp` — `step` |
| PhysicsWorld | Physics server | `servers/physics_2d/godot_physics_server_2d.cpp` |
| RigidBody presets | Body types | `scene/2d/physics_body_2d.cpp` — RigidBody2D, CharacterBody2D |
| Collider (is_sensor) | Area2D | `scene/2d/area_2d.cpp` — overlap detection |
| Collision events | Contact monitoring | `scene/2d/physics_body_2d.cpp` — `_body_enter_tree` |
| Broad-phase | BVH broad-phase | `servers/physics_2d/godot_broad_phase_2d.cpp` |

**Remember:** We use Rapier2d — study Godot's *API design* and *body type organization*, not its solver.
