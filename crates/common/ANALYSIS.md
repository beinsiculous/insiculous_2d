# Common Crate Analysis

> **Audit note (2026-04-15):** Reviewed against current source. No checked-off
> TODOs to prune — the existing items are ongoing design guidelines rather
> than discrete completed tasks. Added a Cross-Crate Duplication note to
> surface the `CameraUniform` tech-debt (ARCH-001) so it lives in the
> analysis document too. (ARCH-001 has since been resolved — see below.)

## Review (January 19, 2026)

### Summary
- Shared types and utilities used across the engine (`Color`, `Transform2D`, `Camera`, `Rect`, `Time`).
- Keeps base math and data structures centralized to reduce duplication.
- Minimal dependency set with `glam`, `serde`, and `bytemuck`.
- `macros` provides a lightweight `with_fields!` helper for builder-style setters.

### Strengths
- Clear module layout with prelude for ergonomic imports.
- Keeps cross-crate types consistent (rendering, physics, UI).
- Small API surface makes versioning and compatibility easier.

### Risks & Follow-ups
- Keep `common` lean with the documented graduation rule (move modules used by fewer than 3 crates).
- Monitor `macros` usage to avoid domain-specific helpers creeping into the crate.
- Confirm serialization expectations for shared types remain stable.
- Consider adding tests for core math helpers if the module grows.

### Cross-Crate Duplication (RESOLVED)
- ARCH-001 is resolved: `renderer::sprite_data` now re-exports
  `common::{Camera, camera::CameraUniform}` — a single GPU-facing layout,
  defined in this crate only (verified Aug 28 2026).
- Historical `Camera2D` / `Camera` naming was unified (commit `947e359`
  renamed `Camera2D` to `Camera`); when adding new camera features,
  re-verify that renderer-side camera helpers converge on `common::Camera`
  instead of spawning parallel types.