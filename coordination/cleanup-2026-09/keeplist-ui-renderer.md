Keep-list built. I re-read nothing — all 24 files were already in context.

---

# HALF A — CONTRACT tests

## crates/ui

| | contract | keep |
|---|---|---|
| C1 | widget gesture state machine: Normal → Hovered → Active → clicked-on-release | `crates/ui/tests/ui_interaction_debug.rs:185 test_interaction_manager_click_logic` — **move inline to `src/interaction/tests.rs`**. The only place `result.clicked == true` is ever asserted (`grep "\.clicked"` in `src/` finds one hit, the negative at `interaction/tests.rs:61`). |
| C2 | press inside, release outside, no click | `crates/ui/tests/ui_interaction_debug.rs:301 test_click_press_inside_release_outside` — move inline |
| C3 | `UIContext::button` fires on release-over only (widget layer, not manager layer) | `crates/ui/tests/ui_interaction_debug.rs:12 test_ui_button_click_detection` — move inline |
| C4 | slider maps pointer x to value and holds it after release | `crates/ui/tests/ui_interaction_debug.rs:74 test_ui_slider_interaction` — move inline. **The only slider test in the crate.** |
| C5 | `InputState` mouse snapshot mapping (pos, down, just_pressed/released, scroll) | `crates/ui/tests/ui_interaction_debug.rs:140 test_input_state_from_input_handler` — move inline to `src/input_state.rs`, drop the `end_frame` third (that's the `input` crate's contract) |
| C6 | `wants_mouse` owns a press→release gesture, and a press that hits nothing does not | `crates/ui/src/interaction/tests.rs:125 test_wants_mouse_holds_from_widget_press_through_release_frame` |
| C7 | a missed release event (focus loss mid-press) must not block picking forever | `crates/ui/src/interaction/tests.rs:172 test_missed_release_event_frees_the_mouse_gesture` |
| C8 | `is_input_blocked_at`: a blocking rect makes widgets under it inert; overlay scope escapes it | `crates/ui/src/interaction/tests.rs:49 test_blocking_rect_makes_outside_widget_inert` |
| C9 | widget-state lifecycle: unseen state is collected, focused/blocked state is retained | `crates/ui/src/interaction/tests.rs:228 test_focused_widget_state_survives_unseen_frame` |
| C10 | `TextEditState`: typing replaces the active selection | `crates/ui/src/text_edit.rs:218 test_typing_replaces_selection` |
| C11 | `TextEditState`: backspace/delete remove the selection, else the adjacent char | `crates/ui/src/text_edit.rs:249 test_backspace_deletes_selection` |
| C12 | `TextEditState`: plain arrow collapses to the edge, shift-arrow extends and drops on anchor return | `crates/ui/src/text_edit.rs:298 test_shift_arrow_extends_selection` |
| C13 | `cursor_from_click` picks the nearest char boundary from prefix widths | `crates/ui/src/text_edit.rs:337 test_cursor_from_click_picks_nearest_boundary` |
| C14 | key repeat: fires on press, silent until `REPEAT_DELAY`, then ~`1/REPEAT_INTERVAL`, resets on release | `crates/ui/src/input_state.rs:344 test_repeat_fires_after_delay_then_at_interval` |
| C15 | `keycode_to_char`: letters shift-case, numpad ignores shift, top-row digits blocked by shift, `Shift+Minus = '_'` | `crates/ui/src/input_state.rs:320 test_keycode_to_char_letters_and_space` |
| C16 | drag-scrub: 4px arm threshold, output is `start + dx` (never compounding), release commits, Escape restores, Ctrl snaps to whole steps | `crates/ui/src/context/scrub_tests.rs:66 test_float_scrub_emits_per_frame_values_and_commits_on_release` |
| C17 | arrow nudge steps by `step`, Shift by 10× | `crates/ui/src/context/scrub_tests.rs:134 test_float_arrow_up_down_steps_value_shift_is_10x` |
| C18 | commit semantics: soft range accepts + flags `out_of_range`, hard range clamps quietly, unparsable reverts, pre-existing out-of-range stays quiet | `crates/ui/src/context/scrub_tests.rs:286 test_float_typed_commit_beyond_soft_range_flags_out_of_range` |
| C19 | `text_input` lifecycle: click focuses + selects all, typing overwrites, Enter commits, Escape cancels, click-away commits | `crates/ui/src/context/tests.rs:499 test_text_input_focus_selects_all_and_typing_overwrites` |
| C20 | programmatic focus (the F2-rename path) arms an edit with no click and releases on commit/Escape | `crates/ui/src/context/focus_tests.rs:19 test_focus_text_input_arms_edit_without_a_click` |
| C21 | `UiLayer` z-bands: submission order scrambled, flushed order is enum order, each command inside its band | `crates/ui/src/draw/tests.rs:192 test_layers_flush_in_enum_order` |
| C22 | layer-stack lifecycle: nesting pops to the outer layer, extra pop is a no-op, flush is idempotent, `clear` resets the stack | `crates/ui/src/draw/tests.rs:274 test_flush_is_idempotent_and_clear_resets_stack` |
| C23 | clip push/pop emit a matched `PushClipRect`/`PopClipRect` pair carrying the bounds | `crates/ui/src/draw/tests.rs:168 test_draw_list_clip_rect` |
| C24 | glyph cache: bounded — cleared at the limit, untouched below it; key quantizes size to tenths | `crates/ui/src/font/glyph_cache.rs:178 test_glyph_cache_evicts_when_full` |
| C25 | typed error path: an unresolvable font handle falls back to a placeholder instead of panicking or drawing nothing (#54) | `crates/ui/src/context/tests.rs:212 test_float_input_with_unresolvable_font_falls_back_to_placeholder` |
| — | **text layout and measurement** — `layout_text` baseline/ascent/descent math, the `offset_y` sign flip, space and zero-width-glyph handling, `height.max(new_line_size)`, `measure_text` advance summing | **MISSING.** `font/layout.rs`'s one test is a struct literal asserting its own values. DejaVu bytes already ship via `include_bytes!` in the editor crate — a fixture font makes all of it headless. |
| — | slider edge clamping (0.0/1.0, click on either end, drag outside the track while held) | **MISSING** (C4 covers the middle of the range only) |
| — | `KeyRepeat` per-key independence — holding ArrowLeft must not advance the Backspace slot | **MISSING.** `timers[key as usize]` over a hand-numbered `RepeatKey` enum is exactly where an off-by-one mis-repeats silently. |
| — | widget-level cursor placement: a click at a known x inside a focused field places the cursor there (prefix-width plumbing between widget and model) | **MISSING** (C13 covers the model only) |

## crates/renderer

| | contract | keep |
|---|---|---|
| R1 | **GPU layout sizes** — `SpriteVertex` 36B / 3 attrs, `SpriteInstance` 76B / 8 attrs, `CameraUniform` 80B, strides match `desc().array_stride` | `crates/renderer/src/sprite_data.rs:304 test_sprite_vertex_bytemuck_cast` (as the merge target) |
| R2 | **GPU layout sizes** — `BloomParams` and `BlurParams` are 16 bytes, as the WGSL uniforms assume | `crates/renderer/src/bloom.rs:558 bloom_params_struct_is_16_bytes` |
| R3 | **GPU layout sizes** — `LineVertex` 28B, 3 attrs, stride 28 | `crates/renderer/src/line_pipeline.rs:269 line_vertex_layout_size` |
| R4 | a default instance is a plain unlit textured quad (`shape` zeroed == legacy behavior, `emissive` 0) | `crates/renderer/src/sprite_data.rs:379 test_sprite_instance_default_shape_is_plain_quad` |
| R5 | `Sprite::to_instance` maps every authored field onto the GPU instance | `crates/renderer/src/sprite.rs:179 test_sprite_to_instance` |
| R6 | depth sort is deterministic and NaN-safe (`total_cmp`, never `partial_cmp().unwrap()`); the `sorted` flag guards re-sorting and resets on add | `crates/renderer/src/sprite/batch.rs:223 test_sprite_batch_sort_handles_nan_depth_without_panicking` |
| R7 | batching groups by texture — one batch per texture, correct counts | `crates/renderer/src/sprite/batch.rs:322 test_sprite_batcher_groups_by_texture` |
| R8 | the same texture under two clip states splits into two batches, each carrying its clip; no clip = the old by-texture behavior | `crates/renderer/src/sprite/batch.rs:341 test_sprite_batcher_splits_same_texture_by_clip` |
| R9 | `clear` resets the clip cursor (an unbalanced push cannot leak into the next frame) and keeps batches allocated | `crates/renderer/src/sprite/batch.rs:378 test_sprite_batcher_clear_resets_clip_cursor` |
| R10 | instance-cache invalidation: identical restage skips the upload, a moved instance forces one, empty↔content transitions count as change | `crates/renderer/src/sprite/instance_cache.rs:95 test_identical_batches_skip_upload` |
| R11 | instance-cache invalidation, the subtle half: **identical bytes with different batch boundaries must still re-upload** (draw ranges changed) | `crates/renderer/src/sprite/instance_cache.rs:121 test_layout_change_triggers_upload_even_with_same_bytes` |
| R12 | scissor quantization: rounds outward to cover partial pixels, negative origin clamps to 0 keeping the far edge, non-finite yields empty | `crates/renderer/src/scissor.rs:77 test_quantize_rounds_outward_to_cover_partial_pixels` |
| R13 | scissor clamping to the live surface — the resize-race guard against wgpu's `scissor ⊆ attachment` validation | `crates/renderer/src/scissor.rs:117 test_clamp_trims_overhang_on_resize_race` |
| R14 | `batch_scissor` decision table: no clip + no default = full surface, default applies to unclipped batches, clip ∩ default, empty result = skip the draw | `crates/renderer/src/scissor.rs:175 test_batch_scissor_empty_result_skips_draw` |
| R15 | device-loss latch is **one-way** (mark is idempotent, never resets) and clones share the flag | `crates/renderer/src/device_status.rs:78 latch_reports_lost_after_mark` |
| R16 | `resize_action` truth table: same size skipped, zero dimension always skipped even when forced, changed size returns, force reconfigures at the same size (hidden-canvas round trip) | `crates/renderer/src/device_status.rs:115 resize_action_returns_new_size_when_changed` |
| R17 | `TextureFilter` → `SamplerConfig`: mag/min/mipmap all agree per variant, every other field stays at default | `crates/renderer/src/texture_filter.rs:72 test_nearest_filter_maps_every_sampler_filter_to_nearest` |
| R18 | typed error paths — `TextureError` variants carry their operands in the message | `crates/renderer/src/texture.rs:497 test_texture_error_display` |
| R19 | `TextureHandle::WHITE` is reserved: `default() == WHITE` and the manager allocates from 1, so no loaded texture can collide with the built-in white | `crates/renderer/src/texture.rs:407 test_texture_handle_default` |
| — | camera view/projection/screen↔world math | **not renderer's contract.** `Camera`/`CameraUniform` live in `crates/common/src/camera.rs`, which already tests `screen_to_world`, the round trip, the y-flip and bounds. The 8 camera tests in `sprite_data.rs:397–537` are cross-crate duplicates or reimplement the production body. The one real hole — `projection_matrix` NDC mapping (`sprite_data.rs:462`) — should be **moved to common**, not kept here. |

---

# HALF B — GUARD tests (footguns)

| footgun | guard |
|---|---|
| UI text y = BASELINE in `label_styled`; text in a box must use `label_in_bounds_styled`, which centers via font metrics, or glyphs straddle the border | ✅ `crates/ui/src/context/tests.rs:275 test_label_in_bounds_styled_keeps_glyphs_inside_bounds` — asserts `position.y - font_size*0.8 >= bounds.y` and the baseline stays inside. Exactly the guard. |
| An elevated `UiLayer` (Floating/Modal) must **physically escape** a Content-layer clip pair | ✅ `crates/ui/src/draw/tests.rs:233 test_elevated_layer_escapes_content_clip_pair` — asserts the popup Rect flushes at an index *after* `PopClipRect`. The add-component-popup bug in miniature. |
| The release frame of a click is NOT `WidgetState::Active`, so consumers gate on `wants_mouse`, not the state | ✅ `crates/ui/src/interaction/tests.rs:125 test_wants_mouse_holds_from_widget_press_through_release_frame` — asserts `wants_mouse()` is still true on the release frame, and false only after the next `begin_frame`. |
| Float sorts use `total_cmp`, never `partial_cmp().unwrap()` | ✅ `crates/renderer/src/sprite/batch.rs:223` (R6) |
| **Cross-batch submission order must be deterministic** | ❌ **MISSING — and currently violated by construction.** `SpriteBatcher.batches` is a `HashMap<(TextureHandle, Option<[u32;4]>), SpriteBatch>` (`batch.rs:85`); iteration order is randomized per process. `sort_all_batches` orders *within* a batch, nothing orders *between* them. Any alpha-blended overlap across two textures can flip between runs. Spec: expose an ordered accessor (sort keys, or a `Vec` with a stable index) and assert two batchers built in different insertion orders emit identical draw sequences. |
| `queue.write_buffer` flushes at `submit()`, not encode time — rewriting one uniform between passes in a single submit means every pass reads the LAST write (this broke bloom); one buffer per distinct per-frame value | ❌ **MISSING.** Spec: assert `BloomPipeline` owns N distinct `Buffer`s for N blur iterations / distinct param sets — a count over the pipeline's buffer collection, not a GPU test. Today nothing prevents the regression from being reintroduced. |
| Bind groups are cached, never created per frame | ❌ **MISSING.** `sprite/pipeline.rs:288` has `cache_texture_bind_group`, but nothing asserts a second draw with the same texture is a cache hit. Spec: a hit/miss counter on the cache, asserted across two identical frames. |
| `DynamicBuffer` grows to the next power of two and never shrinks | ❌ **MISSING.** `sprite_data.rs:251–265` is device-bound. Spec: extract `fn grown_capacity(current: usize, needed: usize) -> Option<usize>` (`None` = no growth) and assert 3→4, 5→8, 100→128, and that a shrink from 128 to 2 returns `None`. |
| **`offset_of!`-style GPU vertex layout check** | ❌ **MISSING — the single highest-value guard in the workspace.** Spec: for `SpriteVertex::desc()` and `SpriteInstance::desc()`, assert every attribute's `(shader_location, offset, format)` triple against the WGSL bindings — locations 0–2 for the vertex, 3–10 for the instance. The eleven offsets are hand-written as `size_of::<[f32; N]>()` (`sprite_data.rs:47,53,141,147,153,159,165,171,177`); swap the depth `[f32;13]` and emissive `[f32;14]` offsets and it compiles, every existing test passes (count and stride are unchanged), and sprites render at wrong depths with wrong glow. Count + stride protect half the invariant; the offsets are unprotected. |

---

# MERGE-INTO

**ui** — each keep absorbs these before the rest is deleted:

- C6 ← `interaction/tests.rs:163` (press missing all widgets)
- C8 ← `:68` (overlay scope), `:84` (outside the rect), `:113` (begin_frame clears blocking state)
- C9 ← `:213` (unseen GC), `:97` (blocked widget keeps its buffer), `:188` + `:246` (has_focus / is_focused set-and-clear), `:200`
- C10 ← `text_edit.rs:202`, `:210` (set_text_select_all, empty case), `:228` (insert at cursor)
- C11 ← `text_edit.rs:236` (backspace mid/start), `:261` (forward delete, at end)
- C12 ← `text_edit.rs:273` (arrow clamp), `:284` (collapse to edge), `:312` (home/end ±shift), `:327` (select-all then home), `:352` (empty-string safety)
- C14 ← `input_state.rs:337` (initial press), `:379` (reset on release), `:394` (repeat reaching `InputState.up_pressed` — the wiring half)
- C15 ← `input_state.rs:297`, `:305`, `:313`, `:330` → one `(KeyCode, shift) -> Option<char>` table
- C16 ← `scrub_tests.rs:50` (sub-threshold click still focuses), `:88` (clamps to soft max), `:114` (press re-seeds state), `:98` (Escape restores), `:372` + `:392` (Ctrl / Ctrl+Shift snap)
- C18 ← `scrub_tests.rs:267` (soft not clamped), `:311` (unchanged/unparsable stay quiet), `:339` + `:356` (hard clamp), `:163` (invalid buffer flags + reverts), `:191` (suffix never enters the buffer)
- C19 ← `context/tests.rs:514` (shift uppercase/underscore), `:538` (Escape), `:552` (click-away commits), `:572` (space), `:388` (float equivalent), `:298` (`wants_keyboard` follows focus), `:404` + `:420` (cursor-position editing through the widget)
- C20 ← `focus_tests.rs:48` (Escape drops programmatic focus)
- C21 ← `draw/tests.rs:212` (bands), `:121` (monotonic depth within a band), `:134` (overlay above base band)
- C22 ← `draw/tests.rs:259` (nesting), `:155` (clear resets overlay mode), `overlay_tests.rs:7` (`begin_overlay` = Floating + blocking back-compat)
- C24 ← `glyph_cache.rs:196` (below the limit nothing evicts), `:166` (key)
- C3 ← `ui_interaction_debug.rs:278` (click outside)

**renderer**

- R1 ← `sprite_data.rs:312` (vertex desc), `:356` + `:371` (instance size + desc), `:540` (CameraUniform 80B) → one `test_gpu_structs_match_shader_layout` in `sprite_data.rs`
- R2 ← `bloom.rs:564` (BlurParams)
- R4 ← `sprite_data.rs:322` (the `emissive == 0.0` default, which is that test's only non-echo assertion)
- R6 ← `batch.rs:206` (ascending sort), `:239` (idempotent), `:281` (flag reset on add), `:266` (clear marks unsorted)
- R9 ← `batch.rs:428` (batches stay allocated after clear), `:366` (no-clip path unchanged)
- R10 ← `instance_cache.rs:109` (moved instance), `:138` (empty↔content)
- R12 ← `scissor.rs:83`, `:88`, `:94`, `:99`, `:106` → one quantize table
- R13 ← `scissor.rs:112`, `:124`, `:130` → one clamp table
- R14 ← `scissor.rs:135`, `:140`, `:147` (intersect), `:152`, `:157`, `:167`, `:186` → one `batch_scissor` table
- R15 ← `device_status.rs:73` (starts clear), `:93` (idempotent), `:85` (clones share)
- R16 ← `device_status.rs:101`, `:106`, `:120` → one `resize_action` table
- R17 ← `texture_filter.rs:64` (Linear), `:80` (other fields default)

Everything not named as a keep or a merge-into is deleted, including all 8 camera tests in `sprite_data.rs` (move the `projection_matrix` NDC check to `common/src/camera.rs`), both `render_targets.rs` tests (they compute `(w / BLOOM_DOWNSAMPLE).max(1)` in the test body and never call `bloom_width()`), `style.rs:288`, `font/mod.rs:209`, `font/layout.rs:133`, and `ui_interaction_debug.rs:232`.

---

# WEAK KEEPS

| keep | asserts now | should assert |
|---|---|---|
| R1 `sprite_data.rs:304` | `size_of == 36/76`, `attributes.len()`, `array_stride` | keep those, **plus** every attribute's `(shader_location, offset, format)` — see the MISSING guard. The merged test is the natural home; without the offsets it protects half the invariant. |
| R4 `sprite_data.rs:379` | `shape == [0.0; 4]`, then echoes `with_shape` | also `emissive == 0.0`, so "default instance = plain unlit textured quad" is one complete statement. Drop the `with_shape` echo. |
| R5 `sprite.rs:179` | position, rotation, scale, color, depth | `to_instance` (sprite.rs:159–170) also forwards `tex_region` and `emissive` and applies `.with_shape(self.shape)`. Set all three to non-defaults and assert they land — `SpriteAnimationSystem` writes `tex_region` every frame and nothing currently tests it reaches the GPU. |
| R6 `batch.rs:223` (merged) | NaN sorts last, real values stay ordered, `sorted` true | add the guard proof: mutate `instances` out of order directly, call `sort_by_depth()`, assert the order was **not** touched. `:239` as written passes even if the `if !self.sorted` guard is deleted. |
| R19 `texture.rs:407` | `default().id == 0` | `TextureHandle::default() == TextureHandle::WHITE`, and that a manager's first allocated handle is `WHITE.id + 1` (texture.rs:137). The reservation is currently a comment. |
| C24 `glyph_cache.rs:178` (merged) | at-limit clears, below-limit keeps; `:166` asserts derived `PartialEq` on obviously-different keys | the non-obvious code is `size_tenths: (font_size * 10.0) as u32`. Assert 16.0 and 16.04 produce the **same** key (deliberate quantization) while 16.0 and 16.1 differ. |
| C22 `draw/tests.rs:274` | `assert_eq!(commands().len(), after_first)` — a length assert | **legitimate here**: length *is* the idempotence contract. Keep as-is; noted so it isn't mistaken for the bloat pattern. It is the only length assert surviving onto the ui keep-list. |
| C25 `context/tests.rs:212` | `.any(|c| matches!(c, TextPlaceholder{..}))` | assert the placeholder's `text == "42.00"` — otherwise an empty-string placeholder passes and the field still renders blank. |
| C5 `ui_interaction_debug.rs:140` | mouse mapping, then `end_frame` clearing edges | keep the mapping asserts, drop the final third (`input` crate's contract), and move to `src/input_state.rs`. |
| C19 `context/tests.rs:499` (merged) | the committed `Option<String>` | also assert the drawn buffer mid-edit, so a field that commits correctly but renders the pre-edit text still fails. |

---

# Counts

```
ui       — current 123, keep 25   (+ 4 MISSING contracts, 0 MISSING guards)
renderer — current  92, keep 19   (+ 4 MISSING guards)
scope    — current 215, keep 44   (+ 8 specs to write)
```

ui keeps: C1–C25. renderer keeps: R1–R19. Deletion is everything else — 171 of 215 tests, with the merge-into list naming which assertions must be absorbed first so nothing on the keep-list ships weaker than what it replaces.

Two things worth escalating beyond the list: the **nondeterministic cross-batch order** (`HashMap` iteration in `SpriteBatcher`) is not a missing test, it is a live bug the missing test would have caught; and the **attribute offset guard** is the one item I would write before deleting anything, since several of the tests being deleted currently give false comfort that the GPU layout is covered.