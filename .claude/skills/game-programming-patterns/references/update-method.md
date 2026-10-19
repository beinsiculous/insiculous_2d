# Update Method

Book: https://gameprogrammingpatterns.com/update-method.html

Intent: "Simulate a collection of independent objects by telling each to process one frame of behavior at a time."

## The problem it solves

A world full of things that each do something every frame (a patrolling guard, a ticking
lifetime, an animation) can't be scripted as one long procedure, because they all run at once.
The pattern gives each kind of behavior a single "advance one frame" entry point. The game loop
walks the collection once per frame and calls it, so each object keeps its own state between
calls and the loop never needs to know what any of them do.

## Reach for it when

- Many live things act independently and simultaneously, and each needs a slice of every frame.
- Behavior has to persist across frames (timers, phases, a patrol's next waypoint), so it
  belongs in state rather than on the call stack.
- You'd otherwise end up with one giant frame function that knows every entity kind.

## Don't reach for it when

- Things only act in response to something: input, a collision, a turn. Then an event or
  a command is clearer than polling every frame ([event-queue](event-queue.md),
  [command](command.md)).
- The objects aren't independent, such as a board-game move that resolves the whole board at
  once. That's one function over the whole state, not N updates.
- Most objects are dormant most of the time. Walking them every frame to do nothing is the cost.
  Keep the active ones in their own list, or query only the components that matter.

## Tradeoffs

- **Order is semantics.** Object B updating after object A sees A's new state, and A
  sees B's old state. Either fix the order deliberately or double-buffer what's read
  ([double-buffer](double-buffer.md)).
- **Changing the list mid-walk.** Spawning or removing while iterating skips, repeats or
  crashes. The book's options are caching the count, walking backwards, or deferring removals.
- **Frame slicing.** Behavior that naturally spans frames ("walk three steps, then wait") has
  to be turned into explicit state, often a [state](state.md) machine.
- **Where `update` lives.** On the object, on a component, or on a system that walks
  components. The further out it lives, the less each object knows, and the more the order has
  to be written down somewhere.

## In Rust

A `Vec<Box<dyn Updatable>>` where each object mutates itself and reaches out to the others is
exactly the aliasing Rust refuses. Idiomatic Rust moves `update` outward into **systems** that
take `&mut World` (or the components they need) and walk entities by id:

- **Iterate over owned ids, not borrowed storage.** Collect the ids first and then mutate. That
  is the Rust form of "cache the count", and it makes mid-walk removal safe.
- **Defer what crosses entities.** Writes to *other* entities go into a command buffer
  applied after the walk, which also settles the order question.
- **An enum is a closed set of behaviors.** `match` on a data enum instead of virtual dispatch
  when the set of behaviors is known and serialized ([type-object](type-object.md)).

An illustrative sketch, not engine code:

```rust
fn tick_timers(world: &mut World, delta_seconds: f32) {
    let carriers: Vec<EntityId> = world.query::<Timer>();
    for entity in carriers {
        if let Some(timer) = world.get_mut::<Timer>(entity) {
            timer.remaining_seconds -= delta_seconds;
        }
    }
}
```

## In insiculous_2d

**Verdict: textbook, run as an explicit ordered list. One registry-driven path is dead in
production** (see Pitfalls).

Every system implements one trait, but the runtime calls a fixed list by hand, and the order is
written down with its reasons:

```rust excerpt=crates/engine_core/src/game/frame_tail.rs
pub(crate) fn step_world_systems(world: &mut World, delta_time: f32) {
    ecs::SpriteAnimationSystem.update(world, delta_time);
    ecs::LifetimeSystem.update(world, delta_time);
    ecs::ClipStateMachineSystem.update(world, delta_time);
}
```

- `crates/ecs/src/system.rs` · `System` — `update(&mut self, world, delta_time)` plus optional
  lifecycle hooks.
- `crates/engine_core/src/game/frame_tail.rs` · `step_world_systems` — animation, then lifetimes
  (an expired entity never animates again), then the clip machine. It runs on the time-scaled
  delta, so pause freezes all three.
- `../games/pong/src/gameplay/mod.rs` · `update_gameplay` — a game steps its own
  `PhysicsSystem` inside `update`, between its input-driven moves and its collision handling.
- `crates/editor_integration/src/project_host.rs` · `transform_hierarchy` — the editor host
  owns and calls `TransformHierarchySystem` directly, as the frame tail does.

Systems walk an owned list of ids, so removing an entity mid-walk is safe:

```rust excerpt=crates/ecs/src/lifetime.rs
        // query_entities returns an owned Vec, so removal during the loop
        // can't invalidate the iteration.
        let carriers: Vec<EntityId> = world.query_entities::<Single<Lifetime>>();
```

- `crates/ecs/src/lifetime.rs` · `LifetimeSystem` — the whole pattern in fifteen lines.

Per-entity behavior is a `match` over a data enum, and cross-entity effects are deferred:

```rust excerpt=crates/engine_core/src/behavior_runner/mod.rs
        // Process all entities with behaviors directly - avoid cloning
        for entity in world.entities() {
            // Get behavior component by reference to avoid cloning
            let Some(behavior) = world.get::<Behavior>(entity) else { continue };
```

- `crates/engine_core/src/behavior_runner/mod.rs` · `BehaviorRunner::apply_commands` — tags,
  velocities, impulses, events and despawns are applied after the walk, in a fixed order.
- `crates/engine_core/src/scripting/registry.rs` · `ScriptBehavior` — scripts get two update
  hooks, `early_update` before the physics step and `update` after the collision drain.
- `crates/engine_core/src/scripting/runner.rs` · `ScriptRunner::early_update` — each phase reads
  one snapshot and queues commands ([double-buffer](double-buffer.md)).

## Pitfalls here

- **`World::update` never runs in production.** `World` carries a `SystemRegistry`, and
  `add_system` will even late-start a system added to a running world, but no runtime code
  calls `World::update`. Only `Scene::update`/`update_with_schedule` do, and those run in tests
  alone. A game that calls `ctx.world.add_system(TransformHierarchySystem::new())` in `init`
  gets a system that never ticks, silently. Call your system's `update` yourself from
  `Game::update`, the way the editor host does. Whether to delete the scheduler or wire it in is
  tracked as insiculous_2d#170.
  - `crates/ecs/src/world.rs` · `World::add_system` — accepted, started, never updated at
    runtime.
  - `crates/ecs/src/system.rs` · `SystemRegistry` — the scheduler the runtime doesn't use.
  - `crates/engine_core/src/scene.rs` · `Scene::update_with_schedule` — tests-only caller.
- **Don't run an engine-tail system twice.** A game that owns its own `LifetimeSystem` halves
  every lifetime, because the frame tail already runs one.
- **One frame of lag across the tail.** Animation advances *after* the game's `update`, so
  anything the game derives from the drawn frame (pong's jaw collider) reads last frame's
  frame. That's acceptable, but make it a decision.
- **Write to other entities through commands.** A behavior or script that mutated a neighbor
  directly mid-walk would make the result depend on `HashMap` iteration order. That is why
  every runner here defers.
