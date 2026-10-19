# Service Locator

Book: https://gameprogrammingpatterns.com/service-locator.html

Intent: "Provide a global point of access to a service without coupling users to the concrete class that implements it."

## The problem it solves

Some systems are wanted almost everywhere: audio, asset loading, input, logging. Threading
each one through every function signature by hand is tedious. Making each one a singleton
welds every caller to one concrete type, and to the moment that type got created. A locator
sits in between. Callers ask one known place for "the audio service" and get whatever was
registered there: the real device, a silent stand-in for tests, or a logging wrapper. They
never name the concrete type and never construct it.

## Reach for it when

- A service really is ambient. Many unrelated callers need it, and passing it explicitly would
  thread it through layers that don't care.
- You need to swap the implementation without touching callers: a real device against a
  headless one, a filesystem against a stub, native against web.
- A missing service should degrade instead of crash. The locator can hand back a null service
  that accepts every call and does nothing.

## Don't reach for it when

- You can just pass the thing in. The book says to use this pattern sparingly, and an explicit
  parameter beats a lookup for readability every time.
- Only one module uses the service. Then it is a field of that module, not a service.
- The lookup would hide a real dependency that tests need to see. A hidden dependency is one
  that a test forgets to set up.

## Tradeoffs

- **The service might not be there.** Every lookup site must handle absence: an assert, a
  `Result`, or a null service. A null service is usually the kindest choice.
- **Hidden coupling.** Callers depend on the locator without saying so in their signatures.
  Reading a function no longer tells you what it touches.
- **Context-blind services.** A service handed to anyone must work from any calling context, so
  it can't assume it runs on a particular thread, phase or borrow.
- **Global by default.** A process-wide locator carries every cost of [singleton](singleton.md).
  Scoping the locator to a context object keeps most of the benefit and loses most of the cost.

## In Rust

A literal global locator in Rust is a `static` holding a `OnceLock<Box<dyn Service>>`, with
`Send + Sync` bounds and a lock around anything mutable. It works, and it is awkward on
purpose. The idiomatic Rust answer is the **context struct**: one value, built once per frame
or per call, holding `&mut` borrows of each service, and passed to the code that needs them.
The borrow checker then proves that no two callers hold the same service mutably. Swapping an
implementation means building the context differently, usually behind a trait object or a
constructor like `headless()`.

An illustrative sketch, not engine code:

```rust
trait AudioService { fn play(&mut self, sound: SoundId); }
struct SilentAudio;
impl AudioService for SilentAudio { fn play(&mut self, _sound: SoundId) {} }

struct FrameContext<'frame> {
    audio: &'frame mut dyn AudioService,
    world: &'frame mut World,
}
```

A `TypeId`-keyed resource map (`HashMap<TypeId, Box<dyn Any>>`) is the dynamic version. Lookup
is by type, and absence comes back as `Option`.

## In insiculous_2d

**Verdict: textbook, done by passing a context, not by global lookup.** The July 2026 audit
counted it as textbook already. Games never reach for a global service: every `Game` method
receives a `GameContext`.

- `crates/engine_core/src/contexts.rs` · `GameContext` — input, players, world, assets, audio,
  UI, achievements, scores, particles, lines, strings and scripts, each a `&mut` borrow of the
  runner's own field, plus the frame's `delta_time` and `window_size`.
- `crates/engine_core/src/game.rs` · `build_context` — the one macro that builds it. The window
  loop and `GameHarness` share it, so a game sees the same services in a test.

```rust excerpt=crates/engine_core/src/game.rs
        crate::contexts::GameContext {
            input: &$runner.input,
            players: &mut $runner.player_input,
            world: &mut $runner.scene.world,
            assets: $assets,
            audio: &mut $runner.audio_manager,
            ui: &mut $runner.ui,
```

- `crates/engine_core/src/contexts.rs` · `RenderContext` — the render-time counterpart, which
  gets the world read-only.

**Null services instead of absent ones.** Each platform service falls back to a stand-in that
accepts every call:

- `crates/audio/src/manager/mod.rs` · `AudioManager::new_or_disabled` — no device, or the web
  before a gesture, means a disabled manager whose playback does nothing.

```rust excerpt=crates/audio/src/manager/mod.rs
        match Self::new() {
            Ok(manager) => manager,
            Err(e) => {
                log::warn!("Failed to initialize audio: {}. Audio will be disabled.", e);
                Self::disabled()
            }
        }
```

- `crates/engine_core/src/gamepad_backend.rs` · `GamepadBackend::new_or_disabled` — no gilrs
  means a backend that never produces events.
- `crates/engine_core/src/assets.rs` · `AssetManager::headless` — handles are issued and paths
  recorded, but nothing is uploaded. `GameHarness` runs on it.

**Swappable seams by trait.**

- `crates/engine_core/src/texture_ref.rs` · `TextureResolver` — everything a scene load needs
  from the filesystem and the GPU. `AssetManager` is the production implementation, and tests
  pass a stub.

**Typed resources for cross-system state.**

- `crates/ecs/src/world.rs` · `World::resource_mut` — at most one value per type, absent as
  `None`.
- `crates/ecs/src/blackboard.rs` · `Blackboard` — a string-keyed resource for scripts to
  coordinate. It is transient: cleared at Play, never saved.

## Pitfalls here

- **Add a service to `GameContext`, not a static.** A new engine service becomes a runner field
  plus one line in `build_context!`. Because the harness builds the context through that same
  macro, the service is headless-testable for free. A static breaks house rule 3 and dodges the
  harness.
- **`time_scale` freezes the whole engine tail, not just particles.** The frame tail scales
  particles, sprite animation, lifetimes, clip machines and grid backdrops by it, so `0.0`
  freezes all of them. `ctx.delta_time` is never scaled, so a game gates its own logic.
- **Disabled is not an error.** `AudioManager::is_enabled()` is the only way to tell that a
  disabled manager is silent. Don't branch gameplay on audio success.
- **The context is wide on purpose.** Its fields are disjoint borrows of separate runner fields,
  and the compiler checks every one of them. A width that is just a list of disjoint borrows is
  not a god object. Regrouping the runner's fields is a deferred backlog item (it waits for a
  fourth manager), not a cleanup to do unprompted.
- History: `log_archive.md` § "Game Programming Patterns Audit (July 13, 2026) — CLOSED".
