# Aseprite Native — reviewer scorecard

One row per review. `real` = findings accepted; `false` = rebutted on inspection of the tree;
`policy` = conflicted with a standing ruling (the issues, a decision of record, Jesse's kickoff
rulings); `notable` quotes the catch worth remembering, because the review files are transient;
`time` is wall clock from the dispatch timestamp to the review file's mtime. External Tools'
scorecard (`../external-tools/reviewer-comparison.md`) is the baseline this effort reads.

| subject | reviewer | findings | real | false | policy | notable | time |
|---|---|---|---|---|---|---|---|
| plan v1 (round 1) | kimi | 9 | 9 | 0 | 0 | decoded memory unbounded by length checks (a 1 MiB file declaring 65,535 frames); a group's opacity and a per-frame palette outside the composite; a plain `Sprite` shows the whole sheet (`Sprite::new` is the full texture); a game-held `SpriteSheet` is never updated; a resized PNG keeps a stale grid; two pipelines, one eviction; a shrunk or removed active clip freezes the sprite (`sprite_system.rs:44`); a failed thumbnail never retried; a zero duration is infinite fps | 3 m 40 s |
| plan v1 (round 1) | gemini | 9 | 8 | 1 | 0 | real: named palette entries desync the parser; the exporter's sheet is `min(frames, 4)` columns and transparent RGB is retained under alpha 0; negative `i16` cel offsets; the cel-length rule rejects compressed cels; linked-cel chains and missing targets; a half-written save; the plain `Sprite`; zero duration; false: a dropped texture resource crashes a cached bind group (`remove_texture` drops a map entry, nothing calls `destroy`, wgpu refcounts the view) | 4 m 58 s |
| plan v1 (round 1) | codex | 6 | 6 | 0 | 0 | two new angles: the chunk-count fallback was reversed (new unless zero, then old); a resized atlas leaves a plain sprite's cell region stale, so a region equal to one old cell is re-mapped; plus the z-index and the opaque background layer on the composite finding; the rest = kimi's and gemini's; dispatched after kimi exited | 1 m 49 s |
