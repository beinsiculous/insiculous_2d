# ECS Crate — Agent Context

You are working in the ECS (Entity Component System) crate. This is the data backbone of the engine.

## Architecture
```
ComponentRegistry (HashMap<TypeId, ComponentStore>)
└── ComponentStore (HashMap<EntityId, Box<dyn Component>>)

Query types: Single<T>, Pair<T, U>, Triple<T, U, V>
```

## Key Types
- `World` — owns entities + ComponentRegistry. All entity/component operations go through here.
- `EntityId` — newtype around u64
- `Component` trait — blanket impl over `Any + Send + Sync` (component.rs:22). Any such type is a Component automatically; the trait only adds `type_name()` / `as_any()` / `as_any_mut()` for downcasting. It does NOT require Debug, Serialize, Deserialize, or Clone (those are needed separately for scene serialization, inspector, and snapshots)
- `ComponentMeta` trait — type_name(), field_names() for inspector/registry
- `WorldHierarchyExt` — set_parent(), get_children(), get_descendants(), get_root_entities()

## Built-in Components
- `Transform2D` — position (Vec2), rotation (f32), scale (Vec2)
- `GlobalTransform2D` — computed world-space transform
- `Sprite` — texture_handle, offset, rotation, scale, color, depth, tex_region
- `Camera` / `Camera2D` — viewport, zoom, main camera flag
- `Name` — entity display name
- `AudioSource`, `AudioListener` — audio components
- `SpriteAnimation` — named-clip animation over a `SheetGrid` (`clips: Vec<(String, AnimationClip)>`, played by name via `play`/`ensure_playing`). `AnimationClip` is NOT a component — it lives inside. While a clip is selected and its frame resolves, the component owns `Sprite.tex_region`
- `Tilemap` — row-major tile grid drawn from a tileset (`sprite_instances()` yields plain data; engine_core expands to the sprite batch)
- `Scripts` (`script.rs`, #44 Stage 1) — `Scripts(Vec<ScriptRef>)`, `ScriptRef { script_id, source_path, params: BTreeMap<String, ScriptValue> }` — the scripting seam as INERT data (nothing executes it yet); Entity params live as ids at runtime, persist by Name on the wire
- `UiLabel` / `UiPanel` / `UiButton` — data-driven screen-space UI (`ui_components.rs`): `UiAnchor` 9-point anchor + pixel offset (NO Transform2D), serde defaults on every field; `@key` text localizes; drawn by engine_core's `ui_element_system`

Note: `RigidBody` and `Collider` are NOT defined in this crate — they live in
`crates/physics/src/components.rs`. They are stored in the ecs `World` as
components like any other type, but the physics crate owns their definitions.

## File Map
- `hierarchy_system.rs` — dirty-flagged transform propagation (value-compare cache; call `reset()` after wholesale world replacement).
- `tilemap.rs` — row-major tile grid from tileset (top-left-tile anchor, tile 0 = empty, default depth -1.0).
- `component_registry/` — dynamic component tier: name-keyed `register::<T>()` fn-pointer table; `register_transient` for editable-never-persisted components.
- `sprite_system.rs` — `SpriteAnimationSystem`: advances clips and writes UV into `Sprite.tex_region`, scheduled with time-scaled delta so pausing freezes animation.

## Critical Patterns
- **Adding components**: `world.add_component(&entity, Transform2D::new(pos)).ok()`
- **Queries**: `world.query_entities::<Pair<Transform2D, Sprite>>()`
- **Typed access**: `world.get::<Transform2D>(entity)` / `world.get_mut::<Sprite>(entity)` — take `EntityId` by value, return `Option`. There is no `get_two_mut`; to touch two components on one entity, read what you need from the first (`get`), then `get_mut` the second sequentially:
  ```rust
  let offset = world.get::<Sprite>(entity).map(|s| s.offset);
  if let (Some(offset), Some(transform)) = (offset, world.get_mut::<Transform2D>(entity)) {
      transform.position += offset;
  }
  ```
- **Type-erased enumeration**: `world.component_types(entity)` -> `Vec<(TypeId, &'static str)>` — the only way to see components you don't know the type of (snapshot loss detection); names come from the concrete component via `.as_ref().type_name()`
- **New components**: derive `DeriveComponentMeta` + `ecs::register_components(|r| r.register::<T>())` (engine types go in the builtin list; game types register in main()) — scene save/load, WorldSnapshot, clipboard, and the command API then cover T automatically. A typed editor line in `editor_component_registry!` is OPTIONAL (buys rich field editors; otherwise the inspector shows a read-only serde view)

## Storage — GPP-02 decision of record (Jul 13 2026)
`ComponentStore` = `HashMap<EntityId, Box<dyn Component>>` is the accepted
simplicity tradeoff. **Trigger to revisit:** profiling shows component access
dominating a frame, or games routinely exceed ~a few thousand live entities.
When it fires, evaluate a **sparse-set layout FIRST** (dense `Vec<T>` per type
+ entity→index map — cache contiguity without archetype migration; the editor
adds/removes components constantly, a workload archetype migration punishes and
HashMaps tolerate); full archetype storage only if sparse sets measurably
aren't enough.

## Documented Conventions
- Typed accessors `get`/`get_mut` take `EntityId` by value; CRUD methods (`add_component`, `remove_component`, `has_component`, `get_component`) take `&EntityId`. Prefer by-value for new APIs.
- `Children` uses a `Vec<EntityId>` deliberately — child order is load-bearing for the editor hierarchy panel and scene serialization. Do not swap to `HashSet`.
- `world.entity_ids()` yields an iterator without allocating, whereas `world.entities()` allocates an owned `Vec<EntityId>`.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| When downcasting a `Box<dyn Component>`, calling `.as_any()` directly hits the blanket impl on the Box; call `.as_ref().as_any()` instead | `src/component.rs test_boxed_component_downcasts_only_through_as_ref_as_any` |
| Manual writes to `GlobalTransform2D` are not change-tracked and get overwritten by hierarchy propagation when the entity is dirty | `tests/hierarchy_dirty.rs test_hand_written_global_transform_is_discarded_once_the_entity_goes_dirty` |
| Circular hierarchy references during reparenting create invalid cyclic trees and are rejected | `tests/world.rs test_set_parent_rejects_cycles_and_names_the_cycle` |
| `World::update` swaps `systems` out so a system cannot reach or mutate the system list during update | `tests/system_lifecycle.rs test_a_panicking_system_does_not_stop_later_systems_from_updating` |
| Event bus events stay readable until flushed at the end of the frame; the next frame starts empty | `tests/world.rs test_events_stay_readable_until_flush_then_the_next_frame_starts_empty` |
| `Box<dyn Component>` is not clonable: anything that copies components (`WorldSnapshot`, duplication) downcasts to each concrete type and calls its own `Clone` | — none |


## Testing
- `cargo test -p ecs` — 0 failed, 0 ignored
- Integration tests in `tests/world.rs`, unit tests inline in source
- Naming: `test_<behavior_description>`

## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

| Our Concept | Godot Equivalent | File |
|-------------|-----------------|------|
| World / hierarchy | Scene tree | `scene/main/node.cpp` — `add_child`, `remove_child`, `get_children` |
| Component + ComponentMeta | Object properties | `core/object/object.cpp` — `set`, `get`, `get_property_list` |
| Entity duplication | Node::duplicate | `scene/main/node.cpp` — search `duplicate` |
| Entity deletion | Node::queue_free | `scene/main/node.cpp` — search `queue_free`, `remove_child` |
| Transform propagation | Node2D transforms | `scene/2d/node_2d.cpp` — how transforms chain |

**Remember:** Godot uses scene tree + properties. Adapt *design patterns* to our Rust ECS, don't copy C++.
