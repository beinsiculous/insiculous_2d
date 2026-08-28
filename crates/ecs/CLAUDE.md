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
- `world.rs` — World struct, entity/component CRUD
- `component.rs` — Component trait, ComponentStore
- `query.rs` — Type-safe query system (Single, Pair, Triple)
- `hierarchy_extension.rs` — Hierarchy operations (WorldHierarchyExt trait)
- `hierarchy_system.rs` — Dirty-flagged transform propagation (value-compare cache; clean frames recompute nothing; `reset()` after wholesale world replacement)
- `lifetime.rs` — `Lifetime` component + `LifetimeSystem` (auto-despawn after N seconds; bullets/effects)
- `tilemap.rs` — `Tilemap` component + `TileInstance` (top-left-tile anchor, row 0 on top, tile 0 = empty, depth default -1.0)
- `component_registry/{mod,tests}.rs` — THE dynamic component tier (#43): name-keyed `ComponentEntry` fn-pointer table (create/insert/extract/remove/has/default, monomorphized at `register::<T>()`); global = `OnceLock<RwLock<..>>` with `register_components(f)` (late registration OK, name collision = panic, poison recovered, re-entrancy = clear panic) + `with_global_registry(f)`; `register_transient` = editable-never-persisted (PlaySoundEffect). Dynamic components must NOT store raw EntityId
- `sprite_components.rs` — Built-in component definitions (incl. `AnimationClip` + `SpriteAnimation`)
- `sprite_system.rs` — `SpriteAnimationSystem`: advances each clip, then writes `current_uv()` into `Sprite.tex_region`. Scheduled by engine_core's `game/frame_tail.rs` with the time-scaled delta (so pausing freezes animation); nothing in ecs drives it
- `ui_components.rs` — UiAnchor + resolve_anchored_pos + UiLabel/UiPanel/UiButton

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

## Documented Conventions
- Typed accessors `get`/`get_mut` take `EntityId` by value; CRUD methods (`add_component`, `remove_component`, `has_component`, `get_component`) take `&EntityId`. Prefer by-value for new APIs.
- `Children` uses a `Vec<EntityId>` deliberately — child order is load-bearing for the editor hierarchy panel and scene serialization. Do not swap to `HashSet`.

## Common Pitfalls
- `Box<dyn Component>` is NOT clonable — there is no `dyn_clone`/`CloneComponent` machinery. Storage is plain `HashMap<EntityId, Box<dyn Component>>`; anything that needs to copy components (e.g. `WorldSnapshot`, entity duplication) downcasts to each known concrete type and calls its own `Clone`
- When downcasting a `Box<dyn Component>`, call `.as_ref().as_any()` (or `.as_mut().as_any_mut()`) — calling `.as_any()` directly on the Box hits the blanket impl on the Box itself, not the concrete type (see component.rs comments)
- TypeId is per-concrete-type — different generic params = different TypeIds
- `GlobalTransform2D` is system-owned (computed by `TransformHierarchySystem`); manual writes to it are NOT change-tracked and get overwritten the next time the entity is dirty. Edit `Transform2D` instead
- Always check for circular references when reparenting in hierarchy
- serde_json for inspector, RON for scene files — both must work

## Testing
- 211 passing (incl. 10 doc tests), 0 ignored — `cargo test -p ecs`
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
