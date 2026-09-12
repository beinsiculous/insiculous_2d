# Editor Crate — Agent Context

You are working in the editor crate. UI panels, tools, inspector, hierarchy — the editor's data and widgets.
This crate has NO dependency on engine_core. It depends on: ecs, ui, input, renderer, physics, common.

## Architecture
```
EditorContext (selection, tool state, play state, camera, theme, status_bar, fonts, inspector_scroll)
│   (CommandHistory itself lives on editor_integration's EditorGame and is threaded into panel renderers)
├── Panels: SceneView, Hierarchy, Inspector, AssetBrowser, Console
├── Dock layout: dock.rs (multi-panel docking, narrow mode below MIN_CENTER_WIDTH)
├── Menu / StatusBar (top + bottom chrome); Toolbar + PlayControls in the scene view's toolbar strip
├── Tools: Select, Move, Rotate, Scale (Q/W/E/R shortcuts)
├── Gizmos: Translate, Rotate, Scale handles
├── Picking: EntityPicker, SelectionRect, screen_to_world()
├── Inspector: Generic serde-based + per-component editors with writeback
│   (one renderer: `InspectorFrame.read_only` draws every row's value in
│   place of its control while a play session runs; sections collapse and
│   an Advanced disclosure hides the rarely-touched fields)
├── Undo/Redo: CommandHistory + EditorCommand trait, StoredComponent for restore
├── Theme: EditorTheme with 30+ color tokens (mockup-derived)
└── Play state: EditorPlayState (Editing/Playing/Paused), WorldSnapshot
```

## File Map
### State + chrome
- `context/` — EditorContext struct (selection, tools, state, view toggles, game frame dimensions, theme, fonts, inspector_scroll).
- `theme/` — EditorTheme: WCAG surface ladder `surface_0..surface_4` with luminance guard tests (≥1.35:1 adjacent / ≥3:1 border, overlay tones subdued against selection accent, game frame distinctness, grid axis hierarchy), muted overlay tokens (`game_frame`, subdued grid and colliders), style converters, and `ui_theme()`.
- `command_api/` — CLI/API dispatch (query list/describe/selection/scene/commands and write set/add/remove/rename/delete/select/undo/redo/batch) through CommandHistory; `docs/EDITOR_COMMAND_API.md`.
- `drag_drop.rs` — `DragDropState`/`DragPayload` (`Texture`, `Script`) cross-panel drag state machine (Idle→Armed→Dragging→Dropped-1-frame).
- `dock/` — multi-panel docking: state, layout, collapse/visibility toggles, chevrons, and clamped resize grabbers (a hovered or dragged handle draws its grabber line and asks `ui.request_cursor` for `ColResize`/`RowResize`, the dock having no window to ask itself). Below `MIN_CENTER_WIDTH` of centre it enters **narrow mode**: the side panels leave the edge allocation and one at a time shows as an overlay over the viewport on the floating band, opened from its header tab at the edge and closed from its own chevron (`narrow_overlay`, `open_narrow_overlay`, `close_narrow_overlay`); a collapsed panel opens as a full overlay (`expanded_content_bounds`) without touching its persisted collapse flag; the overlay runs to the dock's bottom, over a bottom panel.
- `toolbar_strip.rs` — the scene view's toolbar strip: `split` (panel content → strip + viewport), the `begin`/`end` chrome scope on `UiLayer::PanelChrome`, and `layout` placing the tools left, the play controls at the centre where it leaves them whole, and the view group (`VIEW_GROUP_WIDTH`, 140 px) at the right edge; below `TOOLBAR_STRIP_MIN_WIDTH` (391 px) the view group sheds whole and the tools shed one by one into the overflow menu (`overflow_menu.rs`).
- `menu/` — top menu bar; action items carry checked flag and map labels to `EditorAction`.
- `editor_input.rs` — shortcut chord model (exact chord beats any-mods) and `allowed_while_playing()` action deny list.
- `archetype.rs` — `Archetype`: the nine entity factories shared by the Entity menu and command API `create`.

### Inspector / components
- `editable_inspector.rs` — the width-aware `EditableInspector` walk: collapsible headers (the header row IS the toggle, in both play states), the `advanced()` disclosure, and the `read_only`/`collapsed` gates every field method runs first. `InspectorToggles` is what a block asks the view state for.
- `field_widgets.rs` — the standalone field widgets the walk places (label column, f32 soft-range, angle degree field with wrap, boolean, cycle row, component header).
- `read_only_rows.rs` — the read-only form of a row: label plus value in the muted colour, no widget id, the height its editable form would have taken.
- `inspector_state.rs` — `InspectorState`: collapsed sections, open Advanced disclosures, and the open colour editor (`ColorEditorTarget` — entity, registry type NAME, field index, never a walk-time index — plus the swatch anchor and value its row reports each frame, the seen mark, and the edit waiting for that row). One field on `EditorContext`; only the collapsed set persists.
- `color_editor_popup.rs` — the colour editor's widget (large swatch, four channels, a hex field) on the Modal band. It is drawn by a pass of its own BEFORE the panels (`editor_integration`'s `panel_renderer/color_editor.rs`), because a blocking rect is consulted at each widget's own `interact` and one pushed mid-walk cannot make the rows above it inert; the colour row still writes the edit, so one scrub is one undo entry.
- `color_hex.rs` — `#rrggbb`/`#rrggbbaa` text for a `Vec4`, shared by the hex field and the read-only colour row.
- `row_layout.rs` — row-layout math (`field_row`, `remove_button_x`, `pair_slots`, `ellipsize`; all horizontal placement goes through here, never hardcode offsets).
- `field_style.rs` — `FieldId` (widget-ID mapping), `EditableFieldStyle`, and typed `EditResult<T>` returns (keeps the editor crate free of an engine_core dependency); `WidgetSlot` inside component ID stride.
- `component_editors.rs` — per-component editors returning `Option<ComponentEdit<T>>`; shape cycling carries dimensions with commit-before-cycle ordering.
- `physical_floors.rs` — hard floors applied by inspector editors and command API `sanitize` (scale, collider extents, capsule half-height, volume, pitch).
- `behavior_editor.rs` — `edit_behavior()`: variant cycle selector and per-variant editors; `CameraFollow.dead_zone` stays read-only.
- `script_editor.rs` — `edit_scripts()`: `Scripts` component inspector editor, script catalog picker, parameter table, and Open Source button.

### Scene + selection
- `selection.rs` — Selection set (IndexSet preserving insertion order, deterministic primary fallback).
- `hierarchy/` — hierarchy panel tree view, F2 inline rename, `RowGeometry`, `normalized_rename` guard, `Scripts` pseudo-rows, and script drop targets.
- `viewport/` — scene viewport with camera pan/zoom; `to_window_render_camera`/`world_to_screen` equivalence locked by overlay tests.
- `picking/` — `EntityPicker` and `PickableEntity` (AABB from absolute size, flip scales stay clickable).
- `gizmo/` — transform gizmos (annulus rotate ring with dead-center fallthrough, cumulative delta, ratio-based scale, and cancel latch).
- `grid.rs` — authoring grid segments and viewport clipped overlay lines (no internal visibility flag; caller gates rendering via `ViewToggles`).
- `clipboard.rs` — `ClipboardEntity`, `capture_entity_tree`/`spawn_entity_tree`, and `SpawnTreeCommand`.
- `collider_overlay.rs` — collider outline overlay mirroring rapier placement (offset is body-local, Transform2D.scale ignored).

### Persistence + commands
- `commands/` — `EditorCommand` trait, `CommandHistory` dirty tracking watermark, `SetComponentCommand` merge-by-hint, `break_merge()` gesture boundary, and the play-session floor (`begin_session`/`drop_session_entries`/`rebase_session_entries`) with the leaf-level rebase in `rebase.rs`.
- `entity_names.rs` — `entity_display_name`: the one name the hierarchy row, the inspector heading and the command API's `display` field all read.
- `editor_preferences.rs` — `EditorPreferences` JSON serialization (`from_json`/`to_json`), panel layout capture/apply, camera state, flattened `view: ViewToggles` (carrying `grid_visible`, `snap_to_grid`, `colliders_visible`, `game_frame_visible`), `collapsed_components` (the inspector's collapsed sections), `ide_command` (IO handled by integration layer via save_store).
- `asset_browser.rs` — `AssetEntry`, `AssetKind` (`Image`, `Scene`, `Script`), `scan_assets` walking `common::vfs::list_files` for images, scenes, and scripts (`.rhai`, `.rs`), and `AssetBrowserState.selected` — the clicked tile, carried across a rescan by relative path and dropped when its file is gone.
- `stored_component/` — typed registry overlay (`editor_component_registry!`), `category.rs`, and `dynamic.rs` falling through to ECS dynamic registry.
- `world_snapshot.rs` — `WorldSnapshot` save/restore with uncaptured component type detection and drop reporting.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Continuous edits on different entities must never merge even when sharing a field hint | `src/commands/tests.rs test_edits_on_different_entities_never_merge_even_with_the_same_field_hint` |
| Continuous edits merge by field hint until `break_merge()` seals the gesture into distinct history entries | `src/commands/dirty_tests.rs test_break_merge_seals_the_gesture_so_two_scrubs_are_two_entries` |
| Adjacent surfaces in the editor theme ladder must maintain WCAG contrast (≥1.35:1 adjacent / ≥3:1 border) | `src/theme/tests.rs test_adjacent_surfaces_are_distinguishable` |
| Open menu dropdown renders in the overlay band and must block clicks from reaching underlying widgets | `src/menu/tests.rs test_open_dropdown_renders_in_overlay_band_and_blocks_input` |
| Entity picking must compute AABB from absolute visual size so flip-scaled sprites remain clickable | `src/picking/tests.rs test_flip_scaled_sprite_is_picked_at_its_visual_bounds` |
| A paused edit's before-image is the simulated value; a replay onto the restored world must `rebase_onto` first or Undo resurrects simulation state | `src/commands/session_tests.rs test_rebase_replays_only_the_changed_fields_over_the_authored_component` |
| Confirm dialog scrim clicks must block input to underlying background widgets | `src/confirm_dialog.rs test_scrim_click_is_not_a_choice_and_blocks_input` |
| World snapshot restore must detect and report unregistered component types that cannot be captured | `src/world_snapshot/tests.rs test_loss_messages_name_every_dropped_type_or_nothing` |
| Rotate gizmo dead-center clicks must fall through to entity picking | `src/gizmo/tests.rs test_rotate_ring_is_an_annulus_so_a_dead_center_press_falls_through_to_picking` |
| Hard floors for inspector editors and command API must clamp negative or zero dimensions | `src/command_api/write_tests.rs test_set_sanitizes_collider_extents_to_the_gui_floor` |
| Asset scanning must use `common::vfs::list_files` instead of `std::fs` so recursive asset enumeration works on wasm; it never follows symlinks (assets are copies by convention), so a linked tree lists as empty | `src/asset_browser.rs test_nested_images_and_scenes_listed_with_slash_joined_relative_paths_while_txt_is_ignored`; `common/src/vfs/tests.rs test_vfs_list_files_never_follows_symlinks` |
| Wheel zoom is proportional to the delta in notches and clamped to one notch per frame: a trackpad streams fractions of a notch every frame, and a fixed factor per frame made a gentle scroll compound like sixty notches a second; a hard flick that delivers a notch or more a frame still zooms a notch a frame, the mouse wheel's own ceiling | `src/viewport_input.rs test_a_fraction_of_a_wheel_notch_zooms_by_the_same_fraction_of_the_factor`, `test_a_frame_of_wheel_zooms_at_most_one_notch` |


## Key Patterns
- Inspector uses `serde_json::to_value()` to extract component fields generically
- Component editors return `Option<ComponentEdit<T>>` (full new value + `field_hint` for undo merging) that the integration crate applies via `apply_component_edit()`
- `EditorPlayState::Editing` → editable, `Playing` → the same rows drawn read-only, `Paused` → editable; `CommandHistory::begin_session` marks the Play boundary the editor's Stop dialog acts on
- The inspector heading is two lines: `entity_display_name` on top, then `Selection::inspector_heading`'s detail line ("Entity 17", or "3 selected · Entity 17 primary")
- Selection: `editor.selection.primary()` returns the main selected EntityId
- Gizmo drag tracking: editor_integration's `GizmoDragState` captures start transform+collider for every selection root; frames apply `start + cumulative delta` (idempotent — what makes snapping residual-proof), ONE Macro/TransformGizmo command on release, Escape restores starts and pushes nothing
- The toolbar and the play controls are NOT floating chrome: they render inside the strip's scope, positioned by `toolbar_strip::layout`. `scene_view_bounds()` is the viewport BELOW the strip — every overlay, the GPU scissor and every pick map through it, so none of them reach into the band; `toolbar_strip_bounds()` is the band itself.
- Theme is on `EditorContext.theme` (public field); call `theme.gizmo_palette()`, `inspector_style()`, `editable_field_style()`, `grid_colors()`, `collider_overlay_colors()`, `game_frame` instead of hardcoding colors. Menu/Toolbar/Hierarchy `render()` take `&EditorTheme`

## Testing
- `cargo test -p editor` — 0 failed, 0 ignored

## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

| Our Concept | Godot Equivalent | File |
|-------------|-----------------|------|
| EditorContext | EditorNode | `editor/editor_node.cpp` |
| Inspector | EditorInspector | `editor/editor_inspector.cpp` |
| Component editors | EditorProperties | `editor/editor_properties.cpp` — `_property_changed` |
| Picking / selection | Canvas item editor | `editor/plugins/canvas_item_editor_plugin.cpp` — `_gui_input_viewport` |
| Hierarchy panel | SceneTreeDock | `editor/scene_tree_dock.cpp` — `_tool_selected` |
| Gizmos | CanvasItemEditor gizmos | `editor/plugins/canvas_item_editor_plugin.cpp` — search `gizmo` |
| Play/Pause/Stop | EditorRun | `editor/editor_run.cpp`, `editor/editor_node.cpp` — `_run_native` |
| Undo/Redo | EditorUndoRedoManager | `editor/editor_undo_redo_manager.cpp` |
| Dock layout | EditorDockManager | `editor/editor_dock_manager.cpp` |

**Remember:** Godot's editor is plugin-based with docks. Adapt *interaction patterns* to our immediate-mode UI.
