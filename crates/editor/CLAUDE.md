# Editor Crate — Agent Context

You are working in the editor crate. UI panels, tools, inspector, hierarchy — the editor's data and widgets.
This crate has NO dependency on engine_core. It depends on: ecs, ui, input, renderer, physics, common.

## Architecture
```
EditorContext (selection, tool state, play state, camera, theme, status_bar, fonts, inspector_scroll)
│   (CommandHistory itself lives on editor_integration's EditorGame and is threaded into panel renderers)
├── Panels: SceneView, Hierarchy, Inspector, AssetBrowser, Console
├── Dock layout: dock.rs (multi-panel docking)
├── Menu / Toolbar / StatusBar (top + bottom chrome)
├── Tools: Select, Move, Rotate, Scale (Q/W/E/R shortcuts)
├── Gizmos: Translate, Rotate, Scale handles
├── Picking: EntityPicker, SelectionRect, screen_to_world()
├── Inspector: Generic serde-based + per-component editors with writeback
├── Undo/Redo: CommandHistory + EditorCommand trait, StoredComponent for restore
├── Theme: EditorTheme with 30+ color tokens (mockup-derived)
└── Play state: EditorPlayState (Editing/Playing/Paused), WorldSnapshot
```

## File Map
### State + chrome
- `context/` — EditorContext struct (selection, tools, state, theme, fonts, inspector_scroll).
- `theme/` — EditorTheme: WCAG surface ladder `surface_0..surface_4` with luminance guard tests (≥1.35:1 adjacent / ≥3:1 border), style converters, and `ui_theme()`.
- `command_api/` — CLI/API dispatch (query list/describe/selection/scene/commands and write set/add/remove/rename/delete/select/undo/redo/batch) through CommandHistory; `docs/EDITOR_COMMAND_API.md`.
- `drag_drop.rs` — `DragDropState`/`DragPayload` cross-panel drag state machine (Idle→Armed→Dragging→Dropped-1-frame).
- `dock/` — multi-panel docking: state, layout, collapse/visibility toggles, chevrons, and clamped resize grabbers.
- `menu/` — top menu bar; action items carry checked flag and map labels to `EditorAction`.
- `editor_input.rs` — shortcut chord model (exact chord beats any-mods) and `allowed_while_playing()` action deny list.
- `archetype.rs` — `Archetype`: the nine entity factories shared by the Entity menu and command API `create`.

### Inspector / components
- `editable_inspector.rs` — editable field widgets (f32 soft-range, angle degree field with wrap, cycle selector) and width-aware `EditableInspector`.
- `row_layout.rs` — row-layout math (`field_row`, `remove_button_x`, `pair_slots`, `ellipsize`; all horizontal placement goes through here, never hardcode offsets).
- `field_style.rs` — `FieldId` (widget-ID mapping), `EditableFieldStyle`, and typed `EditResult<T>` returns (keeps the editor crate free of an engine_core dependency); `WidgetSlot` inside component ID stride.
- `component_editors.rs` — per-component editors returning `Option<ComponentEdit<T>>`; shape cycling carries dimensions with commit-before-cycle ordering.
- `physical_floors.rs` — hard floors applied by inspector editors and command API `sanitize` (scale, collider extents, capsule half-height, volume, pitch).
- `behavior_editor.rs` — `edit_behavior()`: variant cycle selector and per-variant editors; `CameraFollow.dead_zone` stays read-only.

### Scene + selection
- `selection.rs` — Selection set (IndexSet preserving insertion order, deterministic primary fallback).
- `hierarchy/` — hierarchy panel tree view, F2 inline rename, `RowGeometry`, and `normalized_rename` guard.
- `viewport/` — scene viewport with camera pan/zoom; `to_window_render_camera`/`world_to_screen` equivalence locked by overlay tests.
- `picking/` — `EntityPicker` and `PickableEntity` (AABB from absolute size, flip scales stay clickable).
- `gizmo/` — transform gizmos (annulus rotate ring with dead-center fallthrough, cumulative delta, ratio-based scale, and cancel latch).
- `grid.rs` — authoring grid segments and viewport clipped overlay lines.
- `clipboard.rs` — `ClipboardEntity`, `capture_entity_tree`/`spawn_entity_tree`, and `SpawnTreeCommand`.
- `collider_overlay.rs` — collider outline overlay mirroring rapier placement (offset is body-local, Transform2D.scale ignored).

### Persistence + commands
- `commands/` — `EditorCommand` trait, `CommandHistory` dirty tracking watermark, `SetComponentCommand` merge-by-hint, and `break_merge()` gesture boundary.
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
| Confirm dialog scrim clicks must block input to underlying background widgets | `src/confirm_dialog.rs test_scrim_click_is_not_a_choice_and_blocks_input` |
| World snapshot restore must detect and report unregistered component types that cannot be captured | `src/world_snapshot/tests.rs test_loss_messages_name_every_dropped_type_or_nothing` |
| Rotate gizmo dead-center clicks must fall through to entity picking | `src/gizmo/tests.rs test_rotate_ring_is_an_annulus_so_a_dead_center_press_falls_through_to_picking` |
| Hard floors for inspector editors and command API must clamp negative or zero dimensions | `src/command_api/write_tests.rs test_set_sanitizes_collider_extents_to_the_gui_floor` |


## Key Patterns
- Inspector uses `serde_json::to_value()` to extract component fields generically
- Component editors return `Option<ComponentEdit<T>>` (full new value + `field_hint` for undo merging) that the integration crate applies via `apply_component_edit()`
- `EditorPlayState::Editing` → editable, `Playing` → read-only inspector, `Paused` → editable
- Selection: `editor.selection.primary()` returns the main selected EntityId
- Gizmo drag tracking: editor_integration's `GizmoDragState` captures start transform+collider for every selection root; frames apply `start + cumulative delta` (idempotent — what makes snapping residual-proof), ONE Macro/TransformGizmo command on release, Escape restores starts and pushes nothing
- Theme is on `EditorContext.theme` (public field); call `theme.gizmo_palette()`, `inspector_style()`, `editable_field_style()`, `grid_colors()`, `collider_overlay_colors()` instead of hardcoding colors. Menu/Toolbar/Hierarchy `render()` take `&EditorTheme`

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
