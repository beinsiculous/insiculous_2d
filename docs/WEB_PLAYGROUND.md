# Web Playground — the editor in the browser

The Web Playground (`/playground/`) runs the editor on wasm32 + WebGPU over a project kept
in the browser's IndexedDB. The crate is `crates/playground`; the engine side of the
contract is `docs/EDITOR_COMMAND_API.md` (Stage D is the bridge below) and
`docs/WEB_SAVES.md` (the preferences key).

## The bundle

The bundle layout is:
```
playground/<version>/
├── game.js
├── game_bg.wasm
└── assets/
    ├── manifest.json
    ├── projects.json
    └── projects/
        ├── examples/
        │   └── assets/
        ├── pong/
        │   └── assets/
        └── game-template/
            └── assets/
```

Invocation of record, run from the engine root:
```sh
scripts/build_wasm.sh crates/playground playground --kind playground --version v2 \
    --project examples=Examples=examples \
    --project pong=Pong=crates/playground/assets/projects/pong \
    --project game-template="Game Template"=../games/game-template \
    --sync ../insiculous_web/public
```

The third project is the template repository's own tree, so the bundle needs it cloned
beside the engine — `scripts/lib/repos.sh` in the working set puts it at `../games/game-template`.

`assets/projects.json` is the bundled project manifest list (a JSON array of `ProjectManifest`).
Each entry carries:
```json
{
  "slug": "examples",
  "title": "Examples",
  "bundle_version": "v2",
  "content_hash": "<sha256-hex>",
  "origin": "bundled"
}
```
`content_hash` is computed as sha256 (hex) over the sorted relative file list and bytes of
the project's `<dir>/assets`.

## The bundle contract

`ASSET_BASE = "/playground/v2/assets"` and `BUNDLE_VERSION = "v2"` (`web_entry.rs`). The
version token appears in five places, listed in that file's header: the deployed directory
`public/playground/v2/`, the asset URLs the engine fetches, `projects.json`'s
`bundle_version`, the build script's output directory, and every `StoredFile`'s
`bundle_version`. Bumping it is a coordinated change across all five.

A project's root is computed, never stored: `{ASSET_BASE}/projects/<slug>` on the web,
`<dir>/projects/<slug>` natively; its asset base is `{root}/assets`. Every file the engine
reads or writes is keyed by that base-joined string (`common::vfs`'s canonical key), e.g.
`/playground/v2/assets/projects/examples/assets/scenes/behavior_demo.scene.ron`. A relative
key never resolves; relative paths given to the editor or the API are joined to the open
project's asset base first.

**One embed per page, and one runtime: a page is the editor or, with `?mode=preview`, the
preview.** The VFS, the store, the bridge channels and the persistence chains are
module-level singletons; a second `playground_*` module on the same page would share
them. winit allows one event loop per page, which is the other half of the same rule. The page provides two elements by id: `game-loading` (boot status text, from
`engine_core::web::set_boot_status`) and `playground-banner` (persistence warnings, written
by `persist::set_dom_banner`; an absent element is a silent no-op). The entry dispatches the
`playground-ready` event on `window` immediately after the bridge is set up, signaling that
bridge channels are ready and manifests are loaded.

## The game bundles

The six Rust games each ship a SECOND bundle: the same game crate compiled with its
`editor` feature, so the game runs inside the scene editor in the browser. It is a game
bundle, not a playground one — the game's own `assets/` tree, no `projects.json`, no
project store, no `achievements.json`.

Layout, deployed and built:

```
public/playground/<slug>/<version>/{game.js, game_bg.wasm, assets/...}
```

`EDITOR_ASSET_BASE = "/playground/<slug>/<version>/assets"` in the game's `web_entry.rs` is
the compiled-in base. Its version is a FOUR-place contract — that constant, the build
script's output directory, the site's `src/content/games/<slug>.md` `editor:` path, and the
deployed directory — and is INDEPENDENT of the game's own `wasm:` version: the two bundles
deploy separately.

The invocations of record, from the engine root:

```sh
scripts/build_wasm.sh ../games/pong pong --kind editor --version v2 --sync ../insiculous_web/public
scripts/build_wasm.sh ../games/snake snake --kind editor --version v2 --sync ../insiculous_web/public
scripts/build_wasm.sh ../games/breakout breakout --kind editor --version v2 --sync ../insiculous_web/public
scripts/build_wasm.sh ../games/frogger frogger --kind editor --version v2 --sync ../insiculous_web/public
scripts/build_wasm.sh ../games/asteroids asteroids --kind editor --version v2 --sync ../insiculous_web/public
scripts/build_wasm.sh ../games/space_invaders invaders --kind editor --version v2 --sync ../insiculous_web/public
```

(The site slug for `space_invaders` is `invaders`, as it is for its game bundle.)

**What persists: only the editor's own preferences.** The session passes no save paths, so
an editor session writes none of the game's `beinsiculous.games.<slug>.*` keys — no
achievement the site's board would show, no high score, no rebound key — and takes the
engine's default input bindings. Scene edits live in the in-memory VFS: a save inside the
editor has nowhere to go, because a game bundle carries no project store, and a reload
brings the game's own layout back. The one thing that survives is the editor's camera and
panel layout, in the per-game preferences key (`docs/WEB_SAVES.md` § Keys).

## The store

Database `beinsiculous.playground`, version 1, two object stores:

| store | key | record |
|---|---|---|
| `files` | `[project, path]` | `StoredFile { project, path (project-relative), bytes, revision, bundle_version }` |
| `projects` | `slug` | `ProjectManifest { slug, title, bundle_version, content_hash, origin: bundled \| saved \| imported }` |

`ProjectStore` (`store.rs`) has three implementations: `IndexedDbStore` (the web),
`MemoryStore` (the fallback when IndexedDB will not open — private browsing, a sandboxed
frame — with the banner "Storage unavailable"), and `DirectoryStore` (native, the test
double; a lock file per project keeps its compare-and-swap honest).

**`put` is a compare-and-swap.** Inside one `readwrite` transaction it reads the stored
revision, refuses with `StaleRevision { stored, base }` unless it equals the caller's
`base_revision` (an absent record accepts only 0), else writes `base_revision + 1` and
returns it. A get-then-put across two transactions is forbidden: another tab can commit
between them, and the second write would silently overwrite it. `put` also upserts the
slug's manifest when the store has none (origin `saved`). `replace_project` and
`remove_project` are one transaction each across both stores, the manifest written last as
the commit marker. `sweep_orphans(bundled)` at boot removes files whose slug has neither a
manifest nor a bundled entry (an interrupted import); the open project's own
manifest-less files are removed at boot too, and the bundled files load instead.

`IdbTransactionFuture` (`store/idb_transaction.rs`) turns a transaction's
`complete`/`error`/`abort` events into a future. Every dependent request is issued
synchronously inside the previous request's `onsuccess` — never after an `await`, when the
transaction is already inactive.

## Persistence: one chain per path

`persist::Chains` is target-agnostic and owns no executor. Natively the tests hand-poll its
futures; on wasm the layer around it spawns each started put and feeds the result back.
The vfs write observer (`common::vfs::set_write_observer`, wasm-only) fires once per
`vfs::write`; the boot-phase `vfs::insert` never fires it, so seeding is not a save.

Path states: **idle** · **in flight** (a put running) · **queued** (a put running and newer
bytes waiting; only the newest bytes are kept) · **stranded** (the last put failed with
`Backend`/`Unavailable`; the newest bytes are held and the banner "not saved to this
browser — export your project" stands) · **conflicted** (`StaleRevision`: another tab saved
first; never retried; the banner names the file and says to export before reloading). A
write to a conflicted path issues no put — `MemFs` keeps the bytes for the export.

Two predicates: `is_pending()` is "any path not idle" and feeds the editor's dirty title
(through `EditorRunOptions.persist_pending`), `playground_is_dirty` and the `beforeunload`
warning; `has_active()` is "in flight or queued" and is what a drain awaits — a stranded or
conflicted path must never block a switch. `drain_then_epoch()` bumps the write epoch, THEN
awaits every active chain, bounded at 5 s; a write during a drain is refused ("project is
being replaced — save again after the reload") and the message goes on the banner.

Listeners: `visibilitychange`→hidden re-issues stranded paths; `beforeunload` sets the
warning while anything is pending. There is no `pagehide` handler: a queued put cannot be
issued before the in-flight one resolves without breaking the CAS, and IndexedDB commits
the in-flight transaction on its own.

## The bridge (`bridge.rs`, Stage D)

All paths are project-relative (`assets/scripts/ball.rhai`); the bridge joins the open
project's root and refuses an empty path, a leading `/` or `\`, and any `..` component.
The command channel is a 1024-line FIFO; responses come back in order.

| export | shape | notes |
|---|---|---|
| `playground_dispatch(line)` | `→ bool` | `false` on a full queue or a whitespace-only line (never dropped, never enqueued) |
| `playground_poll_responses()` | `→ string[]` | drains every response line so far |
| `playground_is_dirty()` | `→ bool` | command-history dirty OR `is_pending()`; the page confirms before any switch, import or reset |
| `playground_write_file(path, text)` | `→ Result` | through `vfs::write_string`; a `.rhai` write runs the `source_check` hook first, and a refused script is not written |
| `playground_read_file(path)` | `→ Result<string>` | |
| `playground_list_files()` | `→ Result<string[]>` | project-relative |
| `playground_list_projects()` | `→ ProjectEntry[]` | bundled merged with stored; stored wins on a slug clash; `has_stored_files` gates Reset. The stored list is a boot snapshot: an edit's first put upserts a manifest the list shows after the next reload |
| `playground_open_project(slug)` | `→ Promise` | drains, then resolves; the PAGE sets `?project=<slug>` and reloads |
| `playground_reset_project(slug)` | `→ Promise` | drains, `remove_project`, resolves; the page reloads and the bundled files come back |
| `playground_export_zip()` | `→ Promise<Uint8Array>` | the open project's `assets/**` plus `project.ron` and a README as zip bytes, with the **live** scene in place of the saved one — the visitor exports the work they are looking at. A snapshot already in flight is joined, not refused, so Export a moment after Play ↗ works. Refused during Play or Pause, like `playground_snapshot`: the live world is the simulation then, and the saved scene would drop the unsaved edits made before Play — stop first. Refuses an archive over 64 MiB (the importer's archive cap; its decompressed-bytes cap is not mirrored, so a highly compressible project over 64 MiB unpacked exports but does not re-import) |
| `playground_snapshot(generation)` | `→ Promise<{ sceneEntry, bytes }>` | the project archive with the active scene's entry replaced by the live world as RON, every other file kept. Answered on the editor's next frame, 5 s cap, then rejected with "the editor did not answer — is its tab visible?". Refused during Play or Pause, and while another generation is pending. **Reserves the preview as it files**, and clears the reservation if it rejects |
| `playground_set_preview_open(open)` | `→ void` | while true, in-editor Play is refused with "A preview window is open — close it to Play here". The page clears it on `preview-failed`, `preview-closed`, or a window it finds closed — never on silence |
| `playground_import_zip(bytes)` | `→ Promise<string>` | validates, drains, replaces the project in the store, resolves with the slug; the PAGE then sets `?project=<slug>` and reloads — REQUIRED, same slug or not, as for switch and reset: the drain leaves writes refused until the reload |
| `playground_read_file_bytes(path)` | `→ Result<Uint8Array>` | project-relative binary read through `vfs::read` |
| `playground_conflicted_paths()` | `→ string[]` | sorted project-relative paths currently in conflicted state |
| `playground_script_errors()` | `→ string[]` | runtime errors recorded by `ScriptRunner` during the current Play session |
| `playground_save_state()` | `→ string` | `{"state": "unsaved" \| "saving" \| "saved" \| "failed", "reason": ""}`: a conflicted or stranded path is `failed` with `reason` naming the first such path in sorted order — **conflicted outranks stranded**, a conflict being terminal while a stranded path retries on `visibilitychange` — an in-flight or queued put is `saving`, a dirty command history is `unsaved`, and neither is `saved` |

The engine cannot swap a running project; every switch is a page reload with the query
string naming the slug. An unknown `?project=` redirects to the first bundled project.

## The preview window

`?mode=preview` on the page URL boots the game-only runtime instead of the editor: no
store, no persistence chains, no write observer, no bridge, no asset preload, and every
save path of its `GameConfig` left `None`, so the preview writes no storage key. It boots
to "Waiting for the editor's snapshot…" and does nothing until the editor's page hands it
an archive.

| export | shape | notes |
|---|---|---|
| `playground_load_preview(bytes, sceneEntry)` | `→ Result` | unpacks the archive through the importer's full validation, keys every file under the project root, and starts the game on `sceneEntry`. Refused when one is already loaded — one runtime per page. `Ok` means **scheduled**, not running |
| `playground_preview_state()` | `→ string` | `"booting"` until the first frame has actually stepped, then `"running"`; `"failed: <text>"` from a scene that would not load or the renderer's boot-status failure |
| `playground_preview_pause()` | `→ bool` | toggles; returns the new paused state |
| `playground_preview_restart()` | `→ Result` | reloads the scene from the top and clears the pause; `Err` when nothing is loaded |

The preview loads the scene entry it is handed and never infers one from the archive.
Stop is the page closing the window; the `pagehide` guard already latches the loop for
good. A hidden document gets no animation frames, so both pages install
`engine_core::web::install_hidden_frame_pump`: while a tab is in the background a 100 ms
timer drives its frames through a user event instead. The editor's page needs it most —
opening the preview window hides the editor's tab before the frame that answers the
snapshot — and on a page that is already hidden the pump starts as soon as there is a loop to
wake.

The same timer drives an **idle** editor. Once half a second passes with no input event
and no mouse button held, an editor in Editing or Paused stops re-arming animation frames
and the pump takes over at its 100 ms cadence; the countdown is cleared by any input, and
the first input after idling wakes the ordinary loop on that frame rather than at the
pump's next tick. Only an editor host asks for this (`GameContext::set_idle_throttle_ok`),
and only while no simulation is running — the engine's own time freeze outside Playing
already means nothing is animating — so a game window and a play session keep their full
frame rate. A `resize` or a `RedrawRequested` is window management, not input, and never
wakes it. The numbers it moves are in § Budget.

## Preferences

Editor preferences (camera, grid, panel layout) persist through `save_store` under the
localStorage key `beinsiculous.playground.editor_prefs`, the same JSON document the native
`editor_prefs.json` holds. The editor writes on a **settle rule**: once the preferences have
been unchanged for 0.5 s of frame time (a pan writes once, after the hand lifts), plus an
immediate write on the Play transition (with the editing camera, before the game camera
takes the viewport), after Stop, and on exit. Nothing is written during Play or Pause.

## Export and import

`playground_export_zip()` is a Promise, and the scene it carries is the live world, not
the file on disk: the acceptance test is "exports their work", and the saved scene would
drop the very edit the visitor previewed. For the same reason it is refused during Play or
Pause — stop first. Projects export and import as standard zip
archives (`<slug>.zip`). This is the layout
[the template repo](https://github.com/beinsiculous/game-template) conforms to, and it goes
both ways: an export drops on a clone of the template with
`rm -rf assets/scenes assets/scripts && unzip -o <slug>.zip -x README.md -d .`
(the `rm` is load-bearing — the template's own `main.scene.ron` would otherwise sort ahead of the
export's scene and be the one loaded; the `-x` too — every export carries a `README.md` and
`unzip -o` would replace the clone's own), and a clone goes back to the browser with `zip -r <slug>.zip project.ron assets`
followed by Import project on `/playground/`.

```
<slug>.zip
├── project.ron     # the ProjectManifest
├── README.md       # generated: the title and the docs URL
└── assets/         # scenes, .sheet.ron sidecars, scripts, images, sounds, fonts, locales
```

### Validation

On import, the archive is validated in order before touching any persistence store:
1. Archive file size is capped at 64 MiB and refused before parsing.
2. Entry names are normalized: `\` becomes `/` and a leading `./` is stripped.
3. Directory entries are skipped.
4. Traversal is refused (any `..` component, leading `/` or `\`, or drive prefix).
5. The name is made canonical — `.` segments and repeated `/` dropped — so `assets/./x` is stored as `assets/x`, the key a scene uses.
6. `README.md` at the root is skipped.
7. A canonical name that repeats is refused.
8. Every other entry must be `project.ron` or lie under `assets/`; any other root entry is refused naming the entry.
9. Cumulative decompressed bytes across entries are capped at 64 MiB as entries are read (`Read::take` at the remaining budget; exceeding the budget refuses).
10. `project.ron` is required, parsed as `ProjectManifest`, and its `slug` must pass slug validation (`^[a-z0-9_-]{1,32}$`).
11. Every `*.sheet.ron` runs through `engine_core::sheet_file::parse_sheet_file`.
12. Every `*.scene.ron` runs through `SceneLoader::parse` followed by dry-run instantiation via `SceneLoader::instantiate(&data, &mut World::new(), &mut HeadlessAssets::new())`.
13. Every `*.rhai` runs through `engine_core::scripting::check_source` for param header and Rhai syntax validation.

### Failure contract

A refused archive touches nothing; a failed `replace_project` restores the epoch and the current project keeps saving — a save attempted during its drain window was refused, as on switch and reset, and is re-issued by saving again.

## Acceptance

Run these by hand in a real browser against a staged deploy before each playground
release, and record the result in the pull request. None of them is covered by an
automated run: each needs a real keyboard, pointer and audio device, and the editor
inside a page is where the engine's input path can break in ways no headless test sees.
`/playground/` is also the only surface the engine shares with a page that has its own
shortcuts, its own scroll and its own zoom.

| check | what to do, and what passing looks like |
|---|---|
| Browser shortcuts | With the canvas focused, press Ctrl/Cmd+S, Ctrl/Cmd+O, F5 and Ctrl/Cmd+W; then repeat with the focus on the page instead. Record which ones the page swallows — winit suppresses the browser's own default for the keys the canvas handles, and the tab-closing ones cannot be suppressed at all. Either way no work is lost: a reload or a close with anything pending asks first ("Changes you made may not be saved") |
| Focus into the canvas | Click the canvas. The editor's shortcuts work, and typed characters reach the focused field, drawn with the focus ring around it |
| Focus out of the canvas | Click the command input, the project switcher, or any chrome outside the canvas, then press the editor's shortcuts. The keystrokes go to that element and nothing reaches the game or the scene |
| Tab traversal | Click one inspector text field, then Tab and Shift-Tab. Focus moves exactly one field per press, wraps at the last field to the first (and the reverse for Shift-Tab), and the focus ring follows it |
| Audio activation | Load a project with sound and wait: the page starts silent. The first click or key press enables audio, and a preview window's sounds play from then on |
| Resize | Drag the browser window and open and close a panel. The canvas re-lays out, and the pointer still maps to the same world point — click a sprite where it is drawn, not where it was |
| Zoom | Browser page zoom (Ctrl/Cmd with `+`/`-`, and Ctrl/Cmd with the wheel), then the editor's own wheel zoom over the viewport. The canvas stays aligned under the pointer, and a gentle trackpad scroll does not compound into a hard zoom |
| Typing stays in its field | With a script open in Scripts, and with an inspector text field focused, type Delete, Ctrl/Cmd+Z, letters and spaces. The scene does not change: no entity is deleted, no tool shortcut fires, no undo runs |

## Budget

Four numbers decide whether "lightweight" means anything, all measured on an ordinary
laptop with a cold cache. Only the first is a build artifact and can be read off a
rebuild; the rest need Jesse's browser, and the rows say so until he has run them.
Each is recorded with the measurement it came from so a later regression is visible
rather than felt.

| number | what it covers | recorded |
|---|---|---|
| download size | `game_bg.wasm` of the playground bundle — the `wasm size:` line `scripts/build_wasm.sh` echoes. The only one of the four with a hard gate: `--kind playground` exits non-zero past `PLAYGROUND_SIZE_BUDGET_MIB` at the top of that script | 10.61 MiB (v2, 2026-09-12) |
| time to an editable scene | navigation to the first keystroke an inspector field accepts | pending Jesse's measurement |
| idle CPU | an editor tab left alone in Editing, after the idle throttle has engaged — the number the throttle exists to move | pending Jesse's measurement |
| memory with a preview open | the editor tab plus a `?mode=preview` window on the same project, as one browser-task-manager reading | pending Jesse's measurement |

The idle-CPU number is the one to read twice. An editor with nothing happening stops
asking for animation frames once half a second passes without input, and wakes on the
frame the next input arrives; a reading taken during a drag, a resize or a play session
is a different number from one taken on a still window, and only the still one is the
measurement this row is for.
