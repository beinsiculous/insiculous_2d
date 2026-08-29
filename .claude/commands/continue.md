# Continue Development Workflow

Proceed with the next task from the Studio Board using this structured workflow.

## Prerequisites

Before starting, ensure you have read:
- `AGENTS.md` - Project status, architecture, and high-level guidance
- `training.md` - API patterns and established conventions
- `PROJECT_ROADMAP.md` - Phase context and priorities (background, NOT the task queue)

---

## 1. Identify the Next Task

**Task source of truth: the org taskboard** — https://github.com/orgs/beinsiculous/projects/1

```bash
gh issue list -R beinsiculous/insiculous_2d --state open   # engine + editor tasks
gh issue list -R beinsiculous/<game> --state open          # game-specific tasks live in their own repos
```

Pick the next task by the board's Priority/Phase fields. When the fields tie,
prefer:
1. High priority technical debt (stability/architecture risks)
2. Active roadmap phase milestones (see `PROJECT_ROADMAP.md` for phase context)
3. Medium priority improvements
4. Documentation/testing gaps

**Claim it:** assign yourself and/or comment on the issue before starting.
(The old `coordination/current_tasks/` lock files and TODO.md queue are retired.)

**Decision points:**
- Read the full issue (`gh issue view <N> -R beinsiculous/insiculous_2d --comments`) —
  scope, acceptance criteria, and prior discussion live there, not in the roadmap.
- If the task is large (>2 hours estimated), break it into a todo list before proceeding.

---

## 2. Gather Context

### 2.1 Read Existing Documentation
- Crate's `CLAUDE.md` (if exists) - domain expertise and Godot oracle references
- Crate's open `tech-debt` issues (`gh issue list -R beinsiculous/insiculous_2d --label tech-debt`; for a game, `-R beinsiculous/<game>`) - known issues in this crate
- `PROJECT_ROADMAP.md` - vision, settled decisions, phase context
- Linked/referenced issues on the board - related or blocking work

### 2.2 Understand Current State
- List the crate's source files (`ls crates/<name>/src/`)
- Run existing tests (`cargo test -p <crate>`)
- Check for recent changes (`git log --oneline -5`)

---

## 3. Plan and Document Changes

### 3.1 Plan on the Issue
Before modifying code, for non-trivial tasks:
- Comment your planned approach and design decisions on the GitHub issue
- Note any risks or open questions there — the issue thread is the durable
  record (ANALYSIS.md files are retired; do not create them)
- For risky or architectural changes, consider `/adversarial-review` (plan mode)

### 3.2 Consider Context Isolation
For complex changes that might clutter the main context:
- Use the `Task` tool to spawn subagents for:
  - Exploring large codebases
  - Fixing compilation errors
  - Researching specific patterns

---

## 4. Implement the Feature

### 4.1 Follow Project Patterns
- Use patterns from `training.md`
- Match existing code style in the crate
- Prefer small, focused functions (target: <50 lines); files stay under 600 lines
- Add documentation for public APIs

### 4.2 Test-Driven Approach
- Write tests for new functionality
- Run tests frequently during development (`cargo check --workspace` after each edit)
- Ensure existing tests still pass

---

## 5. Validate the Implementation

### 5.1 Testing
```bash
cargo test -p <crate>                        # Unit tests
cargo test --workspace                       # Full test suite (0 failed, 0 ignored)
cargo clippy --workspace --all-targets       # Must be fully clean, 0 warnings
```

Use the `/finish-task` skill for the full definition-of-done checklist before
claiming the task complete.

### 5.2 Integration (if applicable)
For user-facing features:
- Consider adding to `examples/hello_world.rs` OR
- Create a new example demonstrating the feature
- Verify with `cargo run --example <name>`

**Note:** Internal refactors (e.g., SRP improvements) don't need example updates.

---

## 6. Update Documentation

Open work lives on the Studio Board; history lives in `log_archive.md`.

### 6.1 Update the board (if relevant)
- Resolved debt: close its `tech-debt` issue ("fixes …#N"); append lessons to
  `log_archive.md` if worth keeping
- New debt discovered: file a `tech-debt` issue (or extend the crate's
  low-priority backlog issue) — never recreate a `TECH_DEBT.md` file

### 6.2 Update PROJECT_ROADMAP.md (only if a settled decision changed)
- The roadmap carries vision + decisions, not tasks or metrics

### 6.3 Update AGENTS.md (if architecture or metrics changed)
- Update system descriptions
- Update test counts and status

### 6.4 Log Progress
- Append a timestamped summary to `coordination/PROGRESS.md` (the narrative log)

---

## 7. Final Verification and Close-Out

Before finishing:
- [ ] `/finish-task` checklist passes (tests, clippy, file sizes, docs)
- [ ] All tests pass: `cargo test --workspace` (0 failed, 0 ignored)
- [ ] Clippy fully clean: `cargo clippy --workspace --all-targets`
- [ ] Documentation updated (AGENTS.md as needed; board issues closed/filed)
- [ ] `coordination/PROGRESS.md` entry appended
- [ ] Follow-up work filed as new issues on the board (not buried in docs)

**Close the issue:** commit with a message referencing it —
`fixes beinsiculous/insiculous_2d#N` (or the game repo's `fixes #N`) — or close
it with a comment summarizing the resolution.

---

## Error Handling

If tests fail during verification:
1. Fix compilation errors first
2. Address failing tests (never delete/weaken a test to make it pass)
3. If stuck after 2 attempts, stop thrashing: consult the Godot oracle, write
   findings to `coordination/BLOCKERS.md`, and report on the issue
4. Document any workarounds or tech debt created

---

## Output

When complete, provide:
1. Summary of changes made
2. Files modified/created
3. Test results
4. Issue(s) closed and any follow-up issues filed
