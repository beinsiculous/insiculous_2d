# Double Buffer

Book: https://gameprogrammingpatterns.com/double-buffer.html

Intent: "Cause a series of sequential operations to appear instantaneous or simultaneous."

## The problem it solves

Some state is built up one step at a time but read as if it were a single moment: a frame being
drawn, a set of buttons being updated, a crowd of actors who should all react to *where
everyone was*, not to wherever the ones updated earlier in the loop have already moved. If a
reader looks while the writer is halfway through, it sees half of one frame and half of the next.
The pattern keeps two copies. Writers fill the next one while readers see the current one, and
a swap at a well-defined point makes all of the writes visible at once.

## Reach for it when

- State changes a piece at a time, can be read mid-change, and the reader must never see the
  half-done version (a GPU target, an edge detector, a contact set).
- You need a "previous" to compare against: edges like *just pressed*, *contact started* or
  *contact ended* are the difference between two buffers.
- Every actor in a pass should react to the same snapshot, so update order stops changing
  the outcome.

## Don't reach for it when

- Nothing reads the state while it's being written. Then one copy is enough.
- The swap point is fuzzy. If you can't name the exact moment current becomes previous, two
  buffers just give you two ways to read stale data.
- The state is big and only a sliver of it is ever compared. Keep the sliver (a hash, a count, a
  last-pushed value) instead. See [dirty-flag](dirty-flag.md).

## Tradeoffs

- **Memory.** Two copies of everything you buffer.
- **Swap versus copy.** Swapping is O(1), but any code that held a reference into "current" is
  now holding "previous". Copying (`clone_from`) is slower, but the current buffer stays where
  it is and the previous one is simply a frame old.
- **Latency.** Readers always see last frame's world. That is the point, and it is also a
  one-frame lag you have to reason about.
- **Granularity.** You can buffer the whole state or one piece per object. Per-object buffers
  are easier to retrofit, but you have to remember to swap every one of them.

## In Rust

The borrow checker enforces the core rule for you: a `&mut` writer and a `&` reader of the
same buffer can't coexist. What's left to choose is the swap:

- `std::mem::swap(&mut previous, &mut current)` exchanges two owned buffers with no allocation.
  Reuse the emptied one by `clear()`ing it rather than building a new one.
- `previous.clone_from(&current)` reuses `previous`'s allocation. It's the right shape when the
  writer keeps mutating `current` in place across frames.
- A read-only snapshot type (`Rc<View>` or a plain struct of copies) is a double buffer
  for *logic*. Every reader in a pass gets the same immutable view, and writes queue as
  commands that are applied after the pass.

An illustrative sketch, not engine code:

```rust
struct Contacts {
    current: HashSet<ContactPair>,
    previous: HashSet<ContactPair>,
}

impl Contacts {
    fn end_step(&mut self) {
        std::mem::swap(&mut self.previous, &mut self.current);
        self.current.clear();
    }
    fn started(&self) -> impl Iterator<Item = &ContactPair> + '_ {
        self.current.difference(&self.previous)
    }
}
```

## In insiculous_2d

**Verdict: textbook, used for input, physics contacts, bloom and the script view.** Each one
has its own explicit swap point.

Input keeps *held now* and *held at the end of last frame*. The edge queries read between them,
and `end_frame` is the swap:

```rust excerpt=crates/input/src/button_tracker.rs
    pub fn clear_frame_state(&mut self) {
        self.previous.clone_from(&self.pressed);
        self.just_pressed.clear();
        self.just_released.clear();
    }
```

- `crates/input/src/button_tracker.rs` · `ButtonTracker::clear_frame_state` — copy, not swap,
  because `pressed` lives on across frames. Keyboard, mouse and gamepad all compose this one
  tracker.
- `crates/input/src/gamepad.rs` · `GamepadState::clear_frame_state` — the same model for analog
  axes, so an axis used as a digital source still gets its edges.
- `crates/input/src/input_handler.rs` · `InputHandler::end_frame` — the swap point. The engine
  calls it once, after the game's `update`.

Physics derives *started* and *stopped* contacts from two sets, then swaps them:

```rust excerpt=crates/physics/src/physics_world/stepping.rs
        // Update previous collisions for next step, reusing the allocation
        std::mem::swap(&mut self.previous_collisions, &mut self.current_collisions);
```

- `crates/physics/src/physics_world/stepping.rs` · `previous_collisions` — this is why a collider
  rebuilt under a ball it already touches emits no fresh start (pong relies on that).

Scripts run a phase against one snapshot. Every script reads the same `ScriptView`, and every
write is a queued command applied after the last script has run. So script order can't change
what any script sees:

```rust excerpt=crates/engine_core/src/scripting/runner.rs
        let view = Rc::new(self.build_view(
            world,
            ctx.input,
            ctx.players,
            ctx.delta_time,
            ctx.collisions,
            physics.as_deref(),
        ));

        let mut phase_commands = ScriptCommands::new();
```

- `crates/engine_core/src/scripting/runner.rs` · `ScriptRunner::run_phase` — snapshot, run every
  instance, then `phase_commands.apply`.
- `crates/engine_core/src/scripting/view.rs` · `SelfView` — the per-instance half of the
  snapshot.

The GPU side:

- `crates/renderer/src/render_targets.rs` · `RenderTargets::bloom_pong` — half-resolution ping
  and pong textures. Each blur pass reads one and writes the other.
- `crates/renderer/src/bloom.rs` · `blur_passes` — horizontal into pong, vertical back into
  ping, repeated per iteration. `bloom_ping` is what the composite reads.
- `crates/renderer/src/sprite/instance_cache.rs` · `InstanceCache::stage` — a staging buffer
  and a last-uploaded snapshot, swapped on change. The swap avoids an allocation, and the
  comparison is [dirty-flag](dirty-flag.md) work.

One place does it by hand: a UI-element click is drawn in the frame tail, *after* the event bus
has been read, so it is parked and re-emitted after the next flush.

- `crates/engine_core/src/game.rs` · `pending_ui_events` — the one-frame hold that keeps a click
  from being flushed before any game can read it.

## Pitfalls here

- **Read before the swap.** `InputHandler::update()` fuses processing and `end_frame`, so
  calling it before a `just_*` query erases the edge. Only tests use it, and runtime code
  never should. This is pinned in `crates/input/CLAUDE.md`.
- **`queue.write_buffer` lands at submit, not at encode.** Rewriting one uniform buffer
  between two passes in one submit means both passes read the last write. Bloom keeps one
  buffer per blur direction for this reason, and any new multi-pass effect needs the same.
- **A single-buffered event bus.** `World` events are cleared at the start of each frame, so a
  reader that runs *before* the emitter within a frame never sees that frame's events. Emit
  before you read, or hold the event the way `pending_ui_events` does. See
  [event-queue](event-queue.md).
- **Swapping strands references.** After a `mem::swap`, anything indexed into the old
  "current" now points at "previous". Hand out owned data (`take_collision_events`) rather than
  borrows into a buffer that swaps.
