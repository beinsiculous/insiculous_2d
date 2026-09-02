# Structural refactor designs — insiculous_2d code-quality cleanup

Every design below was written after reading the actual code named in the task. Line numbers
refer to the tree at commit `d58f3c3` (branch `dev`). Sequencing is another agent's job; each
design stands alone and names its own files, hazards, tests, and verdict.

## Ground truth that shapes several verdicts

- `ecs::Behavior` (`crates/ecs/src/behavior.rs`) already carries every serde default that
  `BehaviorData` (`crates/engine_core/src/behavior_data.rs`) has, with identical variant and
  field names and no serde renames. The mirror is byte-for-byte redundant on the wire.
- `GameContext` is built at two sites with an 18-field literal followed by the same 8-line
  writeback tail: `crates/engine_core/src/game.rs:461-498` and
  `crates/engine_core/src/game/app_handler.rs:213-250`.
- The workspace pins `ron = "0.12.0"`. Scene RON goes through exactly two choke points:
  `scene_loader.rs:94` (`ron::from_str`) and `scene_serializer.rs:333` (`to_string_pretty`).
- glam already implements `From<Vec4> for (f32,f32,f32,f32)` and the reverse; the serializer
  uses it once (`scene_serializer.rs:142`, `g.color.into()`). The other seven hand-destructured
  colors (audit 1.7) can all become `.into()`.
- `crates/editor/src/stored_component/mod.rs` is 585 lines. Anything added there breaches the
  600-line ceiling, so new editor types go in sibling files.
- The registry name of the camera component is `"Camera"` (`ecs/src/sprite_components.rs:417`);
  the wire variant is `ComponentData::Camera2D`. Any exclusion-list derivation must carry both.
- `engine_core` has `default = ["physics"]`; the `#[cfg(feature = "physics")]` split in the
  loader/serializer is live and must survive.
- Production gizmo commits use `CommandHistory::push_already_executed` (never merges); only the
  unit test `test_transform_gizmo_merge` exercises gizmo-on-gizmo merging via
  `try_merge_or_execute`.
- `ButtonTracker::clear_frame_state` runs once per frame from `InputHandler::end_frame`; the
  other gamepad callers (`gamepad.rs:229-269`) are inside `#[cfg(test)]`.

---

## A. Scene schema written five times — GO

**Target.** `ComponentData` stays the wire SSOT. The loader match and the serializer if-let
chain each become one-liners over per-component functions, and the serializer side is a table
that also yields the dynamic-tier exclusion list.

```rust
// scene_loader_components.rs — one pure builder per concrete variant (no World access)
fn build_sprite(
    texture: &str, offset: (f32, f32), rotation: f32, scale: (f32, f32),
    color: (f32, f32, f32, f32), depth: f32, emissive: f32,
    tex_region: (f32, f32, f32, f32), visible: bool,
    assets: &mut impl TextureResolver,
) -> Result<Sprite, SceneLoadError> {
    let texture_handle = assets.resolve_texture(texture)?;
    Ok(Sprite { texture_handle: texture_handle.id, offset: offset.into(), rotation,
                scale: scale.into(), color: color.into(), depth, visible, emissive,
                tex_region: [tex_region.0, tex_region.1, tex_region.2, tex_region.3] })
}
// the match arm becomes one line:
ComponentData::Sprite { texture, offset, rotation, scale, color, depth, emissive, tex_region, visible } =>
    Self::add_component_logged(world, entity_id,
        build_sprite(texture, *offset, *rotation, *scale, *color, *depth, *emissive, *tex_region, *visible, assets)?),

// scene_serializer.rs — one extractor per variant, plus the table
type Extractor = fn(&World, EntityId, &dyn Fn(u32) -> String) -> Option<ComponentData>;

/// One row per concrete `ComponentData` variant. The table is the ONLY place that
/// knows both names of a component; `append_dynamic_components` skips every
/// `registry_name` here, so a component is never written twice.
pub(crate) struct ConcreteComponent {
    /// Wire variant name, what `SceneLoader::component_type_name` returns ("Camera2D").
    pub wire_name: &'static str,
    /// ECS registry name, what `ComponentRegistry::persistent_names` returns ("Camera").
    pub registry_name: &'static str,
    pub extract: Extractor,
    /// A representative value of the variant, for the drift test only.
    #[cfg(test)] pub sample: fn() -> ComponentData,
}

fn concrete_components() -> Vec<ConcreteComponent> {   // a fn, not a const: #[cfg] on statements is always legal
    let mut rows = vec![
        row("Transform2D", "Transform2D", extract_transform),
        row("Sprite", "Sprite", extract_sprite),
        row("Camera2D", "Camera", extract_camera),
        row("Tilemap", "Tilemap", extract_tilemap),
        row("GridBackdrop", "GridBackdrop", extract_grid_backdrop),
        row("SpriteAnimation", "SpriteAnimation", extract_sprite_animation),
        row("UiLabel", "UiLabel", extract_ui_label),
        row("UiPanel", "UiPanel", extract_ui_panel),
        row("UiButton", "UiButton", extract_ui_button),
        row("Behavior", "Behavior", extract_behavior),
        row("EntityTag", "EntityTag", extract_entity_tag),
        row("Scripts", "Scripts", extract_scripts),
    ];
    #[cfg(feature = "physics")]
    rows.extend([row("RigidBody", "RigidBody", extract_rigid_body),
                 row("Collider", "Collider", extract_collider)]);
    rows
}

/// Registry names with no wire variant that are still never emitted as `Dynamic`:
/// Name lives on `EntityData.name`; GlobalTransform2D is computed.
const EXCLUDED_NON_WIRE: &[&str] = &["Name", "GlobalTransform2D"];

fn extract_components(world: &World, entity: EntityId, texture_path_fn: &dyn Fn(u32) -> String) -> Vec<ComponentData> {
    let rows = concrete_components();
    let mut components: Vec<ComponentData> =
        rows.iter().filter_map(|row| (row.extract)(world, entity, texture_path_fn)).collect();
    append_dynamic_components(world, entity, &rows, &mut components);
    components
}
```

Row order in the table is the current emission order, so saved files do not reorder.
`append_dynamic_components` skips `rows.iter().any(|r| r.registry_name == name)` plus
`EXCLUDED_NON_WIRE`; the hand list `CONCRETE_OR_EXCLUDED` is deleted.

`component_type_name` stays an exhaustive `match` on the enum. Adding a variant still fails
to compile there; the header comment points at the table as the next edit.

**Files touched.** `crates/engine_core/src/scene_loader_components.rs` (346-line fn becomes
~15 one-line arms plus ~14 builders, each under 25 lines; the two physics builders keep their
`#[cfg]` twin bodies and absorb the `let _ = (..)` unused-suppression so the arm itself has no
cfg), `crates/engine_core/src/scene_serializer.rs` (202-line fn becomes the table plus ~14
extractors), `crates/engine_core/src/scene_serializer_tests.rs` (drift test),
`crates/engine_core/CLAUDE.md` (the "arms in BOTH" note becomes "a builder, an extractor, and
a table row").

**Hazards found in the code.**
- The wire/registry name split (`Camera2D`/`Camera`) is real. A single-name table silently
  repeats the double-write bug for the camera.
- The physics rows cannot live in a `const` array with `#[cfg]` on elements; a function with
  `#[cfg]` on `rows.extend` is the reliable form.
- `Scripts` extraction needs `world` (target-name resolution) and `SpriteAnimation` extraction
  needs the clip clone; both fit the `Extractor` signature. `Dynamic` is not a row.
- `Vec<ConcreteComponent>` is rebuilt per entity per save. Saves are user-initiated; the cost
  is nil. If a reviewer objects, a `OnceLock` is a two-line change.
- Do not touch `SceneLoader::merge_components` (`scene_loader.rs:295`); it compares wire names
  between `ComponentData`s and is unaffected.

**Tests.**
1. The drift guard that closes the audit's double-write bug: register the builtins, insert the
   registry default of every `persistent_names()` type onto one entity via
   `registry.insert_component(world, entity, name, default_json)`, serialize with
   `world_to_scene_data`, then assert (a) no `Dynamic { component_type }` names any row's
   `registry_name`, (b) every row's `registry_name` is `registry.is_registered`, and (c) for
   every persistent registry name exactly one emitted component maps to it (via the table's
   `registry_name` for concrete rows, `component_type` for Dynamic).
2. `SceneLoader::component_type_name(&(row.sample)()) == row.wire_name` for every row.
3. Existing round-trip tests (`scene_serializer_tests.rs`, `scene_dynamic_tests.rs`,
   `tests/scene_loader_parse.rs`) run unchanged; that is the behavioral lock.

**Also in this pass.** The seven remaining color destructurings become `.into()`; the
one-letter bindings (`t`, `s`, `c`, `tm`, `g`, `a`, `rb`, `col`, `l`, `p`, `b` used for two
types) disappear with the extractors.

**Verdict: go.**

---

## B. Grid tuning in five places — GO, Half 2 gated on a spike

Two halves. Half 1 is safe and collapses three of the five copies. Half 2 collapses the wire
side to zero and needs a five-minute compatibility check first.

### Half 1 — the mesh holds the config (safe)

```rust
// crates/engine_core/src/grid/grid_mesh.rs
pub struct GridMesh {
    /// The NORMALIZED configuration this lattice was built from. Tunables are read
    /// from here every step. The lattice fields (topology, cols, rows, spacing) are
    /// what the springs encode: never edit them on a live mesh — rebuild.
    pub config: GridBackdrop,
    pub origin: Vec2,
    /// Physics substeps per frame. Not scene data — stays on the mesh.
    pub substeps: u32,
    nodes: Vec<GridNode>, springs: Vec<Spring>, activity: Vec<f32>,
    line_scratch: Vec<LineVertex>, force_scratch: Vec<Vec2>,
}
impl GridMesh {
    /// The one constructor. `config` must already be normalized
    /// (`GridBackdrop::normalized`) — asserts the lattice invariants, never clamps.
    pub fn from_config(config: &GridBackdrop, origin: Vec2) -> Self {
        let (nodes, springs) = match config.topology {
            GridTopology::Hex => build_hex_topology(config.cols, config.rows, config.spacing, origin),
            GridTopology::Square => build_square_topology(config.cols, config.rows, config.spacing, origin),
        };
        Self { config: config.clone(), origin, substeps: 4, /* scratch as today */ }
    }
}

// crates/engine_core/src/grid/build.rs
pub fn apply_grid_tunables(mesh: &mut GridMesh, config: &GridBackdrop) {
    debug_assert!(mesh.config.same_shape(config), "tunable apply must not change the lattice");
    mesh.config = config.clone();
}
pub fn build_grid_mesh(config: &GridBackdrop, origin: Vec2) -> GridMesh {
    let normalized = config.normalized();
    if normalized != *config { log::warn!(/* as today */); }
    GridMesh::from_config(&normalized, origin)
}
```

`step`, `update_activity`, `accumulate_forces`, `build_line_vertices` read `self.config.x`.
Delete every `with_*` builder, `set_alpha`, `new`, `new_square`, and `from_topology`'s private
default set (stiffness 24, damping 0.08, color 0.2/0.5/1.0/0.8, emissive 0.6). Grep confirmed
that set is dead in production: games call only `default_playfield_grid` and
`step_and_emit_grid` (`../games/{asteroids,breakout,frogger,snake,pong}/src`), and
`build_grid_mesh` overwrites all of it. The activity refs `spacing * 0.2` / `spacing * 2.0`
equal the preset's 6.0 / 60.0 at the preset spacing of 30, so nothing changes visually.

`backdrop_system.rs:120-135` already calls `normalized()`, `same_shape`, `apply_grid_tunables`,
`translate`, `build_grid_mesh` and needs no change.

### Half 2 — the wire variant becomes a newtype (spike first)

The 14-field `ComponentData::GridBackdrop { .. }` struct variant plus
`scene_data/grid_defaults.rs` cannot collapse while the variant is a struct variant: serde has
no variant-level `#[serde(default)]`. They collapse to nothing if the variant becomes
`GridBackdrop(ecs::GridBackdrop)` and `ecs::GridBackdrop` gets ONE container attribute,
`#[serde(default)]` (its `impl Default` is the preset already). Then `GridBackdrop()` and
`GridBackdrop(cols: 10)` in RON both mean "preset, overridden".

The catch: a newtype variant wrapping a struct serializes in RON as `GridBackdrop((cols: 44, ..))`
— old files with `GridBackdrop(cols: 44, ..)` would stop loading. RON's
`Extensions::UNWRAP_VARIANT_NEWTYPES` removes exactly that layer, reading both forms and
writing the unwrapped one. Enable it at the two choke points:

```rust
// scene_loader.rs:94 and scene_serializer.rs:333
fn scene_ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES)
}
scene_ron_options().from_str(content)
scene_ron_options().to_string_pretty(scene, PrettyConfig::default())
```

**Spike (gate for Half 2).** One test, before any schema change: run today's
`examples/assets/scenes/hello_world.scene.ron` text through the extension-enabled options and
assert (a) `physics: Some(PhysicsSettings(..))` and `editor: Some(EditorSettings(..))` still
parse — Option is serialized through `serialize_some`, not `serialize_newtype_variant`, so it
should be untouched, but this is the one thing to prove; (b) re-serializing yields byte-identical
`Behavior(PlayerPlatformer(..))` and `Scripts([..])` (their inner types are an enum and a Vec,
not structs, so unwrapping does not apply). If the spike passes, do Half 2: delete
`grid_defaults.rs` (44 lines), the 14-field variant, the 15-field loader arm and extractor
(both become `.clone()`), and the wire-defaults test becomes "`GridBackdrop()` parses to
`GridBackdrop::default()`". If the spike fails, the fallback is a `grid_default!(field)` macro
that turns the 44-line file into 14 lines; the audit's five places become three.

**Files touched.** `crates/ecs/src/grid_backdrop.rs` (container `#[serde(default)]`, Half 2),
`crates/engine_core/src/grid/{grid_mesh,build,mod,opacity_tests}.rs`,
`crates/engine_core/src/scene_data.rs`, `scene_data/grid_defaults.rs` (deleted, Half 2),
`scene_loader_components.rs`, `scene_serializer.rs`, `scene_loader.rs:94`,
`scene_serializer.rs:333`.

**Hazards.**
- `normalized()` already clamps `damping` and `rest_alpha_fraction` to 0..=1; the
  `with_rest_alpha_fraction` clamp becomes redundant. Keep the re-clamp inside
  `build_line_vertices` because `config` is a public field and the "color.w is the maximum"
  invariant is documented.
- The `hex_grid_rejects_odd_column_count` panic test asserts the constructor's assert message;
  keep the asserts in `from_config` with the same messages.
- `default_playfield_grid` test (`grid/mod.rs:93`) and
  `test_default_backdrop_matches_the_playfield_preset` (`build.rs:121`) read `built.stiffness`
  etc.; they become `built.config.stiffness`.

**Tests.** Existing grid tests rewrite their constructors to
`GridMesh::from_config(&GridBackdrop { cols: 6, rows: 5, spacing: 10.0, damping: 0.2, ..Default::default() }, Vec2::ZERO)`.
Add: "apply_grid_tunables keeps node positions" (already covered by
`backdrop_system` tests; verify they still pass), and the Half 2 spike test stays as a permanent
compatibility lock.

**Verdict: go (Half 1 unconditional, Half 2 after the spike).**

---

## C. `Behavior` ↔ `BehaviorData` mirror — GO, trivial

**Target.** Delete `crates/engine_core/src/behavior_data.rs` (240 lines) and both `From` impls.

```rust
// scene_data.rs
/// `ecs::Behavior` IS the wire schema for behaviors: it carries the serde defaults
/// old scene files rely on. Adding a variant or field there changes scene files.
pub type BehaviorData = ecs::behavior::Behavior;   // keeps every existing import path
Behavior(BehaviorData),                             // the variant is unchanged in name and shape
// scene_loader_components.rs
ComponentData::Behavior(behavior) => Self::add_component_logged(world, entity_id, behavior.clone()),
// scene_serializer.rs (or extract_behavior in design A)
components.push(ComponentData::Behavior(behavior.clone()));
```

The prelude and `lib.rs:95` re-exports keep working through the alias, so
`scene_serializer_tests.rs:294` and `tests/scene_loader_parse.rs:266,313` compile untouched.

**Old scene files, checked one by one.** `examples/assets/scenes/behavior_demo.scene.ron`
omits `tag` on `PlayerTopDown` and `collector_tag` on `Collectible`, and uses `ChaseTagged`,
`FollowEntity`, `Patrol`; `hello_world.scene.ron` uses the full `PlayerPlatformer`,
`Collectible`, and `CameraFollow` forms. Every omitted field has a default on `ecs::Behavior`
(`behavior.rs:151-165`), and `test_camera_follow_parses_legacy_four_field_form` already locks the
pre-look-ahead shape. `../games/*/assets/scenes/*.ron` contain no `Behavior(` at all.

**Files touched.** `behavior_data.rs` (deleted), `scene_data.rs`, `scene_loader_components.rs`,
`scene_serializer.rs`, `lib.rs`/`prelude.rs` (re-export the alias), `crates/ecs/src/behavior.rs`
(module doc: wire-frozen), `crates/engine_core/CLAUDE.md` (the "From impl pair" row in the
SSOT table becomes "ecs::Behavior's serde attributes").

**Hazard.** The single risk is a future serde attribute on `ecs::Behavior` silently changing the
schema. The guard is the test below.

**Tests.** Add to engine_core: parse every `examples/assets/scenes/*.ron` through
`SceneLoader::load_from_str`, asserting success. Add to ecs `behavior.rs`: a round-trip of
every `Behavior::default_for_variant(i)` through RON (the existing tests cover four variants).

**Verdict: go.**

---

## D. Achievements + Scores persistence — GO

**Target.** One merge-on-save slot, one error type, one clock helper, and the toast system in
its own file.

```rust
// crates/engine_core/src/save_store/json_slot.rs  (save_store.rs becomes save_store/mod.rs)
use serde::{de::DeserializeOwned, Serialize};

/// A persisted JSON document that can union itself with the copy already in its slot —
/// the multi-tab posture achievements and scores share (`docs/WEB_SAVES.md`).
pub trait MergeOnLoad: Serialize + DeserializeOwned + Default {
    /// Fold `existing` (the slot's current contents) into `self`.
    fn merge_from(&mut self, existing: Self);
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("IO error: {0}")] Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")] Serde(#[from] serde_json::Error),
}

pub struct JsonSaveSlot<T> { path: Option<PathBuf>, _document: PhantomData<T> }

impl<T: MergeOnLoad> JsonSaveSlot<T> {
    pub fn in_memory() -> Self;
    pub fn at(path: impl Into<PathBuf>) -> Self;
    pub fn path(&self) -> Option<&Path>;
    /// `Ok(None)`: no path, or the slot is absent. A corrupt slot is `Err` — callers
    /// warn and start fresh, replacing it on the next save.
    pub fn load(&self) -> Result<Option<T>, SaveError>;
    /// With `merge`, the slot's current contents are folded into `document` first
    /// (an unreadable or unparsable slot skips the merge). `Ok(false)` = no path.
    pub fn save(&self, mut document: T, merge: bool) -> Result<bool, SaveError> {
        let Some(path) = &self.path else { return Ok(false) };
        if merge { if let Ok(Some(existing)) = self.load() { document.merge_from(existing); } }
        save_store::write(path, &serde_json::to_string_pretty(&document)?)?;
        Ok(true)
    }
}

/// Seconds since the Unix epoch, 0 if the clock is before it.
pub fn unix_seconds() -> u64;
```

`achievements/mod.rs`: `SaveFile: MergeOnLoad` keeps the earliest `unlocked_at` per id (the
current merge rule verbatim). `scores.rs`: `ScoresFile: MergeOnLoad` does the contains-dedupe
plus `sort_and_truncate` per mode, which means `with_save_path`'s post-load sort falls out of
`let mut doc = ScoresFile::default(); doc.merge_from(disk)`. Both managers hold
`slot: JsonSaveSlot<SaveFile>` instead of `save_path`; `save()`, `reset()`, and the write-through
in `unlock`/`submit` become `self.slot.save(self.document(), merge)`.
`pub type AchievementError = SaveError; pub type ScoresError = SaveError;` keep the prelude and
`lib.rs:124-126` compiling; variant construction through an alias (`AchievementError::Io(..)`)
is legal.

Toast split, `crates/engine_core/src/achievements/toast.rs`: `Toast`, `ToastStyle`,
`DEFAULT_TOAST_DURATION`, `faded`, and

```rust
pub(crate) struct ToastQueue { toasts: Vec<Toast>, duration: f32, style: ToastStyle }
impl ToastQueue {
    pub fn push(&mut self, achievement_id: &str, name: &str, description: &str);
    pub fn tick(&mut self, delta_time: f32);
    pub fn draw(&self, ui: &mut UIContext, window_size: Vec2);
    pub fn len(&self) -> usize;
    pub fn clear(&mut self);
}
```

`AchievementManager.toasts: ToastQueue`; `set_toast_duration`, `set_toast_style`,
`toast_style`, `tick`, `draw_toasts` become one-line forwarders so `frame_tail.rs:60-61` and
the seven tests reading `mgr.toasts.len()` or calling `draw_toasts` stay as written.

**Files touched.** `save_store.rs` → `save_store/{mod,json_slot}.rs`, `achievements/mod.rs`,
`achievements/toast.rs` (new), `scores.rs`, `lib.rs`/`prelude.rs` (export `SaveError`,
`unix_seconds`), `input_settings_io.rs` (optional: its `InputSettingsError` is the same
`Io`+`Serde` pair and can alias `SaveError` too; the settings file does not merge, so it does
not adopt the slot).

**Hazards.**
- `AchievementManager::load()` errors when no path is configured; with the slot it becomes
  `self.slot.load()?.ok_or_else(not_found)` and stays an `Io` error, same as today.
- The two managers warn slightly differently on a corrupt slot at construction (achievements
  through `load`, scores through parse). Both collapse to one `warn!` at the `load()` call
  site with the same "starting fresh" message shape.
- `self.unlocks.clone()` / `self.modes.clone()` per save (audit 7.8) remain: the document must
  be owned for `merge_from`. Acceptable; saves are rare.

**Tests.** Existing `concurrent_*_merge_*` and `reset_clears_the_save_despite_merge_on_save`
tests in both suites are the contract and stay. Add one `JsonSaveSlot` unit test with a tiny
`MergeOnLoad` document (a `HashSet<u32>`) covering absent slot, merge union, `merge: false`
overwrite, and corrupt-slot-skips-merge — the protocol tested once. Rename the lone `mgr`
binding to `manager`.

**Verdict: go.**

---

## E. `GameContext` writeback channels — GO

**Decision on the fields.** `chaos_mode` and `time_scale` stay public fields: they are state
the game reads back every frame (`ChaosTheme::for_mode(ctx.chaos_mode)` in every menu) and the
pause pattern assigns `time_scale` every frame. The three fire-and-forget channels become an
explicit request API.

```rust
// crates/engine_core/src/contexts.rs
/// What a game asked the engine to do this frame. Drained after `update()` and after
/// every key handler; nothing here is readable back by the game.
#[derive(Debug, Default)]
pub struct FrameRequests {
    exit: bool,
    window_title: Option<String>,
    engine_ui_clip: Option<common::Rect>,
}

pub struct GameContext<'a> {
    /* every field as today, minus exit_requested / window_title / game_ui_clip */
    pub chaos_mode: ChaosMode,
    pub time_scale: f32,
    requests: FrameRequests,
}

impl GameContext<'_> {
    /// Quit at the end of the frame: the same clean shutdown as closing the window
    /// (`on_exit`, input-settings save, scene teardown).
    pub fn request_exit(&mut self) { self.requests.exit = true; }
    /// Retitle the OS window after this frame. One window-system round-trip, only when called.
    pub fn set_window_title(&mut self, title: impl Into<String>) { self.requests.window_title = Some(title.into()); }
    /// Whether a title was already requested this frame (the editor yields to a Playing game).
    pub fn window_title_requested(&self) -> bool { self.requests.window_title.is_some() }
    /// Clip the ENGINE's post-update UI draws (scene-authored elements, toasts) to `bounds`.
    /// Editor hosts only; plain games never call it.
    pub fn clip_engine_ui(&mut self, bounds: common::Rect) { self.requests.engine_ui_clip = Some(bounds); }
    /// Consume the context and hand back what the engine must absorb. Ends every borrow.
    pub(crate) fn into_outcome(self) -> FrameOutcome {
        FrameOutcome { chaos_mode: self.chaos_mode, time_scale: self.time_scale, requests: self.requests }
    }
}
pub(crate) struct FrameOutcome { pub chaos_mode: ChaosMode, pub time_scale: f32, pub requests: FrameRequests }

// crates/engine_core/src/game.rs
impl<G: Game> GameRunner<G> {
    /// One context builder for both the frame update and key handlers.
    fn build_context(&mut self, delta_time: f32, window_size: Vec2) -> Option<GameContext<'_>> {
        let assets = self.asset_manager.as_mut()?;
        Some(GameContext { input: &self.input, players: &mut self.player_input, world: &mut self.scene.world,
                           assets, /* ... */ requests: FrameRequests::default() })
    }
    /// The one writeback tail (replaces game.rs:490-498 and app_handler.rs:244-250).
    fn absorb(&mut self, outcome: FrameOutcome) {
        self.config.chaos_mode = outcome.chaos_mode;
        self.time_scale = outcome.time_scale;
        self.requests.exit |= outcome.requests.exit;                 // a request is never un-requested
        if let Some(title) = outcome.requests.window_title { self.requests.window_title = Some(title); }
        self.requests.engine_ui_clip = outcome.requests.engine_ui_clip;
    }
}
```

`GameRunner` keeps `requests: FrameRequests` in place of `exit_requested`,
`pending_window_title`, `pending_game_ui_clip`; `frame_tail.rs:44-70` and
`app_handler.rs:42` read `self.requests.*`. Call shape at both sites:

```rust
if let Some(mut ctx) = self.build_context(delta_time, window_size) {
    self.game.update(&mut ctx);
    let outcome = ctx.into_outcome();   // moves owned values out; the &mut self borrow ends here
    self.absorb(outcome);
}
```

Borrow check: a method returning `GameContext<'_>` borrows disjoint fields of `*self` for the
returned lifetime; `into_outcome` consumes the context and returns owned values, so `absorb`
can take `&mut self` immediately after (NLL). `first_frame`/`init` logic in
`initialize_and_update` wraps the same block.

**Migration in `../games` (every write site found by grep).** Thirteen files touch the five
names; only twelve sites write a channel: `ctx.exit_requested = true` in six `menu.rs` files
(`TitleItem::Exit` arm) and six `gameplay/mod.rs` files (`PauseAction::ExitGame` arm) become
`ctx.request_exit()`. `ctx.time_scale = self.pause.time_scale()` (six sites) and
`ctx.chaos_mode = self.settings.chaos` (`pong/src/menu.rs:159`) stay as they are.
`frogger/drawing.rs` and `pong/ui.rs` only use a local named `window_title`.

**Engine-side sites.** `editor_integration/src/editor_game/menu_actions.rs:114` →
`ctx.request_exit()`; `editor_game/mod.rs:446-449` → `if !ctx.window_title_requested() { if let Some(t) = self.pending_title_update() { ctx.set_window_title(t) } }`;
`editor_game/mod.rs:463` → `ctx.clip_engine_ui(bounds)`. Doc comments in `pause.rs:24,56,110`,
`menu_panel.rs`, `CLAUDE.md` ("exit_requested (write true → clean engine shutdown)") and
`training.md` (Pause Pattern snippet) update to the method names.

**Hazards.** The `|=` semantics of exit must survive in `absorb` (it does above). The
`window_title` "only latest wins within a frame" semantics are preserved. Nothing else reads
the three fields (grep over `crates`, `examples`, `src`).

**Tests.** `editor_game/time_freeze_tests.rs` and `play_guard_tests.rs` build contexts through
the editor's test harness; they compile unchanged if the harness uses the constructor path. Add
one engine_core test: `request_exit` on a context built by `build_context` makes `absorb` latch
`requests.exit`, and a second frame without the call keeps it latched.

**Verdict: go.**

---

## F. Editor command consolidation — GO

**Target.** One generic set command with type aliases, one add/remove pair over a component
reference. Merge isolation and `field_hint` semantics are preserved exactly.

```rust
// crates/editor/src/commands/set_commands.rs
use ecs::component_registry::ComponentMeta;

/// Whole-component write for one component type (inspector fields, gizmo drags).
/// Consecutive edits to the same entity AND the same `field_hint` merge into one undo
/// entry. Distinct `T`s are distinct types, so `downcast_ref::<Self>` keeps merge
/// isolation per component exactly as the thirteen macro-generated structs did.
pub struct SetComponentCommand<T: ecs::Component + ComponentMeta + Clone + Send> {
    entity: EntityId,
    old: T,
    new: T,
    field_hint: &'static str,
    display: String,           // "Set Transform2D" — built once from T::type_name()
}
impl<T: ecs::Component + ComponentMeta + Clone + Send> SetComponentCommand<T> {
    pub fn new(entity: EntityId, old: T, new: T, field_hint: &'static str) -> Self {
        Self { entity, old, new, field_hint, display: format!("Set {}", T::type_name()) }
    }
}
impl<T: ..> EditorCommand for SetComponentCommand<T> {
    fn execute(&mut self, world: &mut World) { if let Some(c) = world.get_mut::<T>(self.entity) { *c = self.new.clone(); } }
    fn undo(&mut self, world: &mut World)    { if let Some(c) = world.get_mut::<T>(self.entity) { *c = self.old.clone(); } }
    fn display_name(&self) -> &str { &self.display }
    fn try_merge(&mut self, other: &dyn EditorCommand) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(o) if o.entity == self.entity && o.field_hint == self.field_hint => { self.new = o.new.clone(); true }
            _ => false,
        }
    }
    fn as_any(&self) -> &dyn Any { self }  fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Gizmo drags are Transform2D sets under this hint: they merge with each other and
/// never with an inspector field edit ("position", "rotation", ...).
pub const GIZMO_FIELD_HINT: &str = "gizmo";

pub type SetTransformCommand    = SetComponentCommand<common::Transform2D>;
pub type SetSpriteCommand       = SetComponentCommand<Sprite>;
pub type SetRigidBodyCommand    = SetComponentCommand<RigidBody>;
pub type SetColliderCommand     = SetComponentCommand<Collider>;
pub type SetAudioSourceCommand  = SetComponentCommand<AudioSource>;
pub type SetBehaviorCommand     = SetComponentCommand<Behavior>;
pub type SetUiLabelCommand      = SetComponentCommand<UiLabel>;
pub type SetUiPanelCommand      = SetComponentCommand<UiPanel>;
pub type SetUiButtonCommand     = SetComponentCommand<UiButton>;
pub type SetEntityTagCommand    = SetComponentCommand<EntityTag>;
pub type SetScriptsCommand      = SetComponentCommand<ecs::script::Scripts>;
pub type SetGridBackdropCommand = SetComponentCommand<ecs::GridBackdrop>;
pub type SetNameCommand         = SetComponentCommand<Name>;
```

The macro, its thirteen expansions, and `TransformGizmoCommand` are deleted.
`SetComponentValueCommand` (StoredComponent-typed, never merges, "(API)" suffix) is the command
API's discrete-entry path and is NOT a duplicate; it stays.

**Registry macro.** The `=> SetCmd` half of a registry entry has nothing left to select once
the type is known, so the entry loses the token: `Sprite => Sprite : Rendering { edit edit_sprite }`,
and `registry_edit_block!` constructs `Box::new(SetComponentCommand::<$ty>::new(e, old, new, hint))`
itself. That also deletes the 14-name import block at `stored_component/mod.rs:21-26`. If the
team wants the macro to keep naming a command type, `$cmd:ty` works and the aliases satisfy it;
recommended form is dropping the token.

**Every call site (from grep).**
- Gizmo: `editor_game/viewport_interaction.rs:441`, `editor_game/tests.rs:518`,
  `commands/tests.rs:291,310,315` change `TransformGizmoCommand::new(e, a, b)` to
  `SetTransformCommand::new(e, a, b, GIZMO_FIELD_HINT)`.
- Unchanged through the aliases: `entity_ops.rs:185`, `viewport_interaction.rs:450`,
  `panel_renderer/tests.rs` (nine sites), `commands/{tests,dirty_tests,name_tests}.rs`,
  `editor_game/tests.rs:519`.
- `commands/mod.rs:24-28` re-exports the aliases plus `SetComponentCommand` and
  `GIZMO_FIELD_HINT`.

**Merge semantics checked against the real history code.** Production gizmo commits go through
`push_already_executed` (`viewport_interaction.rs:460`), which never merges, so a drag is one
entry as today. `test_transform_gizmo_merge` pushes two gizmo commands with
`try_merge_or_execute`; both carry `GIZMO_FIELD_HINT` on one entity, so they merge as before.
A gizmo entry and an inspector "position" entry never merge, exactly as two distinct types never
did. `NudgeCommand` stays its own type, so held-arrow nudges still cannot merge into a drag.

```rust
// crates/editor/src/stored_component/component_ref.rs  (NEW — mod.rs is at 585 lines)
/// A component addressed either through the typed registry overlay or by dynamic-tier name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentRef { Typed(ComponentKind), Dynamic(String) }
impl ComponentRef {
    pub fn display_name(&self) -> &str;
    pub(crate) fn add_default(&self, world: &mut World, entity: EntityId);   // Typed: kind.add_default; Dynamic: add_dynamic_default, log::error on Err
    pub(crate) fn capture(&self, world: &World, entity: EntityId) -> Option<StoredComponent>;
    pub(crate) fn remove(&self, world: &mut World, entity: EntityId);
    /// Removing a RigidBody takes its Collider with it (a collider without a body is
    /// meaningless to the physics system). Nothing else cascades; dynamic never does.
    pub(crate) fn cascade(&self) -> Option<ComponentRef> {
        matches!(self, Self::Typed(ComponentKind::RigidBody)).then_some(Self::Typed(ComponentKind::Collider))
    }
}

// crates/editor/src/commands/component_commands.rs — one pair over ComponentRef
pub struct AddComponentCommand { entity: EntityId, target: ComponentRef, display: String, captured: Option<StoredComponent> }
impl AddComponentCommand {
    pub fn new(entity: EntityId, kind: ComponentKind) -> Self { Self::for_ref(entity, ComponentRef::Typed(kind)) }
    pub fn dynamic(entity: EntityId, name: impl Into<String>) -> Self { Self::for_ref(entity, ComponentRef::Dynamic(name.into())) }
}
pub struct RemoveComponentCommand { entity, target: ComponentRef, display: String, stored: Option<StoredComponent>, cascade_stored: Option<StoredComponent> }
// execute: stored = target.capture; if let Some(c) = target.cascade() { cascade_stored = c.capture; c.remove }; target.remove
```

Dynamic call sites to change (five): `panel_renderer/inspector.rs:270`,
`command_api/write.rs:293,353`, `stored_component/mod.rs:338`,
`stored_component/dynamic_tests.rs:120,142` → `AddComponentCommand::dynamic(..)` /
`RemoveComponentCommand::dynamic(..)`. `AddDynamicComponentCommand` and
`RemoveDynamicComponentCommand` are deleted along with their `commands/mod.rs` re-exports.
Display names unify to "Add {name}" / "Remove {name}"; grep found no test asserting
"Add Component" or "Remove Component".

**Hazards.**
- `T: ComponentMeta` holds for all thirteen types: the global registry registers every one of
  them (`register::<T>()` requires it), including `common::Transform2D`, `Name`, and `Camera`.
- Two tests assert display strings and must change: `commands/tests.rs:117,122` expect
  "Set Transform" (becomes "Set Transform2D"). `command_api/write_tests.rs:314` expects
  "Set Transform2D (API)" from `SetComponentValueCommand`, unchanged.
- `EditorCommand: Send` — every `T` above is `Send` (registry components are `Send + Sync`).
- Do not make `display` a `&'static str` via leaking; the `String` matches what the dynamic
  commands already do.

**Tests.** The whole `commands/tests.rs`, `dirty_tests.rs`, `name_tests.rs`, and
`panel_renderer/tests.rs` suites are the contract; they run with the two string edits above.
Add: "SetComponentCommand<Sprite> never merges into SetComponentCommand<Transform2D> on the
same entity and hint" (locks the monomorphization argument), and "gizmo hint never merges
with a field hint on the same entity".

**Verdict: go.**

---

## G. Menu/shortcut dispatch unification — GO

Four pieces, all in service of ONE dispatch function.

### G1. `Archetype` enum (ARCH-101, three string vocabularies → one enum)

```rust
// crates/editor/src/archetype.rs (NEW)
/// The fixed set of entity factories the Entity menu and the command API's `create` share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Archetype { Empty, Sprite, Camera, StaticBody, DynamicBody, KinematicBody, UiLabel, UiPanel, UiButton }
impl Archetype {
    pub const ALL: [Archetype; 9] = [..];
    /// Command-API name ("static-body"). `const fn` so `ARCHETYPES` can stay a const.
    pub const fn kebab(self) -> &'static str;
    pub fn from_kebab(name: &str) -> Option<Self> { Self::ALL.into_iter().find(|a| a.kebab() == name) }
    /// Entity-menu label ("Create Static Body").
    pub const fn menu_label(self) -> &'static str;
}
```

- `command_api/parse.rs:11`: `pub const ARCHETYPES: [&str; 9] = [Archetype::Empty.kebab(), ..]`
  (kept as a public const because `specs.rs` and the docs list it); validation at `:173` becomes
  `Archetype::from_kebab(&archetype).ok_or(..)`; `HostedWrite::Create { archetype: Archetype, .. }`.
- `editor_integration/src/editor_game/api.rs:228 archetype_action` is deleted.
- `entity_ops::handle_create_action(&str, ..)` becomes `create_archetype(Archetype, ..)` with a
  typed match; `menu_actions.rs:43-62` dispatches `EditorAction::CreateEntity(a)`.
- `menu/mod.rs:229-241`: the nine `Create ...` items are generated from
  `Archetype::ALL.iter().map(|a| MenuItem::action(a.menu_label()))` with the two separators
  kept by grouping (`Empty` | `Sprite, Camera` | bodies | UI), so the label list has one source.

### G2. `EditorAction` gains the menu-only actions

`CreateEntity(Archetype)`, `Exit`, `TogglePanel(PanelId)`, `ResetLayout`, `CycleGameLocale`.
`EditorAction` is `Copy + Eq + Hash` (a `HashMap` key in `EditorInputMapping`); `Archetype`
derives the same, and `PanelId` must too (verify `dock/mod.rs`; it is used as a map key already,
so it is at least `Eq + Hash`). Add:

```rust
impl EditorAction {
    /// Actions the menu allows during a play session (today's `if !is_playing()` guards, inverted).
    pub fn allowed_while_playing(self) -> bool {
        matches!(self, Save | SaveAs | Exit | ToggleGrid | ToggleColliders | ToggleSnap
                     | TogglePanel(_) | ResetLayout | CycleGameLocale
                     | PlayResume | TogglePlayPause | StopPlay | ToggleCameraFollow)
    }
}
```

### G3. One label table and one dispatcher

```rust
// crates/editor/src/menu/actions.rs (NEW, beside `panel_id_for_menu_label`)
/// Menu label → editor action. The menu bar speaks labels; everything after it speaks actions.
pub fn action_for_menu_label(label: &str) -> Option<EditorAction> {
    if let Some(panel) = panel_id_for_menu_label(label) { return Some(EditorAction::TogglePanel(panel)); }
    if let Some(a) = Archetype::ALL.into_iter().find(|a| a.menu_label() == label) { return Some(EditorAction::CreateEntity(a)); }
    Some(match label {
        "New Scene" => NewScene, "Open Scene..." => OpenScene, "Save" => Save, "Save As..." => SaveAs, "Exit" => Exit,
        "Undo" => Undo, "Redo" => Redo, "Cut" => Cut, "Copy" => Copy, "Paste" => Paste, "Delete" => Delete, "Duplicate" => Duplicate,
        "Toggle Grid" => ToggleGrid, "Toggle Colliders" => ToggleColliders, "Snap to Grid" => ToggleSnap,
        "Cycle Game Locale" => CycleGameLocale, "Reset Layout" => ResetLayout,
        _ => return None,
    })
}

// editor_integration/src/editor_game/menu_actions.rs — handle_menu_bar shrinks to:
let Some(label) = self.editor.menu_bar.render(ctx.ui, window_size.x, &self.editor.theme) else { return };
match editor::menu::action_for_menu_label(&label) {
    Some(action) if !self.editor.is_playing() || action.allowed_while_playing() =>
        self.dispatch_editor_action(action, false, ctx),
    Some(_) => {}                                  // refused while Playing, as today
    None => log::info!("Unhandled menu action: {label}"),
}
```

`dispatch_editor_action` (`shortcuts.rs:260`) becomes `pub(super)` and gains the five new arms
(bodies move out of `menu_actions.rs:88-133` verbatim). The one behavioral divergence resolves
toward the menu: the `Undo`/`Redo` arms show the "Undo: {name}" status message, so Ctrl+Z now
reports the same way the menu does. The `drag_guard` stays in the dispatcher and therefore now
also protects menu-driven Undo/Delete/Paste/Cut during a live gizmo drag (a small correctness
gain; the menu path had no guard).

### G4. Two tiny helpers

```rust
// crates/editor/src/commands/mod.rs
impl CommandHistory {
    /// Record already-applied commands as ONE undo entry: none = nothing, one = itself,
    /// many = a `MacroCommand` named `name`.
    pub fn push_as_one(&mut self, name: &str, mut commands: Vec<Box<dyn EditorCommand>>) {
        match commands.len() {
            0 => {}
            1 => if let Some(cmd) = commands.pop() { self.push_already_executed(cmd) },
            _ => self.push_already_executed(Box::new(MacroCommand::new(name, commands))),
        }
    }
    /// `execute` counterpart for commands not yet applied (Delete uses this).
    pub fn execute_as_one(&mut self, name: &str, commands: Vec<Box<dyn EditorCommand>>, world: &mut World);
}
// crates/editor/src/editor_input.rs
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers { pub ctrl: bool, pub shift: bool }
impl Modifiers {
    /// Either Ctrl / either Shift held — the editor's modifier model (Alt/Super are the OS's).
    pub fn read(input: &InputHandler) -> Self;
}
```

Replaces the four copies at `viewport_interaction.rs:460`, `menu_actions.rs:156,238,273` and
the five modifier spellings at `shortcuts.rs:206`, `editor_input.rs:341`,
`viewport_interaction.rs:108,526`, `panel_renderer/mod.rs:179`; `ctrl_held` is deleted.

**Files touched.** `crates/editor/src/{archetype.rs (new), editor_input.rs, menu/mod.rs,
menu/actions.rs (new), commands/mod.rs, command_api/{parse,mod,write}.rs, lib.rs}`,
`crates/editor_integration/src/{entity_ops.rs, editor_game/{menu_actions,shortcuts,api,viewport_interaction}.rs, panel_renderer/mod.rs}`.

**Hazards.**
- `EditorInputMapping::set_default_bindings` must NOT bind the new menu-only actions (no
  chords). `resolve` returning them is impossible; `dispatch` handles them anyway.
- `TogglePanel`/`CreateEntity` carry data, so the `use EditorAction as A; match action { A::.. }`
  arms need patterns, not unit paths; the "poll-only consumed" arm list stays exhaustive.
- The `api.rs` drift test that asserted every kebab maps to a label is replaced by the
  round-trip test below.

**Tests.** Keep `test_every_default_chord_resolves_to_its_action`. Add: `from_kebab(kebab(a)) == Some(a)`
over `Archetype::ALL`; walk `MenuBar::editor_default()` and assert every non-separator, enabled
action item maps through `action_for_menu_label` (the drift lock the audit asked for);
`push_as_one` for 0/1/many; a shortcuts test that Ctrl+Z now shows the status message.

**Verdict: go.**

---

## H. Renderer — GO

### H1. Shared camera binding (issue #89, DRY-006)

```rust
// crates/renderer/src/camera_binding.rs (NEW)
/// One camera uniform: buffer + bind-group layout + bind group, uploaded with `update`.
pub struct CameraBinding { buffer: Buffer, layout: BindGroupLayout, bind_group: BindGroup }
impl CameraBinding {
    pub fn new(device: &Device, label: &str) -> Self;      // the 30 lines both pipelines had
    pub fn layout(&self) -> &BindGroupLayout;
    pub fn update(&self, queue: &Queue, camera: &Camera);  // CameraUniform::from_camera + write_buffer
    pub fn bind(&self, pass: &mut wgpu::RenderPass<'_>, index: u32);
}
```

Compose, do not share yet: `SpritePipeline` and `LinePipeline` each own one `CameraBinding`;
their `update_camera` bodies become `self.camera.update(queue, camera)`, and their draw paths
call `self.camera.bind(pass, 0)`. Sharing ONE binding across all three pipelines is also
correct (all three read the same camera per frame, so the write-buffer footgun does not apply)
and would save two uploads, but it needs `SpritePipeline::new` to take the layout across the
crate boundary from `engine_core/src/render_manager.rs:144-147`. Defer that as a follow-up;
composition closes #89 now with zero semantic change.

### H2. Widened pipeline-descriptor helper

```rust
// crates/renderer/src/pipeline_builder.rs (NEW)
pub(crate) struct PipelineSpec<'a> {
    pub label: &'a str, pub layout: &'a PipelineLayout, pub shader: &'a ShaderModule,
    pub vertex_entry: &'a str, pub fragment_entry: &'a str,
    pub buffers: &'a [VertexBufferLayout<'a>], pub topology: PrimitiveTopology,
    pub target: ColorTargetState, pub depth: Option<DepthStencilState>,
}
pub(crate) fn build_render_pipeline(device: &Device, spec: &PipelineSpec<'_>) -> RenderPipeline;
/// Depth test against the shared buffer; sprites write, lines only test.
pub(crate) fn depth_state(write: bool) -> DepthStencilState;
```

`bloom.rs:494 build_fullscreen_pipeline` becomes a five-line call (`buffers: &[]`, `blend: None`,
`depth: None`). `sprite/pipeline.rs:188-238` and `line_pipeline.rs:129-172` route through it.

### H3. `SpritePipeline::new_with_target` split

`fn texture_layout(device) -> BindGroupLayout`, `fn quad_buffers(device) -> (Buffer, Buffer)`,
`CameraBinding::new`, `build_render_pipeline`; `new_with_target` becomes ~25 lines and the ten
narration comments go. Delete the zero-caller public methods `update_instance_buffer`,
`invalidate_texture_cache`, `clear_texture_cache`, `pipeline`, `camera_bind_group_layout`,
`texture_bind_group_layout` (grep: none called outside their own file).

### H4. `SpriteShape` stored as the enum

```rust
pub struct Sprite { /* .. */ pub shape: SpriteShape, pub corner_radius: f32, pub border_width: f32, /* .. */ }
impl Sprite {
    pub fn with_corner_radius(mut self, radius: f32) -> Self { self.shape = SpriteShape::RoundedRect; self.corner_radius = radius.max(0.0); self }
    pub fn as_circle(mut self) -> Self { self.shape = SpriteShape::Circle; self }
    pub fn with_border(mut self, width: f32) -> Self { if self.shape == SpriteShape::Quad { self.shape = SpriteShape::RoundedRect; } self.border_width = width.max(0.0); self }
    pub fn to_instance(&self) -> SpriteInstance { .. .with_shape([self.shape.to_f32(), self.corner_radius, self.border_width, 0.0]) }
}
```

`SpriteInstance.shape` stays `[f32; 4]` (GPU layout), so the `ui_integration/tests.rs`
assertions on instances stay. The only other `renderer::Sprite` user outside engine_core is the
dead `editor/src/viewport/mod.rs:288`.

### H5. Scissor enum

```rust
// crates/renderer/src/scissor.rs
/// A pass-level scissor decision: `Fullscreen` = no scissor call, `Rect` = set it,
/// `Empty` = nothing visible (clear-only passes still run, draws are skipped).
pub enum PassScissor { Fullscreen, Rect([u32; 4]), Empty }
impl PassScissor {
    pub fn resolve(request: Option<[u32; 4]>, surface: (u32, u32)) -> Self;  // None → Fullscreen; Some → clamp_scissor → Rect/Empty
}
```

Two consumers justify it: `bloom.rs:290` (`Option<Option<..>>`) and `line_pipeline.rs:220-227`
(the same three-way decision as a nested match). `SwapchainTarget.composite_scissor` can carry a
`PassScissor` resolved once in `renderer.rs`.

### H6. `wgpu::vertex_attr_array!`

Verified against the WGSL:

```rust
// sprite_data.rs
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
    3 => Float32x2, 4 => Float32, 5 => Float32x2, 6 => Float32x4,
    7 => Float32x4, 8 => Float32, 9 => Float32, 10 => Float32x4];
// line_pipeline.rs
const LINE_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32];
```

The macro's sequential offsets for the instance are 0, 8, 12, 20, 36, 52, 56, 60 — identical
to the hand-counted `size_of::<[f32; N]>()` values (N = 2, 3, 5, 9, 13, 14, 15); line vertex
offsets 0, 8, 24. Shader locations `sprite_instanced.wgsl:23-37` (0..2 vertex, 3..10 instance)
and `line.wgsl:13-15` (0..2) match exactly.

**Files touched.** `crates/renderer/src/{camera_binding.rs (new), pipeline_builder.rs (new),
sprite/pipeline.rs, line_pipeline.rs, bloom.rs, sprite.rs, sprite_data.rs, scissor.rs,
renderer.rs, lib.rs}`, `crates/renderer/CLAUDE.md` (close #89 in the debt note),
`crates/engine_core/src/render_manager.rs` (only if H1's sharing follow-up lands; not now).

**Hazards.** `RenderPipelineDescriptor` fields in wgpu 28 (`multiview_mask`, `cache`) are
already spelled identically in all three copies, so the helper is a pure move. The `#[allow(clippy::arc_with_non_send_sync)]`
at `renderer.rs:230` is unrelated and stays.

**Tests.** Existing layout-size tests (`test_sprite_instance_desc_attributes`,
`line_vertex_layout_size`) stay. Add: for every field, `desc().attributes[i].offset ==
std::mem::offset_of!(SpriteInstance, field)` (and the same for `SpriteVertex`, `LineVertex`),
which is the guard the hand-counting never had; `PassScissor::resolve` for the three outcomes;
`Sprite::with_border` on a plain quad promotes to `RoundedRect` with radius 0.

**Verdict: go.**

---

## I. Input — GO

### I1. `PlayerBindings` holds an `InputMapping`

```rust
// crates/input/src/input_mapping.rs — generic over the source, defaulted so every
// existing `InputMapping<A>` (behavior_runner, games' private enums) compiles unchanged
pub struct InputMapping<A: Copy + Eq + Hash, S: Copy + Eq + Hash = InputSource> {
    bindings: HashMap<A, Vec<S>>,
}
impl<A: Copy + Eq + Hash, S: Copy + Eq + Hash> InputMapping<A, S> {
    pub fn new() -> Self;
    /// Returns true when the pair was actually added (feeds PlayerBindings' dirty flag).
    pub fn bind(&mut self, action: A, source: S) -> bool;
    /// Returns true when a source was actually removed.
    pub fn unbind(&mut self, action: A, source: &S) -> bool;
    pub fn unbind_action(&mut self, action: A); pub fn unbind_source(&mut self, source: &S);
    pub fn bindings(&self, action: A) -> &[S];
    pub fn actions_for(&self, source: &S) -> Vec<A>;
    pub fn has_binding(&self, action: A) -> bool; pub fn is_empty(&self) -> bool; pub fn clear(&mut self);
    pub fn iter(&self) -> impl Iterator<Item = (A, &[S])>;
}
impl<A: Copy + Eq + Hash> InputMapping<A, InputSource> { /* is_active / just_activated / just_deactivated / was_active unchanged */ }

// crates/input/src/player.rs
pub struct PlayerBindings { pad: Option<u32>, mapping: InputMapping<GameAction, PlayerSource>, dirty: bool }
impl PlayerBindings {
    pub fn bind(&mut self, action: GameAction, source: PlayerSource) { self.dirty |= self.mapping.bind(action, source); }
    pub fn unbind(&mut self, action: GameAction, source: &PlayerSource) { self.dirty |= self.mapping.unbind(action, source); }
    pub fn bindings(&self, action: GameAction) -> &[PlayerSource] { self.mapping.bindings(action) }
    pub fn all_bindings(&self) -> impl Iterator<Item = (GameAction, &[PlayerSource])> { self.mapping.iter() }
}
```

`bind` returning `bool` is a signature change on a public method; the two existing external
callers (`input_settings_io.rs:83`, tests) ignore the return value, and `#[must_use]` is not
added, so nothing breaks.

### I2. One standard pad layout

```rust
// crates/input/src/player.rs
/// The standard pad layout, device-relative: dpad + left stick → movement, A/B/X/Y →
/// Action1-4, Start → Menu, Select → Select. `InputMapping::with_default_bindings`
/// maps the same table onto pad 0.
pub const STANDARD_PAD_LAYOUT: &[(GameAction, PlayerSource)] = &[
    (GameAction::MoveUp,    PlayerSource::PadButton(GamepadButton::DPadUp)),
    /* ... the 14 pairs bind_standard_pad_layout has today ... */
];
impl PlayerSource {
    /// Concrete source on `pad`; keyboard/mouse pass through.
    pub fn on_pad(self, pad: u32) -> InputSource;   // PlayerBindings::resolve becomes `self.pad.map(|id| source.on_pad(id))` for pad variants
}
// input_mapping.rs with_default_bindings: keyboard/mouse half hand-written as today, then
for (action, source) in STANDARD_PAD_LAYOUT { mapping.bind(*action, source.on_pad(0)); }
```

The two pad halves are the same fourteen pairs (checked line by line at
`input_mapping.rs:249-291` and `player.rs:396-423`); only the keyboard halves differ and stay.

**On-disk format: unchanged.** `input_settings_io.rs` uses only `all_bindings()`, `bind`,
`set_pad`, `pad()`, and `PlayerSource`'s serde form is untouched.
`round_trip_preserves_pads_and_bindings` is the guard, and `missing_file_returns_defaults_and_creates_hand_editable_file`
proves the written defaults are byte-compatible.

### I3. Previous-frame snapshot in `ButtonTracker` (the audit's highest correctness item)

```rust
pub struct ButtonTracker<T: Copy + Eq + Hash> {
    pressed: HashSet<T>,
    /// `pressed` as of the end of the previous frame — the double buffer gamepad axes
    /// already keep, so edges are observed, never reconstructed.
    previous: HashSet<T>,
    /// Chronological within the frame (typed characters keep their order).
    just_pressed: Vec<T>,
    just_released: HashSet<T>,
}
impl<T: Copy + Eq + Hash> ButtonTracker<T> {
    pub fn press(&mut self, button: T) { if self.pressed.insert(button) { self.just_pressed.push(button); } }
    /// A release of a button that was never pressed (focus loss, synthetic event) is not an edge.
    pub fn release(&mut self, button: T) { if self.pressed.remove(&button) { self.just_released.insert(button); } }
    pub fn is_just_pressed(&self, button: T) -> bool { self.just_pressed.contains(&button) }   // 0–3 elements; faster than hashing
    pub fn was_pressed(&self, button: T) -> bool { self.previous.contains(&button) }
    pub fn just_pressed_buttons(&self) -> &[T] { &self.just_pressed }
    pub fn clear_frame_state(&mut self) { self.previous.clone_from(&self.pressed); self.just_pressed.clear(); self.just_released.clear(); }
}
```

`KeyboardState::was_key_pressed` / `just_pressed_keys`, `MouseState::was_button_pressed`,
`GamepadState::was_button_pressed` / `axis_was_active(axis, dir, threshold)` (reads the
existing `prev_axis_values`), and `InputHandler::was_source_pressed(&InputSource)` dispatching
over them. `source_was_pressed` (`input_mapping.rs:226`) becomes a one-line call to it.

**Behavior change to document.** A press and release inside one frame now reports
`just_activated` (it was swallowed) and no longer reports `just_deactivated` (it was reported).
Every game's `just_activated` gains the fix.

**Files touched.** `crates/input/src/{button_tracker,keyboard,mouse,gamepad,input_handler,input_mapping,player}.rs`,
`crates/input/CLAUDE.md`, `crates/engine_core/src/behavior_runner/mod.rs` (only if it names
`InputMapping<GameAction>` with explicit generics — the default parameter keeps it compiling
either way).

**Hazards.**
- `clear_frame_state` must run exactly once per frame; it does (`InputHandler::end_frame`), and
  the extra gamepad callers are test-only.
- Gamepad disconnect drops the state entirely, so `was_pressed` reads false afterward and the
  documented "disconnect = no just-released edge" holds.
- Three small `HashSet` clones per frame; negligible.
- Any test that releases a never-pressed button and expects `is_just_released` must be found by
  running the suite; none was spotted by grep.

**Tests.** Add: "press and release within one frame fires just_activated once and no
just_deactivated" on both `InputMapping` and `InputSettings`; "release without press reports no
edge" on `ButtonTracker`; "just_pressed_buttons preserves press order". Existing
`input_handler_integration.rs:203-282` and every `player.rs` test stay.

**Verdict: go.**

---

## J. UI — GO

### J1. Shared `edit_field` core

```rust
// crates/ui/src/context/text_input.rs
enum EditFieldEvent {
    Idle { hovered: bool },
    Editing { text: String, invalid: bool },
    Committed(String),
    Cancelled,
}
impl UIContext {
    /// The editing shell every text widget shares: click-to-focus (seeded, select-all),
    /// click-to-place-cursor, Escape cancel, Enter/Tab/click-away commit, key edits.
    /// Draws the unfocused box, the editing box, and the cancelled box itself; the
    /// caller draws the committed text (it knows the formatted value) and calls
    /// `note_edit_commit`.
    fn edit_field(
        &mut self, id: WidgetId, bounds: Rect, font: Option<FontHandle>,
        display_text: &str,
        seed_on_focus: impl FnOnce() -> String,
        is_valid: impl Fn(&str) -> bool,
    ) -> EditFieldEvent;

    /// The face a field draws and measures in: the requested handle when it still
    /// resolves, else the default font. Shared by float_input and text_input, which
    /// closes the audit's font-fallback drift.
    fn resolve_font(&self, requested: Option<FontHandle>) -> Option<FontHandle>;
}
```

`text_input` becomes: `match self.edit_field(id, bounds, self.resolve_font(None), value, || value.to_string(), |_| true)`
→ `Committed(t)` draws the box, `note_edit_commit`, `Some(t)`; everything else `None`.
`float_input` keeps its scrub pre-pass (`float_scrub`) and its Up/Down nudge; the nudge runs
before `edit_field` only when `self.interaction.is_focused(id)` and no Escape/Enter/Tab/click-away
is pending this frame (preserving today's precedence), then `edit_field` with
`|| format!("{:.2}", value)` and `|t| t.parse::<f32>().is_ok()`; `Committed(t)` routes into
`commit_float_input` (parse, hard clamp, out-of-range flag).

### J2. Single text-height function

```rust
// crates/ui/src/font/layout.rs
/// The line height a font reports at `font_size` (`new_line_size`), with the one
/// shared fallback when the font has no metrics. Both layout and measurement use it.
pub(super) fn text_height(font: &Font, font_size: f32) -> f32 {
    font.horizontal_line_metrics(font_size).map(|m| m.new_line_size).unwrap_or(font_size * 1.2)
}
```

`layout_text` sets `height: text_height(font, font_size)` and drops the `max_descent`
accumulation; `measure_text` calls the same function. Decision: font-metric height,
glyph-independent, because `text_pos_in_bounds_with_font` (`context/text.rs:75`) centers on the
MEASURED height and the drawn layout must agree with it. `TextDrawData.height` is read at
`ui_integration/mod.rs:99,173,178` as a bounds size; a descender-tall glyph no longer inflates
it, which is the correct bound for centering.

### J3. One typed-key list — by deleting the list

Delete the 18-line `typed_keys` array (`input_state.rs:164-181`). With design I3's chronological
`just_pressed`, `KeyboardState::just_pressed_keys() -> &[KeyCode]` drives:

```rust
let typed_chars: Vec<char> = kb.just_pressed_keys().iter().filter_map(|&key| keycode_to_char(key, shift)).collect();
```

One list (the `keycode_to_char` match), chronological order for two keys in one frame (today's
array order is neither), and O(pressed) per frame instead of a 51-key scan. If J ships before I,
the interim is `pub(crate) const TYPED_KEYS: &[KeyCode]` plus a test that every listed key maps.

**Files touched.** `crates/ui/src/context/text_input.rs`, `crates/ui/src/font/layout.rs`,
`crates/ui/src/input_state.rs`, `crates/input/src/keyboard.rs` (accessor), `crates/ui/CLAUDE.md`.

**Hazards.**
- `self.interaction.input().clone()` and `self.theme.text_input.clone()` per widget per frame
  (audit 7) stay in this pass; deriving `Copy` on `TextInputStyle` is a separate one-liner
  worth doing while here if `Color` is `Copy` (it is).
- `draw_text_input_editing_invalid` and `draw_text_input_box` keep their signatures; the shell
  calls them.
- The `test_text_layout` test at `layout.rs:133` is a constructor echo and can go.

**Tests.** Existing `text_input` tests (`context/tests.rs`, `scrub_tests.rs`, `focus_tests.rs`)
are the contract and run unchanged. Add: "text_input and float_input resolve a stale font
handle to the default font identically" (locks the drift fix); "measured height equals laid-out
height for a string with descenders"; "two keys pressed in one frame type in press order".

**Verdict: go.**

---

## K. `GameRunner` (33 fields) and `EditorGame` (19 fields) — PARTIAL: two groups now, the rest filed

**GameRunner.** Design E already removes `pending_window_title`, `pending_game_ui_clip`, and
`exit_requested` into `requests: FrameRequests`, and deletes the second 18-field literal via
`build_context`. Do one more group now, because all three fields are touched only by
`game/frame_tail.rs` and `game/locale_font.rs`:

```rust
// crates/engine_core/src/game/locale_font.rs
pub(super) struct Localization {
    pub strings: crate::localization::Strings,
    /// The game's own font, captured after `init()`, restored when a locale has none.
    pub base_font: Option<ui::FontHandle>,
    /// Locale font path → handle, so cycling locales never reloads a file.
    pub fonts_by_path: HashMap<String, ui::FontHandle>,
}
```

`ctx.strings` becomes `&mut self.localization.strings` inside `build_context`; nested disjoint
field borrows are fine. Net: 33 fields → 28, and the two worst duplications gone.
`render_fatal` is a renderer latch, not a game request, so it stays a plain field. Do not group
`lines`, the two batchers, `pending_ui_events`, or the input trio; they are not cohesive and
moving them buys no coupling reduction.

**EditorGame.** The only cohesive pairs are `ApiSession { rx, batch }` and
`SceneConfirm { pending_action, pending_choice }`. `api_batch` is `pub(super)` and read in
`shortcuts.rs` (four places) and `api.rs`; the confirm pair lives in `scene_confirm.rs` plus
`shortcuts.rs:39` and `mod.rs`. Moving four fields into two structs touches roughly fifteen sites
for no borrow or coupling win. Defer and file as an issue, together with turning `update()`'s
comment-numbered phases (0, 0b, 0c, 0d, 1, 1b, 2, 2b, 2c, 3, 4, 4b, 5, 6, 7, 9, 9b, 10, 11, 12
— no phase 8) into named private methods, which is the same file and the same reviewer pass.

**Verdict: partial — E + Localization now; EditorGame split and phase naming filed.**

---

## Verdict summary

| design | verdict | the one risk to watch |
|---|---|---|
| A scene schema | go | wire vs registry name split (Camera2D / Camera) |
| B grid tuning | go; Half 2 gated on the RON `UNWRAP_VARIANT_NEWTYPES` spike | `Option` round-trip under the extension |
| C Behavior mirror | go, trivial | `ecs::Behavior` becomes wire-frozen |
| D persistence | go | none found |
| E context writebacks | go | twelve one-line game edits, docs |
| F set commands | go | two display-string test asserts |
| G dispatch | go | shortcut Undo/Redo gain a status message |
| H renderer | go | none; offsets verified against WGSL |
| I input | go | same-frame press+release edge semantics change |
| J UI | go | J3 depends on I3's tracker |
| K runner splits | partial: E + Localization now, EditorGame deferred | file the EditorGame split + phase naming |

**Files that must be NEW because their natural homes are at the size ceiling or do not exist:**
`crates/editor/src/stored_component/component_ref.rs`, `crates/editor/src/archetype.rs`,
`crates/editor/src/menu/actions.rs`, `crates/renderer/src/camera_binding.rs`,
`crates/renderer/src/pipeline_builder.rs`, `crates/engine_core/src/save_store/json_slot.rs`,
`crates/engine_core/src/achievements/toast.rs`.

**Cross-design dependencies the sequencer should know:** J3 wants I3; K's GameRunner half is
E plus one extra struct; A's serializer table is where C's and B-Half-2's `.clone()` arms land;
G4's `Modifiers` and `push_as_one` are independent of G1–G3 and can ship first.
