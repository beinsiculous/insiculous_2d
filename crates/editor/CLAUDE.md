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
- `command_api/` — **Stages A+B** (`mod.rs` types + envelope + dispatch_line, `parse.rs` incl. rest-of-line JSON for set/add + ARCHETYPES/VERBS, `query.rs`, `write.rs` WriteCtx/ApiBatch/run — every write through CommandHistory, `specs.rs` CommandDoc table + `commands` self-description w/ drift tests): queries list/describe/selection/scene/commands + writes set/add/remove/rename/delete/select/undo/redo/batch (create/save are HostedWrite, performed by editor_integration); name-first `EntityRef`, single-line JSON; pure dispatch — no I/O/threads/cfg (contract doc = `docs/EDITOR_COMMAND_API.md`; write_tests.rs)
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
- `editor_input.rs` — THE editor shortcut table (#40): `EditorBinding` chord model (`Chord{key,ctrl,shift}`/`KeyAnyMods`/`Mouse` — editor-owned, engine `InputSource` untouched), `EditorInputMapping` with event-path `resolve(key,ctrl,shift)` (exact chord beats any-mods; eviction keyed by the full tuple) + poll-path `is_action_pressed/just_pressed`; every editor shortcut ships as a default binding here
- `editor_preferences.rs` — Persisted editor prefs (camera, zoom, last scene, `PanelPrefs` panel layout via `capture_panels`/`apply_panels`)

### Inspector / components
- `inspector.rs` — Generic `inspect_component()` (read-only, serde-based)
- `editable_inspector.rs` — Editable field widgets (f32 with soft-range opts + `angle()` degree field w/ `wrap_degrees`, bool, `string_edit` text input, `cycle()` variant selector); `EditableInspector` is width-aware (`with_width`) — labels ellipsize at the control column, controls clamp to the panel's right edge, the [X] right-aligns
- `row_layout.rs` — pure row-layout math (`field_row`/`remove_button_x`/`pair_slots`/`color_block_height`/`ellipsize`, measurement injected — headless-tested; ALL inspector horizontal placement goes through here, never hardcode offsets)
- `composite_rows.rs` — `edit_vec2` (X/Y composite row) + `edit_color` (RGBA 2×2 grid with aligned columns), measured axis/channel badges
- `text_field.rs` — `edit_string` free fn + read-only `display_string`/`display_u32`
- `ui_component_editors.rs` — `edit_ui_label/panel/button` (UiLabel/UiPanel/UiButton field editors; anchor via cycle selector)
- `field_style.rs` — `FieldId` (widget-ID mapping), `EditableFieldStyle` (layout dims + colors; `label_width` 120), `EditResult<T>`
- `component_editors.rs` — Per-component editors: `edit_transform2d()`, `edit_sprite()`, etc. Return `Option<ComponentEdit<T>>`; field ranges in `mod ranges`; RigidBody Type + Collider Shape are cycle rows (shape cycling carries dimensions, early-return on variant change; headless-locked in `inspector_edit_tests.rs` incl. the commit-before-cycle ordering)
- `behavior_editor.rs` — `edit_behavior()`: variant cycle selector + per-variant fields (tag/target strings editable via `string_edit`; `CameraFollow.dead_zone` stays read-only — it's an `Option<(f32,f32)>` awaiting an Option widget)

### Scene + selection
- `selection.rs` — Selection set (primary + multi-select; insertion-ordered IndexSet, deterministic primary fallback)
- `hierarchy.rs` — Hierarchy panel tree view + F2 inline rename (`begin_rename`/`rename_widget_id`/`HierarchyResponse`, `normalized_rename` guard); tests in `hierarchy_tests.rs`
- `viewport/{mod,tests}.rs`, `viewport_input.rs` — Scene viewport with camera pan/zoom; `to_window_render_camera`/`world_to_screen` overlay↔GPU equivalence locked by `assert_overlay_matches_render_camera` tests (incl. the #42 play-follow pose); `EditorContext.camera_follow` (default true) is the play-session follow flag
- `picking/` — EntityPicker, PickableEntity (AABB from absolute size — flip scales stay clickable), screen_to_world() (SelectionRect deleted in #39 — the live marquee is ViewportInputHandler state + the caller's screen-space rect draw)
- `selection_outline.rs` — viewport selection/hover outlines (consumes the picking `PickableEntity` list; pure `hover_entity_at` hit test; colors via `theme.selection_outline_colors()`)
- `gizmo/` — Transform gizmos (`mod.rs` + `tests.rs`): annulus rotate ring (dead-center clicks fall through to picking), cumulative interaction (`translation`/`scale_factor` from drag start, `released` flag), ratio-based multiplicative scale, `cancel()` + polled suppress-until-release latch, `render(ui, screen_pos, interactive)` clip/gating, mode-switch-mid-drag handle release
- `grid.rs` — Authoring grid (#36): pure `grid_segments()` (LOD, subdivisions, max_lines, origin axes) + `render_grid_overlay()` drawing clipped `ui.line`s via the viewport — the collider-overlay pattern
- `clipboard.rs` — Entity clipboard (#40): `ClipboardEntity` + `capture_entity_tree`/`spawn_entity_tree` (registry-driven, hierarchy rebuilt explicitly), `SpawnTreeCommand` (undo removes the WHOLE subtree; redo re-records the fresh root), `uncaptured_component_names` warning helper; Duplicate and Paste both flow through here
- `collider_overlay.rs` — Collider outline overlay for the scene view (mirrors rapier placement: offset is body-local, Transform2D.scale ignored); toggled via `EditorContext::toggle_colliders()` / C key

### Persistence + commands
- `commands/` — EditorCommand trait + CommandHistory (`mod.rs`; **dirty source of truth**: id-of-top watermark, `is_dirty()`/`mark_saved()`, merges reassign the top a fresh id AND clear redo; dirty_tests.rs is the contract), entity commands, component commands, `impl_set_component_command!` macro for the Set*Commands incl. SetNameCommand (`set_commands.rs`, incl. SetEntityTagCommand; + `RenameEntityCommand` for entities without a Name; name_tests.rs); `push_already_executed`, `try_merge_or_push`, `break_merge()` gesture boundary (scrub/typed-commit seals the top entry — dirty_tests.rs)
- `stored_component/` — **Component registry macro (single source of truth). ADD NEW EDITOR-VISIBLE COMPONENTS HERE** — one line in `editor_component_registry!` generates StoredComponent, ComponentKind (add/capture/remove/is_present/display_name/category), capture_all_components, registered_component_type_ids, inspect_all_components, AND edit_all_components (the editable inspector — entries carry `{ edit edit_x => SetXCommand }` or `{ readonly }`)
- `world_snapshot.rs` — WorldSnapshot save/restore (used by play/stop): registry-driven capture (auto-includes new registry types) + explicit Parent/Children; unregistered component types are detected (`uncaptured_types`/`loss_warning`/`drop_report`) and lost on restore
- Scene save/load file I/O lives in `editor_integration` (via `engine_core::scene_serializer`), not in this crate

## Key Patterns
- Inspector uses `serde_json::to_value()` to extract component fields generically
- Component editors return `Option<ComponentEdit<T>>` (full new value + `field_hint` for undo merging) that the integration crate applies via `apply_component_edit()`
- `EditorPlayState::Editing` → editable, `Playing` → read-only inspector, `Paused` → editable
- Selection: `editor.selection.primary()` returns the main selected EntityId
- Gizmo drag tracking: editor_integration's `GizmoDragState` captures start transform+collider for every selection root; frames apply `start + cumulative delta` (idempotent — what makes snapping residual-proof), ONE Macro/TransformGizmo command on release, Escape restores starts and pushes nothing
- Theme is on `EditorContext.theme` (public field); call `theme.gizmo_palette()`, `inspector_style()`, `editable_field_style()`, `grid_colors()`, `collider_overlay_colors()` instead of hardcoding colors. Menu/Toolbar/Hierarchy `render()` take `&EditorTheme`

## Testing
- 438 passing (incl. 3 doc tests), 0 ignored — `cargo test -p editor`

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
