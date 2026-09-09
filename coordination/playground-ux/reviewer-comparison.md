# Reviewer comparison — Playground UX

One row per review. "Real" = a finding adjudicated ACCEPT that changed the plan or the
diff; "false" = a finding REBUTTED because the claim was wrong on inspection; "policy" =
rebutted only because it conflicted with a standing ruling. The notable catch is quoted in
the row because the review files are transient. Time is wall clock from launch to the
review file's mtime. The subject names the repo, because review numbering is per subject
directory (`insiculous_2d/review/playground-ux/`, `insiculous_web/review/playground-ux/`).

| subject | reviewer | findings | real | false | policy | notable | time |
|---|---|---|---|---|---|---|---|
| engine · plan v1 | kimi | 11 | 11 | 0 | 0 | "an open command-API batch is silently lost on Keep — rebase only walks history entries"; the merge id-reassignment burying a session id below the floor | ~9.5 min |
| engine · plan v1 | codex | 10 | 10 | 0 | 0 | "Export still omits the edit used to pass the acceptance test" — `playground_export_zip` carries the last saved scene; and the whole-component replay importing simulated fields the user never touched | ~1.6 min |
| engine · plan v2 | kimi | 10 | 10 | 0 | 0 | "the 6 s heartbeat release fires on timer throttling, not just death — silently breaking the one-simulation invariant"; a blocked popup filing the snapshot and latching the reservation | ~7.5 min |
| engine · plan v2 | codex | 7 | 7 | 0 | 0 | "Live Export silently deletes the project's other scenes"; `scripts_to_data` drops a parameter whose target is named only after serialization; the first dispatch died on the usage limit and wrote nothing | ~1.5 min (2nd dispatch) |
| engine · plan v3 | kimi | 7 | 7 | 0 | 0 | "batch 3's pointermove focus handler as specified steals focus from every drag on the page" — drag-selecting console text would focus the canvas; and a reloaded editor page dissolving the preview reservation | ~10.5 min |
| engine · plan v3 | codex | 7 | 7 | 0 | 0 | "Transform2D.position is a glam::Vec2, whose JSON representation is an array" — the leaf-level diff would keep the simulated Y when only X was edited; the snapshot bridge returning bytes without the scene entry the preview needs | ~1.3 min |
| engine · plan v4 | codex | 7 | 7 | 0 | 0 | "calling the existing request_redraw() from a timer still requests an animation frame" — the hidden-tab fallback needed a proxy wake; at 390 px the default dock leaves the centre no width at all | ~2 min |
| engine · plan v5 | codex | 4 | 4 | 0 | 0 | "Keep can recreate an empty entity after simulation destroys a paused creation" — CreateEntityCommand::undo re-captures from the missing entity; and the auto-collapsed inspector that could never reopen | ~2 min |
