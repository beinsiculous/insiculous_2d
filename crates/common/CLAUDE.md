# Common Crate — Agent Context

Shared types used across all crates. Its only dependencies are `glam`, `serde`,
`thiserror`, and `bytemuck` (plus wasm-only `web-time`) — anything added here
must stay dependency-light, headless, and GPU-free. (Vector/matrix math comes
straight from `glam`; there is no engine-owned math module.)

## File Map
- `clock.rs` — time-source alias: re-exports `std::time::{Instant, SystemTime,
  UNIX_EPOCH, SystemTimeError}` natively, `web_time::*` on wasm32 (those types
  panic on the web). Import time types from here, never `std::time` directly
  (`Duration` stays std). H2 (Aug 2026)
- `vfs.rs` — asset-read seam: `read`/`read_to_string`/`list_dir_files` are
  `std::fs` passthroughs natively; on wasm they serve an in-memory map the web
  boot phase fills via `insert`. **Canonical key = the joined path string
  `{asset_base}/{relative entry}`** — `MemFs` (the map) compiles + is unit
  tested on ALL targets so the browser's exact lookup semantics are covered by
  the native suite. `list_dir_files` is sorted + direct-children-only on both
  targets. H5 (Aug 2026)
- `color.rs` — `Color` (the engine-wide color type; WCAG `luminance()` / `contrast_ratio()` back the editor theme's surface-ladder guard tests)
- `rect.rs` — `Rect` (axis-aligned bounds: hit tests, intersection/union, UI layout)
- `transform.rs` — `Transform2D` (position/rotation/scale + point transforms)
- `camera.rs` — `Camera` (2D camera) and `CameraUniform` (view/projection data
  uploaded by the renderer; defined here only, not duplicated in renderer)
- `sheet_grid.rs` — `SheetGrid`: row-major cell grid over a sprite sheet /
  tileset plus its cell-index → UV-region math. Constructors: `new(cols, rows)`,
  `from_cell_size` (pixel sizes, truncating — partial trailing cells excluded),
  `from_uv_size` (the compatibility constructor `ecs::Tilemap` uses — it stores
  the given UV size verbatim, which is what keeps non-reciprocal sizes like
  `0.3` bit-identical). `uv_rect` is unchecked (out-of-range indices pass
  through and the sampler clamps); `uv_rect_checked` is the guarded variant
- `hash.rs` — `hash_u32` / `hash_f32` deterministic hashing for frame-driven
  pseudo-random values
- `time.rs` — `Time` (delta/elapsed tracking)
- `macros.rs` — small boilerplate-reduction macros

Every type above is re-exported at the crate root; `Color`, `Transform2D`,
`Camera`, `Rect`, `Time`, and `SheetGrid` are also in `common::prelude`.

## Testing
- `cargo test -p common` — 0 failed, 0 ignored
