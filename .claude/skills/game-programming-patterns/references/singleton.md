# Singleton

Book: https://gameprogrammingpatterns.com/singleton.html

Intent: guarantee one instance of a type and hand everyone a global way to reach it, which the book argues is usually the wrong trade.

## The problem it solves

Some things really do exist once: the audio device, the log, the file system. The classic
pattern makes the type construct itself lazily, refuse a second instance, and expose a global
accessor, so nobody has to pass it around. The book's chapter is mostly a warning. It points out
that the pattern bundles two separate promises, "only one" and "reachable from anywhere", and the
second is what hurts. A global is coupling nobody can see in a function signature. It is state
that every test shares, a lock or a data race waiting to happen once threads arrive, and an
initialization order you don't control.

## Reach for it when

- Something is genuinely process-wide **and** has to be reached from places no handle can get
  to: a callback from the OS or the browser, a free function exported to JavaScript, or a
  logger.
- The state is written once at startup and only read afterwards, so sharing it can't cause
  surprises.
- The alternative is threading a handle through a dozen APIs that would otherwise never need
  it, *and* you can show that the global can't make two tests interfere.

## Don't reach for it when

- You want "only one". Construct one, own it, and pass it. That one rule removes most
  singletons.
- You want "convenient access". Put it on the context the code already receives
  ([service-locator](service-locator.md)) or in a World resource.
- The state changes during play. Mutable global game state is exactly what the book is warning
  against.
- Tests would need to reset it between runs. If a test has to clean up a global, the global is
  in the wrong place.

## Tradeoffs

- **Hidden coupling.** A function that reads a global depends on it without saying so, and no
  signature, mock or reviewer can see that.
- **Tests share it.** `cargo test` runs tests as threads in one process. A process global is
  shared by every test at once, while a `thread_local!` belongs to each test thread separately.
  Both are different from production.
- **Initialization order.** A lazy global initializes on first touch, wherever that happens to
  be. An eager one needs someone to call it before everything else.
- **The upside is real but narrow.** Where no handle can reach, as at a platform boundary, a
  global is the honest answer and a contortion would be worse.

## In Rust

Rust makes the pattern deliberately awkward, which is the book's point enforced by the
compiler. A `static` has to be `Sync`. A mutable global needs `OnceLock`/`LazyLock` (initialize
once), a `Mutex`/`RwLock`, atomics, or `thread_local!`, and `static mut` takes `unsafe` (the
2024 edition denies references to one by default). Every global therefore announces its
synchronization in its type.

The idiomatic replacement is ownership plus passing. Own one value in a long-lived struct, and
lend `&` or `&mut` to whoever needs it. When a collection of systems needs shared, typed
"there's one of these" state, a resource map keyed by `TypeId` on the World gives singleton
semantics without making the state global to the process. An illustrative sketch, not engine
code:

```rust
struct Score { points: u32 }

fn on_brick_broken(world: &mut World) {
    if let Some(score) = world.resource_mut::<Score>() {
        score.points += 10;
    }
}
```

## In insiculous_2d

**Verdict: partial, with a reason for each.** The engine has two true process globals, the
component registry and the entity id generator. Every other static is wasm-only or
playground-only and exists because a browser callback or a JavaScript export has no handle to
reach. The preferred replacement already exists: typed World resources, which the code itself
calls typed singletons.

```rust excerpt=crates/ecs/src/component_registry/mod.rs
static COMPONENT_REGISTRY: OnceLock<RwLock<ComponentRegistry>> = OnceLock::new();

thread_local! {
    /// Re-entrancy guard: `std::sync::RwLock` deadlocks on same-thread
    /// read→write (and may on read→read); catch it with a clear panic
    /// instead of a hang.
    static REGISTRY_LOCK_HELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
```

**Every process global left in `crates/`, with why each is tolerated.** Test-only statics such
as fixture counters and recording loggers are left out.

- `crates/ecs/src/component_registry/mod.rs` · `COMPONENT_REGISTRY` — the name-to-type table
  that the scene loader, serializer, snapshot, clipboard and command API all consult across
  crates. It is written at startup (the engine's built-ins, then a game's types before
  `run_game`) and read afterwards. Registration is idempotent, so tests sharing it can't
  disagree.
- `crates/ecs/src/component_registry/mod.rs` · `REGISTRY_LOCK_HELD` — per-thread guard that
  turns a same-thread re-entrant lock, which would deadlock, into a clear panic.
- `crates/ecs/src/entity.rs` · `GENERATOR` — an atomic counter behind `EntityId::new`. Ids have
  to be unique across *every* World in the process, because snapshot restore and undo recreate
  the same id through `create_entity_with_id`. A per-World counter would let two worlds mint
  the same id.
- `crates/engine_core/src/web/mod.rs` · `PAGE_EXITED` — wasm only. A one-way latch set from a
  `pagehide` listener, which has no handle on the runner.
- `crates/engine_core/src/web/mod.rs` · `IDLE_ACTIVE` — wasm only. Read by the hidden-page pump
  timer.
- `crates/engine_core/src/web/mod.rs` · `RENDER_FATAL` — wasm only. A one-way latch that stops
  the pump from re-driving a dead render path.
- `crates/engine_core/src/web/mod.rs` · `WAKE_PROXY` — wasm only. The running loop's proxy,
  published so browser timers can drive frames.
- `crates/engine_core/src/web/mod.rs` · `PUMP_ARMED` — wasm only. Makes sure only one pump timer
  chain runs.
- `crates/engine_core/src/save_store/mod.rs` · `BACKEND` — wasm only. localStorage, or the memory
  fallback, chosen once per page.
- `crates/common/src/vfs/mod.rs` · `MEM_FS` — wasm only. The in-memory file system that sits
  behind the free functions `common::vfs::read`/`write`. On native, the real file system (also
  process-wide) takes its place.
- `crates/common/src/vfs/mod.rs` · `WRITE_OBSERVER` — wasm, plus native tests. The playground's
  persistence hook, installed once.
- `crates/renderer/src/window.rs` · `INSTALLED` — wasm only. A guard that installs the
  focus-before-winit listeners only once.
- `crates/playground/src/web_entry.rs` · `ACTIVE_STORE` — along with its sibling manifests,
  dirty flags and active manifest. `wasm_bindgen` exports are free functions called from
  JavaScript with no `self`, so the page's one project has to live somewhere reachable. One page
  holds one runtime.
- `crates/playground/src/bridge.rs` · `REQUEST_SENDER` — along with its siblings and
  `BRIDGE_GENERATION`. These are the bridge between the page and the editor, for the same
  reason.
- `crates/playground/src/persist/mod.rs` · `ACTIVE_CHAINS` — wasm only. The save chains that
  browser events drive.
- `crates/playground/src/preview_entry.rs` · `PREVIEW_CONTROLS` — the controls of the one
  preview runtime. winit refuses a second event loop, so a second load is refused here.

The id generator, the only global on the hot path:

```rust excerpt=crates/ecs/src/entity.rs
    pub fn new() -> Self {
        static GENERATOR: LazyLock<EntityIdGenerator> = LazyLock::new(EntityIdGenerator::new);
        Self {
            id: GENERATOR.generate_id(),
            generation: 1,
        }
    }
```

What to use instead:

- `crates/ecs/src/world.rs` · `World::insert_resource` — typed, World-scoped singleton state, for
  example the scripts' `Blackboard` or the grid's `GridImpulses`. It dies with the World, and
  each test's World has its own.
- `crates/engine_core/src/contexts.rs` · `GameContext` — the handles a game needs, passed into
  every `Game` method ([service-locator](service-locator.md)).
- `crates/audio/src/manager/mod.rs` · `AudioManager` — the precedent. Sound ids used to come
  from a global `static NEXT_ID`. They now come from a counter owned by each manager, so a
  fresh manager is deterministic.

## Pitfalls here

- **No new process global.** A new `static`, `OnceLock`, `LazyLock` or `thread_local!` outside a
  wasm entry point needs the same kind of reason as the list above, written beside it. "It's
  convenient" isn't one. Use a World resource or a `GameContext` field.
- **Never touch the registry from inside a registry closure.** Calling `register_components` or
  `with_global_registry` re-entrantly panics by design rather than deadlocking.
- **Tests share the two true globals.** Registration has to stay idempotent, and no test may
  assume an exact entity id value, because other test threads are minting ids at the same time.
- **Native tests never see the wasm globals.** A behavior that depends on one, such as the page
  latch or the pump, is checked in the browser, not by `cargo test`.
