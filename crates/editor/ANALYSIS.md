# Editor Crate Analysis

> **Audit 2026-04-15**: Pruned completed Phase 1 items (undo/redo, scene save/load,
> hierarchy panel, editable inspector, play/pause/stop, theme, status bar — all done).
> Removed the "Component Inspector Requirements" field-type table and editable-component
> checklist, since those have been implemented in `component_editors.rs` /
> `editable_inspector.rs`. Preserved architectural notes, design patterns, and
> cross-crate integration notes as reference material for Phase 2+ work.

## Overview

The editor crate provides a visual scene editor for building game worlds, editing
entity properties, and managing scene hierarchies. It is built on top of the existing
immediate-mode UI system and integrates with the engine's ECS, rendering, and asset
systems. The crate has **no dependency on `engine_core`** — it depends on `ecs`, `ui`,
`input`, `renderer`, `physics`, `common`. Cross-crate wiring lives in `editor_integration`.

### Summary
- Visual scene editor with dockable panels (Hierarchy / Scene View / Inspector / Asset Browser / Console)
- Entity CRUD (create, delete, duplicate) with command-based undo/redo
- Selection with single/multi-select and primary selection
- Transform gizmos (Translate / Rotate / Scale) wired through `TransformGizmo` command with merge-on-drag
- Menu bar, toolbar (Q/W/E/R), play controls, status bar (22px bottom chrome)
- Grid snapping and camera pan/zoom
- Scene viewport with world/screen conversion and grid overlay
- Editable inspector (serde-based read path + per-component edit path with writeback)
- Component add/remove popup in the inspector
- Hierarchy panel with parent-child tree, expand/collapse
- Scene save/load via `engine_core::scene_serializer` (Ctrl+S / Ctrl+Shift+S / Ctrl+O / Ctrl+N)
- Play / Pause / Stop with `WorldSnapshot` capture/restore and read-only inspector during play
- Centralized `EditorTheme` (30+ color tokens) applied to grid, inspector, gizmos, panels

### Status (2026-04-15)
- **Tests**: 214 passing, 0 failed, 0 ignored (`cargo test -p editor --lib`); 3 ignored doctests
- **Phase 1**: Complete (all milestones 1A–1H)
- **Phase 2**: lettering retired Aug 27 2026 — editor work now follows the UX-audit sprint order (`docs/EDITOR_UX_AUDIT.md`, roadmap § "Editor — UX Audit & Work Order"; 2G theme work continues in board issue #26)
- **Dependencies**: ui, ecs, input, renderer, physics, common, audio (no engine_core)

## Architecture

### Module Structure

```
crates/editor/src/
├── lib.rs                   # Crate entry point and re-exports
├── context.rs               # EditorContext — main editor state
├── theme.rs                 # EditorTheme (color tokens + style converters)
├── dock.rs                  # Dockable panel system
├── layout.rs                # Layout constants
├── menu.rs                  # Menu bar and dropdown menus
├── toolbar.rs               # Tool selection (Select/Move/Rotate/Scale)
├── status_bar.rs            # Bottom status bar (22px)
├── play_controls.rs         # Play/Pause/Stop widget
├── play_state.rs            # EditorPlayState enum (Editing/Playing/Paused)
├── editor_input.rs          # Editor-specific input mapping
├── editor_preferences.rs    # Persisted prefs (camera, zoom, last scene)
├── viewport.rs              # SceneViewport (camera, coordinate conversion)
├── viewport_input.rs        # Viewport navigation (pan, zoom, select-rect, focus)
├── grid.rs                  # Grid overlay rendering
├── picking.rs               # EntityPicker, PickableEntity, SelectionRect, AABB
├── gizmo.rs                 # Transform manipulation gizmos
├── selection.rs             # Entity selection management
├── hierarchy.rs             # Hierarchy panel (tree view)
├── inspector.rs             # Read-only component inspector (serde-based)
├── editable_inspector.rs    # Editable field widgets (sliders, Vec2, checkboxes, color)
├── component_editors.rs     # Per-component editors (Transform2D, Sprite, RigidBody, Collider, AudioSource)
├── commands.rs              # EditorCommand trait, CommandHistory, 11 concrete commands
├── stored_component.rs      # StoredComponent enum for type-safe capture/restore
├── world_snapshot.rs        # WorldSnapshot capture/restore (for play/stop)
└── file_operations.rs       # Scene save/load file I/O wrappers
```

### Core Components

#### EditorContext (`context.rs`)
Central editor state. Holds: `Selection`, `Gizmo`, `Toolbar`, `MenuBar`, `DockArea`,
`SceneViewport`, `GridRenderer`, `EntityPicker`, `ViewportInputHandler`,
`EditorInputMapping`, `HierarchyPanel`, `PlayControls`, `EditorTheme`, `StatusBar`,
plus snap/play/dirty/scene_path flags and gizmo-drag-start capture. Command history
lives alongside the context (typically owned by the integration crate, which applies
commands through the public `CommandHistory` API).

#### SceneViewport (`viewport.rs`)
Manages rendering the game world within the scene view panel: camera position/zoom
with smooth interpolation, world↔screen conversion, visible bounds calculation,
focus-on-entity/selection.

#### ViewportInputHandler (`viewport_input.rs`)
Pan (middle mouse or Space+drag), zoom (cursor-centered scroll), selection rectangle
(primary drag), focus (F), reset camera (Home).

#### GridRenderer (`grid.rs`)
Primary + subdivision lines with LOD, axis lines through origin. Colors pulled from
the active `EditorTheme` via `theme.grid_colors()`.

#### Selection (`selection.rs`)
Single/multi-select with a "primary" entity for property editing. Iteration and
query helpers for the hierarchy and inspector panels.

#### DockArea / DockPanel (`dock.rs`)
Flexible panel layout: Left, Right, Top, Bottom, Center, Floating. Resizable panels
with minimum size constraints; visibility toggle.

#### Toolbar / MenuBar (`toolbar.rs`, `menu.rs`)
Standard tool selection (Q/W/E/R) and File/Edit/View/Entity menus. Menu actions are
returned as enums; the integration layer decides what to do with them.

#### Gizmo (`gizmo.rs`)
Translate/Rotate/Scale handles. Gizmo colors pulled from the theme. Drag tracking
via `gizmo_drag_start` on `EditorContext` captures the initial transform so the
release can push a single `TransformGizmo` command (merged drags stay as one undo step).

#### Inspector path (`inspector.rs`, `editable_inspector.rs`, `component_editors.rs`)
Three-tier split:
- `inspector.rs` — generic read-only display via `serde_json::to_value()` for any
  `Serialize` component (handles Vec2/3/4, nested objects, arrays, floats with
  2-decimal formatting).
- `editable_inspector.rs` — primitive editable widgets (`edit_f32`, `edit_vec2`,
  `edit_bool`, `edit_color`, `edit_normalized_f32`, `display_u32`, `component_header`).
- `component_editors.rs` — per-component editors that return typed `*EditResult`
  structs (e.g., `TransformEditResult`) for the integration layer to apply and
  optionally push as undo commands.

#### Commands (`commands.rs`, `stored_component.rs`)
Command pattern: `EditorCommand` trait with `execute` / `undo` / `display_name` /
`try_merge`. `CommandHistory` owns undo/redo stacks with a max-history limit.
11 concrete commands (CreateEntity, DeleteEntity, AddComponent, RemoveComponent,
TransformGizmo, SetTransform, SetSprite, SetRigidBody, SetCollider, SetAudioSource,
Macro). `StoredComponent` enum captures component state for restore in a type-safe
way. `push_already_executed()` and `try_merge_or_push()` support gizmo/slider
continuous-edit merging.

#### Play state (`play_state.rs`, `play_controls.rs`, `world_snapshot.rs`)
`EditorPlayState` { Editing, Playing, Paused }. `WorldSnapshot` captures the full
world via typed clone (no serialization), restored on Stop. Inspector becomes
read-only during Playing. Border tint on the scene view indicates current state.

#### Theme (`theme.rs`)
`EditorTheme` with 30+ mockup-derived color tokens and converter methods:
`grid_colors()`, `inspector_style()`, `editable_field_style()`, `play_state_border()`,
`gizmo_colors()`. Stored on `EditorContext.theme` (public), passed to dock, play
controls, panel renderer, gizmo.

#### Status bar (`status_bar.rs`)
22px bottom chrome. Left: status message (auto-clear after 3s) via `show_message()` /
`show_error()` / `clear_message()`. Center: entity count + FPS. Right: version string
in accent-cyan. Wired into save/load/undo/redo/play-state transitions.

#### Hierarchy (`hierarchy.rs`)
Tree view of world entities using `WorldHierarchyExt` for parent/child traversal.
Supports expand/collapse per entity. Drag-and-drop reparenting is not yet implemented.

#### File operations (`file_operations.rs`)
Wraps `engine_core::scene_saver` / `scene_loader` with editor-state integration
(dirty flag, scene path, selection clear, camera reset on new scene). Keyboard
shortcuts Ctrl+S / Ctrl+Shift+S / Ctrl+O / Ctrl+N handled by the integration layer.

## Design Patterns

### Immediate-Mode UI Integration
The editor uses the existing `ui` crate's immediate-mode paradigm:
```rust
if let Some(action) = editor.menu_bar.render(ui, window_width) {
    handle_menu_action(action);
}
if let Some(tool) = editor.toolbar.render(ui) {
    handle_tool_change(tool);
}
```

### Tool-Mode Synchronization
Tool selection automatically updates gizmo mode:
```rust
ctx.set_tool(EditorTool::Move);
// gizmo.mode() is now GizmoMode::Translate
```

### Coordinate Conversion
The editor maintains a separate camera from the game camera:
```rust
let world_pos = editor.screen_to_world(mouse_pos);
let screen_pos = editor.world_to_screen(entity_pos);
```

### Command Merging for Continuous Edits
Gizmo drags and slider scrubs should collapse to a single undo step. The pattern:
1. On drag start, capture the initial state (e.g., `gizmo_drag_start`).
2. Apply changes directly to components during the drag (no command pushes).
3. On release, build a single command with `before = initial`, `after = current`
   and call `push_already_executed()`.

Alternative: push per-frame commands with `try_merge_or_push()` — `try_merge()` on the
command type merges continuous edits (same entity, same field) into the top of the
undo stack.

### Generic Inspector via Serde
`inspect_component()` takes any `&impl Serialize` and renders it via
`serde_json::to_value()`. New components with `#[derive(Serialize)]` inspect for free.
The editable path deliberately does NOT use serde — per-component editors are explicit
for strong typing, validation, and typed `*EditResult` return values.

### Per-Component Editor Return Types
Component editors (`edit_transform2d`, `edit_sprite`, etc.) return typed result structs
(`TransformEditResult`, `SpriteEditResult`, …) rather than mutating components directly.
This keeps the editor crate free of an `engine_core` dependency and lets the integration
layer decide whether to apply directly, push a command, or batch changes.

## Integration Points (with `editor_integration`)

### Gizmo → Entity Transform
Gizmo drag state lives on `EditorContext.gizmo_drag_start`. The integration layer reads
the gizmo's current delta each frame, applies it to the selected entity's `Transform2D`
in place, and on release builds a `TransformGizmo` command from the captured start and
current state.

### Inspector → ECS
Component editors return `*EditResult` structs. The integration layer matches on the
result, applies the change via `world.get_mut::<T>(entity)`, and optionally pushes a
`Set{Component}` command on the `CommandHistory`.

### Play / Stop → Snapshot
On Play: `WorldSnapshot::capture(world)` → stored on integration context. On Stop:
`snapshot.restore(world)` → wipes play-mode changes. Physics systems also re-sync
colliders on stop (see `physics` crate).

### Scene Save / Load
`file_operations.rs` delegates to `engine_core::scene_saver` (World → SceneData) and
`scene_loader` (SceneData → World). Editor state (camera, grid, last scene) persists
via `EditorPreferences`.

## Strengths

1. **Modular design** — each subsystem (selection, gizmo, dock, inspector, …) is independent
2. **Strong test coverage** — 214 tests, behavior-focused
3. **No engine_core dep** — keeps the editor crate lean; integration lives in `editor_integration`
4. **Serde-based generic inspector** — new components inspect for free
5. **Type-safe command pattern** — `StoredComponent` enum avoids serialization round-trips

## Current Limitations / Future Directions

1. **Asset browser** — scaffolded but not feature-complete (Phase 2+)
2. **Console panel** — placeholder (Phase 2+)
3. **Drag-and-drop reparenting** — hierarchy panel supports tree view + expand/collapse,
   but reparenting via drag is not yet implemented
4. **File dialog** — save/load currently uses hardcoded paths; native file picker is
   Phase 2+
5. **Gizmo rendering** — uses simple shapes; proper handle sprites / screen-space
   scaling are future polish
6. **Theme hot-reload** — theme is static at startup; runtime reloading for live
   tweaking would be a nice-to-have
7. **Multi-viewport** — single scene viewport only; multi-viewport (e.g., for prefab
   editing) is not supported

## Test Summary (2026-04-15)

Totals from `cargo test -p editor --lib`: **214 passing, 0 failed, 0 ignored**
(plus 3 ignored doctests in `lib.rs` and `theme.rs`).

Heaviest coverage sits on `commands` (execute/undo/redo roundtrips), `context` (state
integration), `viewport` (coordinate math), `picking` (AABB / selection rect), `gizmo`
(mode transitions, delta calc), `component_editors` / `editable_inspector` (widget
behavior + writeback results), and `world_snapshot` (capture/restore, hierarchy
preservation).
