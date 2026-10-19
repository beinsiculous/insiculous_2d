# Type Object

Book: https://gameprogrammingpatterns.com/type-object.html

Intent: "Allow the flexible creation of new “classes” by creating a single class, each instance of which represents a different type of object."

## The problem it solves

A game has dozens of monster breeds, lane kinds or brick types. If each one is a class (or, in
Rust, a struct and an impl), adding a breed means writing code and recompiling, and designers
can't add one at all. Most breeds differ only in *data*: health, speed, sprite, what they drop.
The book splits the object in two. A **type object** holds what every instance of a kind shares,
each instance holds a reference to its type plus its own state, and new kinds become new data
records, not new code.

## Reach for it when

- You don't know all the kinds up front, or they should be authored without a rebuild.
- The kinds differ mostly in values, not in what they do.
- Many instances share one kind's data, so a reference is cheaper than a copy. (That overlap is
  [flyweight](flyweight.md).)

## Don't reach for it when

- The set of kinds is small, fixed and owned by programmers. A Rust `enum` with a `match` is
  exhaustive, checked at compile time, and simpler.
- The kinds differ in *behavior*, not data. Pushing behavior into type objects means flags,
  strings and switches that reinvent a language. Use [bytecode](bytecode.md), a script, or a
  trait object instead.
- You'd be parsing meaning out of names or tags. That's an untyped type object: every reader
  re-parses, and a typo degrades silently.

## Tradeoffs

- **You manage the types yourself.** Loading, validation, lookup and lifetime of type records
  are your code now, not the compiler's.
- **Behavior is the hard part.** Type-specific *data* is trivial. Type-specific *behavior* needs
  a fixed set of behaviors the data can select from, function pointers, or a scripting layer.
- **Design decisions from the book:** whether the type is encapsulated or exposed; whether the
  type constructs its instances (a factory method on the breed); whether an instance can change
  type; and whether types inherit from each other (none, single, or copy-down at load time).

## In Rust

- **A closed enum is the zero-cost type object** when the kinds are fixed. `#[derive(Deserialize)]`
  on it makes the *choice* of kind data-driven while the kinds themselves stay compiled.
- **A `const` table of structs** (`const LANES: [LaneDef; 10]`) is a type object that lives in
  code but reads like data. It's checked at compile time and needs no loader.
- **A serde-loaded record** (RON or JSON) is the full pattern: new kinds without a rebuild. Fail
  loudly on bad data at load time, not at first use.
- **A descriptor with a `fn() -> Box<dyn Trait>` field** pairs data with a factory, which is the
  book's "breed constructs its monster" decision. Static descriptors (`&'static str`,
  `&'static [T]`) make this free at runtime.
- An instance refers to its type by **index or key**, not by `&'a Type`, so the instance can
  outlive any one borrow of the type table.

An illustrative sketch, not engine code:

```rust
#[derive(Deserialize)]
pub struct Breed { pub name: String, pub health: u32, pub speed_pixels_per_second: f32 }

pub struct Monster { pub breed_index: usize, pub health: u32 }
```

## In insiculous_2d

**Verdict: textbook, mostly as data and const tables, in four places.**

**Native script descriptors** are type objects with a factory. A `ScriptDescriptor` carries the
id, display name, category and parameter specs, plus `make`, which builds the behavior instance.
`Game::register_scripts` adds a game's own.

```rust excerpt=crates/engine_core/src/scripting/registry.rs
pub struct ScriptDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub params: &'static [ParamSpec],
    pub make: fn() -> Box<dyn ScriptBehavior>,
}
```

- `crates/engine_core/src/scripting/registry.rs` · `ScriptRegistry::register` — keyed by id. A
  duplicate id panics when it is registered instead of shadowing.
- `crates/engine_core/src/scripting/builtin/rotate.rs` · `ROTATE_DESCRIPTOR` — the one built-in
  record, which shows the whole shape.

**Sprite-sheet sidecars** are type objects loaded from disk. A `.sheet.ron` next to a PNG names
its cell size, filter and named clips, so game code plays `"walk"` and never cell 8. A bad file
fails at load, before any texture handle exists.

- `crates/engine_core/src/sheet_file.rs` · `SheetFile` — the schema, versioned.
- `crates/engine_core/src/sheet_file.rs` · `parse_sheet_file` — validates version, cell size,
  frames and fps when the sheet loads.

**Game tables** are const type objects. frogger's ten lanes are one table. Each `LaneDef` names
its row, kind, direction, speed, count and segment art, and the lane logic is the same code for
all of them:

```rust excerpt=../games/frogger/src/gameplay/rules.rs
pub(crate) const LANES: [LaneDef; 10] = [
    // --- Road (rows 11..7, above the start row) ---
    LaneDef {
        row: 11,
        kind: LaneKind::Car,
        dir: 1.0,
        speed: 70.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::SushiCart, sprites: 1 },
```

- `../games/frogger/src/types.rs` · `LaneDef` — the lane type record.
- `../games/breakout/src/levels.rs` · `LevelDef` — breakout's level roster (title, scene file,
  chaos mode). The layouts themselves live in editor-authored scene files.

**Scene behaviors** are a closed, serde-tagged enum. A scene picks `PlayerPlatformer`, `Patrol`
and so on with its own parameters, and the behavior code is compiled into the runner. That's
the right call while the set is small and programmer-owned. Moving the eight variants onto the
script registry (data-defined, open to games) is tracked on the board.

- `crates/ecs/src/behavior.rs` · `Behavior` — the wire-frozen variants with serde defaults.

**Not type objects:** `physics/src/presets.rs` holds named constructors such as
`RigidBody::player_platformer`. They build a configured instance and keep no shared type
record.

## Pitfalls here

- **Type data hidden in strings.** breakout reads a brick's armor and drop from `+`-joined
  `EntityTag` tokens (`parse_brick_tag`) and its score from the row digit in its *name*
  (`brick_value_from_name`). Both degrade silently: an unknown token warns and is skipped, and a
  renamed brick scores the minimum. A brick moved to another row in the editor keeps its old
  row's payout. The game's guide freezes the `brick_r{row}_c{col}` names for this reason, and
  moving the data into a component is breakout#2. A new game should author such data as a typed
  component from the start.
- **Wire-frozen enums.** Adding a `Behavior` variant or field changes scene files. Every new
  field needs a serde default, or old scenes stop loading.
- **Descriptor ids are global per registry.** Two games, or a game and the engine, registering
  the same id panic at registration. Namespace ids the way the engine does (`engine::rotate`).
