# Web Playground — the editor in the browser, and games as data

Effort directory once approved: `coordination/web-playground/` (this file becomes its
`plan.md`; the reviewer ledger `reviewer-comparison.md` sits beside it). Review
conversations live in `review/web-playground/`. Delivery runs the handoff loop:
Claude plans and commits, kimi and gemini review every plan version, gemini executes one
batch per handoff, kimi and Claude code-review each batch before it lands.

Plan history: v1 reviewed by kimi (`review-1.md`, 13 findings) and gemini
(`review-1-gemini.md`, 8 findings) on 2026-09-04, adjudicated in `rebuttal-1.md` — 18
accepted, 3 rebutted (winit's canvas-scoped key listener; request ids over a FIFO; the
IndexedDB deferral, which Jesse reversed into "IndexedDB now"). v2 reviewed by kimi
(`review-2.md`, 9) and gemini (`review-2-gemini.md`, 10), adjudicated in `rebuttal-2.md` —
all accepted (one in part); the persistence design in batch 3 was rewritten (CAS puts, no
debounce, base-joined project roots, atomic `replace_project`, memory fallback, bundle
version + reset) and the Rhai command buffer became a shared handle. Both reviewers cleared
batches 0–2; Jesse ruled batch 1 may run in parallel with round 3. v3 reviewed by kimi
(`review-3.md`, 8) and gemini (`review-3-gemini.md`, 9), adjudicated in `rebuttal-3.md` — 15
accepted, 2 rebutted (contact points pong never used; `zip` 4.6.1 verified by `cargo info`):
per-path put chain, CAS chained inside the IndexedDB callback, write epoch, slug rule, orphan
sweep, prefs saved on Play/Stop and hidden, pinned copy target, content hash + Reset for any
stored bundled slug, mixed INT/FLOAT in Rhai, blank-line refusal, runaway quarantine, Ctrl+S.
This is v4.

## Context

The **Web Playground** sprint (milestone on `insiculous_2d`, board Sprint field across
repos) is next: insiculous_web#4 (the WebGPU gate must await `requestDevice`), #48
(editor-on-wasm), #49 (project export/import + a `game-template` repo). Its stated gates
are met (#6 KvStore = `save_store`, #7 wasm CI guard, the H9 ports). Editor Sprint 6's
five issues shipped Sep 1 2026 (PROGRESS.md carries all five entries) but were never
closed; that is a close-out, not a sprint, and batch 0 does it.

Jesse widened the scope on Sep 4 2026 with four rulings:

1. **What the browser editor loads:** the data-driven sample project first (the boot
   proof), then every one of the six games as its own editor-feature bundle.
2. **Scripting Stages 2 and 3 are in** (audit §6.5): game logic visible in the hierarchy
   and scripts that execute. Stage 1 (`Scripts` as inert data, #44) shipped Aug 28.
3. **Game logic becomes data, not compiled Rust.** The goal is a complete game built in
   the editor and carried by export/import, with no per-game wasm build. Runtime:
   **Rhai** scripts loaded from the project (`source_path` ending in `.rhai`) plus
   engine-registered Rust `ScriptBehavior`s under one registry. Script runner is
   **game-run** (the host calls `ctx.scripts.update(...)` like `BehaviorRunner` today;
   scripts keep physics access; shipped games change nothing) with an editor-side
   lie-detector. Descriptors are **manual** `ScriptDescriptor` consts, no derive macro.
4. **Pong's gameplay loop is the proof**: paddles, AI, serve, scoring, win, HUD — as a
   project of scene + `.rhai` scripts, loadable in the browser editor and exportable.
   Pong's menus, power-ups, chaos modes and achievements stay in Rust and are filed as
   follow-ups with the other five games.

What the exploration established (Sep 4 2026, tree at `0f052b9`):

- **Both editor crates already compile clippy-clean for wasm32.** `scripts/check_wasm.sh`
  runs `--workspace`, and `crates/editor` + `crates/editor_integration` are members. No
  `rfd`, no process spawn, no native-only dependency. The blockers are runtime silent
  failures: `std::fs::read_dir` in `crates/editor/src/asset_browser.rs:86` (0 assets),
  `std::fs` in `crates/editor/src/editor_preferences.rs:69,79` (prefs never persist),
  `parent.exists()` + `create_dir_all` in
  `crates/editor_integration/src/editor_game/scene_io.rs:135-136` (save dies before the
  writer), `std::fs::write` in `crates/engine_core/src/scene_serializer.rs:86`, and
  `std::fs::read_dir` in `crates/editor_integration/src/constants.rs:41`. Scene READS
  already go through `common::vfs` (`scene_loader.rs:88`); there is no `vfs::write`.
- `src/bin/editor.rs` is the only genuinely native file (`std::thread::spawn` at `:85`
  panics on wasm; `env::args`, stdin, `process::exit`) and the gate never compiles it
  (`required-features = ["editor"]`, not default). Its `EditorApp` is the data-only host
  the web needs, and it runs physics + hierarchy only — **no `BehaviorRunner`** during
  Play, so a data-driven scene's behaviors do nothing in the standalone editor today.
- The command API dispatch is transport-agnostic (`answer_api_lines`, `api.rs:43-94`);
  only the stdin producer is native. Stage D was written as "WebSocket"; a
  wasm-bindgen function bridge is the smaller shape and this plan adopts it.
- The site embeds a game by dynamic `import()` of the glue with a fixed `#game-canvas`
  placeholder (`GameEmbed.astro:33-45`), one embed per page. Deploy gates walk every
  route in `dist/`: `postbuild-check.mjs` (25 MiB per-file cap, one `<h1>`, no duplicate
  ids, curly apostrophes), axe over every page, sideways-scroll screenshots incl. 125%
  text. `build_wasm.sh` is a game bundler with a four-place version contract.
- The games are 100% Rust rules. Pong registers no components, ships no scene, and puts
  every rule in `impl Game for PongGame` (`gameplay/` ≈ 630 lines). `Behavior` is frozen
  at 8 variants by ruling; new logic goes through the script seam.
- Nothing in the tree interprets code at runtime. `rhai` 1.26.0 (MIT/Apache, pure Rust,
  `default-features = false, features = ["std", "f32_float"]`, wasm32 supported) is the
  new dependency; `zip` 4.6.1 (verified `cargo info zip@4` on 2026-09-04: `version:
  4.6.1`, `rust-version: 1.82.0`, feature `deflate-flate2 = [_deflate-any, dep:flate2]`)
  with `default-features = false, features = ["deflate-flate2"]` reuses the
  `flate2`/`miniz_oxide` already in `Cargo.lock`. Those
  two, plus the wasm-target-only bridge family the games already carry (`wasm-bindgen`
  pinned `=0.2.126`, `wasm-bindgen-futures`, `web-sys`, `js-sys`) in `crates/playground`,
  are the only new direct dependencies in this plan. IndexedDB is reached through
  `web-sys`'s `Idb*` features with a hand-written request-to-future adapter, not a crate.
- Rhai script functions are pure: a `fn` body cannot read variables of the calling
  `Scope` (gemini, review 1). Parameters therefore travel as an explicit argument. And
  `Engine::call_fn` passes arguments BY VALUE (gemini, review 2): a plain struct handed in
  as `out` is a clone the caller never sees again, so the command buffer must be a shared
  handle (`Rc<RefCell<…>>`) for its mutations to survive the call.
- Cargo honours `[profile.*]` only in the workspace root manifest (kimi, review 2); the
  root `Cargo.toml` has none today, so the playground's profile lives there, not in the
  member crate.
- The VFS's canonical key is the base-joined absolute string (`vfs.rs:8-13`, pinned by
  its test): on the web a project root must be `{ASSET_BASE}/projects/<slug>`, never a
  relative path, or a save lands on a key the loader never reads (kimi/gemini, review 2).
- An IndexedDB transaction becomes inactive once control returns to the event loop with
  no request pending; a Rust `await` between a read and its dependent write throws
  `TransactionInactiveError` on some engines (gemini, review 3). Dependent requests are
  issued synchronously inside the previous request's `onsuccess`.
- Rhai never coerces `INT` to `FLOAT`: `dt * 450` is a "function not found" at runtime
  (gemini, review 3). Rhai's `call_fn` args are by value (review 2). Both shape batch 7.
- `Game::on_exit` is called only by the native app handler (`app_handler.rs:103`); the
  wasm frame loop never calls it (kimi, review 3), so anything saved only in `on_exit`
  never saves in a browser.
- `answer_api_lines` skips blank lines without a response (`api.rs:54-56`), so a paired
  FIFO console must refuse them at dispatch (gemini, review 3).
- winit 0.30.13's web backend registers its `keydown` listener on the canvas element
  (`platform_impl/web/web_sys/canvas.rs:301`), so page-side text inputs never feed the
  editor's shortcuts.

## Decisions of record (taken with Jesse, Sep 4 2026)

- **Branches.** `insiculous_2d`: batch 0 fast-forwards local `jesse` to `dev`, every
  batch commits on `jesse`, one merge into `dev` at the end. `insiculous_web`: same
  shape on its `jesse` branch (Iroh is Jesse's machine; `m` is Danny's), merged into
  `dev`; `main` only receives merges. `games/pong`: its machine branch if it carries one,
  otherwise its default. Deploys are Jesse's push.
- **One bundle, many projects.** The playground is ONE wasm bundle (a data-only host
  wrapped in the editor) that loads any *project* (scenes + `.rhai` scripts + assets).
  Projects are data: bundled with the build, saved to browser storage, exported and
  imported as zip. The six Rust games additionally ship as per-game editor bundles
  (batch 9) because their rules are compiled in; that batch is independent and
  droppable.
- **Scripts: view in, commands out, two hooks.** A script never holds a `&mut World`.
  Each frame the runner builds a `ScriptView` (the owning entity's transform and
  velocity, every Named entity's transform and velocity, per-player input axes and
  just-activated actions, this frame's collision events by name, the blackboard, the
  frame counter, `dt`) and hands it to the script together with its `params`; the script
  returns through a `ScriptCommands` buffer that the runner applies afterwards — exactly
  the `BehaviorCommands` pattern in `behavior_runner/mod.rs:39-54,211-280`. **Every
  command names its target**: `self` or an entity `Name` (set position, kinematic target,
  velocity, reset body, sprite color, sprite visibility, UiLabel text, despawn) — a goal
  sensor resets the ball, not itself; blackboard writes are global. A missing or
  ambiguous target is a per-Play deduplicated error, never a panic. Scripts expose two
  optional hooks: `early_update` runs BEFORE the physics step (input, kinematic movement)
  and `update` runs AFTER the step and the collision drain (reactions, scoring) — pong's
  own order, and the only way a kinematic paddle's collider and sprite agree in the same
  frame. The view is built ONCE per frame and shared by every instance. Rust scripts
  receive `&mut ScriptCommands`; Rhai scripts receive a `ScriptCommandsHandle`
  (`Rc<RefCell<Vec<ScriptCommand>>>`) because Rhai passes by value. Safe, headless
  testable, deterministic, and the same rules for Rust and Rhai.
- **Persistence on the web is IndexedDB, now** (Jesse, reversing the v1 localStorage
  overlay: future-proof upfront rather than migrate later). A `ProjectStore` holds
  per-file records `{ project, path, bytes, revision, bundle_version }` plus each
  project's manifest, in one database `beinsiculous.playground`, object stores `files`
  and `projects`. Rules, each of which closed a review-2 finding:
  - **No debounce, one chain per path.** Saves are user-initiated (Ctrl+S, textarea
    Save, import), so every `vfs::write` on the web inserts into `MemFs` synchronously and
    starts its store put immediately — unless a put for that path is in flight, in which
    case the new put waits for it and takes its resolved revision as base (a tab must not
    race itself). `visibilitychange`→hidden and `pagehide` re-issue only puts that are
    NOT in flight; `beforeunload` warns while anything is pending.
  - **Every put is a compare-and-swap** inside ONE `readwrite` transaction, with the
    dependent `put` issued synchronously inside the read's `onsuccess` (an `await` in
    between would find the transaction inactive): read the stored revision, refuse unless
    it equals the tab's base revision, write base + 1. An absent record accepts only
    base 0. A refusal is `StaleRevision` → status message "saved from another tab or an
    earlier save in this one — reload to see it", the path stays pending. Never
    get-then-put across two requests.
  - **A write epoch guards replaces.** Import, reset and project switch bump the epoch
    first; a `vfs::write` observed under an older epoch is refused with a status message
    ("project is being replaced — save again after the reload") instead of persisting.
  - **Import and reset are atomic**: `replace_project(slug, files, manifest)` is one
    transaction across both object stores; the manifest is the commit marker, boot
    ignores a slug that has files but no manifest, and boot sweeps such orphans.
  - **Slugs are validated** on import: `^[a-z0-9_-]{1,32}$`, or the zip is refused
    naming the slug. Shadowing a bundled slug stays allowed; Reset is the recovery.
  - **A store that fails to open** (private browsing, sandboxed frame) falls back to an
    in-memory store with the persistence banner set; the playground stays usable and
    exportable. A failed put makes the editor visibly dirty with a persistent banner
    ("not saved to this browser — export your project"), never a passing status line.
  - **Stored work wins over bundled content**, but the page offers "Reset to bundled"
    (`playground_reset_project`: drop the slug's stored files, reload) for ANY bundled
    slug that has stored files — a stored manifest for a bundled slug always means
    user-modified or imported. The build writes a content hash per bundled project into
    `projects.json`; a stored manifest records the hash and bundle version it was saved
    under, and when either differs from the booting bundle the page says "the bundled
    project changed since you saved" beside the Reset control. A version bump is a
    consistency check; the hash is the freshness check.
  - **Editor preferences save on both targets**: on every Play/Stop transition, on
    `visibilitychange`→hidden, and before `open_project`/reset — never only in `on_exit`,
    which the wasm loop never calls.
  - **Boot**: preload the bundle, open the store (or fall back), then await the chosen
    project's stored files onto `MemFs`, overwriting bundled ones. Navigation that
    reloads (`open_project`, import, reset) returns a `Promise` the page awaits AFTER
    every put has committed.
  - Natively the same trait is a directory store, so every store contract is tested by
    `cargo test`. `save_store` (localStorage) keeps the editor prefs only.
- **Every project lives under its own root**, and on the web that root is the
  base-joined absolute path `{ASSET_BASE}/projects/<slug>` — the VFS's canonical key
  space; the bundled `examples` included. Store records hold project-RELATIVE paths; the
  boundary joins and strips the root. "Replace this project" removes exactly one prefix.
- **Stage D transport = wasm-bindgen exports**, not a WebSocket: `playground_dispatch`,
  `playground_poll_responses`, `playground_is_dirty`, `playground_write_file`,
  `playground_read_file`, `playground_list_files`, `playground_export_zip`,
  `playground_import_zip`, `playground_list_projects`, `playground_open_project`,
  `playground_reset_project`, `playground_script_errors`. Responses drain once per frame
  in request order from one FIFO channel; the pending queue is capped at 1024 lines and a
  full queue OR a whitespace-only line REFUSES the dispatch (`playground_dispatch` returns
  `false`, shown inline — `answer_api_lines` emits nothing for a blank line) so request
  and response counts never diverge; no request ids (rebuttal 1, gemini F8).
  `docs/EDITOR_COMMAND_API.md` § Stages is corrected.
- **Script source editing on the web** is a page-side `<textarea>` (native HTML: free
  accessibility, undo, IME) that writes through `playground_write_file`; the runner
  recompiles a changed source on the next Play. No multi-line widget is added to the
  wgpu UI.
- **The template repo** `beinsiculous/game-template` is created only with Jesse's
  explicit go-ahead at batch 10 (`gh repo create` is outward-facing).
- **No new `Behavior` variants; one built-in Rust script** (`engine::rotate`) proves
  the registry path. Reimplementing the eight behaviors as built-in scripts is filed,
  not done.

## Ground rules for every batch

- Gates: `cargo test --workspace` (0 failed, 0 ignored), `cargo clippy --workspace
  --all-targets` (0 warnings), every touched file ≤ 600 lines, no new `#[allow]`, no
  `unwrap()` outside tests, no new dependency beyond `rhai` (batch 7), `zip` (batch 5)
  and the wasm-target-only bridge family (`wasm-bindgen`, `wasm-bindgen-futures`,
  `web-sys`, `js-sys`) in `crates/playground` (batch 3). `/finish-task` is the checklist.
- Comment-tag gate on every batch:
  `grep -riEn "kimi|issue #[0-9]+|GPP-[0-9]+|audit §|\(#[0-9]+\)|#[0-9]{1,4}\b|Sprint [0-9]" crates src examples --include=*.rs`
  prints nothing (a hex literal or string match is inspected by hand).
- Wasm gate `scripts/check_wasm.sh` whenever the staged diff touches `crates/common`,
  `crates/engine_core`, `crates/renderer`, `crates/audio`, `crates/input`,
  `crates/editor`, `crates/editor_integration` or `crates/playground` — i.e. every
  engine batch in this plan. The playground crate joins the workspace in batch 3 and is
  covered from then on.
- Games gate `scripts/check_games.sh` whenever a public item of `engine_core`, `ecs`,
  `physics`, `input`, `common` or `renderer` changes; `--test` when behaviour they
  exercise changes. Verify `../games` resolves to the working set first.
- Site gate for every `insiculous_web` batch: `npm run verify` (validate, data tests,
  `astro check`, build + postbuild, axe, screenshots at four widths). New routes are
  gated automatically because the checkers walk `dist/`.
- Bundle gate whenever a wasm bundle is rebuilt: `scripts/build_wasm.sh` must not warn
  past 20 MiB (Cloudflare's hard cap is 25 MiB per file, enforced by `postbuild-check`).
- Review: the commit hook denies unreviewed commits over 100 changed lines. Every code
  batch goes `git diff --cached > review/web-playground/draft-<batch>.diff`, kimi review
  (detached when the diff is large), Claude's own review, adjudication with Jesse,
  `rebuttal-N.md`, fixes applied by the planner, then `ADV_REVIEWED=1 git commit -F
  <message-file> --pathspec-from-file=<scope-file>`. Never skip trailers.
- Every commit is pathspec-scoped. The plan's "done" marks are their own commits
  (`-- coordination/web-playground/plan.md`).
- Before each handoff the planner re-verifies the batch section against the tree and
  commits corrections into the section. One batch out at a time; the planner neither
  edits nor runs cargo while a batch is out.
- **Browser checks are Jesse's.** Agents cannot see a headed WebGPU browser. Each web
  batch lists the exact check ("open `/playground/`, move the player, press Play, save,
  reload, the move persisted") and the batch is not marked done until Jesse reports it.
- Docs match reality at every commit: a guide describing a thing a batch changed is a
  defect in that batch.
- Anything deferred is filed with `/file-issue` before the effort reports done.

## Batch 0 — planner only: branches, close-outs, the small site fix

No executor. Under the review threshold per commit.

1. `insiculous_2d`: `git switch jesse && git merge --ff-only dev` (fall back to merging
   `dev` into `jesse` if it is not a fast-forward). `insiculous_web`: `git fetch` then
   the same on its `jesse`. Confirm `hostname` is Iroh.
2. Create `coordination/web-playground/{plan.md,reviewer-comparison.md}` (this plan;
   an empty ledger with the cleanup's column set) and `review/web-playground/`.
3. Close Editor Sprint 6's five issues (#46, #51, #54, #55, #66) with a comment naming
   the PROGRESS.md entry and commit that shipped each (`git log -S` finds them).
4. **insiculous_web#4**: in `src/components/GameEmbed.astro:69-103`, after
   `requestAdapter()` resolves, `await adapter.requestDevice()` inside the same
   try/catch; a rejection shows the dedicated unsupported-browser message instead of
   the generic "Failed to start". Mirror the same check in the local test page
   `build_wasm.sh` writes (`:132-136`, which today only tests `!navigator.gpu`). Gate:
   `npm run verify`. Commit on `insiculous_web` `jesse` with `fixes
   beinsiculous/insiculous_web#4`.
5. Run the three-model plan review: copy this plan to `review/web-playground/plan.md`,
   `scripts/request-review.sh plan review/web-playground/plan.md --reviewer=kimi` and
   `--reviewer=gemini --out=review/web-playground/review-1-gemini.md` on the same
   snapshot, adjudicate with Jesse, `rebuttal-1.md`, revise until settled. Record every
   decision in this file. Each round keeps the reviewed text as `plan-vN.md` beside the
   reviews. Round 1 done 2026-09-04 (see Plan history); round 2 reviews this v2.
6. Once settled: commit `coordination/web-playground/{plan.md,reviewer-comparison.md}`
   on `jesse` with `ADV_REVIEWED=1` (the plan review is the review), then execute step 4.

## Batch 1 — the write and list seams (engine + editor) — DONE 2026-09-04 (2cdbcc1)

Authored by gemini from `review/web-playground/handoff-1.md`; reviewed by kimi (`review-4.md`,
3 findings: 1 accepted, 2 policy rebuts — never-follow-symlinks is the round-1 ruling, and
`save_store`'s symlink replacement is documented) and Claude (`review-4-claude.md`, 4
accepted); adjudicated in `rebuttal-4.md`. Landed as specified, plus: `MAX_LIST_DEPTH` (cfg
native), empty-path refusal and file-at-prefix removal in `vfs`, `load_preferences_from(slot)`
with warnings (the batch-3 `prefs_slot` seam one batch early), and the absent-or-corrupt
prefs test. Every gate green after the fixes (`gates-1-fixed.log`).

Files: `crates/common/src/vfs.rs` (195 lines), `crates/engine_core/src/scene_serializer.rs`
(`:86`), `crates/editor/src/editor_preferences.rs` (`:66-81`),
`crates/editor_integration/src/editor_game/{mod.rs (563 — near the ceiling), scene_io.rs}`,
`crates/editor_integration/src/constants.rs` (`:40-53`), `crates/editor/src/asset_browser.rs`
(`:80-127`), `crates/editor_integration/src/panel_renderer/asset_browser.rs` (`:104-118`).

Target shapes:

- `common::vfs::write(path: &Path, bytes: &[u8]) -> io::Result<()>` and
  `write_string`: native `std::fs::create_dir_all(parent)` + `std::fs::write`; wasm
  `MemFs::insert`. `common::vfs::list_files(dir: &Path) -> io::Result<Vec<PathBuf>>`:
  recursive, every extension, sorted, depth-capped at 6 natively, walking with
  `symlink_metadata` and NEVER following symlinks (the doc comment says so; a test with
  a symlink loop pins it — the asset browser's guard moves here); wasm prefix scan over
  `MemFs`. `common::vfs::remove_prefix(dir: &Path) -> io::Result<()>` on EVERY target
  (native `remove_dir_all` of that directory; wasm `MemFs::remove_prefix`) — the import
  path's "replace this project" primitive, testable natively. `MemFs` gains documented
  overwrite semantics on `insert`, `list_files(prefix)` and `remove_prefix(prefix)`.
  `MemFs` tests pin: write-then-read round trip, recursive listing order, prefix removal
  leaving siblings.
- `scene_serializer::save_scene_to_file` writes through `vfs::write_string`. Its test
  `save_scene_to_file_writes_a_parseable_file_and_reports_an_unwritable_path`
  (`scene_serializer/tests.rs:177-195`) today asserts that a MISSING PARENT fails; with
  parent creation that case succeeds, so the "unwritable" half must use a path whose
  parent is a regular file (creation genuinely fails) — the contract "an unwritable path
  is reported, not swallowed" is kept, the fixture changes.
- `scene_io.rs:133-138`: delete the `parent.exists()` / `create_dir_all` block —
  `vfs::write` owns parent creation. Every other line of the save choke point stays.
- `EditorPreferences::{load, save}` lose their IO: they become `from_json(&str) ->
  Result<Self, String>` and `to_json(&self) -> Result<String, String>` in the editor
  crate; the load at `editor_integration/src/editor_game/mod.rs:244` and the save in
  `on_exit` (grep `EDITOR_PREFS_PATH` in that file for both sites) do the IO through
  `engine_core::save_store::{read, write}` with slot `Path::new(EDITOR_PREFS_PATH)`.
  Natively the file is byte-identical to today (same JSON, same relative path); on the
  web the slot is a localStorage key the playground entry supplies (batch 3: `EditorRunOptions.
  prefs_slot: Option<PathBuf>`, defaulting to `EDITOR_PREFS_PATH`). If `mod.rs` crosses
  600 lines (it is at 563), split the preferences plumbing into
  `editor_game/preferences.rs`.
- `find_first_scene` (`constants.rs:40-53`) uses `vfs::list_dir_files(dir, "ron")`.
- `asset_browser::scan_assets(base)` walks `vfs::list_files(base)` instead of
  `read_dir`; its `MAX_SCAN_DEPTH` constant and the explicit stack go (the VFS caps
  depth). Test names describe behaviour: nested images listed with `/`-joined
  relative paths; `.txt` still ignored.
- Docs: `crates/common/CLAUDE.md`, `crates/editor/CLAUDE.md`,
  `crates/editor_integration/CLAUDE.md` pitfall tables gain the "save reaches `vfs`"
  rows; `training.md` § Asset Manager notes `vfs::write`.

Gates: standard + wasm + games (`common` public surface grew). Leaves out: any web
entry, any persistence store (batch 3).

## Batch 2 — the data-only host moves into `editor_integration`

Files: `src/bin/editor.rs` (thin afterwards), new
`crates/editor_integration/src/project_host.rs`, `crates/editor_integration/src/lib.rs`
(re-exports), `crates/editor_integration/src/editor_game/mod.rs` (`EditorRunOptions`).

Target shapes:

- `pub struct ProjectHost { project_path: PathBuf, physics: Option<PhysicsSystem>,
  behaviors: BehaviorRunner, transform_hierarchy: TransformHierarchySystem }` with
  `ProjectHost::new(project_path)`, implementing `Game` exactly as `EditorApp` does
  today (`editor.rs:20-77`) PLUS `BehaviorRunner::update` each Playing frame, before
  `physics.update`, with `set_named_entities` rebuilt from the world's `Name`
  components at the first Playing frame (the same lazy point that builds physics).
  `on_play_stopped` drops physics and clears the runner's named map. Batch 7 adds the
  script runner call here — this batch leaves a doc line naming that seam, not a stub.
- `src/bin/editor.rs` keeps only argument parsing, the stdin reader thread and the two
  `run_*` calls; the `EditorApp` struct is deleted. Behaviour identical.
- A headless test in `project_host.rs`: a world with one `Behavior::Patrol` entity
  advances position over simulated Playing frames (proves the standalone editor's Play
  now runs behaviors — it did not before). Test names per `training.md` § Writing Tests.
- Docs: `crates/editor_integration/CLAUDE.md` file map; `README.md` standalone-editor
  paragraph; `docs/EDITOR_COMMAND_API.md` Stage C paragraph names `ProjectHost`.

Gates: standard + wasm. Leaves out: scripting, web entry.

## Batch 3 — the `playground` crate: web entry, IndexedDB project store, bridge

Files: new `crates/playground/` (`Cargo.toml`, `src/lib.rs`, `src/web_entry.rs`,
`src/bridge.rs`, `src/store.rs`, `src/store/{directory.rs, memory.rs, indexed_db.rs,
idb_request.rs}`, `src/projects.rs`, `src/persist.rs`), root `Cargo.toml` (workspace
member AND `[profile.wasm-release]` — cargo ignores profiles in member manifests),
`crates/engine_core/src/web/mod.rs` (one addition), `crates/common/src/vfs.rs` (a
write-observer hook), `crates/editor_integration/src/editor_game/{api.rs, mod.rs}`
(response channel, dirty query, prefs slot). The sample project is `examples/` (its two
scenes; `behavior_demo.scene.ron` is fully data-driven and opens first by sorted order);
the build script (batch 4) copies it to `projects/examples/`.

**First deliverable, before anything else in this batch: the size probe.** Add
`[profile.wasm-release]` (pong's four lines) to the ROOT `Cargo.toml`, then build a bare
`crates/playground` cdylib (entry that only calls `run_game_with_editor_opts` on
`ProjectHost`) with `cargo build -p playground --lib --target wasm32-unknown-unknown
--profile wasm-release` + `wasm-bindgen`; the `.wasm` size is recorded in this section.
Past 15 MiB, the planner decides font (`crates/editor/src/fonts.rs` embeds ~1.8 MB of
DejaVu) and later rhai trimming before batch 4 commits to a page. (Reference: pong ships
at 2.5 MiB.)

Target shapes:

- `crates/playground/Cargo.toml`: `[lib] crate-type = ["cdylib", "rlib"]`, deps
  `editor_integration`, `engine_core`, `common`, `serde`, `serde_json`, `log`; wasm
  target deps `wasm-bindgen = "=0.2.126"` (the pin `build_wasm.sh` asserts),
  `wasm-bindgen-futures`, `js-sys`, `web-sys` with the `Idb*` features the store needs.
  No `[profile.*]` here (root only). No native `main.rs` (the standalone `editor` bin is
  the native face).
- `store.rs`: `pub trait ProjectStore { fn load_project(&self, slug) -> Fut<Result<Vec<
  StoredFile>, StoreError>>; fn put(&self, file: StoredFile, base_revision: u64) ->
  Fut<Result<u64 /* new revision */, StoreError>>; fn replace_project(&self, slug, files:
  Vec<StoredFile>, manifest: ProjectManifest) -> Fut<Result<(), StoreError>>; fn
  remove_project(&self, slug) -> Fut<Result<(), StoreError>>; fn manifests(&self) ->
  Fut<Vec<ProjectManifest>>; }` where `Fut<T> = Pin<Box<dyn Future<Output = T>>>` (no
  `async_trait` crate), `StoredFile { project: String, path: String /* project-relative */,
  bytes: Vec<u8>, revision: u64, bundle_version: String }`, `StoreError::{Unavailable,
  StaleRevision { stored: u64, base: u64 }, Backend(String)}`. **`put` is a
  compare-and-swap**: inside ONE `readwrite` transaction it reads the stored revision,
  refuses with `StaleRevision` unless it equals `base_revision`, else writes
  `base_revision + 1` and returns it. **`replace_project` is one transaction** across both
  object stores (`files` cleared for the slug, new files written, manifest written last as
  the commit marker); `remove_project` likewise; `sweep_orphans()` removes `files` whose
  slug has no manifest (boot calls it). `StoredFile` and `ProjectManifest` both carry
  `bundle_version` and `content_hash`. `store/directory.rs` (native; the test double for
  every contract — a lock file per project makes the CAS honest), `store/memory.rs` (the
  fallback when IndexedDB will not open; all targets), `store/indexed_db.rs` (database
  `beinsiculous.playground` v1, object stores `files` keyed `[project, path]` and `projects`
  keyed `slug`), `store/idb_transaction.rs` (the adapter — it wraps a whole TRANSACTION's
  `complete`/`abort`/`error` events into a `Future`, one `Closure` trio,
  `Rc<RefCell<Option<Result>>>`, waker; every dependent request — the CAS's `put` after
  its `get`, each file of a `replace_project` — is issued synchronously inside the
  previous request's `onsuccess` callback, never after an `await`, because the
  transaction is inactive by then; the only place web-sys IDB verbosity lives).
- `persist.rs`: the write path. `common::vfs` gains `set_write_observer(fn(&Path))`
  (wasm-only; called after every `MemFs::insert` from `vfs::write`). **No debounce, one
  chain per path**: the observer checks the write epoch (a write under an older epoch is
  refused with "project is being replaced — save again after the reload"), strips the
  project root, marks the relative path pending, and either `spawn_local`s
  `store.put(file, base_revision)` immediately or, if a put for that path is in flight,
  queues the bytes behind it; when the in-flight put resolves `Ok(new)`, the queued put
  starts with `base_revision = new`. `base_revision` is the revision the tab last loaded
  or wrote for that path (0 for a new file; an absent record accepts only 0). Outcomes:
  `Ok(new)` records `new` as the base and clears pending; `StaleRevision` → status
  message "saved from another tab or an earlier save in this one — reload to see it",
  path stays pending; `Backend`/`Unavailable` → the persistent banner + the editor's dirty
  indicator via `EditorRunOptions.on_persist_failed: Option<Box<dyn FnMut(&str)>>`.
  `pagehide` and `visibilitychange`→hidden re-issue pending puts that are not in flight
  (best effort); `beforeunload` returns a warning while anything is pending. Import,
  reset and project switch bump the epoch, then clear the pending set. Preferences save
  through `save_store` on Play/Stop, on hidden, and before switch/reset
  (`EditorGame::save_preferences` becomes callable from those edges; `on_exit` keeps its
  call). Tests (native, directory store): put/load round trip; a put with a stale base is
  refused and the stored bytes are untouched; two writers racing from the same base —
  exactly one wins; two puts of one path from one writer — the second chains and lands
  with revision base + 2, no `StaleRevision`; a write under an old epoch is refused;
  `replace_project` leaves the other project intact; a failed `replace_project` leaves the
  previous project intact (manifest is the marker); `sweep_orphans` removes manifest-less
  files and nothing else; manifests merge bundled + stored; the memory store passes the
  same suite.
- `projects.rs`: `ProjectManifest { slug, title, bundle_version, content_hash }`; the
  project ROOT is computed, never stored: `{ASSET_BASE}/projects/<slug>` on the web (the
  base-joined canonical VFS key), `<dir>/projects/<slug>` natively; the project's asset
  base is `{root}/assets` on both. `list_projects()` = bundled `{ASSET_BASE}/projects.json`
  merged with `store.manifests()` (stored wins on a slug clash, so an imported project can
  shadow a bundled one); each entry reports `is_bundled`, `has_stored_files` and whether
  the stored `content_hash`/`bundle_version` differ from the bundled one, which is what
  the page's Reset control reads. `validate_slug` enforces `^[a-z0-9_-]{1,32}$`.
- `web_entry.rs`: `ASSET_BASE = "/playground/v1/assets"` and `BUNDLE_VERSION = "v1"`
  (the version contract, now five places — documented in the crate header),
  `init_web_logging`, `preload_assets(ASSET_BASE)`, open the store (fall back to
  `memory.rs` + banner on failure), `sweep_orphans()`, `list_projects()`, pick the project
  (query string `?project=<slug>`, default the first), await `load_project` and insert
  every stored file at `{root}/{relative path}` onto `MemFs` (stored files overwrite
  bundled ones; a slug with files but no manifest is ignored as an unfinished import),
  then `run_game_with_editor_opts(ProjectHost::new(root), GameConfig::new("Insiculous
  Playground").with_size(1280, 800).with_asset_base_path("{root}/assets"), opts)`. The
  editor's 1024×720 minimum (`constants.rs:22-26`) is respected by the page canvas size.
  Editor prefs slot: `EditorRunOptions.prefs_slot = "beinsiculous.playground.editor_prefs"`
  (localStorage via `save_store`). A `MemFs` test in `common` pins the whole key story:
  insert a bundled file at the key the build script's copy produces
  (`{ASSET_BASE}/projects/examples/assets/scenes/behavior_demo.scene.ron`), `vfs::write`
  an edit at the same key, read it back through a `Path::new("{root}/assets").join(
  "scenes/behavior_demo.scene.ron")` lookup, and confirm a RELATIVE key never resolves.
- `bridge.rs` (wasm-only `#[wasm_bindgen]` exports; each is a thin call into
  target-agnostic functions unit-tested natively): `playground_dispatch(line) -> bool`
  pushes onto the API channel (`EditorRunOptions.api_rx`'s sender in a `thread_local`,
  capped at 1024 pending — a full queue OR a whitespace-only line REFUSES and returns
  `false`, never drops, never enqueues a line the API answers with nothing);
  `playground_poll_responses() -> Vec<JsValue>` drains a response channel —
  `ApiSession` gains `responses: Option<mpsc::Sender<String>>` and `drain_api_requests`
  (`api.rs:212-234`) writes there when set, stdout otherwise; `playground_is_dirty() ->
  bool` = the `CommandHistory` watermark OR pending persistence (the page confirms before
  any switch, import or reset); `playground_write_file(path, text)` /
  `playground_read_file(path)` / `playground_list_files()` canonicalise the path and
  REFUSE `..`, absolute keys and anything outside the open project's root, then go
  through `vfs` (a `.rhai` write also runs `scripting::check_source`, batch 7, so errors
  show on Save in Edit mode); `playground_list_projects()`; `playground_open_project(slug)
  -> Promise` and `playground_reset_project(slug) -> Promise` clear pending, await every
  in-flight put (reset: `remove_project`), and resolve — the PAGE then sets the query and
  reloads (the engine cannot swap a running project — documented).
- `engine_core::web`: `pub fn query_param(name) -> Option<String>` beside
  `set_boot_status` (used by the entry for `?project=`).
- Gate hole closed: `scripts/check_wasm.sh` already covers the new member. Add one line
  to its header naming the playground crate as the reason the editor crates are in the
  gate.
- Docs: new `docs/WEB_PLAYGROUND.md` — the bundle contract (version places, the store's
  database and object stores, the CAS rule and why get-then-put is forbidden, the
  base-joined root rule, the bundle-version/reset rule, the bridge function list with
  argument and return shapes, the FIFO ordering contract and the refusal on a full
  queue, the "one embed per page" constraint, the status element id). `docs/EDITOR_COMMAND_API.md` § Stages: Stage D is the function
  bridge, shipped here. `docs/WEB_SAVES.md` gains a paragraph naming the playground's
  prefs key and pointing at the store doc.

Gates: standard + wasm (the crate must build for wasm32 and natively). Leaves out:
the build script, the page (batch 4), zip (batch 5).

## Batch 4 — bundle build and the `/playground/` page

Repos: `insiculous_2d` (`scripts/build_wasm.sh`, 173 lines) and `insiculous_web`.

Target shapes:

- `build_wasm.sh` gains `--kind games|playground` (default `games`, output
  `<kind>/<slug>/<version>/`) and, for `--kind playground`, a repeatable `--project
  <slug>=<title>=<dir>`: each `<dir>/assets` is copied to
  `assets/projects/<slug>/assets/…` — the `assets/` segment is KEPT, mirroring the source
  tree and the export-zip layout, so the entry's `{root}/assets` base finds every file —
  and `assets/projects.json` lists the manifests with `bundle_version` and a
  `content_hash` (sha256 over the sorted file list and bytes of `<dir>/assets`). The
  crate's own `assets/` copy is skipped for this kind. The `ASSET_BASE` assertion (`:49-58`) reads
  the kind and also asserts `BUNDLE_VERSION`. The `[profile.wasm-release]` check (`:76`)
  inspects the manifest `cargo locate-project --workspace --message-format plain` names
  (a game's root is its own manifest; the playground's is the engine root). Invocation
  of record: `scripts/build_wasm.sh crates/playground playground --kind playground
  --version v1 --project examples=Examples=examples --sync ../insiculous_web/public`.
- `insiculous_web`: `src/pages/playground.astro` on `BaseLayout` with a nav entry
  (`BaseLayout.astro:26-33`); `src/components/PlaygroundEmbed.astro` copying
  `GameEmbed.astro`'s loader shape verbatim (`new Function` import trick, `#game-loading`
  status, `#game-canvas` placeholder at 1280×800 with `tabindex`, `role`, `aria-label`,
  fallback text) and the batch-0 gate. Page controls, all keyboard-reachable and
  labelled: a project `<select>` (from `playground_list_projects`; a change asks
  `playground_is_dirty`, confirms with a native `confirm()`, AWAITS
  `playground_open_project`'s promise, then reloads), a "Reset to bundled" button shown
  for any bundled slug with stored files, with the note "the bundled project changed
  since you saved" when the content hash or bundle version differs (awaits
  `playground_reset_project`, then reloads), a command console (`<input>` + `<output
  aria-live="polite">` over `playground_dispatch`/`playground_poll_responses`, paired in
  FIFO order, a `false` return shown inline as "busy — try again"), the persistence
  banner region (`role="alert"`, filled on `on_persist_failed`), a `beforeunload`
  handler that warns while a put is pending, `canvas.focus()` after the project switch
  completes (never while a text control has focus), keyboard-shortcut list next to the
  embed (the README's playable-game accessibility requirements apply). Copy uses curly
  apostrophes; exactly one `<h1>`; no duplicate ids.
- `README.md` (site) § drop-in convention gains "the editor bundle" subsection;
  `docs/roadmap.md:128-131` updated to "shipped, route `/playground/`";
  `src/pages/engine.astro:34-39` links the playground.
- Deploy is Jesse's push of `jesse → dev` (staging) then `main`.

Gates: `npm run verify`; bundle gate; `shellcheck scripts/build_wasm.sh` if installed.
**Jesse's browser check:** open `/playground/` on staging, select an entity, move it
with the gizmo, press Play (the patrollers walk), Stop, Ctrl+S, reload — the move
persisted; the console answers `list`; switch project with unsaved edits — the page
asks first. Leaves out: export/import (batch 5), scripts.

## Batch 5 — project export and import (#49 items 1–2)

Files: `crates/playground/src/{archive.rs, bridge.rs, persist.rs, projects.rs}`,
`crates/playground/Cargo.toml` (`zip = { version = "4", default-features = false,
features = ["deflate-flate2"] }`), `insiculous_web/src/components/PlaygroundEmbed.astro`.

Target shapes:

- `archive.rs`: `pub fn export_project(project_root) -> Result<Vec<u8>, ArchiveError>`
  writes every file under the project's root (`assets/**`: scenes, `.sheet.ron`
  sidecars, scripts, images), a generated `README.md` (project title, the URL of
  `docs/SCRIPTING.md` and `docs/WEB_PLAYGROUND.md` on GitHub — NOT the template repo,
  which does not exist until batch 10 adds that sentence) and `project.ron`
  (`ProjectManifest`); `pub fn import_project(bytes) -> Result<(ProjectManifest,
  Vec<StoredFile>), ArchiveError>` validates every entry (path traversal refused,
  `project.ron` required and its slug is the project's identity, `.sheet.ron` sidecars
  run through `sheet_file::parse_sheet_file` — schema drift fails loud naming the file,
  the slug must match `^[a-z0-9_-]{1,32}$`, archives over 64 MiB refused) and returns
  the files; it does not touch the VFS. Tests
  (native, directory store): export then import yields byte-identical contents; a zip
  with `../` is refused; a bad sidecar names the file in the error.
- Persistence of an import: the bridge bumps the write epoch, clears pending, awaits
  `store.replace_project(slug, files, manifest)` (ONE transaction — on failure the
  previous project is intact, the banner shows the reason, nothing reloads, and the
  session keeps the import in `MemFs` after `vfs::remove_prefix(root)` + insert so it can
  be exported again), and only on success resolves so the page reloads with
  `?project=<slug from project.ron>`.
- Bridge: `playground_export_zip() -> Vec<u8>` and `playground_import_zip(bytes: &[u8])
  -> Promise<String /* slug */>`; the page offers the export as a Blob download link and
  an `<input type="file" accept=".zip">` for import, confirming through
  `playground_is_dirty` first.
- `docs/WEB_PLAYGROUND.md` § Export layout — the layout is the template repo's
  (batch 10), stated here first so batch 10 conforms to it.

Gates: standard + wasm. **Jesse's browser check:** export, edit something, import the
zip, the edit is gone and the export's state is back; reload — still back; the project
list shows the imported slug. Leaves out: template repo.

## Batch 6 — scripting Stage 2: scripts visible in the hierarchy and the asset browser

Files: `crates/editor/src/hierarchy/mod.rs` (`render` at `:231`),
`crates/editor/src/asset_browser.rs` (`AssetKind`, `:13-18`, `kind_for_extension`
`:69-75`), `crates/editor/src/drag_drop.rs` (`DragPayload`, `:18-23`),
`crates/editor/src/script_editor.rs`, `crates/editor_integration/src/panel_renderer/
{hierarchy.rs, asset_browser.rs}`, `crates/editor/src/editor_preferences.rs`,
`crates/editor_integration/src/editor_game/shortcuts.rs`.

Target shapes (audit §6.4 rows, adjusted for the web):

- Hierarchy emits one pseudo-row per `ScriptRef` under its entity at `depth + 1`,
  labelled by `script_id` (or the `.rhai` file stem), non-selectable as an entity;
  `render` returns `Vec<HierarchyClick>` where `HierarchyClick::Entity(id) |
  Script { entity, index }`; a script click selects the entity and asks the inspector
  to scroll its `Scripts` block into view (`ScrollState` already shared).
- `AssetKind::Script` for `rhai` (and `rs`, display-only), `DragPayload::Script { path }`;
  dropping a `.rhai` on a hierarchy entity row appends a `ScriptRef { script_id: <stem>,
  source_path: <relative path> }` through `SetScriptsCommand` (undoable).
- "Open in IDE": native only — `EditorPreferences.ide_command: Option<String>` and a
  `#[cfg(not(target_arch = "wasm32"))] std::process::Command` spawn in
  `editor_integration`; on wasm the button is absent and the page's textarea is the
  editor (batch 8). Inspector button label "Open source", status-bar message names the
  file either way.
- Tests: hierarchy rows for an entity with two scripts render two pseudo-rows and a
  click on the second reports `Script { index: 1 }`; a `.rhai` drop appends exactly one
  ref and undo removes it.

Gates: standard + wasm. Leaves out: execution (batch 7).

## Batch 7 — scripting Stage 3: registry, runner, Rhai

Files: new `crates/engine_core/src/scripting/{mod.rs, registry.rs, runner.rs, view.rs,
commands.rs, rhai_backend.rs, builtin/rotate.rs}`, `crates/ecs/src/blackboard.rs` (new
resource), `crates/engine_core/src/contexts.rs` (`GameContext.scripts`),
`crates/engine_core/src/game.rs` (`Game::register_scripts`, defaulted; runner
construction at the `GameContext` build site `:271`), `crates/editor/src/texture_field.rs`
(`InspectorExtras.script_catalog`), `crates/editor/src/script_editor.rs` ("+ Add Script"
picker grouped by category; unresolved ids in `theme.error_red`),
`crates/editor_integration/src/project_host.rs` (the runner call), status-bar
lie-detector in `editor_integration`. `Cargo.toml` (`engine_core`): `rhai = { version
= "1.26", default-features = false, features = ["std", "f32_float"] }` — if the wasm
gate needs it, the target block adds `features = ["wasm-bindgen"]`.

Target shapes:

- `pub trait ScriptBehavior { fn early_update(&mut self, view: &ScriptView, params:
  &BTreeMap<String, ScriptValue>, out: &mut ScriptCommands) {} fn update(&mut self,
  view: &ScriptView, params: &BTreeMap<String, ScriptValue>, out: &mut ScriptCommands)
  {} }` (both defaulted, no `&mut World`). Every `ScriptCommands` verb takes a
  `Target::{This, Named(String)}` first argument; `apply` resolves names through the
  world's `Name` components once per frame and reports a missing or ambiguous name as a
  deduplicated error.
  `pub struct ScriptDescriptor { pub id: &'static str, pub display_name: &'static str,
  pub category: &'static str, pub params: &'static [ParamSpec], pub make: fn() ->
  Box<dyn ScriptBehavior> }`; `pub struct ParamSpec { pub name: &'static str, pub
  default: ScriptValue }`. `ScriptRegistry::register(descriptor)` panics on a duplicate
  id at startup (the component registry's collision rule). `engine::rotate` (param
  `degrees_per_second: F32 = 90.0`) is the one built-in.
- `ScriptView` and `ScriptCommands` as in the decision above; `ScriptCommands::apply
  (world, physics: Option<&mut PhysicsSystem>)` mirrors `BehaviorCommands::apply`
  incl. the kinematic/dynamic/no-physics fallback (`behavior_runner/mod.rs:222-249`),
  which is what keeps it headless-testable.
- `ScriptRunner { registry, instances: HashMap<(EntityId, usize), Instance>, rhai:
  RhaiBackend, frames_run: u32, errors: ScriptErrors }`; two entry points,
  `early_update(&mut self, world, input, players, delta_time, physics)` and
  `update(&mut self, world, input, players, delta_time, collisions: &[CollisionData],
  physics: Option<&mut PhysicsSystem>)`, each building the view ONCE, sharing it across
  every instance (a counter test pins "one view per frame"), calling the matching hook,
  then applying the commands. At the start of each frame an instance whose entity no
  longer exists or no longer carries `Scripts` is pruned, so a despawning game does not
  grow the map. Resolution per `ScriptRef`:
  `source_path` ending `.rhai` → Rhai; else registry by `script_id`; unresolved → one
  `log::warn!` per id per Play and an entry in `errors()`. `ScriptErrors` deduplicates
  every error by (file, line, kind) per Play — a script that trips the operation budget
  every frame is reported once; the status bar and the page's `playground_script_errors`
  read the same list. `reset()` at Play start clears instances, the blackboard,
  `frames_run` and `errors`.
- `RhaiBackend`: one `rhai::Engine` with the `ScriptView` and `ScriptCommandsHandle`
  types registered (`register_type_with_name`, getters on the view, methods on the
  handle), `AST` cache keyed by `source_path` + content hash (a changed file recompiles
  on the next Play). Params travel as an ARGUMENT, not scope variables (Rhai `fn`s cannot
  read the calling scope): a `rhai::Map` built per call from the `ScriptRef` (F32→float,
  I32→int, Bool, Str, Vec2→`vec2` type, Entity→the target's name, Color→array). `out` is
  a `ScriptCommandsHandle(Rc<RefCell<Vec<ScriptCommand>>>)` because `call_fn` passes by
  value — the runner drains the shared buffer after the call; a plain struct would lose
  every command silently. Contract: the script defines `fn early_update(view, params,
  out, dt)` and/or `fn update(view, params, out, dt)`; a missing hook is simply not
  called (checked once at compile via `AST::iter_functions`); compile or runtime errors
  reach `ScriptErrors` with file and line, never a panic; `Engine::set_max_operations`
  guards runaway loops, and an instance that trips it is QUARANTINED for the rest of the
  Play session (reported once; Stop/Play re-enables it) so a buggy loop cannot burn its
  budget sixty times a second. **Numbers mix freely**: Rhai never coerces `INT` to
  `FLOAT`, so the engine registers `+ - * / % < <= > >= == !=` for every (`INT`,`FLOAT`)
  and (`FLOAT`,`INT`) pair, F32 params are always `FLOAT`, and every command method that
  takes a float has an `INT` overload — `dt * 450` and `out.set_velocity_x("Ball", 250)`
  both work. `pub fn check_source(text: &str) -> Result<(), ScriptError>` is
  the pure compile check the playground bridge runs on every `.rhai` save, so errors show
  in Edit mode, not only in Play.
- `ecs::Blackboard(BTreeMap<String, ScriptValue>)` World resource, registered
  transient-equivalent (never written to scene files); UiLabel text set through
  `ScriptCommands::set_label_text(name, text)`.
- `GameContext.scripts: &mut ScriptRunner` (engine-owned; built once in `GameRunner`,
  `Game::register_scripts(&mut self, registry: &mut ScriptRegistry)` called before
  `init`). `ProjectHost::update` order per Playing frame: behaviors →
  `ctx.scripts.early_update(...)` → physics step → drain collisions once →
  `ctx.scripts.update(...)` with that Vec → transform hierarchy. This is pong's own order
  (paddles → physics → drain → rules): a paddle's kinematic target set in `early_update`
  is where the collider is when the ball arrives this frame, and the goal's reaction in
  `update` sees this frame's contacts.
- Lie-detector: `EditorGame` records, at Play frame 60, whether any entity carries
  `Scripts` while `ctx.scripts.frames_run() == 0`, and shows "scripts attached but the
  game never ran the script runner" on the status bar once per Play.
- Catalog: `InspectorExtras.script_catalog: &[ScriptCatalogEntry]` (built-ins from the
  registry + every `.rhai` under `assets/scripts/` from the asset scan); "+ Add Script"
  becomes a picker grouped by category; typing a free id remains possible.
- Tests (contract-named, headless): a Rhai script that moves its entity toward a named
  target advances the transform over three frames; a script on entity A that resets
  entity B by name moves B; a missing target name is reported once and the rest of the
  frame's commands still apply; a compile error reaches the error list and leaves other
  scripts running; a runaway loop is reported once across many frames; `engine::rotate`
  rotates by `degrees_per_second * dt`; an unresolved id is reported once; `reset()`
  clears the blackboard; `early_update`'s kinematic target lands before the step and
  `update` sees an injected collision; kinematic target routes through physics when
  present and to `Transform2D` when absent.
- Docs: new `docs/SCRIPTING.md` — the author-facing contract (the `update` signature,
  every view getter and command method with types, params by name, the built-in list,
  error surfacing, "one bundle, many projects"); `PROJECT_ROADMAP.md` § Scripting
  updated (Rhai decision, Stages 2–3 shipped, game-run ruling); `training.md` gains a
  Script Pattern section; `crates/engine_core/CLAUDE.md` file map.

Gates: standard + wasm + games (`GameContext` grew a field; every game constructs
none, so `check_games.sh` suffices) . Leaves out: pong (batch 8), web textarea.

## Batch 8 — pong's gameplay as a project, and script editing on the page

Repos: `insiculous_2d` (`crates/playground/assets/projects/pong/`, `build_wasm.sh`
project list), `insiculous_web` (textarea in `PlaygroundEmbed.astro`).

Target shapes:

- Project `pong` under `crates/playground/assets/projects/pong/assets/`: `scenes/pong.scene.ron`
  (background, two paddles, ball, two walls, two goal sensors, a `Scoreboard` UiLabel —
  the shapes from `games/pong/src/spawning.rs` and `constants.rs`, textures
  `paddle_16px.png` and `ball_8px.png` copied, `#white` for walls), `scripts/paddle.rhai`
  (`early_update`; params `player: I32`, `x: F32`, `speed: F32 = 450`, `ai: Bool`,
  `ai_speed: F32`, `dead_zone: F32`, `target: Entity = Ball`; kinematic target on `self`
  clamped to the playfield), `scripts/ball.rhai` (`early_update` serves on Action1 when
  the blackboard says `serving`, direction from `last_scorer`; `update` maintains speed
  as `gameplay/balls.rs:27-45` does; params `speed: F32 = 250`, `max_vertical: F32 =
  500`), `scripts/goal.rhai` (`update`; param `side: Str`; on a started collision with
  `Ball`: award the point in the blackboard, `reset_body(Named("Ball"), centre)`, set
  `serving`), `scripts/scoreboard.rhai` (`update`; writes "L : R" and "LEFT WINS —
  Action1 to restart" at 7 to `set_label_text(This, …)`; restart zeros the blackboard).
  The pseudo-random serve spread comes from `view.frame` hashed as `serve_direction`
  does.
- `build_wasm.sh` invocation adds `--project pong=Pong=crates/playground/assets/projects/pong`.
- Page: a "Scripts" panel — `<select>` of the project's `.rhai` files (via
  `playground_list_files`), a `<textarea>` bound to `playground_read_file`/
  `playground_write_file`, a Save button (Ctrl+S / Cmd+S inside the textarea
  `preventDefault`s and runs the same Save), `aria-live` status for compile errors: Save
  shows `check_source`'s result immediately (Edit mode), and during Play the panel polls
  `playground_script_errors()` for runtime errors. Focus returns to the canvas only via
  the Play control, never while the textarea is being typed in.
- Tests: the pong project's four scripts compile in a headless test that loads the
  scene with no physics system, presses Action1 through a scripted input frame and
  asserts the ball leaves centre (serve), then INJECTS a synthetic `CollisionData` for
  the ball/left-goal pair into `ScriptRunner::update` and asserts the blackboard awards
  the right side a point, the ball is back at centre and `serving` is set (no physics
  detects anything here; the test pins the rules through the no-physics fallback, not
  rapier).
- Docs: `docs/SCRIPTING.md` § Worked example: pong; `games/pong/README.md` "Deion
  Pivot"/playground note pointing at the project.

Gates: standard + wasm + `npm run verify` + bundle. **Jesse's browser check:** open
`/playground/?project=pong`, Play, a rally with the AI, score, edit `paddle.rhai`'s
speed in the textarea, Save, Stop, Play — the paddle is faster; **reload the tab** —
the edit is still there; with an entity selected, type Delete and Ctrl+Z inside the
textarea — the viewport is untouched (rebuttal 1, gemini F5). Leaves out: menus,
power-ups, chaos modes, achievements (filed).

## Batch 9 — the six games as editor bundles (independent, droppable)

Repos: the six game repos (`src/web_entry.rs`, `Cargo.toml` if the optional dep is
missing on any), `insiculous_2d/scripts/build_wasm.sh`, `insiculous_web`.

Target shapes: each game's `web_entry.rs` selects `run_game_with_editor` under
`#[cfg(feature = "editor")]` and `ASSET_BASE` under `--kind playground`
(`/playground/<slug>/v1/assets`); `build_wasm.sh --features editor` passes the feature
through; the site's `/playground/` project `<select>` lists the six as "Rust games
(layout only — rules are compiled in)", each opening `/playground/<slug>/` (one embed
per page, so a page per bundle: `src/pages/playground/[slug].astro` over a `playground`
content collection). Six commits, one per game repo, each gated by that game's
`cargo test` + `cargo clippy` with and without `--features editor` AND `cargo check
--manifest-path ../games/<g>/Cargo.toml --lib --target wasm32-unknown-unknown --features
editor` (the editor path is otherwise never compiled for wasm until bundle time); one
commit each in the engine and the site. Bundle gate on all six (expect +2–3 MiB each).

**Jesse's browser check:** `/playground/pong/` opens the Rust pong inside the editor;
Play runs the real game. Leaves out: nothing else; if dropped, file it.

## Batch 10 — the template repo (#49 item 3) and docs close-out

- **Ask Jesse before creating `beinsiculous/game-template`.** Contents follow
  `.claude/skills/new-game` (module layout, chaos modes, pause/menu chrome, headless
  tests, `build_wasm.sh` usage) with `assets/scenes/`, `assets/scripts/` and a README
  whose layout matches the export zip exactly (batch 5's layout section), plus the
  "Use this template" setting. The exported README's URL points here.
- `PROJECT_ROADMAP.md` (§ Web Playground → shipped; § Scripting; phase map row);
  `CLAUDE.md` status paragraphs (editor, web, scripting); `README.md`; `log_archive.md`
  entry with the lessons; `coordination/PROGRESS.md` entries per batch; the reviewer
  ledger closed with the default-reviewer number.
- Close #48, #49 with the commits; file follow-ups: the other five games as projects;
  pong menus/power-ups/chaos/achievements as data (needs menu and achievement script
  surfaces); the eight `Behavior`s as built-in scripts; `#[derive(Script)]` once
  `ParamSpec` settles (re-decide #83 then); contact points and normals in `ScriptView`
  for a game whose rules read them (pong's never did); a criterion bench for view-building +
  script dispatch at 200 named entities × 30 instances (a timing assertion is not a
  `cargo test` — flaky on shared CI, and `#[ignore]` is forbidden); batch 9 if dropped.
- Merge `jesse → dev` in both repos once every gate is green; Jesse pushes.

## Verification (end to end)

- Engine: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  `scripts/check_wasm.sh`, `scripts/check_games.sh --test`, the comment-tag grep — all
  clean on the final `jesse`.
- Bundle: `scripts/build_wasm.sh crates/playground playground --kind playground
  --version v1 --project examples=... --project pong=... --sync ../insiculous_web/public`
  under 20 MiB.
- Site: `npm run verify` green; staging deploy; Jesse's browser checks for batches 4, 5,
  8 (and 9) recorded in `coordination/PROGRESS.md` — every one includes a reload, so
  persistence is proven, not assumed.
- Board: insiculous_web#4, #48, #49 closed by commits; Sprint 6's five closed in batch 0;
  every follow-up filed.
