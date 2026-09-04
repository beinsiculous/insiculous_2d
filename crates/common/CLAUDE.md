# Common Crate — Agent Context

Shared types used across all crates. A module used by fewer than three crates does not belong in `common` (graduation rule). Its only dependencies are `glam`, `serde`,
and `bytemuck` (plus wasm-only `web-time`) — anything added here
must stay dependency-light, headless, and GPU-free. (Vector/matrix math comes
straight from `glam`; there is no engine-owned math module.)

## File Map
- `clock.rs` — import time types from here, never `std::time` directly (`std::time` panics on wasm; `Duration` stays std).
- `vfs/` — asset and scene read/write/list seam (`read`, `write`, `list_files`, `remove_prefix`, `set_write_observer`); canonical key is joined path string `{asset_base}/{relative entry}`, with `list_dir_files` and `list_files` sorted on both native and wasm; `set_write_observer` (wasm-only, compiled under `cfg(test)` natively so its once-per-write contract is pinned) notifies the playground's persistence chains on every `write`; boot-phase `insert` never fires it.
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
| Scene and asset writes must go through `common::vfs::write` / `write_string` so parent directories are created natively and writes land in `MemFs` on wasm | `src/vfs/tests.rs test_mem_fs_write_and_read_round_trip` |
| Degenerate or zero-dimension sheet grids must clamp to one usable cell without dividing by zero | `src/sheet_grid.rs test_degenerate_grids_clamp_to_one_usable_cell_without_dividing_by_zero` |
| Screen-space Y is down while world-space Y is up in camera conversions | `src/camera.rs test_screen_y_down_maps_to_world_y_up` |

## Testing
- `cargo test -p common` — 0 failed, 0 ignored

