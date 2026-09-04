# Reviewer comparison — kimi vs gemini on the web-playground effort

Continues the cleanup's ledger (`coordination/cleanup-2026-09/reviewer-comparison.md`), same
columns, so the default-reviewer decision stays a number. "Real" = a finding adjudicated
ACCEPT that changed the plan or the diff; "false" = REBUTTED because the claim was wrong on
inspection; "policy" = rebutted only because it conflicted with a standing ruling. Time is the
review file's mtime minus the dispatch timestamp.

| subject | reviewer | findings | real | false | policy | notable | time |
|---|---|---|---|---|---|---|---|
| plan v1 | kimi | 13 | 13 | 0 | 0 | overlay flushed only on scene save (script edits lost on reload); imported slug absent from the build-time project list; dirty guard on project switch; quota failure must not leave "Scene saved" standing | ~9 min |
| plan v1 | gemini | 8 | 6 | 2 | 0 | Rhai script functions cannot read outer-scope variables, so Scope-seeded params could never work (unique); post-physics-only scripts lag kinematic paddles a frame (unique); commands lacked entity targets; false: winit key leak (canvas-scoped listener), request ids over a FIFO | ~5 min |
| plan v2 | kimi | 9 | 9 | 0 | 0 | revision check is get-then-put, not a CAS (two tabs at 3 both write 4); base-joined VFS keys vs the relative project root — a save lands on a different key and the editor keeps reading the stale bundled copy (unique); `[profile.wasm-release]` ignored outside the workspace root (unique); dropped queue lines break FIFO pairing (unique) | ~10 min |
| plan v2 | gemini | 10 | 10 | 0 | 0 | Rhai `call_fn` passes by value so a plain ScriptCommands loses every command (unique, Critical); IDB unavailable in private browsing aborts boot (unique); compile on save so Edit mode shows errors (unique); shared with kimi: CAS, debounce loss, key mismatch, non-atomic import | ~4.5 min |
