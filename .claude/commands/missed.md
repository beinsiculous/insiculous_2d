# Missed Issues Analysis

Analyze a crate for code quality violations that may have been missed during development.

**Usage:** Invoke with a crate name: `@missed analyze the ecs crate`

**Scope:** Focus on `crates/<crate_name>/src/` directory

---

## Phase 1: Context Gathering

### 1.1 Validate Target Crate

First, verify the crate exists:
```bash
ls crates/<crate_name>/
```

If not found, list available crates:
```bash
ls crates/
```

### 1.2 Read Existing Documentation

Read these sources in order:
1. The crate's open `tech-debt` issues - `gh issue list -R beinsiculous/insiculous_2d --label tech-debt` (known existing debt)
2. `PROJECT_ROADMAP.md` - Vision + settled decisions
3. `AGENTS.md` - High-level project guidance
4. `training.md` - Established patterns to check against

### 1.3 Assess Crate Size

Count files and lines to determine analysis approach:
```bash
find crates/<crate_name>/src -name "*.rs" | wc -l
```

**For large crates (>20 files):** Use targeted grep patterns instead of reading every file.

---

## Phase 2: Targeted Code Quality Audit

Use `Grep` and `Glob` tools for efficient pattern detection rather than reading every file.

### 2.1 DRY Violations (Don't Repeat Yourself)

**Detection patterns:**
```bash
# Find similar error handling patterns
grep -n "expect\|unwrap\|panic!" *.rs | head -20

# Find repeated code blocks (3+ lines)
# Look for identical patterns in different functions
```

**Check for:**
- [ ] Duplicated code blocks (3+ lines repeated across files)
- [ ] Similar functions that could be generalized
- [ ] Copy-pasted logic with minor variations
- [ ] Repeated error handling patterns (`.expect()` with same message)
- [ ] Redundant type conversions

**Tool recommendation:** Use `Grep` with patterns like:
- `pattern: "fn.*\{[^}]*\}"` to find small functions that might be duplicates
- `pattern: "\.expect\("` to find error handling patterns

### 2.2 SRP Violations (Single Responsibility Principle)

**Detection approach:**
```bash
# Find large structs and impl blocks
grep -n "^pub struct" *.rs
wc -l *.rs | sort -n
```

**Check for:**
- [ ] Structs with >3 distinct responsibilities (check fields and methods)
- [ ] Functions doing multiple unrelated things (>50 lines is a warning sign)
- [ ] Files mixing unrelated functionality
- [ ] Methods that should be split (complex control flow)
- [ ] God objects (>500 lines) or god functions (>100 lines in Rust)

**Context note:** In Rust, 100-line functions are very large. Typical good functions are 10-30 lines.

### 2.3 KISS Violations (Keep It Simple, Stupid)

**Detection patterns:**
```bash
# Find complex generics
grep -n "<.*,.*,.*>" *.rs  # 3+ generic parameters

# Find deeply nested code
grep -n "        " *.rs | wc -l  # 8+ space indents = deep nesting
```

**Check for:**
- [ ] Over-engineered abstractions (traits with single implementation)
- [ ] Unnecessary generics or complex trait bounds
- [ ] Complex nested logic (>3 levels of nesting)
- [ ] Premature optimization (unsafe code, complex algorithms)
- [ ] Unused flexibility (config options never changed)

### 2.4 Additional Quality Checks

Use targeted searches:
- **Unused imports:** `cargo clippy` (if available) or check for `#[allow(unused_imports)]`
- **Missing docs:** `grep -L "^///" *.rs` to find undocumented public items
- **Error handling:** `grep -n "unwrap\|expect" *.rs | wc -l` (count potential panic points)
- **Test coverage:** Compare `grep -c "^fn " *.rs` vs `grep -c "#\[test\]" *.rs`

---

## Phase 3: Architecture Review

### 3.1 File Placement Audit

For each major file, verify:
- [ ] File location matches its responsibility
- [ ] Public API is intentionally exposed (not `pub` by default)
- [ ] Internal helpers are private or `pub(crate)`
- [ ] Module structure follows Rust conventions (`mod.rs` or flat structure consistently)

### 3.2 Cross-Crate Dependencies

Check `Cargo.toml`:
```toml
[dependencies]
# Are all dependencies necessary?
# Could any be dev-dependencies?
```

Verify:
- [ ] No circular dependencies between crates
- [ ] Dependencies follow intended architecture (lower crates don't depend on higher ones)
- [ ] No unused dependencies

---

## Phase 4: File Findings as Issues

Debt lives on the Studio Board, not in files (the per-crate `TECH_DEBT.md`
files were retired Aug 28 2026 — never recreate one).

### 4.1 Check for Existing Issues

Search the board before filing: `gh issue list -R beinsiculous/insiculous_2d
--label tech-debt` (games debt lives on the game's own repo). If a finding is
already tracked, note the issue number instead of filing a duplicate; if an
existing issue is resolved by current code, say so in the report (it gets
closed, with lessons appended to `log_archive.md` if worth keeping).

### 4.2 Filing Convention

- **High/Medium findings:** one issue each, labeled `tech-debt`, titled
  `[<crate>][tech-debt] <ID> — <short description>` with a `[CATEGORY-NNN]`
  style id (DRY/SRP/KISS/ARCH/GAP/...). Body carries: file/line pointers, what
  the issue is, the suggested fix, priority, and estimated effort
  (Small/Medium/Large).
- **Low findings:** append as checklist items to the crate's existing
  `[<crate>][tech-debt] Low-priority backlog` issue (create it if the crate
  has none), one line per item with its id and code pointer.
- Add every new issue to the org project:
  `gh project item-add 1 --owner beinsiculous --url <issue-url>`.

### 4.3 Prioritization Guidelines

**High Priority:**
- Data loss risks
- Stability issues
- API contract violations
- Security concerns

**Medium Priority:**
- Significant maintenance burden
- Clear refactoring path
- Pattern drift from established conventions

**Low Priority:**
- Style inconsistencies
- Minor duplication (<5 lines)
- Documentation gaps

---

## Phase 5: Cross-Reference

Cross-check findings against the board (`--label tech-debt`) and
`log_archive.md` (previously resolved items — a finding that resurrects a
resolved item deserves a note about the regression). `PROJECT_ROADMAP.md`
carries no debt section; do not add one.

---

## Phase 6: Report Summary

Provide a structured summary:

### Statistics
```
Total issues found: X
- DRY: X
- SRP: X
- KISS: X
- Architecture: X

By priority:
- High: X
- Medium: X
- Low: X
```

### Top 3 Priority Items
1. **[CODE-001]** Brief description (High priority)
2. **[CODE-002]** Brief description (Medium priority)
3. **[CODE-003]** Brief description (Medium priority)

### Recommendations
- Quick wins (low effort, high impact)
- Architectural improvements (medium effort, long-term benefit)
- Items to monitor (not urgent but watch for growth)

### Issues Filed/Updated
- New `tech-debt` issues (one per High/Medium finding)
- The crate's low-priority backlog issue (checklist items appended)

---

## Output Format

When complete, provide:
1. **Issue URLs filed/updated** (with their `[CATEGORY-NNN]` ids)
2. **Summary statistics** (as shown above)
3. **Top 3 priority issues** with brief context
4. **Recommendations** for next steps
5. **Existing issues found already-resolved** (candidates to close)
