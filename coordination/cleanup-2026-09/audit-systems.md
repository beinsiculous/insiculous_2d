# Code Quality Audit — renderer / ui / input / physics / audio + workspace layout

**Scope:** `crates/renderer/src`, `crates/ui/src`, `crates/input/src`, `crates/physics/src`,
`crates/audio/src`, plus workspace-level layout (Cargo.toml files, `scripts/`, `coordination/`,
`docs/`, folder naming).
**Repo:** `/home/jedi/projects/insiculous/insiculous_2d`
**Commit:** `d58f3c3` (branch `dev`, clean tree)
**Mode:** READ-ONLY. Nothing was edited. No tests were run (counts below are `#[test]` attribute
counts, not `cargo test` output).

---

## 1. DRY violations

### 1.1 Three pipelines, three copies of the camera binding
`crates/renderer/src/sprite/pipeline.rs:103-118` and `crates/renderer/src/line_pipeline.rs:87-125`
build byte-identical camera bind-group layouts, uniform buffers and bind groups.
`pipeline.rs:256-259` and `line_pipeline.rs:184-187` are the same `update_camera` body
(`CameraUniform::from_camera` + `queue.write_buffer`).

Already tracked as Studio Board issue #89 (DRY-006) and called out in
`line_pipeline.rs:8-10`, but still open.

**Fix:** one `CameraBinding { buffer, layout, bind_group }` with `new(device)` and
`update(queue, camera)`, composed by both pipelines.

### 1.2 Pipeline-descriptor boilerplate is copied three times
`crates/renderer/src/bloom.rs:494-542` already has `build_fullscreen_pipeline`, but
`crates/renderer/src/sprite/pipeline.rs:188-238` and
`crates/renderer/src/line_pipeline.rs:129-172` each hand-roll a full
`RenderPipelineDescriptor` repeating identical `multisample`, `cache: None`,
`multiview_mask: None`, `front_face: Ccw` and `cull_mode: None` blocks.

**Fix:** widen the helper to take vertex buffer layouts, topology, blend and depth state, and
route all three pipelines through it.

### 1.3 TextureManager repeats its own load prologue three times
`crates/renderer/src/texture.rs:155-268` — `load_texture`, `load_texture_from_bytes` and
`load_texture_from_rgba` each repeat: max-dimension check → allocate handle from
`next_handle` → `create_texture_from_rgba` → `textures.insert`.

**Fix:** one private `insert_rgba(width, height, data, config) -> Result<TextureHandle, _>`;
the three public entry points differ only in how they get to RGBA bytes.

### 1.4 `float_input` and `text_input` duplicate the whole editing shell — and have already drifted
`crates/ui/src/context/text_input.rs:147-240` (`float_input`) vs `:332-392` (`text_input`)
duplicate: click-to-focus with select-all, click-to-place-cursor via `prefix_widths`,
Escape-cancel, Enter/Tab/click-away commit, `apply_edit_keys`, and the unfocused draw path.

They have **already diverged**: `float_input` resolves its face through `field_font`
(`:453-457`, which falls back when a handle goes stale), while `text_input:347` calls
`self.font_manager.default_font()` directly. A stale font handle behaves differently in the
two widgets.

**Fix:** extract a shared `edit_field` core (focus/keys/draw/commit) that both wrap with their
own parse-and-format policy.

### 1.5 Two independent action-binding stores
`crates/input/src/player.rs:101-129` (`PlayerBindings::bind`/`unbind`/`bindings`) and
`crates/input/src/input_mapping.rs:127-163` (`InputMapping::bind`/`unbind`/`bindings`) are the
same `HashMap<Action, Vec<Source>>` with the same "no duplicate pair", "retain then remove if
empty" semantics. `PlayerBindings` adds a dirty flag and device-relative resolution; that is
the only difference.

**Fix:** `PlayerBindings` holds an `InputMapping<GameAction>` over `PlayerSource` plus the
dirty flag.

### 1.6 The standard pad layout is written twice
`crates/input/src/input_mapping.rs:249-291` (`with_default_bindings`) and
`crates/input/src/player.rs:396-423` (`bind_standard_pad_layout`) both spell out dpad + left
stick → movement, A/B/X/Y → Action1-4, Start → Menu, Select → Select. Adding a pad button
means editing both, with nothing holding them together.

**Fix:** one `const STANDARD_PAD_LAYOUT: &[(GameAction, PadSource)]` both consume.

### 1.7 The list of character-producing keys exists twice
`crates/ui/src/input_state.rs:164-181` enumerates `typed_keys` for the scan loop;
`keycode_to_char` at `:88-124` enumerates the same set again in a match. They agree today and
nothing enforces it — adding a key to one silently produces a dead key or a missing character.

**Fix:** iterate one `TYPED_KEYS` const, or drive the scan from the match's own key set.

### 1.8 Collision-event construction duplicated in stepping
`crates/physics/src/physics_world/stepping.rs:89-134` — the contact-pair loop and the
intersection-pair loop push near-identical `CollisionData { event: CollisionEvent { .. },
contacts }` blocks, differing only in the contacts vector.

**Fix:** `fn push_collision(&mut self, entity_a, entity_b, started, contacts)`.

### 1.9 Audio sink startup duplicated across the two seams
`crates/audio/src/manager/mod.rs:270-288` (SFX) and `crates/audio/src/manager/music.rs:80-89`
(music) duplicate: `Sink::try_new` + `StreamError` mapping, `base * bus * master` volume
derivation, and the looping/one-shot `append` branch.

**Fix:** a private `start_sink(&output, source, base_volume, bus_volume, looping) -> AudioResult<Sink>`.

### 1.10 Every PhysicsConfig preset repeats a redundant scale
`crates/physics/src/presets.rs:90-122` — all five presets end in `.with_scale(100.0)`, which is
already `DEFAULT_PIXELS_PER_METER` (`physics_world/mod.rs:32`) and already what `Default`
produces (`:55`).

**Fix:** delete the five calls.

### 1.11 Two text-height formulas that can disagree
`crates/ui/src/font/layout.rs:102` computes layout height as
`(ascent + max_descent.max(-descent)).max(new_line_size)`, while `measure_text` at `:118`
returns plain `new_line_size`. `UIContext::text_pos_in_bounds` (`context/text.rs:81`) centers
using the **measure** height, then draws using the **layout** path.

**Fix:** one height function used by both.

---

## 2. SRP violations

- **`Renderer` is a god object.** `crates/renderer/src/renderer.rs:48-89` owns the instance,
  surface, adapter, device, queue, surface config, clear color, white texture, render targets,
  bloom pipeline, bloom config, line pipeline, per-frame line vertex count, per-frame viewport
  scissor, device-loss latch and a pending-reconfigure flag. The module doc (`:3-15`) defends
  "init + render" as one concern, but the frame orchestration in
  `render_with_sprites_internal` (`:379-462`) and the per-frame line/scissor state are a third
  and fourth job. **Fix:** split a `FrameGraph` owning the pass sequence and per-frame state.

- **`SpritePipeline::new_with_target` is 183 lines.**
  `crates/renderer/src/sprite/pipeline.rs:70-253` builds two bind-group layouts, the pipeline
  layout, quad vertices, index buffer, instance buffer, camera buffer, camera bind group,
  shader module and the render pipeline. Well over the ~60-line flag.
  **Fix:** four private builders; the ten section comments already mark the seams.

- **`BloomPipeline::run` mixes four responsibilities.** `crates/renderer/src/bloom.rs:218-284`
  writes uniforms, rebuilds bind groups on resize, runs extract, loops blur, composites.
  **Fix:** `ensure_ready(device, queue, targets, config)` + a `run` that is just the sequence.

- **`float_input` is 94 lines with five jobs.**
  `crates/ui/src/context/text_input.rs:147-240` arbitrates scrub vs click, manages focus
  transitions, handles keys, nudges the value, draws, and constructs the result.

- **`sync_entity_to_physics` has a hidden second mode.**
  `crates/physics/src/physics_system/sync.rs:52-109` adds bodies, detects external edits,
  maintains baselines and delegates colliders — and its `else if` branch (`:97-108`) is a
  standalone-collider path where `Transform2D` edits are silently ignored (documented at `:98`,
  but it is a real behavioral fork inside one function).

- **`PhysicsWorld` holds eleven rapier objects plus four entity maps plus the event buffer plus
  the previous-collision set.** `crates/physics/src/physics_world/mod.rs:106-143`. This is the
  known SRP-001 on issue #85 and is the largest god object in the cluster.

- **`crates/physics/src/components.rs` is 599 lines** against the project's 600-line ceiling.
  It breaks on the next field added.

---

## 3. KISS violations

- **Four accessors for two fields.** `crates/renderer/src/renderer.rs:473-499` —
  `device()`/`device_ref()` and `queue()`/`queue_ref()`. **Fix:** keep the borrowing pair;
  callers that need ownership clone the Arc.

- **One-caller indirection.** `renderer.rs:336` (`render_with_sprites`) calls
  `render_with_sprites_internal` (`:379`) and nothing else does. **Fix:** inline.

- **Delegating wrapper with one caller.** `crates/renderer/src/texture.rs:389-391` — private
  `create_sampler` exists only to forward to `SamplerConfig::create_sampler`. Called once, at
  `:377`.

- **Dead public API in the renderer.** `crates/renderer/src/sprite/pipeline.rs:265`
  (`update_instance_buffer`) has zero callers workspace-wide since `prepare_sprites` landed.
  Same for `:315` `invalidate_texture_cache`, `:320` `clear_texture_cache`, `:524` `pipeline()`.

- **Nested-Option state encoding.** `crates/renderer/src/bloom.rs:290-296` —
  `run_composite_pass` takes `scissor: Option<Option<(u32,u32,u32,u32)>>` to encode three
  states (fullscreen / scissored / empty). The doc comment at `:286-289` exists only to explain
  the encoding. **Fix:** a three-variant enum, or reuse `scissor::batch_scissor`'s convention.

- **Dead interaction state maintained in the hot path.**
  `crates/ui/src/interaction/mod.rs:132` declares `hot_widget`, `:184` clears it every frame,
  `:330-332` writes it — and `is_hot` (`:233`) has **no callers**. The state machine at `:336`
  uses `mouse_in_bounds` directly. Likewise `InteractionResult::local_mouse` (`:90`) is
  computed at `:322` on every `interact` call and read by nobody.

- **Unused public surface elsewhere.** `crates/ui/src/draw/mod.rs:129` `is_overlay`;
  `crates/ui/src/context/mod.rs:374` `hit_test` (a one-line `bounds.contains` wrapper);
  `crates/ui/src/font/mod.rs:199` `cache_stats`;
  `crates/ui/src/context/widgets.rs:196` `checkbox_labeled`;
  `crates/physics/src/physics_world/bodies.rs:295,300` `rigid_body_count`/`collider_count`;
  `crates/physics/src/presets.rs:36,53,59,79` `physics_prop`/`small_box`/`pushable_box`/`slippery`;
  `crates/audio/src/manager/music.rs:37` `play_music_once`;
  `cratests/audio/src/manager/mod.rs:315,331,347` `stop_all`/`active_sound_count`/`unload_all`
  (the last three are exercised only by their own unit tests).

- **Floatly-typed shape dispatch.** `crates/renderer/src/sprite.rs:46-64` defines a real
  `SpriteShape` enum, then immediately flattens it to an `f32` inside a `[f32; 4]` (`:39`), and
  `with_border` (`:151`) compares floats (`self.shape[0] == SpriteShape::Quad.to_f32()`) to
  recover the variant. **Fix:** store `SpriteShape` on `Sprite`; convert only in `to_instance`.

---

## 4. Non-human-readable names

Non-test abbreviated bindings by crate: **physics 9, ui 4, renderer 2, input 0, audio 0.**

Worst offenders:

| location | identifier | should be |
|---|---|---|
| `physics/src/physics_world/bodies.rs:72` | `he` | `half_extents_meters` |
| `physics/src/physics_world/bodies.rs:267` | `f` | `force_meters` |
| `physics/src/physics_world/bodies.rs:253` | `imp` | `impulse_meters` |
| `physics/src/physics_world/bodies.rs:19,212,226` | `pos` | `position_meters` |
| `physics/src/physics_world/bodies.rs:20,243` | `vel` | `velocity_meters` |
| `physics/src/components.rs:214` | `hw`, `hh` | `half_width`, `half_height` |
| `physics/src/physics_system/update.rs:19` | `dt` | `clamped_delta_time` |
| `physics/src/physics_world/queries.rs:24,26` | `max_toi`, `toi` | `max_time_of_impact` |
| `ui/src/input_state.rs:156` | `kb` | `keyboard` |
| `ui/src/input_state.rs:155` | `pos` | `mouse_position` |
| `ui/src/context/text_input.rs:279` | `dx` | `pointer_travel_x` |
| `ui/src/context/text_input.rs:527` | `bg` | `background` |
| `ui/src/text_edit.rs:182` | `d` | `distance_to_click` |
| `renderer/src/texture.rs:169,209` | `img` | `image` |

Odd binding: `crates/physics/src/physics_world/stepping.rs:56` — `let event_handler = ();`
is a named binding holding unit, passed at `:72` immediately after a literal `&()` at `:71`.
Delete the binding and pass `&()`.

**WGSL** (`crates/renderer/src/shaders/`):
- `sprite_instanced.wgsl:17,19` — `t_diffuse` / `s_diffuse` (Hungarian-ish prefixes).
- `sprite_instanced.wgsl:103-105` — `p`, `b`, `r`, `q` in `sd_rounded_box`. Defensible as
  tight SDF math, but `b` (half extents) and `r` (radius) are the parameters callers reason about.
- `bloom_extract.wgsl:46` — `l` for luminance.
- All four shaders use `idx` for `@builtin(vertex_index)`.

**Scripts / files:** `validate_demo.sh` and `run_gpu_diagnostics.sh` live at the repo root
outside `scripts/`, and neither name says which project stage it belongs to.
`validate_demo.sh:7` runs `cargo run --example sprite_demo` — an example that does not exist in
`Cargo.toml` or `examples/`. The script is dead.

---

## 5. Comment load

### Top 10 comment-heaviest files (comment lines ÷ code lines, files over 20 code lines)

| ratio | comment | code | file |
|---|---|---|---|
| 1.78 | 41 | 23 | `crates/ui/src/lib.rs` |
| 1.23 | 142 | 115 | `crates/physics/src/physics_system/mod.rs` |
| 0.76 | 141 | 186 | `crates/ui/src/context/mod.rs` |
| 0.70 | 144 | 207 | `crates/input/src/input_handler.rs` |
| 0.66 | 104 | 158 | `crates/input/src/input_mapping.rs` |
| 0.64 | 118 | 185 | `crates/audio/src/manager/mod.rs` |
| 0.55 | 51 | 92 | `crates/physics/src/lib.rs` |
| 0.53 | 32 | 60 | `crates/input/src/mouse.rs` |
| 0.52 | 179 | 345 | `crates/renderer/src/renderer.rs` |
| 0.51 | 93 | 183 | `crates/ui/src/draw/mod.rs` |

Most of this is legitimate API doc. The problems are narration and status text.

### Three narration comments that should be naming or structure

1. `crates/renderer/src/sprite/pipeline.rs:77,102,120,127,145,157,164,171,181,187` — a run of
   ten section headers ("Create texture bind group layout", "Create pipeline layout",
   "Create shader module", "Create the render pipeline"). Each marks exactly where a private
   builder function should start. **Replace with functions, not comments.**
2. `crates/renderer/src/white_texture.rs:15,31,34,50` — "Create a 1x1 white texture",
   "Create white pixel data (1, 1, 1, 1) - RGBA all 255 for white", "Write the white pixel data
   to the texture using the queue", plus a success log line. Every one restates the line under
   it in the function already named `create_white_texture_resource`.
3. `crates/physics/src/physics_system/sync.rs:53,56` — "Get transform for position" above
   `world.get::<Transform2D>(entity)`, and "Check if entity has rigid body component" above
   `if let Some(mut rigid_body) = world.get::<RigidBody>(entity)`.

Bonus: `crates/physics/src/physics_system/mod.rs:20-55` is a 35-line module-doc essay
justifying pass-through methods, including a changelog for the removed `apply_impulse`
pass-through. That is `log_archive.md` material, and it is why this file tops the ratio table.

### Stale status references inside source

- `crates/input/src/gamepad.rs:3-9` — "the engine currently has no gamepad backend (e.g.
  gilrs) producing gamepad events… gamepad state only changes if events are queued manually".
  **Contradicted by the gilrs backend that closed GAP-001 in July 2026** and by
  `crates/input/CLAUDE.md`, which documents `engine_core/gamepad_backend.rs`. Actively misleads.
- `crates/renderer/src/sprite/instance_cache.rs:1` — cites `PATTERNS_AUDIT.md GPP-15`; that
  file no longer exists in the repo.
- Live issue numbers and sprint tags baked into source comments that will outlive them:
  `renderer.rs:78,146,229,411,448` (#41, "review F3", H8/#7, #26);
  `line_pipeline.rs:10,206` (#89, #41); `scissor.rs:1` (#41);
  `bloom.rs:69,214` (#41); `sprite/pipeline.rs:33,53,402,447,541` (GPP-15, #26, #41);
  `sprite/batch.rs:20,82` (#41); `ui/src/input_state.rs:55` (#56);
  `physics/src/physics_system/mod.rs:86` and `sync.rs:7,47` (GPP-09);
  `physics/src/components.rs:225,576` ("kimi F2"); `physics/src/register.rs:1` (#43, GPP-16);
  `audio/src/manager/mod.rs:160` (H7).

### `crates/*/CLAUDE.md` restating code / carrying stale numbers

Test counts in the guides vs `#[test]` attributes actually present (src + tests):

| crate | guide claims | attributes found |
|---|---|---|
| renderer | 73 | 92 |
| ui | 119 | 123 |
| input | 79 | 74 |
| physics | 66 | 64 |
| audio | 27 | 26 |

All five are wrong, in both directions. The root `CLAUDE.md` explicitly warns "Trust this file /
memory for current test counts, not stale numbers inside older docs" — the per-crate guides are
now the stale docs.

Beyond counts, each guide's "File Map" section is a second copy of the module tree
(`crates/renderer/CLAUDE.md` file map, `crates/physics/CLAUDE.md` file map,
`crates/ui/CLAUDE.md` file map). These restate `mod` declarations and drift silently.

---

## 6. Game Programming Patterns alignment

### Done well (keep)

- **Dirty Flag, twice.** `crates/renderer/src/sprite/instance_cache.rs:39-60` byte-compares
  flattened instances plus batch layout before a GPU upload, and
  `crates/physics/src/physics_system/mod.rs:84-98` (`PushedState`) value-compares ECS
  components against a last-pushed baseline. Both are textbook and both are tested.
- **Flyweight.** `TextureHandle` + the per-handle bind-group cache
  (`sprite/pipeline.rs:31,290-312`) means no bind group is ever created inside a frame. The
  crate guide's "cache bind groups, never create per-frame" rule holds everywhere I looked.
- **Data Locality.** `SpriteInstance` (`sprite_data.rs:65-84`) is a flat 76-byte `Pod` uploaded
  as one contiguous buffer.
- **No Service Locator / global state.** Zero `static mut`, `thread_local!` or `OnceLock` in
  any of the five crates. Notably clean for an engine.
- **Game Loop.** `GameLoopManager` lives in `engine_core`, out of scope; the fixed-timestep
  sub-step loop in `crates/physics/src/physics_system/update.rs:57-70` is correctly bounded by
  `MAX_STEPS_PER_UPDATE` with the death-spiral drop documented.
- **Object Pool.** Particles live in `engine_core/particles/`, outside this cluster. Nothing in
  these five crates pools; nothing here needs to.

### Anti-patterns and gaps

- **Double Buffer is applied inconsistently.** `crates/input/src/gamepad.rs:77,159` keeps a
  genuine previous-frame axis snapshot (`prev_axis_values`, refreshed in `clear_frame_state`),
  which is why axis edges are correct. Keyboard and mouse have **no** equivalent:
  `crates/input/src/input_mapping.rs:226-229` *reconstructs* "was pressed last frame" as
  `(pressed && !just_pressed) || just_released`. That inference is wrong when a button is
  pressed and released inside a single frame (both `just_pressed` and `just_released` set →
  reconstructed as "was active", so `just_activated` returns false and the input is swallowed).
  Every `settings.just_activated(...)` in every game rides on this.
  **Fix:** snapshot the pressed set in `ButtonTracker::clear_frame_state`, exactly as the
  gamepad already does, and make `source_was_pressed` read it.
- **Observer is split with a dead half.** `crates/physics/src/physics_system/update.rs:85-88`
  clones every collision event onto the world event bus, and the same events are also handed
  out by `take_collision_events`. No consumer in these five crates reads the bus path; it
  should be confirmed against `engine_core`/games before deletion, but it is a per-event clone
  on every frame for a channel that may have no readers.
- **Per-step allocation in the collision loop.**
  `crates/physics/src/physics_world/stepping.rs:153` reassigns `previous_collisions` with a
  freshly allocated `HashSet` every step. **Fix:** `std::mem::swap` then `clear`.
  (Related known debt GPP-L10 on issue #85 covers the per-step contact `Vec`.)
- **Batch draw order depends on the caller.** `crates/renderer/src/sprite/batch.rs:85` stores
  batches in a `HashMap<(TextureHandle, Option<[u32;4]>), SpriteBatch>`, so iteration order is
  nondeterministic; correctness relies on callers sorting by min depth then handle (documented
  in `crates/renderer/CLAUDE.md`, enforced nowhere in this crate).
- **Stringly/floatly-typed dispatch.** `Sprite::shape` as `[f32; 4]` with a real enum next to it
  (see 3.8 above) is the one instance of this anti-pattern in the cluster.

---

## 7. Rust best-practice issues

- **`panic!` inside a wgpu callback.** `crates/renderer/src/renderer.rs:158-164` — the
  uncaptured-error handler panics on validation errors in debug native builds. Deliberate and
  commented, but it unwinds through an FFI callback. **Fix:** log plus a latch the frame loop
  reads.
- **`#[allow(...)]` inventory (three, all in scope).**
  - `renderer.rs:230` `clippy::arc_with_non_send_sync` — justified, dated, decided with H8/#7. Keep.
  - `ui/src/interaction/mod.rs:28` `clippy::should_implement_trait` on `WidgetId::from_str` —
    removable by renaming to `hashed(s)`; `From<&str>` already exists at `:49`.
  - `ui/src/draw/mod.rs:301` `clippy::too_many_arguments` on `slider` (7 params) — removable
    with a `SliderVisual` params struct.
- **Hand-computed vertex offsets.** `crates/renderer/src/sprite_data.rs:47,53` and
  `:141,147,153,159,165,171,177` compute attribute offsets as `size_of::<[f32; N]>()` with N
  counted by hand (3, 5, 9, 13, 14, 15). Adding or reordering a field means renumbering every
  later offset manually; a mistake is a silent GPU garbage bug.
  **Fix:** `wgpu::vertex_attr_array!`.
- **Lossy `as` cast that can wrap.** `crates/renderer/src/texture.rs:257` —
  `data.len() != (width * height * 4) as usize` multiplies three `u32` **before** the cast, so a
  large texture overflows in `u32` and passes the length check instead of erroring. Same shape
  at `:277` (`create_solid_color` allocates `(width * height) as usize` with no validation) and
  `:290` (`Vec::with_capacity((width * height * 4) as usize)`).
  **Fix:** promote to `u64`/`usize` before multiplying.
- **Clone-to-dodge-a-borrow, per frame, per field.**
  `crates/ui/src/context/text_input.rs:484` and `:526` — `self.theme.text_input.clone()` clones
  the entire `TextInputStyle` on every input draw. With an inspector full of numeric fields
  that is one struct clone per field per frame. **Fix:** derive `Copy` on the style structs, or
  read the handful of fields needed before drawing.
- **String-payload errors where a typed payload exists.**
  `crates/renderer/src/error.rs:20-21` `TextureNotFound(String)` holds a path;
  `crates/ui/src/font/mod.rs:28-29` `FontError::NotFound(String)` is constructed at
  `font/mod.rs:152` and `:174` as `format!("Font {} not found", handle.id)` — the handle is
  right there. **Fix:** `NotFound(FontHandle)`.
- **`ButtonTracker::release` records a phantom edge.**
  `crates/input/src/button_tracker.rs:55-58` inserts into `just_released` unconditionally, so a
  release event for a button that was never pressed (window refocus, synthetic event) reports a
  just-released edge. **Fix:** only insert when `pressed.remove(&button)` returned true.
- **Hand-maintained field lists that must track a struct.**
  `crates/physics/src/register.rs:16-28,36-47` lists `RigidBody` and `Collider` field names as
  string literals to exclude the `#[serde(skip)]` handle. Adding a field to
  `components.rs:48-68` or `:279-297` without updating `register.rs` silently drops it from the
  dynamic tier. Nothing checks the pair.
- **`pub(crate)` rapier handles on public serializable components.**
  `crates/physics/src/components.rs:67` and `:296` leak rapier lifetime into ECS data types.
  Acceptable given the `#[serde(skip)]`, but it is what forces 7.8 above.
- **`unwrap`/`expect` outside tests: none found** in the five crates' non-test code. Good.

---

## 8. Workspace layout

### Files in the wrong place or stale

- **`editor_prefs.json` is committed at the repo root** (`git ls-files` confirms it is tracked).
  It is a runtime artifact the editor writes, so every editor session dirties the working tree.
  **Fix:** untrack and add to `.gitignore`.
- **`examples/pong_editor_screenshot.png`** — a screenshot living inside the examples *source*
  directory alongside `hello_world.rs` and `editor_demo.rs`.
- **`validate_demo.sh` (repo root) is dead** — `:7` runs `cargo run --example sprite_demo`,
  which is not in `Cargo.toml` and not in `examples/`.
- **`run_gpu_diagnostics.sh` (repo root)** is an "EMERGENCY GPU DIAGNOSTICS" script from a
  resolved incident, sitting outside `scripts/`.
- **`coordination/` is now one live file and three headstones.** `TODO.md` is a pointer saying
  the queue moved to the Studio Board (Aug 19 2026); `BLOCKERS.md` contains only a
  commented-out example entry and "(No blockers yet)"; `H1_SPIKE.md` records a rodio decision
  marked FINAL in July. Only `PROGRESS.md` (132 KB) is live, and the root guide already names
  it as the narrative log. **Fix:** fold the three into `log_archive.md`.
- **`docs/plans/` holds three plans from Jan–Feb 2026** for scene saving and a DRY/SRP cleanup,
  all shipped. `docs/EDITOR_UX_AUDIT.md` is 75 KB whose live items the root guide says are now
  board issues.

### Agent tooling and guides

- **`AGENTS.md` is a symlink to `CLAUDE.md`** — documented and fine. But `CLAUDE.md` itself is
  34 KB carrying a full project status report (test counts, sprint history, phase state) that
  drifts every sprint, and four agent-tool directories (`.claude/`, `.junie/`, `.kimi-code/`,
  `.cursor/`) each carry their own skill copies. `scripts/check-skill-parity.sh` in the working
  set covers cross-*repo* drift, not these four in-repo copies.

### Cargo and the crate split

- **Feature naming: clean.** The workspace has exactly one feature, `editor`, named after what
  it enables, gating `editor_integration` plus the `editor` bin and `editor_demo` example.
- **`common` earns its crate.** `camera.rs`, `color.rs` and `rect.rs` are consumed by renderer,
  ui, editor and games; `vfs.rs` and `clock.rs` are the wasm seam that keeps `renderer` and
  `audio` target-agnostic; `sheet_grid.rs` is shared by ecs and engine_core. Keeping it is
  correct.
- **`ecs_macros` earns its crate.** A `proc-macro = true` crate cannot live inside `ecs`.
  80 lines is small but structurally required.
- **One questionable edge: `ui` depends on `input`** (`crates/ui/Cargo.toml`), which pulls winit
  into the UI crate. `InputState::from_input_handler` (`ui/src/input_state.rs:143`) is the only
  coupling point, and it is a snapshot struct. Inverting it (engine_core builds the snapshot and
  hands it to `ui`) would let `ui` drop winit entirely. Not urgent; worth noting.
- Everything else in the dependency graph matches the documented shape: `physics → ecs`,
  `renderer → common`, `audio → common`, `input → winit only`.

---

## Ranked top 10 changes for this cluster

1. **Fix the previous-frame reconstruction in `input/src/input_mapping.rs:226`.**
   Keyboard and mouse edges are *inferred* rather than snapshotted, unlike gamepad axes. A press
   and release inside one frame is misreported and the input is swallowed. Every
   `just_activated` in every game depends on it. Highest correctness risk in the cluster.

2. **Extract the shared camera binding across the three pipelines**
   (`sprite/pipeline.rs:103`, `line_pipeline.rs:87`, both `update_camera`). Largest renderer DRY
   win, already scoped as issue #89, and a prerequisite for any new pipeline.

3. **Unify `float_input` and `text_input`** (`ui/src/context/text_input.rs:147` and `:332`).
   They have already drifted on font resolution; the next divergence is a user-visible bug in
   the inspector.

4. **Collapse the two binding stores and the two pad-layout tables**
   (`input/src/player.rs:101` + `:396`, `input/src/input_mapping.rs:127` + `:249`).
   One rule, one place — the org's stated DRY standard, currently violated in the crate that
   defines the input contract.

5. **Delete the dead interaction state and dead public API.** `hot_widget`/`is_hot`/`local_mouse`
   in `ui/src/interaction/mod.rs` are maintained every frame and read by nobody; roughly a dozen
   never-called public methods across the five crates (listed in §3). Pure subtraction.

6. **Split `SpritePipeline::new_with_target`** (`sprite/pipeline.rs:70`, 183 lines) into named
   builders, deleting the ten narration comments that already mark the seams. Fixes an SRP and a
   comment-load finding in one edit.

7. **Correct the five crate `CLAUDE.md` test counts and drop the file-map sections.** All five
   counts are wrong, in both directions, in the exact way the root guide warns against. Guides
   that restate the module tree drift silently; guides that carry counts should carry none.

8. **Fix the two actively-misleading source comments:** the "no gamepad backend" module doc at
   `input/src/gamepad.rs:3-9` (contradicted by the shipped gilrs backend) and the
   `PATTERNS_AUDIT.md` reference at `renderer/src/sprite/instance_cache.rs:1` (file deleted).

9. **Harden the renderer's numeric edges:** replace the hand-computed offsets in
   `sprite_data.rs` with `wgpu::vertex_attr_array!`, and fix the `width * height * 4` u32
   overflow in `texture.rs:257,277,290`.

10. **Clean the workspace root:** untrack `editor_prefs.json`, delete `validate_demo.sh`
    (its example is gone), move `run_gpu_diagnostics.sh` into `scripts/`, move
    `examples/pong_editor_screenshot.png` out of the source dir, and fold
    `coordination/TODO.md`, `BLOCKERS.md` and `H1_SPIKE.md` into `log_archive.md`.

---

*Audit performed read-only. No files in the repository were modified.*
