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
v4 reviewed by kimi (`review-5.md`, 5) and gemini (`review-5-gemini.md`, 8), adjudicated in
`rebuttal-5.md` — all 13 accepted: the first `put` for a slug upserts its manifest and the
orphan sweep spares bundled slugs (v4's sweep would have deleted every save of a bundled
project), `EditorGame` forwards `register_scripts`, a per-instance `me` beside a per-PHASE
shared view (and `me`-or-name command overloads), blackboard reads with defaults,
drain-then-epoch on switch, root-joined default scene path, StaleRevision gets the export
banner, bounded awaits, zip directory entries skipped, a per-file delete verb filed.
Batch 1 landed (2cdbcc1). v5 reviewed by kimi (`review-6.md`, 10) and gemini
(`review-6-gemini.md`, 10), adjudicated in `rebuttal-6.md` — 19 accepted, 1 in part (Rhai's
standard package already has the math): boot seeds base revisions, `.rhai` paths resolve
against the asset base, one `drain_then_epoch()` for switch/reset/import, a failed import
touches nothing, batch 3's script hooks are `None` until batch 7, the lockfile and the
output path for the playground kind, precise pending states, getters-only `me`, `vec2`,
`game_over`, first-frame physics before the phases, `apply(dt)`, export takes the manifest. v6 reviewed by kimi
(`review-7.md`, 9) and gemini (`review-7-gemini.md`, 7), adjudicated in `rebuttal-7.md` — all
16 accepted: `// @param` header defaults for `.rhai` scripts, the chain's failure state
defined, scenes dry-run on import, epoch bumps at the start of the drain, an errored call's
buffer discarded, `check_source` honest as syntax-only, `Rc` views, side-aware win text,
late-put base recording; deterministic instance order with resets applied before velocities,
`cargo metadata` for the target dir and a kind-aware sync path, queued puts issued on
`pagehide`, a terminal *conflicted* state, the textarea's own dirty flag, backslash zip paths,
zero-vector `normalize`. **This is v7, the settled plan** (Jesse, 2026-09-04: no round 7;
corrections from here go into the acting batch section before its handoff, and every batch's
staged diff is reviewed by kimi and Claude). Batch 2 landed (936bcf9). Batch 3's section was re-verified against the tree before its handoff (2026-09-04): the corrections are listed at the top of that section, and batch 4's and 7's cross-references to the two replaced hooks were updated with it. Batch 3 landed (1462cbe). Batch 4's section was re-verified the same way before its handoff (2026-09-04); its corrections are listed at the top of that section, and the conflicted-path download control it deferred is recorded in batch 5. Batch 4 landed (e362625 in insiculous_2d, f69f09e in insiculous_web): kimi reviews 14–17, Claude review-14-claude, rebuttals 14–17; it is marked done once Jesse's browser check on staging passes. Batch 5's section was re-verified against the tree 2026-09-05; its corrections are listed at the top of that section (the `flate2` backend line, the headless dry-run resolver, the bundle rebuild, the page's script file, the conflicted-paths export), and batch 7's docs bullet gained the export README's second link. Batch 6's section was re-verified against the tree 2026-09-05; its corrections are listed at the top of that section (the response struct that already exists, the host file that does not, the add-in-the-same-undo-entry drop, the prefs field the save path would wipe, the scroll-into-view mechanism). Batch 6 landed (d4b384a): kimi reviews 20–22 (the section correction, the diff, the planner's round 2), Claude review-21-claude, rebuttals 20–22; two follow-ups filed — #102 (`ide_command` set only by hand, lost to autosave) and #103 (edits made while Paused are erased by Stop, audit §1.5, the standing rule kimi re-raised against the new drop path). Batch 7's section was re-verified against the tree 2026-09-05; its corrections are listed at the top of that section (the one context macro, the physics feature gate, the host signature, the bridge's missing hook setter, the error mirror, the `mod.rs` budget, the catalog's scan and build site, the resource rule). Batch 7 landed (f2431ae): kimi reviews 24–25, Claude review-24-claude, rebuttals 24–25; the planner's round-3 hunks (the per-entity velocity fold) are not kimi-reviewed; one follow-up filed — insiculous_2d#105 (a `.rhai` entity header default pre-fills as a Str). Batch 8's section was re-verified against the tree 2026-09-05; its corrections are listed at the top of that section (the page script the panel really lives in, the save that is one refusing call, the poll that cannot know Play, the Rhai clamp that does not exist, the lowercase action and header names, the scene shapes and coordinates pinned from the game, the test's crate and dev-dependencies, the tracked bundle in the site repo, the third repository); kimi review 26, gemini review-26-gemini, Claude review-26-claude, rebuttal 26 — ten accepted (the colliders' friction and restitution and the paddles' bodies that the section had left to defaults, the win test's one-phase lag, the path dev-dependencies, the background's true size, the serve hash, two status elements, the select's cancel reset), one rebutted (a Rhai `&str` parameter takes any string). Batch 8 landed (59365d4 in insiculous_2d, 23c5471 in games/pong): kimi review 27, Claude review-27-claude, rebuttal 27; accepted: the serve doc was inverted and the plan's `"left"` last-scorer default sent the first serve the opposite way from the Rust game, so the default and the restart now match it, plus a test note on the two branches the no-physics fallback never runs; rebutted as policy: the ±230 paddle clamp's 10 px sink into the wall is the Rust game's own constant, filed as beinsiculous/pong#2; the planner's fix hunks are not re-reviewed; the site half (the Scripts panel, its module, the page copy, the re-synced bundle) landed as 6e67f93 in insiculous_web once `npm run verify` ran green under Node 24 via nvm (Iroh had only apt's Node 18; the log is review/web-playground/gates-8-site.log — astro check 0 errors, axe clean over 66 pages, no sideways scroll at four viewports); it is marked done once Jesse's browser check passes. Batch 5 landed (ceb77be in insiculous_2d, 227a5f2 in insiculous_web): kimi reviews 18–19 (the section correction and the diffs; the site and round-2 reviews were written under `--out` names that did not survive), Claude review-19-claude, rebuttals 18–19; the export-cap parity follow-up is insiculous_2d#101; it is marked done once Jesse's browser check on staging passes. Batch 9's section was re-verified against the tree 2026-09-06; its corrections are listed at the top of that section (the optional dependency every game already has, the third build kind and the differently named constant it checks, the clamped canvas size the page must carry, the save keys an editor session must not write and the prefs slot it must name, the `lib.rs` sentence, the `editor` field on the games collection in place of a new one, the static optgroup the population code must not wipe, the shortcuts list moved to a component, the bundle weight as a number, the lockfiles, the standing wasm editor check, the executor substitution). Batch 9 landed 2026-09-06 across the eight repositories (executor: Jesse's Claude Code session on Opus 5 at low effort): kimi review 29, Claude review-29-claude, rebuttal 29 — accepted: the shortcuts panel's Ctrl+S line now names what each surface saves to (a `saveLine` prop), the build script asserts the editor's minimum-size constants it mirrors, pong's README sentence moved after its list; rebutted: the games gate hard-requiring the wasm target (the section's own ruling) and the five lockfiles kimi thought unstaged (zero diff against HEAD); the planner's fix hunks are not re-reviewed; the six editor bundles weigh 8.66–9.35 MiB each; two follow-ups filed — insiculous_web#51 (a game's page does not link to its editor page) and insiculous_2d#107 (a game's editor bundle cannot persist layout edits; lean retire); it is marked done once Jesse's browser check passes. The section correction (review 28) and this landing share one plan commit. Batch 10's section was re-verified against the tree 2026-09-06; its corrections are listed at the top of that section (the export README that names no template, the seat the working set lacks, the dependency form taken as a ruling, the manifest at the template's root, the native runner contract, what pong tracks), and the docs close-out, the ledger, the board and the merge moved to a batch 11 of their own. Batch 10 landed 2026-09-06 (be328ee engine, f860116 template, e8c4e69 site, b4d7049 plan; executor: Jesse's Claude Code session, from handoff-10.md): kimi review 31 (3 findings), Claude review-31-claude (4), rebuttal 31 — accepted: the template obeys the scene's physics block through `physics_for` (kimi's catch: an Examples export would have floated on the desktop), the coin's corners are a list rather than a divisor, the two guide lines, the spare image deleted; the executor's own deviation — the Coin as a `Dynamic` body, because a kinematic sensor fires nothing — accepted and written into the section; the planner's behaviour-changing fixes went back through kimi as review 32 on `draft-10-fixes.diff`, rebuttal 32; the editor-feature clippy joined the ground rules. Batch 11 landed 2026-09-06 (167e1d1 engine, e652201 site, aa2938d the template's README, c3d0dd3 plan; executor: a Claude Code session, from handoff-11.md): kimi review 33 (2 findings), gemini review-33-gemini (1), Claude review-33-claude (3), rebuttal 33 — every finding accepted; gemini's was the effort's last real catch, the drop-in command that would have played the template's coin demo instead of the export because `unzip -o` empties nothing and `main.scene.ron` sorts first, corrected in four documents; kimi's numbering catch corrected batches 5's and 6's rebuttal ranges in this paragraph; the ledger closed with its recommendation as a number. The board followed the same day: insiculous_2d#48 and #49 and insiculous_web#4 closed with comments naming the commits; the eight follow-ups filed as insiculous_2d#108 (the other five games as data projects), #109 (pong's menus, power-ups, chaos and achievements as data), #110 (the eight Behaviors as built-in scripts), #111 (`#[derive(Script)]` once `ParamSpec` settles), #112 (a per-file delete verb), #113 (contact points and normals in `ScriptView`), #114 (the criterion bench) and #115 (a scene with no physics block: one behaviour). Jesse merged `jesse → dev` himself in every repository on 2026-09-06 and the staging deploy went green once M's announce gate learned `/placeholder:` (insiculous_web 1c23569); his first browser check on staging, in Chromium, passed the load and run and filed insiculous_2d#116, #117 and insiculous_web#52 (the pointer offset, the trackpad zoom, the pop-out) — the save-and-reload, the export and the native drop-in stay owed, so batches 4, 5, 8, 9 and 10 keep their landing marks until then. `dev → main` is `review/web-playground/promote-main.sh`, Jesse's call.

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
  Each PHASE the runner builds one shared `ScriptView` — every Named entity's transform
  and velocity, per-player input axes and just-activated actions, the phase's collision
  events by name (empty before the step), the blackboard, the frame counter, `dt` — and
  hands every instance that view plus its own `me: SelfView { name, transform, velocity }`
  and its `params`; the script returns through a `ScriptCommands` buffer that the runner
  applies afterwards — exactly the `BehaviorCommands` pattern in
  `behavior_runner/mod.rs:39-54,211-280`. **Every command names its target**: `me` or an
  entity `Name`, via overloads (set position, kinematic target, velocity, reset body,
  sprite color, sprite visibility, UiLabel text, despawn) — a goal sensor resets the ball,
  not itself; blackboard writes are global and blackboard reads take a default, so an
  unset key is a value, never an error. A missing or ambiguous target is a per-Play
  deduplicated error, never a panic. Scripts expose two optional hooks: `early_update`
  runs BEFORE the physics step (input, kinematic movement) on a view with pre-step
  transforms and no collisions, and `update` runs AFTER the step and the collision drain
  (reactions, scoring) on a view with post-step transforms and the frame's contacts —
  pong's own order, and the only way a kinematic paddle's collider and sprite agree in
  the same frame. One shared view per phase, two per frame, pinned by a counter test.
  Instances run in a deterministic order (`BTreeMap` keyed by entity id then script
  index — a `HashMap` would let the goal's reset and the ball's speed maintenance land in
  either order), and `apply` orders commands by kind: every `reset_body` first, then
  positions, then velocities and kinematic targets — dropping a velocity aimed at an
  entity reset in the same call — then sprite, label and blackboard writes, then
  despawns. A hook that errors mid-call has its buffer for that call DISCARDED: an
  errored script did nothing. Rust scripts receive `&mut ScriptCommands`; Rhai scripts
  receive a `ScriptCommandsHandle` (`Rc<RefCell<Vec<ScriptCommand>>>`) because Rhai
  passes by value, `me` and `view` reach Rhai as `Rc<SelfView>` / `Rc<ScriptView>`
  handles with getters only (a clone is a pointer, not the snapshot), and `me` is named
  `me` because `this` is a Rhai keyword. A `.rhai` script declares its params in a header
  block — `// @param speed: f32 = 450` (types `f32 i32 bool str vec2 entity color`) —
  which is the defaults channel: attach pre-fills `ScriptRef.params` from it, a declared
  param missing on the ref takes the header default at run time, and a param declared
  nowhere is a named error. Safe, headless testable, deterministic, and the same rules
  for Rust and Rhai.
- **Persistence on the web is IndexedDB, now** (Jesse, reversing the v1 localStorage
  overlay: future-proof upfront rather than migrate later). A `ProjectStore` holds
  per-file records `{ project, path, bytes, revision, bundle_version }` plus each
  project's manifest, in one database `beinsiculous.playground`, object stores `files`
  and `projects`. Rules, each of which closed a review-2 finding:
  - **No debounce, one chain per path.** Saves are user-initiated (Ctrl+S, textarea
    Save, import), so every `vfs::write` on the web inserts into `MemFs` synchronously and
    starts its store put immediately — unless a put for that path is in flight, in which
    case the new bytes are QUEUED behind it and start with its resolved revision as base
    (a tab must not race itself). A path is in exactly one state: *idle* (stored bytes
    match), *in flight* (a put running, possibly with newer bytes *queued* behind it),
    *stranded* (its last put failed with `Backend`/`Unavailable`, nothing running, and it
    holds the NEWEST bytes), or *conflicted* (`StaleRevision`: terminal — never retried,
    exited only by export or reload). "Pending" means not idle. On an in-flight `Err` the
    slot frees and the queued bytes start at once with the same base; a stranded path
    always carries the newest bytes and a re-issue always puts those.
    `visibilitychange`→hidden re-issues STRANDED paths only; `pagehide` additionally
    issues QUEUED puts at once (best effort — if the in-flight put has not committed the
    queued one fails its CAS and the bytes stay only in `MemFs`; `beforeunload`'s warning
    while anything is pending is the guard, and the plan says so plainly). A put that
    resolves after its drain timed out records the stored revision as the tab's base
    silently — this tab lost nothing. **Boot seeds the tab's base revisions** from the
    files `load_project` returned, so the first save after a reload is not a conflict.
  - **Every put is a compare-and-swap** inside ONE `readwrite` transaction, with the
    dependent `put` issued synchronously inside the read's `onsuccess` (an `await` in
    between would find the transaction inactive): read the stored revision, refuse unless
    it equals the tab's base revision, write base + 1. An absent record accepts only
    base 0. **The first put for a slug with no stored manifest upserts one** in the same
    transaction (slug, title, `content_hash`, `bundle_version` copied from the bundled or
    imported manifest), so a saved bundled project is never manifest-less. A refusal is
    a GENUINE conflict (the per-path chain rules out a tab racing itself) →
    `StaleRevision` surfaces like `Backend`: the persistent banner, worded "another tab
    saved <file> after you loaded it; reloading discards THIS tab's version — export
    first to keep it", with a "download this file" affordance. Never get-then-put across
    two requests.
  - **Epoch, then drain — one helper.** Import, reset and project switch all call the
    same `drain_then_epoch()`: bump the write epoch FIRST (a `vfs::write` observed under
    an older epoch is refused with "project is being replaced — save again after the
    reload", accurate because the page reloads; without this a first-time put born
    during the drain lands after the remove and resurrects the slug), then DRAIN every
    per-path chain — in-flight AND queued puts — bounded at 5 s (on timeout the promise
    rejects with a status message that names tab visibility as the usual cause and
    invites a foreground retry; the epoch is restored and the current project stays
    loaded). Nothing queued before the switch is lost. **A failed replace touches
    nothing**: the epoch is restored, no `remove_prefix`, no insert; the current project
    runs on and the banner offers the uploaded archive back as a download.
  - **Import and reset are atomic**: `replace_project(slug, files, manifest)` is one
    transaction across both object stores; the manifest is the commit marker; boot
    ignores a slug that has files but no manifest, and `sweep_orphans` removes such files
    ONLY when the slug is also absent from the bundle's `projects.json` (a bundled slug
    with files is a user's saved work, never an orphan).
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
    under plus its `origin: bundled | saved | imported`; when hash or version differs
    from the booting bundle the page says "the bundled project changed since you saved"
    (origin `saved`) or "you imported over the bundled project" (origin `imported`)
    beside the Reset control. A version bump is a consistency check; the hash is the
    freshness check.
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
  --all-targets` (0 warnings), `cargo clippy --features editor --all-targets` at the engine
  root (the only gate that compiles `src/bin/editor.rs`, which sits behind the root
  package's `editor` feature — batch 10's executor found a public item deletable with every
  other gate green and the editor binary broken), every touched file ≤ 600 lines, no new `#[allow]`, no
  `unwrap()` outside tests, no new dependency beyond `rhai` (batch 7), `zip` with its
  `flate2` backend line (batch 5)
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

## Batch 2 — the data-only host moves into `editor_integration` — DONE 2026-09-04 (936bcf9)

Authored by gemini from `review/web-playground/handoff-2.md`; reviewed by kimi (`review-8.md`,
4 accepted) and Claude (`review-8-claude.md`, 4 accepted); adjudicated in `rebuttal-8.md`.
**Ruling recorded:** the host never invents physics — a scene with no `physics:` block runs
Play with no `PhysicsSystem` and behaviors move transforms directly (with a physics system
present, a body-less entity's velocity goes to a rapier body that does not exist, so the old
platformer-gravity default froze every pure-behavior scene). Stated in `project_host.rs`'s
module doc and the integration guide's pitfall table. Also landed: named entities rebuilt
every Playing frame (the command API can create and rename mid-Play), a logged
initialisation failure, `test_`-prefixed contract tests. `Game::update` delegates to
`pub(crate) fn update_frame(world, input, delta_time)` for headless tests — batch 7 adds the
script phases there. Every gate green (`gates-2-fixed.log`).

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
  `on_play_stopped` drops physics and clears the runner's named map. The lazy build runs
  at the TOP of the first Playing frame, before behaviors, so first-frame movement
  reaches rapier rather than the no-physics fallback (if this batch lands otherwise,
  batch 7 moves it when it restructures the frame). Batch 7 adds the script runner call
  here — this batch leaves a doc line naming that seam, not a stub.
- `src/bin/editor.rs` keeps only argument parsing, the stdin reader thread and the two
  `run_*` calls; the `EditorApp` struct is deleted. Behaviour identical.
- A headless test in `project_host.rs`: a world with one `Behavior::Patrol` entity
  advances position over simulated Playing frames (proves the standalone editor's Play
  now runs behaviors — it did not before). Test names per `training.md` § Writing Tests.
- Docs: `crates/editor_integration/CLAUDE.md` file map; `README.md` standalone-editor
  paragraph; `docs/EDITOR_COMMAND_API.md` Stage C paragraph names `ProjectHost`.

Gates: standard + wasm. Leaves out: scripting, web entry.

## Batch 3 — the `playground` crate: web entry, IndexedDB project store, bridge — DONE 2026-09-04 (1462cbe)

**Re-verified against the tree 2026-09-04 before the handoff** (the section changed most
across the six rounds, and it named things the tree contradicts). Corrections, each stated
where it applies below: the adapter file is `store/idb_transaction.rs` (the files list said
`idb_request.rs`); `EditorRunOptions` is named field by field, and two hooks that pointed the
wrong way across the `run_game` boundary are replaced — `on_persist_failed` (a closure the
editor owns cannot be invoked by `persist.rs`, which never holds the editor) becomes the
shared `persist_pending` flag, and `source_check`/`script_errors` move to the bridge's own
`Hooks` (the bridge cannot read options `run_game` has consumed; batch 7's cross-reference
is corrected in place); preferences save on a **settle rule** the editor runs itself (a DOM
`visibilitychange` listener cannot reach the editor synchronously, so the decision's
"hidden / before switch" triggers were unimplementable as written — the settle rule is
strictly stronger and keeps "never only in `on_exit`"); `default_scene_path` joins a base
the editor captures at `init`, since the method has no `ctx`; the entry passes
`initial_scene`; `mod.rs` is at 588 lines and splits; `engine_core::web` is wasm-gated;
line numbers refreshed. The corrected section was reviewed in code mode by kimi
(`review-9.md`, 5 findings) and gemini (`review-9-gemini.md`, 6), adjudicated in
`rebuttal-9.md` — all 11 accepted (one in part): the load side reads `prefs_slot` too, the
settle rule is time-based and skipped during Play, `is_pending` and `has_active` are two
predicates so a conflicted path never blocks a switch, Reset is gated on stored files,
explicit relative save paths join the base, bridge paths are project-relative, the
observer fires once, and `StoredFile` drops `content_hash`.

**Corrected after the round-1 code review (2026-09-04; kimi `review-10.md`, planner
`review-10-claude.md`, `rebuttal-10.md`; fixes to the executor as
`3-fixes-for-gemini.md`):** the wasm layer must spawn every started put and feed its
result back (`Chains::take_started_puts` + `drive_started_puts`) — the batch shipped the
state machine with no driver; the transaction adapter's abort handler surfaces the CAS
result cell; `Chains` tracks `draining` itself and `on_vfs_write` takes no epoch; the
`pagehide` re-put is deleted (below); `ProjectStore` carries no `Send + Sync` bound; the
directory double stages and swaps; the persist file is `persist/{mod.rs, tests/}`.
Round 2 (kimi `review-11.md`, planner `review-11-claude.md`, `rebuttal-11.md`): the
planner's own round-1 prescription for the abort handler was the defect — only a
refusal survives an abort, a callback's early `Ok` never does (quota aborts were silent
data loss); the project root is a directory boundary, not a string prefix; a `.rhai`
write is checked before it is written; a failed preferences write stays pending and
retries; the open project's manifest-less files are removed at boot. Round 3 (kimi
`review-12.md`, `rebuttal-12.md`) reviewed those planner fixes: an outside-root write is
bannered, not only logged; `manifests()` failures are logged; one hitched frame no longer
commits a mid-gesture preference; rebutted: clearing `draining` on a successful drain (a
post-drain write would resurrect a removed project) and the non-BMP key-range claim
(IndexedDB orders strings by UTF-16 code unit).

Files: new `crates/playground/` (`Cargo.toml`, `src/lib.rs`, `src/web_entry.rs`,
`src/bridge.rs`, `src/store.rs`, `src/store/{directory.rs, memory.rs, indexed_db.rs,
idb_transaction.rs}`, `src/projects.rs`, `src/persist.rs` — `persist/{mod.rs, chain.rs}`
if the state machine plus its tests pass 600 lines), root `Cargo.toml` (workspace member
AND `[profile.wasm-release]` — cargo ignores profiles in member manifests),
`crates/engine_core/src/web/mod.rs` (one addition; the module is
`#[cfg(target_arch = "wasm32")]` at `lib.rs:27-28`) and `crates/engine_core/Cargo.toml`
(web-sys features `Location`, `UrlSearchParams`), `crates/common/src/vfs.rs` (a
write-observer hook; the file is at 501 lines with inline tests from `:289` — if the
observer plus the key-story test cross 600, split into `vfs/mod.rs` + `vfs/tests.rs`),
`crates/editor_integration/src/editor_game/{api.rs, mod.rs, scene_io.rs, play_session.rs,
headless.rs}` plus new `editor_game/preferences.rs` (response channel, dirty flags, prefs
slot, base-joined default scene path), `crates/editor/src/editor_preferences.rs` (a
`PartialEq` derive), `docs/EDITOR_COMMAND_API.md`, `docs/WEB_SAVES.md`, new
`docs/WEB_PLAYGROUND.md`. The sample project is `examples/` (its two scenes under
`examples/assets/scenes/`; `behavior_demo.scene.ron` sorts before `hello_world.scene.ron`
and opens first); the build script (batch 4) copies it to `projects/examples/`.

**First deliverable, before anything else in this batch: the size probe.** Add
`[profile.wasm-release]` (pong's four lines, `../games/pong/Cargo.toml:28-32`: `inherits =
"release"`, `opt-level = "s"`, `lto = true`, `strip = "debuginfo"`) to the ROOT
`Cargo.toml`, then build a bare `crates/playground` cdylib (entry that only calls
`run_game_with_editor_opts` on `ProjectHost`) with `cargo build -p playground --lib --target
wasm32-unknown-unknown --profile wasm-release` (output under
`target/wasm32-unknown-unknown/wasm-release/playground.wasm`) + `wasm-bindgen --target web
--out-dir <scratch>`; the `.wasm` size is recorded in this section. Past 15 MiB, the planner
decides font (`crates/editor/src/fonts.rs` embeds 1.81 MB of DejaVu — 760 + 709 + 343 KB)
and later rhai trimming before batch 4 commits to a page. (Reference: pong ships at 2.5 MiB.)
**Measured 2026-09-04:** raw cdylib 8,834,928 bytes (8.43 MiB); after `wasm-bindgen
--target web` 6,949,393 bytes (6.63 MiB). Under the ceiling — no font or rhai trimming
before batch 4.

Target shapes:

- `crates/playground/Cargo.toml`: `[lib] crate-type = ["cdylib", "rlib"]`, deps
  `editor_integration`, `engine_core`, `common`, `serde`, `serde_json`, `log`; wasm
  target deps `wasm-bindgen = "=0.2.126"` (the pin `engine_core` and `renderer` carry and
  `build_wasm.sh` asserts), `wasm-bindgen-futures = "0.4"`, `js-sys = "0.3"`, `web-sys =
  "0.3"` with the `Idb*` features the store needs plus `Window`, `Document`, `Location`,
  `Event`, `BeforeUnloadEvent`, `Element` for the listeners and the banner. Native
  dev-dep `pollster` (already a workspace dependency — not a new crate) is allowed for
  tests that await a store call to completion. No `[profile.*]` here (root only). No
  native `main.rs` (the standalone `editor` bin is the native face).
- `store.rs`: `pub trait ProjectStore { fn load_project(&self, slug) -> Fut<Result<Vec<
  StoredFile>, StoreError>>; fn put(&self, file: StoredFile, base_revision: u64, manifest:
  &ProjectManifest) -> Fut<Result<u64 /* new revision */, StoreError>>; fn
  replace_project(&self, slug, files: Vec<StoredFile>, manifest: ProjectManifest) ->
  Fut<Result<(), StoreError>>; fn remove_project(&self, slug) -> Fut<Result<(),
  StoreError>>; fn manifests(&self) -> Fut<Vec<ProjectManifest>>; fn sweep_orphans(&self,
  bundled: &[String]) -> Fut<Result<(), StoreError>>; }` where `Fut<T> = Pin<Box<dyn
  Future<Output = T>>>` (no `async_trait` crate), `StoredFile { project: String, path:
  String /* project-relative */, bytes: Vec<u8>, revision: u64, bundle_version: String }`, `StoreError::{Unavailable, StaleRevision { stored: u64, base: u64
  }, Backend(String)}`. **`put` is a compare-and-swap**: inside ONE `readwrite` transaction
  it reads the stored revision, refuses with `StaleRevision` unless it equals
  `base_revision`, else writes `base_revision + 1` and returns it. **`replace_project` is
  one transaction** across both object stores (`files` cleared for the slug, new files
  written, manifest written last as the commit marker); `remove_project` likewise; `put`
  upserts the slug's manifest when the store has none (copied from the `manifest`
  argument with `origin` set to `saved`; same transaction); `sweep_orphans(bundled)` removes `files` whose slug has no
  manifest AND is not bundled (boot calls it with `projects.json`'s slugs). `StoredFile`
  carries `bundle_version`; only `ProjectManifest` carries `content_hash` (nothing in this
  crate hashes — the build script computes it, and no consumer reads a per-file hash).
  `store/directory.rs` (native; the test double for every contract — a lock file per
  project makes the CAS honest), `store/memory.rs` (the fallback when IndexedDB will not
  open; all targets), `store/indexed_db.rs` (database `beinsiculous.playground` v1, object
  stores `files` keyed `[project, path]` and `projects` keyed `slug`),
  `store/idb_transaction.rs` (the adapter — it wraps a whole TRANSACTION's
  `complete`/`abort`/`error` events into a `Future`, one `Closure` trio,
  `Rc<RefCell<Option<Result>>>`, waker; every dependent request — the CAS's `put` after
  its `get`, each file of a `replace_project` — is issued synchronously inside the
  previous request's `onsuccess` callback, never after an `await`, because the
  transaction is inactive by then; the only place web-sys IDB verbosity lives).
- `persist.rs`: the write path. `common::vfs` gains `set_write_observer(fn(&Path))`
  (wasm-only; called ONCE per write, inside `vfs::write` after its `MemFs::insert` —
  `write_string` delegates to `write` (`vfs.rs:63-65`) and must not notify again; the
  boot-phase free function `vfs::insert` at `:190` does NOT notify, so seeding never
  looks like a save. A test pins one notification per `write_string`). The state machine is target-agnostic and owns no
  executor: `persist::Chains` (one entry per path: base revision, in-flight future, queued
  bytes, stranded/conflicted marks, the write epoch) is driven natively by hand-polling
  its futures with `std::task::Waker::noop()` (rustc 1.94) against a test-only
  `GatedStore` that wraps `memory.rs` and completes a `put` only when the test releases it
  — that is how "in flight with queued" is observable; the wasm layer is the thin part
  that `spawn_local`s and installs the three listeners. `persist::seed(files:
  &[StoredFile])` runs at boot with `load_project`'s result and records each path's
  revision as the tab's base (the reload-then-save case must not conflict). **No
  debounce, one chain per path**: the observer checks the write epoch (a write under an
  older epoch is refused with "project is being replaced — save again after the reload"),
  strips the project root (a key outside the open project's root is logged once per path
  and ignored — never a panic, never a mis-keyed chain), and either starts
  `store.put(file, base_revision, &manifest)` — the manifest is the open project's
  `ProjectEntry` manifest, handed to `Chains` at boot beside `seed` — immediately or, if a put for that path is in flight, queues the bytes behind it
  (replacing any earlier queued bytes for the same path — only the newest content
  matters); when the in-flight put resolves `Ok(new)`, the queued put starts with
  `base_revision = new`. Path states are exactly *idle / in flight / queued / stranded /
  conflicted* as the decision text defines them. Two predicates, not one:
  `persist::is_pending()` is "any path not idle" (feeds `persist_pending`,
  `playground_is_dirty` and the `beforeunload` warning — stranded or conflicted bytes are
  unsaved work); `persist::has_active()` is "any path in flight or queued" and is what
  `drain_then_epoch` awaits — a stranded or conflicted path has nothing running and must
  never block a switch or reset (the page's confirm over `playground_is_dirty` is the
  guard for its bytes). A `vfs::write` to a CONFLICTED path issues no put: `MemFs` keeps
  the newest bytes (the export carries them), the path stays conflicted, the banner
  stands. `base_revision` is the revision the tab last loaded or wrote for that path (0
  for a new file; an absent record accepts only 0). Outcomes: `Ok(new)` records `new` as
  the base and clears pending (a put resolving after its drain timed out records the
  stored revision as base silently); `StaleRevision` (a genuine cross-tab conflict) → the
  path becomes CONFLICTED, never retried: the persistent banner "another tab saved <file>
  after you loaded it; reloading discards THIS tab's version — export first to keep it"
  plus a "download this file" control; `Backend`/`Unavailable` → the path is STRANDED
  holding the newest bytes (a queued put behind the failed one starts at once with the
  same base) and the persistent banner. **The banner is DOM, written by `persist.rs`
  itself** into the element with id `playground-banner` (text content; absent element is a
  silent no-op, like `set_boot_status`) — there is no callback into the editor. **The
  editor's dirty indicator** comes from the shared flag `EditorRunOptions.persist_pending:
  Option<Arc<AtomicBool>>`: `persist.rs` keeps it equal to `is_pending()`, and
  `sync_dirty_mirror` (`mod.rs:396-398`) ORs it into `set_dirty`, so a stranded or queued
  put shows in the title exactly like an unsaved command. `visibilitychange`→hidden
  re-issues STRANDED paths only (newest bytes); `beforeunload` sets `returnValue` while
  anything is pending. There is NO `pagehide` handler (round-1 code review): a queued put
  cannot be issued before the in-flight one resolves without re-using its base and
  breaking the CAS, and IndexedDB commits the in-flight transaction on its own, so "at
  once" bought nothing and manufactured a false conflict. Import,
  reset and project switch share `drain_then_epoch()` — bump the epoch, THEN await every
  chain, in-flight and queued, bounded at 5 s (the wasm layer's timeout is a
  `setTimeout` promise raced against the drain; the machine itself just reports whether
  anything is still pending).
  **Preferences (corrected mechanism):** `EditorGame` gains `prefs_slot: PathBuf` (from
  `EditorRunOptions.prefs_slot`, default `EDITOR_PREFS_PATH`); **both** `load_preferences`
  (today hardcoded to `EDITOR_PREFS_PATH` at `mod.rs:244`) and the save read that field,
  so the web restores what it wrote. Saves go through `save_store` on a **settle rule**
  the editor runs itself: `save_preferences` becomes `pub(super) fn
  save_preferences_if_changed(&mut self, delta_time: f32)`, called every frame from
  `finish_frame` beside `sync_dirty_mirror` — it captures an `EditorPreferences` (the
  struct derives `PartialEq`; nothing is serialised until a write), compares it with the
  last written one, accumulates `delta_time` while unchanged, and writes once the
  preferences have been stable for 0.5 s (time, not frames — a 20 fps device settles in
  the same half second; a pan writes once, after the hand lifts, not sixty times a
  second). **Skipped while `in_play_session()`** (Playing or Paused): the viewport then
  holds the game's camera (`adopt_game_camera`, `play_session.rs:64-71`), and a settle
  would overwrite the authored camera. The Play transition's immediate write happens at
  the TOP of `handle_play_action` (`play_session.rs:176`) for `Play` from Editing — before
  `start_play_session` swaps the camera — and after `stop_play_session` for `Stop`;
  `on_exit` keeps its call. Natively the file is the same JSON at the same relative path
  as today. This replaces the decision's "on hidden / before switch" triggers: a DOM
  listener cannot reach the editor's state synchronously, and the settle rule covers both
  within half a second of the last change. Headless tests pin it: change the camera,
  tick 29 times at 1/60 s — the slot is unchanged; tick once more — it holds the new
  JSON; a Play transition writes at once with the EDITING camera; a stable half second
  during Play writes nothing; `load_preferences` after a settle write restores the
  camera from the slot (the round trip through `prefs_slot`). `preferences.rs` holds
  the plumbing (`load_preferences`, `load_preferences_from`, the settle logic — today
  `mod.rs:242-298`), which keeps `mod.rs` (588 lines) under the ceiling; if
  `EditorRunOptions` and `run_game_with_editor{,_opts}` (`mod.rs:541-567`) still push it
  over, they move to `editor_game/run_options.rs`.
  **Default scene path (corrected mechanism):** `EditorGame` gains `asset_base: PathBuf`,
  captured in `Game::init` AFTER `self.inner.init(ctx)` (`mod.rs:416` — `ProjectHost::init`
  sets the base there, `project_host.rs:107-111`) from `ctx.assets.base_path()`; the three
  sites that fall back to the bare relative `DEFAULT_SCENE_PATH` — `scene_io.rs:61`,
  `scene_io.rs:249-254` (`default_scene_path`) and `api.rs:172` — join `asset_base` with
  it instead, so an untitled scene saved on the web lands under the project root. The
  same rule covers an EXPLICIT relative path: `save <path>` on the API (`api.rs:166-168`
  takes it verbatim today) and a relative Save As target are joined to `asset_base`
  (an absolute path, or a key already under the base, passes through); on the web a
  relative key would never persist and never reload. `docs/EDITOR_COMMAND_API.md`'s `save
  [path]` row says "relative to the project's assets". `run_headless_editor_api`
  (`headless.rs:122-135`, never calls `init`) sets `asset_base` from its own argument,
  so a headless `save` with no path lands under the project too. Tests pin that after
  `init` with a base the default path starts with the base, and that `save
  scenes/x.scene.ron` writes under the base.
  Tests (native, directory store unless said): put/load round trip; a put with a stale
  base is refused and the stored bytes are untouched; two writers racing from the same
  base — exactly one wins; two puts of one path from one writer — the second chains and
  lands with revision base + 2, no `StaleRevision` (gated store); `seed` then save — a
  file loaded at revision 3 saves as 4 with no conflict; a first put to a slug whose
  manifest exists only in `projects.json` upserts the manifest and a following
  `sweep_orphans` KEEPS it; `drain_then_epoch` — a queued put before the switch lands, a
  write after the bump is refused, a first-time put born during the drain is refused (not
  resurrected after the remove); fail v1 with v2 queued → v2 starts at once, and a
  re-issue puts v2; a conflicted path is never re-issued; a re-issue leaves an
  in-flight-plus-queued path alone; a put resolving after a drain timeout updates the
  base without a conflict; `persist_pending` rises on the first non-idle path and falls
  when the last one settles; `replace_project` leaves the other project intact; a failed
  `replace_project` leaves the previous project intact (manifest is the marker) — an
  IndexedDB-only property (one transaction) no native double can fail mid-way; the
  directory double stages and swaps instead, and the property is on Jesse's browser
  check in batch 4;
  `sweep_orphans` removes files of a non-bundled manifest-less slug and nothing else;
  manifests merge bundled + stored; the memory store passes the same suite.
- `projects.rs`: `ProjectManifest { slug, title, bundle_version, content_hash, origin }`
  (`origin: bundled | saved | imported`); the project ROOT is computed, never stored:
  `{ASSET_BASE}/projects/<slug>` on the web (the base-joined canonical VFS key),
  `<dir>/projects/<slug>` natively; the project's asset base is `{root}/assets` on both.
  `list_projects(bundled: &[ProjectManifest], stored: &[ProjectManifest]) ->
  Vec<ProjectEntry>` is pure and tested natively: bundled `{ASSET_BASE}/projects.json`
  merged with `store.manifests()` (stored wins on a slug clash, so an imported project can
  shadow a bundled one); each entry reports `is_bundled`, `has_stored_files` and `differs_from_bundle`
  (the stored `content_hash` or `bundle_version` differs from the bundled one). **Reset
  is gated on `has_stored_files`** (stored files shadowing a bundled slug ARE the
  divergence — the upserted manifest copies the bundled hash, so the hash alone would
  never show a plain edit); `differs_from_bundle` with `origin` only chooses the note
  beside the control. `validate_slug` enforces `^[a-z0-9_-]{1,32}$` (no regex
  crate — a byte loop).
- `web_entry.rs` (wasm-only): `ASSET_BASE = "/playground/v1/assets"` and `BUNDLE_VERSION =
  "v1"` (the version contract, now five places — documented in the crate header),
  `init_web_logging`, `preload_assets(ASSET_BASE)`, open the store (fall back to
  `memory.rs` + banner on failure), `sweep_orphans(bundled slugs)`, `list_projects()`, pick
  the project (query string `?project=<slug>` via `engine_core::web::query_param`; an
  unknown slug redirects to the first project's query so the URL never claims a project
  that is not loaded), await `load_project`, insert every stored file at
  `{root}/{relative path}` onto `MemFs` through the boot-phase `vfs::insert` (stored files
  overwrite bundled ones; a slug with files but no manifest is ignored as an unfinished
  import), `persist::seed(&files)`, install the persist listeners and the write observer,
  then `run_game_with_editor_opts(ProjectHost::new(root), GameConfig::new("Insiculous
  Playground").with_size(1280, 800).with_asset_base_path("{root}/assets"), opts)` with
  `opts.initial_scene = find_first_scene(&root.join("assets").join("scenes"))` (the
  `vfs::list_dir_files` scan works over `MemFs`; this is how `behavior_demo` opens first)
  and `opts.prefs_slot = Some("beinsiculous.playground.editor_prefs".into())`
  (localStorage via `save_store`). The editor's 1024×720 minimum (`constants.rs:15-26`) is
  respected by the page canvas size. `EditorRunOptions` after this batch, exactly:
  `api_rx: Option<mpsc::Receiver<String>>` (existing), `initial_scene: Option<PathBuf>`
  (existing), `api_responses: Option<mpsc::Sender<String>>`, `prefs_slot:
  Option<PathBuf>`, `dirty_flag: Option<Arc<AtomicBool>>` (WRITTEN by the editor from
  `sync_dirty_mirror`, the same value `set_dirty` gets, so the bridge reads the watermark
  without holding the editor), `persist_pending: Option<Arc<AtomicBool>>` (READ by the
  editor there, written by `persist.rs`). `run_game_with_editor_opts` copies each onto
  `EditorGame`; the standalone binary passes `..Default::default()` for the new fields
  (`EditorRunOptions` derives `Default`). A `MemFs` test in `common` pins the whole key
  story: insert a bundled file at the key the build script's copy produces
  (`{ASSET_BASE}/projects/examples/assets/scenes/behavior_demo.scene.ron`), `vfs::write`
  an edit at the same key, read it back through a `Path::new("{root}/assets").join(
  "scenes/behavior_demo.scene.ron")` lookup, and confirm a RELATIVE key never resolves.
- `bridge.rs` (wasm-only `#[wasm_bindgen]` exports; each is a thin call into
  target-agnostic functions unit-tested natively — path canonicalisation, the FIFO
  refusal rule and the dirty OR live in plain functions): the request channel is
  `mpsc::sync_channel::<String>(1024)` — its `Receiver` is what `api_rx` takes and its
  `SyncSender` lives in the bridge's `thread_local`, so the cap is `try_send`'s refusal —
  `playground_dispatch(line) -> bool` returns `false` on a full queue OR a whitespace-only
  line (never drops, never enqueues a line the API answers with nothing);
  `playground_poll_responses() -> Vec<JsValue>` drains the response channel — `ApiSession`
  (`api.rs:24-32`, `pub(super)`) gains `responses: Option<mpsc::Sender<String>>` and
  `drain_api_requests` (`api.rs:212-235`) sends there when set, stdout otherwise;
  `playground_is_dirty() -> bool` = `dirty_flag` OR `persist::is_pending()` (the page
  confirms before any switch, import or reset); `playground_write_file(path, text) ->
  Result<(), JsValue>` / `playground_read_file(path)` / `playground_list_files()` take
  and return PROJECT-RELATIVE paths (`assets/scripts/ball.rhai`): the bridge joins the
  open project's root, and REFUSES `..`, a leading `/`, an empty path and anything
  that does not canonicalise under the root, then goes through `vfs`; after a `.rhai` write the bridge calls
  `bridge::Hooks.source_check: Option<fn(&str) -> Result<(), String>>` if set — the bridge
  owns a `thread_local` `Hooks { source_check, script_errors: Option<Rc<dyn Fn() ->
  Vec<String>>> }` that `web_entry` fills, both `None` in this batch (nothing in the tree
  compiles Rhai yet; batch 7 supplies `scripting::check_source` and the error list) and
  the result reaches the page through the write's return value; `playground_script_errors`
  ships in batch 7; `playground_list_projects() -> JsValue` (the `ProjectEntry` list as
  JSON); `playground_open_project(slug) -> Promise` and `playground_reset_project(slug) ->
  Promise` call `drain_then_epoch()` (in-flight and queued, bounded 5 s; a rejection
  keeps the current project and shows the message naming tab visibility), then (reset)
  `remove_project`, and resolve — the PAGE then sets the query and reloads (the engine
  cannot swap a running project — documented).
- `engine_core::web`: `pub fn query_param(name: &str) -> Option<String>` beside
  `set_boot_status` (`web/mod.rs:88`), over `window().location().search()` and
  `UrlSearchParams` (features added to `engine_core/Cargo.toml:45-57`).
- Gate hole closed: `scripts/check_wasm.sh` already covers the new member. Add one line
  to its header naming the playground crate as the reason the editor crates are in the
  gate.
- Docs: new `docs/WEB_PLAYGROUND.md` — the bundle contract (version places, the store's
  database and object stores, the CAS rule and why get-then-put is forbidden, the
  base-joined root rule, the bundle-version/reset rule, the bridge function list with
  argument and return shapes, the FIFO ordering contract and the refusal on a full
  queue, the "one embed per page" constraint, the element ids `game-loading` and
  `playground-banner`, the settle rule for preferences). `docs/EDITOR_COMMAND_API.md` §
  Stages (`:159-165`): Stage D is the function bridge, shipped here — replace the
  WebSocket line. `docs/WEB_SAVES.md` gains a paragraph (after § Keys) naming the
  playground's prefs key and pointing at the store doc. `crates/editor_integration/CLAUDE.md`
  file map gains `preferences.rs` and the `EditorRunOptions` field list;
  `crates/common/CLAUDE.md` names the write observer.

Gates: standard + wasm (the crate must build for wasm32 and natively) + games
(`common`'s public surface grows by the observer hook, `engine_core::web` is wasm-only
and needs none). Leaves out: the build script, the page (batch 4), zip (batch 5).

## Batch 4 — bundle build and the `/playground/` page — LANDED 2026-09-04 (e362625 engine, f69f09e site); Jesse's browser check pending

**Re-verified against the tree 2026-09-04 before the handoff.** Corrections, each stated
where it applies below: `build_wasm.sh` is 177 lines (batch 0's device gate added four) and
its `--sync` hard-code moved to `:167`; the site layout is `src/layouts/BaseLayout.astro`
(the embeds live in `src/components/`); the README section is titled "WASM builds"; a
`projects.json` entry needs the `origin` field, which `ProjectManifest` does not default;
the local test page and the `rm -rf` of the output dir are games-shaped too and are named
now; the examples project opens `behavior_demo.scene.ron` (sorted first), which is where
the patrollers are; `shellcheck` is not installed on Iroh; the site's screenshot map is
opt-in per route; the synced bundle is tracked in the site repo; two doc rows batch 3 left
stale are folded in; the conflicted-path "download this file" control moves to batch 5.
The corrected section was reviewed in code mode by kimi (`review-13.md`, 3 findings) and
gemini (`review-13-gemini.md`, 4), adjudicated in `rebuttal-13.md`: the entry gains a
`playground-ready` event (the page's `await init()` resolves before the spawned boot has
loaded the manifests or installed the bridge, so a select populated right after it is
empty forever); both cargo queries in the script name the manifest explicitly; the engine
`.gitignore` gains the build output; a project switch ends unconditionally in navigation
and the page-side `canvas.focus()` clause is gone (the engine focuses its canvas at boot,
`crates/renderer/src/window.rs:114`); Reset confirms; the page keeps the loaded slug
itself; batch 5's download control reads bytes, not text.

Repos: `insiculous_2d` (`scripts/build_wasm.sh`, 177 lines; `crates/playground/src/web_entry.rs`
— one hunk, the readiness event; `.gitignore`; `docs/WEB_PLAYGROUND.md`;
`crates/playground/CLAUDE.md`) and `insiculous_web`.

Target shapes:

- `build_wasm.sh` gains `--kind games|playground` (default `games`; output
  `games/<slug>/<version>/` for games and `playground/<version>/` for the playground —
  no slug segment, so the deployed dir matches `ASSET_BASE = "/playground/v1/assets"`;
  the slug argument names the bundle in messages only) and, for `--kind playground`, a
  repeatable `--project <slug>=<title>=<dir>` (`<dir>` resolves against the caller's
  cwd, the engine root in the invocation of record): each `<dir>/assets` is copied to
  `assets/projects/<slug>/assets/…` — the `assets/` segment is KEPT, mirroring the source
  tree and the export-zip layout, so the entry's `{root}/assets` base finds every file —
  and `assets/projects.json` lists the manifests. **Its entry shape is
  `crates/playground/src/projects.rs:19-30`, deserialized with no defaults**:
  `{ "slug", "title", "bundle_version", "content_hash", "origin": "bundled" }` — `origin`
  is required and snake_case (the entry falls back to a one-project default list when the
  file fails to parse, so a missing field is a silent wrong project list, not an error).
  `content_hash` is sha256 (hex) over the sorted relative file list and bytes of
  `<dir>/assets`. The crate's own `assets/` copy is skipped for this kind (`crates/playground`
  has no `assets/` today; batch 8 adds `assets/projects/pong/` there and passes it as a
  `--project`). **Order**: project copies and `projects.json` first, `assets/manifest.json`
  generated LAST from the finished tree, then `--sync` — a manifest written before the
  copies would leave boot fetching nothing. The boot (`crates/engine_core/src/web/mod.rs:112`,
  `preload_assets`) fetches `{ASSET_BASE}/manifest.json` and inserts every entry under
  `{ASSET_BASE}/{entry}`, so the manifest must list `projects.json` and every
  `projects/examples/assets/**` file — today's `find … ! -name manifest.json` (`:100`)
  already does, once it runs last. **Workspace member, not a standalone crate**: the
  wasm-bindgen pin probe (`:63`) reads the lockfile beside the manifest `cargo
  locate-project --manifest-path "$GAME_DIR/Cargo.toml" --workspace --message-format plain`
  names (the root `Cargo.lock:3302-3303` resolves `0.2.126`; the installed CLI is 0.2.126);
  the built `.wasm` is found under `cargo metadata --manifest-path "$GAME_DIR/Cargo.toml"
  --no-deps --format-version 1`'s `target_directory` (today `:83` assumes
  `$GAME_DIR/target`, which a member lacks; here it resolves to the engine root's `target/`
  — no `CARGO_TARGET_DIR`, no `.cargo/config.toml`). **Both queries name the manifest**: the
  script never `cd`s before them, so a bare query run from the engine root would answer
  for the engine while building a game in `$GAME_DIR` — the games-kind regression gate
  below runs from the engine root for exactly this reason. The output dir and its `rm -rf`
  (`:84,:88`) follow the kind; and `--sync` copies to `<site>/public/<kind's output path>`
  (today `:167` hard-codes `games/$SLUG/$VERSION`). The `ASSET_BASE` assertion (`:49-58`)
  reads the kind — for the playground the expected base is `/playground/$VERSION/assets`
  against `crates/playground/src/web_entry.rs:29`, and it also asserts `BUNDLE_VERSION`
  (`:31`) equals `$VERSION`; the games branch stays as it is, its `grep -o '"/games/…'`
  remediation included. The `[profile.wasm-release]` check (`:76`) inspects the manifest
  `cargo locate-project --workspace --message-format plain` names (a game's root is its
  own manifest; the playground's is the engine root, `Cargo.toml:24`). **The local test
  page** (`:104-154`, mirrors the site's embed contract, NOT deployed) follows the kind
  too: for the playground it lands at `dist/playground/index.html`, imports
  `/playground/<version>/game.js` (`:143` hard-codes the games URL), sizes the canvas
  1280×800 — the entry's `with_size(1280, 800)`; the `src/constants.rs` probe (`:105-116`)
  finds no such file in the crate and would fall back to 800×600 — and carries a
  `<p id="playground-banner" role="alert"></p>` beside `#game-loading`, so the local check
  shows persistence banners instead of dropping them (an absent element is a silent
  no-op); the `--serve` message names the kind's path. The header usage comment (`:1-22`)
  documents `--kind` and `--project`. Invocation of record, from the engine root:
  `scripts/build_wasm.sh crates/playground playground --kind playground --version v1
  --project examples=Examples=examples --sync ../insiculous_web/public`. `examples/assets`
  is 1.7 MiB (fonts 1000K); the examples project's first scene by `find_first_scene`'s
  sorted order is `behavior_demo.scene.ron` (six Patrol behaviors), not `hello_world`.
  The build output lands in `crates/playground/dist/` (the script's `$GAME_DIR/dist`), and
  the engine `.gitignore` ignores `target/` but no `dist/` — add `crates/playground/dist/`
  to it in this batch, or the next `git add -A` commits a multi-MiB binary.
- **The readiness event** (`crates/playground/src/web_entry.rs`, the one Rust hunk):
  `start()` (`:57-66`) spawns `run_playground` and returns, so the page's `await init()`
  resolves while the boot is still fetching assets; `BUNDLED_MANIFESTS` and
  `STORED_MANIFESTS` are empty and `REQUEST_SENDER` is `None` (`bridge.rs:28`) until steps
  2–8 run. Immediately after `setup_bridge` (step 8, `:216-219`) the entry dispatches
  `web_sys::Event::new("playground-ready")` on `window` (`dispatch_event`; the `Event`
  feature is already on, and `persist/mod.rs:463` already calls an `EventTarget` method on
  `window` under the current feature list). A failed boot dispatches nothing: its message
  is already on `#game-loading`. The header's boot order and `docs/WEB_PLAYGROUND.md`'s
  "one embed per page" paragraph name the event. Gates for this hunk: `cargo test
  --workspace`, `cargo clippy --workspace --all-targets`, `scripts/check_wasm.sh`.
- `insiculous_web`: `src/pages/playground.astro` on `BaseLayout` with a nav entry
  (`src/layouts/BaseLayout.astro:26-33`, the `nav` array); `src/components/PlaygroundEmbed.astro`
  copying `GameEmbed.astro`'s loader shape verbatim (`:69-109`: the device-asking WebGPU
  gate, the `new Function` import trick, `#game-loading` status with `role="status"`,
  `#game-canvas` placeholder at 1280×800 with `tabindex`, `role`, `aria-label`, fallback
  text, and the `:global(canvas)` rule — the engine swaps its own canvas in). **Every
  control is static HTML**, present and labelled before the module loads: the a11y and
  screenshot gates run in headless Chromium, where the WebGPU gate stops at "needs
  WebGPU", and axe must still see a complete, labelled page. Page controls, all
  keyboard-reachable and labelled, and **`disabled` until the page's `playground-ready`
  listener fires** (registered on `window` BEFORE `init()` is awaited; the listener
  populates the select, enables the controls and reads the entries) — a control the boot
  never enables sits disabled beside the failure message on `#game-loading`: a project
  `<select>` (populated from `playground_list_projects` — it returns a parsed
  `ProjectEntry[]` (`bridge.rs:178-190`), not a JSON string; the page keeps
  `currentSlug` = the `?project=` parameter, else the first entry's slug (`list_projects`
  puts bundled entries first, matching the boot's default; an unknown parameter never
  reaches the page — the boot redirects), and the select starts on it; a change asks
  `playground_is_dirty` OR any page-side dirty flag (batch 8's textarea keeps `value !==
  lastSaved`), confirms with a native `confirm()` — a cancel restores the select to
  `currentSlug` — AWAITS `playground_open_project`'s promise, then **ends unconditionally
  in navigation**: `location.search = "?project=" + slug` — nothing runs after it; the
  promise rejects with the drain's status message on a 5 s timeout, shown in the banner
  region, the select restored to `currentSlug`), a "Reset to bundled" button shown for any
  bundled slug with `has_stored_files` — **it confirms first, always** (`confirm()`: it
  deletes this browser's saved copy of the project) — with the note "you imported over the
  bundled project" when the entry's `manifest.origin` is `imported`, else "the bundled
  project changed since you saved", shown only when `differs_from_bundle` (a first save
  upserts the manifest with `origin: saved` and the bundled hash — `store/indexed_db/mod.rs:327-333`
  — so the note appears after a bundle change, not after every save; awaits
  `playground_reset_project`, then navigates to `?project=<slug>`), a command console (`<input>` + `<output
  aria-live="polite">` over `playground_dispatch`/`playground_poll_responses`, paired in
  FIFO order, a `false` return shown inline as "busy — try again"; `list` is the smoke
  command, `docs/EDITOR_COMMAND_API.md:30`), the persistence banner region
  (`role="alert"`, the `#playground-banner` element `persist/mod.rs:347` writes), a
  `beforeunload` handler that warns while a put is pending (the engine already warns
  through `persist`'s own listener; the page's covers batch 8's unsaved textarea), no
  page-side focus management — the engine focuses its canvas at boot
  (`crates/renderer/src/window.rs:114`) and the page never moves focus away from a text
  control — and a keyboard-shortcut list next to the embed (the README's playable-game
  accessibility requirements apply). Copy uses curly apostrophes; exactly one `<h1>`; no
  duplicate ids.
- `scripts/screenshot-pages.mjs:55-70` (`shotNames`): add `"/playground/":
  "studio-playground"` — every route is measured for sideways scroll automatically, but a
  PNG is opt-in per route. `scripts/postbuild-check.mjs:48-59`: the `index.html` guard
  walks `public/games/` only; walk `public/playground/` with it — the same silent
  route-overwrite applies to `/playground/` (the build script's test page lands outside
  the synced dir, so this is a guard, not a fix).
- The synced bundle is **tracked in the site repo**, like the six game bundles (85 files
  under `public/games/`): stage `public/playground/v1/**` (`game.js`, `game_bg.wasm`,
  `assets/manifest.json`, `assets/projects.json`, `assets/projects/examples/assets/**`).
- `README.md` (site) § "WASM builds" (`:166-199`) gains "the editor bundle" subsection
  (route, `public/playground/<version>/`, the project layout, the one-embed-per-page
  rule); its item 4 (`:177-179`, "GameEmbed currently renders a placeholder") is stale
  since the embeds shipped and is corrected in the same edit; `docs/roadmap.md:128-131`
  updated to "shipped, route `/playground/`"; `src/pages/engine.astro:34-39` links the
  playground.
- `docs/WEB_PLAYGROUND.md`: the intro's "Batch 4 … adds the build script and the page;
  this file describes what the crate exports today" is rewritten, and a new § "The bundle"
  states the layout (`playground/<version>/{game.js, game_bg.wasm, assets/{manifest.json,
  projects.json, projects/<slug>/assets/**}}`), the `projects.json` entry shape, the hash
  definition and the invocation of record. `crates/playground/CLAUDE.md` file map: the row
  `store/indexed_db.rs` names a file batch 3 split into `store/indexed_db/{mod,cursors}.rs`
  — correct the row (a batch-3 leftover, folded in here because this batch touches the
  guide's bundle contract).
- Deploy is Jesse's push of `jesse → dev` (staging) then `main`.

Gates: `npm run verify`; bundle gate (the invocation of record must not warn past 20 MiB;
report the `wasm size:` line); games-kind regression — `scripts/build_wasm.sh ../games/pong
pong` (no `--sync`) still completes and produces `../games/pong/dist/games/pong/v1/`
unchanged in layout; `shellcheck scripts/build_wasm.sh` is not installed on Iroh — say so
in the report instead of running it. The readiness hunk touches `crates/playground`, so
`cargo test --workspace`, `cargo clippy --workspace --all-targets` and `scripts/check_wasm.sh`
run; no public item of an engine crate changes, so the games gate does not apply.
**Jesse's browser check:** open `/playground/` on staging, select an entity, move it
with the gizmo, press Play (the patrollers walk), Stop, Ctrl+S, reload — the move
persisted; the console answers `list`; switch project with unsaved edits — the page
asks first; batch 3's persistence check rides on this page too: save twice, reload,
switch, reset. Leaves out: export/import (batch 5) and, with them, the "download this
file" control the conflicted-path banner names (the page needs export's Blob-download
shape and `playground_read_file`; batch 5 carries it); scripts.

## Batch 5 — project export and import (#49 items 1–2) — LANDED 2026-09-05 (ceb77be engine, 227a5f2 site); Jesse's browser check pending

**Re-verified against the tree 2026-09-05 before the handoff.** Corrections, each stated
where it applies below: `persist.rs` is `persist/mod.rs` (484 lines) and gains one method,
nothing more — `restore_epoch` (`:338-343`) and the draining refusal (`:152-154`) already
exist; `zip` 4.6.1's `deflate-flate2` feature declares `flate2` with `default-features =
false`, and `flate2` with no backend feature is a `compile_error!` ("You need to choose a
zlib backend", `flate2-1.1.5/src/lib.rs:97-98`) — today the crate would compile only by
feature unification with `png`'s `flate2`, so the crate names the backend itself; the
import's dry run reuses `editor_integration::HeadlessAssets`, not the editor's GPU asset
manager; `docs/SCRIPTING.md` does not exist until batch 7, so the export README links
`docs/WEB_PLAYGROUND.md` alone and batch 7 adds the second link; the Rust hunks change
`game.js`'s exports, so the shipped bundle must be rebuilt and re-synced or the page calls
functions the deployed glue does not have; `PlaygroundEmbed.astro` is 544 lines and would
exceed the 600 ceiling with the new controls, so its script moves to its own file first; the
conflicted-path control needs the list of conflicted paths, which nothing exports today;
`project.ron` needs `ron` in the crate (a workspace dependency already); export needs the
open project's manifest, which only `Chains` holds, privately; the section's tests were
called "directory store" tests but touch no store — they are pure. The corrected section was
reviewed in code mode by kimi (`review-18.md`, 5 findings) and gemini (`review-18-gemini.md`,
7), adjudicated in `rebuttal-18.md`, all accepted: an import stamps an empty `content_hash`
(a round-tripped bundled hash left `differs_from_bundle` false, so the "imported over"
note could never show); export archives `assets/**` only and import returns `assets/**` only
(a project tree holding `project.ron` or `README.md` would have produced duplicate entries
that the importer then refuses); the bridge takes the store BEFORE the drain (a `None` store
after it left `draining` set for the rest of the session); `reissue_stranded` is gated on
`draining` (a tab-hide during the import window could re-put stale bytes over the import);
leading `./` is stripped (`zip -r x.zip .`); downloads are built on click, not ahead (a
pre-built Blob of a conflicted file went stale as the user kept editing; an `<a>` cannot be
`disabled`); the file input is cleared after a rejection (an unchanged value fires no
`change`); the glue's type annotation gains the four exports; the poll is 100 ms, not 50;
the failure contract is worded exactly.

Repos: `insiculous_2d` and `insiculous_web`.

Files (engine): `crates/playground/Cargo.toml`, `Cargo.lock`, `crates/playground/src/lib.rs`
(the module list gains `archive`), `crates/playground/src/archive.rs` (new; if the
validation and its tests together pass 600 lines, the tests go to `archive/tests.rs` as
`persist/tests/` does), `crates/playground/src/bridge.rs` (282 lines; four exports),
`crates/playground/src/web_entry.rs` (248; one cell, one hunk), `crates/playground/src/persist/mod.rs`
(one method, one guard), `crates/playground/src/persist/tests/chains.rs` (one test),
`docs/WEB_PLAYGROUND.md`, `crates/playground/CLAUDE.md`. Files (site):
`src/components/PlaygroundEmbed.astro`, `src/scripts/playground-embed.ts` (new),
`public/playground/v1/**` (the re-synced bundle — every changed file staged), `README.md`
§ "The editor bundle" (one bullet).

**Dependencies** (`crates/playground/Cargo.toml` `[dependencies]`, every target):
```toml
zip = { version = "4", default-features = false, features = ["deflate-flate2"] }
flate2 = { version = "1", default-features = false, features = ["rust_backend"] }
ron = { workspace = true }
```
`flate2` is `zip`'s backend line, not a second archive library: `rust_backend` is
`miniz_oxide`, already in `Cargo.lock` (`:1623`), and `flate2` 1.1.5 is there too (via
`png`), so the lockfile gains `zip` and nothing C. The ground rule's "no new dependency
beyond … `zip` (batch 5)" reads as `zip` plus this backend line. `[dev-dependencies]` gains
`tempfile = "3"` (three crates already carry it). No new `web-sys` feature: bytes cross the
bridge as `Vec<u8>` ↔ `Uint8Array`, and the Blob is built page-side. Gate: `cargo tree -p
playground --target wasm32-unknown-unknown -i flate2 -e features` lists `rust_backend`.

Target shapes:

- **`archive.rs`** — target-agnostic, no `wasm_bindgen`, touches neither the VFS nor the
  store beyond reading files for export:
  - `pub fn export_project(project_root: &Path, manifest: &ProjectManifest) -> Result<Vec<u8>,
    ArchiveError>` walks `common::vfs::list_files(project_root)` (disk natively, a `MemFs`
    prefix scan on wasm, depth capped at 6 — `crates/common/src/vfs/mod.rs:109`) and reads
    each file with `vfs::read` (bytes, `:42`), so the export carries the newest bytes,
    including a conflicted path's (design: "`MemFs` keeps the bytes for the export"). ONLY
    files whose project-relative path starts with `assets/` are archived — a tree that also
    holds a `project.ron` or `README.md` (a native checkout, the template repo) must not
    produce them twice, since the importer refuses a repeated name. Each file is written
    under its project-relative path (`assets/scenes/x.scene.ron`), plus
    `project.ron` (the manifest passed in, `ron::ser::to_string_pretty`) and a generated
    `README.md` (the title, one sentence on what the archive is, and the URL
    `https://github.com/beinsiculous/insiculous_2d/blob/main/docs/WEB_PLAYGROUND.md` —
    NOT `docs/SCRIPTING.md`, which batch 7 creates and links from here, and NOT the template
    repo, which batch 10 adds). Deflate via `SimpleFileOptions::default().compression_method(
    CompressionMethod::Deflated)`; archive bytes need not be deterministic — the round-trip
    test compares contents, not archives.
  - `pub fn import_project(bytes: &[u8], bundle_version: &str) -> Result<(ProjectManifest,
    Vec<StoredFile>), ArchiveError>` validates and returns; it does not touch the VFS or the
    store. `bundle_version` is a parameter because `web_entry::BUNDLE_VERSION` is wasm-only.
    In order: an archive over 64 MiB is refused before parsing; every entry name is
    normalised `\` → `/` (Windows archivers emit backslashes) and a leading `./` is stripped
    (`zip -r x.zip .` writes `./project.ron`); directory entries are skipped
    (`zip -r` and Finder emit them); traversal is refused — a `..` component, a leading `/`
    or `\`, a drive prefix — by the same component rules `bridge::validate_bridge_path`
    applies (extract them into a pure `fn relative_path_is_safe(&str) -> bool` both call);
    a name that repeats after normalisation is refused; `README.md` at the root is skipped;
    every other entry must be `project.ron` or lie under `assets/`, else it is refused
    naming the entry; cumulative decompressed bytes are capped at 64 MiB AS ENTRIES ARE READ
    (`Read::take` at the remaining budget; a read that hits the budget refuses) — a 1 MiB zip
    of zeros must not OOM the tab; `project.ron` is required, parsed as `ProjectManifest`,
    and its `slug` must pass `projects::validate_slug`; every `*.sheet.ron` runs through
    `engine_core::sheet_file::parse_sheet_file(entry_name, text)`; every `*.scene.ron` runs
    through `SceneLoader::parse` and then `SceneLoader::instantiate(&data, &mut World::new(),
    &mut editor_integration::HeadlessAssets::new())` — the same parse-then-dry-run guard the
    editor's load uses (`crates/editor_integration/src/editor_game/scene_io.rs:164-169`), with
    the headless resolver (`editor_integration/src/lib.rs:29`; every texture ref resolves, no
    sidecar lookup; `instantiate` registers the engine components itself) — so schema drift,
    an unknown prefab or a typo fails loud naming the entry instead of committing a project
    the editor cannot open. The dry run does NOT prove a referenced image is in the archive;
    the editor's own load reports that at open. `.rhai` entries are stored unchecked (the
    `source_check` hook is `None` until batch 7). The returned files are the `assets/**`
    entries ONLY — `project.ron` is consumed into the manifest and `README.md` is dropped —
    as `StoredFile { project: slug, path, bytes, revision: 1, bundle_version }`; the returned
    manifest keeps `slug` and `title` from `project.ron` and sets `content_hash:
    String::new()` (the design's "empty for user projects" — a round-tripped bundled hash
    would leave `list_projects`'s `differs_from_bundle` (`projects.rs:82`) false and hide the
    "you imported over the bundled project" note), `origin: Imported`, and `bundle_version` to
    the parameter — the stores' `replace_project` writes records exactly as given
    (`store/memory.rs:98-103`, `store/indexed_db/cursors.rs:133-137`) and normalises
    nothing.
  - `pub enum ArchiveError`, a plain enum with `Display` like `StoreError` (`thiserror` is not
    a playground dependency): at least `TooLarge`, `Zip(String)`, `UnsafePath(String)`,
    `DuplicateEntry(String)`, `OutsideProject(String)`, `MissingManifest`,
    `InvalidManifest(String)`, `InvalidSlug(String)`, `InvalidSheet { entry: String, reason:
    String }`, `InvalidScene { entry: String, reason: String }`, `Io(String)`. Every message
    that concerns an entry names it.
  - Tests, native and pure (no store): a `tempfile` directory holding
    `assets/scenes/hello.scene.ron` (the text of `examples/assets/scenes/hello_world.scene.ron`),
    `assets/images/x.png` (arbitrary bytes) and a hand-written valid `.sheet.ron` → export →
    import yields the manifest's `slug`/`title`, an empty `content_hash`, `origin: Imported`,
    exactly the `assets/**` paths (no `project.ron`, no `README.md` in the returned files) and
    byte-identical contents per path; a tree holding its own `project.ron` exports without a
    duplicate entry; `./assets/x` imports as `assets/x`; an entry named `assets/../x` is `UnsafePath`; a bad
    sidecar is `InvalidSheet` naming the entry; a scene naming an unknown component is
    `InvalidScene` naming the entry; a backslash name imports under `/`; a directory entry is
    skipped; a `foo.txt` at the root is `OutsideProject`; no `project.ron` is
    `MissingManifest`; a deflated archive of 70 MiB of zeros (written with `zip` itself in the
    test — it compresses to tens of KiB) is `TooLarge` from the cumulative cap.
- **The open project's manifest** (`web_entry.rs`): a `static ACTIVE_MANIFEST:
  RefCell<Option<ProjectManifest>>` beside `DIRTY_FLAG` (`:37`), set where `manifest` is
  resolved (`:155`, before `Chains::new` at `:190` takes it) and read by `pub fn
  active_manifest() -> Option<ProjectManifest>`. Export writes that manifest.
- **Conflicted paths and the stranded guard** (`persist/mod.rs`): `pub fn conflicted_paths(&self)
  -> Vec<String>` on `Chains` — the paths whose state is `Conflicted`, sorted; and
  `reissue_stranded` (`:297`) returns at once while `self.draining` — a `visibilitychange`
  during the import's drain-and-replace window must not re-put stale bytes with a stale base
  over the freshly imported file (or, on the failure path, into a project the contract says
  was untouched). Test in `persist/tests/chains.rs`: a stranded path is not re-issued during
  a drain, and is again after `restore_epoch`. The file stays under 600 (484 today).
- **Bridge** (`bridge.rs`; wasm-only `#[wasm_bindgen]` exports like the existing ones):
  - `playground_export_zip() -> Result<Vec<u8>, JsValue>`: root from `CURRENT_PROJECT_ROOT`,
    manifest from `web_entry::active_manifest()` (an error if either is `None`), then
    `archive::export_project`.
  - `playground_import_zip(bytes: Vec<u8>) -> js_sys::Promise` resolving to the slug string:
    `archive::import_project(&bytes, BUNDLE_VERSION)` FIRST — a refused archive rejects
    before anything drains, and the running session is untouched; then `active_store()`,
    rejecting if `None` — BEFORE the drain, so a rejection here leaves `draining` unset; then the SAME
    `persist::drain_then_epoch().await` as switch and reset (nothing queued before the import
    is lost; nothing written after it persists); then `store.replace_project(&slug, files,
    manifest).await` — ONE transaction (`store/indexed_db/mod.rs:349`). On `Ok`: resolve with
    the slug; the page navigates; `draining` stays set, so a late write is refused with
    "project is being replaced — save again after the reload" (`persist/mod.rs:152-154`),
    exactly as after a switch. On `Err` from `replace_project`: **touch nothing** —
    `with_active_chains(|chains| chains.restore_epoch())` (`:338-343`) so the current project
    keeps saving, then reject with the store error's text; no `remove_prefix`, no
    `vfs::insert`, no banner written from Rust (the page shows the rejection in the banner
    region and offers the uploaded archive back).
  - `playground_read_file_bytes(path: String) -> Result<Vec<u8>, JsValue>`:
    `validate_bridge_path`, then `common::vfs::read` — `playground_read_file` (`:146-157`)
    goes through `read_to_string`, which would corrupt a conflicted `.png` or `.wav`.
  - `playground_conflicted_paths() -> Vec<JsValue>`: `with_active_chains(|chains|
    chains.conflicted_paths())`, as strings.
- **The page** (`insiculous_web`). FIRST the file move: `PlaygroundEmbed.astro`'s inline
  `<script>` body (`:90-304`, 215 lines, already TypeScript-typed) moves verbatim to
  `src/scripts/playground-embed.ts`, and the component keeps `<script>import
  '../scripts/playground-embed.ts';</script>` (Astro bundles a module import inside a script
  tag; the site's precedent for a moved block is `src/pages/profile.astro:7` importing a
  stylesheet). The moved script's glue type (the `dynamicImport` cast, `:120-136`) gains the
  four exports: `playground_export_zip: () => Uint8Array`, `playground_import_zip: (bytes:
  Uint8Array) => Promise<string>`, `playground_read_file_bytes: (path: string) => Uint8Array`,
  `playground_conflicted_paths: () => string[]` — `astro check` runs in `npm run verify`.
  Then the controls — static HTML, labelled, `disabled` until the `playground-ready` listener
  enables them, in the toolbar `role="group"` (`:25-38`). **Every download is built on click,
  never ahead**: one helper `downloadBytes(bytes: Uint8Array, filename: string)` makes the
  Blob (`type: 'application/zip'` for archives, `application/octet-stream` otherwise), an
  object URL, a temporary `<a download>` it clicks and removes, and revokes the URL — no
  static `<a>` in the markup (an anchor cannot be `disabled`, and a Blob built when a link
  was rendered goes stale as the user keeps editing).
  - an "Export project" `<button type="button" id="export-button">`: on click
    `playground_export_zip()` → `downloadBytes(bytes, slug + '.zip')`; a thrown export writes
    its reason to the banner region;
  - an "Import project" `<input type="file" id="import-input" accept=".zip,application/zip">`
    with its `<label>`: on `change`, if `playground_is_dirty()` confirm first (a cancel clears
    the input); `await file.arrayBuffer()` → `new Uint8Array(...)` → `await
    playground_import_zip(bytes)`; on resolve set `leavingByChoice = true` (`:145`; the
    page's `beforeunload` must not ask a second time) and `location.search = '?project=' +
    slug` — nothing runs after it; on reject write the reason to the banner region and render
    a "Download <file.name>" `<button>` whose click hands the still-held `File` to the
    download helper, so the archive can be saved back; the select is left as it was —
    nothing changed; in every case (`finally`) `importInput.value = ''`, or re-selecting the
    same fixed file fires no `change`;
  - the conflicted-path control (deferred from batch 4): the 100 ms response poll
    (`pollResponses`, `:148`; `setInterval(pollResponses, 100)` at `:212`) also calls
    `playground_conflicted_paths()` and, when the list differs from the last one rendered,
    renders one "Download <basename>" `<button>` per path under the banner; ITS CLICK reads
    `playground_read_file_bytes(path)` and calls the download helper — the bytes are read at
    click time, so THIS tab's newest version is what gets kept before a reload discards it.
  Copy uses curly apostrophes; exactly one `<h1>`; no duplicate ids. `README.md` § "The
  editor bundle" (`:195-208`) gains one bullet: projects export and import as zip, layout in
  the engine's `docs/WEB_PLAYGROUND.md` § "Export and import".
- **The bundle**: after the engine gates, the invocation of record (`docs/WEB_PLAYGROUND.md:24-28`:
  `scripts/build_wasm.sh crates/playground playground --kind playground --version v1
  --project examples=Examples=examples --sync ../insiculous_web/public`), its `wasm size:`
  line reported (6.86 MiB after batch 4; the gate warns past 20 MiB), and every changed file
  under `public/playground/v1/` staged in the site repo. `git -C ../insiculous_web status
  --porcelain -- public/games` stays empty.
- **Docs**: `docs/WEB_PLAYGROUND.md` § "Export and import" (`:154-156`, the stub) becomes the
  layout —
  ```
  <slug>.zip
  ├── project.ron     # the ProjectManifest
  ├── README.md       # generated: the title and the docs URL
  └── assets/         # scenes, .sheet.ron sidecars, scripts, images, sounds, fonts, locales
  ```
  — the validation list in order, both 64 MiB caps, the failure contract stated exactly ("a
  refused archive touches nothing; a failed `replace_project` restores the epoch and the
  current project keeps saving — a save attempted during its drain window was refused, as on
  switch and reset, and is re-issued by saving again"), and the sentence "this
  is the layout the template repo conforms to" (batch 10 reads it); the bridge table
  (`:129-140`) gains the four exports. `crates/playground/CLAUDE.md`: the file map gains the
  `archive.rs` row; the pitfalls table gains "an import's `replace_project` failure must
  restore the epoch, or the current project stops saving until reload — none; browser check"
  and "a zip's decompressed size is capped as it is read, not after — the zeros test".

Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`scripts/check_wasm.sh` (the diff touches `crates/playground`), the comment-tag grep, the
`cargo tree` line above, every touched file ≤ 600 lines (`archive.rs` is the one near it;
the component is ~330 after its script moves out), the bundle rebuild and sync with its `wasm
size:` line, `npm run verify` in the site, `public/games` untouched. No public item of an
engine crate changes, so the games gate does not apply. **Jesse's browser check:** export;
edit something and save; import the zip — the edit is gone and the export's state is back;
reload — still back; the project list shows the slug with the "you imported over the bundled
project" note; import a zip with a broken scene — refused, the banner names the file, the
session continues and the upload is offered back. Leaves out: the template repo; the `.rhai`
check on import (batch 7's hook).

## Batch 6 — scripting Stage 2: scripts visible in the hierarchy and the asset browser — DONE 2026-09-05 (d4b384a)

**Re-verified against the tree 2026-09-05 before the handoff** (batches 3–5 landed since this
section was written). Corrections, each stated where it applies below: `render` already returns
`HierarchyResponse { clicked: Vec<EntityId>, rename_committed }` — the audit's bare `Vec<EntityId>`
is gone — so the struct stays and its `clicked` element type widens; the hierarchy has no host
file of its own (`render_hierarchy` is `panel_renderer/mod.rs:143-190`, 292 lines, and stays
there); `SetComponentCommand::execute` is a no-op on an entity without the component
(`set_commands.rs:44-48`), so a drop on a script-less entity needs the add in the same undo entry;
`capture_preferences` (`editor_game/preferences.rs:40-55`) rebuilds prefs from live state, so a
field nothing holds is wiped on the next save; `ScrollState` has no setter and the inspector host
cannot see where a block was drawn; `AssetKind` is at `:16-21` and `kind_for_extension` at
`:72-78`; `render_tile` (`panel_renderer/asset_browser.rs:147`, its match at `:158`) matches `(kind,
handle)` exhaustively, so a new kind is a compile-forced arm; `editor_game/mod.rs` is 575 lines
and takes exactly two — the `mod open_source;` line beside its siblings (`:28-37`) and the
per-frame call in `finish_frame` (`:320`); hierarchy row clicks have no `suppresses_click()` guard
today (`hierarchy/mod.rs:425`) because rows were never drop targets, and `dragging_payload()` is
`None` on the release frame (`drag_drop.rs:88-93`), so a drop cannot be gated on it.

Files: `crates/editor/src/hierarchy/{mod.rs, tests.rs}` (`render` at `mod.rs:231`, the harness
`render_frame` at `tests.rs:12-21`), `crates/editor/src/asset_browser.rs`,
`crates/editor/src/drag_drop.rs` (`DragPayload`, `:18-23`), `crates/editor/src/script_editor.rs`,
`crates/editor/src/texture_field.rs` (`InspectorExtras`, `:16-26`),
`crates/editor/src/editable_inspector.rs` (`header`, `:236`), `crates/editor/src/scroll.rs`,
`crates/editor/src/test_support.rs` (`extras`, `:62-64`, an exact `InspectorExtras` literal),
`crates/editor/src/context/mod.rs`, `crates/editor/src/editor_preferences.rs`,
`crates/editor_integration/src/panel_renderer/{mod.rs, asset_browser.rs, inspector.rs}`,
`crates/editor_integration/src/editor_game/{mod.rs, preferences.rs}`, new
`crates/editor_integration/src/editor_game/open_source.rs`, `crates/editor/CLAUDE.md`,
`crates/editor_integration/CLAUDE.md`.

Target shapes (audit §6.4 rows, adjusted for the web and for the tree as it stands):

- **Hierarchy rows.** `render_node` emits one pseudo-row per `ScriptRef` of the entity's
  `Scripts` component, directly under the entity's row at `depth + 1` and before its children,
  labelled by `script_id`, or the `source_path` file stem when the id is empty, or `script` when
  both are; widget id `hierarchy_script_{entity}_{index}`. Pseudo-rows are culled and scrolled
  like rows, stay visible when the entity is collapsed (they are the entity's attributes, not
  its children, and an entity with scripts and no children has no arrow), never enter
  `visible_order` (Shift ranges stay entity ranges) and never carry a selection fill.
  `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum HierarchyClick { Entity(EntityId),
  Script { entity: EntityId, index: usize } }`; `HierarchyResponse.clicked` becomes
  `Vec<HierarchyClick>`. Entity-row and pseudo-row clicks push nothing while
  `drag_drop.suppresses_click()` is true — the release frame of a drop is also the frame
  `ui::interact` reports the click, and the viewport (`viewport_interaction.rs:43`) and asset
  tiles (`asset_browser.rs:229`) already guard the same way. The host
  (`render_hierarchy`) routes `Entity` through the existing Ctrl/Shift modes unchanged and, for
  `Script`, calls `selection.select(entity)` and sets `EditorContext.inspector_scroll_request =
  Some("Scripts")` (new field beside `inspector_scroll_entity`).
- **Scroll into view.** `ScrollState::scroll_to(offset: f32)` (clamped by the next
  `begin_frame`). `InspectorExtras` gains `scroll_target: Option<&'static str>` (in) and
  `scroll_target_y: Option<f32>` (out); `EditableInspector::header` records the header's TOP —
  `current_y` before `component_header` advances it (`editable_inspector.rs:237`), not `y()`
  after — into the out field when `type_name` equals the target and the field is still `None`
  (first header wins; `header_with_remove` goes through `header`, so every block is covered). The inspector host, after `edit_all_components`, converts a
  recorded `y` to an offset (`y + offset_this_frame - (bounds.y + padding)`), calls `scroll_to`
  and clears the request — also clearing it when the block was not found or the inspector is
  read-only (Playing). The scroll lands the frame after the click, the lag `scroll.rs` already
  documents.
- **Asset browser.** `AssetKind::Script` for `rhai` and `rs` (ordered after `Scene`);
  `render_tile`'s new `(AssetKind::Script, _)` arm draws a text glyph (the extension, upper-case,
  `theme.accent_cyan`, `theme.fonts.heading`, centered); `tile_interaction` arms
  `DragPayload::Script { path }` for `.rhai` tiles only (`.rs` is display-only: no drag, no
  click); `render_drag_ghost` draws a `Script` drag as the file name following the cursor. The
  `Texture`-only `if let`s in `texture_field.rs:50,74` and `viewport_interaction.rs:34-35` are
  unchanged: a `Script` dropped on the viewport is consumed by nobody and lapses after its one
  frame. The scan test named in `crates/editor/CLAUDE.md`'s pitfall table gains a `.rhai` and an
  `.rs` fixture and keeps its name.
- **Drop on an entity row.** `render` gains `drag_drop: &mut DragDropState` (host at
  `panel_renderer/mod.rs:151`, harness at `tests.rs:12-21`); each entity row calls
  `take_drop_in(row_rect)` unconditionally and acts only when the returned payload is `Script`
  — a `Texture` dropped on a row is consumed and discarded (today it lapses unconsumed; the same
  outcome for the user) — highlights as a drop target while `dragging_payload()` is a `Script`
  and the row is hovered, and reports `HierarchyResponse.script_dropped: Option<(EntityId,
  String)>`.
  The host appends `ScriptRef { script_id: <stem>, source_path: <asset-relative path>, params:
  empty }` (batch 7's `// @param` defaults pre-fill it later): with `Scripts` present, one
  `SetScriptsCommand::new(entity, old, new, "script_drop")`; without it, one `MacroCommand` of
  `AddComponentCommand::new(entity, ComponentKind::Scripts)` then the set; both go through
  `history.execute` (`commands/mod.rs:182`), which pushes without `try_merge`, so every drop is
  its own undo entry — the discrete-entry rule `assign_sprite_texture` (`entity_ops.rs:161-186`)
  states. Status bar: `Attached <stem> to <entity display name>`.
- **Open source.** `EditorPreferences.ide_command: Option<String>` with `#[serde(default)]`
  (the legacy-prefs test at `editor_preferences.rs:196` gains the field); `capture_preferences`
  copies it from `last_saved_prefs` — nothing else holds it, and a save that rebuilt prefs
  without it would wipe a hand-edited command. `InspectorExtras` gains `can_open_source: bool`
  (in) and `open_source: Option<String>` (out, a `source_path`); `edit_one_script` renders an
  `Open source` action button under the Source row only when `can_open_source` is true AND
  `source_path` is non-empty (a fresh `+ Add Script` ref has none; an empty path would resolve
  to the asset root and open the IDE on the whole folder). Host:
  `can_open_source = cfg!(not(target_arch = "wasm32"))`; a request goes to
  `EditorContext.pending_open_source: Option<String>`, consumed each frame by
  `EditorGame::open_pending_source` in the new `open_source.rs`: the path resolves like
  `scene_io.rs:255-260` (absolute passes, relative joins `asset_base`); with an `ide_command`
  (an empty or whitespace-only value counts as none), `#[cfg(not(target_arch = "wasm32"))]` splits it on whitespace and spawns
  `std::process::Command::new(program).args(rest).arg(path)` — `Opened <path> with <program>`
  on success, `show_error` naming the path and the error otherwise; without one, `show_message`
  `Source: <path> (set ide_command in the editor prefs to open it)`. On wasm the button is absent
  and the page's textarea is the editor (batch 8).
- **Docs.** `crates/editor/CLAUDE.md`: the `drag_drop.rs`, `hierarchy/`, `asset_browser.rs` and
  `editor_preferences.rs` rows and the asset-scan pitfall row; `crates/editor_integration/CLAUDE.md`:
  the asset browser pattern line (`:43`) and the file map (`open_source.rs`).
- **Tests** (contract-named, `test_` prefix): an entity with two scripts renders two pseudo-rows
  and a click on the second reports `Script { index: 1 }` while `visible_order` holds the entity
  once; a `.rhai` drop on an entity with `Scripts` appends exactly one ref and undo removes it;
  the same drop on an entity without `Scripts` adds the component with one ref and ONE undo
  removes the component; `kind_for_extension` classifies `rhai` and `RS` as `Script`; prefs JSON
  without `ide_command` loads with `None` and a set command round-trips; `scroll_to` past the
  content clamps at the next `begin_frame`.

Gates: standard + wasm (every touched crate root is under the gate); `scripts/check_games.sh`
(check only — no public item of the systems crates changes, but the games' `--features editor`
builds pull `editor_integration`). Leaves out: execution (batch 7), the `// @param` pre-fill
(batch 7), the bundle rebuild (nothing in `game.js` changes; the deployed playground picks these
rows up at batch 8's rebuild), drag-to-reparent and every other absent hierarchy affordance of
audit §4.10.

## Batch 7 — scripting Stage 3: registry, runner, Rhai — DONE 2026-09-05 (f2431ae)

**Re-verified against the tree 2026-09-05 before the handoff** (batches 3–6 landed since this
section was written). Corrections, each restated where it applies below:

- `GameContext` is built by ONE macro, `build_context!` (`game.rs:270-289`), expanded at
  `game.rs:477` and `game/app_handler.rs:211`; no other construction site exists in `crates/` or
  `../games/`, so the new field is one macro line. The runner is a `GameRunner` field built in
  `GameRunner::new` (`game.rs:294`), where `register_scripts` runs — before `init`, which
  `initialize_and_update` (`:477-484`) calls on the first frame.
- `behavior_runner` is `#[cfg(feature = "physics")]` (`lib.rs:25-26`, re-export `:75-76`) and
  `cargo check -p engine_core --no-default-features` passes today. `scripting` takes
  `Option<&mut PhysicsSystem>`, so it carries the same gate: the module and its re-exports,
  `GameContext.scripts`, `Game::register_scripts`, the runner field, its construction and the
  macro line. The no-default-features check joins the gates. `editor_integration` and every game
  build with default features, so nothing outside `engine_core` is gated.
- `ProjectHost::update_frame` (`project_host.rs:50-54`) takes `(world, input, delta_time)`; it
  grows `players: &InputSettings`, `scripts: &mut ScriptRunner` and `asset_base: &str`, and its
  three tests (`:135-256`) build the new arguments (`InputSettings::default_two_player()`,
  `input/src/player.rs:237`; `ScriptRunner::new()`). The collision drain is
  `PhysicsSystem::take_collision_events()` (`physics_system/mod.rs:168`); the physics verbs the
  commands map onto are `reset_body(entity, position)` (`:204`), `set_velocity(entity, linear,
  angular)` (`:179`) and `set_kinematic_target(entity, position, rotation)` (`:194`);
  `CollisionData { event: CollisionEvent { entity_a, entity_b, started, stopped }, contacts }`
  is `physics/src/components.rs:405`.
- The bridge's `HOOKS` cell (`bridge.rs:31`) has two readers (`:149`, `:302`) and no writer:
  `pub fn set_hooks(hooks: Hooks)` is added beside `setup_bridge` (`:35`) and `web_entry.rs`
  calls it after `setup_bridge`. `SourceCheckFn` is `fn(&str) -> Result<(), String>` (`:16`),
  so the installed hook is a non-capturing closure over `check_source` mapping the
  `ScriptError` to its `Display` string — `check_source` keeps the typed return named below;
  `ScriptErrorsFn` is `Rc<dyn Fn() -> Vec<String>>` (`:18`).
- The error channel this section left to design follows the `dirty_flag` precedent
  (`EditorRunOptions.dirty_flag`, `editor_game/mod.rs:532`, copied into `EditorGame` at `:551`,
  written by `sync_dirty_mirror` `:358`): `EditorRunOptions.script_errors:
  Option<Arc<Mutex<Vec<String>>>>` (`None` natively) → `EditorGame.script_errors`, overwritten
  from `ctx.scripts.errors()` every Playing frame and CLEARED in `stop_play_session`
  (`play_session.rs:142`), so the page never reports a stopped run's errors; while Paused it
  keeps the last Playing frame's list, which is correct because nothing runs. The mirror exists
  for the bridge only: the native status bar reads `ctx.scripts.errors()` directly.
  `web_entry.rs` allocates the `Arc`, passes it in the options and installs
  `Hooks.script_errors = Some(Rc::new(move || …))` reading it. A poisoned lock yields its inner
  value (`unwrap_or_else(PoisonError::into_inner)`), never an `unwrap`.
- `editor_game/mod.rs` is 577 lines. It takes the `mod script_status;` line beside its siblings
  (`:28-37`), two fields (`script_errors` above and `play_frames: u32`, beside `:88-89`), the
  `register_scripts` forward in the `Game` impl (`:373`), one `self.track_script_status(ctx)`
  call in `update_inner_game` (`:233`, already inside the `is_playing()` branch, after
  `self.inner.update(ctx)` at `:248`) and the options copy line beside `:551` — under 600, but
  with little slack: if the staged draft lands `mod.rs` over 590 lines, `EditorRunOptions` and
  `run_game_with_editor_opts` (`:521-560`) move to new `editor_game/run_options.rs` and are
  re-exported unchanged, so a review-round fix cannot trip the ceiling. The lie-detector and
  the mirror copy live in new `editor_game/script_status.rs` (`track_script_status`, which
  also shows each NEW runner error once on the status bar through a `shown_script_errors:
  usize` watermark); `play_frames` and the watermark reset to 0 in `start_play_session`
  (`play_session.rs:12`) and NOT in `resume_from_pause` (`:90`).
- The catalog's `.rhai` list comes from the asset scan (`editor.asset_browser.entries`,
  `AssetEntry` at `editor/src/asset_browser.rs:27`; `AssetKind::Script` covers `.rhai` AND
  `.rs`, `:78` — the catalog takes `.rhai` only), which today runs on first asset-browser open
  or Rescan (`panel_renderer/asset_browser.rs:111-113`). That scan-and-apply moves into one
  `ensure_scanned(editor, assets)` in the same file, called from there and from the inspector
  host, so the picker is populated before the asset browser was ever opened. The `// @param`
  header parse lives in `engine_core::scripting`, which the `editor` crate cannot see, and the
  panel hosts are free functions over `&mut EditorContext` that cannot see `EditorGame`
  (`panel_renderer/inspector.rs:126-141`, `asset_browser.rs:49-54`): so `ScriptCatalogEntry
  { id: String, display_name: String, category: String, params: BTreeMap<String, ScriptValue>,
  source_path: Option<String> }` is an `editor` type (`script_editor.rs`), the catalog is HELD
  on `EditorContext.script_catalog: Vec<ScriptCatalogEntry>` (beside `asset_browser`), and it
  is BUILT by a pure `build_script_catalog(entries: &[AssetEntry], registry: &ScriptRegistry,
  asset_base: &str) -> Vec<ScriptCatalogEntry>` in new `panel_renderer/script_catalog.rs`
  (registry descriptors plus one entry per scanned `.rhai`, its params from the header read
  through `vfs` at `{asset_base}/{path}`; a header that fails to parse yields an entry with no
  params and a status-bar warning) — testable headless with a temp directory. One
  `refresh_assets(editor, assets, registry)` in `panel_renderer/asset_browser.rs` does scan,
  `apply_scan` and the catalog build; it is called from the Rescan/first-open site and from the
  inspector host ONLY when `!editor.asset_browser.scanned` (the inspector renders every frame;
  a scan per frame would walk the asset tree per frame). `render_asset_browser` already holds
  `ctx` (`:51`), so `ctx.scripts.registry()` is in hand at both sites. A `.rhai` header edited
  without a Rescan refreshes at the next Rescan (the runtime reads the header fresh at Play).
  `InspectorExtras` gains `script_catalog: &'a [ScriptCatalogEntry]` (in; `&editor.script_catalog`
  beside `&mut editor.drag_drop` is a disjoint field borrow in `build_inspector_extras`) and
  `script_picker_open: bool` (in/out, backed by `EditorContext.script_picker_open` beside
  `inspector_scroll_request`, `context/mod.rs:86`, and written BACK at the readback site
  `inspector.rs:233-236` before `drop(extras)` — an out field nobody copies back resets every
  frame); the literals at `panel_renderer/inspector.rs:135` and `editor/src/test_support.rs:63`
  are compile-forced.
- Resources never reach a scene file unless `scene_data` names them (`PhysicsSettings` is the
  only one, `scene_data.rs:41`), and `WorldSnapshot` (`editor/src/world_snapshot.rs`) captures
  components only; `register_transient` (`component_registry/mod.rs:96`) is for components.
  `Blackboard` therefore needs no registration at all — "transient-equivalent" means: no
  `scene_data` field, no snapshot entry. A World resource can only be cleared through the
  world, so `reset` takes it: `reset(&mut self, world: &mut World, asset_base: &str)` inserts a
  fresh `Blackboard` (`insert_resource` replaces), and a stale one left in the world after Stop
  is harmless because the next Play clears it first.
- `archive.rs` is `crates/playground/src/archive.rs`; its README string is at `:130` and gains
  the `docs/SCRIPTING.md` link beside the `WEB_PLAYGROUND.md` one. Docs that describe the
  pre-batch state and are therefore part of this batch: `docs/WEB_PLAYGROUND.md:144` (the
  `playground_script_errors` row, "empty until batch 7") and `:184` (item 13),
  `crates/playground/CLAUDE.md:19` (`Hooks` "for batch 7"), `crates/ecs/CLAUDE.md:29` ("nothing
  executes it yet") plus a `blackboard.rs` row, `crates/editor_integration/CLAUDE.md`'s file map
  (`script_status.rs`), `crates/editor/CLAUDE.md`'s `script_editor.rs` row (the picker),
  `crates/engine_core/CLAUDE.md`'s file map (a `scripting/` row beside `behavior_runner/`,
  `:43`), `PROJECT_ROADMAP.md:165-177` and `training.md` (a `### Script Pattern` under
  § Patterns Reference).
- `cargo search rhai` returns 1.26.0 today; the pin holds, and `cargo info rhai@1.26.0` lists
  the `wasm-bindgen` feature the hedge names (it enables `getrandom/wasm_js`, which only
  `ahash/runtime-rng` — off without default features — would need); timing goes through the
  non-optional `web-time`, so no wasm clock panic lurks in `StandardPackage`. Rhai's `INT` is
  `i64`: a value converted back to `ScriptValue::I32` goes through `i32::try_from`, and an
  out-of-range one is a named `ScriptError`, never an `as` truncation. `theme.error_red` exists
  (`theme/mod.rs:106`). The `behavior_runner` references hold (`BehaviorCommands` `:40-54`,
  `apply_commands` `:211`, the kinematic/dynamic/no-physics fallback `:222-249`). Names the
  shapes below rely on: `Name(pub String)` (`ecs/src/sprite_components.rs:16`), `UiLabel.text`
  (`ecs/src/ui_components.rs:123`), `InputSettings` (`input/src/player.rs`: `move_x` /
  `move_y(player, input)`, `is_active` and `just_activated(player, action, input)`,
  `player_count()`), `PlayerId(pub u8)` (`:38`), `GameAction` (`input_mapping.rs:61`),
  `ScriptValueData::Entity(String)` (`script_data.rs:30`, the wire form the Rhai param map's
  Entity→name mirrors), `vfs::read_to_string` (`common/src/vfs/mod.rs:54`),
  `AssetManager::base_path()` (`assets.rs:403`), `World::validate_entity` (`world.rs:377`, the
  liveness check the prune uses). The `EditorGame` constructor is private
  (`fn new`, `mod.rs:93`), so the wrapper test lives in `editor_game/tests.rs`.
- Size budgets: `game.rs` 505, `bridge.rs` 374, `script_editor.rs` 305, `project_host.rs` 257,
  `web_entry.rs` 255, `contexts.rs` 222 all take their lines under 600; `editor_game/mod.rs` is
  budgeted above. `scripting/` is seven files by design so that none nears the ceiling.

Files: new `crates/engine_core/src/scripting/{mod.rs, registry.rs, runner.rs, view.rs,
commands.rs, rhai_backend.rs, builtin/rotate.rs}` (the module line and its re-exports in
`crates/engine_core/src/lib.rs`, feature-gated like `behavior_runner`), `crates/ecs/src/blackboard.rs` (new
resource, its `mod` and re-export in `crates/ecs/src/lib.rs`), `crates/engine_core/src/contexts.rs` (`GameContext.scripts`),
`crates/engine_core/src/game.rs` (`Game::register_scripts`, defaulted; the `GameRunner` field,
built in `new` at `:294`; the one line in the `build_context!` macro at `:270-289`),
`crates/editor_integration/src/project_host.rs` (the runner calls; `update_frame`'s three new
arguments and its tests), `crates/editor_integration/src/editor_game/{mod.rs, play_session.rs,
tests.rs}` and new `crates/editor_integration/src/editor_game/script_status.rs` (the
`register_scripts` forward, the lie-detector, the error mirror and its Stop clear; if `mod.rs`
lands over 590 lines, new `editor_game/run_options.rs` takes `EditorRunOptions`),
`crates/editor_integration/src/panel_renderer/{mod.rs, inspector.rs, asset_browser.rs}` and new
`crates/editor_integration/src/panel_renderer/script_catalog.rs` (the extras literal and the
`script_picker_open` write-back; `refresh_assets`; `build_script_catalog`),
`crates/editor/src/texture_field.rs` (`InspectorExtras.script_catalog`, `.script_picker_open`),
`crates/editor/src/script_editor.rs` (`ScriptCatalogEntry`; "+ Add Script" picker grouped by
category; unresolved ids in `theme.error_red`), `crates/editor/src/context/mod.rs`
(`script_catalog`, `script_picker_open`),
`crates/editor/src/test_support.rs` (the extras literal), `crates/playground/src/bridge.rs`
(`set_hooks`), `crates/playground/src/web_entry.rs` (the hooks and the error mirror),
`crates/playground/src/archive.rs` (`:130`, the README's second link), and the docs listed in
the Docs bullet. `Cargo.toml` (`engine_core`): `rhai = { version = "1.26", default-features =
false, features = ["std", "f32_float"] }` — if the wasm gate needs it, the target block adds
`features = ["wasm-bindgen"]`.

Target shapes:

- `pub trait ScriptBehavior { fn early_update(&mut self, me: &SelfView, view: &ScriptView,
  params: &BTreeMap<String, ScriptValue>, out: &mut ScriptCommands) {} fn update(&mut
  self, me: &SelfView, view: &ScriptView, params: &BTreeMap<String, ScriptValue>, out:
  &mut ScriptCommands) {} }` (both defaulted, no `&mut World`; a Rust script reads the frame's
  `dt` as `view.delta_time` — the view carries it per the decision — where a Rhai hook receives
  it as its fifth argument for convenience). `SelfView { entity,
  name: Option<String>, transform, velocity }` is per instance; `ScriptView` is per phase
  and shared. The verbs are the decision's list plus `set_rotation(target, radians)`, which
  `engine::rotate` needs: a direct `Transform2D.rotation` write applied with the positions; a
  physics body's rotation belongs to physics and is not this verb's job. Every `ScriptCommands` verb takes a `Target` first argument built from
  `&SelfView` (`Target::Entity(id)`) or a name (`Target::Named(String)`) — Rhai sees
  overloads `out.set_position(me, p)` / `out.set_position("Ball", p)`, never a `Target`
  value; `apply` resolves names through the world's `Name` components once per phase
  and reports a missing or ambiguous name as a deduplicated error.
- `EditorGame::register_scripts` FORWARDS to the inner game (a new defaulted trait method
  behind a transparent wrapper is otherwise a silent no-op); `ProjectHost::register_scripts`
  registers the built-ins. Test (in `editor_game/tests.rs`, where the private constructor is
  reachable): a game registering a descriptor, wrapped in `EditorGame`, exposes it through
  the runner's registry.
  `pub struct ScriptDescriptor { pub id: &'static str, pub display_name: &'static str,
  pub category: &'static str, pub params: &'static [ParamSpec], pub make: fn() ->
  Box<dyn ScriptBehavior> }`; `pub struct ParamSpec { pub name: &'static str, pub
  default: ScriptValue }`. `ScriptRegistry::register(descriptor)` panics on a duplicate
  id at startup (the component registry's collision rule). `engine::rotate` (param
  `degrees_per_second: F32 = 90.0`) is the one built-in.
- `ScriptView`, `SelfView` and `ScriptCommands` as in the decision above;
  `ScriptCommands::apply(world, physics: Option<&mut PhysicsSystem>, delta_time: f32)`
  mirrors `BehaviorCommands::apply` incl. the kinematic/dynamic/no-physics fallback
  (`behavior_runner/mod.rs:222-249`, which integrates `velocity * delta_time` into
  `Transform2D` when there is no physics), which is what keeps it headless-testable —
  and applies by KIND, not by arrival: resets first (`PhysicsSystem::reset_body`), then
  positions, then velocities (`set_velocity`) and kinematic targets (`set_kinematic_target`;
  a velocity aimed at an entity reset in this call is dropped), then
  sprite/label/blackboard writes, then despawns. A hook whose call returned `Err` has its
  commands for that call discarded before `apply` runs.
- `ScriptRunner { registry, instances: BTreeMap<(EntityId, usize), Instance>, rhai:
  RhaiBackend, frames_run: u32, errors: ScriptErrors }` (`BTreeMap`: instances run in
  entity-then-index order every frame — a `HashMap` would randomise which of two scripts'
  commands lands last); two entry points,
  `early_update(&mut self, world, input, players, delta_time, physics)` and
  `update(&mut self, world, input, players, delta_time, collisions: &[CollisionData],
  physics: Option<&mut PhysicsSystem>)` (`input: &InputHandler`, `players: &InputSettings`
  — per-player axes and just-activated actions for `0..players.player_count()`), each building ONE shared view for its phase
  (pre-step transforms and no collisions for `early_update`; post-step transforms and
  the drained collisions for `update` — a counter test pins "one view per phase, two per
  frame") plus a `SelfView` per instance, calling the matching hook, then applying the
  commands. At the start of EACH entry point (not once per frame: a despawn issued in
  `early_update` is applied before `update` runs) an instance whose entity fails
  `World::validate_entity`, no longer carries `Scripts`, or whose recorded `(script_id,
  source_path)` no longer matches the ref at its index is pruned and, in the last case,
  re-resolved on that call — so a despawning game does not grow the map and a `Scripts` edit
  made while Paused (the standing rule, filed as #103) cannot run one script's state under
  another's ref. Resolution per `ScriptRef`:
  `source_path` ending `.rhai` → Rhai, read through `vfs::read_to_string` at
  `{asset_base}/{source_path}` (relative keys never resolve on the web); else registry by
  `script_id`; unresolved → one `log::warn!` per id per Play and an entry in `errors()`.
  `ScriptErrors` deduplicates every error by (file, line, kind) per Play — a script that
  trips the operation budget every frame is reported once; the status bar and the page's
  `playground_script_errors` are two consumers of one list — `errors(&self) ->
  &[ScriptError]`, `ScriptError: Display` with file, line, kind and message; the status bar
  through `script_status.rs`'s watermark, the page through the mirror below. `reset(&mut self,
  world: &mut World, asset_base: &str)` at Play start records the base, clears instances,
  `frames_run` and `errors` and inserts a fresh `Blackboard` into the world; the host passes
  `ctx.assets.base_path()`.
- `RhaiBackend`: one `rhai::Engine::new()` — which already carries Rhai's
  `StandardPackage`, so `sqrt`, `abs`, `min`, `max`, `clamp` need no registration — with
  the `Rc<SelfView>`, `Rc<ScriptView>` and `ScriptCommandsHandle` types registered
  (`register_type_with_name`; the views travel as `Rc` handles so a by-value clone is a
  pointer, not a snapshot of every named entity per instance per phase; GETTERS ONLY on
  the two views, so `me.transform.x = 5.0` fails at evaluation with "unknown property"
  instead of mutating a clone nobody reads; methods on the handle) and a `vec2` type —
  `vec2(x, y)`, `.x`/`.y` getters, `+`/`-` between vectors, `*`/`/` by a number,
  `length()`, `normalize()` (the zero vector normalises to itself, never NaN), `dot()` —
  because Rhai has no vector; `AST` cache keyed by `source_path` + content hash (a changed
  file recompiles on the next Play). Compiling also parses the `// @param name: type =
  default` header block into the compiled unit's OWNED defaults (`BTreeMap<String,
  ScriptValue>`; `ParamSpec`'s `&'static str` is for built-ins only — a runtime-parsed name
  would have to be leaked to fit it) — the defaults channel for `.rhai`
  scripts: the catalog shows them, the attach paths (batch 6's drop at
  `panel_renderer/mod.rs:301-305`, and the picker) pre-fill `ScriptRef.params` from them, a
  declared param missing on a ref takes the
  default at run time, and a param declared nowhere is a named `ScriptError`. Params travel as an ARGUMENT, not scope variables (Rhai `fn`s cannot
  read the calling scope): a `rhai::Map` built per call from the `ScriptRef` (F32→float,
  I32→int, Bool, Str, Vec2→`vec2` type, Entity→the target's name as `ScriptValueData` persists it, Color→array). `out` is
  a `ScriptCommandsHandle(Rc<RefCell<Vec<ScriptCommand>>>)` because `call_fn` passes by
  value — the runner drains the shared buffer after the call; a plain struct would lose
  every command silently. Contract: the script defines `fn early_update(me, view, params,
  out, dt)` and/or `fn update(me, view, params, out, dt)` (`me` because `this` is a Rhai
  keyword); a missing hook is simply not
  called (checked once at compile via `AST::iter_functions`); compile or runtime errors
  reach `ScriptErrors` with file and line, never a panic; `Engine::set_max_operations`
  guards runaway loops, and an instance that trips it is QUARANTINED for the rest of the
  Play session (reported once; Stop/Play re-enables it) so a buggy loop cannot burn its
  budget sixty times a second. `check_source` is a SYNTAX check (`Engine::compile` plus
  the header parse) — Rhai is dynamic, so an unknown function or a getter typo is a
  run-time error; `docs/SCRIPTING.md` and the page's Save status say so ("syntax OK —
  runtime errors show during Play"). **Numbers mix freely**: Rhai never coerces `INT` to
  `FLOAT`, so the engine registers `+ - * / % < <= > >= == !=` for every (`INT`,`FLOAT`)
  and (`FLOAT`,`INT`) pair, F32 params are always `FLOAT`, and every command method that
  takes a float has an `INT` overload — `dt * 450` and `out.set_velocity_x("Ball", 250)`
  both work. `pub fn check_source(text: &str) -> Result<(), ScriptError>` is
  the pure compile check the playground bridge runs on every `.rhai` save (installed as
  `Hooks.source_check` through a non-capturing closure that maps the error to its
  `Display` string, the `fn(&str) -> Result<(), String>` the bridge declares), so errors show
  in Edit mode, not only in Play.
- `ecs::Blackboard(BTreeMap<String, ScriptValue>)` World resource, never written to scene
  files or snapshots — it needs no registration; `scene_data` names the only persisted resource
  and `WorldSnapshot` captures components only. Only the world can clear it, which is why
  `reset` takes `&mut World` and inserts a fresh one. Reads take a default —
  `view.blackboard_bool("serving", true)`, `_int`, `_float`, `_str` — so an unset key is
  a value, never an error (an empty blackboard at Play start must not deadlock a game).
  UiLabel text set through `ScriptCommands::set_label_text(target, text)`.
- `GameContext.scripts: &mut ScriptRunner` (engine-owned; a `GameRunner` field built once in
  `GameRunner::new`, where `Game::register_scripts(&mut self, registry: &mut ScriptRegistry)`
  is called — before `init`; one line in the `build_context!` macro, which is the only
  construction site). Module, field, trait method and runner field are
  `#[cfg(feature = "physics")]` like `behavior_runner`, and `cargo check -p engine_core
  --no-default-features` stays green.
  `ProjectHost::update_frame(world, input, players, scripts, asset_base, delta_time)` order per
  Playing frame: (first frame only: build
  physics from the scene's settings and `scripts.reset(world, asset_base)`)
  → behaviors → `scripts.early_update(...)` → physics step → `take_collision_events()` once →
  `scripts.update(...)` with that Vec → transform hierarchy; `ProjectHost::update` passes
  `&*ctx.players` (a shared reborrow of the context's `&mut InputSettings` — the host never
  needs the mutable one), `ctx.scripts` and `ctx.assets.base_path()`. The bridge hooks batch 3
  left `None` are populated here through a new `bridge::set_hooks(Hooks)` (the cell had no
  writer): `Hooks.source_check = Some(<the check_source closure above>)` and `Hooks.script_errors`
  reads the error MIRROR — `EditorRunOptions.script_errors: Option<Arc<Mutex<Vec<String>>>>`
  (`None` natively), copied onto `EditorGame.script_errors` like `dirty_flag`, overwritten from
  `ctx.scripts.errors()` every Playing frame by `script_status.rs` and cleared by
  `stop_play_session`; the native status bar never reads it; `web_entry.rs` allocates
  it, passes it in the options and installs the reading closure — and `playground_script_errors` ships. This is pong's own order
  (paddles → physics → drain → rules): a paddle's kinematic target set in `early_update`
  is where the collider is when the ball arrives this frame, and the goal's reaction in
  `update` sees this frame's contacts.
- Lie-detector (`editor_game/script_status.rs`, `EditorGame.play_frames` counted per Playing
  frame, reset by `start_play_session` only): at Play frame 60, whether any entity carries
  `Scripts` while `ctx.scripts.frames_run() == 0`, and shows "scripts attached but the
  game never ran the script runner" on the status bar once per Play.
- Catalog: `InspectorExtras.script_catalog: &[ScriptCatalogEntry]` (`ScriptCatalogEntry` in
  `script_editor.rs`, shape in the preamble; built-ins from the registry + every `.rhai` under
  `assets/scripts/` from the asset scan, built by `panel_renderer/script_catalog.rs`'s pure
  `build_script_catalog` onto `EditorContext.script_catalog` inside `refresh_assets`, which
  runs on Rescan, on first asset-browser open and from the inspector host when nothing has
  scanned yet — so the picker is never empty for want of an opened asset browser, and a
  Rescan while Editing refreshes it); "+ Add Script" becomes a picker: the click toggles
  `script_picker_open` (written back to the context before the extras drop), an open picker
  renders one heading row per category and one action
  button per entry (`script_pick_{id}`), a pick appends a `ScriptRef` pre-filled from the
  entry's params and closes the picker, and a last row `custom id…` appends the empty ref
  today's button appends — so typing a free id remains possible. `edit_one_script` draws the
  id in `theme.error_red` when it matches no catalog id and `source_path` does not end
  `.rhai`.
- Tests (contract-named, headless): a Rhai script that moves its entity toward a named
  target advances the transform over three frames; two instances of one script on two
  entities each move THEIR OWN entity (the `me` contract); a script on entity A that
  resets entity B by name moves B; a missing target name is reported once and the rest
  of the frame's commands still apply; `update`'s view carries the injected collision
  and `early_update`'s does not; a game registering a descriptor, wrapped in
  `EditorGame`, exposes it through the registry; a blackboard read of an unset key
  returns its default; two instances issue `set_velocity` and `reset_body` for one entity
  in either order and the entity ends reset and still; a hook that errors after issuing a
  command leaves the world untouched; a `.rhai` with a `// @param` header attached with
  an empty ref runs with the header defaults and the catalog lists them; a compile error
  reaches the error list and leaves other
  scripts running; a runaway loop is reported once across many frames; `engine::rotate`
  rotates by `degrees_per_second * dt`; an unresolved id is reported once; `reset()`
  clears the blackboard; `early_update`'s kinematic target lands before the step and
  `update` sees an injected collision; kinematic target routes through physics when
  present and to `Transform2D` when absent; the picker's pick appends a pre-filled ref and
  the custom row an empty one (a `click_scripts`-style test beside the existing ones in
  `script_editor.rs`); the lie-detector fires once at frame 60 for a `Scripts` entity under
  a runner that never ran and stays silent when it did; a `Scripts` edit between two frames
  that swaps the refs runs each instance under its own ref; an entity despawned in
  `early_update` gets no `update` call that frame; `build_script_catalog` over a temp directory
  with one headed `.rhai` and one built-in lists both with their params.
- Docs: new `docs/SCRIPTING.md` — the author-facing contract (the `update` signature,
  linked from the export README `crates/playground/src/archive.rs:130` generates, beside the
  `WEB_PLAYGROUND.md` link batch 5 wrote;
  every view getter and command method with types, params by name, the built-in list,
  error surfacing, "syntax OK — runtime errors show during Play", "one bundle, many
  projects"); `PROJECT_ROADMAP.md` § Scripting (`:165-177`)
  updated (Rhai decision, Stages 2–3 shipped, game-run ruling); `training.md` gains a
  `### Script Pattern` section under § Patterns Reference; `crates/engine_core/CLAUDE.md`
  file map (`scripting/` beside `behavior_runner/`); `crates/ecs/CLAUDE.md` (`:29` no longer
  says nothing executes it; a `blackboard.rs` row); `crates/editor_integration/CLAUDE.md`
  file map (`script_status.rs`); `crates/editor/CLAUDE.md` (`script_editor.rs` row);
  `crates/playground/CLAUDE.md:19` (`Hooks` is no longer "for batch 7");
  `docs/WEB_PLAYGROUND.md:144` (the `playground_script_errors` row) and `:184` (item 13).

Gates: standard + wasm + games (`GameContext` grew a field; every game constructs
none, so `check_games.sh` suffices) + `cargo check -p engine_core --no-default-features`
(the physics gate). Leaves out: pong (batch 8), web textarea.

## Batch 8 — pong's gameplay as a project, and script editing on the page — LANDED 2026-09-05 (59365d4 in insiculous_2d, 23c5471 in games/pong, 6e67f93 in insiculous_web; marked done once Jesse's browser check passes)

**Re-verified against the tree 2026-09-05 before the handoff** (batch 7 landed since this
section was written). Corrections, each restated where it applies below:

- Three repositories, three staged diffs, one report: `insiculous_2d` (the project files, the
  test and its dev-dependencies, the docs), `insiculous_web` (the panel's markup and script, the
  page copy, the rebuilt bundle) and `games/pong` (its README — that directory is its own
  repository, on its own `jesse` branch, clean today). The planner commits each separately.
- The page's behaviour does not live in `PlaygroundEmbed.astro`: that file (421 lines) is markup
  and styles, and every wasm call lives in `insiculous_web/src/scripts/playground-embed.ts`
  (354 lines) inside the `playground-ready` handler's closure, where `wasm` is a local (`:98`)
  whose export types are declared inline (`:72-96`). The panel's markup goes in the `.astro`
  between the canvas and the console; its behaviour is a NEW module
  `src/scripts/playground-scripts-panel.ts` (one file per panel) exporting one
  `createScriptsPanel(bridge)` — `bridge` is the four exports it needs,
  `playground_list_files`, `playground_read_file`, `playground_write_file`,
  `playground_script_errors`, added to the inline type — returning `{ isDirty(): boolean,
  enable(): void }`. `playground-embed.ts` constructs it once `wasm` exists, calls `enable()`
  where the other controls are enabled (`:197-202`), and replaces its four
  `wasm.playground_is_dirty()` calls — the switch confirm (`:210`), the import confirm
  (`:286`), `beforeunload` (`:341`), and the reset confirm (`:236`), which today asks nothing
  about unsaved edits and gains the same OR — with one `isDirty()` helper that ORs the
  panel's flag in. Astro compiles both files; the site has no unit tests for page scripts
  (`npm run test:data` is the Python data suite), so the panel's contracts are `astro check`
  plus Jesse's browser check.
- There is no separate check export: `playground_write_file` runs the `source_check` hook
  itself for a `.rhai` path and REFUSES the write with the error string (`bridge.rs:157-162`,
  documented at `docs/WEB_PLAYGROUND.md:134`), which surfaces in JS as a thrown value. Save is
  therefore one call: on success `lastSaved = value` and the status reads "saved — syntax OK;
  runtime errors show during Play"; on a throw the status is the thrown string (`ScriptError`'s
  `Display`, `runner.rs:42-56`: `source:<line>: [Syntax] <message>` or
  `header:<line>: [Header] <message>`), the file is not written and the textarea stays dirty.
- The bridge has no Play-state export (`bridge.rs:109-307`), so the panel cannot poll "during
  Play": it polls `playground_script_errors()` on its own 500 ms interval always — the list is
  empty outside Play because batch 7's mirror clears on Stop — into its OWN `<output>`
  (`#script-runtime-errors`), never the Save status element (a shared element would have the
  poll's empty list erase "saved — syntax OK" within half a second), and rewrites it only when
  the joined text changes, because an `aria-live` region re-announces every DOM write.
- The page has no Play control (Play is `Ctrl+P` / `F5` inside the canvas, shortcuts list
  `PlaygroundEmbed.astro:94`), so "focus returns to the canvas only via the Play control"
  reduces to batch 4's standing rule: the page never moves focus. winit 0.30 (`Cargo.lock:4078`)
  binds its keyboard listeners to the canvas element, so a keystroke in the textarea never
  reaches the editor; Jesse's Delete / Ctrl+Z check below is the proof, and no page-side
  guard is written.
- Rhai has no `clamp` (`~/.cargo/registry/src/*/rhai-1.26.0/src/packages/`: `min` / `max` on
  floats and ints in `logic.rs`, `to_float` on ints in `math_basic.rs`, nothing named clamp), so
  the paddle clamps with `max(min(y, limit), -limit)`. The serve hash is
  `(view.frame * 2654435761) % 4294967296` — `frame.wrapping_mul(2654435761)` exactly
  (`balls.rs:13`) for every frame below 3.47 billion, which is 1.8 years of Play; Rhai integers
  are `i64` and overflow-checked, so the product is exact and never wraps in that range — then
  `let t = (hash >> 16).to_float() / 65535.0;` and
  `let dir = vec2(dir_x, t * 1.2 - 0.6).normalize();` (`balls.rs:14-15`). `view.frame` is a
  getter (`rhai_bindings.rs:98`), not a call.
- Action names in the view are lowercase (`runner.rs:322-333`): `view.just_activated(0,
  "action1")`. Player indices are 0-based; under `InputSettings::default()` (`player.rs:190`,
  which is `default_two_player`, `:237-258`) player 0 is W/S + Space and player 1 is arrows +
  Enter.
- The `// @param` header's types are lowercase (`param_header.rs:44-146`,
  `docs/SCRIPTING.md:28-45`): `// @param speed: f32 = 450.0`, `// @param ai: bool = false`,
  `// @param side: str = "left"`, `// @param target: entity = "Ball"` — an `entity` default
  reaches the script as the name string (`:104-107`; the inspector side of that is #105), which
  is what the name overloads take (`view.position(params.target)`). A ref in the scene carries
  only the params that differ from the header defaults; a missing param takes the default at
  run time (batch 7). `docs/SCRIPTING.md` names the command argument `cmd` and the scripts
  follow the doc; this section's earlier `out.` was that argument.
- Scene shapes, from `scene_data.rs:136-360,527-541` and the example scenes: texture refs are
  relative to the project's asset base (`assets.rs:218` joins them), so
  `images/paddle_16px.png` and `images/ball_8px.png`, the two files copied from
  `../games/pong/assets/` to `assets/images/`; the right paddle mirrors with
  `Sprite(scale: (-1.0, 1.0))` (`spawning.rs:25`); a paddle is `RigidBody(body_type: Kinematic,
  can_rotate: false)` — without the body, `set_kinematic_target` never reaches rapier
  (`commands.rs:375-381`) and the collider stays where it spawned — with `Collider(shape:
  CapsuleY(half_height: 50.0, radius: 10.0), friction: 0.0, restitution: 1.0)` (`capsule_y(120, 10)`,
  `physics/src/components.rs:261-266`); the ball is `RigidBody(body_type: Dynamic, gravity_scale:
  0.0, can_rotate: false, ccd_enabled: true)` with `Collider(shape: Circle(radius: 10.0), friction:
  0.0, restitution: 1.0)`; a wall is `RigidBody(body_type: Static)` with `Collider(shape:
  Box(half_extents: (400.0, 10.0)), friction: 0.0, restitution: 1.0)`; a goal is
  `RigidBody(body_type: Static)` with `Collider(shape: Box(half_extents: (10.0, 300.0)), is_sensor:
  true)`. The wire defaults are friction 0.5 and restitution 0 (`scene_data.rs:238-241`), and a
  restitution-0 paddle kills the rally on first contact (the ball script's `vel.x.abs() < 0.1`
  guard then returns forever), so every gameplay collider spells both, as `spawning.rs:40-41,60,91-92`
  does; the label is
  `UiLabel(text: "0 : 0", anchor: TopCenter, offset: (0.0, 24.0), font_size: 32.0)`; an entity
  `Camera` carrying `Camera2D(is_main_camera: true, viewport_size: (800.0, 600.0))` frames the
  800 × 600 field in Play (`play_session.rs:83` reads `main_camera_pose`);
  `physics: Some(PhysicsSettings(gravity: (0.0, 0.0), pixels_per_meter: 100.0, timestep:
  0.016666668))` matches `PhysicsConfig::top_down()`'s default scale
  (`physics_world/mod.rs:30`). Coordinates from `lib.rs:79-98` and `constants.rs`: paddles at
  x = ±370 with transform scale (0.25, 1.5); ball scale 0.25; walls at y = ±290, 800 × 20 —
  `#white` at scale (10.0, 0.25), colour (0.35, 0.35, 0.42, 1.0), which is
  `ChaosTheme::for_mode(Normal)`'s `structure_color` (`chaos_theme.rs:53`); goals at x = ±410;
  a black 960 × 720 `#white` background at depth −100 (scale (12.0, 9.0) — `spawn_background`
  oversizes the window by 20 %, `spawn_helpers.rs:18-24`); emissive 1.5 / 2.5 /
  0.6 for paddles / ball / walls; paddle colours (1.0, 0.3, 0.3, 1.0) and (0.3, 0.5, 1.0, 1.0).
  Names are the Rust game's: `Left Paddle`, `Right Paddle`, `Ball`, `Top Wall`, `Bottom Wall`,
  `Left Goal`, `Right Goal`, plus `Scoreboard`, `Background`, `Camera`. A script attaches as
  `Scripts([(script_id: "paddle", source_path: "scripts/paddle.rhai", params: {"x": F32(370.0),
  "ai": Bool(true)})])` — the wire form `scene_serializer/dynamic_and_scripts_tests.rs:171-174`
  writes (`ComponentData::Scripts(Vec<ScriptRefData>)`, `script_data.rs:36-42`; param values are
  `ScriptValueData` variants), `script_id` the file stem as the `.rhai` drop and the catalog set
  it (`panel_renderer/mod.rs:300-311`, `script_catalog.rs:33-36`), `source_path` relative to the
  asset base. The scene's header comment says what the scripts do and which keys drive it.
- The label needs no font in the project: `ProjectHost::init` loads none
  (`project_host.rs:132-139`), so `game_base_font` is `None` and Play keeps the editor's face
  (`editor_game/mod.rs:252-258`). The project ships no `fonts/`, `locales/` or `sounds/` — two
  textures, one scene, four scripts.
- The test lives in the crate that ships the project: new
  `crates/playground/tests/pong_rules.rs` (the crate has no `tests/` today; a `cdylib` + `rlib`
  crate runs integration tests natively). Dev-dependencies, the `editor_integration/Cargo.toml:30`
  pattern: `engine_core = { path = "../engine_core", features = ["test-support"] }` for
  `StubResolver` and `frame`, `ecs = { path = "../ecs" }` for `Blackboard` (`ecs/src/lib.rs:54`),
  `input = { path = "../input" }` for `InputHandler` and `InputEvent` (path, not `workspace = true`:
  the root `[workspace.dependencies]` names only `common`, `Cargo.toml:30-32`) — neither the blackboard nor
  those two is in `engine_core::prelude`, which does carry `World`, `Name`, `Transform2D`,
  `InputSettings`, `KeyCode`, `CollisionData`, `CollisionEvent` and `SceneLoader`;
  `ScriptRunner` is `engine_core::ScriptRunner` (`lib.rs:80-85`). The project's asset base is
  `concat!(env!("CARGO_MANIFEST_DIR"), "/assets/projects/pong/assets")`; `SceneLoader::load_from_file`
  reads it through the native VFS (`vfs/mod.rs:54-57`), `SceneLoader::instantiate(&data, &mut
  world, &mut StubResolver::default())` builds the world (every texture resolves to white), and
  `runner.reset(&mut world, base)` records the base the `.rhai` reads use. With `physics: None`
  the command fallback integrates `velocity * delta_time` into `Transform2D` and `reset_body`
  writes the transform (`commands.rs:331-340,383-394`), and a script's own `me.velocity` reads
  the `RigidBody` component's stored velocity, which the fallback never updates
  (`commands.rs:435-444`) — so the ball's speed-maintenance guard (`vel.x.abs() < 0.1`, mirrored
  from `balls.rs:30`) returns early in the test, which is the behaviour under test anyway. The
  blackboard is read as `world.resource::<Blackboard>()` (`world.rs:464`) and written for the
  win set-up with `resource_mut` (`:469`) and `Blackboard::set` (`blackboard.rs:25`).
- `build_wasm.sh` has no project list to edit: each project is a `--project` flag, and the
  invocation of record lives in `docs/WEB_PLAYGROUND.md:26-27` (its bundle tree at `:13-22`
  shows one `<slug>`); those lines gain pong. `--sync ../insiculous_web/public` rewrites files
  the site repo TRACKS — all sixteen under `public/playground/v1/`, `game_bg.wasm` and
  `game.js` included — so the site diff carries whatever the sync changed plus the new
  `projects/pong/assets/**`. The `<select>` needs no edit: it is populated from
  `playground_list_projects` at boot (`playground-embed.ts:160-168`), and `?project=pong` is the
  boot's own query parameter (`web_entry.rs:130-150`).
- Docs that describe the pre-batch state and are therefore part of this batch:
  `docs/WEB_PLAYGROUND.md:13-27`; `crates/playground/CLAUDE.md`'s file map (an
  `assets/projects/pong/` row); `docs/SCRIPTING.md` (§ Worked example: pong, placed after
  § Built-In Behaviors); `games/pong/README.md` (a short "Pong as data" section just before
  § "The Deion Pivot: Tong" at `:68`); `insiculous_web/src/pages/playground.astro` (a § Scripts
  paragraph beside § Command API, `:31-38`); the embed's shortcuts list (`PlaygroundEmbed.astro:95`
  says `Ctrl+S` saves the scene — it gains "inside the Scripts panel, saves the script").
  `PROJECT_ROADMAP.md` names none of this and waits for batch 11.
- Size budgets: `PlaygroundEmbed.astro` 421 lines (+ ~30 markup, ~60 style) and
  `playground-embed.ts` 354 (+ the type lines, the helper and the wiring) both stay under 600;
  the new module is small by construction. No Rust file is touched except `Cargo.toml` and the
  new test.

Repos: `insiculous_2d` (`crates/playground/assets/projects/pong/`, `crates/playground/Cargo.toml`
dev-dependencies, `crates/playground/tests/pong_rules.rs`, `docs/WEB_PLAYGROUND.md`,
`docs/SCRIPTING.md`, `crates/playground/CLAUDE.md`), `insiculous_web`
(`src/components/PlaygroundEmbed.astro`, `src/scripts/playground-embed.ts`, new
`src/scripts/playground-scripts-panel.ts`, `src/pages/playground.astro`, the synced bundle under
`public/playground/v1/`), `games/pong` (`README.md`).

Target shapes:

- Project `pong` under `crates/playground/assets/projects/pong/assets/`:
  `scenes/pong.scene.ron` (background, two paddles, ball, two walls, two goal sensors, a
  `Scoreboard` UiLabel and a `Camera` — the shapes, coordinates and colours pinned in the
  preamble, from `games/pong/src/spawning.rs`, `lib.rs:79-98` and `constants.rs`; textures
  `images/paddle_16px.png` and `images/ball_8px.png` copied, `#white` for walls and
  background), `scripts/paddle.rhai` (`early_update`; header `player: i32 = 0`, `x: f32 =
  -370.0`, `speed: f32 = 450.0`, `ai: bool = false`, `ai_speed: f32 = 255.0`, `dead_zone: f32 =
  2.0` — Medium, `types.rs:15-29` — `target: entity = "Ball"`; a human paddle moves by
  `view.move_y(params.player) * params.speed * dt` (`paddles.rs:11-13`), an AI paddle chases
  `view.position(params.target).y` at `ai_speed` outside the dead zone (`:47-55`); the new y
  is `max(min(y, 230.0), -230.0)` — `PADDLE_MAX_Y`, `constants.rs:11` — and lands as
  `cmd.set_kinematic_target(me, vec2(params.x, new_y))`; the scene's refs set `player: 0` on
  the left, `x: 370.0, ai: true` on the right), `scripts/ball.rhai` (`early_update` serves on
  `view.just_activated(0, "action1") || view.just_activated(1, "action1")` when
  `view.blackboard_bool("serving", true)` — absent means serving, so the first Play is not a
  deadlock — AND `!view.blackboard_bool("game_over", false)`, so the restart press cannot serve
  in the same frame the scoreboard resets; direction x from `view.blackboard_str("last_scorer",
  "left")` the way `flow.rs:38-41` does (`"left"` → −1.0), `dir` from the three hash lines in the
  preamble, `cmd.set_velocity(me, dir * params.speed)` then `cmd.set_blackboard_bool("serving",
  false)`; `update` maintains speed as `balls.rs:27-45` does — return when `me.velocity.x.abs()
  < 0.1`, else pin x to `sign * speed` and clamp y to `±max_vertical` with `max(min())`, writing
  only when the change is over 1.0 — ONLY while not `serving`, so it never fights the goal's
  reset in the same phase; header `speed: f32 = 250.0`, `max_vertical: f32 = 500.0`),
  `scripts/goal.rhai` (`update`; header `side: str = "left"`; on
  `view.has_collision_started(me, "Ball")`: the OTHER side scores —
  `cmd.set_blackboard_int("right_score", view.blackboard_int("right_score", 0) + 1)` for the
  left goal — `cmd.set_blackboard_str("last_scorer", <that side>)`, `cmd.reset_body("Ball",
  vec2(0.0, 0.0))`, `cmd.set_blackboard_bool("serving", true)`; the scene's right goal sets
  `side: "right"`), `scripts/scoreboard.rhai` (`update`; writes `` `${left} : ${right}` `` to
  `cmd.set_label_text(me, …)`; at 7 (`WIN_SCORE`) writes "LEFT WINS — Action1 to restart" or
  "RIGHT WINS — Action1 to restart" for the side that reached it and
  `cmd.set_blackboard_bool("game_over", true)`; on Action1 (either player) while `game_over` it
  zeros both scores, clears `game_over` and sets `serving`). Every script carries its
  `// @param` header with these defaults in the lowercase syntax of `docs/SCRIPTING.md:28-45`,
  and every hook is `fn <hook>(me, view, params, cmd, dt)`.
- `docs/WEB_PLAYGROUND.md:26-27`, the invocation of record, adds
  `--project pong=Pong=crates/playground/assets/projects/pong` (the tree at `:13-22` shows both
  slugs); the bundle is rebuilt with that invocation and synced into the site checkout.
- Page: a "Scripts" panel between the canvas and the console — `<label>` + `<select
  id="script-select">` of the project's `.rhai` files (from `playground_list_files`, filtered
  on the extension, sorted), `<label>` + `<textarea id="script-source">` (monospace,
  `spellcheck="false"`) bound to `playground_read_file` on selection and
  `playground_write_file` on Save, a Save `<button>` (`Ctrl+S` / `Cmd+S` with the textarea
  focused `preventDefault`s and runs the same Save — the browser's own save dialog otherwise
  opens), a dirty flag (`value !== lastSaved`) exposed as `isDirty()` and ORed into the page's
  `isDirty()` helper, which every switch / reset / import confirm and `beforeunload` reads, a
  switch of the select while dirty confirming first — the panel keeps `currentPath`, and a
  cancelled confirm sets `select.value = currentPath` and returns before touching the textarea
  (the browser has already moved the select's value when `change` fires; without the reset a
  later Save would write the old file's text under the new file's path) — and two
  `<output aria-live="polite">` elements: `#script-status` carries Save's result immediately —
  "saved — syntax OK; runtime errors show during Play" on success, the thrown `ScriptError`
  string on refusal with the file left unwritten and dirty — and `#script-runtime-errors` carries
  the 500 ms poll of `playground_script_errors()`, rewritten only when its joined text changes.
  The behaviour is `src/scripts/playground-scripts-panel.ts`'s
  `createScriptsPanel(bridge)`; the page never moves focus (batch 4's rule stands), and
  `playground-embed.ts` only constructs the panel, enables it at `playground-ready` and ORs
  its flag.
- Tests (`crates/playground/tests/pong_rules.rs`, headless, no physics system): (1) load
  `pong.scene.ron` through `StubResolver`, `reset` the runner on the project's asset base, run
  one frame with no input and assert the ball is still at centre and `runner.errors()` is
  empty — the four scripts compiled and every name resolved; press Space through
  `test_support::frame` (`InputEvent::KeyPressed(KeyCode::Space)`, player 0's Action1), run
  `early_update` at 1/60 s and assert the ball's `Transform2D` has left centre (the serve,
  integrated by the no-physics fallback) and `serving` is now `Bool(false)`; then INJECT
  `CollisionData { event: CollisionEvent { entity_a: ball, entity_b: left_goal, started: true,
  stopped: false }, contacts: vec![] }` into `ScriptRunner::update` and assert the blackboard
  reads `right_score == I32(1)`, `last_scorer == Str("right")`, `serving == Bool(true)`, the
  ball is back at centre, and the label reads `"0 : 1"` after one more `update` — the rules
  pinned through the fallback, not rapier. (2) with `left_score` set to 6 on the blackboard,
  inject a right-goal collision, run one more no-input `update`, and assert the label starts
  with "LEFT WINS" and `game_over == Bool(true)`; press Space, run both phases, and assert the
  ball did NOT leave centre (the
  restart press cannot serve), both scores are 0, `game_over` is false and `serving` is true.
  Both tests assert `runner.errors()` is empty at the end. Every blackboard write lands when its
  phase's commands apply, and the next phase's view is built from that (`runner.rs:296-299,483`),
  so a rule that reads another script's write is observed one phase later: test (1) already reads
  the label "after one more `update`", and test (2) runs one no-input `update` after the
  collision injection BEFORE asserting the win — in the injection phase the scoreboard still saw
  `left_score == 6`.
- Docs: `docs/SCRIPTING.md` § Worked example: pong (the four scripts' contracts, the blackboard
  keys `left_score` / `right_score` / `last_scorer` / `serving` / `game_over`, the same-frame
  rules above, the keys that drive it); `docs/WEB_PLAYGROUND.md:13-27`;
  `crates/playground/CLAUDE.md` file map; `games/pong/README.md` "Pong as data" before § "The
  Deion Pivot: Tong" (`:68`) pointing at `insiculous_2d/crates/playground/assets/projects/pong/`
  and `/playground/?project=pong`; `insiculous_web/src/pages/playground.astro` § Scripts;
  the embed's shortcuts line.

Gates: standard (`cargo test --workspace` runs the new integration test; the comment-tag grep
extended over `crates/playground/assets --include=*.rhai`) + wasm (the diff touches
`crates/playground/Cargo.toml`, under a covered crate root) + `npm run verify` + bundle
(`build_wasm.sh`'s size line under 20 MiB). No games gate: no public item changes, and the
pong repo's diff is one README. **Jesse's browser check:** open `/playground/?project=pong`,
Play, a rally with the AI (W/S move the left paddle, Space serves), score, edit
`paddle.rhai`'s speed in the textarea, Save, Stop, Play — the paddle is faster; **reload the
tab** — the edit is still there; with an entity selected, type Delete and Ctrl+Z inside the
textarea — the viewport is untouched (rebuttal 1, gemini F5). Leaves out: menus, power-ups,
chaos modes, achievements (filed in batch 11).

## Batch 9 — the six games as editor bundles (independent, droppable) — LANDED 2026-09-06 (5977194 engine, d23f7a7 pong, a6b4126 snake, d144529 breakout, a643a76 frogger, f090b4d asteroids, 07c08d6 space_invaders, 4fba120 site; marked done once Jesse's browser check passes)

**Re-verified against the tree 2026-09-06 before the handoff** (batches 3–8 landed since this
section was written on Sep 4, and it was twenty lines against the three-hundred of its
neighbours). Corrections, each restated where it applies below:

- The optional dependency is already on every game: all six `Cargo.toml`s carry
  `editor = ["dep:editor_integration"]` (`:9`) and the optional path dependency (`:17`), and
  `scripts/check_games.sh:13-25` already builds each with and without it natively. No manifest
  changes. What no gate compiles today is the editor feature ON THE WASM TARGET for a game
  crate; the playground crate proves the editor's wasm path
  (`crates/playground/src/web_entry.rs:18,271` runs `run_game_with_editor_opts` in the
  browser), so the shape is sound, and this batch makes that check standing (below).
- `build_wasm.sh` has no `--features` flag and no layout for a game under `/playground/`:
  `--kind` takes `games` or `playground` (`:59,68`), `--kind playground` demands `--project`
  (`:72-76`) and expects `ASSET_BASE = "/playground/<version>/assets"` plus a `BUNDLE_VERSION`
  constant (`:86-87,97-101`), and its output directory has no slug segment (`:150-151`). So
  the section's "`--features editor` passes the feature through" becomes a THIRD kind,
  `--kind editor`, which names the layout and turns the feature on together — a separate flag
  would let the two disagree. Its rules: no `--project`; the entry check greps
  `const EDITOR_ASSET_BASE: &str = "/playground/<slug>/<version>/assets"` — a differently
  named constant, because the game's plain `ASSET_BASE` stays in the same file and a check on
  that name would pass under either kind; the cargo build line (`:158`) gains
  `--features editor`; `DIST_BASE="$GAME_DIR/dist/playground/$SLUG"` and
  `SYNC_SUBPATH="playground/$SLUG/$VERSION"`; assets copy the games way (`:231-235`, today's
  `else` branch — a game's `assets/` tree, no `projects.json`); the achievements export stays
  games-only (`:252`, already conditioned — the editor bundle registers the same
  achievements, but the board reads `/games/<slug>/achievements.json`, which this bundle does
  not replace); the local test page's `IMPORT_URL` is `/playground/$SLUG/$VERSION/game.js`,
  no banner line, and its canvas is the clamped size (next item). Every one of the script's
  kind branches (`:86-90`, `:149-155`, `:261-285`) becomes a three-way `case`: no `else` may
  serve the new kind by default, because the games `else` would expect the plain
  `ASSET_BASE`, write `dist/games/<slug>/` and sync an editor bundle OVER
  `public/games/<slug>/<version>/`; `--features editor` is appended to the build line only
  under `BUILD_KIND == editor` (the playground crate has no such feature and a plain game
  build must not gain it); the mismatch diagnostic's `ACTUAL_BASE` (`:93`) greps the line of
  the constant being checked, since after this batch a game's entry carries both; the `:59`
  message and the `:43` usage line name the third kind. The usage header (`:8-33`)
  documents it.
- The editor enlarges any window under 1024 × 720
  (`crates/editor_integration/src/constants.rs:16-26`, applied in `run_options.rs:55`), and on
  wasm the placeholder canvas's `width`/`height` attributes are copied onto winit's canvas
  (`crates/renderer/src/window.rs:91-98`) while winit sizes that canvas from the config
  (`window_manager.rs:114-119`) — so the page's canvas must be the CLAMPED size, not the
  game's: 1024 × 720 for the five 800 × 600 games and 1024 × 768 for frogger (720 × 768,
  `../games/frogger/src/constants.rs:14-15`). The site page derives it as
  `Math.max(width ?? 800, 1024)` × `Math.max(height ?? 600, 720)` from the games entry's own
  `width`/`height` (`frogger.md:6-7` carries them; the other five default), and the script's
  test page does the same over `WIN_W`/`WIN_H` (`:271-282`, the existing best-effort regex);
  both mirrors name `constants.rs`'s pair in a comment, the DRY rule for a twin.
- The six web entries (`../games/<g>/src/web_entry.rs`, 41–45 lines each) are near-identical:
  one `ASSET_BASE` (`:23`) and one `start()` that preloads, builds `game_config(ASSET_BASE)`
  with the game's localStorage save keys and calls `run_game` (`:25-41`; pong passes two keys,
  the rest three). Under the feature the entry preloads `EDITOR_ASSET_BASE` instead, passes
  `game_config(EDITOR_ASSET_BASE)` with NO save keys — an editor session must never write
  `beinsiculous.games.<slug>.*`, which the site's achievements board reads
  (`docs/WEB_SAVES.md:11-32`); with `input_settings_path` unset the engine takes the default
  bindings (`crates/engine_core/src/game.rs:364`) — and calls
  `editor_integration::run_game_with_editor_opts(game, config, EditorRunOptions { prefs_slot:
  Some(PathBuf::from("beinsiculous.playground.<slug>.editor_prefs")), ..Default::default() })`,
  because the default slot is the native filename `editor_prefs.json` (`constants.rs:13`,
  `editor_game/mod.rs:119`), which on wasm would be a bare localStorage key outside the
  contract; the playground's own key is `beinsiculous.playground.editor_prefs`
  (`WEB_SAVES.md:34-38`), and each game gets its own because the preferences carry the
  camera. The site slug for space_invaders is `invaders` (`WEB_SAVES.md:15-22`, the
  `public/games/` directory name), so its base is `/playground/invaders/v1/assets`. Everything
  the game does in Play — menus, chaos modes, achievements registering — is untouched.
- Each game's `lib.rs:5-7` says the split "keeps `editor_integration` out of the library —
  editor wiring lives in `main.rs` only"; after this batch the library's `web_entry.rs` carries
  it too, behind the same feature, so the sentence changes in all six.
- The site has no `playground` content collection and needs none: the games collection
  (`../insiculous_web/src/content.config.ts:7-30`) already carries the slug, title and size, so
  it gains one optional field, `editor: z.string().startsWith('/').optional()` — the editor
  bundle's glue path, `/playground/<slug>/v1/game.js` — set on all six entries, and
  `src/pages/playground/[slug].astro` builds one page per entry that has it (the pattern is
  `src/pages/games/[slug].astro:7-13`). Six new files and a schema for one path each would be
  the heavier shape, and the entry's `width`/`height` are what the clamp reads.
- The `<select>` on `/playground/` is populated at boot from `playground_list_projects` by
  `playground-embed.ts:168-181`, which clears it with `innerHTML = ''`, and its `change`
  handler (`:215-242`) calls `playground_open_project` on the value. The six games are static:
  `PlaygroundEmbed.astro` wraps the existing single option (`:32-34`) in
  `<optgroup id="project-select-data" label="Projects">` and adds a second
  `<optgroup label="Rust games (layout only — rules are compiled in)">` from an `editorGames`
  prop — `playground.astro` calls `getCollection('games')`, filters on `editor`, sorts by
  `order` and passes the list, so the embed stays presentational like `GameEmbed` — each
  option's value the page path `/playground/<slug>/`; the population code clears and fills
  the data optgroup only, and the handler treats a value starting with `/` as navigation:
  the same dirty confirm, whose cancel resets `projectSelect.value = currentSlug` exactly as
  the project branch does (`:224-226` — the browser has already moved the value when
  `change` fires, and without the reset the cancelled option can never fire `change`
  again), then `leavingByChoice = true; window.location.href = value`.
- The keyboard-shortcuts list (`PlaygroundEmbed.astro:113-128`, with its `.shortcuts-panel`
  and `.shortcuts-list` styles) is what the editor page needs verbatim, so it moves to
  `src/components/EditorShortcuts.astro` and both embeds render it. `GameEmbed.astro` serves
  the editor page as it is, with two additions: an optional `canvasLabel` prop whose default is
  the current "game canvas; focus it to play" (`:41`) and a named slot `controls` whose
  fallback is the current controls note (`:46-50`), so the editor page passes its own label
  and note and GameEmbed's gate script (`:69-109`) is not written a third time.
- Bundle weight: the plan's "+2–3 MiB each" was a guess. The playground bundle is 9.7 MiB and
  a plain game 2.6–4.1 MiB (`public/playground/v1/`, `public/games/*/v2/`), so expect each
  editor bundle near 9–10 MiB — six of them ~55 MiB raw under `public/playground/<slug>/v1/`,
  tracked by the site repo like the rest of `public/` (114 bundle files today; the pack is
  12 MiB because wasm compresses well). Each is under the 20 MiB gate with room; the total is
  Jesse's call at adjudication — six, or a subset with the rest filed.
- Each game's `Cargo.lock` gains the editor's wasm-side entries the first time the wasm build
  resolves the feature (breakout's working tree already shows it from an earlier check: +98
  lines, `const-random`, `getrandom 0.3`, unstaged); stage it in that game's diff and say so.
- `check_games.sh` gains, per game, TWO wasm checks after its native four (`:13-25`):
  `cargo check --manifest-path ../games/<g>/Cargo.toml --lib --target wasm32-unknown-unknown`
  with and without `--features editor` — the plain one because `web_entry.rs` is
  `cfg(target_arch = "wasm32")` and no native check ever compiles it, so a break in its
  non-feature branch would surface only at bundle time; the feature one because a deployed
  bundle now depends on it. The engine's `wasm-check.yml` runs `check_wasm.sh`, which never
  sees a game. The script gains a preflight that the target is installed
  (`rustup target list --installed | grep -q wasm32-unknown-unknown`, hard fail naming
  `rustup target add wasm32-unknown-unknown`, `build_wasm.sh`'s style), and its header says
  so the way `check_wasm.sh:14` does.
- Docs that describe the pre-batch state and are therefore part of this batch:
  `docs/WEB_PLAYGROUND.md` (a new § The game bundles between § The bundle contract, `:47`, and
  § The store, `:70`: the layout, the six invocations of record, what does and does not
  persist, the prefs key family); `docs/WEB_SAVES.md:34-38` (the per-game prefs key);
  `scripts/build_wasm.sh`'s header; each game's `README.md` (pong § Editor Mode `:45-47` and
  breakout `:85-87` gain the browser sentence; asteroids `:11`, frogger `:13`, snake `:9` and
  space_invaders `:14` have only the command line, so one sentence follows their `## Running`
  block) and `CLAUDE.md` (the "With `--features editor` the identical game runs inside the
  engine's scene editor" sentence, `:19` or breakout's `:20`, gains "and at
  `/playground/<slug>/` in the browser"); `../insiculous_web/README.md:142-160` § The editor
  bundle (the per-game bundles and their directory); `../insiculous_web/docs/roadmap.md:129-132`
  (the browser editor paragraph); `src/pages/playground.astro` (a § Rust games in the editor
  paragraph after § Scripts, `:31-38`). `PROJECT_ROADMAP.md` waits for batch 11.
- Size budgets: `build_wasm.sh` 347 (+ ~40), `PlaygroundEmbed.astro` 542 (− the panel, + the
  optgroups: the extraction is what keeps it under 600), `playground-embed.ts` 365 (+ ~20),
  `GameEmbed.astro` ~200 (+ ~10). No engine crate file is touched; the engine's diff is two
  scripts and two docs.
- Executor: gemini's weekly budget is nearly spent (7.8 % on Sep 6), so Jesse runs the
  executor as a separate Claude Code session on Opus 5 at low effort, from the same handoff.
  The loop is otherwise unchanged — kimi remains the different-vendor code reviewer — and the
  ledger rows name the executor.

Repos: eight repositories, eight staged diffs, one report. The six game repos
(`src/web_entry.rs`, `src/lib.rs` header comment, `Cargo.lock`, `README.md`, `CLAUDE.md`),
`insiculous_2d` (`scripts/build_wasm.sh`, `scripts/check_games.sh`, `docs/WEB_PLAYGROUND.md`,
`docs/WEB_SAVES.md`), `insiculous_web` (`src/content.config.ts`, the six
`src/content/games/*.md`, new `src/pages/playground/[slug].astro`, new
`src/components/EditorShortcuts.astro`, `src/components/GameEmbed.astro`,
`src/components/PlaygroundEmbed.astro`, `src/scripts/playground-embed.ts`,
`src/pages/playground.astro`, `README.md`, `docs/roadmap.md`, the six synced bundles under
`public/playground/<slug>/v1/`). The planner lands each separately: the engine first (the
script the six builds need), the six games, then the site.

Target shapes:

- `scripts/build_wasm.sh --kind editor` (the third kind, rules in the preamble): usage line
  `[--kind games|playground|editor]`, the kind validated at `:68`, no `--project` requirement,
  `EDITOR_ASSET_BASE` checked against `/playground/$SLUG/$VERSION/assets` with the same
  hard-fail wording as the other two, `--features editor` on the cargo build, the
  `dist/playground/<slug>/<version>/` layout and `playground/<slug>/<version>` sync subpath,
  the games-style asset copy, no achievements export, and a test page at
  `dist/playground/<slug>/index.html` importing `/playground/<slug>/<version>/game.js` with the
  clamped canvas (`max(WIN_W, 1024)` × `max(WIN_H, 720)`, the constants named). The header's
  version paragraph gains the editor bundle's FOUR places: the constant, this script's output
  dir, the games entry's `editor:` path, the deployed `public/playground/<slug>/<version>/`.
- `../games/<g>/src/web_entry.rs`, all six: `ASSET_BASE` and its checklist stay as they are;
  `#[cfg(feature = "editor")] const EDITOR_ASSET_BASE: &str = "/playground/<slug>/v1/assets";`
  with a doc comment stating its own four-place contract (its version is independent of the
  game's `v2`); the feature selects the preload base, the config (save keys only WITHOUT the
  feature), the runner (`run_game` / `run_game_with_editor_opts` with the prefs slot
  `beinsiculous.playground.<slug>.editor_prefs`) — one `#[wasm_bindgen(start)] start()` stays
  the entry, the error reporting through `set_boot_status` unchanged. The header comment says
  what the editor bundle is and that it writes none of the game's keys. `lib.rs:5-7`'s
  sentence becomes: the split keeps `editor_integration` behind the `editor` feature in both
  entry points.
- `scripts/check_games.sh`: the target preflight and the two wasm checks per game
  (preamble), and its closing message names them.
- Site: the `editor` field (`content.config.ts`, with a comment like `wasm`'s at `:16-19`);
  `editor: '/playground/<slug>/v1/game.js'` on all six entries; `src/pages/playground/[slug].astro`
  — `getStaticPaths` over the games with `editor`; `<h1>` "<title> in the editor"; a lede that
  says what this is (the Rust game running inside the scene editor, in the browser: the
  entities are live and editable, the rules are compiled in, and nothing persists — reload and
  the game is itself again); `<GameEmbed src={game.data.editor} title={…} canvasLabel="scene
  editor canvas; focus it to edit and play" width={…} height={…}>` with the clamped size and a
  `controls` slot reading "Click the canvas to focus it, then F5 to play the real game; the
  editor's shortcuts are listed below."; `<EditorShortcuts />`; links to `/games/<slug>/` and
  `/playground/`. `EditorShortcuts.astro` is the moved list and styles, rendered by
  `PlaygroundEmbed.astro` where the panel was. `PlaygroundEmbed.astro`'s two optgroups and
  `playground-embed.ts`'s data-group population and path navigation (preamble).
  `playground.astro` § Rust games in the editor: one paragraph — the six open inside the same
  editor, layout only, rules compiled in, nothing persists, the group in the Project select.
- The six bundles, from the engine root, the invocations of record in `WEB_PLAYGROUND.md`:
  `scripts/build_wasm.sh ../games/<g> <slug> --kind editor --version v1 --sync
  ../insiculous_web/public` for pong, snake, breakout, frogger, asteroids and
  space_invaders (`invaders`); the sync writes `public/playground/<slug>/v1/{game.js,
  game_bg.wasm, assets/…}` into the site checkout, all staged there. Every synced tree is
  new (no version dir exists under `public/playground/<slug>/` today), so nothing deployed
  is overwritten.
- Docs: the list in the preamble.

Gates: `scripts/check_games.sh --test` (the six native suites and clippy with and without
the feature, plus the two new wasm checks per game — this is the games' Rust gate, and
`--test` because the six repos' own suites are what stand behind their bundles; the engine
diff has no Rust, so `cargo test --workspace` and `check_wasm.sh` are not required and the
report says so); the
comment-tag grep over `crates src examples` AND over each game's `src`; the bundle gate six
times (each size line under 20 MiB, recorded in the report); `npm run verify` under Node 24
(`nvm use 24` — batch 8's log). No engine wasm gate: no covered crate root is touched.
**Jesse's browser check:** `/playground/`'s Project select shows the Rust games group;
picking Pong opens `/playground/pong/`, the Rust pong is inside the editor with its entities
in the hierarchy; F5 plays the real game (its menu, a rally); Stop restores the layout;
select the ball, move it with the W tool, Play — it starts from there; **reload the tab** —
the game is back to its own layout (nothing persists, by design); `/playground/frogger/`
renders at 1024 × 768 with no sideways scroll on a phone; `/achievements/` shows no unlock
earned inside the editor session; Ctrl+S inside `/playground/pong/` — the status bar reports
the save, nothing breaks, and a reload forgets it; and the pages this batch's shared
components already serve: `/games/pong/` still boots and plays, and `/playground/`'s select
still opens Examples and Pong, with a cancelled dirty switch leaving the select on the
current project. Leaves out: persistence of layout edits for a Rust game (a
Save inside the editor writes the in-memory VFS only — there is no project store in a game
bundle; filed if wanted), a link from `/games/<slug>/` to its editor page (filed), and, if
dropped, the whole batch (batch 11's list).

## Batch 10 — the template repo (#49 item 3) and the export README's link to it — LANDED 2026-09-06 (be328ee engine, f860116 the template's root commit, e8c4e69 site, b4d7049 plan, 22a714f the working set's seat after the remote was created and `scripts/setup.sh` swept clean; marked done once Jesse's browser check passes)

**Re-verified against the tree 2026-09-06 before the handoff** (every batch since 3 landed
after this section was written on Sep 4, and it was five bullets — two of them planner's
work — against the three-hundred lines of its neighbours). The docs close-out, the ledger,
the board and the merge moved to a batch 11 of their own, below, so that this batch is one
diff the reviewers can read as one thing: the template repository and the link that points
at it. Corrections, each restated where it applies below:

- **The exported README does not point at a template.** `crates/playground/src/archive.rs:135`
  writes two links, `docs/WEB_PLAYGROUND.md` and `docs/SCRIPTING.md`, and no test pins the
  string (`archive/tests.rs` greps clean for `github`). This batch adds the third sentence
  (below), which is an engine change under `crates/playground`: the wasm gate applies, the
  playground bundle rebuilds with the invocation of record (`docs/WEB_PLAYGROUND.md:27-30`,
  both `--project` lines) and re-syncs into the site, and the site's `npm run verify`
  runs. Batch 5's sentence at `docs/WEB_PLAYGROUND.md:203` ("this is the layout the template
  repo conforms to") names no repo; it gains the URL and the two directions of the drop-in.
- **The working set has no seat for the repo.** `scripts/lib/repos.sh:41-52` (the root
  repository's `WORKING_SET_REPOS`) lists the six games and the four others; `.idea/vcs.xml:7-12`
  carries one mapping per game; the root `.gitignore:24` ignores `/games/` whole, so it needs
  nothing. `scripts/setup.sh`'s org sweep warns about any org repo the list does not name, so
  the list entry is part of this batch, not a courtesy — but `setup.sh` cannot run green until
  Jesse has created the remote (it clones what the list names), so the root change is staged
  and committed after the remote exists. The games gate `scripts/check_games.sh:23` lists six
  names; the template joins as the seventh, because a template nobody builds rots the first
  time a public seam changes, and this gate is how the six games do not.
- **The engine dependency form is a decision, and it is taken here.** `.claude/skills/new-game/SKILL.md`
  prescribes the relative path (`../../insiculous_2d/crates/engine_core`, every game's
  `Cargo.toml:17`), and a stranger who presses "Use this template" has no engine beside them.
  Ruling (planner's, reversible with a two-line `Cargo.toml` change, recorded for Jesse's
  read): **the path form stays**, and the README's first section is the two-clone layout.
  One dependency form across the seven game repositories; the engine is edited beside a game
  by design (the skill's "fix it in the engine, with tests"); the crate builds in the working
  set today, so the executor can run its gates; and a git dependency on `main` cannot
  compile until `main` carries this effort — `ctx.scripts` shipped Sep 5 on `jesse`.
- **`project.ron` at the template's root is the manifest an export writes** (`archive.rs:124-131`,
  `ron::ser::to_string_pretty` of `ProjectManifest`: `slug`, `title`, `bundle_version`,
  `content_hash`, `origin`). It has no native reader; it is there because the export zip has
  it, so the clone and the archive are the same tree. Validation step 8
  (`docs/WEB_PLAYGROUND.md` § Validation) refuses any root entry but `project.ron`, `README.md` and
  `assets/`, so a clone is never zipped whole: the README names the two paths to zip. An import
  assigns `ProjectOrigin::Imported` and an empty hash (`archive.rs:315-321`), and the template's
  manifest says the same.
- **A foreign export has its own scene name and no coin.** An export of pong carries
  `scenes/pong.scene.ron`, so a template that names `scenes/main.scene.ron` crashes at `init`
  on the very drop-in its README promises (gemini F1). The template therefore names no scene:
  `spawning::spawn_scene` loads the first `.ron` under `assets/scenes/` in sorted order, and
  the helper that does that moves into the engine — `SceneLoader::first_scene_in(dir)` in
  `crates/engine_core/src/scene_loader.rs`, the body of `editor_integration::find_first_scene`
  (`crates/editor_integration/src/constants.rs:37-39`, `common::vfs::list_dir_files`, so it
  reads through the VFS on the web), which is deleted with its re-export
  (`crates/editor_integration/src/lib.rs:28`) and its callers repointed —
  `crates/playground/src/web_entry.rs:18,254` and, found by the executor's deletion grep,
  `src/bin/editor.rs:15,59,90`, which sits outside `crates/` and is compiled by no listed
  gate but the editor-feature clippy now in the ground rules. The coin count is
  the template's own chrome: an absent `"coins"` key reads as 0, and a foreign export's scripts
  run without it. `Blackboard` and `ScriptValue` are not reachable from a game today
  (`engine_core::prelude` re-exports `ecs::{EntityId, World}` and the sprite components,
  `prelude.rs:53-57`, never the blackboard), so the prelude gains one line,
  `pub use ecs::{Blackboard, ScriptValue};`, rather than the template gaining an `ecs`
  dependency (gemini F3) — a game talks to `engine_core` only, and the template teaches that.
  And so that the browser check exports something the template can run, the template's own
  project joins the playground as its third bundled project: the invocation of record
  (`docs/WEB_PLAYGROUND.md:27-30`) gains `--project game-template="Game Template"=../games/game-template`
  and the bundle tree at `:8-24` its entry; the site's project select lists it from
  `projects.json` with no page change.
- **The native runner contract is written and short.** `docs/SCRIPTING.md:171-192` § Running
  scripts outside the editor is the block the template's `gameplay.rs` types verbatim
  (`ctx.scripts.reset` once, `early_update` → `physics.update` → `take_collision_events` →
  `update` per frame). `SceneLoader::load_from_file` reads through the VFS
  (`crates/engine_core/src/scene_loader.rs:87-90`), so the same load runs in the browser;
  `SceneLoader::instantiate(&data, world, assets)` is the call breakout makes
  (`../games/breakout/src/levels.rs:206`). The `Scripts([(script_id, source_path, params)])`
  binding is `crates/playground/assets/projects/pong/assets/scenes/pong.scene.ron:58-64`; the
  hooks a script may use are `docs/SCRIPTING.md:49-127` (collisions are `update`-only, and the
  view carries starts and stops, never ongoing contact). `HeadlessAssets` lives in
  `editor_integration` (`editor_integration::HeadlessAssets`, as `archive.rs:281` imports it),
  so the instantiate dry-run test is behind `#[cfg(feature = "editor")]`, which the games gate
  runs with.
- **What pong tracks, the template tracks:** `AGENTS.md` is a symlink to `CLAUDE.md`;
  `saves/*.json` are committed (`git -C ../games/pong ls-files saves`); `Cargo.lock` is ignored
  by `*.lock` in its `.gitignore` and not tracked — the template follows all three. The review
  tooling `scripts/check-skill-parity.sh:53-71` names is copied from the working set's
  canonical, not from pong, so the parity check reports the new repository clean on day one.
- **Batch 9 landed**, so the "if dropped" follow-up is struck; its own follow-ups (#107,
  insiculous_web#51) are already filed.

**Ask Jesse before creating `beinsiculous/game-template`** — the standing ruling. The remote
is outward-facing and is Jesse's command, after this batch's initial commit exists locally:

```sh
gh repo create beinsiculous/game-template --public --source games/game-template --push
gh repo edit beinsiculous/game-template --template
gh secret set ADD_TO_PROJECT_PAT -R beinsiculous/game-template
```

Nothing in this batch waits on it except the root repository's list entry (its commit) and
the `setup.sh` run that proves the seat.

### The template — `games/game-template/`, a fresh `git init -b main`, no remote, no commit

The smallest honest game in the `new-game` skill's layout, whose gameplay is a scene and two
scripts rather than Rust, so an export from the playground drops onto it and runs. Every
module the skill's "Required module layout" names exists and carries its one job; the Rust
total (every file under `src/`) stays under 1,000 lines — pong's equivalents sum to about
that without its power-ups and UI — and no file exceeds 300; the per-file gate is still 600. Pong is the
model for every file that is not named otherwise; where pong and the skill disagree, pong
wins (the skill's own rule).

- `Cargo.toml`: pong's, package `game_template`, `lib` + `bin`, `crate-type = ["cdylib", "rlib"]`,
  the `editor` feature, the two path dependencies, `glam`, `log`, the wasm-only
  `wasm-bindgen = "=0.2.126"` and `wasm-bindgen-futures`, `[profile.wasm-release]`. No other
  dependency.
- `src/main.rs`: pong's (`game_root!()`, the two save paths under `saves/`, the editor split).
  `src/lib.rs`: the module list, `game_config` (title "Game Template", 800×600, 60 fps),
  `impl Game`: `register_achievements`, `init` (the font at `fonts/font.ttf`, the scene via
  `spawning::spawn_scene`, then `ctx.scripts.reset(ctx.world, ctx.assets.base_path())`, then
  the grid), `update` (the state match: title, chaos select, achievements, playing, paused).
  `src/web_entry.rs`: pong's, with `ASSET_BASE = "/games/game-template/v1/assets"`,
  `EDITOR_ASSET_BASE = "/playground/game-template/v1/assets"`,
  `EDITOR_PREFS_SLOT = "beinsiculous.playground.game-template.editor_prefs"` and the
  `beinsiculous.games.game-template.*` save keys — the site never serves it, and the
  constants exist so `--kind games` and `--kind editor` both build and the version-bump
  checklist is in front of the reader.
- `src/constants.rs` (`WIN_W`, `WIN_H`, the scenes directory `scenes`, the blackboard key
  `"coins"`), `src/types.rs` (the game struct: `state`, `chaos_mode`, `physics:
  PhysicsSystem`, `grid`, `frame_count`, `coins_seen`; `Default`), `src/spawning.rs`
  (`spawn_scene(world, assets, base_path) -> Result<SceneInstance, SceneLoadError>`:
  `SceneLoader::first_scene_in(&base.join("scenes"))`, load, instantiate, nothing else — the
  scene is the spawner, and a foreign export's scene is found by the same call; plus
  `physics_for(instance.physics.as_ref()) -> PhysicsSystem`, the scene's declared gravity and
  scale built the way the project host builds them for Play, top-down when the scene declares
  none — review 31: a scene exported with gravity fell in the browser and floated on the
  desktop until `init` rebuilt the system from the block), `src/gameplay.rs` (the runner block
  from `docs/SCRIPTING.md:171-192`, then the coin count read from the
  `Blackboard` resource (the prelude's, after this batch) under `"coins"`, an absent key
  reading as 0, and on a change: the achievement, a burst at the coin), `src/menu.rs` (`MenuInput::read` + `navigate`; title → play, chaos
  select, achievements; Escape pauses, Escape resumes; `ctx.chaos_mode` mirrored),
  `src/drawing.rs` (`step_and_emit_grid` and the score/status text), `src/achievements.rs`
  (two: "First Coin" and "Ten Coins", `DISPLAY_SECTIONS`, the coverage test),
  `src/effects.rs` (one `burst(world, position, theme)` via `ParticleConfig`). Chaos modes:
  all four variants, the look from `ChaosTheme::for_mode`, the meaning per mode is the
  particle multiplier only — a per-game meaning is the README's first exercise, and the
  reason is stated there (menus and chaos as data need surfaces that are not built).
- `assets/scenes/main.scene.ron`: Camera; Background (`#white`, black, depth −100, 12×9);
  `Player` (`images/ball_8px.png`, kinematic, circle collider, `Scripts([player.rhai])`);
  four static walls (`#white`, a fixed grey the scene carries — nothing recolors a
  scene-spawned wall, and chaos reaches the grid and the particles only); `Coin`
  (`images/ball_8px.png`, gold, a `Dynamic` body with `gravity_scale: 0.0` carrying an
  `is_sensor: true` circle collider, `Scripts([coin.rhai])` — not the `Kinematic` body the
  section first said: rapier reports no intersection between two non-dynamic bodies, and the
  executor proved a kinematic sensor touched by the kinematic Player fires nothing;
  `a_player_sitting_on_the_coin_banks_it` pins the pair). Names are the strings the scripts and the tests use. The one
  image (`ball_8px.png`, the Player and the Coin tinted by the scene) and `fonts/font.ttf`
  are copied from pong's playground project and pong's `assets/`; no new art (the free-tier AI-asset rule, DEION_STYLE.md §6, is satisfied by copying what
  already ships).
- `assets/scripts/player.rhai` (`// @param speed: f32 = 300.0`, `// @param bound_x` and
  `bound_y`; `early_update`: `view.move_x(0)`/`move_y(0)` × speed × dt, clamped, then
  `cmd.set_kinematic_target(me, …)`) and `assets/scripts/coin.rhai` (no parameter — the four corners are a list in the script,
  because a count the inspector could set to zero would divide by it, review 31; the header
  grammar, for the reader, is `f32`, `i32`, `bool`, `str`, `vec2`, `entity`, `color`,
  `param_header.rs:44-141`; `update`: on `view.has_collision_started(me, "Player")`,
  `cmd.set_blackboard_int("coins", n + 1)` and `cmd.reset_body(me, corners[n % 4])` — the
  documented teleport for a body). Both pass `engine_core::scripting::check_source`.
- `project.ron`: `(slug: "game-template", title: "Game Template", bundle_version: "v1",
  content_hash: "", origin: imported)` in the pretty form `archive.rs:124` writes —
  lowercase, because `ProjectOrigin` is `#[serde(rename_all = "snake_case")]`
  (`projects.rs:7`), and `Imported` would be refused at import.
- Headless tests (`src/*_tests.rs` beside their modules, per pong): the scene parses and,
  behind `#[cfg(feature = "editor")]`, instantiates into `World::new()` with
  `HeadlessAssets::new()` yielding exactly the named entities the README lists; every `.rhai`
  under `assets/scripts/` passes `check_source`; `DISPLAY_SECTIONS` covers every registered
  achievement; a runner sim, also behind `#[cfg(feature = "editor")]` because it needs the instantiated
  scene — `reset` with the base `format!("{}/assets", env!("CARGO_MANIFEST_DIR"))` (the games
  gate runs `cargo test` from the engine root, so a relative base finds nothing;
  `crates/playground/tests/pong_rules.rs:15-24` is the shape), ten `early_update` and
  `update` frames with `InputHandler::new()` and `InputSettings::default()` — leaves `Player`
  inside the script's bounds and `runner.errors()` empty
  (`crates/engine_core/src/scripting/tests/rhai.rs:18-72` is the loop to mirror; its
  `test_inputs` helper is crate-private, so the two constructors are spelled out). No timing
  assertions.
- `README.md` (the exported README's URL points here): the title and one sentence; **Start
  here** — the layout the engine documents, in three commands
  (`git clone …/insiculous_2d`, `mkdir games && git clone <your template> games/<your-game>`,
  `cd games/<your-game> && cargo run`) and why the path dependency (the engine is meant to be
  edited beside the game); **Run** (`cargo run`, `cargo run --features editor`, the web build
  from the engine root: `scripts/build_wasm.sh ../games/<your-game> <slug> --kind games` and
  `--kind editor`); **The layout** — the export tree from `docs/WEB_PLAYGROUND.md:205-210`
  verbatim, then the crate's files beside it, then the two directions: an export drops on the
  clone (`rm -rf assets/scenes assets/scripts && unzip -o <slug>.zip -x README.md -d .`, `project.ron` and
  `assets/` replaced; the `rm` is load-bearing — `unzip -o` adds and does not empty, and the
  template's own `main.scene.ron` would sort ahead of the export's scene and be loaded, gemini's
  review-33 catch; the `-x` too — every export carries a `README.md` and `unzip -o` would
  replace the one you are reading) and the clone goes back to the browser (`zip -r <slug>.zip project.ron assets`,
  then Import project on `/playground/`); **Make it yours** (the rename list: package name,
  `game_config`'s title, the four constants in `web_entry.rs`, `project.ron`'s slug and
  title, the save file names; and the sentence that the menu, the chaos modes, the
  achievements and the coin count are the template's Rust — a foreign export's scripts run
  without them, and they are yours to replace) and the first exercise (a meaning per chaos
  mode); **Conventions**
  (a link to the engine's `.claude/skills/new-game/SKILL.md`); **Docs** (the two engine docs
  the export README links).
- `CLAUDE.md` (pong's shape: Commands, Architecture in four paragraphs, the path-dependency
  sentence), `AGENTS.md → CLAUDE.md`, `.gitignore` (pong's), `.github/workflows/add-to-project.yml`
  (pong's, unchanged), `/saves/` in the `.gitignore` (a template starts with no progress; the files
  exist only after a headed run), and the review tooling `scripts/check-skill-parity.sh:53-71` names,
  copied from `/home/jedi/projects/insiculous/` (the canonical), not from pong.

### The engine, the site, the working set

- `crates/playground/src/archive.rs:135`: the README string gains a third sentence after the
  two links — "Take it local: https://github.com/beinsiculous/game-template is a game whose
  gameplay is this archive; clone it (or press Use this template), unzip this archive over
  the clone, and `cargo run`." The line must stay one `format!` argument; the file stays
  under 600 lines (it is 327 today).
- `scripts/check_games.sh:23`: `GAMES=(pong snake breakout frogger asteroids space_invaders
  game-template)`; the header comment's "six games" (`:2`) and the closing echo's "All six
  games passed" (`:40`) both become "the games", with the list as the authority. Nothing else
  in the script changes.
- `crates/engine_core/src/scene_loader.rs`: `SceneLoader::first_scene_in(scenes_dir: &Path)
  -> Option<PathBuf>`, the body of `editor_integration::find_first_scene`, with one test (two
  `.ron` files in a temp directory, the sorted first comes back; a missing directory is
  `None`). `crates/editor_integration/src/constants.rs:37-39` and the re-export at
  `lib.rs:28` are deleted, `crates/playground/src/web_entry.rs:18,254` calls the engine's.
  `crates/engine_core/src/prelude.rs` gains `pub use ecs::{Blackboard, ScriptValue};` beside
  `:53`. Both are public items of `engine_core`, so the games gate runs for that reason too.
- `docs/WEB_PLAYGROUND.md:203`: the sentence names the repository
  (`https://github.com/beinsiculous/game-template`) and the two directions in one line each,
  the same words as the template's README so the two never disagree. § The bundle: the tree
  (`:8-24`) gains `game-template/`, the invocation of record (`:27-30`) its third `--project`
  line, and one sentence says the bundle now needs the template cloned beside the engine
  (`scripts/lib/repos.sh` puts it there).
- The bundle: after the engine gates, the invocation of record (`docs/WEB_PLAYGROUND.md:27-30`,
  three `--project` lines now) with `--sync ../insiculous_web/public`, its `wasm size:` line reported (9.69 MiB after batch 8;
  the gate warns past 20), every changed file under `public/playground/v1/` staged
  in the site repository. `git -C ../insiculous_web status --porcelain -- public/games` stays
  empty.
- The page: `../insiculous_web/src/pages/playground.astro:29-37` § Browser persistence gains
  one sentence at the end of its paragraph — an exported project drops into the game template
  (`<a href="https://github.com/beinsiculous/game-template">`) to keep building locally.
  Curly apostrophes as the page uses; one `<h1>`; no new ids.
- The working set (`/home/jedi/projects/insiculous`, branch `jesse`): `scripts/lib/repos.sh:51`
  gains `"games/game-template:game-template"` after `space_invaders` with a trailing comment
  ("the template repo, #49"), and `.idea/vcs.xml` the mapping in alphabetical order (after
  `frogger`). `bash -n scripts/lib/repos.sh` and `scripts/check-skill-parity.sh` (which now
  visits the template) are the gates the executor runs; `scripts/setup.sh` is the planner's,
  after the remote exists.

Files: `games/game-template/**` (new repository), `crates/playground/src/archive.rs`,
`crates/playground/src/web_entry.rs`, `crates/engine_core/src/scene_loader.rs`,
`crates/engine_core/src/prelude.rs`, `crates/editor_integration/src/constants.rs`,
`crates/editor_integration/src/lib.rs`, `src/bin/editor.rs`, `scripts/check_games.sh`,
`docs/WEB_PLAYGROUND.md`, the two guide lines (`crates/editor_integration/CLAUDE.md:15`,
`CLAUDE.md:335-336`); site `public/playground/v1/**`,
`src/pages/playground.astro`; root `scripts/lib/repos.sh`, `.idea/vcs.xml`. Four
repositories, branch `jesse` in the three that have one and `main` in the new one.

Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`scripts/check_wasm.sh` (the diff touches `crates/playground`), `scripts/check_games.sh --test`
(seven games now — the template's suite, clippy with and without the feature, and its two wasm
checks run here), the comment-tag grep over `crates src examples` and over
`../games/game-template/src`, the bundle rebuild with its size line, the template's two README commands run once
without `--sync` (`scripts/build_wasm.sh ../games/game-template game-template --kind games
--version v1` — its achievements export builds the game natively, `build_wasm.sh:291`, which
on Iroh needs `PKG_CONFIG_PATH=$HOME/.local/lib/pkgconfig` for libudev — and `--kind editor`;
their size lines reported; a documented command is a command that ran), `npm run verify` in
the site (Node 24 via nvm, as batch 8 found), `bash -n scripts/lib/repos.sh` and
`scripts/check-skill-parity.sh` at the root. **Jesse's browser check:** on staging, open
`/playground/`, pick Game Template in the project select, move the player, save, reload (the
move persisted), Export project, open the zip's `README.md`: three links, the third to the
template; then, on Iroh, clone the template into a scratch `games/` beside a fresh engine
clone, `rm -rf assets/scenes assets/scripts && unzip -o <slug>.zip -x README.md -d .` that export over it, confirm the template's
README is still the long one, `cargo run`: the moved scene runs natively with its scripts (a
coin collected bumps the count). Batch 10 is done when its four commits exist — the
template's initial commit on `main`, the engine's, the site's, and the root's list entry,
which waits for the remote — the remote exists, and `scripts/setup.sh` at the root runs
without a warning.

Deliberately left out: a menu, chaos meaning or achievement surface for scripts (filed in
batch 11); a locale table (pong's `Strings` path is a second system for one string);
`deion_assets/` and a Deion pivot section (the template is not a challenge game); publishing
the template's bundles on the site (it has no page, and needs none); a git-form dependency
(the ruling above, reversible).

## Batch 11 — docs close-out, the ledger, the board, the merge — LANDED 2026-09-06 (167e1d1 engine, e652201 site, aa2938d the template's README, c3d0dd3 plan; #48, #49 and insiculous_web#4 closed, follow-ups #108–#115 filed; the merge is the last step)

**Re-verified against the tree 2026-09-06 before the handoff** (written this morning, reviewed
in round 30, and batch 10 landed since). Corrections, each restated where it applies below:
`coordination/PROGRESS.md` appends its `## <date> — <title>` entries at the END, in date order
(`:320-414`, the Sprint 5 and 6 entries), whatever the header's "most recent at top" says of the
older bracket-line style — batch 11's eleven entries go after `:414`. The branch counts moved
with batch 10: the engine is 38 ahead of `dev`, the site 11, the working set 1 (2 once the
seat lands); the template has `main` only and no merge. insiculous_web#4's fix is acf3df4.
Two site docs join the docs bullet: `docs/roadmap.md:129-135` still says "Scripting follows"
and never names the template; `README.md:164-165` names the export layout and not where it
drops. The engine guide's "Next actionable" (`CLAUDE.md:231`) still names Editor Sprint 5, two
sprints stale, and is rewritten rather than appended to. One follow-up joins the list: a
scene with no physics block runs no physics in the playground and the top-down preset in the
template (`games/game-template/src/spawning.rs`, kimi's round-2 finding) — a documented
divergence that wants one behaviour.

- **Docs (executor):** `PROJECT_ROADMAP.md` § Web Playground (`:155-163`) becomes a
  shipped paragraph — the route, export/import, the template's URL, the six games' editor
  pages, the ten batches' dates, "follow-ups on the board" — and the Phase Map's sentence at
  `:81-82` ("**Web Playground** is #48/#49") becomes "shipped Sep 2026; `docs/WEB_PLAYGROUND.md`";
  § Scripting (`:165-183`) is already current (batches 7 and 9 wrote it) and gains only the
  template's name in its last sentence. `CLAUDE.md` (the engine's, which names the
  playground nowhere today): the Editor and Editor Integration bullets (`:222-223`) each gain
  a clause — the editor runs in the browser at `/playground/`; `editor_integration` hosts data
  projects — and the Current Priority paragraph (`:231`) gains one sentence before "Next
  actionable": the Web Playground SHIPPED Sep 6 2026 (#48, #49; the ten batches; the docs).
  `README.md` § Visual Editor (`:97-107`) gains "and runs in the browser at
  beinsiculous.com/playground/, where projects export as zip and drop onto the game
  template", § Project Status (`:451-460`) a **Web** line. `log_archive.md`: a
  "## Web Playground ☑ Sep 6 2026" entry after the retirement entry, with the lessons —
  the ones the rebuttals paid for: the batch-2 miss (a correction filed outside the section
  the executor reads is missed), the one-report-one-path rule (batch 4's three copies), the
  FIFO with no request ids (rebuttal 1), the epoch that a failed import must restore
  (batch 5), the per-game editor bundle over scripts (the game-run ruling), the executor
  substitution in batch 9. `coordination/PROGRESS.md`: one entry per batch, 0 through 10,
  appended after the last entry (`:414`) in date order in the `## 2026-09-0N — <title>` shape
  (`:388` is the shape), each naming the commit(s) from this plan's section headings and the
  reviews from the ledger — this plan's § Context paragraph is the source — plus Jesse's
  browser-check lines for batches 4, 5, 8, 9 and 10 as Jesse reports them (a line per check,
  every one naming the reload). The engine guide's "Next actionable" sentence at `:231`
  names Editor Sprint 5 and is two sprints stale: it is rewritten to point at the board, not
  appended to. Site (`../insiculous_web`, gate `npm run verify`): `docs/roadmap.md:129-135`
  drops "Scripting follows" for the shipped truth — scripts edited on the page, projects
  exported as zip and dropped onto the game template — and `README.md:164-165` gains the
  template's URL as where an export drops.
- **The ledger (planner):** `coordination/web-playground/reviewer-comparison.md` closes the
  way the cleanup's did (`coordination/cleanup-2026-09/reviewer-comparison.md`, its last two
  paragraphs): totals per reviewer over this effort, the overlap, the unique catches, wall
  time, and the default-reviewer recommendation as a number.
- **The board (planner):** close insiculous_2d#48 and #49 with a comment naming the commits
  (#48: batches 1–4, and 6–9 as the milestone the plan extended — § Context; #49: batches 5
  and 10), and insiculous_web#4 (batch 0's fix, the
  `requestDevice` await in `GameEmbed.astro:92`, commit acf3df4, which shipped without the
  `fixes` trailer).
  File the follow-ups with `/file-issue`: the other five games as data projects; pong's
  menus, power-ups, chaos and achievements as data (needs menu and achievement script
  surfaces — the template's README exercise waits on the same); the eight `Behavior`s as
  built-in scripts; `#[derive(Script)]` once `ParamSpec` settles (re-decide #83 then); a
  per-file delete verb (`playground_delete_file` + a store delete keyed `[project, path]` —
  today a file leaves a project only by re-import); contact points and normals in
  `ScriptView` for a game whose rules read them (pong's never did); a criterion bench for
  view-building + script dispatch at 200 named entities × 30 instances over the Rhai path
  (a timing assertion is not a `cargo test`); a scene with no physics block, which runs no
  physics in the playground and the top-down preset in the template — one behaviour, decided.
- **The merge (planner, then Jesse):** `jesse → dev` in all nine repositories that carry
  both — the engine (38 ahead of `dev` after batch 10), the site (11), the six games (2–4
  each), the working set (1, 2 once the seat lands) — every one a fast-forward today; the
  template has `main` only. Jesse pushes, and `dev → main` stays Jesse's call.

Gates: the docs batch has no Rust and no bundle — the comment-tag grep, `cargo test
--workspace` unchanged, `npm run verify` for the site's two docs, and a read of every changed
paragraph against the tree (a guide that describes a thing wrongly is a defect). Files:
`PROJECT_ROADMAP.md`, `CLAUDE.md`, `README.md`, `log_archive.md`, `coordination/PROGRESS.md`
in the engine; `docs/roadmap.md`, `README.md` in the site. The board and merge steps have no
executor.

## Verification (end to end)

- Engine: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  `scripts/check_wasm.sh`, `scripts/check_games.sh --test`, the comment-tag grep — all
  clean on the final `jesse`.
- Bundle: `scripts/build_wasm.sh crates/playground playground --kind playground
  --version v1 --project examples=... --project pong=... --sync ../insiculous_web/public`
  under 20 MiB; with batch 9, the six `scripts/build_wasm.sh ../games/<g> <slug> --kind
  editor --version v1 --sync ../insiculous_web/public` builds, each under 20 MiB.
- Site: `npm run verify` green; staging deploy; Jesse's browser checks for batches 4, 5,
  8 (and 9) recorded in `coordination/PROGRESS.md` — every one includes a reload, so
  persistence is proven, not assumed.
- Board: insiculous_web#4, #48, #49 closed by commits; Sprint 6's five closed in batch 0;
  every follow-up filed.
