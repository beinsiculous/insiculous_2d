# Pattern Audit — clearing the game-programming-patterns skill's five tickets

Effort directory: `insiculous_2d/coordination/pattern-audit/` (the plan; the reviewer ledger
`reviewer-comparison.md` beside it). Review conversations: `insiculous_2d/review/pattern-audit/`
for the plan, every engine diff and every handoff; `games/breakout/review/pattern-audit/` for
batch 4's breakout half (each repo's `request-review.sh` refuses an artifact outside its own
`review/`). Delivery runs the handoff loop: Claude Fable plans and commits; kimi reviews every
plan version and every batch; gemini (agy) joins on every code batch here, because each one
changes a public seam, a test harness or a fixture; a Claude Opus session driven by Jesse, or
agy, executes one batch per handoff (if agy executes a batch, deepseek stands in for gemini on
that batch's review — a vendor never reviews its own work). No batch has a screen or an asset
in it, so codex is not on the roster; Jesse may add it for batch 3's inspector rows.

Plan history: v1 reviewed by kimi (`review-1.md`, 8 findings) and gemini
(`review-1-gemini.md`, 6) on 2026-10-19, adjudicated in `rebuttal-1.md` — kimi 5 real, 2
false, 1 policy; gemini 5 real, 1 false. What changed in v2: a scene carrying one of the three
retired audio names loads with a warning instead of refusing; override merging is decided
rather than left to the executor (struct variants merge by the keys the text names,
`Behavior` and `Scripts` replace whole, an unknown key is a load error); breakout's
`levels_tests.rs` loses its replica of the merge and calls the engine's; the games gate
exits 0 over all seven games (game-template is cloned, the v1 premise was stale); the
fix-delta rule is stated; `check_features.sh` carries `--all-targets`; ron is 0.12.0; batch 2
records the before/after timing argument, the `GlobalTransform2D` reader grep and the
`game_impl.rs` comment. Rebutted with evidence: ProjectHost runs inside `EditorGame` through
`run_game`, so the tail covers it; the guide gate ran green (85 citations); `ron::Value`
parses a named struct as a map; a gizmo drag lags one frame in both schemes. v2 reviewed by
kimi (`review-2.md`, 5) and gemini (`review-2-gemini.md`, 6), adjudicated in `rebuttal-2.md` —
all 11 accepted: the reset marker is filed on every wholesale world replacement (Open, New,
the preview's load, Stop), which fixes a live editor bug — a re-opened scene whose entity ids,
locals and parents coincide with the last one's kept stale globals; the loader stops writing
identity globals (the tail adds the real one before the first render); the merge's edge
rules are stated — an override with no prefab counterpart appends whole, a typed/`Dynamic`
same-name match is a load error, `Dynamic` data merges key by key against the registered
default object, retired names are stripped from every layer first; batch 4 enumerates every
override block mechanically, pins the golden fixture's instances in a test, states the
two-tree staging order and names the engine↔breakout pairing in both commits.
**This is the settled plan** (Jesse, 2026-10-19, two rounds: 14 + 11 findings, 21 accepted,
3 rebutted as false, 1 as policy). Corrections from here go into the acting batch's section
before its handoff.

## Context

The `game-programming-patterns` skill landed in `insiculous_2d` on 2026-10-19 (65084dd,
13f96f2; audit against bce5abc). Its audit fixed the small things inline and filed five
tickets, insiculous_2d#168–#172, each a place where the engine advertises a pattern it does
not actually run. Jesse's ask: a plan to clear them, and to pull in the other open debt
that fits (2026-10-19). Everything below was verified against the tree on 2026-10-19
(engine at 13f96f2, branch `jesse`, machine Iroh).

| # | pattern | the defect | Jesse's call (2026-10-19) |
|---|---|---|---|
| 170 | update-method | `World::update`, `SystemRegistry`, `add_system`, `Scene::update` have zero production callers; `add_system` starts a system that never ticks | **delete the scheduler** |
| 168 | dirty-flag | only `editor_integration` steps `TransformHierarchySystem` (twice per Play frame); `run_game` never does, so a parented sprite in a shipped build draws at the origin | **fix: the frame tail owns it** |
| 171 | state | `HierarchicalStateMachine` is exported from three preludes; its one user is the platformer example, which only displays the parent group; Play/Stop drops it | **delete it** |
| 172 | event-queue | `PlaySoundEffect`/`AudioSource`/`AudioListener` are registered, inspectable, and consumed by nothing; their `sound_id: u32` is a per-session handle, so a saved scene cannot even name a file | **retire them; scene audio arrives with Conductor** |
| 169 | prototype | an override replaces the whole component; breakout's 436 brick lines restate `scale: (0.8, 0.4)`; a prefab edit reaches no instance | **fix on load + `spawn_prefab`; the save side is filed, not built** |

Looped in from the open debt (Jesse, 2026-10-19): **#100** (a gate for the guides' pitfall
tables, which batches 1–3 edit), **#84 BUILD-002** (a standing no-physics check),
**#153** (key-handler events, resolved by contract), and these backlog ticks where a batch
already holds the file: **#86 DRY-003** (hierarchy sin/cos), **#86 GPP-L1** (allocating
`entities()` in read-only passes), **#84 DOC-001** (three doc gaps), **#84 SIZE-002** (already
true on disk — `game.rs` is 550 lines), **#84 KISS-002** (the lifecycle manager's Clone
semantics, documented), **#88 GPP-L8** (`GlyphInfo` duplicating its cache key).
Left out, with the reason: **#110** (Behavior variants as scripts — Large, its own effort),
**#114** (a criterion bench — a new dependency, and the data-locality rule says wait for the
trigger), the trigger-gated GPP items on #84/#85/#93 (render interpolation, tag scans,
pools — the book's own advice is to wait for the profiler).

## Decisions of record (taken with Jesse, 2026-10-19)

1. **One update path.** The frame tail (`GameRunner::step_simulations`) is where engine
   systems tick, by hand, in a fixed order. `SystemRegistry`, `SimpleSystem`,
   `World::add_system`, `World::update`, `World::system_count`, `Scene::update` and
   `Scene::update_with_schedule` are deleted. The `System` trait stays (the tail, physics and
   the hierarchy implement it). `World::initialize/start/stop/shutdown` stay as lifecycle
   flags (the window loop and the headless runner call them). Panic isolation between
   systems retires with the registry: a panicking engine system is a bug to fix, not a
   thing to survive.
2. **The runner owns the transform hierarchy.** `GameRunner` gains a
   `TransformHierarchySystem` field stepped in the tail after `step_world_systems` and
   before `grid_backdrops.update` (the backdrop reads `GlobalTransform2D`). `EditorGame`
   and `ProjectHost` drop their copies. A wholesale world replacement — Stop's snapshot
   restore, Open Scene, New Scene, the preview's load — files a World-resource marker,
   exactly like `GridBackdropReset`: `ecs::request_transform_hierarchy_reset(world)` inserts
   `TransformHierarchyReset`, and `TransformHierarchySystem::update` consumes it before
   propagating. The case that needs it is the snapshot restore: it recreates entities
   under their old ids with the same locals in the frame of the clear, while the cache is
   still warm. Open, New and the preview's load file it as a guard only — entity ids come
   from a process-wide counter and are never recycled (`ecs/src/entity.rs`,
   `ecs/CLAUDE.md:15`), so a loaded scene's fresh ids are never in the cache. (The plan's
   v3 wording claimed a live re-open bug here; batch 2's executor showed the premise false,
   and the record was corrected on 2026-10-19.) The scene loader stops attaching
   `GlobalTransform2D::default()` to parented entities: the tail's first propagation adds
   the real global before that frame's render.
3. **Overrides merge field by field, at the top level only.** `position` as a whole, never
   `position.x`. Presence comes from the source text, not from comparing against defaults:
   an explicit `scale: (1.0, 1.0)` in an override is honoured (the alternative — treating
   a default-valued field as absent — is a silent trap and is refused). Only struct variants
   merge; the two newtype variants, `Behavior` and `Scripts`, replace whole as today. An
   override key the component lacks is a load error naming the entity, the component and
   the field — a typo is no longer skipped in silence. Load and runtime `spawn_prefab`
   share one merge, and so does breakout's level test. The inline `components` layer keeps
   replace semantics. The edges: an override whose component the prefab lacks (or an entity
   with `prefab: None`) is appended whole, with serde defaults for what it omits — today's
   behaviour, the same as a `components` entry; a type-name match between a typed entry and
   a `Dynamic` one is a load error; a `Dynamic` override merges `data` key by key, each key
   validated against the registered type's default object, a non-object payload refused;
   and the retired audio names are stripped from every layer before any merge.
   The serializer keeps flattening prefabs; minimal-override saving needs a prefab-origin
   record and is filed as a follow-up, not built here.
4. **No scene-authored audio in this effort.** The three components, their registry rows,
   editor rows and docs go. A saved scene that still carries one of the three names — a
   playground project in IndexedDB, an older editor save — loads with a warning naming the
   entity and the dropped component, never a refusal: the loader keeps a list of retired
   component names for exactly this. Scene audio is a feature — a path-based sound id, a sound tier
   in `AssetManager`, an asset-browser kind, a public handle lookup, spatial playback the
   crate lacks — and it is filed under the Conductor (Phase K) milestone, which Jesse named
   as the next sprint.
5. **Key handlers set state; they never emit.** #153 is resolved by contract on the `Game`
   trait and a harness test that pins the flush order, not by a typed next-frame lane in
   `EventBus`.
6. **Gates land before the batches that need them.** `scripts/check_guide_tests.sh` (#100)
   and `scripts/check_features.sh` (BUILD-002) are batch 0's, planner-built, under the
   review threshold, so every later batch runs them.
7. **Backlog ride-alongs are ticks, not scope.** A ride-along that turns out non-trivial
   (a reader of `GlyphInfo.character`, a mutating `entities()` loop) is kept as is and
   reported, never forced.
8. **A fix after review is re-reviewed only when it changes behaviour** (Jesse, Sep 3 2026,
   reaffirmed 2026-10-19). A fix that changes behaviour or grows past a few dozen lines goes
   back through the reviewers as its own diff; a doc line, a rename, a pin, a deletion or a
   signature the finding specifies is enumerated in the rebuttal, gated, and committed
   without a second round.

## Ground rules for every batch

- Gates, engine: `cargo test --workspace` (0 failed, 0 ignored), `cargo clippy --workspace
  --all-targets` (0 warnings), `cargo clippy --features editor --all-targets` at the root
  (the only gate that compiles `src/bin/editor.rs`), `cargo check --workspace --all-targets`
  (the examples compile under it), every touched file ≤ 600 lines, no new `#[allow]`, no
  `unwrap()` outside tests, **no new dependency**. The comment-tag gate:
  `grep -riEn "kimi|codex|astra|gemini|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|#[0-9]{1,4}\b|Sprint [0-9]" crates src examples --include=*.rs`
  prints nothing (a hex literal or a string match is inspected by hand). From batch 1 on:
  `scripts/check_guide_tests.sh` and `scripts/check_features.sh` print nothing and exit 0.
- Wasm gate `scripts/check_wasm.sh` whenever the staged diff touches `crates/engine_core`,
  `crates/renderer`, `crates/audio`, `crates/input` or `crates/common` (every batch here but
  batch 1 does; run it on batch 1 anyway, it is cheap). Games gate `scripts/check_games.sh`
  whenever a public item of `engine_core`, `ecs`, `physics`, `input`, `common` or `renderer`
  changes (every batch), `--test` when behaviour they exercise changes (batch 4). Verify
  `../games` resolves to the working set first. The gate exits 0 over every game in its
  list, `game-template` included (all seven are cloned; green on the current tree,
  2026-10-19) — no waiver, and a red line stops the batch.
- Review: the commit hook denies unreviewed commits over 100 changed lines. Every code
  batch goes `git diff --cached > review/pattern-audit/draft-<batch>.diff`, kimi review
  (detached when the diff is large), gemini review, the planner's own review, adjudication
  with Jesse, `rebuttal-N.md`, fixes applied by the planner (decision 8: a behaviour
  change goes back through review as its own diff; anything narrower is enumerated in the
  rebuttal and not re-reviewed), then
  `ADV_REVIEWED=1 git -C insiculous_2d commit -F <message-file> --pathspec-from-file=<scope-file>`.
  Never skip trailers. The breakout half of batch 4 is reviewed and committed the same way
  from `games/breakout`.
- Every commit is pathspec-scoped. The plan's "done" marks are their own commits
  (`-- coordination/pattern-audit/plan.md`).
- Before each handoff the planner re-verifies the batch section against the tree (every
  `file:line`, every symbol, every count) and commits corrections into the section. One
  batch out at a time; the planner neither edits nor runs cargo in that checkout while a
  batch is out.
- Every deletion of a public item shows its grep in the report
  (`grep -rn "<symbol>" crates src examples ../games --include=*.rs | grep -v target/`). Before
  deleting the only test of a live API, keep one strengthened, contract-named test.
- Docs match reality at every commit: a guide, a `training.md` line, a skill reference or a
  pitfall-table row describing a thing a batch changed is a defect in that batch. The five
  reference files under `.claude/skills/game-programming-patterns/references/` are docs like
  any other — each batch rewrites the pitfall that cites its ticket.
- Anything deferred is filed with `/file-issue` before the effort reports done.

## Batch 0 — planner only: the effort, the sprint, the two gates, the plan review — DONE 2026-10-19 (def7ed6 gates, 2c5d3b2 plan; milestone 12 + breakout#6 on milestone 3; Sprint option `4e8e02e5`; two review rounds)

Files: `coordination/pattern-audit/plan.md` (this), `coordination/pattern-audit/reviewer-comparison.md`
(the empty ledger, header copied from `coordination/playground-ux/reviewer-comparison.md`),
`review/pattern-audit/` (created), `scripts/check_guide_tests.sh` (new),
`scripts/check_features.sh` (new), `.claude/skills/finish-task/SKILL.md` § Gate 3 (two
lines), `CLAUDE.md` § Quick Reference (two command lines).

- **The sprint.** Milestone **Pattern Audit** in `insiculous_2d` holding #168–#172, #100 and
  #153 (#100 moves off Debt Sweep (Mediums) — Jesse's call); description in the
  `sprint-planning` § 4a shape: one line, ORDER = batches 1→2→3→4 (each gates the next
  only by the one-batch-out rule; #170 before #168 because both touch what the tail steps),
  gates = the engine gate list above, excluded = #110, #114, the trigger-gated GPP items;
  the description names the batch 4 pairing (the engine's override merge and breakout's
  regenerated scenes revert together).
  The board's Sprint field set on each (`gh project field-list 1 --owner beinsiculous` for
  the option; add it if absent). A breakout issue is filed for batch 4's half ("the bricks
  stop restating the prefab scale once overrides merge") and a milestone of the same name
  created in `beinsiculous/breakout` so `scripts/check-sprint-sync.sh` holds.
- **`scripts/check_guide_tests.sh`** (#100): the loop from the issue body, over every
  `crates/*/CLAUDE.md` "Pitfalls and their guard tests" table — each `src/… test_name` or
  `tests/… test_name` citation must name a file that exists and a `fn test_name` inside it;
  `— none` rows pass; one line per miss, exit 1 on any. Green on the current tree
  (2026-10-19: 85 citations resolve; a negative fixture proved it reports a renamed test and
  a moved file). Listed in `/finish-task`
  Gate 3 beside the tag grep, and in `CLAUDE.md` § Quick Reference. Closes #100.
- **`scripts/check_features.sh`** (#84 BUILD-002): `cargo check -p engine_core
  --no-default-features --all-targets` and the matching clippy with `-D warnings`, exit on
  the first failure. Green on the current tree (2026-10-19, both commands). Ticks BUILD-002
  on #84. Listed beside the wasm gate.
- **The plan review.** `scripts/request-review.sh plan coordination/pattern-audit/plan.md
  --reviewer=kimi` and `--reviewer=gemini` on the same snapshot (the plan is copied to
  `review/pattern-audit/plan.md` for the script; the tracked file is the record). Adjudicate,
  `rebuttal-N.md`, revise, repeat until Jesse calls it settled; record the history in the
  header above.

Leaves out: nothing in `crates/`. Both scripts plus the doc lines stay under 100 changed
lines, so they commit without a code review; if they do not, they get one.

## Batch 1 — ecs + engine_core: delete the scheduler (#170; KISS-002 doc) — DONE 2026-10-19 (6b1ccb7)

Authored by Jesse's Claude Opus session from `review/pattern-audit/handoff-1.md`; reviewed by
kimi (`review-3.md`, 2 findings: 1 accepted and folded into batch 2 — the `System` trait's
unused hooks — 1 false: the game template is clean), gemini (`review-3-gemini.md`, 4: 2
accepted — a stale `LifetimeSystem` doc in a file the batch never touched, and the trait's
doc — 1 false, 1 policy) and the planner (`review-3-claude.md`, none); adjudicated in
`rebuttal-3.md`. The planner's two doc-line fixes (`lifetime.rs`, `system.rs`) enumerated
there, not re-reviewed. Landed as specified; the executor also reworded the trait's hook
docs and renamed the scene lifecycle test to what it tests. 1105 tests, every gate green.

Files: `crates/ecs/src/system.rs` (235 → ~35), `crates/ecs/src/world.rs` (511, shrinks),
`crates/ecs/src/lib.rs` and `crates/ecs/src/prelude.rs` (neither re-exports `SystemRegistry`
or `SimpleSystem` — verified 2026-10-19; `world.rs` names `crate::system::SystemRegistry`
directly — so only `pub mod system` and any doc mention are touched there), `crates/engine_core/src/scene.rs` (`update` and
`update_with_schedule`, lines 123–147), `crates/engine_core/src/lifecycle.rs` (doc only),
`crates/ecs/tests/system_lifecycle.rs` (149 lines, deleted or reduced to one test),
`crates/engine_core/tests/scene_lifecycle.rs` (54, reworked), `crates/ecs/CLAUDE.md`,
`crates/ecs/src/hierarchy_system.rs:47-49` (doc), the skill's `references/update-method.md`.

**Target shape.**
- `system.rs` keeps `pub trait System` (lines 1–34) with a module doc that says what it is
  now: the contract every engine system implements, stepped by whoever owns it — the frame
  tail for the engine's, a game's `update` for its own. `SimpleSystem` (36–71) and
  `SystemRegistry` (73–236) are deleted.
- `World` loses the `systems` field (42, 67), `with_systems` (76–86), `add_system`
  (325–337, doc from 320), `update` (339–351) and `system_count` (371–374). `initialize`, `start`, `stop`
  and `shutdown` keep their flag transitions and their `EcsError` variants
  (`AlreadyInitialized`, `NotInitialized`, `AlreadyRunning`, `NotRunning`); the
  `with_systems` calls and the "N systems" log fragments go. `World::clear`'s doc drops
  "does not affect systems".
- `Scene::update` and `Scene::update_with_schedule` are deleted; `Scene::initialize`,
  `start`, `stop`, `shutdown`, `is_operational` stay (callers: `app_handler.rs:250-251,287,290`,
  `headless.rs:32-33`).
- **Tests.** `system_lifecycle.rs`: every test drives `add_system`/`World::update`. The
  World lifecycle flags are live API; `scene_lifecycle.rs:26-30` already asserts the happy
  path through `Scene`, so what survives is the error path — one strengthened test,
  renamed `tests/world_lifecycle.rs`:
  `test_world_lifecycle_flags_follow_initialize_start_stop_shutdown_and_refuse_out_of_order_calls`
  (initialize twice → `AlreadyInitialized`; start before initialize → `NotInitialized`; stop
  when not running → `NotRunning`; shutdown from running stops first). The panic-isolation
  test is deleted with the registry it tested. `scene_lifecycle.rs` keeps the state
  transitions and loses its `update`/`add_simple` lines.
- **Docs.** `crates/ecs/CLAUDE.md:79` (the `World::update` swap row) is deleted —
  `check_guide_tests.sh` would otherwise fail on the orphaned citation. `ecs/CLAUDE.md` §
  File Map's line for `system.rs`, if any, says "the `System` trait". `hierarchy_system.rs:47-49`
  drops "the runtime never calls `World::update`, so a system registered with `add_system`
  is started but never ticks" for "nothing steps a system you do not step yourself; the
  engine's own run from the frame tail" (batch 2 rewrites it again for the hierarchy — that
  is fine, each batch leaves the doc true). `update-method.md` pitfall at 125–133 ("`World::update`
  never runs in production", three sub-bullets) becomes: there is no scheduler; a system
  ticks where something calls `update` — the frame tail for the engine's systems (named, in
  order), `Game::update` for a game's own; cite `frame_tail.rs step_world_systems`. The
  SKILL.md "update-method" row already says "run as a fixed ordered list in the frame tail".
  `training.md` has no mention of the registry (verified).
- **KISS-002 (#84), doc half.** `lifecycle.rs:51-66`'s `impl Clone for LifecycleManager`
  gets a `///` stating the contract: a clone copies the state and starts with its own,
  unlocked guards — the locks are per instance, never shared. No simplification.

**Gates:** the standard list; `check_games.sh` (a public `ecs` item is removed; no game uses
it — the grep is in the report); `check_wasm.sh`; `check_guide_tests.sh` must be green after
the row deletion.

**Reviewers:** kimi + gemini (a public seam and a test file are deleted).

**Leaves out:** any change to what the tail steps (batch 2); `World`'s other backlog items
(SRP-011, API-010, API-011 on #86 — speculative restructures, untouched).

Size: about −470 / +60 lines.

## Batch 2 — engine_core + ecs + editor_integration + physics: the frame tail owns the hierarchy (#168; #153; the `System` trait trim; DRY-003; GPP-L1) — DONE 2026-10-19 (1809f87)

Authored by Jesse's Claude Opus session from `review/pattern-audit/handoff-2.md`; reviewed by
kimi (`review-4.md`), gemini (`review-4-gemini.md`) and the planner (`review-4-claude.md`);
adjudicated in `rebuttal-4.md` — kimi 2 findings, both rebutted with evidence (the games gate had compiled every game; the one-frame window is the accepted ruling); gemini 4, 1 accepted (a doc line the trim had made stale), 3 rebutted (the headless API never had a frame loop; the window; frame-1 globals are an improvement on never); the planner none. The executor found the plan's Open/New
premise false (ids are never recycled; see decision 2) and kept the marker there as a guard
under an honest test name; it kept `initialize` on the trait because three example lines
called it, and the planner completed the trim before dispatch — the trait is `update` alone,
the examples lose the calls and the platformer its own hierarchy system. 1110 tests, every
gate green.

**Re-verified against the tree 2026-10-19 after batch 1 (6b1ccb7):** `hierarchy_system.rs` is
345 lines — `reset` at 123, `impl System` at 154, `update` at 160, `is_dirty` at 285–301,
`set_global_transform` at 312–318; the loader's two default-global writes are at
`scene_loader.rs:162` and `:243`; the dirty-flag pitfalls are at `dirty-flag.md:135-139`
("The hierarchy runs only under the editor") and `:143-145` (the `reset()` bullet); the
event-queue flush pitfall is at `event-queue.md:137-140`, not 118–122; `on_key_pressed` /
`on_key_released` are `game_trait.rs:89, 92`; every other line below held.

Files: `crates/ecs/src/hierarchy_system.rs` (345 → ~380), `crates/ecs/src/hierarchy.rs`
(234, sin/cos), `crates/ecs/src/system.rs` (40: the trait trim), `crates/ecs/src/lifetime.rs:64`,
`crates/ecs/src/sprite_system.rs:50`, `crates/ecs/src/clip_state_machine.rs:281` (their
`name()` impls), `crates/physics/src/physics_system/update.rs` (12–15, 76–83: its
`initialize`, `shutdown`, `name` impls), `crates/engine_core/src/scripting/tests/contracts.rs`
(290, 383: `physics.initialize(&mut world).unwrap()` lines), `crates/ecs/src/prelude.rs`, `crates/engine_core/src/prelude.rs` (70 → add the
marker and the request), `crates/engine_core/src/game.rs` (550: one field, one constructor
line), `crates/engine_core/src/game/frame_tail.rs` (163: one step in `step_simulations`),
`crates/engine_core/src/game/game_trait.rs` (handler docs, the render loop's iterator),
`crates/engine_core/src/tilemap_render.rs:22`, `crates/editor_integration/src/editor_game/mod.rs`
(469: lines 54, 119, 354), `crates/editor_integration/src/editor_game/play_session.rs` (170,
214), `crates/editor_integration/src/editor_game/play_session_tests.rs` (91–116),
`crates/editor_integration/src/editor_game/scene_io.rs:215-224` (`reset_session`),
`crates/editor_integration/src/project_host/preview.rs` (99, 130),
`crates/engine_core/src/scene_loader.rs` (157–164, 241–244: the default-global writes, removed),
`crates/editor_integration/src/project_host.rs` (398: lines 35, 46, 86, 124, 141),
`crates/editor_integration/src/editor_game/script_status.rs:24`,
`crates/editor_integration/src/editor_game/game_impl.rs:96-99` (the comment naming
`prepare_frame` as the writer of every pickable's `GlobalTransform2D`), a new
`crates/engine_core/tests/transform_hierarchy_tail.rs`, `crates/ecs/tests/hierarchy_dirty.rs`
(one new test), `crates/engine_core/CLAUDE.md:32`, `crates/ecs/CLAUDE.md:38` and its pitfall
table, `crates/editor_integration/CLAUDE.md:26`, the skill's `references/dirty-flag.md`
and `references/event-queue.md`.

**Target shape — #168.**
- `ecs::hierarchy_system` gains `pub struct TransformHierarchyReset;` and
  `pub fn request_transform_hierarchy_reset(world: &mut World)` (inserts the marker), the
  mirror of `grid/backdrop_system.rs:26-39`. `TransformHierarchySystem::update` (161) begins
  with `if world.remove_resource::<TransformHierarchyReset>().is_some() { self.reset(); }`,
  the mirror of `backdrop_system.rs:76`. Both re-exported from `ecs::prelude` and
  `engine_core::prelude`.
- `GameRunner` gains `transform_hierarchy: ecs::TransformHierarchySystem` beside `particles`
  (game.rs ~228, constructed ~345 with `TransformHierarchySystem::new()`).
  `step_simulations` (frame_tail.rs:41-62) steps it **after** `step_world_systems` and
  **before** `self.grid_backdrops.update` (the backdrop reads `GlobalTransform2D`,
  `backdrop_system.rs:118-128`). It is passed `scaled_delta` for the signature and ignores
  it, so a paused game and the editor's Editing state (time_scale 0) still propagate. The
  doc on `step_simulations` names the order and the reason.
- `EditorGame` drops `transform_system` (mod.rs:54, 119) and the step at 354.
  `play_session.rs:170`'s `self.transform_system.reset()` becomes
  `ecs::request_transform_hierarchy_reset(world)`, placed beside `request_backdrop_reset`
  (214) with the existing comment's reason kept (the world was wholesale-replaced; no stale
  baseline may survive). **The same request is filed in `reset_session`
  (`scene_io.rs:215-224`, which Open Scene and New Scene share) and in the preview's load
  (`project_host/preview.rs:99`, beside its `request_backdrop_reset` at `:130`).** Today
  only Stop resets the cache; Open and New call `world.clear()` (`scene_io.rs:178, 286`),
  which restarts entity ids at 0 (`world.rs:428-432`), and `is_dirty` compares only
  `(local, parent)` (`hierarchy_system.rs:295-301`), so a re-opened scene whose entity at
  id *k* shares those with the last scene's id *k* kept its stale global and sat at the
  origin until nudged. **This premise was false** (batch 2's executor, 2026-10-19): ids are
  never recycled, so Open and New cannot revive one; the marker there is a guard, and the
  real case is the snapshot restore. The warm-cache editor test below is therefore named
  for what it can pin — Open files the marker and every global matches its local.
- **The loader stops writing identity globals.** `scene_loader.rs:157-164` and `241-244`
  attach `GlobalTransform2D::default()` to every parented entity; both go. The tail's first
  propagation adds the real global (`set_global_transform` adds when missing,
  `hierarchy_system.rs:313-319`) before that frame's render, so nothing ever draws at the
  origin, and a freshly loaded parented entity is unpickable for the one frame before the
  tail — as a root is today — instead of pickable in a cluster at the origin. `ProjectHost` drops `transform_hierarchy` (35, 46, 124, 141); its
  `reset_play_state` gap closes by construction. **Every host runs through the runner's
  tail:** the standalone editor and the playground wrap ProjectHost in `EditorGame` and call
  `engine_core::run_game` (`editor_game/run_options.rs:78`, `playground/src/web_entry.rs:300`,
  `src/bin/editor.rs:14`), the preview calls `run_game` directly
  (`playground/src/preview_entry.rs:70`), and `GameRunner::initialize_and_update` runs
  `post_update` after every game's `update` (`game.rs:508-514`) — so the one runner-owned
  step covers ProjectHost, the preview and every shipped game.
- **Timing, before and after.** Today the editor steps at the start of frame N+1
  (`prepare_frame`); the tail steps at the end of frame N. Nothing moves a local between
  those two points except a key handler, so a gizmo drag — which runs after `prepare_frame`
  inside `EditorGame::update` — leaves a child's global one frame stale in both schemes.
  The one real difference is the single frame after a scene load, before the tail has run
  once: picking (`viewport_interaction.rs:336`, `gizmo_drag.rs:59`) sees no globals on the
  loaded entities for that frame, so they are unpickable until the tail has run — which,
  with the loader no longer writing identity globals, is the whole effect. Accepted (Jesse,
  2026-10-19) and documented in `editor_integration/CLAUDE.md` and the `game_impl.rs:96-99`
  comment, which now names the frame tail as the writer.
  Entities the editor creates carry `GlobalTransform2D::default()` already
  (`entity_ops.rs:29`, pinned "pickable from frame one" at `:275`). The report carries a
  grep of every `GlobalTransform2D` reader under `crates/`, each classified as reading
  before or after the tail.
- **Tests.** `crates/ecs/tests/hierarchy_dirty.rs` gains
  `test_the_reset_marker_is_consumed_and_the_cache_rebuilt_from_scratch` (cache warm → entity
  teleported without going dirty through the cache's eyes → marker → update → global matches
  the new local; marker gone). `play_session_tests.rs:91-116`
  (`test_stop_resets_the_transform_propagation_baseline`) is rewritten to assert Stop files
  the marker (`world.has_resource::<TransformHierarchyReset>()`), since the editor no longer
  owns a system to inspect — the ecs test covers what the marker does. New
  `crates/engine_core/tests/transform_hierarchy_tail.rs` through `GameHarness` (model:
  `tests/headless_game.rs:56`): a game whose `init` creates a parent and a child
  (`WorldHierarchyExt::set_parent`, `Transform2D` on both), one `step` → the child's
  `GlobalTransform2D.position` is the parent's plus its local; move the parent in `update`,
  step → it followed; named
  `a_parented_childs_global_transform_follows_its_parent_with_no_editor_present`. A second
  test loads a scene string with a `children:` entry through `SceneLoader` inside the
  harness's `init` and asserts the child has no `GlobalTransform2D` before the first step
  and the propagated one after it. In `editor_integration`, a warm-cache test:
  `test_reopening_a_scene_with_coinciding_ids_recomputes_every_global` — load scene A, step
  once, load scene B whose entity at the same id has the same local and parent but a
  different authored position elsewhere in the tree (or simply re-open A after moving an
  entity), step, and assert every global matches its local.
  Every other test that touched `editor.transform_system` or `host.transform_hierarchy`
  (the executor greps) is reworked the same way.
- **Docs.** `engine_core/CLAUDE.md:32` (the frame_tail entry): the hierarchy step, its place
  in the order, the reset marker. `ecs/CLAUDE.md:38`: "reset through
  `request_transform_hierarchy_reset` after a wholesale world replacement"; the pitfall table
  gains a row for the marker citing the new `hierarchy_dirty.rs` test.
  `editor_integration/CLAUDE.md:26`: `project_host.rs` no longer "transform hierarchy".
  `hierarchy_system.rs:47-49, 58, 62`: the engine steps this system from the frame tail for
  every game; a game owns none; the doc example shows the marker, not a hand-stepped system.
  `dirty-flag.md` pitfall 135–139 ("The hierarchy runs only under the editor") becomes "the
  frame tail steps the hierarchy for every game, after the clip machines and before the grid
  backdrop; a wholesale world replacement files `TransformHierarchyReset`, which the next
  update consumes" — the `reset()` bullet at 143–145 points at the marker.
  `training.md:50` ("transform propagation") stays true.

**Target shape — the `System` trait trim (kimi review-3 F2 / gemini F3; Jesse, 2026-10-19:
fold into this batch).** `System` keeps `update(&mut self, world: &mut World, delta_time: f32)`
and nothing else: `initialize`, `start`, `stop`, `shutdown` and `name` go. Verified before the
handoff: the frame tail calls only `update`; `start` and `stop` have no implementor and no
caller; `initialize` and `shutdown` are implemented only by `PhysicsSystem` (both log a line,
`physics_system/update.rs:12-15, 76-79`) and `TransformHierarchySystem` (both log,
`hierarchy_system.rs:155-158, 183-186`); `name()` is implemented by all five engine systems
and read by nothing. The three callers of the hooks go with them: `ProjectHost`'s
`physics.initialize(world)` (`project_host.rs:74`, which only logged — delete the call and
its error branch) and `transform_hierarchy.initialize` (`:141`, gone with the field), and
the two `physics.initialize(&mut world).unwrap()` lines in
`scripting/tests/contracts.rs:290, 383`. `system.rs`'s module doc becomes: the one method
every engine system implements; stepped by its owner — the frame tail for the engine's,
`Game::update` for a game's. The guides: `crates/ecs/CLAUDE.md` and `crates/physics/CLAUDE.md`
lose any line that names a hook (the executor greps `initialize\|shutdown` in both). The
greps for every deleted method go in the report. Any `#[derive(Default)]` or constructor a
system only had for the registry stays — this trim is the trait's surface, not the types.

**Target shape — #153 (event-queue).** `game_trait.rs`'s `on_key_pressed` / `on_key_released`
docs state the contract: the window loop runs a handler between frames, and the next frame's
first act flushes the event bus, so an event emitted here is gone before `update` reads it —
set state in a handler; emit from `update`. A `GameHarness` test in
`transform_hierarchy_tail.rs`'s sibling `key_handler_events.rs`:
`a_world_event_emitted_in_a_key_handler_is_flushed_before_the_next_update_reads_it`
(the harness already mirrors the loop's order on purpose — #153's own note). `event-queue.md`'s
flush pitfall (137–140) gains the handler case. Closes #153.

**Ride-alongs.** DRY-003 (#86): `hierarchy.rs:138-139` and `173-174` each become
`let (sin_r, cos_r) = self.rotation.sin_cos();`. GPP-L1 (#86): `world.entities()` →
`world.entity_ids()` only in passes whose body never mutates the world — the default render
loop in `game_trait.rs`, `tilemap_render.rs:22`, `script_status.rs:24`, and
`project_host.rs:86` if its body only reads; the systems that mutate (`sprite_system.rs:38`,
`clip_state_machine.rs:218`) keep the owned `Vec`, because the iterator borrows the world.
The report lists each site kept and why.

**Gates:** the standard list; `check_wasm.sh` (engine_core); `check_games.sh` (new public
items in `ecs` and `engine_core`); `check_guide_tests.sh` after the table edits.

**Reviewers:** kimi + gemini (the test harness and a public seam change). Jesse's eyes:
`cargo run --example editor_demo --features editor`, Play → move the player → Stop: the
marker child sits above the player in every state, exactly as before.

**Leaves out:** `WorldSnapshot`'s handling of the transient `GlobalTransform2D` (not traced;
the reset marker makes it moot — the first update after Stop rewrites every global);
anything in `renderer`.

Size: about +260 / −110 lines (the trait trim is about forty deleted lines across six files).

## Batch 3 — ecs + editor + engine_core + examples: retire two advertised, unused APIs (#171, #172; GPP-L8) — DONE 2026-10-19 (489a8b5)

Authored by Jesse's Claude Opus session from `review/pattern-audit/handoff-3.md`; reviewed by
kimi (`review-5.md`), gemini (`review-5-gemini.md`) and the planner (`review-5-claude.md`);
adjudicated in `rebuttal-5.md` — kimi 3 findings, 1 accepted (a stale "thirteen" in a doc
comment), 2 rebutted (history; a note for the Conductor filing); gemini 6, 2 accepted
(warning order followed `HashMap` iteration; an unnamed child's warning now names its nearest
named ancestor), 4 rebutted (load→save "data loss" is the ruling and `sound_id` never named a
file; `GlyphInfo` is the planned ride-along; the deletion is decision of record; a code-authored
`spawn_prefab` override is not a saved file); the planner 3, all accepted (the root guide's
"spatial audio" cell, the empty `ComponentCategory::Audio` deleted on Jesse's call, a phrasing).
The executor made two decisions the plan had not: a retired name is stripped only while it is
**unregistered** (a game or Conductor that registers one again owns it), and the strip runs
once over the whole `SceneData` at the top of `instantiate` rather than inside `create_entity`
— so `SceneInstance.prefabs` is stripped (a runtime `spawn_prefab` of such a prefab succeeds)
and the warnings reach `load_warnings` and the status bar. Both ratified. `GlyphInfo` lost
both fields (nothing read them; the compile is the proof). 1106 tests, every gate green.

**Re-verified against the tree 2026-10-19 after batch 2 (1809f87):** `engine_core/src/prelude.rs`
re-exports `HierarchicalStateMachine` at line 77 (not 75) and the audio components at 69;
the platformer attaches the machine at `examples/hello_world/platformer.rs:214`, reads it at
`:377` and prints the group at `:404` (`PlayerState` 47, `PlayerGroup` 56, `player_group` 61);
`state.md`'s "Generic machines don't survive Play/Stop" pitfall starts at 164 and the
`HierarchicalStateMachine` pitfall at 171–176; `event-queue.md`'s audio paragraph is at
131–133 and its `PlaySoundEffect` pitfall at 147–149; the loader's `create_entity` is
`scene_loader.rs:260-290` with the `merge_components` call at 280; `ecs/src/lib.rs` declares
the module at 28 and glob-re-exports it at 53; `set_commands.rs:5`, `component_editors.rs:22`
and `stored_component/mod.rs:9` carry the editor's imports. Every other line below held.


Files, #171: `crates/ecs/src/state_machine.rs` (324 → ~190), `crates/ecs/src/lib.rs:78`,
`crates/ecs/src/prelude.rs:15`, `crates/engine_core/src/prelude.rs:77`,
`crates/editor/src/world_snapshot.rs:11` (doc), `examples/hello_world/platformer.rs`
(211–217, 377, 404), `training.md:54`, `docs/EDITOR_UX_AUDIT.md:91`, the skill's
`references/state.md` (165–176).
Files, #172: `crates/ecs/src/audio_components.rs` (deleted), `crates/ecs/src/lib.rs` (mod +
re-export), `crates/ecs/src/prelude.rs`, `crates/ecs/src/component_registry/mod.rs` (51, 95,
337–340), `crates/ecs/src/component_registry/tests.rs:159-172`,
`crates/engine_core/src/prelude.rs:69`, `crates/editor/src/stored_component/mod.rs:505-506`,
`crates/editor/src/component_editors.rs:345-354`, `crates/editor/src/commands/set_commands.rs:110`,
`crates/editor/src/command_api/write/mod.rs:203`, `crates/editor/src/physical_floors.rs` (61,
150), `crates/editor/src/stored_component/tests.rs` (32–33, 177),
`crates/editor_integration/src/panel_renderer/inspector_tests.rs:23`,
`crates/engine_core/src/scene_serializer/dynamic_and_scripts_tests.rs` (module doc, 8,
67–101), `crates/audio/CLAUDE.md:3-6`, `crates/ecs/CLAUDE.md:26`, `training.md:56, 82`,
`CLAUDE.md` § Core Systems Complete → Audio bullet, the skill's `references/event-queue.md`
(131–133, 147–149).
Files, GPP-L8: `crates/ui/src/font/glyph_cache.rs:35-42`.

**Target shape — #171.** `HierarchicalStateMachine` (state_machine.rs:114-226), its test
(299–323) and the `PlayerGroup`/`player_group` test helpers (240–251) are deleted; the
module doc's first line stops saying "Hierarchical". The three re-exports go.
`platformer.rs` attaches `StateMachine::new(PlayerState::Idle)` (214), reads
`get_mut::<StateMachine<PlayerState>>` (377), and the panel line (404) prints
`player_group(state_machine.current())` — the example's own `player_group` fn (61–66) and
`PlayerGroup` enum (55–59) stay, because the display still shows the group.
`world_snapshot.rs:11` names `StateMachine<S>` as its example of a generic the snapshot
cannot clone. `training.md:54` lists `StateMachine` alone. `EDITOR_UX_AUDIT.md:91` corrects
its stale citation (`editor_demo.rs:194-198` → the platformer file) and notes the drop now
comes with a warning. `state.md` 171–176 becomes: there is no hierarchical machine; derive a
group from the state with a function when you need one (the platformer's `player_group`);
the "Generic machines don't survive Play/Stop" pitfall at 164–169 stays and now carries the
example. `cargo check --workspace --all-targets` is the gate that compiles the example.

**Target shape — #172.** `audio_components.rs` is deleted with its tests; the registry
drops lines 337–340 and its two doc mentions (51, 95) take `GlobalTransform2D` (registered
transient at 351) as the transient example; `component_registry/tests.rs:159-172` asserts
persistence on another persisted type and transience on `GlobalTransform2D`. The editor's
typed overlay loses its two rows (`stored_component/mod.rs:505-506`), `edit_audio_source`
(345–354), the `Set*Command` alias (set_commands.rs:110), the write-verb arm
(command_api/write/mod.rs:203), `clamp_audio_source` and its test (physical_floors.rs:61,
150), and the three test fixtures that used the components; the serializer's dynamic-tier
tests keep `SceneDynSpikeStats` as the game-registered example and lose the audio-specific
cases (the module doc at 1–4 stops mentioning audio). No `.scene.ron` under `examples/` or
`../games/` carries any of the three (verified 2026-10-19), but a playground project in
IndexedDB or an older editor save may — the components were editable in the inspector,
which is #172's premise. So the loader gains a retired-names list, applied to every layer
— a prefab's components, an entity's overrides and its inline components — before
`merge_components` runs in `create_entity` (`scene_loader.rs:260-290`): a `Dynamic` entry
whose type is `AudioSource`, `AudioListener` or `PlaySoundEffect` is dropped with a
`log::warn!` naming the entity and the component, and the scene loads (batch 4's merge then
never sees a retired name); every other unregistered name stays the hard error it is
today, and `ComponentData::Dynamic`'s doc (`scene_data.rs:378-385`) gains the exception. A
test loads a scene string carrying one of each and asserts the entity exists, the
component does not, and the warning was logged (model: whatever the loader's existing
warning tests use).
**Docs:** `audio/CLAUDE.md:3-6` — no ECS components live anywhere; play from game code
through `ctx.audio`; scene-authored audio is Conductor's. `ecs/CLAUDE.md:26` row deleted.
`training.md:56` and `:82` deleted. Root `CLAUDE.md`'s Audio bullet drops "(spatial audio
components exist in ecs but have no runtime system yet)". `event-queue.md` 131–133 and
147–149: audio has no scene-data path and no sound queue; play through `ctx.audio`; the
feature is filed under Conductor (the close-out fills in the number). DRY-004 on #86 loses
its subject (noted at close-out).

**Ride-along GPP-L8 (#88).** `GlyphInfo` (glyph_cache.rs:35-42) drops `character` and
`font_size` if nothing reads them (the executor greps `.character` and `.font_size` on
`GlyphInfo` values); if something does, both stay and the report says what.

**Gates:** the standard list; `cargo check --workspace --all-targets` explicitly (the
example); `check_wasm.sh` (engine_core prelude); `check_games.sh` (public `ecs` items
removed); `check_guide_tests.sh`.

**Reviewers:** kimi + gemini (public seams). No codex: the inspector loses two rows, nothing
is designed; Jesse may add it.

**Leaves out:** any replacement for scene audio (filed); the `ClipStateMachine`, untouched.

Size: about −720 / +50 lines.

## Batch 4 — engine_core + games/breakout: overrides merge field by field (#169; DOC-001)

Files, engine: `crates/engine_core/src/scene_data.rs` (519: `EntityData.overrides` type), a
new `crates/engine_core/src/scene_override.rs` (`OverrideData` and its serde), a new
`crates/engine_core/src/scene_loader_merge.rs` (the merge; `scene_loader.rs` is 401 and
stays under 600 either way), `crates/engine_core/src/scene_loader.rs` (58–79 `spawn_prefab`,
305–331 the merge call sites, 52–53 doc), `crates/engine_core/tests/scene_loader_parse.rs`
(172–201), `crates/engine_core/tests/prefab_spawning.rs` (48, 57–64), `scene_loader.rs:353`
(inline test), `crates/engine_core/tests/fixtures/hello_world_saved.scene.ron` (re-blessed),
`training.md` § Scene Serialization Pattern (the `spawn_prefab` comment), the skill's
`references/prototype.md` (136–139), `crates/engine_core/CLAUDE.md` (scene_loader entry if
it describes layering), plus DOC-001's three lines (`scene_serializer.rs behavior_to_data`,
`contexts.rs` `lines` append-vs-replace, `ui_manager.rs begin_frame` — if that file no
longer exists, the tick says so).
Files, breakout (its own repo, branch `jesse`): `scripts/regenerate_levels.py` (304, 316),
the eight `assets/scenes/level*.scene.ron`, `src/levels_tests.rs` (29–45: `merged_components`
is a test-side replica of the engine's merge over `Vec<ComponentData>`; it would not compile
against `Vec<OverrideData>` and would keep replace semantics if it did — deleted, the test
calls the engine's merge), one harness test if none pins a brick's scale.

**Re-verified against the tree 2026-10-19 after batch 3 (489a8b5):** `scene_data.rs` is 522
lines, `EntityData.overrides` at 128, the newtype variants `Behavior`/`Scripts` at 365/368;
`scene_loader.rs` is 390 lines — `spawn_prefab` 58–71 (its `merge_components` call at 71, doc
at 52–53), `create_entity` 263 with the merge call at 281–283, `merge_components` 294–301, the
inline layering test at 343–362; `scene_loader_parse.rs:172-201` and
`prefab_spawning.rs:48, 57-64, 93-94` hold; `prototype.md`'s pitfall is at 134–139; ron is
0.12.0 in `Cargo.lock`; `crates/ui/src/ui_manager.rs` does not exist, so DOC-001's third line
is ticked as such. Batch 3's retired-names strip runs at the top of `instantiate`, so the merge
never sees a retired name on either path. Breakout: `regenerate_levels.py:279, 284` write the
prefab's scale (they stay), `:304, 316` the override's (they go); **258** override lines carry
`, scale: (0.8, 0.4)` across the eight level files (not 436 — the mechanical check below uses
258); `levels_tests.rs` defines `merged_components` at 29 with callers at 99, 162, 189, 220,
248, 477 and 508. The non-empty override blocks outside breakout are `hello_world.scene.ron`
(127, 136, 174, 189, 205–215, 223, 238, 249, 260, 273, 280–300) and `behavior_demo.scene.ron`
(108–211); the five hello_world coins at 280–300 already restate scale and rotation and must
load unchanged.

**Target shape.**
- `OverrideData { component: ComponentData, fields: Option<BTreeSet<String>> }` in
  `scene_override.rs`. `fields: None` means the whole component (today's semantics);
  `Some(keys)` means only those top-level fields. **Presence comes from the text:** its
  `Deserialize` reads a `Box<ron::value::RawValue>` (ron 0.12.0, the locked version —
  `RawValue::get_ron` and `into_rust` at `src/value/raw.rs:93, 126`), then
  `raw.into_rust::<ComponentData>()` for the typed value, which decides the variant class:
  for a **struct variant** the key set is the string keys of
  `ron::from_str::<ron::Value>(raw.get_ron())` — ron 0.12.0's `deserialize_any` consumes
  the identifier and visits a named struct as a map (`src/de/mod.rs:342-345`,
  `handle_any_struct`'s `StructType::Named` arm), so `Transform2D(position: (1.0, 2.0))`
  yields `Value::Map` with `position` as its one key; for a **newtype variant** (`Behavior`,
  `Scripts` — `scene_data.rs:365, 368`) `fields` is `None` and the override replaces whole,
  as today. No fallback and nothing left to the executor; the first unit test pins the
  `ron::Value` shape. `Serialize` writes the component (the serializer emits
  `overrides: Vec::new()` regardless). `impl From<ComponentData> for
  OverrideData` (whole) and `OverrideData::fields(component, &[&str])` for runtime callers.
  `EntityData.overrides: Vec<OverrideData>`; `PrefabData.components` and
  `EntityData.components` stay `Vec<ComponentData>`.
- `scene_loader_merge.rs`: `merge_override(base: &ComponentData, layer: &OverrideData) ->
  Result<ComponentData, SceneLoadError>`. Both sides go through `serde_json::to_value`
  (externally tagged: `{"Transform2D": {…}}`); same struct variant → copy each key in
  `fields` from the override's inner object onto the base's; `serde_json::from_value` back.
  A key in `fields` that the base object lacks is `SceneLoadError::ComponentError` naming
  the entity, the component and the field (Jesse, 2026-10-19: a typo stops being silent).
  Different variants cannot meet (the layer matches by `component_type_name`), but a typed
  entry and a `Dynamic` entry of the same type name can: that is a load error naming the
  entity and the component. `Dynamic` with the same `component_type` merges `data` objects
  key by key, each override key validated against the registered type's `default_value`
  object (the base `data` may itself omit fields), and a non-object `data` on either side
  is a load error. An override whose component the prefab lacks — or any override on an
  entity with `prefab: None` — is appended whole with serde defaults for what it omits,
  exactly as today and as the `components` layer does; the field-merge contract applies only
  where a prefab component exists. A `fields: None` override — a newtype variant, or a
  runtime caller's whole component — replaces, as today. `merge_components` applies the override layer through
  this and keeps replace semantics for the inline `components` layer, and is exposed as
  `pub fn SceneLoader::effective_components(scene: &SceneData, entity: &EntityData) ->
  Result<Vec<ComponentData>, SceneLoadError>` so `create_entity` and breakout's level test
  share the one merge. `spawn_prefab(…, overrides: &[OverrideData])` uses the same path
  (its only callers are `tests/prefab_spawning.rs:64, 93, 94`); its doc (52–53) says
  "names only the fields it changes".
- **Tests.** `scene_loader_parse.rs:201`'s assertion flips: the prefab's scale survives a
  position-only override (renamed to say so). New: an explicit `scale: (1.0, 1.0)` in an
  override is honoured over a `(0.8, 0.4)` prefab (the refused trap, pinned); a `Dynamic`
  override merges one key and keeps the rest; a `Behavior` override replaces the prefab's
  whole `Behavior`; an override naming a field the component lacks fails to load with the
  entity, component and field in the error; an override for a component the prefab lacks
  appends whole; a typed/`Dynamic` same-name pair fails to load; a `Dynamic` override with
  a misspelled key fails to load;
  `hello_world_prefab_instances_load_at_their_prefabs_scale_and_rotation` runs
  `effective_components` over `hello_world.scene.ron` and pins each named prefab instance's
  scale and rotation — the same numbers the fixture hand-diff checks, so a wrong bless
  cannot pass; `prefab_spawning.rs:62-64` passes
  `OverrideData::fields(Transform2D{…}, &["position"])` and asserts the prefab's scale.
  **The examples change on purpose:** `behavior_demo.scene.ron` and `hello_world.scene.ron`
  carry position-only overrides on prefabs with authored scale and rotation (Coin `0.25`/
  `0.785`, Marker `0.25`, Player `0.5`, Chaser `0.4`, Patroller `0.45`, Companion `0.35`) that
  today load at scale 1, rotation 0 — after this batch they load as authored. The golden
  fixture `tests/fixtures/hello_world_saved.scene.ron` is re-blessed
  (`HELLO_WORLD_GOLDEN_BLESS=1`) and the report hand-diffs it: only the scale/rotation of
  prefab instances whose override omitted them may differ, each named. Jesse runs
  `cargo run --example hello_world` once to see the coins small and tilted.
- **breakout.** `regenerate_levels.py:316` emits `Transform2D(position: (x, y))` and line
  304's `scale` binding goes (the HEADER comment, if it mentions scale, follows); the eight
  level files are regenerated; `assert_same_bricks` holds. `src/levels_tests.rs`'s `merged_components` replica is
  deleted and its callers use `SceneLoader::effective_components`. The planner checks the
  breakout scene diff mechanically: every changed scene line loses exactly
  `, scale: (0.8, 0.4)` and nothing else (258 lines on 2026-10-19). **Staging order:** the executor
  modifies both working trees before any gate runs — `check_games.sh --test` compiles
  breakout's tests against the new engine, so `levels_tests.rs` must already be on the
  engine's merge; the engine commit lands first, breakout's second, and both messages name
  the pairing (the old loader with the regenerated scenes draws every brick at scale 1).
  **The report enumerates every non-empty `overrides:` block** under `../games/*/assets/scenes`,
  `../games/game-template` and `examples/assets/scenes`, each omitted field against its
  prefab's non-default values, as a list — on 2026-10-19 the only such blocks outside
  breakout were `hello_world.scene.ron` and `behavior_demo.scene.ron`. If no breakout harness test pins a brick's loaded `Transform2D.scale` against
  the prefab's `(0.8, 0.4)`, one is added — the collider's half extents and the sprite must
  keep agreeing. The engine commit lands first; breakout's scenes load wrong on the old
  loader.
- **Docs.** `prototype.md` 136–139 becomes "an override names only the fields it changes;
  everything else is the prefab's — including an explicit default; the save path still
  flattens prefabs (filed)". `training.md`'s `spawn_prefab` comment ("overrides replace
  matching component types") says the new rule. `scene_data.rs`'s `EntityData.overrides`
  doc states it. DOC-001's three doc lines land.

**Gates:** the standard list; `check_wasm.sh`; `check_games.sh --test` (every game loads
scenes through this merge); breakout's own `cargo test` and clippy with and without
`--features editor`; `check_guide_tests.sh`.

**Reviewers:** kimi + gemini on the engine diff (a scene-format seam and a blessed fixture);
kimi on the breakout diff from `games/breakout/review/pattern-audit/`.

**Leaves out:** minimal-override saving and a prefab-origin record (filed at close-out);
nested field merges (`position.x`); prefab chaining; any editor change.

Size: about +320 / −40 engine; 436 modified lines in breakout.

## Batch 5 — planner only: close-out

- `.claude/skills/game-programming-patterns/SKILL.md` § Where the engine stands: the
  prototype row (partial → textbook, field-level overrides on load; save flattens), the
  event-queue row (no `PlaySoundEffect`), the dirty-flag row (the tail steps it). § Freshness
  re-pinned to the close-out commit, the "filed the rest as #168 to #172" sentence rewritten
  as history. Each reference file's pitfall was rewritten in its batch; this pass only reads
  them for drift.
- `log_archive.md` § Game Programming Patterns Audit: the October close, dated, with the
  lessons (the editor's double step; presence-from-text; a feature wearing a debt label).
  `coordination/PROGRESS.md`: one entry per batch, in the house shape.
- Issues: #168, #169, #170, #171, #172, #100, #153 closed by their commits (`fixes
  beinsiculous/insiculous_2d#N`); ticks on #84 (BUILD-002, DOC-001, SIZE-002 as already true,
  KISS-002 doc), #86 (DRY-003; GPP-L1 with the kept sites named; DRY-004's subject removed),
  #88 (GPP-L8 or why not); the breakout issue closed by its commit.
- Filed with `/file-issue`: (a) a prefab-origin record and minimal-override saving (the
  #169 remainder, engine_core); (b) scene-authored audio — path-based sound ids, a sound tier
  in `AssetManager`, an asset-browser kind, a public handle lookup, spatial playback — on the
  Conductor (Phase K) milestone beside #140/#141; (c) anything the batch reports left
  undone.
- `reviewer-comparison.md` closed with its totals; `scripts/check-sprint-sync.sh` green;
  `jesse → dev` in `insiculous_2d` and `games/breakout`; Jesse pushes.

## Verification (end to end)

- Every batch: the four cargo gates, `check_guide_tests.sh`, `check_features.sh`, the
  comment-tag grep, `check_wasm.sh`, `check_games.sh` (`--test` on batch 4), files ≤ 600
  lines — run by the planner on the staged tree, never taken from the report.
- Each batch's staged diff: kimi and gemini reviews, the planner's own review, adjudication,
  fixes applied by the planner and enumerated, a pathspec-scoped commit with the review
  asserted.
- The effort's own acceptance, by Jesse: `cargo run --example editor_demo --features editor`
  — the marker child rides above the player through Play/Stop; `cargo run --example
  hello_world` — coins small and tilted as authored; `cargo run -p breakout` from
  `../games/breakout` — every brick the size it was; the inspector's Add Component list
  shows no audio rows.
- `grep -rn "HierarchicalStateMachine\|PlaySoundEffect\|AudioSource\|AudioListener\|SystemRegistry\|add_system\|update_with_schedule" crates src examples ../games docs training.md CLAUDE.md --include=*.rs --include=*.md`
  prints only `log_archive.md`, `coordination/`, this plan, and `docs/EDITOR_UX_AUDIT.md`'s
  history line.
