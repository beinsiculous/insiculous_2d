---
name: finish-task
description: Definition-of-done verification checklist for the insiculous_2d engine. Run before claiming any task complete, before committing, or when the user asks "is it done?". Catches the failure modes that slip through - unrun tests, clippy warnings, ignored doc tests, oversized files, stale docs.
---

# Finish Task — Definition of Done

Run every gate below. Do not summarize the work as complete until all pass.
If a gate fails, fix it or report it explicitly — never soften ("mostly
passing") and never weaken a test/lint to get green.

## Gate 1 — Compile & tests

```bash
cargo check --workspace          # fast fail
cargo test --workspace           # REQUIRED: 0 failed, 0 ignored
```

- If any test is `#[ignore]`d or a doc example uses ` ```ignore `, that's a
  failure — GPU/window-bound doc examples must be ` ```no_run `.
- Paste the actual summary line (`test result: ok. N passed...`) in your
  report. Never state counts you didn't observe.

## Gate 2 — Lints

```bash
cargo clippy --workspace --all-targets   # REQUIRED: 0 warnings
```

The workspace is fully clippy-clean including tests and examples. Any warning
you introduced must be fixed properly — `#[allow(...)]` is not a fix.

## Gate 3 — Code standards sweep (on files you touched)

- No `unwrap()`/`expect()` outside `#[cfg(test)]` (use `let-else`, `?`, `.ok()`).
- No `#[allow(dead_code)]`, no `TODO:` left behind.
- No tags in comments — say why, not where it was decided (`training.md`
  § Comment Policy). Run the gate and paste that it printed nothing:
  `grep -riEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|#[0-9]+\b|Sprint [0-9]" crates src examples --include=*.rs`
- Every touched file ≤ 600 lines (`wc -l` them). Split, don't grow.
- Test names describe behavior (`test_selection_toggle_adds_and_removes`),
  not implementation (`test_toggle_method`).
- Public APIs you added have doc comments; colors/magic numbers routed through
  the proper token/constant (e.g. `EditorTheme`, `constants.rs`).

## Gate 4 — Docs & coordination

- If behavior, counts, or architecture changed: update `AGENTS.md` (and
  `PROJECT_ROADMAP.md` only if a settled decision changed) to match reality.
- If you resolved tech debt, close its `tech-debt` issue (commit "fixes …#N";
  append lessons to `log_archive.md` if worth keeping). If you created debt,
  file a `tech-debt` issue (or extend the crate's low-priority backlog issue)
  — the per-crate `TECH_DEBT.md` files are retired, never recreate one.
- If working under the coordination protocol: append to
  `coordination/PROGRESS.md` and remove your lock file.

## Gate 5 — Honest report

Your final summary must state: what changed (files), the exact test summary
line, clippy status, and anything skipped or deferred — with the reason. If
something is unverified, say "unverified", not "should work".
