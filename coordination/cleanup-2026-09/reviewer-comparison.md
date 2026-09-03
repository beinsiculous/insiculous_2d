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

Observations to weigh at the end: every kimi review verified claims by running the suite and
grepping callers, and each round found at least one contract the keep-list had wrongly
dropped; its two false positives were both plan-level (a cargo rule and a data-flow claim),
none at code level so far. Gemini rows fill in from the engine_core cut onward.
