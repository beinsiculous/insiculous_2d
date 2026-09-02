Root for all paths: `/home/jedi/projects/insiculous/insiculous_2d/`

## Counts

```
editor_integration — current 150, keep 25
ecs inline         — current 119, keep 25
```

---

# HALF A — CONTRACT keeps

## editor_integration (25)

```
CONTRACT | play/pause/resume/stop FSM + snapshot capture | crates/editor_integration/src/editor_game/tests.rs:48 test_play_pause_resume_stop_cycle
CONTRACT | Stop resets the transform-propagation baseline | crates/editor_integration/src/editor_game/tests.rs:95 test_stop_resets_transform_propagation_cache
CONTRACT | save choke point: file written, parses back, dirty cleared | crates/editor_integration/src/editor_game/tests.rs:220 test_save_scene_roundtrip
CONTRACT | Game::render derives the GPU camera + scene-view scissor from the dock | crates/editor_integration/src/editor_game/tests.rs:351 test_render_overrides_camera_from_viewport
CONTRACT | scale tool moves transform AND collider as ONE undo entry | crates/editor_integration/src/editor_game/tests.rs:498 test_gizmo_scale_undo_restores_transform_and_collider_together   [WEAK — see below]
CONTRACT | save refused mid play session, allowed after Stop | crates/editor_integration/src/editor_game/play_guard_tests.rs:27 test_save_refused_while_playing
CONTRACT | New/Open refused mid play session | crates/editor_integration/src/editor_game/play_guard_tests.rs:87 test_scene_replace_refused_during_play_session
CONTRACT | snapshot-loss warning for uncapturable components | crates/editor_integration/src/editor_game/play_guard_tests.rs:134 test_play_surfaces_warning_for_unregistered_components
CONTRACT | Stop restores the AUTHORED world; resume never re-captures | crates/editor_integration/src/editor_game/play_guard_tests.rs:187 test_resume_from_pause_does_not_recapture_snapshot
CONTRACT | load_scene dry-runs into a scratch World before touching the live one | crates/editor_integration/src/editor_game/scene_io_tests.rs:105 test_load_instantiate_failure_preserves_world   (parses fine, instantiate fails — the case a parse-first fix would miss)
CONTRACT | scene physics block reaches the world as a resource and survives re-save | crates/editor_integration/src/editor_game/scene_io_tests.rs:183 test_load_scene_publishes_physics_resource_and_save_keeps_the_block
CONTRACT | engine time is frozen outside Play; the game owns time_scale during Play | crates/editor_integration/src/editor_game/time_freeze_tests.rs:285 test_particles_and_animations_do_not_advance_while_editing
CONTRACT | Play adopts the game camera pose incl. zoom; follow armed at session start | crates/editor_integration/src/editor_game/camera_follow_tests.rs:345 test_play_adopts_game_camera_pose_including_zoom
CONTRACT | camera sync is one-directional: playing+following only, paused never | crates/editor_integration/src/editor_game/camera_follow_tests.rs:373 test_sync_copies_zoom_only_while_playing_and_following
CONTRACT | Stop restores the editing view and re-arms follow (session start ONLY) | crates/editor_integration/src/editor_game/camera_follow_tests.rs:445 test_stop_restores_editing_view_and_rearms_follow
CONTRACT | gizmo commit = one undo entry restoring every dragged root | crates/editor_integration/src/editor_game/gizmo_drag_tests.rs:83 test_commit_records_one_undo_entry_restoring_every_root
CONTRACT | grid snapping: slow drags step cells, formation offsets survive | crates/editor_integration/src/editor_game/gizmo_drag_tests.rs:140 test_slow_snapped_drag_steps_grid_cells_instead_of_freezing
CONTRACT | Escape cancel restores starts (collider included) and pushes no history | crates/editor_integration/src/editor_game/gizmo_drag_tests.rs:221 test_cancel_restores_starts_and_pushes_no_undo_entry
CONTRACT | held arrow = one merged undo entry, sealed on release | crates/editor_integration/src/editor_game/shortcuts_tests.rs:306 test_held_arrow_merges_into_one_undo_entry_sealed_on_release
CONTRACT | API script builds a scene; every step is one GUI-undoable entry | crates/editor_integration/src/editor_game/api_write_tests.rs:242 test_api_script_builds_scene_and_gui_undo_reverts_each_step
CONTRACT | api_batch commits on Play as one entry, is discarded on Stop | crates/editor_integration/src/editor_game/api_write_tests.rs:323 test_play_start_commits_open_batch_as_one_entry
CONTRACT | every advertised archetype maps to a real factory | crates/editor_integration/src/editor_game/api_write_tests.rs:277 test_api_create_archetypes_all_map_to_factories   (the only drift lock over the nine entity factories)
CONTRACT | --api envelope: headless query→create→mutate→save, reloaded by a second session | crates/editor_integration/src/editor_game/headless/tests.rs:139 test_full_authoring_loop_survives_a_reload
CONTRACT | picking AABBs match the RENDER_UNIT-scaled render, with a nonzero panel origin | crates/editor_integration/src/editor_game/picking_tests.rs:401 test_pick_hits_sprite_at_rendered_size_with_offset_panel
CONTRACT | inspector writeback applies + records undo through apply_component_edit | crates/editor_integration/src/panel_renderer/tests.rs:8 test_transform_writeback_applies_and_records_undo   (the ONLY coverage apply_component_edit has in the workspace)
```

### editor_integration contracts with NO keep (MISSING)

- **`UiElementsHidden` inserted on init, removed on Play, re-inserted on Stop** (`editor_game/mod.rs:302`, `shortcuts.rs:92`, `shortcuts.rs:151`) — **MISSING**. No test in the crate mentions the resource. Scene-authored UI silently drawing in the editor, or silently never drawing in Play, would ship green.
- **`GridBackdropReset` requested on Stop** (`shortcuts.rs:154 request_backdrop_reset`) — **MISSING** in this crate.
- **Confirm-dialog state machine** — the six `scene_confirm_tests.rs` tests are real but did not survive the 25-slot budget; if you want #52 guarded, promote `scene_confirm_tests.rs:66 test_dialog_swallows_keys_and_escape_cancels` (it subsumes :31/:41/:52/:86/:98) as a 26th keep.
- **`chrome_owns_mouse`** — you named it as expected. `picking_tests.rs:328 test_chrome_owns_mouse_while_widget_holds_the_gesture` is the guard and it is *good* (it covers the release frame, which is when picking decides). It lost a slot to the RENDER_UNIT keep; promote it as a 27th if the budget stretches. Flagging rather than silently dropping.

## ecs inline (25)

```
CONTRACT | dynamic tier: insert/extract/remove a component by NAME on a World | crates/ecs/src/component_registry/tests.rs:74 test_insert_extract_remove_round_trip_on_a_world
GUARD    | name↔TypeId mapping is per concrete type | crates/ecs/src/component_registry/tests.rs:37 test_registry_register_and_lookup
GUARD    | two types under one name panic at registration, not at scene load | crates/ecs/src/component_registry/tests.rs:146 test_same_name_different_type_registration_panics
CONTRACT | builtin roster + persisted/transient split | crates/ecs/src/component_registry/tests.rs:209 test_global_registry_has_builtin_components
GUARD    | global registry survives a panic in a registration closure; re-entry panics loudly | crates/ecs/src/component_registry/tests.rs:290 test_global_registry_recovers_from_a_poisoned_lock
CONTRACT | games register components after init and the global tier sees them | crates/ecs/src/component_registry/tests.rs:188 test_late_registration_into_global_is_visible
CONTRACT | StateMachine transition: current/previous/just_entered/elapsed reset | crates/ecs/src/state_machine.rs:283 test_transition_updates_current_and_previous
CONTRACT | HierarchicalStateMachine cross-group transition reports parent change | crates/ecs/src/state_machine.rs:412 test_hierarchical_transition_across_groups
CONTRACT | events are readable more than once per frame (the collision-drain rule) | crates/ecs/src/event.rs:273 test_events_readable_multiple_times_before_flush
CONTRACT | flush clears every queue at the frame boundary | crates/ecs/src/event.rs:185 test_flush_clears_all_events
CONTRACT | resources are keyed by type and coexist | crates/ecs/src/resource.rs:179 test_multiple_resource_types
CONTRACT | GlobalTransform2D composition: parent scale/rotation applied to child local | crates/ecs/src/hierarchy.rs:278 test_global_transform_mul
GUARD    | Children is an ordered Vec (order is load-bearing for panel + scene) | crates/ecs/src/hierarchy.rs:232 test_children_component   [WEAK — see below]
CONTRACT | reparenting prunes the old parent's child list | crates/ecs/src/hierarchy_extension.rs:428 test_reparent_entity
CONTRACT | scale propagates through the hierarchy system | crates/ecs/src/hierarchy_system.rs:401 test_scaled_parent_transform_propagation   (position propagation belongs to your tests/hierarchy_dirty.rs keeps — this is the only scale case anywhere)
CONTRACT | Tilemap.sprite_instances geometry: UV cell, row-zero-on-top, non-zero tiles only | crates/ecs/src/tilemap.rs:142 test_sprite_instances_uv_region_for_known_index
CONTRACT | Tilemap RON round trip | crates/ecs/src/tilemap.rs:189 test_tilemap_ron_round_trip
CONTRACT | UiAnchor + offset resolve to a screen rect | crates/ecs/src/ui_components.rs:259 test_resolve_anchored_pos_matrix
CONTRACT | UI components deserialize from partial scene data with sane defaults | crates/ecs/src/ui_components.rs:286 test_serde_defaults_fill_missing_fields
CONTRACT | Lifetime despawns the entity when it crosses zero | crates/ecs/src/lifetime.rs:78 test_entity_despawns_when_lifetime_crosses_zero
CONTRACT | SpriteAnimationSystem writes the resolved cell into Sprite.tex_region | crates/ecs/src/sprite_system.rs:60 test_system_writes_current_frame_region_to_sprite
GUARD    | every component must round-trip through serde_json AND RON | crates/ecs/src/script.rs:124 test_scripts_serde_round_trips_every_value_variant   (the only test in the crate that asserts both wires on one type)
CONTRACT | Behavior RON round trip incl. Option fields | crates/ecs/src/behavior.rs:494 test_camera_follow_serialization_round_trips_dead_zone
CONTRACT | legacy 4-field CameraFollow scenes still parse, new fields default off | crates/ecs/src/behavior.rs:552 test_camera_follow_parses_legacy_four_field_form
CONTRACT | Behavior variant cycling (editor) round-trips index↔variant, wraps | crates/ecs/src/behavior.rs:474 test_default_for_variant_round_trips_variant_index
```

### ecs contracts with NO keep (MISSING)

- **`Behavior` ↔ `BehaviorData` `From` pair** — **MISSING from ecs inline**; the pair lives in `crates/engine_core/src/scene_data.rs`, so it is engine_core's to keep. Nothing in ecs inline touches it.
- **`GridBackdrop`** — only `grid_backdrop.rs:181 test_topology_cycle_order_round_trips_through_index` (editor cycle order) exists; it did not make the 25. The component is data-only in ecs; its behavior (normalized rebuild, `translate`, `ripple`) lives in `engine_core::grid`. If you want one GridBackdrop line in ecs, `grid_backdrop.rs:181` is it.
- **`AudioSource::calculate_attenuation`** (`audio_components.rs:246`) — real non-obvious math with no other home (no runtime system consumes AudioSource yet). Cut for budget; promote if spatial audio ships.
- **`ResourceStorage::clear`** — folded into the `resource.rs:179` keep as a merge-in; today it is `resource.rs:201`.

---

# HALF B — GUARD coverage

## ecs footguns

| Footgun | Guard |
|---|---|
| `Box<dyn Component>` is not clonable; copying downcasts per concrete type | `crates/ecs/src/component_registry/tests.rs:74` (extract/insert by name is the clone-free path) — **but the copying itself is guarded in editor**: `crates/editor/src/world_snapshot/tests.rs:96/:123/:144/:191` (typed-clone capture per concrete type) |
| bare `.as_any()` on the Box hits the blanket impl and every downcast fails | **MISSING.** `component_registry/tests.rs:60` and `:259` do `component.downcast_ref::<T>()` on the boxed value, so the correct path is exercised — but nothing pins the failure mode. A one-line test that a boxed component downcasts through `.as_ref().as_any()` and that the Box's own TypeId differs is the missing guard. |
| TypeId is per concrete type | `crates/ecs/src/component_registry/tests.rs:37 test_registry_register_and_lookup` (`name_for(TypeId::of::<T>())`) + `:146` collision panic |
| `GlobalTransform2D` is system-owned; manual writes are silently overwritten | **MISSING.** `crates/ecs/tests/hierarchy_dirty.rs:172` covers re-adding a *removed* global, but nothing asserts that a hand-written `GlobalTransform2D` is clobbered on the next update (and that `Transform2D` is the edit surface). This is the exact shape of a bug an author would file as "the editor ignores my edit". |
| reparenting must reject cycles | Prefer the integration test: `crates/ecs/tests/world.rs:108 test_hierarchy_cycle_detection` (asserts the error names the cycle) + `:130` self-parent. The inline `hierarchy_extension.rs:301/:292` are weaker duplicates — delete them. |
| serde_json (inspector) AND RON (scene) must both work for every component | `crates/ecs/src/script.rs:124` is the only test asserting both wires on one type. Per-component coverage is one-sided: `ui_components.rs:286` json-only, `tilemap.rs:189` RON-only, all of `behavior.rs` RON-only, registry tests json-only. **Partial** — no component other than `Scripts` is proven on both. |
| `Children` is a `Vec` because child order is load-bearing | `crates/ecs/src/hierarchy.rs:232 test_children_component` — **WEAK**: it asserts add/dedup/remove and lengths, never order. A `HashSet` swap would pass it. |
| `get`/`get_mut` return `Option` and cannot be borrowed simultaneously | **MISSING as a named guard** (partly compile-time). The read-then-`get_mut` sequential pattern is exercised for real by `crates/ecs/src/sprite_system.rs:60` (advance animation, then write the sprite in a second lookup) — that keep is the de-facto guard. |

## editor_integration footguns

| Footgun | Guard |
|---|---|
| physics ignores `Transform2D.scale` → the scale tool must scale the collider | `crates/editor_integration/src/editor_game/tests.rs:498` (one MacroCommand, **WEAK**: the macro is hand-built in the test) + `tests.rs:476 test_scale_collider_scales_shapes_and_offset` (the math) + `gizmo_drag_tests.rs:221` (cancel rolls the collider back). The **commit** path's collider command is untested against production. |
| camera sync must never run the other direction | `crates/editor_integration/src/editor_game/camera_follow_tests.rs:373` (follow off ⇒ viewport untouched; paused ⇒ untouched) + `tests.rs:426` (Editing ⇒ the game camera does not move the view — being deleted; fold its Editing case into `:373`). |
| rotation is deliberately not synced | **MISSING.** No test sets a rotation on the main-camera entity and asserts the viewport ignores it. |
| the early return in `handle_play_mode_camera` before picking/marquee/drops is load-bearing | **MISSING.** `viewport_interaction.rs:23/:50` — nothing asserts that during Play a viewport click does not pick, marquee, or accept an asset drop. `camera_follow_tests.rs:473` covers only the gesture cancellation *at the transition*. |
| the dirty mirror must be driven by production code, not reconstructed in a test | **MISSING — actively mis-covered.** `tests.rs:300` and `tests.rs:332` both write `editor.editor.set_dirty(editor.command_history.is_dirty())` inside the test body; the production mirror in `update()` is never executed. Both should be deleted, and the replacement must call the real update path. |
| Behavior scene fixture: every variant loads, round-trips, reaches the runner | **MISSING.** Closest is `crates/engine_core/tests/scene_loader_parse.rs:236`, which parses the two shipped example scenes. Across every scene in the repo (`examples/assets/scenes/*.ron` + `../games/*/assets/scenes/*.ron`) only **7 of 8** variants appear — `FollowTagged` is in no scene at all — and none of them is saved back out or stepped through `BehaviorRunner`. A single `all_behaviors.scene.ron` fixture asserted for load → save → reload → one runner tick would close three gaps at once. |

---

# MERGE-INTO

Fold these into the named keep so the keep is complete; then delete the source.

**editor_integration**
- → `tests.rs:220` (save): `tests.rs:187` (dirty cleared + "Scene saved" status), `tests.rs:172` (file exists).
- → `tests.rs:351` (render): `tests.rs:398` (hidden scene panel ⇒ zero-size scissor, never `None`).
- → `tests.rs:498` (scale+collider): `tests.rs:476` (`scale_collider` box/circle/offset math).
- → `play_guard_tests.rs:27` (save refused): `:44` (Paused counts), `:66` (works again after Stop).
- → `play_guard_tests.rs:87` (new/open refused): `:107` (world not cleared, snapshot survives, status bar carries the refusal).
- → `play_guard_tests.rs:134` (loss warning): `:151` (clean world must not nag), `:166` (Stop reports what the restore dropped).
- → `play_guard_tests.rs:187` (resume/Stop): `tests.rs:73` (Stop restores world state), `tests.rs:35` (Play captures a snapshot), `tests.rs:95` **only if** you drop it as a standalone keep — I kept it separately.
- → `scene_io_tests.rs:105` (dry run): `:73` (malformed RON), `:91` (missing file), `:126` (the success half: world replaced, selection cleared, clean, path adopted).
- → `scene_io_tests.rs:183` (physics): `:223` (new scene clears the physics resource).
- → `time_freeze_tests.rs:285`: `:262` (the `editor_time_scale` handshake, incl. "a game that paused itself stays paused").
- → `camera_follow_tests.rs:345`: `:361` (no main camera ⇒ zoom 1.0 parity), `:503` (extreme zoom unclamped, zoom 0 ⇒ 1.0), `:473` (play transition kills the in-flight gesture) *or keep :473 separate if you prefer a named gesture guard*.
- → `camera_follow_tests.rs:445`: `:427` (resume must not re-arm a follow the user broke), `tests.rs:455`.
- → `camera_follow_tests.rs:373`: `tests.rs:426` (Editing case).
- → `gizmo_drag_tests.rs:83`: `:119` (zero-delta commit pushes nothing).
- → `gizmo_drag_tests.rs:140`: `:166` (formation offsets survive), `:188` (hold-Ctrl snaps with the pref off), `:203` (zero grid degrades to unsnapped, never NaN).
- → `gizmo_drag_tests.rs:221`: `:267` (commit AND cancel seal the nudge merge window).
- → `shortcuts_tests.rs:306`: `:292` (1 unit / 10 with Shift), `:335` (roots only — children must not double-move), `:354` (suppressed mid-drag), `:378` (Escape cascade: drag first, then selection).
- → `api_write_tests.rs:242`: `:289` (save writes the file and clears the watermark), `:371` (empty name rejected before the factory runs).
- → `api_write_tests.rs:323`: `:383` (Stop discards a batch opened while Paused), `:304` (save refused with an open batch / mid-session).
- → `api_write_tests.rs:277`: `entity_ops_tests.rs:390` (UI entities are named, screen-space, and carry **no** `Transform2D`/`GlobalTransform2D` — that assertion is worth carrying over).
- → `headless/tests.rs:139`: `:184` (unknown verb is one error line, session continues), `:231` (`HeadlessAssets` returns refs verbatim), `:206` (#66 unissued handle never reaches the file).
- → `picking_tests.rs:401`: `:377` (size = sprite.scale × transform.scale × RENDER_UNIT), `:433`/`:444`/`:455` (either component missing ⇒ not pickable).
- → `panel_renderer/tests.rs:8`: `:34` (a `None` edit touches neither world nor history), `:54`/`:79`/`:104`/`:129` (Sprite/RigidBody/Collider/AudioSource as rows of one table), `:154` (continuous edits merge into one undo entry).

**ecs inline**
- → `component_registry/tests.rs:74`: `:50` (factory from JSON), `:65` (unknown type is a typed error), `:121` (malformed JSON rejected with **no** partial attach), `:24`.
- → `component_registry/tests.rs:209`: `:303` (GridBackdrop — make it a row), `:241` (AudioSource builds from full JSON), `:164`/`:177` (transient exclusion + sorted names; `:209` already asserts `is_persisted` both ways).
- → `state_machine.rs:283`: `:274` (initial state), `:319` (tick clears `just_entered`, accumulates), `:332` (elapsed resets), `:349` (`just_left`), `:360` (only the last previous is kept), `:296`/`:307` (same-state no-op vs `force_transition_to`), `:342`, `:370`.
- → `state_machine.rs:412`: `:389`, `:399` (within-group ⇒ no parent change), `:426`, `:440`, `:447`, `:455`.
- → `event.rs:273`: `:166` (emit order), `:178` (empty read), `:218` (count), `:208`, `:230`, `:257`.
- → `event.rs:185`: `:197` (types independent), `:242` (emit works after flush — drop its `type_count` assert).
- → `resource.rs:179`: `:122`, `:131` (get_mut), `:142` (insert returns the previous), `:154`/`:164` (remove + absent), `:201` (clear), `:170`, `:195`.
- → `hierarchy.rs:278`: `:295` (rotation case), `:312` (`transform_point` — same math), `:266`, `:258`.
- → `hierarchy_extension.rs:428`: `:280` (basic link), `:317` (`remove_parent`), `:330` (roots exclude children), `:361` (**ancestors are ordered nearest-first** — carry this assert over, it is the one contract `tests/world.rs` does not cover).
- → `tilemap.rs:142`: `:131` (count = non-zero tiles), `:152` (row 0 on top, columns right), `:161` (all-zero map), `:168` (short/long `tiles` vec truncates, never panics), `:178` (bounds-checked accessors).
- → `ui_components.rs:286`: `:312` (all three default to visible), `:251` (ALL/index round trip — see weak keeps), `:304`.
- → `ui_components.rs:259`: `:238` (the nine anchor fractions).
- → `lifetime.rs:78`: `:99` (expiry fires exactly once, later updates are safe), `:112` (independent timers), `:127`.
- → `sprite_system.rs:60`: `:87` (dt 0 freezes the frame), `:73` (animation without a sprite advances, no panic), `:100` (an unresolvable frame leaves `tex_region` untouched).
- → `script.rs:124`: `:152` (BTreeMap ⇒ deterministic param order), `:162` (variant cycle).
- → `behavior.rs:494`: `:384` (PlayerPlatformer round trip), `:575`.
- → `behavior.rs:552`: `:412` (PlayerPlatformer serde defaults), `:528` (CameraFollow defaults), `:456`.
- → `behavior.rs:474`: `:483` (out-of-range wraps — drop its `assert_eq!(count, 8)`), `:442` (`PatrolTarget::other`), `:448` (`EntityTag::matches`).
- → `hierarchy.rs:232`: `crates/ecs/src/hierarchy_extension.rs:280` if you want the world-level link assertion alongside the container one.

---

# WEAK KEEPS

1. **`editor_game/tests.rs:498 test_gizmo_scale_undo_restores_transform_and_collider_together`** — asserts now: a `MacroCommand` *the test builds* undoes transform + collider (that is `editor/src/commands/tests.rs:370`). Should assert: set up a real `GizmoDragState` with a collider, apply a scale interaction, call `commit_gizmo_drag`, then assert **one** history entry whose undo restores both `Transform2D.scale` and the collider half-extents. As written it cannot fail if `commit_gizmo_drag` stops emitting the `SetColliderCommand` — which is the exact footgun (physics ignores `Transform2D.scale`).
2. **`hierarchy.rs:232 test_children_component`** — asserts now: `is_empty`, `len`, `contains`, dedup on re-add, `remove`. Should assert: **order** — that `add(a); add(b); add(c)` yields `[a, b, c]`, that re-adding `a` keeps it at index 0, and that removing `b` leaves `[a, c]`. Child order drives hierarchy-panel row order and scene-file entity order; today a `HashSet` swap passes.
3. **`ui_components.rs:251 test_anchor_all_index_label_roundtrip`** (merged into `:286`) — asserts now: `index() == position in ALL` and `!label().is_empty()`. Should assert: the actual label strings the editor's cycle row renders, since a non-empty string proves nothing a user would see.
4. **`component_registry/tests.rs:209 test_global_registry_has_builtin_components`** — asserts now: a hand-maintained list of twelve names is registered. Should additionally assert the **inverse**: that `persistent_names()` contains no name outside an expected set, so a new builtin that forgets `register_transient` (or a game type leaking into the persistent set) fails here rather than in a scene diff.
5. **`event.rs:185 test_flush_clears_all_events`** — asserts now: two types read empty after `flush`. Should assert it at the level the engine actually uses (`World::emit` → `read_events` → per-frame flush), because the `EventBus` is never touched directly by game code; the frame-boundary contract is what breaks.

---

# Notes for your integration half

- I did **not** propose any inline keep for: position propagation through the hierarchy system (yours: `tests/hierarchy_dirty.rs`), named-clip playback (yours: `tests/sprite_components.rs`), stale-id rejection and hierarchy cleanup on removal (yours: `tests/world.rs`), cycle/self-parent rejection (yours: `tests/world.rs:108/:130`), or system lifecycle. Where the inline file duplicated one of those, I marked the inline copy for deletion and named your integration test as the survivor.
- `crates/ecs/src/hierarchy_system.rs:401` (scaled parent) is the one propagation case your `hierarchy_dirty.rs` keeps do **not** cover — nothing in that file sets a scale. Keep it inline or add a scale row to a `hierarchy_dirty` test; don't drop both.
- Dead API surfaced by the audit: `GlobalTransform2D::transform_point` (`crates/ecs/src/hierarchy.rs:204`) has no production caller anywhere in the workspace — its only reference is the test I'm folding away. Same for `EventBus::type_count`. Both are candidates for deletion with their tests.