# Technical Debt: ui — LIVE (open items only)

Last audited: June 2026 (July 2026: Game Programming Patterns audit).
Resolved history: root `log_archive.md` § ui.

## Game Programming Patterns Audit (July 2026 — closed; history in `log_archive.md`)
- [ ] **GPP-L8 (Low, Flyweight):** `GlyphInfo` stores `character`/`font_size` duplicating its cache key (`font/glyph_cache.rs:38-42`); `TextDrawData` char duplication tracked as ARCH-003 below. Strip opportunistically.

## Open Items

### [JUN-T1b] Physical-key→char mapping is US-layout-only — Low
- **File:** `input_state.rs` (`keycode_to_char`)
- **Issue:** general text input shipped Jul 2026 (`text_input` widget; digits, A–Z with shift-uppercase, space, `_`), but chars are still synthesized from physical keycodes — non-US layouts type as if US-QWERTY, and punctuation beyond `. - _` is untypeable.
- **Fix:** plumb winit character events through `InputHandler` (input crate change).
- **Priority:** Low (editor string fields work; layout correctness is polish) | **Effort:** Medium

### [ARCH-003] TextDrawData duplicates GlyphDrawData info — Low
- **File:** `draw.rs:26-43` — `text: String` + per-glyph `character` duplicate character info.
- **Fix:** remove `text` (reconstruct from glyphs) or remove `character` from `GlyphDrawData`.

### [JUN-T3] No layout helpers — Low (roadmap)
- Every caller hand-places absolute `Rect`s; no row/column/anchor layout. Phase 2+ concern.

## Metrics

| Metric | Value (June 2026) |
|--------|-------------------|
| Test coverage | 109 tests (100% pass rate) |
| `#[allow(...)]` | 2 (documented clippy exceptions) |
| High priority open | 0 |
| Medium priority open | 0 |
| Low priority open | 4 (JUN-T1b, ARCH-003, JUN-T3, GPP-L8) |
