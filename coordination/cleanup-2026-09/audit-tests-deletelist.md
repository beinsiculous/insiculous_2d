# Test-suite audit — insiculous_2d

Read-only audit of all 1,657 `#[test]` functions across `crates/*`. Every test file was
read in full; nothing was sampled. Rubric: `training.md` § "Writing Tests".

> **Note on file location.** The session ran in plan mode, which permits writing only to
> the plan file. The report was therefore written here instead of
> `…/scratchpad/audit-tests.md`. Content is complete and unabridged.

---

## 1. Per-crate table

Totals are grep-verified (`grep -rc '#\[test\]'`). "Delete" is bloat to remove outright;
"merge" counts tests that fold into a smaller number of table-driven tests; "rename" is
method-named tests; "strengthen" is right-subject/weak-assert.

| crate | total | delete | merge (tests → groups) | rename | strengthen |
|---|---:|---:|---|---:|---:|
| audio | 26 | 4 | 4 → 2 | 0 | 0 |
| common | 41 | 1 | 2 → 1 | 1 | 4 |
| ecs (inline) | 119 | 32 | 11 → 5 | 2 | 3 |
| ecs (`tests/`) | 94 | 26 | 7 → 3 | 1 | 2 |
| ecs_macros | 3 | 1 | 0 | 0 | 0 |
| editor | 477 | 86 | 41 → 16 | 15 | 8 |
| editor_integration | 150 | 35 | 21 → 8 | 2 | 2 |
| engine_core (src) | 322 | 42 | 32 → 12 | 14 | 8 |
| engine_core (`tests/`) | 72 | 10 | 10 → 4 | 6 | 4 |
| input | 74 | 15 | 11 → 5 | 0 | 0 |
| physics | 64 | 7 | 9 → 3 | 1 | 4 |
| renderer | 92 | 31 | 15 → 7 | 5 | 7 |
| ui | 123 | 19 | 21 → 10 | 6 | 6 |
| **total** | **1657** | **309** | **184 → 76** | **53** | **48** |

309 deletions is 18.6% of the suite. After deletes and merges the suite lands near 1,240.

**Six files empty out entirely** and should be removed with their tests:
`crates/ecs/tests/component.rs`, `crates/ecs/tests/init.rs`, `crates/ecs/tests/system.rs`,
`crates/engine_core/tests/init.rs`, `crates/input/tests/input_handler.rs`, and
`crates/ui/tests/ui_interaction_debug.rs` (five of its seven tests move inline first — see §6).

---

## 2. DELETE list

`path:line test_name — reason`. Line numbers are the `fn` line.

### common (1)

```
crates/common/src/transform.rs:168 test_transform_point — translation-only case, subsumed by test_matrix_applies_scale_before_rotation_before_translation:223 and test_inverse_transform_point_round_trips:192 (duplicate)
```

### ecs_macros (1)

```
crates/ecs_macros/tests/derive_test.rs:31 test_field_names_generated — len + contains; strictly weaker than test_field_names_order_preserved:40, which asserts the exact array (duplicate)
```

### ecs — inline `src/**` (32)

```
crates/ecs/src/component_registry/tests.rs:24 test_define_component_creates_struct — macro default-field echo; :106 asserts the same defaults where they matter (Default echo)
crates/ecs/src/state_machine.rs:274 test_initial_state — constructor echo; just_entered-at-construction asserted at :322 (constructor echo)
crates/ecs/src/state_machine.rs:332 test_elapsed_resets_on_transition — :293 already asserts elapsed()==0.0 after a transition (duplicate)
crates/ecs/src/state_machine.rs:342 test_is_check — method-named test of a one-line equality wrapper; exercised at :438 (accessor echo)
crates/ecs/src/state_machine.rs:370 test_state_machine_with_simple_enum — re-runs transition/just_left/just_entered against a second enum; StateMachine<S> is generic, no new contract (duplicate)
crates/ecs/src/state_machine.rs:426 test_hierarchical_previous_parent_tracking — asserts !parent_just_changed() for a same-group transition, which is :399 (duplicate)
crates/ecs/src/state_machine.rs:447 test_hierarchical_tick_and_elapsed — duplicate of :319 through a pass-through delegate (duplicate)
crates/ecs/src/state_machine.rs:455 test_hierarchical_same_state_is_noop — duplicate of :296 through a pass-through delegate (duplicate)
crates/ecs/src/behavior.rs:433 test_behavior_state_default_is_idle — Default field echo; the Idle-start contract is asserted at engine_core/src/behavior_runner/mod.rs:471 (Default echo)
crates/ecs/src/behavior.rs:456 test_behavior_default_is_player_platformer_with_serde_defaults — identical four values as :412 (duplicate)
crates/ecs/src/behavior.rs:575 test_chase_tagged_serialization — RON round trip of one String field; round-trip proven by :384 and :494 (duplicate)
crates/ecs/src/hierarchy_extension.rs:292 test_set_parent_rejects_self_parent — duplicate of ecs/tests/world.rs:130 (duplicate)
crates/ecs/src/hierarchy_extension.rs:301 test_set_parent_rejects_cycle — duplicate of ecs/tests/world.rs:108, which is stronger (asserts the error names the cycle) (duplicate)
crates/ecs/src/hierarchy_extension.rs:345 test_get_descendants — duplicate of ecs/tests/world.rs:426 (duplicate)
crates/ecs/src/hierarchy_extension.rs:377 test_is_ancestor_of — duplicate of ecs/tests/world.rs:426 (duplicate)
crates/ecs/src/hierarchy_extension.rs:392 test_is_descendant_of — duplicate of ecs/tests/world.rs:426 (duplicate)
crates/ecs/src/hierarchy_extension.rs:407 test_remove_entity_hierarchy — duplicate of ecs/tests/world.rs:405, which is 100 deep and asserts no residue (duplicate)
crates/ecs/src/event.rs:208 test_has_events — has_events is count::<E>() > 0; :218 covers it (wrapper duplicate)
crates/ecs/src/event.rs:230 test_type_count — type_count() has no production caller and no downstream contract (introspection echo)
crates/ecs/src/event.rs:257 test_complex_event_data — emit/read with a 3-field payload; identical contract to :166 (duplicate)
crates/ecs/src/resource.rs:170 test_contains_resource — is_some() wrapper implied by :122 and :154 (wrapper duplicate)
crates/ecs/src/resource.rs:195 test_get_nonexistent_returns_none — :161 and :165 already assert the missing-resource path (duplicate)
crates/ecs/src/hierarchy.rs:225 test_parent_component — Parent::new(id).entity() == id (constructor echo)
crates/ecs/src/hierarchy.rs:258 test_global_transform_identity — Default echo; identity is exercised by every propagation test in ecs/tests/hierarchy_dirty.rs (Default echo)
crates/ecs/src/hierarchy.rs:266 test_global_transform_from_local — three field copies of a conversion (field echo)
crates/ecs/src/hierarchy.rs:312 test_transform_point — same math as :278; also the ONLY reference to GlobalTransform2D::transform_point, which has no production caller anywhere (verified) (duplicate + dead API)
crates/ecs/src/ui_components.rs:304 test_component_meta_names — type_name() asserts whose downstream contract is covered for all three types at component_registry/tests.rs:222-224 (type_name echo)
crates/ecs/src/hierarchy_system.rs:294 test_root_entity_transform_propagation — duplicate of ecs/tests/hierarchy_dirty.rs:23/:172 (duplicate)
crates/ecs/src/hierarchy_system.rs:316 test_child_entity_transform_propagation — duplicate of ecs/tests/hierarchy_dirty.rs:23 (duplicate)
crates/ecs/src/hierarchy_system.rs:351 test_grandchild_transform_propagation — duplicate of ecs/tests/hierarchy_dirty.rs:40 (duplicate)
crates/ecs/src/hierarchy_system.rs:440 test_disabled_system_does_nothing — duplicate of ecs/tests/hierarchy_dirty.rs:153, which asserts the stale global AND the recovery (duplicate)
crates/ecs/src/grid_backdrop.rs:188 test_component_meta_names_every_field — type_name() + two contains() with no downstream contract; registration asserted at component_registry/tests.rs:304 (type_name echo)
```

### ecs — `tests/**` (26)

```
crates/ecs/tests/component.rs:4 test_component_trait — trivial direct case of the blanket Component impl; the real footgun contract is world.rs:446 test_component_types_reports_concrete_type_names (duplicate)
crates/ecs/tests/component.rs:22 test_component_in_world — add/has/get; duplicate of world.rs:30 test_component_management (duplicate)
crates/ecs/tests/component.rs:49 test_multiple_components — add two, assert has both; duplicate of world.rs:144 test_query_entities (duplicate)
   -> the file is now empty; delete crates/ecs/tests/component.rs
crates/ecs/tests/entity.rs:4 test_entity_creation — Entity::new().is_active() (Default echo); covered by :21
crates/ecs/tests/entity.rs:12 test_entity_id_uniqueness — duplicate of entity_generation.rs:33 test_entity_id_generator (duplicate)
crates/ecs/tests/entity.rs:40 test_entity_id_value — asserts id.value() > 0, no contract (weak assert)
crates/ecs/tests/entity.rs:49 test_entity_id_display — Display starts_with "Entity(" with no consumer (label echo)
crates/ecs/tests/init.rs:3 test_init — third copy of "new world has 0 entities, 0 systems" (world.rs:4 and world.rs:97 are the others) (duplicate)
crates/ecs/tests/init.rs:17 test_init_and_use — duplicate of world.rs:30 test_component_management (duplicate)
   -> the file is now empty; delete crates/ecs/tests/init.rs
crates/ecs/tests/system.rs:4 test_system_trait — asserts a test-local struct's own counter (tests the fixture)
crates/ecs/tests/system.rs:38 test_simple_system — asserts system.name(); the comment concedes nothing else is verifiable (label echo)
crates/ecs/tests/system.rs:61 test_system_in_world — system_count()==1 with a comment admitting nothing is verified (count echo)
crates/ecs/tests/system.rs:95 test_multiple_systems — system_count()==2, same shape (count echo)
   -> the file is now empty; delete crates/ecs/tests/system.rs
crates/ecs/tests/system_lifecycle.rs:102 test_system_lifecycle — the guard logic it asserts (`if is_started() && !is_stopped()`) is written inside TestSystem in this file (tests the fixture)
crates/ecs/tests/system_lifecycle.rs:139 test_system_registry_lifecycle — len()==2 then a no-assert init/start/stop/shutdown walk; the real refusals are :250 (count echo + no state check)
crates/ecs/tests/system_lifecycle.rs:222 test_system_lifecycle_errors — every error it asserts is raised by TestSystem's own start/stop guards, not by production code (tests the fixture)
crates/ecs/tests/system_lifecycle.rs:281 test_system_update_safety — no assert at all (constructs and returns)
crates/ecs/tests/entity_generation.rs:7 test_entity_generation_creation — subsumed by test_entity_generation_lifecycle:16 (duplicate)
crates/ecs/tests/entity_generation.rs:61 test_world_entity_generation_tracking — subsumed by :89 test_world_entity_reference_validation (duplicate)
crates/ecs/tests/entity_generation.rs:76 test_world_entity_validation_after_removal — subsumed by :147 test_world_entity_operations_with_generation (duplicate)
crates/ecs/tests/entity_generation.rs:107 test_world_entity_generation_after_reuse — pins "we don't reuse entity IDs yet"; asserts an unimplemented state, not a contract (pins a non-behavior)
crates/ecs/tests/entity_generation.rs:173 test_entity_reuse_detection — the entire body is inside `if entity_id1.value() == entity_id2.value()`, which is never true today: zero assertions execute (structurally a no-op)
crates/ecs/tests/world.rs:97 test_world_initialization — duplicate of :4 test_world_creation (duplicate)
crates/ecs/tests/world.rs:237 test_spawn_creates_entity — subsumed by :246 test_spawn_with_single_component (duplicate)
crates/ecs/tests/world.rs:275 test_spawn_returns_correct_entity_id — subsumed by :292 test_spawn_multiple_entities_independent (duplicate)
crates/ecs/tests/sprite_components.rs:284 test_component_trait — compile-only assert_component calls, no runtime assert (constructs and returns)
```

### editor (86)

```
crates/editor/src/context/tests.rs:4 test_editor_context_new — Default echo; every field re-asserted by the tool/popup/dirty/scene_path tests
crates/editor/src/context/tests.rs:58 test_editor_context_camera_zoom_clamp — one-line delegation to SceneViewport; duplicate of viewport/tests.rs:93
crates/editor/src/context/tests.rs:94 test_editor_context_grid — pure delegation to GridRenderer; duplicate of grid.rs:375 + grid.rs:387
crates/editor/src/context/tests.rs:205 test_gizmo_has_priority — single assert that a fresh gizmo is inactive; no state change (Default echo)
crates/editor/src/context/tests.rs:223 test_add_component_popup_default_closed — first line of test_add_component_popup_toggle asserts the same (duplicate)
crates/editor/src/context/tests.rs:256 test_dirty_flag_default_false — covered by test_title_bar_text_clean (duplicate)
crates/editor/src/context/tests.rs:262 test_set_dirty_flips_flag — setter/getter echo; the flag's contract is in test_title_bar_text_dirty
crates/editor/src/context/tests.rs:271 test_scene_path_default_none — test_scene_display_name_untitled covers the None path observably (duplicate)
crates/editor/src/menu/tests.rs:4 test_menu_item_action — MenuItem::action("Test").label() == "Test" (constructor echo)
crates/editor/src/menu/tests.rs:10 test_menu_item_with_shortcut — destructures the literal it just built (constructor echo)
crates/editor/src/menu/tests.rs:40 test_menu_item_with_checked_builder — builder echo
crates/editor/src/menu/tests.rs:64 test_menu_item_separator — constructs a Separator, asserts it is a Separator (constructor echo)
crates/editor/src/menu/tests.rs:71 test_menu_item_submenu — constructor echo + items.len()
crates/editor/src/menu/tests.rs:82 test_menu_item_with_enabled — builder echo
crates/editor/src/menu/tests.rs:92 test_menu_new — Default echo
crates/editor/src/menu/tests.rs:100 test_menu_add_item — items.len()==3 after three add_item calls (count echo)
crates/editor/src/menu/tests.rs:110 test_menu_with_items — same, len only (count echo)
crates/editor/src/menu/tests.rs:132 test_menu_bar_new — Default echo
crates/editor/src/menu/tests.rs:139 test_menu_bar_add_menu — count echo
crates/editor/src/menu/tests.rs:160 test_menu_bar_height — asserts the literal constant 24.0 (constant echo)
crates/editor/src/menu/tests.rs:166 test_menu_bar_close_all — covered by test_apply_toggle_opens_and_closes and test_outside_press_closes_open_menu (duplicate)
crates/editor/src/commands/tests.rs:328 test_set_transform_undo — identical command and assertion as :19 test_command_history_execute_and_undo (duplicate)
crates/editor/src/commands/tests.rs:398 test_max_history_limit — weaker duplicate of :423, which asserts the same count plus eviction order (duplicate)
crates/editor/src/hierarchy_tests.rs:97 test_default_expanded — test_toggle_collapse asserts the same initial state on its first line (duplicate)
crates/editor/src/hierarchy_tests.rs:199 test_hierarchy_panel_new — same assert as :97 (duplicate)
crates/editor/src/hierarchy_tests.rs:205 test_root_entities_rendering_order — exercises only ecs set_parent/get_root_entities, no HierarchyPanel call (another crate's tests)
crates/editor/src/hierarchy_tests.rs:224 test_collapsed_hides_children — never checks that children are hidden; re-asserts is_expanded. Real contract is :365 (duplicate)
crates/editor/src/hierarchy_tests.rs:241 test_deep_hierarchy_structure — pure ecs get_descendants, no editor code (another crate's tests)
crates/editor/src/viewport/tests.rs:4 test_viewport_new — Default echo
crates/editor/src/viewport/tests.rs:85 test_viewport_pan — one pan_immediate from the origin, asserts the argument back (setter echo)
crates/editor/src/selection.rs:156 test_selection_new — Default echo
crates/editor/src/selection.rs:164 test_selection_select — subsumed by :177 test_selection_select_clears_previous (duplicate)
crates/editor/src/selection.rs:192 test_selection_add — subsumed by :306 and :319 (duplicate)
crates/editor/src/selection.rs:208 test_selection_remove — subsumed by :335 (duplicate)
crates/editor/src/selection.rs:223 test_selection_remove_primary_updates — weaker duplicate of :335 (duplicate)
crates/editor/src/selection.rs:264 test_selection_select_multiple — subsumed by :351 and :374 (duplicate)
crates/editor/src/selection.rs:388 test_selection_iterator — order-insensitive, weaker duplicate of :306 (duplicate)
crates/editor/src/dock/tests.rs:58 test_dock_area_add_panel — panels().len() after two adds (count echo)
crates/editor/src/dock/tests.rs:67 test_dock_area_get_panel — asserts the title it just constructed; get_panel is exercised by every layout test (constructor echo)
crates/editor/src/stored_component/tests.rs:80 test_capture_includes_all_component_types — captured.len()==18 only; :224 asserts the count plus registry agreement (count-only duplicate)
crates/editor/src/stored_component/tests.rs:107 test_gameplay_components_registered_under_gameplay_category — category() echo for two variants; :211 covers all of them (duplicate)
crates/editor/src/stored_component/tests.rs:154 test_remove_absent_component_is_safe — "does not panic", then asserts a never-added component is absent (no state check)
crates/editor/src/stored_component/tests.rs:174 test_display_names_match_variant_names — display_name() string echo plus a non-empty loop (label echo)
crates/editor/src/grid.rs:368 test_grid_renderer_new — Default echo
crates/editor/src/grid.rs:375 test_grid_visibility_toggle — bare bool flip; downstream contract is :398 test_hidden_grid_produces_no_segments (setter echo)
crates/editor/src/gizmo/tests.rs:41 test_gizmo_mode_default — Default echo, repeated on test_gizmo_new's first line
crates/editor/src/gizmo/tests.rs:46 test_gizmo_mode_names — name() string echo (label echo)
crates/editor/src/gizmo/tests.rs:54 test_gizmo_new — Default echo (mode/position/is_active)
crates/editor/src/gizmo/tests.rs:62 test_gizmo_interaction_default — Default echo of a plain data struct
crates/editor/src/world_snapshot/tests.rs:11 test_snapshot_empty_world — entity_count()==0 on an empty world (Default echo)
crates/editor/src/world_snapshot/tests.rs:18 test_snapshot_captures_entities — count-only; subsumed by :29 test_snapshot_restore_preserves_entity_ids (duplicate)
crates/editor/src/viewport_input.rs:351 test_viewport_input_handler_new — Default echo
crates/editor/src/viewport_input.rs:358 test_zoom_factor_calculation — calls the test module's own calculate_zoom_factor, documented as "mirrors the logic in handle_input" (tests the fixture / reimplements production)
crates/editor/src/viewport_input.rs:367 test_zoom_factor_inverted — same test-local reimplementation (tests the fixture)
crates/editor/src/viewport_input.rs:373 test_screen_to_world_delta — same, for the test-local copy (tests the fixture)
crates/editor/src/viewport_input.rs:383 test_screen_to_world_delta_with_zoom — same (tests the fixture)
crates/editor/src/viewport_input.rs:393 test_viewport_input_config_default — Default echo
crates/editor/src/toolbar.rs:211 test_toolbar_set_position — setter echo; bounds math is covered by :329 (setter echo)
crates/editor/src/toolbar.rs:219 test_editor_tool_default_is_move_so_a_gizmo_shows — Default echo; real contract at context/tests.rs:213 (duplicate)
crates/editor/src/toolbar.rs:225 test_editor_tool_names — name() string echo (label echo)
crates/editor/src/toolbar.rs:233 test_editor_tool_shortcuts — shortcut() strings vs hardcoded literals; asserts nothing about the actual bindings (label echo — see gap E2)
crates/editor/src/toolbar.rs:241 test_editor_tool_all — ALL.len() + contains (ALL.len() echo)
crates/editor/src/toolbar.rs:251 test_toolbar_new — Default echo
crates/editor/src/toolbar.rs:257 test_toolbar_with_position — builder echo reading a private field back
crates/editor/src/toolbar.rs:263 test_toolbar_set_tool — sets Move, already the default, then asserts Move (tautological setter echo)
crates/editor/src/commands/dirty_tests.rs:28 test_new_history_is_clean — Default echo; covered by :176 and :188 (duplicate)
crates/editor/src/selection_outline.rs:215 test_render_outline_emits_four_lines_for_selected_sprite — draw-list length only; subsumed by :265, which asserts counts AND colors (draw-list length)
crates/editor/src/editable_inspector.rs:522 test_field_id_creation — no assert at all (verified: body is `let _widget_id: ui::WidgetId = id.into();` plus a comment) (constructs and returns)
crates/editor/src/editable_inspector.rs:529 test_editable_field_style_default — Default echo of row_height/label_width/padding literals
crates/editor/src/editable_inspector.rs:571 test_editable_inspector_builder — constructs no inspector; identical body to :529 (duplicate + Default echo)
crates/editor/src/component_editors.rs:450 test_component_edit_carries_value_and_hint — struct-literal echo
crates/editor/src/component_editors.rs:460 test_component_edit_equality — derived PartialEq echo
crates/editor/src/component_editors.rs:480 test_transform_default_values — tests common::Transform2D::default() (another crate's tests)
crates/editor/src/component_editors.rs:488 test_sprite_default_values — tests ecs::Sprite::default() (another crate's tests)
crates/editor/src/component_editors.rs:497 test_rigid_body_default_values — tests physics RigidBody::default() (another crate's tests)
crates/editor/src/component_editors.rs:505 test_collider_default_values — tests physics Collider::default() (another crate's tests)
crates/editor/src/component_editors.rs:514 test_audio_source_default_values — tests ecs AudioSource::default() (another crate's tests)
crates/editor/src/status_bar.rs:160 test_show_message — setter echo; first two lines of :167 test_message_auto_clears (duplicate)
crates/editor/src/status_bar.rs:195 test_update_stats — update_stats(42, 60.0) then asserts 42 and 60.0 back (setter echo)
crates/editor/src/status_bar.rs:218 test_clear_message_stops_timer — the same clear_message path is asserted at :181 (duplicate)
crates/editor/src/inspector.rs:320 test_inspector_style_default — Default echo (padding/line_height literals)
crates/editor/src/editor_preferences.rs:126 test_editor_preferences_defaults — Default echo; load-path fallback covered at :171 (duplicate)
crates/editor/src/play_controls.rs:186 test_play_controls_default — Default echo (button_size/spacing literals)
crates/editor/src/play_controls.rs:268 test_play_control_action_eq — derived PartialEq echo on an enum
crates/editor/src/typography.rs:44 test_font_scale_is_ordered — asserts 12.0 < 14.0 < 16.0 from the Default it constructs; no consumer of the ordering (Default echo)
crates/editor/src/behavior_editor.rs:228 test_ranges_are_well_formed — start()<end() plus contains() of variant defaults, all re-asserted with real values at :240 (duplicate)
```

> Keep, do not confuse with the above: the same-named `test_ranges_are_well_formed` at
> `crates/editor/src/component_editors.rs:469` encodes rapier semantics (damping/friction
> above 1, positive extents) and stays.

### editor_integration (35)

The 14 delete/duplicate tests in `entity_ops_tests.rs` all exercise
`entity_ops::delete_selected_entities` / `duplicate_selected_entities`, which are
`#[cfg(test)]`-gated in `entity_ops.rs:219` and `:299`. The first is a line-for-line copy
of `editor::commands::DeleteEntityCommand::execute`; the second is
`EditorGame::duplicate_selected_entities` minus the command. They test a fixture, and the
production paths they shadow have no coverage at all (see gap I1).

```
crates/editor_integration/src/entity_ops_tests.rs:89 test_create_increments_counter — internal counter whose only visible effect is asserted by test_create_names_are_unique (duplicate)
crates/editor_integration/src/entity_ops_tests.rs:115 test_delete_removes_entity — tests test-only code (tests the fixture)
crates/editor_integration/src/entity_ops_tests.rs:127 test_delete_clears_selection — tests test-only code
crates/editor_integration/src/entity_ops_tests.rs:138 test_delete_reparents_children_to_grandparent — tests test-only code; move the case onto DeleteEntityCommand (see gap I2)
crates/editor_integration/src/entity_ops_tests.rs:158 test_delete_orphans_children_when_root — tests test-only code; move the case onto DeleteEntityCommand
crates/editor_integration/src/entity_ops_tests.rs:176 test_delete_empty_selection_is_noop — tests test-only code
crates/editor_integration/src/entity_ops_tests.rs:188 test_delete_multiple_selected — tests test-only code; production multi-delete is a MacroCommand this never touches
crates/editor_integration/src/entity_ops_tests.rs:206 test_duplicate_copies_components — tests test-only code; duplicates editor/src/clipboard.rs:328
crates/editor_integration/src/entity_ops_tests.rs:222 test_duplicate_offsets_position — tests test-only code (offset comes from the fixture)
crates/editor_integration/src/entity_ops_tests.rs:236 test_duplicate_selects_new_entity — strict subset of :206
crates/editor_integration/src/entity_ops_tests.rs:249 test_duplicate_preserves_original — tests test-only code
crates/editor_integration/src/entity_ops_tests.rs:263 test_duplicate_recursive_copies_children — tests test-only code; count assert subsumed by :279
crates/editor_integration/src/entity_ops_tests.rs:279 test_duplicate_children_have_correct_parent — tests test-only code; duplicates editor/src/clipboard.rs:328
crates/editor_integration/src/entity_ops_tests.rs:300 test_duplicate_name_appends_copy — duplicates editor/src/clipboard.rs:457
crates/editor_integration/src/entity_ops_tests.rs:314 test_duplicate_empty_selection_is_noop — tests test-only code
crates/editor_integration/src/entity_ops_tests.rs:413 test_handle_create_action_dispatches_ui_labels — duplicate of api_write_tests.rs:277, which drives all nine archetypes
crates/editor_integration/src/editor_game/tests.rs:35 test_play_action_captures_snapshot — strict subset of :48 and :73 (duplicate)
crates/editor_integration/src/editor_game/tests.rs:127 test_editor_game_initial_scene_state — three Default asserts with no downstream contract
crates/editor_integration/src/editor_game/tests.rs:172 test_save_creates_file — strict subset of :187 and :220 (duplicate)
crates/editor_integration/src/editor_game/tests.rs:207 test_new_scene_warns_if_dirty — asserts what :135/:149 assert; the warning it names is never checked (duplicate)
crates/editor_integration/src/editor_game/tests.rs:246 test_save_as_updates_path — no save happens: set_scene_path then read back (setter echo)
crates/editor_integration/src/editor_game/tests.rs:259 test_dirty_flag_set_on_entity_create — no entity is created; set_dirty(true) then is_dirty() (setter echo)
crates/editor_integration/src/editor_game/tests.rs:293 test_scene_display_in_status — the same strings are asserted by :316, which also covers change detection (duplicate)
crates/editor_integration/src/editor_game/tests.rs:300 test_undo_redo_on_empty_history_do_not_mark_dirty — duplicates editor/src/commands/dirty_tests.rs:188; the "mirror sync" is written inside the test, not in production
crates/editor_integration/src/editor_game/tests.rs:332 test_dirty_mirror_follows_history — same defect; the history half is editor/src/commands/dirty_tests.rs:44/:78
crates/editor_integration/src/editor_game/tests.rs:426 test_sync_viewport_from_main_camera_only_while_playing — duplicate of camera_follow_tests.rs:373, which also covers zoom
crates/editor_integration/src/editor_game/tests.rs:455 test_stop_restores_editing_camera — duplicate of camera_follow_tests.rs:445 plus :361
crates/editor_integration/src/panel_renderer/tests.rs:184 test_writeback_missing_entity_is_safe — no writeback happens; it is ecs behavior, covered by ecs/tests/world.rs:313 (another crate's tests)
crates/editor_integration/src/panel_renderer/tests.rs:195 test_writeback_missing_component_is_safe — same: world.get_mut returning None, no production code under test
crates/editor_integration/src/panel_renderer/tests.rs:212 test_set_transform_via_command_and_undo — duplicate of editor/src/commands/tests.rs:328
crates/editor_integration/src/panel_renderer/tests.rs:243 test_transform_slider_merges_into_single_undo — duplicate of editor/src/commands/tests.rs:344; the integration angle is kept by :154
crates/editor_integration/src/panel_renderer/tests.rs:282 test_add_component_via_command_and_undo — duplicate of editor/src/commands/tests.rs:208
crates/editor_integration/src/panel_renderer/tests.rs:301 test_remove_component_via_command_and_undo — duplicate of editor/src/commands/tests.rs:226
crates/editor_integration/src/editor_game/camera_follow_tests.rs:531 test_follow_toggle_chord_resolves_over_focus_binding — pure EditorInputMapping::resolve logic through a wrapper; belongs in editor/src/editor_input.rs:411 (see gap I7 — add the rows before deleting)
crates/editor_integration/src/editor_game/shortcuts_tests.rs:421 test_resolve_dispatch_table_is_the_single_shortcut_system — three rows lifted from editor/src/editor_input.rs:411 (duplicate)
```

### engine_core — `src/**` (42)

```
crates/engine_core/src/scene_serializer_tests.rs:454 test_default_texture_path — asserts the test file's own fixture helper, zero production code (tests the fixture)
crates/engine_core/src/achievements/tests.rs:191 hidden_achievement_flag_persists — .hidden() then read .hidden; nothing persists despite the name (builder echo)
crates/engine_core/src/achievements/tests.rs:198 default_toast_style_matches_documented_appearance — 13 asserts on ToastStyle::default() with no downstream contract (Default echo)
crates/engine_core/src/achievements/tests.rs:216 manager_uses_default_toast_style_until_overridden — setter/getter echo; the real contract is at :226
crates/engine_core/src/localization.rs:666 active_font_roundtrip — set_active_font then active_font() (setter/getter echo)
crates/engine_core/src/render_manager.rs:533 test_render_manager_default — Default::default() vs new(); strict subset of :453 (Default echo)
crates/engine_core/src/render_manager.rs:539 test_camera_access — setter echo; viewport contract already at :593
crates/engine_core/src/assets/sprite_sheet.rs:344 test_cache_serves_a_sidecar_filter — filter resolution asserted at :289, caching at :353 (duplicate)
crates/engine_core/src/particles/manager.rs:200 new_pool_has_no_alive_particles — Default ctor + DEFAULT_CAPACITY constant echo
crates/engine_core/src/particles/particle.rs:158 config_builder_chains — five with_*(v) then assert v back (builder echo)
crates/engine_core/src/particles/emitter.rs:71 new_emitter_is_active — ctor args read back (constructor echo)
crates/engine_core/src/particles/emitter.rs:79 pause_resume — sets `active` and reads it; pause's observable effect is proved at particles/system.rs:81 (setter echo)
crates/engine_core/src/assets.rs:445 test_asset_config_default — AssetConfig::default() field echo
crates/engine_core/src/assets.rs:462 test_asset_config_from_game_config_defaults_to_linear_filter — same assertion as :445 and game_config.rs:234 (duplicate)
crates/engine_core/src/assets.rs:487 test_asset_error_display — asserts thiserror's derived Display contains its own payload; discriminates no variant (derive echo)
crates/engine_core/src/texture_ref.rs:205 test_solid_color_path_is_resolvable_on_load — :213 already asserts the sentinel check (duplicate)
crates/engine_core/src/pause.rs:226 default_labels_match_builtin_english — asserts PauseMenuLabels::default() equals the consts it is built from (Default echo)
crates/engine_core/src/menu_input.rs:131 test_read_on_idle_handler_reports_nothing — all-false struct; negatives covered at :196 (constructs-and-asserts-default)
crates/engine_core/src/grid/grid_mesh.rs:349 grid_construction_sizes — node/spring counts; topology.rs:200 asserts the same formula over two sizes (duplicate)
crates/engine_core/src/grid/grid_mesh.rs:357 border_nodes_are_pinned — checks nodes[0] and nodes[5]; topology.rs:240 checks EVERY node (weaker duplicate)
crates/engine_core/src/game_loop_manager.rs:121 test_game_loop_manager_creation — new() then three zeroed getters (constructor echo)
crates/engine_core/src/ui_element_system.rs:135 draws_panels_buttons_and_labels — draw_list().len() > before; content covered at :172 and :244 (draw-list length)
crates/engine_core/src/scene_data_tests.rs:8 test_editor_settings_serialization — contained in :23 (duplicate)
crates/engine_core/src/scene_data_tests.rs:60 test_scene_data_serialization — asserts only name + entities.len() after round-trip; field-level round-trip is scene_serializer_roundtrip_tests.rs:46 (duplicate)
crates/engine_core/src/scene_data_tests.rs:102 test_prefab_with_overrides — asserts the RON string contains "Enemy"/"enemy1"; override semantics live in tests/prefab_spawning.rs:92 (substring smoke)
crates/engine_core/src/scene_data_tests.rs:156 test_physics_components — asserts the RON string contains "RigidBody"/"Collider"; extraction covered at scene_serializer_tests.rs:204/:244 (substring smoke)
crates/engine_core/src/game_config.rs:207 test_game_config_locale_defaults_and_builders — Default field echo; tests no builder despite the name
crates/engine_core/src/game_config.rs:234 test_game_config_defaults_to_linear_texture_filter — one-line Default echo, also asserted at :228 and assets.rs:445
crates/engine_core/src/chaos_mode.rs:64 default_is_normal — Default echo
crates/engine_core/src/chaos_mode.rs:69 all_variants_have_nonempty_labels — label() non-empty, no downstream contract (label echo)
crates/engine_core/src/chaos_mode.rs:100 all_covers_four_distinct_variants — ALL.len() via a HashSet of labels (ALL.len() echo)
crates/engine_core/src/chaos_theme.rs:115 test_struct_update_override_keeps_rest_of_palette — tests Rust's `..base` syntax, not production code; already a doc example at chaos_theme.rs:8-17
crates/engine_core/src/scene_manager.rs:87 test_scene_manager_creation — new() then is_empty()/len()==0 (constructor echo)
crates/engine_core/src/scene_manager.rs:124 test_scene_manager_active_mut — takes active_mut() and reads a name; nothing is mutated (getter echo)
crates/engine_core/src/scene_manager.rs:138 test_scene_manager_scenes_access — lengths after two pushes; :103 covers the stack (getter echo)
crates/engine_core/src/scene_loader.rs:332 test_entity_tag_component_type_name — its only downstream contract (override matching) is covered by :338 (type_name echo)
crates/engine_core/src/input_settings_io.rs:243 save_creates_nested_parent_directories — delegates to save_store::write, whose mkdir -p is asserted at save_store.rs:187 (cross-module duplicate)
crates/engine_core/src/glyph_texture_cache.rs:137 fresh_cache_starts_empty — new() then textures().is_empty() (constructor echo)
crates/engine_core/src/behavior_runner/mod.rs:352 test_behavior_runner_creation — new() then named_entities.is_empty() (constructor echo)
crates/engine_core/src/behavior_runner/mod.rs:358 test_set_named_entities — insert one name, assert contains_key (setter echo)
crates/engine_core/src/window_manager.rs:242 test_window_manager_resize — default size echo then resize()/size() echo
crates/engine_core/src/ui_manager.rs:50 test_ui_manager_creation — no assert at all (verified: body is `let _manager = UIManager::new();` plus a comment) (constructs and returns)
```

### engine_core — `tests/**` (10)

```
crates/engine_core/tests/init.rs:4 test_init — init() is `log::info!; Ok(())`; asserts is_ok() then re-matches the same Result (constructs and returns)
crates/engine_core/tests/init.rs:19 test_engine_error — constructs two EngineError variants and asserts the payload back plus thiserror's Display (derive echo)
   -> the file is now empty; delete crates/engine_core/tests/init.rs
crates/engine_core/tests/timing.rs:6 test_timer_creation — Timer::new() then zero getters (constructor echo)
crates/engine_core/tests/lifecycle.rs:7 test_lifecycle_creation — :67 starts from Created and covers it (constructor echo + duplicate)
crates/engine_core/tests/lifecycle.rs:99 test_lifecycle_concurrent_initialization — exactly the begin_initialization().is_err() assertion inside :67; no concurrency despite the name (duplicate)
crates/engine_core/tests/scene_lifecycle.rs:145 test_scene_entity_management — tests ecs World CRUD, nothing Scene-specific (another crate's tests)
crates/engine_core/tests/scene_lifecycle.rs:209 test_multiple_scenes — runs :7's sequence on two instances, asserts nothing about their interaction (duplicate)
crates/engine_core/tests/prefab_spawning.rs:66 test_instance_retains_prefab_table — positive proved by :74, negative by :114 (getter echo)
crates/engine_core/tests/behavior_optimization.rs:14 test_behavior_runner_no_excessive_cloning — the name claims allocation behavior; asserts only that 10 components still exist after 100 updates ("doesn't panic")
crates/engine_core/tests/scene_loader_parse.rs:160 test_parse_entity_tag_component — scene_serializer_tests.rs:310 already parses EntityTag back out of RON (duplicate)
```

### input (15)

```
crates/input/tests/input_handler.rs:4 test_input_handler_creation — Default echo; identical assertions live in tests/keyboard.rs:4, tests/mouse.rs:4, tests/gamepad.rs:80. Opens with a construct-and-discard `let _input_handler`
crates/input/tests/input_handler.rs:25 test_keyboard_access — tests the keyboard()/keyboard_mut() getters; the press/just-press behavior is tests/keyboard.rs:16 (getter test)
crates/input/tests/input_handler.rs:50 test_mouse_access — the delta-suppression scenario is verbatim tests/mouse.rs:20 (duplicate)
crates/input/tests/input_handler.rs:80 test_gamepad_access — duplicate of tests/gamepad.rs:80 + :90 through a getter
crates/input/tests/input_handler.rs:108 test_input_handler_update — duplicate of tests/input_event_queue.rs:44; lines 134-139 also repeat the same two asserts twice
   -> the file is now empty; delete crates/input/tests/input_handler.rs
crates/input/tests/keyboard.rs:4 test_keyboard_state_creation — byte-for-byte the "INITIAL STATE" block of :16 (Default echo)
crates/input/tests/keyboard.rs:118 test_convert_physical_key — no code and no assertion, only comments (verified) (constructs and returns). convert_physical_key IS testable — see gap P3
crates/input/tests/mouse.rs:4 test_mouse_state_creation — Default echo; every field re-asserted by :20, :84, :122
crates/input/tests/mouse.rs:148 test_multiple_mouse_buttons — per-button independence is a ButtonTracker property, covered by src/button_tracker.rs and tests/keyboard.rs:51 (duplicate)
crates/input/tests/gamepad.rs:4 test_gamepad_state_creation — Default echo
crates/input/tests/gamepad.rs:17 test_gamepad_button_press_and_release — GamepadState.buttons is a ButtonTracker<GamepadButton>; tracker lifecycle re-run at a delegation site, covered by src/button_tracker.rs:87/:106/:115 (duplicate)
crates/input/tests/gamepad.rs:55 test_gamepad_axis — update_axis then read back; the real axis contract is src/gamepad.rs:227/:252/:264 (setter/getter echo)
crates/input/tests/gamepad.rs:80 test_gamepad_manager_creation — new() then get_gamepad(0).is_none() (Default echo)
crates/input/tests/input_mapping.rs:141 test_default_bindings_preset — subsumed by :150, :162, :172 (a bindings(a).contains(s) assert implies has_binding(a)) (duplicate)
crates/input/tests/input_handler_integration.rs:167 test_custom_action_enum — the generic-over-a-game-enum property is compile-time and already exercised by tests/input_mapping.rs; the runtime half repeats :8 and :42 (duplicate)
```

### physics (7)

```
crates/physics/src/physics_system/tests.rs:13 test_physics_system_creation — echoes new()'s literals; the timestep's real contract is pinned behaviorally by test_catch_up_steps_are_capped_after_a_stall (constructor echo)
crates/physics/src/physics_system/tests.rs:20 test_physics_system_custom_config — with_config(gravity).gravity() is a pure passthrough getter (constructor echo)
crates/physics/src/physics_world/tests.rs:13 test_physics_world_creation — Default echo, counts are 0
crates/physics/src/presets.rs:130 test_rigid_body_presets — re-lists player_platformer()'s literals from 115 lines above (preset literal echo)
crates/physics/src/presets.rs:138 test_collider_presets — re-lists player_box friction 0.8 / bouncy restitution 0.9 (preset literal echo)
crates/physics/src/presets.rs:147 test_physics_config_presets — re-lists platformer() gravity -980 / top_down() gravity ZERO (preset literal echo)
crates/physics/src/lib.rs:64 test_full_physics_workflow — triple duplicate: the crate doc example at lib.rs:15-38 asserts the falling body, physics_system/tests.rs:150 asserts it again, and the "ground should not move" half duplicates physics_system/tests.rs:174 (duplicate)
```

> On the three preset tests: if the tuned numbers are judged a real contract, collapse all
> three into one table-driven `test_preset_values_are_the_documented_tuning` rather than
> keeping three files' worth of literal re-listing.

### renderer (31)

```
crates/renderer/src/sprite_data.rs:291 test_sprite_vertex_new — position/tex_coords/color echoed back (constructor echo)
crates/renderer/src/sprite_data.rs:322 test_sprite_instance_new — all six args echoed back; the one real bit (emissive==0.0) moves into :379 (constructor echo)
crates/renderer/src/sprite_data.rs:342 test_sprite_instance_with_emissive — with_emissive(..,2.5) then emissive==2.5 (builder echo)
crates/renderer/src/sprite_data.rs:397 test_camera2d_default — six Camera::default() field asserts; the type is common::Camera (Default echo)
crates/renderer/src/sprite_data.rs:408 test_camera2d_new — constructor args echoed (constructor echo)
crates/renderer/src/sprite_data.rs:475 test_camera2d_view_projection_combines_both — the expectation is verbatim the production body at common/src/camera.rs:118 (reimplements production)
crates/renderer/src/sprite_data.rs:488 test_camera2d_screen_to_world_center — duplicate of common/src/camera.rs:227 with a looser tolerance (1.0 px vs 0.001) (weaker cross-crate duplicate)
crates/renderer/src/sprite_data.rs:497 test_camera2d_world_to_screen_origin — the inverse of :488; covered by common/src/camera.rs:237 (cross-crate duplicate)
crates/renderer/src/sprite_data.rs:523 test_camera_uniform_from_camera — the expectation is the production body of CameraUniform::from_camera (common/src/camera.rs:194) (reimplements production)
crates/renderer/src/sprite/batch.rs:169 test_sprite_batch_new — handle/empty/!sorted echoed from the constructor
crates/renderer/src/sprite/batch.rs:178 test_sprite_batch_add_instance — len()==1, !sorted; both covered by :255 and :281 (count echo)
crates/renderer/src/sprite/batch.rs:194 test_sprite_batch_add_instances — len()==3 after adding 3 (count echo)
crates/renderer/src/sprite/batch.rs:255 test_sprite_batch_len_and_is_empty — len/is_empty on a Vec wrapper (accessor echo)
crates/renderer/src/sprite/batch.rs:295 test_sprite_batcher_new — sprite_count()==0, batches().is_empty() on a #[derive(Default)] struct (Default echo)
crates/renderer/src/sprite/batch.rs:302 test_sprite_batcher_add_sprite — sprite_count()==1; subsumed by :322 and :391 (count echo)
crates/renderer/src/sprite/batch.rs:310 test_sprite_batcher_add_sprites — subsumed by :322, which asserts the same three sprites' grouping (duplicate)
crates/renderer/src/sprite/batch.rs:391 test_sprite_batcher_sprite_count — three adds, three count asserts (count echo)
crates/renderer/src/sprite/batch.rs:445 test_sprite_batcher_batches_mutable — proves batches_mut() returns &mut, which the compiler proves (accessor echo)
crates/renderer/src/texture.rs:401 test_texture_handle_new — TextureHandle::new(42).id == 42 (constructor echo)
crates/renderer/src/texture.rs:413 test_texture_handle_equality — derived PartialEq (derive echo)
crates/renderer/src/texture.rs:423 test_texture_handle_hash — derived Hash via a HashMap round trip (derive echo)
crates/renderer/src/texture.rs:436 test_texture_handle_copy — derived Copy, compiler-enforced (derive echo)
crates/renderer/src/texture.rs:445 test_texture_load_config_default — format.is_none() (Default echo)
crates/renderer/src/texture.rs:451 test_texture_load_config_with_format — struct-update literal echoed back (constructor echo)
crates/renderer/src/texture.rs:462 test_sampler_config_default — eleven Default field asserts; the one downstream consumer is covered by texture_filter.rs:64/72/80 (Default echo)
crates/renderer/src/texture.rs:477 test_sampler_config_custom — struct-update literal echoed back (constructor echo)
crates/renderer/src/texture_filter.rs:59 test_texture_filter_defaults_to_linear — the real default-on-deserialize contract lives in engine_core::texture_filter_serde (Default echo)
crates/renderer/src/bloom.rs:549 bloom_config_defaults_are_sane — threshold > 0.0, intensity > 0.0, blur_iterations >= 1; a threshold of 0.0001 passes (is-finite-class assert)
crates/renderer/src/render_targets.rs:144 bloom_dimensions_halve_surface — computes (1920 / BLOOM_DOWNSAMPLE).max(1) IN THE TEST and asserts it equals 960; bloom_width() is never called (verified) (reimplements production, invokes no production code)
crates/renderer/src/render_targets.rs:153 bloom_dimensions_never_zero — same shape with 1u32 (reimplements production)
crates/renderer/src/line_pipeline.rs:279 line_vertex_new_populates_fields — position/color/emissive echoed from the constructor (constructor echo)
```

### ui (19)

```
crates/ui/src/context/tests.rs:7 test_ui_context_with_theme — light theme's button bg differs from default's; theme-constant echo, no downstream contract
crates/ui/src/context/tests.rs:15 test_ui_context_set_theme — same assertion as :7 via a different setter (duplicate)
crates/ui/src/context/tests.rs:38 test_ui_context_panel — panel() forwards bounds+theme to draw_list.panel; the test asserts only that width/height passed through (pass-through echo)
crates/ui/src/context/tests.rs:51 test_ui_context_rect — assert_eq!(draw_list().len(), 1) and nothing else (verified) (draw-list length)
crates/ui/src/context/tests.rs:58 test_ui_context_circle — assert_eq!(draw_list().len(), 1) and nothing else (verified) (draw-list length)
crates/ui/src/context/tests.rs:65 test_ui_context_hit_test — hit_test is bounds.contains(point); common/src/rect.rs:200 asserts the identical points (cross-crate duplicate)
crates/ui/src/context/tests.rs:92 test_ui_context_label_without_font — identical assertion to :24, minus the position check (duplicate)
crates/ui/src/context/tests.rs:121 test_font_rendering_retry_after_font_load — the comment concedes no font is loaded; both frames assert the TextPlaceholder that :106 already asserts (asserts nothing new)
crates/ui/src/context/tests.rs:149 test_text_align_default — TextAlign::default() == Left (Default echo)
crates/ui/src/context/tests.rs:174 test_ui_context_clip_rect — assert_eq!(len(), 3); draw/tests.rs:168 asserts the same three commands AND their content one layer down (draw-list length + duplicate)
crates/ui/src/interaction/tests.rs:200 test_interaction_manager_state — subsumed by :213 (retention across end_frame) and :228 (edit buffer retained) (duplicate)
crates/ui/src/draw/tests.rs:51 test_draw_list_text_with_data — builds a TextDrawData, pushes it, asserts the literals back; no transformation happens (tests the fixture)
crates/ui/src/draw/tests.rs:99 test_draw_list_clear — subsumed by :274 test_flush_is_idempotent_and_clear_resets_stack (duplicate)
crates/ui/src/draw/tests.rs:110 test_draw_command_depth — constructs DrawCommand::Rect{depth:5.0} and asserts depth()==5.0 (tests the fixture)
crates/ui/tests/ui_interaction_debug.rs:232 test_input_timing_with_game_loop_order — same button and click as :12 collapsed into one frame; the rest is the input crate's end_frame contract (duplicate)
crates/ui/src/font/glyph_cache.rs:205 test_rasterized_glyph — constructs a RasterizedGlyph literal, asserts its own literals back (tests the fixture)
crates/ui/src/style.rs:288 test_color_reexport_works — Color::from_hex(0xFF0000).r ≈ 1.0; common/src/color.rs:219 owns this (cross-crate duplicate)
crates/ui/src/font/mod.rs:209 test_font_manager_metrics_no_font — a HashMap::get miss; context/tests.rs:155 and :212 cover the behavior-level contract (duplicate)
crates/ui/src/font/layout.rs:133 test_text_layout — constructs a TextLayout literal and asserts its own 100.0/16.0; invokes zero production code while layout_text/measure_text in the same file are untested (tests the fixture — see gap U2)
```

### audio (4)

```
crates/audio/src/manager/tests.rs:25 test_clamp_helpers_enforce_valid_ranges — clamp_volume/clamp_speed are one-line private wrappers; the test re-states .clamp(0.0,1.0)/.max(0.1) inline and the public behavior is asserted at :192 (reimplements production)
crates/audio/src/manager/tests.rs:98 test_disabled_manager_music_controls_are_safe — calls stop/pause/resume/update with no asserted effect; its one real assertion is :212 ("doesn't panic")
crates/audio/src/manager/tests.rs:173 test_stop_on_unknown_handle_is_noop — asserts active_sound_count()==0 on a disabled manager, where the count is structurally always 0 (:82 asserts exactly that); the assertion cannot fail (vacuous)
crates/audio/src/manager/tests.rs:181 test_stop_and_stop_all_are_safe_when_nothing_plays — same vacuous count on a disabled manager (vacuous)
```

The rest of the audio suite is healthy — confirmed, not assumed. The typed-error paths
(`AudioError::IoError` vs `DecodeError`), volume-bus multiplication, music/SFX separation,
and the gesture-gated `enable_output` upgrade all carry real assertions.

---

## 3. MERGE groups

### editor (41 tests → 16)

1. `context/tests.rs:290 + :297 + :305` (title-bar clean/dirty/untitled-dirty) → one table `test_title_bar_text_reflects_scene_name_and_dirty_state[(path, dirty) -> expected]`.
2. `context/tests.rs:69 + :176` → `test_update_layout_wires_the_viewport_so_screen_and_world_agree`.
3. `commands/tests.rs:171 + :463` → `test_delete_undo_restores_every_captured_component`.
4. `hierarchy_tests.rs:123 + :139` → `test_expand_state_is_per_entity_and_survives_other_rows`.
5. `hierarchy_tests.rs:168 + :178 + :188` → table `test_display_name_falls_back_by_component_priority`.
6. `viewport/tests.rs:22 + :33` → `test_viewport_center_maps_to_the_camera_position_both_ways`.
7. `viewport/tests.rs:59 + :71` → table `test_visible_world_bounds_scale_inversely_with_zoom[zoom -> bounds]`.
8. `viewport/tests.rs:189 + :199 + :208` → one test looping the three poses through `assert_overlay_matches_render_camera`.
9. `selection.rs:280 + :293` → `test_set_primary_only_accepts_an_already_selected_entity`.
10. `dock/tests.rs:80 + :96` → table `test_edge_panels_lay_out_against_their_own_edge`.
11. `command_api/tests.rs:162 + :257` → keep :257 (a superset), fold in the position-value assert.
12. `stored_component/tests.rs:198 + :211` → `test_category_table_and_kind_category_agree_both_ways`.
13. `world_snapshot/tests.rs:96 + :123 + :144` → `test_snapshot_round_trips_component_values` (one entity carrying all of them).
14. `theme/tests.rs:18 + :49 + :58` → table `test_color_roles_that_must_read_apart`. Keep `:26` (play-state borders) separate — it asserts channel dominance, a different rule.
15. `row_layout.rs` ellipsize trio → table `test_ellipsize_fits_the_budget[(label, budget) -> output]`.
16. `commands/dirty_tests.rs:33 + :44` → `test_every_record_path_marks_the_scene_dirty`.
17. `inspector.rs:272 + :281 + :287` → table `test_format_simple_value_renders_each_json_scalar`.
18. `editable_inspector.rs:506 + :514` → `test_edit_result_reports_change_and_falls_back`.

### editor_integration (21 → 8)

1. `entity_ops_tests.rs:12 + :23 + :34` → `test_create_empty_gets_transform_name_position_and_selection`.
2. `entity_ops_tests.rs:43 + :53 + :63 + :77` → table over `(factory, expected components)`.
3. `editor_game/tests.rs:21 + :28` → table `[(640,480)→(1024,720), (1920,1080)→unchanged]` for `clamp_editor_window_size`.
4. `editor_game/tests.rs:135 + :149 + :269 + :281` → `test_new_scene_resets_world_and_editor_state`.
5. `panel_renderer/tests.rs:54 + :79 + :104 + :129` → one table-driven writeback test (Sprite / RigidBody / Collider / AudioSource); keep `:8` as the canonical transform case.
6. `picking_tests.rs:433 + :444 + :455` → `test_pickables_require_both_sprite_and_global_transform`.
7. `play_guard_tests.rs:27 + :44` → table over `[Playing, Paused]`.
8. Cross-file: the deleted `editor_game/tests.rs:426/:455` content already lives in `camera_follow_tests.rs`, which becomes the single home for camera-split behavior.

### ecs (18 → 8)

1. `component_registry/tests.rs:303` → fold `GridBackdrop` into the builtin-name list at `:209`.
2. `state_machine.rs:389 + :440` → `test_hierarchical_parent_is_derived_from_the_state`.
3. `resource.rs:154 + :164` → `test_remove_returns_the_resource_and_none_when_absent`.
4. `tilemap.rs:131 + :161` → `test_only_non_zero_tiles_yield_instances` (the all-zero map is a row, not a test).
5. `ui_components.rs:286 + :312` → fold the three `visible` defaults into `test_serde_defaults_fill_missing_fields`.
6. `tests/world.rs:246 + :260 + :292` → `test_spawn_attaches_every_with_component_and_keeps_entities_independent`.
7. `tests/sprite_components.rs:301 + :314 + :323 + :332` → one table-driven `test_component_meta_field_order_matches_the_inspector` over the four types. Field ORDER is the real contract (the inspector renders in that order), so keep the exact arrays — just stop writing them four times.
8. `tests/entity_generation.rs:33 + :50` → `test_generated_ids_and_references_validate_only_their_own_generation`.

### engine_core (42 → 16)

`M1` `sheet_file.rs:194 + :206` → `omitted_optional_fields_take_their_documented_defaults`.
`M2` `assets.rs:453 + :471 + :478` → table `asset_config_from_game_config_maps_filter_and_base_path`.
`M3` `assets.rs:493 + :505 + :513` → table `rgba_validation_accepts_exact_length_and_names_the_expected_size`.
`M4` `texture_ref.rs:155 + :167 + :173 + :179` → table `parse_hex_color_reads_6_and_8_digit_forms_and_rejects_the_rest`.
`M5` `texture_ref.rs:184 + :190` → `solid_color_path_writes_alpha_only_when_translucent` (keep `:196` round-trip separate).
`M6` `menu_input.rs:110 + :116 + :121 + :127` → table `navigate_wraps_prefers_up_and_survives_an_empty_list`.
`M7` `chaos_mode.rs:76 + :84 + :92` → one table over `ChaosMode::ALL` asserting the `(is_insane, is_ridiculous, is_insiculous)` triple.
`M8` `game_config.rs:239 + :247 + :254` → `texture_filter_wire_format_is_stable`.
`M9` `debug.rs:198 + :214` → `circle_outline_closes_a_ring_of_CIRCLE_SEGMENTS_vertices_on_radius`.
`M10` `scene_serializer_tests.rs:244 + :483` → one collider-shape table (Circle + Box).
`M11` `scene_serializer_tests.rs:204 + :507` → one rigid-body table (Dynamic-with-all-fields + Static + Kinematic).
`M12` `scene_data_tests.rs:23 + :46` → `editor_settings_round_trip_and_absent_field_reads_none`. After the four deletions this leaves the file with one test; fold it into `scene_serializer_roundtrip_tests.rs` and delete the file.
`M13` `tests/scene_loader_parse.rs:60 + :82 + :104 + :131` → table `sprite_fields_default_when_omitted_and_parse_when_explicit`.
`M14` `tests/timing.rs:85 + :118` → `seconds_accessors_match_duration_accessors_and_accumulate`.
`M15` `tests/scene_lifecycle.rs:115 + :239` → `invalid_transitions_are_refused_and_leave_the_state_untouched`.
`M16` `tests/sprite_animation_scene.rs:157 + :241` (optional) → one test asserting the paused-animation contract on both the serializer and load halves.

> Deliberate duplication, do NOT merge: `sheet_file.rs:206` and
> `tests/sprite_animation_scene.rs:392` both assert `ClipData`'s `looping` serde default.
> Same DTO, two wire surfaces. If one must go, keep the scene one.

### ui (21 → 10)

1. `context/tests.rs:24 + :106` (+ deleted `:92`) → table `test_label_emits_placeholder_with_text_position_color_and_size`.
2. `context/tests.rs:252 + :260 + :268` → one `measure_text` table.
3. `context/tests.rs:232 + :242` → `test_label_centered_offsets_by_half_the_measured_width`.
4. `context/tests.rs:162 + :275` → one alignment table over Left/Center/Right.
5. `interaction/tests.rs:4 + :14 + :24` → `test_widget_id_is_stable_and_collision_free`.
6. `interaction/tests.rs:188 + :246` → `test_focus_set_and_clear_tracks_the_focused_widget`.
7. `draw/tests.rs:8 + :24` → one Rect-command test with `corner_radius` as a parameter.
8. `context/scrub_tests.rs:267 + :286` → assert value AND `out_of_range` in one test.
9. `input_state.rs:297 + :305 + :313 + :320 + :330` → one table over `(KeyCode, shift) -> Option<char>`.
10. `font/glyph_cache.rs:178 + :196` → `test_eviction_boundary`.

### renderer (15 → 7)

11. `sprite_data.rs:304 + :312` → `test_sprite_vertex_matches_shader_layout` (both assert 36).
12. `sprite_data.rs:356 + :371` → `test_sprite_instance_matches_shader_layout` (both assert 76).
13. `sprite_data.rs:418 + :432 + :447` → table `view_matrix` over (camera, world point, expected view point).
14. `scissor.rs:112 + :130` → one test with both rows.
15. `device_status.rs:73 + :78 + :93` → `test_latch_is_one_way`. Keep `:85` (clone sharing) separate.
16. `texture_filter.rs:64 + :72` → one table over `[Linear, Nearest]`.
17. `bloom.rs:558 + :564` → `test_uniform_structs_match_shader_layout` (both 16 bytes).

### physics (9 → 3)

- `physics_world/tests.rs:20 + :33 + :48` → `test_body_and_collider_lifecycle_add_then_remove`.
- `components.rs:460 + :468 + :476 + :486` → `test_collision_event_membership_is_order_independent`.
- `components.rs:497 + :506` → `test_collision_event_other_returns_the_partner_or_none`.

### input (11 → 5)

- `tests/input_mapping.rs:150 + :162 + :172` → table `test_default_preset_bindings` over `[(action, source)]`.
- `tests/gamepad.rs:90` → fold into `src/gamepad.rs:277`, which already covers register ×2 / unregister / ids.
- `tests/input_event_queue.rs:6 + :21` → `test_queued_events_apply_only_on_process` (one before/after test).
- `src/manager` audio (below) aside, the file `tests/gamepad.rs` shrinks to its one unique test, `:131 test_gamepad_manager_update` — the only assertion that `clear_frame_state` fans out to child pads.

### audio (4 → 2), common (2 → 1)

- `audio/src/manager/tests.rs:7 + :16` → `test_sound_settings_clamp_volume_and_floor_speed`. Both are real contracts (a separate impl from the manager helpers, `sound.rs:59/:69`); merge for tidiness only.
- `common/src/color.rs:211 + :219` → one conversion table; `from_rgb8(255,128,0)` and `from_hex(0xFF8000)` assert identical values.

---

## 4. RENAME list

```
crates/editor/src/context/tests.rs:16 test_editor_context_set_tool -> test_set_tool_syncs_the_gizmo_mode
crates/editor/src/context/tests.rs:37 test_editor_context_camera -> test_pan_accumulates_and_zoom_camera_multiplies
crates/editor/src/context/tests.rs:129 test_editor_context_play_state -> test_play_session_enters_pauses_and_exits
crates/editor/src/hierarchy_tests.rs:106 test_toggle_collapse -> test_toggle_expanded_flips_the_row_between_expanded_and_collapsed
crates/editor/src/viewport/tests.rs:11 test_viewport_set_bounds -> test_viewport_size_and_center_derive_from_bounds
crates/editor/src/viewport/tests.rs:147 test_viewport_reset_camera -> test_reset_camera_immediate_returns_to_origin_at_unit_zoom
crates/editor/src/viewport/tests.rs:246 test_viewport_focus_on -> test_focus_on_targets_the_requested_point
crates/editor/src/selection.rs:238 test_selection_toggle -> test_toggle_adds_an_unselected_entity_and_removes_a_selected_one
crates/editor/src/selection.rs:250 test_selection_clear -> test_clear_drops_every_entity_and_the_primary
crates/editor/src/grid.rs:449 test_lod_grid_size -> test_lod_doubles_cell_size_as_zoom_halves
crates/editor/src/grid.rs:506 test_calculate_grid_lines -> test_grid_lines_span_the_bounds_on_grid_multiples
crates/editor/src/picking/tests.rs:44 test_pick_single_entity -> test_click_inside_an_entity_picks_it
crates/editor/src/picking/tests.rs:84 test_pick_miss -> test_click_off_every_entity_picks_nothing
crates/editor/src/picking/tests.rs:147 test_pick_in_rect -> test_screen_rect_picks_only_entities_inside_it
crates/editor/src/stored_component/tests.rs:46 test_capture_empty_entity -> test_bare_entity_captures_no_components

crates/editor_integration/src/editor_game/tests.rs:269 test_load_scene_resets_selection -> test_new_scene_clears_selection  (it never loads a scene)
crates/editor_integration/src/editor_game/tests.rs:281 test_physics_settings_preserved_on_new -> test_new_scene_clears_physics_settings  (the name asserts the opposite of the body)

crates/ecs/src/behavior.rs:448 test_entity_tag -> test_entity_tag_matches_only_its_own_tag
crates/ecs/src/hierarchy_extension.rs:361 test_get_ancestors -> test_get_ancestors_are_ordered_nearest_first
crates/ecs/src/event.rs:242 test_flush_preserves_queue_allocations -> test_emit_after_flush_starts_a_fresh_frame
crates/ecs/tests/world.rs:184 test_world_clear_removes_entities_and_components -> keep (already behavioral)
crates/ecs/tests/entity.rs:21 test_entity_active_state -> test_set_active_toggles_the_entity_between_active_and_inactive

crates/engine_core/src/render_manager.rs:453 test_render_manager_new -> uninitialized_manager_reports_no_device_and_no_fatal
crates/engine_core/src/render_manager.rs:555 test_sync_main_camera_copies_main_camera_entity_position -> sync_main_camera_copies_position_only
crates/engine_core/src/render_manager.rs:576 test_sync_main_camera_is_noop_without_main_camera_entity -> non_main_cameras_never_drive_the_render_camera
crates/engine_core/src/render_manager.rs:593 test_resize_without_renderer -> resize_updates_the_camera_viewport_without_a_renderer
crates/engine_core/src/game_loop_manager.rs:129 test_game_loop_manager_update -> update_returns_the_elapsed_delta_and_counts_the_frame
crates/engine_core/src/game_loop_manager.rs:141 test_game_loop_manager_multiple_updates -> total_time_is_the_sum_of_every_delta
crates/engine_core/src/game_loop_manager.rs:197 test_game_loop_manager_reset -> reset_clears_delta_total_and_frame_count
crates/engine_core/src/scene_manager.rs:94 test_scene_manager_with_scene -> with_scene_starts_with_that_scene_active
crates/engine_core/src/scene_manager.rs:103 test_scene_manager_push_pop -> pop_returns_the_top_scene_and_reactivates_the_one_below
crates/engine_core/src/scene_loader.rs:338 test_merge_components -> override_layer_replaces_the_prefabs_component_of_the_same_type
crates/engine_core/src/scene_data_tests.rs:46 test_scene_data_without_editor_settings_backward_compat -> pre_editor_scene_files_load_with_editor_none
crates/engine_core/src/window_manager.rs:263 test_window_manager_logical_physical_size -> physical_size_scales_logical_size_by_the_scale_factor
crates/engine_core/src/particles/particle.rs:181 particle_t_clamped -> t_clamps_to_unit_range_and_reads_one_for_zero_lifetime
crates/engine_core/src/ui_manager.rs:56 test_ui_manager_frame_lifecycle -> end_frame_returns_the_commands_drawn_during_the_frame
crates/engine_core/tests/timing.rs:22 test_timer_reset -> reset_returns_elapsed_and_delta_to_zero
crates/engine_core/tests/timing.rs:48 test_timer_update -> update_records_delta_since_last_update_and_growing_elapsed
crates/engine_core/tests/lifecycle.rs:15 test_lifecycle_initialization -> completed_initialization_is_operational_and_startable
crates/engine_core/tests/lifecycle.rs:30 test_lifecycle_start_stop -> stop_returns_a_running_manager_to_initialized
crates/engine_core/tests/lifecycle.rs:49 test_lifecycle_shutdown -> completed_shutdown_ends_operational
crates/engine_core/tests/behavior_optimization.rs:75 test_behavior_runner_with_physics_integration -> follow_entity_moves_toward_its_named_target  (no physics is involved at all)

crates/renderer/src/sprite_data.rs:304 test_sprite_vertex_bytemuck_cast -> test_sprite_vertex_size_matches_shader_stride
crates/renderer/src/sprite_data.rs:356 test_sprite_instance_bytemuck_cast -> test_sprite_instance_size_matches_shader_stride
crates/renderer/src/sprite/batch.rs:266 test_sprite_batch_clear -> test_clear_empties_instances_and_marks_unsorted
crates/renderer/src/sprite/batch.rs:428 test_sprite_batcher_clear -> test_clear_empties_batches_but_keeps_them_allocated
crates/renderer/src/sprite.rs:179 test_sprite_to_instance -> test_sprite_fields_map_onto_the_gpu_instance
crates/renderer/src/texture.rs:407 test_texture_handle_default -> test_default_handle_is_the_reserved_white_texture

crates/ui/src/context/tests.rs:24 test_ui_context_label -> test_label_emits_placeholder_at_the_given_position
crates/ui/src/context/tests.rs:74 test_ui_context_progress_bar -> test_progress_bar_fill_width_is_the_fraction_of_the_track
crates/ui/src/context/tests.rs:200 test_float_input_draws_box -> test_float_input_draws_background_and_border
crates/ui/src/font/glyph_cache.rs:166 test_glyph_key -> test_glyph_key_quantizes_font_size_to_tenths

crates/physics/src/components.rs:440 test_collider_shapes -> test_capsule_y_half_height_excludes_the_two_cap_radii
crates/common/src/transform.rs:176 test_transform_forward -> test_forward_points_along_the_rotation
```

---

## 5. STRENGTHEN list

### The draw-list-length family (right subject, count-only assert)

```
crates/editor/src/grid.rs:560 test_render_overlay_emits_clipped_lines — now: lines > 2. Should assert the two axis lines' screen endpoints (viewport.world_to_screen of (±400,0) and (0,±300)) and their AxisX/AxisY colors, keeping the clip push/pop asserts.
crates/editor/src/collider_overlay.rs:326 test_render_overlay_emits_line_commands_for_collider_entities — now: lines == 4. Should assert the 4 endpoints are the box's corners mapped through world_to_screen, and the color is colors.solid.
crates/editor/src/selection_outline.rs:237 test_selected_and_hovered_entity_outlined_once_not_twice — now: 4 lines. Should also assert all 4 carry colors.primary, proving hover lost to selection rather than merely not adding lines.
crates/engine_core/src/grid/grid_mesh.rs:430 build_line_vertices_produces_two_per_spring — now: verts.len() == spring_count*2. Should assert each pair's positions equal that spring's two node positions, and at rest every alpha == color.w * rest_alpha_fraction.
crates/engine_core/src/grid/mod.rs:76 test_step_and_emit_pushes_grid_vertices — now: !lines.is_empty(). Should assert lines.len() == grid.spring_count()*2 exactly, and that debug_colliders=false adds nothing beyond that.
crates/engine_core/src/debug.rs:190 box_outline_emits_four_segments — now: lines.len() == 8. Should assert the four corner coordinates from center ± half_extents.
crates/engine_core/src/ui_manager.rs:56 test_ui_manager_frame_lifecycle — now: !commands.is_empty(). Should assert a Text/TextPlaceholder command carrying "Test" at (10,10).
crates/ui/src/context/tests.rs:186 test_float_input_returns_original_without_interaction — the assert_eq!(result, 2.75) IS the contract; the trailing assert!(draw_list().len() >= 2) is noise. Drop the length assert.
crates/ui/src/context/tests.rs:200 test_float_input_draws_box — now: len() >= 3. Should assert the two Rects carry the field's bounds and the text placeholder reads "42.00".
crates/ui/src/context/tests.rs:232 / :242 test_label_centered* — now: !draw_list().is_empty(). Centering is the entire subject. Should assert position.x == center.x - measure_text(text).x / 2.0.
crates/ui/src/context/tests.rs:162 test_ui_context_label_in_bounds — now: len() == 1 with the comment "should not panic". Should assert the placeholder's x for TextAlign::Center sits at bounds.x + (bounds.width - measured)/2.0, as :275 already does for the styled/Left case.
```

### Tests that reimplement production math in the assert

```
crates/engine_core/src/debug.rs:205 capsule_y_outline_includes_sides_and_two_caps — the expected vertex count is computed by reimplementing the production formula in the test. Should assert the two straight sides sit at x = ±radius over the correct half-height, and cap vertices lie on the cap centers' radius.
crates/physics/src/physics_world/tests.rs:294 test_zero_scale_falls_back_to_default_and_produces_finite_positions — the post-step assert is is_finite (the rubric names this), and the meaningful half duplicates :312. Should assert the fallen position matches a 100 px/m step (about -0.136 px after one 1/60 frame), so a wrong-but-finite scale fails.
```

### GPU layout (the rubric's protected category, currently half-protected)

```
crates/renderer/src/sprite_data.rs:312 test_sprite_vertex_desc_attributes — now: attributes.len() == 3, array_stride == 36, step_mode. Should assert each attribute's (offset, format, shader_location). Count and stride stay correct if two offsets are swapped, and the shader then reads colors as positions with no compile error.
crates/renderer/src/sprite_data.rs:371 test_sprite_instance_desc_attributes — same, and worse: eight attributes with hand-written size_of::<[f32; N]>() offsets. Should assert all eight (offset, format, shader_location) triples against WGSL locations 3-10.
crates/renderer/src/sprite_data.rs:379 test_sprite_instance_default_shape_is_plain_quad — now: shape == [0.0; 4] then a with_shape echo. Should also assert emissive == 0.0 (absorbing the deleted :322).
crates/renderer/src/sprite.rs:179 test_sprite_to_instance — asserts position/rotation/scale/color/depth. to_instance also forwards tex_region and emissive and applies .with_shape. Set non-default tex_region, emissive and shape and assert all three land — the animation system writes tex_region every frame and nothing tests that it reaches the GPU instance.
crates/renderer/src/texture.rs:407 test_texture_handle_default — asserts id == 0. The real contract (texture.rs:137 `next_handle: TextureHandle::WHITE.id + 1`) is that default() == WHITE and no allocated handle can ever equal it. Assert both.
```

### Assertions that cannot fail, or that pass on a broken implementation

```
crates/ecs/tests/system_lifecycle.rs:306 test_panic_recovery_in_systems — VERIFIED: SystemRegistry::update_all DOES use catch_unwind (system.rs:213), so this guards a real contract and must NOT be deleted. But it has no assert. Should assert that a normal system added after the panicking one still advances its update_count.
crates/renderer/src/sprite/batch.rs:239 test_sprite_batch_sort_idempotent — passes even if the `if !self.sorted` guard is removed and it re-sorts. Should mutate `instances` out of order directly, call sort_by_depth(), and assert the order was NOT touched.
crates/renderer/src/scissor.rs:140 test_intersect_disjoint_rects_is_empty — asserts only r[2]==0 && r[3]==0; the origin could be anything. Assert the full [u32; 4].
crates/physics/src/physics_world/tests.rs:65 test_step_simulation — the sole assertion is inside `if let Some(..)`, so a None (body never created) passes silently. Use .expect("body exists") and assert a bounded fall.
crates/physics/src/lib.rs:124 test_collision_detection — boxes start 10.0 apart and the assert is distance >= 10.0, which passes with ZERO collision response. Assert strictly greater with a margin, or delete in favour of physics_world/tests.rs:106.
crates/physics/src/physics_system/tests.rs:462 test_reset_body_zeros_velocity_and_sets_position — asserts only vel.length() < 1.0; the name promises the position half and never checks it. Assert Transform2D.position ≈ (100.0, 200.0) after the next update.
crates/engine_core/tests/behavior_optimization.rs:75 — asserts both entities still have Behavior components. Should assert the follower's Transform2D moved toward the player and stops at follow_distance; otherwise FollowEntity could be a no-op and this passes.
crates/engine_core/src/particles/manager.rs:244 overfull_pool_overwrites_oldest — asserts alive_count == 4. Should assert the four survivors are the LAST four spawned, which is the name's actual claim.
crates/ecs/src/behavior.rs:483 test_default_for_variant_wraps_out_of_range_indices — asserts assert_eq!(count, 8), which breaks on every new variant. Should assert only the wrap behavior without pinning the count.
crates/ecs/src/event.rs:242 — asserts type_count() == 1 after flush (an allocation detail). Should assert only that emit-after-flush produces a readable event in the new frame.
crates/ecs/src/ui_components.rs:251 test_anchor_all_index_label_roundtrip — asserts !label().is_empty(). Should assert the actual labels the editor cycle row displays.
crates/ecs/tests/world.rs:66 test_system_management — after ecs/tests/system.rs is deleted this becomes the only "add_system" coverage, and its assert is count-only. Should assert the system actually ran, via a resource marker as test_lifecycle_hooks_receive_world does.
```

### Weak-but-right in editor / editor_integration / engine_core / ui / common

```
crates/editor/src/viewport/tests.rs:246 test_viewport_focus_on — now: pos.x > 0.0 && pos.y > 0.0. Should assert target_camera_position() == (500, 300) exactly.
crates/editor/src/grid.rs:506 test_calculate_grid_lines — now: h_lines.len() >= 5 and "contains something near 0". Should assert the exact vectors [-64, -32, 0, 32, 64].
crates/editor/src/editor_input.rs:512 test_pan_has_multiple_bindings — now: bindings.len() >= 2. Should assert the bindings are exactly Space and the middle mouse button.
crates/editor/src/context/tests.rs:162 test_editor_context_default_panels — now: len == 4 plus four is_some(). Should assert each panel's DockPosition (hierarchy Left, inspector Right, scene Center, assets Bottom) — the default layout IS the contract.
crates/editor/src/editor_preferences.rs:137 test_editor_preferences_roundtrip — asserts correctly but writes to a FIXED path std::env::temp_dir()/test_editor_prefs.json. Two concurrent test binaries race on it. Use tempfile::tempdir() as asset_browser.rs already does.
crates/editor_integration/src/editor_game/tests.rs:498 test_gizmo_scale_undo_restores_transform_and_collider_together — asserts a MacroCommand the TEST builds (that is editor/src/commands/tests.rs:370). Should drive the real path: a GizmoDragState with a collider, mutate both, call commit_gizmo_drag, assert one undo entry restores both. Nothing today proves commit_gizmo_drag records the collider at all.
crates/editor_integration/src/editor_game/api.rs:272 test_answer_api_lines_describe_by_name — asserts describe output (editor command_api coverage). Should assert the blank-line / one-response-per-line envelope explicitly, which is answer_api_lines' own contract.
crates/engine_core/src/game_loop_manager.rs:129 — dt < 0.02 after a 10 ms sleep is fragile on a loaded CI box. Should assert dt >= 0.010 && dt <= MAX_DELTA_TIME.
crates/engine_core/src/scene_manager.rs:94 — now: is_some()/len()==1. Should assert active().name() == "test_scene".
crates/engine_core/tests/scene_loader_parse.rs:30 test_parse_scene_with_prefabs — now: prefabs.len()==1 && contains_key("Enemy"). Should also assert the entity's parsed overrides Transform2D.
crates/engine_core/tests/lifecycle.rs:125 test_lifecycle_wait_for_state — spawns a thread and an mpsc channel that never touch the lifecycle (the comment admits it), then asserts wait_for_state returns for an already-current state. Drop the fixture; add a real waiter — one thread blocked in wait_for_state, released by another thread's transition.
crates/engine_core/tests/scene_lifecycle.rs:7 test_scene_lifecycle_states — correct assertions wrapped around a stray println! debug block (lines 19-26) and duplicated local bindings. Keep the assertions, delete the scaffolding.
crates/ui/tests/ui_interaction_debug.rs:140 test_input_state_from_input_handler — the first two thirds are a real, otherwise-uncovered InputState mapping contract; the last third re-tests InputHandler::end_frame, which is the input crate's job. Keep the mapping asserts, move them to src/input_state.rs, drop the end_frame section.
crates/ui/src/font/glyph_cache.rs:166 test_glyph_key — asserts derived PartialEq over obviously different keys. The non-obvious code is size_tenths: (font_size * 10.0) as u32. Assert 16.0 and 16.04 produce the SAME key while 16.0 and 16.1 differ.
crates/common/src/color.rs:232 test_color_conversions — asserts one field of each of three conversions. Should assert all four channels survive Color -> Vec4 -> [f32;4] -> Color.
crates/common/src/color.rs:226 test_color_lerp — asserts one channel at t=0.5. Should assert all four channels at t = 0.0, 0.5, 1.0.
crates/common/src/transform.rs:176 test_transform_forward — only the identity case. Should assert forward() at a non-zero rotation (90 degrees -> (0,1)).
crates/common/src/transform.rs:184 test_transform_lerp — asserts only position.x. Should assert rotation and scale interpolate too.
```

---

## 6. Duplicated fixtures and helpers

### The single worst offender: the press-frame / release-frame click harness

Written **eleven** times across two crates, in two mutually incompatible shapes. A widget
gesture is press→release across two frames, and every test file re-derives that fact:

- editor: `gizmo/tests.rs:13-38` (`frame`/`press_at`/`move_to`/`release`), `inspector_edit_tests.rs:48 click_through`, `script_editor.rs:25 click_scripts`, `confirm_dialog.rs:8 click_at`, plus inline copies in `toolbar.rs:283/:304` and `play_controls.rs:235`.
- ui: `context/tests.rs:359 focus_float_input` + `:377 type_key`, `context/tests.rs:454 focus_text_input` + `:472 type_key_text` (byte-for-byte the same shape), `context/scrub_tests.rs:13/:30/:37/:43`, `context/scrub_tests.rs:226 type_and_commit` (a sixth copy, inlined "because opts differ per test"), `context/focus_tests.rs:11 frame`, `interaction/tests.rs:38 input_with_mouse`.
- engine_core: `pause.rs:210 frame` and `menu_panel.rs:325 frame` call `end_frame()` first; **`menu_input.rs:139 frame` does not** — same name, divergent semantics. That is exactly the drift a shared helper prevents. `tests/camera_follow.rs:105/:126` is a fourth shape.

One `test_support` module per crate, parameterized on `(widget_fn, opts)`, removes roughly
250 lines and makes "a click is two frames" a single documented fact.

### editor_integration: `DummyGame` — eleven definitions

`struct DummyGame; impl Game for DummyGame {}` is defined in `editor_game/tests.rs:15`,
`picking_tests.rs:499`, `gizmo_drag_tests.rs:14`, `camera_follow_tests.rs:326`,
`play_guard_tests.rs:13`, `api_write_tests.rs:225`, `scene_io_tests.rs:15`,
`shortcuts_tests.rs:272`, `scene_confirm_tests.rs:14`, `time_freeze_tests.rs:256`,
`editor_game/api.rs:250`. Should be one `pub(crate) mod test_support` behind `#[cfg(test)]`.

Also in that crate: `fn test_texture_path_fn(handle: u32) -> String` four times
(`editor_game/tests.rs:167`, `play_guard_tests.rs:20`, `scene_io_tests.rs:145`,
`scene_io_tests.rs:187` as `tex`); `fn spawn_at(world, pos)` byte-identical in
`gizmo_drag_tests.rs:19` and `shortcuts_tests.rs:277`; and `GizmoDragState { … }` hand-built
in `shortcuts_tests.rs:360/:391` and `gizmo_drag_tests.rs:278/:290` while `drag_state_for`
(`gizmo_drag_tests.rs:68`) already exists.

### engine_core: the scene round-trip quartet

| helper | copies |
|---|---|
| `fn test_texture_path(handle) -> String` (handle 0 → `#white`, else `#texture_N`), byte-identical | `scene_serializer_tests.rs:12`, `scene_serializer_roundtrip_tests.rs:12`, `scene_scripts_tests.rs:18`, `scene_dynamic_tests.rs:16` |
| `struct StubResolver` impl `TextureResolver` | `scene_serializer_roundtrip_tests.rs:20`, `scene_scripts_tests.rs:22`, `scene_dynamic_tests.rs:24`, `tests/prefab_spawning.rs:17`, `tests/scene_loader_parse.rs:189` (declared inside the test fn), plus `tests/sprite_animation_scene.rs:22 BareResolver` (a superset that also counts cache clears) |
| `fn roundtrip(world) -> World` (world → `world_to_scene_data` → RON → parse → instantiate) | `scene_serializer_roundtrip_tests.rs:32`, `scene_scripts_tests.rs:32`, `scene_dynamic_tests.rs:35`, `tests/sprite_animation_scene.rs:116` |
| sidecar RON literal | `sheet_file.rs:161 GOLDEN` and `assets/sprite_sheet.rs:271 SIDECAR` are near-identical |

**Temp-file discipline is inconsistent and two copies leak.** Three idioms coexist:
`tempfile::tempdir()` in `achievements/tests.rs:87`, `scores.rs:255`, `save_store.rs:169`;
hand-rolled `std::env::temp_dir()` + PID suffix + manual `cleanup()` in
`input_settings_io.rs:163`; and manual `temp_dir()` with a trailing `remove_dir_all` in
`localization.rs:592` and `scene_serializer_tests.rs:436` — **those last two leak the
directory on a failing assert**, because cleanup is a trailing statement rather than a
`TempDir` guard. `editor/src/editor_preferences.rs:137` is worse: a fixed shared path that
races across concurrent test binaries.

### renderer: a helper that exists and is ignored next door

`sprite/instance_cache.rs:84` defines `fn instance(x: f32)` and `:88 fn batch_with(...)` —
exactly the fixtures `sprite/batch.rs` needs. Yet `batch.rs` writes
`SpriteInstance::new(Vec2::ZERO, 0.0, Vec2::ONE, [0.0,0.0,1.0,1.0], Vec4::ONE, d)` out
**16 times** (lines 180, 197-199, 208-210, 225-227, 241-242, 260, 268-269, 283, 288) and
`Sprite::new(TextureHandle::new(n))` about 20 times. Both files are under
`crates/renderer/src/sprite/`; one shared `#[cfg(test)] mod fixtures` in `sprite/mod.rs`.

### editor: entity and viewport builders

- `fn entity(id: u64) -> EntityId { EntityId::with_generation(id, 1) }` — `hierarchy_tests.rs:9`, `selection.rs:137`, inlined in `context/tests.rs:327` and `selection_outline.rs`.
- `fn setup_entity(world)` — `commands/tests.rs:8` and `commands/dirty_tests.rs:11`; `commands/selection_restore_tests.rs:304` has the same thing named `spawn`.
- `fn named_entity(world, name, ...)` — `command_api/tests.rs:12`, `clipboard.rs:320`, `Rig::spawn_player` in `command_api/write_tests.rs:34`.
- `fn pickable(id, pos, size, depth)` — identical in `context/tests.rs:327` and `selection_outline.rs:206`.
- **"SceneViewport with 800×600 bounds"** — hand-rolled in at least seven places: `context/tests.rs:331`, `viewport/tests.rs` (inline, many), `viewport_input.rs:412` + `:501`, `selection_outline.rs:161`, `collider_overlay.rs` (×2), `grid.rs`. One `fn test_viewport()` would do. `Vec2::new(800.0, 600.0)` appears ~30 times in the ui crate alone.
- `InspectorExtras { drag_drop, texture_display: None, warnings: Vec::new() }` — `inspector_edit_tests.rs:16`, `component_editors/grid_backdrop.rs:7`, inlined in `script_editor.rs:33`, `ui_component_editors.rs:12`, `stored_component/tests.rs:23`.
- Inspector row geometry is mirrored twice: `inspector_edit_tests.rs:29 first_row()` and `script_editor.rs:11 action_button_center()` each recompute `EditableInspector`'s layout math and will drift the moment `EditableFieldStyle` changes.

### physics and input

- physics: the 3-component spawn (`Transform2D` + `RigidBody` + `Collider::box_collider(32,32)` + `initialize` + `update`) is retyped in **14** tests in `physics_system/tests.rs` (`:27, :47, :78, :112, :150, :174, :198, :218, :363, :394, :422, :441, :462`), all 5 in `tests/external_edits.rs`, and both in `src/lib.rs`. `physics_system/tests.rs:254 overlapping_pair()` is the only extracted builder, used by 4 tests. One `spawn_body(world, pos, rb, collider)` removes ~200 lines.
- physics: the collision-pair find closure `|e| (a,b) || (b,a)` is written longhand at `physics_world/tests.rs:135, :169, :181, :223` even though `CollisionEvent::involves` exists and is used at `:384` and `:423` **in the same file**. The test file duplicates production logic it also tests.
- input: `frame(input, &[events])` at `src/player.rs:430` is the only queue-and-process helper, and it lives inside a `#[cfg(test)] mod`, so the integration tests cannot see it — `tests/input_handler_integration.rs` repeats `queue_event(...); process_queued_events();` 30+ times and `tests/input_event_queue.rs` another 10.
- ui: glyph fixtures are triplicated — `glyph_cache.rs:150 dummy_glyph()`, the inline `RasterizedGlyph` at `glyph_cache.rs:205`, and the inline `GlyphDrawData` at `draw/tests.rs:62` are the same zeroed glyph.
- audio: `tiny_wav()` / `write_temp_wav(tag)` (`manager/tests.rs:52`, `:71`) are correctly single-source. Minor: `write_temp_wav` names files by `process::id()` only, so two tests sharing a `tag` in one process would collide. The tags are currently unique but the helper does not enforce it.

---

## 7. Integration tests vs inline unit tests

Short answer: **duplication is real but localized.** Three of the eleven crates have a
`tests/` directory that mostly re-tests inline behavior; the rest are principled splits,
two of them documented in the source.

### Genuinely duplicated — delete or collapse

| file | verdict |
|---|---|
| `crates/ecs/tests/component.rs` (3) | **All 3 duplicated** by `tests/world.rs:30` and `:144`. Delete the file. |
| `crates/ecs/tests/init.rs` (2) | **Both duplicated** by `tests/world.rs:4` and `:30`. "New world has 0 entities, 0 systems" exists three times. Delete the file. |
| `crates/ecs/tests/system.rs` (4) | **All 4** either test a test-local struct or assert `system_count()` with comments conceding nothing is verified. Delete the file. |
| `crates/input/tests/input_handler.rs` (5) | **5/5 duplicated.** Every test duplicates a device-level test in a sibling `tests/` file or `input_event_queue.rs`. Contributes no unique assertion. Delete the file. |
| `crates/input/tests/gamepad.rs` (6) | **5/6 duplicated or Default-echo.** `src/gamepad.rs` owns the real contracts (threshold crossing/re-arm, negative-direction isolation, manager iteration). Only `:131 test_gamepad_manager_update` is unique — the sole assertion that `clear_frame_state` fans out to child pads. The file should shrink to that one test. |
| `crates/engine_core/tests/init.rs` (2) | Nothing to duplicate: `init()` is a two-line no-op and `EngineError` is a plain thiserror enum. Delete the file. |
| `crates/engine_core/tests/behavior_optimization.rs` (3) | **Duplicated in substance.** `:14` and `:75` construct behaviors, run `runner.update`, and assert components still exist; `src/behavior_runner/mod.rs:375/:427/:475` already drives those behaviors and asserts real FSM phases. `:115` (gamepad-vs-Space jump) is unique and stays. |
| `crates/ui/tests/ui_interaction_debug.rs` (7) | **Not a genuine integration test.** Nothing crosses a boundary the inline modules cannot reach — `ui` already depends on `input` in prod and dev, and every type it touches is `pub`. The file name records a 2025 debugging session that was never cleaned up. Detail below. |

### Principled splits — keep as they are

| file | verdict |
|---|---|
| `crates/ecs/tests/hierarchy_dirty.rs` (9) | Keep, and it is the **stronger** side: it asserts recompute counts as well as values. Four inline tests in `src/hierarchy_system.rs` and six in `src/hierarchy_extension.rs` duplicate it and are on the delete list. Only the scaled-parent case (`hierarchy_system.rs:401`) and the ancestor-ordering case (`hierarchy_extension.rs:361`) are unique to the inline files. |
| `crates/ecs/tests/sprite_components.rs` (25) | No overlap with `src/sprite_system.rs`: the integration file tests `SpriteAnimation` in isolation, the inline file tests the system writing `Sprite.tex_region`. Correctly split. |
| `crates/engine_core/tests/camera_follow.rs` (14) | Best file in the crate. `src/behavior_runner/mod.rs` tests Patrol and ChaseTagged only; CameraFollow and look-ahead exist only here. |
| `crates/engine_core/tests/sprite_animation_scene.rs` (15) | Deliberate: `src/scene_serializer_tests.rs:199-201` is a comment explicitly delegating SpriteAnimation coverage to this file. |
| `crates/engine_core/tests/scene_loader_parse.rs` (11) | Deliberate: `src/scene_loader.rs:325` carries a comment splitting public-API parse tests out here and keeping private-method tests inline. One real cross-file duplicate (`:160`). |
| `crates/engine_core/tests/lifecycle.rs` (9) | Not duplicated — `src/lifecycle.rs` has exactly one test (`test_lifecycle_survives_lock_poisoning`). Two *internal* duplicates instead (`:7` and `:99` vs `:67`). |
| `crates/engine_core/tests/scene_lifecycle.rs` (8) | Not duplicated by `src/scene_manager.rs` (that tests the scene *stack*; this tests one Scene's lifecycle). It is an indirect re-test of `LifecycleManager` at a second layer, which is acceptable. `:145` and `:209` are genuine duplicates. |
| `crates/engine_core/tests/prefab_spawning.rs` (5) | Runtime `spawn_prefab` and failure atomicity exist nowhere else. `:92` and `src/scene_loader.rs:338` exercise the override layer at different altitudes; keep both. |
| `crates/physics/tests/external_edits.rs` (5) | Keep. Zero occurrences of `external_edits_pushed_last_update` in `physics/src/**` tests. The GPP-09 contracts exist only here, and it genuinely needs the public `physics::` surface. |
| `crates/physics/tests/ball_brick_bounce.rs` (1) | Keep. Nothing in `physics/src/**` tests CCD or restitution-1.0 reflection. A cross-cutting regression that belongs outside the unit files. |
| `crates/input/tests/input_event_queue.rs` (5) | Keep. The only file asserting deferred-queue semantics (queued ≠ applied until `process_queued_events`) and multi-event ordering. |
| `crates/input/tests/input_handler_integration.rs` (15) | Keep. The only crossing point between `InputMapping<A>` and `InputHandler` device state; `src/input_mapping.rs` has no inline test module at all. Its axis tests sit one layer above `src/gamepad.rs` — a broken `InputSource::GamepadAxis` match arm would pass the src tests and fail these. |
| `crates/input/tests/input_mapping.rs` (13) | Keep. Tests the generic `InputMapping<A>` binding table; `src/player.rs` tests `InputSettings`/`PlayerBindings`. No overlapping assertion. |
| `crates/input/tests/keyboard.rs`, `tests/mouse.rs` | Keep (minus the noted deletes). `src/keyboard.rs` and `src/mouse.rs` have no inline test modules at all. The mouse delta tests (`:20`, `:38`, `:58`) and wheel accumulation (`:122`) are the sole coverage of `MouseState`'s frame-delta model and are genuinely non-obvious math — the strongest tests in that directory. |
| `crates/engine_core/tests/timing.rs` (5) | Not duplicated by `src/game_loop_manager.rs` (different types; `src/timing.rs` has no test module). **But worse: `engine_core::Timer` has zero consumers.** Verified by grep across all engine crates and all six games — it is exported at `lib.rs:88` and `prelude.rs:45` and used by nothing but this test file. (`EffectTimer` in `pickups.rs` is a different type and is used.) 147 lines testing an unconsumed public API. |

### The three-file input-queue question, answered

The three files do **not** test one queue three times — **two of them do.**
`input_event_queue.rs` tests the queue. `input_handler_integration.rs` tests `InputMapping`
evaluation against handler state, a different unit; it *uses* `queue_event`/
`process_queued_events` constantly but never asserts the queuing itself.
`input_handler.rs` tests neither and duplicates both.

### `crates/ui/tests/ui_interaction_debug.rs`, test by test

Verdict: **delete one, move five inline, delete the file.**

1. `:12 test_ui_button_click_detection` — **not duplicated.** The only test asserting `UIContext::button` returns `true` on the release frame. `context/tests.rs:323` drives the identical three frames but asserts `wants_mouse()`, never the return value. Move to `src/context/tests.rs`.
2. `:74 test_ui_slider_interaction` — **not duplicated, and uniquely valuable.** `grep -rn "\.slider("` over `crates/ui/src/` returns exactly one hit: the implementation at `context/widgets.rs:148`. This is the **only** slider test in the crate. Move to `src/context/tests.rs`.
3. `:140 test_input_state_from_input_handler` — **half unique.** The mouse-snapshot mapping is covered nowhere else; the final third tests `InputHandler::end_frame`, which is the input crate's job. Move the mapping half to `src/input_state.rs`, drop the rest.
4. `:185 test_interaction_manager_click_logic` — **not a strict duplicate.** `interaction/tests.rs` asserts `Hovered` (`:93`), `!clicked` under a blocking rect (`:61`), and press→hold→release for `wants_mouse` (`:125`), but **never `result.clicked == true`** — `grep "\.clicked"` finds exactly one assertion in `src/`, the negative one. This is the canonical Normal→Hovered→Active→clicked state machine. Move to `src/interaction/tests.rs`.
5. `:232 test_input_timing_with_game_loop_order` — **delete** (see §2).
6. `:278 test_click_outside_button` — not duplicated inline. Move to `src/context/tests.rs`.
7. `:301 test_click_press_inside_release_outside` — real drag-off-cancels contract, not duplicated inline. Move to `src/context/tests.rs`.

---

## 8. Coverage gaps

Highest-value only. Each is a contract the rubric says should be pinned and is not.

### Editor and editor integration

**I1 — no test drives the production delete or duplicate paths.**
`EditorGame::delete_selected_entities` (`menu_actions.rs:151`, including the multi-select
`MacroCommand`) and `EditorGame::duplicate_selected_entities` (`:172`, `SpawnTreeCommand` +
`DUPLICATE_OFFSET` + selection-follows-the-copy) have **zero** tests. The 14 tests that look
like coverage exercise `#[cfg(test)]` copies in `entity_ops.rs`. This is the single largest
hole in the workspace.

**I2 — `DeleteEntityCommand`'s child reparenting is untested anywhere real.**
`editor/src/commands/entity_commands.rs:105-112` reparents children to the grandparent or
roots them; `commands/tests.rs:171/:192` only test a childless entity. Move the two
hierarchy cases from `entity_ops_tests.rs:138/:158` onto the command.

**I3 — `commit_gizmo_drag` never proves it records the collider.** Cancel does
(`gizmo_drag_tests.rs:221`); commit is tested only for transforms and via a hand-built macro.

**I4 — `drain_api_requests` (`editor_game/api.rs:185`) is entirely untested**: the
`gizmo_has_priority` skip (requests must stay queued mid-drag), the 256-line per-frame cap,
and the post-drain `note_selection` all have no test. Only the pure `answer_api_lines` half
is covered.

**E1 — `ViewportInputHandler` pan and wheel-zoom are untested through production code.**
Both are only "covered" by the test-module reimplementations on the delete list. Needs a
middle-button drag through `handle_input` asserting the camera moved by `-dx/zoom, +dy/zoom`,
and a scroll asserting zoom multiplies and clamps at `min_zoom`/`max_zoom`.

**E2 — toolbar shortcut hints can lie to the user.** `EditorTool::shortcut()` returns
"Q/W/E/R" and `EditorInputMapping` binds Q/W/E/R — two tables, both asserted only against
hardcoded literals. A drift test (`resolve(shortcut_key) == the tool's action`) would catch
a rebind that leaves the hint stale.

**E3 — menu item click → returned label is untested.** `MenuBar::render` returns
`Option<String>` for the clicked item and nothing tests it; existing tests cover only
open/close geometry. Also untested: a **disabled** item must not return a click.

**E4 — inspector writeback through `apply_component_edit`**, named in `CLAUDE.md` as a
single source of truth, is never asserted end to end. Nothing proves an edit reaches
`CommandHistory` as one entry, merges by `field_hint`, and undoes to the pre-edit value.

**I7 — the KeyF chord is missing from the editor's own default-chord table**
(`editor/src/editor_input.rs:411`): bare F = `FocusSelection`, Ctrl+Shift+F =
`ToggleCameraFollow`. Today it is asserted only from `editor_integration`. **Add those two
rows before deleting `camera_follow_tests.rs:531`**, or the coverage is lost.

### Engine core

**C1 — `main_camera_pose`'s zoom sanitizer is untested.** `render_manager.rs:426-445`
deliberately replaces a non-finite or ≤0 authored zoom with `1.0`, with a comment saying
`zoom: 0.0` in a scene file "must never divide the projection (or the editor viewport) by
zero". No test passes a bad zoom. Hand-authored value, documented guard, a division.

**C2 — `AssetManager::set_base_path` cache invalidation** (`assets.rs:421`) is documented
("relative paths recorded under the old base must not resolve to the old base's textures")
and untested. A stale hit silently loads the wrong art after a project switch in the editor.

**C3 — `#rgba` sentinel end to end.** `create_texture_from_rgba` records the non-unique
`"#rgba"` path and `texture_ref.rs` degrades it to white on load. Both halves are tested
separately; nothing tests create → `texture_path()` → scene save → resolve, which is the
documented "does not survive scene save/load" contract a game author would trip over.

**C4 — `PauseMenu::draw_labeled` with localized labels.** `PauseMenuLabels` exists solely
for localization, and after this audit's deletion of the defaults test it has **zero**
coverage. A wired-wrong `labels.items` would ship silently in Pong's pirate locale.

**C5 — `GridMesh::translate` is only observed through `backdrop_system.rs:251`.** No test
asserts the invariant: `rest` **and** `position` both shift while `velocity` is untouched. A
translate that moved only `position` would pass the system test for a frame and then spring
the whole grid back.

### ECS, physics, input, renderer, ui

**X1 — world-level event integration is untested.** `EventBus` is covered in isolation but
`World::emit` / `read_events` / the per-frame flush (`world.rs:505`) has no test anywhere —
and the collision-event "drain once per frame" footgun documented in `CLAUDE.md` rides on it.

**P1 — collision groups and filters are wired but never tested.** `Collider.collision_groups`
/ `collision_filter` feed `InteractionGroups` at `physics_world/bodies.rs:97-102` via
`Group::from_bits_truncate`, and no test asserts that two colliders in non-overlapping groups
produce **no** collision event. A regression breaks every game's ball/paddle/brick layering.

**P2 — `Collider.offset` is never tested.** `bodies.rs:89` converts the offset to meters and
applies it as the collider's translation; nothing checks that an offset collider collides at
the offset position. This is the one place the documented "physics ignores `Transform2D.scale`"
footgun could be mitigated, and it is unverified.

**P3 — the winit boundary is untested.** `convert_physical_key` (`keyboard.rs:50`) and
`handle_window_event` (`input_handler.rs:226`) are the only untested paths in the input crate,
and they are where a winit upgrade breaks. Both are headlessly testable:
`PhysicalKey::Code(KeyCode::KeyA) → Some(KeyA)` and `PhysicalKey::Unidentified(..) → None` are
constructible, and a synthesized `WindowEvent::MouseWheel` would pin the
`SCROLL_PIXELS_PER_LINE = 16.0` normalization, which has zero coverage today.

**R1 — vertex and instance attribute offsets are unverified.** `SpriteVertex::desc()` and
`SpriteInstance::desc()` hand-compute eleven offsets as `size_of::<[f32; N]>()`. Tests assert
only attribute *count* and *stride*. Swap the depth (`[f32;13]`) and emissive (`[f32;14]`)
offsets and everything compiles, every test passes, and sprites render at wrong depths. This
is squarely inside the rubric's protected "GPU layout the shader assumes" category and is
currently half-protected.

**U2 — `font::layout`, the crate's only real text math, has zero real tests.** `layout_text`
(baseline convention, `offset_y` sign flip, space handling, `max_descent` accumulation) and
`measure_text` are entirely uncovered; the file's single test is a struct literal. DejaVu
bytes already ship in the editor crate via `include_bytes!`, so a small font fixture makes all
of it headless-testable — and this is the code behind the "UI text y = baseline" footgun.

**A1 — nothing in the audio suite exercises an *enabled* manager's sink bookkeeping.**
`active_sound_count`, `update()`'s `retain(|a| !a.sink.empty())`, `stop`, and `stop_all` are
asserted only against a disabled manager where the count is structurally 0 — which is why two
of the four audio deletions are vacuous rather than merely weak. A test-only sink seam would
give the SFX lifecycle its first real coverage.

---

## 9. Incidental findings

Not test issues, surfaced while reading. Each is worth an issue on the Studio Board.

1. **`engine_core::Timer` (`src/timing.rs`) is dead.** Exported at `lib.rs:88` and
   `prelude.rs:45`; grep across all engine crates and all six games finds no consumer.
   `crates/engine_core/tests/timing.rs` is 147 lines testing an API nothing calls.
2. **`GlobalTransform2D::transform_point` (`crates/ecs/src/hierarchy.rs:204`) is dead.**
   Its only reference in the workspace is the test on the delete list. (The
   `transform_point` hits in `asteroids` and `common` are different functions.)
3. `crates/engine_core/src/grid/grid_mesh.rs:462-469` is an eight-line blank gap left by
   removed tests.
4. `crates/engine_core/tests/scene_lifecycle.rs:19-26` carries a `println!` debug block
   inside an otherwise sound test.
5. `crates/editor/src/editor_preferences.rs:137` writes to a fixed shared temp path, which
   races across concurrent test binaries or two checkouts.
6. Confirmed clean: **0 `#[ignore]` attributes** anywhere in `crates/`.

---

## 10. Keep-list

Inverted deliverable: the goal is a few hundred excellent tests, not a trimmed 1,240.
Below, per crate, are the player/author-visible **contracts** and the **footguns** worth a
guard, each with the ONE existing test that locks it, or `MISSING`. **Everything not named
here is deleted.** 312 keeps out of 1,657.

Every `file:line` was re-derived from the tree for this section. That mattered: §2–§8 above
carry line numbers reported for `camera_follow_tests.rs` that are wrong by ~300 lines. Where
the two disagree, this section is correct.

### common — 41 → 16

| kind | contract / footgun | test |
|---|---|---|
| GUARD | screen +Y down, world +Y up | `common/src/camera.rs:248 test_screen_y_down_maps_to_world_y_up` |
| CONTRACT | screen↔world round-trip | `common/src/camera.rs:237 test_world_to_screen_round_trips_screen_to_world` |
| GUARD | matrix is T·R·S, never T·S·R | `common/src/transform.rs:223 test_matrix_applies_scale_before_rotation_before_translation` |
| CONTRACT | inverse_transform_point round-trips | `common/src/transform.rs:192 test_inverse_transform_point_round_trips_translated_rotated_point` |
| CONTRACT | transform_direction is the linear part under non-uniform scale | `common/src/transform.rs:239 test_transform_direction_agrees_with_matrix_under_nonuniform_scale` |
| CONTRACT | sRGB luminance (feeds every WCAG guard) | `common/src/color.rs:264 test_known_srgb_luminance` |
| CONTRACT | white/black contrast is 21:1 | `common/src/color.rs:250 test_white_black_contrast_is_21` |
| CONTRACT | hex → Color (feeds `#solid:RRGGBB`) | `common/src/color.rs:219 test_color_from_hex` |
| GUARD | from_cell_size truncates partial trailing cells, UVs pixel-exact | `common/src/sheet_grid.rs:180 test_from_cell_size_truncates_partial_trailing_cells` |
| GUARD | from_uv_size keeps a non-reciprocal cell size | `common/src/sheet_grid.rs:225 test_from_uv_size_preserves_non_reciprocal_cell_size` |
| CONTRACT | uv_rect row-major cell mapping | `common/src/sheet_grid.rs:160 test_uv_rect_maps_index_to_row_major_cell` |
| CONTRACT | uv_rect_checked None past cell count | `common/src/sheet_grid.rs:209 test_uv_rect_checked_is_none_past_cell_count` |
| GUARD | degenerate grids never divide by zero | `common/src/sheet_grid.rs:171 test_new_clamps_zero_dimensions` |
| CONTRACT | web boot base-joined keys resolve | `common/src/vfs.rs:148 test_boot_phase_keys_resolve_through_base_joined_reads` |
| CONTRACT | list_dir_files sorted, filtered, direct children only | `common/src/vfs.rs:163 test_list_dir_files_finds_locales_under_production_like_keys` |
| CONTRACT | `with_fields!` macro output | `common/src/macros.rs:68 test_with_fields_macro` |

Dead APIs, delete rather than test: `Rect::contains`/`intersects`/`intersection`,
`Camera::world_bounds`/`contains_point`, `common::Time` — all zero production callers.

### ecs — inline `src/` 119 → 19

| kind | contract / footgun | test |
|---|---|---|
| CONTRACT | transition updates current and previous | `ecs/src/state_machine.rs:284 test_transition_updates_current_and_previous` |
| CONTRACT | same-state transition is a no-op | `ecs/src/state_machine.rs:297 test_same_state_transition_is_noop` |
| CONTRACT | tick clears just_entered, accumulates elapsed | `ecs/src/state_machine.rs:320 test_tick_clears_just_entered_and_accumulates_time` |
| CONTRACT | hierarchical transition within a group | `ecs/src/state_machine.rs:400 test_hierarchical_transition_within_group` |
| CONTRACT | hierarchical transition across groups | `ecs/src/state_machine.rs:413 test_hierarchical_transition_across_groups` |
| CONTRACT | emit → read within a frame | `ecs/src/event.rs:167 test_emit_and_read_events` |
| CONTRACT | flush clears every type's queue | `ecs/src/event.rs:186 test_flush_clears_all_events` |
| CONTRACT | events readable until flush | `ecs/src/event.rs:274 test_events_readable_multiple_times_before_flush` |
| CONTRACT | resource insert / replace | `ecs/src/resource.rs:143 test_insert_replaces_previous` |
| CONTRACT | ancestors ordered nearest-first | `ecs/src/hierarchy_extension.rs:362 test_get_ancestors` |
| CONTRACT | reparent prunes the old parent's child list | `ecs/src/hierarchy_extension.rs:429 test_reparent_entity` |
| CONTRACT | a scaled parent scales its child's global | `ecs/src/hierarchy_system.rs:402 test_scaled_parent_transform_propagation` |
| CONTRACT | GlobalTransform composition under rotation | `ecs/src/hierarchy.rs:296 test_global_transform_mul_with_rotation` |
| CONTRACT | tilemap instances, row zero on top | `ecs/src/tilemap.rs:152 test_sprite_instances_offsets_row_zero_on_top` |
| GUARD | a short `tiles` vec truncates, never panics | `ecs/src/tilemap.rs:168 test_short_tiles_vec_is_truncated_not_a_panic` |
| CONTRACT | entity despawns when lifetime crosses zero | `ecs/src/lifetime.rs:78 test_entity_despawns_when_lifetime_crosses_zero` |
| CONTRACT | animation system writes `Sprite.tex_region` | `ecs/src/sprite_system.rs:60 test_system_writes_current_frame_region_to_sprite` |
| GUARD | zero delta freezes the frame (how pause works) | `ecs/src/sprite_system.rs:87 test_system_with_zero_delta_freezes_the_frame` |
| GUARD | CameraFollow parses the legacy four-field form | `ecs/src/behavior.rs:553 test_camera_follow_parses_legacy_four_field_form` |
| CONTRACT | insert/extract/remove by name on a world | `ecs/src/component_registry/tests.rs:75 test_insert_extract_remove_round_trip_on_a_world` |
| GUARD | persistent_names sorted for stable scene diffs | `ecs/src/component_registry/tests.rs:178 test_persistent_names_are_sorted_for_stable_scene_diffs` |
| GUARD | transient types never reach persistent_names | `ecs/src/component_registry/tests.rs:165 test_transient_types_are_excluded_from_persistent_names` |
| GUARD | same name, different type panics clearly | `ecs/src/component_registry/tests.rs:148 test_same_name_different_type_registration_panics` |
| CONTRACT | Scripts serde covers every param variant | `ecs/src/script.rs:124 test_scripts_serde_round_trips_every_value_variant` |
| CONTRACT | UiAnchor resolves anchored positions | `ecs/src/ui_components.rs:260 test_resolve_anchored_pos_matrix` |
| GUARD | `Box<dyn Component>` downcast via `.as_ref().as_any()` | **MISSING** |
| GUARD | `Children` Vec order is load-bearing, not a HashSet | **MISSING** |
| GUARD | `GlobalTransform2D` manual writes are overwritten | **MISSING** |
| GUARD | every registered component survives BOTH serde_json and RON | **MISSING** |
| CONTRACT | `World::emit`/`read_events`/per-frame flush | **MISSING** |

*(19 keeps after folding state_machine to 3, event to 2, registry to 3.)*

### ecs — `tests/` 94 → 21

| kind | contract / footgun | test |
|---|---|---|
| CONTRACT | clean frame recomputes nothing, still dirty-checks | `ecs/tests/hierarchy_dirty.rs:23 test_no_change_second_frame_recomputes_zero` |
| CONTRACT | leaf change recomputes one, sibling stays correct | `ecs/tests/hierarchy_dirty.rs:40 test_leaf_change_recomputes_one` |
| CONTRACT | parent change recomputes subtree only | `ecs/tests/hierarchy_dirty.rs:61 test_parent_change_recomputes_subtree_only` |
| CONTRACT | parent deletion orphans and prunes the cache | `ecs/tests/hierarchy_dirty.rs:107 test_parent_deletion_orphans_recompute_and_cache_prunes` |
| GUARD | identical write must not dirty (sleeping-body writeback) | `ecs/tests/hierarchy_dirty.rs:131 test_identical_write_stays_clean` |
| GUARD | disable leaves globals stale, re-enable detects drift | `ecs/tests/hierarchy_dirty.rs:153 test_reenable_after_disable_catches_stale` |
| CONTRACT | ensure_playing restarts only a different/stopped clip | `ecs/tests/sprite_components.rs:200 test_ensure_playing_restarts_a_different_or_stopped_clip` |
| CONTRACT | non-looping clip clamps and stops for good | `ecs/tests/sprite_components.rs:135 test_non_looping_clip_clamps_on_the_last_frame_and_stops` |
| GUARD | shorter clip never exposes a stale frame index | `ecs/tests/sprite_components.rs:81 test_switching_to_a_shorter_clip_never_exposes_a_stale_frame` |
| CONTRACT | current_uv maps the frame through the SheetGrid | `ecs/tests/sprite_components.rs:263 test_current_uv_maps_the_frame_index_through_the_grid` |
| GUARD | fps 0/neg/NaN, empty frames, non-finite dt never panic | `ecs/tests/sprite_components.rs:223 test_non_advancing_fps_values_never_panic_or_advance` |
| GUARD | omitted tex_region = FULL texture, omitted visible = true | `ecs/tests/sprite_components.rs:340 test_sprite_deserializes_omitted_region_and_visibility_to_full_and_visible` |
| CONTRACT | ComponentMeta field ORDER = inspector render order | `ecs/tests/sprite_components.rs:301 test_sprite_animation_component_meta` |
| CONTRACT | pause holds the frame, resume continues | `ecs/tests/sprite_components.rs:153 test_pause_holds_the_frame_and_resume_continues_from_it` |
| CONTRACT | every component op refuses a stale entity id | `ecs/tests/world.rs:313 test_stale_entity_id_rejected_by_component_ops` |
| CONTRACT | clear + create_entity_with_id revives an id (snapshot) | `ecs/tests/world.rs:332 test_snapshot_restore_revives_entity_id` |
| CONTRACT | removing a parent orphans children, no dangling Parent | `ecs/tests/world.rs:369 test_remove_parent_entity_orphans_children_to_root` |
| CONTRACT | remove_entity_hierarchy leaves no residue, 100 deep | `ecs/tests/world.rs:405 test_remove_entity_hierarchy_deep_chain_leaves_no_residue` |
| GUARD | component_types reports the CONCRETE name, never the Box's | `ecs/tests/world.rs:446 test_component_types_reports_concrete_type_names` |
| GUARD | reparenting rejects cycles, error names the cycle | `ecs/tests/world.rs:108 test_hierarchy_cycle_detection` |
| CONTRACT | typed queries Single and Pair select correctly | `ecs/tests/world.rs:144 test_query_entities` |
| CONTRACT | world FSM init → start → update → stop → shutdown | `ecs/tests/system_lifecycle.rs:343 test_world_lifecycle_integration` |
| CONTRACT | a late-added system gets its missed hooks | `ecs/tests/system_lifecycle.rs:207 test_late_added_system_gets_missed_hooks` |
| GUARD | one panicking system does not kill the registry | `ecs/tests/system_lifecycle.rs:306 test_panic_recovery_in_systems` |

`system_lifecycle.rs:306` has no assert and **still stays** — `update_all` really does use
`catch_unwind` (`ecs/src/system.rs:213`). Add an assert that a system added after the
panicking one still updates. This reverses §2's read of it as bloat.

### ecs_macros — 3 → 1

| kind | contract | test |
|---|---|---|
| CONTRACT | derive emits type_name + field_names in declaration order | `ecs_macros/tests/derive_test.rs:40 test_field_names_order_preserved` |

### editor — 477 → 62

| kind | contract / footgun | test |
|---|---|---|
| CONTRACT | execute → undo → redo ordering | `editor/src/commands/tests.rs:19 test_command_history_execute_and_undo` |
| CONTRACT | history cap drops oldest, preserves undo order | `editor/src/commands/tests.rs:423 test_max_history_drops_oldest_and_preserves_undo_order` |
| GUARD | delete-undo resurrects the SAME entity id | `editor/src/commands/tests.rs:491 test_delete_undo_resurrects_same_entity_id` |
| GUARD | a Set command survives a delete/undo cycle | `editor/src/commands/tests.rs:527 test_set_command_survives_delete_undo_cycle` |
| CONTRACT | continuous edits merge by field_hint into one undo | `editor/src/commands/tests.rs:344 test_set_transform_merge` |
| CONTRACT | removing RigidBody cascades to Collider | `editor/src/commands/tests.rs:244 test_remove_rigid_body_cascades_to_collider` |
| CONTRACT | undo back to the saved watermark reads clean | `editor/src/commands/dirty_tests.rs:78 test_undo_back_to_saved_watermark_reads_clean` |
| GUARD | save → merge → undo stays dirty (merges reassign ids) | `editor/src/commands/dirty_tests.rs:128 test_save_then_merge_then_undo_stays_dirty` |
| GUARD | a merge clears redo history | `editor/src/commands/dirty_tests.rs:147 test_merge_clears_redo_history` |
| GUARD | `break_merge` seals the gesture boundary | `editor/src/commands/dirty_tests.rs:199 test_break_merge_prevents_merge_across_gestures` |
| GUARD | **merge isolation between command types / entities** | **MISSING** — `try_merge` defaults false at `commands/mod.rs:54` with impls at `set_commands.rs:51/:104/:259`; every merge test is same-type, same-entity |
| CONTRACT | clipboard capture/spawn round-trips a hierarchy | `editor/src/clipboard.rs:328 test_capture_and_spawn_round_trips_a_hierarchy` |
| GUARD | SpawnTree redo resurrects the same ids (GPP-14) | `editor/src/clipboard.rs:376 test_spawn_tree_redo_resurrects_the_same_ids` |
| CONTRACT | DeleteTree removes subtree, undo restores ids | `editor/src/clipboard.rs:427 test_delete_tree_removes_whole_subtree_and_undo_restores_ids` |
| CONTRACT | duplicate renames every spawned name | `editor/src/clipboard.rs:457 test_duplicate_suffix_renames_every_spawned_name` |
| CONTRACT | undo of delete restores the selection (#59) | `editor/src/commands/selection_restore_tests.rs:30 test_undo_delete_restores_the_selection` |
| GUARD | merged entries keep the first before-image | `editor/src/commands/selection_restore_tests.rs:109 test_merged_entries_keep_the_first_before_image` |
| GUARD | stale ids pruned from the restore | `editor/src/commands/selection_restore_tests.rs:136 test_stale_ids_are_pruned_from_the_restore` |
| CONTRACT | rename adds Name, undo removes the component | `editor/src/commands/name_tests.rs:40 test_rename_adds_name_and_undo_removes_the_component` |
| GUARD | **command-API doc drift** — every doc example parses | `editor/src/command_api/specs.rs:188 test_every_doc_example_parses` |
| GUARD | **command-API verb drift** — parser verbs match docs | `editor/src/command_api/specs.rs:200 test_parser_verbs_match_docs` |
| CONTRACT | responses are single-line | `editor/src/command_api/tests.rs:216 test_responses_are_single_line` |
| CONTRACT | error envelope carries kind + message | `editor/src/command_api/tests.rs:229 test_error_envelope_kind_and_message` |
| CONTRACT | ambiguous name error carries the matches | `editor/src/command_api/tests.rs:243 test_ambiguous_name_error_carries_matches` |
| GUARD | add refuses an unissued texture handle, leaves nothing | `editor/src/command_api/write_tests.rs:169 test_add_rejects_unissued_texture_handle_without_leaving_component` |
| GUARD | non-finite numbers refused | `editor/src/command_api/write_tests.rs:140 test_set_rejects_non_finite_numbers` |
| GUARD | collider extents sanitized to the GUI floor | `editor/src/command_api/write_tests.rs:196 test_set_sanitizes_collider_extents` |
| CONTRACT | a batch aborts by rolling back in reverse | `editor/src/command_api/write_tests.rs:340 test_batch_abort_rolls_back_in_reverse` |
| GUARD | writes refused while playing | `editor/src/command_api/write_tests.rs:371 test_writes_refused_while_playing` |
| CONTRACT | add/set/remove reach dynamic game components | `editor/src/command_api/write_tests.rs:482 test_add_set_remove_work_on_dynamic_components` |
| GUARD | **registry drift** — type ids match world enumeration | `editor/src/stored_component/tests.rs:224 test_registered_type_ids_match_world_enumeration` |
| CONTRACT | every settable type round-trips through JSON | `editor/src/stored_component/tests.rs:294 test_stored_component_from_json_round_trips_all_settable_types` |
| CONTRACT | snapshot round-trips dynamic components | `editor/src/stored_component/dynamic_tests.rs:73 test_world_snapshot_round_trips_dynamic_components` |
| CONTRACT | snapshot restore preserves entity ids | `editor/src/world_snapshot/tests.rs:29 test_snapshot_restore_preserves_entity_ids` |
| CONTRACT | restore discards play-session changes | `editor/src/world_snapshot/tests.rs:53 test_snapshot_restore_discards_play_changes` |
| GUARD | unregistered component types reported once (data loss) | `editor/src/world_snapshot/tests.rs:217 test_snapshot_reports_unregistered_component_types_once` |
| GUARD | **chord drift** — every default chord resolves to its action | `editor/src/editor_input.rs:412 test_every_default_chord_resolves_to_its_action` |
| CONTRACT | a rebind evicts only the exact chord | `editor/src/editor_input.rs:474 test_rebind_evicts_only_the_exact_chord` |
| GUARD | **menu-label ↔ PanelId drift** (ARCH-101), incl. deliberate `None`s | `editor/src/dock/tests.rs:268 test_panel_id_for_menu_label_map` |
| GUARD | a flip-scaled sprite is picked at its visual bounds | `editor/src/picking/tests.rs:64 test_flip_scaled_sprite_is_picked_at_its_visual_bounds` |
| GUARD | equal-depth hits order by id deterministically | `editor/src/picking/tests.rs:124 test_equal_depth_hits_order_by_id_deterministically` |
| CONTRACT | scale factor is the per-axis offset ratio | `editor/src/gizmo/tests.rs:196 test_scale_factor_is_offset_ratio_per_axis` |
| GUARD | Escape cancel latch suppresses the gesture to mouse-up | `editor/src/gizmo/tests.rs:220 test_cancel_latch_suppresses_rest_of_gesture_until_mouse_up` |
| GUARD | rotation seam crossing returns a small delta | `editor/src/gizmo_math.rs:58 test_seam_crossing_returns_small_delta` |
| CONTRACT | viewport screen↔world round-trip | `editor/src/viewport/tests.rs:44 test_viewport_coordinate_roundtrip` |
| GUARD | render camera and overlay agree (one view, two consumers) | `editor/src/viewport/tests.rs:222 test_window_render_camera_screen_roundtrip` |
| GUARD | camera convergence is frame-rate independent | `editor/src/viewport/tests.rs:119 test_update_is_frame_rate_independent` |
| CONTRACT | framing zooms to fit one entity's extents | `editor/src/context/tests.rs:357 test_frame_selected_zooms_to_fit_single_entity_extents` |
| CONTRACT | subdivisions zoom-gated, never on primary lines | `editor/src/grid.rs:475 test_subdivisions_gated_by_zoom_and_never_on_primary_lines` |
| CONTRACT | panel resize clamps to min and half the dock | `editor/src/dock/tests.rs:217 test_resized_size_clamps_to_min_and_half_dock` |
| GUARD | **WCAG surface ladder** ≥1.35:1, elevation gets lighter | `editor/src/theme/tests.rs:103 test_adjacent_surfaces_are_distinguishable` |
| GUARD | **WCAG popup border** ≥3:1 against the panel | `editor/src/theme/tests.rs:119 test_popup_reads_against_panel` |
| GUARD | selection outline derivation contract | `editor/src/theme/tests.rs:67 test_selection_outline_derivation_contract` |
| CONTRACT | pair slots shrink on narrow panels, cap on wide | `editor/src/row_layout.rs:192 test_pair_slots_shrink_on_narrow_panel_and_cap_on_wide` |
| GUARD | a pending string edit commits before a variant cycle | `editor/src/inspector_edit_tests.rs:168 test_pending_string_edit_commits_before_variant_cycle_applies` |
| CONTRACT | soft-range typed value raises a status-bar warning (#55) | `editor/src/inspector_edit_tests.rs:206 test_typed_value_outside_soft_range_raises_a_warning` |
| CONTRACT | collider shape cycle carries size into the new variant | `editor/src/inspector_edit_tests.rs:143 test_collider_shape_cycle_carries_size_into_new_variant` |
| CONTRACT | numeric fields draw/measure in the mono face (#54) | `editor/src/fonts.rs:53 test_numeric_field_draws_and_measures_in_the_mono_face` |
| CONTRACT | visible order = draw order, skips collapsed subtrees | `editor/src/hierarchy_tests.rs:365 test_visible_order_follows_draw_order_and_skips_collapsed_subtrees` |
| CONTRACT | F2 rename commits, reports the name, exits the mode | `editor/src/hierarchy_tests.rs:289 test_rename_commit_reports_new_name_and_exits_mode` |
| CONTRACT | removing primary falls back to earliest remaining | `editor/src/selection.rs:335 test_remove_primary_falls_back_to_earliest_remaining` |
| GUARD | toolbar click must not reselect the sprite underneath | `editor/src/toolbar.rs:304 test_toolbar_button_click_survives_chrome_interact` |
| GUARD | an open dropdown blocks input beneath it | `editor/src/menu/tests.rs:266 test_open_dropdown_renders_in_overlay_band_and_blocks_input` |
| GUARD | legacy prefs without a `panels` field still load | `editor/src/editor_preferences.rs:177 test_legacy_prefs_without_panels_field_still_load` |
| CONTRACT | a rescan preserves loaded handles by path | `editor/src/asset_browser.rs:191 test_apply_scan_preserves_loaded_handles_by_path` |
| GUARD | collider overlay ignores `Transform2D.scale`, like physics | `editor/src/collider_overlay.rs:295 test_transform_scale_is_ignored_like_physics` |
| CONTRACT | capsule-Y extends half-height plus radius | `editor/src/collider_overlay.rs:261 test_capsule_y_extends_half_height_plus_radius_vertically` |
| CONTRACT | hover picks topmost by depth, stable tiebreak | `editor/src/selection_outline.rs:310 test_hover_picks_topmost_by_depth_with_stable_tiebreak` |
| GUARD | shrinking content re-clamps the scroll offset | `editor/src/scroll.rs:124 test_shrinking_content_reclamps_offset` |
| GUARD | release under threshold is a click, not a drag | `editor/src/drag_drop.rs:124 test_release_under_threshold_is_a_click_not_a_drag` |
| GUARD | shortcuts gate on `wants_keyboard`; typing must not delete | **MISSING** |
| CONTRACT | `ViewportInputHandler` pan and wheel-zoom through `handle_input` | **MISSING** — only the deleted test-module reimplementation covers it |
| GUARD | `EditorTool::shortcut()` hint matches the actual binding | **MISSING** |
| CONTRACT | menu item click returns its label; a disabled item does not | **MISSING** |
| GUARD | `EditorPreferences::load` on truncated JSON | **MISSING** |

### editor_integration — 150 → 30

| kind | contract / footgun | test |
|---|---|---|
| CONTRACT | play → pause → resume → stop | `editor_game/tests.rs:48 test_play_pause_resume_stop_cycle` |
| CONTRACT | stop restores the world from the snapshot | `editor_game/tests.rs:73 test_stop_restores_world_state` |
| GUARD | stop resets the transform-propagation cache | `editor_game/tests.rs:95 test_stop_resets_transform_propagation_cache` |
| CONTRACT | scene save → load round-trip | `editor_game/tests.rs:220 test_save_scene_roundtrip` |
| CONTRACT | OS title updates only on change | `editor_game/tests.rs:316 test_pending_title_update_only_on_change` |
| GUARD | render overrides camera from viewport (viewport is SSOT) | `editor_game/tests.rs:351 test_render_overrides_camera_from_viewport` |
| GUARD | a hidden scene panel writes a zero scissor | `editor_game/tests.rs:398 test_render_writes_zero_scissor_when_scene_panel_hidden` |
| GUARD | scale tool scales collider shapes + offset (physics ignores scale) | `editor_game/tests.rs:476 test_scale_collider_scales_shapes_and_offset` |
| CONTRACT | engine time frozen while not playing | `editor_game/time_freeze_tests.rs:17 test_time_scale_is_frozen_while_not_playing` |
| GUARD | particles and animations hold still while editing | `editor_game/time_freeze_tests.rs:40 test_particles_and_animations_do_not_advance_while_editing` |
| GUARD | save refused while playing | `editor_game/play_guard_tests.rs:27 test_save_refused_while_playing` |
| GUARD | scene replace refused during a play session | `editor_game/play_guard_tests.rs:88 test_scene_replace_refused_during_play_session` |
| GUARD | Play warns about unregistered components (data loss) | `editor_game/play_guard_tests.rs:135 test_play_surfaces_warning_for_unregistered_components` |
| GUARD | Stop reports dropped component types | `editor_game/play_guard_tests.rs:167 test_stop_reports_dropped_component_types` |
| GUARD | resume from pause does not recapture the snapshot | `editor_game/play_guard_tests.rs:188 test_resume_from_pause_does_not_recapture_snapshot` |
| GUARD | malformed RON preserves the live world (scratch dry-run) | `editor_game/scene_io_tests.rs:73 test_load_malformed_ron_preserves_world` |
| GUARD | instantiate failure preserves the live world | `editor_game/scene_io_tests.rs:105 test_load_instantiate_failure_preserves_world` |
| CONTRACT | load publishes physics resource, save keeps the block | `editor_game/scene_io_tests.rs:183 test_load_scene_publishes_physics_resource_and_save_keeps_the_block` |
| CONTRACT | save auto-names script targets via CommandHistory | `editor_game/scene_io_tests.rs:141 test_save_auto_names_script_targets_through_command_history` |
| CONTRACT | viewport mirrors game camera only while playing + following | `editor_game/camera_follow_tests.rs:60 test_sync_copies_zoom_only_while_playing_and_following` |
| GUARD | pause/resume preserves a broken follow | `editor_game/camera_follow_tests.rs:114 test_pause_resume_preserves_a_broken_follow` |
| CONTRACT | stop restores the editing view and re-arms follow | `editor_game/camera_follow_tests.rs:132 test_stop_restores_editing_view_and_rearms_follow` |
| GUARD | entering play cancels a pending viewport gesture | `editor_game/camera_follow_tests.rs:160 test_play_transition_cancels_pending_viewport_gesture` |
| CONTRACT | gizmo commit records ONE undo entry for every root | `editor_game/gizmo_drag_tests.rs:83 test_commit_records_one_undo_entry_restoring_every_root` |
| CONTRACT | cancel restores starts, pushes no undo entry | `editor_game/gizmo_drag_tests.rs:221 test_cancel_restores_starts_and_pushes_no_undo_entry` |
| GUARD | snapped multi-drag preserves relative offsets | `editor_game/gizmo_drag_tests.rs:166 test_snapped_multi_drag_preserves_relative_offsets` |
| GUARD | zero grid size never poisons positions with NaN | `editor_game/gizmo_drag_tests.rs:203 test_zero_grid_size_never_poisons_positions` |
| GUARD | chrome owns the mouse while a widget holds the gesture | `editor_game/picking_tests.rs:10 test_chrome_owns_mouse_while_widget_holds_the_gesture` |
| CONTRACT | picking hits a sprite at rendered size, offset panel (RENDER_UNIT) | `editor_game/picking_tests.rs:83 test_pick_hits_sprite_at_rendered_size_with_offset_panel` |
| CONTRACT | held arrow merges into one undo entry, sealed on release | `editor_game/shortcuts_tests.rs:49 test_held_arrow_merges_into_one_undo_entry_sealed_on_release` |
| GUARD | nudge suppressed during a gizmo drag | `editor_game/shortcuts_tests.rs:97 test_nudge_is_suppressed_during_a_gizmo_drag` |
| CONTRACT | `apply_component_edit` writes back and records undo | `panel_renderer/tests.rs:8 test_transform_writeback_applies_and_records_undo` |
| CONTRACT | `apply_component_edit` merges continuous edits | `panel_renderer/tests.rs:154 test_apply_component_edit_merges_continuous_edits` |
| CONTRACT | full headless authoring loop survives a reload | `editor_game/headless/tests.rs:28 test_full_authoring_loop_survives_a_reload` |
| GUARD | an unissued texture handle never reaches the file | `editor_game/headless/tests.rs:95 test_unissued_texture_handle_is_refused_and_never_reaches_the_file` |
| CONTRACT | Play commits an open api_batch as one entry | `editor_game/api_write_tests.rs:112 test_play_start_commits_open_batch_as_one_entry` |
| CONTRACT | a dirty world parks the action for the confirm dialog | `editor_game/scene_confirm_tests.rs:41 test_dirty_world_parks_the_action_for_the_dialog` |
| CONTRACT | **production delete + duplicate paths** | **MISSING** — the 14 `entity_ops_tests.rs` tests exercise `#[cfg(test)]` copies at `entity_ops.rs:219/:299`; `menu_actions.rs:151/:172` are untested. Largest hole in the workspace |
| CONTRACT | `DeleteEntityCommand` child reparenting | **MISSING** — `entity_commands.rs:105-112` |
| CONTRACT | `drain_api_requests` gizmo skip, 256-line cap, note_selection | **MISSING** — `editor_game/api.rs:185` |
| GUARD | **Behavior scene fixture** — every variant through RON into the runner | **MISSING** (partial: `engine_core/src/scene_serializer_tests.rs:280` covers `PlayerPlatformer`) |

### engine_core — 394 → 78

| kind | contract / footgun | test |
|---|---|---|
| GUARD | **scene-table drift** — every persistent type appears once, no Dynamic duplicate | **MISSING** — `scene_serializer.rs:294` `CONCRETE_OR_EXCLUDED` is a hand-maintained 16-name list; `scene_serializer_tests.rs:390` guards one entry |
| GUARD | GlobalTransform2D is never serialized | `scene_serializer_tests.rs:390 test_global_transform_not_serialized` |
| CONTRACT | full world → RON → world round-trip | `scene_serializer_roundtrip_tests.rs:46 test_roundtrip_serialize_deserialize` |
| CONTRACT | GridBackdrop round-trips every field, parses bare | `scene_serializer_roundtrip_tests.rs:206 test_grid_backdrop_round_trips_every_field_and_parses_bare` |
| CONTRACT | hierarchy survives save | `scene_serializer_tests.rs:334 test_hierarchy_preserved` |
| CONTRACT | RigidBody extraction, all body types | `scene_serializer_tests.rs:204 test_entity_with_rigid_body` |
| CONTRACT | Collider extraction, all shapes | `scene_serializer_tests.rs:244 test_entity_with_collider` |
| CONTRACT | entity ordering stable across saves | `scene_serializer_tests.rs:461 test_multiple_entities_ordering` |
| CONTRACT | a dynamic component's payload survives RON | `scene_dynamic_tests.rs:56 test_dynamic_payload_survives_ron_round_trip` |
| GUARD | transient components never written | `scene_dynamic_tests.rs:115 test_transient_components_are_not_saved` |
| GUARD | dynamic emissions name-sorted, so saves diff cleanly | `scene_dynamic_tests.rs:138 test_dynamic_emissions_are_name_sorted` |
| GUARD | an unknown dynamic component fails the load loudly | `scene_dynamic_tests.rs:162 test_unknown_dynamic_component_fails_the_load_loudly` |
| CONTRACT | Scripts round-trip every param type | `scene_scripts_tests.rs:55 test_scripts_scene_round_trip_preserves_every_param_type` |
| GUARD | an entity param naming a missing entity is dropped with a warning | `scene_scripts_tests.rs:94 test_entity_param_referencing_missing_name_is_dropped_with_warning` |
| CONTRACT | save auto-names referenced unnamed targets | `scene_scripts_tests.rs:151 test_save_auto_names_referenced_unnamed_targets` |
| GUARD | every bundled example scene still parses | `tests/scene_loader_parse.rs:233 test_bundled_example_scenes_parse` |
| GUARD | legacy CameraFollow scene without look_ahead parses | `tests/scene_loader_parse.rs:281 test_legacy_camera_follow_scene_without_look_ahead_still_parses` |
| GUARD | pre-editor scene files load with `editor: None` | `scene_data_tests.rs:46 test_scene_data_without_editor_settings_backward_compat` |
| CONTRACT | tilemap parses and instantiates with a resolved tileset | `tests/scene_loader_parse.rs:183 test_tilemap_parses_and_instantiates_with_resolved_tileset` |
| CONTRACT | override layer replaces the prefab's same-type component | `scene_loader.rs:338 test_merge_components` |
| CONTRACT | golden `.sheet.ron` round-trips | `sheet_file.rs:176 test_golden_sheet_file_round_trips` |
| CONTRACT | omitted `filter` defaults to Nearest | `sheet_file.rs:194 test_omitted_filter_defaults_to_nearest` |
| GUARD | unknown version rejected, naming the file | `sheet_file.rs:224 test_unknown_version_is_rejected_naming_the_file` |
| GUARD | unusable fps rejected, naming the clip | `sheet_file.rs:252 test_unusable_fps_values_are_rejected_naming_the_clip` |
| GUARD | frame index past the grid rejected, naming the clip | `sheet_file.rs:295 test_frame_index_past_the_grid_is_rejected_naming_the_clip` |
| CONTRACT | into_parts excludes a partial trailing cell | `sheet_file.rs:283 test_into_parts_excludes_a_partial_trailing_cell` |
| GUARD | old SpriteAnimation format loads as an inert default | `tests/sprite_animation_scene.rs:83 test_old_format_sprite_animation_loads_as_inert_default` |
| CONTRACT | SpriteAnimation round-trips through scene RON | `tests/sprite_animation_scene.rs:178 test_sprite_animation_round_trips_through_scene_ron` |
| GUARD | the sidecar wins over baked scene values | `tests/sprite_animation_scene.rs:259 test_sidecar_grid_and_clips_win_over_baked_scene_values` |
| GUARD | a missing sidecar falls back to baked values | `tests/sprite_animation_scene.rs:285 test_missing_sidecar_falls_back_to_the_baked_values` |
| GUARD | autoplay naming a dropped clip leaves it stopped | `tests/sprite_animation_scene.rs:309 test_autoplay_naming_a_clip_the_sidecar_dropped_leaves_it_stopped` |
| GUARD | scene load clears the sidecar cache first | `tests/sprite_animation_scene.rs:333 test_scene_load_clears_the_sidecar_cache_first` |
| GUARD | the ClipData wire format is stable | `tests/sprite_animation_scene.rs:347 test_clip_wire_format_is_stable` |
| CONTRACT | an animated sprite's region reaches the renderer | `tests/sprite_animation_scene.rs:471 test_animated_sprite_region_reaches_the_renderer` |
| GUARD | a failed sheet validation loads no texture | `assets/sprite_sheet.rs:310 test_prepare_sheet_fails_before_any_texture_is_loaded` |
| CONTRACT | clearing the cache picks up an edited sidecar | `assets/sprite_sheet.rs:364 test_clearing_the_cache_picks_up_an_edited_sidecar` |
| GUARD | a malformed sidecar falls back quietly | `assets/sprite_sheet.rs:390 test_cache_falls_back_quietly_on_a_malformed_sidecar` |
| GUARD | generated texture refs ignored by the cache | `assets/sprite_sheet.rs:400 test_cache_ignores_generated_texture_references` |
| GUARD | RGBA validation names the expected byte count | `assets.rs:505 test_rgba_validation_rejects_length_mismatch` |
| CONTRACT | write → read round-trips, no temp file left (atomicity) | `save_store.rs:175 test_write_then_read_round_trips_and_leaves_no_temp_file` |
| CONTRACT | the memory store matches native slot semantics (wasm) | `save_store.rs:204 test_memory_store_matches_slot_semantics` |
| CONTRACT | **input JSON fixture** — settings round-trip pads and bindings | `input_settings_io.rs:174 round_trip_preserves_pads_and_bindings` |
| GUARD | missing settings file writes hand-editable defaults | `input_settings_io.rs:204 missing_file_returns_defaults_and_creates_hand_editable_file` |
| GUARD | corrupt settings file falls back without panicking | `input_settings_io.rs:220 corrupt_file_falls_back_to_defaults_without_panicking` |
| GUARD | wrong-version settings file falls back to defaults | `input_settings_io.rs:232 wrong_version_falls_back_to_defaults` |
| CONTRACT | achievements persist across a round-trip | `achievements/tests.rs:86 persistence_round_trip` |
| GUARD | concurrent managers merge unlocks, not clobber | `achievements/tests.rs:108 concurrent_managers_merge_unlocks_instead_of_clobbering` |
| GUARD | an unwritable save path errors without panicking | `achievements/tests.rs:166 save_to_unwritable_path_errors_without_panicking` |
| CONTRACT | scores persist and rank correctly | `scores.rs:254 test_persistence_round_trip` |
| GUARD | corrupt score file warns and starts fresh | `scores.rs:279 test_corrupt_file_warns_and_starts_fresh` |
| GUARD | concurrent stores merge instead of clobbering | `scores.rs:289 test_concurrent_stores_merge_instead_of_clobbering` |
| CONTRACT | `tr` falls back to English, then to the key | `localization.rs:281 tr_falls_back_to_english_then_key` |
| GUARD | corrupt and wrong-version locale sources are skipped | `localization.rs:308 corrupt_and_wrong_version_sources_are_skipped` |
| CONTRACT | load_dir reads RON files by stem | `localization.rs:328 load_dir_reads_ron_files_by_stem` |
| CONTRACT | the current font follows the locale | `localization.rs:361 current_font_follows_locale` |
| CONTRACT | `#solid:RRGGBB` round-trips through parse | `texture_ref.rs:196 test_solid_color_path_round_trips_through_parse` |
| GUARD | generated-texture sentinels flagged unresolvable | `texture_ref.rs:213 test_generated_texture_sentinels_are_flagged` |
| GUARD | texture-filter wire format survives serde, lowercase alias | `game_config.rs:239 test_game_config_texture_filter_survives_serde_roundtrip` |
| CONTRACT | time_scale is zero only while paused | `pause.rs:402 time_scale_is_zero_only_while_paused` |
| CONTRACT | Menu pauses, the same button resumes | `pause.rs:234 menu_press_pauses_and_same_button_resumes` |
| CONTRACT | click a row executes it, hover moves the highlight | `pause.rs:315 click_on_a_row_executes_it_and_hover_moves_the_highlight` |
| GUARD | a resting cursor does not hover but still clicks | `menu_panel.rs:372 resting_cursor_does_not_hover_but_still_clicks` |
| CONTRACT | row_at round-trips every row center, rejects the bands | `menu_panel.rs:343 row_at_round_trips_every_row_center_and_rejects_the_bands` |
| CONTRACT | navigation wraps both directions | `menu_input.rs:110 test_navigate_wraps_both_directions` |
| GUARD | a held stick scrolls once, not every frame | `menu_input.rs:166 test_held_stick_scrolls_once_not_every_frame` |
| CONTRACT | particles decay to death, dead slots reused | `particles/manager.rs:233 spawn_reuses_dead_slots` |
| CONTRACT | direction spread stays within the cone | `particles/manager.rs:278 direction_spread_stays_within_cone` |
| CONTRACT | an inactive emitter emits nothing | `particles/system.rs:81 inactive_emitter_emits_nothing` |
| CONTRACT | grid energy decays with damping | `grid/grid_mesh.rs:395 energy_decays_with_damping` |
| GUARD | border nodes pinned, interior free | `grid/topology.rs:240 hex_border_nodes_are_pinned_and_interior_free` |
| GUARD | a hidden grid still simulates | `grid/grid_mesh.rs:471 hidden_grid_still_simulates` |
| GUARD | non-finite tunables fall back to the preset | `grid/build.rs:90 test_non_finite_tunables_fall_back_to_the_preset_and_compare_equal` |
| CONTRACT | a resting grid is more transparent than a moving one | `grid/opacity_tests.rs:38 resting_grid_is_more_transparent_than_moving_grid` |
| GUARD | moving the entity translates the mesh without a rebuild | `grid/backdrop_system.rs:251 test_moving_the_entity_translates_the_mesh_without_a_rebuild` |
| GUARD | a NaN tunable does not rebuild every frame | `grid/backdrop_system.rs:182 test_shape_change_rebuilds_but_a_nan_tunable_does_not_rebuild_every_frame` |
| CONTRACT | camera converges within 10 frames at lerp 0.5 | `tests/camera_follow.rs:153 test_camera_converges_within_10_frames_at_lerp_half` |
| CONTRACT | the dead zone ignores targets inside the box | `tests/camera_follow.rs:193 test_dead_zone_ignores_targets_inside_the_box` |
| CONTRACT | holding a direction leads by look_ahead | `tests/camera_follow.rs:271 test_holding_right_leads_the_camera_by_look_ahead_x` |
| GUARD | negative and NaN look_ahead degrade to plain follow | `tests/camera_follow.rs:404 test_negative_and_nan_look_ahead_degrade_to_plain_follow` |
| CONTRACT | patrol arrival waits then reverses | `behavior_runner/mod.rs:375 test_patrol_arrival_enters_waiting_then_reverses_direction` |
| CONTRACT | chase enters and leaves the chasing phase on range | `behavior_runner/mod.rs:427 test_chase_enters_and_leaves_chasing_phase_on_range` |
| CONTRACT | jump fires from a gamepad action and from Space | `tests/behavior_optimization.rs:115 test_platformer_jump_fires_from_gamepad_action_and_from_space` |
| GUARD | device loss is fatal immediately, regardless of streak | `render_manager.rs:477 classify_device_lost_is_fatal_immediately_regardless_of_streak` |
| GUARD | a fatal RenderManager refuses to render | `render_manager.rs:498 fatal_render_manager_refuses_to_render` |
| GUARD | the surface-error streak resets on a good frame | `render_manager.rs:520 surface_error_streak_resets_on_successful_frame` |
| CONTRACT | main-camera sync copies position only | `render_manager.rs:555 test_sync_main_camera_copies_main_camera_entity_position` |
| GUARD | delta time clamped after a stall | `game_loop_manager.rs:154 test_delta_time_is_clamped_after_a_stall` |
| CONTRACT | throttle enforces the target FPS | `game_loop_manager.rs:169 test_throttle_enforces_target_fps` |
| CONTRACT | a pickup is collected once even with two collectors | `pickups.rs:245 test_pickup_collected_once_even_with_two_collectors` |
| CONTRACT | EffectTimer fires exactly when crossing zero | `pickups.rs:320 test_effect_timer_lifecycle` |
| GUARD | `UiElementsHidden` suppresses everything | `ui_element_system.rs:146 hidden_resource_suppresses_everything` |
| CONTRACT | panels draw before buttons and labels | `ui_element_system.rs:172 panels_draw_before_buttons_and_labels` |
| CONTRACT | a button click returns a press event | `ui_element_system.rs:206 button_click_returns_press_event` |
| GUARD | UI stays at its screen position under a moved, zoomed camera | `ui_integration/tests.rs:25 test_ui_stays_at_screen_position_under_moved_zoomed_camera` |
| CONTRACT | nested clips intersect on the batch | `ui_integration/tests.rs:225 test_nested_clips_intersect_on_the_batch` |
| GUARD | pop restores the parent clip for later commands | `ui_integration/tests.rs:250 test_pop_restores_parent_clip_for_later_commands` |
| CONTRACT | the gamepad button translation table is exhaustive | `gamepad_backend.rs:234 button_translation_table_is_exhaustive_and_correct` |
| CONTRACT | dead zone zeroes small values, rescales the rest | `gamepad_backend.rs:282 dead_zone_zeroes_small_values_and_rescales_the_rest` |
| GUARD | hat transitions press/release only on crossings | `gamepad_backend.rs:297 hat_transitions_press_and_release_only_on_crossings` |
| CONTRACT | same glyph, different fonts needs separate textures | `glyph_texture_cache.rs:184 same_glyph_same_size_different_fonts_needs_separate_textures` |
| CONTRACT | tilemap expands into one batch with correct instances | `tilemap_render.rs:61 test_tilemap_expands_into_one_batch_with_correct_instances` |
| CONTRACT | spawning a prefab applies overrides | `tests/prefab_spawning.rs:92 test_spawn_prefab_applies_overrides` |
| GUARD | a failed prefab spawn removes the half-built entity | `tests/prefab_spawning.rs:125 test_spawn_prefab_failure_removes_half_built_entity` |
| CONTRACT | the lifecycle FSM refuses invalid transitions | `tests/lifecycle.rs:67 test_lifecycle_state_transitions` |
| GUARD | the lifecycle survives lock poisoning | `lifecycle.rs:306 test_lifecycle_survives_lock_poisoning` |
| CONTRACT | background covers the window with overscan, behind all | `spawn_helpers.rs:33 test_background_covers_window_with_overscan_behind_everything` |
| GUARD | `ctx.chaos_mode` writes during update/key handlers persist | **MISSING** |
| GUARD | post-tonemap UI pass — authored UI colors display exactly | **MISSING** |
| GUARD | `main_camera_pose` replaces a non-finite/≤0 zoom with 1.0 | **MISSING** — `render_manager.rs:426-445` |
| GUARD | `set_base_path` drops the path-dedup cache | **MISSING** — `assets.rs:421` |
| CONTRACT | `PauseMenu::draw_labeled` with localized labels | **MISSING** — zero coverage once the defaults test goes |
| CONTRACT | `GridMesh::translate` shifts rest AND position, not velocity | **MISSING** |
| CONTRACT | `#rgba` sentinel end-to-end through scene save | **MISSING** |

Paths above are relative to `crates/engine_core/`.

### ui — 123 → 24

| kind | contract / footgun | test |
|---|---|---|
| CONTRACT | a button returns true on the release frame | `ui/tests/ui_interaction_debug.rs:12` — *move inline* |
| CONTRACT | slider maps a click to a value (the only slider test) | `ui/tests/ui_interaction_debug.rs:74` — *move inline* |
| CONTRACT | click state machine reaches `clicked` | `ui/tests/ui_interaction_debug.rs:185` — *move inline* |
| CONTRACT | a click outside the button does not fire | `ui/tests/ui_interaction_debug.rs:278` — *move inline* |
| GUARD | press inside, release outside cancels | `ui/tests/ui_interaction_debug.rs:301` — *move inline* |
| CONTRACT | `InputState` maps the handler's mouse snapshot | `ui/tests/ui_interaction_debug.rs:140` — *move inline, drop the end_frame third* |
| GUARD | `wants_mouse` holds press → release frame | `ui/src/interaction/tests.rs:125 test_wants_mouse_holds_from_widget_press_through_release_frame` |
| GUARD | a missed release event frees the gesture | `ui/src/interaction/tests.rs:172 test_missed_release_event_frees_the_mouse_gesture` |
| GUARD | a blocking rect makes an outside widget inert | `ui/src/interaction/tests.rs:49 test_blocking_rect_makes_outside_widget_inert` |
| GUARD | overlay-scope widget stays interactive over a blocking rect | `ui/src/interaction/tests.rs:68 test_overlay_scope_widget_stays_interactive_over_blocking_rect` |
| CONTRACT | blocked widget state survives the frame | `ui/src/interaction/tests.rs:97 test_blocked_widget_persistent_state_survives_frame` |
| CONTRACT | focused state survives an unseen frame | `ui/src/interaction/tests.rs:228 test_focused_widget_state_survives_unseen_frame` |
| GUARD | an elevated layer escapes a Content clip pair (z-bands) | `ui/src/draw/tests.rs:233 test_elevated_layer_escapes_content_clip_pair` |
| CONTRACT | layers flush in enum order | `ui/src/draw/tests.rs:192 test_layers_flush_in_enum_order` |
| CONTRACT | layer depths are banded | `ui/src/draw/tests.rs:212 test_layer_depths_are_banded` |
| CONTRACT | push/pop layer nesting | `ui/src/draw/tests.rs:259 test_push_pop_layer_nest` |
| CONTRACT | flush idempotent, clear resets the stack | `ui/src/draw/tests.rs:274 test_flush_is_idempotent_and_clear_resets_stack` |
| CONTRACT | typing replaces the selection | `ui/src/text_edit.rs:218 test_typing_replaces_selection` |
| CONTRACT | plain arrow collapses selection to the edge | `ui/src/text_edit.rs:284 test_plain_arrow_collapses_selection_to_edge` |
| CONTRACT | cursor_from_click picks the nearest boundary | `ui/src/text_edit.rs:337 test_cursor_from_click_picks_nearest_boundary` |
| GUARD | empty-string operations are safe | `ui/src/text_edit.rs:352 test_empty_string_operations_are_safe` |
| GUARD | a typed commit beyond the soft range is NOT clamped but IS flagged | `ui/src/context/scrub_tests.rs:267 test_float_scrub_typed_commit_beyond_soft_range_not_clamped` |
| GUARD | an invalid buffer flags red and reverts on commit | `ui/src/context/scrub_tests.rs:163 test_float_invalid_buffer_flags_and_reverts_on_commit` |
| GUARD | Escape restores the scrub's start value | `ui/src/context/scrub_tests.rs:98 test_float_scrub_escape_restores_start_value` |
| GUARD | a scrub needs the click threshold; sub-threshold still focuses | `ui/src/context/scrub_tests.rs:50 test_float_scrub_requires_threshold_click_still_focuses` |
| CONTRACT | repeat fires after the delay, then at the interval | `ui/src/input_state.rs:344 test_repeat_fires_after_delay_then_at_interval` |
| CONTRACT | keycode_to_char, shift blocks the top row | `ui/src/input_state.rs:313 test_keycode_to_char_shift_blocks_top_row` |
| CONTRACT | programmatic focus arms the edit without a click (F2) | `ui/src/context/focus_tests.rs:19 test_focus_text_input_arms_edit_without_a_click` |
| GUARD | the glyph cache evicts when full | `ui/src/font/glyph_cache.rs:178 test_glyph_cache_evicts_when_full` |
| GUARD | UI text y = baseline; boxed text uses `label_in_bounds_styled` | **MISSING** |
| CONTRACT | `layout_text` / `measure_text` — the crate's only text math | **MISSING** — `font/layout.rs`'s single test is a struct literal |
| CONTRACT | PushClipRect/PopClipRect drive `SpriteBatcher::set_clip` | **MISSING** |
| GUARD | KeyRepeat slots are per-key independent | **MISSING** |

### renderer — 92 → 17

| kind | contract / footgun | test |
|---|---|---|
| GUARD | **`offset_of!` vertex layouts** — every attribute's offset, format, shader_location | **MISSING** — descriptors hand-compute 11 offsets as `size_of::<[f32; N]>()`; tests assert only count and stride. Highest-value guard in the workspace |
| CONTRACT | vertex stride matches the shader | `renderer/src/sprite_data.rs:304 test_sprite_vertex_bytemuck_cast` |
| CONTRACT | instance stride matches the shader | `renderer/src/sprite_data.rs:356 test_sprite_instance_bytemuck_cast` |
| CONTRACT | a default instance is a plain unlit quad | `renderer/src/sprite_data.rs:379 test_sprite_instance_default_shape_is_plain_quad` |
| CONTRACT | `CameraUniform` is bytemuck-safe | `renderer/src/sprite_data.rs:540 test_camera_uniform_bytemuck` |
| CONTRACT | `Sprite::to_instance` maps every field | `renderer/src/sprite.rs:179 test_sprite_to_instance` |
| CONTRACT | batching groups by texture | `renderer/src/sprite/batch.rs:322 test_sprite_batcher_groups_by_texture` |
| CONTRACT | a clip splits a same-texture batch | `renderer/src/sprite/batch.rs:341 test_sprite_batcher_splits_same_texture_by_clip` |
| GUARD | NaN depth sorts without panicking (`total_cmp`) | `renderer/src/sprite/batch.rs:223 test_sprite_batch_sort_handles_nan_depth_without_panicking` |
| GUARD | the sorted flag resets on add | `renderer/src/sprite/batch.rs:281 test_sprite_batch_sorted_flag_reset_on_add` |
| GUARD | identical batches skip the upload | `renderer/src/sprite/instance_cache.rs:95 test_identical_batches_skip_upload` |
| GUARD | a layout change uploads even with identical bytes | `renderer/src/sprite/instance_cache.rs:121 test_layout_change_triggers_upload_even_with_same_bytes` |
| CONTRACT | quantize rounds outward to cover partial pixels | `renderer/src/scissor.rs:77 test_quantize_rounds_outward_to_cover_partial_pixels` |
| GUARD | non-finite scissor inputs yield an empty rect | `renderer/src/scissor.rs:99 test_quantize_non_finite_inputs_yield_empty` |
| GUARD | clamp trims overhang on a resize race | `renderer/src/scissor.rs:117 test_clamp_trims_overhang_on_resize_race` |
| CONTRACT | a clip intersects the default scissor | `renderer/src/scissor.rs:167 test_batch_scissor_clip_intersects_default` |
| GUARD | the device-loss latch is one-way, shared across clones | `renderer/src/device_status.rs:85 latch_clones_share_state` |
| GUARD | a same-size reconfigure is forced when asked | `renderer/src/device_status.rs:120 resize_action_forces_reconfigure_at_same_size` |
| CONTRACT | a filter maps every sampler field | `renderer/src/texture_filter.rs:64 test_linear_filter_maps_every_sampler_filter_to_linear` |
| GUARD | `write_buffer` flushes at submit — one buffer per per-frame value | **MISSING** |
| GUARD | bind groups cached, never created per frame | **MISSING** |
| GUARD | `TextureHandle::WHITE` is reserved; no handle can equal it | **MISSING** — `texture.rs:137` is a comment, not a test |
| CONTRACT | `bloom_width`/`bloom_height` | **MISSING** — the two "tests" re-derive the arithmetic and never call them |

### physics — 64 → 20

| kind | contract / footgun | test |
|---|---|---|
| GUARD | a parented rigid body is treated as world-space | `physics/src/physics_system/tests.rs:491 test_parented_entity_with_rigid_body_is_treated_as_world_space` |
| GUARD | a started event is delivered exactly once across zero-step updates | `physics/src/physics_system/tests.rs:281 test_started_event_is_delivered_exactly_once_across_zero_step_updates` |
| GUARD | a zero-step update emits no events (no stale re-delivery) | `physics/src/physics_system/tests.rs:302 test_zero_step_update_emits_no_collision_events` |
| GUARD | every catch-up sub-step's events survive | `physics/src/physics_system/tests.rs:319 test_events_from_all_sub_steps_in_one_update_survive` |
| GUARD | a second `take_collision_events` in a frame returns empty | `physics/src/physics_system/tests.rs:341 test_take_collision_events_drains_the_buffer` |
| GUARD | `apply_force` lasts exactly one update | `physics/src/physics_system/tests.rs:363 test_apply_force_lasts_exactly_one_update` |
| GUARD | a force on a zero-step frame acts next stepped frame | `physics/src/physics_system/tests.rs:394 test_force_applied_on_zero_step_frame_acts_on_next_stepped_frame` |
| GUARD | `reset_body` is deferred for same-frame spawns | `physics/src/physics_system/tests.rs:78 test_reset_body_is_deferred_for_same_frame_spawns` |
| GUARD | catch-up steps capped after a stall | `physics/src/physics_system/tests.rs:112 test_catch_up_steps_are_capped_after_a_stall` |
| CONTRACT | gravity moves a dynamic body | `physics/src/physics_system/tests.rs:150 test_gravity_affects_dynamic_body` |
| GUARD | direct world removal cleans up physics state | `physics/src/physics_system/tests.rs:47 test_direct_world_removal_cleans_up_physics_state` |
| CONTRACT | clear allows a resync from ECS | `physics/src/physics_system/tests.rs:218 test_clear_allows_resync_from_ecs` |
| CONTRACT | an external transform edit teleports a live body (GPP-09) | `physics/tests/external_edits.rs:14 test_external_transform_edit_teleports_live_body` |
| GUARD | the physics writeback is not mistaken for an external edit | `physics/tests/external_edits.rs:49 test_physics_writeback_is_not_mistaken_for_external_edit` |
| GUARD | an identical transform write pushes nothing | `physics/tests/external_edits.rs:80 test_identical_transform_write_pushes_nothing` |
| CONTRACT | a collider edit rebuilds the live rapier collider | `physics/tests/external_edits.rs:104 test_collider_edit_rebuilds_live_rapier_collider` |
| CONTRACT | removing the Collider drops the rapier collider | `physics/tests/external_edits.rs:145 test_collider_component_removal_drops_rapier_collider` |
| CONTRACT | a sensor collider fires intersection events | `physics/src/physics_world/tests.rs:400 test_sensor_collider_fires_intersection_events` |
| CONTRACT | contact points are world-space pixels | `physics/src/physics_world/tests.rs:359 test_contact_points_are_in_world_space` |
| CONTRACT | raycast normalizes direction, distance in pixels | `physics/src/physics_world/tests.rs:335 test_raycast_normalizes_direction_so_distance_is_in_pixels` |
| GUARD | an invalid pixels-per-meter scale is sanitized at creation | `physics/src/physics_world/tests.rs:312 test_invalid_scale_in_struct_literal_is_sanitized_at_world_creation` |
| CONTRACT | capsule-Y half-height excludes the two cap radii | `physics/src/components.rs:440 test_collider_shapes` *(rename)* |
| CONTRACT | a shape cycle carries tuned dimensions | `physics/src/components.rs:537 test_shape_cycle_carries_tuned_dimensions` |
| CONTRACT | physics components round-trip through the dynamic tier | `physics/src/register.rs:62 test_physics_components_round_trip_through_the_dynamic_tier` |
| CONTRACT | CCD + restitution: a ball bounces off a static brick | `physics/tests/ball_brick_bounce.rs:51 ball_bounces_off_static_brick` |
| GUARD | collision groups / filters exclude non-overlapping layers | **MISSING** — `physics_world/bodies.rs:97-102` |
| CONTRACT | `Collider.offset` collides at the offset position | **MISSING** — `bodies.rs:89` |
| GUARD | a live `RigidBody` config edit still needs a rebuild | **MISSING** (pins the known limitation) |
| CONTRACT | kinematic bodies move, ignore gravity, still emit events | **MISSING** |

### input — 74 → 15

| kind | contract / footgun | test |
|---|---|---|
| CONTRACT | queued events do not apply until `process_queued_events` | `input/tests/input_event_queue.rs:6 test_input_event_queuing` |
| CONTRACT | `update` clears just-pressed / just-released | `input/tests/input_event_queue.rs:44 test_update_clears_just_states` |
| CONTRACT | multiple events apply in order | `input/tests/input_event_queue.rs:69 test_multiple_events_processing_order` |
| GUARD | `InputMapping::new()` is EMPTY — nothing bound implicitly | `input/tests/input_mapping.rs:14 test_new_mapping_is_empty` |
| CONTRACT | unbinding a source removes it from every action | `input/tests/input_mapping.rs:105 test_unbind_source_removes_from_all_actions` |
| GUARD | a second bound source does not re-trigger activation | `input/tests/input_handler_integration.rs:63 test_second_source_does_not_retrigger_activation` |
| CONTRACT | releasing one source keeps the action active | `input/tests/input_handler_integration.rs:82 test_releasing_one_source_keeps_action_active` |
| CONTRACT | an axis source drives an action across frames | `input/tests/input_handler_integration.rs:214 test_axis_source_drives_action_across_frames` |
| GUARD | a negative axis binding ignores positive deflection | `input/tests/input_handler_integration.rs:244 test_negative_axis_binding_ignores_positive_deflection` |
| GUARD | disconnect drops state with no just-released edge | `input/tests/input_handler_integration.rs:264 test_connect_event_registers_and_disconnect_drops_state` |
| CONTRACT | first position update records position, no delta | `input/tests/mouse.rs:20 test_first_position_update_records_position_without_delta` |
| CONTRACT | movement delta resets each frame | `input/tests/mouse.rs:58 test_movement_delta_resets_each_frame` |
| CONTRACT | wheel accumulates, normalized to lines | `input/tests/mouse.rs:122 test_mouse_wheel` |
| GUARD | an axis fires once on crossing and re-arms below threshold | `input/src/gamepad.rs:227 axis_just_activated_fires_once_on_crossing_and_rearms_below_threshold` |
| GUARD | opposite directions track edges independently | `input/src/gamepad.rs:264 opposite_directions_track_edges_independently` |
| CONTRACT | `clear_frame_state` fans out to every child pad | `input/tests/gamepad.rs:131 test_gamepad_manager_update` |
| CONTRACT | default pairing isolates player devices | `input/src/player.rs:486 default_pairing_isolates_player_devices` |
| CONTRACT | `assign_pad` repoints pad sources only | `input/src/player.rs:513 assign_pad_repoints_pad_sources_without_touching_keyboard` |
| CONTRACT | `move_y` merges digital + stick and clamps | `input/src/player.rs:547 move_y_merges_digital_and_stick_and_clamps` |
| CONTRACT | binding changes set dirty; `take_dirty` clears it | `input/src/player.rs:446 binding_changes_set_dirty_and_take_dirty_clears_it` |
| CONTRACT | a repeated press does not re-trigger just_pressed | `input/src/button_tracker.rs:96 test_repeated_press_does_not_retrigger_just_pressed` |
| CONTRACT | `convert_physical_key` / `handle_window_event` (the winit boundary) | **MISSING** — `tests/keyboard.rs:118` is comments with no code; both are headlessly testable |
| GUARD | `AXIS_ACTIVATION_THRESHOLD` default is 0.5 | **MISSING** — every axis test passes 0.5 explicitly |

### audio — 26 → 9

| kind | contract / footgun | test |
|---|---|---|
| GUARD | a disabled manager loads and plays as a no-op | `audio/src/manager/tests.rs:82 test_disabled_manager_loads_and_plays_as_noop` |
| GUARD | a disabled manager still rejects invalid handles | `audio/src/manager/tests.rs:91 test_disabled_manager_still_rejects_invalid_handles` |
| GUARD | disabled music loads but reports not playing | `audio/src/manager/tests.rs:212 test_disabled_manager_music_loads_but_reports_not_playing` |
| CONTRACT | typed errors: IoError vs DecodeError | `audio/src/manager/tests.rs:138 test_load_sound_from_invalid_bytes_returns_decode_error` |
| CONTRACT | `enable_output` preserves sounds, ids and volumes | `audio/src/manager/tests.rs:255 test_enable_output_preserves_sounds_ids_and_volumes` |
| CONTRACT | music started while disabled is recorded as pending | `audio/src/manager/tests.rs:293 test_start_music_while_disabled_records_pending` |
| GUARD | `stop_music` clears the pending request | `audio/src/manager/tests.rs:306 test_stop_music_while_disabled_clears_pending` |
| GUARD | a new music request replaces the pending one | `audio/src/manager/tests.rs:318 test_new_music_request_replaces_pending` |
| GUARD | a failed `play_music` leaves no pending request | `audio/src/manager/tests.rs:335 test_play_music_missing_file_leaves_no_pending` |
| CONTRACT | volume setters clamp out-of-range values | `audio/src/manager/tests.rs:192 test_volume_setters_clamp_out_of_range_values` |
| CONTRACT | an ENABLED manager's sink bookkeeping | **MISSING** — `active_sound_count`, `update()`'s retain, `stop`, `stop_all` are asserted only against a disabled manager where the count is structurally 0 |

### Structural guards — status roll-up

| guard | status |
|---|---|
| WCAG surface ladder + popup border | EXISTS — `editor/src/theme/tests.rs:103`, `:119` |
| command-API doc/verb drift | EXISTS — `editor/src/command_api/specs.rs:188`, `:200` |
| input JSON fixture + legacy fallbacks | EXISTS — `engine_core/src/input_settings_io.rs:174`, `:204`, `:220`, `:232` |
| menu-label ↔ PanelId drift (ARCH-101) | EXISTS — `editor/src/dock/tests.rs:268` (asserts the deliberate `None`s too) |
| physics root-entity rule | EXISTS — `physics/src/physics_system/tests.rs:491` |
| scene-serializer table drift | **MISSING** |
| `offset_of!` GPU vertex layouts | **MISSING** |
| Behavior scene fixture (all variants) | **PARTIAL** — one variant at `engine_core/src/scene_serializer_tests.rs:280` |
| merge isolation between command types / entities | **MISSING** — gesture boundary covered, cross-type and cross-entity not |

### Per-crate totals

| crate | current | keep |
|---|---:|---:|
| common | 41 | 16 |
| ecs (inline `src/`) | 119 | 19 |
| ecs (`tests/`) | 94 | 21 |
| ecs_macros | 3 | 1 |
| editor | 477 | 62 |
| editor_integration | 150 | 30 |
| engine_core | 394 | 78 |
| ui | 123 | 24 |
| renderer | 92 | 17 |
| physics | 64 | 20 |
| input | 74 | 15 |
| audio | 26 | 9 |
| **total** | **1657** | **312** |

Six files empty out entirely: `ecs/tests/{component,init,system}.rs`,
`engine_core/tests/init.rs`, `input/tests/input_handler.rs`, and
`ui/tests/ui_interaction_debug.rs` after its six keeps move inline.
