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
    Collision events: world event bus + take_collision_events() drain
```

## Update Flow
1. `PhysicsSystem::update(world, delta_time)`
2. Garbage-collect rapier state for entities removed from the ECS directly
3. Sync ECS → physics: adds missing bodies/colliders AND pushes **external
   ECS-side edits** (GPP-09, value-compare vs a last-pushed baseline):
   editing `Transform2D` teleports the live body (velocity preserved),
   editing `Collider` rebuilds its rapier collider, removing `Collider`
   drops it. `set_body_transform` / `set_velocity` / `reset_body` remain the
   explicit APIs. `RigidBody` config edits still require body recreation.
4. Flush deferred resets/velocities (for entities spawned the same frame)
5. Clear the collision event buffer, then run 0..=8 fixed-timestep sub-steps
   (each `step()` APPENDS its events)
6. Reset one-update forces (`apply_force`) if any steps ran
7. Sync rapier body positions/velocities → ECS components (Dynamic/Kinematic)
8. Emit collision events to the world event bus (game code drains its copy
   afterwards via `take_collision_events()`)

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
- `lib.rs` — public exports
- `prelude.rs` — convenience re-exports
- `physics_world/` — Rapier2d world wrapper
  - `mod.rs` — `PhysicsConfig` (validated scale), struct, construction, unit conversion
  - `bodies.rs` — add/remove bodies & colliders, per-body accessors, `reset_forces`
  - `stepping.rs` — `step()`, collision event extraction, `clear_collision_events`
  - `queries.rs` — `raycast` (direction normalized internally)
  - `tests.rs`
- `physics_system/` — ECS driver
  - `mod.rs` — struct, builders, deferred-op queue, pass-through API
  - `sync.rs` — ECS↔rapier sync + orphan GC
  - `update.rs` — `System` impl (fixed-timestep loop)
  - `tests.rs`
- `components.rs` — RigidBody, Collider ECS components, CollisionEvent/Data; editor-facing enum helpers (`RigidBodyType::ALL/label/index`, `ColliderShape::VARIANT_NAMES/variant_with_carried_dimensions` — clean round-trips are exact, capsule↔capsule is a pure axis swap)
- `presets.rs` — Pre-configured physics: `RigidBody::player_platformer()`, `Collider::platform(w, h)`, etc.

## Key Patterns
- All rapier types stay inside `PhysicsWorld` — ECS components are our own types
- Body handles stored in RigidBody component for rapier lookup
- Presets: `player_platformer()`, `pushable()`, `platform(w, h)`, `bouncy(w, h)`, `player_box(w, h)`
- `PhysicsSystem::set_velocity` is the universal "launch this body" API
  (deferred-safe for same-frame spawns); `PhysicsWorld::apply_impulse` exists
  for genuine mass-aware impulses (used by engine_core's behavior_runner)
- `apply_force` lasts one update (forces are reset after the step loop)
- `PhysicsConfig.solver_iterations` / `.friction_iterations` map to rapier's
  `num_solver_iterations` / `num_additional_friction_iterations`

## Known Tech Debt
See `TECH_DEBT.md` — remaining (all Low): RigidBody config edits on live
bodies not pushed (rebuild required), SRP-001 (PhysicsWorld manages many
rapier types), API-001 (timing getters), partial MISSING-001
(gravity/collider-dim validation).

## Testing
- 66 passing (57 lib + 6 integration + 3 doc), 0 ignored — `cargo test -p physics`
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
