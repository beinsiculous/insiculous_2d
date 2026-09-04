# UI Crate — Agent Context

Immediate-mode UI framework with fontdue text rendering. The dual glyph cache is intentional: `ui` caches rasterized bitmaps (`font/glyph_cache.rs`) to avoid re-rasterization, while `engine_core` caches GPU textures (`glyph_texture_cache.rs`) to avoid re-uploads.

## Pattern
```rust
ui.begin_frame(&input, window_size);
ui.panel(rect);
ui.label("text", pos);
if ui.button("id", "label", rect) { /* clicked */ }
let val = ui.slider("id", current, rect);
ui.end_frame(); // collects draw commands
```

## File Map
- `context/` — `UIContext` frame lifecycle, widgets (button, slider, checkbox), and edit fields (drag-scrub with 4px click threshold, Up/Down nudge, soft vs hard ranges).
- `font/` — `FontManager` loading and measurement; `GlyphCache` stores rasterized bitmaps as `Arc<[u8]>`.
- `draw/` — `DrawCommand` and `DrawList` with `UiLayer` z-bands; elevated layers flush after Content at `end_frame` so popups escape panel clipping.
- `interaction/` — widget state and persistent focus; `wants_mouse()` indicates a widget owns the current press→release gesture.
- `input_state.rs` — per-frame `InputState` snapshot and `KeyRepeat` (dt-driven hold repeat).

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| UI text y = baseline in `label_styled`; text inside a box must use `label_in_bounds_styled` to center via font metrics | `src/context/tests.rs test_label_in_bounds_styled_keeps_glyphs_inside_bounds_at_every_alignment` |
| An elevated `UiLayer` (Floating/Modal) must physically escape a Content-layer clip pair | `src/draw/tests.rs test_elevated_layer_escapes_content_clip_pair` |
| The release frame of a click is NOT `WidgetState::Active`, so consumers must gate on `wants_mouse` rather than widget state | `src/interaction/tests.rs test_wants_mouse_holds_from_widget_press_through_release_frame` |


## Known Tech Debt
- Tracked on the Studio Board: issue #88 — all Lows: US-layout-only key mapping (JUN-T1b; general text input itself shipped Jul 2026), TextDrawData redundancy (ARCH-003), no layout helpers (JUN-T3), GlyphInfo cache-key duplication (GPP-L8)

## Testing
- `cargo test -p ui` — 0 failed, 0 ignored. `keycode_to_char` covers digits, A–Z (shift = uppercase), space, `.` `-` and Shift+Minus = `_`. `TextDrawData.font_id` marks which font glyphs came from — downstream glyph caches key on it

## Godot Oracle
- Immediate-mode patterns: Godot doesn't use immediate-mode, but see `scene/gui/control.cpp` for widget lifecycle
- Font rendering: `modules/text_server_advanced/text_server_adv.cpp`
