# Prototype

Book: https://gameprogrammingpatterns.com/prototype.html

Intent: make new objects by copying a configured example instead of naming a class for every variant.

## The problem it solves

A game has dozens of kinds of thing that differ only in their settings: a red brick and a blue
brick, a fast ghost and a slow ghost. If every variant is its own class, or its own hand-written
spawn function, then every variant means more code, and a designer can't add one. The book's
answer is to keep one fully set-up example of each and to make new instances by copying it,
with the per-instance differences applied on top. Its stronger version moves the example out of
code into data. An entry in a data file names another entry as its prototype and lists only what
it changes, which is delegation for data.

The book is candid: as a *code* design pattern it found few cases where a clone method beat a
spawn function. The idea earns its keep as **data**. That is where this engine uses it.

## Reach for it when

- Many instances share one setup and differ in a few fields: bricks in a level, enemies on a
  wave.
- The variants should be authored in data (a scene file, the editor) rather than in Rust.
- You need a faithful copy of something live: duplicating a selection in the editor, or
  snapshotting a world before Play.

## Don't reach for it when

- There are two or three variants that rarely change. A spawn function with a small recipe
  struct is plainer and is checked by the compiler.
- The copy's semantics are murky. If some fields should be shared and others duplicated, you're
  about to learn the deep-versus-shallow question the hard way. Make the ownership explicit first.
- You'd be building a chain of prototypes three deep. Delegation reads well for one level and
  turns into archaeology after that.

## Tradeoffs

- **Copy semantics.** Copying deep is safe but costs memory; copying shallow shares state that
  someone will later mutate. Every prototype system has to answer which fields are which.
- **Overrides are the whole design.** "Copy, then change these" is only as good as the
  granularity of "these". Whole-object overrides force every instance to restate fields it
  didn't mean to change (see Pitfalls here).
- **Data loses the compiler.** A misspelled prototype name or a wrong field type is a load-time
  error instead of a compile error, so a data prototype needs good error messages.
- **One more indirection when reading.** To learn what an instance *is*, you read the
  prototype and then the overrides.

## In Rust

Rust builds the book's `clone()` method into the language as the `Clone` trait, and the type
system answers the deep-or-shallow question field by field. An owned `Vec` or `String` field is
deep-copied, an `Rc`/`Arc` field is shared, and a `&` field can't outlive what it borrows. So
`#[derive(Clone)]` *is* the prototype method, and it is honest about what it shares.

The friction is trait objects. `Box<dyn Trait>` is not `Clone`, because `Clone` returns `Self`
by value, which a `dyn` type can't do. The usual workaround is a `clone_box(&self) ->
Box<dyn Trait>` method on the trait. The alternative this engine takes is to downcast to each
registered concrete type and call its own `Clone`.

The book's own preferred alternative, the spawn function, fits Rust well as a recipe struct plus
a function. An illustrative sketch, not engine code:

```rust
struct EnemyRecipe { speed: f32, hit_points: u32 }

fn spawn_enemy(world: &mut World, recipe: &EnemyRecipe, position: Vec2) -> EntityId {
    world.spawn()
        .with(Transform2D::new(position))
        .with(Health(recipe.hit_points))
        .with(Speed(recipe.speed))
        .id()
}
```

## In insiculous_2d

**Verdict: partial, and in the right place.** The prototype lives in data, as scene-file
prefabs. Runtime code spawns through recipe functions, which is the alternative the book
prefers. The editor uses typed copying to duplicate and snapshot.

- `crates/engine_core/src/scene_data.rs` · `PrefabData` — a named list of components in a scene
  file's `prefabs` table. An `EntityData` names it with `prefab:` and applies `overrides`.
- `crates/engine_core/src/scene_loader.rs` · `SceneInstance::spawn_prefab` — spawns a prefab at
  runtime from the instance that loaded the scene. A failed component leaves no half-built
  entity behind.
- `crates/engine_core/src/scene_loader.rs` · `SceneLoader::apply_component_layer` — the override
  rule. A layer replaces the *whole* component of the same type, or appends it.

```rust excerpt=crates/engine_core/src/scene_loader.rs
        let entity_id = world.create_entity();
        let merged = SceneLoader::merge_components(&prefab.components, overrides, &[]);
        for component in &merged {
            if let Err(e) = SceneLoader::add_component_to_entity(entity_id, component, world, assets) {
                world.remove_entity(&entity_id).ok();
                return Err(e);
            }
        }
        Ok(entity_id)
```

breakout authors its bricks this way. Each level defines prefabs such as `Donut`, and every
brick is a one-line instance of one:

- `../games/breakout/assets/scenes/level1.scene.ron` · `Donut` — sprite, animation, a static
  body and a collider, shared by every Donut brick.

```ron excerpt=../games/breakout/assets/scenes/level1.scene.ron
        EntityData(name: Some("brick_r0_c0"), prefab: Some("Donut"), overrides: [Transform2D(position: (-306.0, 222.0), scale: (0.8, 0.4))]),
```

- `crates/engine_core/tests/prefab_spawning.rs` · `spawn_prefab` — the runtime path's tests.

The editor copies by type rather than through a `clone` on `Box<dyn Component>`:

- `crates/editor/src/clipboard.rs` · `capture_entity_tree` — copy and duplicate capture a whole
  subtree of registered components.
- `crates/editor/src/clipboard.rs` · `spawn_entity_tree` — the paste side, with fresh ids, an
  offset on the root only, and an optional name suffix.
- `crates/editor/src/world_snapshot.rs` · `WorldSnapshot::capture` — Play/Stop copies every
  entity and restores it on Stop.

Where code spawns, it uses recipes, not clones:

- `../games/space_invaders/src/spawning.rs` · `ShotRecipe` — one recipe each for the player's
  shot and the invaders', fed to `spawn_shot`.
- `crates/editor/src/archetype.rs` · `Archetype` — the editor's fixed set of entity factories,
  which the Entity menu and the command API share.

No game calls `spawn_prefab` at runtime today. That is a choice, not a gap. A game whose spawn
data should be editable without recompiling is the one that will want it.

## Pitfalls here

- **Overrides replace whole components, not fields.** An override `Transform2D(position: …)`
  with no `scale` deserializes to the default scale `(1.0, 1.0)` and *replaces* the prefab's
  scale. That is why every breakout brick restates `scale: (0.8, 0.4)`. The flip side: change a
  prefab's scale and no instance follows it. Field-level overrides are tracked as
  insiculous_2d#169.
- **Unregistered components don't copy.** Snapshot, clipboard and duplicate copy only the types
  the editor knows: the component registry plus a few internal entries. A generic component such as `StateMachine<MyState>`, attached directly,
  is reported by `WorldSnapshot::loss_warning` and dropped on Stop.
- **`spawn_prefab` doesn't track what it spawned.** The new entity is not added to the
  instance's named entities, so the caller owns its lifetime.
- **Prefabs don't chain.** A `PrefabData` cannot name another prefab. Variants of variants
  repeat their shared components.
