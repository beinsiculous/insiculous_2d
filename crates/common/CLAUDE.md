# Common Crate — Agent Context

Shared types used across all crates. A module used by fewer than three crates does not belong in `common` (graduation rule). Its only dependencies are `glam`, `serde`,
and `bytemuck` (plus wasm-only `web-time`) — anything added here
must stay dependency-light, headless, and GPU-free. (Vector/matrix math comes
straight from `glam`; there is no engine-owned math module.)

## File Map
- `clock.rs` — import time types from here, never `std::time` directly (`std::time` panics on wasm; `Duration` stays std).
- `vfs/` — asset and scene read/write/list seam (`read`, `file_size`, `change_stamp`, `write`, `list_files`, `remove_prefix`, `set_write_observer`, `set_remove_observer`; `entries.rs` adds `DirEntry`, `list_entries`, `create_dir_all`, `exists`, and `write_new`); `change_stamp` is a file's length and modified time natively and `None` on wasm, whose map keeps no clock, so a cache keyed by it falls back to content; canonical key is joined path string `{asset_base}/{relative entry}`, with `list_dir_files` and `list_files` sorted on both native and wasm; `set_write_observer` (wasm-only, compiled under `cfg(test)` natively so its once-per-write contract is pinned) notifies the playground's persistence chains on every successful `write` or exclusive `write_new`, and the `.keep` file that `create_dir_all` writes on wasm; boot-phase `insert` never fires it; `set_remove_observer`, its twin, fires once for every key `remove_prefix` drops on wasm (`MemFs::remove_prefix` returns them) and is never called natively, where the filesystem is the only copy.
- `vfs/entries.rs` — direct children, including empty native directories and directories implied by web paths; symlink entries list nothing, and a linked root is followed as `list_files` follows it; an entry whose metadata fails is skipped. A native `write_new` whose write fails after the create removes the half file, so the retry is not refused. `create_dir_all` keeps a web directory with `.keep`, included in exports. `write_new` requires an existing parent and refuses a live file or directory, through native `create_new` or the memory map's live-key check. The existing write observer carries web puts to the persistence chain, which supplies the current base revision even when recreating a tombstoned path. The `MemFs` methods are the same web algorithms tested natively.
- `color.rs` — `Color`: WCAG `luminance()` and `contrast_ratio()` back the editor theme's surface-ladder guard tests.
- `camera.rs` — `Camera` and `CameraUniform` (view/projection uniform uploaded by renderer; defined here only to avoid cross-crate duplication).
- `sheet_grid.rs` — `SheetGrid`: `from_uv_size` preserves non-reciprocal cell sizes for `ecs::Tilemap`, and `from_cell_size` truncates partial trailing cells.
- `hash.rs` — deterministic `hash_u32` and `hash_f32` for frame-driven pseudo-random values.

Every type above is re-exported at the crate root; `Color`, `Transform2D`,
`Camera`, `Rect`, and `SheetGrid` are also in `common::prelude`.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Importing time types directly from `std::time` panics on wasm32; import from `common::clock` instead | — none |
| Wasm asset lookups require the exact joined key `{asset_base}/{relative entry}` matching preloaded VFS keys | `src/vfs/tests.rs test_boot_phase_keys_resolve_through_base_joined_reads` |
| A removal reaches the playground's store only through the remove observer, once per key `MemFs::remove_prefix` drops; natively nothing is told, since the filesystem is the only copy | `src/vfs/tests.rs test_mem_fs_remove_prefix_leaves_siblings` `src/vfs/tests.rs test_native_remove_prefix_never_notifies_the_remove_observer` |
| Scene and asset writes must go through `common::vfs::write` / `write_string` so parent directories are created natively and writes land in `MemFs` on wasm | `src/vfs/tests.rs test_mem_fs_write_and_read_round_trip` |
| Degenerate or zero-dimension sheet grids must clamp to one usable cell without dividing by zero | `src/sheet_grid.rs test_degenerate_grids_clamp_to_one_usable_cell_without_dividing_by_zero` |
| Screen-space Y is down while world-space Y is up in camera conversions | `src/camera.rs test_screen_y_down_maps_to_world_y_up` |
| A folder scan must include empty directories and direct children, follow the configured root as `list_files` does (a linked asset root still lists) and never a symlink entry below it; one unreadable entry is skipped, not its directory | `src/vfs/tests.rs test_directory_entries_include_empty_folders_and_direct_files_only` `src/vfs/tests.rs test_directory_entries_follow_the_root_but_never_a_file_or_directory_symlink_entry` |
| Empty web folders need an exported `.keep` marker, and directory prefixes must end at a slash boundary | `src/vfs/tests.rs test_memory_directory_entries_derive_each_folder_once_and_include_keep_files` |
| Exclusive creation must preserve a live file and notify persistence only after success; web paths must also refuse directory and ancestor-file collisions | `src/vfs/tests.rs test_write_new_refuses_live_paths_preserves_bytes_and_notifies_only_success` `src/vfs/tests.rs test_memory_exclusive_create_refuses_live_files_and_directory_collisions_but_recreates_removed_paths` |
| An empty path must not create a file or scan the whole web filesystem | `src/vfs/tests.rs test_directory_and_exclusive_create_operations_refuse_empty_paths` |

## Testing
- `cargo test -p common` — 0 failed, 0 ignored

