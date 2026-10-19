# Spatial Partition

Book: https://gameprogrammingpatterns.com/spatial-partition.html

Intent: "Efficiently locate objects by storing them in a data structure organized by their positions."

## The problem it solves

"What is near this point?" is the question behind collisions, picking, aggro ranges, area damage
and audio falloff. Asked naively, it compares every object against every other object, so the
cost is quadratic: double the objects and the work goes up four times. A spatial partition files
objects by where they are, in a grid, a quadtree, a BSP or a bounding-volume hierarchy, so a query
looks only at the few cells or nodes that overlap the area it asks about. Everything else is
skipped without being looked at.

## Reach for it when

- There are many positioned objects, and you ask location queries often enough that they show
  up in a profile.
- The query is the same every frame, such as broad-phase collision, so the setup pays for itself
  over and over.
- The objects cluster unevenly and a flat list keeps scanning empty space. That calls for an
  adaptive partition.

## Don't reach for it when

- The object count is small. A linear scan over a few hundred items is a few microseconds and
  has no bugs in it. The book says plainly that the pattern only pays at large counts.
- A library you already depend on owns the problem. Two partitions over the same objects drift
  apart.
- The world is a grid already. A board game or tilemap is its own partition, because the index
  *is* the position.

## Tradeoffs

- **Moving objects must be refiled.** Every movement is a remove and an insert, or a refit.
  Moving objects favor a structure that is cheap to update (a uniform grid, or a BVH that is
  refitted rather than rebuilt).
- **Flat or hierarchical.** A flat grid is simple and has predictable memory, but wastes cells on
  empty space and struggles when objects pile up in one cell. A hierarchy adapts to density at
  the cost of complexity.
- **Fixed or adaptive.** Fixed subdivision (a grid, or a quadtree over fixed bounds) is
  insertion-order independent. Adaptive subdivision (BSP, k-d tree, BVH) balances better, but
  rebalancing costs time.
- **Where objects live.** If the partition is the only place an object lives, the partition is
  your storage. If it is an index over objects stored elsewhere, you must keep the two in step.
- **Memory.** It all comes on top of the objects themselves.

## In Rust

Store **ids, not references**: a grid of `Vec<EntityId>` cells, or a tree of index nodes,
rebuilt or refitted from the positions once per frame. That avoids aliasing the objects the ECS
owns, and lets a system read the partition while it holds `&mut World` for other work. For
anything physical, use the crate's own structure. rapier keeps a broad phase for its colliders
and a query pipeline for ray casts and point and shape queries.

An illustrative sketch of the simplest form, not engine code:

```rust
struct UniformGrid {
    cell_size_pixels: f32,
    cells: HashMap<IVec2, Vec<EntityId>>,
}

impl UniformGrid {
    fn cell_of(&self, position_pixels: Vec2) -> IVec2 {
        (position_pixels / self.cell_size_pixels).floor().as_ivec2()
    }

    fn insert(&mut self, entity: EntityId, position_pixels: Vec2) {
        let cell = self.cell_of(position_pixels);
        self.cells.entry(cell).or_default().push(entity);
    }
}
```

## In insiculous_2d

**Verdict: delegated to rapier, and that is the right call.** The engine's one hot spatial
question, which pairs of colliders might touch, is answered by rapier's broad phase inside
`PhysicsWorld`. The July 2026 pattern audit recorded this as correctly delegated. Everything else
that asks "what's near" is a linear scan over counts too small to need more.

- `crates/physics/src/physics_world/mod.rs` · `DefaultBroadPhase` — the broad phase `PhysicsWorld`
  owns and hands to every step. It stays in sync with the colliders automatically, because rapier
  owns both.

```rust excerpt=crates/physics/src/physics_world/stepping.rs
        self.physics_pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
```

- `crates/physics/src/physics_world/stepping.rs` · `step` — one fixed physics step. The
  broad phase runs inside it.

The linear scans that remain, and why each one is fine for now:

- `crates/editor/src/picking/mod.rs` · `pick_at_world_pos` — a click tests each pickable's AABB.
  It runs once per click over one scene, not once per frame. The picking accuracy gaps
  (unrotated boxes, sprites only, a margin in world units) are tracked on the board separately.

```rust excerpt=crates/editor/src/picking/mod.rs
        let mut hits: Vec<(EntityId, f32)> = entities
            .iter()
            .filter(|e| {
                let aabb = e.aabb().expand(self.pick_margin);
                aabb.contains_point(world_pos)
            })
            .map(|e| (e.entity_id, e.depth))
            .collect();
```

- `crates/engine_core/src/behavior_runner/mod.rs` · `find_nearest_tagged_position` — each
  chasing or camera-following behavior scans every entity for the nearest tagged one, every frame.
  That is O(behaviors × entities). It is the first place a partition, or simply a once-per-frame
  list of tagged positions, would pay, and only once scenes carry many chasers.
- `crates/engine_core/src/behavior_runner/mod.rs` · `check_tagged_overlap` — the same scan for
  "is a tagged entity within this radius?"

What is *not* a spatial index, despite the name:

- `crates/engine_core/src/grid/grid_mesh.rs` · `GridMesh` — the `grid` module is a **visual effect**,
  a spring-mass lattice drawn as glowing lines that ripples when hit. Its impulses walk every node
  because its step walks every node anyway. Don't store gameplay objects in it.
- `crates/ecs/src/tilemap.rs` · `Tilemap` — a row-major board where `tiles[row * width + column]`
  is the lookup. Like snake's cell grid and frogger's terrain rows, it is a board that is its own
  index, not a partition over moving objects.

## Pitfalls here

- **Don't build a second broad phase for colliders.** rapier already has one, and a second index
  over the same bodies goes stale the moment physics moves them. For "what's at this point" or
  "what does this ray hit" over colliders, rapier's query pipeline is the tool. `PhysicsWorld`
  doesn't expose one yet, so adding it is a physics-crate seam, not a game-side grid.
- **Measure before you partition a behavior scan.** The behavior helpers walk the
  non-allocating `world.entity_ids()`, so a scan costs a hash lookup per entity and no
  allocation. A partition only earns its keep at hundreds of chasers.
- **Rotated objects aren't their AABB.** Picking uses unrotated boxes, so a 45° sprite selects
  from well outside its art. Any coarse spatial test needs an exact second pass on the
  candidates it returns.
