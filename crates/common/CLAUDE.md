# Common Crate — Agent Context

Shared types used across all crates. A module used by fewer than three crates does not belong in `common` (graduation rule). Its only dependencies are `glam`, `serde`,
and `bytemuck` (plus wasm-only `web-time`) — anything added here
must stay dependency-light, headless, and GPU-free. (Vector/matrix math comes
straight from `glam`; there is no engine-owned math module.)

## File Map
- `clock.rs` — import time types from here, never `std::time` directly (`std::time` panics on wasm; `Duration` stays std).
- `vfs/` — asset and scene read/write/list seam (`read`, `file_size`, `change_stamp`, `write`, `list_files`, `remove_prefix`, `set_write_observer`, `set_remove_observer`; `entries.rs` adds `DirEntry`, `list_entries`, `create_dir_all`, `exists`, `write_new`, `remove_file`, `remove_dir` (an empty folder only) and `rename`); `change_stamp` is a file's length and modified time natively and `None` on wasm, whose map keeps no clock, so a cache keyed by it falls back to content; canonical key is joined path string `{asset_base}/{relative entry}`, with `list_dir_files` and `list_files` sorted on both native and wasm; `set_write_observer` (wasm-only, compiled under `cfg(test)` natively so its once-per-write contract is pinned) notifies the playground's persistence chains on every successful `write` or exclusive `write_new`, and the `.keep` file that `create_dir_all` writes on wasm; boot-phase `insert` never fires it; `set_remove_observer`, its twin, fires once for every key `remove_prefix`, `remove_file` or `rename` drops on wasm and is never called natively, where the filesystem is the only copy.
- `vfs/entries.rs` — direct children, including empty native directories and directories implied by web paths; symlink entries list nothing, and a linked root is followed as `list_files` follows it; an entry whose metadata fails is skipped. A native `write_new` fills a temporary sibling and hard-links it to the final name exclusively, so a failed write never exposes a half file at the final name; where the filesystem refuses the link for any reason but an existing destination (FAT's EPERM, a mount's EINVAL) it creates the file directly as before, a half-written file removed by its inode, and a failed write is the error whatever its kind, and a published file is a success even when its temporary sibling would not go. `create_dir_all` keeps a web directory with `.keep`, included in exports. `write_new` requires an existing parent and refuses a live file or directory, through a native hard-link publish or the memory map's live-key check. File `rename` hard-links without replacement and compares inode, device, and length before unlinking; unsupported links refuse instead of copying. Targets without a stable standard-library file-identity check refuse before linking. Directory `rename` claims the destination with an exclusive `create_dir` before the operating system's rename, since POSIX `rename` would replace an empty folder that appeared after the preflight. A folder is emptied file by file and its folders go deepest first through `remove_dir`, never recursively; there is no recursive removal in the seam. Web renames publish every destination before notifying any source removal; a persistence failure can leave a duplicate as recovery. The existing write observer carries web puts to the persistence chain, which supplies the current base revision even when recreating a tombstoned path. The `MemFs` methods are the same web algorithms tested natively; `rename_observed` and `remove_file_observed` release the map borrow before each callback so the persistence tests use the production observer path.
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
| Rename must preserve an existing destination, including an empty folder or a file that appears after preflight | `src/vfs/mutate_tests.rs test_rename_refuses_an_existing_file_or_empty_directory` `src/vfs/mutate_tests.rs test_rename_preserves_a_destination_created_between_preflight_and_link` `src/vfs/mutate_tests.rs test_rename_preserves_an_empty_folder_created_between_preflight_and_rename` |
| A source externally replaced before the identity check survives beside the linked original; unsupported links never fall back to copy-then-remove | `src/vfs/mutate_tests.rs test_rename_keeps_a_source_replaced_before_the_identity_check` `src/vfs/mutate_tests.rs test_rename_refuses_unsupported_hard_links_without_copying_or_removing` |
| A failed exclusive create never publishes partial final-name bytes or blocks a retry | `src/vfs/mutate_tests.rs test_exclusive_publish_never_exposes_a_partial_file_under_the_final_name` |
| A filesystem without hard links must still create new assets, exclusively | `src/vfs/mutate_tests.rs test_exclusive_publish_creates_directly_where_the_filesystem_refuses_links` |
| An emptied folder goes without recursion: one holding a file that arrived since is refused, natively and in memory | `src/vfs/mutate_tests.rs test_remove_dir_takes_an_empty_folder_and_refuses_one_that_holds_a_file` |
| A web folder rename notifies all destination puts before any source removal, including the `.keep` for an empty nested folder | `src/vfs/mutate_tests.rs test_memory_folder_rename_observes_every_put_before_any_remove` |
| An empty path must not create a file or scan the whole web filesystem | `src/vfs/tests.rs test_directory_and_exclusive_create_operations_refuse_empty_paths` |

## Testing
- `cargo test -p common` — 0 failed, 0 ignored

