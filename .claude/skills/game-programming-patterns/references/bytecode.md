# Bytecode

Book: https://gameprogrammingpatterns.com/bytecode.html

Intent: "Give behavior the flexibility of data by encoding it as instructions for a virtual machine."

## The problem it solves

Some behavior changes far more often than the engine does. Examples are a spell's effect, an
enemy's routine, or a level's rule. If that behavior is compiled Rust, every tweak costs a rebuild
and a programmer, and anyone who isn't a programmer can't touch it at all. Hand them the host
language instead and you've handed them the whole process: a typo can crash the game, and a loop
can hang it. The book's answer is a tiny instruction set run by a virtual machine the engine
owns. Behavior becomes data that can be loaded, edited and hot-swapped, and the VM decides
exactly what that data is allowed to touch.

## Reach for it when

- Many behaviors have to be authored or changed by people who don't rebuild the engine, such as
  designers, modders, or the playground's visitors.
- The behavior has to be **sandboxed**, so that a bad script can neither crash nor freeze the
  host.
- The behavior has to run where native code loading can't, such as wasm32 with no `dlopen`.

## Don't reach for it when

- The behavior is hot-path code. An interpreter runs many times slower than compiled Rust, so
  physics, rendering and per-particle work stay native.
- Only a programmer will ever change it. Then a Rust trait or an enum is cheaper to write,
  debug and test.
- What you actually want is tunable *numbers*. That's [type-object](type-object.md), or a
  serde-loaded table, not a language.

## Tradeoffs

- **You're building a language.** That means a front end (parser or editor), error messages and
  tooling. Small scripting systems tend to grow features until they turn into a bad
  general-purpose language.
- **Debugging gets worse.** A native debugger sees the VM loop, not the script. Errors have to
  be carried back to the author with a file and a hook name.
- **The sandbox is only as good as its limits.** A VM with an operation budget but no memory
  budget can still be killed by one script that allocates without bound.
- **Speed costs for flexibility.** Every instruction is a dispatch.

## In Rust

In Rust, "write your own VM" is rarely the right first move. The ecosystem has embeddable,
pure-Rust interpreters (Rhai, and others) that already give you the book's design decisions:
a value representation (a tagged enum, the book's recommendation), a sandbox, operation limits,
and host functions you register. The book's shape is still the right way to *think* about the
boundary:

- **External primitives** are the only verbs a script gets. Register a small, explicit set of
  host functions, and don't expose `&mut World`.
- **Reads are snapshots, writes are commands.** The script sees owned copies of the state it's
  allowed to read, and it pushes intents into a buffer that the host applies later. That keeps
  the borrow checker happy and makes every script effect visible in one place.
- **Bounded execution.** Cap the operation count, the call depth and the data sizes, so a
  runaway script turns into an error instead of a hang.

An illustrative sketch, not engine code:

```rust
let mut engine = rhai::Engine::new_raw();
engine.set_max_operations(100_000);
engine.set_max_string_size(64 * 1024);
engine.register_fn("move_by", |commands: &mut CommandBuffer, offset_x: f32, offset_y: f32| {
    commands.push(Command::MoveBy(Vec2::new(offset_x, offset_y)));
});
```

## In insiculous_2d

**Verdict: no hand-written VM, and none planned. An embedded Rhai interpreter fills the role.**
Rhai was chosen because it is pure Rust, runs on wasm32, and has an operation budget for runaway
loops. Dylib hot-reload was dropped for good, because there is no `dlopen` on wasm32
(`PROJECT_ROADMAP.md`). Scripts are `.rhai` files with `// @param` headers and optional
`early_update` and `update` hooks (`docs/SCRIPTING.md`).

- `crates/engine_core/src/scripting/rhai_bindings.rs` · `create_rhai_engine` — the engine's
  only VM configuration. It sets the operation budget and registers exactly three groups of host
  functions: math, views and commands.
- `crates/engine_core/src/scripting/rhai_backend.rs` · `RhaiCompiledScript` — the cached
  compiled unit, keyed by a hash of the source, so a byte-identical source is never recompiled.
- `crates/engine_core/src/scripting/view.rs` · `ScriptView` — the read side: an owned snapshot
  of named entities, collisions, player input, the blackboard and clip state, built before the
  hook runs.
- `crates/engine_core/src/scripting/view.rs` · `SelfView` — the script's own entity, read-only.
- `crates/engine_core/src/scripting/commands.rs` · `ScriptCommand` — the write side, and the
  book's "external primitives" almost exactly: a closed enum of every effect a script can have.
  Examples are `SetVelocity`, `SetSpriteColor`, `PlayClip`, `SetBlackboard` and `Despawn`.
- `crates/engine_core/src/scripting/runner.rs` · `ScriptRunner::run_phase` — runs each hook,
  drains its commands, applies them at the end of the phase, and quarantines a script that blew
  its budget.

The whole sandbox fits in one excerpt: an operation budget, size and depth limits, and three
groups of host functions.

```rust excerpt=crates/engine_core/src/scripting/rhai_bindings.rs
    let mut engine = Engine::new();
    engine.set_max_operations(200_000);
    engine.set_max_string_size(MAX_SCRIPT_STRING_BYTES);
    engine.set_max_array_size(MAX_SCRIPT_COLLECTION_ENTRIES);
    engine.set_max_map_size(MAX_SCRIPT_COLLECTION_ENTRIES);
    engine.set_max_call_levels(MAX_SCRIPT_CALL_DEPTH);
    register_math_and_conversions(&mut engine);
    register_views(&mut engine);
    register_commands(&mut engine);
```

A script that blows the budget or a limit doesn't hang the frame or exhaust memory. It is
classified (`RunawayLoop` or `ResourceLimit`), quarantined for the rest of the Play session, and
reported once:

```rust excerpt=crates/engine_core/src/scripting/runner.rs
                    match call_result {
                        Ok(()) => phase_commands.commands.extend(handle.drain()),
                        Err(error) => {
                            if matches!(error.kind, ScriptErrorKind::RunawayLoop | ScriptErrorKind::ResourceLimit) {
                                self.quarantined.insert(key);
                            }
                            self.errors.push(error);
                        }
                    }
```

Native Rust scripts implement the same hooks through `ScriptBehavior` (see
[subclass-sandbox](subclass-sandbox.md)), so a game can move a behavior between Rhai and Rust
without its scene changing shape. The eight closed `Behavior` variants that predate scripting are
due to become built-in scripts; that is tracked on the board, not done
([type-object](type-object.md)).

## Pitfalls here

- **Operations and bytes are separate budgets.** The operation budget alone never bounded
  memory: a few dozen operations of string doubling could ask for more than the machine has.
  Strings are capped at 64 KiB, collections at 10,000 entries and the call stack at 64 levels
  (`docs/SCRIPTING.md`). A new host function that returns large data has to fit under them too.
  Raise a limit when a real payload needs it, not before.
- `crates/engine_core/src/scripting/rhai_backend.rs` · `classify_runtime_error` — the one place a
  Rhai error becomes a quarantine decision.
- **Never hand a script `&mut World`.** Every new verb is a `ScriptCommand` variant plus a
  registered function, applied by the runner. A host function that reaches the world directly
  breaks the "all effects at one point" rule that ordering and quarantine depend on.
- **Views are snapshots.** A script reads the state from before its phase, not the effects of
  commands queued earlier in the same phase. Two scripts can't see each other's writes until the
  next phase.
- **Hook phase matters.** `early_update` runs before the physics step and `update` runs after
  the collision drain (`docs/SCRIPTING.md`). Velocity belongs in `early_update`, and reacting to
  contacts belongs in `update`.
