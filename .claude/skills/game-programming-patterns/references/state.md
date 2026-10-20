# State

Book: https://gameprogrammingpatterns.com/state.html

Intent: let an object's behavior change with its current mode by making that mode one explicit value, not a tangle of flags.

## The problem it solves

The book starts with a heroine who can jump, duck and dive. Each new move adds a boolean
(`is_jumping`, `is_ducking`), and each boolean doubles the combinations the code has to keep
consistent. Soon she can duck in mid-air, or jump while diving, because nothing forbids it. The
fix is to name the modes: a finite state machine has a fixed set of states, exactly one current
state, and transitions on input. Illegal combinations become unrepresentable. The chapter then
widens the idea to entry and exit actions, concurrent machines (what she's doing and what she's
holding), hierarchical states (ducking is a kind of on-ground), and pushdown automata (a stack,
so "fire, then return to whatever you were doing" works).

## Reach for it when

- Behavior depends on a mode, and the modes are mutually exclusive: menu, serving, playing,
  game over, or idle, patrolling, chasing.
- You find two or more booleans that must never be true together.
- Something happens on *entering* a mode (start a clip, reset a timer) and has to happen exactly
  once.
- The modes should be authored in data, for example an animation that plays one clip and then
  moves to the next.

## Don't reach for it when

- The behavior is continuous, not modal. Steering toward a target is arithmetic, not a state.
- You need planning or deep decision-making. The book is blunt that FSMs are not Turing
  complete. Past a dozen states with cross-cutting transitions, you want a behavior tree,
  utility scoring or a script.
- Two concerns are independent. One machine for both multiplies the states. Use two machines.
- It's a single boolean that nothing else interacts with. A `bool` is a two-state machine
  already.

## Tradeoffs

- **Explicitness against ceremony.** An enum and a `match` are cheap. State objects with
  enter/exit hooks are more structure, and they pay off only when enter and exit actually do
  work.
- **Where transitions live.** A transition can be decided inside the state's own code, or by a
  table outside it. Code is flexible, and a table can be authored and checked.
- **Data per state.** State-local data (a selection index, a wait timer) carried in the variant
  is precise, but it means swapping the whole value on transition.
- **History.** A flat machine forgets where it came from. Remembering "previous" is cheap. A real
  stack of states is a different structure with its own bugs.

## In Rust

The book's C++ needs a class per state plus virtual dispatch. In Rust, an **enum with data per
variant** is the natural state machine. `match` is exhaustive, so the compiler flags a new state
that some code doesn't handle, and a variant's fields exist only while that state is current.
Transitions are assignments, and `std::mem::replace` swaps a state out when its old value is
still needed.

Two Rust specifics shape the code:

- **Matching on `self.state` and then calling `&mut self` methods** conflicts with the borrow on
  `self.state`. Make the state `Copy` (or clone it when it's tiny), or match on a copy of its
  discriminant first.
- **The typestate pattern** goes further and encodes the state in a *type*
  (`Door<Open>`/`Door<Closed>`), so illegal transitions fail to compile. That suits APIs and
  protocols, not runtime game modes that change every frame.

An illustrative sketch, not engine code:

```rust
#[derive(Clone, Copy, PartialEq)]
enum Heroine { Standing, Ducking { charge_seconds: f32 }, Jumping }

fn step(state: Heroine, pressed_down: bool, delta_seconds: f32) -> Heroine {
    match state {
        Heroine::Standing if pressed_down => Heroine::Ducking { charge_seconds: 0.0 },
        Heroine::Ducking { charge_seconds } if pressed_down => {
            Heroine::Ducking { charge_seconds: charge_seconds + delta_seconds }
        }
        Heroine::Ducking { .. } => Heroine::Standing,
        other => other,
    }
}
```

## In insiculous_2d

**Verdict: textbook, at four levels.** There's a generic FSM component, a data-driven animation
FSM, a game-state enum in every game, and enum lifecycles in the engine and editor.

**1. The generic component.** `StateMachine<S>` holds the current and previous state, a
just-entered flag (the book's entry action, as data), and time spent in the state:

```rust excerpt=crates/ecs/src/state_machine.rs
    pub fn transition_to(&mut self, new_state: S) {
        if self.current == new_state {
            return;
        }
        self.previous = Some(std::mem::replace(&mut self.current, new_state));
        self.just_transitioned = true;
        self.elapsed = 0.0;
    }
```

- `crates/ecs/src/state_machine.rs` · `StateMachine::transition_to` — a no-op when the state is
  already current, so re-asserting a state doesn't re-enter it.
- `crates/ecs/src/state_machine.rs` · `StateMachine::just_entered` — true for the first tick
  after a transition. Systems read it in place of an enter callback.
- `crates/ecs/src/behavior.rs` · `BehaviorPhase` — its real user: `Idle`, `Patrolling`, `Waiting`
  and `Chasing`. It replaced a set of boolean flags under which "waiting and chasing at once"
  was representable.
- `crates/engine_core/src/behavior_runner/handlers.rs` · `transition_to` — chase and patrol
  transition on range and on the machine's `elapsed()` clock.

**2. The data-driven animation FSM.** A `ClipStateMachine` is a string-keyed table. Each state
names the clip it plays and what finishing that clip does, and the table is authored in scene
files or built by game code:

- `crates/ecs/src/clip_state_machine.rs` · `ClipStateMachine` — the table, which is authored,
  plus a runtime pose (current state, and the last state whose clip was applied).
- `crates/ecs/src/clip_state_machine.rs` · `OnFinished` — `Stay`, `Next(state)` or `Despawn`:
  the book's transitions as data.
- `crates/ecs/src/clip_state_machine.rs` · `set_clip_state` — game code's transition.
- `../games/pong/src/spawning.rs` · `tong_machine` — a ten-state table built in code (open,
  closing, closed, opening and scored-on, for each facing).

**3. A game-state enum per game.** Every game has a `GameState` enum and matches on it at the
top of `update`:

```rust excerpt=../games/pong/src/lib.rs
        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::DifficultySelect { selection } => self.update_difficulty_input(ctx, selection),
            GameState::ChaosSelect { selection } => self.update_chaos_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }
```

- `../games/pong/src/types.rs` · `GameState` — menu state carries its own selection, and the end
  of a match carries its winner.
- `../games/pong/src/gameplay/flow.rs` · `start_game` — entry actions as named functions that end
  by assigning the new state.

**4. Engine and editor lifecycles.**

- `crates/engine_core/src/lifecycle.rs` · `LifecycleState` — Created → Initializing → Initialized
  → Running → ShuttingDown → ShutDown, plus Error. `Scene` uses it.
- `crates/editor/src/play_state.rs` · `EditorPlayState` — Editing, Playing, Paused.
- `crates/engine_core/src/startup_splash.rs` · `Stage` — Pending → Splashing → Handoff → Running:
  the startup cards hold the game until handoff.

**There is no pushdown stack.** A `Scene` holds one `World`, menus are game-state variants, and
pause is a `PauseMenu` overlay that sets `time_scale` to zero. Add a stack only for a real
"return to whatever I was doing" need. Pause doesn't need one.

## Pitfalls here

- **`match self.state.clone()` is fine only while the state is small.** Every `GameState` today
  holds only `u8`, `bool` or `Copy` enums, so the clone is a stack copy. If a variant ever gains a
  `Vec` or a `String`, that clone allocates every frame. Switch to a discriminant match or to
  `std::mem::replace`.
- **`StateMachine::tick` clears `just_entered`.** Read the flag before the tick in the same
  frame, or the enter action never fires.
- **Generic machines don't survive Play/Stop.** A `StateMachine<MyState>` attached directly as a
  component is generic, so it can't go in the component registry. `WorldSnapshot` reports it and
  drops it on Stop. Embed the machine in a concrete component the snapshot knows, or keep the
  state in the game struct.
- `crates/editor/src/stored_component/mod.rs` · `BehaviorState` — the precedent. It holds a
  `StateMachine<BehaviorPhase>` and is an internal entry in the editor's stored-component list,
  so Play/Stop keeps it.
- `examples/hello_world/platformer.rs` · `StateMachine<PlayerState>` — the example that loses
  its machine on Stop, with the warning.
- **There is no hierarchical machine.** When states fall into groups, derive the group from the
  state with a function rather than storing a parent beside it.
- `examples/hello_world/platformer.rs` · `player_group` — the group the panel prints, computed
  from `PlayerState`.
- **Clip machines are string-keyed.** A misspelled state name is a runtime miss, not a compile
  error. pong's tests check every state against the clips its sheet really has. Do the same.
