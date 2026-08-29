# Continue Development Workflow

Proceed with the next task from the project roadmap using this structured workflow.

## Prerequisites

Before starting, ensure you have read:
- `AGENTS.md` - Project status, architecture, and high-level guidance
- `PROJECT_ROADMAP.md` - Current priorities and incomplete tasks
- `training.md` - API patterns and established conventions

---

## 1. Identify the Next Task

Read `PROJECT_ROADMAP.md` and identify the next incomplete task using this priority order:
1. High priority technical debt (stability/architecture risks)
2. Active roadmap phase milestones
3. Medium priority improvements
4. Documentation/testing gaps

**Decision point:** If the task is large (>2 hours estimated), use `SetTodoList` to break it into subtasks before proceeding.

---

## 2. Gather Context

### 2.1 Read Existing Documentation
- Crate's `ANALYSIS.md` (if exists) - previous analysis and plans
- Crate's open `tech-debt` issues (`gh issue list -R beinsiculous/insiculous_2d --label tech-debt`; for a game, `-R beinsiculous/<game>`) - known issues in this crate
- `PROJECT_ROADMAP.md` - vision, settled decisions, phase context

### 2.2 Understand Current State
- List the crate's source files (`ls crates/<name>/src/`)
- Run existing tests (`cargo test -p <crate>`)
- Check for recent changes (`git log --oneline -5`)

---

## 3. Plan and Document Changes

### 3.1 Update ANALYSIS.md
Before modifying code:
- Document the current state of relevant components
- Outline the planned approach and design decisions
- Note any risks or open questions

If no `ANALYSIS.md` exists:
- Create one with current state assessment
- Document the problem and proposed solution

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
- Prefer small, focused functions (target: <50 lines)
- Add documentation for public APIs

### 4.2 Test-Driven Approach
- Write tests for new functionality
- Run tests frequently during development
- Ensure existing tests still pass

---

## 5. Validate the Implementation

### 5.1 Testing
```bash
cargo test -p <crate>          # Unit tests
cargo test --workspace         # Full test suite
cargo build --workspace        # Compilation check
```

### 5.2 Integration (if applicable)
For user-facing features:
- Consider adding to `examples/hello_world.rs` OR
- Create a new example demonstrating the feature
- Verify with `cargo run --example <name>`

**Note:** Internal refactors (e.g., SRP improvements) don't need example updates.

---

## 6. Update Documentation

### 6.1 Update ANALYSIS.md
- Mark completed work
- Document any deviations from original plan
- Note follow-up work identified

### 6.2 Update the board (if relevant)
- Resolved debt: close its `tech-debt` issue ("fixes …#N"); append lessons to
  `log_archive.md` if worth keeping
- New debt discovered: file a `tech-debt` issue (or extend the crate's
  low-priority backlog issue) — never recreate a `TECH_DEBT.md` file

### 6.3 Update PROJECT_ROADMAP.md (only if a settled decision changed)
- The roadmap carries vision + decisions, not tasks or metrics

### 6.4 Update AGENTS.md (if architecture changed)
- Update system descriptions
- Update metrics and status

---

## 7. Final Verification

Before finishing:
- [ ] All tests pass: `cargo test --workspace`
- [ ] No compilation warnings: `cargo build --workspace`
- [ ] Documentation updated (ANALYSIS.md as needed; board issues closed/filed)
- [ ] Follow-up work filed as new issues on the board

---

## Error Handling

If tests fail during verification:
1. Fix compilation errors first
2. Address failing tests
3. If stuck, use `Task` tool to spawn a subagent for debugging
4. Document any workarounds or tech debt created

---

## Output

When complete, provide:
1. Summary of changes made
2. Files modified/created
3. Test results
4. Any follow-up work identified
