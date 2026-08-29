# Insiculous 2D — Project Roadmap

> **Open tasks live on the org Studio Board, not in this file:**
> https://github.com/orgs/beinsiculous/projects/1 (issues on
> `beinsiculous/insiculous_2d`; games in their own org repos). Query with
> `gh issue list -R beinsiculous/insiculous_2d`. This file carries the vision,
> the settled decisions, and the phase map — the *why*, never the open-item
> list. Completed-work detail lives in `log_archive.md`. Technical debt is the
> board's `tech-debt` label. (Task tracking migrated to issues Aug 28 2026.)

## Vision: Deion the Insiculous

**The world of Deion is the project's identity.** Deion the Insiculous — a
SNES-styled hero, a ball of DEIONized water with an icicle mohawk — lives in a
food-coded world. Every game we ship is a window into that world: unique
"Deion Style" pixel-art characters and assets, not stand-in shapes. The
geometry-wars neon look that carried games 1–6 is demoted to an FX/accent
layer; SNES-era sprite art is the primary style.

**The 20 Games Challenge is the vehicle**, not the destination: arcade classics
teach the engine and expose gaps, building toward original Deion titles. The
challenge is **paused at game 7 (Tetris)** while the Deion Pivot (Phases E–I)
lands; it resumes with the new asset style from day one.

**Studio premise (Jesse, Aug 19 2026): Be Insiculous is an AI dev studio**, not
a game dev studio that happens to use AI. AI-assisted development is the primary
workflow and part of the product story — the studio umbrella also covers Mily's
ongoing non-game AI work. Consequences: **free web releases showcase the AI
workflow, AI art included**; **marketplace releases (Steam, iOS, Android —
anything charging money) ship hand-drawn art only** (tiered AI-asset rule
below). The first marketplace release target is **Insiculous Arcade** (Phase J).

Engine status, test counts, and per-system capability detail live in
`CLAUDE.md` (kept current) — not here.

## Settled Decisions (decisions of record — don't re-litigate)

- **Art source: mix** (Jul 28 2026) — Jesse hand-draws hero assets (Aseprite →
  PNG); simple tiles/props are code-generated **offline into PNGs** (never
  runtime rgba). All 6 games get full Deion-world theming; ChaosTheme neon is
  the FX/accent layer.
- **Web-first in the CURRENT look** (Jesse, Aug 19 2026): the six games shipped
  to the website as they were (neon look, AI stand-ins); Deion re-skins roll
  out to the site as updates. Free itch.io follows the site; Steam/iOS/Android
  wait for Phase J.
- **Tiered AI-asset rule** (Jesse, Aug 19 2026 — supersedes "AI art never
  ships"; **music included** Aug 28 2026): the money line is the boundary.
  AI-generated assets (art AND MIDI/SF2/audio) **may ship in free releases**
  (studio website, free itch.io) as part of the AI-workflow showcase; they
  **never ship in paid/marketplace releases**. Quarantine mechanics: `ai/` dir
  + `ai_` prefix + `check_no_ai_assets.sh` on paid publish paths. SSOT:
  `../games/deion_assets/DEION_STYLE.md` §6.
- **Web assets fetch-by-default**; **WebGPU-only at launch** (WebGL2 fallback
  revisited at the post-launch review, issue #13); **games stay standalone**.
- **Audio backend: rodio, FINAL** (H1 spike + Jesse's listen test, Jul 30
  2026 — `coordination/H1_SPIKE.md`).
- **Asset metrics**: 16px base cell, nearest filter, 5× integer scale to
  `RENDER_UNIT = 80` — one art cell = one world unit = one collider unit.
  Split: Jesse draws hero sheets, key characters, palette sign-off; agents do
  everything else. Pixellab tooling lessons (validated workflows + the
  create_character skeleton trap) are archived in `log_archive.md` § "Roadmap
  slimmed" and DEION_STYLE.md.
- **Perspective cameras permanently rejected**; isometric works via the
  project-and-y-sort pattern (see memory/log archive).

## Phase Map

| Phase | Theme | Status | Where the detail lives |
|-------|-------|--------|------------------------|
| A | Games 1–5 (pong, breakout, invaders, snake, asteroids) | ☑ Jul 2026 | `log_archive.md` |
| B | Engine gaps (CameraFollow, Lifetime, Tilemap) + game 6 Frogger | ☑ Jul 2026 | `log_archive.md` |
| E | Asset pipeline (filter knob, SheetGrid, named clips, `.sheet.ron` — schema freeze Jul 30 2026) | ☑ core; remainders on board | #10 (E7 alpha-cutoff), #11 (E5 `#rgba` error, gated on #69), #67 (E8 inspector wiring) |
| F | Deion style guide + asset production (parallel art track) | F1 ☑ (DEION_STYLE.md); rest on board | #68–#71 (sync `--check`, gen_tiles, placeholder sheets, first animated Deion) |
| G | Re-skin games 1–6 (Tong, Chicken Coop, Food Pyramid, Hot Dog!, Burger Invaders, Meatieroids — identities settled Aug 9 2026) | On board | #72–#79; castings SSOT DEION_STYLE.md §5; per-game README "Deion Pivot" sections |
| H | WASM port (engine + all 6 games on wasm32/WebGPU) | ☑ COMPLETE Aug 27 2026 | Port recipe + web footguns: `log_archive.md`, crate CLAUDE.mds (renderer/engine_core), memory |
| I | Deployment (site live at beinsiculous.com, all 6 games playable) | I1/I2 ☑ Aug 19 2026; rest on board | #15 (itch.io), #16 (Steam checklist), #80 (paid-path purge gate), #13 (WebGL2 review). Site: `../insiculous_web/` (Mily's repo `milyramic`, Astro 5 on Cloudflare Workers; drop-in convention `public/games/<slug>/v1/`) |
| J | Insiculous Arcade — marketplace compilation | OUTLINE ONLY (below) | This file § Phase J |
| K | Conductor — adaptive MIDI music | On board (K1 spike gates all) | #60–#65; architecture: `review/plan-conductor.md`, memory |
| C/D | Games 7–20 | PAUSED until Phase G done (below) | This file § Paused phases |

**Editor** work follows the UX-audit sprint order (its own section below);
**Web Playground** is #48/#49.

## Phase J — Insiculous Arcade (marketplace compilation) — OUTLINE ONLY

First marketplace release (Jesse, Aug 19 2026): **all non-original
20-games-challenge games compiled into one Deion-skinned package** for paid
storefronts (Steam, iOS, Android). Deliberately unplanned — this section exists
so the target is named and its gates are on record.

Hard gates (all must hold before any store submission):
- Phase G complete — every included game fully Deion re-skinned.
- Hand-drawn art swap complete — no AI stand-ins anywhere in the package;
  `check_no_ai_assets.sh` passes on the shipping asset tree (the #80 gate).
- Phase H/I stable — the games have shipped and soaked on the free web tier.

Open questions (unanswered until Phase J planning starts): launcher/wrapper
design (one binary hosting six games vs a hub scene), per-store native
packaging (Steamworks; iOS/Android toolchains are entirely new scope),
input/UX for storefront cert requirements, pricing.

Note: "arcade scaffolding" in engine_core docs (`MenuInput`,
`spawn_background`, …) is unrelated engine vocabulary predating this product
name — leave it.

## Paused Phases C/D — Games 7–20

Resume after Phase G, with Deion styling from day one. No board issues by
design (the board carries actionable work); this table is the resumption point.

| # | Game | Requires | Deion casting | Key new patterns |
|---|------|----------|---------------|------------------|
| 7 | Tetris | Tilemap | TBD (DEION_STYLE.md castings) | Grid logic, piece rotation, line clearing |
| 8 | Galaga | Lifetime, SpriteAnimation | TBD | Formation paths, multi-bullet patterns |
| 9 | Pac-Man | Tilemap, SpriteAnimation | TBD | Pathfinding AI (BFS), ghost modes |
| 10 | Simple Platformer | CameraFollow, SpriteAnimation | **Deion himself — first playable Deion** | Multi-level progression, camera smoothing |
| 11 | Run & Gun | CameraFollow, Tilemap, Lifetime | TBD | Horizontal scroll, checkpoints |
| 12 | Zelda-style Top-Down | CameraFollow, Tilemap, SpriteAnimation | TBD | Room transitions, NPC dialog, items |
| 13 | Tower Defense | Tilemap, SpriteAnimation | TBD | Wave spawning, tower placement, pathfinding |
| 14 | Sokoban / Puzzle | Tilemap | TBD | Move history/undo, editor-compatible levels |
| 15 | Metroidvania | CameraFollow, Tilemap, SpriteAnimation | TBD | Ability gating, persistent world, map |
| 16 | Bullet Hell | — | TBD | High entity counts, pattern scripting |
| 17 | Roguelike Dungeon | — | TBD | Procgen, fog of war, persistent saves |
| 18 | Fighting Game (simple) | — | TBD | Frame-precise animation, hitboxes, combos |
| 19 | Strategy / Mini-RTS | — | TBD | Unit selection, pathfinding at scale |
| 20 | **Original Deion the Insiculous platformer** | full engine + editor + asset pipeline | Deion | The capstone: the founding concept |

## Editor — UX Audit & Work Order (Aug 27 2026)

**Two north stars (Jesse) — weigh every editor decision against these:**
1. **AI-first.** An AI agent must be able to author games through the editor's
   **command layer** (audit §9: query API → write commands → headless `--api`,
   transport-ready from day one). Driving a UI by screenshots and pixel
   coordinates is the worst interface an agent can have.
2. **The editor running in the browser is the super-ultra-win.** The engine
   already runs on wasm32, so **wasm32-compatibility is a standing constraint**
   on every editor decision (why dylib scripting is dropped — no `dlopen`).
   Inherently native-only subsystems (file dialogs, process spawn, `cargo`
   builds, "Open in IDE") are fine as cfg-gated native features with the web
   replacement named up front (OPFS / VFS fetch / deferred remote build).

The full file:line-anchored audit: `docs/EDITOR_UX_AUDIT.md` (2026-08-27). Its
§7 work order is adopted as five sprints; **live items are Studio Board issues
(Phase = Editor)**. Sprints 1–4 complete Aug 27–28 2026; Sprint 5
("architecture": §4.2, §4.3, §6.7, §6.5 Stage 1, §9 Stage C) landed Aug 28 2026
pending close-out. The old "Phase 2 (Ideal Editor UI)" lettering is retired
(history in `log_archive.md`).

**Editor colors**: SSOT is `crates/editor/src/theme.rs` (`EditorTheme` tokens,
WCAG guard tests). The old mockup-derived palette table was dropped from this
file — it was **pending the audit §5.1 gamma verification** (screen colors
measured ~2× brighter than declared tokens); do not derive or re-pick colors
from `crates/editor/IdealEditor.png` until that is settled.

## Web Playground — the learn-to-code front (Aug 27 2026)

The open-source goal: people **learn to code and build their own games with
this engine**, on beinsiculous.com — north star #2 given a shipping
destination. Live items: **#48** (editor-on-wasm milestone: capability split,
boot a bundled sample scene, embed behind the WebGPU gate) and **#49**
(client-side project zip export/import + an org `game-template` repo).
GitHub-App/OAuth publish-to-own-repo is deliberately not designed until the
playground proves engagement.

## Scripting — the ScriptRef seam

Adopted from audit §6.3/§6.5/§6.6(4), Aug 27 2026. Stable serializable identity
for game logic: `ScriptRef { script_id, source_path, params }` +
`Scripts(Vec<ScriptRef>)`, string-keyed (every closed enum lives upstream of
game crates). **Stage 1 — `Scripts` as inert, editor-editable data — SHIPPED
Aug 28 2026 (#44)**; the scene file now carries game-logic bindings. Later
stages: execution via a runtime `ScriptRegistry` + `ScriptBehavior` trait
(`Game::register_scripts`, defaulted), then editor-owned build-and-relaunch
(audit §6.5 Stage 5). Crate placement: data types in `ecs`, runtime in
`engine_core`, catalog via `InspectorExtras`. **dylib hot-reload is dropped**
(TypeId instability across reloads, FFI unwind UB, no `dlopen` on wasm32);
revisit only if build-and-relaunch proves to be the actual bottleneck.

## Technical Debt

On the board: `gh issue list -R beinsiculous/insiculous_2d --label tech-debt`
(games items on their own repos, e.g. `beinsiculous/breakout`). Resolved items
move to `log_archive.md`. The per-crate `TECH_DEBT.md` files were retired
Aug 28 2026 (issues #81–#93).

## Development Guidelines

### For Every Game
1. Standalone cargo project in `../games/<name>/` (sibling to this repo)
2. Depends on `engine_core` (+ `ecs` if needed directly) — no editor dep
3. `README.md` with controls, how to run, patterns demonstrated
4. `cargo run` from the game directory launches it
5. **Deion Style**: sprites from `.sheet.ron` sheets per DEION_STYLE.md
   (post-Phase F); ChaosTheme neon is the accent layer

### AI-Friendly Development
1. **CLI-testable** — all logic testable without GPU/window; `cargo test --workspace` validates everything
2. **No manual testing** — if a feature can't be verified by `cargo test`, it needs a test
3. **Small, focused files** — files over 600 lines get split
4. **Explicit over implicit** — no magic numbers, hidden side effects, clever tricks
5. **Strong typing** — enums over strings, newtypes over primitives
6. **Verify before claiming** — run `cargo test --workspace` before claiming done

### Editor Architecture
1. **Feature-gated** — editor code compiles out without `--features editor`
2. **Design system** — all colors/spacing from `EditorTheme`, never hardcoded
3. **Command pattern** — all operations undoable
4. **Live editing** — property changes visible immediately

## Quick Reference

```bash
cargo test --workspace                                   # all engine tests
cargo run --example hello_world                          # engine example
cargo run --bin editor --features editor -- ../games/pong  # editor on a project
cd ../games/pong && cargo run                            # a game directly
gh issue list -R beinsiculous/insiculous_2d              # the open work
```

**Key files:** `CLAUDE.md` (agent ruleset + engine status; `AGENTS.md`
symlinks to it) · `training.md` (API patterns) · `log_archive.md` (completed
history) · `docs/EDITOR_UX_AUDIT.md` · `docs/EDITOR_COMMAND_API.md` ·
`docs/WEB_SAVES.md` · `coordination/H1_SPIKE.md` ·
`../games/deion_assets/DEION_STYLE.md` (style + castings + tiered AI rule) ·
`../games/` (game projects) · `../insiculous_web/` (the site, Mily's repo).
