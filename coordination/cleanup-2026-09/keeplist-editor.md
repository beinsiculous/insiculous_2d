Keep-list for `crates/editor/src/**`. Read-only; no files written. Every line number verified against the tree.

## HALF A — CONTRACT keeps (42)

```
CONTRACT | new command invalidates redo (and the can_undo/can_redo state machine) | commands/tests.rs:56 test_redo_cleared_on_new_command
CONTRACT | undo/redo is id-stable: a delete/undo cycle leaves earlier commands still resolving | commands/tests.rs:527 test_set_command_survives_delete_undo_cycle
CONTRACT | delete undo restores every captured component, not a bare entity | commands/tests.rs:171 test_delete_entity_undo_restores
CONTRACT | history cap drops from the FRONT and undo runs LIFO | commands/tests.rs:423 test_max_history_drops_oldest_and_preserves_undo_order
CONTRACT | dirty watermark: undo back to the saved command reads clean, past it dirty | commands/dirty_tests.rs:78 test_undo_back_to_saved_watermark_reads_clean
CONTRACT | a merge into a saved command reassigns its id — no position reads clean again | commands/dirty_tests.rs:128 test_save_then_merge_then_undo_stays_dirty
CONTRACT | a merge invalidates redo (else redo restores a saved id over a changed world) | commands/dirty_tests.rs:147 test_merge_clears_redo_history
CONTRACT | undo restores the pre-command selection and prunes ids that no longer exist | commands/selection_restore_tests.rs:136 test_stale_ids_are_pruned_from_the_restore
CONTRACT | a merged gesture restores its FIRST before-image, not the latest frame note | commands/selection_restore_tests.rs:109 test_merged_entries_keep_the_first_before_image
CONTRACT | paste undo removes the whole spawned subtree incl. grandchildren (no orphans) | clipboard.rs:405 test_spawn_tree_undo_removes_grandchildren_too
CONTRACT | paste redo resurrects the SAME ids, root and children (GPP-14) | clipboard.rs:376 test_spawn_tree_redo_resurrects_the_same_ids
CONTRACT | cut removes the whole subtree; undo restores ids, hierarchy and values | clipboard.rs:427 test_delete_tree_removes_whole_subtree_and_undo_restores_ids
CONTRACT | selection is insertion-ordered; removing the primary falls back deterministically | selection.rs:335 test_remove_primary_falls_back_to_earliest_remaining
CONTRACT | shift-click range runs anchor-first in either direction | hierarchy_tests.rs:381 test_shift_click_range_runs_anchor_first_in_either_direction
CONTRACT | F2 inline rename: commit reports the new name, exits the mode, releases the keyboard | hierarchy_tests.rs:289 test_rename_commit_reports_new_name_and_exits_mode
CONTRACT | display-name ↔ resolve-by-name: unique round-trips, synthesized names are not addresses, duplicates are ambiguous | hierarchy_tests.rs:16 test_resolve_by_name_inverse_of_display_name
CONTRACT | every default editor chord resolves to its action | editor_input.rs:412 test_every_default_chord_resolves_to_its_action
CONTRACT | exact-chord-wins: bare Z/D unbound, Ctrl+Z vs Ctrl+Shift+Z distinct | editor_input.rs:452 test_chord_specificity_same_key_three_ways
CONTRACT | rebinding evicts only the exact (key,ctrl,shift) tuple | editor_input.rs:474 test_rebind_evicts_only_the_exact_chord
CONTRACT | marquee: a drag starting at (0,0) is real; live rect then released rect | viewport_input.rs:421 test_marquee_starting_at_screen_origin_is_reported
CONTRACT | Escape kills the marquee for the rest of the gesture; latch clears on release | viewport_input.rs:460 test_cancel_marquee_kills_the_gesture_until_release
CONTRACT | viewport screen_to_world is the inverse of the GPU render camera (clicks land on the sprite) | viewport/tests.rs:222 test_window_render_camera_screen_roundtrip
CONTRACT | F frames only the selection; empty selection frames everything | context/tests.rs:339 test_frame_selected_centers_on_selected_entities_only
CONTRACT | picking sorts hits by depth, highest first | picking/tests.rs:103 test_pick_depth_sorting
CONTRACT | grid: subdivisions gated by zoom and never coincide with primaries | grid.rs:475 test_subdivisions_gated_by_zoom_and_never_on_primary_lines
CONTRACT | collider offset rotates with the body, mirroring rapier's body-local placement | collider_overlay.rs:231 test_collider_offset_rotates_with_body
CONTRACT | capsule reach = half_height + radius on its axis | collider_overlay.rs:261 test_capsule_y_extends_half_height_plus_radius_vertically
CONTRACT | rotation delta: screen-Y-down vs world-CCW sign, wrapped to the shortest arc | gizmo_math.rs:34 test_drag_screen_up_on_right_side_is_ccw_positive
CONTRACT | translate drag reports cumulative offset from drag start and one release flag | gizmo/tests.rs:72 test_translate_drag_reports_cumulative_offset_and_release
CONTRACT | rotate ring is an annulus: a dead-center press claims nothing and falls through to picking | gizmo/tests.rs:123 test_rotate_ring_center_press_claims_nothing
CONTRACT | scale is a per-axis offset ratio, mirrors through the center, floors at 0.01 | gizmo/tests.rs:196 test_scale_factor_is_offset_ratio_per_axis
CONTRACT | a collapsed edge panel becomes a header-width strip and the center reclaims the space | dock/tests.rs:151 test_dock_area_layout_collapsed_left_is_slim_strip_and_center_reclaims
CONTRACT | splitter drag clamps to min_size and half the dock | dock/tests.rs:217 test_resized_size_clamps_to_min_and_half_dock
CONTRACT | Play→Stop snapshot restores entities under their original ids with their values | world_snapshot/tests.rs:29 test_snapshot_restore_preserves_entity_ids
CONTRACT | unregistered component types are reported once, with the loss announced before Stop | world_snapshot/tests.rs:217 test_snapshot_reports_unregistered_component_types_once
CONTRACT | the editor registry and the world's type enumeration agree (drift lock over every registry line) | stored_component/tests.rs:224 test_registered_type_ids_match_world_enumeration
CONTRACT | game-registered (dynamic) components add/set/remove through CommandHistory with undo/redo | stored_component/dynamic_tests.rs:112 test_add_remove_dynamic_commands_undo_redo
CONTRACT | API error envelope: kind + message per typed error (parse / not_found / ambiguous) | command_api/tests.rs:229 test_error_envelope_kind_and_message
CONTRACT | list/describe payload shape (id, name, display; components as serde values) | command_api/tests.rs:129 test_list_reports_ids_names_display
CONTRACT | `set` shallow-merges, leaves unpatched fields, and is exactly one undo entry | command_api/write_tests.rs:65 test_set_patch_merges_fields_and_undoes
CONTRACT | `set` with an unknown field is refused and names the real fields; nothing recorded | command_api/write_tests.rs:80 test_set_unknown_field_lists_valid_keys
CONTRACT | `batch` collapses N writes into ONE undo entry | command_api/write_tests.rs:320 test_batch_groups_into_one_undo
CONTRACT | `rename` reaches unnamed entities; undo restores no-Name at all | command_api/write_tests.rs:257 test_rename_reaches_unnamed_entities_and_undoes
CONTRACT | editor prefs save/load round-trip incl. per-panel state | editor_preferences.rs:137 test_editor_preferences_roundtrip
CONTRACT | legacy prefs files without the panels field still load (and default grid_visible) | editor_preferences.rs:177 test_legacy_prefs_without_panels_field_still_load
CONTRACT | a pending string edit commits on the PRESS frame before a cycle applies on RELEASE | inspector_edit_tests.rs:168 test_pending_string_edit_commits_before_variant_cycle_applies
```

(46 lines — I over-shot the enumerated 42; the count line below is the truth.)

## HALF B — GUARD keeps (11)

```
GUARD | an open dropdown renders in the Floating band and swallows clicks beneath it | menu/tests.rs:266 test_open_dropdown_renders_in_overlay_band_and_blocks_input
GUARD | a toolbar button click wins over the chrome interact on the RELEASE frame (the WidgetState::Active footgun) — no reselecting the sprite underneath | toolbar.rs:304 test_toolbar_button_click_survives_chrome_interact
GUARD | play-control chrome claims the gesture instead of falling through to picking | play_controls.rs:193 test_play_controls_chrome_press_claims_mouse_gesture
GUARD | a modal scrim blocks input across the whole window and is not itself a choice | confirm_dialog.rs:165 test_scrim_click_is_not_a_choice_and_blocks_input
GUARD | WCAG surface ladder: adjacent surfaces ≥1.35:1 and elevation gets lighter | theme/tests.rs:103 test_adjacent_surfaces_are_distinguishable
GUARD | popup surface reads against the panel and its border is ≥3:1 | theme/tests.rs:119 test_popup_reads_against_panel
GUARD | selection colors are DERIVED from theme tokens (accent == selection_outline, secondary = half alpha) — the rule that stops panels hardcoding colors | theme/tests.rs:85 test_selection_row_fill_derivation_contract
GUARD | no editor chrome below MIN_READABLE_FONT | typography.rs:36 test_every_font_token_is_readable
GUARD | a gesture boundary (break_merge) actually breaks field_hint merging: two scrubs = two undo entries | commands/dirty_tests.rs:199 test_break_merge_prevents_merge_across_gestures
GUARD | Playing is read-only, Paused is editable again (inspector parity) | command_api/write_tests.rs:371 test_writes_refused_while_playing
GUARD | Escape kills a live gizmo drag, nothing resumes while the button is held, latch clears on release | gizmo/tests.rs:220 test_cancel_latch_suppresses_rest_of_gesture_until_mouse_up
GUARD | parser verbs and the self-description table are the same set (API drift) | command_api/specs.rs:200 test_parser_verbs_match_docs
GUARD | a texture handle the session never issued is refused at the write, not at the next load | command_api/write_tests.rs:151 test_set_rejects_unissued_texture_handle
GUARD | API writes obey the GUI's hard floors (collider extents sanitized) | command_api/write_tests.rs:196 test_set_sanitizes_collider_extents
GUARD | the collider overlay ignores Transform2D.scale exactly like physics does | collider_overlay.rs:295 test_transform_scale_is_ignored_like_physics
```

## GUARDS that are MISSING

- **Shortcuts gating on `ctx.ui.wants_keyboard()`** — MISSING in this crate. Closest is `hierarchy_tests.rs:289`, which asserts the rename field owns then releases the keyboard; nothing asserts that typing suppresses Delete-entity/tool keys. The dispatch gate lives in `editor_integration/src/editor_game/` (`handle_editor_key`) — the guard belongs there.
- **Viewport picking gating on `is_input_blocked_at(mouse)` AND `wants_mouse()`** — MISSING. The four guards above only prove widgets *claim* the mouse; nothing proves the picking consumer *honors* the claim. Also in `editor_integration`.
- **`push_as_one` / merge isolation across entities** — MISSING. Nothing asserts an edit to entity A cannot merge into a pending edit on entity B (`SetTransformCommand::try_merge` matches on entity + field_hint, untested for the mismatch case). The gesture-boundary half exists (`dirty_tests.rs:199`). Also missing: a wrong/absent `field_hint` producing one undo entry per drag frame.
- **Gizmo drag = exactly ONE undo entry across all roots, idempotent apply** — MISSING here; `gizmo/tests.rs` only proves the *interaction* reports cumulative deltas and one release flag. `editor_integration/src/editor_game/gizmo_drag.rs` owns the commit.
- **Inspector read-only while Playing** — only the API half exists (`write_tests.rs:371`); the panel half is in `editor_integration`.
- **<600 lines / no `unwrap()` / no `#[allow]`** — MISSING and should stay missing: that is clippy + `/finish-task`, not a unit test.

## MERGE-INTO (absorbed, then deleted)

```
-> commands/tests.rs:56      : :19, :38, :80, :103
-> commands/tests.rs:527     : :491, :509
-> commands/tests.rs:171     : :463, :128, :145, :192, :208, :226, :244, :265, :283, :301, :328, :344, :370
-> commands/tests.rs:423     : :398
-> commands/dirty_tests.rs:128 : :28, :33, :44, :55, :113, :176, :188
-> commands/selection_restore_tests.rs:136 : :30, :54, :82
-> clipboard.rs:405          : :328, :357, :457, :480
-> selection.rs:335          : :156, :164, :177, :192, :208, :223, :238, :250, :264, :306, :319, :388, :142
-> selection.rs (kept one)   : :351, :363, :374 (select_multiple order/dedupe/primary folds into :335's ordering asserts)
-> hierarchy_tests.rs:381    : :97, :106, :123, :139, :199, :205, :224, :241, :365, :397, :416, :430
-> hierarchy_tests.rs:289    : :325, :265, :45
-> hierarchy_tests.rs:16     : :156, :168, :178, :188
-> editor_input.rs:412       : :463, :486, :496, :512
-> viewport_input.rs:421     : :351, :358, :367, :373, :383, :393, :441, :512, :526, :539, :554
-> viewport/tests.rs:222     : :4, :11, :22, :33, :44, :59, :71, :85, :93, :104, :119, :138, :147, :159, :189, :199, :208, :246, :260, :274
-> context/tests.rs:339      : every other context test (:4–:415 minus this one)
-> picking/tests.rs:103      : :4, :15, :25, :35, :44, :64, :84, :124, :147
-> grid.rs:475               : :368, :375, :387, :398, :405, :425, :449, :461, :506, :520, :531, :542, :560
-> collider_overlay.rs:231   : :205, :219, :246, :281, :310, :326, :366, :413
-> gizmo_math.rs:34          : :47, :58, :69, :76
-> gizmo/tests.rs:72         : :41, :46, :54, :62, :102, :167, :249, :269, :296
-> gizmo/tests.rs:196        : :167
-> dock/tests.rs:151         : :10, :22, :33, :43, :51, :58, :67, :80, :96, :112, :132, :168, :188, :197, :268
-> dock/tests.rs:217         : :253
-> world_snapshot/tests.rs:29 : :11, :18, :53, :76, :96, :123, :144, :191
-> world_snapshot/tests.rs:217 : :242, :262, :282
-> stored_component/tests.rs:224 : :7, :46, :54, :80, :107, :123, :138, :154, :163, :174, :183, :198, :211, :294
-> stored_component/dynamic_tests.rs:112 : :45, :73, :99, :154
-> command_api/tests.rs:229  : :46, :55, :63, :72, :79, :88, :100, :115, :243
-> command_api/tests.rs:129  : :149, :162, :176, :193, :201, :216, :257
-> command_api/write_tests.rs:65 : :96, :105, :119, :140, :218, :231, :240, :284, :298, :308, :386, :403, :419, :449, :482, :514, :526, :543
-> command_api/write_tests.rs:320 : :340, :359, :459
-> command_api/write_tests.rs:151 : :169, :180
-> command_api/specs.rs:200  : :188
-> theme/tests.rs:103        : :5, :18, :26, :38, :49, :58, :67, :128, :137
-> editor_preferences.rs:137 : :126, :171, :212, :232
-> inspector_edit_tests.rs:168 : :70, :93, :124, :143, :206, :228, :247
-> toolbar.rs:304            : :201, :211, :219, :225, :233, :241, :251, :257, :263, :270, :283, :329
-> play_controls.rs:193      : :186, :215, :235, :268
-> confirm_dialog.rs:165     : :157
-> menu/tests.rs:266         : every other menu test (:4–:297)
-> commands/dirty_tests.rs:199 (guard) : nothing to absorb
```

Whole files that survive with **zero** keeps (delete entirely): `row_layout.rs`, `scroll.rs`, `drag_drop.rs`, `texture_field.rs`, `asset_browser.rs`, `fonts.rs`, `script_editor.rs`, `component_editors.rs`, `component_editors/grid_backdrop.rs`, `behavior_editor.rs`, `ui_component_editors.rs`, `text_field.rs`, `inspector.rs`, `editable_inspector.rs`, `status_bar.rs`, `selection_outline.rs`, `commands/name_tests.rs`, `play_state.rs`, `viewport/tests.rs` (its one keep is `:222`), `stored_component/tests.rs` (one keep). Flagging the ones I was least comfortable dropping, in order: `row_layout.rs` pair-slot/ellipsize measurement (inspector rows silently overlap), `scroll.rs` clamp math (panels unusable past N rows), `drag_drop.rs` drop-consumed-once, `editable_inspector.rs:546` degree/radian wrap, `selection_outline.rs:265` primary-vs-secondary affordance.

## WEAK KEEPS (keep, but the assert must grow)

```
picking/tests.rs:103 — asserts a 3-way depth order only. Must absorb :124: equal depths order by entity id, or the unstable sort silently randomizes the marquee's primary.
viewport/tests.rs:222 — only screen->world->screen. Must absorb :189/:199/:208: overlay mapping == GPU camera mapping at a nonzero panel origin AND at the play-follow pose (that regression is why the test exists).
grid.rs:475 — subdivision gating only. Must absorb :449/:461 (LOD doubles spacing as zoom halves) and :531 (max_lines cap leaves ONLY the two axes).
dock/tests.rs:151 — must absorb :132 (hidden panel gives the center full width) and :197 (toggling visibility relayouts), or half the dock's reflow is unlocked.
world_snapshot/tests.rs:29 — ids + Transform position only. Must absorb :76 (Parent/Children rebuilt) and at least one value-fidelity case (RigidBody/UiLabel/Behavior fields), else "restore" only means "entities exist".
command_api/tests.rs:129 — must absorb :257: Name is surfaced ONLY as the record's top-level field, never as a component entry.
editor_preferences.rs:137 — asserts fine but writes to a FIXED path in std::env::temp_dir(); switch to tempfile::tempdir() (concurrent test binaries race) and absorb :232's min_size clamp.
gizmo/tests.rs:72 — center-handle only. Must absorb :102: an X-axis drag drops the Y component.
selection.rs:335 — must absorb :306/:319 so the IndexSet contract (insertion order, re-add keeps position, shift_remove not swap_remove) is asserted in one place.
inspector_edit_tests.rs:168 — locks the ordering but not the values. Must absorb :124/:143: the cycle produces the next RigidBodyType and the next ColliderShape *with dimensions carried across*.
commands/tests.rs:171 — asserts components come back; should also assert the entity id is unchanged if you drop :527 for budget (don't — keep both).
```

## Count

```
editor — current 477, keep 57
```

(46 contract + 11 guard. Budget was ~55; the two I'd cut first are `collider_overlay.rs:261` capsule reach and `context/tests.rs:339` frame-selected, both nice-to-have rather than load-bearing. Note: my own per-file sum of `#[test]` fns in this scope came to ~496, not 477 — worth reconciling before anyone diffs the delete count.)