# Object Pool

Book: https://gameprogrammingpatterns.com/object-pool.html

Intent: "Improve performance and memory use by reusing objects from a fixed pool instead of allocating and freeing them individually."

## The problem it solves

Some things are born and die by the hundreds: sparks, bullets, debris. Allocating and freeing
each one costs time, and on a long-running console or a small heap it fragments memory until a
large allocation fails while plenty of total memory is still free. The pattern allocates one
fixed block of objects up front. Each slot carries an "in use" mark. Creating an object claims a
free slot and initializes it, and destroying one just clears the mark. The allocator never sees
the churn.

## Reach for it when

- You create and destroy many objects of the same size at a high rate, and profiling shows the
  allocation, or the setup and teardown around it, on the frame.
- Fragmentation is a real risk: a fixed heap, a long session, or a platform with a strict memory
  budget.
- Creating the object is expensive beyond the allocation, for example a GPU resource or an OS
  handle.

## Don't reach for it when

- Live counts are small and bounded. Four bullets don't need a pool.
- The objects vary in size. A pool sized for the largest one wastes the difference in every slot.
- What you're "allocating" is cheap bookkeeping in a structure that already reuses its memory,
  such as an entity made of a few hash-map inserts.
- You haven't measured. A pool adds sizing decisions, exhaustion policy and stale-state bugs.

## Tradeoffs

- **Sizing.** Too small and it runs out. Too large and the memory sits idle, and is scanned.
- **Exhaustion has to be a decision.** The book's options are: make it impossible, skip creating
  the new object, forcibly reuse an existing one (often the oldest or least visible), or grow the
  pool. Each one is visible to players in a different way.
- **Reused slots are not cleared.** A slot left half-initialized carries the last occupant's
  state. Initialize every field on reuse.
- **Finding a free slot.** A linear scan is simple, but it costs time that grows with the pool. A
  free list threaded through the dead slots makes it O(1).
- **Ids into a pool go stale.** When a slot is reused, an old reference now points at a different
  object. Generational ids exist to catch this.

## In Rust

Rust has no garbage collector to fight and a fast allocator, so the "allocation is slow" half of
the argument is weaker than in the book. The idiomatic form is still useful where churn is
measurable:

- A `Vec<Slot>` allocated once, with an `alive` flag or `Option<T>` per slot, so assigning into
  a slot never allocates.
- A free list kept as a `Vec<usize>` of dead indices (pop to spawn, push to free), instead of
  threading it through the dead objects' memory as the book's C++ does.
- **Generational indices** (`{ index, generation }`). The generation is bumped when a slot is
  reused, so a stale handle reads as dead instead of aliasing the new occupant. This is the Rust
  way to make a pool's ids safe without references.
- `Vec::clear` keeps its capacity. A persistent buffer that you clear each frame is the
  lightest pool there is.

An illustrative sketch, not engine code:

```rust
struct Pool<Item> {
    slots: Vec<Option<Item>>,
    free_indices: Vec<usize>,
}

impl<Item> Pool<Item> {
    fn spawn(&mut self, item: Item) -> Option<usize> {
        let index = self.free_indices.pop()?;
        self.slots[index] = Some(item);
        Some(index)
    }

    fn despawn(&mut self, index: usize) {
        if self.slots[index].take().is_some() {
            self.free_indices.push(index);
        }
    }
}
```

## In insiculous_2d

**Verdict: partial, and deliberately so.** Particles are a textbook ring pool, and per-frame
buffers are pooled by keeping their capacity. Entities, including bullets, are *not* pooled, and
entity ids are never recycled. Both are decisions with reasons, not gaps.

- `crates/engine_core/src/particles/manager.rs` · `ParticleManager` — a fixed `Vec<Particle>`
  with an `alive` flag per slot and a round-robin cursor. There are no allocations per frame
  after construction.
- `crates/engine_core/src/particles/manager.rs` · `DEFAULT_CAPACITY` — 16,384 slots, sized
  generously so the pool doesn't fill in practice.
- `crates/engine_core/src/particles/manager.rs` · `ParticleManager::next_slot` — the exhaustion
  policy is forcible reuse. When every slot is alive, the slot at the cursor (the one written
  longest ago in round-robin order) is overwritten. Losing some stale dust beats a stutter.

```rust excerpt=crates/engine_core/src/particles/manager.rs
        for _ in 0..slot_count {
            let slot_index = self.cursor;
            self.cursor = (self.cursor + 1) % slot_count;
            if !self.pool[slot_index].alive {
                return slot_index;
            }
        }
```

- `crates/engine_core/src/particles/manager.rs` · `ParticleManager::spawn_burst` — assigns a
  whole `Particle` into the slot, so no field carries over from the previous occupant.
- `crates/engine_core/src/game.rs` · `game_batcher` — the sprite batchers persist across
  frames and are cleared with their capacity kept. That makes them a pool of batch storage.
- `crates/renderer/src/sprite_data.rs` · `DynamicBuffer` — the GPU instance buffer grows to the
  next power of two when it has to, and never shrinks.

**Why bullets aren't pooled.** The live counts are tiny and capped (asteroids allows four bullets
per ship, space_invaders one hero shot). An entity is a few hash-map inserts, and `Lifetime` plus
the engine's `LifetimeSystem` already handles despawning, while physics collects the rapier body
on its next update. The arcade-wide pooling question is recorded as "pool only if profiling says
so", and is tracked on the board as a games low.

- `../games/space_invaders/src/spawning.rs` · `spawn_shot` — a shot is a plain entity with a
  `Lifetime` safety net.
- `../games/asteroids/src/spawning.rs` · `spawn_bullet` — the same shape. The ship's
  `MAX_LIVE_BULLETS` cap keeps the live count bounded.
- `crates/ecs/src/lifetime.rs` · `LifetimeSystem` — counts each `Lifetime` down and removes the
  entity at zero.

**Why entity ids are never recycled.** Ids come from a process-wide monotonic counter, and a
removed entity's generation entry is kept, so a stale id reads as "not alive" instead of
"belongs to someone else". Undo and Play/Stop restore depend on it.
`World::create_entity_with_id` brings an entity back under its *original* id (in redo,
Cut-undo, delete-undo and the Play/Stop snapshot), so selections and later undo entries that
name it stay valid. That is only safe because no other
entity can ever have been given that id in the meantime.

- `crates/ecs/src/entity.rs` · `EntityId::new` — the `LazyLock` id generator; every id is
  `{ id, generation: 1 }`.
- `crates/ecs/src/world.rs` · `World::remove_entity` — marks the generation dead and keeps the
  entry.
- `crates/ecs/src/world.rs` · `World::create_entity_with_id` — revival under the same id, for
  snapshot restore and undo.

## Pitfalls here

- **Don't recycle entity ids "for efficiency."** Recycling breaks undo, which is id-exact, and it
  breaks snapshot restore. Generational ids would turn the resulting bugs from silent aliasing
  into "not alive" errors, but the editor would still lose its references.
- **Dead generation entries accumulate.** Every despawn leaves one tombstone in
  `World`'s generation map until `World::clear`. That is cheap per entry, but it is unbounded
  over a long session that spawns constantly. It's recorded as an accepted tradeoff on
  insiculous_2d#86. Never "fix" it by recycling ids.
- **A full particle pool is slow to spawn into.** `next_slot` walks the whole pool before
  falling back to overwrite. Size the pool so it never fills. Current bursts (tens of particles
  living under a second) stay far below 16,384.
- **Don't own a second `LifetimeSystem`.** The frame tail already runs one for every game. A
  game that steps its own would halve every lifetime.
- **Pooling entities would fight the ECS.** A "dead" pooled entity is still alive to every query,
  physics sync and the renderer, unless every system learns to skip it. If entity churn ever shows
  up in a profile, look at [data-locality](data-locality.md) and the storage decision first.
