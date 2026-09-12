# playground — Crate Guide

The editor in the browser: a `cdylib` + `rlib` that boots the editor over `ProjectHost`
on wasm32, keeps the open project in IndexedDB, and exposes the command API and a file
interface to the page as `wasm_bindgen` functions. Everything that can run natively is
target-agnostic and unit-tested natively; the wasm-only files are the thin layer and are
compiled by `scripts/check_wasm.sh`. The contract is `docs/WEB_PLAYGROUND.md`.

## Dependency graph
```
playground ──→ editor_integration, engine_core, common   (+ the wasm bridge family on wasm32 only)
```
No crate depends on `playground`. It is the only workspace member that pulls the editor
crates into the wasm gate.

## File Map
- `lib.rs` — module list; `web_entry` is wasm-only.
- `web_entry.rs` (wasm) — `ASSET_BASE`, `BUNDLE_VERSION` (the five-place version contract in the header), boot order: `?mode=preview` returns to `preview_entry` before anything else; otherwise logging → preload → open the store (memory fallback + banner) → sweep orphans → manifests → pick the project from `?project=` → load stored files onto `MemFs` → seed the chains → observer + listeners → bridge channels → dispatch `playground-ready` → the hidden-frame pump (the preview window hides this tab, and the snapshot must still be answered) → `run_game_with_editor_opts`.
- `bridge.rs` — the `playground_*` exports and the pure rules behind them (`validate_bridge_path`, `can_dispatch`, `dirty_or`); the internal `live_scene(generation)` behind `playground_snapshot` and the live-scene `playground_export_zip`; `Hooks` for `source_check` / `script_errors` / `scene_snapshot` / `preview_open`.
- `store.rs` — `ProjectStore`, `StoredFile`, `StoreError`, `Fut`.
- `store/directory.rs` — native test double, lock file per project.
- `store/memory.rs` — every target; the fallback when IndexedDB will not open.
- `store/indexed_db/{mod,cursors}.rs` (wasm) — database `beinsiculous.playground` v1, stores `files` and `projects`; the CAS `put`.
- `store/idb_transaction.rs` (wasm) — the transaction-to-future adapter; the only place web-sys IndexedDB verbosity lives.
- `persist/mod.rs` — `Chains`: one chain per path, the five path states, `is_pending` vs `has_active`, the DOM banner; the wasm driver and listeners.
- `persist/tests/` — `mod.rs`, `chains.rs` (hand-polled state-machine tests), and `stores.rs` (native directory double tests).
- `projects.rs` — `ProjectManifest`, `ProjectEntry`, `list_projects` (pure merge), `validate_slug`, the computed project root.
- `archive.rs` — target-agnostic project zip export and import validation (`collect_asset_entries` + `write_archive` behind `export_project` and `export_snapshot`); `archive/tests.rs`.
- `preview.rs` — target-agnostic: `unpack_preview` (the importer's refusals plus `MissingScene`) and `preview_state`, the rule behind the page's readiness question (a scene error, then the boot status by phase: progress before the first frame, terminal after it).
- `preview_entry.rs` (wasm) — the `?mode=preview` runtime: `announce_preview_mode` and the four `playground_preview_*` / `playground_load_preview` exports. No store, no chains, no bridge, no preload.
- `assets/projects/pong/` — bundled project: pong scene (`scenes/pong.scene.ron`), paddle/ball textures, and gameplay scripts (`scripts/{paddle,ball,goal,scoreboard}.rhai`).

## Pitfalls and their guard tests

| Pitfall | Guard Test |
|---|---|
| A put issued while one is in flight for the same path must chain behind it with `base + 1`, never race it into a `StaleRevision` | `persist/tests/chains.rs test_two_puts_chain_with_gated_store` |
| The save indicator reads two signals: chain state alone calls a dirty history "Saved", the history flag alone calls a pending put "Saved" | `persist/tests/chains.rs test_save_status_walks_the_four_states_and_the_dirty_flag` |
| The history half of that signal must be the history-only flag, not the combined one the window title renders: the combined flag lags a put that completes between frames, and a poll landing in that window reports unsaved edits over a fully saved scene | `editor_integration/src/editor_game/history_dirty_tests.rs test_history_dirty_flag_tracks_command_history_alone_not_the_combined_signal` |
| A conflicted path is named ahead of a stranded one in the failure reason — a conflict is terminal, a stranded path still retries | `persist/tests/chains.rs test_save_status_names_a_conflicted_path_ahead_of_a_stranded_one` |
| A conflicted path is terminal: no re-issue, no new put on a later write | `persist/tests/chains.rs test_conflicted_path_never_reissued` |
| A file loaded at revision N saves as N + 1 with no conflict (seed records the base) | `persist/tests/chains.rs test_seed_then_save_advances_revision_without_conflict` |
| Two writers from the same base: exactly one wins | `persist/tests/stores.rs test_two_writers_racing_from_same_base_exactly_one_wins` |
| A write during a drain is refused, not queued | `persist/tests/chains.rs test_writes_during_a_drain_are_refused_until_the_epoch_is_restored` |
| `sweep_orphans` removes only manifest-less, non-bundled slugs | `persist/tests/stores.rs test_sweep_orphans_removes_non_bundled_manifestless_slugs` |
| A bridge path may not escape the project root (`..`, leading `/`, empty) | `bridge.rs test_validate_bridge_path_rules` |
| A whitespace-only line or a full queue is refused, never dropped | `bridge.rs test_can_dispatch_refuses_empty_whitespace_and_full_channel` |
| `vfs::write_string` notifies the observer once; boot `insert` never does | `common/src/vfs/tests.rs test_write_string_notifies_observer_once` |
| Only the base-joined key resolves; a relative key never does | `common/src/vfs/tests.rs test_memfs_key_story_bundled_edit_and_relative_miss` |
| On wasm every started put must be spawned and its result fed back, or the chain sits in flight forever (the round-1 defect) | — none; browser check |
| A dependent IndexedDB request must be issued inside the previous `onsuccess`, never after an `await` (the transaction is inactive by then) | — none; wasm-only |
| The adapter's abort handler must surface the CAS result cell, or a `StaleRevision` arrives as `Backend` | — none; wasm-only |
| An import's `replace_project` failure must restore the epoch, or the current project stops saving until reload | — none; browser check |
| The preview loads the scene entry it is handed, and the bundled projects keep their scenes directly under `assets/scenes/` so the editor's default and `first_scene_in` agree | `preview.rs test_bundled_projects_keep_their_scenes_directly_under_assets_scenes` |
| The preview page never opens the store or installs the write observer | — none; browser check |
| A hidden tab gets no animation frames: the editor page and the preview both install the hidden-frame pump, and a page hidden all through its boot is pumped from the moment its loop exists | — none; browser check |
| A zip's decompressed size is capped as it is read, not after | `archive/tests.rs test_import_project_refuses_archive_exceeding_decompressed_cap` |

## Godot Oracle
Godot keeps projects on a real filesystem; the closest reference is `editor/editor_file_system.cpp`
for scan/refresh semantics. The CAS-per-file design has no Godot analogue.
