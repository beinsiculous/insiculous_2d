# UI Crate Analysis

> **Audit (2026-04-15):** Removed the "Font Rendering First-Frame Bug Fix"
> deep-dive section — the bug was fixed in commit `023bbf1` and the regression
> is guarded by `test_font_rendering_retry_after_font_load()` in `context.rs`.
> Kept the review summary (strengths, risks, architectural notes) and added a
> brief historical callout so the fix's rationale stays discoverable via git.
> Active debt lives on the Studio Board (issue #88, `tech-debt` label).

## Review (January 2026, still current)

### Summary
- Immediate-mode UI system with context-driven widget creation and draw command output.
- Modules cover context, draw batching, font handling, interaction, and styling.
- Depends on `input` for interaction and `fontdue` for font rasterization.
- The renderer is *not* a dependency — the UI crate emits `DrawCommand`s which
  `engine_core::ui_integration::render_ui_commands` translates into sprite
  batches. This keeps the UI crate renderer-agnostic.

### Strengths
- Immediate-mode API keeps usage lightweight and easy to integrate.
- Draw command output keeps renderer-specific concerns out of the UI crate.
- Style/theme types are re-exported for easy reuse in gameplay code.
- `Rect` is re-exported from `common` rather than duplicated (ARCH-002,
  resolved — history in `log_archive.md`).

### Risks & Follow-ups
- `FontManager` bundles loading, rasterization, caching, and layout — fine at
  current scale but flagged for splitting if responsibilities grow
  (SRP-001, since resolved — history in `log_archive.md`).
- Document the engine_core bridge (`render_ui_commands` in
  `crates/engine_core/src/ui_integration.rs`) as the canonical integration
  point for anyone wiring a new host application.
- Consider adding more worked examples for UI composition and layout patterns —
  current consumers are `hello_world.rs` and the editor panels.

## Cross-Crate Integration

The UI crate is deliberately isolated from rendering:

```
ui (emits DrawCommand) → engine_core::ui_integration → renderer (sprites/batches)
```

- `UIContext::end_frame()` returns a `&[DrawCommand]` slice.
- `engine_core` owns a CPU→GPU glyph texture cache (`glyph_textures` in
  `contexts.rs`), separate from the UI crate's CPU-side glyph bitmap cache in
  `FontManager`. The dual cache is intentional: the UI crate caches rasterized
  bitmaps to avoid re-rasterization, `engine_core` caches GPU textures to
  avoid re-uploads.
- Clip-rect support is exposed via `PushClipRect`/`PopClipRect` draw commands
  (commits `fb29094`, `7deb96d`) — consumers must implement scissor testing.

## Historical Fixes (for context)

These bugs have been fixed and are regression-guarded by tests; detailed
post-mortems live in git history rather than being duplicated here:

| Fix | Commit | Regression test / coverage |
|-----|--------|----------------------------|
| First-frame font placeholder flicker (removed static `PRINTED` flag in `label_styled`) | `023bbf1` | `test_font_rendering_retry_after_font_load` in `context.rs` |
| Glyph color multiplication (grayscale on all RGBA channels) | `a8571c0` | sprite color tests |
| Button glyph rendering (was placeholder only) | `b977ed5` | `button_styled` tests |
| `measure_text` re-rasterizing past the cache | `e0adda2` | `font.rs` metrics tests |
| `active_widget` click-detection timing | `2db46d1` | `interaction.rs` tests |
| Text centering in `label_in_bounds` and `button` | `65b5727` | layout tests in `context.rs` |

If any of these regress, `git log --oneline crates/ui/` contains the original
investigation commits.

## Active Tech Debt

Current list: Studio Board issue #88 (`tech-debt` label) — all Low as of
Aug 28 2026 (GPP-L8 glyph cache-key duplication, JUN-T1b US-layout-only key
mapping, ARCH-003 `TextDrawData`/`GlyphDrawData` field redundancy, JUN-T3 no
layout helpers). The Mediums this section used to list (DRY-004, SRP-001)
were resolved — history in `log_archive.md`.