# Code Quality Audit — engine_core / ecs / ecs_macros / common / src / examples

Scope: `crates/engine_core/src`, `crates/ecs/src`, `crates/ecs_macros/src`,
`crates/common/src`, plus root `src/` and `examples/`.
Read-only audit. No files were modified. `cargo test` and `cargo clippy` were
NOT run (out of scope for a read-only pass); every line number below was read
directly, not inferred.

Sizes: engine_core 17,530 lines · ecs 6,881 · common 1,699 · ecs_macros 80 ·
root src + examples 1,160.

---

## 1. DRY violations

### 1.1 The scene schema is written five times

Adding one concrete component type requires coordinated edits in five places,
and only one of them fails loudly if you miss it.

| # | location | form |
|---|---|---|
| 1 | `crates/engine_core/src/scene_data.rs` | the `ComponentData` enum (25 variants) |
| 2 | `crates/engine_core/src/scene_loader_components.rs:21` | `component_type_name`, a hand-written variant→string match |
| 3 | `crates/engine_core/src/scene_loader_components.rs:42` | `add_component_to_entity`, one arm per variant |
| 4 | `crates/engine_core/src/scene_serializer.rs:80` | `extract_components`, one `if let` per type |
| 5 | `crates/engine_core/src/scene_serializer.rs:294` | `CONCRETE_OR_EXCLUDED`, a hand-maintained `&[&str]` of 16 names |

**#5 has no drift guard.** Grepped the workspace: nothing tests it. Miss a name
there and the component is written to the scene file **twice** — once as its
concrete variant, once as `ComponentData::Dynamic`. It is also cross-namespace:
the list holds `"Camera"` (the ECS type name, what `registry.persistent_names()`
returns) while `component_type_name` returns `"Camera2D"` (the wire name), so the
two lists are not comparable by eye.

The file header at `scene_loader_components.rs:5` already warns "new component
types need an arm in BOTH" — but names only two of the five sites.

**Fix:** derive the exclusion set from the wire enum, or add a test asserting
every concrete `ComponentData` variant resolves to a registry name present in
`CONCRETE_OR_EXCLUDED`.

### 1.2 The 14 grid tuning parameters exist in five places

| location | form |
|---|---|
| `crates/ecs/src/grid_backdrop.rs:57-85` | component fields |
| `cratests/engine_core/src/grid/grid_mesh.rs:32-82` | simulation fields (same 14, plus `substeps`, `origin`, scratch buffers) |
| `crates/engine_core/src/scene_data.rs` `GridBackdrop` variant | wire fields with 14 `#[serde(default = …)]` attrs |
| `crates/engine_core/src/scene_data/grid_defaults.rs` | 14 default functions (44 lines) |
| `crates/engine_core/src/grid/build.rs:21-28` **and** `:52-57` | a field-by-field apply loop **and** a builder chain, both copying the same set |

`build.rs` copies the parameter set twice within one file: once as
`mesh.stiffness = config.stiffness; mesh.damping = config.damping; …` and once
as `.with_stiffness(normalized.stiffness).with_damping(normalized.damping)…`.

**Fix:** make `GridMesh` hold a `GridBackdrop` (or a shared `GridTuning`
struct) instead of mirroring its fields. That collapses `build.rs` to one
assignment and makes adding a tunable a one-line change instead of five.

### 1.3 `ecs::Behavior` and `engine_core::BehaviorData` are the same 10-variant enum twice

`crates/ecs/src/behavior.rs:16` already derives `Serialize`/`Deserialize`. The
mirror in `crates/engine_core/src/behavior_data.rs` (240 lines) plus its two
hand-written `From` impls exists only to attach serde defaults for old scene
files. Adding a behavior variant costs four edits: the ecs enum, the
`BehaviorData` enum, both `From` arms, and the runner dispatch arm in
`behavior_runner/mod.rs`.

**Fix:** put the serde defaults on `ecs::Behavior` itself and delete the mirror;
or accept the duplication and add a test that round-trips every variant so a
missed arm fails loudly.

### 1.4 Achievements and scores are the same persistence machine

`crates/engine_core/src/achievements/mod.rs` and
`crates/engine_core/src/scores.rs` each define, independently:

- `in_memory()` / `with_save_path()` constructor pair
- `save()` returning `Ok(false)` when no path is configured
- `reset()` that clears then persists with `merge: false`
- `save_to(&self, path: &Path, merge: bool)` with a structurally identical
  read-merge-write body over the `save_store` seam
- an error enum that is exactly `Io(#[from] io::Error)` +
  `Serde(#[from] serde_json::Error)` — `achievements/mod.rs:93`, `scores.rs:38`
- the unix-seconds idiom, verbatim: `achievements/mod.rs:277` and `scores.rs:102`

```rust
SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
```

**Fix:** one `MergingJsonStore<T: Merge>` over `save_store`, one error type,
one `unix_seconds()` helper.

### 1.5 `follow_entity` and `follow_tagged` are the same function

`crates/engine_core/src/behavior_runner/handlers.rs:185` and `:211` differ only
in how the target position is found (`named_entities` lookup vs
`find_nearest_tagged_position`). Both then run identical distance-gate and
velocity-push logic.

**Fix:** one function taking an already-resolved `Option<Vec2>` target.

### 1.6 Texture loading duplicates path resolution — and one path skips the cache

`crates/engine_core/src/assets.rs:201` (`load_texture_filtered`) and `:239`
(`load_texture_with_config`) both perform relative-path joining against
`config.base_path`, log gating on `config.log_loading`, and `handle_to_path`
insertion.

Worse: `load_texture_with_config` **skips the `loaded_by_path` dedupe cache
entirely**. That cache was added specifically so a scene reload does not
re-decode and re-upload its whole texture set; this path still does.

### 1.7 Colors are destructured by hand in eight places

`common::Color` exists with `From<Vec4>` in both directions, but the wire schema
uses bare `(f32, f32, f32, f32)` and the components use `glam::Vec4`. Result:

- `scene_serializer.rs:103` — `color: (s.color.x, s.color.y, s.color.z, s.color.w)`
- `scene_serializer.rs:234` — same shape for `UiLabel`
- `scene_serializer.rs:243` — twice in one expression for `UiPanel`
  (`background` and `border`)
- `scene_loader_components.rs:79`, `:151`, `:288`, `:307` (×2) — the inverse,
  `glam::Vec4::new(color.0, color.1, color.2, color.3)`

`GridBackdrop` already uses `color: g.color.into()` at `scene_serializer.rs:142`,
proving the conversion exists and is simply not used elsewhere.

---

## 2. SRP violations

### 2.1 `SceneLoader::add_component_to_entity` — 346 lines

`crates/engine_core/src/scene_loader_components.rs:42-387`. One function
handling 15 component constructions plus two `#[cfg(feature = "physics")]`
split bodies (each with a `#[cfg(not(...))]` "suppress unused variable" tuple
at `:226` and `:278`). Largest single function in the cluster.

**Fix:** one `fn build_<component>(data, assets) -> Result<_, SceneLoadError>`
per arm; the match becomes a dispatch table.

### 2.2 `extract_components` — 202 lines

`crates/engine_core/src/scene_serializer.rs:80-281`. The same problem inverted.
Splitting both symmetrically makes the loader/serializer pairing visible for the
first time.

### 2.3 `GameRunner` has 33 fields

`crates/engine_core/src/game.rs:189-278`. It owns: window, renderer, assets,
audio, input, gamepad backend, player bindings, input-save retry state, UI,
frame timing, glyph cache, time scale, exit flag, render-fatal latch, pending
window title, pending UI clip, scene, achievements, scores, particles, line
buffer, grid backdrops, two sprite batchers, localization strings, base font,
locale font cache, pending UI events, and an init flag (plus two wasm-only
fields).

Several are cohesive clusters wanting their own struct:

- `pending_window_title`, `pending_game_ui_clip`, `exit_requested`,
  `render_fatal` → one "frame writeback / latches" group
- `strings`, `base_font`, `locale_fonts` → one localization group

The docstring at `game.rs:180-188` claims delegation to five focused managers,
but three of the five (`AssetManager`, `AudioManager`, `GameLoopManager`) are
just owned fields, and a sixth listed manager does not appear at all (see 3.1).

### 2.4 `GameContext` has 18 public fields, five of them writeback channels

`crates/engine_core/src/contexts.rs:52-122`. `chaos_mode`, `time_scale`,
`exit_requested`, `window_title`, and `game_ui_clip` are all "write here and the
engine reads it back after `update()`". Nothing at a write site indicates that.

**Fix:** a small explicit API — `ctx.request_exit()`, `ctx.set_title(..)` —
leaving the plain data fields as fields.

### 2.5 `AchievementManager` mixes domain and presentation

`crates/engine_core/src/achievements/mod.rs` holds registration, unlock state,
JSON persistence, toast timers, a 14-field `ToastStyle` (`:109-136`), and
`draw_toasts` (`:326-358`), which does UI layout math and calls
`ui.panel_styled` / `ui.label_styled`.

**Fix:** move `ToastStyle`, `faded`, and `draw_toasts` to
`achievements/toast.rs`.

### 2.6 `ecs::World` is a god object

`crates/ecs/src/world.rs:33-52` — 10 fields, 41 public methods, spanning
entities, generation tracking, component storage, system registry, resource
storage, event bus, and hierarchy. This is a conventional ECS shape and I would
not rewrite it, but it is the widest single surface in the cluster.

### 2.7 The reference examples are the worst offenders by function length

- `examples/hello_world.rs:330` — `update` is 185 lines
- `examples/hello_world.rs:184` — `init` is 143 lines
- `examples/editor_demo.rs:131` — `init` is 89 lines
- `examples/editor_demo.rs:229` — `update` is 85 lines

These are the files agents and new game authors copy from, so their shape
propagates into `../games/`.

Other functions over 60 lines:

| lines | location |
|---|---|
| 82 | `engine_core/src/game/render.rs:44` — `render_frame` |
| 71 | `engine_core/src/game.rs:281` — `GameRunner::new` |
| 65 | `engine_core/src/game/frame_tail.rs:15` — `post_update` |
| 105 | `ecs/src/hierarchy_system.rs:161` — `update` |
| 63 | `src/bin/editor.rs:98` — `main` |
| 61 | `engine_core/src/gamepad_backend.rs:68` — `pump` |

---

## 3. KISS violations

### 3.1 `SceneManager` has zero users

`crates/engine_core/src/scene_manager.rs` is 153 lines plus five tests. The only
reference anywhere in the workspace is the `pub use` at `lib.rs:90`.
`GameRunner` holds a bare `Scene` (`game.rs:244`), not a `SceneManager`.

Yet both `crates/engine_core/CLAUDE.md` and the root `CLAUDE.md` list it as one
of the five focused managers of the Manager Pattern.

**Fix:** wire it in or delete it — and update both guides. 153 lines of
documented phantom API actively misleads the next agent session.

### 3.2 `UIManager` is a three-method forwarder

`crates/engine_core/src/ui_manager.rs`. `begin_frame` calls
`ui_context.begin_frame_dt`; `ui_context()` returns `&mut self.ui_context`;
`end_frame` calls two `UIContext` methods. It adds a name and no behavior. Its
two tests test `UIContext`.

### 3.3 `EngineError` is dead in production

Defined at `crates/engine_core/src/lib.rs:145`, referenced only in
`tests/init.rs` and re-exported from the prelude. Meanwhile `run_game` — the
public entry point every game calls — returns
`Result<(), Box<dyn std::error::Error>>` (`game.rs:153`).

### 3.4 `TextureResolver` has one production impl and six copy-pasted test stubs

`crates/engine_core/src/texture_ref.rs:31`. `AssetManager` is the only real impl
(`:51`). The trait is a legitimate headless-testing seam and should stay — but
see 8.1 for the stub duplication it caused.

### 3.5 Three `#[allow(clippy::too_many_arguments)]`

`crates/engine_core/src/behavior_runner/handlers.rs:18`, `:95`, `:139`. Each is
a behavior handler taking its variant's fields as loose positional parameters
(up to 10, several of them `f32`, so a swapped pair compiles silently).

**Fix:** pass the `Behavior` variant itself, or a per-variant params struct.
Removes the lint and the ordering hazard together. These are the only three
`#[allow(...)]` in the entire audited scope — otherwise the no-allow rule holds.

### 3.6 `sort_batch_refs` recomputes its sort key inside the comparator

`crates/engine_core/src/game/render.rs:132-145`. Each comparison scans both
batches' full instance lists twice (once for min depth, once for max). For a
frame with many batches this is repeated O(n) work per comparison inside an
O(n log n) sort.

**Fix:** decorate-sort-undecorate — compute `(min, max, texture, clip)` once per
batch, sort the tuples.

---

## 4. Non-human-readable names

This is the strongest category in the cluster. The house rule
(`estimatedStartTime`, not `estStart`) is largely honored, and there is **no
abbreviated identifier in any public signature** in the audited crates. No
abbreviated file or folder names anywhere.

Short `let` bindings outside test modules, per crate:

| crate | count | worst files |
|---|---|---|
| engine_core | ~90, concentrated in 4 files | `particles/manager.rs` (23), `localization.rs` (17), `ui_element_system.rs` (15), `assets/sprite_sheet.rs` (15) |
| ecs | ~11 | `state_machine.rs` — `sm` ×18, almost all in tests and doc examples |
| common | ~25 | `color.rs`, `rect.rs`, `transform.rs` — `r`/`g`/`b`/`a`/`t`, all tight math, acceptable |
| ecs_macros | 0 | — |
| root src + examples | 2 | — |

**Genuine offenders:**

1. `crates/engine_core/src/achievements/mod.rs:198` — `let mut mgr = Self::in_memory();`
   The only non-test `mgr` in the audited scope. Should be `manager`.
2. `crates/engine_core/src/scene_serializer.rs:88-272` — component bindings named
   `t` (Transform2D), `s` (Sprite), `c` (Camera), `tm` (Tilemap),
   `g` (GridBackdrop), `a` (SpriteAnimation), `rb` (RigidBody), `col` (Collider),
   `l` (UiLabel), `p` (UiPanel), `b` (UiButton, then reused for Behavior at `:261`).
   These are not math variables. `b` meaning two different types 12 lines apart
   is the sharpest edge.
3. `crates/engine_core/src/behavior_runner/handlers.rs:38,49,75,121,177` —
   `vel_x`, `vel`. Borderline; `velocity` costs nothing.

`ctx` is accepted project-wide and used consistently.

---

## 5. Comment load

### 5.1 Top 10 by non-doc comment lines per code line

| ratio | comment lines | code lines | file |
|---|---|---|---|
| 0.561 | 23 | 41 | `engine_core/src/game/frame_tail.rs` |
| 0.323 | 30 | 93 | `engine_core/src/game/render.rs` |
| 0.312 | 20 | 64 | `engine_core/src/prelude.rs` |
| 0.211 | 41 | 194 | `engine_core/src/game/app_handler.rs` |
| 0.187 | 20 | 107 | `src/bin/editor.rs` |
| 0.179 | 17 | 95 | `engine_core/src/particles/system.rs` |
| 0.178 | 13 | 73 | `engine_core/src/game/web.rs` |
| 0.162 | 57 | 351 | `examples/hello_world.rs` |
| 0.142 | 40 | 282 | `engine_core/src/game.rs` |
| 0.126 | 36 | 286 | `ecs/src/hierarchy_system.rs` |

### 5.2 Assessment

Most of this is **good** comment: it records *why*, not *what*. Keep:

- `game/render.rs:118` — the Firefox in-process-WebGPU crash rationale for the
  fail-stop
- `game/render.rs:51-54` — why game and UI sprites use separate batchers
  (painter's algorithm)
- `ecs/component.rs:64` and `:102` — the `Box<dyn Component>` blanket-impl
  downcast trap
- `component_registry/mod.rs:292` — why the re-entrancy guard exists (RwLock
  same-thread deadlock)
- `scores.rs:112` — why equal scores insert after existing ties

### 5.3 Three comments that should be replaced by naming or structure

1. **`crates/engine_core/src/game/render.rs:46`**
   ```rust
   // Prepare glyph textures for text rendering
   if let Some(asset_manager) = &mut self.asset_manager {
       self.glyph_textures.prepare(ui_commands, asset_manager);
   ```
   Pure restatement of `glyph_textures.prepare`.

2. **`crates/engine_core/src/game/render.rs:110`**
   ```rust
   // Get textures from asset manager (need to reborrow after RenderContext)
   ```
   The reborrow note describes a scope the code already shows. The block wants
   to be a named method (`submit_frame`).

3. **`crates/engine_core/src/game/render.rs:50` and `:86`**
   ```rust
   // Phase 1: Game sprites — …
   // Phase 2: UI sprites — …
   ```
   Section headers inside an 82-line function are the function asking to be
   split into `collect_game_sprites` / `collect_ui_sprites`.

Honourable mention: `frame_tail.rs` at 0.561 is the highest ratio in the
cluster, and nearly every one of its comment blocks restates the method name it
sits above (`// Step the particle system`, `// Forward the line vertices`,
`// Draw achievement toasts`). The *reasons* embedded in them (time_scale
freezing, splice order) are worth keeping; the narration is not.

### 5.4 Stale status-narrative references in source

Issue numbers, review-round tags, and audit codes appear in source comments:

| location | count |
|---|---|
| `crates/engine_core/src` | 55 |
| `crates/ecs/src` | 15 |
| `crates/common/src` | 1 |
| root `src/` | 2 |
| `examples/` | 0 |

Representative:

- `game.rs:260` — `(GPP-15)`
- `game.rs:155` — `(idempotent — issue #43)`
- `scene_serializer.rs:135` — `// GridBackdrop (#46)`
- `scene_serializer.rs:270` — `// Scripts — … (issue #44)`
- `window_manager.rs:124` — `(kimi #41 F1)`
- `script_data.rs:111, 165, 193, 242` — four separate `kimi #44 F*` tags in one file
- `render_manager.rs:38, 145, 314, 351, 402` — five `issue #NN` tags
- `component_registration.rs:2` — `(issue #43, ecs GPP-16)`
- `contexts.rs:98, 142` — `(issue #41/#52)`, `(issue #41)`
- `grid/mod.rs:42`, `scene_data/grid_defaults.rs:3`, `frame_tail.rs:33` — `(#46)`

Several are load-bearing (they point at a rationale worth keeping), but the
ticket number is the least durable part. Rule worth adopting: **keep the reason,
drop the number** — the number resolves to a closed issue nobody will open.

---

## 6. Game Programming Patterns alignment

### 6.1 Followed well

| pattern | where | note |
|---|---|---|
| **Command** | `behavior_runner/handlers.rs:4` | handlers collect `BehaviorCommands`, applied after iteration — the textbook fix for mutating the world you are iterating |
| **Update Method / Game Loop** | `game_loop_manager.rs` | genuinely the only frame timer; the native/wasm frame-driving split is documented as deliberate (`game/app_handler.rs`) rather than accidental |
| **Component** | `ecs/src/component_registry/mod.rs:48-59` | `ComponentEntry` is a fn-pointer table monomorphized at `register::<T>()`. Clean type erasure; beats the usual `Box<dyn Trait>` on both allocation and clarity |
| **Prototype** | `scene_loader.rs` — `SceneInstance::spawn_prefab` | override semantics match scene-file semantics; failed spawns leave no debris |
| **Object Pool** | `particles/`, and `game/render.rs:54` | persistent `SpriteBatcher`s `clear()` to retain capacity — steady-state frames allocate nothing |
| **Observer** | `ecs/src/event.rs` | typed per-frame queues; the deliberate one-frame UI-press latency is documented at `game.rs:272` |
| **State** | `ecs/src/state_machine.rs`, `ecs/src/behavior.rs` `BehaviorPhase` | `Idle ⇄ Chasing`, `Patrolling → Waiting` are real FSMs, not bool soup |
| **Dirty Flag** | `ecs/src/hierarchy_system.rs` | value-compare cache; clean frames recompute nothing. Also `pending_window_title` — one window round-trip per frame, only when requested |
| **Service Locator** | *not used* | `GameContext` is passed explicitly. Correct call |

### 6.2 Anti-patterns hit

**Singleton / global mutable state.**
`COMPONENT_REGISTRY: OnceLock<RwLock<ComponentRegistry>>` at
`crates/ecs/src/component_registry/mod.rs:289` is the cluster's one true global.
It is handled about as carefully as a global can be:

- name collisions panic at startup (`:124`) rather than corrupting scenes later
- lock poisoning is recovered (`:373`, `:384`) — registration is idempotent inserts
- a `thread_local` re-entrancy guard (`:291-320`) converts a same-thread RwLock
  deadlock into a clear panic

Residual cost is real: registration order is global process state, so a test
that registers a game component can affect another test in the same binary; and
the standalone editor binary cannot see game types at all (documented at
`component_registration.rs:20`, but a direct consequence of the singleton).

Other globals: `ecs/src/entity.rs:19` (`LazyLock<EntityIdGenerator>` — process-wide
entity ids), `engine_core/src/save_store.rs:110` (`thread_local` web backend),
`engine_core/src/web/mod.rs:27` (`static PAGE_EXITED: AtomicBool`). All three are
narrow and justified.

**God object.** `GameRunner` (33 fields), `World` (41 public methods). See 2.3, 2.6.

**Stringly-typed dispatch.** The dynamic tier keys on `&'static str`
(`insert_component(world, entity, "Health", json)`); `Scores` keys on
caller-supplied mode strings; `Behavior` targets entities by tag string. All
deliberate seams — and the scene loader's unregistered-name path is a *hard
error* (`scene_loader_components.rs:365`) precisely because the string can be
wrong, which is the right call.

**Hidden coupling.** `GameContext`'s five writeback fields (2.4). Also
`ctx.chaos_mode` is read-write with engine persistence, which is documented in
`CLAUDE.md` as a footgun — meaning it has already cost someone time.

**Data Locality.** `ComponentStore = HashMap<EntityId, Box<dyn Component>>`
(`ecs/src/component.rs:40`) is pointer-chasing per component access. The crate
guide records this as an accepted GPP-02 tradeoff with an explicit revisit
trigger (profiling shows component access dominating a frame, or games exceed a
few thousand entities). Correctly decided and correctly documented — noting it
only for completeness.

---

## 7. Rust best-practice issues

### 7.1 `unwrap` / `expect` discipline — excellent

Scanned every non-test file in scope. **Zero `unwrap()` or `expect()` in
production code paths.** Every hit was a doc example or inside `#[cfg(test)]`.
The one production `expect` is `crates/ecs/src/event.rs:93`
(`"event queue type mismatch"`), guarding a `TypeId`-enforced invariant — the
legitimate use.

### 7.2 Zero `#[must_use]` across all three crates

Notable omissions:

- `Scores::submit` (`scores.rs:101`) returns "did it qualify" — easy to drop
- `AchievementManager::unlock` (`achievements/mod.rs:268`) returns "did this
  call unlock it"
- every builder method on `GridMesh` (`grid_mesh.rs:139-182`) returns `Self`
  and silently does nothing if discarded
- `SpriteAnimation::play` / `ensure_playing` return `false` for an unknown clip

### 7.3 `String` error types

`ComponentFactoryFn` and all of `ComponentRegistry`'s fallible methods return
`Result<_, String>` (`component_registry/mod.rs:29, 202-278`). Callers in the
scene loader re-wrap those strings into `SceneLoadError::ComponentError` via
`format!` (`scene_loader_components.rs:376`). A typed `RegistryError` would let
the loader distinguish "unknown type" from "deserialization failed" instead of
matching on prose.

### 7.4 `Box<dyn Error>` at the public entry point

`run_game` (`game.rs:153`) returns `Result<(), Box<dyn std::error::Error>>`
while the crate defines and exports an unused `EngineError` (3.3).

### 7.5 `EntityId` API inconsistency

`get` / `get_mut` / `component_types` take `EntityId` by value
(`world.rs:283, 293, 313`); `add_component` / `remove_component` /
`has_component` take `&EntityId` (`world.rs:258, 270, 301`). `EntityId` is
`Copy`, so the reference form buys nothing. This is recorded as a known
convention in `crates/ecs/CLAUDE.md`, making it a decision rather than an
oversight — but it is a papercut at every call site and a recurring source of
`&`/no-`&` compile errors for agents.

### 7.6 `as` casts

`Color::to_rgba8` (`common/src/color.rs:149-155`) uses `(self.r * 255.0) as u8`,
which **truncates rather than rounds**: `0.5` → `127`, not `128`. So
`from_rgba8(x) → to_rgba8()` is not identity for most values. Because
`#solid:RRGGBB` scene refs are built from these bytes
(`texture_ref.rs:119`), the drift is user-visible across a save/load cycle.
Fix: `(self.r * 255.0).round().clamp(0.0, 255.0) as u8`.

Other `as` casts reviewed (`window_manager.rs:189`, `gamepad_backend.rs:71`,
`menu_panel.rs:147`, `pause.rs:151`, `debug.rs:74`) are all bounded by
construction and fine.

### 7.7 `impl From` opportunities

`AssetConfig: From<&GameConfig>` (`assets.rs:99`) is exactly right and shows the
pattern is understood. `WindowConfig` is still assembled by hand at
`game.rs:283-285` and would take the same treatment.

### 7.8 `clone()` volume

Files by `.clone()` count: `script_data.rs` (18), `behavior_data.rs` (14),
`scene_loader.rs` (9), `scene_serializer.rs` (8), `scene_loader_components.rs`
(7), `achievements/mod.rs` (7).

Most are unavoidable at the ECS↔wire boundary. Worth a look:
`scene_serializer.rs:129` — `tiles: tm.tiles.clone()` clones a whole tilemap's
tile vector on every save; and `scores.rs:160` / `achievements/mod.rs:380` —
`self.modes.clone()` / `self.unlocks.clone()` on every write-through save.

### 7.9 `pub` fields

`GameConfig` (29 `pub` items), `GameContext` (18), `RenderContext` (6),
`ToastStyle` (14), `GridMesh` (13 pub + 5 private), `GridBackdrop` (15). For
plain config/data structs this is the right call. The one that is not is
`GameContext`'s writeback subset (2.4).

### 7.10 `Default` implementations

Correctly derived or hand-written throughout; `SceneData`, `GridData`,
`ColliderShapeData`, `AssetConfig`, `ToastStyle`, `WorldConfig` all hand-write
`Default` because they have non-zero defaults. No misuse found.

---

## 8. Near-identical siblings and misnamed modules

### 8.1 `StubResolver` is copy-pasted six times

| location |
|---|
| `crates/engine_core/src/scene_dynamic_tests.rs:24` |
| `crates/engine_core/src/scene_serializer_roundtrip_tests.rs:20` |
| `crates/engine_core/src/scene_scripts_tests.rs:22` |
| `crates/engine_core/tests/prefab_spawning.rs:17` |
| `crates/engine_core/tests/scene_loader_parse.rs:189` |
| `crates/editor_integration/src/editor_game/scene_io_tests.rs:22` |

Identical 8-line body in every case. The `roundtrip(world)` helper and
`test_texture_path` are duplicated alongside it in three of them.

`scene_dynamic_tests.rs` and `scene_scripts_tests.rs` share their opening ~35
lines almost verbatim.

**Fix:** one `#[cfg(test)] pub mod test_support` in `engine_core`, exported for
`editor_integration`'s dev-dependency use.

### 8.2 Modules whose name does not describe their contents

| module | contains | better name |
|---|---|---|
| `engine_core/src/assets.rs` | texture manager + config + handle→path bookkeeping. No fonts, no sounds, no generic assets — despite the crate guide saying "Asset loading (textures, fonts)" | `texture_assets.rs` |
| `engine_core/src/grid/build.rs` | normalizes a `GridBackdrop` config and applies it to an existing `GridMesh` (as much sync as build) | `grid/sync.rs` or `grid/apply.rs` |
| `engine_core/src/achievements/mod.rs` | also contains the toast rendering system | split per 2.5 |
| `engine_core/src/scene_manager.rs` | nothing that runs | delete per 3.1 |

---

## 9. Ranked top 10 highest-value changes

1. **Add a drift guard for `CONCRETE_OR_EXCLUDED`** (`scene_serializer.rs:294`).
   The only finding here that can silently corrupt a user's scene file, by
   double-writing a component as both a concrete variant and `Dynamic`. One test
   comparing the concrete variant set against the registry closes it.

2. **Split `add_component_to_entity`** (346 lines,
   `scene_loader_components.rs:42`) into per-component builder functions.
   Largest readability win in the cluster, and it makes the loader/serializer
   symmetry visible for the first time.

3. **Collapse the grid parameter quintuplication** by having `GridMesh` hold a
   `GridBackdrop`. Removes ~60 lines of mechanical copying across three files
   and turns "add a tunable" from a five-file change into a one-line change.

4. **Delete `SceneManager` or wire it in** — and remove it from both `CLAUDE.md`
   files if deleted. 153 lines of documented phantom API is actively misleading
   to the next agent session, which is the single worst property a file can have
   in this codebase.

5. **Extract one `MergingJsonStore`** shared by `Scores` and
   `AchievementManager`. Two independent copies of a read-merge-write
   persistence protocol with subtly different merge rules is exactly where a
   multi-tab web bug will land.

6. **Make `run_game` return `EngineError`** instead of `Box<dyn Error>`, and use
   the type the crate already defines and exports. Cheap; fixes the crate's
   public face.

7. **Add `#[must_use]`** to `Scores::submit`, `AchievementManager::unlock`,
   `SpriteAnimation::play`/`ensure_playing`, and every `GridMesh` builder method.
   Currently zero occurrences across three crates.

8. **Consolidate the six `StubResolver` copies** into one shared test-support
   module in `engine_core`, exported for `editor_integration`.

9. **Split `AchievementManager`'s toast rendering** into `achievements/toast.rs`.
   Separates domain state from presentation and brings the file under a
   comfortable size.

10. **Fix `load_texture_with_config` to use the dedupe cache**
    (`assets.rs:239`), and factor the shared path-resolution tail out of it and
    `load_texture_filtered`. This one is a latent performance regression — the
    cache added to stop scene reloads re-uploading textures does not cover this
    path — not just a style point.

---

## Appendix: what is in good shape

Worth recording so a future pass does not "fix" it:

- **No `unwrap`/`expect` in production code.** Genuinely clean.
- **Only three `#[allow(...)]` in ~26k lines**, all the same lint, all in one file.
- **The component registry's fn-pointer design** is better than the obvious
  alternative and should not be replaced with trait objects.
- **The `common` crate** is small, dependency-light, and well factored.
  `Color`'s WCAG `luminance`/`contrast_ratio` are correct (verified the sRGB
  piecewise transfer at `color.rs:163`).
- **`ecs_macros`** is 80 lines, does one thing, handles the enum/union error case
  with a proper `syn::Error`, and has a doc test that actually runs.
- **The comment culture** — recording *why* and recording failed approaches — is
  the right instinct. The problem is ticket numbers and narration, not the
  practice.
