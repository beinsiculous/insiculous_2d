---
name: game-programming-patterns
description: Robert Nystrom's Game Programming Patterns distilled for this engine - one file per pattern with the problem it solves, when to reach for it, when not to, its tradeoffs, its Rust shape and where insiculous_2d already uses or deliberately avoids it. Use when designing or reviewing how engine or game code is structured - how entities or systems talk to each other, adding a state machine, spawning many short-lived objects, caching derived data, reaching for a global, adding a per-frame pass, timing the loop, or making behavior data-driven - and whenever anyone names one of the book's patterns (command, flyweight, observer, prototype, singleton, state, double buffer, game loop, update method, bytecode, subclass sandbox, type object, component, event queue, service locator, data locality, dirty flag, object pool, spatial partition).
---

# Game Programming Patterns, for insiculous_2d

Each file in `references/` takes one chapter of
[the book](https://gameprogrammingpatterns.com/contents.html) and covers the problem it solves,
when to reach for it, when not to, its tradeoffs, how it looks in Rust, and **what this engine
already does** with the decision of record behind it. Read the one file you need. Don't propose a
pattern the engine has deliberately turned down without reading why it was turned down.

The book's introduction comes down to this: architecture exists so that change stays cheap.
Decoupling buys that, and you pay for it in indirection, performance and code you have to read
before you can trust it. Every pattern below is a trade, so make it knowingly. The simplest code
that is still easy to change wins.

## Start from the symptom

| you're facing | read |
|---|---|
| I need undo/redo, replays, rebindable input, or "do this later" | [command](references/command.md) |
| many objects share heavy data (textures, meshes, glyphs) | [flyweight](references/flyweight.md) |
| two systems must react to the same thing without knowing each other | [event-queue](references/event-queue.md) first, then [observer](references/observer.md) for why not callbacks |
| I need many objects that start from the same setup | [prototype](references/prototype.md), [type-object](references/type-object.md) |
| "there's only one of these, so make it global" | [singleton](references/singleton.md), then [service-locator](references/service-locator.md) |
| a nest of booleans decides behavior | [state](references/state.md) |
| readers see half-updated state mid-frame | [double-buffer](references/double-buffer.md) |
| frame time, fixed steps, the speed of physics vs rendering | [game-loop](references/game-loop.md) |
| adding a per-frame pass over entities | [update-method](references/update-method.md) |
| designers or players should change behavior without recompiling | [bytecode](references/bytecode.md), [type-object](references/type-object.md) |
| many variants share one set of helper operations | [subclass-sandbox](references/subclass-sandbox.md) |
| one object spans rendering, physics, audio, AI | [component](references/component.md) |
| systems need audio, assets or input without being wired to them | [service-locator](references/service-locator.md) |
| the frame is slow and the profiler says memory access | [data-locality](references/data-locality.md) |
| I recompute this every frame, but it rarely changes | [dirty-flag](references/dirty-flag.md) |
| spawning and despawning thousands of short-lived things | [object-pool](references/object-pool.md) |
| "what's near this point?" over many moving things | [spatial-partition](references/spatial-partition.md) |

## Where the engine stands

| pattern | verdict | where |
|---|---|---|
| command | textbook | editor undo (`EditorCommand`, `CommandHistory`), deferred script/behavior/physics command buffers |
| flyweight | textbook | `TextureHandle`, deduped asset loads, shared `SpriteSheet`s, GPU instancing |
| observer | deliberately avoided | collision callbacks were deleted; callbacks survive only at platform edges |
| prototype | partial | scene-file prefabs plus `spawn_prefab`; games spawn through recipe functions |
| singleton | partial, kept for reasons | the component registry and the entity id generator are process-global; prefer World resources |
| state | textbook | `StateMachine`, `ClipStateMachine`, each game's `GameState` enum |
| double-buffer | textbook | input previous/current, physics contact sets, bloom ping-pong, the instance cache |
| game-loop | textbook | `GameLoopManager` (variable, clamped frame), fixed-step physics accumulator |
| update-method | textbook | `ecs::System`, run as a fixed ordered list in the frame tail |
| bytecode | not used; Rhai stands in | sandboxed Rhai scripts with an operation budget |
| subclass-sandbox | partial, as a trait | the `Game` trait over `GameContext`, `ScriptBehavior` |
| type-object | textbook, as data | the `Behavior` enum, `ScriptDescriptor`, `.sheet.ron`, game tables |
| component | textbook, kept simple | the hand-written ECS |
| event-queue | textbook | the World `EventBus`, drained collision events, the input event queue |
| service-locator | textbook, passed not looked up | `GameContext`, World resources, null services |
| data-locality | deferred until a trigger | HashMap-of-boxes storage is the recorded tradeoff |
| dirty-flag | textbook, by comparing values | transform hierarchy cache, physics edit baseline, upload skip |
| object-pool | partial | the particle ring pool; bullets are entities with `Lifetime` |
| spatial-partition | delegated to rapier | rapier's broad phase; editor picking is a linear scan |

## House rules that outrank the book

1. **Headless-testable or it doesn't land.** Whatever pattern you choose has to run under
   `GameHarness` with no GPU and no window (`CLAUDE.md` § Testing). A pattern that needs a live
   window to exercise is the wrong pattern here.
2. **No callbacks that fire under `&mut World`.** Collision callbacks were removed because they
   ran under the world borrow and could mutate nothing. For "tell someone it happened", use an
   event, a deferred command buffer or a returned value ([observer](references/observer.md)).
3. **No new process globals.** Put state on a World resource or a `GameContext` field. The few
   statics that remain are listed in [singleton](references/singleton.md) with the reason each
   one is tolerated.
4. **`GameLoopManager` is the only frame timer.** Don't add another clock ([game-loop](references/game-loop.md)).
5. **Data-locality work waits for its trigger.** It needs profiling evidence, or entity counts
   past the recorded threshold ([data-locality](references/data-locality.md)).
6. **Name a pattern in plain words in source.** In `.rs` files, write "dirty-flagged" or "object
   pool", never an audit code like `GPP-04`. The comment-policy grep gate rejects them.

## Freshness

Paths and symbols were checked against `insiculous_2d` at commit `bce5abc` on 2026-10-19.
Citations name a file and a symbol, never a line number. If a cited symbol has moved, trust the
code over this skill and fix the file you were reading. The same day's audit against these files
fixed what was small and filed the rest as insiculous_2d#168 to #172. Each reference file's
pitfalls point to the ticket it is waiting on. The full history of the July 2026
pattern audit is in `log_archive.md` § "Game Programming Patterns Audit".
