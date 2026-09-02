# Test-suite keep-list — insiculous_2d

All 1,657 `#[test]` functions in `crates/*` were read in full. This report selects the tests
worth keeping; **everything not named here is deleted.** Target reached: 312 keeps, 19% of
the current suite.

Two kinds of test earn a place:

- **CONTRACT** — a player- or author-visible contract. Lifecycle and state machines,
  cross-component wiring, persistence and legacy formats, non-obvious math, typed error
  paths, GPU layout sizes, derive-macro output, undo/redo and dirty semantics, scene
  round-trip, input edge semantics.
- **GUARD** — catches a known footgun or antipattern being reintroduced. Sources: the root
  `CLAUDE.md` § "Known Footguns", each crate guide's pitfalls and contracts, and the
  documented decisions of record.

`MISSING` means nothing locks that contract today. Those rows are the highest-value writes
on the list.

**Every `file:line` below was re-derived from the tree**, not carried over from the earlier
delete-list pass. That mattered: the line numbers reported for
`editor_integration/src/editor_game/camera_follow_tests.rs` in the first round were wrong by
roughly 300 lines. Nothing here is cited from memory.

---

## 1. Per-crate table

| crate | current | keep | keep % |
|---|---:|---:|---:|
| common | 41 | 16 | 39% |
| ecs (inline `src/`) | 119 | 19 | 16% |
| ecs (`tests/`) | 94 | 21 | 22% |
| ecs_macros | 3 | 1 | 33% |
| editor | 477 | 62 | 13% |
| editor_integration | 150 | 30 | 20% |
| engine_core | 394 | 78 | 20% |
| ui | 123 | 24 | 20% |
| renderer | 92 | 17 | 18% |
| physics | 64 | 20 | 31% |
| input | 74 | 15 | 20% |
| audio | 26 | 9 | 35% |
| **total** | **1657** | **312** | **19%** |

**Six files empty out entirely** and go with their tests: `ecs/tests/component.rs`,
`ecs/tests/init.rs`, `ecs/tests/system.rs`, `engine_core/tests/init.rs`,
`input/tests/input_handler.rs`, and `ui/tests/ui_interaction_debug.rs` — the last only after
five of its seven tests move inline (§9).

---

## 2. common — 41 → 16

The shared-math crate. Its keeps are the conventions every other crate silently assumes.

| kind | contract | test |
|---|---|---|
| GUARD | screen +Y is down, world +Y is up | `common/src/camera.rs:248 test_screen_y_down_maps_to_world_y_up` |
| CONTRACT | screen↔world round-trips | `common/src/camera.rs:237 test_world_to_screen_round_trips_screen_to_world` |
| GUARD | the matrix is T·R·S, never T·S·R | `common/src/transform.rs:223 test_matrix_applies_scale_before_rotation_before_translation` |
| CONTRACT | inverse_transform_point round-trips a translated, rotated point | `common/src/transform.rs:192 test_inverse_transform_point_round_trips_translated_rotated_point` |
| CONTRACT | transform_direction is the linear part under non-uniform scale | `common/src/transform.rs:239 test_transform_direction_agrees_with_matrix_under_nonuniform_scale` |
| CONTRACT | sRGB relative luminance (feeds every editor WCAG guard) | `common/src/color.rs:264 test_known_srgb_luminance` |
| CONTRACT | white/black contrast is 21:1 | `common/src/color.rs:250 test_white_black_contrast_is_21` |
| CONTRACT | hex → Color (feeds the `#solid:RRGGBB` scene ref) | `common/src/color.rs:219 test_color_from_hex` |
| GUARD | from_cell_size truncates partial trailing cells; UVs stay pixel-exact | `common/src/sheet_grid.rs:180 test_from_cell_size_truncates_partial_trailing_cells` |
| GUARD | from_uv_size keeps a non-reciprocal cell size (1.0/3 would shift every region) | `common/src/sheet_grid.rs:225 test_from_uv_size_preserves_non_reciprocal_cell_size` |
| CONTRACT | uv_rect maps an index to the row-major cell | `common/src/sheet_grid.rs:160 test_uv_rect_maps_index_to_row_major_cell` |
| CONTRACT | uv_rect_checked is None past the cell count | `common/src/sheet_grid.rs:209 test_uv_rect_checked_is_none_past_cell_count` |
| GUARD | degenerate grids stay usable and never divide by zero | `common/src/sheet_grid.rs:171 test_new_clamps_zero_dimensions` |
| CONTRACT | web boot keys resolve through base-joined reads | `common/src/vfs.rs:148 test_boot_phase_keys_resolve_through_base_joined_reads` |
| CONTRACT | list_dir_files is sorted, extension-filtered, direct children only | `common/src/vfs.rs:163 test_list_dir_files_finds_locales_under_production_like_keys` |
| CONTRACT | `with_fields!` generates working builders (macro output) | `common/src/macros.rs:68 test_with_fields_macro` |

**MERGE-INTO** — `camera.rs:227`→`:237`; `transform.rs:204`→`:239`; `color.rs:211`→`:219`,
`color.rs:256`→`:250`; `sheet_grid.rs:193`,`:236`→`:171`, `:200`→`:209`;
`vfs.rs:181`,`:191`→`:163`; `hash.rs:31`→`:21`.

**WEAK KEEPS** — `color.rs:219` asserts two of four channels; assert all four, since a
dropped alpha corrupts every `#solid:RRGGBB` round-trip.

**Deliberately not kept.** `Rect::contains`, `Rect::intersects`, `Rect::intersection`,
`Camera::world_bounds` and `Camera::contains_point` all have **zero production callers** —
the editor uses its own `AABB`. `common::Time` is re-exported by two crates and called by
none. Delete the APIs, not just the tests. `hash.rs:21` is kept as a keep only because the
games call `hash_f32`/`hash_u32` directly.

---

## 3. ecs — inline `src/` 119 → 19

| kind | contract | test |
|---|---|---|
| CONTRACT | transition updates current and previous | `ecs/src/state_machine.rs:284 test_transition_updates_current_and_previous` |
| CONTRACT | a same-state transition is a no-op | `ecs/src/state_machine.rs:297 test_same_state_transition_is_noop` |
| CONTRACT | tick clears just_entered and accumulates elapsed | `ecs/src/state_machine.rs:320 test_tick_clears_just_entered_and_accumulates_time` |
| CONTRACT | hierarchical transition within a group | `ecs/src/state_machine.rs:400 test_hierarchical_transition_within_group` |
| CONTRACT | hierarchical transition across groups tracks previous_parent | `ecs/src/state_machine.rs:413 test_hierarchical_transition_across_groups` |
| CONTRACT | emit → read within a frame | `ecs/src/event.rs:167 test_emit_and_read_events` |
| CONTRACT | flush clears every type's queue | `ecs/src/event.rs:186 test_flush_clears_all_events` |
| CONTRACT | events stay readable until flush | `ecs/src/event.rs:274 test_events_readable_multiple_times_before_flush` |
| CONTRACT | resource insert/get/replace | `ecs/src/resource.rs:143 test_insert_replaces_previous` |
| CONTRACT | ancestors are ordered nearest-first | `ecs/src/hierarchy_extension.rs:362 test_get_ancestors` |
| CONTRACT | reparenting prunes the old parent's child list | `ecs/src/hierarchy_extension.rs:429 test_reparent_entity` |
| CONTRACT | a scaled parent scales its child's global transform | `ecs/src/hierarchy_system.rs:402 test_scaled_parent_transform_propagation` |
| CONTRACT | GlobalTransform composition under rotation | `ecs/src/hierarchy.rs:296 test_global_transform_mul_with_rotation` |
| CONTRACT | tilemap emits instances only for non-zero tiles, row zero on top | `ecs/src/tilemap.rs:152 test_sprite_instances_offsets_row_zero_on_top` |
| GUARD | a short `tiles` vec truncates instead of panicking | `ecs/src/tilemap.rs:168 test_short_tiles_vec_is_truncated_not_a_panic` |
| CONTRACT | an entity despawns when its lifetime crosses zero | `ecs/src/lifetime.rs:78 test_entity_despawns_when_lifetime_crosses_zero` |
| CONTRACT | the animation system writes `Sprite.tex_region` (cross-component wiring) | `ecs/src/sprite_system.rs:60 test_system_writes_current_frame_region_to_sprite` |
| GUARD | zero delta freezes the frame — how pause freezes animations | `ecs/src/sprite_system.rs:87 test_system_with_zero_delta_freezes_the_frame` |
| GUARD | `CameraFollow` parses the legacy four-field form | `ecs/src/behavior.rs:553 test_camera_follow_parses_legacy_four_field_form` |

Plus these registry rows, which are the dynamic-component tier's whole contract — counted in
the 19 above by folding `state_machine` down to three:

| kind | contract | test |
|---|---|---|
| CONTRACT | insert / extract / remove by name on a real world | `ecs/src/component_registry/tests.rs:75 test_insert_extract_remove_round_trip_on_a_world` |
| GUARD | persistent_names is sorted so scene diffs stay stable | `ecs/src/component_registry/tests.rs:178 test_persistent_names_are_sorted_for_stable_scene_diffs` |
| GUARD | transient types never reach `persistent_names` | `ecs/src/component_registry/tests.rs:165 test_transient_types_are_excluded_from_persistent_names` |
| GUARD | same name, different type panics with a clear message | `ecs/src/component_registry/tests.rs:148 test_same_name_different_type_registration_panics` |
| CONTRACT | the builtin roster is complete | `ecs/src/component_registry/tests.rs:210 test_global_registry_has_builtin_components` |
| CONTRACT | `Scripts` serde covers every param variant | `ecs/src/script.rs:124 test_scripts_serde_round_trips_every_value_variant` |
| CONTRACT | UiAnchor resolves anchored positions | `ecs/src/ui_components.rs:260 test_resolve_anchored_pos_matrix` |
| CONTRACT | spatial audio attenuation math | `ecs/src/audio_components.rs:246 test_audio_source_attenuation` |

*(Final count for this section: 19 keeps after the merges below.)*

**MERGE-INTO** — `state_machine.rs:275`,`:308`,`:333`,`:350`,`:361` → `:284`;
`:390`,`:441`,`:448`,`:456` → `:400`. `event.rs:179`,`:198`,`:219` → `:167`.
`resource.rs:123`,`:132`,`:155`,`:165`,`:180`,`:202` → `:143`.
`tilemap.rs:132`,`:142`,`:162`,`:178`,`:189` → `:152`.
`behavior.rs:385`,`:413`,`:443`,`:449`,`:475`,`:495`,`:529` → `:553` and the scene
round-trip in engine_core. `ui_components.rs:239`,`:287`,`:313` → `:260`.
`component_registry/tests.rs:32`,`:38`,`:51`,`:66`,`:107`,`:122`,`:139`,`:189`,`:242`,`:304`
→ `:75` and `:210`.

**WEAK KEEPS** — `ecs/src/behavior.rs:553` is the only legacy-format guard in the crate;
add an assertion that the missing `look_ahead` field lands on its documented default rather
than merely parsing.

**MISSING in ecs.**
1. `World::emit` / `read_events` / the per-frame flush (`world.rs:505`) has no test at any
   level. The documented "drain collision events once per frame" footgun rides on it.
2. `Children` is a `Vec` because child order is load-bearing for the hierarchy panel and
   scene serialization, and the guide forbids swapping it for a `HashSet`. Nothing asserts
   `get_children` preserves insertion order across add/remove/re-add.
3. `GlobalTransform2D` is system-owned; manual writes are silently overwritten. Untested.
4. Every registered component must survive **both** serde_json (inspector) and RON (scene
   files). No test walks the registry asserting both.
5. `Box<dyn Component>` downcasts must go through `.as_ref().as_any()`. The concrete-name
   guard exists (§4) but nothing exercises the downcast path itself.

---

## 4. ecs — `tests/` 94 → 21

The integration file is the stronger side of every duplicated pair, so it takes the keep and
the inline copy is deleted. `hierarchy_dirty.rs` asserts recompute *counts*, which is the
entire point of the dirty-flag design.

| kind | contract | test |
|---|---|---|
| CONTRACT | a clean frame recomputes nothing but still dirty-checks | `ecs/tests/hierarchy_dirty.rs:23 test_no_change_second_frame_recomputes_zero` |
| CONTRACT | a leaf change recomputes exactly one; siblings stay correct | `ecs/tests/hierarchy_dirty.rs:40 test_leaf_change_recomputes_one` |
| CONTRACT | a parent change recomputes its subtree and nothing else | `ecs/tests/hierarchy_dirty.rs:61 test_parent_change_recomputes_subtree_only` |
| CONTRACT | deleting a parent orphans children and prunes the cache | `ecs/tests/hierarchy_dirty.rs:107 test_parent_deletion_orphans_recompute_and_cache_prunes` |
| GUARD | writing identical values must not dirty the entity — the sleeping-body physics writeback | `ecs/tests/hierarchy_dirty.rs:131 test_identical_write_stays_clean` |
| GUARD | a disabled system leaves globals stale; re-enabling detects the drift | `ecs/tests/hierarchy_dirty.rs:153 test_reenable_after_disable_catches_stale` |
| CONTRACT | `ensure_playing` restarts a different or stopped clip, never a running one, and rejects unknown names | `ecs/tests/sprite_components.rs:200 test_ensure_playing_restarts_a_different_or_stopped_clip` |
| CONTRACT | a non-looping clip clamps on the last frame and stops for good | `ecs/tests/sprite_components.rs:135 test_non_looping_clip_clamps_on_the_last_frame_and_stops` |
| GUARD | switching to a shorter clip never exposes a stale frame index | `ecs/tests/sprite_components.rs:81 test_switching_to_a_shorter_clip_never_exposes_a_stale_frame` |
| CONTRACT | `current_uv` maps the frame index through the SheetGrid | `ecs/tests/sprite_components.rs:263 test_current_uv_maps_the_frame_index_through_the_grid` |
| GUARD | a broken clip (fps 0/negative/NaN, empty frames, non-finite dt) never panics or advances | `ecs/tests/sprite_components.rs:223 test_non_advancing_fps_values_never_panic_or_advance` |
| GUARD | an omitted `tex_region` deserializes to the FULL texture and an omitted `visible` to true — a plain serde default would render nothing | `ecs/tests/sprite_components.rs:340 test_sprite_deserializes_omitted_region_and_visibility_to_full_and_visible` |
| CONTRACT | ComponentMeta field ORDER, which is the order the inspector renders | `ecs/tests/sprite_components.rs:301 test_sprite_animation_component_meta` |
| CONTRACT | pause holds the frame; resume continues from it | `ecs/tests/sprite_components.rs:153 test_pause_holds_the_frame_and_resume_continues_from_it` |
| CONTRACT | every component op refuses a stale entity id | `ecs/tests/world.rs:313 test_stale_entity_id_rejected_by_component_ops` |
| CONTRACT | clear + create_entity_with_id revives an id — the WorldSnapshot restore contract | `ecs/tests/world.rs:332 test_snapshot_restore_revives_entity_id` |
| CONTRACT | removing a parent orphans children to root with no dangling Parent | `ecs/tests/world.rs:369 test_remove_parent_entity_orphans_children_to_root` |
| CONTRACT | remove_entity_hierarchy leaves no residue, 100 deep | `ecs/tests/world.rs:405 test_remove_entity_hierarchy_deep_chain_leaves_no_residue` |
| GUARD | `component_types` reports the CONCRETE component name, never the Box's — the blanket-`Any` footgun | `ecs/tests/world.rs:446 test_component_types_reports_concrete_type_names` |
| GUARD | reparenting rejects cycles, and the error names the cycle | `ecs/tests/world.rs:108 test_hierarchy_cycle_detection` |
| CONTRACT | typed queries: Single and Pair select exactly the right entities | `ecs/tests/world.rs:144 test_query_entities` |
| CONTRACT | the world FSM: initialize → start → update → stop → shutdown | `ecs/tests/system_lifecycle.rs:343 test_world_lifecycle_integration` |
| CONTRACT | a system added after start receives its missed hooks against the real world | `ecs/tests/system_lifecycle.rs:207 test_late_added_system_gets_missed_hooks` |
| GUARD | one panicking system does not take down the registry | `ecs/tests/system_lifecycle.rs:306 test_panic_recovery_in_systems` |

**MERGE-INTO** — `sprite_components.rs:15`,`:30`,`:41`,`:54`,`:68`,`:170`,`:186`→`:200`;
`:97`,`:111`,`:124`→`:135`; `:239`,`:251`→`:223`; `:275`→`:263`;
`:314`,`:323`,`:332`→`:301` as one table over the four builtin types.
`world.rs:4`,`:13`,`:30`,`:184`,`:203`,`:216`→`:332`; `:353`,`:387`→`:369`; `:426`→`:405`;
`:130`→`:108`; `:470`,`:480`→`:446`; the five spawn tests collapse into `:144`.
`hierarchy_dirty.rs:89`,`:172`→`:61`; `:186`→`:23`. `system_lifecycle.rs:170`→`:207`.

**WEAK KEEPS.**
- `system_lifecycle.rs:306` has **no assert**, and it stays anyway. `SystemRegistry::update_all`
  really does use `catch_unwind` (`ecs/src/system.rs:213`), so the contract is real. Add an
  assert that a normal system added after the panicking one still advances its update count.
  This reverses the first-round read that it was assert-free bloat.
- `world.rs:144` becomes the only `add_system` coverage once `tests/system.rs` is deleted,
  and that half asserts a count only. Assert the system actually ran, via a resource marker,
  the way `:207` does.

---

## 5. ecs_macros — 3 → 1

| kind | contract | test |
|---|---|---|
| CONTRACT | the derive emits type_name and field_names in declaration order | `ecs_macros/tests/derive_test.rs:40 test_field_names_order_preserved` |

`:26` and `:31` merge in; `:31` is a strictly weaker form of `:40` (len + contains versus the
exact array).

---

## 6. editor — 477 → 62

### Undo, redo, dirty (10)

| kind | contract | test |
|---|---|---|
| CONTRACT | execute → undo → redo ordering | `editor/src/commands/tests.rs:19 test_command_history_execute_and_undo` |
| CONTRACT | the history cap drops the oldest and preserves undo order | `editor/src/commands/tests.rs:423 test_max_history_drops_oldest_and_preserves_undo_order` |
| GUARD | delete-undo resurrects the SAME entity id | `editor/src/commands/tests.rs:491 test_delete_undo_resurrects_same_entity_id` |
| GUARD | a Set command survives a delete/undo cycle | `editor/src/commands/tests.rs:527 test_set_command_survives_delete_undo_cycle` |
| CONTRACT | continuous edits merge by field_hint into one undo entry | `editor/src/commands/tests.rs:344 test_set_transform_merge` |
| CONTRACT | removing RigidBody cascades to its Collider | `editor/src/commands/tests.rs:244 test_remove_rigid_body_cascades_to_collider` |
| CONTRACT | undo back to the saved watermark reads clean | `editor/src/commands/dirty_tests.rs:78 test_undo_back_to_saved_watermark_reads_clean` |
| GUARD | save → merge → undo stays dirty (merges reassign ids) | `editor/src/commands/dirty_tests.rs:128 test_save_then_merge_then_undo_stays_dirty` |
| GUARD | a merge clears redo history | `editor/src/commands/dirty_tests.rs:147 test_merge_clears_redo_history` |
| GUARD | `break_merge` seals the gesture boundary | `editor/src/commands/dirty_tests.rs:199 test_break_merge_prevents_merge_across_gestures` |

### Clipboard and tree commands (4)

| kind | contract | test |
|---|---|---|
| CONTRACT | capture and spawn round-trip a hierarchy | `editor/src/clipboard.rs:328 test_capture_and_spawn_round_trips_a_hierarchy` |
| GUARD | SpawnTree redo resurrects the same ids (GPP-14) | `editor/src/clipboard.rs:376 test_spawn_tree_redo_resurrects_the_same_ids` |
| CONTRACT | DeleteTree removes the subtree and undo restores ids | `editor/src/clipboard.rs:427 test_delete_tree_removes_whole_subtree_and_undo_restores_ids` |
| CONTRACT | duplicate renames every spawned name | `editor/src/clipboard.rs:457 test_duplicate_suffix_renames_every_spawned_name` |

### Selection restore and rename (4)

| kind | contract | test |
|---|---|---|
| CONTRACT | undo of a delete restores the selection (#59) | `editor/src/commands/selection_restore_tests.rs:30 test_undo_delete_restores_the_selection` |
| GUARD | merged entries keep the first before-image | `editor/src/commands/selection_restore_tests.rs:109 test_merged_entries_keep_the_first_before_image` |
| GUARD | stale ids are pruned from the restore | `editor/src/commands/selection_restore_tests.rs:136 test_stale_ids_are_pruned_from_the_restore` |
| CONTRACT | rename adds Name; undo removes the component | `editor/src/commands/name_tests.rs:40 test_rename_adds_name_and_undo_removes_the_component` |

### Command API (10)

| kind | contract | test |
|---|---|---|
| GUARD | every doc example still parses (spec drift) | `editor/src/command_api/specs.rs:188 test_every_doc_example_parses` |
| GUARD | parser verbs match the docs (spec drift) | `editor/src/command_api/specs.rs:200 test_parser_verbs_match_docs` |
| CONTRACT | responses are single-line | `editor/src/command_api/tests.rs:216 test_responses_are_single_line` |
| CONTRACT | the error envelope carries kind and message | `editor/src/command_api/tests.rs:229 test_error_envelope_kind_and_message` |
| CONTRACT | an ambiguous name error carries the matches | `editor/src/command_api/tests.rs:243 test_ambiguous_name_error_carries_matches` |
| GUARD | add refuses an unissued texture handle without leaving a component | `editor/src/command_api/write_tests.rs:169 test_add_rejects_unissued_texture_handle_without_leaving_component` |
| GUARD | non-finite numbers are refused | `editor/src/command_api/write_tests.rs:140 test_set_rejects_non_finite_numbers` |
| GUARD | collider extents are sanitized to the GUI floor | `editor/src/command_api/write_tests.rs:196 test_set_sanitizes_collider_extents` |
| CONTRACT | a batch aborts by rolling back in reverse | `editor/src/command_api/write_tests.rs:340 test_batch_abort_rolls_back_in_reverse` |
| GUARD | writes are refused while playing | `editor/src/command_api/write_tests.rs:371 test_writes_refused_while_playing` |
| CONTRACT | add/set/remove reach dynamic (game-registered) components | `editor/src/command_api/write_tests.rs:482 test_add_set_remove_work_on_dynamic_components` |

### Registry, StoredComponent, snapshot (6)

| kind | contract | test |
|---|---|---|
| GUARD | registered type ids match world enumeration (registry drift) | `editor/src/stored_component/tests.rs:224 test_registered_type_ids_match_world_enumeration` |
| CONTRACT | every settable type round-trips through JSON | `editor/src/stored_component/tests.rs:294 test_stored_component_from_json_round_trips_all_settable_types` |
| CONTRACT | WorldSnapshot round-trips dynamic components | `editor/src/stored_component/dynamic_tests.rs:73 test_world_snapshot_round_trips_dynamic_components` |
| CONTRACT | snapshot restore preserves entity ids | `editor/src/world_snapshot/tests.rs:29 test_snapshot_restore_preserves_entity_ids` |
| CONTRACT | restore discards play-session changes | `editor/src/world_snapshot/tests.rs:53 test_snapshot_restore_discards_play_changes` |
| GUARD | unregistered component types are reported once (data-loss warning) | `editor/src/world_snapshot/tests.rs:217 test_snapshot_reports_unregistered_component_types_once` |

### Shortcuts, picking, gizmo math (7)

| kind | contract | test |
|---|---|---|
| GUARD | every default chord resolves to its action (binding drift) | `editor/src/editor_input.rs:412 test_every_default_chord_resolves_to_its_action` |
| CONTRACT | a rebind evicts only the exact chord (full-tuple eviction) | `editor/src/editor_input.rs:474 test_rebind_evicts_only_the_exact_chord` |
| GUARD | a flip-scaled sprite is picked at its visual bounds | `editor/src/picking/tests.rs:64 test_flip_scaled_sprite_is_picked_at_its_visual_bounds` |
| GUARD | equal-depth hits order by id deterministically | `editor/src/picking/tests.rs:124 test_equal_depth_hits_order_by_id_deterministically` |
| CONTRACT | scale factor is the per-axis offset ratio | `editor/src/gizmo/tests.rs:196 test_scale_factor_is_offset_ratio_per_axis` |
| GUARD | the Escape cancel latch suppresses the rest of the gesture until mouse-up | `editor/src/gizmo/tests.rs:220 test_cancel_latch_suppresses_rest_of_gesture_until_mouse_up` |
| GUARD | a rotation seam crossing returns a small delta, not a full turn | `editor/src/gizmo_math.rs:58 test_seam_crossing_returns_small_delta` |

### Viewport, grid, dock layout math (6)

| kind | contract | test |
|---|---|---|
| CONTRACT | viewport screen↔world round-trips | `editor/src/viewport/tests.rs:44 test_viewport_coordinate_roundtrip` |
| GUARD | the window render camera and the overlay agree (one view, two consumers) | `editor/src/viewport/tests.rs:222 test_window_render_camera_screen_roundtrip` |
| GUARD | camera convergence is frame-rate independent | `editor/src/viewport/tests.rs:119 test_update_is_frame_rate_independent` |
| CONTRACT | framing zooms to fit a single entity's extents | `editor/src/context/tests.rs:357 test_frame_selected_zooms_to_fit_single_entity_extents` |
| CONTRACT | subdivisions are zoom-gated and never land on primary lines | `editor/src/grid.rs:475 test_subdivisions_gated_by_zoom_and_never_on_primary_lines` |
| CONTRACT | panel resize clamps to min and half the dock | `editor/src/dock/tests.rs:217 test_resized_size_clamps_to_min_and_half_dock` |

### Theme — WCAG guards (3)

| kind | contract | test |
|---|---|---|
| GUARD | adjacent surfaces are distinguishable (surface_0..4 ≥1.35:1, elevation gets lighter) | `editor/src/theme/tests.rs:103 test_adjacent_surfaces_are_distinguishable` |
| GUARD | a popup reads against the panel (border ≥3:1) | `editor/src/theme/tests.rs:119 test_popup_reads_against_panel` |
| GUARD | selection outline derivation stays inside its contract | `editor/src/theme/tests.rs:67 test_selection_outline_derivation_contract` |

### Inspector, rows, fonts (5)

| kind | contract | test |
|---|---|---|
| CONTRACT | pair slots shrink on a narrow panel and cap on a wide one | `editor/src/row_layout.rs:192 test_pair_slots_shrink_on_narrow_panel_and_cap_on_wide` |
| GUARD | a pending string edit commits before a variant cycle applies | `editor/src/inspector_edit_tests.rs:168 test_pending_string_edit_commits_before_variant_cycle_applies` |
| CONTRACT | a typed value outside the soft range raises a status-bar warning (#55) | `editor/src/inspector_edit_tests.rs:206 test_typed_value_outside_soft_range_raises_a_warning` |
| CONTRACT | a collider shape cycle carries the size into the new variant | `editor/src/inspector_edit_tests.rs:143 test_collider_shape_cycle_carries_size_into_new_variant` |
| CONTRACT | numeric fields draw and measure in the mono face (#54) | `editor/src/fonts.rs:53 test_numeric_field_draws_and_measures_in_the_mono_face` |

### Hierarchy, selection, chrome (5)

| kind | contract | test |
|---|---|---|
| CONTRACT | visible order follows draw order and skips collapsed subtrees | `editor/src/hierarchy_tests.rs:365 test_visible_order_follows_draw_order_and_skips_collapsed_subtrees` |
| CONTRACT | F2 rename commits, reports the new name, and exits rename mode | `editor/src/hierarchy_tests.rs:289 test_rename_commit_reports_new_name_and_exits_mode` |
| CONTRACT | removing the primary falls back to the earliest remaining | `editor/src/selection.rs:335 test_remove_primary_falls_back_to_earliest_remaining` |
| GUARD | a toolbar button click survives chrome interact — it must not reselect the sprite underneath | `editor/src/toolbar.rs:304 test_toolbar_button_click_survives_chrome_interact` |
| GUARD | an open dropdown renders in the overlay band and blocks input beneath it | `editor/src/menu/tests.rs:266 test_open_dropdown_renders_in_overlay_band_and_blocks_input` |

### Persistence, overlays, gestures (7)

| kind | contract | test |
|---|---|---|
| GUARD | legacy prefs without a `panels` field still load | `editor/src/editor_preferences.rs:177 test_legacy_prefs_without_panels_field_still_load` |
| CONTRACT | applying a scan preserves loaded handles by path | `editor/src/asset_browser.rs:191 test_apply_scan_preserves_loaded_handles_by_path` |
| GUARD | the collider overlay ignores `Transform2D.scale`, exactly like physics | `editor/src/collider_overlay.rs:295 test_transform_scale_is_ignored_like_physics` |
| CONTRACT | capsule-Y extends half-height plus radius | `editor/src/collider_overlay.rs:261 test_capsule_y_extends_half_height_plus_radius_vertically` |
| CONTRACT | hover picks topmost by depth with a stable tiebreak | `editor/src/selection_outline.rs:310 test_hover_picks_topmost_by_depth_with_stable_tiebreak` |
| GUARD | shrinking content re-clamps the scroll offset | `editor/src/scroll.rs:124 test_shrinking_content_reclamps_offset` |
| GUARD | a release under threshold is a click, not a drag | `editor/src/drag_drop.rs:124 test_release_under_threshold_is_a_click_not_a_drag` |

**MERGE-INTO (editor).** Title-bar trio `context/tests.rs:290`,`:297`,`:305` → one table.
Name-fallback trio `hierarchy_tests.rs:168`,`:178`,`:188` → one table.
`viewport/tests.rs:22`,`:33`→`:44`; `:59`,`:71` → one zoom table;
`:189`,`:199`,`:208`→`:222`. `dock/tests.rs:80`,`:96` → one edge table.
`selection.rs:164`,`:192`,`:208`,`:223`,`:264`,`:388` → `:306`/`:335`/`:351`.
`row_layout.rs:237`,`:242`,`:251` → one ellipsize table.
`inspector.rs:272`,`:281`,`:287` → one scalar-format table.
`theme/tests.rs:18`,`:49`,`:58` → one "roles that must read apart" table.
`world_snapshot/tests.rs:96`,`:123`,`:144`,`:191` → `:29`.
`commands/tests.rs:171`,`:463` → one "delete-undo restores every captured component".

**WEAK KEEPS (editor).**
- `grid.rs:475` is fine, but the sibling `:560 test_render_overlay_emits_clipped_lines`
  asserts `lines > 2`. If that one is kept instead, assert the two axis endpoints and their
  AxisX/AxisY colors.
- `collider_overlay.rs:326` asserts `lines == 4`. Assert the four corners mapped through
  `world_to_screen` and the `colors.solid` value.
- `editor_input.rs:512` asserts `bindings.len() >= 2`. Assert the bindings are exactly Space
  and the middle mouse button.
- `context/tests.rs:162` asserts four panels exist. Assert each panel's DockPosition —
  hierarchy Left, inspector Right, scene Center, assets Bottom — since the default layout is
  the actual contract.
- `editor_preferences.rs:137` writes to a fixed `temp_dir()/test_editor_prefs.json`, which
  races across concurrent test binaries. Use `tempfile::tempdir()` as `asset_browser.rs`
  already does.

**MISSING in editor.**
1. **Merge isolation.** `commands/tests.rs:344` merges on the *same* entity. Nothing asserts
   that an edit to entity B refuses to merge into a pending edit on entity A. The gesture
   boundary half is covered by `dirty_tests.rs:199`; this half is not.
2. **`ViewportInputHandler` pan and wheel-zoom** are untested through production code — the
   only "coverage" is the test module's own reimplementation of `calculate_zoom_factor` and
   `screen_to_world_delta`, which is deleted.
3. **Toolbar shortcut hints can lie.** `EditorTool::shortcut()` and `EditorInputMapping` are
   two tables, both asserted only against hardcoded literals. A drift test would catch a
   rebind that leaves the hint stale.
4. **Menu item click → returned label**, including that a disabled item returns nothing.
5. **Add-component popup contents** — that it lists exactly `available_components` and that
   choosing one records the right command.
6. **`EditorPreferences::load` on truncated JSON.** Missing-file and legacy shape are
   covered; the realistic on-disk corruption is not.

---

## 7. editor_integration — 150 → 30

| kind | contract | test |
|---|---|---|
| CONTRACT | play → pause → resume → stop | `editor_game/tests.rs:48 test_play_pause_resume_stop_cycle` |
| CONTRACT | stop restores the world from the snapshot | `editor_game/tests.rs:73 test_stop_restores_world_state` |
| GUARD | stop resets the transform-propagation cache | `editor_game/tests.rs:95 test_stop_resets_transform_propagation_cache` |
| CONTRACT | scene save → load round-trip | `editor_game/tests.rs:220 test_save_scene_roundtrip` |
| CONTRACT | the OS title updates only on change | `editor_game/tests.rs:316 test_pending_title_update_only_on_change` |
| GUARD | render overrides the camera from the viewport — the viewport is the single source of truth | `editor_game/tests.rs:351 test_render_overrides_camera_from_viewport` |
| GUARD | a hidden scene panel writes a zero scissor | `editor_game/tests.rs:398 test_render_writes_zero_scissor_when_scene_panel_hidden` |
| GUARD | the scale tool scales collider shapes and offset — physics ignores `Transform2D.scale` | `editor_game/tests.rs:476 test_scale_collider_scales_shapes_and_offset` |
| CONTRACT | engine time is frozen while not playing | `editor_game/time_freeze_tests.rs:17 test_time_scale_is_frozen_while_not_playing` |
| GUARD | particles and animations hold still while editing | `editor_game/time_freeze_tests.rs:40 test_particles_and_animations_do_not_advance_while_editing` |
| GUARD | save is refused while playing | `editor_game/play_guard_tests.rs:27 test_save_refused_while_playing` |
| GUARD | scene replace is refused during a play session | `editor_game/play_guard_tests.rs:88 test_scene_replace_refused_during_play_session` |
| GUARD | Play warns about unregistered components (snapshot data loss) | `editor_game/play_guard_tests.rs:135 test_play_surfaces_warning_for_unregistered_components` |
| GUARD | Stop reports dropped component types | `editor_game/play_guard_tests.rs:167 test_stop_reports_dropped_component_types` |
| GUARD | resuming from pause does not recapture the snapshot | `editor_game/play_guard_tests.rs:188 test_resume_from_pause_does_not_recapture_snapshot` |
| GUARD | malformed RON preserves the live world (the scratch-World dry run) | `editor_game/scene_io_tests.rs:73 test_load_malformed_ron_preserves_world` |
| GUARD | an instantiate failure preserves the live world | `editor_game/scene_io_tests.rs:105 test_load_instantiate_failure_preserves_world` |
| CONTRACT | load publishes the physics resource and save keeps the block | `editor_game/scene_io_tests.rs:183 test_load_scene_publishes_physics_resource_and_save_keeps_the_block` |
| CONTRACT | save auto-names script targets through CommandHistory | `editor_game/scene_io_tests.rs:141 test_save_auto_names_script_targets_through_command_history` |
| CONTRACT | the viewport mirrors the game camera only while playing and following | `editor_game/camera_follow_tests.rs:60 test_sync_copies_zoom_only_while_playing_and_following` |
| GUARD | pause/resume preserves a broken follow | `editor_game/camera_follow_tests.rs:114 test_pause_resume_preserves_a_broken_follow` |
| CONTRACT | stop restores the editing view and re-arms follow | `editor_game/camera_follow_tests.rs:132 test_stop_restores_editing_view_and_rearms_follow` |
| GUARD | entering play cancels a pending viewport gesture | `editor_game/camera_follow_tests.rs:160 test_play_transition_cancels_pending_viewport_gesture` |
| CONTRACT | a gizmo commit records ONE undo entry restoring every root | `editor_game/gizmo_drag_tests.rs:83 test_commit_records_one_undo_entry_restoring_every_root` |
| CONTRACT | cancel restores the starts and pushes no undo entry | `editor_game/gizmo_drag_tests.rs:221 test_cancel_restores_starts_and_pushes_no_undo_entry` |
| GUARD | a snapped multi-drag preserves relative offsets | `editor_game/gizmo_drag_tests.rs:166 test_snapped_multi_drag_preserves_relative_offsets` |
| GUARD | a zero grid size never poisons positions with NaN | `editor_game/gizmo_drag_tests.rs:203 test_zero_grid_size_never_poisons_positions` |
| GUARD | chrome owns the mouse while a widget holds the gesture | `editor_game/picking_tests.rs:10 test_chrome_owns_mouse_while_widget_holds_the_gesture` |
| CONTRACT | picking hits a sprite at its rendered size with an offset panel (RENDER_UNIT) | `editor_game/picking_tests.rs:83 test_pick_hits_sprite_at_rendered_size_with_offset_panel` |
| CONTRACT | a held arrow merges into one undo entry, sealed on release | `editor_game/shortcuts_tests.rs:49 test_held_arrow_merges_into_one_undo_entry_sealed_on_release` |
| GUARD | nudge is suppressed during a gizmo drag | `editor_game/shortcuts_tests.rs:97 test_nudge_is_suppressed_during_a_gizmo_drag` |
| CONTRACT | `apply_component_edit` writes back and records undo | `panel_renderer/tests.rs:8 test_transform_writeback_applies_and_records_undo` |
| CONTRACT | `apply_component_edit` merges continuous edits | `panel_renderer/tests.rs:154 test_apply_component_edit_merges_continuous_edits` |
| CONTRACT | the full headless authoring loop survives a reload | `editor_game/headless/tests.rs:28 test_full_authoring_loop_survives_a_reload` |
| GUARD | an unissued texture handle never reaches the file | `editor_game/headless/tests.rs:95 test_unissued_texture_handle_is_refused_and_never_reaches_the_file` |
| CONTRACT | Play commits an open api_batch as one entry; Stop discards one opened while paused | `editor_game/api_write_tests.rs:112 test_play_start_commits_open_batch_as_one_entry` |
| CONTRACT | a dirty world parks the action for the confirm dialog | `editor_game/scene_confirm_tests.rs:41 test_dirty_world_parks_the_action_for_the_dialog` |

**MERGE-INTO** — `editor_game/tests.rs:21`,`:28` → one clamp table.
`:135`,`:149`,`:269`,`:281` → one `test_new_scene_resets_world_and_editor_state`.
`panel_renderer/tests.rs:54`,`:79`,`:104`,`:129` → one writeback table (keep `:8` as the
canonical transform case). `picking_tests.rs:115`,`:126`,`:137` → one
`test_pickables_require_both_sprite_and_global_transform`.
`play_guard_tests.rs:45` → `:27` as a `[Playing, Paused]` table.
`api_write_tests.rs:172` folds into `:112`.
`entity_ops_tests.rs:12`,`:23`,`:34` → one create test; `:43`,`:53`,`:63`,`:77` → one
archetype table — but see MISSING below before keeping any of them.

**WEAK KEEPS.**
- `editor_game/tests.rs:498 test_gizmo_scale_undo_restores_transform_and_collider_together`
  asserts a MacroCommand the *test* builds, which is
  `editor/src/commands/tests.rs:370`. Rewrite it to drive `commit_gizmo_drag` with a
  collider present. Nothing today proves commit records the collider at all — only cancel
  does, at `gizmo_drag_tests.rs:221`.
- `editor_game/api.rs:272` asserts describe output, which is editor-crate coverage. Assert
  the blank-line / one-response-per-line envelope instead, which is `answer_api_lines`'
  own contract.

**MISSING in editor_integration.**
1. **The production delete and duplicate paths have no test at all.**
   `EditorGame::delete_selected_entities` (`menu_actions.rs:151`, including the multi-select
   `MacroCommand`) and `duplicate_selected_entities` (`:172`, `SpawnTreeCommand` +
   `DUPLICATE_OFFSET` + selection-follows-the-copy) are untested. The 14 tests in
   `entity_ops_tests.rs` that look like coverage exercise `#[cfg(test)]`-gated copies in
   `entity_ops.rs:219` and `:299`. This is the largest hole in the workspace.
2. **`DeleteEntityCommand`'s child reparenting** (`editor/src/commands/entity_commands.rs:105-112`)
   is untested anywhere real. Move the two hierarchy cases from `entity_ops_tests.rs:138`
   and `:158` onto the command itself.
3. **`drain_api_requests`** (`editor_game/api.rs:185`): the `gizmo_has_priority` skip, the
   256-line per-frame cap, and the post-drain `note_selection` all have no test.
4. **A Behavior scene fixture** carrying every variant through RON into the runner.
   `engine_core/src/scene_serializer_tests.rs:280` covers `PlayerPlatformer` only.

---

## 8. engine_core — 394 → 78

The persistence crate, and the heaviest allocation on the list.

### `.sheet.ron` sidecar schema (6)

| kind | contract | test |
|---|---|---|
| CONTRACT | the golden sidecar round-trips | `engine_core/src/sheet_file.rs:176 test_golden_sheet_file_round_trips` |
| CONTRACT | omitted `filter` defaults to Nearest | `engine_core/src/sheet_file.rs:194 test_omitted_filter_defaults_to_nearest` |
| GUARD | an unknown version is rejected, naming the file | `engine_core/src/sheet_file.rs:224 test_unknown_version_is_rejected_naming_the_file` |
| GUARD | unusable fps is rejected, naming the clip | `engine_core/src/sheet_file.rs:252 test_unusable_fps_values_are_rejected_naming_the_clip` |
| GUARD | a frame index past the grid is rejected, naming the clip | `engine_core/src/sheet_file.rs:295 test_frame_index_past_the_grid_is_rejected_naming_the_clip` |
| CONTRACT | `into_parts` excludes a partial trailing cell | `engine_core/src/sheet_file.rs:283 test_into_parts_excludes_a_partial_trailing_cell` |

### Scene RON — save, load, round-trip (14)

| kind | contract | test |
|---|---|---|
| CONTRACT | full world → RON → world round-trip | `engine_core/src/scene_serializer_roundtrip_tests.rs:46 test_roundtrip_serialize_deserialize` |
| CONTRACT | GridBackdrop round-trips every field and parses bare | `engine_core/src/scene_serializer_roundtrip_tests.rs:206 test_grid_backdrop_round_trips_every_field_and_parses_bare` |
| GUARD | `GlobalTransform2D` is never serialized (computed, not authored) | `engine_core/src/scene_serializer_tests.rs:390 test_global_transform_not_serialized` |
| CONTRACT | hierarchy survives save | `engine_core/src/scene_serializer_tests.rs:334 test_hierarchy_preserved` |
| CONTRACT | RigidBody extraction, all body types | `engine_core/src/scene_serializer_tests.rs:204 test_entity_with_rigid_body` |
| CONTRACT | Collider extraction, all shapes | `engine_core/src/scene_serializer_tests.rs:244 test_entity_with_collider` |
| CONTRACT | entity ordering is stable across saves | `engine_core/src/scene_serializer_tests.rs:461 test_multiple_entities_ordering` |
| CONTRACT | a dynamic component's payload survives the RON round-trip | `engine_core/src/scene_dynamic_tests.rs:56 test_dynamic_payload_survives_ron_round_trip` |
| GUARD | transient components are never written | `engine_core/src/scene_dynamic_tests.rs:115 test_transient_components_are_not_saved` |
| GUARD | dynamic emissions are name-sorted so repeated saves diff cleanly | `engine_core/src/scene_dynamic_tests.rs:138 test_dynamic_emissions_are_name_sorted` |
| GUARD | an unknown dynamic component fails the load loudly | `engine_core/src/scene_dynamic_tests.rs:162 test_unknown_dynamic_component_fails_the_load_loudly` |
| CONTRACT | Scripts round-trip every param type | `engine_core/src/scene_scripts_tests.rs:55 test_scripts_scene_round_trip_preserves_every_param_type` |
| GUARD | an entity param naming a missing entity is dropped with a warning | `engine_core/src/scene_scripts_tests.rs:94 test_entity_param_referencing_missing_name_is_dropped_with_warning` |
| CONTRACT | save auto-names referenced unnamed targets | `engine_core/src/scene_scripts_tests.rs:151 test_save_auto_names_referenced_unnamed_targets` |

### Scene loading and legacy formats (5)

| kind | contract | test |
|---|---|---|
| GUARD | every bundled example scene still parses | `engine_core/tests/scene_loader_parse.rs:233 test_bundled_example_scenes_parse` |
| GUARD | a legacy CameraFollow scene without `look_ahead` still parses | `engine_core/tests/scene_loader_parse.rs:281 test_legacy_camera_follow_scene_without_look_ahead_still_parses` |
| GUARD | pre-editor scene files load with `editor: None` | `engine_core/src/scene_data_tests.rs:46 test_scene_data_without_editor_settings_backward_compat` |
| CONTRACT | a tilemap parses and instantiates with a resolved tileset | `engine_core/tests/scene_loader_parse.rs:183 test_tilemap_parses_and_instantiates_with_resolved_tileset` |
| CONTRACT | the override layer replaces the prefab's component of the same type | `engine_core/src/scene_loader.rs:338 test_merge_components` |

### Sprite-sheet scene integration (8)

| kind | contract | test |
|---|---|---|
| GUARD | the old SpriteAnimation format loads as an inert default | `engine_core/tests/sprite_animation_scene.rs:83 test_old_format_sprite_animation_loads_as_inert_default` |
| CONTRACT | SpriteAnimation round-trips through scene RON | `engine_core/tests/sprite_animation_scene.rs:178 test_sprite_animation_round_trips_through_scene_ron` |
| GUARD | the sidecar wins over baked scene values (re-cutting a sheet propagates) | `engine_core/tests/sprite_animation_scene.rs:259 test_sidecar_grid_and_clips_win_over_baked_scene_values` |
| GUARD | a missing sidecar falls back to the baked values | `engine_core/tests/sprite_animation_scene.rs:285 test_missing_sidecar_falls_back_to_the_baked_values` |
| GUARD | autoplay naming a clip the sidecar dropped leaves it stopped | `engine_core/tests/sprite_animation_scene.rs:309 test_autoplay_naming_a_clip_the_sidecar_dropped_leaves_it_stopped` |
| GUARD | scene load clears the sidecar cache first | `engine_core/tests/sprite_animation_scene.rs:333 test_scene_load_clears_the_sidecar_cache_first` |
| GUARD | the ClipData wire format is stable | `engine_core/tests/sprite_animation_scene.rs:347 test_clip_wire_format_is_stable` |
| CONTRACT | an animated sprite's region reaches the renderer | `engine_core/tests/sprite_animation_scene.rs:471 test_animated_sprite_region_reaches_the_renderer` |

### Save files and settings (12)

| kind | contract | test |
|---|---|---|
| CONTRACT | write → read round-trips and leaves no temp file (atomicity) | `engine_core/src/save_store.rs:175 test_write_then_read_round_trips_and_leaves_no_temp_file` |
| CONTRACT | the memory store matches native slot semantics (the wasm path) | `engine_core/src/save_store.rs:204 test_memory_store_matches_slot_semantics` |
| CONTRACT | input settings round-trip pads and bindings | `engine_core/src/input_settings_io.rs:174 round_trip_preserves_pads_and_bindings` |
| GUARD | a missing settings file writes hand-editable defaults | `engine_core/src/input_settings_io.rs:204 missing_file_returns_defaults_and_creates_hand_editable_file` |
| GUARD | a corrupt settings file falls back to defaults without panicking | `engine_core/src/input_settings_io.rs:220 corrupt_file_falls_back_to_defaults_without_panicking` |
| GUARD | a wrong-version settings file falls back to defaults | `engine_core/src/input_settings_io.rs:232 wrong_version_falls_back_to_defaults` |
| CONTRACT | achievements persist across a round-trip | `engine_core/src/achievements/tests.rs:86 persistence_round_trip` |
| GUARD | concurrent managers merge unlocks instead of clobbering | `engine_core/src/achievements/tests.rs:108 concurrent_managers_merge_unlocks_instead_of_clobbering` |
| GUARD | an unwritable save path errors without panicking | `engine_core/src/achievements/tests.rs:166 save_to_unwritable_path_errors_without_panicking` |
| CONTRACT | scores persist and rank correctly | `engine_core/src/scores.rs:254 test_persistence_round_trip` |
| GUARD | a corrupt score file warns and starts fresh | `engine_core/src/scores.rs:279 test_corrupt_file_warns_and_starts_fresh` |
| GUARD | concurrent stores merge instead of clobbering | `engine_core/src/scores.rs:289 test_concurrent_stores_merge_instead_of_clobbering` |

### Localization, texture refs, config (8)

| kind | contract | test |
|---|---|---|
| CONTRACT | `tr` falls back to English, then to the key | `engine_core/src/localization.rs:281 tr_falls_back_to_english_then_key` |
| GUARD | corrupt and wrong-version locale sources are skipped | `engine_core/src/localization.rs:308 corrupt_and_wrong_version_sources_are_skipped` |
| CONTRACT | `load_dir` reads RON files by stem | `engine_core/src/localization.rs:328 load_dir_reads_ron_files_by_stem` |
| CONTRACT | the current font follows the locale | `engine_core/src/localization.rs:361 current_font_follows_locale` |
| CONTRACT | `#solid:RRGGBB` round-trips through parse | `engine_core/src/texture_ref.rs:196 test_solid_color_path_round_trips_through_parse` |
| GUARD | generated-texture sentinels are flagged as unresolvable | `engine_core/src/texture_ref.rs:213 test_generated_texture_sentinels_are_flagged` |
| GUARD | the texture-filter wire format survives serde and accepts the lowercase alias | `engine_core/src/game_config.rs:239 test_game_config_texture_filter_survives_serde_roundtrip` |
| GUARD | a sheet that fails validation loads no texture | `engine_core/src/assets/sprite_sheet.rs:310 test_prepare_sheet_fails_before_any_texture_is_loaded` |

### Assets and sidecar cache (4)

| kind | contract | test |
|---|---|---|
| CONTRACT | clearing the cache picks up an edited sidecar | `engine_core/src/assets/sprite_sheet.rs:364 test_clearing_the_cache_picks_up_an_edited_sidecar` |
| GUARD | a malformed sidecar falls back quietly | `engine_core/src/assets/sprite_sheet.rs:390 test_cache_falls_back_quietly_on_a_malformed_sidecar` |
| GUARD | generated texture references are ignored by the cache | `engine_core/src/assets/sprite_sheet.rs:400 test_cache_ignores_generated_texture_references` |
| GUARD | RGBA validation names the expected byte count | `engine_core/src/assets.rs:505 test_rgba_validation_rejects_length_mismatch` |

### Runtime systems (21)

| kind | contract | test |
|---|---|---|
| CONTRACT | time_scale is zero only while paused | `engine_core/src/pause.rs:402 time_scale_is_zero_only_while_paused` |
| CONTRACT | Menu pauses and the same button resumes | `engine_core/src/pause.rs:234 menu_press_pauses_and_same_button_resumes` |
| CONTRACT | clicking a row executes it; hover moves the highlight | `engine_core/src/pause.rs:315 click_on_a_row_executes_it_and_hover_moves_the_highlight` |
| GUARD | a resting cursor does not hover but still clicks | `engine_core/src/menu_panel.rs:372 resting_cursor_does_not_hover_but_still_clicks` |
| CONTRACT | `row_at` round-trips every row center and rejects the bands | `engine_core/src/menu_panel.rs:343 row_at_round_trips_every_row_center_and_rejects_the_bands` |
| CONTRACT | navigation wraps in both directions | `engine_core/src/menu_input.rs:110 test_navigate_wraps_both_directions` |
| GUARD | a held stick scrolls once, not every frame | `engine_core/src/menu_input.rs:166 test_held_stick_scrolls_once_not_every_frame` |
| CONTRACT | particles decay to death and dead slots are reused | `engine_core/src/particles/manager.rs:233 spawn_reuses_dead_slots` |
| CONTRACT | direction spread stays within the cone | `engine_core/src/particles/manager.rs:278 direction_spread_stays_within_cone` |
| CONTRACT | an inactive emitter emits nothing | `engine_core/src/particles/system.rs:81 inactive_emitter_emits_nothing` |
| CONTRACT | grid springs return to rest and energy decays with damping | `engine_core/src/grid/grid_mesh.rs:395 energy_decays_with_damping` |
| GUARD | border nodes are pinned; interior nodes are free | `engine_core/src/grid/topology.rs:240 hex_border_nodes_are_pinned_and_interior_free` |
| GUARD | a hidden grid still simulates | `engine_core/src/grid/grid_mesh.rs:471 hidden_grid_still_simulates` |
| GUARD | non-finite tunables fall back to the preset | `engine_core/src/grid/build.rs:90 test_non_finite_tunables_fall_back_to_the_preset_and_compare_equal` |
| CONTRACT | a resting grid is more transparent than a moving one | `engine_core/src/grid/opacity_tests.rs:38 resting_grid_is_more_transparent_than_moving_grid` |
| GUARD | moving the entity translates the mesh without a rebuild | `engine_core/src/grid/backdrop_system.rs:251 test_moving_the_entity_translates_the_mesh_without_a_rebuild` |
| GUARD | a NaN tunable does not rebuild every frame | `engine_core/src/grid/backdrop_system.rs:182 test_shape_change_rebuilds_but_a_nan_tunable_does_not_rebuild_every_frame` |
| CONTRACT | the camera converges within 10 frames at lerp 0.5 | `engine_core/tests/camera_follow.rs:153 test_camera_converges_within_10_frames_at_lerp_half` |
| CONTRACT | the dead zone ignores targets inside the box | `engine_core/tests/camera_follow.rs:193 test_dead_zone_ignores_targets_inside_the_box` |
| CONTRACT | holding a direction leads the camera by look_ahead | `engine_core/tests/camera_follow.rs:271 test_holding_right_leads_the_camera_by_look_ahead_x` |
| GUARD | negative and NaN look_ahead degrade to plain follow | `engine_core/tests/camera_follow.rs:404 test_negative_and_nan_look_ahead_degrade_to_plain_follow` |

Plus these, completing the 78:

| kind | contract | test |
|---|---|---|
| CONTRACT | patrol arrival enters waiting then reverses | `engine_core/src/behavior_runner/mod.rs:375 test_patrol_arrival_enters_waiting_then_reverses_direction` |
| CONTRACT | chase enters and leaves the chasing phase on range | `engine_core/src/behavior_runner/mod.rs:427 test_chase_enters_and_leaves_chasing_phase_on_range` |
| CONTRACT | jump fires from a gamepad action and from Space | `engine_core/tests/behavior_optimization.rs:115 test_platformer_jump_fires_from_gamepad_action_and_from_space` |
| GUARD | device loss is fatal immediately, regardless of streak | `engine_core/src/render_manager.rs:477 classify_device_lost_is_fatal_immediately_regardless_of_streak` |
| GUARD | a fatal RenderManager refuses to render | `engine_core/src/render_manager.rs:498 fatal_render_manager_refuses_to_render` |
| GUARD | the surface-error streak resets on a successful frame | `engine_core/src/render_manager.rs:520 surface_error_streak_resets_on_successful_frame` |
| CONTRACT | main-camera sync copies position only | `engine_core/src/render_manager.rs:555 test_sync_main_camera_copies_main_camera_entity_position` |
| GUARD | delta time is clamped after a stall | `engine_core/src/game_loop_manager.rs:154 test_delta_time_is_clamped_after_a_stall` |
| CONTRACT | throttle enforces the target FPS | `engine_core/src/game_loop_manager.rs:169 test_throttle_enforces_target_fps` |
| CONTRACT | a pickup is collected once even with two collectors | `engine_core/src/pickups.rs:245 test_pickup_collected_once_even_with_two_collectors` |
| CONTRACT | EffectTimer fires exactly when crossing zero | `engine_core/src/pickups.rs:320 test_effect_timer_lifecycle` |
| GUARD | `UiElementsHidden` suppresses everything | `engine_core/src/ui_element_system.rs:146 hidden_resource_suppresses_everything` |
| CONTRACT | panels draw before buttons and labels | `engine_core/src/ui_element_system.rs:172 panels_draw_before_buttons_and_labels` |
| CONTRACT | a button click returns a press event | `engine_core/src/ui_element_system.rs:206 button_click_returns_press_event` |
| GUARD | UI stays at its screen position under a moved, zoomed camera | `engine_core/src/ui_integration/tests.rs:25 test_ui_stays_at_screen_position_under_moved_zoomed_camera` |
| CONTRACT | nested clips intersect on the batch | `engine_core/src/ui_integration/tests.rs:225 test_nested_clips_intersect_on_the_batch` |
| GUARD | pop restores the parent clip for later commands | `engine_core/src/ui_integration/tests.rs:250 test_pop_restores_parent_clip_for_later_commands` |
| CONTRACT | the gamepad button translation table is exhaustive | `engine_core/src/gamepad_backend.rs:234 button_translation_table_is_exhaustive_and_correct` |
| CONTRACT | the dead zone zeroes small values and rescales the rest | `engine_core/src/gamepad_backend.rs:282 dead_zone_zeroes_small_values_and_rescales_the_rest` |
| GUARD | hat transitions press and release only on crossings | `engine_core/src/gamepad_backend.rs:297 hat_transitions_press_and_release_only_on_crossings` |
| CONTRACT | the same glyph in different fonts needs separate textures | `engine_core/src/glyph_texture_cache.rs:184 same_glyph_same_size_different_fonts_needs_separate_textures` |
| CONTRACT | a tilemap expands into one batch with correct instances | `engine_core/src/tilemap_render.rs:61 test_tilemap_expands_into_one_batch_with_correct_instances` |
| CONTRACT | spawning a prefab applies overrides | `engine_core/tests/prefab_spawning.rs:92 test_spawn_prefab_applies_overrides` |
| GUARD | a failed prefab spawn removes the half-built entity | `engine_core/tests/prefab_spawning.rs:125 test_spawn_prefab_failure_removes_half_built_entity` |
| CONTRACT | the lifecycle FSM refuses invalid transitions | `engine_core/tests/lifecycle.rs:67 test_lifecycle_state_transitions` |
| GUARD | the lifecycle survives lock poisoning | `engine_core/src/lifecycle.rs:306 test_lifecycle_survives_lock_poisoning` |
| CONTRACT | the background covers the window with overscan, behind everything | `engine_core/src/spawn_helpers.rs:33 test_background_covers_window_with_overscan_behind_everything` |

**MERGE-INTO (engine_core).** `sheet_file.rs:206`→`:194`; `:216`→`:239` in `game_config.rs`;
`:234`,`:242`→`:224` as an error table; `:268`→`:283`.
`assets.rs:453`,`:471`,`:478` → one config table; `:493`,`:513`→`:505`.
`texture_ref.rs:155`,`:167`,`:173`,`:179` → one hex table; `:184`,`:190`→`:196`.
`menu_input.rs:116`,`:121`,`:126` → `:110`.
`chaos_mode.rs:76`,`:84`,`:92` → one triple table.
`game_config.rs:247`,`:254`→`:239`.
`debug.rs:198`,`:214` → one circle test.
`scene_serializer_tests.rs:483`→`:244`; `:507`→`:204`.
`scene_data_tests.rs:23` folds into `scene_serializer_roundtrip_tests.rs:46`; the file then
disappears. `tests/scene_loader_parse.rs:60`,`:82`,`:104`,`:131` → one sprite-defaults table.
`tests/timing.rs:85`,`:118` → one accessor test — but see the dead-API note below.
`tests/scene_lifecycle.rs:115`,`:239` → one invalid-transition test.

**WEAK KEEPS (engine_core).**
- `grid/grid_mesh.rs:430` asserts `verts.len() == spring_count*2`. Assert each pair's
  positions equal that spring's two node positions, and that at rest every alpha equals
  `color.w * rest_alpha_fraction`.
- `grid/mod.rs:76` asserts `!lines.is_empty()`. Assert `lines.len() == spring_count*2`.
- `debug.rs:205` computes its expected vertex count by reimplementing the production
  formula. Assert the two straight sides sit at `x = ±radius` over the correct half-height.
- `particles/manager.rs:244` asserts `alive_count == 4`. Assert the four survivors are the
  *last four spawned*, which is the name's actual claim.
- `game_loop_manager.rs:129` asserts `dt < 0.02` after a 10 ms sleep, which is fragile on a
  loaded CI box. Assert `dt >= 0.010 && dt <= MAX_DELTA_TIME`.
- `tests/behavior_optimization.rs:75` asserts both entities still have Behavior components.
  Assert the follower actually moved toward the player and stopped at `follow_distance`.
- `tests/lifecycle.rs:125` spawns a thread and an mpsc channel that never touch the
  lifecycle. Drop the fixture; add a real waiter released by another thread's transition.
- `tests/scene_lifecycle.rs:7` carries a stray `println!` debug block at lines 19-26.

**MISSING in engine_core.**
1. **Scene-serializer table drift — the highest-value guard in this crate.**
   `append_dynamic_components` (`scene_serializer.rs:294`) holds a hand-maintained 16-name
   `CONCRETE_OR_EXCLUDED` list that must stay in sync with the `ComponentData` enum and the
   registry's `persistent_names()`. Two silent failures: a component with a concrete variant
   but missing from the list serializes **twice**; a component wrongly in the list **never
   persists**. `scene_serializer_tests.rs:390` guards exactly one of the sixteen entries.
   Write: save a world holding the registry default of every persistent type, assert each
   name appears exactly once and no `Dynamic` row duplicates a concrete one.
2. **`main_camera_pose`'s zoom sanitizer** (`render_manager.rs:426-445`) replaces a
   non-finite or ≤0 authored zoom with 1.0, so a `zoom: 0.0` in a hand-written scene never
   divides the projection by zero. No test passes a bad zoom.
3. **`AssetManager::set_base_path` cache invalidation** (`assets.rs:421`) — a stale hit
   silently loads the wrong art after a project switch in the editor.
4. **`PauseMenu::draw_labeled` with localized labels.** `PauseMenuLabels` exists only for
   localization, and after the defaults test is deleted it has zero coverage. A wired-wrong
   `labels.items` would ship silently in Pong's pirate locale.
5. **`GridMesh::translate` directly** — nothing asserts that `rest` and `position` both
   shift while `velocity` is untouched.
6. **`#rgba` sentinel end-to-end** — create → `texture_path()` → scene save → resolve, the
   documented "does not survive scene save/load" contract.
7. **`Strings::load_dir` with one corrupt file among good ones** — the half-saved
   translator-file case the module docs promise survives.

---

## 9. ui — 123 → 24

| kind | contract | test |
|---|---|---|
| CONTRACT | a button returns true on the release frame | `ui/tests/ui_interaction_debug.rs:12 test_ui_button_click_detection` — **move to `src/context/tests.rs`** |
| CONTRACT | the slider maps a click to a value — the crate's only slider test | `ui/tests/ui_interaction_debug.rs:74 test_ui_slider_interaction` — **move inline** |
| CONTRACT | the click state machine reaches `clicked` (the only positive assertion in the crate) | `ui/tests/ui_interaction_debug.rs:185 test_interaction_manager_click_logic` — **move to `src/interaction/tests.rs`** |
| CONTRACT | a click outside the button does not fire | `ui/tests/ui_interaction_debug.rs:278 test_click_outside_button` — **move inline** |
| GUARD | press inside, release outside cancels the click | `ui/tests/ui_interaction_debug.rs:301 test_click_press_inside_release_outside` — **move inline** |
| CONTRACT | `InputState` maps the handler's mouse snapshot | `ui/tests/ui_interaction_debug.rs:140 test_input_state_from_input_handler` — **move to `src/input_state.rs`, dropping the `end_frame` third** |
| GUARD | `wants_mouse` holds from press through the release frame | `ui/src/interaction/tests.rs:125 test_wants_mouse_holds_from_widget_press_through_release_frame` |
| GUARD | a missed release event frees the gesture | `ui/src/interaction/tests.rs:172 test_missed_release_event_frees_the_mouse_gesture` |
| GUARD | a blocking rect makes an outside widget inert | `ui/src/interaction/tests.rs:49 test_blocking_rect_makes_outside_widget_inert` |
| GUARD | an overlay-scope widget stays interactive over a blocking rect | `ui/src/interaction/tests.rs:68 test_overlay_scope_widget_stays_interactive_over_blocking_rect` |
| CONTRACT | a blocked widget's persistent state survives the frame | `ui/src/interaction/tests.rs:97 test_blocked_widget_persistent_state_survives_frame` |
| CONTRACT | unseen widget state is garbage-collected, focused state is not | `ui/src/interaction/tests.rs:228 test_focused_widget_state_survives_unseen_frame` |
| GUARD | an elevated layer escapes a Content-layer clip pair (the z-band contract) | `ui/src/draw/tests.rs:233 test_elevated_layer_escapes_content_clip_pair` |
| CONTRACT | layers flush in enum order | `ui/src/draw/tests.rs:192 test_layers_flush_in_enum_order` |
| CONTRACT | layer depths are banded | `ui/src/draw/tests.rs:212 test_layer_depths_are_banded` |
| CONTRACT | push/pop layer nesting | `ui/src/draw/tests.rs:259 test_push_pop_layer_nest` |
| CONTRACT | flush is idempotent and clear resets the stack | `ui/src/draw/tests.rs:274 test_flush_is_idempotent_and_clear_resets_stack` |
| CONTRACT | typing replaces the selection | `ui/src/text_edit.rs:218 test_typing_replaces_selection` |
| CONTRACT | shift+arrow extends the selection; a plain arrow collapses it to the edge | `ui/src/text_edit.rs:284 test_plain_arrow_collapses_selection_to_edge` |
| CONTRACT | `cursor_from_click` picks the nearest boundary | `ui/src/text_edit.rs:337 test_cursor_from_click_picks_nearest_boundary` |
| GUARD | empty-string operations are safe | `ui/src/text_edit.rs:352 test_empty_string_operations_are_safe` |
| GUARD | a typed commit beyond the soft range is NOT clamped but IS flagged | `ui/src/context/scrub_tests.rs:267 test_float_scrub_typed_commit_beyond_soft_range_not_clamped` |
| GUARD | an invalid buffer flags red and reverts on commit | `ui/src/context/scrub_tests.rs:163 test_float_invalid_buffer_flags_and_reverts_on_commit` |
| GUARD | Escape restores the value a scrub started from | `ui/src/context/scrub_tests.rs:98 test_float_scrub_escape_restores_start_value` |
| GUARD | a scrub requires the click threshold; a sub-threshold press still focuses | `ui/src/context/scrub_tests.rs:50 test_float_scrub_requires_threshold_click_still_focuses` |
| CONTRACT | repeat fires after the delay, then at the interval | `ui/src/input_state.rs:344 test_repeat_fires_after_delay_then_at_interval` |
| CONTRACT | `keycode_to_char` maps letters, digits and space, and shift blocks the top row | `ui/src/input_state.rs:313 test_keycode_to_char_shift_blocks_top_row` |
| CONTRACT | programmatic focus arms the edit without a click (F2 rename) | `ui/src/context/focus_tests.rs:19 test_focus_text_input_arms_edit_without_a_click` |
| GUARD | the glyph cache evicts when full | `ui/src/font/glyph_cache.rs:178 test_glyph_cache_evicts_when_full` |

**MERGE-INTO (ui).** `interaction/tests.rs:4`,`:14`,`:24` → one WidgetId test;
`:188`,`:246` → one focus test. `draw/tests.rs:8`,`:24` → one Rect test.
`context/tests.rs:24`,`:92`,`:106` → one label table; `:252`,`:260`,`:268` → one
measure_text table; `:232`,`:242` → one centering test; `:162`,`:275` → one alignment table.
`input_state.rs:297`,`:305`,`:320`,`:330` → `:313` as one `(KeyCode, shift) -> Option<char>`
table. `scrub_tests.rs:286`→`:267`. `glyph_cache.rs:196`→`:178`.

**WEAK KEEPS (ui).**
- `context/tests.rs:200 test_float_input_draws_box` asserts `len() >= 3`. If kept, assert
  the two Rects carry the field's bounds and the placeholder reads `"42.00"`.
- `context/tests.rs:232`/`:242` assert `!draw_list().is_empty()` when centering is the whole
  subject. Assert `position.x == center.x - measure_text(text).x / 2.0`.
- `glyph_cache.rs:166 test_glyph_key` asserts derived `PartialEq`. The non-obvious code is
  `size_tenths: (font_size * 10.0) as u32` — assert 16.0 and 16.04 produce the SAME key
  while 16.0 and 16.1 differ.

**Verdict on `ui/tests/ui_interaction_debug.rs`.** Not a genuine integration test. Nothing
crosses a boundary the inline modules cannot reach: `ui` depends on `input` in both prod and
dev, and every type it touches is `pub`. The file name records a 2025 debugging session that
was never cleaned up. Six of its seven tests are unique and move inline (above);
`:232 test_input_timing_with_game_loop_order` is a duplicate of `:12` and is deleted. The
file then goes.

**MISSING in ui.**
1. **`font::layout` — the crate's only real text math — has zero real tests.** `layout_text`
   (baseline convention, `offset_y` sign flip, space handling, `max_descent` accumulation)
   and `measure_text` are entirely uncovered; the file's single test is a struct literal.
   This is the code behind the documented "UI text y = baseline" footgun, and DejaVu bytes
   already ship in the editor crate, so a font fixture makes it headless-testable.
2. **The clip-rect seam.** `scissor.rs` proves the math and `batch.rs` proves `set_clip`
   splits batches, but nothing asserts `PushClipRect`/`PopClipRect` actually drive
   `SpriteBatcher::set_clip`.
3. **`KeyRepeat` per-key independence** — nothing asserts that holding ArrowLeft does not
   advance the Backspace slot, and `timers[key as usize]` over a hand-numbered enum is
   exactly where an off-by-one hides.
4. **Slider edge behavior** — clamping at 0.0/1.0, and dragging far outside the track.

---

## 10. renderer — 92 → 17

| kind | contract | test |
|---|---|---|
| CONTRACT | the vertex stride matches what the shader assumes | `renderer/src/sprite_data.rs:304 test_sprite_vertex_bytemuck_cast` |
| CONTRACT | the instance stride matches what the shader assumes | `renderer/src/sprite_data.rs:356 test_sprite_instance_bytemuck_cast` |
| CONTRACT | a default instance is a plain unlit quad | `renderer/src/sprite_data.rs:379 test_sprite_instance_default_shape_is_plain_quad` |
| CONTRACT | `CameraUniform` is bytemuck-safe for the GPU | `renderer/src/sprite_data.rs:540 test_camera_uniform_bytemuck` |
| CONTRACT | `Sprite::to_instance` maps every field onto the GPU instance | `renderer/src/sprite.rs:179 test_sprite_to_instance` |
| CONTRACT | batching groups by texture | `renderer/src/sprite/batch.rs:322 test_sprite_batcher_groups_by_texture` |
| CONTRACT | a clip splits a same-texture batch | `renderer/src/sprite/batch.rs:341 test_sprite_batcher_splits_same_texture_by_clip` |
| GUARD | a NaN depth sorts without panicking (`total_cmp`, never `partial_cmp().unwrap()`) | `renderer/src/sprite/batch.rs:223 test_sprite_batch_sort_handles_nan_depth_without_panicking` |
| GUARD | the sorted flag resets on add | `renderer/src/sprite/batch.rs:281 test_sprite_batch_sorted_flag_reset_on_add` |
| GUARD | identical batches skip the upload | `renderer/src/sprite/instance_cache.rs:95 test_identical_batches_skip_upload` |
| GUARD | a layout change triggers an upload even with identical bytes | `renderer/src/sprite/instance_cache.rs:121 test_layout_change_triggers_upload_even_with_same_bytes` |
| CONTRACT | quantize rounds outward to cover partial pixels | `renderer/src/scissor.rs:77 test_quantize_rounds_outward_to_cover_partial_pixels` |
| GUARD | non-finite scissor inputs yield an empty rect | `renderer/src/scissor.rs:99 test_quantize_non_finite_inputs_yield_empty` |
| GUARD | clamp trims overhang on a resize race | `renderer/src/scissor.rs:117 test_clamp_trims_overhang_on_resize_race` |
| CONTRACT | a clip intersects the default scissor | `renderer/src/scissor.rs:167 test_batch_scissor_clip_intersects_default` |
| GUARD | the device-loss latch is one-way and shared across clones | `renderer/src/device_status.rs:85 latch_clones_share_state` |
| GUARD | a same-size reconfigure is forced when asked, and skipped otherwise | `renderer/src/device_status.rs:120 resize_action_forces_reconfigure_at_same_size` |
| CONTRACT | a filter maps every sampler field | `renderer/src/texture_filter.rs:64 test_linear_filter_maps_every_sampler_filter_to_linear` |

**MERGE-INTO (renderer).** `sprite_data.rs:312`→`:304`; `:371`→`:356`;
`:418`,`:432`,`:447` → one view-matrix table (then deleted — see below).
`scissor.rs:83`,`:88`,`:94`,`:106`→`:77`; `:112`,`:124`,`:130`→`:117`; `:135`,`:147`→`:167`.
`device_status.rs:73`,`:78`,`:93`→`:85`; `:101`,`:106`,`:115`→`:120`.
`texture_filter.rs:72`,`:80`→`:64`. `bloom.rs:558`,`:564` → one 16-byte layout test.

**WEAK KEEPS (renderer).**
- `sprite_data.rs:304`/`:356` assert stride and attribute *count* only. Assert every
  attribute's `(offset, format, shader_location)` — see MISSING below.
- `sprite.rs:179` asserts position/rotation/scale/color/depth. `to_instance` also forwards
  `tex_region`, `emissive` and `shape`. Set all three to non-defaults and assert they land:
  the animation system writes `tex_region` every frame and nothing proves it reaches the GPU.
- `sprite/batch.rs:239 test_sprite_batch_sort_idempotent` passes even if the `if !self.sorted`
  guard is deleted. Mutate `instances` out of order, call `sort_by_depth()`, assert the order
  was NOT touched.
- `scissor.rs:140` asserts only `r[2] == 0 && r[3] == 0`; the origin could be anything.
  Assert the full `[u32; 4]`.
- `texture.rs:407 test_texture_handle_default` asserts `id == 0`. The real contract
  (`texture.rs:137`, `next_handle: TextureHandle::WHITE.id + 1`) is that
  `default() == WHITE` and no allocated handle can ever equal it.

**MISSING in renderer.**
1. **`offset_of!` on the GPU vertex layouts — the single highest-value guard in the
   workspace.** `SpriteVertex::desc()` and `SpriteInstance::desc()` hand-compute eleven
   offsets as `size_of::<[f32; N]>()`, and the tests assert only count and stride. Swap the
   depth (`[f32;13]`) and emissive (`[f32;14]`) offsets and everything compiles, every test
   passes, and sprites render at wrong depths. Assert all eight instance triples against
   WGSL locations 3-10 and all three vertex triples.
2. **`RenderTargets::bloom_width()/bloom_height()`** are untested; the two tests that
   pretend to cover them re-derive the arithmetic in the test body and never call the
   functions. Extract `fn bloom_dims(w, h)` and test that.
3. **`TextureHandle::WHITE` reservation** is a comment, not a test. Reset `next_handle` to 0
   and sprites silently sample white.
4. Camera math tests here (`:475`, `:488`, `:497`, `:523`) either duplicate
   `common/src/camera.rs` or reimplement the production body as the expectation. All deleted;
   `common` owns that contract.

---

## 11. physics — 64 → 20

| kind | contract | test |
|---|---|---|
| GUARD | a parented entity with a RigidBody is treated as world-space — physics ignores the hierarchy | `physics/src/physics_system/tests.rs:491 test_parented_entity_with_rigid_body_is_treated_as_world_space` |
| GUARD | a started event is delivered exactly once across zero-step updates | `physics/src/physics_system/tests.rs:281 test_started_event_is_delivered_exactly_once_across_zero_step_updates` |
| GUARD | a zero-step update emits no collision events (no stale re-delivery) | `physics/src/physics_system/tests.rs:302 test_zero_step_update_emits_no_collision_events` |
| GUARD | events from every catch-up sub-step in one update survive | `physics/src/physics_system/tests.rs:319 test_events_from_all_sub_steps_in_one_update_survive` |
| GUARD | `take_collision_events` drains the buffer — a second take returns empty | `physics/src/physics_system/tests.rs:341 test_take_collision_events_drains_the_buffer` |
| GUARD | `apply_force` lasts exactly one update | `physics/src/physics_system/tests.rs:363 test_apply_force_lasts_exactly_one_update` |
| GUARD | a force applied on a zero-step frame acts on the next stepped frame | `physics/src/physics_system/tests.rs:394 test_force_applied_on_zero_step_frame_acts_on_next_stepped_frame` |
| GUARD | `reset_body` is deferred for same-frame spawns | `physics/src/physics_system/tests.rs:78 test_reset_body_is_deferred_for_same_frame_spawns` |
| GUARD | catch-up steps are capped after a stall | `physics/src/physics_system/tests.rs:112 test_catch_up_steps_are_capped_after_a_stall` |
| CONTRACT | gravity moves a dynamic body; a static body does not move | `physics/src/physics_system/tests.rs:150 test_gravity_affects_dynamic_body` |
| GUARD | direct world removal cleans up physics state | `physics/src/physics_system/tests.rs:47 test_direct_world_removal_cleans_up_physics_state` |
| CONTRACT | clear resets physics state and allows a resync from ECS | `physics/src/physics_system/tests.rs:218 test_clear_allows_resync_from_ecs` |
| CONTRACT | an external transform edit teleports a live body, preserving velocity (GPP-09) | `physics/tests/external_edits.rs:14 test_external_transform_edit_teleports_live_body` |
| GUARD | the physics writeback is not mistaken for an external edit | `physics/tests/external_edits.rs:49 test_physics_writeback_is_not_mistaken_for_external_edit` |
| GUARD | an identical transform write pushes nothing | `physics/tests/external_edits.rs:80 test_identical_transform_write_pushes_nothing` |
| CONTRACT | a collider edit rebuilds the live rapier collider | `physics/tests/external_edits.rs:104 test_collider_edit_rebuilds_live_rapier_collider` |
| CONTRACT | removing the Collider component drops the rapier collider | `physics/tests/external_edits.rs:145 test_collider_component_removal_drops_rapier_collider` |
| CONTRACT | a sensor collider fires intersection events | `physics/src/physics_world/tests.rs:400 test_sensor_collider_fires_intersection_events` |
| CONTRACT | contact points are in world space (pixels) | `physics/src/physics_world/tests.rs:359 test_contact_points_are_in_world_space` |
| CONTRACT | raycast normalizes direction so distance is in pixels | `physics/src/physics_world/tests.rs:335 test_raycast_normalizes_direction_so_distance_is_in_pixels` |
| GUARD | an invalid pixels-per-meter scale is sanitized at world creation | `physics/src/physics_world/tests.rs:312 test_invalid_scale_in_struct_literal_is_sanitized_at_world_creation` |
| CONTRACT | capsule-Y half-height excludes the two cap radii | `physics/src/components.rs:440 test_collider_shapes` — **rename** `test_capsule_y_half_height_excludes_the_two_cap_radii` |
| CONTRACT | a shape cycle carries tuned dimensions into the new variant | `physics/src/components.rs:537 test_shape_cycle_carries_tuned_dimensions` |
| CONTRACT | physics components round-trip through the dynamic tier | `physics/src/register.rs:62 test_physics_components_round_trip_through_the_dynamic_tier` |
| CONTRACT | a ball bounces off a static brick (CCD + restitution regression) | `physics/tests/ball_brick_bounce.rs:51 ball_bounces_off_static_brick` |

**MERGE-INTO (physics).** `physics_world/tests.rs:20`,`:33`→`:48`;
`components.rs:460`,`:468`,`:476`,`:486` → one order-independence table;
`:497`,`:506` → one `other()` test; `:515`,`:523`,`:574`→`:537`.
`physics_system/tests.rs:174` folds into `:150`.
The three `presets.rs` tests collapse to at most one table-driven
`test_preset_values_are_the_documented_tuning` — and a sibling cleanup plan proposes
deleting four of the presets outright, so confirm before writing it.

**WEAK KEEPS (physics).**
- `physics_world/tests.rs:65 test_step_simulation` puts its only assertion inside
  `if let Some(..)`, so a body that was never created passes silently. Use
  `.expect("body exists")`.
- `physics_world/tests.rs:294` asserts `is_finite`, which the rubric names explicitly.
  Assert the actual fallen position for a 100 px/m step.
- `physics_system/tests.rs:462 test_reset_body_zeros_velocity_and_sets_position` asserts
  only `vel.length() < 1.0` and never checks the position the name promises.
- `src/lib.rs:124 test_collision_detection` starts the boxes 10.0 apart and asserts
  `distance >= 10.0`, which passes with **zero** collision response. Deleted in favour of
  `physics_world/tests.rs:106`.

**Fixture note.** The 3-component spawn (`Transform2D` + `RigidBody` + `Collider` +
`initialize` + `update`) is retyped in 14 tests in `physics_system/tests.rs`, all five in
`tests/external_edits.rs`, and both in `src/lib.rs`. One `spawn_body(world, pos, rb,
collider)` shared by the keeps removes about 200 lines. Separately, the collision-pair find
closure is written longhand at `physics_world/tests.rs:135`, `:169`, `:181`, `:223` even
though `CollisionEvent::involves` exists and is used at `:384` in the same file — the test
file duplicates production logic it also tests.

**MISSING in physics.**
1. **Collision groups and filters.** `Collider.collision_groups`/`collision_filter` feed
   `InteractionGroups` at `physics_world/bodies.rs:97-102`, and nothing asserts that two
   colliders in non-overlapping groups produce **no** event. This underpins every game's
   ball/paddle/brick layering.
2. **`Collider.offset`** — `bodies.rs:89` applies it as the collider's translation, and no
   test checks that an offset collider collides at the offset position.
3. **Kinematic bodies** are constructed but never simulated: moved by `set_body_transform`,
   unaffected by gravity, still generating events.
4. **Live `body_type` change** is documented as requiring a body rebuild. A pinning test
   would make the limitation visible and catch the day it is fixed.
5. **`PhysicsSystem::raycast`** (the ECS-facing wrapper) is untested; only
   `PhysicsWorld::raycast` has tests.

---

## 12. input — 74 → 15

| kind | contract | test |
|---|---|---|
| CONTRACT | queued events do not apply until `process_queued_events` | `input/tests/input_event_queue.rs:6 test_input_event_queuing` |
| CONTRACT | `update` clears just-pressed and just-released states | `input/tests/input_event_queue.rs:44 test_update_clears_just_states` |
| CONTRACT | multiple events apply in order | `input/tests/input_event_queue.rs:69 test_multiple_events_processing_order` |
| GUARD | `InputMapping::new()` is EMPTY — nothing is bound implicitly | `input/tests/input_mapping.rs:14 test_new_mapping_is_empty` |
| CONTRACT | unbinding a source removes it from every action | `input/tests/input_mapping.rs:105 test_unbind_source_removes_from_all_actions` |
| GUARD | a second bound source does not re-trigger activation (strict edge) | `input/tests/input_handler_integration.rs:63 test_second_source_does_not_retrigger_activation` |
| CONTRACT | releasing one source keeps the action active while another is held | `input/tests/input_handler_integration.rs:82 test_releasing_one_source_keeps_action_active` |
| CONTRACT | an axis source drives an action across frames | `input/tests/input_handler_integration.rs:214 test_axis_source_drives_action_across_frames` |
| GUARD | a negative axis binding ignores positive deflection | `input/tests/input_handler_integration.rs:244 test_negative_axis_binding_ignores_positive_deflection` |
| GUARD | connect registers and disconnect drops state with no just-released edge | `input/tests/input_handler_integration.rs:264 test_connect_event_registers_and_disconnect_drops_state` |
| CONTRACT | the first position update records position without a delta | `input/tests/mouse.rs:20 test_first_position_update_records_position_without_delta` |
| CONTRACT | movement delta accumulates within a frame and resets each frame | `input/tests/mouse.rs:58 test_movement_delta_resets_each_frame` |
| CONTRACT | wheel deltas accumulate and normalize to lines | `input/tests/mouse.rs:122 test_mouse_wheel` |
| GUARD | an axis fires once on crossing the threshold and re-arms below it | `input/src/gamepad.rs:227 axis_just_activated_fires_once_on_crossing_and_rearms_below_threshold` |
| GUARD | opposite directions track edges independently | `input/src/gamepad.rs:264 opposite_directions_track_edges_independently` |
| CONTRACT | `clear_frame_state` fans out to every child pad | `input/tests/gamepad.rs:131 test_gamepad_manager_update` |
| CONTRACT | default pairing isolates player devices | `input/src/player.rs:486 default_pairing_isolates_player_devices` |
| CONTRACT | `assign_pad` repoints pad sources without touching the keyboard | `input/src/player.rs:513 assign_pad_repoints_pad_sources_without_touching_keyboard` |
| CONTRACT | `move_y` merges digital and stick input and clamps | `input/src/player.rs:547 move_y_merges_digital_and_stick_and_clamps` |
| CONTRACT | binding changes set dirty; `take_dirty` clears it (save-on-change) | `input/src/player.rs:446 binding_changes_set_dirty_and_take_dirty_clears_it` |
| CONTRACT | press sets pressed and just_pressed; a repeated press does not re-trigger | `input/src/button_tracker.rs:96 test_repeated_press_does_not_retrigger_just_pressed` |

**MERGE-INTO (input).** `input_mapping.rs:150`,`:162`,`:172` → one preset table (and `:141`
is subsumed by them). `input_event_queue.rs:21`→`:6`. `tests/gamepad.rs:90` folds into
`src/gamepad.rs:277`. `tests/mouse.rs:38`→`:58`. `button_tracker.rs:87`,`:106`,`:115`→`:96`.

**Integration-vs-unit verdict.** The three queue-shaped files do **not** test one queue three
times — two of them do. `input_event_queue.rs` owns the queue contract.
`input_handler_integration.rs` tests `InputMapping` evaluation against handler state, a
different unit, and is the only place that crossing is exercised.
**`input/tests/input_handler.rs` tests neither and duplicates both — all five tests go and
the file with them.** `tests/gamepad.rs` shrinks to its one unique test (`:131`); the rest
are Default echoes or `ButtonTracker` delegation already covered in `src/`.
`tests/keyboard.rs` and `tests/mouse.rs` have no `src/` counterpart at all — `src/keyboard.rs`
and `src/mouse.rs` carry no inline test modules — and the mouse delta tests are the strongest
material in that directory.

**MISSING in input.**
1. **The winit boundary.** `convert_physical_key` (`keyboard.rs:50`) and
   `handle_window_event` (`input_handler.rs:226`) are the only untested paths in the crate,
   and they are where a winit upgrade breaks. Both are headlessly testable:
   `PhysicalKey::Code(KeyCode::KeyA) → Some(KeyA)` and `PhysicalKey::Unidentified(..) → None`
   are constructible, and a synthesized `WindowEvent::MouseWheel` would pin the
   `SCROLL_PIXELS_PER_LINE = 16.0` normalization, which has zero coverage today.
   The existing `tests/keyboard.rs:118 test_convert_physical_key` is a comment block with no
   code and no assertion — delete it and write the real one.
2. **`AXIS_ACTIVATION_THRESHOLD` is never pinned.** Every axis test passes 0.5 explicitly.
   Changing the constant breaks stick feel in all six games with a green suite.

---

## 13. audio — 26 → 9

| kind | contract | test |
|---|---|---|
| GUARD | a disabled manager loads and plays as a no-op | `audio/src/manager/tests.rs:82 test_disabled_manager_loads_and_plays_as_noop` |
| GUARD | a disabled manager still rejects invalid handles | `audio/src/manager/tests.rs:91 test_disabled_manager_still_rejects_invalid_handles` |
| GUARD | disabled music loads but reports not playing | `audio/src/manager/tests.rs:212 test_disabled_manager_music_loads_but_reports_not_playing` |
| CONTRACT | a missing file returns `IoError`, invalid bytes return `DecodeError` | `audio/src/manager/tests.rs:138 test_load_sound_from_invalid_bytes_returns_decode_error` |
| CONTRACT | `enable_output` preserves sounds, ids and volumes | `audio/src/manager/tests.rs:255 test_enable_output_preserves_sounds_ids_and_volumes` |
| CONTRACT | music started while disabled is recorded as pending | `audio/src/manager/tests.rs:293 test_start_music_while_disabled_records_pending` |
| GUARD | `stop_music` clears the pending request | `audio/src/manager/tests.rs:306 test_stop_music_while_disabled_clears_pending` |
| GUARD | a new music request replaces the pending one (last wins) | `audio/src/manager/tests.rs:318 test_new_music_request_replaces_pending` |
| GUARD | a failed `play_music` leaves no pending request | `audio/src/manager/tests.rs:335 test_play_music_missing_file_leaves_no_pending` |
| CONTRACT | volume setters clamp out-of-range values | `audio/src/manager/tests.rs:192 test_volume_setters_clamp_out_of_range_values` |

**MERGE-INTO (audio).** `:7`,`:16` → one `SoundSettings` clamp test. `:127` folds into `:138`
as an error table. `:108`,`:242`,`:275`,`:347` → `:255`.

The audio suite is the healthiest in the workspace — confirmed by reading, not assumed. Its
four deletions are the vacuous ones: `:25` reimplements `.clamp()` inline, `:98` calls four
music controls with nothing asserted, and `:173`/`:181` assert `active_sound_count() == 0` on
a **disabled** manager, where the count is structurally always zero and the assertion cannot
fail.

**MISSING in audio.** Nothing in the suite exercises an *enabled* manager's sink bookkeeping.
`active_sound_count`, `update()`'s `retain(|a| !a.sink.empty())`, `stop` and `stop_all` are
asserted only against a disabled manager — which is why two of the deletions above are
vacuous rather than merely weak. A test-only sink seam would give the SFX lifecycle its first
real coverage. Note that a sibling cleanup plan proposes deleting `stop_all`,
`active_sound_count`, `play_music_once` and `unload_all` as dead public API; if that lands,
this gap closes by subtraction instead.

---

## 14. Cross-cutting

**Guards the lead cited all exist and are on the keep-list**, verified against the tree:
the editor theme WCAG ladder (`editor/src/theme/tests.rs:103`, `:119`), the command-API spec
drift pair (`editor/src/command_api/specs.rs:188`, `:200`), the input-settings legacy
fallbacks (`engine_core/src/input_settings_io.rs:204`, `:220`, `:232`), and the physics
root-entity pin (`physics/src/physics_system/tests.rs:491`).

**The four guards the lead asked for, by status:**

| guard | status |
|---|---|
| scene-serializer table drift | **MISSING** — spec in §8, MISSING 1 |
| `offset_of!` on GPU vertex layouts | **MISSING** — spec in §10, MISSING 1 |
| Behavior scene fixture (all variants) | **PARTIAL** — `engine_core/src/scene_serializer_tests.rs:280` covers `PlayerPlatformer` only |
| `push_as_one` / merge isolation | **HALF** — the gesture boundary is `editor/src/commands/dirty_tests.rs:199`; the entity-isolation half is MISSING |

**Duplicated fixtures to consolidate while deleting.** The press-frame/release-frame click
harness is written **eleven** times across `editor` and `ui`, in two mutually incompatible
shapes — and `engine_core`'s three `frame()` helpers already disagree (`pause.rs:210` and
`menu_panel.rs:325` call `end_frame()` first; `menu_input.rs:139` does not). `DummyGame` is
defined eleven times in `editor_integration`. `test_texture_path` and `StubResolver` each
have four to six copies in `engine_core`. `renderer/src/sprite/instance_cache.rs:84` already
defines the fixtures `sprite/batch.rs` writes out sixteen times by hand. One
`#[cfg(test)] mod test_support` per crate removes several hundred lines and closes the
`menu_input` divergence.

**Temp-file discipline.** Three idioms coexist in `engine_core`, and two of them
(`localization.rs:592`, `scene_serializer_tests.rs:436`) leak the directory on a failing
assert because cleanup is a trailing statement rather than a `TempDir` guard.
`editor/src/editor_preferences.rs:137` is worse — a fixed shared path that races across
concurrent test binaries.

**Three dead public types**, each worth an issue rather than a test:
`engine_core::Timer` (no consumer in any crate or any of the six games — and
`engine_core/tests/timing.rs` is 147 lines testing it),
`ecs::GlobalTransform2D::transform_point` (no caller outside its own test), and
`common::Time` (re-exported twice, called by nothing). `common::Rect::contains`/`intersects`/
`intersection` and `Camera::world_bounds`/`contains_point` are dead too.

**Suite hygiene confirmed clean:** 0 `#[ignore]` attributes anywhere in `crates/`.
