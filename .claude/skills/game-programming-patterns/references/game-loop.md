# Game Loop

Book: https://gameprogrammingpatterns.com/game-loop.html

Intent: "Decouple the progression of game time from user input and processor speed."

## The problem it solves

A game keeps moving when nobody touches it, so it can't sit and wait for input the way an
application does. And it must run at the same *game* speed on a fast machine, a slow one and a
phone. The loop polls input without blocking, advances the simulation by an amount of time it
measured, renders, and repeats. The interesting part is how it handles time: a fixed step, a
variable step, or a fixed simulation step with rendering free to run as fast as it can.

## Reach for it when

- Always, in some form. The question is never *whether* to have a loop but who owns it and
  how it measures time. Even a turn-based game needs one for animation and audio.
- The simulation must behave the same regardless of frame rate. That points to a fixed step
  for anything that integrates (physics, anything whose result depends on step size).

## Don't reach for it when

- The platform already owns the loop, and that is the whole reason you'd write a second one.
  On the web `requestAnimationFrame` paces frames, and blocking the main thread is forbidden.
  Natively, winit owns the event loop. Hook into theirs rather than wrapping one around it.
- You'd add a second clock. Two timers that each measure *the frame* will disagree after the
  first stall.

## Tradeoffs

- **Fixed step.** Deterministic and stable, but on a slow machine it falls behind, and on a
  fast one it renders the same state more than once.
- **Variable step.** It adapts to any speed, but a long frame becomes one huge step, so physics
  tunnels or explodes and results depend on frame rate. The book discourages it for simulation.
- **Fixed update, variable render** (an accumulator). Simulation is stable and rendering is as
  smooth as the display allows. It costs a cap on catch-up steps (or you get the *death spiral*,
  where catching up makes the frame slower still), plus either interpolation between the last
  two steps or a visible cadence mismatch when the display rate isn't a multiple of the step.
- **Power.** Running flat out wastes battery. Sleeping out the frame budget, or trusting vsync,
  caps it.

## In Rust

Rust adds little to the pattern itself. It matters at the edges:

- winit's `ApplicationHandler` inverts control: you implement callbacks, and the platform
  calls them. "Who owns the loop" is answered for you, and a frame becomes a method you call
  from the right callback.
- `Instant` is not available on `wasm32-unknown-unknown`, so a cross-platform clock has to sit
  behind one seam (`web-time` or similar).
- Clamp `Duration` into `f32` seconds once, at the clock, and give it a name with the unit
  (`delta_seconds`), so no callee re-derives it.

An illustrative sketch of the accumulator, not engine code:

```rust
accumulated_seconds += frame_seconds.min(MAX_FRAME_SECONDS);
let mut steps_taken = 0;
while accumulated_seconds >= STEP_SECONDS && steps_taken < MAX_STEPS {
    simulation.step(STEP_SECONDS);
    accumulated_seconds -= STEP_SECONDS;
    steps_taken += 1;
}
let blend = accumulated_seconds / STEP_SECONDS; // render previous.lerp(current, blend)
```

## In insiculous_2d

**Verdict: textbook, as fixed-step physics inside a variable, clamped frame.** There is no
render interpolation (tracked, see Pitfalls).

One clock measures the frame and clamps a stall to a tenth of a second:

```rust excerpt=crates/engine_core/src/game_loop_manager.rs
    pub fn update(&mut self) -> f32 {
        let now = Instant::now();
        self.delta_time = (now - self.last_frame_time).as_secs_f32().min(MAX_DELTA_TIME);
        self.last_frame_time = now;
        self.frame_count += 1;
        self.total_time += self.delta_time;
        self.delta_time
    }
```

- `crates/engine_core/src/game_loop_manager.rs` · `MAX_DELTA_TIME` — a debugger pause or
  a window drag becomes one clamped step, not a leap.
- `crates/engine_core/src/game_loop_manager.rs` · `GameLoopManager::throttle` — sleeps out
  `target_fps`'s budget natively and does nothing on wasm, where the browser paces the loop.

Physics runs its own fixed step on top of that variable frame, capped so it can't spiral:

```rust excerpt=crates/physics/src/physics_system/update.rs
        let mut steps = 0;
        while self.time_accumulator >= self.fixed_timestep && steps < MAX_STEPS_PER_UPDATE {
            self.physics_world.step(self.fixed_timestep);
            self.time_accumulator -= self.fixed_timestep;
            steps += 1;
        }
```

- `crates/physics/src/physics_system/mod.rs` · `MAX_STEPS_PER_UPDATE` — eight catch-up steps,
  then the leftover time is dropped with a warning. A small slowdown is the price of stability.
- `crates/physics/src/physics_system/update.rs` · `time_accumulator` — the leftover is kept
  across frames but never used to blend rendering.

Winit owns the loop, and the engine drives one frame from a *different* callback per platform.
That is deliberate, and the two drivers must never be unified:

```rust excerpt=crates/engine_core/src/game/app_handler.rs
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Native frame driver (unchanged behavior). On the web this callback
        // has no vsync pacing and stalls while occluded differently, so
        // frames are driven from RedrawRequested instead.
        #[cfg(not(target_arch = "wasm32"))]
        self.drive_frame(event_loop);
        #[cfg(target_arch = "wasm32")]
        let _ = event_loop;
    }
```

- `crates/engine_core/src/game/app_handler.rs` · `GameRunner::drive_frame` — the one frame,
  shared by both drivers: update and render, exit, the fail-stop on device loss, throttle, re-arm.
- `crates/engine_core/src/game/app_handler.rs` · `arm_next_frame` — on the web the loop re-arms
  itself, and a quiet window drops to a 100 ms timer.
- `crates/engine_core/src/web/mod.rs` · `install_hidden_frame_pump` — a hidden document gets no
  animation frames, so a timer drives frames through the event-loop proxy.
- `crates/engine_core/src/game.rs` · `GameRunner::step_frame` — the window-free middle of a
  frame. The real loop and the headless harness both call it, so a change lands in both.
- `crates/engine_core/src/test_support.rs` · `GameHarness` — tests own the clock. `step` takes
  an explicit `delta_time`, so a headless frame is deterministic.

**Pause is not a clock.** `ctx.delta_time` is never scaled. `time_scale` scales only the
engine's own tail (particles, animation, lifetimes, clip machines, grids), and a game gates its
own logic and its physics step while paused.

- `crates/engine_core/src/contexts.rs` · `GameContext::time_scale` — the contract, in its doc
  comment.

## Pitfalls here

- **`GameLoopManager` is the only frame timer.** Read `ctx.delta_time`. Don't keep an `Instant`
  of your own, and don't make a second loop.
- **Never unify the native and wasm drivers.** An occluded or minimized native window stops
  receiving redraws, and on the web `about_to_wait` isn't vsync-paced. This is a pinned pitfall
  in `crates/engine_core/CLAUDE.md`.
- **No interpolation, so cadence shows.** Physics steps at 60 Hz, and the frame draws
  whatever transform the last step wrote. On a 144 Hz display a moving body is held for two
  frames, then three, unevenly. On 75 Hz one frame in five repeats. This is tracked in
  insiculous_2d#84's low-priority backlog, with a trigger: act when stutter is actually observed
  on a high-refresh display. Don't fix it speculatively.
- **A game that steps physics while paused drifts.** The engine's tail freezes, but your
  `PhysicsSystem` doesn't, unless you return before calling it (pong's pause branch does).
- **Don't add a `LifetimeSystem` of your own.** The frame tail already runs one on the
  time-scaled delta, so a second one halves every lifetime. See
  [update-method](update-method.md).
