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
- `web_entry.rs` (wasm) — `ASSET_BASE`, `BUNDLE_VERSION` (the five-place version contract in the header), boot order: logging → preload → open the store (memory fallback + banner) → sweep orphans → manifests → pick the project from `?project=` → load stored files onto `MemFs` → seed the chains → observer + listeners → bridge channels → dispatch `playground-ready` → `run_game_with_editor_opts`.
- `bridge.rs` — the `playground_*` exports and the pure rules behind them (`validate_bridge_path`, `can_dispatch`, `dirty_or`); `Hooks` for batch 7's `source_check` / `script_errors`.
- `store.rs` — `ProjectStore`, `StoredFile`, `StoreError`, `Fut`.
- `store/directory.rs` — native test double, lock file per project.
- `store/memory.rs` — every target; the fallback when IndexedDB will not open.
- `store/indexed_db/{mod,cursors}.rs` (wasm) — database `beinsiculous.playground` v1, stores `files` and `projects`; the CAS `put`.
- `store/idb_transaction.rs` (wasm) — the transaction-to-future adapter; the only place web-sys IndexedDB verbosity lives.
- `persist/mod.rs` — `Chains`: one chain per path, the five path states, `is_pending` vs `has_active`, the DOM banner; the wasm driver and listeners.
- `persist/tests/` — `mod.rs`, `chains.rs` (hand-polled state-machine tests), and `stores.rs` (native directory double tests).
- `projects.rs` — `ProjectManifest`, `ProjectEntry`, `list_projects` (pure merge), `validate_slug`, the computed project root.

## Pitfalls and their guard tests

| Pitfall | Guard Test |
|---|---|
| A put issued while one is in flight for the same path must chain behind it with `base + 1`, never race it into a `StaleRevision` | `persist/tests/chains.rs test_two_puts_chain_with_gated_store` |
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

## Godot Oracle
Godot keeps projects on a real filesystem; the closest reference is `editor/editor_file_system.cpp`
for scan/refresh semantics. The CAS-per-file design has no Godot analogue.
