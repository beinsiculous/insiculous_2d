# playground — Crate Guide

The editor in the browser: a `cdylib` + `rlib` that boots the editor over `ProjectHost`
on wasm32, keeps the open project in IndexedDB, and exposes the command API and a file
interface to the page as `wasm_bindgen` functions. Everything that can run natively is
target-agnostic and unit-tested natively; the wasm-only files are the thin layer and are
compiled by `scripts/check_wasm.sh`. The contract is `docs/WEB_PLAYGROUND.md`.

## Dependency graph
```
playground ──→ editor_integration, editor, engine_core, audio, music_midi, common   (+ the wasm bridge family on wasm32 only)
```
No crate depends on `playground`. It is the only workspace member that pulls the editor
crates into the wasm gate.

## File Map
- `lib.rs` — module list; `web_entry` is wasm-only.
- `web_entry.rs` (wasm) — `ASSET_BASE`, `BUNDLE_VERSION` (the five-place version contract in the header), boot order: `?mode=preview` returns to `preview_entry` before anything else; otherwise logging → preload → open the store (memory fallback + banner) → sweep orphans → manifests → pick the project from `?project=` → load stored files onto `MemFs` (a tombstone's bytes never; its path dropped, the bundled copy with it) → seed the chains (tombstones included) → the write and remove observers + listeners → bridge channels → dispatch `playground-ready` → the hidden-frame pump (the preview window hides this tab, and the snapshot must still be answered) → `run_game_with_editor_opts`.
- `bridge.rs` — the `playground_*` exports and the pure rules behind them (`validate_bridge_path`, `can_dispatch`, `dirty_or`); the internal `live_scene(generation)` behind `playground_snapshot` and the live-scene `playground_export_zip`; `Hooks` for `source_check` / `script_errors` / `scene_snapshot` / `preview_open` / `open_song`; `playground_write_file_bytes` and `playground_sample_references`, the file import's two exports over `upload.rs`.
- `upload.rs` — the file import: `plan_upload` (the name rule, the loader's byte limit before any decode, the loader itself), `import_file` (the destination by kind, a taken name answered unwritten unless replaced, the open song's `.mid` refused, a replaced song's sidecar and journal pair removed before its MIDI is written), `songs_referencing`; `upload/tests.rs`.
- `store.rs` — `ProjectStore` (the CAS `put`, and `remove_file`, which writes a tombstone), `StoredFile` (`deleted` marks a tombstone), `StoreError`, `Fut`.
- `store/directory.rs` — native test double, lock file per project.
- `store/memory.rs` — every target; the fallback when IndexedDB will not open.
- `store/indexed_db/{mod,cursors}.rs` (wasm) — database `beinsiculous.playground` v1, stores `files` and `projects`; the CAS `put` and `remove_file`.
- `store/idb_transaction.rs` (wasm) — the transaction-to-future adapter; the only place web-sys IndexedDB verbosity lives.
- `persist/mod.rs` — `Chains`: what the store holds per path (`PathChain`), the five path states, `is_pending` vs `has_active`, the DOM banner; the wasm driver and listeners.
- `persist/queue.rs` — the one FIFO of puts and removals in write order: `on_vfs_write`, `on_vfs_remove`, `poll_head`, `handle_completion`, the head-of-line retry.
- `persist/tests/` — `mod.rs` (the gated store, the pong-chains helpers), `chains.rs` (hand-polled state-machine tests), `queue.rs` (write order, removals, tombstones), `song_saves.rs` (a song's save journal reopened after every operation), and `stores.rs` (native directory double tests).
- `projects.rs` — `ProjectManifest`, `ProjectEntry`, `list_projects` (pure merge), `validate_slug`, the computed project root.
- `archive.rs` — target-agnostic project zip export and import validation (`collect_asset_entries` + `write_archive` behind `export_project` and `export_snapshot`, which splices the live scene and the open song's live files and drops only that song's journal pair); `archive/songs.rs` (step 14: a final song file refuses the archive, a journal leftover is left out with a note); `archive/tests.rs`, `archive/song_tests.rs`.
- `preview.rs` — target-agnostic: `unpack_preview` (the importer's refusals plus `MissingScene`) and `preview_state`, the rule behind the page's readiness question (a scene error, then the boot status by phase: progress before the first frame, terminal after it).
- `preview_entry.rs` (wasm) — the `?mode=preview` runtime: `announce_preview_mode` and the four `playground_preview_*` / `playground_load_preview` exports. No store, no chains, no bridge, no preload.
- `assets/projects/pong/` — bundled project: pong scene (`scenes/pong.scene.ron`), paddle/ball textures, and gameplay scripts (`scripts/{paddle,ball,goal,scoreboard}.rhai`).

## Pitfalls and their guard tests

| Pitfall | Guard Test |
|---|---|
| A put issued while one is in flight for the same path must chain behind it with `base + 1`, never race it into a `StaleRevision` | `persist/tests/chains.rs test_two_puts_chain_with_gated_store` |
| The save indicator reads two signals: chain state alone calls a dirty history "Saved", the history flag alone calls a pending put "Saved" | `persist/tests/chains.rs test_save_status_walks_the_four_states_and_the_dirty_flag` |
| The history half of that signal must be the history-only flag, not the combined one the window title renders: the combined flag lags a put that completes between frames, and a poll landing in that window reports unsaved edits over a fully saved scene | `editor_integration/src/editor_game/history_dirty_tests.rs test_history_dirty_flag_tracks_command_history_alone_not_the_combined_signal` |
| A conflicted put holds the queue: nothing behind it issues, and the reason names its path | `persist/tests/chains.rs test_a_conflicted_put_holds_the_queue_and_names_its_path` |
| The store sees one operation at a time, in write order and never coalesced, so every durable prefix is a state the song's open-time rules settle; overlapping saves reopen the latest pair that landed | `persist/tests/queue.rs test_two_paths_written_in_order_land_in_that_order_and_never_overlap_in_flight` `persist/tests/queue.rs test_a_path_written_twice_lands_twice_in_order` `persist/tests/song_saves.rs test_overlapping_saves_reopen_the_latest_song_whose_pair_landed_after_every_operation` |
| Failure is head-of-line: a failed final strands the head and its save's removals never issue; the head retries on the next write as well as on `visibilitychange` | `persist/tests/song_saves.rs test_a_failed_final_strands_the_head_its_removals_never_issue_and_the_retry_lands_the_rest` `persist/tests/chains.rs test_a_stranded_head_retries_on_the_next_write_and_lands_before_it` |
| A removal is a tombstone one revision up, so a recreate names that revision across a reload and a stale tab's removal is stale; a stale removal is dropped, never a conflict | `store/tests.rs test_memory_store_satisfies_store_contract_suite` `persist/tests/queue.rs test_remove_reseed_recreate_lands_at_the_tombstones_next_revision` `persist/tests/song_saves.rs test_save_reseed_save_lands_because_the_journal_tombstones_carry_their_revisions` `persist/tests/queue.rs test_a_removal_from_a_tab_that_loaded_an_older_revision_is_dropped_and_the_queue_moves_on` |
| A write behind its own path's removal waits for it, queued or issued, and lands one revision above the tombstone: a removal cancelled would leave the store's states out of write order (an import's replaced sidecar surviving beside the new MIDI) | `persist/tests/queue.rs test_a_write_behind_its_own_queued_removal_waits_for_it_and_lands_above_the_tombstone` `persist/tests/queue.rs test_a_write_behind_an_issued_removal_waits_and_lands_one_revision_above_the_tombstone` |
| A replaced upload removes the sidecar and the journal pair before it writes the MIDI, so every boundary opens | `persist/tests/song_saves.rs test_a_replaced_upload_removes_the_sidecar_and_pair_first_and_every_boundary_opens` |
| The boot puts no tombstone's bytes in `MemFs` and drops the bundled copy of its path, or a removed file comes back on reload — empty, or as the bundle shipped it | — none; browser check |
| A conflicted path is terminal: no re-issue, no new put on a later write | `persist/tests/chains.rs test_conflicted_path_never_reissued` |
| A file loaded at revision N saves as N + 1 with no conflict (seed records the base) | `persist/tests/chains.rs test_seed_then_save_advances_revision_without_conflict` |
| Two writers from the same base: exactly one wins | `persist/tests/stores.rs test_two_writers_racing_from_same_base_exactly_one_wins` |
| A write during a drain is refused, not queued | `persist/tests/chains.rs test_writes_during_a_drain_are_refused_until_the_epoch_is_restored` |
| A drain finishes everything written before it and refuses the writes after, or a switch right after a save keeps the save's first operation and loses the rest | `persist/tests/chains.rs test_drain_finishes_the_queue_written_before_it_and_refuses_new_writes` |
| A drain retries a stranded head once — the switch is the visitor acting — and a head stranded again, at its start or under it, is not active: `drain_then_epoch` then refuses the switch naming the path (the queue behind it would go with the page), where a conflicted head lets it go | `persist/tests/chains.rs test_a_drain_retries_a_stranded_head_once_and_a_second_failure_leaves_it_stranded_not_active` — the refusal itself is wasm-only, a browser check |
| A restored epoch (a drain's timeout, a failed `replace_project`) issues the next operation, and a drain's start retries a stranded head: on wasm nothing drives either unless the caller calls `drive_started_operations` right after, and a drain that forgets times out on its own retry | — none; browser check |
| `sweep_orphans` removes only manifest-less, non-bundled slugs | `persist/tests/stores.rs test_sweep_orphans_removes_non_bundled_manifestless_slugs` |
| A bridge path may not escape the project root (`..`, leading `/`, empty) | `bridge.rs test_validate_bridge_path_rules` |
| A whitespace-only line or a full queue is refused, never dropped | `bridge.rs test_can_dispatch_refuses_empty_whitespace_and_full_channel` |
| `vfs::write_string` notifies the observer once; boot `insert` never does | `common/src/vfs/tests.rs test_write_string_notifies_observer_once` |
| Only the base-joined key resolves; a relative key never does | `common/src/vfs/tests.rs test_memfs_key_story_bundled_edit_and_relative_miss` |
| On wasm every started operation must be spawned and its result fed back, or the head sits in flight forever and the queue behind it with it | — none; browser check |
| A dependent IndexedDB request must be issued inside the previous `onsuccess`, never after an `await` (the transaction is inactive by then) | — none; wasm-only |
| The adapter's abort handler must surface the CAS result cell, or a `StaleRevision` arrives as `Backend` | — none; wasm-only |
| An import's `replace_project` failure must restore the epoch, or the current project stops saving until reload | — none; browser check |
| The preview loads the scene entry it is handed, and the bundled projects keep their scenes directly under `assets/scenes/` so the editor's default and `first_scene_in` agree | `preview.rs test_bundled_projects_keep_their_scenes_directly_under_assets_scenes` |
| The preview page never opens the store or installs the write observer | — none; browser check |
| A hidden tab gets no animation frames: the editor page and the preview both install the hidden-frame pump, and a page hidden all through its boot is pumped from the moment its loop exists | — none; browser check |
| Export carries the open song's live files and drops only its own journal pair, so another song's interrupted save keeps its recovery | `archive/song_tests.rs export_carries_the_live_song_over_its_saved_files_and_drops_only_its_own_journal_pair` |
| Import refuses a final sidecar without its `.mid` and a `.mid` without `MThd`, and leaves a failing journal file out instead of refusing | `archive/song_tests.rs import_refuses_a_final_sidecar_without_its_midi_file_and_a_mid_that_is_not_midi` `archive/song_tests.rs import_takes_a_lone_journal_sidecar_and_leaves_out_a_truncated_one_with_a_note` |
| An imported file is refused by its name, then by its size, then by its loader, before anything is written: a quote would break the page's quoted `song open`, whitespace the provenance gate | `upload/tests.rs a_name_that_would_break_the_quoted_open_the_gate_or_the_path_rule_is_refused_and_a_samples_space_becomes_an_underscore` `upload/tests.rs every_kind_is_refused_by_its_size_before_any_decode` `upload/tests.rs an_empty_a_truncated_and_a_header_only_file_of_each_kind_is_refused` |
| A replaced song is never the composer's open one, and loses its sidecar and journal pair before its MIDI is written | `upload/tests.rs a_replaced_song_is_refused_while_open_and_otherwise_loses_its_sidecar_and_journal_first` |
| A zip's decompressed size is capped as it is read, not after | `archive/tests.rs test_import_project_refuses_archive_exceeding_decompressed_cap` |

## Godot Oracle
Godot keeps projects on a real filesystem; the closest reference is `editor/editor_file_system.cpp`
for scan/refresh semantics. The CAS-per-file design has no Godot analogue.
