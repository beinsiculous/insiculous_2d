# Editor Crate — Agent Context

You are working in the editor crate. UI panels, tools, inspector, hierarchy — the editor's data and widgets.
This crate has NO dependency on engine_core. It depends on: ecs, ui, input, renderer, physics, common, and music_midi without its `synth` feature (the composer edits and saves songs, never renders them — `cargo test -p editor` alone is what proves that build).

## Architecture
```
EditorContext (selection, tool state, play state, camera, theme, status_bar, fonts, inspector_scroll)
│   (CommandHistory itself lives on editor_integration's EditorGame and is threaded into panel renderers)
├── Panels: SceneView, Hierarchy, Inspector, AssetBrowser, Composer (hidden until a song asks for it), Console
├── Dock layout: dock.rs (multi-panel docking, narrow mode below MIN_CENTER_WIDTH)
├── Menu / StatusBar (top + bottom chrome); Toolbar + PlayControls in the scene view's toolbar strip
├── Tools: Select, Move, Rotate, Scale (Q/W/E/R shortcuts)
├── Gizmos: Translate, Rotate, Scale handles
├── Picking: EntityPicker, SelectionRect, screen_to_world()
├── Inspector: Generic serde-based + per-component editors with writeback
│   (one renderer: `InspectorFrame.read_only` draws every row's value in
│   place of its control while a play session runs; sections collapse and
│   an Advanced disclosure hides the rarely-touched fields)
├── Undo/Redo: CommandHistory + EditorCommand trait, StoredComponent for restore
├── Theme: EditorTheme with 30+ color tokens (mockup-derived)
└── Play state: EditorPlayState (Editing/Playing/Paused), WorldSnapshot
```

## File Map
### State + chrome
- `context/` — EditorContext struct (selection, tools, state, view toggles, game frame dimensions, theme, fonts, inspector_scroll).
- `theme/` — EditorTheme: WCAG surface ladder `surface_0..surface_4` with luminance guard tests (≥1.35:1 adjacent / ≥3:1 border, overlay tones subdued against selection accent, game frame distinctness, grid axis hierarchy), muted overlay tokens (`game_frame`, subdued grid and colliders), the piano roll's four (`note_fill`, `note_selected_border`, `key_black`, `playhead`, each guarded at 3:1 over the roll), style converters, and `ui_theme()`.
- `command_api/` — CLI/API dispatch (query list/describe/selection/scene/commands and write set/add/remove/rename/delete/select/undo/redo/batch) through CommandHistory; `docs/EDITOR_COMMAND_API.md`. `command_api/song.rs` is the `song …` arm: its parser, the `song` and `song notes` answers, the pure song writes through `SongDocument::apply`, and `song_write_refusal` (Playing and Paused, an open scene batch) the host's song verbs share.
- `song/` — the composer's song, a second document: `SongDocument` (the sidecar and the MIDI data, its path, the file it was imported from, `EditingPolicy` from the import's report; dirty = its watermark moved OR it was never saved), `SongEdit` and its merge rule (`edit.rs`; `edit/apply.rs` checks and applies each variant, a sidecar edit through `SongFile::check_schema` on the file it would produce, `Batch` on copies so a refused member lands nothing), the exact undo/redo `Change`s (`change.rs`), `SongHistory` (`history.rs`, the scene watermark's contract and nothing of its session floor; `revision`, which a host compares to know the document moved under a prepared song), `SongSelection`, and the save journal and open's recovery (`io.rs` — `saveable_song_path`, the path rule Save and the web import share, `sample_directory`, `rendered_files`; `io/open.rs`; `io/samples.rs` — the song's sample folder, its owner-naming `LICENSE`, and the song's own stanza in its home folder's `LICENSE`). `SetProvenance` is the song's own author and license, which Save requires.
- `composer/` — the song's panel, data and drawing only (the host owns the song, the drag and what each action does): `state.rs` (`ComposerState` — `ComposerFocus` and Tab's cycle, the one active track among the tracks with a channel, the four scroll states, the zoom, the keyboard cursors, the control requests a key leaves for the next render, the Songs list's scan of `music/` and `ai/music/`, `step_sidebar_section` over the section starts the sidebar records each frame, and `follow`, which resets it all when the host's song generation moves), `layout.rs` (`compute` — Full, Compact below 720 px with one sidebar section behind tabs, Tabs below 480 px with one area at a time — and `PianoRollGeometry`: `tick_to_x`/`x_to_tick`/`pitch_to_y`/`y_to_pitch`, `note_rect`, `beat_lines` over the tempo map, thinned at the grid's LOD), `keys.rs` (`COMPOSER_KEYS`, the key table as data, and `resolve`), `piano_roll.rs` (wheel scroll and zoom, the notes drawn from a `MovePreview`, a press that selects and begins a drag, a Ctrl+press that toggles), `note_list.rs` (a row of text per note, units named), `sidebar.rs` with `sidebar/walk.rs` (every control counted so the arrows' cursor and the render agree, every note wrapped to the sidebar's width) and `sidebar/blocks.rs` (parts, instruments with provenance, bindings, each with a remove button), `transport.rs` (the track selector, the preview's Play and Stop — disabled during a play session — the zoom, and `status_text` — where the preview is, with BPM and meter, in text), the playhead line in `piano_roll.rs` at the tick the host writes into `ComposerState.playhead_tick`, and the song's Author and License fields in the sidebar's Song section; `mod.rs` holds `ComposerAction`, `clamp_move` and `moved_notes_edit` (the one `Batch` a drag or a nudge commits) and `drop_action`.
- `drag_drop.rs` — `DragDropState`/`DragPayload` (`Texture`, `Script`, `Sound`, `Song`, `SoundFont`) cross-panel drag state machine (Idle→Armed→Dragging→Dropped-1-frame).
- `dock/` — multi-panel docking: state, layout, collapse/visibility toggles, chevrons, and clamped resize grabbers (a hovered or dragged handle draws its grabber line and asks `ui.request_cursor` for `ColResize`/`RowResize`, the dock having no window to ask itself). Below `MIN_CENTER_WIDTH` of centre it enters **narrow mode**: the side panels leave the edge allocation and one at a time shows as an overlay over the viewport on the floating band, opened from its header tab at the edge and closed from its own chevron (`narrow_overlay`, `open_narrow_overlay`, `close_narrow_overlay`); a collapsed panel opens as a full overlay (`expanded_content_bounds`) without touching its persisted collapse flag; the overlay runs to the dock's bottom, over a bottom panel.
- `toolbar_strip.rs` — the scene view's toolbar strip: `split` (panel content → strip + viewport), the `begin`/`end` chrome scope on `UiLayer::PanelChrome`, and `layout` placing the tools left, the play controls at the centre where it leaves them whole, and the view group (`VIEW_GROUP_WIDTH`, 140 px) at the right edge; below `TOOLBAR_STRIP_MIN_WIDTH` (391 px) the view group sheds whole and the tools shed one by one into the overflow menu (`overflow_menu.rs`).
- `menu/` — top menu bar; action items carry checked flag and map labels to `EditorAction`.
- `editor_input.rs` — shortcut chord model (exact chord beats any-mods) and `allowed_while_playing()` action deny list.
- `archetype.rs` — `Archetype`: the nine entity factories shared by the Entity menu and command API `create`.

### Inspector / components
- `editable_inspector.rs` — the width-aware `EditableInspector` walk: collapsible headers (the header row IS the toggle, in both play states), the `advanced()` disclosure, and the `read_only`/`collapsed` gates every field method runs first. `InspectorToggles` is what a block asks the view state for.
- `field_widgets.rs` — the standalone field widgets the walk places (label column, f32 soft-range, angle degree field with wrap, boolean, cycle row, component header).
- `read_only_rows.rs` — the read-only form of a row: label plus value in the muted colour, no widget id, the height its editable form would have taken.
- `inspector_state.rs` — `InspectorState`: collapsed sections, open Advanced disclosures, and the open colour editor (`ColorEditorTarget` — entity, registry type NAME, field index, never a walk-time index — plus the swatch anchor and value its row reports each frame, the seen mark, and the edit waiting for that row). One field on `EditorContext`; only the collapsed set persists.
- `color_editor_popup.rs` — the colour editor's widget (large swatch, four channels, a hex field) on the Modal band. It is drawn by a pass of its own BEFORE the panels (`editor_integration`'s `panel_renderer/color_editor.rs`), because a blocking rect is consulted at each widget's own `interact` and one pushed mid-walk cannot make the rows above it inert; the colour row still writes the edit, so one scrub is one undo entry.
- `color_hex.rs` — `#rrggbb`/`#rrggbbaa` text for a `Vec4`, shared by the hex field and the read-only colour row.
- `row_layout.rs` — row-layout math (`field_row`, `remove_button_x`, `pair_slots`, `ellipsize`; all horizontal placement goes through here, never hardcode offsets).
- `field_style.rs` — `FieldId` (widget-ID mapping), `EditableFieldStyle`, and typed `EditResult<T>` returns (keeps the editor crate free of an engine_core dependency); `WidgetSlot` inside component ID stride.
- `component_editors.rs` — per-component editors returning `Option<ComponentEdit<T>>`; the collider shape row cycles all six shapes (Box, Circle, CapsuleY, CapsuleX, Capsule between two points, Compound of shapes), carrying the old shape's extent where the mapping exists, and a compound shows its part count read-only — its parts are edited in the scene file. Commit-before-cycle ordering.
- `physical_floors.rs` — hard floors applied by inspector editors and command API `sanitize` (scale, collider extents, capsule half-height, a sound's volume and spatial reach).
- `sound_field.rs` — the inspector's sound slot, the texture field's twin: shows a `SoundEmitter`'s path and takes a `DragPayload::Sound` dropped on it.
- `component_editors/sound_emitter.rs` — `edit_sound_emitter`: a Path/Binding kind cycle (to Binding clears `spatial` and `looping`, which the scene loader refuses on a binding), the sound slot or the binding's name, and only the rows the kind allows — a binding hides `volume`, `looping` and `spatial`. Its component sits alone under the popup's Audio group.
- `behavior_editor.rs` — `edit_behavior()`: variant cycle selector and per-variant editors; `CameraFollow.dead_zone` stays read-only.
- `script_editor.rs` — `edit_scripts()`: `Scripts` component inspector editor, script catalog picker, parameter table, and Open Source button.

### Scene + selection
- `selection.rs` — Selection set (IndexSet preserving insertion order, deterministic primary fallback).
- `hierarchy/` — hierarchy panel tree view, F2 inline rename, `RowGeometry`, `normalized_rename` guard, `Scripts` pseudo-rows, and script drop targets.
- `viewport/` — scene viewport with camera pan/zoom; `to_window_render_camera`/`world_to_screen` equivalence locked by overlay tests.
- `picking/` — `EntityPicker` and `PickableEntity` (AABB from absolute size, flip scales stay clickable).
- `gizmo/` — transform gizmos (annulus rotate ring with dead-center fallthrough, cumulative delta, ratio-based scale, and cancel latch).
- `grid.rs` — authoring grid segments and viewport clipped overlay lines (no internal visibility flag; caller gates rendering via `ViewToggles`).
- `clipboard.rs` — `ClipboardEntity`, `capture_entity_tree`/`spawn_entity_tree`, and `SpawnTreeCommand`.
- `collider_overlay.rs` — collider outline overlay mirroring rapier placement (offset is body-local, Transform2D.scale ignored).

### Persistence + commands
- `confirm_dialog.rs` — the Modal-band `ConfirmDialog`: the scene's unsaved-changes prompt, the Stop prompt, the song's (`unsaved_song`), Play's over a dirty song (`play_with_unsaved_song`: Save / Play saved / Cancel, or Save / Cancel for a song never saved — the alternative button is `alt_label: Option`) and Save As's onto another song (`overwrite_song`: Overwrite / Cancel).
- `commands/` — `EditorCommand` trait, `CommandHistory` dirty tracking watermark, `SetComponentCommand` merge-by-hint, `break_merge()` gesture boundary, and the play-session floor (`begin_session`/`drop_session_entries`/`rebase_session_entries`) with the leaf-level rebase in `rebase.rs`.
- `entity_names.rs` — `entity_display_name`: the one name the hierarchy row, the inspector heading and the command API's `display` field all read.
- `editor_preferences.rs` — `EditorPreferences` JSON serialization (`from_json`/`to_json`), panel layout capture/apply, camera state, flattened `view: ViewToggles` (carrying `grid_visible`, `snap_to_grid`, `colliders_visible`, `game_frame_visible`), `collapsed_components` (the inspector's collapsed sections), `ide_command` (IO handled by integration layer via save_store).
- `asset_browser.rs` — `AssetEntry`, `AssetKind` (`Image`, `Scene`, `Script`, `Sound`, `Song`, `SoundFont`), `scan_assets` walking `common::vfs::list_files` for images, scenes, scripts (`.rhai`, `.rs`), the recorded sounds the audio manager decodes (`.wav`, `.mp3`, `.ogg`, `.flac`), songs (`.mid`) and SoundFonts (`.sf2`) — the last two are dropped on the composer, never on a sound slot — and `AssetBrowserState.selected` — the clicked tile, carried across a rescan by relative path and dropped when its file is gone.
- `stored_component/` — typed registry overlay (`editor_component_registry!`), `category.rs`, and `dynamic.rs` falling through to ECS dynamic registry.
- `world_snapshot.rs` — `WorldSnapshot` save/restore with uncaptured component type detection and drop reporting.

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| Continuous edits on different entities must never merge even when sharing a field hint | `src/commands/tests.rs test_edits_on_different_entities_never_merge_even_with_the_same_field_hint` |
| Continuous edits merge by field hint until `break_merge()` seals the gesture into distinct history entries | `src/commands/dirty_tests.rs test_break_merge_seals_the_gesture_so_two_scrubs_are_two_entries` |
| Adjacent surfaces in the editor theme ladder must maintain WCAG contrast (≥1.35:1 adjacent / ≥3:1 border) | `src/theme/tests.rs test_adjacent_surfaces_are_distinguishable` |
| Open menu dropdown renders in the overlay band and must block clicks from reaching underlying widgets | `src/menu/tests.rs test_open_dropdown_renders_in_overlay_band_and_blocks_input` |
| Entity picking must compute AABB from absolute visual size so flip-scaled sprites remain clickable | `src/picking/tests.rs test_flip_scaled_sprite_is_picked_at_its_visual_bounds` |
| A paused edit's before-image is the simulated value; a replay onto the restored world must `rebase_onto` first or Undo resurrects simulation state | `src/commands/session_tests.rs test_rebase_replays_only_the_changed_fields_over_the_authored_component` |
| Confirm dialog scrim clicks must block input to underlying background widgets | `src/confirm_dialog.rs test_scrim_click_is_not_a_choice_and_blocks_input` |
| Play over a never-saved song must not offer "Play saved": there is no saved file to play | `src/confirm_dialog.rs test_play_saved_is_offered_only_when_a_saved_song_exists` |
| A song never saved is dirty though its watermark reads clean, or a new song would reach Play unprompted | `src/song/history_tests.rs a_new_song_is_dirty_until_its_first_save_though_its_history_is_clean` |
| The GUI and the API must share the song's one mutation path: every `SongEdit` variant has a row (an exhaustive match fails to compile without one), and a song the composer cannot edit is refused by both with the report's reason | `src/song/parity_tests.rs every_variant_lands_the_same_through_the_gui_and_the_api_and_undoes_the_same` `src/song/parity_tests.rs a_song_the_composer_cannot_edit_is_refused_by_both_paths_with_the_reports_reason` |
| A redone added note must keep its id, or every later entry naming it fails to redo: the history replays exact `Change`s (notes restored under their ids), never the edit again | `src/song/edit_tests.rs a_redone_note_keeps_its_id_so_the_edits_after_it_redo_too` |
| An undone tempo or meter edit restores the timing exactly, but `SongData` keeps writing the timing events in its own order on track 0 afterwards, so the undone song's bytes differ from the original's (reimport is identical) | `src/song/parity_tests.rs every_variant_lands_the_same_through_the_gui_and_the_api_and_undoes_the_same` |
| A song save interrupted at any write reopens as the old song or the new one: the `.new` pair matching itself is recovered over the finals, a matching leftover purged, a differing lone `.mid.new` opened as an import, mismatched finals with no pair an error | `src/song/open_tests.rs a_save_interrupted_at_any_step_reopens_as_the_old_song_or_the_new_one` `src/song/open_tests.rs mismatched_finals_with_no_pair_are_an_error_naming_both_files` |
| A save's prune waits for its `.new` pair, so no interruption leaves a song naming a sample the prune removed | `src/song/open_tests.rs every_sample_the_reopened_song_names_is_on_disk_wherever_its_save_was_interrupted` |
| A song saved under a subfolder or `ai/music` gets a flat folder of its own in its home (`<home>/samples/<path with / as __>`), owned by the song its `LICENSE`'s `Source:` names: another song's folder is refused naming both, a folder from before owners were named is adopted by its own stem only, and every sample is on disk when the song reopens | `src/song/folder_tests.rs each_song_gets_a_flat_sample_folder_in_its_home_and_reopens_with_every_sample` `src/song/folder_tests.rs a_save_into_a_folder_another_song_owns_is_refused_naming_both_in_either_order` `src/song/folder_tests.rs a_folder_from_before_owners_were_named_is_adopted_by_its_own_song_only` |
| `__` flattens a song's path into its sample folder's name, so the path rule keeps it unambiguous — no component starting or ending with `_`, no `__`, no whitespace (the gate splits `Files:` on it), nothing under a home's `samples/` (another song's prune lists it) — and a song directly in its home keeps the looser rule it was saved under | `src/song/folder_tests.rs the_path_rule_refuses_ambiguous_spaced_and_sample_folder_paths_and_grandfathers_flat_ones` |
| A song's own provenance is required at Save, and Save writes it as the song's stanza in its home folder's `LICENSE`, found by the `.mid` path alone, never touching another song's files | `src/song/folder_tests.rs a_song_without_its_own_provenance_refuses_the_save_and_writes_nothing` `src/song/folder_tests.rs two_songs_saved_into_one_home_keep_each_others_stanzas_and_pass_the_gate_grammar` `src/song/folder_tests.rs a_stanza_a_person_wrote_for_the_mid_alone_is_rewritten_not_duplicated` |
| The playhead must cross a note legibly: its token holds 3:1 against the well, the black-key rows and a note's fill | `src/theme/tests.rs test_the_piano_rolls_marks_read_over_the_roll` |
| The preview seeks to the start of a beat, so the API's beat is whole: a fractional, negative or non-finite one is `invalid`, never rounded | `src/command_api/song_tests.rs a_fractional_negative_or_non_finite_beat_is_invalid` |
| The song's provenance is an edit like any other — `song edit` sets it and undo takes it back | `src/command_api/song_tests.rs song_edit_sets_the_songs_provenance_and_undo_takes_it_back` |
| A sidebar note wraps at the sidebar's width instead of running off its edge, which dropped the very words that said what to do | `src/composer/sidebar_tests.rs every_note_wraps_inside_a_narrow_sidebar_so_none_is_cut_at_its_edge` `src/composer/sidebar/walk.rs a_note_wraps_at_spaces_to_the_width_and_a_long_word_keeps_its_own_line` |
| The arrows' reveal measures from the scroll origin, the top inset included: measured from the first row, a revealed bottom row landed an inset short of fully inside, and a row not wholly inside is neither ringed nor clickable, so the cursor vanished below the edge | `src/composer/sidebar_tests.rs walking_down_to_the_last_control_of_a_short_sidebar_rings_it_wholly_inside` |
| Left and Right step the sidebar's sections: below the full width they change the one tab drawn; at the full width, where every section is drawn, they move the cursor to the neighbouring section's first control (a section with none passed over), wrapping — changing the unseen tab there only sent the cursor back to the top | `src/composer/sidebar_tests.rs left_and_right_at_the_full_width_move_the_cursor_to_each_sections_first_control_and_wrap` `src/composer/sidebar_tests.rs left_and_right_behind_tabs_change_the_tab_and_put_the_cursor_at_its_top` |
| A provenance field committed with its own value (Enter on it, a press away) is no edit: it would dirty the song and stop a playing preview for nothing | `src/composer/sidebar_tests.rs a_provenance_field_committed_with_its_own_value_is_no_edit` |
| During a play session the game holds the music: the transport's Play and Stop draw disabled and a click asks nothing | `src/composer/transport_tests.rs play_asks_for_the_preview_while_editing_and_nothing_during_a_play_session` |
| A menu's shortcut is drawn in its row's box at its measured width, and the dropdown widens until every centred label clears its shortcut: a guessed width drawn from the baseline sat raised over "Save As..." | `src/menu/tests.rs test_every_shortcut_sits_right_of_its_label_inside_the_dropdown` |
| A long status message keeps its words: the stats step aside rather than draw over it, and only a message wider than the bar up to the version is shortened, its whole text kept as the tooltip | `src/status_bar.rs test_a_long_message_moves_the_stats_aside_and_a_short_one_keeps_them` |
| A disabled Trigger still answers Enter, so the host can say why it is not live; a click reaches it only while the preview is heard | `src/composer/sidebar_tests.rs a_bindings_trigger_is_live_only_while_the_preview_sounds_and_enter_still_reaches_the_host` |
| A sample without provenance refuses the save before anything is written; a saved sample is named in its folder's `LICENSE` by a whitespace-free file name the provenance gate's tokenizer can read | `src/song/io_tests.rs a_sample_without_provenance_refuses_the_save_naming_the_instrument_and_writes_nothing` `src/song/io_tests.rs a_saved_sample_lands_in_the_songs_folder_named_in_its_license_and_passes_the_gate_grammar` |
| Every song write is refused in Playing and Paused (stricter than scene writes) and inside an open scene batch | `src/command_api/song_tests.rs every_song_write_is_refused_while_playing_paused_inside_a_scene_batch_and_without_a_song` |
| A held Shift+arrow nudge is one undo entry: its batches merge member by member while the key is held, and batches of another shape are another entry | `src/song/edit_tests.rs a_held_nudge_merges_its_batches_member_by_member_into_one_entry` |
| While the composer holds the keyboard its table decides first: a key it lacks goes on to the editor only as a Ctrl chord or F5, and the single-key editor toggles (G, C, S, Q/W/E/R) are swallowed — so no composer key reaches the scene or the game | `src/composer/keys_tests.rs a_key_the_table_lacks_goes_to_the_editor_with_ctrl_or_as_f5_and_is_swallowed_otherwise` |
| A dragged chord is clamped as a whole — the tightest note bounds the set — so it keeps its intervals and no frame of a drag is ever refused | `src/composer/piano_roll_tests.rs a_dragged_chord_is_clamped_whole_so_it_keeps_its_intervals_inside_the_song` |
| Selection is never colour alone: a selected note carries a thicker border and a glyph, a selected row the accent fill and a leading "▸" | `src/composer/piano_roll_tests.rs a_selected_note_carries_a_thicker_border_and_a_glyph_besides_its_colour` |
| Two bottom panels: the composer is added after Assets so the first layout pass stacks it above, it starts hidden, and a prefs file written before it existed leaves it hidden | `src/context/tests.rs test_the_composer_starts_hidden_stacks_above_assets_and_an_old_prefs_file_leaves_it_hidden` |
| The path field's commit is a save only on Enter: a click away or Tab only leaves the field, which shows the song's path again | `src/composer/sidebar_tests.rs the_path_fields_enter_saves_there_and_leaving_it_any_other_way_saves_nothing` |
| Save Song As… scrolls the sidebar to its path field and puts the cursor on it before focusing it: the ui keeps a focused field until it is drawn, and the walk draws only the rows inside the sidebar, so a field focused off-screen would own the keyboard with nothing to receive it | `src/composer/sidebar_tests.rs save_song_as_scrolls_the_sidebar_to_its_path_field_before_focusing_it` |
| Save Song As… also switches the sidebar to the Song section, the field's own, since below the full width only one section is drawn | `src/composer/sidebar_tests.rs save_song_as_switches_the_sidebar_to_the_song_section_before_focusing_the_path_field` |
| A number field lands every frame it changes as a `FieldEdit` — the first frame of a gesture marked so it opens an entry of its own, later frames continuing it — and the walk closes the gesture on the field's commit, on its losing focus with no scrub running, and on a frame that does not draw it: a scrub's release reports the input value with only the commit flag (a host waiting for it would see the document's value back), and a ui-wide commit flag shared with the inspector would seal the wrong document | `src/composer/sidebar_tests.rs a_number_fields_gesture_opens_discrete_continues_while_live_and_closes_on_release_focus_loss_or_an_undrawn_row` |
| World snapshot restore must detect and report unregistered component types that cannot be captured | `src/world_snapshot/tests.rs test_loss_messages_name_every_dropped_type_or_nothing` |
| Rotate gizmo dead-center clicks must fall through to entity picking | `src/gizmo/tests.rs test_rotate_ring_is_an_annulus_so_a_dead_center_press_falls_through_to_picking` |
| Hard floors for inspector editors and command API must clamp negative or zero dimensions | `src/command_api/write_tests.rs test_set_sanitizes_collider_extents_to_the_gui_floor` `src/physical_floors.rs test_clamp_sound_emitter_holds_a_typed_minus_one_at_the_floors` |
| The inspector cannot author what the scene loader refuses: cycling a `SoundEmitter` to Binding clears `spatial` and `looping` | `src/component_editors/tests.rs test_cycling_a_spatial_loop_to_a_binding_clears_what_a_binding_cannot_have` |
| Asset scanning must use `common::vfs::list_files` instead of `std::fs` so recursive asset enumeration works on wasm; it never follows symlinks (assets are copies by convention), so a linked tree lists as empty | `src/asset_browser.rs test_nested_images_and_scenes_listed_with_slash_joined_relative_paths_while_txt_is_ignored`; `common/src/vfs/tests.rs test_vfs_list_files_never_follows_symlinks` |
| Wheel zoom is proportional to the delta in notches and clamped to one notch per frame: a trackpad streams fractions of a notch every frame, and a fixed factor per frame made a gentle scroll compound like sixty notches a second; a hard flick that delivers a notch or more a frame still zooms a notch a frame, the mouse wheel's own ceiling | `src/viewport_input.rs test_a_fraction_of_a_wheel_notch_zooms_by_the_same_fraction_of_the_factor`, `test_a_frame_of_wheel_zooms_at_most_one_notch` |


## Key Patterns
- Inspector uses `serde_json::to_value()` to extract component fields generically
- Component editors return `Option<ComponentEdit<T>>` (full new value + `field_hint` for undo merging) that the integration crate applies via `apply_component_edit()`
- `EditorPlayState::Editing` → editable, `Playing` → the same rows drawn read-only, `Paused` → editable; `CommandHistory::begin_session` marks the Play boundary the editor's Stop dialog acts on
- The inspector heading is two lines: `entity_display_name` on top, then `Selection::inspector_heading`'s detail line ("Entity 17", or "3 selected · Entity 17 primary")
- Selection: `editor.selection.primary()` returns the main selected EntityId
- Gizmo drag tracking: editor_integration's `GizmoDragState` captures start transform+collider for every selection root; frames apply `start + cumulative delta` (idempotent — what makes snapping residual-proof), ONE Macro/TransformGizmo command on release, Escape restores starts and pushes nothing
- The toolbar and the play controls are NOT floating chrome: they render inside the strip's scope, positioned by `toolbar_strip::layout`. `scene_view_bounds()` is the viewport BELOW the strip — every overlay, the GPU scissor and every pick map through it, so none of them reach into the band; `toolbar_strip_bounds()` is the band itself.
- Theme is on `EditorContext.theme` (public field); call `theme.gizmo_palette()`, `inspector_style()`, `editable_field_style()`, `grid_colors()`, `collider_overlay_colors()`, `game_frame` instead of hardcoding colors. Menu/Toolbar/Hierarchy `render()` take `&EditorTheme`

## Testing
- `cargo test -p editor` — 0 failed, 0 ignored

## Godot Oracle — When Stuck
Use `WebFetch` to read from `https://github.com/godotengine/godot/blob/master/`

| Our Concept | Godot Equivalent | File |
|-------------|-----------------|------|
| EditorContext | EditorNode | `editor/editor_node.cpp` |
| Inspector | EditorInspector | `editor/editor_inspector.cpp` |
| Component editors | EditorProperties | `editor/editor_properties.cpp` — `_property_changed` |
| Picking / selection | Canvas item editor | `editor/plugins/canvas_item_editor_plugin.cpp` — `_gui_input_viewport` |
| Hierarchy panel | SceneTreeDock | `editor/scene_tree_dock.cpp` — `_tool_selected` |
| Gizmos | CanvasItemEditor gizmos | `editor/plugins/canvas_item_editor_plugin.cpp` — search `gizmo` |
| Play/Pause/Stop | EditorRun | `editor/editor_run.cpp`, `editor/editor_node.cpp` — `_run_native` |
| Undo/Redo | EditorUndoRedoManager | `editor/editor_undo_redo_manager.cpp` |
| Dock layout | EditorDockManager | `editor/editor_dock_manager.cpp` |

**Remember:** Godot's editor is plugin-based with docks. Adapt *interaction patterns* to our immediate-mode UI.
