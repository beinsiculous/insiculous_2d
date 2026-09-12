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
- `context/` — `UIContext` frame lifecycle, widgets (button, slider, checkbox), edit fields (drag-scrub with 4px click threshold, Up/Down nudge, soft vs hard ranges, Tab/Shift-Tab traversal between the fields registered this frame — `InteractionManager` keeps the frame's registration order and a commit schedules the neighbour the next frame claims), the hover tooltip (`tooltip.rs` — `UIContext::tooltip(anchor, text)` takes a widget's rect and one sentence; the panel is drawn from `end_frame` on `UiLayer::Tooltip` once the pointer has rested for `TOOLTIP_DELAY`; the frame's anchor is settled at `end_frame` against the frame's complete blocking regions — a widget the pointer is not on claims nothing, of live nested widgets the smallest wins, and a button down, held or clicked inside one frame, keeps the panel down), and the pointer shape (`cursor.rs` — `CursorIcon` with `request_cursor`/`requested_cursor`, reset each `begin_frame`; the host maps it to the platform's).
- `font/` — `FontManager` loading and measurement; `GlyphCache` stores rasterized bitmaps as `Arc<[u8]>`.
- `draw/` — `DrawCommand` and `DrawList` with `UiLayer` z-bands; elevated layers flush after Content at `end_frame` so popups escape panel clipping.
- `interaction/` — widget state and persistent focus; `wants_mouse()` indicates a widget owns the current press→release gesture. Blocking regions carry the `UiLayer` that claimed them: a widget in an overlay scope is inert only under a region from a higher layer, so a modal's scrim reaches a strip or a dropdown opened after it while a dropdown over the strip stays live. Every gesture consults `is_blocked_for_scope`, the tooltip's eligibility and the float scrub's arming press included — a gesture that reads raw mouse state must run that test itself on the press, and only on the press: an armed gesture is the field's until release or Escape, whatever opens over it meanwhile (a modal from a keyboard chord mid-drag included — undo covers that; cutting the gesture short was the worse trade). A widget's state is pruned the frame it goes unseen, so a gesture cannot outlive its widget.
- `input_state.rs` — per-frame `InputState` snapshot and `KeyRepeat` (dt-driven hold repeat).

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| UI text y = baseline in `label_styled`; text inside a box must use `label_in_bounds_styled` to center via font metrics | `src/context/tests.rs test_label_in_bounds_styled_keeps_glyphs_inside_bounds_at_every_alignment` |
| An elevated `UiLayer` (Floating/Modal) must physically escape a Content-layer clip pair | `src/draw/tests.rs test_elevated_layer_escapes_content_clip_pair` |
| The release frame of a click is NOT `WidgetState::Active`, so consumers must gate on `wants_mouse` rather than widget state | `src/interaction/tests.rs test_wants_mouse_holds_from_widget_press_through_release_frame` |
| An overlay scope opened after a modal, on a lower band, must not escape the modal's scrim (the toolbar strip's Play behind the Stop dialog) | `src/interaction/tests.rs test_a_higher_layers_blocking_region_reaches_into_a_lower_overlay_scope` |
| A gesture that reads raw mouse state rather than the interaction result — the float scrub — must run the blocking test on its arming press, or a numeric row under a modal popup scrubs when the popup's own field is dragged; and only on the press, or the toolbar strip's chrome rect cuts a live inspector scrub short | `src/context/scrub_tests.rs test_a_float_field_under_a_modal_popup_neither_arms_nor_scrubs`, `test_an_armed_scrub_survives_the_pointer_crossing_a_chrome_scope` |
| A Tab or Shift-Tab commit must hand focus to the neighbouring field on a LATER frame than the one that commits, never on the committing frame: a target drawn after the committing field matched the pending id there and then, entered edit mode, and its own commit branch read the same `tab_pressed` snapshot and re-committed — one physical key press cascaded through every field after the first, each committing its seeded value as a real edit. The `!tab_pressed` guard on the match is the whole mechanism, and it covers forward, backward and wrap uniformly, because `InputState` is one snapshot per frame; reasoning about where the target id came from instead of gating on the snapshot passed review twice and was false for the ordinary top-to-bottom case | `src/context/traversal_tests.rs test_one_tab_press_commits_only_the_focused_field_and_moves_focus_one_step_forward`, `test_shift_tab_from_the_first_field_wraps_to_the_last` |
| Every widget on screen offers its tooltip each frame; only the one the pointer rests on may claim it, or the last widget drawn wipes the anchor the hovered one claimed — and where two nest (a header and its chevron), the frame's anchor is settled once at `end_frame` or the two flip it every frame and the delay never elapses — and settled against the complete regions, or the smaller anchor an overlay opened over wins and nothing shows; a click that begins and ends inside one frame sets the press edge without the held flag | `src/context/tooltip_tests.rs test_only_the_anchor_under_the_pointer_claims_the_frame`, `test_the_innermost_of_two_nested_anchors_is_the_one_the_pointer_rests_on`, `test_an_overlay_opened_after_a_smaller_anchor_it_covers_shows_its_own_tooltip`, `test_a_click_that_begins_and_ends_inside_one_frame_takes_the_tooltip_down` |


## Known Tech Debt
- Tracked on the Studio Board: issue #88 — all Lows: US-layout-only key mapping (JUN-T1b; general text input itself shipped Jul 2026), TextDrawData redundancy (ARCH-003), no layout helpers (JUN-T3), GlyphInfo cache-key duplication (GPP-L8)

## Testing
- `cargo test -p ui` — 0 failed, 0 ignored. `keycode_to_char` covers digits, A–Z (shift = uppercase), space, `.` `-` and Shift+Minus = `_`. `TextDrawData.font_id` marks which font glyphs came from — downstream glyph caches key on it

## Godot Oracle
- Immediate-mode patterns: Godot doesn't use immediate-mode, but see `scene/gui/control.cpp` for widget lifecycle
- Font rendering: `modules/text_server_advanced/text_server_adv.cpp`
