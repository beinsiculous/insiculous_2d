# Code-quality cleanup — insiculous_2d

## Context

The engine is ~72k lines across 11 crates with 1,657 `#[test]` functions (1,703 with doc tests), all green and clippy-clean, grown
through issue-driven sprints. Jesse wants a codebase that reads without documentation:
DRY/KISS/SRP held constantly, human-readable names everywhere, `//` comments reserved for
pitfalls and failed approaches (`///` doc comments stay, stating the contract in a few lines),
tests that only fail when a player or author would notice, the Game Programming Patterns
catalog followed without its anti-patterns, and no test counts in docs ("a test run tells us
if it's true").

Four read-only audits (core, editor, systems, tests) and two design passes were run on commit
`d58f3c3`. Every claim the plan acts on was verified against the tree; three audit claims were
corrected (the input "swallowed press" is a low-severity phantom `just_deactivated`; the
cache-skipping texture loader has zero callers and is a deletion; the games never call the
`GridMesh` builders, so the grid fold does not ripple). Full reports live in `coordination/cleanup-2026-09/` (batch 0 moves them there from the
session scratchpad): `audit-{core,systems,editor,tests}.md`, `plan-sequence.md`,
`design-structure.md` — the implementer reads `design-structure.md` for the exact target
shapes of batches 4–9; this file is the executable summary.

Decisions taken with Jesse (Sep 2 2026): all ten batches in order; delete every dead public
API including the audio quartet and the physics event-bus mirror; convert the three
fire-and-forget `GameContext` fields to methods (12 one-line game edits); apply the test
rubric fully and remove test counts from every doc.

What is in good shape and must be preserved: zero `unwrap`/`expect` in production code; the
fn-pointer `ComponentRegistry`; `editor_component_registry!` (judged justified — it generates
an enum and nine dispatch methods from one list); Command/Observer/State/Dirty-Flag/Flyweight/
Prototype/Object-Pool implementations; the settled decisions (`ComponentStore` HashMap,
cfg-split frame drivers, rodio).

## Ground rules for every batch

- Gates: `cargo test --workspace` (0 failed, 0 ignored), `cargo clippy --workspace
  --all-targets` (0 warnings), every touched file ≤ 600 lines, no new `#[allow]`, no
  `unwrap()` outside tests. `/finish-task` is the checklist.
- Games gate whenever a public item of `engine_core`, `ecs`, `physics`, `input`, `common` or
  `renderer` changes: `scripts/check_games.sh` (check + clippy; or `scripts/check_games.sh --test` for batches
  that change behaviour game tests exercise, e.g. 1, 4, 5).
- Wasm gate `scripts/check_wasm.sh` whenever `engine_core`, `renderer`, `audio`, `input`,
  `common` change.
- Review: the commit hook (`scripts/commit-review-hook.sh`, threshold 100 changed lines)
  denies unreviewed big commits. Every batch except 10 exceeds it: `git diff > review/draft.diff`,
  `scripts/request-review.sh code review/draft.diff --reviewer=kimi`, adjudicate every finding
  with Jesse, then commit prefixed `ADV_REVIEWED=1`. Never write skip trailers. Before any code,
  this plan itself gets the kimi plan review (the ExitPlanMode hook).
- Branch and landing (Jesse, Sep 2): batch 0 creates `jesse` from `dev` (`git switch -c jesse
  dev`; no `jesse` exists locally or on origin — if one appears first, merge `dev` into it). All
  batches commit on `jesse`. `jesse` merges into `dev` at the end, once everything is green.
  No per-batch re-sync.
- Every batch commit is pathspec-scoped (`-- <paths>`), so foreign edits in the tree fail the
  commit instead of riding along under `ADV_REVIEWED=1`.
- Reviewer failure path: if `request-review.sh` errors or truncates, retry once; then split the
  diff per crate and review (and commit) the parts; if it still fails, Jesse decides explicitly.
- Tag-gate exceptions: a match that must survive is whitelisted by Jesse's adjudication,
  recorded in the batch's rebuttal, with the gate pattern updated.
- Docs match reality at every commit: each batch carries the doc-line edits its deletions
  force; the guide restructure is batch 10.
- Every batch is sequential (review round 1, F5: parallel batches would race on
  `review/draft.diff` and full-tree commits).
- `scripts/check_wasm.sh` is a compile-time gate; wasm runtime behaviour is guaranteed by
  construction through the `common::clock` and `common::vfs` seams, never by the gate.
- The comment-tag gate (batch 3) is part of EVERY batch's gate list.
- Per crate, the keep-list test cut is its own commit and lands BEFORE any behaviour change
  in that crate (review round 1, F4); a deleted-by-list test that would have failed under the
  batch's change is reinstated and adjudicated with Jesse before the change commits.
- Anything deferred is filed with `/file-issue` before the effort reports done.

## Batch 1 — correctness fixes, each with a regression test — DONE Sep 3 2026 (2188b9a)

Authored by gemini, reviewed by kimi and Claude (`review/cleanup-2026-09/review-13*.md`,
`rebuttal-13.md`); the tap-edge ruling was revised during review (both edges fire).

### Original brief

1. `crates/input/src/button_tracker.rs`: `release` records `just_released` only when
   `pressed.remove` returned true; add `previous: HashSet<T>` snapshotted in
   `clear_frame_state`, `was_pressed`, and `just_pressed` as a chronological `Vec`.
   `InputHandler::was_source_pressed` dispatches over keyboard/mouse/gamepad (gamepad axes use
   the existing `prev_axis_values`); `source_was_pressed` in `input_mapping.rs:226` becomes a
   call to it. Both edges are defined from the snapshot: `just_activated` = a bound source is
   `just_pressed` this frame AND the action was not active last frame; `just_deactivated` = no
   bound source is pressed now AND (the action was active last frame OR a bound source was
   released this frame). A sub-frame tap therefore fires BOTH edges in one frame (Jesse,
   Sep 3, on kimi's batch-1 F1: latch-style consumers must not stick after a tap). Tests: release-without-press is no edge; press+release inside one frame fires
   `just_activated` and `just_deactivated` in that frame and nothing the next; the existing
   second-key-while-held test stays.
2. `crates/common/src/color.rs:149` `to_rgba8` rounds and clamps. Test: byte round-trip
   identity for all 256 values; a `#solid:RRGGBB` ref survives ref → colour → ref.
3. `crates/renderer/src/texture.rs:257,277,290`: multiply in `usize` (check whether
   `MAX_TEXTURE_DIMENSION` already bounds it; if so, hardening only).
4. Drift guard for the scene serializer's exclusion list (`scene_serializer.rs:294`): save a
   world holding the registry default of every persistent type; assert each appears exactly
   once and no `Dynamic` duplicates a concrete row. (Batch 8 replaces the list with a table.)
5. `crates/ui/src/font/layout.rs`: one `text_height(font, size)` used by both `layout_text`
   and `measure_text`. The font's `new_line_size` wins (it is what `measure_text` returns and
   what centering already uses); `layout_text` stops inflating the height for descender-tall
   glyphs. Test: measured == laid-out height for a string with descenders (the ui cut's
   `test_measured_height_matches_laid_out_height_within_a_pixel_for_descenders` asserts
   within a pixel today and tightens to exact equality here).
6. `crates/editor/src/inspector.rs`: `inspect_component` draws through `label_styled` with
   `InspectorStyle` colours (the read-only Play-mode inspector is currently unthemed). Test:
   header draw command carries `style.header_color`.
7. `GizmoPalette::default()` / `GridColors::default()` derive from `EditorTheme::default()`;
   `DockPanel.header_height` field deleted in favour of `HEADER_HEIGHT`.

Extra verification: games `cargo test` gate (every `just_activated` rides on item 1), wasm gate,
and a visual glance at `hello_world` and `editor_demo` for the text-height change (Jesse).

## Batch 2 — pure deletions and workspace hygiene — DONE Sep 3 2026 (cc31078; three passes, reviews 14 and 16)

- engine_core: delete `scene_manager.rs` (zero callers) and its re-export; delete
  `ui_manager.rs` (three-method forwarder; `GameRunner` holds `UIContext`); delete
  `AssetManager::load_texture_with_config` (zero callers); `TextureHandle { id: 0 }` →
  `TextureHandle::WHITE` in `ui_integration/mod.rs` (ticks #84 DRY-011).
- physics: delete the `world.emit_event(collision.clone())` bus mirror in
  `physics_system/update.rs` (no `read_events::<CollisionData>` anywhere) and its
  `training.md` sentence; delete the five redundant `.with_scale(100.0)` in `presets.rs` and
  the four unused presets.
- audio: delete `play_music_once`, `stop_all`, `active_sound_count`, `unload_all` and their
  tests. The three keep-list tests that call `play_music_once` (`manager/tests.rs:212`, `:228`,
  `:318`) are rewritten onto `play_music` in the same commit, before/after gated like every
  keep-list cut.
- renderer: delete `update_instance_buffer`, `invalidate_texture_cache`,
  `clear_texture_cache`, `pipeline()`, `camera_bind_group_layout`, `texture_bind_group_layout`;
  keep one accessor pair (`device`/`queue` borrow; convert the four engine_core callers);
  inline `create_sampler` and `render_with_sprites_internal`.
- ui: delete `hot_widget`/`is_hot`/`local_mouse`, `is_overlay`, `hit_test`, `cache_stats`,
  `checkbox_labeled`.
- editor: delete the 14 dead items in `audit-editor.md` §7.1 (`generate_entity_sprite` and
  `batch_entities` also drop the renderer sprite imports from `viewport/mod.rs`), the
  `prelude` module, `enter/exit/toggle_play_mode`, `pan_camera`, `visible_item_count`,
  `close_all`, the six zero-user `layout.rs` constants, the dead `depth` parameter in
  `inspector.rs`, the `let _ = component_index;` vestige, the `run_game_with_editor_api`
  middle link, and whichever dock `set_/toggle_` twin has no caller.
- common: drop the unused `thiserror` dependency (ticks #91 KISS-002).
- Workspace: `git rm --cached editor_prefs.json` + `.gitignore`; delete `validate_demo.sh`
  (runs a non-existent example); `git mv run_gpu_diagnostics.sh scripts/`; delete
  `examples/pong_editor_screenshot.png` (update the note in `docs/EDITOR_UX_AUDIT.md`); delete
  the three shipped plans in `docs/plans/`. Keep `coordination/*` (referenced by the ruleset).
- Docs in-batch: root `CLAUDE.md` manager list, `training.md` directory map and Manager
  Pattern snippet, `crates/engine_core/CLAUDE.md` manager line.
- **Additions from the test-cut reviews (Sep 2–3), each grep-shown in the batch review before
  deletion.** `common::Time` (also unwind its re-exports, e.g. `crates/renderer/src/lib.rs`),
`common::Camera::world_bounds`/`contains_point` (the editor uses its own `AABB` and
`visible_world_bounds`), `Transform2D::lerp`, `Color::lerp`, `Rect::intersects`/
`intersection`, `EventBus::type_count`/`count`/`has_events` (+ `World::has_events`),
`ResourceStorage::contains`, `StateMachine::just_left`/`force_transition_to`,
`HierarchicalStateMachine::in_group`, `SpriteAnimation::is_complete`, `Tilemap::tile`,
`GlobalTransform2D::transform_point`/`from_transform`, `Children::is_empty`/`len`/`contains`,
`Parent::new`/`entity`, `SimpleSystem`, `ecs::init()`, and the whole `ecs::generation` module
(`EntityGeneration`, `EntityIdGenerator`, `EntityReference`) — all reported caller-free by the
`ecs` cut review (review-5 F5); from the `input` cut: `GamepadManager::connected_ids`,
`get_gamepad_mut`, `InputHandler::gamepads_mut`, `GamepadState::prev_axis_value`,
`InputMapping::{unbind_source, unbind_action, actions_for, has_binding, clear,
just_deactivated}`, `InputSettings::{just_deactivated, is_active_any, axis_value}` (some
carry keeper assertions that go with them); from the `physics` cut: `PhysicsSystem::raycast`
and `PhysicsWorld::raycast`, `apply_force`/`reset_forces` (the one-update-force guard goes
with them), `set_body_transform` (internal only), `CollisionEvent::involves_entity`,
`Collider::with_collision_groups` (fields stay: inspector + RON read them),
`tracked_entities`/`rigid_body_count`/`collider_count`/`pixels_to_meters*`/
`meters_to_pixels*`/`with_iterations`/`with_fixed_timestep`/`set_gravity`/`gravity()` —
0 callers outside physics; and NINE unused presets (`player_top_down`, `pushable`,
`physics_prop`, `small_box`, `pushable_box`, `bouncy`, `slippery`, `low_gravity`,
`high_gravity`), not the four the audit counted; from the `renderer` cut:
`SpriteBatch::add_instances`, `SpriteBatcher::add_sprites`/`batches_mut`/`sprite_count`
(test-only consumer), `SpriteInstance::new`, `TextureHandle::new`,
`TextureError::TextureNotFound` (never constructed), and `scissor::intersect_scissor`
becomes private; from the `audio` cut: `play_music_once`, `stop_all`, `active_sound_count`,
`unload_all` (confirmed 0 callers); from the `ui` cut: `UIContext::{with_theme, hit_test,
font_metrics, label_in_bounds, checkbox_labeled, slider_range, font_manager,
font_manager_mut}`, `FontManager::{cache_stats, clear_cache, rasterize_glyph}`,
`Theme::light`, `DrawList::{is_overlay, image_rounded, text_placeholder}`,
`InteractionManager::{is_hot, is_active, has_focus}`, `WidgetId: From<u64>` (0 hits outside
the crate; some used internally — keep those); from the `engine_core` cut:
`LifecycleManager::wait_for_state`, `ParticleEmitter::resume`, `GridMesh::{with_alpha,
set_alpha}`, `Particle::t()` (plus `SceneManager` and `Timer`, already listed). The
`test-support` feature and self dev-dependency landed with the engine_core cut, so batch 5
only needs editor_integration to enable it; from the `editor` cut: `Selection::set_primary`,
`CommandHistory::try_merge_or_execute` (and note: on a successful merge it never writes the
merged value to the world — fix or delete in batch 6), `WorldSnapshot::entity_count`,
`ComponentKind::category()`, `ViewportInputHandler::{with_config, is_selecting, reset}`,
`ViewportInputConfig` (only built via Default), `Gizmo::{set_axis_length, set_rotation,
set_scale}`, `GizmoMode::name`, `GridRenderer::set_axes_visible`, `EditorContext::pan_camera`,
`SceneViewport::{focus_on, set_interpolation_speed}`, `EntityPicker::with_pick_margin`,
`Menu::visible_item_count`, `MenuBar::close_all`, `StatusBar::set_version`,
`HierarchyPanel::{collapse, expand}`. Batch 6 decision: two edits with an EMPTY `field_hint`
currently merge (string equality); decide whether an empty hint refuses to merge and pin it.
NOT `Transform2D::forward` — asteroids aims every
bullet with it (`ship.rs:122`); its test was restored (review-4 F1). Rule: show the
workspace AND `../games` grep for every candidate in the batch-2 review before deleting it. NOT `Rect::contains`/`center`/`expand` — the keep-list called `Rect`
dead, but `contains` decides every widget interaction and the editor uses the other two;
their tests were restored (review-3 F1). Likewise the `Color ↔ Vec4` conversion test stays,
strengthened to all four channels: it is the live scene-save path (review-3 F2).

Extra verification: games check gate, wasm gate, `cargo run --example hello_world` smoke for
the UIManager removal (frame path is not headless).

## Batch 3 — mechanical sweep: comments, names, allows (~700 lines, ~110 files, no code gained)

Comment policy:
- `///` stays but states the contract in a few lines. Cut the module-doc essays to that:
  `physics/src/physics_system/mod.rs:20-55` (changelog → `log_archive.md`), `ui/src/lib.rs`,
  `ui/src/context/mod.rs`, `input/src/input_handler.rs`, `input/src/input_mapping.rs`,
  `audio/src/manager/mod.rs`, `renderer/src/renderer.rs`, `editor/src/theme/mod.rs`,
  `editor/src/commands/mod.rs`, `engine_core/src/prelude.rs`.
- `//` survives only as a pitfall, invariant, or failed approach (keep: the Firefox WebGPU
  crash rationale in `game/render.rs`, the two-batcher painter's reason, the `Box<dyn
  Component>` downcast trap, the RwLock re-entrancy guard, the tie rule in `scores.rs`).
- Keep the reason, drop the tag: issue numbers, `kimi … F3` review tags, `audit §9`, sprint
  names, `GPP-15`/`H7`/`E5` codes. A comment that is only a tag is deleted.
- Delete narration that restates the next line (`frame_tail.rs`, `white_texture.rs`,
  `sync.rs:53,56`, `component_commands.rs:99`, `render.rs:46,110`, `scene_io.rs:252`).
- Strip the numbers from "Phase N"/"Create …" section headers now (`render.rs`,
  `sprite/pipeline.rs`, `EditorGame::update` 0–12 with no 8); batches 8/9 turn each into a
  function name.
- Fix the misattached doc comments (`theme/mod.rs:33-40`, `scene_io.rs:225-239`) and three
  stale hex values; rewrite the false "no gamepad backend" module doc in `input/src/gamepad.rs`;
  delete the `PATTERNS_AUDIT.md` reference in `renderer/src/sprite/instance_cache.rs:1`.
- Gate: this returns nothing afterwards:
  ```sh
  grep -riEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|Sprint [0-9]" crates src examples --include=*.rs
  ```
  (case-insensitive, no blanket exclusions — a match inside an attribute or hex literal is
  inspected by hand; the gate runs in every later batch too)
  ```sh
  ```

Names (production code; tight math in `common` stays; WGSL untouched — no headless check):
physics `bodies.rs` `he/f/imp/pos/vel` → `half_extents_meters/force_meters/impulse_meters/
position_meters/velocity_meters`, `components.rs:214` `hw/hh`, `update.rs:19` `dt` →
`clamped_delta_time`, `queries.rs` `toi`, `stepping.rs:56` `let event_handler = ();` deleted;
ui `kb/pos/dx/bg/d`; renderer `img`; engine_core `mgr`, `scene_serializer.rs` single-letter
component bindings (`b` bound to two types), handlers `vel_x/vel`; editor `c/b/w/n/pc/btn`.

Allows: `behavior_runner/handlers.rs` handlers take `&Behavior` and destructure (three
`too_many_arguments` go, and no more swappable `f32` positional pairs); `WidgetId::from_str` →
`WidgetId::hashed` (23 sites); `slider` takes a `SliderVisual` struct; the three editor allows
wait for batch 7; `renderer.rs:230` `arc_with_non_send_sync` stays (decided with H8).

## Batch 4 — DRY in the systems crates (~700 lines; design-structure.md §H, §I, §J)

- renderer: `camera_binding.rs` `CameraBinding { buffer, layout, bind_group }` composed by the
  sprite and line pipelines (closes #89); `pipeline_builder.rs` `build_render_pipeline(device,
  PipelineSpec)` used by all three pipelines; `TextureManager::insert_rgba` shared by the three
  loaders; `Sprite` stores `SpriteShape` (flattened only in `to_instance`); `sprite_data.rs`
  and `line_pipeline.rs` attributes via `wgpu::vertex_attr_array!` (offsets verified identical
  to the WGSL locations; add `offset_of!` tests); `PassScissor { Fullscreen, Rect, Empty }`
  replaces `Option<Option<..>>`; extract pure `bloom_dims(width, height)` (the `.max(1)`
  guard) and `DynamicBuffer::grown_capacity(current, needed)` so both get headless tests
  (renderer cut review, F4 and the skipped guard).
- physics: `push_collision` helper in `stepping.rs`; `previous_collisions` reuses its
  allocation.
- audio: private `start_sink(output, source, base, bus, looping)`; factor the effective
  volume into a pure `effective_volume(base, bus, master)` so the multiplication is tested
  without a device (audio cut review, F3).
- common: `clamp_volume` used by the audio manager and the three `with_volume` builders in
  `ecs/audio_components.rs` (closes #82, ticks #86 DRY-004).
- input: fixture test FIRST (a bindings JSON written by today's code, checked in under
  `crates/engine_core/tests/fixtures/`, must load identically after); then
  `InputMapping<A, S = InputSource>` backs `PlayerBindings` (`bind`/`unbind` return `bool` to
  feed the dirty flag); one `STANDARD_PAD_LAYOUT` const with `PlayerSource::on_pad(pad)` feeds
  both `with_default_bindings` and `bind_standard_pad_layout`.
- ui: `edit_field` core (focus/keys/commit/draw shell + `resolve_font`) under `float_input`
  and `text_input` (fixes the font-resolution drift); delete the `typed_keys` array and drive
  typed characters from the chronological `just_pressed_keys()` (batch 1); `TextInputStyle`
  derives `Copy`.

Extra verification: games test gate, wasm gate, and a native `hello_world` run for the vertex
layout and shape enum (GPU-only) — Jesse's check.

## Batch 5 — DRY in engine_core (~500 removed, ~300 added; §A-colours, §B, §C, §D, §E, §K)

- Delete `behavior_data.rs`: `pub type BehaviorData = ecs::behavior::Behavior;` keeps every
  import path; `ecs::Behavior` already carries identical serde defaults. Guard: parse every
  `examples/assets/scenes/*.ron`; round-trip every variant through RON in `ecs`.
- Grid, half 1: `GridMesh { config: GridBackdrop, origin, substeps, … }` with one constructor
  `from_config`; delete the `with_*` builders and `from_topology`'s private defaults (dead in
  production); `apply_grid_tunables` becomes one assignment. Half 2 (newtype wire variant via
  RON `UNWRAP_VARIANT_NEWTYPES`, deleting `grid_defaults.rs`) only if the compatibility spike
  test in `design-structure.md` §B passes; else a `grid_default!` macro.
- `save_store/json_slot.rs`: `MergeOnLoad` trait + `JsonSaveSlot<T>` + `SaveError` +
  `unix_seconds()` built on `common::clock` (both current call sites already are; `std::time`
  panics on wasm); `AchievementManager` and `Scores` hold a slot; `AchievementError`/
  `ScoresError` become aliases. `achievements/toast.rs` takes `ToastQueue`/`ToastStyle`/
  `draw`. Test the slot protocol once with a tiny document.
- `GameContext`: `FrameRequests { exit, window_title, engine_ui_clip }` private, with
  `request_exit()`, `set_window_title()`, `window_title_requested()`, `clip_engine_ui()`,
  `into_outcome()`; `GameRunner::build_context` + `absorb` replace the two 18-field literals
  (`game.rs:461`, `app_handler.rs:213`). `chaos_mode`/`time_scale` stay fields. Games: twelve
  `ctx.exit_requested = true` → `ctx.request_exit()` (six `menu.rs`, six `gameplay/mod.rs`);
  editor_integration three sites; docs in `pause.rs`, `menu_panel.rs`, `CLAUDE.md`,
  `training.md` Pause Pattern.
- `Localization { strings, base_font, fonts_by_path }` grouped out of `GameRunner`.
- `follow_entity`/`follow_tagged` → `follow_target(Option<Vec2>, …)`; seven colour
  destructurings → `.into()`; `sort_batch_refs` computes its key once; `WindowConfig:
  From<&GameConfig>`; `run_game` returns `Result<(), EngineError>` (games call `.unwrap()` /
  `{e}` — source-compatible); `#[must_use]` on `Scores::submit`, `SpriteAnimation::play`/
  `ensure_playing` (not on `unlock`: games discard it 34 times). `run_game` returning
  `EngineError` also breaks `examples/hello_world.rs:520` and `examples/behavior_demo.rs:102`,
  whose `main` returns `Result<(), Box<dyn Error>>` with `run_game` as the tail — change both to
  `run_game(..)?; Ok(())` — and `crates/editor_integration/src/editor_game/mod.rs:560`, which
  calls it as a bare statement: `run_game_with_editor` propagates `EngineError` (its return
  type widens to match). Only the games are source-compatible.
- `test_support.rs` behind a `test-support` feature: one `StubResolver`, `roundtrip`,
  `test_texture_path`; engine_core takes the self dev-dependency
  `engine_core = { path = ".", features = ["test-support"] }` so its own `tests/` see the one
  copy, and `editor_integration` enables the feature as a dev-dependency; six copies deleted.
  `tests/common/mod.rs` is for helpers only integration tests need.

Extra verification: scene round-trip suite + new behaviour fixture; games test gate (all use
`run_game`, twelve edited); wasm gate.

## Batch 6 — DRY in editor commands and the command API (~450 removed, ~250 added; §F, §G)

- `SetComponentCommand<T>` replaces `impl_set_component_command!`; the 13 names become type
  aliases; `TransformGizmoCommand` = `SetTransformCommand` with `GIZMO_FIELD_HINT` (five call
  sites); registry entries drop the `=> SetCmd` token. Two display-string asserts change
  ("Set Transform" → "Set Transform2D"). Add: distinct `T`s never merge; gizmo hint never merges
  with a field hint.
- `stored_component/component_ref.rs` `ComponentRef { Typed(ComponentKind), Dynamic(String) }`;
  one Add/Remove pair with `::dynamic(..)` constructors; the dynamic twins deleted (five sites).
- `physical_floors.rs`: shared floor constants/`clamp_*` used by `sanitize()` and the field
  editors.
- `write.rs`: `build_add_patch_set` reuses the Set path; `api.rs` uses `WriteCtx::record` and
  passes the parsed request instead of re-parsing the raw line.
- `archetype.rs` `Archetype` enum (`kebab`, `from_kebab`, `menu_label`, `ALL`); `ARCHETYPES`
  derives from it; `handle_create_action(&str)` → `create_archetype(Archetype)`; menu items
  generated from `Archetype::ALL` (closes #90 ARCH-101).
- `menu/actions.rs` `action_for_menu_label` → `EditorAction` (gains `CreateEntity(Archetype)`,
  `Exit`, `TogglePanel(PanelId)`, `ResetLayout`, `CycleGameLocale`, and
  `allowed_while_playing()`); `handle_menu_bar` shrinks to one dispatch through
  `dispatch_editor_action`. Shortcut Undo/Redo gain the menu's status message; the drag guard
  now also protects menu-driven Undo/Delete/Paste/Cut. Test: every enabled menu item maps.
- `CommandHistory::push_as_one` / `execute_as_one` (four copies); `Modifiers::read(input)` in
  `editor_input.rs` (five spellings, `ctrl_held` deleted); `WidgetSlot` enum behind `FieldId`
  replaces the 99/+50/+60 arithmetic; one `uncaptured_component_names`; `loss_warning`/
  `drop_report` share a body; `scene_io.rs` calls `World::clear()`, gains `reset_session()`,
  returns a typed `SceneIoError`.

Extra verification: `commands/tests.rs`, `dirty_tests.rs`, `write_tests.rs`, selection-restore
and archetype drift tests; a headless `--api` transcript exercising set/add/create/undo/save.

## Batch 7 — DRY in the editor UI (~400 removed, ~200 added)

- `EditResult::assign(slot, hint, name)` replaces the 82 identical writeback blocks across
  `component_editors.rs`, `behavior_editor.rs`, `ui_component_editors.rs`,
  `component_editors/grid_backdrop.rs`.
- `EditableInspector::next_field()`/`advance()` replace the ten repeated preambles; one
  `remove_button`; `InspectorFrame` context struct replaces the long argument lists (the last
  three `#[allow(too_many_arguments)]` go).
- `panel_renderer/add_component_popup.rs`: one walk both renders and measures;
  `categorized_components()` computed once per frame.
- Theme aliases removed (`bg_*` → `surface_N`, `pause_yellow` → `warn_yellow`, `inspector_*`
  → base tokens); the surface-ladder guard tests are the safety net.
- The seven `8.0` and four `20.0` route through `layout::PADDING`/`LINE_HEIGHT`; `rect_border`
  replaces four hand-drawn lines; `render_node` loses its duplicate recursion;
  `draw_world_segments` serves the three overlays; `EditableFieldStyle` borrowed not cloned per
  component; `DockPanel.bounds`, `ViewportInputHandler.config`, `GridRenderer.config` private.

Extra verification: editor suites; an `editor_demo` visual pass (a wrong theme token is
invisible to tests) — Jesse's check.

## Batch 8 — SRP splits, engine side (~900 lines moved; §A)

- `scene_loader_components.rs`: one pure `build_<component>` per arm (each < 25 lines; the
  physics pair as plain functions absorbing the unused-suppression). `scene_serializer.rs`:
  one `extract_<component>` per type plus the `ConcreteComponent { wire_name, registry_name,
  extract }` table that also yields the dynamic exclusion set (`CONCRETE_OR_EXCLUDED` deleted;
  `EXCLUDED_NON_WIRE = ["Name", "GlobalTransform2D"]` is the only hand list left; the batch-1
  drift test now checks the table; `component_type_name` stays exhaustive so a new variant
  still fails to compile). Roots are emitted in a stable order (EntityId ascending — ids are
  assigned in file order at load, so load → save preserves order; today `get_root_entities`
  iterates a `HashMap`, review round 1 F1) — this is what makes the golden check below
  meaningful.
- `game/render.rs`: `collect_game_sprites`, `collect_ui_sprites`, `submit_frame`;
  `game/frame_tail.rs`: `step_simulations`, `draw_scene_ui`, `apply_window_requests`.
- `ecs/hierarchy_system.rs` `update` split into collect and propagate.
- `bloom.rs`: `ensure_ready` + sequence-only `run`; `sprite/pipeline/builders.rs` (the file is
  at 552 lines).
- Examples: `hello_world.rs` `update`/`init` split into named phases (the file every game is
  copied from); `editor_demo.rs` shares the platformer via `#[path]` instead of a hand-synced
  copy.

Extra verification: a golden scene test (load → save → byte-identical to the checked-in
expected RON for `hello_world.scene.ron`, valid only after the stable root order lands; the
generated golden is hand-diffed by Jesse against the pre-change file — zero semantic diff
modulo root order — before check-in, so batch 8's own bug cannot be blessed); games test gate (breakout level tests load scenes); `hello_world` run.

## Batch 9 — SRP splits, editor side (~1000 lines moved)

- `EditorGame::update` becomes named phase methods (one dirty-sync point if handler order
  allows, else two with the reason named); `ApiSession` and `SceneConfirm` pull four fields out.
- `command_api/write/verbs.rs`: one function per verb (the 267-line `run`); `parse.rs`: one
  parser per verb.
- `shortcuts.rs`: `handle_play_action` and `dispatch_editor_action` grouped by category.
- `viewport_interaction.rs`: `PickableCache` built once per frame, shared with
  `panel_renderer` (today up to four `build_pickable_entities` per frame).
- Test seams (editor_integration cut, Sep 3): `drain_api_requests`, `handle_editor_key`,
  `handle_viewport_picking` and `render_inspector` take a `&mut GameContext` they only partly
  use, so their guards (mid-drag API skip and the 256-line cap, `wants_keyboard` gating,
  no picking during Play, read-only inspector while Playing) cannot be written headlessly;
  narrow each to the fields it uses — as `delete_selected_entities`/`duplicate_selected_entities`
  and the clipboard trio already were (`&mut World`) — and write those guards here. The
  dirty mirror (`EditorContext.is_dirty` following `CommandHistory`) is synced only inside
  `update`; give it a world-only entry point and pin it (the deleted tests had reconstructed
  the mirror in the test body).
- `behavior_editor.rs`: per-variant editors; `render_inspector_editable`,
  `render_asset_browser`, `render_node` split at their visible seams;
  `stored_component/kind.rs` takes `ComponentKind`, `ComponentCategory`,
  `categorized_components`.

Extra verification: all editor suites, headless API tests, and an `editor_demo` manual pass
(play/pause/stop, undo merge, marquee, gizmo) — Jesse's check.

## Batch T — test suite: keep-list, not trim-list — DONE Sep 3 2026

Landed on `jesse` as one reviewed commit per crate (common 98898e0, ecs 6db8d96, input
1601d96, physics 6f8ec4b, audio 528fb62, renderer 19b2faf, ui 5dc0331, engine_core 18eb6dd,
editor 5630436, editor_integration 28be855; docs counts b4e2aae). 1,657 → 633 `#[test]`
functions; every gate green; every review round adjudicated in
`review/cleanup-2026-09/rebuttal-*.md`. Batches 1–10 follow.

### Original brief

Jesse's framing (Sep 2 2026): "200 really good tests that keep critical functionality certain"
beat 1,600 that only buy coverage, plus tests that catch footguns and antipatterns being
reintroduced. So the suite is rebuilt from a keep-list rather than pruned. Two categories
survive, everything else is deleted:

1. **Contract tests** — one per player/author-visible contract (`training.md` rubric):
   lifecycle and state machines, cross-component wiring (animation writes `tex_region`,
   physics writeback), persistence and legacy formats (scene RON, `.sheet.ron`, input JSON,
   saves), non-obvious math (UV cells, camera bounds, snapping), typed error paths, undo/redo
   and dirty semantics, selection restore, input edge semantics, scene round-trip.
2. **Guard tests** — one per known footgun or antipattern, named for what they prevent:
   the `CLAUDE.md` "Known Footguns" and each crate guide's "Common Pitfalls" each get a guard
   (`Box<dyn Component>` downcast, physics ignoring scale → collider overlay contract, double
   `take_collision_events`, `write_buffer` flush ordering where headless-checkable), plus the
   structural guards the audits want: scene-table drift, `offset_of!` vertex layouts,
   Behavior scene fixture, input JSON fixture, `JsonSaveSlot` protocol, menu-label ↔
   `EditorAction` drift, `Archetype` kebab round-trip, merge isolation between command types,
   the WCAG surface-ladder luminance guards (already exist, keep), command-API spec drift
   (exists, keep), and a workspace guard that fails on any `#[ignore]` or ` ```ignore `.

Rust Book conventions (ch. 11, Jesse's ask: follow it as closely as we can):
- **Three actions per test**: set up, run the code under test, assert. A test that skips the
  middle step (constructs and asserts its own literals) is not a test.
- **Unit tests** live in the file they test, in `#[cfg(test)] mod tests { use super::*; }`;
  they may reach private items. When the file is at the 600-line ceiling, the module moves to
  a sibling `tests.rs` declared as `#[cfg(test)] mod tests;` from the module directory. Retire
  the other two conventions the editor crate grew: crate-root `*_tests.rs` files declared in
  `lib.rs` (`hierarchy_tests.rs`, `inspector_edit_tests.rs`, engine_core's
  `scene_*_tests.rs`) and `#[path = "entity_ops_tests.rs"] mod tests_file;`.
- **Integration tests** in `crates/<crate>/tests/` exercise only the public API "the way any
  other external code would" — one file per public surface, not a mirror of the unit tests
  (the audit's duplicated `tests/` files go for exactly this reason). Shared setup lives in
  `tests/common/mod.rs` (never `tests/common.rs`, which cargo would run as a test file):
  input's queue-and-process `frame` helper, engine_core's scene round-trip quartet, physics'
  body spawner move there.
- **`assert_eq!`/`assert_ne!` over `assert!(a == b)`** so failures print both values; a
  custom message wherever the value alone would not explain the failure.
- **`#[should_panic(expected = "…")]`** always carries `expected` (both existing ones do;
  keep it that way), so a different panic cannot pass the test.
- **Tests that chain fallible setup return `Result<(), E>` and use `?`** instead of `.unwrap()`
  ladders; never `should_panic` on a `Result` test (assert `is_err()` instead).
- **Doc tests are executable examples** of public API: keep them, ` ```no_run ` only for
  GPU/window-bound ones, never ` ```ignore `.
- Binary crates stay thin: `src/bin/editor.rs` (a 63-line `main`) should call into library
  code so its behaviour is testable through `use`.

Process, per crate, in the batch that touches it:
- `audit-tests.md` (being reframed as a keep-list now) names for each contract/footgun the ONE
  existing test that best locks it, or MISSING. Keep those; strengthen weak asserts (draw-list
  length → content, `len() >= 5` → exact); merge near-duplicates into the kept test so it is
  complete; write the MISSING ones; delete everything else — including every constructor/
  `Default`/`label()` echo, assert-free test, "doesn't panic" test, test that reimplements
  production math (`viewport_input.rs:328-382`), and duplicate of another crate's coverage
  (`component_editors.rs` five `test_*_default_values`).
- One `test_support` module per crate for fixtures (world builders, tiny WAV/PNG, stub
  resolvers); duplicate helper names (`test_texture_path_fn` ×3, `test_ranges_are_well_formed`
  ×2) collapse.
- Test names state the contract or the footgun (`test_release_without_press_is_not_an_edge`),
  never the method.
- **No count target.** Jesse's "200 vs 1,600" was a frame of reference for priorities, not
  a budget: whatever number falls out is the number. The keep-lists below were produced by
  agents that squeezed to a per-crate slot count and flagged what they cut to fit — every
  such "promote if the budget stretches" item IS kept (the confirm-dialog guard,
  `chrome_owns_mouse` on the release frame, `AudioSource::calculate_attenuation`, the
  `GridBackdrop` topology cycle, `ps:491` roots-only, `pw:359` world-space contacts, the
  `ihi:214`/`mouse:122` marginals). The only test of a test is: would a reader understand
  what it protects, and would anyone care if it failed. No doc records the count (batch 10).
- Verification: after each crate's cut, `cargo test -p <crate>` is green and the kept tests
  each map to a named contract or footgun in the crate's guide (the guide's "Common Pitfalls"
  becomes the index of its guard tests).

What the test audit already established (report at
`~/.claude/plans/lets-audit-the-code-staged-valiant-agent-aexplore-tests-401344dbeb61c8dc.md`,
every test file read in full; the keep-list reframe is being appended to it):
- **Floor, before the keep-list cut:** 309 outright deletions (18.6%), 184 tests folding into
  76 table-driven tests, 53 method-named renames, 48 right-subject/weak-assert tests to
  strengthen. Six files empty out and are removed: `ecs/tests/{component,init,system}.rs`,
  `engine_core/tests/init.rs`, `input/tests/input_handler.rs`,
  `ui/tests/ui_interaction_debug.rs` (five of its seven tests move inline first — the only
  slider test and the only `button` returns-true-on-release test live there).
- **Do NOT delete** `ecs/tests/system_lifecycle.rs test_panic_recovery_in_systems`:
  `SystemRegistry::update_all` really uses `catch_unwind`; strengthen it (a system added after
  the panicking one still updates).
- **Prerequisite:** add the KeyF rows (F = `FocusSelection`, Ctrl+Shift+F =
  `ToggleCameraFollow`) to `editor/src/editor_input.rs:411`'s default-chord table before
  deleting `camera_follow_tests.rs:531`.
- **The largest hole is not bloat:** `EditorGame::delete_selected_entities` and
  `duplicate_selected_entities` have zero tests; the 14 tests in
  `editor_integration/src/entity_ops_tests.rs` exercise `#[cfg(test)]`-gated copies of
  production code. Delete the copies and the tests, then write contract tests against the
  real paths (multi-select `MacroCommand`, `SpawnTreeCommand` + offset, selection follows the
  copy) and move the two child-reparenting cases onto `DeleteEntityCommand`.
- **Two more dead APIs** (grep-confirmed, add to batch 2): `engine_core::Timer`
  (`src/timing.rs`, exported from the prelude, no consumer in any crate or game;
  `tests/timing.rs` is 147 lines testing it) and `GlobalTransform2D::transform_point`.
- **Fixtures to consolidate** (one `test_support` per crate): the press-frame/release-frame
  click harness written eleven times across `editor` and `ui` with divergent semantics
  (`menu_input.rs frame` skips `end_frame`, its siblings don't); `DummyGame` defined eleven
  times in `editor_integration`; the scene round-trip quartet (`test_texture_path`,
  `StubResolver`, `roundtrip`, sidecar literal) in `engine_core`; the physics three-component
  spawn retyped in 21 tests; `SpriteInstance::new(..)` retyped 16 times in `sprite/batch.rs`
  while `instance_cache.rs` next door has the fixture; `input`'s only queue-and-process helper
  is `#[cfg(test)]`-private so the integration tests repeat it 40 times. Temp-file discipline:
  `tempfile::tempdir()` everywhere (`localization.rs:592` and `scene_serializer_tests.rs:436`
  leak on a failing assert; `editor_preferences.rs:137` races on a fixed path).
- **MISSING contract/guard tests to write** (highest value): I1 production delete/duplicate;
  I2 `DeleteEntityCommand` reparenting; I3 `commit_gizmo_drag` records the collider; I4
  `drain_api_requests` (mid-drag skip, 256-line cap, `note_selection`); E1 viewport pan and
  wheel-zoom through `handle_input`; E2 toolbar shortcut hint ↔ binding drift; E3 menu click
  returns the label and a disabled item does not; E4 `apply_component_edit` end to end (one
  history entry, merges by `field_hint`, undoes); C1 `main_camera_pose` zoom sanitizer; C2
  `set_base_path` cache invalidation; C3 `#rgba` sentinel through save/load; C4
  `draw_labeled` localized pause labels; C5 `GridMesh::translate` moves rest and position, not
  velocity; X1 `World` emit/read/flush and the drain-once collision footgun; P1 collision
  groups filter events; P2 `Collider.offset` collides at the offset; P3 `convert_physical_key`
  and the scroll-line normalisation; R1 vertex/instance attribute `(offset, format,
  location)` triples; U2 `font::layout` baseline/`offset_y`/space handling with the
  `examples/assets/fonts/font.ttf` fixture (Linux Libertine; bounds are fixture-derived); A1 an enabled audio manager's sink bookkeeping via a test-only sink seam.
- **Strengthen** (right subject, weak assert): the draw-list-length family becomes content
  asserts (grid/collider/selection overlays assert endpoints and colours; `float_input`
  asserts bounds and "42.00"; label centering asserts the x); `sort_idempotent` proves the
  guard by mutating out of order; `test_collision_detection` asserts a real response margin;
  `reset_body` asserts the position half; `overfull_pool` asserts the LAST four survive;
  `test_default_for_variant_wraps` stops pinning the variant count.
- **Renames** (53): method-named → behaviour-named, including the two that assert the
  opposite of their name (`test_physics_settings_preserved_on_new` clears them;
  `test_load_scene_resets_selection` never loads a scene).

**Keep-list (per crate; the working list the implementer follows, with the bullets above as
constraints).** Format: contract or footgun → the ONE test that locks it, or MISSING.

*ui — 123 → 25 kept (C1–C25) + 4 MISSING; renderer — 92 → 19 kept (R1–R19) + 4 MISSING guards.*
Full tables with merge-into lists are in the test agent's output
(`…/tasks/a13690c803361b9f1.output`); the shape:
- ui contracts: widget gesture state machine Normal→Hovered→Active→clicked-on-release
  (`tests/ui_interaction_debug.rs:185`, moves inline — the only place `clicked == true` is
  asserted); press-inside/release-outside; `button` fires on release only; slider (the only
  slider test in the crate); `InputState` mouse snapshot; `wants_mouse` gesture ownership;
  missed-release frees the gesture; blocking rect + overlay scope; widget-state lifecycle;
  `TextEditState` typing/backspace/shift-arrow/cursor-from-click; key repeat timing;
  `keycode_to_char` table; drag-scrub (4px arm, non-compounding, Escape restore, Ctrl snap);
  arrow nudge; soft/hard range commit semantics; `text_input` lifecycle; programmatic focus
  (F2 rename); `UiLayer` flush order; layer-stack lifecycle; clip push/pop pair; glyph-cache
  bound + size-tenths key; unresolvable-font placeholder. MISSING: `font::layout` math with a
  DejaVu fixture; slider edge clamping; `KeyRepeat` per-key independence; widget-level cursor
  placement.
- ui guards (all exist, keep): baseline-vs-box (`label_in_bounds_styled` keeps glyphs inside);
  elevated layer escapes a Content clip pair; release frame is not `Active`.
- renderer contracts: GPU struct sizes/strides (`SpriteVertex` 36, `SpriteInstance` 76,
  `CameraUniform` 80, bloom params 16, `LineVertex` 28); default instance = plain unlit quad;
  `to_instance` maps every field (strengthen: `tex_region`, `emissive`, shape); NaN-safe
  `total_cmp` depth sort + `sorted` guard; batching by texture and by clip state; `clear`
  resets the clip cursor; instance-cache skip/upload including same-bytes-different-layout;
  scissor quantize/clamp/`batch_scissor` tables; one-way device-loss latch; `resize_action`
  table; `TextureFilter`→`SamplerConfig`; typed `TextureError`; `WHITE` handle reserved.
  The 8 camera tests in `sprite_data.rs` are cross-crate duplicates of `common` — delete;
  move the one real hole (`projection_matrix` NDC mapping) to `common/src/camera.rs`.
- renderer MISSING guards: attribute `(location, offset, format)` triples vs WGSL (write this
  BEFORE deleting anything — several deleted tests give false comfort here); `BloomPipeline`
  owns one buffer per distinct per-frame value (the `write_buffer`-flush footgun); bind-group
  cache hit on the second identical frame; `DynamicBuffer` grows to the next power of two and
  never shrinks (extract `grown_capacity(current, needed)` to make it headless).
- **Found while auditing (add to batch 4):** `SpriteBatcher.batches` is a `HashMap`, so
  cross-batch order is only deterministic because `engine_core/src/game/render.rs
  sort_batch_refs` sorts afterwards, and the renderer's own guide says callers must. Move the
  ordering into the renderer (`SpriteBatcher::sorted_batches()` or a `Vec` with a stable
  index) and add the guard: two batchers built in different insertion orders emit identical
  draw sequences.

*physics — 64 → 21; input — 74 → 16; audio — 26 → 8* (full tables with merge-into lists in
`…/tasks/a71db13e513ebaddd.output`); the shape:
- physics contracts: static body never moves; collision start/ongoing/stop FSM driven
  through one buffer-clearing loop (pins "the driver clears once per frame"); sensors emit
  events without contacts; world-space contact points; capsule half-height math;
  pixels-per-meter sanitized (strengthen: assert the real fall, not `is_finite`); raycast
  distance in pixels; catch-up cap with no backlog leak; same-frame spawn op buffering
  (strengthen: assert the position half of `reset_body`); ECS→rapier sync + orphan GC;
  `clear()` then resync; shape cycle carries dimensions; dynamic-tier round-trip leaks no
  handle. Gravity on dynamic bodies is covered by the crate doc example — no `#[test]`.
  Guards (exist): zero-step frame emits no events; every sub-step's events survive;
  `apply_force` lasts one update; physics entities are roots; live transform edit teleports
  and keeps velocity; writeback is not an external edit; live collider edit rebuilds; CCD +
  restitution reflection. MISSING: colliders are absolute pixels / `Transform2D.scale` ignored
  (the top footgun in `CLAUDE.md`, untested); collision groups/filter; `Collider.offset`;
  kinematic bodies; `RigidBody` config edits still require recreation (documented only in a
  comment). Thirteen keeps share one `spawn_body` + `no_gravity_system` helper reachable from
  `tests/` (a `test-support` feature or `tests/common/mod.rs`); the merged collision keep uses
  `CollisionEvent::involves` instead of the longhand closure.
- input contracts: queued ≠ applied until process, in order; `update` clears edges;
  `ButtonTracker` model; mouse frame-delta model; wheel accumulation (strengthen: a
  `PixelDelta` event pins `SCROLL_PIXELS_PER_LINE`); `InputMapping` bind/unbind/reverse
  index; action lifecycle across frames; axis-as-button; threshold crossing + re-arm;
  per-player routing + `assign_pad`; merged digital+analog clamped; dirty tracking. Guards:
  `just_activated` strict edge (batch 1 adds the sub-frame tap case); `InputMapping::new()`
  is empty; pads auto-register and disconnect leaves no edge; stick +Y is up (strengthen:
  a sub-threshold deflection is not active). `tests/keyboard.rs`, `tests/input_handler.rs`,
  `tests/gamepad.rs` contribute zero keeps. MISSING: `convert_physical_key` and
  `handle_window_event` (the winit boundary); the 0.5 threshold default (every test passes it
  explicitly).
- audio contracts: disabled mode is a working no-op; missing file → `IoError`; garbage →
  `DecodeError`; unload invalidates the handle; `enable_output` preserves handles, ids and
  buses; pending music last-request-wins / cleared by `stop_music` / none after a failed
  load; bus volumes clamp (strengthen: the `base × bus × master` product is never asserted);
  `SoundSettings` clamp + speed floor. Three keeps call `play_music_once`, which batch 2
  deletes — rewrite them on `play_music` first. MISSING: the wasm always-starts-disabled gate
  is `#[cfg]`-unreachable natively (refactor to `should_start_disabled(is_wasm)` or accept as
  `check_wasm.sh` territory); `IoError` loses the path (pin or document it).

*editor_integration — 150 → 25; ecs inline — 119 → 25* (full tables with merge-into lists
in `…/tasks/a4135e60c14f2de32.output`); the shape:
- editor_integration contracts: play/pause/resume/stop FSM + snapshot; Stop resets the
  propagation baseline; save choke point (file written, parses back, dirty cleared);
  `render` derives camera + scissor from the dock; scale tool moves transform AND collider
  as one undo entry (WEAK: the macro is hand-built in the test — drive `commit_gizmo_drag`
  with a real `GizmoDragState`, the top footgun in `CLAUDE.md`); save/new/open refused mid
  play session; snapshot-loss warning; Stop restores the authored world, resume never
  re-captures; `load_scene` dry-runs into a scratch world (the instantiate-fails case); scene
  physics block round-trips as a resource; engine time frozen outside Play; Play adopts the
  game camera pose incl. zoom, sync is one-directional and paused never syncs, Stop restores
  the editing view; gizmo commit = one entry restoring every root; grid snapping steps cells
  and keeps formation offsets; Escape cancel restores starts incl. collider; held arrow = one
  merged entry sealed on release; API script builds a scene with each step GUI-undoable;
  `api_batch` commits on Play, discarded on Stop; every archetype maps to a factory; headless
  `--api` authoring loop survives a reload; picking AABBs match the `RENDER_UNIT`-scaled
  render with an offset panel; inspector writeback through `apply_component_edit` (its ONLY
  coverage). Promote as 26th/27th if the budget stretches: the confirm-dialog swallows keys
  and Escape cancels; `chrome_owns_mouse` on the release frame.
  MISSING: `UiElementsHidden` inserted on init / removed on Play / re-inserted on Stop (no
  test names the resource); `GridBackdropReset` on Stop; rotation deliberately not synced;
  during Play a viewport click must not pick, marquee, or accept a drop; the dirty mirror
  (`tests.rs:300`, `:332` reconstruct it inside the test — delete both, drive the real
  `update`); a Behavior scene fixture (`FollowTagged` appears in no scene in the repo;
  nothing loads → saves → steps every variant — one `all_behaviors.scene.ron` closes three
  gaps).
- ecs inline contracts: dynamic tier insert/extract/remove by name; name↔TypeId per
  concrete type; same-name-different-type panics at registration; builtin roster +
  persisted/transient split (strengthen: assert the inverse, no name outside the expected
  set); global registry survives a poisoned lock, re-entry panics loudly; late registration
  visible; `StateMachine` transition semantics; hierarchical cross-group transition; events
  readable more than once per frame (the drain rule) and flushed at the boundary
  (strengthen: through `World::emit`/`read_events`, which is what game code uses);
  resources keyed by type; `GlobalTransform2D` composition; `Children` is an ordered Vec
  (WEAK: asserts no order — a `HashSet` swap passes; assert `[a, b, c]` survives re-add and
  remove); reparenting prunes the old list; scale propagates (the only scale case anywhere —
  keep it or add a scale row to `hierarchy_dirty`); `Tilemap` UV/row-zero/non-zero geometry
  and RON round trip; `UiAnchor` resolution and serde defaults; `Lifetime` despawn;
  `SpriteAnimationSystem` writes `tex_region`; `Scripts` round-trips on BOTH wires (the only
  component proven on both — json for the inspector, RON for scenes); Behavior RON round
  trip incl. `Option`, legacy four-field `CameraFollow`, variant cycling. Guards MISSING: a
  boxed component downcasts only through `.as_ref().as_any()` (the Box's own TypeId
  differs); a hand-written `GlobalTransform2D` is clobbered on the next update. Dead API to
  delete with its tests: `GlobalTransform2D::transform_point`, `EventBus::type_count`.

*engine_core — 394 → 72 (59 inline + 13 in `tests/`; ship all 72, not the agent's "hard-60"
cut)* (full tables in `…/tasks/ae026d3ede6e234ea.output`); the shape:
- Persistence is the bulk (47): scene RON save pipeline (field-for-field round trip, Sprite
  extraction, hierarchy nesting, derived state never reaches the wire, `GridBackdrop` every
  field + bare = preset, prefab base/overrides/inline layering — strengthen: all three layers,
  later wins); dynamic tier (Value round trip, transient never persisted, unregistered type
  refuses the whole load); Scripts (every param type, Entity params remap by NAME, save
  auto-names targets); legacy shapes (pre-editor scenes, Sprite serde defaults, legacy
  `CameraFollow`, Tilemap with a resolved tileset); prefabs (overrides, failed spawn leaves no
  debris); `.sheet.ron` every validation path as one error table naming file and clip; sidecar
  pipeline guards (validation BEFORE any handle is allocated; sidecar is SSOT on reload; a
  malformed sidecar warns and falls back); texture-ref sentinels; `ClipData` wire golden in
  AND out with no derived UVs (the most valuable test in the crate — the only one asserting
  wire field names), `SpriteAnimation` round trip, sidecar wins over baked values, animation →
  `tex_region` → renderer instance; save slots (achievements/scores round trip, two writers
  merge instead of clobbering, atomic save leaves no `.tmp`, full list evicts lowest, ties
  oldest-first, `MemoryStore` matches slot semantics, input bindings + pad routing survive,
  missing file → hand-editable defaults); config + locale files (pre-field JSON loads, filter
  alias in / variant out / typo refused, RGBA validation typed error, `tr` fallback chain,
  corrupt locale skipped, `load_dir` by stem, `AssetConfig` mapping).
- Everything else (25): lifecycle invalid-transition matrix; poisoned lock never blocks
  shutdown; a started Scene runs its schedule; pause rows execute and every confirm unpauses;
  `time_scale` is 0 only while paused (strengthen: pair with a `frame_tail` test proving
  particles and `SpriteAnimationSystem` receive `delta × time_scale` — today unguarded);
  held stick scrolls once; `row_at` is the hit-test SSOT (keep the `count == 0` row, the only
  guard against the `count - 1` underflow); spring rest lengths; resting grid translucent;
  negative/NaN tunables clamp; moving the entity translates without a rebuild (strengthen:
  frozen delta still draws, impulses drained not banked); pool overwrites the oldest
  (strengthen: the LAST four survive); emitter rate; dead-zone convergence; NaN look-ahead
  degrades to plain follow; chase hysteresis; delta clamped after a stall; surface-error
  streak latches fatal; main camera syncs position only (strengthen: zoom 0/NaN/negative →
  1.0, untested anywhere); UI stays on the same screen pixels under any camera (strengthen:
  one colour byte survives — the post-tonemap contract); clipped commands carry their clip;
  tilemap expands to one batch; glyph cache keys on font; `@key` resolution; hat crossings;
  sensor↔kinematic pickup once.
- Guards MISSING: `chaos_mode`/`time_scale` writeback persistence (`game.rs` and
  `app_handler.rs` have zero tests — batch 5's `build_context`/`absorb` test covers this);
  post-tonemap UI byte; loader attaches `Name` so names survive load → save → save;
  `main_camera_pose` zoom sanitizer; `set_base_path` drops the dedupe cache; `SidecarCache`
  warns on present-but-unreadable; the **scene-serializer table drift guard** (batch 1 item
  4 — it replaces seven per-variant extraction tests that cannot fail when a type is
  forgotten). The round-trip absorber pulls `test_texture_path`/`StubResolver`/`roundtrip`
  into the shared `test_support` (batch 5). No keeps on `scene_manager.rs`, `ui_manager.rs`,
  `Timer` (all deleted in batch 2).

*editor — 477 → 57 + the budget cuts restored* (full tables in
`…/tasks/aa041f004c7a36305.output`); the shape:
- Contracts (46): new command invalidates redo; undo/redo is id-stable across a delete/undo
  cycle; delete undo restores every component (keep the id assert too); history cap drops
  from the front, undo is LIFO; dirty watermark reads clean at the saved command and dirty
  past it; a merge into a saved command reassigns its id; a merge invalidates redo; undo
  restores the pre-command selection and prunes stale ids; a merged gesture keeps its FIRST
  before-image; paste undo removes the whole subtree, redo resurrects the same ids; cut
  restores ids, hierarchy and values; selection is insertion-ordered with deterministic
  primary fallback (strengthen: re-add keeps position, `shift_remove`); shift-click range;
  F2 rename commit exits and releases the keyboard; display-name ↔ resolve-by-name; every
  default chord resolves; exact-chord-wins; rebind evicts only the exact tuple; marquee from
  (0,0) is real; Escape kills the marquee until release; `screen_to_world` is the inverse of
  the render camera (strengthen: at a nonzero panel origin AND the play-follow pose); F frames
  the selection; picking sorts by depth (strengthen: equal depths order by id); grid
  subdivisions gated by zoom (strengthen: LOD doubling, `max_lines` leaves only the axes);
  collider offset rotates with the body; capsule reach; rotation delta sign + shortest arc;
  translate drag cumulative + release flag (strengthen: X-axis drag drops Y); rotate ring is
  an annulus; scale is a per-axis ratio floored at 0.01; collapsed panel becomes a strip
  (strengthen: hidden panel gives the center full width, toggling relayouts); splitter
  clamps; snapshot restore preserves ids (strengthen: Parent/Children rebuilt and one
  value-fidelity case); unregistered types reported once; editor registry ↔ world type
  enumeration drift lock; dynamic components add/set/remove with undo; API error envelope;
  list/describe shape (strengthen: Name only at top level); `set` shallow-merges as one
  entry; unknown field refused naming the real fields; `batch` = one entry; `rename` reaches
  unnamed entities; prefs round trip (use `tempfile::tempdir()`) and legacy prefs load;
  pending string edit commits before a variant cycle applies (strengthen: the cycle carries
  dimensions).
- Guards (11, all exist): open dropdown renders in the Floating band and swallows clicks;
  toolbar click survives the chrome interact on the release frame; play controls claim the
  gesture; modal scrim blocks input and is not a choice; WCAG adjacent surfaces ≥ 1.35:1;
  popup border ≥ 3:1; selection colours derived from theme tokens; no chrome below
  `MIN_READABLE_FONT`; `break_merge` stops `field_hint` merging; writes refused while
  Playing; cancel latch suppresses the gesture until mouse-up; parser verbs match the spec
  table; unissued texture handle refused at the write; API writes obey the GUI floors; the
  collider overlay ignores `Transform2D.scale` like physics.
- Restored under the no-budget rule (the agent's own discomfort list): `row_layout.rs`
  pair-slot/ellipsize measurement, `scroll.rs` clamp math, `drag_drop.rs` drop-consumed-once,
  `editable_inspector.rs:546` degree/radian wrap, `selection_outline.rs:265`
  primary-vs-secondary affordance, `collider_overlay.rs:261` capsule reach,
  `context/tests.rs:339` frame-selected. Files with zero keeps after that are deleted whole.
- Guards MISSING (belong in `editor_integration`, add there): shortcuts gate on
  `wants_keyboard()` (typing must not trigger Delete/tool keys); picking honours
  `is_input_blocked_at` AND `wants_mouse()`; merge isolation across entities (A cannot merge
  into B; a wrong `field_hint` yields one entry per frame); gizmo drag = ONE undo entry across
  all roots with idempotent apply; the inspector panel is read-only while Playing.
- Count reconciled: the editor crate has exactly 477 `#[test]` functions (the agent's ~496
  was a miscount).

*common — 41 → 16; ecs `tests/` — 94 → 21; ecs_macros — 3 → 1.* common keeps are the
conventions every crate assumes: screen +Y down / world +Y up; screen↔world round trip; the
matrix is T·R·S; inverse transform round trip; `transform_direction` under non-uniform
scale; sRGB luminance and 21:1 contrast (feed the WCAG guards); hex → Color (strengthen: all
four channels); `SheetGrid` truncates partial cells, keeps non-reciprocal UV sizes, row-major
`uv_rect`, checked variant, degenerate grids never divide by zero; `vfs` boot keys and
`list_dir_files`; the `with_fields!` macro output. ecs `tests/` keeps are the stronger side
of every duplicated pair: dirty-flag recompute COUNTS (clean frame recomputes zero, leaf
change one, parent change its subtree, deletion prunes the cache, identical write stays
clean, re-enable catches stale); animation clip semantics (`ensure_playing`, non-looping
clamp, shorter clip never exposes a stale frame, `current_uv`, broken clips never panic,
omitted `tex_region`/`visible` deserialize to full/visible, `ComponentMeta` field ORDER,
pause/resume); world contracts (stale id rejected, snapshot revives an id, orphaning,
100-deep removal leaves no residue, concrete type names not the Box's, cycle error names
the cycle, typed queries, world FSM, late-added system gets its hooks, panic recovery —
strengthen with an assert). ecs_macros keeps the field-order test.

**The consolidated keep-list (`scratchpad/audit-tests.md`, 1,016 lines, 312 keeps, every
line number re-derived from the tree) is the authority over the per-cluster outputs quoted
above where they differ.** Batch 0 below copies it somewhere durable.

The dead-API additions from the cut reviews now live under Batch 2 (moved Sep 3 — gemini's first batch-2 pass missed them here).

## Batch 0 — before any code

- Plan review: done (two kimi rounds, `review/review-{1,2}.md`, `rebuttal-{1,2}.md`; settled
  by Jesse after v3).
- Records: move the audits, the sequencing plan, the structural design, the consolidated
  keep-list and the per-cluster keep-lists from `review/support-*.md` into a tracked
  `coordination/cleanup-2026-09/` (this plan included as `plan.md`); `review/` keeps only
  review-round artifacts. Archive or delete the directory when the effort closes.
- Board: file one tracking issue on `beinsiculous/insiculous_2d` in the house shape and claim it.
- Branch: `git switch -c jesse dev`.

## Batch 10 — docs and guides (~300 removed, ~120 added, Markdown only; review skipped)

- Remove every test count from every doc: root `CLAUDE.md` ("Key Metrics", "Test Status",
  "Run all 1703 tests", per-system counts in "Core Systems Complete"), `training.md`, the ten
  `crates/*/CLAUDE.md` "Testing" sections (keep only the `cargo test -p <crate>` command),
  `README.md` if present. The invariant "0 failed, 0 ignored" stays.
- Crate guides drop File Map lines that restate the `mod` tree; a line survives only if it says
  something `ls` and `grep mod` cannot.
- Each crate guide's "Common Pitfalls" becomes a "pitfall → guard test" table naming the test
  that guards it (the durable record of the keep-list; the audit reports stay transient in
  `review/`).
- Root `CLAUDE.md`/`training.md`: three managers (GameLoop/Render/Window), collision-bus
  sentence gone, `run_game` returns `EngineError`, Pause Pattern uses `request_exit()`, the
  SSOT table row "Behavior ↔ BehaviorData From pair" → "`ecs::Behavior` serde attributes",
  "arms in BOTH loader and serializer" → "a builder, an extractor, and a table row".
- `log_archive.md`: the physics pass-through changelog, the deleted `docs/plans` note, lessons.
- Issue bookkeeping: close #82, #89; tick items on #84, #86, #90, #91; file the deferred issues.

## Deliberately not doing (filed as issues in batch 10)

- `EditorGame` field regrouping beyond `ApiSession`/`SceneConfirm`; full `GameRunner`
  regrouping; `World`/`EditorContext` delegation shells; `Renderer` FrameGraph — speculative
  restructures with no failing behaviour.
- `PhysicsWorld` split (#85 SRP-001) — rapier needs the breadth; stays on #85.
- `EntityId` by-value vs by-ref convention — ripples into every game call site.
- `ComponentRegistry` `String` errors → typed `RegistryError` — its own small issue.
- `ui` depending on `input` (winit in the UI crate) — a design pass of its own.
- Sharing ONE `CameraBinding` across all three pipelines (saves two 64-byte uploads) — needs
  the layout to cross the crate boundary in `render_manager.rs`; follow-up to #89.
- `render_*` names that mutate — immediate-mode convention, keep.
- WGSL identifier renames and the `panic!` in the wgpu error callback — no headless check.
- Bool parameters → enums; `assets.rs`/`editable_inspector.rs` file renames — churn without a
  behaviour change.
- Merge policy moved from commands into `CommandHistory` — redesign, not cleanup.
- Root `CLAUDE.md` status-report trimming and the four in-repo agent-tool mirrors (#94).
- Settled and untouched: `ComponentStore` HashMap, cfg-split frame drivers, rodio,
  `editor_component_registry!`.

## Verification (end to end)

1. After every batch: the gates above, the games/wasm gates where triggered, the batch's named
   extra checks, kimi code review adjudicated, commit with `ADV_REVIEWED=1`.
2. After batch 3: the comment-tag grep returns nothing; `git diff --stat` for the batch shows no
   file gained code.
3. After batch 8: `hello_world.scene.ron` load → save is byte-identical to the checked-in
   golden; every `examples/assets/scenes/*.ron` and `../games/*/assets/scenes/*.ron` loads.
4. After batch 10: `grep -rn "[0-9]\+ tests\|passing\|passed:" CLAUDE.md training.md
   crates/*/CLAUDE.md` returns nothing; `gh issue list -R beinsiculous/insiculous_2d --label
   tech-debt` reflects the closes/ticks.
5. Manual (Jesse): `hello_world` after batch 4 (vertex layout, shape enum), `editor_demo`
   after batches 7 and 9 (theme tokens; play/pause/stop, undo merge, marquee, gizmo).
