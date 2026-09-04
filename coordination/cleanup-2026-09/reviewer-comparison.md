# Reviewer comparison — kimi vs gemini on the cleanup's code reviews

Kept to decide the default reviewer at the end of the effort (Jesse, Sep 3 2026).
"Real" = a finding adjudicated ACCEPT that changed the diff; "false" = a finding REBUTTED
because the claim was wrong on inspection; "policy" = rebutted only because it conflicted
with a standing ruling (dead API deleted in batch 2, count pins removed). Time is wall clock
from launch to file.

| subject | reviewer | findings | real | false | policy | notable | time |
|---|---|---|---|---|---|---|---|
| plan v1 | kimi | 9 | 9 | 0 | 0 | caught the HashMap root-order flaw that made batch 8's golden test unmeetable | ~9 min |
| plan v2 | kimi | 11 | 9 | 2 | 0 | wrong on cargo self-dev-dependency and on `Sprite.shape` reaching scene files | ~9 min |
| common cut r1 | kimi | 4 | 2 | 0 | 2 | caught `Rect::contains` live (keep-list wrong) | ~7 min |
| common cut r2 | kimi | 3 | 3 | 0 | 0 | caught `Transform2D::forward` live in asteroids (my grep miss) | ~5 min |
| ecs cut | kimi | 7 | 5 | 0 | 2 | volume clamp, ComponentMeta names, BehaviorState default | ~13 min |
| input cut | kimi | 3 | 3 | 0 | 0 | default preset Menu/Select unpinned (BehaviorRunner consumer) | ~12 min |
| physics cut | kimi | 5 | 5 | 0 | 0 | all minor pins; docs-count finding led to the early count-strip commit | ~7 min |
| audio cut | kimi | 3 | 1 | 0 | 2 | load_sound happy path | ~6 min |
| renderer cut | kimi | 4 | 3 | 0 | 1 | `CameraUniform::from_camera` had no test in `common` (keep-list rationale wrong) | ~14 min |
| ui cut r1 | kimi | — | — | — | — | killed at the 10-minute tool cap (3,455-line diff) | >10 min |
| ui cut r2 | kimi | 4 | 4 | 0 | 0 | same-frame press+release click (the wasm tap) mislabelled a duplicate | ~9 min |
| engine_core cut | kimi | 6 | 5 | 0 | 1 | prefab RON wire unpinned; toast draw path; sidecar filter; static review (did not run the suite) | ~15 min |
| engine_core cut | gemini | 4 | 4 | 0 | 0 | Camera/Tilemap/Behavior extraction arms unpinned (unique); 3 of 4 overlap kimi; cited file:line links; static | ~6 min |
| editor cut | kimi | 4 | 4 | 0 | 0 | font load incl. bold + two #54 locks; Behavior defaults-within-ranges; a sweep of small pins; verified every called API against the tree (no compile break) | ~24 min |
| editor cut | gemini | 5 | 5 | 0 | 0 | same two blocking locks as kimi; unique: `type_key` release asymmetry in the new harness, scroll clamp bounds | ~12 min |
| editor_integration cut | kimi | 5 | 3 | 0 | 2 | verified every deleted guard against surviving coverage in other crates (zoom guard, shortcut table, rename refusal); the two rebuts asked for the mirror test in the reconstructed form | ~16 min |
| editor_integration cut | gemini | 6 | 6 | 0 | 0 | unique: the `dirty_editor` fixture desync (a real harness fidelity bug), pure-write refusal plumbing, clipboard trio narrowing; verdict 'reject pending minor revisions' for minors | ~7 min |
| batch 1 (gemini authored) | kimi | 5 | 4 | 0 | 1 | major: the plan's own tap ruling leaves latch consumers stuck — led Jesse to revise the ruling; checked-mul overflow; docs; verified callers of `just_activated` and `TextDrawData.height` | ~11 min |
| batch 1 (gemini authored) | claude | 4 | 3 | 0 | 0 | redundant literal list in the drift guard, missing equality tests, formatting churn; F3 superseded by kimi's better fix | — |
| batch 2 (gemini authored) | kimi | 4 | 3 | 0 | 1 | grepped every removed API for callers incl. `read_events::<CollisionData>`; found the stale hello_world comment; proposed scripting the games gate; the policy rebut asked to keep the ruled-out bus mirror | ~14 min |
| batch 2 (gemini authored) | claude | 3 | 3 | 0 | 0 | the batch was half done — the dead-API additions from the cut reviews were missed because the plan filed them under the wrong section (planner's error, fixed in the plan) | — |
| handoff-loop skill (plan-mode review of the workflow doc) | kimi | 10 | 10 | 0 | 0 | critical: "a pathspec commit takes the working tree, not the index the reviewers read" — a real hole in the planner's own commit step; plus untracked-file blindness, abandoned-batch reclaim, filename collisions, detached-review failure signals | ~12 min |
| batch 2 pass 2 (gemini authored) | kimi | 3 | 3 | 0 | 0 (one scoped: the retired ANALYSIS.md family is a batch-10 line) | "the per-crate CLAUDE.md files are supposed to be the accurate map of the trimmed API" — three crate guides still documented `Time`, `just_deactivated`/`is_active_any`, `reset_forces`/`raycast`/`pushable`/`bouncy`/`apply_force`; also caught that the new games gate silently dropped clippy | ~15 min (22:17 → 22:32) |
| batch 2 pass 2 (gemini authored) | claude | 3 | 3 | 0 | 0 | unexplained `PartialOrd, Ord` derive added to `EntityId` in a pure-deletion batch; `QueryPipeline` maintained but never read after `raycast` went; missed the crate-guide drift kimi found (checked only the three lines the fixes named) | — |
| batch 3 (gemini authored) | kimi | 3 | 2 | 1 | 0 | "four compiling, running doctests are deleted — weakened checks beyond the stated comment-hygiene intent" (the crates' only public-API usage pins); also caught the log_archive entry mis-dating the apply_impulse removal; the false one asked for a games gate that had already run | ~7 min (07:48 → 07:55) |
| batch 3 (gemini authored) | claude | 4 | 4 | 0 | 0 | "the tuple is the too_many_arguments allow in another costume" — one handler ducked the lint with `(state, commands): (&mut _, &mut _)`; the silent `let … else { return }` fallbacks; two reasons dropped with their tags; fixes applied by Claude per Jesse's ruling, no second round | — |
| handoff-loop skill v3 + comment/naming policy in the guides (code-mode review of the doc diff) | kimi | 5 | 5 | 0 | 0 | "Step 7 still claims the committed bytes are the reviewed bytes after step 6 makes that false by design" — the planner-applies-fixes ruling had broken the loop's central assertion; also: no hook runs the tag gate the guides said ran at every commit, and bare `#42` slipped the gate (45 in the tree) | ~7 min (08:34 → 08:41) |
| batch 4 (gemini authored) | kimi | 3 | 2 | 1 | 0 | "User clicks inside an already-focused field to place the caret on the exact frame an Up/Down key-repeat fires ... the caret placement is lost that frame" — sharpened the nudge-ordering finding into a reachable gesture; the false one missed that the fixture test asserts existence before loading | ~13 min (08:59 → 09:12) |
| batch 4 (gemini authored) | claude | 5 | 5 | 0 | 0 | the raw-RGBA loader's error precedence swap (`InvalidFormat` masking `TextureTooLarge`); the two design-listed ui tests never added; a dummy pad id for keyboard sources | — |
| batch 5 (gemini authored) | kimi | 6 | 3 | 2 | 1 | "`assert!(error_msg.contains(\"ExpectedMapColon\"))` asserts ron 0.12's internal `Error` variant name ... the test guards a dependency's diagnostics, not an engine contract" — and the self-regenerating golden fixture; the two false ones (games stop compiling, must_use breaks their clippy) came from a view that could not see the six game repos, where the sites were already migrated and gated green; the policy rebut was the plan's own builder deletion | ~9 min (12:07 → 12:16) |
| batch 5 (gemini authored) | claude | 12 | 11 | 0 | 1 | the handoff-listed guide lines the executor skipped; the two-tab race and reset pitfalls dropped with `save_to`; `FrameRequests` public fields and the missing exit-latch test (fixed by a pure `absorb`); the policy item was the executor's macro-not-method, accepted as the correct borrow reading — the design's claim was wrong | — |
| plan batch 6 correction (claude authored) | kimi | 7 | 7 | 0 | 0 | "`crates/editor/src/component_editors/tests.rs:209` asserts `history.undo_name() == Some(\"Set Transform\")` — same flip, same batch" in a file the section had declared unchanged; the `entity_ops.rs` factory test that calls the deleted `handle_create_action`; the play-time menu policy the section left unspecified, which a natural implementation would have turned into dead Save/Exit items during a playtest; and the scene_io line numbers pointing at the load-warning code instead of the reset block | ~14 min (12:58 → 13:12) |
| batch 6 (gemini authored) | kimi | 5 | 3 | 0 | 2 | "the old `handle_menu_bar` arms did `status_bar.show_message(format!(\"Undo: {}\", name))` before undoing; the refactored path routes through `dispatch_editor_action`, whose `A::Undo`/`A::Redo` arms contain no status message" — the one regression in the batch; the two policy rebuts were the plan's own drag-guard extension and the one-command no-wrap contract review 21 had asked for; it read the invented "+N more" truncation as a feature and missed every untouched guide | ~10 min (14:23 → 14:33) |
| batch 6 (gemini authored) | claude | 9 | 8 | 0 | 0 | the same Undo regression; every handoff-listed guide line untouched, including the add-component skill that now told an agent to invoke a deleted macro; the loss-message truncation as an unasked behaviour change; `ARCHETYPES` hand-enumerated beside `Archetype::ALL`; the ninth item was the pre-existing engine_core clippy pair, not the batch's | — |
| plan batch 7 correction (claude authored) | kimi | 4 | 4 | 0 | 0 | "`u32` (:381) and `string` (:389) build no id — they only call `self.row()` … a dead `id` binding → unused-variable warning, and the crate's gates deny warnings"; the six `component_editors/tests.rs` fixtures the section's call-site list missed; the popup's 4/4 padding split that "top padding 8" would have shifted; every load-bearing count and line number re-verified and confirmed (98 sites, eight token values, the three private fields' readers) | ~8 min |
| batch 7 (gemini authored) | kimi | 2 | 2 | 0 | 0 | "the safety rests entirely on one private-field invariant. The moment someone re-exposes `config` (or adds a `config_mut()`/`set_config`), snapping NaNs entity positions again with no test to catch it" — accepted in part as a NaN assert on the setter's existing clamp test, the guard deletion itself rebutted as the plan's ruling kimi had cleared in review-23; the one-frame arrow-toggle change (also found by claude) taken as a fix | ~6 min |
| batch 7 (gemini authored) | claude | 3 | 3 | 0 | 0 | the same arrow-toggle timing change, traced to the plan section's own pseudo-code rather than the executor; a comment stating its reason twice in the new module; a missing contract line on the popup renderer — every section item confirmed landed by grep, every deleted name grepped across the live guides | — |
| plan batch 8 correction (claude authored) | kimi | 5 | 5 | 0 | 0 | "`world_to_scene_data` hardcodes `prefabs: HashMap::new()` … so the golden fixture necessarily contains `prefabs: {}` where the source has five definitions … the realistic outcomes are a false-positive batch halt, or worse, pressure to \"fix\" the serializer" — the largest expected hand-diff the section had not listed; the hierarchy `visit` return contract silent on the removal path, with the prune trigger as the consequence and the missing test named; `log_archive.md:201` in the grep scope; `Name` in the drift test; the glyph prepare needing `ui_commands`. Every load-bearing claim re-verified and confirmed: creation-monotonic ids, the emission order physics included, the fourteen registry names, the bloom borrow, the `--all-targets` gap | ~12 min |
| batch 8 (gemini authored) | kimi | 3 | 2 | 0 | 1 | "the golden test hard-couples engine_core's suite to a living example asset outside the crate, with no regeneration procedure … the handoff records it must be generated *before* the serializer split, which makes ad-hoc regen after the split wrong-by-construction" — accepted as a bless path; the stale loader header; the ungated physics import in the serializer tests rebutted as outside the check-only gate. Cleared the hierarchy counts in all four cache cases and the serializer skip set in both feature shapes. Did not see that 8.3 was missing — a diff review cannot | ~16 min |
| batch 8 (gemini authored) | claude | 6 | 5 | 0 | 0 | "8.3 did not land; the report says it did" — the staged loader diff was one import hunk while the report enumerated five helpers, cfg-on-arm and deleted tuples; the executor's own out-of-scope hunk re-introducing the suppression tuple 8.3 was told to delete; `wire_name` kept alive by a trace log; the short-circuited cache stamp; the loader header. Golden fixture verified against the pre-batch serializer in a worktree | — |
| batch 8 pass 2, item 8.3 (gemini authored) | kimi | 1 | 0 | 0 | 1 | "the `physics`-off arms are compiled nowhere in practice … a syntax/type error inside them would slip through" — rebutted with the gate log (no-default-features check and clippy both ran clean); cleared both cfg shapes, every helper's parity and message text byte for byte | ~3 min |
| batch 8 pass 2, item 8.3 (gemini authored) | claude | 0 | 0 | 0 | 0 | every checklist item of the fixes prompt confirmed by grep; the report pasted its greps this time | — |

## Verdict (Sep 3 2026, after three diffs reviewed by both)

Totals over the three shared diffs (engine_core, editor, editor_integration): kimi 15 findings,
12 real, 0 false, 3 policy; gemini 15 findings, 15 real, 0 false, 0 policy. Neither reviewer
produced a false claim at code level. Overlap: 8 of 15 findings were found by both; kimi's
unique catches were the prefab RON wire, the toast draw path and the `commands` discovery keys;
gemini's were the Camera/Tilemap/Behavior extraction arms, the `type_key` release asymmetry,
the `dirty_editor` fixture desync and the pure-write refusal plumbing. Kimi is the more
thorough verifier (it re-runs the suite and greps callers across crates, and once rebutted its
own suspicions after checking); gemini is roughly twice as fast, cites file:line links, and
found more harness-fidelity bugs — the class of defect only a second reader catches. Gemini's
verdicts skew stricter ("reject pending minor revisions" for minors) while kimi's are
calibrated to severity.

Recommendation: keep **kimi as the default single reviewer** (its verification discipline is
what made it trustworthy on the 3,000-to-12,000-line diffs, and the plan-level flaws it caught
were the expensive ones), and **run both on any diff that changes a test harness, a fixture, or
a public seam** — every gemini-only finding was in that class. For small, single-crate diffs
one reviewer is enough; alternate them so neither's blind spots become the default's.

Observations to weigh at the end: every kimi review verified claims by running the suite and
grepping callers, and each round found at least one contract the keep-list had wrongly
dropped; its two false positives were both plan-level (a cargo rule and a data-flow claim),
none at code level so far. Gemini rows fill in from the engine_core cut onward.
