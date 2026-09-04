# Web Playground — the editor in the browser

The Web Playground (`/playground/`) runs the editor on wasm32 + WebGPU over a project kept
in the browser's IndexedDB. The crate is `crates/playground`; the engine side of the
contract is `docs/EDITOR_COMMAND_API.md` (Stage D is the bridge below) and
`docs/WEB_SAVES.md` (the preferences key). Batch 4 of `coordination/web-playground/plan.md`
adds the build script and the page; this file describes what the crate exports today.

## The bundle contract

`ASSET_BASE = "/playground/v1/assets"` and `BUNDLE_VERSION = "v1"` (`web_entry.rs`). The
version token appears in five places, listed in that file's header: the deployed directory
`public/playground/v1/`, the asset URLs the engine fetches, `projects.json`'s
`bundle_version`, the build script's output directory, and every `StoredFile`'s
`bundle_version`. Bumping it is a coordinated change across all five.

`{ASSET_BASE}/projects.json` is the bundled project list (a JSON array of `ProjectManifest`).
A project's root is computed, never stored: `{ASSET_BASE}/projects/<slug>` on the web,
`<dir>/projects/<slug>` natively; its asset base is `{root}/assets`. Every file the engine
reads or writes is keyed by that base-joined string (`common::vfs`'s canonical key), e.g.
`/playground/v1/assets/projects/examples/assets/scenes/behavior_demo.scene.ron`. A relative
key never resolves; relative paths given to the editor or the API are joined to the open
project's asset base first.

**One embed per page.** The VFS, the store, the bridge channels and the persistence chains
are module-level singletons; a second `playground_*` module on the same page would share
them. The page provides two elements by id: `game-loading` (boot status text, from
`engine_core::web::set_boot_status`) and `playground-banner` (persistence warnings, written
by `persist::set_dom_banner`; an absent element is a silent no-op).

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
| `playground_script_errors()` | `→ string[]` | empty until batch 7 fills the bridge's `Hooks` |

The engine cannot swap a running project; every switch is a page reload with the query
string naming the slug. An unknown `?project=` redirects to the first bundled project.

## Preferences

Editor preferences (camera, grid, panel layout) persist through `save_store` under the
localStorage key `beinsiculous.playground.editor_prefs`, the same JSON document the native
`editor_prefs.json` holds. The editor writes on a **settle rule**: once the preferences have
been unchanged for 0.5 s of frame time (a pan writes once, after the hand lifts), plus an
immediate write on the Play transition (with the editing camera, before the game camera
takes the viewport), after Stop, and on exit. Nothing is written during Play or Pause.

## Export and import

Batch 5. The zip layout and the import validation are specified there, not here.
