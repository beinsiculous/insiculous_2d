I re-framed the audit as a keep-list. Everything below is read-only analysis; I wrote no files.

---

# ENGINE_CORE KEEP-LIST

**Total: 72 keeps of 394.** The enumerated contract list you gave does not compress below ~70 without dropping a named contract — this crate owns four independent on-disk formats plus the sheet↔scene chain. A hard-60 cut line is given at the end.

Each keep absorbs its family (see MERGE-INTO). Format: `KIND | contract | file:line test_name`.

---

## HALF A — CONTRACT TESTS (47 persistence, 25 everything else)

### Scene RON: World → RON save pipeline (6)

```
CONTRACT | world→RON→world, field for field                | src/scene_serializer_roundtrip_tests.rs:46  test_roundtrip_serialize_deserialize
CONTRACT | Sprite extraction, all 9 fields                 | src/scene_serializer_tests.rs:80            test_entity_with_sprite
CONTRACT | hierarchy: roots at top level, children nested  | src/scene_serializer_tests.rs:334           test_hierarchy_preserved
CONTRACT | derived state never reaches the wire            | src/scene_serializer_tests.rs:390           test_global_transform_not_serialized
CONTRACT | GridBackdrop every field + bare = preset        | src/scene_serializer_roundtrip_tests.rs:206 test_grid_backdrop_round_trips_every_field_and_parses_bare
CONTRACT | prefab base / overrides / inline layering       | src/scene_loader.rs:338                     test_merge_components
```
Reason for :390 — the only test proving the save pipeline is a whitelist, not "everything in the World".

### Scene RON: dynamic registry tier (3)

```
CONTRACT | Value→RON→Value preserves ints, floats, strings, nesting | src/scene_dynamic_tests.rs:56  test_dynamic_payload_survives_ron_round_trip
CONTRACT | transient components are never persisted                 | src/scene_dynamic_tests.rs:115 test_transient_components_are_not_saved
CONTRACT | an unregistered component refuses the whole load         | src/scene_dynamic_tests.rs:162 test_unknown_dynamic_component_fails_the_load_loudly
```

### Scene RON: Scripts (2)

```
CONTRACT | every ScriptValue type round-trips; Entity params remap by NAME | src/scene_scripts_tests.rs:55  test_scripts_scene_round_trip_preserves_every_param_type
CONTRACT | save auto-names an unnamed referenced target             | src/scene_scripts_tests.rs:151 test_save_auto_names_referenced_unnamed_targets
```

### Scene RON: loader and legacy shapes (4)

```
CONTRACT | pre-editor scene files load with editor: None    | src/scene_data_tests.rs:46           test_scene_data_without_editor_settings_backward_compat
CONTRACT | Sprite serde defaults for pre-field scenes       | tests/scene_loader_parse.rs:104      test_parse_sprite_tex_region_and_visible_default
CONTRACT | legacy CameraFollow without look-ahead parses    | tests/scene_loader_parse.rs:281      test_legacy_camera_follow_scene_without_look_ahead_still_parses
CONTRACT | Tilemap parses, instantiates, resolves a tileset | tests/scene_loader_parse.rs:183      test_tilemap_parses_and_instantiates_with_resolved_tileset
```

### Prefabs (2)

```
CONTRACT | runtime spawn applies overrides over prefab components | tests/prefab_spawning.rs:92  test_spawn_prefab_applies_overrides
CONTRACT | a spawn that fails mid-build leaves no debris          | tests/prefab_spawning.rs:125 test_spawn_prefab_failure_removes_half_built_entity
```

### `.sheet.ron` schema — every validation path (5)

```
CONTRACT | golden sidecar parses and re-serializes identically      | src/sheet_file.rs:176 test_golden_sheet_file_round_trips
CONTRACT | authored errors fail loud, naming file and clip          | src/sheet_file.rs:224 test_unknown_version_is_rejected_naming_the_file
CONTRACT | grid = PNG ÷ cell; partial trailing cell excluded        | src/sheet_file.rs:283 test_into_parts_excludes_a_partial_trailing_cell
CONTRACT | a frame index past the last cell is rejected by name     | src/sheet_file.rs:295 test_frame_index_past_the_grid_is_rejected_naming_the_clip
CONTRACT | sidecar path REPLACES the extension, never appends       | src/sheet_file.rs:310 test_sidecar_path_replaces_the_extension
```
:224 becomes the table absorbing zero-cell (:234), empty frames (:242) and unusable fps (:252) — one test, four rows, each asserting the file name and, where applicable, the clip name.

### Sidecar pipeline (3)

```
GUARD    | validation runs BEFORE any texture handle is allocated | src/assets/sprite_sheet.rs:310 test_prepare_sheet_fails_before_any_texture_is_loaded
GUARD    | sidecar is SSOT on reload: a re-cut sheet propagates   | src/assets/sprite_sheet.rs:364 test_clearing_the_cache_picks_up_an_edited_sidecar
CONTRACT | a half-saved sidecar warns and falls back, never fails | src/assets/sprite_sheet.rs:390 test_cache_falls_back_quietly_on_a_malformed_sidecar
```

### Texture-ref sentinels (2)

```
CONTRACT | #solid:RRGGBB[AA] is written and parsed as inverses     | src/texture_ref.rs:196 test_solid_color_path_round_trips_through_parse
CONTRACT | #rgba/#solid degrade to white; #white/#solid: rebuild   | src/texture_ref.rs:213 test_generated_texture_sentinels_are_flagged
```

### `ClipData` — the one DTO shared by scene RON and sidecar (4)

```
CONTRACT | clip wire format golden, in AND out, no derived UVs   | tests/sprite_animation_scene.rs:347 test_clip_wire_format_is_stable
CONTRACT | SpriteAnimation round-trips sheet, grid, clips, autoplay | tests/sprite_animation_scene.rs:178 test_sprite_animation_round_trips_through_scene_ron
CONTRACT | sidecar grid+clips win over baked scene values         | tests/sprite_animation_scene.rs:259 test_sidecar_grid_and_clips_win_over_baked_scene_values
CONTRACT | animation → Sprite.tex_region → renderer instance      | tests/sprite_animation_scene.rs:471 test_animated_sprite_region_reaches_the_renderer
```
:347 is the single most valuable test in the crate — it is the only one asserting the wire *field names* (`frames:`, `looping:`, `cols:`) and the absence of `cell_uv`.

### Save slots — achievements / scores / save_store / input settings (10)

```
CONTRACT | achievements JSON survives a process boundary       | src/achievements/tests.rs:86   persistence_round_trip
GUARD    | two writers merge instead of clobbering (two tabs)  | src/achievements/tests.rs:108  concurrent_managers_merge_unlocks_instead_of_clobbering
GUARD    | save is atomic — no .tmp left behind                | src/achievements/tests.rs:152  save_leaves_no_temp_file_behind
CONTRACT | full list rejects non-qualifying and evicts lowest  | src/scores.rs:205              test_full_list_rejects_non_qualifying_and_evicts_lowest
CONTRACT | comparator: score desc, ties oldest-first           | src/scores.rs:223              test_equal_scores_rank_the_earlier_entry_first
GUARD    | scores merge across writers, reset still clears     | src/scores.rs:289              test_concurrent_stores_merge_instead_of_clobbering
CONTRACT | slot write→read round-trip, atomic, mkdir -p        | src/save_store.rs:175          test_write_then_read_round_trips_and_leaves_no_temp_file
CONTRACT | MemoryStore matches slot semantics (the wasm shape) | src/save_store.rs:204          test_memory_store_matches_slot_semantics
CONTRACT | input bindings + pad routing survive save/load      | src/input_settings_io.rs:174   round_trip_preserves_pads_and_bindings
CONTRACT | missing file → defaults, written out hand-editable  | src/input_settings_io.rs:204   missing_file_returns_defaults_and_creates_hand_editable_file
```

### Config, asset config, locale files (7)

```
CONTRACT | pre-localization/pre-filter config JSON still loads  | src/game_config.rs:214   test_game_config_locale_serde_defaults
CONTRACT | texture_filter wire: variant name out, alias in, typo refused | src/game_config.rs:254 test_game_config_texture_filter_accepts_lowercase_alias
CONTRACT | RGBA dimension/length validation is a typed error    | src/assets.rs:505        test_rgba_validation_rejects_length_mismatch
CONTRACT | lookup chain: locale → en → the key itself           | src/localization.rs:544  tr_falls_back_to_english_then_key
CONTRACT | corrupt / wrong-version locale files are skipped     | src/localization.rs:571  corrupt_and_wrong_version_sources_are_skipped
CONTRACT | load_dir keys locales by file stem, ignores non-.ron | src/localization.rs:591  load_dir_reads_ron_files_by_stem
CONTRACT | AssetConfig maps GameConfig's filter and base path   | src/assets.rs:453        test_asset_config_from_game_config_carries_texture_filter
```

**Persistence subtotal: 47.**

### Lifecycle and state machines (3)

```
CONTRACT | the full invalid-transition matrix           | tests/lifecycle.rs:67        test_lifecycle_state_transitions
GUARD    | a poisoned lock never blocks shutdown        | src/lifecycle.rs:306         test_lifecycle_survives_lock_poisoning
CONTRACT | a started Scene actually runs its schedule   | tests/scene_lifecycle.rs:82  test_scene_with_schedule
```

### Pause and time_scale (2)

```
CONTRACT | each row executes its action, every confirm unpauses | src/pause.rs:288 confirm_executes_highlighted_item
CONTRACT | time_scale is 0.0 only while paused                  | src/pause.rs:402 time_scale_is_zero_only_while_paused
```

### Menu input and panel geometry (2)

```
CONTRACT | a held stick scrolls once, not every frame (edge not level) | src/menu_input.rs:166 test_held_stick_scrolls_once_not_every_frame
CONTRACT | row_at is the geometry SSOT hit-testing agrees with          | src/menu_panel.rs:343 row_at_round_trips_every_row_center_and_rejects_the_bands
```

### Grid spring math and topology (4)

```
CONTRACT | every spring's rest length equals the spacing         | src/grid/topology.rs:153        hex_springs_all_have_rest_length_equal_to_spacing
CONTRACT | resting grid is translucent, motion brightens it      | src/grid/opacity_tests.rs:38    resting_grid_is_more_transparent_than_moving_grid
GUARD    | negative/NaN scene tunables clamp, springs never invert | src/grid/build.rs:100        test_negative_coefficients_clamp_instead_of_inverting_the_springs
CONTRACT | moving the entity translates the mesh, no rebuild     | src/grid/backdrop_system.rs:251 test_moving_the_entity_translates_the_mesh_without_a_rebuild
```

### Particles (2)

```
CONTRACT | fixed pool overwrites the oldest, never allocates | src/particles/manager.rs:244 overfull_pool_overwrites_oldest
CONTRACT | emitter rate and spawn position drive the pool    | src/particles/system.rs:95   active_emitter_spawns_at_configured_rate
```

### Camera follow (2)

```
CONTRACT | dead zone converges with the target on the box edge   | tests/camera_follow.rs:208 test_dead_zone_converges_with_target_on_the_box_edge
GUARD    | negative/NaN look-ahead degrades to plain follow, always finite | tests/camera_follow.rs:404 test_negative_and_nan_look_ahead_degrade_to_plain_follow
```

### Behavior FSM (1)

```
CONTRACT | chase enters/leaves with hysteresis between the two ranges | src/behavior_runner/mod.rs:427 test_chase_enters_and_leaves_chasing_phase_on_range
```

### Frame timing and render path (9)

```
CONTRACT | a long stall is clamped to MAX_DELTA_TIME            | src/game_loop_manager.rs:154   test_delta_time_is_clamped_after_a_stall
GUARD    | surface-error streak latches fatal without a lost callback | src/render_manager.rs:506 surface_error_streak_latches_fatal_without_device_lost_callback
CONTRACT | main camera syncs position only, viewport stays render-managed | src/render_manager.rs:555 test_sync_main_camera_copies_main_camera_entity_position
GUARD    | UI lands on the same screen pixels under any camera/zoom | src/ui_integration/tests.rs:25 test_ui_stays_at_screen_position_under_moved_zoomed_camera
CONTRACT | clipped commands carry the quantized clip on their batch | src/ui_integration/tests.rs:189 test_clipped_commands_land_in_a_clip_tagged_batch
CONTRACT | tilemap expands to one batch with correct per-tile UVs | src/tilemap_render.rs:61       test_tilemap_expands_into_one_batch_with_correct_instances
CONTRACT | glyph cache keys on (char, size, font)               | src/glyph_texture_cache.rs:184 same_glyph_same_size_different_fonts_needs_separate_textures
CONTRACT | data-driven UI text resolves @keys through Strings   | src/ui_element_system.rs:244   button_and_label_text_resolve_localization_keys
CONTRACT | hat axis emits press/release only on crossings       | src/gamepad_backend.rs:297     hat_transitions_press_and_release_only_on_crossings
```

### Gameplay mechanism the games depend on (1)

```
CONTRACT | dynamic sensor ↔ kinematic body catch, once per pickup | src/pickups.rs:357 test_falling_sensor_pickup_collected_by_kinematic_body
```

---

## HALF B — GUARD MAP

```
GUARD | ctx.chaos_mode is read-write, engine persists the writeback
      | MISSING. game.rs:492 and game/app_handler.rs:245 do `self.config.chaos_mode = ctx.chaos_mode`
        and neither file has a single #[test]. Same for the sibling `time_scale` writeback.

GUARD | post-tonemap UI pass: authored UI white is 255, not 188; game and UI batches separate end to end
      | MISSING in engine_core. ui_integration/tests.rs proves UI *geometry* and clip batching but never
        asserts a colour byte, and nothing here asserts the two batch streams stay separate through submit.
        Partial coverage may live in the renderer crate; from engine_core's side this is unguarded.

GUARD | loader attaches a Name component so names survive load → save
      | MISSING. scene_loader.rs:216-219 attaches it with a comment saying exactly why; no test loads a
        named entity and asserts world.get::<Name>() is Some, and no test does save → load → SAVE.
        scene_scripts_tests.rs:169 asserts a Name exists, but only on the auto-name path.

GUARD | main_camera_pose replaces non-finite or <=0 zoom with 1.0
      | MISSING. render_manager.rs:426-445 carries the guard and the comment ("a zoom: 0.0 in a scene file
        must never divide the projection (or the editor viewport) by zero"); no test passes a bad zoom.

GUARD | set_base_path drops the path-dedup cache
      | MISSING. assets.rs:421-424 does it with the rationale in the doc comment; untested.

GUARD | SidecarCache warn-vs-quiet: NotFound quiet, any other read error warns
      | MISSING. assets/sprite_sheet.rs:170-181 branches on ErrorKind::NotFound; only the absent and the
        malformed cases are tested, never the present-but-unreadable one.

GUARD | a failing sheet leaves no texture handle behind
      | src/assets/sprite_sheet.rs:310  test_prepare_sheet_fails_before_any_texture_is_loaded  (KEPT)

GUARD | sidecar is SSOT on reload; re-cutting propagates without re-saving scenes
      | src/assets/sprite_sheet.rs:364  test_clearing_the_cache_picks_up_an_edited_sidecar  (KEPT)
        + tests/sprite_animation_scene.rs:333 proves clear_sidecar_cache runs once per load (fold in).

GUARD | pause freezes particles AND sprite animations through the same time-scaled delta
      | PARTIAL. src/pause.rs:402 proves time_scale goes to 0.0; game/frame_tail.rs:23/:29/:35 multiplies
        the delta into particles, SpriteAnimationSystem and the grid backdrop — and frame_tail.rs has no
        tests. src/grid/backdrop_system.rs:273 is the only test proving a frozen delta actually freezes a
        consumer (and that impulses are dropped, not banked). The particle and animation halves are unguarded.

GUARD | SCENE-SERIALIZER TABLE DRIFT
      | MISSING — and it is the single highest-value test not written.
        extract_components (scene_serializer.rs:89 onward) is a hand-written match over concrete types.
        Today eight separate tests each assert one variant's field list; NONE of them fails when a new
        component type is added and forgotten. Write one test: build a World holding the registry default
        of every persistent type, run world_to_scene_data, and assert (a) each concrete ComponentData
        variant appears exactly once, (b) no Dynamic row names a type that also has a concrete variant,
        (c) the concrete-variant count equals the registry's persistent-type count minus the transient set.
        That single test replaces scene_serializer_tests.rs:128/:163/:204/:244/:280/:483/:507 and catches
        the failure they cannot.
```

---

## MERGE-INTO

**The fixture families — one absorber:** `src/scene_serializer_roundtrip_tests.rs:46 test_roundtrip_serialize_deserialize` is the round-trip that should absorb the others. It already owns the canonical `world → world_to_scene_data → serialize_to_ron → parse → instantiate → assert fields` path. Fold into it (or into the shared `test_support` module it should pull `test_texture_path` / `StubResolver` / `roundtrip_single_entity` into): the UI-component round-trips (`:123`, `:151`, `:179`), the Name-as-EntityData.name assertion (`scene_serializer_tests.rs:66`), the EntityTag RON round-trip (`scene_serializer_tests.rs:310`), and the dropped-file duplicates in `scene_data_tests.rs:60`. That kills 4 copies of `test_texture_path`, 6 of `StubResolver` and 4 of `roundtrip` in one move.

```
→ sheet_file.rs:224          absorbs :234 zero cell, :242 empty frames, :252 unusable fps  (one error table)
→ sheet_file.rs:176          absorbs :194 omitted-filter default, :206 omitted-looping default, :216 lowercase alias
→ sheet_file.rs:283          absorbs :268 grid-from-PNG-dimensions
→ sprite_sheet.rs:364        absorbs :353 cache-does-not-re-read
→ sprite_sheet.rs:390        absorbs :400 generated refs never probed, :325/:334 missing sidecar/PNG
→ texture_ref.rs:196         absorbs :155/:167/:173/:179 hex parse table, :184/:190 alpha-omission
→ texture_ref.rs:213         absorbs :205
→ achievements:86            absorbs :26/:35/:42 unlock semantics, :51 one-toast, :63 toast expiry, :183 missing file
→ achievements:152           absorbs :142 parent dir, :129 reset-clears, :166 unwritable-path error
→ scores:205                 absorbs :193 ordering/best, :245 modes independent, :266 no-write-on-reject
→ scores:289                 absorbs :254 round-trip, :279 corrupt→fresh, :304 reset-clears
→ save_store:175             absorbs :168 missing→None, :187 mkdir -p, :195 replace
→ input_settings_io:204      absorbs :220 corrupt→defaults, :232 wrong version, :243 mkdir -p
→ localization:544           absorbs :537 current-locale text, :583 missing dir
→ localization:591           absorbs :607 locale_keys, :615 available_locales, :624 current_font, :632 font_dirty, :640/:650 cycle
→ game_config:254            absorbs :239 serde round-trip, :247 variant name
→ assets.rs:453              absorbs :471/:478 base path, :445 defaults
→ assets.rs:505              absorbs :493 zero dims, :513 exact length
→ scene_dynamic:115          absorbs :138 name-sorted rows
→ scene_dynamic:56           absorbs :88 audio components (add them to the fixture world)
→ scene_scripts:55           absorbs :94 missing-name dropped, :125 forward reference
→ scene_scripts:151          absorbs :180 collision skip
→ prefab_spawning:92         absorbs :66 table retained, :74 stamps a new entity
→ prefab_spawning:125        absorbs :114 unknown prefab
→ scene_loader_parse:104     absorbs :60/:82 emissive, :131 explicit region, :8 basic parse, :160 EntityTag
→ sprite_animation:178       absorbs :128 serializer fields, :157/:241 paused, :202 static region, :221 snapshot overwrite, :83 old format
→ sprite_animation:259       absorbs :285 missing sidecar fallback, :309 stale autoplay, :333 cache cleared per load
→ scene_serializer_tests:80  absorbs :128/:163/:204/:244/:280/:483/:507 — but only once the DRIFT GUARD above exists
→ pause:288                  absorbs :234 toggle edges, :256 pad start, :274 back, :315 click/hover, :358 stray click, :372 wrap+reset
→ menu_input:166             absorbs :110/:116/:121/:127 navigate table, :147 any pad, :187 numpad, :196 idle pad
→ menu_panel:343             absorbs :303/:316 panel rect, :358 click, :372 resting cursor, :388/:399 outside, :423 style
→ topology:153               absorbs :169 degree three, :187 centred, :200 spring count, :209/:223 square lattice, :240 pinning
→ opacity_tests:38           absorbs :9 square variant, :63 return to rest, :83 uniform alpha, :99 never exceeds, :118 attack/release
→ build.rs:100               absorbs :65 odd columns, :75 clamping, :90 NaN idempotence, :121 preset parity
→ backdrop_system:251        absorbs :163 mesh lifecycle, :182 rebuild-on-shape, :207 in-place edits, :233 global transform, :273 frozen, :289 reset, :308 ordering
→ particles/manager:244      absorbs :213 spawn count, :221 death, :233 slot reuse, :253 clear, :263 integration, :292 gravity, :305 colour interp
→ particles/system:95        absorbs :81 inactive, :110 no transform, :122 spawn position
→ camera_follow:208          absorbs :153/:171/:182 lerp+offset, :193 inside box, :228 no target
→ camera_follow:404          absorbs :255 zero look-ahead, :271 lead, :291 vertical, :313 decay, :334 cancel, :360 absorbed lead, :384 ramp
→ behavior_runner:427        absorbs :375 patrol, :475 no target, behavior_optimization:14/:75
→ game_loop_manager:154      absorbs :129 update, :141 accumulate, :169/:183 throttle, :197 reset
→ render_manager:506         absorbs :462/:468/:477/:482 classify, :488 latch, :498 refuses to render, :520 streak reset
→ render_manager:555         absorbs :576 non-main cameras, :593 resize
→ ui_integration:25          absorbs :63/:81/:114 SDF shape params, :94 border-as-one-sprite
→ ui_integration:189         absorbs :133 axis-aligned lines, :167 culled line, :225 nested, :250 pop restores
→ glyph_texture_cache:184    absorbs :143 empty/non-text, :165 cached not reported, :197 different sizes
→ ui_element_system:244      absorbs :135 draws, :146 hidden, :158 invisible, :172 ordering, :206 click, localization.rs:562 @-resolve
→ pickups:357                absorbs :209/:225/:245/:264 collect semantics, :286/:303 removal, :320/:336/:345 EffectTimer
→ gamepad_backend:297        absorbs :234 button table, :264 axis table, :282 dead zone, :326 disabled backend
→ tests/lifecycle.rs:67      absorbs :7 creation, :15/:30/:49 phases, :99 re-init, :113 error state, :125/:152 wait_for_state
→ tests/scene_lifecycle:82   absorbs :7 states, :59 update gating, :115/:239 errors, :188 world propagation, :145/:209 (deleted outright)
```

---

## WEAK KEEPS (keep, but the asserts must grow)

```
src/scene_serializer_tests.rs:80   — asserts one entity's Sprite fields. Must become, or be replaced by, the
                                     registry DRIFT GUARD; as written it cannot fail when a type is forgotten.
src/scene_loader.rs:338            — only exercises the `overrides` layer. Must cover all three layers and
                                     prove "later wins": base + overrides + inline on the same component type.
tests/sprite_animation_scene.rs:178 — after absorbing :202/:221 it must also assert the static-sprite region
                                     survives AND that an autoplaying clip overwrites the saved snapshot.
src/particles/manager.rs:244       — asserts alive_count == 4. Must assert the survivors are the LAST four
                                     spawned; the name's actual claim is untested.
src/scores.rs:289                  — after absorbing :304 it must assert both halves: a concurrent save merges,
                                     an explicit reset still clears.
src/render_manager.rs:555          — must additionally cover the zoom guard (zoom 0.0 / NaN / negative → 1.0),
                                     which today has no test anywhere.
src/ui_integration/tests.rs:25     — proves position and size under camera moves; must also assert one colour
                                     byte survives unchanged, which is the post-tonemap contract.
src/grid/backdrop_system.rs:251    — after absorbing :273 it must assert the frozen-delta case: still draws,
                                     impulses drained not banked, no energy gained.
src/pause.rs:402                   — proves time_scale flips; must be paired with a frame_tail test proving
                                     particles and SpriteAnimationSystem actually receive delta * time_scale.
src/menu_input.rs:166              — after absorbing the navigate table, keep the count==0 row: it is the only
                                     guard against the `count - 1` underflow panic.
```

---

## LINE COUNTS

```
engine_core src   — current 322, keep 59
engine_core tests/ — current  72, keep 13
engine_core total — current 394, keep 72
```

**If 60 is hard, cut these 12 first** (in order): `assets.rs:453`, `save_store.rs:204`, `localization.rs:591`, `game_config.rs:214`, `scene_data_tests.rs:46`, `glyph_texture_cache.rs:184`, `gamepad_backend.rs:297`, `menu_panel.rs:343`, `particles/system.rs:95`, `tests/scene_loader_parse.rs:183`, `grid/topology.rs:153`, `src/lifecycle.rs:306`. That lands at 60 and costs you: the AssetConfig mapping, the wasm-shaped store semantics, locale file discovery, one legacy config shape, the editor-camera block, glyph cache keying, hat edges, menu hit-testing, emitter rate, tilemap scene load, spring rest lengths, and lock-poisoning resilience. I would rather ship 72.

**Coordination note honoured:** no keeps spent on `scene_manager.rs`, `ui_manager.rs`, or `Timer` / `tests/timing.rs` — all three are dead or untethered (`engine_core::Timer` is exported in `lib.rs:88` and `prelude.rs:45` and has zero consumers across the engine, editor and all six games; its 5 tests are the only thing calling it).