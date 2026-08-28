# Technical Debt — Workspace Rollup — LIVE (open items only)

Last updated: July 13, 2026 (Game Programming Patterns audit — closed; history in `log_archive.md`).
Resolved history for every crate: `log_archive.md`.

This file is the high-level index of open technical debt across the workspace.
Each crate carries the authoritative detail in its own `crates/<name>/TECH_DEBT.md`
(the games in `../games/TECH_DEBT.md`); this rollup tracks open counts and the
items worth scheduling. High + Medium items are mirrored in `PROJECT_ROADMAP.md`.

---

## Status by Crate

| Crate | Last Audited | Open (High / Med / Low) | Notes |
|-------|--------------|--------------------------|-------|
| `audio` | Aug 2026 | 0 / 0 / 0 | Clean (manager.rs split resolved Aug 27 2026 — see `log_archive.md`) |
| `common` | Feb 2026 | 0 / 2 / 3 | `CameraUniform` duplicated in renderer; cross-crate volume clamping |
| `ecs` | Jul 2026 | 0 / 1 / 5 | GPP-16 registry extensibility; GPP-02 is a decision-of-record |
| `ecs_macros` | Feb 2026 | 0 / 1 / 2 | Over-specified `syn` features |
| `editor` | Aug 2026 | 0 / 0 / 0 | Fully clean (GPP-L7 resolved by Sprint 4 drag-state rework) |
| `editor_integration` | Jul 2026 | 0 / 0 / 2 | Clean of Mediums (file picker, menu-label strings remain) |
| `engine_core` | Jul 2026 | 0 / 1 / 7 | ARCH-006 behavior registry |
| `input` | Jul 2026 | 0 / 1 / 3 | GAP-001 gamepad backend |
| `physics` | Jul 2026 | 0 / 0 / 6 | Clean of Mediums; Lows incl. EDIT-001 (RigidBody config edits need rebuild) |
| `renderer` | Jul 2026 | 0 / 0 / 2 | Clean of Mediums (DRY-006, ARCH-006 remain) |
| `ui` | Jul 2026 | 0 / 0 / 5 | — |
| `../games` | Jul 2026 | 0 / 2 / 2 | GPP-11 shadow bricks, GPP-12 brick-tag Type Object (GPP-03 closed with game 3) |

Workspace-wide invariants (verified by the June 2026 audits): no files over
600 lines, no `#[allow(dead_code)]`, no `unwrap()` outside tests, and
`cargo clippy --workspace` is clean.

---

## Open High-Priority Items

None. (GPP-01 resolved Jul 13 2026 — see `log_archive.md`.)

## Open Medium-Priority Items

### engine_core (1)
- **[ARCH-006]** Behaviors hardcoded in scene serialization, bypassing `ComponentRegistry` — route through a registry/`Custom` variant; pairs with Phase 4 scripting and ecs GPP-16 (Large)

### ecs (1)
- **[GPP-16]** `global_registry()` not extensible by games — one-shot init extension point

### games (2)
- **[GPP-11]** Breakout shadow `Vec<Brick>` → `BrickState` component
- **[GPP-12]** Stringly-typed brick tags → typed `BrickSpec` component

### ui (1)

### input (1)
- **[GAP-001]** No gamepad backend — gilrs poll in the engine event loop

### common (2)
- **[ARCH-001]** `CameraUniform` duplicated in renderer — use `common::CameraUniform` everywhere
- **[DRY-002]** Volume clamping duplicated across `audio` and `ecs` — `clamp_volume()` in common

### ecs_macros (1)
- **[KISS-001]** `syn = { features = ["full", "parsing"] }` overkill — `["derive"]` only

---

## Process

- Audit a crate → record findings in `crates/<name>/TECH_DEBT.md` with `[CATEGORY-NNN]` ids, priority, and suggested fix
- Fix High/Medium where the fix is contained; **move resolved items to `log_archive.md`** with the resolution and date (live docs carry open work only)
- Update this rollup and the `PROJECT_ROADMAP.md` Technical Debt section after each audit pass
- Feature gaps (missing systems, e.g. audio streaming, gamepad backend, touch input) are tracked as roadmap work, not debt
