# Reviewer comparison — Pattern Audit

One row per review. "Real" = a finding adjudicated ACCEPT that changed the plan or the
diff; "false" = a finding REBUTTED because the claim was wrong on inspection; "policy" =
rebutted only because it conflicted with a standing ruling. The notable catch is quoted in
the row because the review files are transient. Time is wall clock from launch to the
review file's mtime. Review numbering is per subject directory
(`insiculous_2d/review/pattern-audit/`, `games/breakout/review/pattern-audit/`).

| subject | reviewer | findings | real | false | policy | notable | time |
|---|---|---|---|---|---|---|---|
| engine · plan v1 | kimi | 8 | 5 | 2 | 1 | "`merged_components` is a test-side replica of the engine's merge … a compile error, so `cargo test` in breakout (a batch-4 gate) fails to build" — breakout's `levels_tests.rs`, which the plan's file list missed; and game-template being cloned after all. False: ProjectHost "never runs through GameRunner's tail" (it runs inside `EditorGame` through `run_game`); the guide gate "might be red" (it had run green) | ~8 min |
| engine · plan v1 | gemini | 6 | 5 | 1 | 0 | "the merge logic copies the key `PlayerTopDown` from the override's inner object onto the base's … `invalid length 2, expected an enum with a single variant`" — `Behavior` and `Scripts` are newtype variants and must replace whole. False: `ron::Value` "unconditionally returns ExpectedDifferentType" on `Transform2D(…)` (0.12.0 visits a named struct as a map); the one-frame gizmo lag (identical in both schemes) | ~12 min |
| engine · plan v2 | kimi | 5 | 5 | 0 | 0 | "`load_scene` does `world.clear()` + re-instantiate and today does not reset the transform system … every such entity clean, the loader-written `GlobalTransform2D::default()` survives, and the entity renders and picks at the origin" — a live editor bug the plan's Stop-only marker would have inherited | ~12 min |
| engine · plan v2 | gemini | 6 | 6 | 0 | 0 | the same load-path marker, independently; "`check_games.sh --test` fails compilation on Breakout, blocking the engine commit" — the cross-repo staging order; and the picking cluster at the origin for the frame after a load, which the loader's default globals cause | ~18 min |
