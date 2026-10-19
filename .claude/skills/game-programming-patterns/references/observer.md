# Observer

Book: https://gameprogrammingpatterns.com/observer.html

Intent: let one piece of code announce that something happened, without knowing who cares, while any number of listeners subscribe to hear it.

## The problem it solves

The physics code knows the hero just fell off a bridge. The achievement code wants to award "Fell
Off a Bridge". Calling the achievement system from inside physics welds two unrelated domains
together. Every new listener (audio, a tutorial hint, a stat counter) becomes another edit to
physics. In the book's form, the *subject* keeps a list of *observers* and notifies each one
when the event occurs. Physics knows only that someone might be listening.

## Reach for it when

- One event has several unrelated listeners, and the source shouldn't name any of them.
- The listener set changes at runtime (a tutorial that subscribes for its first five minutes).
- You're at a platform boundary that only speaks callbacks, such as the OS, the browser or the
  GPU driver. You can't change its shape, only contain it.

## Don't reach for it when

- The listener needs to *change* the world in response. A synchronous callback runs inside the
  subject's work, often while the subject holds the very state the listener wants to mutate. In
  Rust that's a borrow error, or a `RefCell` panic waiting to happen.
- Order matters. Observers run in subscription order, which is rarely the order the game needs.
- There is exactly one listener. Call it, or return a value.
- You can't say when a listener unsubscribes. That's the lapsed-listener leak.

## Tradeoffs

- **Coupling moves, it doesn't vanish.** It shifts from compile time to runtime. You can no
  longer grep for who reacts to an event; you have to run the game and look.
- **Synchronous notification blocks the subject.** A slow or re-entrant observer stalls the
  code that fired it, and an observer that fires another event can recurse.
- **Lifetime is the hard part.** A destroyed observer still in the list is a dangling reference.
  An observer nobody removes keeps the whole object graph it points to alive.
- **The book's own later view** is that observers work best as plain functions, and that a lot
  of what people build with observers is better as data flowing through a queue.

## In Rust

The C++ form, a subject holding pointers to live observers that mutate shared state, fights Rust
at every step:

- A `Vec<Box<dyn FnMut(&Event)>>` can't capture `&mut World`, because the subject is usually
  already borrowing it. People reach for `Rc<RefCell<…>>`, and the borrow error turns into a
  runtime panic.
- `Weak<RefCell<T>>` fixes dangling observers and leaves you with upgrade-or-skip at every
  notification.

So idiomatic Rust game code mostly *replaces* Observer:

- **An event queue.** The subject pushes plain data and listeners read it later, when they hold
  their own borrows. See [event-queue](event-queue.md).
- **A returned value.** In an immediate-mode UI, `if ui.button(…)` returns `true`, and the caller
  reacts in its own scope.
- **A latch.** At a callback-only boundary, the callback sets an atomic flag and nothing else;
  the main loop polls it. An illustrative sketch, not engine code:

```rust
let lost = Arc::new(AtomicBool::new(false));
let lost_for_callback = lost.clone();
driver.on_lost(move || lost_for_callback.store(true, Ordering::Relaxed));
// in the frame loop
if lost.load(Ordering::Relaxed) { stop_rendering(); }
```

## In insiculous_2d

**Verdict: deliberately avoided.** No engine API notifies through callbacks that run under a
world borrow. Collision callbacks existed once and were deleted outright: they fired under
`&mut World`, so they couldn't mutate anything, and nothing outside physics' own tests used them.
"Tell someone it happened" goes through the event bus, drained event vectors, deferred command
buffers or return values.

- `log_archive.md` · `CollisionCallback` — the record of the deletion and its reason.
- `crates/physics/src/physics_system/mod.rs` · `PhysicsSystem::take_collision_events` — the
  replacement. Take once per frame, own the `Vec`, mutate freely while you react.
- `crates/ecs/src/state_machine.rs` · `StateMachine` — its module doc states the house position:
  systems read the state and act on it, with no callbacks.
- `crates/ui/src/context/widgets.rs` · `UIContext::button` — immediate mode. The click is the
  return value, read in the caller's own scope:

```rust excerpt=crates/ui/src/context/widgets.rs
    /// Create a button widget.
    ///
    /// Returns `true` if the button was clicked this frame.
```

**Where callbacks survive: the platform edges, each one contained by a latch.**

- `crates/renderer/src/renderer.rs` · `set_device_lost_callback` — wgpu's device-lost callback
  does one thing, which is set a one-way latch. The render path polls it.

```rust excerpt=crates/renderer/src/renderer.rs
        let device_lost = crate::device_status::DeviceLossLatch::new();
        let loss_latch = device_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            log::error!("GPU device lost ({reason:?}): {message}");
            loss_latch.mark_lost();
        });
```

- `crates/renderer/src/device_status.rs` · `DeviceLossLatch` — the latch itself: `Arc<AtomicBool>`,
  marked once and never reset.
- `crates/engine_core/src/web/mod.rs` · `install_page_exit_guard` — browser `pagehide`/`pageshow`
  listeners that store into a `PAGE_EXITED` atomic and nothing more.
- `crates/common/src/vfs/mod.rs` · `set_write_observer` — a single-slot, wasm-only `fn(&Path)`
  observer. A plain function pointer captures nothing, so it can't dangle. The playground's
  persistence chains are its one user.

## Pitfalls here

- **Don't reintroduce a callback API for gameplay events.** If you're adding
  `on_collision(|…| …)`, the answer is already `take_collision_events()` or an event on the World
  bus. A callback can't touch the world it fires under.
- **A boundary callback does the minimum.** Set a flag, push to a queue, log. Never mutate engine
  state from inside a wgpu, winit or DOM callback. The frame loop reads the flag on its own terms.
- **`set_write_observer` replaces, it doesn't add.** A second call silently drops the first
  observer. Keep it to one owner.
- **The archive mentions a second collision channel that no longer exists.** `log_archive.md`
  says collisions were mirrored onto the World event bus. That mirror was later removed, because
  nothing read it. `take_collision_events()` is the one channel today.
