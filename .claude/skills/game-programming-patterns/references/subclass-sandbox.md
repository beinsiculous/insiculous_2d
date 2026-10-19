# Subclass Sandbox

Book: https://gameprogrammingpatterns.com/subclass-sandbox.html

Intent: "Define behavior in a subclass using a set of operations provided by its base class."

## The problem it solves

Dozens of variants (superpowers, enemy kinds, games on one engine) all need the same handful of
engine services: play a sound, spawn an effect, move something. If each variant reaches into
those systems on its own, every variant is coupled to every system, and a change to the audio
API touches fifty files. The book puts one base class in the middle. It exposes a small set of
operations (the "sandbox"), each variant implements one abstract method using only those
operations, and the coupling collapses to a single place.

## Reach for it when

- There are many variants of one thing, each written as a single entry point.
- The variants need overlapping engine services, and you want them coupled to the sandbox
  rather than to each system.
- You want a beginner-proof surface: the operations are the whole menu, so a variant can't
  wander off and touch something it shouldn't.

## Don't reach for it when

- There are only two or three variants. A function each is simpler.
- The variants have little in common. Then the base class is pure ceremony.
- The base class has started to accumulate everything any variant ever needed. The book calls
  that end state a bowl of code stew. Move clusters of operations out into
  [components](component.md) or into their own service objects.

## Tradeoffs

- **The base class is the coupling point.** That's the point of the pattern, and also its risk:
  it couples to every system the variants use, and it's the classic brittle base class. One
  change to it ripples into every variant.
- **How many operations is a dial.** Few operations keep the base small but push work into
  variants. Many operations make variants trivial but bloat the base. A middle path is to return
  helper objects (`sound()`, `particles()`) instead of exposing every verb as a method.
- **Where the base gets its state** is a real design decision: constructor injection, two-stage
  init, static state, or a [service locator](service-locator.md).

## In Rust

Rust has no inheritance, so the pattern takes two common shapes:

- **A trait with provided methods.** A required method is the "subclass hook", and default
  methods are shared behavior. Trait defaults can't hold state or call each other's *overridden*
  versions, though, and there's no `super`: overriding a default replaces it entirely.
- **A trait whose methods take a context parameter.** The operations live on the context struct,
  not on the trait. That's the more idiomatic sandbox. The variant gets `&mut Context`, and the
  context's public fields and methods are the entire menu. It also avoids the "how does the base
  get its state" question, because the state arrives with every call.

An illustrative sketch, not engine code:

```rust
pub trait Power {
    fn activate(&mut self, sandbox: &mut PowerSandbox);
}

pub struct PowerSandbox<'frame> {
    pub sound: &'frame mut SoundQueue,
    pub particles: &'frame mut ParticleManager,
}
```

## In insiculous_2d

**Verdict: partial, in the context-parameter form, and used deliberately at two levels.**

**The `Game` trait is the engine's sandbox.** A game implements one required method, `update`,
and the operations are whatever `GameContext` carries: the world, input, per-player bindings,
assets, audio, UI, particles, lines, strings, achievements and scores. A few request methods
(`request_exit`, `set_window_title`) cover what a game may ask of the engine itself.

```rust excerpt=crates/engine_core/src/game/game_trait.rs
    /// Called every frame. Update your game logic here.
    /// This is the only required method.
    fn update(&mut self, ctx: &mut GameContext);
```

- `crates/engine_core/src/game/game_trait.rs` · `Game` — one required method, with defaulted
  hooks (`init`, `on_key_pressed`, `on_play_stopped`, `on_exit`).
- `crates/engine_core/src/game/game_trait.rs` · `render` — the one provided method with real
  behavior: tilemaps, then ECS sprites, then UI on top.
- `crates/engine_core/src/contexts.rs` · `GameContext` — the operation set, passed by `&mut` to
  every hook.
- `crates/engine_core/src/contexts.rs` · `request_exit` — a sandbox operation that records a
  request instead of acting, so the engine applies it at the end of the frame.

**`ScriptBehavior` is the closest literal match.** Both hooks default to empty, and the only
operations are reading `SelfView` and `ScriptView` and writing `ScriptCommands`. A native script
can't touch the world even if it wants to.

```rust excerpt=crates/engine_core/src/scripting/builtin/rotate.rs
        let delta_radians = degrees_per_second.to_radians() * view.delta_time;
        out.set_rotation(me, me.transform.rotation + delta_radians);
```

- `crates/engine_core/src/scripting/registry.rs` · `ScriptBehavior` — two optional hooks, each
  given exactly the views and the command buffer.
- `crates/engine_core/src/scripting/builtin/rotate.rs` · `RotateBehavior` — the built-in
  example. It uses nothing outside the sandbox.

**Not a sandbox:** `EditorGame<G>` implements `Game` by wrapping another `Game` and forwarding to
it. That's a decorator, which adds editor behavior around an unchanged game.

- `crates/editor_integration/src/editor_game/game_impl.rs` · `EditorGame` — forwards each hook
  to the inner game and overrides the camera after the inner `render`.

## Pitfalls here

- **Overriding `render` drops everything the default does.** There's no `super` for a trait
  default, so a game that overrides `render` loses tilemaps, ECS sprites and UI unless it redoes
  them. No game overrides it today. If one needs to, first extract the default body into a
  function it can call, rather than copying forty lines.
- **`GameContext` is the stew risk.** Every new engine service tends to become another field.
  Before you add one, ask whether it belongs on a World resource ([service-locator](service-locator.md))
  or behind an existing field.
- **Read a field's doc against the code.** Docs can lag behind behavior. The pattern audit found
  `time_scale` documented as "particles only" when the whole frame tail ran on it, and a crate
  guide listing a `physics` field the context never had. When the two disagree, the code wins:
  fix the doc.
- **Requests, not actions.** Engine-level effects from a hook (exit, retitle) are recorded on the
  context and applied after `update` returns. A game that expects them to take effect
  immediately is wrong by one frame.
