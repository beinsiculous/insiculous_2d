# Event Queue

Book: https://gameprogrammingpatterns.com/event-queue.html

Intent: "Decouple when a message or event is sent from when it is processed."

## The problem it solves

Something happens in one part of the game, such as a click, a pickup or a contact, and a
different part has to react. If the sender calls the receiver straight away, three things go
wrong. The sender has to know who is listening. The reaction runs in the middle of the sender's
work, while the sender's data is half-updated. And whatever the receiver does, the sender pays
for it inside its own loop. A queue cuts that call in two. The sender appends a small record of
what happened and carries on. The receiver drains the queue at a point it chooses, when it is
safe to act. The book's own example is audio: gameplay asks for a sound, and the audio code
plays it later, on its own schedule.

## Reach for it when

- The sender and the receiver need to be decoupled **in time**, not just in knowledge. The
  receiver must act later, at its own safe point.
- The receiver's reaction would mutate state the sender is still borrowing or iterating. In Rust
  this is the common case, and the borrow checker usually tells you.
- Several requests in one frame should be batched, merged or capped before they're acted on.
- The producer runs somewhere the consumer can't, such as a physics sub-step, a platform
  callback or a UI pass.

## Don't reach for it when

- Only the *who* needs decoupling, not the *when*. Then a returned value, a command or a
  direct call is simpler, and the book says the same.
- The receiver needs an answer back before the sender can continue. A queue is one-way.
- There is exactly one producer and one consumer in the same function. Pass the `Vec`.

## Tradeoffs

- **A central queue is a global in disguise.** Anyone can write to it and anyone can read it,
  and the coupling you removed returns as "who else drains this?"
- **Stale data.** By the time an event is processed, the world it describes may have moved on.
  An event should carry what the receiver needs (ids, positions), not references into state
  that may have changed.
- **Feedback loops.** A receiver that emits an event its own sender reacts to can ping-pong
  forever. Draining once per frame, and not re-draining what you emit, breaks the cycle.
- **Lifetime and latency.** Someone has to decide when events die. If they die too early, a
  reader misses them. If they die too late, a reader handles them twice. The flush point also
  sets how many frames late a reaction arrives.

## In Rust

Ownership makes this pattern idiomatic in Rust rather than an optimization. A queue is a
`Vec<Event>` (or a `VecDeque` for FIFO with partial drains), and events are plain owned data,
usually `Clone` enums or structs. The key move is *draining*. `std::mem::take(&mut queue)`
hands the consumer an owned `Vec` and leaves the producer's buffer empty. The consumer can then
mutate anything while it iterates, because no borrow of the producer survives.

An illustrative sketch, not engine code:

```rust
#[derive(Clone, Copy)]
enum GameEvent { CoinCollected { collector: EntityId, value: u32 } }

fn react(world: &mut World, queue: &mut Vec<GameEvent>) {
    for event in std::mem::take(queue) {
        let GameEvent::CoinCollected { collector, value } = event;
        if let Some(score) = world.get_mut::<Score>(collector) { score.points += value; }
    }
}
```

A type-erased bus (`HashMap<TypeId, Box<dyn Any>>`, one `Vec<E>` per event type) gives you
publish and subscribe by type without a central enum. The cost is a downcast on every read.

## In insiculous_2d

**Verdict: textbook.** There are three queues, each with one owner and a stated lifetime. The
July 2026 audit counted it as textbook already.

- `crates/ecs/src/event.rs` · `EventBus::emit` — one `Vec<E>` per event type, keyed by `TypeId`
  and reached through `World::emit_event` and `World::read_events`.
- `crates/ecs/src/world.rs` · `World::flush_events` — clears every queue. The runner calls it
  first thing in each frame, so an event lives from its emit until the start of the next frame.
- `crates/engine_core/src/game.rs` · `step_frame` — the flush, then last frame's button presses
  re-emitted onto the fresh bus.

```rust excerpt=crates/engine_core/src/game.rs
        // Flush events from previous frame before processing new input
        self.scene.world.flush_events();

        // Emit last frame's UI-element button presses now that the bus is
        // fresh — games read them in update() one frame after the click.
        for event in self.pending_ui_events.drain(..) {
            self.scene.world.emit_event(event);
        }
```

- `crates/engine_core/src/ui_element_system.rs` · `UiButtonPressed` — produced *after*
  `update()`, so it is held in `pending_ui_events` and arrives one frame late. That latency is
  deliberate and pinned by a guard test.
- `crates/engine_core/src/behavior_runner/mod.rs` · `EntityCollected` — emitted by the behavior
  runner before it despawns what was collected.

**Collisions drain instead of broadcasting.** Physics never posts to the world bus. A mirror of
`CollisionData` onto the bus existed once, and it was deleted on 2026-09-03 with no readers
anywhere. Older notes in `log_archive.md` still describe it. `take_collision_events()` is the
only channel.

- `crates/physics/src/physics_system/mod.rs` · `PhysicsSystem::take_collision_events` —
  drain-style. Take once per frame after `update()`, then share the `Vec`. A second take in the
  same frame comes back empty.
- `../games/pong/src/gameplay/mod.rs` · `take_collision_events` — pong's frame drains once and
  hands one slice to every consumer.

```rust excerpt=../games/pong/src/gameplay/mod.rs
        let collisions: Vec<CollisionData> = self.physics.take_collision_events();

        self.handle_gameplay_input(ctx);
        self.maintain_all_ball_velocities();
        self.check_goals(ctx, &collisions);
        self.check_powerup_collisions(ctx, &collisions);
```

- `crates/engine_core/src/pickups.rs` · `Pickups::collect` — a consumer that takes
  `&[CollisionData]`, never the physics system.
- `crates/input/src/input_handler.rs` · `InputHandler::process_queued_events` — window and
  gamepad events are queued as they arrive and applied at the start of the frame, so every
  reader in the frame sees the same input.
- `crates/engine_core/src/grid/backdrop_system.rs` · `GridImpulses` — a World-resource work
  queue filled by `ripple()` and removed whole by the backdrop system each frame. Impulses
  queued while time is frozen are dropped, not saved up.

**Audio has no general sound queue, and no scene-data path.** The audio manager's one deferral
holds the last music request until a user gesture enables output on the web. A game plays a
sound through `ctx.audio`; scene-authored audio is filed under the Conductor milestone.

## Pitfalls here

- **The flush is at the start of the frame, not the end.** An event emitted anywhere in the
  frame tail (after `update()`) is flushed before any game reads it. `UiButtonPressed` gets
  around this by holding presses in `pending_ui_events`. Any new engine-side producer that
  runs after `update()` needs the same treatment, or its events vanish. A game's key handler is
  the same case: the window loop runs `on_key_pressed` / `on_key_released` between frames, so an
  event emitted there is flushed before `update` reads it. A handler sets state; `update` emits.
  - `crates/engine_core/tests/key_handler_events.rs` ·
    `a_world_event_emitted_in_a_key_handler_is_flushed_before_the_next_update_reads_it`
- **Take collisions once.** Two `take_collision_events()` calls in one frame give the second
  caller nothing. Drain into a local and pass `&collisions` around, as every game does.
- **There is no sound component to queue on.** The old `PlaySoundEffect` component was retired
  with the other audio components; a saved scene that still names one loads with a warning and
  the entry is dropped. Call `ctx.audio.play(handle)` from game code.
- **Don't add observers to dodge the queue.** Collision callbacks were deleted because they
  fired under `&mut World` and could change nothing. See [observer](observer.md).
- History: `log_archive.md` § "July 13, 2026 — GPP-08 + GPP-10 + GPP-L9".
