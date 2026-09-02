# Code quality audit — `crates/editor` + `crates/editor_integration`

Scope: `crates/editor/src` (72 files, ~21.5k lines) and `crates/editor_integration/src` (28 files, ~7.3k lines).
Read-only audit. Nothing was edited.

## Headline

Both crates are in genuinely good shape on the mechanical axes:

- **Zero `unwrap()` / `expect()` outside tests** (the only production hit, `collider_overlay.rs:406`, is inside a `#[cfg(test)]` module).
- **Three `#[allow(...)]` total**, all `clippy::too_many_arguments` (`texture_field.rs:31`, `stored_component/mod.rs:297`, `stored_component/mod.rs:513`).
- **No `static`, `OnceLock`, `thread_local`, or `lazy_static` anywhere** in either crate. No Service Locator.
- No `#[ignore]`, no ` ```ignore ` doc examples.

The problems are structural: duplicated dispatch, dead public API, five competing colour palettes, and 235 issue-number references baked into source comments.

---

## 1. DRY violations

### 1.1 `impl_set_component_command!` generates 13 byte-identical structs
`crates/editor/src/commands/set_commands.rs:73`

The macro emits `{ entity, old, new, field_hint }` + `execute` + `undo` + `display_name` + `try_merge` + two `as_any` impls, 13 times. The only variation is `$ty` and a display string.

`SetComponentValueCommand` (`commands/component_commands.rs:224`) already does exactly this job generically over `StoredComponent` for the command API — so the crate has both a macro-expanded typed version and a dynamic version of one concept.

**Fix:** a single generic `SetComponentCommand<T: Component + Clone>` carrying `display: &'static str` and `field_hint`. Merge isolation survives, because `SetComponentCommand<Sprite>` and `SetComponentCommand<Transform2D>` are distinct types and `downcast_ref::<Self>()` still discriminates.

### 1.2 Dynamic Add/Remove commands mirror the typed pair line-for-line
`crates/editor/src/commands/component_commands.rs:124-213` vs `:16-115`

`AddDynamicComponentCommand` / `RemoveDynamicComponentCommand` are structural copies of `AddComponentCommand` / `RemoveComponentCommand`, differing only in a `String` key vs a `ComponentKind` and the absence of the RigidBody→Collider cascade.

**Fix:** one `ComponentRef { Typed(ComponentKind), Dynamic(String) }` and a single Add/Remove pair.

### 1.3 `TransformGizmoCommand` is `SetTransformCommand` without a field hint
`crates/editor/src/commands/set_commands.rs:22`

Same fields, same `execute`/`undo`. Only `try_merge` differs (entity-only vs entity + field_hint). Fold into the generic Set command with `field_hint: "gizmo"`.

### 1.4 The command API re-implements the GUI's hard physical floors
`crates/editor/src/command_api/write.rs:170` `sanitize()`

The scale `.max(Vec2::splat(0.01))`, collider extent `.max(0.5)`, volume `.clamp(0.0, 1.0)` and pitch `.max(0.1)` rules all exist inline in the GUI editors:
`component_editors.rs:147`, `:174`, `:284`, `:292`, `:300`, `:306`, `:367`, `:371`.

The function's own doc comment says *"Mirror the GUI's hard physical floors"* — the code admits the duplication. Two copies of what a legal collider is, held together only by a comment.

**Fix:** one `clamp_to_physical_floors` per component type, called by both paths.

### 1.5 Two dispatch tables for the same 14 user-facing actions
`crates/editor_integration/src/editor_game/menu_actions.rs:42-135`
`crates/editor_integration/src/editor_game/shortcuts.rs:271-395`

Undo, Redo, Save, SaveAs, NewScene, OpenScene, Cut, Copy, Paste, Delete, Duplicate, ToggleGrid, ToggleColliders and ToggleSnap each appear in both, **already with divergent behaviour**: the menu path shows a status message on Undo/Redo (`menu_actions.rs:73`, `:81`), the shortcut path does not (`shortcuts.rs:272`, `:281`).

**Fix:** map menu labels to `EditorAction` and route both through `dispatch_editor_action`.

### 1.6 "Collapse N commands into one undo entry" is written four times
- `editor_game/viewport_interaction.rs:460`
- `editor_game/menu_actions.rs:156` (delete)
- `editor_game/menu_actions.rs:238` (paste)
- `editor_game/menu_actions.rs:273` (cut)

Identical `match len { 0 => {}, 1 => push, _ => push(MacroCommand::new(name, cmds)) }`.

**Fix:** `CommandHistory::push_as_one(name, commands)`.

### 1.7 "Is either Ctrl held" appears five times, three spellings
- `editor_integration/src/editor_game/shortcuts.rs:206`
- `editor/src/editor_input.rs:341`
- `editor_integration/src/editor_game/viewport_interaction.rs:108` (inline)
- `editor_integration/src/editor_game/viewport_interaction.rs:526` (a `ctrl_held()` helper the *same file* ignores 400 lines earlier)
- `editor_integration/src/panel_renderer/mod.rs:179`

Shift is duplicated three times alongside it (`shortcuts.rs:208`, `editor_input.rs:342`, `panel_renderer/mod.rs:181`).

### 1.8 Two implementations of the component remove `[X]` button
`crates/editor/src/editable_inspector.rs:235` `header_with_remove` and
`crates/editor/src/component_editors.rs:430` `remove_button`

Both use `btn_size = 18.0`, both call `remove_button_x`, both build `FieldId::new(component_index, 99, 0)`, and both carry the same comment about avoiding ID collisions.

### 1.9 The `hint = Some(...)` field-edit block appears 96 times
| file | occurrences |
|---|---|
| `editor/src/component_editors.rs` | 35 |
| `editor/src/behavior_editor.rs` | 27 |
| `editor/src/ui_component_editors.rs` | 19 |
| `editor/src/component_editors/grid_backdrop.rs` | 15 |

82 of the 96 are literally `if let EditResult::Changed(v) = inspector.<kind>(label, value, range) { new.field = v; hint = Some("field"); }`.

**Fix:** a three-argument declarative macro or a `FieldSink` accumulator. Removes ~250 lines and makes `edit_behavior` (187 lines) tractable.

### 1.10 Ten `EditableInspector` methods repeat the same preamble/postamble
`crates/editor/src/editable_inspector.rs:274-498`

Every one of `texture`, `f32`, `f32_hard`, `angle`, `bool`, `vec2`, `u32`, `string`, `action_button`, `string_edit`, `cycle`, `color` does: build `FieldId::new(self.component_index, self.field_index, 0)`, call `self.row()`, bump `field_index`, bump `current_y`.

**Fix:** `fn next_field(&mut self) -> (FieldId, RowLayout)` + `fn advance(&mut self, height: f32)`.

### 1.11 `build_add_patch_set` duplicates the body of the `Set` verb
`crates/editor/src/command_api/write.rs:80` vs `:241-245`

Both do `current_value` → `merge_patch` → `stored_component_from_json` → `sanitize` → `validate_texture_handles`.

### 1.12 The API frame hook parses every query line twice
`crates/editor_integration/src/editor_game/api.rs:45` then `:57`

`answer_api_lines` calls `parse_line(line)`, then for queries hands the **raw string** to `command_api::dispatch_line`, which parses it again. The comment at `:55` acknowledges it. Two parse paths that must agree.

### 1.13 `api.rs` re-implements `WriteCtx::record`
`crates/editor_integration/src/editor_game/api.rs:144-147` vs `crates/editor/src/command_api/write.rs:52`

The same "append to the open batch, else push onto the history" branch, inline.

### 1.14 `scene_io.rs` re-implements `World::clear()` twice
`crates/editor_integration/src/editor_game/scene_io.rs:154-159` and `:252-255`

Both do `for entity in world.entities() { world.remove_entity(&entity).ok(); }`. `world_snapshot.rs:119` calls the real `World::clear()`.

The same seven-line "reset the editor session" block (set_dirty, fresh CommandHistory, `api_batch = None`, selection.clear, `gizmo_drag = None`, `gizmo.cancel`) also appears in both `load_scene` (`:179-186`) and `new_scene` (`:257-268`).

### 1.15 Two implementations of "what would a capture lose"
`crates/editor/src/clipboard.rs:57` `uncaptured_component_names`
`crates/editor/src/world_snapshot.rs:85` (inline in `capture`)

Both diff `world.component_types(entity)` against `registered_component_type_ids()`, both special-case `Parent`/`Children`, both dedupe and sort.

### 1.16 The add-component popup renders and measures the same layout twice
`crates/editor_integration/src/panel_renderer/inspector.rs`

- `:226-255` renders the typed categories; `:258-281` renders the dynamic "Game" section with the same label + button + `y += 24.0` loop.
- `:300` `categorized_popup_height` re-walks `categorized_components()` to recompute the row math the render loop performs.
- `:291` `dynamic_section_height` does the same for the dynamic half.

Three places must stay in sync, and `categorized_components()` allocates a fresh `Vec<(Category, Vec<Kind>)>` on each of two calls per frame.

### 1.17 `loss_warning` and `drop_report` differ only in their message string
`crates/editor/src/world_snapshot.rs:141` and `:154`

Same emptiness guard, same count, same `display_names(...).join(", ")`.

### 1.18 `render_node` duplicates its own child-recursion loop
`crates/editor/src/hierarchy.rs:313-320` (the off-screen early return) vs `:412-419`

### 1.19 Three overlay renderers share one unfactored shape
`grid.rs:329`, `collider_overlay.rs:144`, `selection_outline.rs:87`

All do: `push_clip_rect` → map world segments through `viewport.world_to_screen` → `ui.line` → `pop_clip_rect`.

---

## 2. SRP violations

### 2.1 Functions over 60 lines — 31 of them

| lines | location |
|---|---|
| 267 | `editor/src/command_api/write.rs:212` `run` |
| 187 | `editor/src/behavior_editor.rs:41` `edit_behavior` |
| 182 | `editor_integration/src/panel_renderer/inspector.rs:109` `render_inspector_editable` |
| 162 | `editor_integration/src/panel_renderer/asset_browser.rs:45` `render_asset_browser` |
| 158 | `editor_integration/src/editor_game/shortcuts.rs:21` `handle_play_action` |
| 155 | `editor/src/command_api/parse.rs:100` `parse_line` |
| 146 | `editor/src/viewport_input.rs:127` `handle_input` |
| 140 | `editor_integration/src/editor_game/shortcuts.rs:260` `dispatch_editor_action` |
| 135 | `editor/src/command_api/specs.rs:22` `command_docs` |
| 127 | `editor/src/theme/mod.rs:203` `default` |
| 127 | `editor/src/hierarchy.rs:299` `render_node` |
| 117 | `editor_integration/src/editor_game/mod.rs:349` `update` |
| 116 | `editor_integration/src/panel_renderer/mod.rs:37` `render_scene_view` |
| 105 | `editor_integration/src/editor_game/menu_actions.rs:35` `handle_menu_bar` |
| 104 | `editor_integration/src/editor_game/viewport_interaction.rs:76` `handle_shared_viewport_input` |
| 99 | `editor/src/component_editors.rs:252` `edit_collider` |
| 99 | `editor/src/component_editors/grid_backdrop.rs:33` `edit_grid_backdrop` |
| 97 | `editor/src/script_editor.rs:101` `edit_one_param` |
| 96 | `editor_integration/src/editor_game/api.rs:89` `run_hosted_write` |
| 84 | `editor/src/composite_rows.rs:88` `edit_color` |
| 82 | `editor/src/play_controls.rs:104` `render` |
| 82 | `editor/src/gizmo/mod.rs:497` `render_scale` |
| 79 | `editor/src/menu/mod.rs:396` `render_dropdown_static` |
| 77 | `editor_integration/src/panel_renderer/inspector.rs:17` `render_inspector` |
| 76 | `editor_integration/src/editor_game/scene_io.rs:57` `save_scene_with` |
| 75 | `editor/src/grid.rs:185` `grid_segments` |
| 75 | `editor_integration/src/editor_game/viewport_interaction.rs:292` `handle_gizmo` |
| 73 | `editor_integration/src/editor_game/scene_io.rs:133` `load_scene` |
| 72 | `editor/src/menu/mod.rs:177` `editor_default` |
| 70 | `editor/src/composite_rows.rs:18` `edit_vec2` |

`command_api/write.rs:212 run()` is the worst: one match arm per verb, each doing validation, mutation, history recording and JSON response building.

### 2.2 `EditorGame` is a 19-field god object
`crates/editor_integration/src/editor_game/mod.rs:40`

Holds: the inner game, editor context, transform system, font-loaded flag, world snapshot, entity counter, command history, gizmo drag, clipboard, physics settings, editing camera stash, editor font, game base font, frozen time scale, last window title, API receiver, pending scene action, pending dialog choice, initial scene, API batch.

At minimum separable: the API pair (`api_rx`, `api_batch`) and the confirm-dialog pair (`pending_scene_action`, `pending_dialog_choice`).

### 2.3 `EditorGame::update()` is a 117-line numbered phase list — and the numbering has drifted
`crates/editor_integration/src/editor_game/mod.rs:349`

Phases run: `0, 0b, 0c, 0d, 1, 1b, 2, 2b, 2c, 3, 4, 4b, 5, 6, 7, 9, 9b, 10, 11, 12`. **There is no phase 8.** The numbers are a comment-maintained ordering that named private methods would carry for free.

### 2.4 `EditorContext` mixes UI widget state with domain state
`crates/editor/src/context/mod.rs:25` — 25 fields.

UI widgets: `toolbar`, `menu_bar`, `dock_area`, `hierarchy`, `status_bar`, `play_controls`, `drag_drop`, `asset_browser`, `fonts`, `inspector_scroll`.
Domain: `selection`, `scene_path`, `is_dirty`, `play_state`, `camera_follow`, `snap_to_grid`.

It is also largely a delegation shell: 10 methods forward to `SceneViewport` (`:188-231`), 6 to `GridRenderer` (`:237-259`), 8 wrap `EditorPlayState` (`:321-382`).

### 2.5 `SceneViewport` does three jobs
`crates/editor/src/viewport/mod.rs:18`

Camera state + interpolation, coordinate conversion, **and** sprite generation for the renderer (`generate_entity_sprite:273`, `batch_entities:295`) — both of which are dead (see §7.1) and are the only reason the file imports `renderer::sprite::{Sprite, SpriteBatcher}`.

### 2.6 `handle_shared_viewport_input` does six things and queries the world three times
`crates/editor_integration/src/editor_game/viewport_interaction.rs:76`

Chrome gating, camera-follow breaking, framing shortcuts, picking, marquee drawing, marquee application. It calls `build_pickable_entities(ctx.world)` at `:117`, `:134` and `:217` — three full `query_entities` passes plus three `Vec` allocations in one frame. `panel_renderer/mod.rs:98` makes a fourth.

### 2.7 `stored_component/mod.rs` is a `mod.rs` holding 475 lines of macro body
Rather than re-exports. Same for `commands/mod.rs` (holds the whole `CommandHistory`) — defensible, but `stored_component/mod.rs` also holds `ComponentCategory`, `ComponentKind`, `render_dynamic_edit_blocks`, `restore_components`, `available_components` and `categorized_components`.

### 2.8 `Gizmo::render` mutates drag state, hit-tests, and draws
`crates/editor/src/gizmo/mod.rs:339` — three responsibilities behind a name that promises one.

---

## 3. KISS violations

### 3.1 `editor::prelude` has zero users
`crates/editor/src/lib.rs:140`

A 20-line module duplicating ~50 of the crate's top-level re-exports. `grep -rn 'editor::prelude'` across the whole workspace returns nothing.

### 3.2 Three-level entry-point chain, middle link has one caller
`crates/editor_integration/src/editor_game/mod.rs:524` → `:532` → `:551`

`run_game_with_editor` → `run_game_with_editor_api` → `run_game_with_editor_opts`. Two entry points suffice (`run_game_with_editor` and `run_game_with_editor_opts`).

### 3.3 Macro verdicts

- **`impl_set_component_command!` is NOT justified** — see §1.1. It does a generic's job.
- **`editor_component_registry!` IS justified.** It generates an enum plus nine methods (`apply_to`, `capture_all_components`, two `type_ids` fns, `ComponentKind` + 5 methods, `edit_all_components`, `inspect_all_components`, `settable_component_names`, `stored_component_from_json`, `capture_component_by_name`, `type_name`, `capture_all_values`) from one declarative list. Rust cannot express enum-variant generation with generics, and the design demonstrably works — adding `GridBackdrop` was one line. **Keep it.**

### 3.4 Five separate hardcoded colour palettes, one of which disagrees

| location | palette |
|---|---|
| `editor/src/theme/mod.rs:203` | `EditorTheme::default()` |
| `editor/src/gizmo/mod.rs:146` | `GizmoPalette::default()` |
| `editor/src/grid.rs:27` | `GridColors::default()` |
| `editor/src/inspector.rs:28` | `InspectorStyle::default()` |
| `editor/src/field_style.rs:112` | `EditableFieldStyle::default()` |

`GridColors::default()` matches the theme exactly (pure duplication). **`GizmoPalette::default()` does not**: its X axis is `(0.9, 0.2, 0.2)` where `EditorTheme.gizmo_x` is `(1.0, 0.0, 0.0)`; its Y is `(0.2, 0.9, 0.2)` vs `(0.0, 1.0, 0.0)`. `InspectorStyle` and `EditableFieldStyle` both define `label_color (0.7,0.7,0.7)`, `value_color WHITE`, `header_color (0.9,0.9,0.5)` — a third and fourth copy.

The crate guide says "never hardcode colors in panels" — the *defaults* are where they leaked back in.

### 3.5 Redundant theme tokens
`crates/editor/src/theme/mod.rs`

- `pause_yellow` (`:283`) and `warn_yellow` (`:288`) are both `0xffcc00`.
- `inspector_header` (`:309`) is `accent_cyan` (`:235`).
- `inspector_label` (`:307`) is `text_secondary` (`:243`).
- `inspector_value` (`:308`) is `text_primary` (`:242`).
- `bg_primary` / `bg_viewport` / `bg_input` / `bg_header` (`:228-231`) are aliases of `surface_1` / `surface_0` / `surface_3` / `surface_2`, with a comment admitting they exist "to avoid churning 30+ call sites this sprint".

### 3.6 Dead parameter threaded through two functions
`crates/editor/src/inspector.rs:110` and `:152`

`depth: usize` is passed into `render_value`, forwarded to `render_field`, incremented to `depth + 1` at `:168` — and never read in any condition.

### 3.7 `InspectorStyle` carries four fields nothing reads — and the read-only inspector ignores the theme
`crates/editor/src/inspector.rs`

`inspect_component` and its helpers consult only `style.line_height` and `style.indent`. `padding`, `label_color`, `value_color` and `header_color` are never used — every label goes through the bare `ui.label()` (zero `label_styled` calls in the file).

Consequence: `EditorTheme::inspector_style()` (`theme/mod.rs:350`) computes three colours for nothing, and **the read-only inspector users see during Play renders unthemed**. This is a visible bug hiding as dead configuration.

### 3.8 `layout.rs` is a constants module nobody uses
`crates/editor/src/layout.rs`

Its own doc says *"Use these constants instead of magic numbers throughout the editor."*

- `PADDING` is imported in exactly one file (`hierarchy.rs:12`) while `8.0` is hardcoded in seven others: `panel_renderer/mod.rs:19`, `:39`, `panel_renderer/inspector.rs:24`, `panel_renderer/asset_browser.rs:25`, `status_bar.rs:129`, `inspector.rs:31`, `field_style.rs:116`.
- `LINE_HEIGHT` is imported once while `20.0` is hardcoded at `panel_renderer/inspector.rs:23`, `:101`, `:118` and `inspector.rs:32`.
- **Zero users:** `PADDING_SMALL`, `SPACING`, `MENU_BAR_HEIGHT`, `MENU_ITEM_HEIGHT`, `TOOLBAR_HEIGHT`, `TOOLBAR_BUTTON_SIZE`.

### 3.9 Two sources for the dock header height
`crates/editor/src/dock/mod.rs:171` stores `header_height: f32` as a field (read by `dock/render.rs:107`), while `DockPanel::content_bounds()` at `:150` and `effective_size()` at `:141` read the free `HEADER_HEIGHT` constant. Changing the field would silently desync content bounds from the drawn header.

### 3.10 Five zoom setters, three position setters
`crates/editor/src/viewport/mod.rs`: `set_camera_zoom:120`, `adopt_camera_zoom:131`, `set_target_zoom:138`, `zoom_at:146`, `reset_camera_immediate:173`; `set_camera_position:83`, `set_target_camera_position:89`, `pan:104`, `pan_immediate:109`. Two of these are dead (§7.1).

### 3.11 A rectangle border drawn as four hand-written lines
`crates/editor_integration/src/panel_renderer/mod.rs:124-147` — when `ui.rect_border` exists and is used elsewhere in the same crate (`viewport_interaction.rs:198`, `panel_renderer/asset_browser.rs:146`).

### 3.12 Four near-duplicate dock mutators
`crates/editor/src/dock/mod.rs:224/232` (`set_panel_visible` / `toggle_panel_visible`) and `:240/250` (`set_panel_collapsed` / `toggle_panel_collapsed`).

---

## 4. Non-human-readable names

Better than expected. The standard abbreviations the brief listed (`mgr`, `cfg`, `tex`, `dt`, `idx`, `buf`, `ent`, `comp`, `sel`) are **essentially absent** — one hit in the whole cluster: `let btn` at `crates/editor/src/play_controls.rs:126`. The real offenders are elsewhere.

### 4.1 Counts

| category | editor | editor_integration |
|---|---|---|
| single-letter closure params in production (`\|p\|`, `\|e\|`, `\|c\|`, `\|s\|`, `\|k\|`, `\|t\|`) | ~120 | ~27 |
| classic abbreviations (`mgr`/`cfg`/`tex`/`idx`/`buf`/`ent`/`comp`/`sel`) | 1 | 0 |
| single-letter `let` bindings in production (excl. math/tests) | 6 | 1 |

### 4.2 Worst offenders

- `editor/src/dock/render.rs:60` `let c = bounds.center();`
- `editor/src/dock/render.rs:253` `let b = panel.bounds;`
- `editor/src/dock/render.rs:313` `let c = theme.accent_cyan;`
- `editor/src/confirm_dialog.rs:66` `let w = 88.0;`
- `editor/src/script_editor.rs:199` `let mut n = 1usize;`
- `editor/src/viewport/mod.rs:259` `let pc = self.viewport_center();`
- `editor_integration/src/panel_renderer/mod.rs:122` `let w = if editor.in_play_session() { 3.0 } else { 1.0 };` — a line **width** named `w`, two lines below `bounds.width`.

### 4.3 Names that lie — `render_*` functions that mutate

| function | what it also does |
|---|---|
| `editor/src/gizmo/mod.rs:339` `Gizmo::render` | starts and ends drags, clears the cancel latch, releases stale handles |
| `editor/src/hierarchy.rs:241` `HierarchyPanel::render` | mutates `collapsed`, `renaming`, `visible_order`, scroll offset |
| `editor/src/dock/render.rs:80` `DockArea::render` | mutates panel `collapsed` state |
| `editor/src/toolbar.rs:136` `Toolbar::render` | returns a tool change |
| `editor_integration/src/panel_renderer/inspector.rs:109` `render_inspector_editable` | executes undo commands, calls `break_merge`, shows status messages |

Idiomatic for immediate-mode UI, but the names should say so (`draw_and_interact`, or a returned response type as `HierarchyResponse` already does).

### 4.4 Module / file names that don't describe their contents

- `editor/src/editable_inspector.rs` — the widget functions now live in `field_style.rs`, `row_layout.rs`, `composite_rows.rs`, `text_field.rs`, `texture_field.rs`. The name no longer partitions the concept.
- `editor/src/stored_component/mod.rs` — also holds `ComponentKind`, `ComponentCategory`, `edit_all_components`, `inspect_all_components`, `categorized_components`. The name describes one of five exports.
- `editor_integration/src/entity_ops.rs:312` — declares its tests as `mod tests_file` via `#[path]`. A module named after its file rather than its contents, and the only place in either crate using that pattern.

---

## 5. Comment load

### 5.1 Top 10 by narration-comment ratio
(`//` lines only, excluding `///` doc comments, excluding test files)

| ratio | narration | code | file |
|---|---|---|---|
| 0.29 | 97 | 330 | `editor_integration/src/editor_game/mod.rs` |
| 0.22 | 72 | 325 | `editor_integration/src/editor_game/shortcuts.rs` |
| 0.20 | 53 | 258 | `editor/src/theme/mod.rs` |
| 0.19 | 23 | 117 | `editor/src/commands/entity_commands.rs` |
| 0.16 | 41 | 252 | `editor_integration/src/panel_renderer/inspector.rs` |
| 0.13 | 54 | 405 | `editor_integration/src/editor_game/viewport_interaction.rs` |
| 0.13 | 25 | 179 | `editor_integration/src/editor_game/scene_io.rs` |
| 0.12 | 22 | 181 | `editor/src/commands/component_commands.rs` |
| 0.10 | 29 | 271 | `editor/src/hierarchy.rs` |
| 0.10 | 21 | 208 | `editor/src/commands/mod.rs` |

Including doc comments, `theme/mod.rs` reaches 0.63 and `commands/mod.rs` 0.62.

### 5.2 Three narration comments that structure should replace

```rust
// crates/editor_integration/src/editor_game/mod.rs:380
// 1. Run transform hierarchy system
self.transform_system.update(ctx.world, ctx.delta_time);
```

```rust
// crates/editor/src/commands/component_commands.rs:99
// Restore primary component.
if let Some(ref stored) = self.stored { stored.apply_to(world, self.entity); }
// Restore cascaded component.
if let Some(ref stored) = self.cascade_stored { stored.apply_to(world, self.entity); }
```

```rust
// crates/editor_integration/src/editor_game/scene_io.rs:252
// Clear existing world
for entity in world.entities() { world.remove_entity(&entity).ok(); }
```

### 5.3 Stale status references in source: 235 across ~50 files

| token | count |
|---|---|
| `kimi` | 55 |
| `#42` | 19 |
| `#59` | 18 |
| `#43` | 13 |
| `#53` | 10 |
| `#51` | 10 |
| `#52` | 9 |
| `#55` | 8 |
| `GPP-14` | 7 |
| `audit §9` | 7 |
| `#44` | 7 |
| `audit §3.3` | 6 |
| `#54` | 6 |
| `#45` | 6 |
| `#22` | 6 |
| `#66` | 5 |
| `#41` | 5 |
| `#32` | 5 |
| `audit §1.4` | 4 |
| `#56` | 4 |
| `#46` | 4 |
| `#24` | 4 |
| `audit §5.2/5.3/5.6/4.5/4.9` | 12 |
| `#39`, `#50`, `#7`, `#28`, `#34`, `#21` | ~12 |

The 55 `kimi` references (`kimi F1`, `kimi round 6 F2`, `kimi batch-2 F1`, `kimi plan-round F4`, `kimi #43 F2`) name a review counterpart and a finding number that mean nothing to a future reader. The useful half of each comment is the invariant it states, which almost always precedes it.

Also present in source: `theme/mod.rs:248` "caught by the sprint-4 visual pass"; `editor_game/mod.rs:456` "the last incarnation of the §4.1 bleed, caught by the Sprint 5 screenshot pass"; `panel_renderer/mod.rs:176` "kimi #51 F1"; `stored_component/mod.rs:164` "issue #43".

### 5.4 Two documentation bugs found while reading

**(a) An orphaned doc comment lands on the wrong field.**
`crates/editor/src/theme/mod.rs:33-40`

```rust
    // ── Backgrounds ─────────────────────────────────────────────
    /// Main panel backgrounds (`#1e1e1e`)
    // ── Surface elevation ladder (audit §5.2) ──────────────────────
    // surface_0 (lowest: viewport well) .. surface_4 (floating popups).
    // ...
    /// Elevation 0: the viewport well behind everything.
    pub surface_0: Color,
```

Plain `//` comments don't break a doc-comment run, so **both `///` lines attach to `surface_0`**, which is documented twice and contradictorily. `bg_primary` at `:53` ends up with no doc comment at all.

Several hex values in those docs are stale: `bg_primary` is documented `#1e1e1e` but is now `surface_1 = 0x2a2a2a`; `bg_input` says `#2d2d2d` but is `0x545454`; `bg_viewport` says `#000000` but is `0x0a0a0a`.

**(b) `new_scene`'s doc comment is attached to `default_scene_path`.**
`crates/editor_integration/src/editor_game/scene_io.rs:225-239`

The block opens "Create a new empty scene, clearing the world. / Refused during a play session (Playing or Paused)…" then switches mid-block to "Where 'Open Scene…'/'Save As…' default to:". It sits above `default_scene_path` at `:234`. `new_scene` at `:241` is undocumented.

---

## 6. Game Programming Patterns alignment

### 6.1 Command — clean, with one leak
`crates/editor/src/commands/mod.rs:94`

`CommandHistory` is a textbook implementation: id watermark for the dirty flag, selection before/after images (`HistoryEntry:82`), bounded history, `push_already_executed` for pre-applied changes, `push_already_executed_with_before` for cross-frame batches.

**Merging leaks into the commands.** Every command must implement `try_merge` and hand-roll a `downcast_ref`, and `field_hint: &'static str` exists on 13 command types purely to serve the history's merge policy. Merge policy belongs to the history, not to each command.

The `merge_sealed` flag + `break_merge()` escape hatch is sound, but is called from six scattered sites: `viewport_interaction.rs:432`, `:483`, `shortcuts.rs:43`, `editor_game/mod.rs:500`, `panel_renderer/inspector.rs:168`.

### 6.2 Dirty Flag — correct and well-tested
Id-of-top vs `saved_id` (`commands/mod.rs:177`), merges reassign the id (`:313`, `:336`), `clear()` resets the watermark (`:260`). `dirty_tests.rs` is a real contract (11 tests).

One wart: the per-frame mirror onto `EditorContext.is_dirty` is synced **twice** per frame (`editor_game/mod.rs:373` and `:434`) because handlers between the two points record commands.

### 6.3 State — `EditorPlayState` is fine, its guards are scattered
`is_playing()` is checked at 20+ call sites rather than the state owning what it permits. `menu_actions.rs:42-99` repeats `if !self.editor.is_playing()` as a match guard on ten separate arms.

### 6.4 Observer — absent, correctly
An immediate-mode editor has no use for it. `HierarchyResponse` (`hierarchy.rs:85`) and `ViewportInputResult` (`viewport_input.rs:59`) are the right shape: return what happened, let the caller decide.

### 6.5 Service Locator / global state — none
No `static`, `OnceLock`, `thread_local` or `lazy_static` in either crate. The one global — the ECS component registry — is reached through `ecs::with_global_registry` and owned by another crate. Notably good.

### 6.6 God objects — two
`EditorGame` (19 fields, §2.2) and `EditorContext` (25 fields, §2.4).

### 6.7 Stringly-typed dispatch — one unavoidable, one lazy

**Unavoidable:** component names as strings in the dynamic tier (`stored_component/dynamic.rs`, `command_api/write.rs:259-280`, `:327-360`). Game types are unknown at editor compile time; there is no alternative.

**Lazy — the archetype path stringifies twice:**
1. `command_api::ARCHETYPES` uses kebab names (`"static-body"`).
2. `editor_game/api.rs:228 archetype_action` maps them to **menu-bar labels** (`"Create Static Body"`).
3. `entity_ops.rs:150 handle_create_action` matches those labels again.

Three string vocabularies for nine fixed archetypes, guarded only by a drift test. One `Archetype` enum with `from_kebab()` / `label()` collapses all of it.

Menu-label matching generally is tracked as ARCH-101 in the crate guide — still open.

---

## 7. Rust best-practice issues

### 7.1 Dead public API (definition only — no caller, no test)

| location | item |
|---|---|
| `editor/src/viewport/mod.rs:273` | `generate_entity_sprite` |
| `editor/src/viewport/mod.rs:295` | `batch_entities` |
| `editor/src/theme/mod.rs:330` | `EditorTheme::dark()` |
| `editor/src/theme/mod.rs:335` | `EditorTheme::color_to_vec4()` |
| `editor/src/context/mod.rs:496` | `inspector_bounds` |
| `editor/src/context/mod.rs:501` | `hierarchy_bounds` |
| `editor/src/picking/mod.rs:273` | `reset_cycle` |
| `editor/src/picking/mod.rs:162` | `with_pick_margin` |
| `editor/src/grid.rs:165` | `set_axes_visible` |
| `editor/src/gizmo/mod.rs:232` | `set_axis_length` |
| `editor/src/viewport/mod.rs:192` | `set_interpolation_speed` |
| `editor/src/status_bar.rs:60` | `set_version` |
| `editor/src/toolbar.rs:102` | `with_button_size` |
| `editor/src/lib.rs:140` | the whole `prelude` module |

`generate_entity_sprite` + `batch_entities` form a 36-line "Entity Rendering" section that is the **only** reason `viewport/mod.rs` imports `renderer::sprite::{Sprite, SpriteBatcher}`. The crate guide notes the equivalent grid cleanup ("the sprite pipeline with zero callers is gone") — this instance survived.

### 7.2 Public API exercised only by its own test

| location | item |
|---|---|
| `editor/src/context/mod.rs:366` | `enter_play_mode` |
| `editor/src/context/mod.rs:371` | `exit_play_mode` |
| `editor/src/context/mod.rs:376` | `toggle_play_mode` |
| `editor/src/context/mod.rs:198` | `pan_camera` |
| `editor/src/menu/mod.rs:138` | `visible_item_count` |
| `editor/src/menu/mod.rs:475` | `close_all` |

Production uses `set_play_state` directly (`shortcuts.rs:88`, `:96`, `:104`, `:156`), so the three play-mode mutators are a parallel API nobody calls.

### 7.3 Vestigial statement
`crates/editor_integration/src/panel_renderer/inspector.rs:286` — `let _ = component_index;`, discarding a variable that is used 100 lines above it.

### 7.4 String-typed errors throughout the integration crate
`save_scene`, `save_scene_as`, `save_scene_with`, `load_scene`, `load_scene_with_feedback`, `run_headless_editor_api` all return `Result<_, String>` (`scene_io.rs:30`, `:43`, `:62`, `:138`; `headless.rs:127`). Every message is built with `format!`. No `impl Error`, no matchable variant. Callers re-wrap into another `format!` (`shortcuts.rs:290`, `menu_actions.rs:102`) — four near-identical error blocks.

### 7.5 `String` params where `&str` suffices
`crates/editor/src/commands/component_commands.rs:133` and `:181` — `new(entity: EntityId, name: String)`, which immediately stores it plus a `format!`-derived display string.

### 7.6 `pub` fields that should be private
- `editor/src/dock/mod.rs:81` `DockPanel.bounds` — layout **output**, written by `DockArea::layout()`, but publicly writable.
- `editor/src/viewport_input.rs:89` `ViewportInputHandler.config`
- `editor/src/grid.rs:123` `GridRenderer.config` — this is exactly why `context/mod.rs:307` needs an explicit "the setter's ≥1 clamp is not a guarantee" NaN guard.

### 7.7 Bool parameters that should be enums
- `editor/src/editable_inspector.rs:235` `header_with_remove(type_name, removable: bool)`
- `editor/src/editor_input.rs:339` `binding_pressed(binding, input, just: bool)`
- `editor/src/gizmo/mod.rs:314` `begin_drag_if(dragging: bool, …)`
- `editor/src/editable_inspector.rs:142` `cycle_step(index, count, forward: bool)`
- `editor/src/dock/mod.rs:124` `with_resizable(resizable: bool)`

### 7.8 Magic widget-ID arithmetic — three schemes in two files
- `FieldId::new(component_index, 99, 0)` for remove buttons: `editable_inspector.rs:253`, `component_editors.rs:441`.
- `FieldId::new(component_index + 50, 0, 0)` for the add button: `panel_renderer/inspector.rs:189`.
- `FieldId::new(component_index + 60 + popup_btn_idx, 0, 0)` for popup rows: `:245`, `:268`.

The +50/+60 gap allows exactly ten components before collision.

### 7.9 Avoidable per-frame work
- `build_pickable_entities` runs up to **four** times per frame (`viewport_interaction.rs:117`, `:134`, `:217`; `panel_renderer/mod.rs:98`), each a `query_entities` plus a `Vec` allocation.
- `categorized_components()` allocates a `Vec<(Category, Vec<Kind>)>` twice per frame while the popup is open (`panel_renderer/inspector.rs:226`, `:302`).
- `EditableInspector` clones `EditableFieldStyle` once per component per frame (`stored_component/mod.rs:52`, `:73`, `:97`, `:535`) — a 25-field struct.

### 7.10 Not found
No `Box<dyn>` where an enum would do (the `Box<dyn EditorCommand>` usage is correct — the set is open by design). No `Option<Option<>>`. No lossy `as` casts on real data. `clone()` counts are modest (max 9 in a production file, `script_editor.rs`).

---

## 8. Test observations

627 `#[test]` functions across 31 test files/modules. Quality is high in the command, clipboard, dirty-watermark, selection-restore and shortcut suites — those test contracts, not implementations.

### 8.1 Tests that assert against reimplemented production logic
`crates/editor/src/viewport_input.rs:328` and `:343`

`calculate_zoom_factor` and `screen_to_world_delta` are defined **inside the test module** and commented "mirrors the logic in `handle_input`". Four tests assert against these copies rather than the code: `test_zoom_factor_calculation:358`, `test_zoom_factor_inverted:366`, `test_screen_to_world_delta:372`, `test_screen_to_world_delta_with_zoom:382`.

`training.md` names this pattern explicitly as forbidden ("Reimplement production `if` logic inside the test").

### 8.2 Constructor-echo tests duplicating other crates' coverage
`crates/editor/src/component_editors.rs:480-520` — five `test_*_default_values` tests asserting `Default` on `Transform2D` (`common`), `Sprite` (`ecs`), `RigidBody` / `Collider` (`physics`), `AudioSource` (`ecs`). Each of those crates tests its own defaults.

### 8.3 Assert-free / redundant tests
- `editor/src/editable_inspector.rs:522` `test_field_id_creation` — the comment says "can't verify internal value without accessor". It asserts nothing.
- `editor/src/editable_inspector.rs:570` `test_editable_inspector_builder` — constructs a style and re-asserts exactly what `:529 test_editable_field_style_default` already asserts.
- `editor/src/grid.rs:368` `test_grid_renderer_new`, `:374` `test_grid_visibility_toggle`, `:387` `test_grid_size_setting` — constructor echoes.
- `editor/src/grid.rs:506` `test_calculate_grid_lines` — tests a private helper with `len() >= 5`.
- `editor/src/inspector.rs:320` `test_inspector_style_default` — asserts fields the module never reads (§3.7).

### 8.4 Near-duplicate tests in `selection.rs`
22 tests for 130 lines of code.
- `:388 test_selection_iterator` is a weaker duplicate of `:306 test_selected_iterates_in_insertion_order`.
- `:156 test_selection_new`, `:164 test_selection_select`, `:192 test_selection_add`, `:250 test_selection_clear`, `:238 test_selection_toggle`, `:208 test_selection_remove` are constructor echoes subsumed by the four behavior tests below them (`:319`, `:335`, `:351`, `:374`).

### 8.5 Four coexisting test-module conventions
1. Inline `#[cfg(test)] mod tests { … }` — most files.
2. `#[cfg(test)] mod tests;` submodule — `context`, `theme`, `dock`, `menu`, `gizmo`, `viewport`, `picking`, `commands`, `stored_component`, `world_snapshot`, `command_api`.
3. Crate-root sibling declared in `lib.rs` — `hierarchy_tests.rs`, `inspector_edit_tests.rs`.
4. `#[path = "entity_ops_tests.rs"] mod tests_file;` — `entity_ops.rs:312`, unique in the cluster.

### 8.6 Minor
Three distinct functions named `test_texture_path_fn` in different modules; two named `test_ranges_are_well_formed`. Harmless, but confusing in failure output.

Picking tests are split across crates (`editor/src/picking/tests.rs` for AABB/pick math, `editor_integration/src/editor_game/picking_tests.rs` for `build_pickable_entities` and marquee semantics) — correctly split, not duplicates.

---

## 9. Ranked — the 10 highest-value changes

1. **Unify the menu and shortcut dispatch tables** (`menu_actions.rs:42` + `shortcuts.rs:271`).
   Two implementations of 14 user-facing actions is the largest correctness risk in the cluster — they already differ in status-bar behaviour, and every new action must be added in two places. Map menu labels to `EditorAction` and route both through `dispatch_editor_action`.

2. **Collapse `impl_set_component_command!` into one generic `SetComponentCommand<T>`**, folding in `TransformGizmoCommand` and the dynamic Add/Remove pair.
   Removes ~250 lines and eliminates the "touch three files per new editable component" tax. Merge isolation is preserved by monomorphisation.

3. **Split `command_api/write.rs:212 run()` into per-verb functions.**
   A 267-line match is the worst SRP offender and sits in the file most likely to grow with command-API Stages C and D.

4. **Delete the dead API surface** (§7.1).
   `batch_entities` / `generate_entity_sprite` (which also drops a `renderer` dependency edge from `viewport/mod.rs`), `color_to_vec4`, `dark()`, `inspector_bounds`, `hierarchy_bounds`, `reset_cycle`, `with_pick_margin`, `set_axes_visible`, `set_axis_length`, `set_interpolation_speed`, `set_version`, `with_button_size`, and the zero-user `prelude`. Pure subtraction, no behavioural risk.

5. **One physical-floor module shared by `sanitize()` and the field editors** (§1.4).
   Today the GUI and the command API can silently disagree about what a legal collider or a legal audio volume is, and only a comment holds them together.

6. **Make `inspect_component` honour `InspectorStyle`'s colours — or delete the four unused fields** (§3.7).
   The read-only inspector users see during Play currently ignores the theme entirely. This is a visible bug hiding as dead configuration, and it is cheap either way.

7. **Replace the three archetype string vocabularies with one `Archetype` enum** (§6.7).
   Kebab name, menu label and factory match arm are three lists that must agree, guarded only by a drift test.

8. **Strip the 235 issue/review references from source comments**, keeping the invariant each one states (§5.3).
   Start with the 55 `kimi` mentions. Fix the two misattached doc comments (`theme/mod.rs:34`, `scene_io.rs:225`) and the three stale hex values in the same pass.

9. **Extract the add-component popup from `panel_renderer/inspector.rs` into its own module**, and derive its height from the same walk that renders it (§1.16).
   Removes a 182-line function and a two-place layout duplication that must currently be kept in sync by hand.

10. **Add `CommandHistory::push_as_one(name, commands)` and a shared `modifiers(input) -> (ctrl, shift)`** (§1.6, §1.7).
    Two tiny helpers that each delete four to five copies, and both are the kind of duplication that keeps re-appearing with every new feature.

### Honourable mentions (below the line, still worth filing)

- Cache `build_pickable_entities` once per frame instead of up to four times (§7.9).
- Route the seven hardcoded `8.0` paddings and four `20.0` line heights through `layout.rs`, or delete `layout.rs` (§3.8).
- Reconcile `GizmoPalette::default()` with `EditorTheme.gizmo_*` — they disagree today (§3.4).
- Replace `Result<_, String>` in `scene_io.rs` / `headless.rs` with a typed error (§7.4).
- Delete the four tests in `viewport_input.rs` that assert against reimplemented logic, and replace them with tests that drive `handle_input` (§8.1).
