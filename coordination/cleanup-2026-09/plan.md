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
- The comment-tag gate (batch 3) is part of EVERY batch's gate list. Widened Sep 3 2026 (review-18
  F5): bare `#42` counts too — `grep -riEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|#[0-9]{1,4}\b|Sprint [0-9]" crates src examples --include=*.rs`.
  Batch 3 ran with the narrower pattern and left 45 bare-`#N` matches; the planner sweeps them in
  a comment-only commit right after batch 4 lands (under the review threshold), and batch 4's
  executor is held to the pattern it was handed.
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

## Batch 3 — mechanical sweep: comments, names, allows — DONE Sep 3 2026 (4d6c715; review 17, fixes applied by Claude per Jesse)

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
  delete the `PATTERNS_AUDIT.md` reference in `renderer/src/sprite/instance_cache.rs:1`. The
  three stale hex values (audit-editor.md:442): `bg_primary` documented `#1e1e1e` but is
  `surface_1 = 0x2a2a2a`, `bg_input` says `#2d2d2d` but is `0x545454`, `bg_viewport` says
  `#000000` but is `0x0a0a0a` — the doc line follows the value, not the reverse.
- Evidence for the lists above: `audit-systems.md` §4–5, `audit-editor.md` §4–5, `audit-core.md`
  §3.5 and §4 (all in this directory). Out of scope, left as they are: the two `dead_code`
  allows in `ecs/benches/ecs_benchmark.rs` (a bench, not production code).
- Gate: this returns nothing afterwards:
  ```sh
  grep -riEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|Sprint [0-9]" crates src examples --include=*.rs
  ```
  (case-insensitive, no blanket exclusions — a match inside an attribute or hex literal is
  inspected by hand; the gate runs in every later batch too). Baseline Sep 3 2026, after
  batch 2: 276 matches across the workspace, heaviest in `editor_integration/src/editor_game/`
  and `editor/src/{hierarchy,commands,command_api}/`.

Names (production code; tight math in `common` stays; WGSL untouched — no headless check):
physics `bodies.rs` `he/f/imp/pos/vel` → `half_extents_meters/force_meters/impulse_meters/
position_meters/velocity_meters`, `components.rs:214` `hw/hh`, `update.rs:19` `dt` →
`clamped_delta_time`, `stepping.rs:56` `let event_handler = ();` deleted (`queries.rs` and its
`toi` went with `raycast` in batch 2);
ui `kb/pos/dx/bg/d`; renderer `img`; engine_core `mgr`, `scene_serializer.rs` single-letter
component bindings (`b` bound to two types), handlers `vel_x/vel`; editor `c/b/w/n/pc/btn`.

Allows: `behavior_runner/handlers.rs` handlers take `&Behavior` and destructure (three
`too_many_arguments` go, and no more swappable `f32` positional pairs); `WidgetId::from_str` →
`WidgetId::hashed` (19 sites after batch 2; drops the `should_implement_trait` allow);
`DrawList::slider` (`ui/src/draw/mod.rs:296`, seven params) takes a `SliderVisual` struct; the three editor allows
wait for batch 7; `renderer.rs:230` `arc_with_non_send_sync` stays (decided with H8).

## Batch 4 — DRY in the systems crates — DONE Sep 3 2026 (168dadd; review 19, fixes applied by Claude per Jesse)

- renderer: `camera_binding.rs` `CameraBinding { buffer, layout, bind_group }` composed by the
  sprite and line pipelines (closes #89); `pipeline_builder.rs` `build_render_pipeline(device,
  PipelineSpec)` used by all three pipelines; `TextureManager::insert_rgba` shared by the three
  loaders; `Sprite` stores `SpriteShape` (flattened only in `to_instance`); `sprite_data.rs`
  and `line_pipeline.rs` attributes via `wgpu::vertex_attr_array!` (offsets verified identical
  to the WGSL locations; add `offset_of!` tests); `PassScissor { Fullscreen, Rect, Empty }`
  replaces `Option<Option<..>>` (`bloom.rs:295` and the nested match at `line_pipeline.rs:219-224`;
  plan-sequence's `CompositeScissor` name is superseded); extract pure `bloom_dims(width, height)`
  (the `.max(1)` guard — it lives in `render_targets.rs:51-57`, not bloom.rs) and
  `DynamicBuffer::grown_capacity(current, needed)` (`sprite_data.rs:240-241`) so both get headless
  tests (renderer cut review, F4 and the skipped guard). Already done elsewhere, not batch-4 scope:
  design §H3's method deletions (batch 2), §I3 and §J2 (batch 1). `bloom.rs` is 583 lines, so
  `pipeline_builder.rs` is required, not optional.
- physics: `push_collision` helper in `stepping.rs` for the three `CollisionEvent` constructions
  (`:95`, `:121`, `:138`); `previous_collisions` reuses its allocation.
- audio: private `start_sink(output, source, base, bus, looping)` behind `manager/mod.rs:270` and
  `music.rs:68`; factor the effective volume into a pure `effective_volume(base, bus, master)` for
  the four products (`manager/mod.rs:281`, `music.rs:72`, `:176`, `:181`) so the multiplication is
  tested without a device (audio cut review, F3).
- common: `clamp_volume` used by the audio manager (its private one at `manager/mod.rs:23` goes)
  and the three `with_volume` builders in `ecs/audio_components.rs:64,183,223` (closes #82; #86 and
  #89 are backlog issues — tick DRY-004 and DRY-006 there, do not close them).
- input: fixture test FIRST (a bindings JSON written by today's `input_settings_io::save`, checked
  in under `crates/engine_core/tests/fixtures/` — the directory does not exist yet — with a test
  that loads it and compares every binding and pad; it must pass before the refactor starts and
  after it ends); then
  `InputMapping<A, S = InputSource>` backs `PlayerBindings` (`bind`/`unbind` return `bool` to
  feed the dirty flag); one `STANDARD_PAD_LAYOUT` const with `PlayerSource::on_pad(pad)` feeds
  both `with_default_bindings` and `bind_standard_pad_layout`.
- ui: `edit_field` core (focus/keys/commit/draw shell + `resolve_font`) in a new
  `context/edit_field.rs` (`text_input.rs` is 539 lines) under `float_input` and `text_input`
  (fixes the drift: `text_input.rs:347` reads `default_font()` directly while `float_input` goes
  through `field_font` at `:453`); delete the `typed_keys` array (`input_state.rs:164-181`, still
  there) and drive typed characters from the chronological `just_pressed_keys()` that batch 1
  shipped; `TextInputStyle` derives `Copy`. `player.rs` is 586 lines: if `STANDARD_PAD_LAYOUT`
  and `on_pad` push it over 600, they move to a `pad_layout.rs` sibling.

Extra verification: games test gate, wasm gate, and a native `hello_world` run for the vertex
layout and shape enum (GPU-only) — Jesse's check.

## Batch 5 — DRY in engine_core (~500 removed, ~300 added; §A-colours, §B, §C, §D, §E, §K) — DONE Sep 3 2026 (21b743d; review 20, fixes applied by Claude per Jesse; the six game repos each carry the twelve-site migration on a new `jesse` branch)

Outcome notes for later batches: the § B half-2 spike **failed** — ron's `UNWRAP_VARIANT_NEWTYPES` also unwraps `Option::Some`, so `physics: Some(PhysicsSettings(..))` stops parsing; the `grid_default!` macro fallback landed and `tests/grid_spike.rs` is the tripwire. The § E `build_context` is a `macro_rules!`, not a method: a `&mut self` builder returning `GameContext<'_>` borrows all of `*self`, so `self.game.update(&mut ctx)` cannot follow it — the design's "disjoint fields" claim was wrong. `#[must_use]` on `Scores::submit` put seven `let _ =` sites in the games; Jesse kept it (they mark where a new-record banner goes).

- Delete `behavior_data.rs` (240 lines, 10 `BehaviorData` uses outside it): `pub type BehaviorData =
  ecs::behavior::Behavior;` keeps every import path; `ecs::Behavior` already carries identical
  serde defaults. Guard, FIRST: a golden RON scene with every `Behavior` variant
  (`Behavior::default_for_variant(i)`) written by today's serializer, checked in under
  `crates/engine_core/tests/fixtures/`, loading to equal values and re-saving byte-identical after
  the change; plus parse both `examples/assets/scenes/*.ron` and round-trip every variant through
  RON in `ecs`. The `CLAUDE.md:54` SSOT row "Behavior ↔ BehaviorData" becomes "`ecs::Behavior`'s
  serde attributes".
- Grid, half 1: `GridMesh { config: GridBackdrop, origin, substeps, … }` with one constructor
  `from_config`; delete `new`/`new_square` (`grid_mesh.rs:90,100`), the ten `with_*` builders
  (`:139-174`) and `from_topology`'s private defaults (`:105-130`, dead in production);
  `apply_grid_tunables` becomes one assignment and `build.rs:49-54`'s builder chain goes. Half 2 (newtype wire variant via
  RON `UNWRAP_VARIANT_NEWTYPES`, deleting `grid_defaults.rs`) only if the compatibility spike
  test in `design-structure.md` §B passes (ron is 0.12; the choke points are `scene_loader.rs:94`
  and `scene_serializer.rs:333`); else a `grid_default!` macro.
- `save_store.rs` (196 lines) becomes `save_store/{mod,json_slot}.rs`: `MergeOnLoad` trait + `JsonSaveSlot<T>` + `SaveError` +
  `unix_seconds()` built on `common::clock` (both current call sites already are; `std::time`
  panics on wasm); `AchievementManager` and `Scores` hold a slot; `AchievementError`/
  `ScoresError` become aliases. `achievements/toast.rs` takes `ToastQueue`/`ToastStyle`/
  `draw` (today `achievements/mod.rs:84-160` and `draw_toasts` at `:326`). Test the slot protocol
  once with a tiny document.
- `GameContext`: `FrameRequests { exit, window_title, engine_ui_clip }` private, with
  `request_exit()`, `set_window_title()`, `window_title_requested()`, `clip_engine_ui()`,
  `into_outcome()`; `GameRunner::build_context` + `absorb` replace the two 18-field literals
  (`game.rs:460`, `app_handler.rs:213`). `chaos_mode`/`time_scale` stay fields. Games: twelve
  `ctx.exit_requested = true` → `ctx.request_exit()` (six `menu.rs`, six `gameplay/mod.rs`);
  editor_integration three sites (`menu_actions.rs:114`, `editor_game/mod.rs:445-447`, `:461`);
  docs in `pause.rs`, `menu_panel.rs`, `CLAUDE.md`, `training.md` Pause Pattern (four
  `exit_requested` mentions).
- `Localization { strings, base_font, fonts_by_path }` grouped out of `GameRunner` (today's
  fields `strings`, `base_font`, `locale_fonts` at `game.rs:264-270`; 33 fields → 28 with E).
- `update_follow_entity`/`update_follow_tagged` (`handlers.rs:209,235`) → `follow_target(Option<Vec2>, …)`;
  the nine hand-written colour tuple conversions (four in `scene_serializer.rs`, five in
  `scene_loader_components.rs`) → `.into()`; `sort_batch_refs` computes its key once; `WindowConfig:
  From<&GameConfig>` (the `AssetConfig` pattern at `assets.rs:99`); `run_game` narrows from
  `Result<(), Box<dyn Error>>` to `Result<(), EngineError>` (`EngineError` exists at `lib.rs:131`,
  used only by `init()`; games call `.unwrap()` on both `run_game` and `run_game_with_editor` —
  source-compatible); `#[must_use]` on `Scores::submit`, `SpriteAnimation::play`/
  `ensure_playing` (not on `unlock`: games discard it 34 times). `run_game` returning
  `EngineError` also breaks `examples/hello_world.rs:520` and `examples/behavior_demo.rs:102`,
  whose `main` returns `Result<(), Box<dyn Error>>` with `run_game` as the tail (`hello_world.rs:531`,
  `behavior_demo.rs:108`) — change both to `run_game(..)?; Ok(())` — and
  `run_game_with_editor`/`run_game_with_editor_opts` (`editor_game/mod.rs:522`), whose return type
  narrows to match; if the editor entry point has an error source `EngineError` cannot carry, say
  so in the report rather than widening `EngineError` ad hoc. Only the games are source-compatible.
- ALREADY DONE (landed with the engine_core test cut): `test_support.rs` behind the `test-support`
  feature, the self dev-dependency, `editor_integration` enabling it, the six copies deleted. Not
  batch-5 scope; `tests/common/mod.rs` stays for helpers only integration tests need.

Extra verification: scene round-trip suite + new behaviour fixture; games test gate (all use
`run_game`, twelve edited); wasm gate.

## Batch 6 — DRY in editor commands and the command API (~450 removed, ~250 added; §F, §G) — DONE Sep 3 2026 (b25eeb7; reviews 22 and 22-claude, fixes applied by Claude per Jesse's standing ruling; closes #90)

Corrected Sep 3 2026 against the tree after batch 5, then again after review 21 (kimi, seven
findings, all folded in): every line reference below was re-derived from the current files. The
design's `editor_game/tests.rs:518`, `panel_renderer/tests.rs`, `commands/name_tests.rs` and
`commands/tests.rs:117,122` no longer exist or moved. The design's "api.rs drift test" is
`api_tests.rs:69 test_api_create_archetypes_all_map_to_factories`, which drives every
`ARCHETYPES` entry through `create` end-to-end and stays as it is; the doc comments at
`parse.rs:9-10` and `api.rs:223-225` describe the label mapping this batch deletes. The three
remaining `#[allow(too_many_arguments)]` in the editor crate (`texture_field.rs:31`,
`stored_component/mod.rs:297,513`) are batch 7's, not this batch's.

- `SetComponentCommand<T>` (`commands/set_commands.rs`, design §F) replaces
  `impl_set_component_command!` (`set_commands.rs:73-118`) and its 13 expansions (`:120-158`); the
  13 names become type aliases, re-exported from `commands/mod.rs:19-24` together with
  `SetComponentCommand` and `GIZMO_FIELD_HINT`. `TransformGizmoCommand` (`set_commands.rs:14-63`)
  is deleted: the one production site `viewport_interaction.rs:441` and the two test sites
  `commands/tests.rs:257,259` become `SetTransformCommand::new(entity, old, new, GIZMO_FIELD_HINT)`;
  `commands/mod.rs:23` drops the re-export. Three asserts change from "Set Transform" to
  "Set Transform2D": `commands/tests.rs:45,50` and `component_editors/tests.rs:209`. The gizmo
  drag's undo label changes with it — the status bar reads "Undo: Set Transform2D" where it read
  "Undo: Transform Gizmo" — user-visible and cosmetic; no test pins it. The registry macro loses
  its command token: the `$cmd:ident` in `registry_edit_block!` (`stored_component/mod.rs:43,63`,
  used at `:57,81`) and the `=> Set…Command` half of each of the 13 `edit` entries (`:483-501`)
  go, the block constructs `SetComponentCommand::<$ty>::new(e, old, new, hint)` itself, and the
  Set* import block at `stored_component/mod.rs:22-26` shrinks to `CommandHistory,
  RemoveComponentCommand`. Unchanged through the aliases: `entity_ops.rs` (`SetSpriteCommand`, one
  site), `viewport_interaction.rs:450` (`SetColliderCommand` with `"gizmo_scale"`),
  `commands/dirty_tests.rs`, the rest of `commands/tests.rs` and `component_editors/tests.rs`.
  `display` is a `String` built once from `T::type_name()`. Add: distinct `T`s never merge on one
  entity + hint; the gizmo hint never merges with a field hint.
- `stored_component/component_ref.rs` (new — `mod.rs` is 584 lines):
  `ComponentRef { Typed(ComponentKind), Dynamic(String) }` with `display_name`, `add_default`,
  `capture`, `remove`, `cascade` (RigidBody → Collider, today inline at
  `component_commands.rs:88-92`; dynamic never cascades). `AddComponentCommand`/
  `RemoveComponentCommand` (`component_commands.rs:16-112`) hold a `ComponentRef`; `::new(entity,
  kind)` stays, `::dynamic(entity, name)` is added; `AddDynamicComponentCommand`/
  `RemoveDynamicComponentCommand` (`component_commands.rs:114-258`) are deleted with their
  re-exports at `commands/mod.rs:14-17`. Five sites: `panel_renderer/inspector.rs:269`,
  `command_api/write.rs:293,353`, `stored_component/mod.rs:338` (and the doc comment at `:511`),
  `stored_component/dynamic_tests.rs:9,87,97`. Display names unify to "Add {name}" /
  "Remove {name}" (no test asserts "Add Component" or "Remove Component").
- `crates/editor/src/physical_floors.rs` (new): the floors that `sanitize()`
  (`command_api/write.rs:168-207`) and the field editors (`component_editors.rs:147,174` scale;
  `:284,292,300,306,314,320` collider; `ranges::VOLUME`/`ranges::PITCH` at `:367,371`) both apply
  today — scale ≥ 0.01 (Transform2D and Sprite), collider half-extents and radius ≥ 0.5, capsule
  half-height ≥ 0.0, volume 0..=1, pitch ≥ 0.1 — become named constants plus `clamp_transform`/
  `clamp_sprite`/`clamp_collider`/`clamp_audio_source`; `sanitize` calls the `clamp_*`, the
  editors use the constants (or the clamp), so each number appears once. Deliberately unchanged:
  the inspector-only floors on damping, friction and restitution (`component_editors.rs:232,236,
  335,339`) stay inspector-only — widening `sanitize` to them is a behaviour change, not DRY.
  `docs/EDITOR_COMMAND_API.md` § Sanitation already describes the shared set; no doc edit.
- `write.rs`: `build_add_patch_set` (`write.rs:80-96`) and the `PureWrite::Set` arm of `run`
  (`:226-250`) share one body that builds the sanitized, texture-validated
  `SetComponentValueCommand` (Set keeps its Name refusal, its non-finite check and its "add it
  first" message). A `pub fn record_executed(history, batch, cmd)` in `write.rs` backs
  `WriteCtx::record` (`:52-57`) and replaces the copy at `api.rs:144-147`. `answer_api_lines`
  (`api.rs:42-58`) stops re-parsing queries: a `pub fn answer_query(&Query, &QueryCtx) -> String`
  (run + envelope) serves both `dispatch_line` (`mod.rs:186-199`, kept — the documented
  transport-agnostic entry with three test callers) and the query arm in `api.rs`.
- `crates/editor/src/archetype.rs` (new, design §G1): `Archetype` with `ALL`, `const fn kebab`,
  `from_kebab`, `const fn menu_label`, and its own test `from_kebab(kebab(a)) == Some(a)` over
  `ALL`. `ARCHETYPES` (`parse.rs:11-14`) derives from it and stays `pub` (`specs.rs:177`,
  `mod.rs:24`, `api_tests.rs:75,80` read it); its doc comment (`parse.rs:9-10`) stops claiming a
  drift test locks a mapping — the list derives from the enum, nothing can drift. The validation
  at `parse.rs:173-178` becomes `from_kebab`; `HostedWrite::Create { archetype: Archetype, .. }`
  (`mod.rs:117`; the response string at `api.rs:148` uses `kebab()`). `archetype_action`
  (`api.rs:223-239`, doc comment included) is deleted; `entity_ops::handle_create_action(&str, ..)
  -> Option<EntityId>` (`entity_ops.rs:136-159`) becomes `create_archetype(Archetype, ..) ->
  EntityId` (every variant spawns, so no `Option`); callers `api.rs:105` and `menu_actions.rs:52`,
  plus its own test `entity_ops.rs:254-291 test_world_factories_place_name_and_select_the_new_entity`,
  rewritten to iterate `Archetype::ALL` with the same name/placement/selection asserts and
  without the `"Create Nonsense"` → `None` case (an unknown archetype now dies at parse time).
  The nine Entity-menu items (`menu/mod.rs:222-236`) are generated from `Archetype::ALL`, the
  three separators kept by grouping. Closes #90 (ARCH-101).
- `EditorAction` (`editor_input.rs:25-114`, design §G2) gains `CreateEntity(Archetype)`, `Exit`,
  `TogglePanel(PanelId)`, `ResetLayout`, `CycleGameLocale` and `allowed_while_playing()`
  (`PanelId` is already `Copy + Eq + Hash`, `dock/mod.rs:18`); `set_default_bindings` binds none
  of them. `allowed_while_playing` is a deny list, exactly the variants today's
  `if !self.editor.is_playing()` guards (`menu_actions.rs:46-93`) block: `CreateEntity(_)`, `Cut`,
  `Copy`, `Paste`, `Delete`, `Duplicate`, `Undo`, `Redo`, `NewScene`, `OpenScene` return false;
  every other variant returns true — `Save`/`SaveAs` (the choke point still refuses with its
  "stop Play" message), `Exit`, the three toggles, `TogglePanel`, `ResetLayout`,
  `CycleGameLocale` all dispatch during Play exactly as the menu does today. A disallowed action
  is dropped silently, today's fall-through. `crates/editor/src/menu/actions.rs` (new, §G3):
  `action_for_menu_label(&str) -> Option<EditorAction>` beside `panel_id_for_menu_label`
  (`dock/mod.rs:44`), exported from `menu`. `handle_menu_bar` (`menu_actions.rs:35-133`) shrinks
  to render + one `dispatch_editor_action` call behind that guard; `dispatch_editor_action`
  (`shortcuts.rs:260`) becomes `pub(super)` and gains the five arms (bodies from
  `menu_actions.rs:114-132`, verbatim). The Undo/Redo arms (`shortcuts.rs:271-286`) gain the
  menu's "Undo: {name}" / "Redo: {name}" status message (`menu_actions.rs:72-87`); the
  `drag_guard` now also covers menu-driven Undo/Redo/Delete/Duplicate/Paste/Cut. Tests: every
  enabled, non-separator item of `MenuBar::editor_default()` maps through
  `action_for_menu_label`; the deny list is exactly the ten variants above; Ctrl+Z shows the
  status message (`shortcuts_tests.rs`); keep `test_every_default_chord_resolves_to_its_action`.
  `editor_input.rs` is 516 lines: if the enum growth brings it near 600, `Modifiers` (next
  bullet) goes to its own file instead.
- Small helpers (§G4 and the plan-sequence remainder):
  `CommandHistory::push_as_one(name, commands)` replaces `viewport_interaction.rs:460-472`
  (0/1/many), `menu_actions.rs:239-249` (Paste) and `:274-284` (Cut); `execute_as_one` replaces
  Delete's `menu_actions.rs:157-166` (`scene_io.rs:76-87`, the script-target rename macro, may use
  it too). Contract, in the doc comment and the test: none records nothing, ONE is pushed or
  executed raw — never wrapped — so its own display name stays the undo label
  (`shortcuts_tests.rs:146` pins "Delete Entity" for one, `:185` "Delete Entities" for many),
  many become a `MacroCommand` named `name`. `Modifiers { ctrl, shift }` +
  `Modifiers::read(&InputHandler)` in `editor_input.rs` replaces the five reads
  `shortcuts.rs:206-209`, `editor_input.rs:341-342`, `viewport_interaction.rs:108-109`,
  `viewport_interaction.rs:526-529` (`fn ctrl_held`, deleted; its caller at `:350` reads `.ctrl`)
  and `panel_renderer/mod.rs:179-182`; the `ctrl_held`/`shift_held` FIELDS of
  `ViewportInputResult` (`viewport_input.rs:67-69`) and the bool parameters of
  `apply_gizmo_drag`/`apply_click_selection` are results, not reads, and stay.
  `WidgetSlot { Field(n), Remove, AddButton, PopupRow(n) }` with `FieldId::slot(component_index,
  WidgetSlot)` in `field_style.rs` replaces `editable_inspector.rs:253` and
  `component_editors.rs:440` (`Remove`, today field 99), `panel_renderer/inspector.rs:188`
  (`AddButton`, today `component_index + 50`) and `:244,267` (`PopupRow(n)`, today
  `component_index + 60 + n`); every slot encodes inside the component's own
  `COMPONENT_ID_STRIDE` (a reserved field index per slot, the popup row in the subfield), so the
  add button and popup rows stop borrowing other components' id ranges. Test: for one component,
  `Field(0..n)`, `Remove`, `AddButton` and `PopupRow(0..m)` are pairwise distinct and none equals
  an id of component + 1 (batch 7 merges the two `remove_button` bodies; batch 6 changes only the
  id line). One `uncaptured_component_names`: `clipboard.rs:57-73` (subtree) and the inline copy
  in `WorldSnapshot::capture` (`world_snapshot.rs:81-96`, whole world) share one body over an
  entity list — the hierarchy-type exclusion is one rule spelled twice today; keep one spelling.
  `loss_warning`/`drop_report` (`world_snapshot.rs:136-158`) share one body that takes the full
  message template — the two messages differ in shape, not by a prefix; `world_snapshot/tests.rs:118,120`
  pin "lost on Stop" and "dropped 1". `scene_io.rs`: the two `for entity in world.entities() {
  world.remove_entity(&entity).ok(); }` loops (`:155-157` in `load_scene`, `:251-253` in
  `new_scene`) become `world.clear()` (`ecs::World::clear`, `world.rs:428` — entities and
  components only; resources survive, as today); `reset_session()` for the block repeated at
  `:179-186` and `:256-266` (dirty, history, api_batch, selection, gizmo_drag, gizmo.cancel —
  `new_scene`'s extra `entity_counter`/`physics_settings`/resource lines stay in `new_scene`);
  `SceneIoError { MidSimulation, CreateDirectory(std::io::Error), Write(String),
  Load(SceneLoadError) }` with a hand-written `Display` (editor_integration has no `thiserror`
  dependency and gets none) replaces `Result<(), String>` at `scene_io.rs:30,43,62,138`
  (`save_scene_to_file` returns `String`, `SceneLoader::load_from_file`/`instantiate` return
  `SceneLoadError`); callers format with `{e}` unchanged; `play_session_tests.rs:107` asserts on
  the message and becomes `err.to_string().contains("stop Play")`; `headless.rs:143` keeps its
  `String` error and maps with `{e}`.

Extra verification: `commands/{tests,dirty_tests,selection_restore_tests}.rs`,
`component_editors/tests.rs`, `command_api/{tests,write_tests}.rs`,
`stored_component/{tests,dynamic_tests}.rs`, `world_snapshot/tests.rs`, the `entity_ops.rs`
tests, `editor_game/{api_tests,shortcuts_tests,scene_io_tests,play_session_tests}.rs`; a
headless `--api` transcript against `../games/pong` exercising create, set (a scale of 0 must
come back 0.01), add, remove, undo and save to a path under `target/`, pasted into the report.

## Batch 7 — DRY in the editor UI (~400 removed, ~200 added; this section is the whole spec) — DONE Sep 3 2026 (367fc5f; reviews 24 and 24-claude, fixes applied by Claude per Jesse's standing ruling; the editor_demo visual pass done by Jesse Sep 3 2026)

Re-verified against the tree on Sep 3 2026 after batch 6 (`b25eeb7`); every line number below is
from that tree. No `design-structure.md` section covers this batch — the target shapes are here,
and where this section and `audit-editor.md` or `plan-sequence.md` disagree, this section wins
(the audit counted 82 plain writeback blocks and ten preambles; the tree has 98 `Changed` sites
and twelve field methods).

Scope: `crates/editor` and `crates/editor_integration` only. No public item of a systems crate
changes, so the games and wasm gates are not required — say so in the report rather than skip
silently, and run `scripts/check_games.sh` anyway if the diff strays into another crate root
(the games compile `editor_integration` under `--features editor`, through
`run_game_with_editor`, which this batch does not touch).

Items, in order of work; `cargo test -p editor` after each, `cargo test -p editor_integration`
after 7.3, 7.4, 7.6 and 7.7.

### 7.1 `EditResult::assign`

`crates/editor/src/field_style.rs` (`EditResult<T>` at :187) gains one method:

```rust
/// Write a changed value into `slot` and record `name` as the field hint;
/// an unchanged result leaves both alone.
pub fn assign(self, slot: &mut T, hint: &mut Option<&'static str>, name: &'static str)
```

It replaces every plain block of the shape `if let EditResult::Changed(v) = inspector.<kind>(…)
{ new.<field> = v; hint = Some("<field>"); }` in `component_editors.rs` (38 `Changed` sites),
`behavior_editor.rs` (28), `ui_component_editors.rs` (17) and `component_editors/grid_backdrop.rs`
(15) — about 80 of the 98. Blocks that transform the value before storing it (the scale floor at
`component_editors.rs:144-149`, `normalized_cols`/`clamp` at `grid_backdrop.rs:54-61`, every
`cycle` index → enum, `round() as u32/i32`), branch on it, or return early (`edit_name`, :93)
keep their `if let`. `script_editor.rs` and `composite_rows.rs` are out of scope: their `Changed`
arms build `ScriptValue`s and composite values, not slot-plus-hint. A method, not a macro. Test:
one contract test in `field_style.rs` — `Changed` writes the slot and the hint, and a later
`Unchanged` on another field leaves the first field's hint standing (the hint is "last changed
field wins"; an `Unchanged` that cleared it would break undo merging).

### 7.2 `EditableInspector` — `next_field`/`advance`, one remove button, borrowed style

`crates/editor/src/editable_inspector.rs` (530 lines):

- Twelve methods repeat `let layout = self.row(); … self.field_index += 1; self.current_y +=
  self.style.row_height;`, and ten of them also build `let id = FieldId::new(self.component_index,
  self.field_index, 0)` first — `texture` :273, `f32` :299, `f32_hard` :318, `angle` :332, `bool`
  :361, `vec2` :371, `action_button` :399, `string_edit` :417, `cycle` :432 (two ids, subfields 0
  and 1), `color` :490. `u32` (:381) and `string` (:389) are read-only displays with no widget id.
  Add private `fn next_field(&mut self) -> (FieldId, RowLayout)` (the subfield-0 id and the row
  layout; does not bump) and `fn advance(&mut self, height: f32)` (bumps `field_index`, adds
  `height` to `current_y`). Each id-bearing method becomes `let (id, layout) = self.next_field();
  … self.advance(self.style.row_height)`; `u32` and `string` keep `self.row()` and use only
  `advance` (an unused `id` binding would be a warning, and the gates deny warnings); `color`
  advances by its own block height; `cycle` derives its subfield-1 id with `FieldId::new(..,
  self.field_index, 1)` before advancing. Pin: the widget-id tests in
  `component_editors/tests.rs` and `fonts.rs` run unchanged.
- One remove button. `header_with_remove(type_name, removable: bool)` (:235) has exactly two
  callers, both passing `true` (`stored_component/mod.rs:96` and `:534`), and draws the same 18px
  [X] at `remove_button_x` that `component_editors::remove_button` (`component_editors.rs:430`)
  draws. Delete the bool and the duplicated drawing: `pub fn header_with_remove(&mut self,
  type_name: &str) -> bool` is `let header_y = self.current_y; self.header(type_name);
  remove_button(self.ui, self.component_index, self.x, header_y, self.width)`. `remove_button`
  stays `pub(crate)` in `component_editors.rs` and stays the one implementation — the registry's
  `@removable … edit` arm (`stored_component/mod.rs:75`) keeps calling it directly, because there
  the editor fn drew the header. (The audit's bool → enum item is moot: the parameter is gone.)
- Borrowed style. `EditableInspector<'a>` holds `style: &'a EditableFieldStyle`; `new(ui, style,
  x, y)` takes it; `with_style` (:208) is deleted — its callers are the three registry arms
  (`stored_component/mod.rs:50`, `:71`, `:95`), the dynamic block (`:533`), all
  `.with_style($field_style.clone())`, and the `fonts.rs:165` test. Every test that calls
  `EditableInspector::new` — `script_editor.rs:239`, `grid_backdrop.rs:146`,
  `ui_component_editors.rs:181`, `fonts.rs:165`, and the six fixtures in
  `component_editors/tests.rs` (:69, :77, :103, :115, :143, :173) — binds `let style =
  EditableFieldStyle::default();` outside the frame closure and passes `&style`. Not the `Copy`
  alternative: a by-value 25-field struct is the clone in another costume.

### 7.3 `InspectorFrame` — the last three `#[allow(clippy::too_many_arguments)]` go

- `texture_field.rs:31-41` `edit_texture_field` has eight parameters and its `_id: FieldId` is
  unused. Delete the parameter and its two arguments: `EditableInspector::texture` stops passing the
  id `next_field` hands it (the id has no consumer once the parameter is gone; `texture` keeps
  `next_field` for the layout and the field-index bump), and the test at `texture_field.rs:118`
  drops its `FieldId::new(0, 0, 0)`. Seven
  parameters; the allow goes.
- `stored_component/mod.rs:295` `edit_all_components` (11 parameters) and `:511`
  `render_dynamic_edit_blocks` (11). Add to `editable_inspector.rs`, re-exported from `lib.rs`
  beside `EditableInspector`:

  ```rust
  /// One inspector render pass: the UI context, the two styles and the
  /// content column every component block shares. The host builds it once
  /// per frame; the registry-generated editors thread it through.
  pub struct InspectorFrame<'a> {
      pub ui: &'a mut UIContext,
      pub inspect_style: &'a InspectorStyle,
      pub field_style: &'a EditableFieldStyle,
      /// Left edge of the content column.
      pub x: f32,
      /// Width of the content column.
      pub width: f32,
      /// Vertical gap before each component block.
      pub section_gap: f32,
  }
  ```

  `edit_all_components(frame: &mut InspectorFrame<'_>, world: &mut World, entity: EntityId,
  history: &mut CommandHistory, y: f32, extras: &mut InspectorExtras<'_>) -> (f32, usize)` and
  `render_dynamic_edit_blocks(frame: &mut InspectorFrame<'_>, world: &World, entity: EntityId,
  y: f32, component_index: &mut usize, removals: &mut Vec<String>) -> f32`. The
  `registry_edit_block!` arms (`:40-108`) take `$frame` in place of `$ui, $x, $width,
  $inspect_style, $field_style, $gap` and read its fields (`EditableInspector::new(frame.ui,
  frame.field_style, frame.x, $y)`; read `inspector.y()` before `apply_component_edit` runs, as
  today, so the `frame.ui` borrow has ended). The one host caller is
  `editor_integration/src/panel_renderer/inspector.rs:147`. `inspect_all_components` (:345,
  seven parameters, no allow) is untouched. Both allows deleted; no new one anywhere — the
  standing `arc_with_non_send_sync` in `renderer.rs:216` is the documented exception and is not
  in this batch. `stored_component/mod.rs` is at 582 lines: the macro-argument cut must leave it
  ≤ 600; if it does not, `render_dynamic_edit_blocks` moves to `stored_component/dynamic.rs`
  (batch 9 plans that move; doing it here is allowed only if the ceiling forces it, and the
  report says so).

### 7.4 `panel_renderer/add_component_popup.rs` — one walk renders and measures

`editor_integration/src/panel_renderer/inspector.rs:186-297` (the [+ Add Component] button and
the popup), the height helpers `dynamic_section_height` (:301) and `categorized_popup_height`
(:310), `popup_anchor_y` (:326) and its test move to a new `panel_renderer/add_component_popup.rs`:

```rust
/// One row of the popup, in draw order.
enum PopupRow { Heading(&'static str), Typed(ComponentKind), Game(String) }
/// The rows the popup shows for this entity: each category heading followed
/// by its addable kinds, then a "Game" heading and the addable dynamic
/// components. Built once per frame — `categorized_components()` allocates,
/// and this is its only call.
fn popup_rows(available: &[ComponentKind], available_dynamic: &[String]) -> Vec<PopupRow>
/// Height of the popup for these rows: top padding plus heading and button rows.
fn popup_height(rows: &[PopupRow]) -> f32
pub(super) fn render_add_component_section(
    editor: &mut EditorContext, ctx: &mut GameContext, entity_id: EntityId,
    command_history: &mut CommandHistory, content_x: f32, y: f32, component_index: usize,
) -> f32
```

The render loop walks `rows` once; `popup_height` walks the same Vec — no second
`categorized_components()` call, no second filter pass. Heading rows advance 18, button rows 24; the
height budgets 8 of vertical padding, of which the first row starts 4 below the popup top
(`popup_y = popup_y0 + 4.0` today, :222) and 4 stay below the last row — keep that split, it is
not "top padding 8"; `content_x + 8.0` is `layout::PADDING`. A category heading with no addable kind is omitted, as today, and the "Game"
heading appears only when a dynamic component is addable. `WidgetSlot::PopupRow(index)` stays one
counter across both sections, so every button keeps its id. Pin: `popup_rows` is pure — test
that an empty category yields no heading, that "Game" appears only with an addable dynamic
component, and that `popup_height` equals `8 + 18 × headings + 24 × buttons` for a fixture with
both kinds of row (compute the expected number from the row counts, never by calling the deleted
functions). `inspector.rs` shrinks to the component blocks and the warning plumbing.

### 7.5 Theme aliases removed

`crates/editor/src/theme/mod.rs`: delete `bg_primary` (:31), `bg_viewport` (:33), `bg_input`
(:35), `bg_header` (:38), `pause_yellow` (:112), `inspector_label` (:148), `inspector_value`
(:150), `inspector_header` (:152), their initialisers (:206-209, :261, :285-287) and their doc
comments. Every value is already a base token: `bg_primary = surface_1`, `bg_viewport =
surface_0`, `bg_input = surface_3`, `bg_header = surface_2`, `pause_yellow = warn_yellow =
0xffcc00`, `inspector_label = text_secondary = 0xcccccc`, `inspector_value = text_primary =
WHITE`, `inspector_header = accent_cyan = 0x00d9ff`. The call sites, all of them:
`dock/render.rs:99` (bg_primary → surface_1), `:109` and `:219` (bg_header → surface_2),
`menu/mod.rs:318` (bg_header → surface_2), `panel_renderer/asset_browser.rs:117` (bg_input →
surface_3), and inside `theme/mod.rs`: `inspector_style()` (:320-322) and
`editable_field_style()` (:330-336) read the three inspector tokens and `bg_input`; `ui_theme()`
reads `bg_input` (:353-355, :366, :373-374) and `bg_primary` (:358, :363). `bg_viewport` and
`pause_yellow` have no reader (dead tokens). `theme/tests.rs:19` drops the
`("bg_primary/bg_header", …)` ladder entry — it duplicates the `surface_1/2` entry above it.
Show the grep for each of the eight names across `crates src examples` in the report.
`docs/EDITOR_UX_AUDIT.md` names the old tokens as history and is left alone;
`crates/editor/CLAUDE.md:26` and `:82` describe the converters and stay true — check them.

### 7.6 Layout constants, `rect_border`, `render_node`, `draw_world_line`

- `layout::PADDING` (8.0) replaces `let padding = 8.0;` at `panel_renderer/mod.rs:19`, `:39`,
  `panel_renderer/inspector.rs:24`, `status_bar.rs:124`, the `PADDING` const at
  `panel_renderer/asset_browser.rs:25`, and the `padding: 8.0` defaults at `inspector.rs:31`
  (`InspectorStyle`) and `field_style.rs:144` (`EditableFieldStyle`). `layout::LINE_HEIGHT`
  (20.0) replaces `let line_height = 20.0;` at `panel_renderer/inspector.rs:23`, `:100`, `:117`
  and the `line_height: 20.0` default at `inspector.rs:32`. `layout` is already `pub mod` in
  `editor/src/lib.rs:76`. Nothing else in `layout.rs` changes; other `8.0`/`20.0` literals (menu
  dropdown padding, test geometry, ranges) are not paddings or line heights and stay.
- `panel_renderer/mod.rs:124-147`: the four `ctx.ui.line` calls become one
  `ctx.ui.rect_border(bounds, border_color, outline_width, 0.0)`
  (`UIContext::rect_border(bounds, color, width, corner_radius)`,
  `crates/ui/src/context/mod.rs:298`; the crate already uses it at `viewport_interaction.rs:197`).
  The visual pass confirms the play-state border still reads at both widths.
- `hierarchy/mod.rs:289` `render_node`: the off-screen early return (:300-311) repeats the child
  loop at :400-408. Restructure so the row drawing (:313-398) runs only when the row is on
  screen and ONE child loop follows: `if row_visible { …draw… } let mut next_y = y + ROW_HEIGHT;
  if self.is_expanded(entity) { for child in children … }`. `visible_order.push` stays
  unconditional (it feeds keyboard navigation and Shift range select). Pin: `hierarchy/tests.rs`
  runs unchanged. (Batch 9 splits the function into row and children; not here.)
- New `crates/editor/src/world_lines.rs`, re-exported from `lib.rs`:
  `pub fn draw_world_line(ui: &mut UIContext, viewport: &SceneViewport, start: Vec2, end: Vec2,
  color: Color, width: f32)` maps both ends through `viewport.world_to_screen`, skips the line
  when either end is non-finite (today only the grid guards this, `grid.rs:334-337`; the reason
  moves with the guard), and calls `ui.line`; `pub fn draw_world_segments(ui, viewport,
  segments: impl IntoIterator<Item = (Vec2, Vec2)>, color: Color, width: f32)` loops over it.
  `grid.rs:329-345` calls `draw_world_line` per segment (its colour and width vary per segment);
  `collider_overlay.rs:167-172` and `selection_outline.rs:138-153` (`draw_outline`) become
  `draw_world_segments` calls. The three `push_clip_rect`/`pop_clip_rect` pairs stay where they
  are — each renderer owns its clip. Pin: the draw-command tests in `grid.rs`,
  `collider_overlay.rs` and `selection_outline.rs` run unchanged; add one test in
  `world_lines.rs` that a non-finite endpoint emits no line draw command.

### 7.7 Private fields

- `dock/mod.rs:81` `DockPanel.bounds` → private. Written only by `DockArea::layout` (:270-313),
  read by `dock/render.rs` (a child module, so no accessor is needed) and by `content_bounds()`
  / `effective_size()`. Grep for readers outside `dock/`; add `pub fn bounds(&self) -> Rect`
  only if one exists.
- `viewport_input.rs:89` `ViewportInputHandler.config` → private. No reader or writer outside
  the file (`:389` is a same-file test). No setter.
- `grid.rs:118` `GridRenderer.config` → private. `set_grid_size` (:160, clamps ≥ 1.0) and
  `grid_size` exist; `render_grid_overlay` (:343) reads `config.width_for` in the same file. The
  one outside writer is the test `editor_integration/src/editor_game/viewport_interaction_tests.rs:148-160`
  (`test_zero_grid_size_never_poisons_positions`), which exists only to reach the guard in
  `context/mod.rs:297-306` `snap_to_grid_position`. With the field private the setter's clamp is
  the guarantee: delete the guard (`if grid_size <= 0.0 { return pos; }` and the doc sentence
  about the public field) and that test — the invariant stays pinned by
  `grid.rs:359` `test_grid_size_floors_at_one_pixel`. Say so in the report with the grep for
  `grid.config`.

### Deliberately not in this batch

`DockPanel.header_height` versus `HEADER_HEIGHT` (audit 3.9); the four-times-per-frame
`build_pickable_entities` (batch 9's `PickableCache`); splitting `edit_behavior`,
`render_inspector_editable` and `render_node` (batch 9); the `GridColors`/`GizmoPalette`
default duplication (not in the plan); `menu/mod.rs`'s own dropdown padding constant.

Gates: the standard set (test, clippy, tag grep, ≤ 600 lines, no `#[allow]`, no `unwrap()`
outside tests); per-crate tests per item as above; the games and wasm gates are not required —
assert it with `git diff --cached --name-only | grep -E "^crates/(ecs|physics|input|common|renderer|engine_core|audio)/"`
printing nothing. Guides in the same change: `crates/editor/CLAUDE.md` (file map gains
`world_lines.rs`; the `editable_inspector.rs` line names `InspectorFrame`; `:37` `layout.rs`
says what it now governs), `crates/editor_integration/CLAUDE.md:37` (`panel_renderer/` gains
`add_component_popup.rs`), root `CLAUDE.md` § SSOT "Inspector writeback / undo merge" row (names
`apply_component_edit` and the registry — still true; verify). Extra verification: an
`editor_demo` visual pass after the commit — Jesse's check, because a wrong theme token or a
border stroke on the wrong edge is invisible to tests.

## Batch 8 — SRP splits, engine side (~900 lines moved; §A; this section is the whole spec, re-verified against the tree Sep 3 2026) — DONE Sep 3 2026 (edde064 without 8.3 — reviews 26 and 26-claude, rebuttal-26, fixes applied by Claude per Jesse's standing ruling; 8.3 was reported done and had not landed, went back to gemini as `8-fixes-for-gemini.md` and landed as ef10209 — reviews 27 and 27-claude, rebuttal-27, no fix hunks; the native `editor_demo` run done by Jesse Sep 3 2026; the 8.2 hand-diff of the golden fixture waived by Jesse the same day — the fixture equals the pre-batch serializer's output, verified in a worktree, and no one outside the repo consumes saved scenes yet)

Re-verification notes, so the executor does not chase what earlier batches already did or
what the design got wrong:
- **Already shipped:** `Localization { strings, base_font, fonts_by_path }` (batch 5, §K —
  `game/locale_font.rs`); the colour tuple `.into()` conversions (batch 5); the
  `sprite/pipeline.rs` builder split (batch 4, §H — the file is 431 lines and
  `new_with_target` is 63; **no `sprite/pipeline/builders.rs` in this batch**); the
  exclusion-list drift guard (batch 1,
  `scene_serializer/dynamic_and_scripts_tests.rs:220`).
- **Design §A's builder signatures would not pass clippy.** Its `build_sprite` takes ten
  parameters; `too_many_arguments` fires at eight and no `#[allow]` is added. 8.3 below says
  what a builder is instead.
- **Design §A's table appends the physics rows last.** Today RigidBody and Collider are
  emitted between SpriteAnimation and UiLabel; appending them after Scripts reorders every
  saved scene and fails the golden. 8.4 places them.
- **`plan-sequence.md`'s "collect-dirty and propagate" split of the hierarchy system was
  wrong.** The walk is single-pass on purpose: a node's dirtiness is decided from its
  parent's result during the same descent. 8.6 splits it along its real seams.
- **The two examples are not "synced".** `hello_world.rs` has locale cycling, volume keys,
  the font search and a localised panel; `editor_demo.rs` has `on_play_stopped`,
  `add_editor_names`, `log::` instead of `println!` and a shorter panel. 8.8 says which wins.

Order of work is the numbering. 8.1 and 8.2 land before anything touches the loader or the
serializer, because the golden pins today's bytes and must be generated by today's code.

### 8.1 Stable root order

`world_to_scene_data` (`scene_serializer.rs:31`) sorts `roots` by `EntityId::value()` before
emitting — ids are assigned in file order at load, so load → save preserves order. Today
`get_root_entities` iterates a `HashMap` (`ecs/src/hierarchy_extension.rs:198`); leave it, the
editor's hierarchy panel already sorts its own copy (`editor/src/hierarchy/mod.rs:252`). The
existing `every_root_entity_is_saved_in_a_stable_order` (`scene_serializer/tests.rs:406`)
only checks containment and that two saves agree, which a `HashMap` passes; make it assert
the names come back as `["First", "Second", "Third"]`, in that order.

### 8.2 Golden scene fixture — generated BEFORE 8.3 and 8.4

New integration test `crates/engine_core/tests/hello_world_golden.rs` (model:
`tests/behavior_fixture.rs`): read `../../examples/assets/scenes/hello_world.scene.ron`
(the path form `tests/scene_loader_parse.rs:69` uses), `SceneLoader::parse` +
`SceneLoader::instantiate` into a fresh `World` through `test_support::StubResolver`, save with
`world_to_scene_data(&world, "Hello World", parsed.physics, &test_texture_path)` and
`serialize_to_ron`, and assert the string is byte-identical to the committed
`crates/engine_core/tests/fixtures/hello_world_saved.scene.ron`. Generate that fixture once,
with 8.1 landed and 8.3/8.4 not yet started, by a one-off run (a temporary
`std::fs::write` you delete, or a `--nocapture` print you paste) — the checked-in test never
regenerates it. Say in the report which commit state generated it.

Jesse hand-diffs the fixture against the source scene before the commit. The differences
that are expected and only these: prefab flattening (each entity's `prefab`/`overrides`
become its `components`: the prefab's order, an override replacing its base component in
place, a new one appended — `merge_components` at `scene_loader.rs:295`), the wood texture reference becoming `#white` (the stub resolves every
reference to the white texture and `test_texture_path` writes handle 0 as `#white`),
comments gone, every serde-defaulted field written out, `editor: None`, **`prefabs: {}`**
(prefab definitions do not survive a save in today's serializer — `world_to_scene_data`
writes an empty map at `scene_serializer.rs:42`; the instances are what is flattened, and
preserving the definitions is out of scope), a benign float reformat where one shows
(`timestep: 0.016666668` re-serialised by ron's shortest round-trip form), root order as in
the file. Anything else is a bug in 8.1 or in today's serializer and stops the batch.

### 8.3 Loader: one arm per variant, the logic in named functions

`scene_loader_components.rs` (438 lines; `add_component_to_entity` is 346 of them). The
target is not the design's ten-parameter builders. It is:

- Each `match` arm destructures its variant and is ≤ 20 lines of pure field copying into
  the component's struct literal, or a call. A variant whose component IS the struct literal
  (Transform2D, Sprite after `assets.resolve_texture(texture)?`, Camera, Tilemap,
  GridBackdrop, UiLabel, UiPanel, UiButton) has no builder function — there is nothing to
  build. Camera's `-1000.0`/`1000.0` become `CAMERA_NEAR`/`CAMERA_FAR` consts with a doc line.
- Logic moves out by name: `collider_shape_from_data(&ColliderShapeData) -> ColliderShape`
  (the four-arm map), `rigid_body_of_type(RigidBodyTypeData) -> RigidBody` (the three-arm
  map; the seven assignments stay in the arm), `build_scripts(refs, entity_id, world) ->
  ecs::Scripts` (the pending-target resource dance, keeping its comment),
  `insert_dynamic_component(world, entity_id, component_type, data) -> Result<(),
  SceneLoadError>` (the registry check and both error messages, keeping the fail-loud
  comment), and `warn_if_inert(&SpriteAnimation, entity_id)` for the old-format warning.
- **The physics pair gets `#[cfg]` on the match arms, not inside them.** Two arms per
  variant: `#[cfg(feature = "physics")] ComponentData::RigidBody { body_type, .. } => { … }`
  and `#[cfg(not(feature = "physics"))] ComponentData::RigidBody { .. } =>
  log::warn!("RigidBody component in scene but physics feature is disabled")`. The
  `let _ = (…)` unused-suppression tuples go with nothing to replace them. Verify both
  shapes compile: `cargo check -p engine_core` and `cargo check -p engine_core
  --no-default-features`.
- `component_type_name` stays an exhaustive `match` (a new variant still fails to compile
  there); its doc comment says the serializer's table in 8.4 is the next edit.
- If the file ends over 600 lines, split the physics arms' helpers into
  `scene_loader_components/physics.rs` and keep the rest in `mod.rs`; do not split
  otherwise.

### 8.4 Serializer: one extractor per variant, one table, one hand list

New `scene_serializer/components.rs` holds:

```rust
pub(super) type Extractor = fn(&World, EntityId, &dyn Fn(u32) -> String) -> Option<ComponentData>;

/// One row per concrete `ComponentData` variant. The only place that knows both
/// names of a component; `append_dynamic_components` skips every `registry_name`
/// here, so a component is never written twice.
pub(super) struct ConcreteComponent {
    /// Wire variant name — what `SceneLoader::component_type_name` returns ("Camera2D").
    pub wire_name: &'static str,
    /// ECS registry name — what `ComponentRegistry::persistent_names` returns ("Camera").
    pub registry_name: &'static str,
    pub extract: Extractor,
}

pub(super) fn concrete_components() -> Vec<ConcreteComponent> { … }

/// Registry names with no wire variant that are still never emitted as `Dynamic`:
/// Name lives on `EntityData.name`; GlobalTransform2D is computed.
const EXCLUDED_NON_WIRE: &[&str] = &["Name", "GlobalTransform2D"];

pub(super) fn extract_components(world, entity, texture_path_fn) -> Vec<ComponentData>
```

plus one `extract_<component>` per variant (Transform2D, Sprite, Camera, Tilemap,
GridBackdrop, SpriteAnimation — keeping its autoplay comment —, RigidBody, Collider, UiLabel,
UiPanel, UiButton, Behavior, EntityTag, Scripts — keeping its target-name comment), each a
`world.get::<T>(entity).map(|t| ComponentData::X { … })`. `extract_components` is
`rows.iter().filter_map(|row| (row.extract)(world, entity, texture_path_fn)).collect()`
followed by `append_dynamic_components(world, entity, &rows, &mut components)`, which skips a
name when `rows.iter().any(|row| row.registry_name == name)` or `EXCLUDED_NON_WIRE`
contains it. `CONCRETE_OR_EXCLUDED` is deleted (grep in the report).

**Row order is today's emission order, physics included:** Transform2D, Sprite, Camera2D/
Camera, Tilemap, GridBackdrop, SpriteAnimation, *RigidBody, Collider*, UiLabel, UiPanel,
UiButton, Behavior, EntityTag, Scripts. The physics rows cannot sit in a `const` array with
`#[cfg]` on elements, and the design's `rows.extend` after Scripts reorders every save; build
the `Vec` in three statements — the first six rows, `#[cfg(feature = "physics")]
rows.extend([…two rows…])`, the remaining six rows. The golden from 8.2 is what proves the
order held. `Dynamic` is not a row. The `Vec` is rebuilt per entity per save; saves are
user-initiated, so no `OnceLock` unless a reviewer asks.

`scene_serializer.rs` keeps `world_to_scene_data`, `entity_to_entity_data`,
`serialize_to_ron`, `save_scene_to_file` and the module declarations; the `//!` header says
the loader/serializer pairing is a builder-or-arm, an extractor and a table row. Do not touch
`SceneLoader::merge_components` (`scene_loader.rs:295`) — it compares wire names between
`ComponentData`s and is unaffected.

Tests (`scene_serializer/dynamic_and_scripts_tests.rs` and `tests.rs`):
1. **Rewrite `exclusion_list_drift_guard_saves_every_persistent_type_exactly_once`** to read
   the table instead of its own hand-written wire→registry map (that map is a third copy of
   the name split): count each emitted component under its registry name — a concrete
   variant maps through the row whose `wire_name` equals
   `SceneLoader::component_type_name(&component)`, a `Dynamic` through `component_type` —
   then assert (a) no `Dynamic` names any row's `registry_name`, (b) every row's
   `registry_name` is `registry.is_registered`, (c) every persistent registry name appears
   exactly once — `Name` counted once through `EntityData.name` being `Some`, never through
   `components`, as the test pre-seeds it today (`:243`). Keep the test's name.
2. **New:** on the same all-defaults entity, every row's `extract` returns `Some`, and
   `SceneLoader::component_type_name` of what it returned equals the row's `wire_name` — the
   table cannot pair an extractor with the wrong name. No `sample` field on the row (a
   test-only field is dead code in production and the design's `#[cfg(test)]` field would
   need a cfg'd constructor); the emitted component is the sample.
3. Every existing round-trip test runs unchanged; that plus the golden is the behavioural
   lock.

### 8.5 `GameRunner` frame tail: named phases

`game/render.rs` `render_frame` (`:44`, ~110 lines) → `collect_game_sprites(window_size)`
(main-camera sync, the `RenderContext` call, scissor forward, particle append),
`collect_ui_sprites(ui_commands)` and `submit_frame()` (the two sorts, the two ref lists,
the render call with its device-loss fail-stop comment). `render_frame` becomes the glyph
prepare — `self.glyph_textures.prepare(ui_commands, asset_manager)` stays there as the first
statement, because it consumes `ui_commands` and the game's `RenderContext` reads the
textures it produces — followed by the three calls. `sort_batch_refs` and `append_particle_sprites` stay.

`game/frame_tail.rs` `post_update` (`:15`, ~60 lines) → `step_simulations(delta_time)`
(particles, sprite animations, grid backdrops, `set_lines`; the time-scale reason moves to
its doc comment), `draw_scene_ui(window_size)` (clip push, `draw_ui_elements`, toasts and
tick, clip pop; the splice-order and clip reasons move to its doc comment) and
`apply_frame_requests(first_frame)` (window title, base-font capture, `apply_locale_font`).
`post_update` becomes the three calls. Pitfall comments survive; the narration of the next
line does not.

### 8.6 `TransformHierarchySystem::update` along its real seams

`ecs/src/hierarchy_system.rs:161` (107 lines) → `seed_roots(&mut self, world)` (the stack
refill, keeping its no-allocation comment), `visit(&mut self, world, frame: TraversalFrame)
-> usize` returning the number of cache entries it stamped or inserted (so `live_entries`
still adds up; an entry `propagate_without_local` removes counts zero, exactly today's
arithmetic at `:187-206`, or the `cache.len() > live_entries` prune stops firing and stale
baselines linger — nothing in `hierarchy_dirty.rs` exercises an entity losing its
`Transform2D`, so only the sentence holds it), which itself calls `propagate_without_local(world, entity, ancestor_dirty)`
(the no-`Transform2D` branch, keeping its comment) and `is_dirty(&mut self, entity, &local,
parent_id) -> bool` (the cache-stamp check — its comment about refreshing the stamp on EVERY
visit is an invariant and stays), and `prune_removed(&mut self, live_entries)`. `update` is
then: enabled check, frame bump, seed, `while let Some(frame) = self.stack.pop() { visited
+= 1; … }`, prune, counters. The `recomputed`/`visited` counters keep their exact meaning —
`ecs/tests/hierarchy_dirty.rs` asserts them and is the lock; run `cargo test -p ecs` after
this item alone.

### 8.7 `BloomPipeline::run` sequence-only

`renderer/src/bloom.rs:219` (70 lines, five numbered section headers): `write_params(queue,
config, is_srgb)` (step 1), `bind_groups_for(&mut self, device, queue, targets) ->
&CachedBindGroups` (step 2; rebuilds on a size change and returns the cached groups — the
`let Some(cached) … else { return }` goes because the method just built them), then `run` is
`write_params`, `bind_groups_for`, the extract pass, `blur_passes(encoder, cached, targets,
iterations)` (step 4 with its one-buffer-per-direction reason as the doc comment) and the
composite. The numbered headers become these names, per the comment policy. `bloom.rs` is
565 lines; if the split crosses 600, move `create_single_tex_layout`,
`create_composite_layout` and `build_fullscreen_pipeline` (`:415-525`) to
`bloom/layouts.rs` and say so in `crates/renderer/CLAUDE.md:55`.

### 8.8 One platformer, two mains

`examples/hello_world/platformer.rs` (new) holds `PlatformerGame` and everything both
examples define today: `EXAMPLES_DIR`, `PLAYER_SPAWN`, `DemoAction` and `demo_actions()`,
`GameState`, `PlayerState`/`PlayerGroup`/`player_group`, the struct, `new`, `player_entity`,
`reset_player`, `toggle_music`, the `Game` impl and `add_editor_names`. Both
`examples/hello_world.rs` and `examples/editor_demo.rs` become their file doc, `#[path =
"hello_world/platformer.rs"] mod platformer;`, and `main` (hello_world keeps its
800×600 config and `run_game`; editor_demo keeps its 1280×720 config and
`run_game_with_editor`; both initialise `env_logger` at `info` as editor_demo does today).
Cargo already names both examples explicitly (`Cargo.toml:66-73`) and ignores a directory
under `examples/` with no `main.rs`, so nothing in `Cargo.toml` changes.

**Which copy wins where they differ — hello_world's, plus editor_demo's two hooks.** The
shared game keeps the locale cycling (`L`), the volume keys, the bundled-then-system font
search with `font_loaded`, and the full localised panel (250 px tall: title, score, state,
volume slider, music and reset buttons, volume bar, toggle hint, font status) — that is the
demo game authors copy from. It also keeps `on_play_stopped` (clears the physics world so
Stop re-syncs from the restored ECS; a no-op under `run_game`, which never calls it) and
`add_editor_names`, called from `init` in both (it only adds a `Name` to named scene
entities and a `GlobalTransform2D` where one is missing). Every `println!`/`eprintln!`
becomes the matching `log::` level — the controls banner and the "add a WAV file" hints are
`info`, failures `warn`. Nothing else about behaviour changes: the same scene, the same
fallback level, the same sounds.

Phases, each ≤ 40 lines: `init` → `load_level` (scene load, physics config from the
scene's settings, `scene_instance`; on error `spawn_fallback_level`), `attach_player_state`
(the `HierarchicalStateMachine`), `load_audio` (jump sound, music), `load_font` (bundled,
then the system list), `log_ready` (the entity counts and the banner). `update` →
`handle_debug_actions` (jump sound on Action1, music, locale, volume up/down, reset, UI
toggle), `step_world` (behaviors, physics, hierarchy), `collect_pickups` (the
`EntityCollected` drain), `update_player_state` (velocity → state transition), `draw_panel`
(the `show_ui` block).

Docs: root `CLAUDE.md:317` ("synced with hello_world.rs" → shares
`examples/hello_world/platformer.rs`), `training.md:94`, `README.md:479`.

### 8.9 Docs the batch forces

`crates/engine_core/CLAUDE.md:40` (render.rs names its three phases), `:42` (frame_tail's
three), `:86-87` (the "arms in BOTH" sentence becomes: a new component type needs a match
arm in `scene_loader_components.rs`, an extractor and a table row in
`scene_serializer/components.rs`, and the drift test proves the row), root `CLAUDE.md:50`
(the SSOT row "World → RON save" names the table in `scene_serializer/components.rs`),
`.claude/skills/add-component/SKILL.md:42` **and its twin**
`.junie/skills/add-component/SKILL.md:42` (step 3 names `extract_components()` — both
copies, same wording), `crates/ecs/CLAUDE.md:41` (verify it still holds; it should),
`crates/renderer/CLAUDE.md:55` only if 8.7 split the file. Grep every deleted or renamed
name (`CONCRETE_OR_EXCLUDED`, `render_frame`, `post_update`, `HelloWorld`, `PlatformerGame`'s
old home) across the living guides — every `CLAUDE.md`, `README.md`, `training.md`, the
`.claude/` and `.junie/` skills — not `review/`, `coordination/`, `docs/EDITOR_UX_AUDIT.md`
or `log_archive.md` (history stays as written; `log_archive.md:201` names `render_frame`).

### Deliberately not in this batch

`EditorGame::update`'s phases and the editor-side splits (batch 9); `GameRunner`'s remaining
field clusters (filed, §K); any change to `ComponentData` itself or to
`SceneLoader::merge_components`; `get_root_entities`' iteration order in `ecs`; a
`OnceLock` for the table; the `examples/behavior_demo.rs` shape.

Extra verification: the golden test (8.2, generated before 8.3/8.4, hand-diffed by Jesse
against the source scene before check-in so the batch's own bug cannot be blessed); both
physics shapes of `engine_core` compile; `cargo clippy --example editor_demo --features
editor -- -D warnings` (the `--all-targets` gate skips it — `required-features`); games test
gate `scripts/check_games.sh --test` (breakout's level tests load scenes through the loader);
wasm gate (engine_core, ecs and renderer change); a native `hello_world` and `editor_demo`
run — Jesse's check, after the commit.

## Batch 9 — SRP splits, editor side (~1000 lines moved; this section is the whole spec, re-verified against the tree Sep 3 2026) — DONE Sep 3 2026 (f66920d; reviews 29 and 29-claude, rebuttal-29, fixes applied by Claude per Jesse's standing ruling; the section itself was reviewed by kimi and gemini as review-28 and review-28-gemini before the handoff; the editor_demo manual pass — play/pause/stop, undo merge, marquee, gizmo with Escape, the --api pipe with a batch across Play — is Jesse's, after the commit)

Re-verification notes, so the executor does not chase what earlier batches already did or
what the audit and `plan-sequence.md` § Batch 9 got wrong:
- **`update()` has no numbered phases any more.** Batch 3 swept them; the method is 100
  lines of prose-commented calls (`editor_game/mod.rs:348-462`) and already delegates to
  `editor_time_scale`, `render_scene_confirm_dialog`, `handle_menu_bar`,
  `render_toolbar_and_play_controls`, `drain_api_requests`, `render_panels`,
  `handle_viewport_picking`, `handle_gizmo`, `update_inner_game`, `render_status_bar` and
  `pending_title_update`. What 9.1 names is what is still inline.
- **`ComponentKind` cannot leave `stored_component/mod.rs`.** It is generated by
  `editor_component_registry!` from the `removable:` list (`mod.rs:459`); a `kind.rs` that
  "takes `ComponentKind`" was never possible. 9.11 moves what is movable.
- **The asset browser has no folder tree.** `plan-sequence.md`'s `render_folder_tree` /
  `render_file_grid` name a panel that does not exist; the panel is a header, a bounded
  thumbnail loader, a scrolled tile grid and click-to-assign (`asset_browser.rs:45-205`). 9.10
  splits at those seams.
- **Already shipped:** the add-component popup left the inspector (batch 7,
  `panel_renderer/add_component_popup.rs`) — `render_inspector_editable` is 91 lines, not the
  audit's 182; `EditResult::assign` (batch 7) shrank `edit_behavior` to 109 lines, not 187;
  `render_node`'s duplicated child loop (audit 1.18) is gone, one loop remains at
  `hierarchy/mod.rs:390-401`; the menu bar already routes through `dispatch_editor_action`
  (batch 6).
- **The struct fields are `api_rx`/`api_batch` and `pending_scene_action`/
  `pending_dialog_choice`** (`mod.rs:77-91`), not the design's `rx`/`pending_action`/
  `pending_choice`. Design §K said to defer the grouping; this plan (settled with Jesse,
  Sep 2) does it, and 9.2 keeps it to a rename.
- **`GameContext` is not headless** (`AssetManager` needs a wgpu device), but `UIContext::new()`
  and `InputHandler::new()` are, and `input.mouse_mut().update_position` /
  `handle_button_press` / `handle_button_release` and `keyboard_mut().handle_key_press` drive
  them — the editor crate's `test_support.rs:66-99` and `component_editors/tests.rs:47-52`
  are the models. That is what makes the 9.6 guards writable.

Order of work is the numbering. 9.1 and 9.2 are the same file and go first; 9.5 (the
pickables) changes the signature 9.6 narrows, so 9.5 precedes 9.6.

### 9.1 `EditorGame::update` — named phases, one dirty sync

`update` becomes eleven calls, ≤ 25 lines, in today's order (the phase names avoid
`begin_frame`/`end_frame`, which are `UIContext`'s). **The snippet is the end state after
9.6.** 9.1 lands it with today's `render_panels(ctx)` and `handle_viewport_picking(ctx)`
calls and no `pickables` line; 9.5 adds that line and the two arguments, 9.6 narrows the
picking call. Nothing from 9.5 or 9.6 rides in 9.1:

```rust
fn update(&mut self, ctx: &mut GameContext) {
    let window_size = ctx.window_size;
    self.prepare_frame(ctx);
    self.render_early_overlays(ctx);
    self.handle_menu_bar(ctx, window_size);
    self.render_toolbar_and_play_controls(ctx);
    self.drain_api_requests(ctx);
    let pickables = build_pickable_entities(ctx.world);   // 9.5 says why here
    let content_areas = self.render_panels(ctx, &pickables);
    self.handle_viewport_picking(ctx.ui, ctx.input, ctx.world, &pickables);
    self.handle_gizmo(ctx, &content_areas);
    self.update_inner_game(ctx);
    self.finish_frame(ctx);
}
```

- `prepare_frame(ctx)`: the time freeze (`:354`), the editor-font re-assert (`:358`), the
  viewport interpolation (`:367`), `note_selection` (`:377`), the transform system (`:380`),
  the play-camera sync (`:383`), `update_layout` (`:386`). The comments that carry reasons
  (why the freeze runs before the inner game; why the font is re-asserted every frame; why the
  selection is noted before any handler; why interpolation runs before the camera sync) move
  onto the method's doc comment or stay as inline pitfalls; the narration goes.
- `render_early_overlays(ctx)`: the confirm dialog (`:392`) then the drag ghost
  (`:397-402`), with the ordering reason (the modal's scrim must land before the ghost can arm
  a gesture) as the doc comment.
- `finish_frame(ctx)`: `sync_dirty_mirror()`, `render_status_bar`, the window-title publish
  (`:443-449`, with its Playing-owns-the-title reason) and the engine-UI clip (`:451-461`,
  with its layer-poisoning reason).
- **One derivation of the dirty mirror.** Today the mirror is written at `:372` (frame
  start) and `:433` (before the status bar) in `update`, and set to `false` by hand at
  `scene_io.rs:147` and `:222`. `pub(super) fn sync_dirty_mirror(&mut self)` becomes the one
  place the flag is computed — `self.editor.set_dirty(self.command_history.is_dirty())`,
  doc-commented as such; it takes nothing, which is narrower than the "world-only entry
  point" the summary asked for. It is called from three places: `finish_frame`, and the two
  `scene_io.rs` sites in place of their literal `false` (`save_scene_with` after
  `mark_saved`, `reset_session` after the fresh history) — those stay because
  `scene_io_tests.rs:81`, `:175` and `:215` pin that a save, a load and a new scene read
  clean at once, without a frame; the comment at `:145-146` stays true. The frame-start
  write at `:372` goes: its only readers between the two syncs are `scene_io.rs:172` and
  `:278` (`self.editor.is_dirty()` in `load_scene` and `new_scene`), which are `EditorGame`
  methods and read the source of truth instead — `self.command_history.is_dirty()`, the
  same read `scene_confirm.rs:48` already makes. After this the only reader of
  `EditorContext::is_dirty` outside the tests is `title_bar_text` (`context/mod.rs:414`),
  rendered after the sync. Test edits: `test_support.rs:57`'s `set_dirty(true)` becomes
  `sync_dirty_mirror()` (the fixture has just recorded a command) and the "unreachable
  headless until batch 9's seam" comment above it is deleted — it names a batch, and the
  seam exists; `tests.rs:100`'s `set_dirty(true)` sits on an EMPTY history, so a sync there
  would read clean — that test records one command first (the `CreateEntityCommand`
  shape `dirty_editor` uses), then syncs, and its existing title assertions become the
  mirror's pin. `scene_io_tests.rs:43` and `:203` keep their `set_dirty(true)` — they
  simulate a mirror the fixture never derived, and rewriting them is not this batch's.

No separate mirror test: `test_pending_title_update_only_on_change`, re-seated on a recorded
command, is the pin (the deleted tests reconstructed the mirror in the test body; this one
drives the seam).

`mod.rs` is 566 lines. The phases move code within the file; if the result crosses 600,
`load_preferences`/`save_preferences` (`:246-276`) move to `editor_game/preferences.rs` and
the module header lists it.

### 9.2 `ApiSession` and `SceneConfirm`

Two structs, each in the file that owns the behaviour, replacing four `EditorGame` fields:

```rust
// editor_game/api.rs
/// The command-API transport state: `None` receiver = API not enabled.
#[derive(Default)]
pub(super) struct ApiSession {
    pub receiver: Option<std::sync::mpsc::Receiver<String>>,
    pub batch: Option<editor::command_api::write::ApiBatch>,
}
// editor_game/scene_confirm.rs
/// The unsaved-changes dialog's state machine; a `Some` action blocks the frame's input.
#[derive(Default)]
pub(super) struct SceneConfirm {
    pub pending_action: Option<PendingSceneAction>,
    pub pending_choice: Option<editor::ConfirmChoice>,
}
```

Fields `api: ApiSession` and `scene_confirm: SceneConfirm` on `EditorGame`; the field doc
comments (`mod.rs:74-91`) move onto the struct fields. Every site is a rename:
`api_rx` → `api.receiver` (`mod.rs:77`, `:112`, `:544`; `api.rs:186`), `api_batch` →
`api.batch` (`mod.rs:91`, `:116`; `api.rs:64`, `:100`, `:138`, `:147`; `shortcuts.rs:48`,
`:116`; `scene_io.rs:226`), `pending_scene_action` → `scene_confirm.pending_action`
(`mod.rs:80`, `:113`; `scene_confirm.rs:49`, `:59`, `:77`, `:87`, `:91`, `:116`, `:121`;
`shortcuts.rs:39`; `scene_confirm_tests.rs:17`, `:23`, `:28`, `:33`, `:51`, `:62`), `pending_dialog_choice` → `scene_confirm.pending_choice` (`mod.rs:83`,
`:114`; `scene_confirm.rs:69`, `:122`, `:126`; `scene_confirm_tests.rs:52`, `:57`, `:63`);
`api_batch` also at `api_tests.rs:177` and `:191`. The tests compare the fields, so neither
struct needs `PartialEq`. `EditorRunOptions.api_rx` (`mod.rs:530`) is
public and keeps its name; `run_game_with_editor_opts` assigns it into `api.receiver`.
Nineteen fields become seventeen; nothing else changes.

### 9.3 `command_api/write` — one function per verb

`write.rs` (476 lines; `run` is `:215-476`, 262 of them) becomes a directory: `git mv` it to
`write/mod.rs`, which keeps `ApiBatch`, `WriteCtx`, `record_executed`, the patch helpers
(`reject_non_finite` through `sanitize`, `:69-212`) and `run`; the verb bodies move to
`write/verbs.rs` as `pub(super)` functions, each `fn <verb>(ctx: &mut WriteCtx<'_>, …) ->
Result<Value, ApiError>`: `set(ctx, entity, component, patch)`, `add(ctx, entity, component,
value)`, `remove(ctx, entity, component)`, `rename(ctx, entity, name)`, `delete(ctx,
entity)`, `select(ctx, entity)`, `undo(ctx)`, `redo(ctx)`, `batch_begin(ctx, name)`,
`batch_end(ctx)`, `batch_abort(ctx)`. `run` keeps the Playing refusal and the
note-selection rule (`:216-227`) and becomes a match of one-line calls. Two four-line
copies inside `add` and `remove` (`:257-260` and `:325-328`, the typed-kind lookup) become
`typed_kind(component: &str) -> Option<ComponentKind>` in `verbs.rs`; the selection-restore
dance shared by `undo` and `redo` (`:417-420`, `:432-435`) becomes `restore_selection(ctx)`.
Every error string and every response payload is byte-identical — `write_tests.rs` (16
tests) is the lock and does not change. `command_api/mod.rs:23` (`pub mod write;`) is
unchanged; `write_tests.rs` stays where it is.

### 9.4 `command_api/parse.rs` — one parser per verb

`parse_line` (`:108-262`, 155 lines) keeps the set/add peel (`:111-141`, calling
`parse_set(line)` and `parse_add(line)`), the tokenizer call, the verb match, and the
trailing-token check. A verb whose arm is one expression today (`list`, `selection`,
`scene`, `commands`, `undo`, `redo`, `save`) stays an arm. Every arm with argument
validation becomes `fn parse_<verb>(tokens: &mut impl Iterator<Item = String>) ->
Result<Request, ApiError>`: `parse_describe`, `parse_remove`, `parse_rename`,
`parse_create`, `parse_delete`, `parse_select`, `parse_batch`. Inside `parse_create`, the
trailing `x y` detection (`:187-202`) becomes `split_trailing_position(&mut Vec<String>) ->
Option<(f32, f32)>` with its "a trailing numeric pair is a position" reason as the doc
comment. `create`, `set` and `add` return before the trailing-token check today and still do.
Every usage string is byte-identical; `command_api/tests.rs` and `write_tests.rs` (the
parse cases at `write_tests.rs:399`) are the lock.

### 9.5 Pickables built once per frame

Today `build_pickable_entities` runs up to four times a frame: the scene view's outline and
hover (`panel_renderer/mod.rs:98`), framing (`viewport_interaction.rs:116`), the click pick
(`:133`), the marquee (`:216`) and the texture drop (`:251`). It runs once, in `update`, only while
not Playing (`Vec::new()` otherwise — today no Play frame builds one, and the scene view and
picking both skip it while Playing), **after `drain_api_requests` and before `render_panels`** — the invariant is: after the
last handler that can delete an entity this frame (the menu bar and the command API run
before; the panels and the viewport delete nothing), and before the first consumer. A list
built earlier could hand a click the id of an entity the menu just deleted, and
`Selection::select` does not check the world. The doc comment on the local says so. The build point does not change what framing,
picking or the marquee see within a frame: a pickable's position is its `GlobalTransform2D`,
which only `TransformHierarchySystem` writes, in `prepare_frame`, before either the old or
the new build point (an inspector scrub writes `Transform2D`, visible to the pickables one
frame later, today and after). No `PickableCache` type: a `&[PickableEntity]` is the whole contract, and a cache that lived on
the struct could outlive its frame.

Signatures: `render_panel_content(editor, ctx, panel_id, bounds, command_history,
pickables: &[PickableEntity])` and `render_scene_view(editor, ctx, bounds, pickables)`;
`handle_viewport_picking(&mut self, ui: &mut UIContext, input: &InputHandler, world: &mut
World, pickables: &[PickableEntity])` (9.6 narrows the context in the same edit);
`handle_shared_viewport_input` the same four; `apply_marquee_selection(&mut self, pickables,
start, end, shift_held, ctrl_held)` — its `world` parameter existed only to build the list
and goes; `handle_viewport_texture_drop(&mut self, world, pickables, handle, path,
drop_pos)`. `viewport_interaction_tests.rs:327-372` (`marquee_rig` and the marquee
test) build their list with `build_pickable_entities(&world)` and pass it.

`handle_shared_viewport_input` (`:76-170`, 95 lines) then splits at its seams:
`handle_framing(&mut self, input_result: &ViewportInputResult, keyboard_owned: bool,
ctrl_held: bool, pickables)` (`:108-129`, keeping the Ctrl-skips-framing reason),
`handle_click_pick(&mut self, input_result, pickables)` (`:131-150`) and
`handle_marquee(&mut self, ui, input_result, pickables)` (`:156-169`: the live draw
and the released apply). The chrome gate and the follow-break stay in the caller.

### 9.6 Test seams — narrow the four `GameContext` takers, write the guards

- **`drain_api_requests`** (`api.rs:178-212`): the guards are the mid-drag skip and the
  256-line cap; neither needs assets. Split `pub(super) fn take_api_lines(&mut self) ->
  Vec<String>` (the `gizmo_has_priority` return, the cap, the `try_recv` loop, `:179-196`)
  from the glue that stays: `take_api_lines` → `answer_api_lines` → `note_selection` →
  stdout. The glue still takes `ctx` because the texture resolver reads `ctx.assets`; that
  is why this one is a split and not a signature change. `MAX_LINES_PER_FRAME` becomes a
  module `const` so the test can name it.
  Tests (`api_tests.rs`): `test_api_drain_takes_at_most_the_frame_cap_and_leaves_the_rest_queued`
  (send cap + 10 lines through an `mpsc` channel installed on `api.receiver`; the first
  take returns cap lines, the second the remaining ten) and
  `test_api_drain_leaves_the_channel_untouched_while_a_gizmo_drag_is_live` (make the gizmo
  active the way `gizmo/tests.rs:14-21` does — a press frame on the center handle through
  `editor.gizmo.render` — then take: empty, and the channel still holds every line).
- **`handle_editor_key`** (`shortcuts.rs:191-247`): the routing decision separates from
  the acting. `pub(super) enum KeyRoute { Consumed, PlayControl(PlayControlAction),
  ForwardToGame, Editor { action: EditorAction, shift: bool } }` and `pub(super) fn
  route_editor_key(&mut self, key: KeyCode, keyboard_owned: bool, modifiers:
  editor::Modifiers) -> KeyRoute` (`&mut self` because `confirm_dialog_consumes_key` clears
  the pending action on Escape). The order is today's: dialog, then `keyboard_owned`, then
  the always-intercepted play controls (`StopPlay` → `Stop`; `TogglePlayPause` → `Pause` if
  playing else `Play`; `ToggleCameraFollow` → `ToggleCameraFollow` only inside a play
  session, otherwise `Consumed`), then Playing → `ForwardToGame`, then the resolved action
  or `ForwardToGame`. `handle_editor_key(key, ctx)` becomes: read `ctx.ui.wants_keyboard()`
  and `Modifiers::read(ctx.input)`, route, then a four-arm match that acts (the `Stop` arm
  keeps its `on_play_stopped` call). The doc comment at `:186-190` and the two comments opening the body move
  to the router.
  Test (`shortcuts_tests.rs`): `test_key_routing_respects_text_focus_play_state_and_the_dialog`
  — Delete with `keyboard_owned` → `Consumed`; Delete while Editing → `Editor { Delete }`;
  Delete while Playing → `ForwardToGame`; Ctrl+Shift+P while Playing → `PlayControl(Stop)`;
  with a pending confirm action, Escape → `Consumed` and the action cleared. Bindings come
  from `EditorInputMapping::new()` (`editor_input.rs:261`; the Delete and play chords at
  `:302-303` and `:329-330`).
- **`handle_viewport_picking`** — narrowed in 9.5. Test
  (`viewport_interaction_tests.rs`): `test_viewport_click_selects_while_editing_and_nothing_while_playing`
  — a sprite entity at the origin (the `test_pickables_need_sprite…` fixture at `:263`), a
  `UIContext` and an `InputHandler` driven through a press frame and a release frame at the
  sprite's screen position (`editor.viewport` set to an 800×600 panel; `ui.begin_frame` /
  `end_frame` around each call so `chrome_owns_mouse` reads a real frame); while Editing the
  release frame selects the entity; the same two frames while Playing leave the selection
  empty. The failure this pins: picking against a live simulation.
- **`render_inspector`** (`panel_renderer/inspector.rs:18-90`): `render_inspector(editor,
  ui: &mut UIContext, world: &mut World, texture_path: &dyn Fn(u32) -> Option<String>,
  bounds, command_history)`; the resolver replaces the `ctx.assets.texture_path` read at
  `:126-129` (the same shape `answer_api_lines` takes). `render_panel_content` builds the
  closure from `ctx.assets` at the call. `render_inspector_editable` splits at
  `build_inspector_extras(editor, world, entity, texture_path) -> InspectorExtras` (`:126-134`)
  and `warn_after_edit(editor, ui, world, command_history, entity, name_before, warnings)`
  (`:167-187`: the `take_edit_commit` seal — which is why it takes the history — the
  name-ambiguity check, the one joined status message); the function itself is the frame,
  the `edit_all_components` call and the popup call. The popup narrows with it:
  `render_add_component_section(editor, ui, world, command_history, entity_id, origin: Vec2,
  component_index)` (seven parameters; `origin` is today's `content_x`/`y` pair), and its
  `ctx.window_size.y` read (`add_component_popup.rs:116`) becomes `ui.window_size().y`
  (`ui/src/context/mod.rs:160`, the same value the engine passes to `begin_frame`).
  Test (inline `#[cfg(test)]` in `inspector.rs`, the crate's convention for panel files):
  `test_inspector_offers_add_component_while_editing_and_not_while_playing` — a
  `Transform2D` entity selected as primary, one `ui` frame per state; the draw list
  (`ui.draw_list().commands()` after `end_frame`) carries a text command whose `text` is
  `"+ Add Component"` while Editing (`DrawCommand::TextPlaceholder` — no font is loaded
  headless; match `Text { data, .. }` too) and none while Playing. A Playing inspector that
  offers Add Component would mutate a world the Stop restore is about to discard.
- **`sync_dirty_mirror`** — 9.1.

`editor_integration/src/editor_game/test_support.rs` gains the three input helpers these
tests need (`press_mouse`, `release_mouse`, `ui_frame`), ≤ 30 lines, header naming
`crates/editor/src/test_support.rs:66-99` as the twin they copy (the third copy after
`ui` and `editor`; each is `#[cfg(test)]` and crate-private, which is why it is a copy).

### 9.7 `shortcuts.rs` — play transitions to `play_session.rs`, dispatch by category

`handle_play_action` (`shortcuts.rs:21-175`, 155 lines) moves to a new
`editor_game/play_session.rs` (`play_session_tests.rs` is already its sibling) and splits:
`start_play_session(&mut self, world)` (`:31-93`: the drag/confirm/merge resets, then
`commit_open_api_batch()` (`:44-60`, with its "already applied to the world the snapshot
captures" reason), the snapshot capture with its loss warning, `adopt_game_camera(world)`
(`:70-87`: save the editing pose, arm follow, adopt the main-camera pose or zoom 1.0), the
state change, popup close, `UiElementsHidden` removal), `resume_from_pause()` (`:94-98`),
`pause()` (`:102-108`), `stop_play_session(&mut self, world) -> bool` (`:109-161`:
`discard_open_api_batch()` (`:111-122`), `restore_snapshot(world)` (`:124-140`, keeping
both the loss-reporting and the transform-baseline reasons), `restore_editing_camera()`
(`:144-147`), the `UiElementsHidden` re-insert, the backdrop reset, the state change) and
`toggle_camera_follow_with_feedback()` (`:162-173`). `handle_play_action` keeps its
signature and doc, the marquee-cancel guard (`:22-28`) and a four-arm match of calls; every
`log::` line and status message survives verbatim. `apply_selection_restore` stays in
`shortcuts.rs`.

`dispatch_editor_action` (`:257-414`, 158 lines) becomes a router whose one exhaustive
`match` groups the variants with `|` patterns and calls four category methods:
`dispatch_edit_action` (Undo, Redo, Duplicate, Delete, Copy, Paste, Cut, SelectAll, Cancel,
the four nudges, RenameSelected, CreateEntity), `dispatch_file_action` (Save, SaveAs,
NewScene, OpenScene, Exit), `dispatch_view_action` (ZoomIn, ZoomOut, ResetZoom, ToggleGrid,
ToggleColliders, ToggleSnap, TogglePanel, ResetLayout, CycleGameLocale) and
`dispatch_tool_action` (ToolSelect, ToolMove, ToolRotate, ToolScale). `PlayResume` (`:370`)
is a one-line arm in the router, as are the poll-only arm (`:380-386`) and the
peeled-by-the-caller arm (`:387`), each with its comment. Each category method matches its own variants and ends in `other =>
log::error!("{other:?} is not a <category> action")` — a misrouted action is a dispatch
bug and must not be swallowed silently, per `training.md` § No `#[allow]`. The `drag_guard`
closure (`:259-267`) becomes `fn refuse_during_drag(&mut self) -> bool` (shows the message,
returns whether a drag is live) so the six guarded arms read `if
self.refuse_during_drag() { return; }`. Five bodies over four lines become methods with
the arm's name: `select_all_entities` (`:331-340`), `begin_rename_of_primary` (`:352-369`),
`create_entity_at_view_center(archetype)` (`:388-399`), `reset_layout_with_feedback`
(`:402-405`), `cycle_game_locale_with_feedback` (`:406-412`); the two Save arms share
`report_save_result(result)` (`:281-293`, one status error + one log). The router
exhaustiveness is the compile-time proof every action is still handled; the routing itself
has no test — `shortcuts_tests.rs` drives the bodies through the public paths it already
uses, unchanged.

`shortcuts.rs` after this: `route_editor_key` + `handle_editor_key` (9.6), the router and
its four category methods and six body methods, `apply_selection_restore`,
`undo_with_feedback`/`redo_with_feedback`, `cancel_cascade`, `nudge_selection` — about 380
lines. `play_session.rs` about 220. The `mod.rs:28-35` module list and the crate guide's file
map (9.11) name the new file.

### 9.8 Gizmo drag leaves `viewport_interaction.rs`

`handle_gizmo` (`viewport_interaction.rs:291-357`), `apply_gizmo_drag` (`:366-416`),
`commit_gizmo_drag` (`:424-462`), `cancel_gizmo_drag` (`:467-490`) and `scale_collider`
(`:496-510`) move to `editor_game/gizmo_drag.rs` (28 lines today, the state structs) as
the same `impl<G: Game> EditorGame<G>` block, doc comments intact. `handle_gizmo` splits
its drag-start capture (`:327-345`) into `capture_drag_start(&mut self, world)`, so the
method is the panel lookup, the clipped render, capture, apply, commit. The seven gizmo
tests in `viewport_interaction_tests.rs:21-216` move byte-for-byte to a new
`gizmo_drag_tests.rs` (declared in `mod.rs` with the others); the marquee, chrome and
pickable tests (`:217-408`) stay. `viewport_interaction.rs` ends around 320 lines,
`gizmo_drag.rs` around 260. `chrome_owns_mouse` and `build_pickable_entities` stay where
they are.

### 9.9 `behavior_editor.rs` — one editor per variant

`edit_behavior` (`:41-149`) keeps the header, the variant cycle and the hint-to-edit tail
(`:46-62`, `:148`); the eight arms become `fn edit_<variant>(inspector: &mut
EditableInspector<'_>, behavior: &mut Behavior) -> Option<&'static str>` returning the
field hint — `edit_player_platformer`, `edit_player_top_down`, `edit_follow_entity`,
`edit_follow_tagged`, `edit_patrol`, `edit_collectible`, `edit_chase_tagged`,
`edit_camera_follow`. Each takes the owning `&mut Behavior` and destructures with `let
Behavior::X { .. } = behavior else { log::error!("…"); return None; };` — the shape
`training.md` § No `#[allow]` prescribes, because `CameraFollow`'s six fields plus the
inspector and the hint would be eight parameters. The dispatch is a fn-pointer lookup so
the borrow is trivially sound: `type VariantEditor = fn(&mut EditableInspector<'_>, &mut
Behavior) -> Option<&'static str>; fn variant_editor(behavior: &Behavior) -> VariantEditor`
(one exhaustive match), then `let hint = variant_editor(&new)(inspector, &mut new);`. The
three copies of the tuple-pair edit (`:86-101`, `:121-128`, `:136-143`) become
`edit_point(inspector, label, point: &mut (f32, f32), range, hint: &mut Option<&'static
str>, field_hint)` — six parameters. Labels, ranges, field hints and the `Dead Zone`
read-only line (with its reason) are byte-identical. Lock: `component_editors/tests.rs:59`
(the field-4 tag commit) and `:130` (the commit-before-cycle ordering), plus a new test in `behavior_editor.rs`'s own module:
`test_every_variant_renders_without_a_phantom_edit` — for every `default_for_variant`, one
inert `ui` frame (no input) through `edit_behavior` returns `None`, which is what a
per-variant editor that sets its hint unconditionally would break.

### 9.10 The three renderers — `render_inspector_editable`, `render_asset_browser`, `render_node`

- `render_inspector_editable`: 9.6.
- `render_asset_browser` (`asset_browser.rs:45-205`, 160 lines) →
  `render_header(editor, ui, assets, bounds)` (`:51-64`: the Rescan button, the scan,
  the count label), `load_pending_thumbnails(entries, assets)` (`:67-83`, keeping the
  per-frame cap), the scroll block staying inline (`:85-99`, with its record-before-consume
  reason), `render_tile(ui, theme, assets, entry, slot)` (`:113-162`: background,
  image / placeholder / scene glyph, filename), `tile_interaction(ui, editor, entry, index,
  slot, mouse_pos) -> Option<(u32, String)>` (`:164-185`: the press arms a drag, the click
  assigns) and `assign_clicked_texture(editor, world, command_history, handle, path)`
  (`:188-201`). The loop body is then cull, `render_tile`, and, unless Playing,
  `tile_interaction`. Asset-browser drawing has no headless lock beyond `tile_rect`; the
  assign path is `entity_ops::assign_sprite_texture`, already tested.
- `render_node` (`hierarchy/mod.rs:289-403`, 115 lines) → `render_node` (push
  `visible_order`, read `is_expanded` once with its reason, `if row_visible { self.render_row(…) }`,
  `render_children`), `render_row(&mut self, ctx, entity, depth, y, is_expanded)` (`:303-388`:
  the fills, the arrow — keeping the inert-while-renaming reason — then either
  `render_rename_field(&mut self, ctx, entity, name_x, y)` (`:343-364`; it clears
  `self.renaming`) or
  `render_row_label(ctx, entity, row: &RowGeometry, arrow_clicked)` (`:365-388`)) and
  `render_children(&mut self, ctx, entity, depth, y) -> f32` (`:390-401`). `struct RowGeometry
  { x, name_x, row_rect, has_children, is_selected }` is built once per row so the label
  half does not take seven parameters. `hierarchy/tests.rs` (six tests through `render`,
  incl. the rename commit at `:133` and the row fills at `:246`) is the lock.

### 9.11 `stored_component/mod.rs` — what can leave

`ComponentKind` stays (macro-generated — see the notes). Moves: `ComponentCategory` with its
`ALL` and `label` (`:102-135`) and `categorized_components` (`:539-554`) to
`stored_component/category.rs`; `render_dynamic_edit_blocks` (`:496-520`) to
`stored_component/dynamic.rs`, `pub(super)`, next to the dynamic helpers it calls.
`mod.rs` keeps `pub use category::{ComponentCategory, categorized_components}` so
`lib.rs:135-139` and `add_component_popup.rs:6-9` compile unchanged; the macro invocation
references `render_dynamic_edit_blocks` through `dynamic::`. `available_components` and
`restore_components` stay. `mod.rs` ends around 490 lines. `stored_component/tests.rs`
(`:44`, `:128`) and the popup's `test_popup_rows_…` are the lock.

### 9.12 Docs the batch forces

`crates/editor_integration/CLAUDE.md`: `:27` (`update()` = named phases — now true; name
`prepare_frame`/`render_early_overlays`/`finish_frame`), `:30` (`drain_api_requests` splits
`take_api_lines`; `api_batch` → `ApiSession`), `:31` (`shortcuts.rs`: `route_editor_key`
+ the four category dispatchers; play transitions moved), a new `play_session.rs` line
after it, `:33` (`gizmo_drag.rs` now holds the drag methods and `scale_collider`), `:34`
(`viewport_interaction.rs`: picking, marquee, framing, the once-per-frame pickables; tests
split into `viewport_interaction_tests.rs` and `gizmo_drag_tests.rs`), `:48` ("synced at
update 0d and again before the status bar" → synced once by `sync_dirty_mirror` before the
status bar; `scene_io` reads the history), `:71` (the sentence listing the four untestable
`ctx` takers: `take_api_lines`, `route_editor_key`, `handle_viewport_picking` and
`render_inspector` now have headless guards; `update`/`init` remain). `crates/editor/CLAUDE.md`:
`:30` (`write.rs` → `write/` with `verbs.rs`; `parse.rs` one parser per verb), `:57`
(`edit_behavior` dispatches to per-variant editors), `:73` (`stored_component/` names
`category.rs`), `:61` (`hierarchy/` names `render_row`/`render_children`). Root `CLAUDE.md:222-223`
name `dispatch_editor_action`, `handle_editor_key` and `gizmo_drag.rs`, all of which survive;
no change. `.claude/skills/add-component/SKILL.md:72` and its `.junie` twin name
`stored_component/mod.rs` for the registry line, which stays; no change. Grep every moved or
renamed name (`api_rx`, `api_batch`, `pending_scene_action`, `pending_dialog_choice`,
`handle_shared_viewport_input`, `render_inspector_editable`, `scale_collider`,
`categorized_components`, `render_dynamic_edit_blocks`) across the living guides — every
`CLAUDE.md`, `README.md`, `training.md`, `docs/EDITOR_COMMAND_API.md`, the `.claude/` and
`.junie/` skills — not `review/`, `coordination/`, `docs/EDITOR_UX_AUDIT.md` or
`log_archive.md`.

### Deliberately not in this batch

`EditorContext`'s delegation shell and its widget/domain split (audit 2.4; filed); a
`category()` on `EditorAction` in the editor crate (the router's `|` patterns keep the
grouping in the one crate that dispatches); a shared cross-crate test harness for the
press/release frames (each crate's copy is `#[cfg(test)]`; a `ui` feature-gated harness
is its own decision); any change to `EditorInputMapping`'s bindings or to what
`handle_play_action` logs; `render_*` renames for functions that mutate (immediate-mode
convention, per `plan-sequence.md`); `Gizmo::render`'s three jobs (audit 2.8); the
`headless.rs` transport.

Extra verification: all editor suites (`cargo test -p editor -p editor_integration` after
each of 9.1, 9.3, 9.5, 9.7, 9.9; the workspace at the end); `cargo clippy --example
editor_demo --features editor -- -D warnings` (the `--all-targets` gate skips it); the
games gate `scripts/check_games.sh` (check + clippy — no public item of the six shared crates
changes, but `editor` is a dependency of every game's `editor` feature); no wasm gate
(`editor` and `editor_integration` are not in `check_wasm.sh`'s scope); an `editor_demo`
manual pass (play/pause/stop, undo merge on an inspector drag, marquee, gizmo drag with
Escape, the API `--api` pipe with a `batch begin` across Play) — Jesse's check, after the
commit.

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

## Batch 10 — docs and guides (Markdown only; this section is the whole spec, re-verified against the tree Sep 4 2026; reviewed like every batch — the hook's `--numstat` counts Markdown, so "review skipped" was never true)

Re-verification notes — what earlier batches already did, so the executor does not chase it:
the collision-bus sentence, the "arms in BOTH loader and serializer" phrasing and the
"Behavior ↔ BehaviorData From pair" SSOT row are gone (batches 5 and 8); root `CLAUDE.md`
§ Key Metrics and § Test Status carry no count; `crates/*/CLAUDE.md` "Testing" sections carry
no count except `ecs_macros`; the physics pass-through changelog is in `log_archive.md`;
`crates/common/CLAUDE.md` no longer lists `thiserror`. What remains is below, numbered in the
order of work.

### 10.1 The last test counts

- `README.md` § Test Summary (the nine-row table totalling 955 and the "Run all tests" line
  under it): the table goes; the section becomes two lines — `cargo test --workspace`, headless,
  and the invariant "0 failed, 0 ignored". `README.md` § Project Status "Test Status" line
  already says that and stays.
- `crates/ecs_macros/CLAUDE.md:10` "4 tests (3 integration + 1 doc), run with …" → the command only.
- `training.md:801` "Current count lives in `CLAUDE.md`" — no doc records a count; the
  sentence goes.
- Gate: `grep -rnE "[0-9]+ tests\b|passing|passed:" CLAUDE.md training.md README.md
  crates/*/CLAUDE.md` prints nothing. `docs/EDITOR_UX_AUDIT.md` and `log_archive.md` are
  history and keep theirs.

### 10.2 File Maps stop restating the `mod` tree

Applies to the File Map / Files section of every crate guide that has one: `engine_core`
(114 lines), `editor` (54), `renderer` (43), `common` (30), `editor_integration` (16),
`physics` (15), `ecs` (12), `ui` (9), `audio` (7). The rule, per line: a line that names a
file and paraphrases its module doc is deleted (`ls crates/<c>/src` and `grep -n "^pub mod\|^mod"`
say the same thing, and the module doc says it better). A line survives only if it says
something neither can — a cross-file rule ("new render passes go in their own module like
`tilemap_render.rs`"), a pitfall ("never unify the two frame drivers — an occluded native
window stops receiving redraws"), a contract that spans files (`clock.rs`'s "import time types
from here, never `std::time`"), or a name the file name does not carry (`SidecarCache`, the
`#white` sentinel). A surviving line is trimmed to that sentence; the file name stays as its
anchor. Expected: `engine_core` keeps roughly a third, `editor` and `renderer` about half,
`common` most of it (its lines are contracts), the rest lose a line or two.
- Where a deleted line carried a pitfall, that pitfall moves to the 10.3 table instead of
  vanishing. Read each line for that before deleting it.
- Every guide's dead-name check, before you report: for each deleted-in-this-effort symbol
  named in `coordination/cleanup-2026-09/plan.md` § Batch 2 (its bullets list them), grep the
  living guides — `CLAUDE.md`, `training.md`, `README.md`, `crates/*/CLAUDE.md`,
  `docs/EDITOR_COMMAND_API.md`, `docs/WEB_SAVES.md`, `.claude/`, `.junie/`, `.kimi-code/` — and
  paste any hit; batches 2–9 each swept their own lines, so the expected result is none.

### 10.3 Pitfalls become "pitfall → guard test" tables

One section per crate guide named `## Pitfalls and their guard tests`, a two-column table:
the pitfall as one sentence, and the test that fails when it regresses as `src/file.rs
test_name` or `tests/file.rs test_name` (path relative to the crate root — several guards are
integration tests, `ecs/tests/hierarchy_dirty.rs`, `physics/tests/external_edits.rs`). Sources,
in order: every bullet in the guide that states a trap, whatever its section is called —
`Common Pitfalls`, `Key Patterns`, `Critical Patterns`, `Key Guidelines`, `Design Notes`, and
physics' `Collision Event Contract` (take the events once per frame) and `Physics Entities Must
Be Root Entities` (a bullet that states a convention with no failure mode stays a bullet where
it is); the root `CLAUDE.md` § Known Footguns bullets that belong to that crate
(the root list stays as the cross-crate summary — do not delete from it); and the lines 10.2
moved here. The test name comes from the CONTRACT rows of
`coordination/cleanup-2026-09/keeplist-*.md` (grep the pitfall's key noun there) or from the
test the bullet already names. A pitfall with no guard test gets `— none` in the second
column, verbatim; it is reported in the batch report, not fixed here (writing tests is not
this batch). `crates/editor_integration/CLAUDE.md` § Common Pitfalls already has the shape as
bullets; it becomes the table. `input`'s table comes from its `Design Notes` plus the two
10.4 keepers; only `ecs_macros` gains no table. No row ceiling: a row earns its place by naming
a failure mode, and `engine_core` is expected to run past ten because its File Map carries that
many contracts (the frame-driver split, the gesture-gated audio retry cap, the web preload
order, device-loss fail-stop, the save_store merge rule).

### 10.4 The nine `crates/*/ANALYSIS.md` are deleted

`git rm crates/{audio,common,ecs,editor,engine_core,input,physics,renderer,ui}/ANALYSIS.md`.
The family was retired Aug 2026 (`continue.md` § 3.1 says "do not create them"), eight of the
nine teach deleted API (`common::Time`, `play_music_once`/`stop_all`/`unload_all`,
`SceneManager`, `device_ref`, `EngineApplication`, `apply_force`/`reset_forces`, archetype storage, stale
counts). A read of all nine against the tree on Sep 4 2026 found about forty lines still true,
non-obvious and stated nowhere else; those move first, each as one row of the crate's 10.3
table or one sentence in the guide, then the files go. The keepers, verified against the tree:
- `common/ANALYSIS.md:23` — the graduation rule (a module used by fewer than three crates does
  not belong in `common`) → one sentence in `crates/common/CLAUDE.md`'s intro.
- `ui/ANALYSIS.md:46-50` — the dual glyph cache is intentional: `ui` caches rasterized bitmaps
  (`font/glyph_cache.rs`), `engine_core` caches GPU textures (`glyph_texture_cache.rs`) → one
  sentence in both guides.
- `input/ANALYSIS.md:118-124` — `GamepadState::axis_value` (`gamepad.rs:118`) returns the raw
  axis, no dead zone; `AXIS_ACTIVATION_THRESHOLD` filters digital sources only → a
  `crates/input/CLAUDE.md` § Design Notes bullet. `input/ANALYSIS.md:99-104` — `update()`
  (`input_handler.rs:283`) fuses `process_queued_events`/`end_frame` and can eat `just_*` reads
  → the same section.
- `audio/ANALYSIS.md:65` — `_stream: OutputStream` (`manager/mod.rs:65`) is kept alive on
  purpose; dropping it kills all audio → 10.3 row. `audio/ANALYSIS.md:29` — the audio
  components live in `ecs` to avoid a circular dependency → the "why" clause on the existing
  guide line.
- `ecs/ANALYSIS.md:128-133` — `World::update` swaps `systems` out (`world.rs:82-84`,
  `system.rs:206`) so a system cannot reach the system list; fragile by design → 10.3 row.
  `ecs/ANALYSIS.md:118-126` — `query_entities` allocates through `entities()` (`world.rs:362`);
  `entity_ids()` is the iterator → the `#86` GPP-L1 item already says it; a guide bullet.
- `engine_core/ANALYSIS.md:130-138` and `211-214` — the boundary policy: `ui` defines
  `DrawCommand`, `renderer` defines `Sprite`, `engine_core` owns the bridge; cross-cutting glue
  biases toward `engine_core`, and `ui`/`renderer` never depend on each other transitively
  through it → a short "Crate boundary" paragraph in `crates/engine_core/CLAUDE.md`.
- `physics/ANALYSIS.md:141-151` — `PhysicsWorld::apply_impulse` (`physics_world/bodies.rs:252`)
  needs a synced body and silently no-ops on a same-frame spawn; `set_velocity` defers → 10.3
  row, guard test from the keep-list or `— none`.
- `editor/ANALYSIS.md:202-206` — typed `EditResult<T>` returns exist so `editor` stays free of
  an `engine_core` dependency → one clause on the guide's `field_style.rs` line.
- `renderer/ANALYSIS.md` — nothing; every live point is already in its guide.
The references follow:
- `.claude/commands/missed.md:28` and `.junie/commands/missed.md:28`: the ANALYSIS.md item is
  deleted (item 2, the `tech-debt` issues, is the replacement and is already there). Closes #95.
- `.junie/commands/continue.md`: line 45 (§ 2.1's "Crate's ANALYSIS.md (if exists)" item) is
  deleted; § 3.1 and § 6.1 ("Update ANALYSIS.md") each collapse to their heading plus the one
  sentence "The issue thread is the durable record; the per-crate analysis files are retired,
  do not create them." (worded without the file name, so the gate below stays mechanical);
  checklist line 138 becomes "Documentation updated (board issues closed/filed)". The rest of
  that file's drift is #94's, not this batch's. `.claude/commands/continue.md:67` is the
  canonical retirement line and is not touched.
- `docs/EDITOR_UX_AUDIT.md:579` cites `ANALYSIS.md:242` for the Console-panel placeholder — history,
  stays as written. `log_archive.md:343` "Earlier (from ANALYSIS.md)" — history, stays.
- Show the grep: `grep -rn "ANALYSIS" --include=*.md . | grep -v "^./target\|^./review\|^./coordination\|^./.claude/worktrees\|log_archive\|EDITOR_UX_AUDIT"`
  prints exactly one line, `./.claude/commands/continue.md:67` (the retirement notice). Any other
  line is a live reference the batch missed.

### 10.5 Root guides

- `training.md` § Test Status Summary: the 10.1 sentence. § Key Commands "Check for TODOs"
  stays (it is a gate). Nothing else in `training.md` — the API sections were kept current
  batch by batch.
- `CLAUDE.md` § Known Footguns keeps every bullet (cross-crate summary); § Key Metrics stays.
- `log_archive.md`: the `docs/plans/` deletion (batch 2, `cc31078`: three shipped Jan–Feb 2026
  plans for scene saving and a DRY/SRP pass; history lives in git) gets one dated line in the
  Sep 2026 cleanup entry. The effort's lessons entry is the planner's, written at close from
  `reviewer-comparison.md` — not this batch.

### 10.6 Planner's own work, after the commit (not the executor's)

- Issue bookkeeping (needs `gh`): close #82 (one `clamp_volume` in `common`, batch 4) and #95; tick
  #84 DRY-011 (`ui_integration` uses `TextureHandle::WHITE`; the three `TextureHandle { id: 0 }`
  in `assets.rs:307` and `texture_ref.rs:74,91` are the white-handle producers and stay as the
  item's remainder — say so on the item), #86 GPP-L12 (`ecs/src/event.rs`'s module doc states the same-frame
  emit-then-read contract), #89 DRY-006 (already ticked; ARCH-006 is #10's — #89 stays OPEN as the renderer
  backlog issue, since 10.6's deferred renderer items land there), #90 ARCH-101 (`editor::archetype::Archetype`, batch 6; UX-001 stays open), #91 KISS-002
  (`thiserror` gone from `crates/common/Cargo.toml`, batch 2).
- The deferred items below are appended as checklist items to the existing per-crate
  "Low-priority backlog" issues (the house shape's one exception to one-item-per-issue), not
  filed as ten new issues: `EditorGame` regrouping and `MenuAction`-style merge-policy move →
  #90; `GameRunner` regrouping, bool parameters → enums, `assets.rs` rename → #84;
  `World` delegation shell, `RegistryError`, `EntityId` by-value convention → #86;
  `Renderer` FrameGraph, shared `CameraBinding`, WGSL renames and the error-callback `panic!`
  → #89; `ui` depending on `input` → #88; `EditorContext` delegation shell and the
  `editable_inspector.rs` rename → a new editor backlog issue (the editor crate has none).
  Root `CLAUDE.md` status trimming → #94's thread. `PhysicsWorld` split is already #85 SRP-001.
- `coordination/cleanup-2026-09/` stays tracked as the effort's record (batch 0 said "archive
  or delete when the effort closes": it stays — `reviewer-comparison.md` is what the next
  effort reads).

### Gates

`cargo test --workspace` and `cargo clippy --workspace --all-targets` still run (a Markdown
batch cannot break them, and the report must show it did not); the 10.1 and 10.4 greps; the
tag gate over `crates src examples` (unchanged by this batch, shown anyway); no `.rs` file
touched (`git diff --cached --name-only -- '*.rs'` prints nothing); every touched Markdown file
renders — no dangling table row, no unclosed fence (`grep -c '^```' <file>` is even for each).
Review: kimi and Claude code-mode over the staged diff, same as every batch.

### Deliberately not in this batch

- The four in-repo agent-tool mirrors' remaining drift (#94), beyond the ANALYSIS.md lines.
- `docs/EDITOR_UX_AUDIT.md`, `log_archive.md`, `coordination/` and `review/`: history.
- Root `CLAUDE.md` status-report trimming (the Core Systems Complete / Current Priority
  paragraphs) — a rewrite with an owner, not a cleanup; goes to #94's thread per 10.6.
- Rewriting `training.md`'s pattern sections — kept current per batch; a rewrite is its own effort.
- New tests for a `— none` row in a 10.3 table.

## Deliberately not doing (appended to the per-crate backlog issues in batch 10, § 10.6)

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
