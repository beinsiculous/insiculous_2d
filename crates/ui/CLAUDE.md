# UI Crate — Agent Context

Immediate-mode UI framework with fontdue text rendering.

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
- `context/` — UIContext: `mod.rs` (struct, lifecycle incl. `begin_frame_dt`, fonts, primitives incl. `image`/`rect_border`), `text.rs` (label/measure), `widgets.rs` (button, slider, checkbox), `text_input.rs` (float_input + free-form text_input: select-all-on-focus, cursor, selection, arrows/Home/End, key repeat; commits on Enter/Tab/click-away, Escape cancels), `tests.rs`
- `font/` — `mod.rs` (FontManager facade: loading/storage), `glyph_cache.rs` (GlyphCache; bitmaps shared via `Arc<[u8]>`), `layout.rs` (text layout/measurement)
- `draw.rs` — Draw command generation (`Rect` re-exported from `common`)
- `interaction/` — Widget state, mouse hit detection, focus, per-widget persistent state (`edit: TextEditState`); `wants_mouse()` = a widget owns the current press→release gesture (tests in `interaction/tests.rs`)
- `input_state.rs` — per-frame `InputState` snapshot + `KeyRepeat` (dt-driven hold repeat)
- `text_edit.rs` — pure `TextEditState` (buffer/cursor/selection editing model)
- `style.rs` — Theme definitions (`Color` re-exported from `common`), private palette consts

## Known Tech Debt
- See `TECH_DEBT.md` — all Lows: US-layout-only key mapping (JUN-T1b; general text input itself shipped Jul 2026), TextDrawData redundancy (ARCH-003), unused scroll_delta (JUN-T2), no layout helpers (JUN-T3)

## Testing
- 101 tests (incl. 2 doc), run with `cargo test -p ui`. `keycode_to_char` covers digits, A–Z (shift = uppercase), space, `.` `-` and Shift+Minus = `_`. `TextDrawData.font_id` marks which font glyphs came from — downstream glyph caches key on it

## Godot Oracle
- Immediate-mode patterns: Godot doesn't use immediate-mode, but see `scene/gui/control.cpp` for widget lifecycle
- Font rendering: `modules/text_server_advanced/text_server_adv.cpp`
