# Command

Book: https://gameprogrammingpatterns.com/command.html

Intent: turn a request into a value you can store, pass around, replay or reverse, instead of a call that happens and is gone.

## The problem it solves

A method call is spent the moment it runs. You can't put it in a list, hand it to someone else,
run it later, run it again, or take it back. Games keep needing all five. A button should mean
"jump" this week and "fire" next week, so the binding has to be data. An AI and a player should
drive the same unit through the same verbs. A level editor needs undo. A replay is the game's
own inputs played back. The pattern reifies the call: the request becomes an object (in Rust,
usually an enum value or a boxed trait object) that carries everything needed to perform it,
and optionally to reverse it.

## Reach for it when

- You need undo/redo. Each change records enough to unapply itself, and a history holds the
  stack.
- Input must be rebindable, or players and AI must issue the same actions.
- Work has to happen *later* than it was requested. The usual reason in Rust is that the code
  deciding what to do is holding a borrow the doing needs.
- You want a replay, a network stream or a macro of actions. All three are lists of commands.

## Don't reach for it when

- The call is made once and never stored, deferred or reversed. A function is already the
  simplest command.
- Undo would be cheaper as a snapshot. When an action touches most of the state, saving the
  state beats inverting every edit.
- Every command would carry so little that the enum just mirrors your method list. You've added
  a dispatch layer and bought nothing.

## Tradeoffs

- **Undo is only as good as the before-image.** A command that captures too little reverses to
  the wrong state. A command applied to a world that has moved on since it was recorded can
  reverse into nonsense. Stable identities (ids that survive undo) matter more than any
  cleverness in the command.
- **Merging is a design decision.** A drag that produces sixty edits a second should be one undo
  entry, not sixty. Deciding where one gesture ends and the next begins is real work.
- **Deferral costs a frame of truth.** Code that queues a command and then reads the state sees
  the old value until the queue drains. Order across the queue becomes part of the contract.
- **Closures against types.** The book prefers closures where the language has them. They're
  lighter, but a closure can't be inspected, merged, serialized or given a display name.

## In Rust

Rust gives you three shapes, and the choice is mostly about what else the command has to do:

- **An enum of verbs** when the set is closed and the commands are data. It's `Clone`, `Debug`,
  serializable and matchable, which makes it ideal for deferred queues and replays.
- **`Box<dyn Trait>`** when the set is open-ended and each command owns its own undo logic. That
  means type erasure, so merging needs `as_any()` plus a downcast.
- **A closure** (`Box<dyn FnOnce(&mut World)>`) for fire-and-forget work. It can't undo itself
  or explain itself.

The borrow checker makes the deferred form the default rather than an option. While you iterate
over the world you can't mutate it, so you push commands into a buffer and apply them after the
loop. An illustrative sketch, not engine code:

```rust
enum Order { Despawn(EntityId), SetVelocity(EntityId, Vec2) }

let mut orders = Vec::new();
for entity in world.query_entities::<Single<Health>>() {
    if world.get::<Health>(entity).is_some_and(|health| health.points <= 0) {
        orders.push(Order::Despawn(entity));
    }
}
for order in orders { apply(world, order); }
```

## In insiculous_2d

**Verdict: textbook.** The July 2026 audit counted it as textbook already. Command shows up in
three places: editor undo, deferred runtime buffers, and input actions.

**Editor undo.** Every editor write is an `EditorCommand` pushed through one `CommandHistory`:

```rust excerpt=crates/editor/src/commands/mod.rs
pub trait EditorCommand: Send {
    /// Apply the action to the world.
    fn execute(&mut self, world: &mut World);

    /// Reverse the action.
    fn undo(&mut self, world: &mut World);

    /// Human-readable name shown in Edit menu (e.g., "Move Entity").
    fn display_name(&self) -> &str;
```

- `crates/editor/src/commands/mod.rs` · `CommandHistory::execute` — run, then push; this clears
  the redo stack.
- `crates/editor/src/commands/mod.rs` · `CommandHistory::push_already_executed` — the idiom for a
  change the caller has already applied live (texture drops, command-API writes).
- `crates/editor/src/commands/mod.rs` · `CommandHistory::is_dirty` — the unsaved-changes flag is
  the undo stack's top id against a saved watermark, so undoing back to the save reads as
  clean.
- `crates/editor/src/commands/set_commands.rs` · `SetComponentCommand` — one generic
  whole-component write that serves thirteen component types through type aliases.
- `crates/editor/src/commands/entity_commands.rs` · `MacroCommand` — a group of commands that
  undo as one entry.
- `crates/editor/src/command_api/write/mod.rs` · `push_already_executed` — the text command API
  writes through the same history, so a scripted edit is undoable like a clicked one.

Merging is per type, per entity and per field hint, so one scrub is one entry:

```rust excerpt=crates/editor/src/commands/set_commands.rs
    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(newer) if newer.entity == self.entity && newer.field_hint == self.field_hint => {
                self.new = newer.new.clone();
                true
            }
            _ => false,
        }
    }
```

**Deferred buffers at runtime.** Anything that decides while it holds a world borrow queues a
command and applies it afterward:

- `crates/engine_core/src/scripting/commands.rs` · `ScriptCommand` — the closed verb set a script
  may emit. Scripts read a snapshot and act only through this buffer.
- `crates/engine_core/src/behavior_runner/mod.rs` · `BehaviorCommands` — collected while behaviors
  iterate, then applied by `apply_commands`.
- `crates/physics/src/physics_system/mod.rs` · `DeferredBodyOp` — a velocity or reset for a body
  that rapier hasn't synced yet. It drains in call order, so "reset then launch" works.

**Input actions.** `InputMapping<A>` binds sources to a game's own action enum, so a game reads
"Jump", never "Space". The action is the command's name, and the behavior lives in game code.

- `crates/input/src/input_mapping.rs` · `InputMapping` — action → bound sources.
- `crates/editor/src/editor_input.rs` · `EditorAction` — the editor's shortcut verbs, resolved
  from chords and dispatched in one `match`.

## Pitfalls here

- **A gesture is one command.** Gizmo drags mutate the world live, then commit ONE command on
  release, and Undo/Redo are suppressed while a drag is live. Ctrl+Z in mid-drag used to undo
  against half-applied state (`log_archive.md` § "GPP-L7 — gizmo drags vs. commands").
- **Ids must survive undo.** Redo of a create and undo of a delete resurrect the *same*
  `EntityId` via `create_entity_with_id`, so selection and older commands stay valid. Never mint
  a fresh id in an undo path (`log_archive.md` § "GPP-14").
- **`push_already_executed` means the live mutation and `execute()` must agree.** If the inline
  write and the command's `execute` differ, redo produces a different world from the original
  action. `CreateEntityCommand` shipped exactly that bug once: redo silently recreated nothing.
- **Seal gestures with `break_merge()`.** Two separate scrubs of one field merge into one entry
  unless the gesture boundary is sealed. Guard test:
  `test_break_merge_seals_the_gesture_so_two_scrubs_are_two_entries`.
- **A deferred command reads stale state until it drains.** A script that sets a position and
  then reads it back sees the old value until the phase ends. Design scripts around that.
