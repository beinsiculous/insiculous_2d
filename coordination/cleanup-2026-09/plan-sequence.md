# insiculous_2d code-quality cleanup: batch plan

Ten batches, each one commit and one kimi code-mode review. Order: correctness, deletions, mechanical sweep, DRY (systems crates, engine_core, editor commands, editor UI), SRP splits (engine side, editor side), docs.

## What verification changed before sequencing

Every claim below was checked against the tree at d58f3c3 and the Studio Board. Four corrections shape the batches:

- **The cache-skipping texture loader has zero callers.** `AssetManager::load_texture_with_config` is a deletion, not a cache fix.
- **The physics event-bus mirror has zero readers** anywhere in the workspace or the six games (`world.emit_event(collision.clone())` in `physics_system/update.rs:87`; no `read_events::<CollisionData>` exists). Only `take_collision_events` is consumed.
- **`GizmoPalette::default()` and `GridColors::default()` are only pre-theme initial values.** `Gizmo::apply_theme` and `panel_renderer` overwrite them from `EditorTheme` every frame, so the palette disagreement is a one-line default fix, not a visible bug.
- **All six games call `GridMesh::new(..).with_color(..).with_emissive(..)`** and hold `Option<GridMesh>`. The audit's "GridMesh holds a GridBackdrop" fold would ripple into every game, so it waits for issue #96 (games adopt GridBackdrop). The minimal fix, one copy of the tunable list in `grid/build.rs`, ships now.

Also confirmed: `SceneManager` and `editor::prelude` have no users; `UIManager` is a three-method forwarder; the games write `ctx.chaos_mode` (13), `ctx.exit_requested` (12) and `ctx.time_scale` (6) directly; the games discard `achievements.unlock`'s bool 34 times; `run_game` is called as `.unwrap()` in `main.rs` and formatted with `{e}` in `web_entry.rs`, so a typed return is source-compatible; `PhysicsConfig` presets are used by four games but only through the named constructors; `ecs::Behavior` and `engine_core::BehaviorData` carry identical variant names; `editor_preferences.rs` already defaults on a missing file; `PlayerBindings` is what the persisted input-settings JSON serializes, so its serde shape is a contract.

## Ground rules for every batch

- Per-batch gates: `cargo test --workspace` at 0 failed / 0 ignored, `cargo clippy --workspace --all-targets` at 0 warnings, every touched file at or under 600 lines, no new `#[allow]`, no `unwrap()` outside tests.
- Games gate whenever a batch touches a public item of `engine_core`, `ecs`, `physics`, `input`, `common` or `renderer`:

```sh
for g in pong snake breakout frogger asteroids space_invaders; do
  cargo check --manifest-path ../games/$g/Cargo.toml &&
  cargo clippy --manifest-path ../games/$g/Cargo.toml --all-targets
done
```

  Batches 1, 4 and 5 also run `cargo test --manifest-path ../games/$g/Cargo.toml` for each game because they change runtime behaviour the game tests exercise (input edges, physics presets, `run_game`, collision data).
- Wasm gate (`scripts/check_wasm.sh`) whenever `engine_core`, `renderer`, `audio`, `input` or `common` change.
- The commit hook denies any commit of 100 or more changed lines that has not been through code-mode review. Every batch except batch 10 exceeds that, so only batch 10 can skip kimi. Commit with `ADV_REVIEWED=1` after adjudication; never write skip trailers.
- Docs must match reality at every commit (finish-task gate 4). Each batch carries the doc line edits its deletions force. The guide restructure itself is batch 10.
- Batches 6 and 8 touch disjoint crates (editor family vs engine_core/renderer/ecs) and may run in parallel on two agents. Everything else is sequential, because later batches move code that earlier batches clean.
- Branch: the repo is on `dev`; the working-set rule says commit to the branch matching the machine (`hostname`), confirm before the first commit.

## Batch 1: correctness fixes with regression tests

Files: `crates/input/src/{button_tracker,input_handler,input_mapping,player}.rs`, `crates/common/src/color.rs`, `crates/engine_core/src/texture_ref.rs` (test only), `crates/renderer/src/texture.rs`, `crates/engine_core/src/scene_serializer.rs` (test only), `crates/ui/src/font/layout.rs`, `crates/ui/src/context/text.rs`, `crates/editor/src/{inspector,gizmo/mod,grid,dock/mod}.rs`.

1. `ButtonTracker::release` inserts into `just_released` only when `pressed.remove(&button)` returned true. Test: a release with no prior press yields no edge (window refocus case).
2. Previous-frame snapshot in `ButtonTracker`: `previous_pressed: HashSet<T>`, swapped with `pressed` (then cloned back) in `clear_frame_state`, accessor `was_pressed`. `InputHandler` gains `is_source_was_pressed`; `source_was_pressed` in `input_mapping.rs` reads it instead of reconstructing from `just_pressed`/`just_released`. `just_activated` becomes "a bound source is `just_pressed` and the action was not active last frame", so a press-and-release inside one frame still fires once and no phantom `just_deactivated` follows. `InputSettings` in `player.rs` shares the same helper, so both layers change together. Tests: tap inside one frame activates once and never deactivates; second bound key while one is held does not re-trigger (existing test, keep); gamepad axis edges unchanged.
3. `Color::to_rgba8` rounds and clamps: `(channel * 255.0).round().clamp(0.0, 255.0) as u8`. Tests: `from_rgba8(x).to_rgba8() == x` for every byte value; a `#solid:RRGGBB` texture ref survives ref → colour → ref (this is user-visible in scene files today).
4. `texture.rs:257,277,290` multiply in `usize`. Read `MAX_TEXTURE_DIMENSION` first: if it already bounds `width * height * 4` below 2^32, this is hardening with no test; otherwise add a test that an oversized dimension pair with a short slice errors instead of passing.
5. Drift guard for `CONCRETE_OR_EXCLUDED` (`scene_serializer.rs:294`): a test saves a world holding one of every concrete component type and asserts each appears exactly once in the output and no `ComponentData::Dynamic` appears. This is the only audit finding that can silently corrupt a scene file.
6. One text-height function in `font/layout.rs` used by both `layout` and `measure_text`. Test: measured height equals layout height for the same font and size, which is what `text_pos_in_bounds` assumes.
7. `inspect_component` draws through `label_styled` with `InspectorStyle` colours, so the read-only inspector shown during Play honours the theme. Test: the header draw command carries `style.header_color` (headless `UIContext`).
8. `GizmoPalette::default()` returns `EditorTheme::default().gizmo_palette()`; `GridColors::default()` returns `EditorTheme::default().grid_colors()`. Test: equality, so the palettes can never disagree again.
9. `DockPanel.header_height` field deleted; `layout::HEADER_HEIGHT` is the only source, used by `dock/render.rs:107` and `content_bounds`/`effective_size` alike.

Size: about 300 lines. Review: yes. Extra verification: games test gate (every `just_activated` in every game rides on item 2), wasm gate.

## Batch 2: pure deletions and workspace hygiene

engine_core:
- Delete `scene_manager.rs` and the `pub use` at `lib.rs:90`.
- Delete `ui_manager.rs`; `GameRunner` holds `ui: UIContext` directly (its two tests test `UIContext` and move to the ui crate or drop). Call sites: `game.rs`, `game/frame_tail.rs`, `game/render.rs`, `game/app_handler.rs`.
- Delete `assets.rs` `load_texture_with_config` (zero callers).
- `ui_integration/mod.rs:88` `TextureHandle { id: 0 }` → `TextureHandle::WHITE` (ticks #84 DRY-011).

physics:
- Delete the `emit_event` mirror in `physics_system/update.rs:85-88` and the training.md sentence "Events also reach the world event bus".
- Delete the five `.with_scale(100.0)` in `presets.rs` (already the default).
- Delete `physics_prop`, `small_box`, `pushable_box`, `slippery` (zero callers; training.md names none of them). Keep `rigid_body_count`/`collider_count` as headless test seams.

audio: delete `play_music_once`, `stop_all`, `active_sound_count`, `unload_all` with their tests. Adjudication point: plausible game features with no consumer in six games.

renderer: delete `update_instance_buffer`, `invalidate_texture_cache`, `clear_texture_cache`, `pipeline()` in `sprite/pipeline.rs`; delete the Arc-returning `device()`/`queue()` and rename `device_ref`/`queue_ref` to `device`/`queue` (callers: `render_manager.rs:144,147,323,328,333`, `game.rs:382` become borrows); inline `create_sampler` (`texture.rs:389`) and `render_with_sprites_internal` (`renderer.rs:379`).

ui: delete `hot_widget`, `is_hot`, `InteractionResult::local_mouse`, `is_overlay`, `hit_test`, `cache_stats`, `checkbox_labeled`.

editor / editor_integration:
- The 13 dead items in the editor audit §7.1: `generate_entity_sprite`, `batch_entities` (drops the `renderer::sprite` imports from `viewport/mod.rs`), `EditorTheme::dark`, `color_to_vec4`, `inspector_bounds`, `hierarchy_bounds`, `reset_cycle`, `with_pick_margin`, `set_axes_visible`, `set_axis_length`, `set_interpolation_speed`, `set_version`, `with_button_size`.
- The `prelude` module in `editor/src/lib.rs:140`.
- Test-only API with their tests: `enter_play_mode`, `exit_play_mode`, `toggle_play_mode`, `pan_camera`, `visible_item_count`, `close_all`.
- `layout.rs`: `PADDING_SMALL`, `SPACING`, `MENU_BAR_HEIGHT`, `MENU_ITEM_HEIGHT`, `TOOLBAR_HEIGHT`, `TOOLBAR_BUTTON_SIZE`.
- The dead `depth` parameter through `inspector.rs:110,152`; `let _ = component_index;` at `panel_renderer/inspector.rs:286`; the `run_game_with_editor_api` middle link (fold into `run_game_with_editor_opts`); the caller-less half of each dock `set_/toggle_` pair.

common: drop the unused `thiserror` dependency from `crates/common/Cargo.toml` and its guide (ticks #91 KISS-002).

Workspace:
- `git rm --cached editor_prefs.json` and add it to `.gitignore`.
- Delete `validate_demo.sh` (runs a non-existent example).
- `git mv run_gpu_diagnostics.sh scripts/`.
- Delete `examples/pong_editor_screenshot.png`; update the stale-screenshot note in `docs/EDITOR_UX_AUDIT.md:7`.
- Delete the three shipped plans under `docs/plans/` (history lives in git and `log_archive.md`).
- Keep `coordination/BLOCKERS.md`, `TODO.md` and `H1_SPIKE.md`: the ruleset, `/continue`, the roadmap and `game/web.rs:46` reference them.

In-batch doc edits: root `CLAUDE.md` manager list (three managers: `GameLoopManager`, `RenderManager`, `WindowManager`), `training.md` directory map and Manager Pattern snippet, `crates/engine_core/CLAUDE.md:15,80,85`.

Size: about 900 lines removed, 60 added. Review: yes. Extra verification: games check gate, wasm gate, `cargo run --example hello_world` for the UIManager removal (the frame path is not headless).

## Batch 3: mechanical sweep (comments, names, allows)

Comment policy (the user's refinement folded in):
- **`///` doc comments stay.** They are the public-API convention and the finish-task gate requires them on new public items. They state the contract in a few lines and carry no history. Module-doc essays are cut to the contract: `physics/src/physics_system/mod.rs:20-55` (35-line pass-through justification with a changelog, becomes one paragraph; the changelog goes to `log_archive.md`), and the same treatment for the other essay-heavy module docs the audit ranked: `ui/src/lib.rs`, `ui/src/context/mod.rs`, `input/src/input_handler.rs`, `input/src/input_mapping.rs`, `audio/src/manager/mod.rs`, `renderer/src/renderer.rs`, `editor/src/theme/mod.rs`, `editor/src/commands/mod.rs`, `engine_core/src/prelude.rs`. Each keeps what a caller needs and drops what a historian would want.
- **Plain `//` comments are minimal.** Only pitfalls, invariants and failed approaches survive (the Firefox in-process WebGPU crash in `render.rs`, the painter's-algorithm reason for two batchers, the `Box<dyn Component>` downcast trap, the RwLock re-entrancy guard, the tie-ordering rule in `scores.rs`). Everything else goes: narration that restates the next line (`frame_tail.rs`, `white_texture.rs:15,31,34,50`, `sync.rs:53,56`, `component_commands.rs:99`, `render.rs:46,110`, `scene_io.rs:252`), "Phase N" and numbered section headers, issue numbers, sprint names, reviewer tags, audit section references.
- **Keep the reason, drop the tag.** "Colliders are absolute-pixel sized (GPP-09)" becomes "Colliders are absolute-pixel sized". "must not snap an active ripple to rest (kimi #46 F3)" keeps the first clause. A comment that is only a tag with no reason is deleted.
- **Numbered headers now, names later.** Strip the numbers in `render.rs` (Phase 1/2), `sprite/pipeline.rs` (ten "Create …" headers), `EditorGame::update` (0 through 12, no 8) and keep the sentence; batches 8 and 9 replace each sentence with a function name.
- Fix the two misattached doc comments (`theme/mod.rs:33-40`: move the `// ── Surface ladder` header out of the doc-comment run so `bg_primary` gets its doc back; `scene_io.rs:225-239`: split the block between `default_scene_path` and `new_scene`) and the three stale hex values in the theme docs.
- Rewrite the false module doc in `input/src/gamepad.rs:3-9` (the gilrs backend exists), delete the `PATTERNS_AUDIT.md` reference in `renderer/src/sprite/instance_cache.rs:1`.
- File pointers that still resolve (`coordination/H1_SPIKE.md` in `game/web.rs`) stay.
- Gate for the sweep: this returns nothing afterwards (attribute lines and hex literals excluded):

```sh
grep -rEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|Sprint [0-9]" crates src examples --include=*.rs | grep -vE "#\[|0x[0-9a-fA-F]"
```

Names (production code only; tight math in `common` stays):
- physics `physics_world/bodies.rs`: `he` → `half_extents_meters`, `f` → `force_meters`, `imp` → `impulse_meters`, `pos` → `position_meters`, `vel` → `velocity_meters`; `components.rs:214` `hw/hh` → `half_width/half_height`; `physics_system/update.rs:19` `dt` → `clamped_delta_time`; `queries.rs` `max_toi/toi` → `max_time_of_impact/time_of_impact`; `stepping.rs:56` `let event_handler = ();` deleted, pass `&()`.
- ui: `input_state.rs` `kb` → `keyboard`, `pos` → `mouse_position`; `text_input.rs:279` `dx` → `pointer_travel_x`, `:527` `bg` → `background`; `text_edit.rs:182` `d` → `distance_to_click`.
- renderer `texture.rs:169,209` `img` → `image`.
- engine_core `achievements/mod.rs:198` `mgr` → `manager`; `scene_serializer.rs:88-272` the single-letter component bindings (`t/s/c/tm/g/a/rb/col/l/p/b`, with `b` bound to two types) become `transform/sprite/camera/tilemap/grid/animation/rigid_body/collider/label/panel/button/behavior`; `behavior_runner/handlers.rs` `vel_x/vel` → `velocity_x/velocity`.
- editor: `dock/render.rs:60,253,313` `c/b/c` → `center/bounds/accent`; `confirm_dialog.rs:66` `w` → `button_width`; `script_editor.rs:199` `n` → `candidate_index`; `viewport/mod.rs:259` `pc` → `viewport_center`; `panel_renderer/mod.rs:122` `w` → `outline_width`; `play_controls.rs:126` `btn` → `button`.
- WGSL identifiers untouched: no headless way to validate a shader edit.

Allows removed:
- `behavior_runner/handlers.rs:18,95,139`: each handler takes `behavior: &Behavior` and destructures its own variant inside (seven parameters, under the lint threshold, and no more swappable `f32` positional pairs).
- `ui/src/interaction/mod.rs:28`: `WidgetId::from_str` → `WidgetId::hashed` (23 call sites).
- `ui/src/draw/mod.rs:301`: `slider` takes a `SliderVisual` struct.
- The three editor allows (`texture_field.rs:31`, `stored_component/mod.rs:297,513`) wait for batch 7, where an `InspectorFrame` context struct changes those signatures anyway.
- `renderer.rs:230` `arc_with_non_send_sync` stays (decided with H8).

Size: about 700 changed lines across roughly 110 files, nearly all comment lines. Review: yes, line-local and fast. Verification: standard gates, the grep gate, `git diff --stat` showing no file gained code, games check gate (handler signatures are crate-private, but `Behavior` is re-exported).

## Batch 4: DRY in the systems crates

renderer:
- `camera_binding.rs`: `CameraBinding { buffer, layout, bind_group }` with `new(device)` and `update(queue, camera)`, composed by `SpritePipeline` and `LinePipeline` (ticks #89 DRY-006; the second 64-byte upload per frame stays, a shared binding owned by `Renderer` is a later step if anyone measures it).
- `build_fullscreen_pipeline` in `bloom.rs` widens to `build_render_pipeline(device, PipelineSpec { layout, shader, entry points, vertex buffers, topology, blend, depth, target })` in a new `pipeline_builder.rs`, used by bloom, sprite and line pipelines.
- `TextureManager::insert_rgba(width, height, data, config)` behind `load_texture`, `load_texture_from_bytes`, `load_texture_from_rgba`.
- `Sprite` stores `SpriteShape`; `to_instance` flattens it; `with_border` compares the enum.
- `sprite_data.rs`: `wgpu::vertex_attr_array!` replaces the hand-counted offsets; the existing size assertions stay.
- `run_composite_pass` takes `CompositeScissor { Fullscreen, Region(x, y, w, h), Empty }` instead of `Option<Option<..>>`.

physics: `PhysicsWorld::push_collision(entity_a, entity_b, started, contacts)` in `stepping.rs`; `previous_collisions` reuses its allocation (`mem::swap` then `clear`).

audio: private `start_sink(output, source, base_volume, bus_volume, looping) -> AudioResult<Sink>` shared by `manager/mod.rs` (SFX) and `manager/music.rs`.

common: `clamp_volume(f32) -> f32` used by the audio manager's `clamp_volume` and the three `with_volume` builders in `ecs/src/audio_components.rs` (closes #82, ticks #86 DRY-004).

input:
- Fixture test first: a bindings JSON file produced by today's `input_settings_io` is checked in under `crates/engine_core/tests/fixtures/` and must load identically after the refactor. The file is hand-editable by users, so its shape is a contract.
- A private `BindingTable<S>` (`bind`/`unbind`/`bindings`, `#[serde(transparent)]`) backs both `PlayerBindings` and `InputMapping`; `PlayerBindings` keeps only the dirty flag and device-relative resolution on top.
- One `STANDARD_PAD_LAYOUT: &[(GameAction, PadSource)]` feeds `with_default_bindings` and `bind_standard_pad_layout`.

ui:
- One `TYPED_KEYS` const drives both the scan loop in `input_state.rs:164` and `keycode_to_char`.
- `context/edit_field.rs`: the shared focus / click-to-place / Escape / Enter / Tab / click-away / `apply_edit_keys` / unfocused-draw shell, wrapped by `float_input` (parse, format, scrub, nudge) and `text_input` (plain string). Both resolve their face through `field_font`, which ends the drift. `text_input.rs` sits at 539 lines, so the new file is required anyway.
- `TextInputStyle` derives `Copy` (every field is `Color` or `f32`), ending the per-field-per-frame clone.

Size: about 700 lines. Review: yes, expect two rounds (edit_field and the vertex layout). Extra verification: games test gate (pong's two-player bindings and every `ctx.players` call), wasm gate, and a native `hello_world` run by Jesse because the vertex layout and the shape enum are only visible on a GPU (the instance-cache byte tests cover the CPU side).

## Batch 5: DRY in engine_core

- Delete `behavior_data.rs`; `ComponentData::Behavior(ecs::Behavior)`. Step one is a golden fixture: a RON scene containing every Behavior variant, written by today's serializer, checked in under `crates/engine_core/tests/fixtures/`. After the change it must load to equal values and re-save byte-identical. Any serde default present on `BehaviorData` but missing on `ecs::Behavior` is added there. Adding a behavior then costs one enum arm plus one runner arm instead of four edits.
- `handlers.rs`: `follow_entity` and `follow_tagged` become one `follow_target(target: Option<Vec2>, …)`; the two callers resolve the target first.
- `persistence.rs`: `JsonSaveSlot<T: Serialize + DeserializeOwned>` doing read-merge-write over `save_store` with a caller-supplied merge closure, one `PersistenceError { Io, Serde }`, one `unix_seconds()`. `Scores` and `AchievementManager` keep their public API; `AchievementError` and `ScoresError` become type aliases so the prelude keeps its names.
- The eight hand-written colour conversions in `scene_serializer.rs` and `scene_loader_components.rs` become `.into()` (glam already converts `Vec4` to and from the 4-tuple).
- `test_support.rs` behind a `test-support` cargo feature: `StubResolver`, `roundtrip`, `test_texture_path`; `editor_integration` enables it in `[dev-dependencies]`. The six copies are deleted.
- `grid/build.rs`: build the bare mesh, then call `apply_grid_tunables`; the builder-chain copy goes. The full `GridTuning` fold is deferred behind #96.
- `achievements/toast.rs` takes `ToastStyle`, `faded`, `draw_toasts`; the `achievements::ToastStyle` re-export path is unchanged.
- `render.rs` `sort_batch_refs` computes `(min_depth, max_depth, texture, clip)` once per batch and sorts the tuples.
- `WindowConfig: From<&GameConfig>` (the `AssetConfig` pattern).
- `run_game` returns `Result<(), EngineError>` with `From` impls for the underlying error types; `EngineError` is already exported from the prelude and used by nothing else.
- `#[must_use]` on `Scores::submit`, `SpriteAnimation::play`/`ensure_playing`, and the `GridMesh` builders. Not on `AchievementManager::unlock`: the games discard its result 34 times and would start warning.

Size: about 500 removed, 300 added. Review: yes. Extra verification: the scene round-trip suite plus the new behavior fixture, games test gate (asteroids uses `CollisionData`, every game calls `run_game` and `AchievementManager`), wasm gate.

## Batch 6: DRY in editor commands and the command API

- `SetComponentCommand<T: Component + Clone>` carrying `display: &'static str` and `field_hint` replaces `impl_set_component_command!`; the 13 names stay as type aliases so no call site moves (`SetTransformCommand` has 25). `TransformGizmoCommand` becomes the Transform alias with hint `"gizmo"` (entity-plus-hint merging is equivalent to its entity-only merge). Merge isolation survives because each `SetComponentCommand<T>` is a distinct type under `downcast_ref`.
- `AddComponentCommand`/`RemoveComponentCommand` take `ComponentRef { Typed(ComponentKind), Dynamic(String) }`; the dynamic pair is deleted; constructors take `&str`.
- `CommandHistory::push_as_one(name, commands)` replaces the four `match len` copies (`viewport_interaction.rs:460`, `menu_actions.rs:156,238,273`).
- `physical_floors.rs`: the floor constants (`SCALE_FLOOR`, `COLLIDER_EXTENT_FLOOR`, `PITCH_FLOOR`, the volume range) and `clamp_transform`/`clamp_collider`/`clamp_audio_source`, used by `sanitize()` in `command_api/write.rs` and by the `f32_hard` ranges in `component_editors.rs`.
- `write.rs`: `build_add_patch_set` reuses the Set verb's body; `api.rs:144` uses `WriteCtx::record`; `answer_api_lines` hands the parsed request to `dispatch` instead of the raw line.
- `Archetype` enum with `ALL`, `from_kebab`, `label`, `spawn`; `command_api::ARCHETYPES` and `entity_ops::handle_create_action` derive from it; the drift test becomes an exhaustiveness check (ticks #90 ARCH-101).
- Menu labels map to `EditorAction` through one `editor_action_for_menu_label` and route through `dispatch_editor_action`, so Undo/Redo shows its status message on both paths and a new action is added once. Menu-only items (Exit, panel toggles, Reset Layout, Cycle Game Locale, Create …) stay as the small remaining arms.
- `scene_io.rs`: `World::clear()` at both sites, a `reset_session()` helper for the seven-line block, and a typed `SceneIoError` replacing `Result<_, String>` (also in `headless.rs`).
- `Modifiers::read(input) -> Modifiers { ctrl, shift }` in `editor_input.rs` replaces the five spellings (`shortcuts.rs:206`, `editor_input.rs:341`, `viewport_interaction.rs:108,526`, `panel_renderer/mod.rs:179`).
- A `WidgetSlot { Field(n), Remove, AddButton, PopupRow(n) }` behind `FieldId` replaces the 99 / +50 / +60 arithmetic and its ten-component collision limit.
- One `uncaptured_component_names` shared by `clipboard.rs` and `world_snapshot.rs`; `loss_warning` and `drop_report` share a body with the message as a parameter.

Size: about 450 removed, 250 added. Review: yes, two rounds likely because merge semantics are subtle. Extra verification: `commands/tests.rs`, `dirty_tests.rs`, `write_tests.rs`, the selection-restore suite, the archetype drift test, and a headless `--api` transcript exercising set, add, create, undo, save.

## Batch 7: DRY in the editor UI

- `EditResult::assign(self, slot: &mut T, hint: &mut Option<&'static str>, name: &'static str)` replaces the 82 identical blocks in `component_editors.rs`, `behavior_editor.rs`, `ui_component_editors.rs`, `component_editors/grid_backdrop.rs` (a method, not a macro; about 250 lines gone).
- `EditableInspector::next_field() -> (FieldId, RowLayout)` and `advance(height)` replace the repeated preamble in the twelve field methods; one `remove_button` replaces `header_with_remove` plus `component_editors::remove_button`.
- An `InspectorFrame<'a>` context struct (ui, world, theme, drag state, scroll) replaces the long argument lists of `edit_texture_field`, `edit_all_components`, `render_dynamic_edit_blocks`, removing the last three `#[allow(too_many_arguments)]`.
- `panel_renderer/add_component_popup.rs`: one walk both renders and measures the typed and dynamic sections; `categorized_components()` is computed once per frame.
- Theme aliases removed: `bg_primary/bg_viewport/bg_input/bg_header` → `surface_1/surface_0/surface_3/surface_2` at their call sites, `pause_yellow` → `warn_yellow`, `inspector_header/label/value` → `accent_cyan/text_secondary/text_primary` (the `inspector_style()` converter reads the base tokens). The surface-ladder guard tests are the safety net.
- The seven hardcoded `8.0` and four `20.0` route through `layout::PADDING` and `layout::LINE_HEIGHT`; `rect_border` replaces the four hand-drawn lines at `panel_renderer/mod.rs:124`; `render_node` loses its duplicate child loop; one `draw_world_segments(ui, viewport, segments, color)` serves `grid.rs`, `collider_overlay.rs`, `selection_outline.rs`.
- `EditableFieldStyle` is borrowed rather than cloned per component per frame; `DockPanel.bounds`, `ViewportInputHandler.config`, `GridRenderer.config` go private with the setters that already exist (which lets the NaN guard at `context/mod.rs:307` go).

Size: about 400 removed, 200 added. Review: yes. Extra verification: editor tests, and an `editor_demo` visual pass because a wrong theme token is invisible to tests.

## Batch 8: SRP splits, engine side

- `scene_loader_components.rs` (346-line function) becomes `scene_loader_components/{mod, physics, ui, sprites}.rs` with one `build_<component>(data, world, assets) -> Result<_, SceneLoadError>` per arm; the `#[cfg(feature = "physics")]` split becomes two plain functions with no "suppress unused" tuples. `scene_serializer.rs` `extract_components` becomes `scene_serializer/components.rs` with one `<component>_data(world, entity) -> Option<ComponentData>` per type, so the loader/serializer pairing is visible for the first time.
- `game/render.rs` `render_frame` → `collect_game_sprites`, `collect_ui_sprites`, `submit_frame` (the former Phase 1/2 headers). `game/frame_tail.rs` `post_update` → `step_simulations`, `draw_scene_ui`, `apply_window_requests`; the kept reasons (time-scale freeze, splice order) move to the doc comment of the method they explain.
- `ecs/src/hierarchy_system.rs` `update` (105 lines) splits into collect-dirty and propagate phases.
- `renderer/src/bloom.rs`: `ensure_ready(device, queue, targets, config)` plus a sequence-only `run`. `sprite/pipeline.rs` `new_with_target` (183 lines) → `sprite/pipeline/builders.rs` with `texture_layout`, `quad_geometry`, `instance_buffer`, `render_pipeline` (the file sits at 552 lines, so the split is required anyway).
- `GameRunner`: extract `Localization { strings, base_font, locale_fonts }` only; the other clusters are deferred.
- Examples: `hello_world.rs` `update` (185 lines) → `handle_player_input`, `update_camera`, `handle_pickups`, `draw_hud`; `init` (143) → `spawn_level`, `spawn_player`, `spawn_ui`. `editor_demo.rs` shares the platformer through `#[path = "hello_world/platformer.rs"] mod platformer;` instead of a hand-synced copy. These are the files game authors copy, so their shape propagates.

Size: about 900 lines moved. Review: yes. Extra verification: a golden scene test (load `examples/assets/scenes/hello_world.scene.ron`, save, compare to a checked-in expected RON), games test gate (breakout's level tests load scenes through the loader), a `hello_world` run.

## Batch 9: SRP splits, editor side

- `EditorGame::update` (117 lines, phases 0 through 12 with no 8) becomes named phase methods: `freeze_engine_time`, `sync_dirty_flag`, `run_chrome`, `run_viewport_and_gizmo`, `delegate_to_game`, `publish_window_title`, `clip_engine_ui`. One dirty-sync point if handler order allows, otherwise two with the reason named in the doc comment. `ApiSession { receiver, batch }` and `PendingDialog { action, choice }` pull four fields out of the 19-field struct.
- `command_api/write.rs` `run` (267 lines) → `command_api/write/verbs.rs` with one function per verb; `parse.rs` `parse_line` (155) → one parser per verb.
- `shortcuts.rs` `handle_play_action` (158) and `dispatch_editor_action` (140) grouped by category: `dispatch_edit_action`, `dispatch_file_action`, `dispatch_view_action`, `dispatch_play_action`.
- `viewport_interaction.rs` `handle_shared_viewport_input` (104 lines, three world queries): a `PickableCache` built once per frame and shared with `panel_renderer/mod.rs:98` (four queries become one), then `gate_chrome`, `handle_framing`, `handle_pick`, `handle_marquee`.
- `behavior_editor.rs` `edit_behavior` (187, already shrunk by `assign`) → per-variant editors. `render_inspector_editable` (182, popup already extracted) → `render_component_blocks` + `render_add_section`. `render_asset_browser` (162) → `render_folder_tree` + `render_file_grid`. `hierarchy.rs` `render_node` (127) → `render_row` + `render_children`.
- `stored_component/kind.rs` takes `ComponentKind`, `ComponentCategory`, `categorized_components`; `render_dynamic_edit_blocks` moves to `dynamic.rs`, leaving `mod.rs` as the registry macro and its re-exports.

Size: about 1000 lines moved. Review: yes. Extra verification: all editor suites, the headless API tests in `headless.rs`, and an `editor_demo` manual pass by Jesse (play/pause/stop, undo merge on inspector drag, marquee, gizmo drag with Escape).

## Batch 10: docs and guides

- The crate `CLAUDE.md` files. All ten carry counts or file maps, not just the five the systems audit named: `renderer` (claims 73), `ui` (119), `input` (79), `physics` (66), `audio` (27), `ecs` (211), `common` (44), `editor` (438), `engine_core` (381), `ecs_macros` (4). Drop every test count in favour of the `cargo test -p <crate>` command. Drop File Map lines that restate the `mod` tree. A line survives only if it says something `ls` and `grep mod` cannot: an invariant, a footgun, a decision of record (the theme-ladder tests are the spec; GPP-02 on `ComponentStore`; the physics scale rule). The per-crate Godot oracle tables stay.
- Root `CLAUDE.md` and `training.md`: three managers, no test counts (keep the "0 failed, 0 ignored" invariant), the collision-bus sentence gone, `run_game` returns `EngineError`, the deleted APIs gone from the directory map. The root file's status narrative is not restructured here (see deferred).
- `log_archive.md`: the physics pass-through changelog, the deleted `docs/plans` note, and one lesson per batch worth keeping.
- #84 DOC-001 doc gaps (`behavior_to_data`, the former `ui_manager::begin_frame`, the `lines` buffer contract).
- Issue bookkeeping: close #82 (batch 4); tick DRY-011 and DOC-001 on #84, DRY-004 on #86, DRY-006 on #89, ARCH-101 on #90, KISS-002 on #91; file the deferred issues listed below with the `file-issue` skill.

Size: about 300 removed, 120 added, all Markdown. Review: skip (the only batch under the hook's threshold in spirit; if it exceeds 100 lines the hook still fires, and a docs-only review is quick).

## Cross-crate ripple into ../games

No batch renames a type the games import. Items shaped to avoid ripple:

| item | why it does not ripple |
|---|---|
| `run_game` returns `EngineError` | games call `.unwrap()` or format `{e}`; both compile unchanged |
| `AchievementError`, `ScoresError` | become type aliases of `PersistenceError` |
| `GridMesh` | builders and `new` kept; the tuning fold waits for #96 |
| `Sprite.shape` enum | no game reads or writes the field |
| `PhysicsConfig` presets | games use the named constructors, whose values are unchanged |
| `#[must_use]` on `unlock` | skipped; 34 discarded results in the games |
| `Behavior` handlers | crate-private signatures; the `Behavior` enum itself is unchanged |
| `CollisionData` | asteroids consumes the `Vec` from `take_collision_events`, which stays |

Sequencing rule: every batch touching a prelude-exported item runs the games check gate before review; batches 1, 4 and 5 run the games test gate.

## Deliberately not doing (file as issues in batch 10)

- `GameContext` writeback fields to methods: 31 direct writes in the games, the footgun is documented in `CLAUDE.md`, churn outweighs clarity.
- Full `GameRunner` regrouping (33 fields), `World` surface (41 methods), `EditorContext` delegation shell, `Renderer` FrameGraph split: restructures without a failing behaviour; the audit itself says not to rewrite `World`.
- `PhysicsWorld` split (#85 SRP-001): rapier's API needs the breadth; stays on #85 as recorded.
- `EntityId` by-value vs by-ref inconsistency: documented convention that ripples into every `add_component(&entity, …)` in every game.
- `ComponentRegistry` `Result<_, String>` to a typed `RegistryError`: moderate value, its own small issue.
- `ui` depending on `input` (winit in the UI crate): an inversion is its own design pass.
- `render_*` names that mutate: immediate-mode convention; the returned response types already say what happened.
- WGSL identifier renames and the `panic!` in the wgpu uncaptured-error callback: no headless verification; the panic is deliberate, debug-only and commented.
- Bool parameters to enums (`header_with_remove`, `binding_pressed`, `cycle_step`, `with_resizable`): no clarity gain for the churn.
- `assets.rs` → `texture_assets.rs`, `editable_inspector.rs` rename, `grid/build.rs` → `sync.rs`: file moves without behaviour change.
- `GridMesh` holding a `GridTuning` shared with `GridBackdrop`: after #96 lands.
- Merge policy moved from commands into `CommandHistory`: a redesign, not cleanup.
- Cross-frame `build_pickable_entities` caching beyond one frame, `world.entities()` allocation (#86 GPP-L1), per-contact `Vec` reuse (#85 GPP-L10): profile first.
- Editor test findings (§8: reimplemented-logic tests, constructor echoes, selection duplicates, four test-module conventions): the separate test-suite workstream.
- Root `CLAUDE.md` status-report trimming and the four in-repo agent-tool mirrors (#94): separate decisions for the user.
- Settled and untouched: `ComponentStore` HashMap (GPP-02), the cfg-split native/wasm frame drivers, rodio, `editor_component_registry!`.

## Adjudication points for the user before batch 2 runs

1. The four unused audio methods (`stop_all`, `unload_all`, `active_sound_count`, `play_music_once`): delete, or keep as intended product surface. The plan deletes them.
2. The physics event-bus mirror (documented in training.md, read by nobody): delete with its sentence. The plan deletes it.
