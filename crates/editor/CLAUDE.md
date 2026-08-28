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
- `context/` — EditorContext struct (selection, tools, state, theme, fonts, inspector_scroll); tests in `context/tests.rs`
- `lib.rs` — Public re-exports
- `theme/` — EditorTheme (`mod.rs` + `tests.rs`; WCAG surface ladder `surface_0..surface_4` + `popup_border` with luminance guard tests — the ≥1.35:1 adjacent / ≥3:1 border tests are the spec, tune hexes only with them green; color tokens, `fonts: FontSizes` typography tokens, gizmo/grid/inspector style converters, `ui_theme()` → derives the ui crate Theme)
- `fonts.rs` — crate-shipped DejaVu faces (`EDITOR_FONT_REGULAR/BOLD/MONO` via include_bytes + `EditorFonts` handles; LICENSE ships in assets/fonts/)
- `scroll.rs` — shared `ScrollState` (per-panel vertical scroll; two documented call orders)
- `command_api/` — **Stage A query API** (`mod.rs` types + dispatch_line, `parse.rs`, `query.rs`): list/describe/selection/scene as single-line JSON; name-first `EntityRef` (`resolve_by_name` in hierarchy.rs, ambiguity = error), `#<u64>` id fallback; pure dispatch — no I/O/threads/cfg (transports live with callers; contract doc = `docs/EDITOR_COMMAND_API.md`)
- `typography.rs` — `FontSizes` {small 12/body 14/heading 16} + `MIN_READABLE_FONT` guard
- `drag_drop.rs` — `DragDropState`/`DragPayload` cross-panel drag state machine (Idle→Armed→Dragging→Dropped-1-frame)
- `asset_browser.rs` — pure asset scan (`scan_assets`), `AssetBrowserState`, `fit_rect`
- `texture_field.rs` — inspector texture slot (drop target) + `InspectorExtras`
- `gizmo_math.rs` — pure rotate-drag math (Y-flip + shortest-arc wrap)
- `dock/` — Multi-panel docking: `mod.rs` (state + layout, collapse/visibility toggles, `panel_id_for_menu_label`), `render.rs` (chrome, collapse chevrons, resize grabbers + clamped `resized_size`), `tests.rs`
- `layout.rs` — Layout helpers
- `menu/` — Top menu bar (`mod.rs` + `tests.rs`); action items carry a `checked` flag (`MenuBar::set_checked`) rendered as an accent square
- `toolbar.rs` — Tool selection toolbar
- `status_bar.rs` — Bottom status bar (22px); `show_message`/`show_error`/`clear_message`
- `play_controls.rs`, `play_state.rs` — Play/Pause/Stop widget + state enum
- `editor_input.rs` — Editor-only input (hotkeys, etc.)
- `editor_preferences.rs` — Persisted editor prefs (camera, zoom, last scene, `PanelPrefs` panel layout via `capture_panels`/`apply_panels`)

### Inspector / components
- `inspector.rs` — Generic `inspect_component()` (read-only, serde-based)
- `editable_inspector.rs` — Editable field widgets (sliders, Vec2, checkboxes, color, `string_edit` text input, `cycle()` variant selector)
- `text_field.rs` — `edit_string` free fn + read-only `display_string`/`display_u32`
- `ui_component_editors.rs` — `edit_ui_label/panel/button` (UiLabel/UiPanel/UiButton field editors; anchor via cycle selector)
- `field_style.rs` — `FieldId` (widget-ID mapping), `EditableFieldStyle` (layout dims + colors), `EditResult<T>`
- `component_editors.rs` — Per-component editors: `edit_transform2d()`, `edit_sprite()`, etc. Return `Option<ComponentEdit<T>>`; field ranges in `mod ranges`
- `behavior_editor.rs` — `edit_behavior()`: variant cycle selector + per-variant fields (String fields read-only until the ui crate grows text input)

### Scene + selection
- `selection.rs` — Selection set (primary + multi-select; insertion-ordered IndexSet, deterministic primary fallback)
- `hierarchy.rs` — Hierarchy panel tree view
- `viewport.rs`, `viewport_input.rs` — Scene viewport with camera pan/zoom
- `picking/` — EntityPicker, PickableEntity (AABB from absolute size — flip scales stay clickable), SelectionRect, screen_to_world()
- `selection_outline.rs` — viewport selection/hover outlines (consumes the picking `PickableEntity` list; pure `hover_entity_at` hit test; colors via `theme.selection_outline_colors()`)
- `gizmo.rs` — Transform gizmos (translate, rotate, scale handles)
- `grid.rs` — Background grid rendering
- `collider_overlay.rs` — Collider outline overlay for the scene view (mirrors rapier placement: offset is body-local, Transform2D.scale ignored); toggled via `EditorContext::toggle_colliders()` / C key

### Persistence + commands
- `commands/` — EditorCommand trait + CommandHistory (`mod.rs`; **dirty source of truth**: id-of-top watermark, `is_dirty()`/`mark_saved()`, merges reassign the top a fresh id AND clear redo; dirty_tests.rs is the contract), entity commands, component commands, `impl_set_component_command!` macro for the 5 Set*Commands (`set_commands.rs`); `push_already_executed`, `try_merge_or_push`
- `stored_component/` — **Component registry macro (single source of truth). ADD NEW EDITOR-VISIBLE COMPONENTS HERE** — one line in `editor_component_registry!` generates StoredComponent, ComponentKind (add/capture/remove/is_present/display_name/category), capture_all_components, registered_component_type_ids, inspect_all_components, AND edit_all_components (the editable inspector — entries carry `{ edit edit_x => SetXCommand }` or `{ readonly }`)
- `world_snapshot.rs` — WorldSnapshot save/restore (used by play/stop): registry-driven capture (auto-includes new registry types) + explicit Parent/Children; unregistered component types are detected (`uncaptured_types`/`loss_warning`/`drop_report`) and lost on restore
- Scene save/load file I/O lives in `editor_integration` (via `engine_core::scene_serializer`), not in this crate

## Key Patterns
- Inspector uses `serde_json::to_value()` to extract component fields generically
- Component editors return `Option<ComponentEdit<T>>` (full new value + `field_hint` for undo merging) that the integration crate applies via `apply_component_edit()`
- `EditorPlayState::Editing` → editable, `Playing` → read-only inspector, `Paused` → editable
- Selection: `editor.selection.primary()` returns the main selected EntityId
- Gizmo drag tracking: `gizmo_drag_start` field captures initial transform, then a single `TransformGizmo` command is pushed on release
- Theme is on `EditorContext.theme` (public field); call `theme.gizmo_palette()`, `inspector_style()`, `editable_field_style()`, `grid_colors()`, `collider_overlay_colors()` instead of hardcoding colors. Menu/Toolbar/Hierarchy `render()` take `&EditorTheme`

## Testing
- 362 passing (incl. 3 doc tests), 0 ignored — `cargo test -p editor`

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
