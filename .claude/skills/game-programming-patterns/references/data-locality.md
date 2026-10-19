# Data Locality

Book: https://gameprogrammingpatterns.com/data-locality.html

Intent: "Accelerate memory access by arranging data to take advantage of CPU caching."

## The problem it solves

A CPU computes far faster than it can fetch from main memory. It hides the gap with caches that
pull memory in cache-line-sized chunks, so reading the *next* bytes is nearly free and reading
bytes somewhere else costs a long stall. Code built from objects behind pointers, such as a list
of boxed entities each pointing at boxed components, makes every step of a loop a jump to a new
address. The loop is "fast" on paper and spends most of its time waiting for memory. The pattern
lays data out in the order it is processed: contiguous arrays of plain values that one pass
walks front to back.

## Reach for it when

- **A profiler says memory access dominates a hot loop.** The book is blunt that this comes
  first, and so is this engine.
- One pass touches one or two fields of very many things every frame: positions of thousands of
  particles, instance data for thousands of sprites.
- The data is plain (`Copy`, no pointers inside) and processed in bulk, with no lookups one item
  at a time.

## Don't reach for it when

- You haven't measured. Cache work done on a guess costs abstraction and buys nothing you can
  show.
- Counts are in the hundreds. At that size the whole working set fits in cache whatever its
  layout.
- The access pattern is random by nature (lookups by id, editor edits). A contiguous array
  doesn't help a hash lookup.
- The layout would fight a workload you also need. Archetype storage makes iteration fast and
  makes adding and removing components slow, and an editor adds and removes constantly.

## Tradeoffs

- **Abstraction cost.** Trait objects, `Box`, and per-entity heap allocations are exactly what
  the pattern removes. Code gets more concrete and less polymorphic.
- **Hot/cold splitting.** Splitting rarely-used fields out of a hot struct shrinks it, at the
  cost of a second array to keep in step.
- **Coupling to the access pattern.** A layout tuned for one pass can hurt another. Choose it
  for the pass that matters, and measure both.
- **Bookkeeping.** Dense arrays need swap-remove plus an index map, or "alive" flags with holes
  that every pass has to skip.

## In Rust

Rust makes the contiguous layout the default and the pointer-chasing layout explicit. A
`Vec<T>` of a `Copy` struct is one allocation, walked in order. `Box<dyn Trait>` is a visible
choice to pay for indirection. The ECS shapes rank like this:

- **`HashMap<Entity, Box<dyn Component>>`**: simplest, fully dynamic, the worst locality.
- **Sparse set**: a dense `Vec<T>` per component type, plus an entity-to-index map. It
  iterates contiguously, and adding or removing a component is cheap.
- **Archetypes**: one table per component combination. It is the fastest to iterate, and
  moving an entity between tables whenever its components change is expensive.

An illustrative sparse set (a sketch, not engine code):

```rust
struct SparseSet<Component> {
    dense: Vec<Component>,
    dense_owners: Vec<EntityId>,
    index_of: HashMap<EntityId, usize>,
}
```

`#[repr(C)]` plus `bytemuck::Pod` keeps a struct one flat block of bytes that can go to the GPU
as is. That is data locality all the way down to the driver.

## In insiculous_2d

**Verdict: deferred until a trigger, by a recorded decision.** The component store is
deliberately the simple layout. Contiguous layouts are used only where the data is bulk and
plain.

The decision of record, verbatim from `crates/ecs/CLAUDE.md` § Storage:

```text excerpt=crates/ecs/CLAUDE.md
`ComponentStore` = `HashMap<EntityId, Box<dyn Component>>` is the accepted
simplicity tradeoff. **Trigger to revisit:** profiling shows component access
dominating a frame, or games routinely exceed ~a few thousand live entities.
When it fires, evaluate a **sparse-set layout FIRST** (dense `Vec<T>` per type
+ entity→index map — cache contiguity without archetype migration; the editor
adds/removes components constantly, a workload archetype migration punishes and
HashMaps tolerate); full archetype storage only if sparse sets measurably
aren't enough.
```

**The trigger has not fired.** No profile shows component access dominating a frame. The
workspace has no benchmark at all yet: the criterion bench for script dispatch is an open
ticket, waiting for a data project with hundreds of entities. Live entity counts in the six
games are in the tens to low hundreds.

- `crates/ecs/src/component.rs` · `ComponentStore` — one boxed component per entity, keyed by
  id:

```rust excerpt=crates/ecs/src/component.rs
pub struct ComponentStore {
    /// The components, indexed by entity ID
    components: HashMap<EntityId, Box<dyn Component>>,
}
```

- `crates/ecs/src/world.rs` · `World::query_entities` — walks every entity and checks
  membership type by type, then collects a `Vec`. Fine at this engine's counts.
- History: a broken archetype storage existed and was deleted in February 2026
  (`log_archive.md` § "February 2026 Fixes").

**Where the data is contiguous, because it's bulk and plain:**

- `crates/engine_core/src/particles/manager.rs` · `ParticleManager` — one fixed `Vec<Particle>`
  walked front to back by `step`, with no allocation per frame.

```rust excerpt=crates/engine_core/src/particles/manager.rs
pub struct ParticleManager {
    pool: Vec<Particle>,
    /// Cursor into the pool — the next slot considered for spawning.
    cursor: usize,
```

- `crates/renderer/src/sprite_data.rs` · `SpriteInstance` — `#[repr(C)]` and `Pod`, batched per
  texture into one `Vec` and uploaded as one buffer.
- `crates/ecs/src/tilemap.rs` · `Tilemap` — `tiles: Vec<u32>`, row-major.

## Pitfalls here

- **Don't propose sparse sets or archetypes without the evidence.** Bring the profile or the
  entity count that fires the trigger, and evaluate a sparse set *first*. Archetypes come only if
  sparse sets measurably fall short.
- **Prefer `world.entity_ids()` over `world.entities()` in hot paths.** The first is an
  iterator, and the second allocates a `Vec` every call (a backlog item on the ECS ticket).
- **Keep a bulk store bulk.** A particle or instance struct that gains a `String`, a `Box` or a
  `Vec` field stops being `Pod`. The particle pool then starts chasing pointers through the
  same loop the layout exists to keep tight.
- **A frame-time regression is a profiling job, not a layout guess.** Measure first, and
  record the number where the next person will find it.
