# Component

Book: https://gameprogrammingpatterns.com/component.html

Intent: "Allow a single entity to span multiple domains without coupling the domains to each other."

## The problem it solves

A game object touches rendering, physics, audio, input and AI all at once. If one class holds
all of that, every domain is reachable from every other: the physics code can see the sprite,
the AI can see the audio voice, and the class grows until no one change is local. Inheritance
can't fix it, because the domains don't nest. A thing that is physical but invisible and a thing
that is visible but not physical can't share one sensible base class. The pattern breaks the
object into one piece per domain. The entity becomes the thing those pieces have in common, and
each domain only ever touches its own piece.

## Reach for it when

- One kind of object spans domains that should stay ignorant of each other, such as a paddle
  that is drawn, animated and collided.
- A class keeps growing in every direction, and most of its methods only use a third of its
  fields.
- You need to mix behaviors freely: some things need only physics, some only a sprite, and you
  keep wishing for multiple inheritance.

## Don't reach for it when

- The object lives in a single domain. A menu row or a particle doesn't need an entity made of
  parts. A plain struct is simpler and faster.
- The parts are always present together and always change together. That is one component,
  not three.
- You'd be splitting purely for tidiness. Every split is another lookup, and another way for the
  pieces to disagree.

## Tradeoffs

- **Wiring cost.** An entity is now several objects that someone has to assemble, and you can
  forget a piece. A ball with a `RigidBody` but no `Collider` compiles and silently doesn't
  collide.
- **Talking between parts.** The book gives three ways for components to reach each other:
  shared state on the container, direct references between siblings, or messages. Each one
  gives back some of the decoupling you split for. Use whichever is least coupled and still
  works.
- **Indirection.** A type-erased lookup (`Box<dyn …>` behind a map) costs a hash and a pointer
  chase on every access. That is cheap for hundreds of entities. Data locality is the pattern for
  when it stops being cheap.
- **Cache upside.** If the storage is per type, a pass over one component type touches only
  that type's data. This engine has deliberately not cashed that in yet (see below).

## In Rust

The book's C++ version has the entity own pointers to component objects, and the components
hold back-pointers to each other. In Rust, that web of mutable aliases fights the borrow checker
at every step, which is why idiomatic Rust lands on ECS, the entity-component-system form:

- The **entity is an id**, not an object. Usually it is an index plus a generation, so a stale id
  can be detected instead of dangling.
- **Components are plain data** (`#[derive(Clone, Serialize)]` structs). They live in storage
  owned by a `World` and are keyed by entity, and they carry no back-references.
- **Behavior lives in systems** that borrow the world, read some component types and write
  others. "Components talk through shared state" becomes "systems talk through the
  components they share", and the borrow checker lets only one system hold `&mut World` at a
  time.

An illustrative sketch, not engine code:

```rust
struct Position(Vec2);
struct Velocity(Vec2);

fn integrate(world: &mut World, delta_seconds: f32) {
    for entity in world.query::<(Position, Velocity)>() {
        let velocity = world.get::<Velocity>(entity).0;
        world.get_mut::<Position>(entity).0 += velocity * delta_seconds;
    }
}
```

Rust's real tension is that one system wants two `&mut` components of one entity at once. The
options are a sequential read-then-write (this engine), split borrows over separate per-type
storages, or interior mutability. Each one is a decision, not a detail.

## In insiculous_2d

**Verdict: textbook, kept simple on purpose.** The ECS is hand-written and the whole engine sits
on it. The July 2026 audit counted it as textbook already.

- `crates/ecs/src/world.rs` · `World` — owns entities, the per-type component stores, typed
  resources and the event bus. Every entity and component operation goes through it.
- `crates/ecs/src/entity.rs` · `EntityId` — `{ id, generation }`, so a stale id reads as "not
  alive" instead of aliasing a new entity.
- `crates/ecs/src/component.rs` · `ComponentStore` — `HashMap<EntityId, Box<dyn Component>>`,
  one store per `TypeId` inside `ComponentRegistry`. Any `Any + Send + Sync` type is a component
  through a blanket impl.
- `crates/ecs/src/world.rs` · `World::query_entities` — `Pair<A, B>`-style queries return an
  owned `Vec<EntityId>`, so a system can remove entities while it walks the result.
- `crates/ecs/src/world.rs` · `World::get_mut` — takes the id by value and returns `Option`.
  There is no way to borrow two components mutably at once.
- `crates/ecs/src/entity_builder.rs` · `EntityBuilder::with` — `world.spawn().with(…).id()`
  assembles an entity from its parts.
- `crates/physics/src/components.rs` · `RigidBody` — the physics crate owns its own components.
  The ECS crate never learns what a rigid body is. That is the decoupling the pattern promises,
  held at the crate boundary.

pong's paddle is the pattern at full width. One entity spans naming, transform, rendering,
animation, an animation state machine and physics, and each piece belongs to the domain that
reads it:

```rust excerpt=../games/pong/src/spawning.rs
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(Vec2::new(x, 0.0), 0.0, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(tong_machine(facing, Jaw::Open))
        .with(RigidBody::new_kinematic().with_rotation_locked(true))
        .with(Collider::new(jaw_collider(Jaw::Open, facing))
            .with_friction(0.0)
            .with_restitution(1.0))
        .id()
```

- `../games/pong/src/spawning.rs` · `spawn_paddle` — the excerpt above.

Components talk the book's first way, through shared state. `LifetimeSystem` reads only
`Lifetime`, and the physics system reads `Transform2D` plus its own `RigidBody`/`Collider`, then
writes back `Transform2D`. No component holds a reference to another.

- `crates/ecs/src/lifetime.rs` · `LifetimeSystem` — a whole domain in one component plus one
  pass.
- `crates/physics/src/physics_system/sync.rs` · `sync_physics_to_ecs` — physics writes its
  results back into the shared `Transform2D`.

**Adding a component** has up to five integration points (registry, scene RON, snapshot, command
API, inspector). Use `/add-component`, not this file.

## Pitfalls here

- **The boxed-downcast trap.** On a `Box<dyn Component>`, calling `.as_any()` hits the blanket
  impl on the *Box* and the downcast fails silently. Always go through `.as_ref().as_any()`. A
  guard test in `crates/ecs/src/component.rs` holds this.
- **No two `&mut` at once.** Read what you need from the first component with `get`, then
  `get_mut` the second (the `crates/ecs/CLAUDE.md` § Critical Patterns example).
- **`Box<dyn Component>` doesn't clone.** Anything that copies entities (`WorldSnapshot`, the
  clipboard, duplication) downcasts to each registered concrete type and calls its own `Clone`.
  An unregistered component is lost on Play/Stop.
- **Storage layout is a recorded decision, not an oversight.** The HashMap-of-boxes store stays
  until its trigger fires. See [data-locality](data-locality.md) and `crates/ecs/CLAUDE.md`
  § Storage before you propose sparse sets or archetypes.
