Re-cut as a keep-list. I already had every file in context; I verified the remaining unknowns (audio's wasm gate at `manager/mod.rs:131`, the bus multiplication at `music.rs:84/:188/:193`, `AudioError` variants, `clear_collision_events` called once per frame at `physics_system/update.rs:55`).

# COUNT LINES

- **physics — current 64, keep 21**
- **input — current 74, keep 16**
- **audio — current 26, keep 8**

(physics is 1 over the ~20 guide, input 1 over the 15 guide. I flag both rather than pad or force a bad merge; the two marginal ones are named at the end of each section.)

---

# PHYSICS — 21 keeps

Paths relative to `/home/jedi/projects/insiculous/insiculous_2d/`. `ps` = `crates/physics/src/physics_system/tests.rs`, `pw` = `crates/physics/src/physics_world/tests.rs`, `comp` = `crates/physics/src/components.rs`, `ee` = `crates/physics/tests/external_edits.rs`.

```
CONTRACT | static body never moves          | ps:174 test_static_body_does_not_move
CONTRACT | collision start/ongoing/stop FSM | pw:106 test_collision_started_event
CONTRACT | sensors: events, no contacts     | pw:400 test_sensor_collider_fires_intersection_events
CONTRACT | contact points are world-space   | pw:359 test_contact_points_are_in_world_space
CONTRACT | capsule half-height math         | comp:440 test_collider_shapes
CONTRACT | pixels-per-meter sanitized       | pw:312 test_invalid_scale_in_struct_literal_is_sanitized_at_world_creation
CONTRACT | raycast distance is in pixels    | pw:335 test_raycast_normalizes_direction_so_distance_is_in_pixels
CONTRACT | catch-up cap, no backlog leak    | ps:112 test_catch_up_steps_are_capped_after_a_stall
CONTRACT | same-frame spawn op buffering    | ps:78  test_reset_body_is_deferred_for_same_frame_spawns
CONTRACT | ECS->rapier sync + orphan GC     | ps:47  test_direct_world_removal_cleans_up_physics_state
CONTRACT | clear() then resync from ECS     | ps:218 test_clear_allows_resync_from_ecs
CONTRACT | shape cycle carries dimensions   | comp:574 test_shape_cycle_round_trips_are_exact_where_the_mapping_is_clean
CONTRACT | dynamic-tier round-trip, no handle leak | reg:62 test_physics_components_round_trip_through_the_dynamic_tier
GUARD | zero-step frame emits NO events     | ps:302 test_zero_step_update_emits_no_collision_events
GUARD | every sub-step's events survive     | ps:319 test_events_from_all_sub_steps_in_one_update_survive
GUARD | apply_force lasts exactly one update| ps:363 test_apply_force_lasts_exactly_one_update
GUARD | physics entities must be ROOTS      | ps:491 test_parented_entity_with_rigid_body_is_treated_as_world_space
GUARD | live transform edit teleports, keeps velocity | ee:14 test_external_transform_edit_teleports_live_body
GUARD | writeback is not an external edit   | ee:49  test_physics_writeback_is_not_mistaken_for_external_edit
GUARD | live collider edit rebuilds rapier  | ee:104 test_collider_edit_rebuilds_live_rapier_collider
GUARD | CCD + restitution reflection        | tests/ball_brick_bounce.rs:51 ball_bounces_off_static_brick
```

`reg` = `crates/physics/src/register.rs`.

**Non-obvious choices.** Gravity-on-dynamic-bodies gets **no keep**: the crate doc example at `crates/physics/src/lib.rs:15-38` already asserts `position.y < 100.0` after an update, and doc tests run in the suite — `ps:150` and `lib.rs:64` are third and second copies. `comp:429` (box half-extents) folds into the capsule keep rather than standing alone. The two marginal keeps, if you must hit exactly 20: `ps:491` (pins a limitation, not a feature) and `pw:359` (a fixed bug that cannot recur without a rapier upgrade).

## Half A — physics contracts

| contract | keeper |
|---|---|
| gravity on dynamic bodies | `crates/physics/src/lib.rs:15-38` crate doc example (no `#[test]` needed) |
| static bodies do not move | `ps:174` |
| collision started event | `pw:106` (merge `:147` ongoing, `:191` stopped) |
| collision stopped event | folded into `pw:106` |
| sensors | `pw:400` |
| capsule half-height math | `comp:440` |
| validated pixels-per-meter + fallback | `pw:312` (merge `:294`'s config assertion) |
| raycast | `pw:335` (merge `:84`, `:322` zero-direction) |
| fixed-timestep accumulator + catch-up cap | `ps:112` |
| buffered `set_velocity` / `reset_body` for same-frame spawns | `ps:78` (merge `:422`, `:441`, `:462`) |
| transform teleport preserving velocity | `ee:14` |
| collider rebuild on live edit | `ee:104` |
| component removal drops the rapier collider | `ee:145` → **merge into `ee:104`** |
| CCD + restitution reflection | `ball_brick_bounce.rs:51` |
| collider absolute-pixel sizing / `RENDER_UNIT = 80` | **MISSING** |
| collision groups / filter (`bodies.rs:97-102`) | **MISSING** |
| `Collider.offset` applied as collider translation (`bodies.rs:89`) | **MISSING** |
| Kinematic body behavior | **MISSING** |
| `PhysicsSystem::raycast` wrapper (`mod.rs:218`) | **MISSING** |

## Half B — physics guards

| footgun | guard |
|---|---|
| Physics ignores `Transform2D.scale`; colliders are absolute pixels | **MISSING** — no test in the crate touches `scale`. Highest-value gap on this list. |
| Physics entities must be ROOT entities | `ps:491` — **confirmed, it exists**, at `crates/physics/src/physics_system/tests.rs:491`, and it asserts both halves (rapier body built at the child's *local* position, and the result written straight back into the local transform). |
| `step()` appends, `clear_collision_events` runs once per frame | implicitly by `ps:302` + `ps:319`; the call site is `physics_system/update.rs:55`. `pw:147`/`pw:191` call `clear_collision_events` manually, which is why they read as frame-driver simulations — folding them into `pw:106` keeps that visible. |
| zero-step frame emits NO events (no stale re-delivery) | `ps:302` (merge `:281`) |
| several catch-up sub-steps deliver all events | `ps:319` |
| second `take_collision_events` in a frame returns empty | `ps:341` → **merge into `ps:302`** |
| `apply_force` lasts exactly one update | `ps:363` (merge `:394` zero-step survival) |
| live `RigidBody` config edits (body_type, damping, gravity_scale) still need the body recreated | **MISSING** — `ee` covers transform and collider edits only. `component_editors.rs:210` documents the limitation in a comment with nothing pinning it. |

---

# INPUT — 16 keeps

`ieq` = `crates/input/tests/input_event_queue.rs`, `ihi` = `crates/input/tests/input_handler_integration.rs`, `im` = `crates/input/tests/input_mapping.rs`, `bt` = `crates/input/src/button_tracker.rs`, `gp` = `crates/input/src/gamepad.rs`, `pl` = `crates/input/src/player.rs`.

```
CONTRACT | queued != applied until process, in order | ieq:6  test_input_event_queuing
CONTRACT | update() clears just-pressed/released     | ieq:44 test_update_clears_just_states
CONTRACT | ButtonTracker model incl. key-repeat      | bt:96  test_repeated_press_does_not_retrigger_just_pressed
CONTRACT | mouse frame-delta model                   | mouse:20 test_first_position_update_records_position_without_delta
CONTRACT | wheel accumulates, clears per frame       | mouse:122 test_mouse_wheel
CONTRACT | InputMapping bind/unbind/reverse-index    | im:23  test_bind_and_query
CONTRACT | action lifecycle across frames            | ihi:8  test_action_activation_from_key_press
CONTRACT | axis-as-button at the mapping layer       | ihi:214 test_axis_source_drives_action_across_frames
CONTRACT | axis threshold crossing + re-arm          | gp:227 axis_just_activated_fires_once_on_crossing_and_rearms_below_threshold
CONTRACT | per-player device routing + assign_pad    | pl:486 default_pairing_isolates_player_devices
CONTRACT | merged digital + analog, clamped          | pl:547 move_y_merges_digital_and_stick_and_clamps
CONTRACT | save-on-change dirty tracking             | pl:446 binding_changes_set_dirty_and_take_dirty_clears_it
GUARD | just_activated is a STRICT edge              | ihi:63 test_second_source_does_not_retrigger_activation
GUARD | InputMapping::new() is EMPTY                 | im:14  test_new_mapping_is_empty
GUARD | pad auto-registers; disconnect leaves no edge| ihi:139 test_gamepad_action_integration_with_auto_registration
GUARD | stick +Y is up                               | ihi:286 test_default_bindings_include_pad_zero_movement
```

`mouse` = `crates/input/tests/mouse.rs`.

**Non-obvious choices.** `tests/keyboard.rs` contributes **zero** keeps: `KeyboardState.keys` is a `ButtonTracker<KeyCode>` (`src/keyboard.rs:9`), so `bt:96` owns the model and `ieq:44` proves the handler→`KeyboardState` delegation. `tests/input_handler.rs` and `tests/gamepad.rs` contribute zero (every test is a `Default` echo, a getter, or a `ButtonTracker`/`src/gamepad.rs` duplicate — `tests/gamepad.rs:131`, the manager's `clear_frame_state` fan-out, is the only near-miss and is implied by `pl:582`/`ihi` frame cycling). The four `im` default-preset tests are dropped: `ihi:8`'s merged group presses W/arrows/Space/Enter through `with_default_bindings()`, so the preset is checked behaviorally instead of as a literal table. The two marginal keeps if you must hit 15: `ihi:214` (overlaps `gp:227` one layer up) and `mouse:122`.

## Half A — input contracts

| contract | keeper |
|---|---|
| deferred queue, queued ≠ applied | `ieq:6` (merge `:21`) |
| multi-event ordering | `ieq:69` → **merge into `ieq:6`** |
| `just_pressed`/`just_released` clear on `update` | `ieq:44` |
| mouse first-position no-delta | `mouse:20` |
| mouse delta accumulates / resets per frame | `mouse:38`, `:58` → **merge into `mouse:20`** |
| wheel accumulation | `mouse:122` |
| pixels-to-lines scroll normalization (`SCROLL_PIXELS_PER_LINE = 16.0`) | **MISSING** |
| `InputMapping` bind / unbind / reverse-index | `im:23` (merge `:35`, `:49`, `:65`, `:76`, `:94`, `:105`, `:126`) |
| `just_activated` strict edge | `ihi:63` (merge `:82` release-one-keeps-active) |
| `InputSettings` per-player routing | `pl:486` (merge `:513`, `:533`, `:572`) |
| merged digital + analog axes | `pl:547` |
| axis threshold crossing + re-arm | `gp:227` (merge `:252`, `:264`) |
| gamepad disconnect | `ihi:264` → **merge into `ihi:139`** |
| bindings persistence (dirty → save) | `pl:446` (merge `:438`, `:463`, `:479`) |
| `convert_physical_key` / `handle_window_event` winit boundary | **MISSING** — `tests/keyboard.rs:118` is an assert-free placeholder |

## Half B — input guards

| footgun | guard |
|---|---|
| `InputMapping::new()` is EMPTY, nothing bound implicitly | `im:14` |
| pads auto-register on first event | `ihi:139` |
| disconnect leaves sources released with NO just-released edge | `ihi:264` (fold into `ihi:139` as one hotplug keep) |
| scroll pixels ÷ 16 → lines | **MISSING** |
| stick +Y is up (gilrs convention) | `ihi:286` — asserts `LeftStickY 1.0 → MoveUp` and `!MoveDown`; also the only place the default pad preset is checked behaviorally |
| `AXIS_ACTIVATION_THRESHOLD` default is 0.5 | **MISSING** — every axis test passes `0.5` explicitly, so changing the constant is invisible |

---

# AUDIO — 8 keeps

All in `/home/jedi/projects/insiculous/insiculous_2d/crates/audio/src/manager/tests.rs`.

```
CONTRACT | disabled mode is a working no-op   | :82  test_disabled_manager_loads_and_plays_as_noop
CONTRACT | missing file -> IoError            | :127 test_load_sound_missing_file_returns_io_error
CONTRACT | garbage bytes -> DecodeError       | :138 test_load_sound_from_invalid_bytes_returns_decode_error
CONTRACT | unload invalidates the handle      | :150 test_unloaded_sound_can_no_longer_be_played
CONTRACT | enable_output preserves handles, ids, buses | :255 test_enable_output_preserves_sounds_ids_and_volumes
CONTRACT | pending music: last request wins, cleared by stop_music, none after a failed load | :318 test_new_music_request_replaces_pending
CONTRACT | bus volumes clamp                  | :192 test_volume_setters_clamp_out_of_range_values
CONTRACT | SoundSettings clamp + speed floor  | :7   test_sound_settings_volume_clamping
GUARD    | disabled play_music: Ok, but is_music_playing() stays false | folded into :318 (via :293's assertion)
```

**Coordination note applied.** No keep spends on `stop_all`, `active_sound_count`, `play_music_once` or `unload_all`. That drops `:161` (unload_all), `:173`/`:181` (both assert `active_sound_count() == 0` on a disabled manager, where the count is structurally always 0 — vacuous regardless). **Three keeps currently call `play_music_once` and must be rewritten if it is deleted**: `:318` (use `play_music` twice with different volumes), `:212` (folding into `:318` removes the call), `:228` (its `IoError` assertion should move onto `play_music`).

## Half A — audio contracts

| contract | keeper |
|---|---|
| disabled-mode invariants | `:82` (merge `:91` invalid handle, `:108` `new_or_disabled`) |
| `enable_output` preserves handles + bus volumes | `:255` (merge `:242` result/state agreement, `:275` twice-is-noop, `:347` pending consumed-or-kept) |
| pending music last-request-wins / cleared by `stop_music` | `:318` (merge `:293`, `:306`, `:335`, `:212`) |
| typed `IoError` vs `DecodeError` | `:127` + `:138` |
| `InvalidHandle` after unload | `:150` |
| volume bus **multiplication** (`base × sfx × master`, `music.rs:84/:188/:193`) | **MISSING** — only the clamping of each bus is tested, never the product |
| `SoundSettings` clamping | `:7` (merge `:16`) |
| sound ids are manager-local | `:36` → **merge into `:255`**, which already asserts the id sequence continues |

## Half B — audio guards

| footgun | guard |
|---|---|
| disabled `play_music` returns Ok but `is_music_playing()` stays false | `:212` → fold into the `:318` keep (`:293` asserts exactly this) |
| wasm32 always starts disabled (browsers gate audio behind a gesture) | **MISSING and unreachable** — the branch is `manager/mod.rs:131` under `#[cfg(target_arch = "wasm32")]`, so a native `cargo test` can never execute it. Either accept it as `scripts/check_wasm.sh` territory or refactor the gate into a testable `fn should_start_disabled(is_wasm: bool)`. |
| `IoError` carries no path; decode errors carry a message | **MISSING** — `AudioError::IoError(#[from] io::Error)` (`error.rs:31`) genuinely loses the path, which is a real debugging footgun and nothing pins or documents it in a test. `:127` only matches the variant. |

---

# MERGE-INTO

Physics — into `ps:78`: `:422`, `:441`, `:462`. Into `ps:47`: `:27`. Into `ps:218`: `:198`. Into `ps:302`: `:281`, `:341`. Into `ps:363`: `:394`. Into `pw:106`: `:147`, `:191`. Into `pw:312`: `:294`. Into `pw:335`: `:84`, `:322`. Into `comp:440`: `:429`. Into `comp:574`: `:537`, `:515`, `:523`. Into `ee:104`: `:145`, `:80`.

Input — into `ieq:6`: `:21`, `:69`, `:88`. Into `im:23`: `:35`, `:49`, `:65`, `:76`, `:94`, `:105`, `:126`. Into `ihi:8`: `:22`, `:42`. Into `ihi:63`: `:82`. Into `ihi:139`: `:264`. Into `ihi:214`: `:244`. Into `gp:227`: `:252`, `:264`. Into `pl:486`: `:513`, `:533`, `:572`, `:582`. Into `pl:446`: `:438`, `:463`, `:479`. Into `bt:96`: `:87`, `:106`, `:115`.

Audio — into `:82`: `:91`, `:108`. Into `:255`: `:242`, `:275`, `:347`, `:36`. Into `:318`: `:293`, `:306`, `:335`, `:212`. Into `:7`: `:16`.

**The `spawn_body` helper.** The 3-component spawn (`Transform2D` + `RigidBody` + `Collider` + `initialize` + `update`) is retyped in 14 places. **Thirteen of the 21 physics keeps should share one helper**: `ps:174`, `ps:112`, `ps:78`, `ps:47`, `ps:218`, `ps:302` (already via `overlapping_pair`, which should be re-expressed on top of it), `ps:319`, `ps:363`, `ps:491`, and all four `ee` tests. Signature: `fn spawn_body(world: &mut World, pos: Vec2, body: RigidBody, collider: Collider) -> EntityId`, plus `fn no_gravity_system() -> PhysicsSystem` for the eight tests that build `PhysicsSystem::with_config(PhysicsConfig::new(Vec2::ZERO))` verbatim. It must be reachable from `crates/physics/tests/` too (a `pub` item behind a `test-support` feature, or a `tests/common/mod.rs` mirror), or the four `ee` keeps keep their copies. `tests/ball_brick_bounce.rs:17/:30`'s `spawn_brick`/`spawn_ball` are deliberately tuned fixtures — leave them.

Second duplication worth fixing while merging: `pw:135/:169/:181/:223` write the a↔b pair-matching closure longhand, while `pw:384/:423` in the same file use `CollisionEvent::involves`. The merged `pw:106` keep should use `involves` throughout.

# WEAK KEEPS

- **`ps:78` (after merging `:462`)** — the merged-in `test_reset_body_zeros_velocity_and_sets_position` asserts only `vel.length() < 1.0`; its name promises the position half and never checks it. The merged keep must assert `Transform2D.position ≈ (100, 200)` after the following update.
- **`pw:312` (after merging `:294`)** — `:294`'s post-step assertion is `pos.x.is_finite() && pos.y.is_finite()`, which the house rubric names explicitly as not-a-test. The merged keep should assert the actual fall for a 100 px/m scale (≈ −0.136 px after one 1/60 step), so a wrong-but-finite scale fails.
- **`pw:106` (after merging `:147`, `:191`)** — the three source tests each pass on a *vacuous* find (`.find(...)` then `assert!(is_some())` is fine, but the ongoing/stopped phases rely on manual `clear_collision_events` calls that the real frame driver makes at `update.rs:55`). The merged keep should drive the phases through one buffer-clearing loop so it also pins "the driver clears once per frame".
- **`ihi:286`** — asserts only `is_active` for `LeftStickY 1.0`. It should also assert a sub-threshold deflection (e.g. `0.3`) is *not* active, so it pins the 0.5 threshold as well as the sign convention.
- **`mouse:122`** — asserts wheel accumulation and per-frame clear, but never touches the pixel-delta path, so `SCROLL_PIXELS_PER_LINE = 16.0` (`input_handler.rs:41`) is unguarded. Extend it with one `WindowEvent::MouseWheel` in `PixelDelta` form asserting 32 px → 2.0 lines.
- **`:192` (audio)** — asserts each bus clamps, but never that the buses *multiply* into the sink volume. Extend it to assert the derived `base × sfx × master` product, or accept that the multiplication stays untested until an enabled-manager seam exists.

Two of my four earlier strengthen findings do **not** reappear: `pw:65 test_step_simulation` (the assertion inside `if let Some(..)` that passes on `None`) and `lib.rs:124 test_collision_detection` (`distance >= 10.0` on boxes that start 10.0 apart, so it passes with zero collision response) are both **off the keep-list entirely** — deleted rather than strengthened, since `pw:106` and the crate doc example cover what they meant to.

*(This supersedes the delete-list framing of my previous report; the delete rationale there still applies to everything not named above.)*