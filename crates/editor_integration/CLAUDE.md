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
run_game_with_editor_opts(game, config, EditorRunOptions { api_rx, initial_scene, api_responses, prefs_slot, dirty_flag, persist_pending }) → full option set. The standalone binary hands its project's first scene here (find_first_scene, sorted) and EditorGame opens it through the REAL load path after init, so scene_path/physics/dirty are recorded; ProjectHost never loads scenes itself, and load_scene publishes PhysicsSettings as a world resource for the host's lazy physics and behavior preview. The playground supplies the other four: a response channel for the bridge, a localStorage prefs slot, the dirty flag the editor WRITES from sync_dirty_mirror, and the persist-pending flag it READS there (ORed into the title's dirty mark). asset_base is not an option: EditorGame captures ctx.assets.base_path() after the inner game's init, and every bare relative scene path joins it.
```

## Dependency Graph
```
engine_core ──→ ecs, renderer, input, physics, audio, ui
editor ──→ ecs, ui, input, renderer, physics, common  (NO engine_core)
editor_integration ──→ editor, engine_core, ecs, ui, input, renderer, common
```

## File Map
- `project_host.rs` — data-only game host for the standalone editor (physics preview, behavior runner, transform hierarchy).
- `editor_game/mod.rs` — struct and Game impl (`update()` = named phases `prepare_frame`/`render_early_overlays`/`finish_frame`) + `run_game_with_editor`.
- `editor_game/preferences.rs` — preferences load/save and debounced settle persistence (0.5s stability window).
- `editor_game/scene_io.rs` — save/load/new scene (load dry-runs into a scratch World before touching the live one).
- `editor_game/api.rs` — command-API frame hook (`answer_api_lines`, `drain_api_requests` with ≤256 lines/frame cap, skipped during gizmo drags).
- `editor_game/shortcuts.rs` — key dispatch: `route_editor_key` + four category dispatchers; Escape cancel cascade, arrow nudge merge/seal.
- `editor_game/play_session.rs` — play transitions (`start_play_session`, `pause`, `resume_from_pause`, `stop_play_session`, camera follow).
- `editor_game/gizmo_drag.rs` — drag-start capture, `handle_gizmo`, `scale_collider`.
- `editor_game/viewport_interaction.rs` — picking, marquee, framing, once-per-frame pickables.
- `editor_game/test_support.rs` — fixture module: `DummyGame`, `editor_game()`, interaction builders.
- `entity_ops.rs` — pure entity CRUD; UI entities get Name only (anchor+offset placement model, no Transform2D).
- `panel_renderer/` — panel contents: scene view, hierarchy, inspector, add_component_popup.

## Key Patterns
- **Engine-time freeze (Jul 2026)**: `EditorGame::update` sets `ctx.time_scale = 0.0` whenever not Playing (`editor_time_scale()`, headless-testable), holding the game's own value in `frozen_time_scale` and handing it back on Play/Resume — particles AND sprite animations hold still while Editing/Paused, and a game that paused itself stays paused across an editor Pause.
- **Camera sync (Jul 2026, split #42 Aug 2026)**: the editor viewport is the single source of truth for the view. `EditorGame::render` overrides `ctx.camera` with `viewport.to_window_render_camera(window_size)` every frame; while Playing AND `is_camera_following()`, `sync_viewport_from_main_camera` mirrors the game's main-camera entity — position AND zoom — onto the viewport (editing pan/zoom saved on Play, restored on Stop; no main camera = zoom 1.0 parity). Manual pan/zoom during a play session breaks the follow (`break_camera_follow`, status-bar notice); the Follow toolbar button / Ctrl+Shift+F re-arms it; follow re-arms at session START and Stop only — pause→resume preserves the user's choice. While Playing, `handle_play_mode_camera` runs pan/zoom ONLY (the early return before picking/marquee/drops is load-bearing). Rotation is deliberately not synced (viewport math has no rotation term). Never sync the other direction.
- **Scale tool scales colliders**: physics ignores Transform2D.scale, so the gizmo scale branch also calls `scale_collider` and records one `MacroCommand` (transform+collider) per drag.
- **Asset browser** (`panel_renderer/asset_browser.rs`): scan-on-open + Rescan, lazy thumbnails (≤4 loads/frame), click-to-assign, drag-drop (ghost via ui overlay; viewport drop assigns on sprite hit, spawns on empty space — both undoable).
- `EditorGame::update()` — main orchestration. Editor input → conditional game update (only if Playing) → render panels
- Input routing: Editing/Paused → editor gets input. Playing → game gets input, editor hotkeys still work.
- Dirty state: `CommandHistory::is_dirty()` is the source of truth; `EditorContext.is_dirty` is a per-frame mirror (synced once by `sync_dirty_mirror` before the status bar; `scene_io` reads the history); the OS window title renders `title_bar_text()` change-gated via `ctx.set_window_title` (game owns the title while Playing)
- Inspector writeback: generated per-component by `editor_component_registry!` (editor crate) — `edit_*()` returns `Option<ComponentEdit<T>>` → `editor::apply_component_edit()` writes to world and records undo via `try_merge_or_push` (continuous edits merge by `field_hint`)
- Play/Stop: snapshot world on Play (typed clone via `WorldSnapshot`), restore on Stop
- Save/Load: Ctrl+S / Ctrl+Shift+S / Ctrl+O / Ctrl+N — `save_scene_with` (scene_io.rs) is the MANDATORY save choke point; save AND new/open are refused with a status-bar error during a play session (Playing or Paused — the world is mid-simulation). `SceneLoader` for load. Hardcoded paths (no file picker yet)
- Status messages: `editor.status_bar.show_message("Saved")` after successful operations
- Minimum window size: 1024x720 enforced for editor usability
- **Editor prefs**: camera/grid/panel layout loaded in `init`, saved in `on_exit` (`editor_prefs.json`); menu Exit calls `ctx.request_exit()` (clean shutdown), never `process::exit`
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
Tracked on the Studio Board: issue #90 (all files < 600 lines since June 2026; remaining: no file picker UX-001, menu-label string matching ARCH-101)

## Testing
- `cargo test -p editor_integration` — 0 failed, 0 ignored. Every test locks a contract or a footgun and is named for it; tests sit inline (`constants.rs`, `entity_ops.rs`, `panel_renderer/*`) or as `*_tests.rs` siblings under `editor_game/`, one per production module or feature.
- `entity_ops` is fully headless-testable (no UI dependency). `GameContext` is NOT constructible headless (`AssetManager` needs a wgpu device); headless guards test `take_api_lines`, `route_editor_key`, `handle_viewport_picking`, and `render_inspector`; `update`/`init` remain untestable without a device; the world-only methods (`handle_play_action`, `delete_selected_entities`/`duplicate_selected_entities`, `answer_api_lines`, `apply_gizmo_drag`/`commit_gizmo_drag`, `nudge_selection`, `cancel_cascade`, `save_scene_with`, `load_scene`) are what the suite drives. Keep new `EditorGame` methods taking `&mut World` unless they genuinely need assets or UI.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Physics ignores `Transform2D.scale`: scale tool must rebuild the collider and commit it with the transform as one entry | `src/editor_game/gizmo_drag_tests.rs test_scale_drag_rebuilds_the_collider_and_undoes_it_with_the_transform_as_one_entry` |
| Snapping accumulated position eats sub-cell residuals: drags must apply `start + delta`, never `+=` | `src/editor_game/gizmo_drag_tests.rs test_slow_snapped_drag_steps_grid_cells_instead_of_freezing` |
| A widget's release frame is not `Active`, but it is the frame picking decides on; `chrome_owns_mouse` must hold through release | `src/editor_game/viewport_interaction_tests.rs test_chrome_owns_mouse_through_the_release_frame_and_under_an_overlay` |
| World snapshot restore replaces the world wholesale; `Stop` must reset the transform-propagation baseline | `src/editor_game/play_session_tests.rs test_stop_resets_the_transform_propagation_baseline` |
| A paused world is mid-simulation; save/new/open must be refused while Paused, not just Playing | `src/editor_game/play_session_tests.rs test_save_is_refused_mid_session_and_allowed_after_stop` |
| `load_scene` must dry-run into a scratch World so a parse or instantiate failure never corrupts the live world | `src/editor_game/scene_io_tests.rs test_failed_parse_or_missing_file_preserves_the_live_world` |
| Scene saves must reach `common::vfs` (via `scene_serializer`) so parent creation and wasm storage are handled uniformly | `src/editor_game/scene_io_tests.rs test_save_scene_with_creates_parent_directories_and_writes_valid_scene` |
| The host never invents physics: a scene with no `physics:` block runs Play with no `PhysicsSystem` and behaviors move transforms directly (with physics present, a body-less entity's velocity goes to a rapier body that does not exist) | `src/project_host.rs test_physics_builds_only_when_the_scene_declares_physics_settings`, `test_patrol_entity_advances_over_playing_frames_without_physics` |
| Editor shortcuts must respect text focus so typing in an inspector field does not trigger global shortcuts | `src/editor_game/shortcuts_tests.rs test_key_routing_respects_text_focus_play_state_and_the_dialog` |


## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

This crate maps to Godot's editor plugin + node integration layer:
- `editor/editor_node.cpp` — how Godot's editor wraps the running scene
- `editor/scene_tree_dock.cpp` — entity CRUD operations (create, delete, duplicate, reparent)
- `editor/plugins/canvas_item_editor_plugin.cpp` — viewport interaction, picking, gizmo wiring
- `editor/editor_inspector.cpp` — how property changes flow back to objects
- `editor/editor_undo_redo_manager.cpp` — command pattern equivalent
