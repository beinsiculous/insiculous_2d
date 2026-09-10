# Playground UX — the editor gets the window, and an edit-to-preview loop

Effort directory: `coordination/playground-ux/` (this plan; the reviewer ledger
`reviewer-comparison.md` beside it). Review conversations: one subject per repo, because
each repo's `request-review.sh` refuses an artifact outside its own `review/` and its read
fence is that repo — `insiculous_2d/review/playground-ux/` for the plan, the engine diffs
and every handoff, `insiculous_web/review/playground-ux/` for the site diffs. Delivery runs
the handoff loop: Claude plans and commits, kimi and codex (Astra) review every plan version
and every batch's diff, a Claude Opus session driven by Jesse executes one batch per handoff.

Plan history: v1 reviewed by kimi (`review-1.md`, 11 findings) and codex (`review-1-codex.md`,
10 findings) on 2026-09-09, adjudicated in `rebuttal-1.md` — all 21 accepted: the Stop
dialog commits an open API batch into the session, replays only the undo stack's session
entries and purges the redo stack's, seals merging on undo and redo, suspends eviction for
the session and replays the fields the user changed over the authored values; the preview
reservation is taken when the snapshot request is filed, every launch carries a generation
token, the preview acknowledges loaded or failed and heartbeats, the window name is per
tab, the snapshot names script targets in the serialized copy only, and Export carries the
live scene with a Save button beside it. v2 reviewed by kimi (`review-2.md`, 10) and codex
(`review-2-codex.md`, 7), adjudicated in `rebuttal-2.md` — all 17 accepted: Play cancels a
pending stop dialog and the dialog swallows every key, the API `stop` verb reports a parked
dialog, a macro rebases child by child, the field diff is recursive; the heartbeat is
dropped and the reservation releases only on confirmed closure or failure, a blocked popup
files nothing, one finalizer ends every launch, `preview-loaded` waits for the first
running frame through a preview-state export, the hidden-tab timer fallback moves into
batch 4, the snapshot serializes a named scratch world, Export keeps every other file and
the preview loads an explicit scene entry; later batches gain the mutation-path list, the
prefs migration rule, the combined save signal and a `view_toggles.rs`. v3 reviewed by
kimi (`review-3.md`, 7) and codex (`review-3-codex.md`, 7), adjudicated in `rebuttal-3.md`
— all 14 accepted: equal-length arrays diff by index (a `Vec2` is an array), the dialog
cancel also sits on resume, a boot timeout closes the window before releasing, the closed
poll lasts the window's lifetime, the snapshot resolves an envelope with its scene entry,
the tab identity lives in `window.name`, a reloaded editor re-takes a live preview's
reservation, `preview-ready` is ignored outside a launch, Export waits behind a pending
snapshot, the drag-focus listener fires only over the canvas, the play controls lay out
from the strip's right edge and never clip, Try again stays live while the wasm init has
not resolved, and the final rebuild covers all seven bundles. v4 reviewed by codex only
(`review-4-codex.md`, 7), adjudicated in `rebuttal-4.md` — all 7 accepted: the hidden-tab
frame is driven by a proxy user event rather than an animation frame, one shared answer
per generation serves both the preview and Export, the dock auto-collapses side panels to
keep a minimum centre width, a structural command whose precondition fails after restore
is dropped and counted, the recovery key carries its owning tab id, and the batch-4 doc
lines match the envelope. v5 reviewed by codex only (`review-5-codex.md`, 4), adjudicated
in `rebuttal-5.md` — all 4 accepted and applied in place, as Jesse ruled before the round:
a snapshot completion is held by every subscriber before polling and never erased by a
launch's cleanup, a paused creation the simulation later destroyed is dropped and counted,
the dock's narrow mode shows one side panel as an overlay instead of collapsing it
unreachably, and a paused edit referencing a runtime-only entity is dropped and counted.
**This is the settled plan** (2026-09-09, five rounds: 21 + 17 + 14 + 7 + 4 findings, all
accepted). Corrections from here go into the acting batch's section before its handoff.

## Context

The **Playground UX** sprint (milestone in `insiculous_2d` and `insiculous_web`, one order
in its description, the board's Sprint field across both) is Astra's two review batches of
`/playground/` (2026-09-08), filed as 22 issues the week the Web Playground shipped. The
acceptance test is the milestone's own: on an ordinary laptop, a new visitor changes a
sample, plays that exact change in another window, returns safely, and exports their work
without reading the documentation.

The milestone's order, in six tiers:

| tier | issues | gate |
|---|---|---|
| 1 foundations | 2d#103 paused edits vs Stop; 2d#128 inspector heading; 2d#130 asset browser; 2d#131 toolbar strip; 2d#119 wheel notch; 2d#120 drag into the canvas | #103 gates #121 |
| 2 shell + preview loop | web#57 application shell (web#52 fullscreen and pop-out inside it); 2d#121 game-only preview from a snapshot; web#58 preview window | #121 gates web#58 |
| 3 first run | 2d#122 starter scene, hint, console demoted; web#51 game page link | — |
| 4 styling | 2d#123 audit reconciliation first; 2d#129 inspector stable in Play; 2d#133 inspector sections + colour editor; 2d#132 quieter overlays; 2d#124 tooltips, resizing, Reset Layout | #123 before the rest |
| 5 states | 2d#125 save-state export gates web#60 save status; web#59 compatibility panel | — |
| 6 criteria | 2d#126 browser usability + in-editor a11y; 2d#127 performance budget | before the sprint's deploy |

What the exploration established (2026-09-09, tree at 72fccf6 engine / bbc95df site):

Engine:
- Every edit guard tests `is_playing()`, so Paused is editable by construction; Stop's
  `restore_snapshot` (`editor_game/play_session.rs:116`) never consults the history. The
  hierarchy drop has no play guard at all (`crates/editor/src/hierarchy/mod.rs:410`), but a
  drag can only be armed while not Playing and `start_play_session` resets it.
  `CommandHistory` (`crates/editor/src/commands/mod.rs`, 378) has an id watermark
  (`is_dirty` = `top_id != saved_id`; `top_id` is private) and no truncate API;
  `EditorCommand::execute` returns `()` and a missing entity is a silent no-op;
  `CreateEntityCommand` recorded via `push_already_executed` keeps `captured == true`, so
  re-executing it recreates nothing. `ConfirmDialog` (Confirm/Alt/Cancel on the Modal layer)
  has one consumer, `editor_game/scene_confirm.rs`, the pattern to copy. Near the ceiling:
  `editor_game/mod.rs` 544, `context/mod.rs` 556.
- The inspector heading is `Selection::inspector_heading()` (`selection.rs:114`, `Entity: N`,
  no name) drawn with `label_with_font` at the content top
  (`panel_renderer/inspector.rs:65-80`) — the "UI text y = baseline" footgun is the likely
  clip. The hierarchy's name rule is `HierarchyPanel::entity_display_name`
  (`hierarchy/mod.rs:180`).
- The inspector's Play path is `inspect_all_components` (a serde dump: no width, no
  scroll_to) against `edit_all_components` (`stored_component/mod.rs:270/311`, both
  generated by the registry macro at `:441`); one `ScrollState` per panel.
- Asset browser: a press arms a drag AND a motionless release assigns
  (`panel_renderer/asset_browser.rs:240-289`; `assign_clicked_texture` at 290); labels are
  raw `label_in_bounds_styled` with no truncation; `row_layout::ellipsize` exists (`:123`)
  with two consumers; no tooltip widget exists anywhere (`UiLayer::Tooltip` is an empty band).
- Toolbar: `toolbar_position_for` (`crates/editor/src/toolbar.rs:186`) floats inside the
  scene view; the toolbar (56 px buttons) and the play controls (40 px) draw on the Content
  band, and so do the overlays (grid, colliders, selection, play border) in
  `panel_renderer/mod.rs:51-140`. No strip is reserved in `DockArea::layout`. View toggles
  are scattered (grid on `GridRenderer`, `show_colliders` and `snap_to_grid` on
  `EditorContext`); no camera-bounds overlay exists; the View menu lists Grid, Colliders
  and Snap only.
- Input: `SCROLL_PIXELS_PER_LINE = 16.0` (`input_handler.rs:43`); `ScrollState::WHEEL_STEP =
  30.0` per notch; the viewport reads `input_state.scroll_delta` and clamps to one notch per
  frame; `focus_before_winit_does` (`renderer/src/window.rs:165`) handles `pointerdown` only.
- Bridge (`crates/playground/src/bridge.rs`, 380): 14 exports, no play, preview, snapshot or
  save-state verb, no push channel (the page polls every 100 ms). The bridge cannot reach
  the editor's World synchronously — it lives in the browser-owned `GameRunner`; the
  `script_errors` mirror (an `Arc` shared through `EditorRunOptions` and `bridge::Hooks`)
  is the model for anything the page asks of the editor. `persist::Chains` already exposes
  `path_state`, `is_pending` and `has_active` on the Rust type; only `conflicted_paths`
  reaches JS. `web_entry::start()` is the one entry and always boots the editor; no
  game-only wasm entry exists. `run_game` on wasm returns right after spawning the app;
  two event loops in one page are impossible (winit refuses a second; the canvas, VFS,
  bridge and chains are singletons). `playground_export_zip` carries the scene as last
  saved — unsaved edits live only in the World. `import_project` validates slug, paths and
  dry-runs the scene without a store; `vfs::insert` seeds MemFs without notifying persist;
  `first_scene_in` takes the sorted first `.ron`; the `scene` query already reports
  `play_state`.
- Theme (`crates/editor/src/theme/mod.rs`, 417) carries grid, axis, collider and selection
  tokens; no focus-ring token. The wasm frame loop re-arms `request_redraw`
  unconditionally (`game/app_handler.rs:22-50`); `throttle()` is a no-op on wasm.
  `behavior_demo.scene.ron` has 16 entities, 5 with no `name` key (four walls, one
  obstacle). `docs/EDITOR_UX_AUDIT.md` is 835 lines, sections 0–10.

Site:
- `/playground/` is `BaseLayout` + `PlaygroundEmbed` (527 lines) + four prose sections; the
  canvas is 1280×800 inside `main.container` (68rem, about 1048 px usable). No Play button
  and no scene Save button exist in the DOM (Play is Ctrl+P/F5 into the canvas).
  `/playground/<slug>/` is `BaseLayout` + `GameEmbed` with a clamped 1024×720 canvas. No
  chrome-less layout, no `/full/` route. The engine sizes its surface from the canvas's
  client box on every `Resized`, sets no `min_inner_size`, and winit writes the canvas's
  inline `width`/`height` once at creation — a CSS box narrower than 1024 px is accepted,
  not enlarged.
- `playground-embed.ts` (378): the WebGPU gate at 60-69 writes text into `#game-loading`,
  duplicated verbatim in `GameEmbed.astro:90-97`; four `window.confirm`s; every failure
  writes `banner.textContent`; no `window.open`, no `requestFullscreen`, no `aria-live`
  writes. No play verb exists in the command API (`docs/EDITOR_COMMAND_API.md:182`).
- Gates: `postbuild-check` (one `<h1>`, no duplicate ids per file, `data-wasm-src` resolves
  verbatim, `playgroundProject` against `projects.json` at a hardcoded `v1` path — bump it
  with the embed default); `a11y-check` and `announce-check` walk `dist/` with no
  playground scenario in `OPENED_ELEMENT_ROUTES`; `screenshot-pages` fails any route that
  scrolls sideways at 1440, 390 and 641 px and, under `LARGE_TEXT=1`, 390 at 125% text and
  320 reflow. **The gates never see the engine running**: the gate's Chromium has
  `navigator.gpu` but no adapter, so every audit lands on the WebGPU-failure state with the
  controls disabled; anything that exists only after `playground-ready` is a manual check.
- Bundles: `public/playground/v1/` (10.2 MB wasm) plus six game editor bundles at `v1`
  (9.1–9.8 MB). A version dir is immutable once deployed; the sprint's rebuilds go to `v2`.
- The PR template's manual pass: a keyboard walkthrough, one VoiceOver or NVDA listen per
  new interaction, 200% text on a phone.

## Decisions of record (taken with Jesse, 2026-09-09)

- **Branches.** `jesse` in both repos (equal to `dev` at the start); every batch commits on
  `jesse`, one merge into `dev` at the end; `main` only receives merges. Deploys are
  Jesse's push.
- **Executor:** a separate Claude Opus session in another tab, driven by Jesse from
  `review/playground-ux/handoff-<batch>.md`. Gemini's quota is spent, so it neither builds
  nor reviews the plan; the executor's vendor is the planner's, so the roles rule does not
  require its plan review. Plan reviewers: **kimi + codex** (every batch has a screen in
  it). Gemini joins in code mode when its quota returns and a batch changes a harness, a
  fixture or a public seam (the bridge in batches 4 and 11).
- **2d#103 mechanism:** Stop warns and offers to keep. Paused stays editable; when the
  history holds edits recorded since Play, Stop asks on the Modal layer — Keep, Discard,
  Cancel — and Keep rebases those edits onto the restored world.
- **Scope:** the whole sprint, tiered batches in the milestone's order, one plan review
  round before batch 1; later corrections land in the acting batch's section before its
  handoff, never in a side section.
- **Proposed, for the review to attack:** the application shell fills the window on the
  canonical URL, so the separate `/playground/full/` route is not built and **web#52
  closes with web#57** on the Fullscreen button alone; and there is **no site-side Play
  button** — no play verb exists, synthesising a key event is out, the editor's own Play
  sits in the new opaque strip, and the page's `Play ↗` opens the preview window.
- **One simulation during a detached preview** is enforced by the engine: the page sets a
  `preview_open` flag through the bridge while the window lives, and the editor refuses its
  own Play with a status-bar line while it is set; the snapshot itself is refused during a
  play session.
- **The snapshot is a one-shot request answered on the editor's next frame** (the bridge
  cannot reach the World), exported as a `Promise`; the preview is the same wasm bundle
  booted with `?mode=preview` on the page URL, which skips the store, the chains, the
  observer and the listeners and runs `ProjectHost` without the editor wrapper.
- **Keep preserves the fields the user changed** (round 1, codex F2): a paused edit
  replays as the field-level difference between its before-image and its after-image,
  patched onto the authored component; every field the user did not touch keeps its
  authored value, and a nudge replays its delta. Keep replays only what is applied at Stop:
  a session entry the user undid is purged, never replayed (kimi F2, codex F3).
- **The preview launch is a protocol with a reservation and a generation** (rounds 1 and
  2): filing the snapshot request reserves the preview (the editor's own Play is refused
  from the click, not from the handshake), and a blocked popup files nothing; every launch
  carries a generation token on the request, the answer and every message; one launch is
  in flight at a time and one finalizer ends it; the preview page acknowledges
  `preview-loaded` only once its runtime reports a running frame, or `preview-failed`; the
  window name is per tab. **The reservation is released only on confirmed closure or a
  reported failure** — never on silence, because a hidden window's timers are throttled
  and silence cannot tell dead from asleep; a crashed preview is recovered by closing its
  window, and the banner says so.
- **Export carries the live scene and every other file** (codex F8, then codex F1 of
  round 2): the acceptance test is "exports their work", so `playground_export_zip`
  becomes the live-snapshot export that replaces only the active scene's entry, and the
  toolbar gains a Save button through the hosted `save` verb. The preview loads an
  explicit scene entry the opener names, never "the first scene it finds".

## Ground rules for every batch

- Gates, engine: `cargo test --workspace` (0 failed, 0 ignored), `cargo clippy --workspace
  --all-targets` (0 warnings), `cargo clippy --features editor --all-targets` at the root
  (the only gate that compiles `src/bin/editor.rs`), every touched file ≤ 600 lines, no new
  `#[allow]`, no `unwrap()` outside tests, **no new dependency this sprint**. The comment-tag
  gate on every batch:
  `grep -riEn "kimi|codex|astra|gemini|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|#[0-9]{1,4}\b|Sprint [0-9]" crates src examples --include=*.rs`
  prints nothing (a hex literal or a string match is inspected by hand). `/finish-task` is
  the checklist.
- Wasm gate `scripts/check_wasm.sh` on every engine batch (every one touches a crate it
  covers). Games gate `scripts/check_games.sh` whenever a public item of `engine_core`,
  `ecs`, `physics`, `input`, `common` or `renderer` changes; `--test` when behaviour they
  exercise changes. Verify `../games` resolves to the working set first.
- Gates, site: `npm run verify` under Node 24 (`nvm use 24` on Iroh) — validate, data
  tests, `astro check`, build + postbuild, axe, announce, screenshots at every width. New
  routes are gated automatically because the checkers walk `dist/`. After a layout or an
  interactive change, the PR template's manual pass is Jesse's.
- Bundle gate whenever a wasm bundle is rebuilt: `scripts/build_wasm.sh` must not warn past
  20 MiB; the rebuilt version dir lands in `public/` in the same diff that moves
  `data-wasm-src`, and a version dir is never overwritten once deployed.
- Review: the commit hook denies unreviewed commits over 100 changed lines. Every code batch
  goes `git diff --cached > review/playground-ux/draft-<batch>.diff` in the repo it
  touches, kimi review (detached when the diff is large), codex review (with a screenshot
  when Jesse has one), the planner's own review, adjudication with Jesse, `rebuttal-N.md`,
  fixes applied by the planner, then `ADV_REVIEWED=1 git commit -F <message-file>
  --pathspec-from-file=<scope-file>`. Never skip trailers.
- Every commit is pathspec-scoped. The plan's "done" marks are their own commits
  (`-- coordination/playground-ux/plan.md`).
- Before each handoff the planner re-verifies the batch section against the tree and
  commits corrections into the section. One batch out at a time; the planner neither
  edits nor runs cargo or npm in that checkout while a batch is out.
- **Browser checks are Jesse's.** Agents cannot see a headed WebGPU browser. Each batch
  lists the exact check and is not marked done until Jesse reports it.
- Docs match reality at every commit: a guide describing a thing a batch changed is a
  defect in that batch.
- Anything deferred is filed with `/file-issue` before the effort reports done.

## Batch 0 — planner only: branches, directories, the plan review, the claims

1. Both repos on `jesse` (done 2026-09-09; each equal to `dev`).
2. This directory and `review/playground-ux/` in both repos (done).
3. Plan review round 1 from inside `insiculous_2d`: kimi on `review/playground-ux/plan.md`,
   codex on the same snapshot as `review-1-codex.md`; adjudicate with Jesse, one
   `rebuttal-1.md`, revise as `plan-v2.md`, repeat until Jesse calls it settled.
4. Commit this plan and the ledger on `jesse` with the plan review asserted, pathspec
   `coordination/playground-ux/`.
5. Claim each issue (assign Jesse, comment the batch and this path) as its batch goes out.

## Batch 1 — engine: Stop keeps or discards paused edits; the inspector heading (2d#103, 2d#128) — DONE 2026-09-09 (f268bf4)

Authored by Jesse's Claude Opus session from `review/playground-ux/handoff-1.md`; reviewed
by kimi (`review-4.md`, 4 findings: 3 accepted, 1 policy rebut — post-session eviction is
the cap doing its job), codex (`review-6-codex.md`, 5 accepted) and the planner
(`review-4-claude.md`, 2 accepted, 2 observations); adjudicated in `rebuttal-4-code.md`.
The planner's fix hunks (11 files) went back through kimi (`review-5.md`, 4) and codex
(`review-7-codex.md`, 3) as their own diff; those seven corrections were applied and
enumerated, not re-reviewed (Jesse's ruling). Landed as specified, plus: `rebase_onto`
takes `&mut World` so a macro can rebase child by child (the executor's deviation,
accepted); `Rebase::Drop` carries an entry count and `Rebase::Partial` a macro's dropped
children; `StopOutcome { kept, dropped }` feeds the status line; the cut re-captures its
subtree at execute; a kept creation replays its creation-time components (reversing this
plan's round-5 ruling, with Jesse); a one-key object whose key changed is replaced whole;
API writes are refused under either dialog; the heading keeps its bold face through
`ui::UIContext::label_in_bounds_with_font`. Every gate green (`gates-1-final.log`); the six
games pass `check_games.sh` (`gates-1-games.log`; `game-template` is not cloned in this
working set). Jesse's browser check — the heading on a named, an unnamed and a
multi-selected entity; the Keep/Discard/Cancel dialog after an edit while Paused — is
owed on the next playground bundle.

**Re-verified against the tree 2026-09-09 before the handoff** (after the plan commit
f193c04): `SetComponentCommand`, `RenameEntityCommand` and `NudgeCommand` live in
`commands/set_commands.rs` (`:23`, `:105`, `:160`); `AddComponentCommand` and
`SetComponentValueCommand` in `commands/component_commands.rs` (`:16`, `:147`);
`CreateEntityCommand`, `DeleteEntityCommand` and `MacroCommand` in
`commands/entity_commands.rs` (`:22`, `:76`, `:147`); **`SpawnTreeCommand` is in
`crates/editor/src/clipboard.rs:164`**, not under `commands/`. `Selection::inspector_heading`
is at `selection.rs:116`; `render_early_overlays` at `editor_game/mod.rs:317`;
`confirm_dialog_consumes_key` at `scene_confirm.rs:126`; `undo_with_feedback` and
`redo_with_feedback` at `shortcuts.rs:375` and `:385`; the API's `undo`/`redo` at
`command_api/write/verbs.rs:231` and `:244`. **There is no API `stop` verb** — the pure
writes are set, add, remove, rename, delete, select, undo, redo and the three batch
verbs (`write/mod.rs:227-237`), the hosted ones create and save — so the "stop pending"
response below applies only if a later batch adds one; the test asserts instead that the
API cannot reach `stop_play_session` at all. `docs/EDITOR_COMMAND_API.md:68` states the
rule this batch changes ("Stop DISCARDS a batch opened while Paused") and joins the docs
list. `editor_game/test_support.rs` and `headless.rs` exist for the tests.

Files: `crates/editor/src/commands/mod.rs` (378 → ~475; a new `session_tests.rs` beside
it), the command files that carry a before-image (`commands/set_commands.rs`,
`commands/entity_commands.rs`, `commands/component_commands.rs`, and `clipboard.rs` for
`SpawnTreeCommand`), `crates/editor/src/confirm_dialog.rs` (166),
`crates/editor_integration/src/editor_game/{play_session.rs (221), scene_confirm.rs (143),
shortcuts.rs (434), mod.rs (544)}`, new `editor_game/stop_confirm.rs` and
`stop_confirm_tests.rs`, `crates/editor/src/command_api/write/verbs.rs`,
`crates/editor/src/selection.rs` (239), `crates/editor/src/hierarchy/mod.rs` (564),
`crates/editor_integration/src/panel_renderer/inspector.rs` (340).

**2d#103 — target shapes.**

- The Play boundary is a **stack position and an id floor taken together at Play**.
  `CommandHistory` gains `session: Option<Session { start_len: usize, floor_id: u64 }>`
  and: `begin_session()` (= `break_merge()`, then `start_len = undo_stack.len()` and
  `floor_id = next_id`), `end_session()`, `in_session() -> bool`,
  `session_entry_count() -> usize` (= `undo_stack.len() - start_len`),
  `drop_session_entries() -> usize` (truncates the undo stack to `start_len` and purges
  every redo entry with id ≥ `floor_id`), and
  `rebase_session_entries(&mut self, world: &mut World, replace: impl FnOnce(&mut World)) -> usize`.
  The top id is the wrong boundary (an undone pre-session entry sits in the redo stack with
  an id above it), and an id comparison alone is not enough either: `try_merge_or_push`
  reassigns a merged entry's id (`:335`), so **`undo` and `redo` now seal merging**
  (`merge_sealed = true`, like `break_merge`) — a command issued after an undo starts a
  fresh entry instead of merging into the pre-session top and dragging its id above the
  floor. Membership is the position; the floor id only names the redo entries to purge.
  **Eviction is suspended for the session**: `enforce_limit` never pops below `start_len`
  while a session is open (a 101st paused edit must not evict the paused create it
  depends on).
- **Re-executing a create is a no-op after restore** (`CreateEntityCommand` recorded via
  `push_already_executed` keeps `captured == true`; `SpawnTreeCommand::execute` returns
  while alive), so Keep is the history's own redo path: `rebase_session_entries` undoes the
  undo stack's session entries newest-first on the paused world, runs `replace` (the
  snapshot restore), calls `rebase_onto` (below) and re-executes each oldest-first — ids
  are stable across restore (`create_entity_with_id`), so the entries resurrect their
  entities under their original ids. **Session entries on the redo stack are purged, never
  replayed**: Keep keeps what was applied at Stop, and an edit the user undid stays undone.
- **A paused edit's images are simulated values.** Its before-image (`SetComponentCommand
  .old`, `NudgeCommand`'s old positions, `RenameEntityCommand.old`,
  `SetComponentValueCommand.old`) would make an Undo after Keep write a simulated pose into
  the authored scene, and its after-image carries every field the simulation moved, not
  only the one the user edited (a paused rotation edit's `new` also holds the simulated
  position). The rule: **Keep preserves the fields the user changed, at their edited
  values, over the authored values for everything else.** New trait method with a default
  no-op, `EditorCommand::rebase_onto(&mut self, _world: &World) {}`, called on the restored
  world right before each re-execute: `SetComponentCommand<T>` and `SetComponentValueCommand`
  compute the changed fields as the **leaf-level** JSON diff of `old` against `new`
  (`serde_json::Value` objects recurse, and **equal-length arrays recurse by index** — a
  `glam::Vec2` serializes as `[x, y]`, so a paused edit of X alone must not carry the
  simulated Y; unequal-length arrays and scalars are leaves — so a sibling leaf the
  simulation moved, inside a nested object or a vector, is never patched), patch those leaves onto the
  authored component read from `world`, and set `new` to the patched value and `old` to
  the authored one; `NudgeCommand` reads the authored positions and replays its delta;
  `RenameEntityCommand` re-reads `old`; **`MacroCommand` rebases and executes each child in
  turn** against the result of the previous child (nested macros included) — rebasing
  every child first against the authored value would let the second whole-component write
  erase the first edit, and a create-then-edit batch would rebase the edit before its
  target exists; `AddComponentCommand` drops its capture so redo re-adds the default.
  Delete, Remove and DeleteTree re-capture in `execute` already; Create and SpawnTree need
  nothing. The same rebase-then-execute-per-entry order is what `rebase_session_entries`
  itself follows. **`rebase_onto` returns `Rebase::Apply | Rebase::Drop`**: a structural
  command whose precondition fails on the restored world — a delete of an entity that no
  longer exists, an add of a component the authored entity already has, a remove of one
  it lacks, any edit of an absent entity — reports `Drop`, and a dropped entry leaves the
  retained history (a macro prunes its dropped children and drops itself when empty).
  Keeping such an entry would let its later Undo create a phantom entity
  (`DeleteEntityCommand::undo` calls `create_entity_with_id` unconditionally) or strip an
  authored component that the add had overwritten without a before-image. Two more cases
  report `Drop`: **a Create or SpawnTree whose entity no longer exists at Stop** (a paused
  creation the simulation destroyed after a Resume — `CreateEntityCommand::undo`
  re-captures from the entity, so replaying it would resurrect an empty phantom; one that
  still exists keeps its Stop-time state), and **a rebased `Scripts` component whose
  `ScriptValue::Entity` reference points at an entity that will not exist** in the rebased
  world (kept paused creations count as present) — retaining it would let
  `scripts_to_data` silently lose the binding on preview and export. The status line
  reads "Kept N paused edit(s); M could not be applied".
- Undo and redo inside a session **stop at the boundary** rather than being refused
  outright: `can_undo`/`undo` refuse when `undo_stack.len() == start_len`, `can_redo`/`redo`
  when the redo top's id is below the floor — one place, so the GUI and the command API
  agree. `shortcuts.rs`'s `undo_with_feedback`/`redo_with_feedback` and the API's `undo`/
  `redo` verbs say "Undo stops at the Play boundary — Stop first" when blocked inside a
  session.
- `play_session.rs`: `start_play_session` calls `command_history.begin_session()` right
  after `commit_open_api_batch()` (the batch commit must land before the boundary is
  taken). New `pub(super) enum PausedEdits { Keep, Discard }`;
  `restore_snapshot(world, paused_edits)`: Discard → `snapshot.restore(world)`; Keep →
  `command_history.rebase_session_entries(world, |w| snapshot.restore(w))`; then the
  transform reset and the drop report as today. `stop_play_session(world, paused_edits)`
  becomes **private to the confirm flow** (only `request_stop` and the dialog call it —
  the report shows the grep of every former caller, and any other Stop path — a menu
  item, the API, an Escape cascade, batch 2's strip button — routes through `request_stop`):
  the guard, then **`commit_open_api_batch()`** — an API batch open at Stop was applied to
  the paused world and held outside the history (`api.rs:29-32`), so it is committed into
  the session first and counted by the dialog; Keep replays it and Discard drops it with
  the rest (v1's "stays discarded" was silent data loss on Keep) — then
  `drop_session_entries()` on Discard, `restore_snapshot`, `end_session()`, the rest
  unchanged. `handle_play_action`'s Stop arm calls `request_stop`.
- New `editor_game/stop_confirm.rs` (~110, modelled on `scene_confirm.rs`):
  `#[derive(Default)] pub(super) struct StopConfirm { pub pending: bool, pub pending_choice: Option<ConfirmChoice> }`;
  `EditorGame::request_stop(&mut self, world) -> bool` — not in a session → false; the
  open API batch committed into the session; no session entries →
  `stop_play_session(world, Discard)` as today; else pause if Playing (a modal over a
  running simulation is wrong, and it makes Cancel = "stay Paused" true in both cases),
  `pending = true`, return false. `render_stop_confirm_dialog(ctx)` renders
  `ConfirmDialog::keep_paused_edits(n)` (title "Edits made while paused", message "Keep
  the N edit(s) you made while paused?", buttons Keep / Discard / Cancel) right after the
  scene dialog in `render_early_overlays`; Confirm → Keep, Alt → Discard, Cancel →
  `pending = false` and "Cancelled — still paused"; Keep and Discard notify the inner game
  (`on_play_stopped`) and report "Kept N paused edit(s)" / "Discarded N paused edit(s)".
  `scene_confirm.rs`'s `confirm_dialog_consumes_key` checks both dialogs and **swallows
  every key while either is pending** (its contract, extended): Escape clears whichever is
  up, Enter queues Confirm on whichever is up, anything else is consumed. **Entering Play
  cancels a pending stop dialog on both transitions**: `start_play_session`'s defensive
  drop (`play_session.rs:23-25`) clears `stop_confirm.pending` and `pending_choice` beside
  `scene_confirm.pending_action`, and **`resume_from_pause` does the same** — the dialog
  parks the session Paused, and a Play from Paused resumes rather than starts, so a cancel
  only on the start path would let the simulation resume under a live modal and a queued
  Keep restore a session the user had resumed. No API verb stops a session today; if one
  is ever added it answers `{"stop": "pending", "reason": "answer the Keep/Discard
  dialog"}` when the dialog parks instead of claiming success. Entries targeting an entity
  that existed only during Play are dropped by the rebase and counted on the status line,
  never kept as no-ops (an undoable no-op resurrects phantoms); an entity created while
  paused and then simulated after a Resume comes back in its Stop-time state
  (`CreateEntityCommand::undo` re-captures).
- `editor_game/mod.rs` → ~551: `mod stop_confirm;`, the field, its init, one render call.
  Nothing goes on `EditorContext`.

**2d#128 — target shapes.** `entity_display_name` leaves `HierarchyPanel` for one shared
free function in `crates/editor/src/entity_names.rs` (`pub fn entity_display_name(world,
entity) -> String`, the same fallbacks: `Name` → "Sprite (Entity N)" → "RigidBody (Entity
N)" → "Entity N"); both panels call it. `Selection::inspector_heading` becomes
`inspector_heading(&self, display_name: &str) -> Option<(String, String)>`: the heading
line is the display name, the detail line is "Entity 17" or "3 selected · Entity 17
primary". `panel_renderer/inspector.rs` draws the heading with `label_in_bounds_styled` in
a rect of the heading font's line height starting at the content top (the baseline
footgun is why it clipped), then the detail in `text_muted` and the small font, then the
rows. Test: heading and detail for a named, an unnamed and a multi-selected entity.

Tests: `crates/editor/src/commands/session_tests.rs` —
`test_session_floor_blocks_undo_below_the_play_boundary_and_redo_of_pre_session_entries`
(also: a `try_merge_or_push` right after `begin_session` starts a fresh entry),
`test_drop_session_entries_removes_only_the_entries_recorded_after_the_floor` (including a
session entry in the redo stack; `is_dirty()` reads as before the session),
`test_rebase_session_entries_replays_creates_and_edits_onto_the_replaced_world_with_authored_before_images`
(pre-session Set; session: an already-created entity and a Set whose `old` is a simulated
value; rebase with a `replace` that resets the pre-session value and removes the created
entity → the entity is alive under the same id, the edit holds, undo returns the authored
value, undo again removes the entity),
`test_rebase_replays_only_the_changed_fields_over_the_authored_component` (authored
(0,0) rotation 0; the paused edit's `old` is the simulated (100,50) rotation 0 and its
`new` (100,50) rotation 90 → after rebase the component is (0,0) rotation 90, and undo
returns (0,0) rotation 0; a nudge of +1 on the simulated position lands at authored +1),
`test_rebase_purges_undone_session_entries_instead_of_replaying_them` (pause → edit →
undo → rebase → the edit is absent and the redo stack holds no session entry),
`test_undo_and_redo_seal_merging_so_a_session_command_cannot_merge_into_a_pre_session_entry`,
`test_eviction_spares_the_session_entries` (101 paused edits after a paused create; the
create survives; after `end_session` the limit applies again).
`editor_integration/src/editor_game/stop_confirm_tests.rs` (fixtures from
`test_support.rs`) — `test_stop_without_paused_edits_restores_immediately`,
`test_stop_with_paused_edits_freezes_the_game_and_parks_the_dialog` (also via
Pause → edit → Resume → Stop),
`test_keep_applies_the_paused_edit_to_the_restored_world_and_undo_returns_to_the_authored_value`,
`test_keep_replays_an_open_api_batch_made_while_paused` (`batch begin`, two `set` lines,
Stop → the dialog counts them, Keep applies them, Discard drops them),
`test_discard_restores_the_snapshot_and_truncates_the_history_to_the_play_boundary`,
`test_cancel_keeps_the_session_paused_with_its_edits` (via Escape),
`test_undo_while_paused_stops_at_the_play_boundary` (the GUI path and the API line),
`test_every_stop_path_routes_through_the_dialog` (the play-control action and the
shortcut each reach `request_stop`, and no API line can reach `stop_play_session`), `test_play_while_the_stop_dialog_is_pending_cancels_the_dialog` (driven
through the real Paused → Playing dispatcher path, which resumes; the session stays intact
and every other key is swallowed meanwhile),
`test_rebase_keeps_the_authored_y_when_only_x_was_edited_while_paused` (authored (0,0),
simulated (100,50), X edited to 120 → Keep yields (120,0)),
`test_rebase_drops_a_paused_delete_of_a_runtime_only_entity_so_undo_creates_no_phantom`
(Keep, then Undo and Redo: no empty entity appears, the count says one could not be
applied), `test_rebase_drops_a_paused_add_over_an_authored_component` (the authored
Collider survives Keep and Undo),
`test_rebase_drops_a_paused_creation_the_simulation_destroyed` (create while paused,
resume until a script destroys it, Stop → Keep: no entity, one counted),
`test_rebase_drops_a_script_reference_to_a_runtime_only_entity` (the authored parameter
stands, one counted),
`test_keep_rebases_a_macro_child_by_child` (one batch sets rotation then scale on the
same component → both hold; a create-then-edit batch → the edit lands on the created
entity), `test_rebase_leaves_a_simulated_nested_sibling_leaf_alone`. The header comment of
`play_session_tests.rs::test_play_cancels_an_in_flight_asset_drag` documents the old
"discarded by Stop" rule and is updated.

Docs: `docs/EDITOR_COMMAND_API.md:68` ("Stop DISCARDS a batch opened while Paused")
becomes "Stop commits a batch opened while Paused into the session and the Keep/Discard
dialog decides its fate with the other paused edits". `crates/editor_integration/CLAUDE.md`
— a File Map row for `stop_confirm.rs`; the Play/Stop pattern line gains "Stop with edits recorded since Play (only possible while
Paused) asks Keep / Discard / Cancel on the Modal layer — Keep rebases them onto the
restored world, Discard truncates to the Play boundary, Cancel stays Paused; undo and redo
inside a session stop at that boundary; edits to entities that existed only during Play do
not survive Keep"; pitfall rows "Edits made while Paused must never be silently erased by
Stop" and "Undo inside a play session must not cross the Play boundary" naming their
tests. `crates/editor/CLAUDE.md` — "`Paused` → editable; `CommandHistory::begin_session`
marks the Play boundary the editor's Stop dialog acts on"; the `commands/` file-map row
gains the session floor; a pitfall row "A paused edit's before-image is the simulated
value; a replay onto the restored world must `rebase_onto` first or Undo
resurrects simulation state" naming the rebase test; the inspector heading rule.

Gates: standard engine + wasm. Leaves out: anything visual beyond the heading.

## Batch 2 — engine: the toolbar strip (2d#131) — DONE 2026-09-09 (0e7c6bd)

Authored by Jesse's Claude Code session in the other window from
`review/playground-ux/handoff-2.md`; reviewed by kimi (`review-8.md`, 4 findings, all
accepted), codex (`review-8-codex.md`, 5, all accepted) and the planner (`review-8-claude.md`,
2 of its own plus the executor's flagged gizmo rect, folded in on Jesse's ruling); adjudicated
in `rebuttal-8.md`. The planner's fix hunks (15 files) went back through kimi (`review-9.md`, 2)
and codex (`review-9-codex.md`, 2) as their own diff; those four corrections were applied and
enumerated in `rebuttal-9.md`, not re-reviewed (Jesse's ruling, batch 1's precedent). Landed as
specified, plus what the reviews forced: **layer-aware blocking in `ui`** — a blocking region
carries the `UiLayer` that claimed it and a widget in an overlay scope is inert only under a
higher layer's region, because the strip's own scope had exempted Play from the Stop dialog's
scrim; the narrow overlay and its tabs start below the centre's header AND strip (the section's
"below the strip" measured from the dock top) and run to the dock's bottom; the overlay closes
from its chevron; a collapsed panel opens whole through `expanded_content_bounds` without its
persisted flag changing; the centred play controls clamp to the right edge between 323 and
367 px; Escape closes the overflow menu in the key router, in every play state;
`scene_view_bounds()` is `None` for an empty viewport; the gizmo clips to the viewport; and
`Toolbar`'s position API is gone with `toolbar_position_for`. The strip's band is a
`begin`/`end` scope, not the single `render` this section named (the executor's deviation,
accepted: the band must cover the widgets, and scopes cannot nest). Every gate green
(`gates-2-final.log`); all seven games pass `check_games.sh` (`gates-2-final-games.log`;
`game-template` is cloned in this working set after all). Filed: `#134` (`float_input`'s
drag-scrub ignores blocking regions — pre-existing, found by the acceptance test). Owed:
Jesse's browser check — the strip at a desktop width, the overflow menu at a narrow one, the
inspector opening over the viewport at 390 and 320 px and closing from its chevron, and Play
inert behind the Stop dialog — on the next playground bundle.

**Re-verified against the tree 2026-09-09 before the handoff** (after batch 1, f268bf4):
`toolbar_position_for` is `crates/editor/src/toolbar.rs:186` with two tests of its own at
`:200-201` that go with it, its one production caller is
`editor_game/mod.rs:190` inside `render_toolbar_and_play_controls` (`:186`), it is
re-exported at `crates/editor/src/lib.rs:144`, and a comment at `context/mod.rs:139` names
it — all four move or go, and the report shows the grep. `Toolbar { button_size: 56.0 }`
(`toolbar.rs:66`, `:85`) with `bounds()` at `:112`, `chrome_bounds()` at `:121`, `render()`
at `:130`; `PlayControls { button_size: 40.0 }` (`play_controls.rs:47`) with `chrome_bounds`
at `:85` and `render` at `:104`. `EditorContext::update_layout` is `context/mod.rs:436`,
`dock_area.layout()` at `:448`, `set_viewport_bounds` at `:452`, `scene_view_bounds()` at
`:473`, `panel_content_bounds` at `:478`; the world scissor takes `scene_view_bounds()` at
`editor_game/mod.rs:491-493` and the render camera comes from
`viewport.to_window_render_camera(ctx.window_size)` at `:486` (the full window, as kimi's
note said); `scene_view_bounds()`'s other callers are `mod.rs:250`, `:266` and `:366`.
`render_scene_view` is `panel_renderer/mod.rs:51` with the grid at `:66`, colliders `:101`,
selection `:127` and the play border `:139`. **The dock already has the collapse API narrow
mode builds on**: `DockPanel::collapsed` (`dock/mod.rs:92`), `is_collapsible` (`:130`),
`content_bounds` (`:150`, zero rect when collapsed), `DockArea::set_panel_collapsed` (`:237`),
`toggle_panel_collapsed` (`:247`) and `layout()` (`:257`, the edge allocator over
`remaining`). The default hierarchy is 200 px with a **150** minimum (not 200) and the
inspector 280 with a 200 minimum (`context/mod.rs:104-113`). `UiLayer::PanelChrome` has
no user yet; `ui.begin_overlay_in(UiLayer::…, rect)` is how a band is entered
(`confirm_dialog.rs:98`, `asset_browser.rs:321`), and `menu/tests.rs:36` pins that an
overlay band's own widgets work while widgets beneath it are blocked — the strip wants
exactly that. The overlay-equivalence tests to run first are
`crates/editor/src/viewport/tests.rs:105`
(`test_overlay_matches_gpu_camera_at_a_panel_offset_and_the_play_follow_pose`),
`grid.rs:425` and `collider_overlay.rs:259`. Line counts today: `context/mod.rs` 556,
`editor_game/mod.rs` 552, `panel_renderer/mod.rs` 470, `toolbar.rs` 279,
`play_controls.rs` 241, `dock/mod.rs` 317, `layout.rs` 30.

Files: `crates/editor/src/layout.rs` (30), new `crates/editor/src/toolbar_strip.rs`,
`crates/editor/src/toolbar.rs` (279), `crates/editor/src/play_controls.rs` (241),
`crates/editor/src/context/mod.rs` (556 — one call and one accessor, no more),
`crates/editor_integration/src/editor_game/mod.rs` (544 — the render call moves),
`crates/editor_integration/src/panel_renderer/mod.rs` (470).

- `layout.rs` gains `TOOLBAR_STRIP_HEIGHT` (~40). `EditorContext::update_layout` splits the
  scene view's content bounds: a strip on top, the viewport below; `scene_view_bounds()`
  returns the viewport only; new `toolbar_strip_bounds() -> Option<Rect>`. The split and
  the drawing live in `toolbar_strip.rs` (`pub fn split(content: Rect) -> (Rect, Rect)`,
  `pub fn render(ui, strip, theme)`), not in either near-ceiling file.
- The strip is drawn opaque (`surface_1` fill, a `border_subtle` bottom rule) on
  **`UiLayer::PanelChrome`**, and `Toolbar` and `PlayControls` render into it on that layer:
  compact buttons (`button_size` 28–32; the shortcut hint becomes a smaller caption until
  tooltips arrive in batch 10), a readable active state (`toolbar_active` fill plus an
  `accent_blue` outline), tools at the left, Play/Pause/Stop at a fixed x (the strip's
  centre) so they never move when the tool set or the play state changes, Follow beside
  them. `toolbar_position_for` is deleted (grep shown in the report) and
  `render_toolbar_and_play_controls` positions from `toolbar_strip_bounds()`.
- The world scissor (`editor_game/mod.rs:483-488`) and every overlay in
  `render_scene_view` use `scene_view_bounds()`, so grid, axes, colliders and gizmos cannot
  draw over the strip; the play-state border is drawn around the viewport, under the strip.
- **Narrow viewports** (round 1, codex F10): the strip has a minimum usable width
  (`TOOLBAR_STRIP_MIN_WIDTH`, the tools plus the play controls plus the gaps); above it the
  play controls sit at the centre; below it **the play controls are laid out from the
  strip's right edge first and never clip** (they are the sprint's primary control and the
  in-editor Play), the right-hand group batch 10 adds sits beside them, and the tool group
  is what sheds: tools that no longer fit collapse into one overflow button that opens a
  small menu on the Floating layer listing the hidden tools with their shortcuts. The
  layout test runs at the viewport width a 390 px page produces with the hierarchy and
  inspector docked, not only at a comfortable width, and asserts reachability — the play
  controls' rect fully inside the strip and every tool reachable directly or through the
  overflow — not merely non-overlap. **The dock keeps a minimum centre width**: the
  default hierarchy is 200 px and the inspector 280 with a 200 minimum
  (`context/mod.rs:107-113`), so at 390 px `DockArea::layout` today hands the centre
  nothing and no strip layout could help; `layout()` gains `MIN_CENTER_WIDTH` (the strip's
  minimum: tools overflow button, play controls, and the View and Reset controls batch 10
  adds) and, when the centre would fall below it, enters **narrow mode**: the side panels
  leave the edge allocation (the centre takes the width), and one side panel at a time is
  shown as an **overlay over the viewport on the Floating layer** at its own width, opened
  from its header tab at the edge; opening one closes the other, and the strip with the
  play controls stays reachable underneath. Collapsing alone would thrash — reopening a
  collapsed inspector violates the minimum and collapses it again, so the visitor could
  never reach the field the acceptance test asks them to change. Tests: the centre width
  at 390 px with both panels docked; opening the inspector and editing a field at 390 and
  320 px, then reaching the hierarchy.
- **Re-verify before the handoff** (kimi's note): `to_window_render_camera(window_size)`
  derives from the full window; the section names every overlay and picking path that maps
  through the viewport rect (`panel_renderer/mod.rs`, `viewport_interaction.rs`,
  `gizmo_drag.rs`) and the executor runs the overlay-equivalence tests first — if any path
  still maps through the old content rect it is off by the strip's height.
- Tests: `toolbar_strip::split` yields two disjoint rects that tile the input; the strip is
  inside the panel's content bounds; a `Toolbar` and a `PlayControls` laid out in a
  strip do not overlap at the narrow width and at a comfortable one. Docs:
  `crates/editor/CLAUDE.md` file map and the panels paragraph;
  `crates/editor_integration/CLAUDE.md` if it names the floating toolbar.

Gates: standard engine + wasm. Leaves out: the View toggles in the strip (batch 10).

## Batch 3 — engine: asset browser, wheel notch, drag into the canvas (2d#130, 2d#119, 2d#120) — DONE 2026-09-09 (e5ad961)

Authored by Jesse's Claude Code session in the other window from
`review/playground-ux/handoff-3.md`; reviewed by kimi (`review-10.md`, 3 findings: 2 accepted,
1 rebutted in part — no consumer outside the editor reads the wheel delta, the unit went into the
docs), codex (`review-10-codex.md`, 2 accepted) and the planner (`review-10-claude.md`, 2
accepted); adjudicated in `rebuttal-10.md`. The planner's fix hunks (12 files) went back through
kimi (`review-11.md`, 2 minor) and codex (`review-11-codex.md`, none) as their own diff; the one
correction taken is enumerated in `rebuttal-11.md`, not re-reviewed. Landed as specified, plus:
the hover shows the same relative path the click does and neither writes over a persistent
error (`StatusBar::is_showing_error`); the pointermove listener also bails while the page has a
non-collapsed text selection (the console log is an `<output>`, never the active element — the
documented cost is that a stale selection keeps the canvas unfocused through a later cross-page
drag until a click); the wheel accessors name their unit; the wheel-zoom test is named for a notch;
the selection travels by relative path across a rescan and Assign is enabled only for a loaded
image tile (the executor's deviations, accepted); and the drag ghost renderer moved to
`panel_renderer/drag_ghost.rs` to keep `asset_browser.rs` under the ceiling. Every gate green
(`gates-3-final.log`); all seven games pass `check_games.sh`. Filed: nothing new — a press/release
test for the tile's click is the first item of the new editor_integration backlog, `#135`. Owed: Jesse's browser checks — a hard trackpad flick
zooms a few times a second and the inspector still scrolls at a usable speed; a drag from the page
background into a partly visible canvas on a scrolled page moves nothing; drag-selecting the
console's text on a scrolled page keeps the selection and the focus.

**Re-verified against the tree 2026-09-09 before the handoff** (after batch 2, 0e7c6bd; every
line count below still holds). **2d#130:** `AssetBrowserState` is `crates/editor/src/asset_browser.rs:44`
with three fields — `entries`, `scanned`, `scroll` — and no selection yet; in
`panel_renderer/asset_browser.rs`, `render_asset_browser` is `:75` (the scroll at `:91-96`, the
tile loop `:102-121`, the assign at `:123-126`), `render_header` `:129` (the Rescan button at
`:136-137`, 70×20 at `bounds.x + PADDING`, and the count label after it — the Assign button goes
beside them), the filename label `:236-243` (`label_in_bounds_styled` into a
`TILE_SIZE × TILE_LABEL_HEIGHT` rect, centred, which is where `ellipsize` applies),
`tile_interaction` `:249` (the drag arms at `:266-269`, the click returns the assignment at
`:272-274`, `suppresses_click` already guards it), `assign_clicked_texture` `:291` with its three
status messages at `:300`, `:303`, `:306`, and `entity_ops::assign_sprite_texture` at
`crates/editor_integration/src/entity_ops.rs:161` (returns false with no Sprite or an unchanged
handle). `theme.selection_fill` exists (`theme/mod.rs:76`). **2d#119:** `SCROLL_PIXELS_PER_LINE`
is a *private* const at `crates/input/src/input_handler.rs:43` (16.0), used once at `:256`;
`WHEEL_STEP` is a module const in `crates/editor/src/scroll.rs:29` (30.0), not an associated
const, applied at `:51` and pinned by the test at `:98`; the viewport's clamp is
`viewport_input.rs:201-203`. The input guide's pitfalls row (`crates/input/CLAUDE.md:71`, "PixelDelta
÷ 16") and its guard test `crates/input/tests/mouse.rs:47`
(`test_wheel_lines_and_trackpad_pixels_accumulate_as_lines_and_clear_each_frame`) both change with
the constant — they are in the batch. **2d#120:** `focus_before_winit_does` is
`crates/renderer/src/window.rs:164` (wasm-only, `#[cfg(target_arch = "wasm32")]`), the pointerdown
closure and its target discipline `:176-183`, the listener install `:184-191`;
`crates/renderer/Cargo.toml:21-32` lists the web-sys features — `Document` and `Element` are there,
**`MouseEvent` is not** (`buttons` needs it; the renderer guide's pitfalls row is `CLAUDE.md:52`).
**What batch 2 changed underneath:** a drop into the canvas is taken with
`drag_drop.take_drop_in(scene_bounds)` at `viewport_interaction.rs:33-35`, and `scene_bounds` is
now `scene_view_bounds()` — the viewport BELOW the toolbar strip, and `None` when the strip eats
the content — so a drag released over the strip is not a canvas drop; and a blocking region now
carries its layer, so the drag ghost's DragGhost region makes widgets in Floating scopes under
it inert too (rebuttal 8). Neither needs code here; both are why the drop test releases inside
`scene_view_bounds()`, not the panel's content rect.

Files: `crates/editor/src/asset_browser.rs` (236),
`crates/editor_integration/src/panel_renderer/asset_browser.rs` (374),
`crates/editor/src/row_layout.rs` (234, `ellipsize` at 123),
`crates/input/src/input_handler.rs` (385), `crates/input/tests/mouse.rs`,
`crates/input/CLAUDE.md`, `crates/editor/src/scroll.rs` (149),
`crates/editor/src/viewport_input.rs` (500), `crates/renderer/src/window.rs` (194) and
`crates/renderer/Cargo.toml` (web-sys features only).

- **2d#130.** `AssetBrowserState` gains `selected: Option<usize>`. A single click selects
  the tile (a `selection_fill` highlight; the full relative path on the status bar);
  press-and-move still arms the drag; assigning takes a drag onto the object or the
  texture field, or the new **Assign** button in the asset-browser header (enabled when a
  tile is selected and the primary selection has a `Sprite`; through
  `assign_sprite_texture` as today so undo stays; the three status messages of
  `assign_clicked_texture` move to it). Labels are `row_layout::ellipsize`d to `TILE_SIZE`
  with the full name on the status bar on hover (batch 10's tooltip replaces the status-bar
  line). A list view is left out and filed if Astra insists. Tests: a click no longer
  assigns; Assign does, through the history; an ellipsized label measures under the tile.
- **2d#119.** `SCROLL_PIXELS_PER_LINE` → `SCROLL_PIXELS_PER_NOTCH = 100.0` (a browser
  notch, the constant a notch actually is); `scroll.rs`'s `WHEEL_STEP` re-tuned so a mouse
  notch moves a panel a readable amount (60–100 px) and a trackpad moves it about 1:1
  with the finger; the viewport's one-notch-per-frame clamp stays. Test: a 100 px pixel
  delta is one notch. Jesse's trackpad check is the acceptance: a hard flick zooms a few
  times a second, the inspector still scrolls at a usable speed.
- **2d#120.** `focus_before_winit_does` also listens to `pointermove` in the capture phase
  and focuses the canvas with `preventScroll` before winit's handler when **all** of:
  `buttons != 0`, `event.target` is the canvas (the same target discipline the `pointerdown`
  handler keeps at `window.rs:176-183` — the drag is over the canvas, not merely happening
  somewhere on the page), and the canvas is not `document.activeElement`; it bails when
  the active element is an input, textarea, select, button, summary, dialog or a
  contenteditable, so drag-selecting text in the dock's console or textarea never yanks
  focus into the editor mid-gesture (`MouseEvent::buttons`, `Document::active_element`,
  `Element::closest` — the web-sys features go in `crates/renderer/Cargo.toml`; no new
  crate). wasm-only; the wasm gate compiles it. Jesse's checks: a drag from the page
  background into a partly visible canvas on a scrolled playground page moves nothing;
  drag-selecting text in the console on a scrolled page keeps the selection and the focus.
- Docs: `crates/editor/CLAUDE.md` asset browser line; `crates/input/CLAUDE.md` if it names
  the constant; `crates/renderer/CLAUDE.md` pitfall row for the focus listener.

Gates: standard engine + wasm + `check_games.sh` (the wheel normalization every game's input
goes through changes, private though the constant is).

## Batch 4 — engine: the game-only preview (2d#121) — DONE 2026-09-09 (e542875)

Authored by Jesse's Claude Code session in the other window from
`review/playground-ux/handoff-4.md`; reviewed by kimi (`review-12.md`, 6 findings: 4 accepted, 2
rebutted — the old-page/new-wasm pairing is what versioned bundle directories exist for, and the
`assets/scenes/` prefix is this section's own rule), codex (`review-12-codex.md`, 3, all accepted)
and the planner (`review-12-claude.md`, 5); adjudicated in `rebuttal-12.md`. The planner's fix
hunks (11 files) went back through kimi (`review-13.md`, 2) and codex (`review-13-codex.md`, 1) as
their own diff; the one correction taken is enumerated in `rebuttal-13.md`, not re-reviewed. Landed
as specified, plus what the reviews forced: **the completion is read, not taken** — every
subscriber of a generation reads the same answer, so an Export joined behind a launch gets the
bytes instead of a 5 s timeout (the executor's `take` had pinned the opposite); **the hidden-frame
pump is on both pages**, the editor's first (the window hides the editor's tab before the frame
that answers the snapshot), starts as soon as there is a loop to wake on a page hidden all through
its boot, and runs one chain at a time; **readiness reads the boot status by phase** — after the
first frame any status the engine writes ("Graphics device lost", "Game ended") is `failed:`, not
a substring match; a failed `run_game` frees the controls slot; and Export's refusal during Play or
Pause is documented (the saved-scene fallback was rebutted as the silent loss the live export
exists to prevent). The executor's deviations, accepted: `Arc<Completion>` for `Rc` (the mailbox
sits in an `Arc` and `Rc` would have forced `unsafe impl`); pendency read off the completion, not
the `pending` field, so the generation stays owed during the frame that computes it; the scene-load
failure recorded on `PreviewControls` rather than written over the renderer's boot status; the
state test and the keep-every-scene test written in the crates whose code they test. Every gate
green (`gates-4-final.log`, 907 tests); all seven games pass `check_games.sh`. Filed: nothing new.
Owed: Jesse's browser checks — Play ↗ with the editor's tab going to the background answers the
snapshot and the preview reaches "running"; a preview opened behind (a background tab) reaches
"running" once its loop exists without being focused; Export a moment after Play ↗ returns the
same bytes; the preview page writes no localStorage key and installs no write observer; a hidden
editor keeps answering under Chrome's background-timer throttling.


Files: new `crates/editor_integration/src/editor_game/snapshot.rs` and
`snapshot_tests.rs`, `editor_game/run_options.rs` (67), `editor_game/scene_io.rs` (289),
`editor_game/play_session.rs`, `editor_game/mod.rs` (557 → ~563), `project_host.rs` (295 →
~300) with new child `project_host/preview.rs`, `crates/editor_integration/src/lib.rs`;
`crates/playground/src/archive.rs` (327 → ~390), new `crates/playground/src/preview.rs`
(target-agnostic) and `preview_entry.rs` (wasm-only), `bridge.rs` (380 → ~455),
`web_entry.rs` (272 → ~285), `lib.rs`.

- **`snapshot.rs`** (~120): `pub struct SceneSnapshot { pub scene_entry: String, pub ron: String }`;
  `pub struct SceneSnapshotRequest { pending: Mutex<Option<u64>> /* the generation filed, None when idle */, answer: Mutex<Option<(u64, Result<SceneSnapshot, String>)>> }`
  with the bridge side `file(generation) -> bool` (refused while another generation is
  pending — one request in flight), `cancel(generation)` (clears a pending request and any
  answer of that generation, so a timed-out request never resolves a later one),
  `subscribe(generation) -> Rc<Completion>` (**one shared completion per generation, held
  by every subscriber before it polls**: the preview launch and an Export started behind
  it each hold the same `Completion`, which settles once with the answer, the rejection
  or the cancellation; a launch's cleanup never erases a completed result — the slot is
  freed only when the last subscriber drops it or the next generation is filed, so an
  Export that polls after the launch finalized still reads the result; an answer of
  another generation is never handed over), and the editor side
  `pub(crate) take_request() -> Option<u64>` / `answer(generation, …)`. Export copies its
  bytes before the preview's buffer is transferred to the window.
  `EditorGame::scene_snapshot(&mut self, world, texture_path_fn) -> Result<SceneSnapshot, SceneIoError>`:
  refused in a play session (`MidSimulation`); **serializes a named scratch world**:
  `editor::world_snapshot::WorldSnapshot::capture(world)` `restore`d into a fresh `World`, then
  `engine_core::script_data::ensure_script_target_names(&mut scratch)` (the pure naming
  rule that already exists, `script_data.rs:195-249`; save applies it to the live world
  through the history, the snapshot applies it to the scratch), then
  `world_to_scene_data(&scratch)` + `serialize_to_ron` — naming after serialization would
  be too late, because `scripts_to_data` drops a parameter whose target has no `Name`
  (`:169-180`); the live world, the history, the dirty mark, `scene_path` and every file
  are untouched, also when serialization fails. `scene_entry` is the scene path (or `default_scene_path()`)
  stripped of the asset base and prefixed `assets/`; a path outside the base is
  `SceneIoError::OutsideProject(PathBuf)` (new variant). `answer_scene_snapshot(&mut self,
  ctx)` runs each frame **before** `drain_api_requests`'s mid-drag skip (it is read-only
  against the world; a mid-drag answer captures the drag state, and the doc says so), so a
  long drag cannot starve it past the 5 s cap. `EditorRunOptions` gains
  `scene_snapshot: Option<Arc<SceneSnapshotRequest>>` and `preview_open: Option<Arc<AtomicBool>>`,
  both threaded onto `EditorGame`; `handle_play_action(Play)` while Editing refuses with
  the status error "A preview window is open — close it to Play here" when the flag is
  set. `SceneSnapshot` and `SceneSnapshotRequest` are re-exported from `lib.rs`.
- **`project_host/preview.rs`** (~190 with tests, a child module for `pub(crate)` access):
  `#[derive(Default)] pub struct PreviewControls { paused: AtomicBool, restart_requested: AtomicBool }`
  with `toggle_paused() -> bool`, `is_paused()`, `request_restart()`;
  `pub(crate) enum PreviewTick { Restarted, Frozen, Running }`;
  `pub struct PreviewHost { host: ProjectHost, scene_path: PathBuf, controls: Arc<PreviewControls> }`
  with `new(project_path, scene_path, controls)`, `pub(crate) load_scene(world, assets)`
  (clear → instantiate → publish `PhysicsSettings` → transform reset → the host's play
  state reset, the `scene_io::load_scene` model) and `pub(crate) tick_controls(world,
  assets) -> PreviewTick` (a requested restart reloads, clears the pause and calls
  `request_backdrop_reset`). `impl Game`: `init` = `host.init(ctx)`, load the editor's
  regular font (`editor::fonts::EDITOR_FONT_REGULAR`, already `include_bytes` in the
  bundle) as the UI default for `UiLabel`s, `load_scene` (a failure is logged — the unpack
  dry-ran it); `update` = `Frozen` → `ctx.time_scale = 0.0` and return, else
  `time_scale = 1.0` and `host.update_frame(...)`. `ProjectHost` gets `mod preview;` and
  the re-exports.
- **`archive.rs`**: `export_project` is split into `collect_asset_entries(root)` and
  `write_archive(entries, manifest)`; new
  `pub fn export_snapshot(project_root, manifest, scene_entry: &str, scene_ron: &str) -> Result<Vec<u8>, ArchiveError>`
  **replaces only the entry at `scene_entry`** with the live scene and keeps every other
  file — the export is a project backup, and dropping the other scenes would lose authored
  work on re-import; the entry must pass `bridge::relative_path_is_safe` (`bridge.rs:87`) and start with
  `assets/scenes/`, else `ArchiveError::OutsideProject(String)` (already a variant, `archive.rs:37`). Which scene the
  preview loads is named explicitly, never inferred from the archive.
- **`preview.rs`** (~130 with tests):
  `pub struct UnpackedPreview { pub root: String, pub scene: PathBuf, pub files: Vec<(String, Vec<u8>)> }`;
  `pub fn unpack_preview(bytes, scene_entry: &str, asset_base, bundle_version) -> Result<UnpackedPreview, ArchiveError>`
  through `import_project` (every refusal it has), `projects::project_root(asset_base, &manifest.slug)`,
  keys `{root}/{path}`; `scene` is `{root}/{scene_entry}`, which must be one of the
  unpacked files, else `ArchiveError::MissingScene(String)` (new). A test over the bundled
  projects pins "scenes live directly under `assets/scenes/`" so `first_scene_in` and the
  editor's default scene keep agreeing.
- **`preview_entry.rs`** (wasm-only, ~120): `web_entry::start()` (the cdylib's one
  `#[wasm_bindgen(start)]`, `web_entry.rs:63`) reads `query_param("mode")` and, on
  `preview`, calls `announce_preview_mode()` (boot status "Waiting for the editor's
  snapshot…") and returns before `run_playground` — no store, no chains, no observer, no
  listeners, no preload. Exports:
  `playground_load_preview(bytes: Vec<u8>, scene_entry: String) -> Result<(), JsValue>`
  (refused when one already loaded — one runtime per page; `unpack_preview` → `vfs::insert`
  each file → `run_game(PreviewHost::new(root, scene, controls), GameConfig::new("Preview")
  .with_size(1280, 800).with_asset_base_path(...))` — every save path defaults to `None`, so
  the preview writes no localStorage key), **`playground_preview_state() -> String`**
  (`"booting"` until the host's first successful frame, then `"running"`; `"failed: <text>"`
  from the renderer's boot-status failure — `game/web.rs:125-129` already writes it — or a
  scene-load failure, which the host now surfaces through the same status instead of only
  logging; `Ok` from `playground_load_preview` means scheduled, not running, so the page
  reports readiness from this export), `playground_preview_pause() -> bool` (toggle;
  returns the new paused state), `playground_preview_restart() -> Result<(), JsValue>`
  (Err when nothing loaded). Stop = the page closes the window; the `pagehide` guard
  already latches the loop. **Hidden-document frames**: `request_redraw` is an animation
  frame under winit's web backend and a hidden tab gets none, so the hidden path does not
  use it. `run_game` gains a user-event type and keeps the loop's `EventLoopProxy` in a wasm thread-local
  (`engine_core::web`); a `visibilitychange` listener starts, on the transition to hidden,
  a 100 ms `setTimeout` loop that sends a `WakeUp` user event
  (`EventLoopProxy::send_event`, scheduled by winit through `web_sys/schedule.rs`, not
  rAF), and `ApplicationHandler::user_event` — which the engine does not implement today —
  drives the frame; the loop stops when the document is visible again and rAF resumes.
  Starting on the transition means a frame rAF had already scheduled and will never
  deliver does not matter. Pulled forward from batch 12, because a browser that opens the
  preview as a foreground tab backgrounds the editor before the frame that answers the
  snapshot. Tests: the wake path headless where the engine can; the tab case is Jesse's.
- **`bridge.rs`**: `Hooks` gains `scene_snapshot` and `preview_open`. One internal
  `live_scene(generation) -> Promise<(scene_entry, Uint8Array)>` does the work:
  `file(generation)`, poll `take_answer(generation)` every 50 ms up to 5 s (the drain's
  polling shape), on timeout `cancel(generation)` and reject with "the editor did not
  answer — is its tab visible?"; the editor's refusal passes through as the rejection
  (`MidSimulation` reads "scene is mid-simulation — stop Play first"); then
  `export_snapshot(root, active_manifest, entry, ron)`. Two exports use it:
  **`playground_snapshot(generation: u64)`**, resolving an **envelope `{ sceneEntry, bytes }`**
  (a JS object — the page must name the scene it previews, and bytes alone would leave it
  guessing), refused with "a snapshot is already in flight" while another generation is
  pending, and which also **sets `preview_open` when it files the request** — the
  reservation is taken at the click, so the editor's own Play is refused before the preview
  window has booted (v1 set it on the handshake and left a window in which two simulations
  could start) — and clears it when it rejects; and **`playground_export_zip()`**, which
  becomes the live-scene export resolving bytes (a Promise now; the acceptance test is
  "exports their work", and the saved-scene export would have dropped the very edit the
  visitor previewed) and which, when a snapshot is already pending, **waits for that
  generation's answer and exports those bytes** rather than refusing — Export a moment
  after Play ↗ is an ordinary two-click sequence. `playground_set_preview_open(open:
  bool)` lets the page release the reservation on `preview-failed`, `preview-closed` or a
  closed window — never on silence. `web_entry.rs` builds both `Arc`s and hands them to
  `Hooks` and `EditorRunOptions`. `docs/WEB_PLAYGROUND.md` § Export and import and the
  site's export handler (batch 6) follow the Promise.
- Tests: `snapshot_tests.rs` —
  `test_scene_snapshot_serializes_the_live_world_with_unsaved_edits_and_leaves_file_history_and_path_alone`
  (load from a tempdir via `load_scene` with the stub resolver, execute a `SetComponentCommand<Transform>`
  through the history, the RON carries the new value, `scene_entry == "assets/scenes/<name>.scene.ron"`,
  the file bytes are unchanged, `is_dirty()` still true, `scene_path` unchanged),
  `test_scene_snapshot_names_unnamed_script_targets_in_the_scratch_world_and_keeps_their_parameters`
  (a script parameter targeting an unnamed entity survives the snapshot, re-imports and
  resolves to the intended entity; the live world, history and dirty mark are untouched,
  also when serialization fails), `test_scene_snapshot_is_refused_mid_session` (Playing and
  Paused), `test_play_is_refused_while_a_preview_window_is_open`,
  `test_preview_state_reports_running_only_after_the_first_successful_frame_and_failed_on_a_scene_load_error`,
  `test_export_snapshot_keeps_every_other_scene` (a project with two scenes exports both,
  the active one replaced by the live bytes, and re-imports),
  `test_snapshot_request_refuses_a_second_generation_while_one_is_pending_and_drops_a_cancelled_answer`
  (file 1, cancel 1, the editor answers 1, `take_answer(2)` after filing 2 yields nothing
  until the editor answers 2). `project_host/preview.rs` —
  `test_restart_returns_the_world_to_the_loaded_scene_and_clears_the_pause` (a Patrol
  behaviour advances x; toggle pause; request restart; `Restarted`; the authored position;
  not paused; physics re-initialises on the next frame),
  `test_pause_freezes_the_step_until_toggled_back`. `archive/tests.rs` —
  `test_export_snapshot_carries_only_the_live_scene_and_reimports`,
  `test_export_snapshot_refuses_a_scene_entry_outside_assets_scenes`. `preview.rs` —
  `test_unpack_preview_keys_every_file_under_the_project_root_and_names_the_scene`,
  `test_unpack_preview_refuses_a_zip_escaping_the_root`,
  `test_unpack_preview_refuses_a_snapshot_without_a_scene`. The wasm-only files compile
  under `scripts/check_wasm.sh`.
- Docs: `docs/WEB_PLAYGROUND.md` § The bridge gains `playground_snapshot(generation)` (a
  Promise of `{ sceneEntry, bytes }`: the project archive with the active scene's entry
  replaced by the live world as RON and every other file kept; answered on the editor's
  next frame, 5 s cap; refused during Play or Pause; reserves the preview as it files),
  `playground_export_zip()`'s row updated to the live-scene Promise, and
  `playground_set_preview_open(open)` (in-editor Play refused while true; the page clears
  it when the window closes); new § The preview window (`?mode=preview` on the page URL
  boots the game-only runtime: no store, no persistence, no preload; the four exports;
  the preview loads the scene entry it is handed; Stop = close the window; one runtime per
  page because winit refuses a second event loop); § The bundle contract's "one embed per page" gains
  "and one runtime: a page is the editor or, with `?mode=preview`, the preview".
  `crates/editor_integration/CLAUDE.md` — File Map rows for `editor_game/snapshot.rs` and
  `project_host/preview.rs`, the `run_game_with_editor_opts` paragraph lists the two new
  options, pitfall rows "A scene snapshot must serialize the live world without touching
  the file, the saved mark or the history" and "Preview Restart must reload the scene AND
  reset the host's play state or physics keeps the previous run's bodies", each naming its
  test. `crates/playground/CLAUDE.md` — rows for `preview.rs` and `preview_entry.rs`, the
  boot line, pitfall rows "The preview loads the scene entry it is handed, and the bundled
  projects keep their scenes directly under `assets/scenes/` so the editor's default and
  `first_scene_in` agree" (naming the bundled-projects test) and "The preview page never
  opens the store or installs the write observer (browser check)".

**Re-verified against the tree 2026-09-09 before the handoff** (after batch 3, e5ad961; every
line count in the Files line holds — `run_options.rs` 67, `scene_io.rs` 289, `play_session.rs` 272,
`project_host.rs` 295, `archive.rs` 327, `bridge.rs` 380, `web_entry.rs` 272 — except
`editor_game/mod.rs`, which is **557** after batches 2–3, not on its way to 558: it takes the two
fields and the one call and nothing else; `snapshot.rs` takes the rest). **Nothing of this batch
exists yet**: no `snapshot.rs`, no `project_host/preview.rs`, no `preview.rs`/`preview_entry.rs`,
no `EventLoopProxy` anywhere in the engine. **The snapshot:** `WorldSnapshot` is the editor
crate's (`editor::world_snapshot`, `capture` `:75`, `restore` `:102`; `mod.rs:19` already imports
it) and captures only registry-known components plus the hierarchy — the same set Play/Stop
already loses, so the scratch world loses nothing a save would keep. The naming rule is
`plan_script_target_names` (`script_data.rs:195`) applied by `ensure_script_target_names`
(`:243-249`); `save_scene_with` (`scene_io.rs:83-124`) is the model — it plans at `:99`, executes
through the history, then `world_to_scene_data` at `:124` — and the drop it guards against is
`scripts_to_data`'s unnamed-target arm at `:169-180`. `texture_path_fn` is the closure
`save_scene_as` builds at `:70-71` (`texture_ref_for_save(handle, assets.texture_path(handle))`),
so the RON matches a saved scene byte for byte. `default_scene_path` is `pub(super)` at `:247`;
`SceneIoError` has `MidSimulation`, `Write`, `Load` (`:13-17`) — `OutsideProject(PathBuf)` is
new there. `handle_play_action` lives in **`play_session.rs:239`**, not `mod.rs`; the Play arm
(`:248-255`) is the one place to refuse — the strip (`mod.rs:219`), the key router
(`shortcuts.rs:116`) and the stop dialog's resume (`:363`) all route through it; the error goes
through `status_bar.show_error` (`status_bar.rs:67`). `answer_scene_snapshot` goes in
`EditorGame::update` before the `drain_api_requests(ctx)` call at `mod.rs:464`; the mid-drag skip
is inside `take_api_lines` (`api.rs:222-224`), which the answer path does not call. The stub
resolver is `engine_core::test_support::StubResolver` (`test_support.rs:36`, feature
`test-support`, already a dev-dependency); `scene_io_tests.rs:60-80` is the tempdir + `load_scene`
shape and `:67` pushes a command through the history. **The host:** `ProjectHost`'s fields are
private and `update_frame` (`project_host.rs:51`), `reset_play_state` (`:124` — the "play state
reset") are `pub(crate)`, which is why `preview.rs` is a child module; its `Game::init` (`:132`)
sets the asset base from `project_path/assets` and initialises the hierarchy, and `load_scene`'s
model is `scene_io.rs:150-200` (clear `:172`, instantiate `:173`, publish `PhysicsSettings`
`:178-190`). `project_host.rs:165` already has a Patrol test to copy the shape from.
`editor::fonts::EDITOR_FONT_REGULAR` is `fonts.rs:18` (`pub mod fonts`); the UI default is
`load_font` + `set_default_font` (`ui/src/context/mod.rs:97`, `:112`). `GameContext.time_scale` is
`contexts.rs:83`. **The archive:** `export_project(project_root: &Path, manifest)` `archive.rs:88`,
`import_project(bytes, bundle_version) -> (ProjectManifest, Vec<StoredFile>)` `:162-165`;
`archive/tests.rs` exists. `projects::project_root(base, slug)` is `projects.rs:56`;
`SceneLoader::first_scene_in` is `engine_core` (`scene_loader.rs:98`), fed `<root>/assets/scenes`
at `web_entry.rs:254`. `common::vfs::insert(path: String, bytes: Vec<u8>)` is `vfs/mod.rs:215`
(the boot loop uses it at `web_entry.rs:188`). **The bridge:** `Hooks` (`bridge.rs:23`) has
`source_check` and `script_errors`, set by `set_hooks` from `web_entry.rs:231-241`; the project
root is the `CURRENT_PROJECT_ROOT` thread-local `setup_bridge` fills (`:227`); every Promise
export uses `wasm_bindgen_futures::future_to_promise` (`:218`); `playground_export_zip` is the
synchronous `Result<Vec<u8>, JsValue>` at `:244-253`. "The drain's polling shape" is
`persist::drain_then_epoch` (`persist/mod.rs:415-447`): a 50 ms `set_timeout` promise awaited
through `JsFuture`, `common::clock::Instant` for the 5 s cap, and its timeout text already blames
tab visibility. `EditorRunOptions` (`run_options.rs:15-31`) has seven fields; the
editor_integration guide's paragraph (`CLAUDE.md:15`) lists six — add `script_errors` with the two
new ones. **The preview page:** the preview must not call `run_playground`, `setup_bridge`,
`set_hooks` or `persist::install_listeners` (`persist/mod.rs:451`, the editor's own
`visibilitychange`/`beforeunload` pair); with no `CURRENT_PROJECT_ROOT` the old exports refuse on
that page by themselves. `set_boot_status` only writes `#game-loading` (`web/mod.rs:88-95`) and
nothing reads it back: add a `boot_status() -> Option<String>` reader beside it (the same element's
text) for the `failed:` branch of `playground_preview_state`; the renderer's failure write is
`game/web.rs:125-130`. **The event loop:** `run_game` builds `EventLoop::new()` with the `()`
user event (`game.rs:206`) and `GameRunner` is `ApplicationHandler<()>` (`app_handler.rs:143`); on
the web the only frame driver is `RedrawRequested` → `drive_frame` (`:246-252`) and
`request_redraw` is rAF (`:181`, `web.rs:86`); `about_to_wait` is native-only (`:258-266`). So the
wake path is all new: `EventLoop::<WakeUp>::with_user_event().build()`, `ApplicationHandler<WakeUp>`,
`create_proxy()` into a thread-local beside the page-exit guard (`web/mod.rs:27-77` is the shape:
a static, an installer called from `run_game`'s web arm at `game.rs:219-222`), and a `user_event`
that calls `drive_frame`. `Document::visibility_state` needs the **`VisibilityState`** web-sys
feature in `engine_core`'s list (`Cargo.toml:46-61`; `Window`, `Document`, `Event`, `EventTarget`
are there; a feature is not a dependency). **Docs:** `docs/WEB_PLAYGROUND.md` § The bridge is
`:172` (the `playground_export_zip()` row `:189`), § Export and import `:207`, § The bundle
contract `:53` with its "**One embed per page.**" paragraph at `:68`; the editor_integration
guide's File Map is `:25-42` and its pitfalls table `:76+`; the playground guide's File Map `:16-28`
(the boot line `:18`) and pitfalls `:31+`. **Earlier batches:** batch 1's stop dialog resumes Play
through `handle_play_action` (`shortcuts.rs:363`), so the refusal covers it; batch 2's strip is the
Play button's home and nothing here moves it; batch 3 left `panel_renderer/asset_browser.rs` at 580
and `viewport_input.rs` at 500 — neither is in this batch.

Gates: standard engine + wasm. Leaves out: the page (batch 6).

## Batch 5 — site: the application shell, fullscreen, the compatibility panel (web#57, web#52, web#59) — DONE 2026-09-10 (insiculous_web 570eaab)

Authored by Jesse's Claude Code session in the other window from
`insiculous_web/review/playground-ux/handoff-5.md`; reviewed there (numbering is per repo) by
kimi (`review-1.md`, 6 findings: 3 accepted, 1 false — `init()` cannot reject after the loop
starts, `web_entry.rs` spawns and returns — and 2 policy: `#save-status` is batch 11's, and the
fill-mode `!important` does not fight the 1024×720 clamp because the engine follows the shown
box), codex (`review-1-codex.md`, 1, accepted — Try again re-wired every control after a rejected
init) and the planner (`review-1-claude.md`, 5, all accepted); adjudicated in `rebuttal-1.md`.
The planner's fix hunks (8 files) went back through kimi (`review-2.md`, 2) and codex
(`review-2-codex.md`, 1) as their own diff; the three one-line tightenings taken are enumerated in
`rebuttal-2.md`, not re-reviewed. Landed as specified, plus what the reviews forced: **the
controls are wired once** behind a flag, so a retry re-runs only the probe, the import and
`default()`; **the stage hides under the compatibility panel** (the black canvas was web#59's own
complaint); **the status carries the WebGPU reason**, so the live region announces it; a game
with no still of its own takes the Pong default *with Pong's alt*; the panel's copy claims no
subject, because six game pages render it too; **a save refusal on the banner — the engine's or
the dispatch's — is cleared by the next save that succeeds** on an explicit `ok: true`; Help is
audited at phone width as well. The executor's decisions, accepted: Aa third in the bar (the h1
is not focusable; the painted order is Jesse's listed keyboard order), the h1 as a named slot,
the toolbar's `projectControls` and Help's `projectHelp` props for the reduced slug-page shape,
and the focusable, named `.dock-body` axe demanded. Filed: web#61 (the layout gate runs no
opened-element scenario). **Owed: Jesse's browser checks** below — the batch's code is landed,
the mark is not final until they are reported.

No engine dependency: the shell is CSS and markup, and the engine already follows the
canvas's CSS box. The compatibility panel joins this batch because the gates only ever
audit the WebGPU-failure state, so it is what axe and the announce gate see on every
playground page.

Files: new `src/layouts/AppLayout.astro`, new `src/components/PlaygroundToolbar.astro`,
new `src/components/PlaygroundHelp.astro`, new `src/components/CompatibilityPanel.astro`,
new `src/scripts/webgpu-gate.ts`, `src/components/PlaygroundEmbed.astro` (527),
`src/components/GameEmbed.astro` (194), `src/components/EditorShortcuts.astro` (74),
`src/pages/playground.astro` (81), `src/pages/playground/[slug].astro` (89),
`src/pages/games/[slug].astro` (one prop on its `GameEmbed`),
`src/scripts/playground-embed.ts` (378), `scripts/lib/a11y-scenarios.mjs` (250),
`README.md` § The editor bundle (L143-182), `docs/roadmap.md` L130-139.

**Re-verified against the tree, 2026-09-10** (`insiculous_web` at `bbc95df` on `jesse`,
clean; batches 1-4 touched nothing in this repo, so every line above is as the plan found
it): the counts hold. `PlaygroundEmbed.astro:48-85` is the toolbar markup (ids
`project-select`, `project-select-data`, `reset-button`, `reset-note`, `export-button`,
`import-input`, all unchanged), its rules at L181-305 (`.toolbar`, `.field-group` L190-201
with the width-bug comment, `.reset-group`, `.btn-reset`, `.reset-note`, `.archive-group`,
`.btn-toolbar`, `.label-import`, `.input-file`) move with it; the optgroup label to shorten
is L57; the Scripts section is L99-120 and its rules from L341, the Console L122-140 and its
rules from L439; the canvas is L87-97 (its placeholder `width`/`height` attributes stay);
the component's only script is `import '../scripts/playground-embed.ts'` at L148. The new
controls take fixed ids: `save-button`, `play-button`, `fullscreen-button`, `help-button`,
`playground-help`, `dock`, `compatibility-panel`, `compatibility-reason`,
`compatibility-retry`. **The hosted save verb exists**: `crates/editor/src/command_api/parse.rs:269`
maps `save` to `HostedWrite::Save`, and the integration layer answers **one JSON line per
dispatched line, in order** (`editor_game/api.rs` pushes one response per request;
`command_api/mod.rs:165-178` shape them `{"ok":true,"data":…}` and
`{"ok":false,"error":…}`) — so the banner wiring is a sink queue in `playground-embed.ts`:
every dispatch pushes who asked (`console` for the form at L340-360, `save` for the button),
`pollResponses` (L116-126) shifts one sink per line, still appends every line to
`#command-output`, and a `save` sink whose line parses to `ok: false` writes its `error`
to `#playground-banner`; the button is enabled where Export is (L209). `webgpu-gate.ts`'s
consumers today are `playground-embed.ts:60-70` and `GameEmbed.astro:84-100` (the same
probe twice, same Firefox sentence); the preview page is batch 6's consumer and does not
exist yet. `src/scripts/` is inside `astro check` (`tsconfig.json` excludes `src/lib`, not
`src/scripts`), so the module is typed. `GameEmbed` renders playable on
`src/pages/games/[slug].astro:32` (six pages) and `playground/[slug].astro:41`, so the panel
reaches both through it; the `subject` prop becomes **`screenshot` and `screenshotAlt`**
(a path and its alt) — the playground passes `/images/platformer-in-editor-2026-08.png`,
both game pages pass the game's first `screenshots[]` entry (every game has one;
`/images/pong-in-web.png` when the array is empty, the schema's default). With `fill`,
`GameEmbed` renders no `.controls-note`; the slug page's line (L48-52) moves into its Help
above the shortcuts, with the page's `saveLine`. `EditorShortcuts.astro` has a `saveLine`
prop and an `<h2>` at L21; `headingLevel` is new. Page prose that moves: `playground.astro:17-24`
(label, h1, lede — the h1 goes to the bar, the label and the lede are dropped, the
`description` prop carries "saved in this browser"), L27-77 the four sections → Help;
`[slug].astro:32-35` (label, h1, lede) and L56-62 Elsewhere → Help. "IndexedDB" is at
`playground.astro:16,23,31` and `docs/roadmap.md:132` ("saved in this browser" there too);
`README.md` § The editor bundle does not say it and keeps its technical register. The
static gates the new markup meets (`postbuild-check.mjs` check 4-6): exactly one `<h1>` per
page (the dialog's heading is an h2), no duplicate ids per page, alt on every `<img>`, no
positive tabindex, curly apostrophes in prose, no word glued to an inline tag across a
source line. `global.css:68-73` gives `canvas { max-width: 100%; height: auto; display:
block }`, which the stage rule's `height: 100% !important` beats along with winit's inline
height. `AppLayout` replicates `BaseLayout.astro:47-65` in its head (the two fontsource
imports, `global.css`, `AccessibilityBootScript`, description/og/canonical/sitemap/rss,
`<title>`); `noindex` adds `<meta name="robots" content="noindex">`. `BaseLayout`'s header
is `position: sticky` (L151) and `AccessibilityControls`' panel is `position: absolute;
top: calc(100% + 0.5rem)` against it (L138-143) — the app bar is `position: relative` so
the panel drops below the bar. The scenario shape (`a11y-scenarios.mjs:122-140`) is
`{ route, seed, label, viewport?, open, waitFor }`: `seed: {}` is required (destructured at
`a11y-check.mjs:92` and `announce-check.mjs:124`), `open` runs in the page and returns
`true` or a string that fails the gate; the phone viewport is `{ width: 390, height: 844 }`
(L204). `screenshot-pages.mjs` imports only `addPopulatedStateInitScript` from that module
and runs no scenarios — the hook is the planner's filing at take-back, not this batch's.
Node here is v24.18.0; the commit hook's threshold is 100 lines
(`scripts/commit-review-hook.sh:35`); `review/playground-ux/` exists in this repo and is
empty, so review numbering starts at 1 there.

- **`AppLayout.astro`** (FaceLayout is the precedent for a second layout): the skip link;
  `<header class="app-bar">` (banner) holding `<nav aria-label="Studio">` with the wordmark
  link, the **visible one-line `<h1>`** (static: "Web Playground"; slug pages "<Game> in
  the editor" — a JS-filled h1 ships unnamed in every audit), a `bar` slot for the page's
  toolbar, and `<AccessibilityControls/>`; `<main id="main" class="workspace"
  tabindex="-1">` with no `.container`; a one-line `<footer>` (contentinfo). Body grid
  `auto minmax(0,1fr) auto` at `min-height: 100dvh` (min-height, so a short viewport
  scrolls vertically, which is ungated, rather than clipping). Not sticky. Props: `title`,
  `description?`, `noindex?`.
- **`PlaygroundToolbar.astro`** (split from `PlaygroundEmbed` L48-85, ids unchanged): the
  project select (optgroup label shortened to "Rust games" — the parenthetical is the
  intrinsic width that caused the sideways-scroll bug at L196-201; its copy becomes a
  `bar-note` beside the select), the save-status `<output id="save-status"
  aria-live="polite" for="project-select">` placeholder (filled in batch 11), **Save**
  (`playground_dispatch("save")` — the hosted save verb the console already reaches; its
  refusal while a batch is open or mid-simulation lands on the banner; the same thing
  Ctrl+S does in the canvas, now visible), `Play ↗` (rendered `disabled` with a `title`
  until batch 6), Export, Import, Reset + note,
  `Fullscreen` (`aria-pressed`, `hidden` until `document.fullscreenEnabled`; targets
  `document.documentElement` so the app bar with its own controls stays visible;
  `fullscreenchange` syncs `aria-pressed`; Escape exits natively; the name never flips),
  `Help` (`aria-haspopup="dialog"`, never disabled — it must open without the engine).
  The field groups keep `min-width: 0; flex: 1 1 auto`; the bar is `flex-wrap: wrap`.
- **`PlaygroundHelp.astro`**: a native `<dialog id="playground-help" aria-labelledby>`
  opened with `showModal()` (the `profile-name-dialog.js` rule: no hand-rolled focus
  management; Escape and focus restore come free), holding the four moved prose sections
  as h3s under its h2 — Saving ("saved in this browser"), Scripts, Rust games in the
  editor, Command API — a "Technical notes" h3 where "IndexedDB" alone survives,
  `<EditorShortcuts headingLevel={3}/>` (a one-line prop on that component), and
  `<form method="dialog"><button type="submit">Close</button></form>`. Styled on the
  `.name-dialog` tokens (`width: min(40rem, calc(100vw - 2rem)); max-height: 85dvh;
  overflow: auto`).
- **`PlaygroundEmbed.astro`**: the banner region and `#game-loading` stay; a `.stage` grid
  cell holds the canvas with
  `.playground-embed :global(canvas) { width: 100% !important; height: 100% !important; max-width: 100%; display: block }`
  — `:global` because winit's canvas has no scoping attribute, `!important` because winit
  writes inline `width`/`height` once at creation and only `!important` beats an inline
  declaration; the canvas box is the stage's grid area, so a window resize, the dock
  opening or fullscreen changes the box and the engine follows on `Resized`. Scripts and
  Console move into `<details id="dock" class="dock"><summary>Scripts and console</summary>
  <div class="dock-body">…</div></details>` as the workspace's third `auto` row — opening it
  shrinks the `1fr` stage, not an overlay (an overlay would cover the editor's own status
  bar); `.dock-body { max-height: min(40dvh, 22rem); overflow: auto }`, two columns from
  66rem; the two sections become `<section aria-labelledby>` with h2s; the console's label
  becomes "Command". `min-width: 0` on `.stage`, `.workspace` and the body grid.
- **Compatibility panel (web#59)**: `src/scripts/webgpu-gate.ts` exports
  `probeWebGpu(): Promise<{ ok: true } | { ok: false; reason: 'no-api' | 'no-adapter' | 'no-device' }>`
  (the adapter + device probe, device destroyed after) and `describeWebGpuFailure(reason)`;
  consumed by `playground-embed.ts`, `GameEmbed.astro`'s script and the preview page.
  `CompatibilityPanel.astro` = `<section id="compatibility-panel" hidden aria-labelledby>`
  with an h2 ("This browser can't run it yet"), `#compatibility-reason`, the Firefox flag
  line, **Try again** (`#compatibility-retry` → `bootPlayground()` again — the current
  `if (src)` body refactored into a function with a `booting` guard; the stage shows before
  `wasm.default()` so winit finds a laid-out canvas; **the button is disabled once
  `wasm.default()` has resolved**, because a page that holds a wasm instance with a
  started event loop cannot boot twice — but a `wasm.default()` that *rejected* (a
  chunked download failing mid-stream is the common case) started no loop, so Try again
  stays live for it; a failure after the init resolved says "reload the page" instead), a
  real labelled screenshot
  (`screenshot` and `screenshotAlt` props: `/images/platformer-in-editor-2026-08.png` on
  the playground, the game's own first screenshot on a game page) and the game-template link as the honest native alternative — nothing in
  the browser can demo without WebGPU. "saved in this browser" replaces "IndexedDB" in the
  page copy and the meta description.
- **`/playground/<slug>/`** pages: `AppLayout`, a reduced bar (an editor-pages nav,
  Fullscreen, Help holding the shortcuts and the "Elsewhere" links), `GameEmbed` gains a
  `fill` prop (no padded or bordered wrapper; the same `!important` canvas rule); the
  1024×720 clamp stays — it only sets the placeholder and winit's one-time inline style.
- **Gates**: `a11y-scenarios.mjs` gains `OPENED_ELEMENT_ROUTES` entries for the help dialog
  on `/playground/` and `/playground/pong/` (open by clicking `#help-button`; `waitFor:
  'dialog#playground-help[open]'`) and the dock open at desktop and at 390 px (`dock.open =
  true`; `waitFor: 'details#dock[open]'`) — axe and announce both run them.
  `screenshot-pages` runs no scenarios, so the open dock's overflow at 320 px is Jesse's
  check, and a scenario hook for that script is filed. Heading order: h1 → dialog h2 → h3s →
  dock h2s. `npm run verify` under Node 24. `README.md` § The editor bundle and
  `docs/roadmap.md` describe the shell.
- Leaves out: `Play ↗`'s behaviour (batch 6), the save status (batch 11), the hint (batch 7).
- Jesse's checks: the keyboard order across the bar (skip link → Studio → Aa → select →
  Play ↗ → Export → Import → Reset → Fullscreen → Help → canvas → dock summary → the
  dock's controls); Escape closes Help and returns focus to the button; Escape leaves
  fullscreen and the button reads not pressed; in Chrome the winit canvas fills the stage
  with no sideways scroll at 390 and 320; resize the window and open the dock — the
  editor's panels re-lay rather than a squeezed image; one screen-reader listen of the
  dialog and the dock; 200% text on a phone.

## Batch 6 — site: Play ↗ opens the preview window, and the v2 bundles (web#58) — DONE 2026-09-10 (insiculous_web f519f3b, insiculous_2d edf055b, one commit per game)

After batch 4 lands. The seven bundles are rebuilt at **`v2`**:
`scripts/build_wasm.sh crates/playground playground --kind playground --version v2
--project examples=Examples=examples --project pong=Pong=crates/playground/assets/projects/pong
--project game-template="Game Template"=../games/game-template --sync ../insiculous_web/public`
(the invocation of record, `docs/WEB_PLAYGROUND.md:28-33`, with the version bumped) and
each game `--kind editor --version v2 --sync ../insiculous_web/public`; the
site moves in one change: `PlaygroundEmbed`'s default `src`, the six `editor:` frontmatter
paths and `postbuild-check.mjs`'s `PLAYGROUND_PROJECTS` constant, with the `v2` dirs in
`public/` in the same diff.

Files, `insiculous_web`: new `src/pages/playground/preview.astro`, new
`src/scripts/playground-preview.ts`, new `src/scripts/playground-preview-protocol.ts`, new
`src/scripts/playground-preview-launch.ts` (the opener's launch and window lifetimes),
`src/scripts/playground-embed.ts`, `src/components/PlaygroundEmbed.astro` (the blocked-popup
alert), `src/components/PlaygroundToolbar.astro`, `astro.config.mjs` (the sitemap filter),
`scripts/postbuild-check.mjs` (the constant and its comment), `src/content/games/*.md` (the
six `editor:` paths), `README.md`, `docs/roadmap.md`, and the seven `public/playground/**/v2/`
directories the builds sync. Files, `insiculous_2d`: `crates/playground/src/web_entry.rs`
(the two constants and the header), `docs/WEB_PLAYGROUND.md` (the invocations and the
contract's prose). Files, each of the six game repos: `src/web_entry.rs` (one constant).

**Re-verified against the tree, 2026-09-10** (`insiculous_web` at `570eaab`, `insiculous_2d` at
`11d3d27`, the six games clean on `jesse`; batch 4's exports are the engine's contract, at
`docs/WEB_PLAYGROUND.md:189-215`). Six corrections, each folded into the bullets' meaning:

1. **The v2 bump is source in eight repositories, not only `public/`.** `build_wasm.sh:106-123`
   hard-fails unless the compiled-in base matches `--version`: the playground's
   `crates/playground/src/web_entry.rs:30` (`ASSET_BASE`) and `:32` (`BUNDLE_VERSION`) with
   the header's five lines (`:4-9`) that quote them; each game's `EDITOR_ASSET_BASE`
   (`games/pong/src/web_entry.rs:48`, `snake:47`, `breakout:46`, `frogger:47`, `asteroids:47`,
   `space_invaders:48`; their header comment is generic and needs nothing). `game-template`
   is a *project source* of the playground bundle, not an editor bundle: its constants stay.
   The docs that quote v1: `docs/WEB_PLAYGROUND.md:29` and `:99-104` (the invocations),
   `:55-64` (the contract's prose), and `insiculous_web/README.md:154`. Each game is its own
   repository on `jesse`: stage the one line there, do not commit; the planner commits each
   (under a hundred lines, no review gate). A `Cargo.lock` a build touches is reported, not
   staged. Bundle sizes to beat: v1 is 10.2 MiB (playground) and 9.8 MiB (pong), the gate
   is 20 MiB. The bundles are tracked (`public/playground/v1` is 28 files in git), so the v2
   dirs are staged; the reviewers' diff excludes `public/` (`git diff --cached -- . ':!public'`)
   and the report carries `git diff --cached --stat -- public` on its own.
2. **The preview page never receives `playground-ready`.** In preview mode `start()` returns
   right after `announce_preview_mode()` (`web_entry.rs:62-71`); the event is dispatched only
   inside `run_playground` (`:258`). So `await wasm.default()` **resolving is the ready
   signal**: `playground-preview.ts` posts `preview-ready` right after it, and registers no
   `playground-ready` listener.
3. **`playground_snapshot(generation: u64)` takes a BigInt.** wasm-bindgen maps `u64` to a JS
   `bigint`, so the call is `wasm.playground_snapshot(BigInt(generation))` with the page's own
   counter kept as a number; type the export
   `(generation: bigint) => Promise<{ sceneEntry: string; bytes: Uint8Array }>`.
4. **There is no cancel export.** `finishLaunch`'s "cancels the pending request" is the page
   dropping the envelope: every continuation checks the generation, and the engine's request
   caps itself at 5 s (`bridge.rs:248`, "the editor did not answer — is its tab visible?").
   A second Play ↗ inside that window after a closure is refused by the engine as another
   generation pending; that rejection goes through `finishLaunch(failed)` and its text lands
   on the banner — correct behaviour, not a defect to work around.
5. **`playground_export_zip()` is already a Promise** (`bridge.rs:380`), and the site's handler
   (`playground-embed.ts:354-363`) still treats it as bytes — against the v2 bundle it would
   zip a Promise object. The `await` is part of this diff, and Export is disabled from the
   click until it settles.
6. **Placement.** `playground-embed.ts` is 489 lines; the opener's two lifetimes go into the
   new `playground-preview-launch.ts`, wired from `wireControls` and given the wasm module,
   the banner, `#play-button`, the blocked alert and the canvas (the exact export shape is the
   executor's one reportable decision). `#play-button` already exists
   (`PlaygroundToolbar.astro:555-560`, `disabled`, with a `title` this batch deletes) and
   enables on `playground-ready` beside Export (`playground-embed.ts:267`). `#preview-blocked`
   is static markup in `PlaygroundEmbed.astro`'s `.banner-region` (`:778-783`), `hidden`, with
   `#preview-retry` inside. `AppLayout` already carries `noindex` (`AppLayout.astro:25`) and the
   `heading` and `bar` slots; the sitemap filter is the regex at `astro.config.mjs:13`;
   `postbuild-check.mjs:148-153` resolves every `data-wasm-src` verbatim (the reason the query
   rides on the page URL), `:155` wants exactly one `<h1>`, `:171` checks duplicate ids per
   file, and `PLAYGROUND_PROJECTS` is `:41-42`. `PlaygroundEmbed.astro:769` is the default
   `src`; the six `editor:` paths are line 6 of each `src/content/games/<slug>.md`. The
   preview page boots orphan check → `probeWebGpu()` → glue, so the audits, which load it with
   no opener, see `#preview-orphan` and a hidden compatibility panel.

- **`preview.astro`** (`AppLayout`, `noindex`, title "Game preview"): the bar slot with
  `<output id="preview-project" aria-live="polite">`, Pause (`aria-pressed`), Restart,
  Close (`window.close()` is legal on a script-opened window); `data-wasm-src=
  "/playground/v2/game.js"` — **`?mode=preview` rides on the page URL**, which is what
  the engine's `query_param` reads; a query on the glue path would fail `postbuild-check`'s
  verbatim join; the same `#game-loading`, `#playground-banner` and `#game-canvas` ids (the
  duplicate-id check is per file) with the canvas labelled "Game preview canvas; focus it
  to play", the compatibility panel, and `#preview-orphan` ("This window is opened by
  Play ↗ on the Web Playground") shown when there is no opener. The gates load it without
  an opener and without WebGPU and audit exactly that.
- **The protocol** (`src/scripts/playground-preview-protocol.ts`, shared by both sides):
  every launch has a **generation** (a counter in the opener, carried in the preview URL
  as `?mode=preview&generation=N` and on every message); messages are
  `preview-ready { generation }`, `preview-snapshot { generation, title, sceneEntry, bytes }`,
  `preview-loaded { generation }`, `preview-failed { generation, error }` and
  `preview-closed { generation }`; a message whose generation is not the current one is
  ignored on both sides. There is **no heartbeat**: a hidden window's timers are throttled
  to about one wake a minute, so silence cannot tell a dead preview from a sleeping one,
  and releasing on silence would let two simulations run. The window name is **per tab**
  — `playground-preview-<tabId>` with `tabId` kept in **`window.name`** (set once when it
  is empty; a reload keeps it, a duplicated tab starts with an empty name and mints its
  own — `sessionStorage` is copied into a duplicate, which is why it is not the identity)
  — so two playground tabs never navigate each other's preview into a document whose
  opener check can never pass. `preview-ready` is **handled only while a launch is in
  flight** and ignored otherwise: a preview the user reloads posts a fresh `preview-ready`
  into a settled opener, and its own 15 s no-snapshot deadline is the right terminal state.
- **The launch and the window are two lifetimes.** Launch states: `idle → opening → ready →
  loaded | failed`. `finishLaunch(generation, outcome)` is the launch's only exit: it
  re-enables the button, clears the in-flight guard, cancels the pending request if any,
  clears the boot deadline, and on `failed` **closes the window if it is still open**
  (`previewWindow.close()` — the opener owns it, and a slow preview that finished booting
  after the deadline must not start behind a failure line) before releasing the
  reservation. The **window lifetime** is the `closed` poll (1 s) plus the reservation: they
  begin when the window opens and end together on `preview-closed` or `closed`, whichever
  comes first — never on `loaded`, so a preview that crashes later is still released the
  moment the user closes its window. Every continuation (the snapshot's resolution, each
  message handler, the poll) checks the generation and the terminal state before acting,
  so a late answer or message is dropped. A blocked popup never enters either lifetime
  (below).
- **A reloaded editor re-takes a live preview.** On a successful open the opener writes
  `sessionStorage['beinsiculous.playground.preview'] = { windowName, tabId }` and removes
  it when the window lifetime ends. At boot, if the key is present **and its `tabId`
  equals this document's own identity in `window.name`** (a duplicated tab copies
  `sessionStorage` but starts with an empty `window.name`, so it must not adopt the
  original tab's preview and refuse its own Play), the editor probes `window.open('',
  name)`: a live named window returns its handle (the empty URL navigates nothing); no
  such window returns a fresh blank one, which is closed at once and the key removed. A
  key with another tab's id is removed without probing. On a live handle the editor re-takes the reservation
  (`playground_set_preview_open(true)`), adopts the handle for the `closed` poll and shows
  "A preview window is still open — close it to Play here". It never messages that preview
  (its generation is gone); closure is what releases it. Without this, a reload of the
  editor tab left a running preview and an editor that would happily Play — two
  simulations with no banner.
- **`playground-preview.ts`**: bail to the orphan message unless `mode=preview`, a
  `generation` and `window.opener`; `probeWebGpu()`; register `playground-ready` before
  `await wasm.default()`; on ready post `preview-ready`; accept `preview-snapshot` only
  when `event.origin === location.origin && event.source === window.opener` and the
  generation matches; `playground_load_preview(bytes, sceneEntry)` → on `Err` post
  `preview-failed` with the error and show it in `#game-loading`; on `Ok` poll
  `playground_preview_state()` every 100 ms: `running` → post `preview-loaded`, set the
  title, enable the controls, focus the canvas with `preventScroll`; `failed: …` → post
  `preview-failed` with the text; still `booting` at a **20 s boot deadline** → post
  `preview-failed` with "the preview did not start". **No snapshot within 15 s of boot** →
  an actionable message ("The editor did not send a scene — close this window and press
  Play ↗ again") with the Close button focused (the opener's own deadline fires too).
  Pause and Restart through the exports (`aria-pressed` from the pause export's return);
  `pagehide` posts `preview-closed`.
- **The opener** (`playground-embed.ts`): `openPreview()` — refused with a banner line
  while a launch is in flight (the button is `disabled` from the click until the
  finalizer runs, so a double click cannot file a second request or close a healthy
  window); otherwise `window.open(url, windowName)` is the **first side effect** of the
  click handler (no await before it); **a null return shows `#preview-blocked` and
  returns before anything is filed** — no request, no reservation, no generation consumed,
  nothing for a finalizer to clear (`role="alert"`, unhidden: "Your browser blocked the
  preview window. [Try again] — or press F5 in the editor to play here"; Retry is its own
  gesture calling `openPreview()`); with a window in hand, `generation += 1`, the state is
  `opening`, the 20 s boot deadline starts, the window lifetime starts (the `closed` poll
  and the sessionStorage key), and `await wasm.playground_snapshot(generation)` (which
  reserves the preview in the engine as it files) is held as the pending envelope; a
  rejection (a running play session, a pending request, the 5 s timeout) goes to
  `finishLaunch(generation, failed)`, which closes the window. `onPreviewMessage`
  (registered once at boot; origin, source and generation checked): on `preview-ready`
  during a launch, post the envelope's `sceneEntry` and `bytes` with a transfer list (or
  when it resolves, if still pending); on `preview-loaded` → `finishLaunch(loaded)` (the
  reservation and the poll stay); on `preview-failed` → `finishLaunch(failed)`; on
  `preview-closed`, or `previewWindow.closed` on the 1 s poll → end the window lifetime
  (release the reservation, remove the key, and `finishLaunch(closed)` if a launch was
  still in flight) then `returnFocusToEditor()`, which bails if
  `document.activeElement` is a textarea, a text-like input, a select or contenteditable,
  else focuses the canvas with `preventScroll`. While the reservation is held the editor
  refuses its own Play, and the banner (not only the status bar) reads "A preview window
  is open — close it to Play here"; **a crashed preview keeps the reservation until its
  window is closed**, which is the honest residual of dropping the heartbeat. **Reuse** is
  the window name: a second Play ↗ (after the previous launch settled) navigates the
  existing window to a fresh document with the new generation, so
  `playground_load_preview` is never called twice in one document and the old document's
  `preview-closed` is ignored by generation. The page never calls `focus()` on the preview
  window — the browser decides window or tab and whether to raise it. **Export** awaits the
  now-live `playground_export_zip()` and the acceptance check re-imports the zip and reads
  the edited value. `isDirty()` is unchanged.
- Docs: `README.md` § The editor bundle (the preview route; "one embed per page" still
  true; the v2 bump); `docs/roadmap.md`. Gates: `npm run verify`; the bundle gate on all
  seven builds.
- Jesse's check is the sprint's acceptance test: change a sample, Play ↗, see the change in
  the other window, Pause, Restart, Close; focus lands on the canvas, and does not when the
  textarea had it; the editor's scene, selection and undo history are untouched; with
  popups blocked the alert and Retry work and the editor's own Play is not refused; Play ↗
  twice reloads the same window; **with the browser set to open the preview as a tab**
  (which backgrounds the editor) the scene still arrives; Export after the preview
  re-imports with the edit and with every other scene present; reload the editor tab while
  the preview runs → the banner says a preview is still open and Play is refused until it
  is closed; duplicate the editor tab and Play ↗ from the duplicate → a second preview
  window, the first untouched; reload the preview window → its 15 s message, the editor
  unaffected.

## Batch 7 — engine + site: the first run (2d#122, web#51)

- Engine: `examples/assets/scenes/behavior_demo.scene.ron` names its five unnamed entities
  (`wall_top`, `wall_bottom`, `wall_left`, `wall_right`, `obstacle`) and frames the scene on
  the player through the scene's main camera; it stays the sample project's first scene. A
  test walks `examples/assets/scenes/*.ron` and pins "every entity in a bundled sample
  scene has a name".
- Site: a dismissable contextual hint above the workspace ("Pick the player in the
  Hierarchy, change its colour in the Inspector, then press Play ↗") — a `<p>` with a
  Dismiss button, remembered in `localStorage` inside try/catch, present at load so no
  live region is needed; the console is already demoted by batch 5's dock and the
  "layout only" copy already sits beside the select — this batch checks both against the
  shipped shell. `/games/<slug>/` gains one paragraph after the embed for entries whose
  frontmatter has `editor:`, the link opening the paragraph so no word glues to a tag,
  curly apostrophe only: "Open <title> in the editor — layout only: the rules are compiled
  in, and nothing you change there persists." (`.editor-link`, mono, dim). Gates: `npm run
  verify`; the playground bundle is re-synced at the sprint's current version so the
  renamed entities reach the site.

**Re-verified against the tree, 2026-09-10** (`insiculous_2d` at `61ef805`, `insiculous_web` at
`ce06776`, both on `jesse`; `v2` is **not deployed** — `origin/main` sits at `c1acf81`, before
batch 5 — so the v2 directory may be re-synced). Twelve corrections, each folded into the
bullets' meaning; where a bullet and a correction disagree, the correction wins:

1. **The scene has no camera entity at all**, so "frames the scene on the player through the
   scene's main camera" means adding one: `EntityData(name: Some("camera"), components:
   [Transform2D(position: (0.0, 0.0)), Camera2D(is_main_camera: true)])` — the syntax is
   `scene_data.rs:176-189` (zoom defaults to 1.0; `viewport_size` is render-managed and
   ignored, `render_manager.rs:343-354`; `hello_world.scene.ron:152-166` is the worked example).
   The player starts at the origin and the arena is 800×600 around it, so a static camera at
   the origin frames the whole arena in the 1024×720 preview and on every native frame. What
   a main camera changes in the editor: Play adopts its pose and zoom and re-arms follow
   (`editor_game/play_session.rs:108-113`); without one, Play only resets zoom to 1.0. The
   editor's viewport at open comes from the prefs, never the scene (`preferences.rs:28-29`;
   `SceneData.editor` is not read at load), so the open view is unchanged. **No
   `CameraFollow`**: the arena is smaller than the viewport, and a following camera would show
   empty space past the walls.
2. **The player needs no tag; add none.** `ChaseTagged` resolves its target by `EntityTag`
   alone (`behavior_runner/mod.rs:283-295`) and the `Player` prefab carries none in the file,
   but `update_player_top_down` tags its own entity `player` every frame
   (`behavior_runner/handlers.rs:97`, applied at `mod.rs:217-220`; the serde default
   `default_player_tag`), so the chasers already chase from the second frame, in the editor's
   Play and in the preview alike. The first draft of this paragraph called that a defect;
   `review-14.md` F1 refuted it. The scene's behaviours change nothing in this batch.
3. **The five unnamed entities are the four walls and the centre obstacle**,
   `behavior_demo.scene.ron:164-195`; the hierarchy shows them as `Sprite (Entity N)`
   (`crates/editor/src/entity_names.rs:21-22`). The names are the bullet's: `wall_top`,
   `wall_bottom`, `wall_left`, `wall_right`, `obstacle`. The header comment's list of what the
   scene demonstrates gains the camera line. It stays the sorted-first scene: `behavior_demo`
   sorts before `hello_world` (`SceneLoader::first_scene_in`, `scene_loader.rs:98-100`).
4. **No test walks the directory today**; `crates/engine_core/tests/scene_loader_parse.rs:68-96`
   (`bundled_example_scenes_parse_and_hello_world_follows_its_player`, file at 165 lines)
   loads the two scenes by name. Extend that test rather than add a file: walk the directory
   with `std::fs::read_dir` for `.ron` entries (a third scene is covered without a code
   change), **sort the entries by file name before any positional assertion** — `read_dir`'s
   order is the filesystem's, and the test's hello_world assertions key off `scenes[0]`
   (`:80-94`) — or find each scene by its name; assert every `EntityData.name` is `Some` and
   non-empty, and assert `behavior_demo` carries a `Camera2D { is_main_camera: true, .. }`. `tests/behavior_fixture.rs:119-133`, `hello_world_golden.rs`
   and `common/src/vfs/tests.rs:239-253` read the scene's name or its path only — untouched.
5. **Engine gates for this batch**: `cargo test --workspace` and both clippy runs as the ground
   rules say; the comment-tag grep; `scripts/check_wasm.sh` is **not** required — no crate
   source changes — and the rebuilt playground bundle is the wasm gate, as in batch 6. The
   games gate does not apply (no public item changes). `editor_prefs.json` at the engine root
   is modified by a local editor run and belongs to nobody: never stage it.
6. **The re-sync is the invocation of record, unchanged**: `docs/WEB_PLAYGROUND.md:28-33` at
   `--version v2` (it rebuilds the wasm and re-copies the three projects). Only
   `public/playground/v2/` moves: the six game editor bundles carry their own game's assets,
   not the examples project, and are not rebuilt. `projects.json`'s `content_hash` for
   `examples` changes with the scene — that hash is how a stored copy is reported as differing
   from the bundle (`playground/src/projects.rs:82`), and no visitor holds a v2 store because
   v2 is not deployed. `git diff --cached --stat -- public` will show `projects.json`, the
   scene file, and `game_bg.wasm`/`game.js` if the build is not byte-stable — the report
   says which. Size to beat 10.2 MiB, gate 20 MiB; no version constant moves.
7. **Site placement of the hint.** `/playground/` is `src/pages/playground.astro` (25 lines):
   `AppLayout` with `PlaygroundToolbar` in the `bar` slot, then `PlaygroundHelp`, then
   `PlaygroundEmbed`. `main.workspace` is a one-row grid (`AppLayout.astro:139-144`), so a
   `<p>` slotted beside the embed would need a second row; the embed's `.head-region`
   (`PlaygroundEmbed.astro:34-49`) is the auto row above the stage and already holds the
   banners and the status line — "above the workspace" is that region, first. New
   `src/components/PlaygroundHint.astro`, rendered first in `.head-region`: a `<p>` carrying
   the settled copy and a `<button type="button">Dismiss</button>`, static markup present at
   load, `hidden` toggled by its own script (the `AccessibilityControls.astro:37-40` and
   `:66-69` pattern — `try`/`catch` around every storage access). Key
   `beinsiculous.playground.hint`, beside `beinsiculous.playground.preview`
   (`playground-preview-protocol.ts:15`). To keep a returning visitor from seeing the hint
   flash before it hides, read the key before first paint with `<script is:inline>` (the
   `AccessibilityBootScript.astro:8` pattern) — that script's shape is the executor's one
   reportable decision. Dismiss hides the button that has focus, so it moves focus to `#main`
   (`tabindex="-1"`, `AppLayout.astro:68`); no live region, nothing announced.
   The game editor pages (`/playground/<slug>/`, `GameEmbed`) do **not** get the hint: its copy
   names the Hierarchy's player and Play ↗, both the Web Playground's.
8. **The two "check against the shipped shell" items hold; nothing to do.** The console is
   behind the closed `<details id="dock">` "Scripts and console" (`PlaygroundEmbed.astro:65-66`)
   and Help names it there (`PlaygroundHelp.astro:62-64`); the "layout only" copy sits beside
   the select (`PlaygroundToolbar.astro:48`, `.bar-note`) and in Help § Rust games in the
   editor (`:53-58`). The report records both as checked.
9. **The game page paragraph.** `src/pages/games/[slug].astro` (170 lines) renders `<GameEmbed>`
   at `:32-39` and `<article>` after it; the paragraph goes between them, conditional on
   `game.data.editor` (schema `content.config.ts:28`; all six entries carry it at line 6),
   the whole sentence inside the `<a>` to `/playground/<slug>/` so no word glues to
   the tag, `.editor-link` styled as `.empty` is there (`:139-143`: `--text-dim`,
   `--font-mono`, 0.9rem). The glued-tag and curly-apostrophe rules are enforced by
   `postbuild-check.mjs:15-21`. The `/games/` list's "edit … on the playground →" button
   (`GameRow.astro:24`) is a different link — the scripts project — and stays.
10. **Docs**: web `docs/roadmap.md:138-139` says "the first run and the save status are what is
    left" — after this batch the save status alone is; web `README.md` § The editor bundle
    (`:143-205`) names the sessionStorage key at `:184-185` — the hint's localStorage key goes
    beside it. No engine doc describes the demo's entities (`README.md:471` names the example
    only); `docs/WEB_PLAYGROUND.md:64` quotes the scene's path, which does not change.
11. **Site gates**: `npm run verify` under Node 24; the hint is present at load, so axe and
    announce audit it on the static page and the screenshot gate checks it wraps at 320 and
    390 px; the comment-tag grep over the touched site files. Touched-file sizes:
    `PlaygroundEmbed.astro` 419, `games/[slug].astro` 170, `playground-embed.ts` 515 (not
    touched by this batch — the hint has its own script).
12. **Reviews and the report.** Two diffs, one per repo, each reviewed in its own subject by
    kimi and codex: the engine's (scene + test) as `insiculous_2d/review/playground-ux/draft-7.diff`
    (numbering continues from 13), the site's excluding `public/` as
    `insiculous_web/review/playground-ux/draft-7.diff` (numbering continues from 4). **One
    report**, `insiculous_2d/review/playground-ux/report-7.md`, carrying both repos' status.
- Jesse's check: on `/playground/` in a fresh profile the hint shows above the stage, Dismiss
  hides it and a reload keeps it hidden; the Hierarchy lists `wall_top` … `obstacle` and
  `camera`; the editor's own Play frames the arena; Play ↗ shows the arena centred in the
  preview with the player at its centre; `/games/pong/` shows the editor line under the
  embed and it opens `/playground/pong/`.

## Batch 8 — docs: the audit reconciled (2d#123)

The executor walks `docs/EDITOR_UX_AUDIT.md` §1–§5 and §7 item by item and marks each
**shipped** (the closing issue or commit, found with `git log -S`), **open** (its issue in
this sprint, or a disposition), or **retired** (with the reason); the doc's head gains a
status banner naming the Studio Board as the only work order; `log_archive.md` gains the
pointer; `PROJECT_ROADMAP.md` § "Editor — UX Audit & Work Order" points at the sprint
instead of the audit. No source changes. Astra reviews the result.

## Batch 9 — engine: the inspector during Play, sections and the colour editor (2d#129, 2d#133)

- **2d#129.** One renderer: `edit_all_components` gains a `read_only` flag on the frame it
  takes (the executor names the type from `stored_component/mod.rs:270` and
  `editable_inspector.rs`); in Play the same rows draw with fields disabled — if `crates/ui`
  has no disabled state for `float_input`, `text_input` and the cycle rows, one `disabled`
  flag on the shared field style, drawn in `text_muted` and ignoring interaction — remove
  buttons hidden, and the heading carries a "live" marker; `inspect_all_components` keeps
  only the dynamic tier's use or is deleted (grep in the report). **Every other mutation
  entry point is closed during Play**, not only the fields: the add-component popup does
  not open, the Name field is disabled, the hierarchy's F2 rename is refused with the
  status line, and any context-menu verb that writes is refused — a component added
  mid-simulation would mutate the live world outside the history, and a rename would be
  restored away by Stop. The scroll offset already survives the Play/Stop boundary. Tests:
  the row set and order are identical in both states for an entity with every removable
  component; the add path, the Name field and F2 are closed while Playing.
- **2d#133.** Component sections collapse (a per-type-name `collapsed` set on the editor
  context, persisted in `EditorPreferences` with `#[serde(default)]` so an older prefs
  file still loads — the convention at `editor_preferences.rs:38-45`; a test round-trips
  an old-format file); an **Advanced** disclosure inside a section
  for fields the registry marks `advanced` (a new per-field attribute in the
  `editor_component_registry!` invocation, default false; first candidates: damping,
  gravity scale, depth, tex_region); the RGBA row's swatch opens a colour editor on the
  **Floating** layer (a window-anchored popup like the add-component popup: a larger
  swatch, four sliders, a hex text field) writing through the same `SetComponentCommand`
  merge. Tests: collapsed sections skip their rows; the popup commits one undo entry per
  gesture.
- Gates: standard engine + wasm. Split into two handoffs if the diff passes a few thousand
  lines.

## Batch 10 — engine: quieter overlays, View toggles, tooltips, resizing, Reset Layout (2d#132, 2d#124)

- **2d#132.** Theme tokens: `grid_primary`/`grid_secondary` and the axes dimmed,
  `collider_outline` toned down, accents (`accent_blue`, `selection_outline`, the play
  border) reserved for selection, focus and runtime state, `surface` separators for
  ordinary panel chrome. The scattered View toggles fold into one
  `ViewToggles { grid, colliders, camera_bounds, snap }` defined in its own
  `crates/editor/src/view_toggles.rs` (accessors, menu sync and preference plumbing there;
  `context/mod.rs`, at 556 today and near the ceiling after batch 2, carries the field
  only), exposed in the View menu with check marks and as small toggle buttons at the
  strip's right. In the preferences the struct carries `#[serde(default)]` and
  `snap_to_grid` keeps its legacy key as a serde alias, so a user's saved snap setting
  survives the move; a test round-trips an old-format prefs file. A camera-bounds overlay is new: the main `Camera`'s
  viewport rect in a muted token. The luminance guard tests extend to the new tokens.
- **2d#124.** A tooltip widget in `crates/ui` (`ui.tooltip(anchor: Rect, text: &str)` drawn
  on `UiLayer::Tooltip` after a ~500 ms hover, one at a time); tooltips on the strip's
  tools, the play controls, the view toggles and the panel headers; the asset browser's
  labels switch to it. Resize handles gain a visible affordance on hover (a 2 px accent
  line) and the resize cursor (`CursorIcon::ColResize`/`RowResize` through the window
  manager — the executor checks what the engine exposes and adds a `requested_cursor` on
  the render context if nothing does). Reset Layout joins the panel chrome (a small button
  at the strip's right end). One set of spacing, field-height, heading and button tokens in
  `layout.rs` and `theme/` used by every panel. Tests: the tooltip appears only after the
  delay and clears on move; `ViewToggles` round-trips through the preferences.
- Gates: standard engine + wasm. Two handoffs if large.

## Batch 11 — engine + site: save state (2d#125, web#60)

- Engine: `playground_save_state() -> String` (JSON `{ "state": "unsaved" | "saving" |
  "saved" | "failed", "reason": "stranded: <path>" | "conflicted: <path>" | "" }`) derived
  in `persist` from the chains and **the same combined signal the window title uses**
  (`mod.rs:369-374`: the command history's dirty flag OR persist pending — two different
  signals, and reading only one would say "Saved" over unsaved editor edits or "Unsaved"
  forever over a clean history with a pending put): any `Conflicted` or `Stranded` path →
  failed with its reason; any `InFlight` or `Queued` → saving; the history dirty → unsaved;
  else saved. A headless test over `Chains` with the gated store walks the four states and
  the four combinations of the two flags. A row in `docs/WEB_PLAYGROUND.md` § The bridge.
- Site: `src/scripts/playground-save-status.ts` — `createSaveStatus(bridge, element)`
  returning `{ poll, current }`, polled from the existing 100 ms `pollResponses` (no second
  interval), writing the `#save-status` output placed in batch 5: "Unsaved changes" /
  "Saving…" / "Saved in this browser" / "Save failed — <reason>"; text set only on change,
  and "Saving…" written only if still saving after 400 ms, so a quick Ctrl+S is not
  announced twice. Export stays one click from it. Gates: engine standard + wasm; site
  `npm run verify`. The bundle is rebuilt: `v2` if it has not deployed since batch 6, else
  `v3`, per the README's immutable-version rule — and **a bump touches every consumer**:
  `PlaygroundEmbed`'s default, the six `editor:` paths, `postbuild-check`'s constant and
  `src/pages/playground/preview.astro`'s `data-wasm-src` (a stale `v2` there would still
  resolve, so the gate would not catch it); the batch report shows
  `grep -rn "/playground/v" src scripts` naming one version.

## Batch 12 — engine + docs: browser usability and the performance budget (2d#126, 2d#127)

- **2d#126.** `docs/WEB_PLAYGROUND.md` § Acceptance: the written list (browser shortcuts,
  focus into and out of the canvas, audio activation, resize, zoom, and "typing in Scripts
  or an inspector field never reaches the game or the editor's shortcuts"), run before each
  playground deploy and recorded in the PR. Engine: a `focus_ring` theme token drawn around
  the focused text field; Tab and Shift-Tab move focus between the inspector's text fields
  (`focus_text_input` exists). Wider keyboard traversal across every widget is filed as a
  follow-up if it does not fit.
- **2d#127.** An idle throttle: in Editing or Paused with no input event for 500 ms the
  editor requests its next frame through a 100 ms timer instead of `requestAnimationFrame`
  (the timer path itself lands in batch 4 for hidden documents; this batch extends it to
  the idle case on a visible one; Play and a running animation are excepted); the bundle
  gate gains a hard size budget (`--kind playground`
  fails past it; Jesse sets it after measuring, default 12 MiB). Jesse measures the four
  numbers on his laptop — download size, time to an editable scene, idle CPU, memory with a
  preview open — and they are recorded in `docs/WEB_PLAYGROUND.md` § Budget.
- Gates: engine standard + wasm + the bundle gate. **The final rebuild covers all seven
  bundles** — the playground and the six game editor bundles, at the version that ships,
  with batch 11's consumer list and grep — because batches 9–12 change the `editor` crate
  and only the playground bundle is re-synced between batch 6 and here; until this batch
  the six `/playground/<slug>/` pages run the batch-4 editor, and batches 9–11 say so in
  their reports rather than rebuilding seven bundles each time.

## Batch 13 — planner only: close-out

Docs (`PROJECT_ROADMAP.md` § Playground UX becomes a shipped paragraph, the engine
`CLAUDE.md`'s editor bullets, `insiculous_web/docs/roadmap.md`, its `README.md` § WASM
builds for the version), the ledger closed with its number, every issue closed with the
commit named, follow-ups filed, `jesse → dev` in both repos,
`scripts/check-sprint-sync.sh`. Jesse pushes; the staging deploy; Jesse's acceptance run;
`dev → main`.

## Verification (end to end)

- Every engine batch: the four cargo gates, the comment-tag grep, `check_wasm.sh`,
  `check_games.sh` when a shared crate's public item changes, files ≤ 600 lines.
- Every site batch: `npm run verify` under Node 24, plus the PR template's manual pass by
  Jesse on each new interaction.
- Every rebuilt bundle: `build_wasm.sh` warns nothing past 20 MiB; `postbuild-check`
  resolves every `data-wasm-src`.
- The sprint's acceptance test, by Jesse on staging: change a sample, Play ↗, see the
  change in the other window, close it, the editor is untouched, Export downloads.
- Each batch's staged diff: kimi and codex reviews, the planner's own review, adjudication,
  fixes applied by the planner, a pathspec-scoped commit with the review asserted.
