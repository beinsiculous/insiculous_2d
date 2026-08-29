# Task Queue — MOVED to the GitHub Studio Board (Aug 19 2026)

**The source of truth for open work is now the org taskboard:**
https://github.com/orgs/beinsiculous/projects/1

Issues live in their home repos (engine → `beinsiculous/insiculous_2d`,
game → that game's repo, site → `insiculous_web`) and every issue joins the
board with Priority (P0–P3) and Phase (E–J / Editor / Tech Debt / Ops)
fields. The full backlog was migrated from this file + PROJECT_ROADMAP.md on
Aug 19 2026; the TECH_DEBT.md files were retired into `tech-debt`-labeled
issues on Aug 28 2026 (record in `log_archive.md`).

Agent workflow (replaces this file's queue + the lock files):
- Find work:   `gh issue list -R beinsiculous/insiculous_2d` (or the game repo)
- Claim:       assign yourself / comment on the issue (replaces coordination/current_tasks/ locks)
- Finish:      reference the issue in the commit ("fixes beinsiculous/insiculous_2d#6"), close on merge
- Narrative:   coordination/PROGRESS.md remains the detailed work log — keep appending entries there

PROJECT_ROADMAP.md remains the *why/architecture* record (phases, settled
decisions, specs); it no longer carries the live open-item queue.
