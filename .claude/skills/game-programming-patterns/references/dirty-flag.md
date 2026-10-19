# Dirty Flag

Book: https://gameprogrammingpatterns.com/dirty-flag.html

Intent: "Avoid unnecessary work by deferring it until the result is needed."

## The problem it solves

Some data is derived from other data: a world transform from a chain of local transforms, an
uploaded GPU buffer from the sprites on screen, a save file from the settings in memory. Deriving
it is expensive, and the inputs usually haven't changed since last time. Recomputing it on every
change wastes work when changes come in bursts. Recomputing it every frame wastes work when
nothing moved. The pattern keeps the derived result, notes when an input changes, and rederives
only when someone actually needs the result and an input really has changed.

## Reach for it when

- The primary data changes more often than the derived data is read. A parent moves five times
  before anything reads its children's world positions.
- Deriving is expensive enough to show up on a frame, or it has a cost outside the CPU, such as
  a GPU upload or a disk write.
- You can't update the derived data incrementally. If a running total would do, keep a running
  total instead.

## Don't reach for it when

- The derivation is cheap. A flag adds a branch, memory and a way to be wrong, so it has to be
  worth more than all of that.
- The derived data is read about as often as the primary data changes. In that case there's
  nothing to defer.
- You can't guarantee that every write is noticed. A missed write leaves the derived data
  silently stale, which is worse than slow.

## Tradeoffs

- **Every writer must set the flag.** That is the whole risk of the pattern. With a stored flag,
  correctness depends on every mutation path remembering to set it. Encapsulation (private fields
  behind setters) is what makes a stored flag safe.
- **Deferred work arrives all at once.** Cleaning a big hierarchy on demand can land a spike on
  the frame that reads it.
- **Memory.** You keep the derived result, and often a baseline of the inputs too.
- **When to clean.** The book lists three choices: on demand when the result is read, at a known
  checkpoint (such as once per frame), or in the background. A checkpoint is the easiest to
  reason about in a game loop.
- **How fine-grained.** One flag per object costs memory and catches precise changes. One flag
  per batch costs less, but redoes more work.

## In Rust

A stored `dirty: bool` works well in Rust when the data sits behind private fields and `&mut self`
setters, because the type system then forces every write through a method that sets the flag. An
ECS breaks that guarantee. Components are public data, and any system holding `get_mut` can write
a field directly. For data like that, the Rust-friendly form is **value comparison**: keep a copy
of the inputs as last used, and call the result dirty when the current inputs differ. That costs
one compare per entity per check, and it can't miss a writer.

An illustrative sketch, not engine code:

```rust
struct CachedDerived<Input: PartialEq + Clone, Output> {
    last_input: Option<Input>,
    output: Option<Output>,
}

impl<Input: PartialEq + Clone, Output> CachedDerived<Input, Output> {
    fn get(&mut self, current: &Input, derive: impl FnOnce(&Input) -> Output) -> &Output {
        if self.last_input.as_ref() != Some(current) {
            self.last_input = Some(current.clone());
            self.output = None;
        }
        self.output.get_or_insert_with(|| derive(current))
    }
}
```

## In insiculous_2d

**Verdict: textbook, done by comparing values, and with no shared infrastructure on purpose.**
The engine has four independent dirty checks, each local to the crate it serves. The July 2026
pattern audit recorded why the transform hierarchy compares values instead of storing ECS-level
flags. Comparing values catches every writer by construction, including the editor's
`WorldSnapshot` restore, which re-adds components wholesale. A physics writeback of an identical
value stays clean. And no ECS API had to change. The same entry records the decision that the
engine has no shared change-tracking layer. Physics keeps its own baseline, and the renderer keeps
a coarse upload gate.

- `crates/ecs/src/hierarchy_system.rs` · `CachedNode` — the per-entity baseline: the local
  transform and parent link as last propagated, plus a frame stamp for pruning.
- `crates/ecs/src/hierarchy_system.rs` · `TransformHierarchySystem::is_dirty` — dirty means the
  local transform or parent link differs from the baseline, or the entity has never been seen.

```rust excerpt=crates/ecs/src/hierarchy_system.rs
        match self.cache.get_mut(&entity) {
            Some(cached) => {
                cached.stamp = self.frame;
                cached.local != *local || cached.parent != parent_id
            }
            None => true,
        }
```

- `crates/ecs/src/hierarchy_system.rs` · `TransformHierarchySystem::reset` — drops every
  baseline. The editor calls it after replacing the world wholesale on Stop.
- `crates/physics/src/physics_system/mod.rs` · `PushedState` — physics' own baseline. An edited
  `Transform2D` teleports the body, an edited `Collider` rebuilds it, and rapier's own writeback
  refreshes the baseline, so simulated motion is never mistaken for an edit.
- `crates/physics/src/physics_system/sync.rs` · `sync_entity_to_physics` — where the
  comparison runs.
- `crates/renderer/src/sprite/instance_cache.rs` · `InstanceCache::stage` — the coarse gate.
  It compares the frame's flattened sprite instances as raw bytes against what was last
  uploaded, and skips the GPU write when they match.

```rust excerpt=crates/renderer/src/sprite/instance_cache.rs
        let unchanged = self.staging_layout == self.layout
            && cast_slice::<SpriteInstance, u8>(&self.staging)
                == cast_slice::<SpriteInstance, u8>(&self.instances);
```

- `crates/renderer/src/sprite/pipeline.rs` · `prepare_sprites` — calls `stage` and uploads
  only when it reports a change.

The two stored flags are both behind encapsulation, which is exactly where a stored flag is
safe:

- `crates/input/src/player.rs` · `PlayerBindings::bind` — `dirty |= mapping.bind(…)`, so only a
  real change sets the flag. The fields are private, so every change goes through a setter.
- `crates/input/src/player.rs` · `InputSettings::take_dirty` — read-and-clear. The runner saves
  the bindings to disk only on a frame where something changed.
- `crates/editor/src/commands/mod.rs` · `CommandHistory::is_dirty` — "unsaved changes" is not a
  flag at all. It compares the id of the top undo entry against a saved watermark, so undoing back
  to the saved state reads as clean again.

## Pitfalls here

- **The hierarchy runs only under the editor.** `TransformHierarchySystem` is owned and stepped
  by `editor_integration` (`EditorGame`, `ProjectHost`). The plain `run_game` frame tail doesn't
  step it. A shipped build that parents entities gets no `GlobalTransform2D` propagation, and a
  parented sprite draws at the origin. Tracked as insiculous_2d#168. Until it lands, a game that
  parents entities owns a `TransformHierarchySystem` and calls `update` before drawing.
- **Don't write `GlobalTransform2D` by hand.** It isn't change-tracked. The next time the entity
  goes dirty, propagation overwrites it. A guard test in `crates/ecs/tests/hierarchy_dirty.rs`
  holds this.
- **A wholesale world replacement needs `reset()`.** After a snapshot restore, the ids still
  match, so a surviving baseline could hide a real change. The editor's Stop calls `reset`, and
  physics has the same contract in `PhysicsSystem::clear`.
- **A refresh of the stamp is not optional.** `is_dirty` refreshes the stamp even for a clean
  entity under a dirty ancestor. Without that, pruning would evict live entries.
- **`RigidBody` config edits aren't pushed.** Its velocity fields are owned by the writeback, so
  comparing the whole struct would report false edits. Editing a live body's settings needs the
  body recreated. This is tracked on the board as a physics low.
- **Don't build a shared change-tracking layer.** That idea was turned down. If a new consumer
  needs a dirty check, give it its own local baseline, the way physics and the renderer each have
  theirs.
