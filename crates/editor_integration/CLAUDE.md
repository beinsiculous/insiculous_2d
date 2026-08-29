# Editor Integration Crate — Agent Context

You are working in editor_integration. This bridges engine_core and editor without circular deps.
**This is where editor features get wired up to the running game.**

## Architecture
```
EditorGame<G: Game>  — transparent wrapper implementing Game trait
├── inner: G         — the actual game
├── editor: EditorContext
├── font_handle, ui state, play state, gizmo drag tracking
└── Intercepts: init(), update(), on_key_pressed()

run_game_with_editor(game, config) → wraps game in EditorGame, calls run_game()
run_game_with_editor_opts(game, config, EditorRunOptions { api_rx, initial_scene }) → full option set (#53: the standalone binary hands its project's first scene here — find_first_scene, sorted — and EditorGame opens it through the REAL load path after init, so scene_path/physics/dirty are recorded; EditorApp never loads scenes itself, and load_scene publishes PhysicsSettings as a world resource for the host's lazy physics preview)
```

## Dependency Graph
```
engine_core ──→ ecs, renderer, input, physics, audio, ui
editor ──→ ecs, ui, input, renderer, physics, common  (NO engine_core)
editor_integration ──→ editor, engine_core, ecs, ui, input, renderer, common
```

## File Map
- `editor_game/` — EditorGame<G> wrapper, split by feature:
  - `mod.rs` — struct + slim `Game` impl (`update()` = ~30 lines of named phases) + `run_game_with_editor`
  - `menu_actions.rs` — menu bar dispatch + shared delete/duplicate helpers
  - `scene_io.rs` — save/load/new scene (load parses + dry-runs into a scratch World BEFORE touching the live one — no failure mode costs the current scene; failures surface on status bar)
  - `api.rs` — command-API frame hook: `answer_api_lines` (headless-tested; routes queries/pure writes to editor::command_api, performs HostedWrite create — factories + viewport spawn pos — and save — through save_scene_with) + `drain_api_requests` (≤256 lines/frame, skipped during gizmo drags, stdout flushed per batch); `api_batch` on EditorGame (committed on Play, dropped on new/load scene); ship-point tests in `api_write_tests.rs`
  - `shortcuts.rs` — play state transitions + the unified key dispatch (#40): `handle_editor_key` resolves EVERY shortcut through `EditorInputMapping::resolve` and `dispatch_editor_action` executes it (play controls always intercepted; while Playing raw keys forward to the game unresolved; poll-only actions consume; gizmo-drag-live suppresses transform/existence mutations); Escape `cancel_cascade` (gizmo drag → marquee → deselect), arrow `nudge_selection` (merging NudgeCommand sealed by `break_merge` on key release = one undo per hold), Ctrl+A via `selectable_entities`
  - `shortcuts_tests.rs` — nudge merge/seal, cancel cascade, select-all, resolve drift lock
  - `gizmo_drag.rs` — `GizmoDragState`/`DragEntity`: per-root drag-start capture (apply/commit/cancel state); tests in `gizmo_drag_tests.rs` incl. the snap-residual regression
  - `viewport_interaction.rs` — picking, marquee (live rect draw + Ctrl/Shift release semantics), gizmo drag apply/commit/cancel, Ctrl-hold-to-snap
- `entity_ops.rs` — Pure entity CRUD (`&mut World` + `&mut Selection`, no UI). Component dispatch lives in `editor::ComponentKind` (registry macro). UI entities (`create_ui_label/panel/button`) get Name only — NO Transform2D (anchor+offset is their placement model)
- `panel_renderer/` — Panel contents: `mod.rs` (dispatch, scene view, hierarchy), `inspector.rs` (thin shell: registry-generated `editor::edit_all_components()` for editing, `inspect_all_components` read-only during play, add-component popup)
- `constants.rs` — `DEFAULT_SCENE_PATH`, `EDITOR_PREFS_PATH`, min window size, `MIN_ENTITY_SCALE`, `DUPLICATE_OFFSET`
- `lib.rs` — Public re-exports

## Key Patterns
- **Engine-time freeze (Jul 2026)**: `EditorGame::update` sets `ctx.time_scale = 0.0` whenever not Playing (`editor_time_scale()`, headless-testable), holding the game's own value in `frozen_time_scale` and handing it back on Play/Resume — particles AND sprite animations hold still while Editing/Paused, and a game that paused itself stays paused across an editor Pause.
- **Camera sync (Jul 2026, split #42 Aug 2026)**: the editor viewport is the single source of truth for the view. `EditorGame::render` overrides `ctx.camera` with `viewport.to_window_render_camera(window_size)` every frame; while Playing AND `is_camera_following()`, `sync_viewport_from_main_camera` mirrors the game's main-camera entity — position AND zoom — onto the viewport (editing pan/zoom saved on Play, restored on Stop; no main camera = zoom 1.0 parity). Manual pan/zoom during a play session breaks the follow (`break_camera_follow`, status-bar notice); the Follow toolbar button / Ctrl+Shift+F re-arms it; follow re-arms at session START and Stop only — pause→resume preserves the user's choice. While Playing, `handle_play_mode_camera` runs pan/zoom ONLY (the early return before picking/marquee/drops is load-bearing). Rotation is deliberately not synced (viewport math has no rotation term). Never sync the other direction.
- **Scale tool scales colliders**: physics ignores Transform2D.scale, so the gizmo scale branch also calls `scale_collider` and records one `MacroCommand` (transform+collider) per drag.
- **Asset browser** (`panel_renderer/asset_browser.rs`): scan-on-open + Rescan, lazy thumbnails (≤4 loads/frame), click-to-assign, drag-drop (ghost via ui overlay; viewport drop assigns on sprite hit, spawns on empty space — both undoable).
- `EditorGame::update()` — main orchestration. Editor input → conditional game update (only if Playing) → render panels
- Input routing: Editing/Paused → editor gets input. Playing → game gets input, editor hotkeys still work.
- Dirty state: `CommandHistory::is_dirty()` is the source of truth; `EditorContext.is_dirty` is a per-frame mirror (synced at update 0d and again before the status bar); the OS window title renders `title_bar_text()` change-gated via `ctx.window_title` (game owns the title while Playing)
- Inspector writeback: generated per-component by `editor_component_registry!` (editor crate) — `edit_*()` returns `Option<ComponentEdit<T>>` → `editor::apply_component_edit()` writes to world and records undo via `try_merge_or_push` (continuous edits merge by `field_hint`)
- Play/Stop: snapshot world on Play (typed clone via `WorldSnapshot`), restore on Stop
- Save/Load: Ctrl+S / Ctrl+Shift+S / Ctrl+O / Ctrl+N — `save_scene_with` (scene_io.rs) is the MANDATORY save choke point; save AND new/open are refused with a status-bar error during a play session (Playing or Paused — the world is mid-simulation). `SceneLoader` for load. Hardcoded paths (no file picker yet)
- Status messages: `editor.status_bar.show_message("Saved")` after successful operations
- Minimum window size: 1024x720 enforced for editor usability
- **Editor prefs**: camera/grid/panel layout loaded in `init`, saved in `on_exit` (`editor_prefs.json`); menu Exit sets `ctx.exit_requested` (clean shutdown), never `process::exit`
- **Font scoping**: editor font pinned at init and re-asserted every frame; `update_inner_game` swaps to `strings.active_font().or(game_base_font)` around `inner.update` so the game view localizes while chrome doesn't. View → "Cycle Game Locale" cycles `ctx.strings`
- **Scene-authored UI**: `UiElementsHidden` inserted on init and Stop (after snapshot restore), removed on Play — UiLabel/UiPanel/UiButton only draw while the game runs

## Phase 1 Status
Phase 1A–1H **complete**: entity CRUD, component add/remove, undo/redo, play/pause/stop, scene save/load, theme, status bar.
Current editor work follows the UX-audit sprint order (Aug 27 2026): see
`PROJECT_ROADMAP.md` § "Editor — UX Audit & Work Order" and
`docs/EDITOR_UX_AUDIT.md` (§7 = the 5-sprint work order; live items are Studio
Board issues, Phase = Editor). The old "Phase 2 (Ideal Editor UI)" lettering is
retired.

## Known Tech Debt
See `TECH_DEBT.md` (all files < 600 lines since June 2026; remaining: no file picker, menu-label string matching)

## Testing
- 123 passing (incl. 1 compile-only doc test), 0 ignored — `cargo test -p editor_integration` (component-dispatch tests moved to the editor crate with the registry; `editor_game/time_freeze_tests.rs` locks the engine-time freeze; `editor_game/play_guard_tests.rs` locks the play-session save/new/open guards + snapshot loss warnings)
- `entity_ops` is fully headless-testable (no UI dependency)

## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

This crate maps to Godot's editor plugin + node integration layer:
- `editor/editor_node.cpp` — how Godot's editor wraps the running scene
- `editor/scene_tree_dock.cpp` — entity CRUD operations (create, delete, duplicate, reparent)
- `editor/plugins/canvas_item_editor_plugin.cpp` — viewport interaction, picking, gizmo wiring
- `editor/editor_inspector.cpp` — how property changes flow back to objects
- `editor/editor_undo_redo_manager.cpp` — command pattern equivalent
