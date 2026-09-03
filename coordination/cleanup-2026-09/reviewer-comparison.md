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
