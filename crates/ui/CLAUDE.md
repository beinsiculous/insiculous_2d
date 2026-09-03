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
- `context/` — UIContext: `mod.rs` (struct, lifecycle incl. `begin_frame_dt`, fonts, primitives incl. `image`/`rect_border`), `edit_field.rs` (shared editing shell: `edit_field`, `EditFieldEvent`, `EditFieldParams`, `resolve_font`, `apply_edit_keys`, `prefix_widths`, and box/editing drawing), `text.rs` (label/measure), `widgets.rs` (button, slider, checkbox), `text_input.rs` (float_input: `FloatFieldOpts`/`FloatInputResult`, drag-scrub with 4px click threshold + Escape-restore, Up/Down nudge w/ repeat, SOFT ranges — typed commits exceed min..=max unless hard_clamp, parse failure = red `border_invalid` + revert, display-only suffix; free-form text_input: select-all-on-focus, cursor, selection, arrows/Home/End, key repeat; commits on Enter/Tab/click-away, Escape cancels; scrub_tests.rs), `tests.rs` (widget, text-input, programmatic-focus (F2 rename) and `begin_overlay` contracts)
- `test_support.rs` — shared test fixtures: the two-frame click harness (`press_at`/`release`/`type_key`/`focus_field`), the Linux Libertine font fixture
- `font/` — `mod.rs` (FontManager facade: loading/storage), `glyph_cache.rs` (GlyphCache; bitmaps shared via `Arc<[u8]>`), `layout.rs` (text layout/measurement)
- `draw/` — Draw commands (`command.rs`) + `DrawList` with **UiLayer z-bands** (`mod.rs`: Content/PanelChrome/Floating/Modal/Tooltip/DragGhost, 900+index*15 depth bands; elevated layers flush after Content at `end_frame`, so a Floating popup physically escapes a panel clip pair; pre-flush `commands()` = Content only — lifecycle doc on the accessors; `begin_overlay` = Floating sugar, `begin_overlay_in` picks the band)
- `interaction/` — Widget state, mouse hit detection, focus, per-widget persistent state (`edit: TextEditState`); `wants_mouse()` = a widget owns the current press→release gesture (tests in `interaction/tests.rs`)
- `input_state.rs` — per-frame `InputState` snapshot (typed characters collected directly from `keyboard.just_pressed_keys()`) + `KeyRepeat` (dt-driven hold repeat)
- `text_edit.rs` — pure `TextEditState` (buffer/cursor/selection editing model)
- `style.rs` — Theme definitions (`TextInputStyle: Copy`, `Color` re-exported from `common`), private palette consts

## Known Tech Debt
- Tracked on the Studio Board: issue #88 — all Lows: US-layout-only key mapping (JUN-T1b; general text input itself shipped Jul 2026), TextDrawData redundancy (ARCH-003), no layout helpers (JUN-T3), GlyphInfo cache-key duplication (GPP-L8)

## Testing
- `cargo test -p ui` — 0 failed, 0 ignored. `keycode_to_char` covers digits, A–Z (shift = uppercase), space, `.` `-` and Shift+Minus = `_`. `TextDrawData.font_id` marks which font glyphs came from — downstream glyph caches key on it

## Godot Oracle
- Immediate-mode patterns: Godot doesn't use immediate-mode, but see `scene/gui/control.cpp` for widget lifecycle
- Font rendering: `modules/text_server_advanced/text_server_adv.cpp`
