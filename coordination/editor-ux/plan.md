# Editor UX — the plan

**2026-11-11, Jesse's ruling: batch 5 is settled, as a navigation and a lock.** Ruling (h′)
replaces (h): where the round summaries below describe the window as a two-phase transfer —
the owner record, the invitation, the freeze and retire, *Take over*, *Bring it back* — that
design is withdrawn after six rounds under `review/editor-ux-window/` (rebuttal-1 to -6
there). § Design E and the batch 5 paragraph are the text of record.

**2026-11-11, Jesse's ruling: batch 6 migrates nothing.** Where the round summaries below say
batch 6 migrates a visitor's stored scenes — the marker, puts → references → tombstones — that
design is withdrawn; a stored pre-convention project keeps loading through five boot aliases
and its store is never written. The batch 6 section is the text of record.

**v6 — 2026-11-03, after round 4 — SETTLED for batches 1–4b and 6–8; § E / batch 5 continues
under its own review subject, `review/editor-ux-window/`.** Jesse's word (2026-11-03): if round 4
still found criticals in the hand-off, the other seven batches settle, the plan commits and the
sprint is filed, and batch 5 stays under review. Round 4 (`review/editor-ux/rebuttal-4.md`: kimi
7, gemini 8, codex 9 — twenty-four findings, 20 accepted, 1 put to Jesse as ruling (j), 2 rebutted
as policy, 1 on the tree) found four. What v6 closes outside § E: the native rename has no copy
fallback (refused without hard links) and a directory rename checks its target; the two-syscall
residual on hard-link filesystems is ruling (j); `write_new` is exclusivity over the *live* file
with the path's current revision; song families include `.midi`; the reference scan confirms
*before* a rename; the `LICENSE` stanza is preflighted; the Scripts panel is coordinated *before*
a mutation; a tab strip overflows into a `»` menu; the scissor snaps; the migration re-checks a
record's content before tombstoning it, aliases a bundled-only `images/` read while incomplete,
and keeps a `skipped` list. In § E, recorded for the next subject: the game surface's confirm is
its retire step at every entry; the parked page never auto-returns; the admission rule is
`token:N+1` both ways; a failed post-grant load is a recovery card; and the freeze needs a seam in
`editor_integration` (keys are `WindowEvent` callbacks), so **ruling (e)'s batch-5 exception is
withdrawn** and batch 5 queues like every other batch.

**v5 — 2026-11-03, after round 3** (`review/editor-ux/rebuttal-3.md`: kimi 9, gemini 8, codex 14 —
thirty-one findings, 28 accepted, 2 rebutted as policy, 1 on the tree). **The hand-off, once more**:
the destination queues for the Web Lock *before* it posts `ready`, so the grant — not `retired` —
is what lets it load and go live, and a third tab cannot slip in between release and acquisition;
the source retires through one privileged `playground_retire()` that settles the DOM editors,
saves every document through the internal path and drains persistence (the Save button's
`dispatch('save')` only queues for a later frame, and the frozen bridge would have refused it);
the owner record carries the generation; an invitation is consumed once; `closing` is a hint
checked against `locks.query()`; *Focus it* pings before it opens; the game page always asks;
import and reset act on the owned slug only. **The asset browser**: the native rename is two
algorithms (files by hard link with an inode check and a `create_new` copy fallback, directories
by the OS's own no-replace rename); creation is an exclusive create; a bundled-only delete writes a
tombstone (accepted in round 2, written now); the Scripts dock panel follows create, rename and
delete; a dirty open document asks before a rename; *New Song* becomes *New Song sidecar* on a
`.mid` (File ▸ New Song goes with External Tools batch 7); audio renames carry their `LICENSE`
stanza. **The migration** keeps progress in a manifest marker, runs puts → references → tombstones,
and leaves a colliding family unmigrated. **The dock**: a budget-collapsed group opens as an
overlay; a panel missing from a saved layout joins its default group; a submenu that fits nowhere
clamps to the edge over its parent.

**v4 — 2026-11-03, after round 2** (`review/editor-ux/rebuttal-2.md`: kimi 11, gemini 11, codex
14, the planner 2 — thirty-eight findings, 35 accepted, 1 partial, 2 rebutted). **The hand-off is
rewritten**: the destination boots *without* the project and loads only after the source has
retired (all three reviewers caught v3 going live on stale bytes); an invitation in the URL lets
exactly the invited boot past the owner card; a Web Lock, not the record, decides ownership; the
key carries the surface (`playground:pong` vs `game:pong`); the freeze is a mutation gate (empty
input frame, bridge refusal, DOM controls disabled), not a blocking rectangle; the protocol is a
state machine with a commit point whose continuations re-check after every await; the window
announces `closing` on `pagehide` and the parked page never polls by name; the game page's dialog
says the edits *will be lost*. **The scale** gains scalar lengths, leaves wheel notches alone, and
hands one points-sized window to every chrome consumer (the confirm dialogs and the status bar
were still physical). **The dock's** budget correction is effective, never persisted; the
all-hidden migration has a fallback; an inactive tab is not a keyboard host. **The asset
browser** selects by tile identity, renames without replacing through `hard_link`, rolls a family
rename back and shows an orphan companion, exports `.keep`, and the convention admits the
engine's generated names. **Batch 6's** migration moves the stored `images/` family and its
tombstones and resumes per file.

**v3 — 2026-11-03, Jesse's rulings folded in.** Jesse accepted every proposal of round 1's
report in one line ("I like all 6 of your proposals keep fold them in"): rulings (a)–(i) are
his, not proposed; the dirty game page gets the warning dialog, not a snapshot transfer (codex
F2, partial); ordering is the web contract and there is no journal (codex F4, partial); batch 4b
stays, gated on 4a; batch 7 (tab drag) runs in this effort after batch 3 has settled; `images/`
becomes `sprites/`; External Tools batch 7 goes first and this effort's batch 1 follows it, with
only batch 5 free to interleave. Round 2 reviews this version.

**v2 — 2026-11-03, after round 1** (`review/editor-ux/rebuttal-1.md`: kimi 15, gemini 8, codex 13,
the planner 5 — forty-one findings, every one accepted, two of them partially and put to Jesse:
codex F2's snapshot transfer for a dirty game page, codex F4's journal). What moved: **the web
hand-off is an acknowledged two-phase transfer** over a `BroadcastChannel`, with an owner record
per project, a synchronous window handle, a flush export, a `beforeunload` warning and
reacquisition by name; the Playground refuses on a memory store and a game page warns while dirty.
**The scale** converts `EditorInputState` at its seam, brackets the engine's UI tail, builds the
full render-camera transform (centre and zoom), clamps once, records a tooltip's scale, and makes a
working submenu its first item. **The dock's** `GroupPrefs` gains `hidden` and a migration
precedence, a vertical budget (`MIN_CENTER_HEIGHT`) and a keyboard route to a tab's menu. **The
asset browser** splits into 4a (folders, New) and 4b (rename, delete): the router targets the
current folder within a kind, companions sit beside their primary and are badges rather than tiles,
pairs move together, `rename` refuses an existing target, web puts precede removes, failed
mutations keep the prompt open, `write_sheet_file` is named. **The convention** exempts `fonts/`
filenames and `LICENSE`, batch 6 runs the checker over the current trees first and migrates a
visitor's stored v2 scenes. Ruling (e) loses its interleave exception; ruling (i) gains the web
`.keep` marker.

**v1 — 2026-11-03, draft for round 1.** Planner: Claude Fable 5.1, from Jesse's brief of the
same day. Nothing here is settled; the rulings in § The rulings are *proposed* and wait for
Jesse. Reviews land in `review/editor-ux/` (this repo) and, for the site batch, in
`../insiculous_web/review/editor-ux/`.

## Context

Jesse's brief (2026-11-03, verbatim in spirit): the editor's UI and UX need to improve, natively
and on the web. Five asks, in his order:

1. **On the web, open the editor in its own window** — completely outside and isolated from the
   website, when the visitor chooses to.
2. **An adjustable font size** inside the editor.
3. **The toolbar at the top is clunky** — it looks like buttons, not a proper toolbar.
4. **Control over the panels**, like Godot's Editor layout menu (decide which panels show) and
   Unity's docking (move a panel where you want it). Today the Composer sits on top of Assets at
   the bottom instead of beside it as a tab.
5. **Create native files and folders from the Assets tab** — scenes, scripts, sidecars, songs,
   folders — and establish the house convention for naming and sorting a project's assets, for
   the examples and for the engine. Today the trees are messy.

The last two editor efforts are the shape this one follows: Playground UX
(`coordination/playground-ux/plan.md`, thirteen batches, Sep 2026) built the toolbar strip, the
dock's narrow mode, the preview window and the site's application shell; External Tools
(`coordination/external-tools/plan.md`, batches 1–6 landed by 2026-11-03) built the console, the
settings panel, the asset tiles' context menu and the file watcher. Both are read as the standing
rulings on anything they touched; where this plan moves one of their decisions it says so.

**Part:** Games — the Insiculous 2D engine's editor, and the Studio's `/playground/` pages that
host it.

## The rulings (Jesse's, 2026-11-03)

Each is a decision the plan needed that only Jesse makes. Proposed by the planner with v1 and
v2, accepted by Jesse in one word after round 1; a later change to any of them is a dated line
under it, never a silent rewrite.

- **(a) One knob, not two: UI scale, and the font sizes ride it.** Asked for: an adjustable font
  size. Ruled: a *UI scale* (100 % → 200 % in 25 % steps, plus *Follow the display*) that
  scales every editor font token AND every layout measure together, because the layout constants
  (`layout.rs`: `ROW_HEIGHT` 24, `HEADER_HEIGHT` 24, `TOOLBAR_STRIP_HEIGHT` 40) are sized for the
  14 px body font — a font grown alone clips at 18 px in a 24 px row. The scale is the one
  mechanism #179 (HiDPI: the playground is half size on a 2× display) also needs, so one batch
  closes both. A separate "main font size" (Godot has both) is a follow-up if the one knob proves
  not enough. WCAG 1.4.4 wants text resizable to 200 %: the top step is 200 %, not 150 %.
- **(b) The scale is the editor's, applied at the ui crate's boundary, never the game's.** The
  six games keep drawing their UI in physical pixels (their HiDPI story is #152, not this effort).
  The ui crate gains `pixels_per_point`; every call the editor makes is in points; the ui stores
  and emits physical pixels. The inner game, running inside a Play session, is wrapped at scale
  1.0 exactly where `EditorGame` already swaps the default font around it
  (`editor_game/mod.rs:343-352`).
- **(c) Panel placement by menu first, by drag second.** Batch 3 gives every panel a *Move to ▸
  Left / Right / Top / Bottom* and *Group with ▸ <panel>* through a new **Window** menu and the
  header tab's context menu, and gives each edge *tab groups*. Dragging a tab onto another group
  or an edge (Unity's gesture) is batch 7, after the menu-driven model has shipped and settled.
  The menu is the accessible path anyway: a drag has no keyboard.
- **(d) The bottom edge is one tab group by default: Assets · Conductor · Console.** The Composer
  "on top of Assets" was a stacking rule, not a layout choice (`default_layout.rs:31-47` adds
  three bottom panels; `dock/mod.rs` stacks them). The right edge is one group too: Inspector ·
  External Tools (today a hidden second right panel beside the Inspector).
- **(e) Sequencing against External Tools batch 7 (the Conductor rename, #227).** Batch 7 there
  trims the composer and renames `PanelId::COMPOSER`, `editor/src/composer/`, the View item and
  the theme tokens; its round-trip gate was met 2026-11-03 (`review/external-tools/roundtrip-7.md`)
  and its handoff is the next thing that effort dispatches. This plan's dock batch touches the
  same lines (`default_layout.rs`, `panel_id_for_menu_label`, the View menu). Ruled:
  **External Tools batch 7 goes out first; this effort's batch 1 follows once it has landed**
  (one batch out at a time across both efforts), and batch 3 here then builds on the renamed
  panel. Batches 1 and 2 do NOT interleave with it — batch 1 edits `theme/mod.rs`, whose tokens
  batch 7 renames, and batch 2 edits `menu/mod.rs` and `editor_game/mod.rs`, which batch 7 also
  touches (round 1, kimi F12). Only batch 5 (the site and the `playground` crate) is disjoint
  from it, and may go while batch 7 is out at Jesse's word. Ruled 2026-11-03. *Withdrawn after
  round 4 (codex F6): the freeze needs an `EditorGame::retire()` / `set_frozen` seam in
  `editor_integration`, so batch 5 is not disjoint; it waits like the rest.*
- **(f) The toolbar is icons with tooltips; Astra draws the shapes in code.** Icon buttons drawn
  from line and circle primitives (as the chevrons already are, `dock/render.rs:60-72`), never
  from a font glyph — the strip must read the same in every font the editor can be running
  (`toolbar.rs:252-253` says why). Every button's tooltip carries its name and its shortcut, so
  the shortcut captions stamped into each button (`toolbar.rs:231-232`) go. Astra (codex)
  reviews the plan and the batch's stills; the shapes are hers to approve, within the
  constraints § Design A states.
- **(g) The asset convention: one folder per kind, `snake_case`, the suffix names what the
  artist measures.** § Design D states the convention as a table; it becomes
  `docs/PROJECT_LAYOUT.md` and a check script, and the engine's own trees — `examples/assets`,
  `crates/playground/assets/projects/pong`, and the template repo — are brought to it. The
  games' trees already conform except where `deion_assets`'s sync writes them, and that script's
  layout is `DEION_STYLE.md` §6's, which this plan does not re-rule. The one rename the
  convention forces is **`images/` → `sprites/`** (three trees use `images/`, six games use
  `sprites/`). Jesse ruled 2026-11-03: `sprites/` wins.
- **(h′) On the web, a Playground project is open in one page at a time, and moving the editor
  to a window is a navigation, not a transfer.** Every Playground editor page takes a Web Lock
  named for its project **before** it boots the engine and holds it until the page goes; a page
  that cannot have the lock says so on a card and does not start. *Open in a window* requires
  everything saved, then opens the window and sends the page itself to a parked card; the
  window boots as any page does. To come back, the visitor closes the window and presses *Bring
  the editor back here* on the parked card. There is no *Take over* and no *Move back* button:
  a page gives a project up by closing or by moving it forward, never by being asked from
  elsewhere. The Playground refuses the window while its store is the in-memory fallback. A
  Rust game's editor page stores nothing, so it takes no lock and its window is a second,
  independent editor — the bar says so.
  *What (h) promised and (h′) does not:* a window that fails to start no longer leaves the
  source editing — the source has already parked, with everything saved; closing the failed
  window and one click on the card bring the editor back. And the return is two actions, not
  one.
  *Ruled by Jesse 2026-11-11 ("replace it with your plan"), replacing (h) of 2026-11-03 — the
  two-phase transfer with an owner record, a freeze and *Take over* — after its design drew a
  critical in each of five rounds, eight in the last.*
- **(i) Rename and delete land with create, in their own batch (4b); there is no trash.** Delete
  asks through the existing confirm dialog and removes the file (or the folder, named with its
  count). A paired asset — a `.png` with its `.sheet.ron`, a `.mid` with its `.mid.ron` and any
  `.recovered-*` of its stem — is one asset to rename, delete and show. On the web a folder is the
  set of paths under it; an empty one is kept by a hidden `.keep` entry the scanner skips and the
  export *keeps* (the importer drops directory entries, so the file is what carries the folder
  through a round trip — round 2, kimi F11); a folder made today is there tomorrow (round 1,
  gemini F4); natively `mkdir` is enough.
- **(j) — RULED by Jesse 2026-11-06 ("go ahead and accept the residual") — the native file rename's
  residual is accepted, stated in the panel's doc comment; no `libc`.** Proposed after round 4. A
  rename refuses an existing target through `hard_link(from, to)` then an inode-and-length check
  then `remove_file(from)`; between the check and the unlink there are two adjacent syscalls in
  which an external tool that atomically replaces the very file being renamed loses its new bytes
  (codex F1, round 4). No dependency-free primitive closes it (`renameat2(RENAME_NOREPLACE)` needs
  `libc`; `MoveFileExW` without replace needs `windows-sys`). Either Jesse accepts the residual,
  stated in the panel's doc comment, or admits `libc` for the Linux and macOS primitive. **The
  plan assumes the residual.** Without hard links the rename is refused outright (kimi F1).

## What the tree holds (verified 2026-11-03 by the planner, branch `jesse`, after 50962e2)

The executor re-greps every line here before the batch starts; earlier batches move things.

**The toolbar strip.** `crates/editor/src/toolbar_strip.rs` (449 lines) owns `split`, the
`begin`/`end` chrome scope on `UiLayer::PanelChrome`, and `layout` (tools left, play controls
centred where they fit, the view group at the right; `TOOLBAR_STRIP_MIN_WIDTH` at :40-46 sums the
groups; `GROUP_GAP` 8, `OVERFLOW_BUTTON_WIDTH` 34). `toolbar.rs` draws each tool as
`ui.button(id, tool.name(), bounds)` — a labelled, bevelled stock button — with a halo
`rect_rounded(bounds.expand(2.0), toolbar_active)` and a `rect_border` when selected, and the
shortcut letter as a caption at the top-right (:204-237); the overflow button is a stock button with
three dots (:239-262); button width is sized for the longest tool name at the body size (:79-83).
`play_controls.rs` (301 lines) draws Play/Pause/Resume/Stop/Follow as stock buttons with text
labels over `play_button_bg`/`stop_button_bg` (:129-200), `button_size` 48 and the "Resume" and
"Follow" widths hand-widened (:74-78, :107-109). `view_toggles.rs` (303 lines) draws the four
toggles as 24 px squares carrying text glyphs `#`, `O`, `[]`, `%` (:55-63, :181) and Reset Layout
as a fifth; `VIEW_GROUP_WIDTH` (:189) is summed from them. `overflow_menu.rs` (331 lines) lists the
shed tools and toggles by name. Theme tokens the strip reads: `toolbar_active`, `accent_blue`,
`accent_cyan`, `shortcut_hint`, `play_button_bg`, `stop_button_bg`, `surface_1`, `border_subtle`
(`theme/mod.rs`). The strip's tests (`toolbar_strip.rs:224-449`) pin: the split tiles the content;
the play controls hold the centre and never move with state; nothing overlaps or clips at 800,
390, 360 and `MIN + 7` px; the view group sheds one pixel under the minimum; a modal scrim keeps
Play inert.

**Fonts and sizes.** `typography.rs`: `FontSizes { small 12, body 14, heading 16 }` and
`MIN_READABLE_FONT` 12 with a guard test. `theme/mod.rs:357-400 ui_theme()` copies `fonts.body`
into `ui::Theme.text.font_size` and `text_input.font_size`, `fonts.small` into the tooltip. The
ui crate's own defaults are 13/13/16 (`ui/src/style.rs:174,203,223`). `fonts.rs` ships DejaVu
regular/bold/mono by `include_bytes!`. Every layout measure is a `const` in `layout.rs`
(`PADDING` 8, `TOOLBAR_STRIP_HEIGHT` 40, `HEADER_HEIGHT` 24, `ROW_HEIGHT` 24, `FIELD_HEIGHT` 20,
`BUTTON_HEIGHT` 26, `GAP` 4, `MIN_PANEL_SIZE` 100, `DEFAULT_PANEL_WIDTH` 250). The panel
renderers carry their own (`panel_renderer/asset_browser.rs:26-44`: `TILE_SIZE` 72, a 26 px
header, `TILE_GAP` 10).

**How a frame reaches pixels.** `engine_core/src/game.rs:450 step_frame(delta_time, window_size)`
→ `:484 update_ui_begin` → `ui.begin_frame_dt(&self.input, window_size, dt)`
(`ui/src/context/mod.rs:148`); `window_size` is the physical size (`game.rs:416`). The ui's
`InputHandler` mouse position is physical (winit `CursorMoved`). Text is laid out by
`ui/src/font/layout.rs:58-71`, which rasterises each glyph at the requested `font_size`
(`glyph_cache.rs:86-93`, keyed by tenths of a point) and records physical glyph boxes in
`TextDrawData.glyphs`. `engine_core/src/ui_integration/mod.rs:81 render_ui_commands` turns the
draw list into sprites, looking each glyph's texture up by `(character, width, height)` (:206-211).
The window's scale factor is tracked (`window_manager.rs:128,232,237`, set on
`ScaleFactorChanged` at `app_handler.rs:337`) and **read by nothing that lays anything out**; on
the web `renderer/src/window.rs:136-143 shown_size` multiplies the canvas box by
`device_pixel_ratio` for the surface size only. `GameContext` exposes no scale. **The editor's
world passes** take their rect from `scene_view_bounds()` (`game_impl.rs:139-150`:
`to_window_render_camera(ctx.window_size)` and `viewport_scissor`), in the same pixels the dock
was laid out in; `viewport_interaction.rs` and `gizmo_drag.rs` read the same bounds for picking.
`EditorGame` already brackets the inner game's `update` with a font swap
(`editor_game/mod.rs:343-352`) — the place the scale is bracketed too.

**The dock.** `crates/editor/src/dock/mod.rs` (505 lines): `PanelId` 0–6 (`SCENE_VIEW`,
`INSPECTOR`, `HIERARCHY`, `ASSET_BROWSER`, `CONSOLE`, `COMPOSER`, `SETTINGS`; :30-45),
`panel_id_for_menu_label` (:56-65, four labels), `DockPosition` (Left/Right/Top/Bottom/Center/
Floating — `Floating` is declared and laid out by nobody), `DockPanel { position, size, min_size,
visible, collapsed, hint }`, `DockArea { panels: Vec<DockPanel>, narrow, narrow_overlay }`, and
`layout()` (:376-468): one pass allocating each visible edge panel in `panels` order — so two bottom
panels stack, the later one above — then the centre, then the narrow tabs. `render.rs` (411 lines)
draws a header per panel with the title and one chevron, the collapsed strip, the narrow tab and
overlay, and the resize grabber. `tests.rs` (417 lines) pins the narrow-mode contract. The default
set is `context/default_layout.rs` (Hierarchy left 200, Inspector right 280, Scene centre, Assets
bottom 180, Composer bottom 260 hidden, Console bottom 160 hidden, External Tools right 320
hidden). `editor_preferences.rs` (334 lines) persists `PanelPrefs { id, visible, collapsed, size }`
per non-centre panel (:11-21, :110-146) — **no position**; a prefs file from before a panel leaves
it at its default. The View menu (`menu/mod.rs:221-236`) lists five panel items and the toggles;
`EditorAction::ResetLayout` (`editor_input.rs:141`) restores `default_dock_area()`. The composer
renders through its host, not `render_panel_content` (`editor_game/mod.rs:310-313`,
`panel_renderer/mod.rs:68`). The context menu (`context_menu.rs`) is a generic
`ContextMenu::open(anchor, items, note)` with `ContextMenuAction::Open(OpenRequest)` as its only
action kind; `open_for_file` builds the tile's two items.

**The asset browser.** `crates/editor/src/asset_browser.rs` (433 lines): `scan_assets` walks
`common::vfs::list_files` recursively and returns a **flat** list sorted by `(kind, name)` (:214-250)
— folders are invisible except in the tile's tooltip; `AssetKind` is seven kinds by extension
(:22-40); `AssetBrowserState` keeps the selection by index with a path carried across rescans, the
double-click, the keyboard hold, the grid's columns and two render requests.
`editor_integration/src/panel_renderer/asset_browser.rs` (**583 lines — at the ceiling**) draws a
header (Rescan, Assign, a count) and the grid; `asset_browser/tile.rs` (203) the tile, its drag
and its right-press menu; `asset_browser/keyboard.rs` (136) the keys. `common/src/vfs/mod.rs` (424
lines) offers `read`, `read_to_string`, `write`, `write_string`, `list_files`, `list_dir_files`,
`remove_prefix`, `change_stamp`, `file_size`, `insert` (web) — **no `create_dir`, no
`remove_file`, no `rename`, no listing of directories**. New files the editor already writes: a
scene (`scene_io.rs`, default `scenes/scene.ron`), a song pair and its `LICENSE` stanza
(`song_io.rs`, default `music/untitled.mid`), `editor_prefs.json`; sheet sidecars and scripts are
written by nothing in the editor.

**The trees.** `examples/assets/`: `fonts/ images/ locales/ music/(+samples/) scenes/ scripts/
soundfonts/ sounds/`; `hello_world.scene.ron:73` names `"assets/images/wood_texture.png"` (with an
`assets/` prefix the other scenes do not carry — verify at batch 6 whether that path resolves
against a different base). `crates/playground/assets/projects/pong/assets/`: `images/ scenes/
scripts/`. `../games/game-template/assets/`: `fonts/ images/ scenes/ scripts/`. The six games:
`ai/music/ fonts/ (locales/) (scenes/) soundfonts/ sprites/(+sync.list)`, every sprite PNG beside
its `.sheet.ron`, `ai_` prefixed. `docs/SONG_FORMAT.md` and `training.md` rule `music/`,
`ai/music/`, `soundfonts/`, `sounds/` and their `LICENSE`s; `DEION_STYLE.md` §6 rules art names.

**The web.** `../insiculous_web/src/components/PlaygroundEmbed.astro` (421 lines) is the
workspace: head region, the canvas stage, the DOM dock (scripts, command console); its header
says **exactly one embed per page** — the engine binds to fixed DOM ids and module singletons.
`PlaygroundToolbar.astro` (246) is the bar: Project select, Save, Play ↗, Export, Import, Reset,
Fullscreen, Help, by ids `playground-embed.ts` finds. `AppLayout.astro` (157) is the application
layout: studio wordmark, the Aa accessibility controls, an `h1` slot, the `bar` slot, `main`, a
footer. `pages/playground.astro` (25), `pages/playground/[slug].astro` (108, one page per game
with an `editor` bundle in `src/content`), `pages/playground/preview.astro` (157) — the window
`Play ↗` opens, booting the same bundle with `?mode=preview` (`web_entry.rs:76`), guarded by
`playground-preview-protocol.ts` (a generation, a reservation under
`beinsiculous.playground.preview`, `ownTabId()` in `window.name`) and
`playground-preview-launch.ts`. `web_entry.rs:151` reads `?project=<slug>`. Gates: `npm run
verify` = validate, data tests, `astro check`, build + `postbuild-check.mjs` (resolves every
`data-wasm-src`), axe over every page, the announce check, and the phone/125 % screenshots.

**Issues this plan absorbs or touches:** #179 (HiDPI scale — batch 2 closes it), #233 (narrow
overlay over the menu bar's dropdown — batch 3 re-reads it), #99's *Preferences* line (positions
persist from batch 3), #159 (the v3 playground bundle — batch 6 rebuilds the bundled pong's tree,
so the bundle it ships in is v3), #197/#200/#186/#188/#187 (the composer's — External Tools batch
7's to re-read, not this plan's).

**Files near the 600-line ceiling** (measured 2026-11-03): `panel_renderer/asset_browser.rs` 583,
`editor_game/mod.rs` 567, `context/mod.rs` 549, `menu/mod.rs` 538, `dock/mod.rs` 505,
`editable_inspector.rs` 596. A batch that adds to one of them splits it first, as its own
commit-sized step inside the batch, and the handoff says so.

## Design

### A. The toolbar strip as a toolbar

The strip keeps its contract — `split`, the chrome scope, the centre-held play controls, the shed
order (view group whole, then tools from the end), `TOOLBAR_STRIP_MIN_WIDTH` as the dock's narrow
threshold — and changes what it draws.

**Target shapes.**

```rust
// crates/editor/src/icons.rs (new, pure, ≤ 300 lines)
/// The editor's glyphs, drawn from strokes so they read the same in every font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Select, Move, Rotate, Scale,            // the tools
    Play, Pause, Resume, Stop, Follow,      // the transport (Resume shares Play's shape)
    Grid, Colliders, GameFrame, Snap,       // the view toggles
    ResetLayout, More,                      // the fifth view button, the overflow chevron
}
/// Draw `icon` centred in `bounds`, strokes at `weight`, in `color`. Every stroke stays inside
/// `bounds.shrink(2.0)`; the test below asserts it for every icon at 16, 20 and 24 px.
pub fn draw_icon(ui: &mut UIContext, icon: Icon, bounds: Rect, color: Color, weight: f32);
/// The strokes of `icon` in a unit box, for the test and for Astra's review still.
pub fn icon_strokes(icon: Icon) -> Vec<Stroke>;   // Stroke = Line(a, b) | Circle(c, r) | Arc(..)
```

```rust
// crates/editor/src/toolbar.rs — the tool group as a segmented control
/// One square icon button per tool, drawn as a connected segment: one rounded background for
/// the group (`theme.toolbar_segment`), a divider between neighbours, the active tool filled
/// with `theme.toolbar_active` and ringed in `accent_blue`. No label; the tooltip is
/// "Move (W) — Drag the gizmo's arrows…" built from `name()`, `shortcut()` and `hint()`.
pub const TOOL_BUTTON_SIZE: f32 = 28.0;
pub fn button_width(&self) -> f32  // = TOOL_BUTTON_SIZE; the strip's layout keeps calling it
```

`play_controls.rs`: Play/Pause/Resume/Stop/Follow become icon buttons of `TOOL_BUTTON_SIZE` in a
second segment; Play's segment is tinted `play_button_bg`, Stop's `stop_button_bg`, as today;
"Resume" and "Follow" lose their hand-widened widths (`first_button_width`, `follow_width`
collapse to the one size — `WIDEST_CONTENT_WIDTH` is recomputed and its test re-pinned). Follow is
a toggle: filled when on. `view_toggles.rs`: the four toggles and Reset Layout are icon buttons of
the same size in a third segment, filled when on; the text glyphs go. The three segments sit on
the strip with `GROUP_GAP` between them and a one-pixel `separator` line at each gap. The strip's
height stays `TOOLBAR_STRIP_HEIGHT` (40) with the 28 px segments centred — a one-line change if
Astra wants 36. Hover: `menu_open_highlight` as today's chevron does.

**Theme.** Two new tokens, `toolbar_segment` (the group's background, one ladder step above
`surface_1`) and `icon_stroke` (= `text_primary` unless Astra rules otherwise); the luminance
guard tests in `theme/tests.rs` gain a row each: an icon stroke on its segment ≥ 3:1 (WCAG 1.4.11
non-text contrast), the active fill against the segment ≥ 1.35:1 like the ladder.

**Width.** With 28 px squares the tool group shrinks from four labelled buttons to 4 × 28 + 3 × 4 =
124 px; `TOOLBAR_STRIP_MIN_WIDTH` falls accordingly, which widens the range where the dock stays
out of narrow mode on a phone — a gain, and the reason the strip's four width tests are re-pinned
against the new constants rather than against literals.

**Tests.** `icons.rs`: every icon's strokes lie inside the shrunk box at three sizes; no icon is
empty. `toolbar.rs`: the active tool is the one filled (one fill rect per frame); the tooltip of a
tool names its shortcut (the existing `test_tool_shortcut_hints_match_the_editor_bindings` grows
one assertion). `play_controls.rs`: `WIDEST_CONTENT_WIDTH` is the measured widest state (the
existing test). `toolbar_strip.rs`: the four layout tests unchanged in intent. The stills for
Astra: the strip at 1200 px in each play state, and at 390 px with the overflow open — native, via
the headless draw-list dump the Playground UX batches used (`test_support::frame`), plus one real
screenshot of the editor_demo at 1280×800 for the codex round.

**Leaves out.** The menu bar and the status bar (#99's lines); a command palette; icon theming
per game.

### B. UI scale — points at the ui boundary

**The rule.** The ui crate gains one number, `pixels_per_point` (default 1.0). *Every argument the
caller passes and every value the ui returns is in points; everything the ui stores and emits is
in physical pixels.* So: a rect handed to `interact`, `button`, `panel_styled`, `label_*`,
`push_clip_rect`, `begin_overlay_in`, `tooltip` is multiplied on the way in; `mouse_pos()`,
`measure_text*`, `window_size()`, `scroll_delta()` and an interaction result's positions are
divided on the way out; the draw list is pushed in physical pixels, so `render_ui_commands` and
the glyph texture cache in `engine_core` change by nothing. Text is rasterised at `font_size ×
scale` (`layout.rs:71` passes the product to `get_or_rasterize`), which is why glyphs stay crisp
at 2× rather than being a bitmap stretched.

```rust
// crates/ui/src/context/mod.rs
/// Physical pixels per point for every call until the next `set_pixels_per_point`. Stored
/// state is physical, so the value may change mid-frame: the editor sets its scale, draws its
/// chrome, sets 1.0 around the game's own update, and sets it back.
pub fn set_pixels_per_point(&mut self, scale: f32);   // clamped to 0.5..=4.0, non-finite → 1.0
pub fn pixels_per_point(&self) -> f32;
```

One implementation seam — `fn to_physical(&self, rect: Rect) -> Rect`, `fn to_points(&self,
position: Vec2) -> Vec2` and `fn to_physical_length(&self, points: f32) -> f32` in
`context/mod.rs` — and every public entry calls one of them. The third exists because a rect is
not the only measure: `corner_radius`, `border_width`, a line's weight, a circle's radius, a
slider's track and thumb are scalar points and scale with the rest (kimi F10, round 2); the
theme's metrics are points. `scroll_delta()` is wheel *notches*, dimensionless
(`scroll.rs:32,54` multiplies by `WHEEL_STEP`), and is **not** converted — only positions, deltas
and lengths are (codex F12). The
blocking regions and the widget rects the interaction layer keeps are physical, so
`is_input_blocked_at(point)` converts its argument and a game at scale 1.0 asks in physical
pixels and gets the right answer against chrome laid out at 1.5. A tooltip offered at one scale
and drawn at `end_frame` records the scale it was offered at and draws at it (round 1, codex F6).
`measure_text` measures the rasterisation at `size × scale` and returns `ceil(physical) / scale`,
because `layout.rs` rounds glyph boxes to whole pixels and `scale × measure(size)` drifts over a
long label (gemini F7, round 1) and a floor would clip the last pixel (gemini F8, round 2); its
test asserts a relative tolerance (1 %) on a 60-character string, not 0.5 pt.

**The engine.** `GameContext` gains `display_scale: f32` — the window's `scale_factor()` natively
(`window_manager.rs:232`), `device_pixel_ratio` on the web (one helper beside `shown_size`) —
threaded by `build_context!` (`game.rs:274-287`). The runner's **UI tail** — `post_update` →
`draw_scene_ui` (`game.rs:532`, `frame_tail.rs:119-136`: scene `UiLabel`/`UiPanel`/`UiButton`
and the achievement toasts, at physical `window_size`) — runs *after* `EditorGame::update`
returns, so it would draw at the editor's scale (kimi F6, codex F6). The editor leaves a request
beside `clip_engine_ui`: `ctx.ui_scale_for_engine_tail: Option<f32>` (the editor sets `Some(1.0)`;
a plain game leaves `None`), and `draw_scene_ui` sets the ui's scale to it for the tail and
restores after, with its clip converted to physical. (`scripts/check_wasm.sh` is a gate:
`engine_core` changes.)

**The editor.** `EditorPreferences.ui_scale: UiScale` where
`enum UiScale { FollowDisplay, Percent(u16) }` (serde: `"follow"` or a number; absent →
`FollowDisplay`). **One resolver**, `resolve_ui_scale(prefs: &UiScale, display_scale: f32) ->
f32`, clamps to `0.5..=4.0` and maps a non-finite or zero value to 1.0, and every consumer — the
ui, the dock's layout size, the camera, the scissor — reads that one finite value (kimi F10: a
hand-edited `ui_scale: 0` must not divide the window by zero). `EditorGame::update` calls
`ctx.ui.set_pixels_per_point(scale)`, and `ctx.window_size / scale` is **`points_window_size()`,
the one value every chrome consumer reads** — the dock, the menu bar (`menu_actions.rs:46`), the
status bar (`console.rs:47`), the scene and Stop confirm dialogs (`scene_confirm.rs:158`,
`stop_confirm.rs:68`), the add-component popup — because `ConfirmDialog::panel_rect(window_size)`
centred on physical pixels at scale 2 puts the Save/Discard buttons past the window (codex F9).

Three seams convert back to physical:

1. **Viewport input.** The one raw read is `EditorInputMapping::update_state`
   (`editor/src/editor_input.rs:462-472`: position, movement delta, wheel), which builds
   `EditorInputState` for picking, the marquee, pan and the gizmo (kimi F5). It takes the scale
   and converts position and delta there; nothing downstream changes.
2. **The render camera.** `to_window_render_camera(window_size)` (`viewport/mod.rs:256-263`)
   offsets the camera by `(window/2 − viewport_centre) / zoom` in the viewport's own units and
   keeps its zoom; with the dock in points and the surface in physical pixels, the GPU's centre
   lands at the point coordinates, not the physical ones (codex F5). It becomes
   `to_window_render_camera(window_physical, scale)`: the viewport centre × scale, the zoom ×
   scale; the camera-equivalence tests gain scales 1.5 and 2 at a non-zero viewport offset and
   assert GPU position = overlay position × scale. Two spaces, stated once in `viewport/mod.rs`'s
   module doc: the editor's camera (`screen_to_world`, `world_to_screen`, `camera_zoom`,
   :200-227) is in points; the GPU camera alone is physical; picking, the marquee, pan and the
   gizmo never see the scale (gemini F9, round 2 — rebutted as a bug, kept as the contract).
3. **The scissor.** `viewport_scissor = scene_view_bounds() × scale` (`game_impl.rs:139-150`),
   origin floored and extent ceiled to whole pixels, and the GPU camera's centre taken from the
   same snapped rect, so a fractional scale opens no one-pixel seam between the scene, the gizmos
   and the chrome (gemini F8, round 4).

The inner game is bracketed: `set_pixels_per_point(1.0)` before `self.inner.update(ctx)` at
`editor_game/mod.rs:343`, the editor's scale after, beside the existing font swap; the engine tail
is bracketed by the request above. The game's `ctx.window_size` stays physical — its HUD is laid
out as it is today (#162 governs where it draws in the editor).

**The menu — and the submenu that must exist first.** `MenuItem::Submenu` renders today as one
dead button labelled `"<label> >"` (`menu/mod.rs:525-529`, "for now"); this batch's first item is a
working nested dropdown — opened by hover or click on its row, drawn on the same band to the
right of its parent when that fits, else to its left, else clamped to the window's edge *over*
its parent with its own region blocking the rows beneath (at 390 px and 150 % the ui is 260 pt
wide and a dropdown is 200 — codex F11, gemini F10 round 2; a submenu drawn *below* the row would
cover the parent's sibling rows and a hover there would dismiss it, gemini F6 round 3),
closed with it, walked by the arrow keys where the parent is — with a test that a nested action
fires, that Escape closes both, and that a submenu at 390 px / 150 % and at 1280×800 / 200 %
lies inside the window. Then: View ▸ *UI Scale* ▸ `Follow the
display (125 %)` · `100 %` · `125 %` · `150 %` · `175 %` · `200 %`, check-marked; the follow item
shows the live value. Shortcuts `Ctrl+=`, `Ctrl+-`, `Ctrl+0` step and reset
(`EditorAction::UiScaleUp / UiScaleDown / UiScaleReset`, `EditorInputMapping`); on the web the
browser may take `Ctrl+=` first, and the menu is the path that always works — the doc line says
so. Persisted on change like every preference.

**Minimums.** `MIN_EDITOR_WINDOW_WIDTH/HEIGHT` (1024×720) stay as physical minimums; at 200 % the
dock goes narrow at 1024 px, which is the designed behaviour, not a defect.

**Tests.** `ui`: a 10×10 rect at (10,10) at scale 2 pushes a 20×20 rect at (20,20); a label at 14
pt at scale 2 rasterises glyphs whose heights are those of the 28 pt rasterisation;
`measure_text` at scale 1.25 and 2 equals the scale-1 measure within 1 % on a 60-character
string; a button at a points rect is clicked by a physical press at 2× the coordinates and not
at 1×; a blocking region pushed at scale 2 blocks a physical point asked for at scale 1; a tooltip
offered at scale 2 and drawn after a switch to 1 draws at 2; `rect_border` at scale 2 with a 1 pt
width pushes a 2 px border; one wheel notch scrolls `WHEEL_STEP` points at scale 1 and 2 alike.
`editor`: the camera-equivalence
tests at 1.5 and 2 with a non-zero viewport offset; `update_state` at 1.5 reports a physical
(300, 300) press as (200, 200) with its delta halved. `editor_integration`: the scissor rect at
scale 1.5 is 1.5× `scene_view_bounds()`; at 1.5, a click selects the entity under the pointer, a
marquee's corners are where the pointer went, a pan moves the camera by the pointer's distance,
a gizmo drag moves the entity under the handle; a Play-mode scene with a `UiButton` anchored
bottom-right is clicked at 1.5 and the engine tail drew it at scale 1; the scene confirm dialog,
the Stop dialog and the status bar at scale 2 lie inside the window; `resolve_ui_scale` on 0,
1, 65535 and NaN; a prefs file without `ui_scale` loads as `FollowDisplay`; the menu's check
follows the preference; a nested menu action fires. Stills for codex: the editor at 100 % and
150 % at 1280×800, and the playground at 390 px.

**Leaves out.** The games' HiDPI (#152); a separate font-size knob (ruling (a)); scaling the
`MIN_EDITOR_WINDOW_*` constants.

### C. The dock: tab groups and the Window menu

**Model.** An edge holds an ordered list of **groups**; a group holds an ordered list of panels and
one **active** panel; a group has the size (its panels no longer do). The centre is the Scene View
alone, as today. Visibility stays per panel: a group whose every panel is hidden takes no space; a
group's header shows a tab per *visible* panel.

```rust
// crates/editor/src/dock/mod.rs → split: dock/mod.rs (DockArea, PanelId, DockPosition),
// dock/groups.rs (TabGroup and the moves), dock/layout.rs (the allocation), dock/render.rs
pub struct TabGroup {
    pub position: DockPosition,          // Left | Right | Top | Bottom (Center is the scene)
    pub panels: Vec<PanelId>,            // tab order
    pub active: PanelId,                 // the shown one; always a member
    pub size: f32, pub min_size: f32,    // = the largest member's min
    pub collapsed: bool,
    bounds: Rect,
}
impl DockArea {
    pub fn groups(&self) -> &[TabGroup];
    pub fn group_of(&self, id: PanelId) -> Option<usize>;
    /// Move `id` into its own new group at the end of `edge`; a group left empty is dropped.
    pub fn move_panel_to_edge(&mut self, id: PanelId, edge: DockPosition);
    /// Move `id` into `other`'s group, as the tab after it, and make it active.
    pub fn group_panel_with(&mut self, id: PanelId, other: PanelId);
    pub fn activate(&mut self, id: PanelId);       // also sets visible = true
    pub fn show_panel(&mut self, id: PanelId)      // keeps its contract: activate + expand + narrow overlay
}
```

`DockPanel` keeps `id, title, hint, visible` and loses `position, size, min_size, collapsed,
resizable` to the group (`min_size` moves as the group's `min_size` = max over members).
`DockPosition::Floating` is deleted — nothing lays it out, and the drag batch does not need it
(a dropped tab lands in a group). `layout()` allocates group by group in edge order, exactly as it
allocates panels today, then the narrow tabs one per *group* (the tab shows the active panel's
initial; opening it opens the group). `render.rs` draws a header per group: a tab per visible
panel (title, `label_in_bounds_styled`, the active tab in `surface_2`, the others `surface_1`,
hover highlight), the one chevron at the right. Tabs ellipsize down to a 48 pt minimum; past that
the strip shows the tabs that fit and a `»` button listing the rest (the overflow menu's shape,
`overflow_menu.rs`), and F6 with the arrows still walks every tab (codex F9, round 4: six panels
in one narrow group at 200 % otherwise overlap the chevron). Test: every movable panel in the
right group at its minimum width and at 200 %. A left click on a tab activates; a right click or
`Shift+F10` on a tab opens the context menu (next paragraph). `handle_resize` resizes the group.

**The Window menu.** A new top-level menu between View and Entity:

```
Window
  ✓ Hierarchy        ▸  Move to ▸ Left · Right · Top · Bottom
  ✓ Inspector           Group with ▸ (every other visible panel)
  ✓ Assets              Hide
    Conductor
    Console
    External Tools
  ──
  Reset Layout
```

Each panel row is a submenu, on the nested dropdown batch 2 built. The same three items are the
tab's context menu, through `ContextMenu` with a new `ContextMenuAction::Dock(DockAction)`
variant, `enum DockAction { MoveTo(PanelId, DockPosition), GroupWith(PanelId, PanelId),
Hide(PanelId) }`, dispatched by `menu_actions.rs`. View keeps its five panel items as toggles
(they call `show_panel` / hide) — the two menus agree because both read the dock.

**Corrected after batch 3 (2026-11-05):** each panel row's last item is `Show` for a hidden panel
and `Hide` for a visible one — `DockAction::Show(PanelId)` joins the enum — and opening a row's
submenu changes nothing; the diagram above had no way to show a hidden panel, and the executor
filled the gap by revealing the panel whenever its submenu opened, which round 9 removed (browsing
to Reset Layout rearranged the dock). A tab click, an overflow row and a move reveal through
`DockArea::reveal`: activation within a group already showing content, `show_panel` otherwise,
so a visit to the narrow overlay never writes the saved collapse flag. A dragged payload's target
comes forward after it rests on the tab for `DRAG_DWELL_FRAMES` (host rule in `frame.rs`), and
`MIN_CENTER_HEIGHT` is the scene's header, its toolbar and 120 pt of viewport.

**The keyboard route** (codex F12 — a menu that only a pointer reaches is no alternative to a
drag). `F6` cycles the ui's focus across the group tab strips (the focus ring the Playground UX
batch 12 gave the editor); with a strip focused, `Left`/`Right` move among its visible tabs,
`Enter` activates the focused tab, `Shift+F10` or the Menu key opens that tab's action menu
(`ContextMenu` at the tab), and `Escape` returns focus to the strip. A headless test walks
Inspector to Bottom and back with no mouse event. The menu bar's own mnemonics and arrow
navigation stay out of scope (#99).

**The vertical budget** (codex F11). `MIN_CENTER_HEIGHT` — the toolbar strip plus 120 pt — joins
`MIN_CENTER_WIDTH`. When the top and bottom groups' effective sizes would leave the centre below
it, `layout()` shrinks those groups to their minimums from the centre outward, then collapses the
group furthest from the centre, until the centre fits; the correction is an *effective* size
`layout()` computes each frame and never writes — `size` stays what the user set, so a window
tiled small for a minute does not overwrite a saved layout (gemini F5, round 2); only a drag or
the user's own collapse persists. **A group the budget collapsed still opens** (codex F12, round 3:
at 1280×800 / 200 % the bottom group's 200 pt minimum does not fit): its chevron, `show_panel` and
the Window menu open it as a temporary overlay through the narrow-overlay path the dock already
has, and the test interacts with the Conductor's content there, not only with the rectangles. A `MoveTo` that cannot fit even with
every group on that edge collapsed is refused with a status-bar line. Tests: three bottom groups
of 200 pt at 200 % on 1280×800; an oversized saved layout restored into a smaller window.

**Preferences.** `PanelPrefs` → `GroupPrefs { position, size, collapsed, panels: Vec<u32>,
active: u32, hidden: Vec<u32> }` under a new key `groups` — `hidden` because visibility is per
panel and a group record without it cannot say a default-visible panel was hidden or a
default-hidden one shown (gemini F6, kimi F7, codex F13). `panels: Vec<PanelPrefs>` is still read
(a file from today) and mapped with a stated precedence: each legacy panel lands in its default
group; the group's `size` and `collapsed` are those of the *first visible* legacy panel in panel
order (Assets' for the bottom group), `hidden` is every legacy panel saved `visible: false`,
`active` is the first visible member; the legacy list is dropped on the next save. Unknown ids
skipped, sizes clamped to the minimum as today, a group whose `active` is not a visible member
takes its first visible one, and a legacy group whose every panel is hidden takes the default
size, expanded, its default primary as `active` and every member in `hidden` (gemini F6); **a
registered panel absent from every saved group joins its default group**, hidden if its default is
hidden (gemini F7 round 3 — a panel a later release adds must not be unreachable until Reset
Layout).

**Default layout** (ruling (d)): Left [Hierarchy]; Right [Inspector · External Tools]; Bottom
[Assets · Conductor · Console]; Assets active, External Tools and the two hidden as today.

**Hosts to re-thread.** `panel_press`, `composer_keys.rs:77-78`, `asset_browser_keys`,
`console.rs` and `settings_keys` read `dock_area.get_panel(id)` for bounds and visibility;
`get_panel` stays (bounds now = its group's when active, `Rect::default()` when not),
`set_panel_visible` / `toggle_panel_visible` / `set_panel_collapsed` keep their signatures and act
on the group. **An inactive tab is not a keyboard host** (codex F10): `activate` clears the
deactivated panel's focus (the composer's, the asset browser's, the console's), and each
`settle_*` requires its panel to be its group's active one — `panel_press` returns `Neither` on a
frame without a press (`panel_press.rs:39-41`), so an empty bounds alone would leave the
Conductor eating Ctrl+Z behind another tab. `narrow_overlay` is per group. #233 (the narrow overlay over the menu bar's dropdown)
is re-read while here: if the fix is the band order, it lands; if not, the issue gets the line.

**Tests.** `dock/tests.rs` keeps every narrow-mode test green (the executor's first gate). New:
two panels on one edge are one group's two tabs by default; `move_panel_to_edge` leaves an empty
group dropped and the centre re-widened; `group_panel_with` makes the moved panel active and the
group's size the target's; a hidden active tab yields the next visible one; a prefs file with the
old `panels` list lands every panel in its default group with the precedence above (Assets
visible at 180 beside a hidden, collapsed Composer at 260 yields a bottom group of 180,
expanded, Composer hidden); a new-format round trip with a hidden default-visible panel and a
shown default-hidden one; the vertical budget's two cases (and that `size` is unchanged after
them); the all-hidden legacy group; the keyboard walk; F6 to the bottom strip, Right to Assets,
Enter, then Ctrl+Z undoes the scene and not the song; the Window menu's checks follow
visibility after a hide from the tab menu. Stills for codex: the default layout at
1280×800; the bottom group with Console active; a two-tab right group.

**Leaves out.** Dragging tabs (batch 7); floating windows (`Floating` goes); per-panel sizes
inside a group.

### D. The asset browser: folders, create, and the convention

**The convention** — `docs/PROJECT_LAYOUT.md`, the SSOT a game, an example and the browser's
*New* menu all follow:

| folder | holds | names |
|---|---|---|
| `scenes/` | `<name>.scene.ron` | `snake_case`; a two-player variant `<name>_2p` |
| `scripts/` | `<name>.rhai` | the entity or role it drives: `paddle.rhai`, `goal.rhai` |
| `sprites/` | `<name>.png` + `<name>.sheet.ron`, always the pair | `DEION_STYLE.md` §6: `snake_case`, a size suffix names the cell or the sheet (`_32x32`, `_480x288`), `ai_` prefix on an AI first cut |
| `sounds/` | recorded one-shots and loops + `LICENSE` | `snd_` is not a prefix — `jump.wav`, not `snd_jump.wav` (one example file renames) |
| `music/` | `<stem>.mid` + `<stem>.mid.ron` + `LICENSE`; `samples/` per `SONG_FORMAT.md` | `SONG_FORMAT.md` |
| `soundfonts/` | `<name>.sf2` + `LICENSE` | the game's or the bank's name |
| `fonts/` | `.ttf`/`.otf` + `LICENSE` where the licence asks | **the face's distributed filename, kept as is** (`BlackSamsGold-ej5e.ttf`): a face is a vendored artefact, often under a licence that names its file, and `locales/*.ron` name these files — so `fonts/` is the one folder the `snake_case` rule does not reach, and the checker says so rather than exempting it silently (kimi F11); `font.ttf` is the one the engine looks for first and stays |
| `locales/` | `<code>.ron` | the locale code: `en.ron`, `pirate.ron` |
| `ai/` | an agent's first cut, mirroring the tree (`ai/music/` today) | `ai_` prefix as well, so the paid-tier purge is greppable either way |

Rules: one folder per kind at the asset root, no kind in another's folder, no nested kind
folders (`sprites/props/` is allowed — a subfolder *within* a kind — `sprites/scenes/` is not);
every `.png` under `sprites/` has its sidecar; every audio folder has its `LICENSE`
(`check_audio_provenance.sh` already); `snake_case`, ASCII, no spaces, one dot before the
extension chain — for every file except a font's filename and `LICENSE`, the two stated
exceptions, and the engine's own generated shapes, which the doc lists and the checker accepts: a
sample copied under a name collision, `<stem>-<hash>.<ext>` (`song/io/samples.rs:55`), a
recovered score, `<stem>.recovered-<stamp>.mid` (codex F13), and the `.keep` the web export writes
for an empty folder (kimi F9 round 3); the checker's test runs it over a
real song save with a collision. `scripts/check_project_layout.sh <project>` enforces the folder-per-kind, the sidecar
pair and the names over `examples/`, `crates/playground/assets/projects/*`, `../games/*`; the
games' `sprites/` is `DEION_STYLE.md`'s and the checker reads a `sync.list` as the authority for
what is there. **Batch 6's first step runs the checker over every current tree and lists each
violation in its report before any file moves**, so the rule is tested against reality before it
is enforced (kimi F11). The checker is a gate of every later batch and of `check_games.sh`.

**Folders in the browser.** `scan_assets` returns a tree:

```rust
// crates/editor/src/asset_browser.rs → asset_browser/mod.rs + asset_browser/tree.rs
pub struct AssetFolder { pub relative_path: String, pub name: String }
pub struct AssetScan { pub files: Vec<AssetEntry>, pub folders: Vec<AssetFolder> }
pub fn scan_assets(base: &Path, daw: &[String]) -> AssetScan;   // folders from vfs::list_entries
impl AssetBrowserState {
    pub current_folder: String,                 // "" = the root; persisted in prefs
    // no pending_folders cache (4a retired the planned field, round 11): the scan is
    // authoritative — read_dir lists an empty directory natively, its .keep does on the web
    pub selected: Option<TileId>,               // TileId { kind: Folder | File, path }: an index into
                                                // the file-only list names the wrong thing once
                                                // folders sort first (kimi F9, round 2)
    pub fn visible_tiles(&self) -> Vec<Tile>    // folders first (by name), then files by (kind, name)
    pub fn enter(&mut self, folder: &str); pub fn up(&mut self);
}
```

`common::vfs::list_entries(dir) -> io::Result<Vec<DirEntry>>` (natively `read_dir` without
following symlinks, like `list_files`; on the web the directories implied by the stored paths,
`.keep` entries included), `vfs::create_dir_all` (on the web: a `.keep` entry — ruling (i)),
`vfs::exists`, and in 4b `vfs::remove_file`, `vfs::remove_dir_all` (refusing the asset root and
anything outside it) and `vfs::rename` (refusing an existing target on both platforms — native
`std::fs::rename` would overwrite it silently, gemini F8; natively **two algorithms**, round 3's
kimi F8, gemini F2, codex F2 and F11, tightened in round 4: a **file** is `std::fs::hard_link(from,
to)` — fails if `to` exists — then `from`'s inode and length are compared with `to`'s and `from`
is removed only when they match, so an external tool's atomic replace of `from` before the check
survives as a visible duplicate; the two adjacent syscalls after the check are ruling (j)'s
residual (codex F1); **where hard links are unsupported the rename is refused** — "this
filesystem cannot rename safely; rename in your file manager" — because a copy-then-remove unlinks
a source an external tool may have just replaced (kimi F1); a **directory** checks that `to` does
not exist and refuses if it does — POSIX `rename` would replace an *empty* one, which is the
visitor's structure too (gemini F4) — then `std::fs::rename`, which the OS refuses onto a file or a
non-empty directory; its residual is a directory *created* between the check and the rename,
replaced only if empty; no new dependency) — each with the web half over the
`MemoryStore`/IndexedDB map (`remove_prefix` is the model). The write observers
(`set_write_observer` / `set_remove_observer`) fire for each, so the playground's persistence and
the file watcher see them. **Ordering on the web** (kimi F3, codex F4): a rename queues every
destination put before any source remove — per file, and across a folder — so a persistence
failure between them leaves a visible duplicate, never a loss; there is no journal, and the
panel's doc comment says a duplicate is the recovery. A memory-store failure-injection test
reloads after each boundary.

**Companions are not tiles.** `kind_for_path` calls every `.ron` a scene (`asset_browser.rs:183`),
so today each `.sheet.ron` and `.mid.ron` is a scene tile that opens sheet syntax in the scene
loader (kimi F14 — pre-existing). 4a classifies `.sheet.ron` and `.mid.ron` as **companions**
before the generic `.ron`: they are not tiles; the PNG's tile carries a "sheet" badge when its
sidecar exists and the song's tile stands for its pair, as the composer already treats it; a
badged tile's context menu offers *Open sidecar in <tool>* (`FileType::of` keeps `Ron` for the
external-tools route). A `.keep` is never a tile, but it **is** exported: the archive importer
drops directory entries (`archive.rs:255`), so the marker file is what carries an empty folder
through export and import (kimi F11); the test is a round trip of an empty `scenes/levels/`.

The panel: a **breadcrumb** row replaces the count in the header (`Assets ▸ sprites ▸ props`,
each segment a button; the count moves to the status line of the row), a **folder tile** per
subfolder (an icon from `icons.rs`, `Icon::Folder`; double-click or Enter enters; Backspace or the
`..` tile goes up), then the files of the current folder only. The selection, the keyboard walk and
the drag are untouched in kind. A filter by kind is not in this batch.

**Create (4a).** A **New ▾** header button and the empty-space right-press open one menu: *New
Folder…*, *New Scene…*, *New Script…*, *New Sheet sidecar* (enabled on a selected `.png` without
one; default cell = the image's size, a one-cell sheet), *New Song sidecar* (enabled on a selected
MIDI file without one — `is_midi_path` names `.mid` and `.midi`, `external_tools/file_type.rs:173`,
kimi F6 round 4 — `song init`'s contract, `EDITOR_COMMAND_API.md:193`; File ▸ New Song and
`EditorAction::NewSong` go with External Tools batch 7, `external-tools/plan.md:1626`, so there is
no score maker to call — codex F13 round 3; and none is wanted: the composer pivot moved
composition to MIDI Evil, so the file the engine *generates* for a song is the sidecar, and this
is its creation — kimi F7 round 4, rebutted; a MIDI file arrives by the web bar's import or by a
copy into `music/` natively), then the two Open items (and in 4b *Rename…*, *Delete…*
before them). Each *…* opens a `PromptDialog` — one name field, Create/Cancel, on the Modal band,
Enter confirms, Escape cancels, built on `confirm_dialog.rs`'s shape and `text_field.rs` — which
validates against the convention (snake_case, the kind's extension appended if missing) and
refuses an existing name — and the write itself is **`vfs::write_new`**, an exclusive create
(`OpenOptions::create_new` natively; on the web, exclusivity is the absence of a *live* file and
the put carries the path's current base revision, so a name deleted earlier — a tombstone at
revision 1 — is recreated by the store's own compare-and-swap rather than refused at revision 0,
kimi F4 and codex F3 round 4), because an existence check followed by `std::fs::write` replaces a
file an external editor made in between (codex F5 round 3); every creation, sidecars included,
takes it. **Where a new file lands** (gemini F4): in the *current folder when it
lies under the kind's root* (`scenes/levels/` for a scene made there), else in the kind's root
(created if absent) — the convention decides the kind's root, the author decides the subfolder. A
new *folder* lands in the current one. **A companion lands beside its primary**, named from it
(`sprites/props/hero.png` → `sprites/props/hero.sheet.ron`), never routed (kimi F8). Templates: a
scene is `SceneData(name: "<name>", entities: [], prefabs: {})` through the serializer (not a
string); a script is the `update(me, view, params, cmd, dt)` stub from `docs/SCRIPTING.md` with the
`@param` comment line; a sidecar is `SheetFile` v1 through **`write_sheet_file(&SheetFile) ->
String` in `sheet_file.rs`**, which does not exist today (the only serialisation is inside a test,
:188) and is part of 4a with a round-trip test. After a mutation succeeds: rescan, select the new
tile, reveal it, status line "Created scenes/level2.scene.ron". **A failed mutation** (permission
denied, disk full, a parent half-created — kimi F13) keeps the prompt open with the error text
under the field; the browser's state changes only after the vfs call returned `Ok`; a folder
operation that did part of its work names what it did.

**Corrected after batch 4a (2026-11-05, rounds 11–12 and the site's rounds 1–2).** `write_new`'s
native half removes the half file a failed `write_all` leaves, by inode and device (Unix) so a
replacement another program put at the path in the window is never touched; elsewhere the half
file stays and the error names it. The atomic form — a temporary sibling published with
no-overwrite semantics — needs the hard-link primitive 4b introduces and lands there. The
`pending_folders` cache above is gone (it resurrected a folder deleted outside the editor);
`list_entries` follows the configured root as `list_files` does and never a symlink entry below
it, and one unreadable entry is skipped, not its folder. The site's drain is
`playground_poll_file_changes()` (writes and creations only, until 4b records removals and
renames), and the scripts panel never replaces or clears a dirty buffer on a refresh, reloads a
clean one only when the bytes differ (its own Save fires the observer too), and clears only the
error its own refresh wrote. **The playground bundle is never rebuilt in place:** the site's
`README.md` ("Don't overwrite builds in place — bump the version folder") and `build_wasm.sh`'s
refusal of a populated target make every rebuild a new version folder — 4a shipped **v3** (the
handoff had said v2, the planner's error), so every "v3" this section and batch 6 wrote before
2026-11-05 reads as *the version after the previous batch's*.

**Rename and delete (4b).** On a selected tile or folder: *Rename…* (the prompt, pre-filled) and
*Delete…* (the confirm dialog, the folder's file count named). **Pairs move together** (gemini F5,
kimi F9, codex F7): a `.png` with its `.sheet.ron`, a `.mid` with its `.mid.ron` and any
`<stem>.recovered-*.mid` are one asset — one rename, one delete, one preflight for a name
collision across the whole family. A family rename that fails on a later member rolls the earlier
ones back; a rollback that fails too returns an explicit result naming both paths and the browser
rescans from disk; an unpaired companion is an **orphan tile** with a badge, never hidden
(gemini F11, codex F8 — a hidden orphan cannot be renamed or deleted from the editor). **An audio
family includes its provenance** (codex F14 round 3): renaming a sound or a song rewrites the
stanza naming it in the folder's `LICENSE`, and `check_audio_provenance.sh` is 4b's gate; the
stanza is **preflighted** before anything moves (gemini F7 round 4) — no stanza means nothing to
carry and the rename proceeds (the provenance check already fails that file today), an unwritable
`LICENSE` refuses the rename before the first step. A song family is `is_midi_path` plus
`<song path>.ron` and its `.recovered-*` files, so `theme.midi` + `theme.midi.ron` is one asset
too (kimi F6 round 4). **On the
web, a delete or rename of a path only the bundle holds writes a tombstone** (round 2 codex F7,
accepted and not written down — round 3 kimi F5, codex F10): `Chains::on_vfs_remove` returns for a
path without a chain (`persist/queue.rs:90-95`) and the store treats an absent record as a no-op,
so today the bundled PNG comes back at reload; 4b creates the chain and the tombstone, tested
through the observers and a bundle-plus-store reload. **The Scripts dock panel is coordinated before a mutation** (kimi F6 round 3, codex F8 round
4): it keeps `currentPath` and lists files once at `enable()` (`playground-scripts-panel.ts:20,36`),
and a message *after* the move cannot stop a save the visitor makes in between — so the bridge
calls a JS hook the panel registers (the write observer's shape) with the affected paths *before*
the move: the panel settles or refuses its open buffer (a dirty open script refuses its delete and
asks before its rename), blocks an intervening save, and is handed the new path in the same call;
a folder rename or delete lists every script under it; a create refreshes the list — a site half
of 4a and 4b, in `insiculous_web`. **Open documents** (kimi F9, codex F8): renaming the open scene
or song, or a folder above it, first asks *Save and rename · Cancel* while the document is dirty
(gemini F5 round 3 — the on-disk file would move with the old bytes under the dirty buffer), then
updates the document's path so the next Save writes the new one;
deleting the open scene or song, or a folder above either, is refused naming the document. **The
reference scan** runs *before* the rename — a recursive scan of every `.ron` and `.rhai` under the
asset root for the old relative path — and when it finds any, the prompt becomes a confirm: "3
files name the old path: scenes/level1.scene.ron, … — *Rename anyway* · *Cancel*" (gemini F6
round 4: a warning after the move gave no chance to stop it); it says what it scanned, and a clean
scan renames at once.

`panel_renderer/asset_browser.rs` is at 583 lines: the header (breadcrumb, New, Assign) moves to
`asset_browser/header.rs` and the mutations to `asset_browser/mutate.rs` first.

**Tests.** `vfs`: `list_entries` lists folders and files, follows no symlink, and on the memory
store derives folders from paths; `create_dir_all` on the web writes the `.keep`; `exists`; (4b)
`remove_dir_all` refuses the root and a path outside it, and removing `sprites/props` leaves
`sprites/props.png` (the slash `remove_prefix` already appends, `vfs/mod.rs:396`); `rename`
refuses an existing target, fails closed when `to` appears between the check and the link,
moves the pair, rolls the PNG back when the sidecar's step fails, and on the memory store a
failure injected after the first put leaves both copies and after the last put leaves none of
the old; an orphan `.sheet.ron` is a badged tile. `asset_browser`: `visible_tiles` at the root shows
the folders then the root files only, companions and `.keep` as no tile; the badge follows the
sidecar's existence; the router puts a scene made in `scenes/levels/` there and one made in
`sprites/` into `scenes/`; a companion lands beside a nested PNG; a name with a space or a capital
is refused naming the rule. `editor_integration`: New Scene writes a file the loader loads; New
Script writes a file the script runner compiles; New Sheet sidecar writes a file
`parse_sheet_file` reads back equal; a failed write keeps the prompt open and the browser
unchanged; (4b) Delete of the open scene and of its folder are refused; Rename of the open scene
moves the Save target; the reference scan finds a nested scene and a script. Stills for codex: the
browser at the root and inside `sprites/`, the New menu open, a badged tile.

**Leaves out.** Drag a tile into a folder (a follow-up issue); multi-select; a kind filter; undo of
file operations (file mutations are not commands — stated in the panel's doc comment); a
persistence journal (ordering is the contract).

### E. The editor in its own window (web)

**Settled 2026-11-11 under ruling (h′), after six rounds in `review/editor-ux-window/`** (kimi
and codex each round; round 6: codex "ship after addressing F1–F2", kimi two criticals in the
store and none in the move — all six folded in; the text below has not been read by a round
since, and its batch section is re-verified against the tree before its handoff, as every
batch's is).

**Why this is not the transfer v6 carried.** Draft 1 handed a live editor from one page to another while both ran: an invitation, a queued
lock, a freeze, a retire, a commit point, a deferred load, a rollback. Four plan rounds found a
critical in it each time, and round 1 of this subject found eight more, every one checked
against the tree: the deferred load reads files under a manifest flag taken before the source
saved and deletes them as an interrupted import (`web_entry.rs:170–222`); a rollback after
`drain_then_epoch` leaves `Chains.draining` set, so the thawed source cannot save
(`persist/mod.rs:333`, `persist/queue.rs:62`); the freeze does not reach commands already in the
bridge's channel or an import already awaiting IndexedDB; a canvas text field has no
commit-on-focus-loss path to "settle" (`ui/src/context/edit_field.rs:171`); *Take over* retires
a memory-backed source's only copy; the six game bundles carry none of the new exports. Each
needs a new seam, and each new seam is the next round's critical.

**The observation that removes the mechanism:** the Playground already hands a project from one
page load to the next, safely, every time a visitor switches project — the embed asks about
unsaved edits, calls `playground_open_project` (which is `drain_then_epoch`, `bridge.rs:286`),
and navigates (`playground-embed.ts:350–375`). The engine's event loop cannot be stopped once
booted (`playground-embed.ts:111–118`), but it does not need to be: a page that navigates away
is retired by the browser. So the window is **that same flow with a different destination**, and
the one thing it lacks — two pages must never run the same project at once — is a lock taken
**before the engine boots**, in the page's own script, where nothing has been read yet.

What that buys: no freeze, no retire, no deferred load, no `editor_integration` seam; no
protocol — no invitation, no generation, no commit point, no rollback, **no message between
pages at all**; and the game surfaces need no ownership, because a game page stores nothing.
The engine's share is small and none of it is machinery for the move: two read-only exports,
boot reads that fail instead of deleting, one more term in the dirty flag, and the new bundle
version folder those require (§ What changes where).

**What rounds 2 and 3 taught, and draft 4 does about it.** Every finding of those rounds landed
on something draft 2 had kept from the transfer: a queue (a waiting tab granted ahead of the
window), a named window (re-opened, it navigates whatever that window has become), a broadcast
(a return posted to nobody, or to two parked tabs), a failure signal (one startup path that
never raises it). Draft 4 keeps none of them. Pages do not talk to each other; the lock is the
only shared thing, and a person closing a window is the only way a project changes hands
backwards.

#### The lock

`navigator.locks.request('beinsiculous.playground:<slug>', { ifAvailable: true }, hold)`, made
by the embed script **before** `wasm.default()` (`playground-embed.ts:536`), on every Playground
editor boot: the page, the window, a reload. `hold` returns a promise the page never resolves,
so the lock lasts the document's life and the browser releases it when the page navigates or
closes, whatever the path. **No page ever waits in the lock's queue** (round 2 codex F5, round
3 kimi F2, F3, codex F5: a queued request outlives the intent that made it and is granted ahead
of whatever the visitor asks for next).

- **The slug is fixed before the request, or the page does not boot** (round 2 kimi F3). A URL
  without `?project=` lets the engine pick the first bundled project after boot
  (`web_entry.rs:145–168`), so the embed resolves it first: it fetches the bundle's
  `projects.json` (the list the engine reads), takes the first slug, and
  `history.replaceState`s `?project=<slug>` onto the URL before anything else — the engine then
  reads the same parameter the lock was named from. **A fetch that fails, does not answer in
  10 s (an `AbortSignal`; round 6 kimi F4), or a list that is empty or will not parse, stops the
  page before the engine** with "The project list did not
  load — *Try again*": a guessed name would lock one project while the engine, whose own fetch
  may succeed a moment later, opens another (round 4 kimi F3, round 5 codex F2). A slug the
  engine does not know is redirected by the engine as today (`location.set_search`), which is a
  new boot with an explicit parameter.
- **Granted** → the grant is kept for the document: the embed's existing *Try again* after a
  rejected `wasm.default()` (`playground-embed.ts:552–554`) boots under the lock the page
  already holds and never asks a second time (round 4 kimi F5). The boot continues exactly as
  today, and everything the engine reads from the
  store — the manifest list, the files, the orphan sweep — is read after the grant, because the
  engine has not started.
- **Not granted** → the **owner card** in place of the loading status: "*Pong* is open in
  another tab or window. Close it, then *Try again*." with **Try again** (the same request once
  more) and **Close this page** (a window closes itself; a tab navigates to the plain parked
  card). The card is an `aria-live="polite"` region.
- **A boot that arrives by a hand-off** — the URL carries `&move=1`, set by *Open in a window*,
  by *Bring the editor back here* and by the post-drain recovery navigation, the three places
  a page is sent to a project that was just saved elsewhere — may find the lock still held,
  because its source is still unloading. It shows
  "Moving the editor…" and asks again every 100 ms for at most 5 s (a bounded count of
  `ifAvailable` requests, never a queued one), then falls to the owner card like any boot. The
  embed strips `move` from the URL at `playground-ready` (`history.replaceState`), so a reload
  is an ordinary boot — not earlier, because the engine reads it once (below).
- **A page whose engine fails to start keeps its lock until it is closed.** There is no
  failure signal to release it on — the engine reports a failed start as status text from more
  than one place (`web_entry.rs:79–82`; the renderer's, `engine_core/src/game/web.rs:125`,
  after `wasm.default()` has resolved; round 3 codex F3) — and none is added: a failed page is
  closed like any other, which is what every card already asks for.
- **The lock API absent or throwing** (an old browser, a policy): the page boots as it does
  today, *Open in a window* is disabled with its reason in the bar note, and `WEB_PLAYGROUND.md`
  says the one-page guarantee holds among pages that have the API.
- **Import of an archive that names another project is fenced** (round 2 codex F3: the store's
  revisions do **not** catch it — an import stamps every file revision 1, `archive.rs:376`, so a
  page holding that project at revision 1 overwrites the import with no conflict). A new
  read-only export `playground_archive_slug(bytes) -> Result<String, JsValue>` (the manifest's
  slug through `archive::import_project`'s own validation) tells the embed the target before
  anything is written; when it is not the open project, the embed takes that project's lock
  with `ifAvailable` and runs `playground_import_zip` inside the callback — refused with "*X* is
  open in another tab or window — close it before importing over it" when the lock is held.
  The revision-1 collision itself is a store bug that exists without this batch; it is filed,
  not fixed here.
- **Not fenced, stated:** a page from the release before this one holds no lock; it ends when
  that tab reloads.

Project switching inside the Playground — in the page or in the window — is already a
navigation (`?project=<slug>`), so the new boot asks for the new slug's lock as any boot does,
and the old slug's lock went with the old document.

#### Open in a window (the Playground)

The bar button **Open in a window ↗**, beside Play ↗, enabled on `playground-ready` when the
lock API exists and the engine's store persists (below). On click, in order:

1. **Synchronously**, before any `await`: if `isDirty()` (`playground-embed.ts:143` — the
   engine's flag, pending persistence, or the Scripts panel's buffer) the bar note says "Save
   first — the window opens the saved project" and nothing else happens. Otherwise `handle =
   window.open('about:blank', '_blank', 'popup,width=1280,height=800')` — **always a new,
   unnamed window** (round 3 codex F2: a window named for the project is still that window
   after the visitor switches project inside it, and re-opening the name would navigate
   whatever it has become). The click's activation is spent here; `null` → the static
   `#window-blocked` alert with *Try again*, the preview's pattern
   (`playground-preview-launch.ts`), and nothing else happens.
2. The page covers itself with an inert "Moving the editor to its window…" panel — every bar
   control, the Scripts panel and the console are under it, so the page submits nothing more —
   and **settles what the bridge has already accepted**: it sends one read-only query line
   (`selection`) through `playground_dispatch` with a sink of its own and waits for that
   line's response. The bridge answers in the order it accepted (`responseSinks`,
   `playground-embed.ts:206–210`, polled on a 100 ms interval that hidden tabs slow but do not
   stop), so when the barrier's answer arrives, every command accepted before the click has run
   — however many updates that took (the engine takes at most 256 a frame,
   `editor_game/api.rs:215`, on a 100 ms timer when idle). Nothing here waits on an animation
   frame: a popup that opens as a foreground tab hides the source, and a hidden document's
   frames do not come (round 3 codex F1; round 4 kimi F2, codex F3, F4). Then `isDirty()` is
   read again. Dirty, the barrier refused (the channel full), or no answer in 5 s →
   `handle.close()`, the panel goes, and the bar note says which.
3. `await wasm.playground_open_project(slug)` — the drain the project switch runs. It rejects →
   `handle.close()`, the panel goes, the banner shows the error: the project-switch branch's own
   failure path (`playground-embed.ts:371–375`), unchanged.
4. **If `handle.closed`** — the visitor shut the blank window during the drain — the page
   navigates itself to `/playground/?project=<slug>&move=1` instead of parking under
   a card that names a window that does not exist (round 4 kimi F6). Otherwise `handle.location
   = '/playground/window/?project=<slug>&move=1'` (in try/catch, the same fallback on a throw),
   `leavingByChoice = true`, `window.location.replace('/playground/parked/?project=<slug>')`.
   The panel carries one link, **Reload this page**, for the visitor who stops that navigation:
   after a successful drain the page cannot save again (`persist/queue.rs:62`), and the panel
   is what they are left with, not a live editor that silently drops saves (round 2 codex F4).

**"Everything saved" has three parts, and each has an owner.**
- *Nothing else is in flight.* The embed's imports, archive import, reset and project switch
  each run to completion under one shared in-flight count (the import path already disables
  the transition controls, `setTransitionControlsDisabled`; a file import does not — it joins),
  and the button refuses while the count is not zero: "Wait for the import to finish" (round 4
  kimi F1: a file read that resolves after the drain writes bytes persistence refuses).
- *An open command batch is unsaved work.* `batch begin` followed by an edit applies the edit
  and keeps it out of the history until `batch end` (`command_api/write/mod.rs:49`), so the
  dirty mirror (`editor_game/mod.rs:233`) reads clean over a changed world (round 4 codex F2).
  The mirror gains one term — an open API batch holding at least one command is dirty — which
  also makes the page's existing `beforeunload` warning tell the truth about it.
- *The store agrees.* `isDirty()` already covers pending persistence, and the drain is step 3.

There is no step in which both pages edit, and nothing to roll back after step 3: from the
drain on, the source is leaving, as it is in a project switch.

**Storage that does not persist.** The engine falls back to an in-memory store when IndexedDB
will not open (`web_entry.rs:95–103`) and keeps that choice for the life of the page, so the
button asks the engine, not the browser (round 2 kimi F1, codex F1). A new read-only export
`playground_storage_persists() -> bool` answers from the store the engine chose; `false`
disables the button with "This browser is not keeping your edits, so the editor cannot move —
it would lose them" in the bar note. **A hand-off's destination does not fall back** (round 3 codex F4, round 5 kimi F2, round 6
codex F2 — the return and the recovery navigation carry `move=1` for this reason, or the
prescribed way back would itself open the bundled project in memory): an engine boot whose
URL carries `move=1` and whose IndexedDB open fails stops there — before project
selection, so an imported-only project is not redirected to a bundled one — with the
startup-failed status "This window could not open your saved project. Close it and choose
*Bring the editor back here* on the page." The saved project is untouched in the store.

**A boot that cannot read the store stops; it never deletes and never substitutes** (round 1
kimi F1, codex F1; round 4 kimi F4, codex F1 — raised in every round, and a prerequisite now:
after a move the source's copy is gone, so the destination's boot is the only reader). Today
`IndexedDbStore::manifests` turns a failed read into an empty list (`indexed_db/mod.rs:452–462`)
and a record that will not decode is skipped, the boot then takes stored files without a
manifest for an interrupted import and **removes them** (`web_entry.rs:193–222`), and
`load_project(...).unwrap_or_default()` (`:193`) turns a failed read into the bundled project.
This batch makes the reads strict, and scoped to the project being opened so one bad record
does not lock a visitor out of the others (round 5 codex F3):
- `ProjectStore::manifests` returns `Fut<Result<ManifestList, StoreError>>`, `ManifestList {
  manifests, undecodable: Vec<String> }` — the keys of records that would not decode (four
  implementations, one caller, `web_entry.rs:142`). A failed read is an `Err`.
- `load_project` returns an `Err` for a record of that project whose value will not read or
  decode — today `load_project_cursor` skips it and answers `Ok` (`cursors.rs:85–89`; round 5
  kimi F3, codex F1) — and `put` refuses to write over an existing record it cannot decode
  instead of treating it as revision 0 (`indexed_db/mod.rs:278`).
- The boot reads the manifests **first** (today the sweep runs before them,
  `web_entry.rs:134–142`) and stops with the existing "Startup failed: …" status, **before
  `sweep_orphans`, before the unknown-project redirect, before the interrupted-import removal,
  before any write**, on: a failed manifest read; a failed or undecodable file read; **a
  requested slug that is not bundled and not in the decoded list while any manifest was
  undecodable** (it may be this one — today that redirects to the first bundled project,
  `web_entry.rs:151–164`; round 6 kimi F3, codex F1); or stored files for the opened slug with
  no manifest while any manifest was undecodable.
- `sweep_orphans` takes the validated list — `sweep_orphans(&self, known: &[String])`, the
  bundled slugs and the decoded manifests' — instead of reading the projects store a second
  time with a callback that ignores a failed read and then deletes every non-bundled file
  (`cursors.rs:261–268`; round 6 kimi F2); it runs only when nothing was undecodable, and its
  own `Result`, discarded today (`web_entry.rs:139`), stops the boot.
- **`replace_project` never moves a revision backwards** (rounds 2, 5 and 6 — codex F3, kimi
  F1 twice): an import stamps every file revision 1 (`archive.rs:376`) and the stores write it
  as given, so a page holding the project at revision 1 saves over the import with no conflict
  — the one path a tab from the release before this one, which holds no lock, can still take.
  Each store stamps a replaced project's files one above the highest revision any record of
  that project held, tombstones included; a path the import drops is absent, which no non-zero
  base matches. A stale page's next save is `StaleRevision`, as the queue already reports it. The interrupted-import removal stays for the case it was written for: a
  complete, successfully read list that does not name the project.
- Tests: over `MemoryStore` with read-failure injection — a failed manifest read and a failed
  file read each stop the boot with every record intact; two healthy projects beside one
  undecodable manifest both open and the third's files survive; and **over the real IndexedDB
  store in the browser test** — a file record with malformed metadata and intact bytes stops
  the boot and is byte-identical afterwards; an imported-only project whose own manifest is
  undecodable stops the boot with its URL unchanged and its files intact; a failed read inside
  the sweep deletes nothing; a page holding a project at revision 1 gets `StaleRevision` after
  another page imports over it.

**A canvas text field mid-edit** (a name being typed, not yet committed) is not a dirty
document, and clicking a bar button does not commit it — exactly as with the project switch,
Export, or a reload today. The window opens the saved project; `WEB_PLAYGROUND.md` § The editor
window says so in one sentence.

#### The window

`pages/playground/window.astro` renders `PlaygroundEmbed` + `PlaygroundToolbar` inside
`AppLayout` with a new `windowed` prop: the studio wordmark nav, the footer and the visible
heading row go (the `h1` stays, visually hidden — every page keeps its name for the audits), the
Aa controls and the skip link stay, the bar is the window's top edge, and `main` fills the
viewport. `noindex`; the route carries `data-wasm-src` like the others, so `postbuild-check.mjs`
resolves it. The dock (scripts, console) travels with it — it is the accessible half of the
editor. In the window the bar has no *Open in a window*; its note reads "To move the editor
back: close this window, then choose *Bring the editor back here* on the page."

The window is an ordinary editor page: it takes the lock, boots, edits, saves, switches
project. Its `beforeunload` warns while anything is dirty, as the page's does today.

#### Coming back

**The parked page** (`pages/playground/parked.astro`, static, `AppLayout`, no engine, no
script beyond one link): "The editor for *Pong* is open in its own window. To work here again,
close that window, then:" **Bring the editor back here** → `/playground/?project=<slug>&move=1`. The
window closed → the editor starts. The window still open → "Moving the editor…" for its 5 s of
asking, then the owner card,
"Close it, then *Try again*". Nothing is asked of the window and nothing is taken from it; the
parked page listens to nothing and decides nothing — not on silence, a timer, `pagehide` or
`visibilitychange`. Opened without `?project=` (the owner card's *Close this page* in a tab) it
is the plain card "This page is closed — *Open the Playground*".

#### A Rust game's editor page

`pages/playground/[slug]/window.astro` renders `GameEmbed` with `fill` + the game page's bar in
the same `windowed` layout. A game page has no store — its edits live in that page's memory and
nothing persists — so there is nothing two pages could write over: **it takes no lock, parks
nothing and needs nothing from the game's bundle.** *Open in a window ↗* on a game page is
`window.open('/playground/<slug>/window/', '_blank', 'popup,…')` — a new window every time,
never a named one, so a second click cannot navigate a window that holds unsaved edits (round 2
kimi F2, codex F2); `null` → the same blocked alert (round 3 kimi F5). The bar note says "The
window starts from the game's own scene — edits on this page stay on this page." The source
page keeps running. No dialog, because nothing is lost.

#### What changes where

`insiculous_web`:
- `src/layouts/AppLayout.astro` — the `windowed` prop.
- `src/pages/playground/window.astro`, `src/pages/playground/[slug]/window.astro`,
  `src/pages/playground/parked.astro` — new.
- `src/components/PlaygroundToolbar.astro` — the button, the window's note, the blocked alert,
  the owner card's and the two panels' markup. `src/components/GameEmbed.astro` — the game
  page's button and note.
- `src/scripts/playground-project-lock.ts` — new, pure where it can be: the lock name, the slug
  resolution and its refusal, the `ifAvailable` request, the move boot's bounded retries, the
  absent-API path, the fenced import. Unit-tested with a fake `LockManager` and a fake `fetch`.
- `src/scripts/playground-window.ts` — new: the open click's four steps over injected `open`,
  `barrier`, `isDirty`, `inFlight`, `drain`, `navigate`; the storage question on both sides.
  Unit-tested with fakes: dirty or in-flight refuses before `open`; a blocked popup stops before
  anything; dirty after the barrier, a refused barrier and a barrier that never answers each
  close the handle; a failed drain closes the handle and leaves the page; a handle closed during
  the drain sends the page to the ordinary editor; the order open → barrier → drain → window
  location → own navigation.
- `src/scripts/playground-embed.ts` (557 lines) — the lock awaited before `wasm.default()`, the
  button's wiring, the import's fence, the shared in-flight count, the barrier's sink. It must not cross 600; the new logic lives in the two
  new files.
- `scripts/postbuild-check.mjs` — the new routes, if they need naming. `README.md` § the
  playground, `docs/roadmap.md` one line.

`insiculous_2d`:
- `crates/playground/src/bridge/window.rs` (new; `bridge.rs` is 548 lines) —
  `playground_storage_persists()` and `playground_archive_slug(bytes)`, both read-only.
- `crates/playground/src/store.rs` and its four implementations (`memory.rs`, `directory.rs`,
  `indexed_db/mod.rs` + `cursors.rs`, the test-only `GatedStore`) — `manifests` returns a
  `Result<ManifestList, _>`, `load_project` and `put` no longer read an undecodable record as
  absent, `sweep_orphans` takes the known slugs, `replace_project` stamps above the project's
  highest revision; `web_entry.rs` — the strict reads above, the `move=1` boot that does not fall back,
  and the store choice kept where the export reads it.
- `crates/editor_integration/src/editor_game/mod.rs` — `sync_dirty_mirror`'s one term for an
  open API batch, with its test.
- **A NEW bundle version folder** (the site never rebuilds a deployed one in place — the
  version after the newest deployed, v6 once batch 6's v5 has landed) with the five version
  places and the site's three pins.
- `docs/WEB_PLAYGROUND.md` — § The editor window (the lock, the move, coming back, the fenced
  import, what is not fenced) beside § The preview window; the bridge table's two rows; § The
  store's line on a boot that cannot read.

**No freeze, no retire, no host handle; `persist/` and `engine_core` untouched;
`EDITOR_COMMAND_API.md` unchanged.**

**A cost, stated** (round 5 kimi F4): nothing reserves the window's turn. A second tab sitting
on its owner card whose *Try again* is pressed in the moment between the source parking and the
window asking gets the project, and the window shows the owner card. Everything is saved;
closing that tab and *Try again* in the window finishes the move. A reservation is the queue
rounds 2 and 3 removed.

#### Gates

The engine's standard set, `check_games.sh` (`editor_integration` changes) and `check_wasm.sh`; `npm run verify`
whole — axe over the three new pages, the phone and 125 % shots, the announce check (the owner
card and the two panels are live regions; every button is reachable and named by keyboard
alone). The two new scripts' unit tests in the site's existing test harness
(`tests/playground_scripts_panel.mjs` is the shape). **One browser test with real Web Locks and
real IndexedDB**, pages in one context, driven the way `insiculous_web#80`'s boot check is
(headless Chromium over the built site): page A boots, edits, saves; *Open in a window*; the
window shows the saved edit and A is the parked card; the window edits and saves and is closed;
*Bring the editor back here* shows the window's edit; with the window still open, the same
button meets the owner card and *Try again* starts it once the window is closed; a second tab
on the same project meets the owner card; `/playground/` and `/playground/?project=<first
slug>` in two tabs meet the same lock; a page whose engine fell back to the memory store has
the button disabled; an archive naming a project open in another tab is refused; **the move
made with the source tab hidden the moment the window opens** (the barrier must not need a
frame); a command batch left open refuses the move. If the harness
cannot host several pages the executor says so and the run is Jesse's check. Jesse's Chromium
check besides: the blocked-popup path on both surfaces; the game page's independent window.

Reviewers: kimi and codex (the lock and the move; the window, the parked card and the owner
card — three stills), the planner's own review beside them.

#### Leaves out

*Take over* and *Move back* — asking a live page to give up its project, or one page telling
another to start (both are the transfer again); *Go to the window* (the browser's own window
list does it, and a named window is what round 3 broke); releasing a failed page's lock; any
freeze, retire or snapshot of a running editor; a recovery card for a boot that cannot read
(the startup-failed status is the answer); a game-page transfer of unsaved edits (a game
page persists nothing — ruled 2026-11-03); opening a *panel* in its own window; remembering the
window's size and position; a heartbeat, a channel, or any decision made on a page's silence.

## Ground rules for every batch

- Branch `jesse` in `insiculous_2d` (and in `insiculous_web` for batch 5). One batch out at a
  time, across both efforts (ruling (e)).
- The executor stages everything it touched, new files included, commits nothing, and writes
  one report at `review/editor-ux/report-<batch>.md` (`../insiculous_web/review/editor-ux/` for
  the site half of batch 5), ending with `git status --porcelain -- <scope>` and `git diff
  --cached --stat | tail -3`. INCOMPLETE is a valid ending.
- Gates, mechanically: `cargo test --workspace` (0 failed, 0 ignored); `cargo clippy --workspace
  --all-targets` (0 warnings); the comment-tag grep from `CLAUDE.md` § Solo Session Guardrails
  rule 5; `scripts/check_guide_tests.sh`; `scripts/check_games.sh` whenever the staged diff touches
  `crates/editor`, `crates/editor_integration`, `crates/ui`, `crates/common` or `crates/engine_core`
  (every game builds with `--features editor`); `scripts/check_wasm.sh` whenever it touches
  `crates/engine_core`, `crates/renderer`, `crates/audio`, `crates/input`, `crates/common` or
  `crates/ui`; `scripts/check_project_layout.sh` from batch 6 on; `wc -l` ≤ 600 on every touched
  `.rs`; a gate that cites lines not on disk is a stale artifact, cleaned with `cargo clean -p`
  on the named crates (a game's own `target/` too) before the gate is believed.
- No `#[allow]`, no `unwrap()` outside tests, no new dependency, no abbreviations, comments carry
  reasons (no issue numbers, reviewer tags or batch names in source).
- Every deletion of a public item shows its grep in the report; every doc that names a changed
  thing (crate guides, `training.md`, `docs/WEB_PLAYGROUND.md`, the site's `README.md`) changes in
  the same batch.
- Reviewers per batch: kimi always; codex with stills for every batch here but the vfs seam of
  batch 4 is also gemini's; gemini for batches 2 and 5 (a ui seam, a page protocol). The planner's
  own review on every batch. A fix that changes behaviour goes back as its own round.
  **Amended 2026-11-05 (Jesse): from batch 2 on the executor is Astra (`codex`), in its own
  session, because the batches are UI work.** A vendor never reviews its own vendor's work, so
  codex leaves the reviewer seat for batches 2–7: the reviewers are **kimi and gemini** on every
  batch, and the UI eye over the stills is **the planner's and kimi's** (Jesse: "she's pretty
  decent at it") — the handoff names the stills' tracked paths so a repo-reading reviewer can
  open them, and the executor's report describes each still in words beside its path.
  `request-review.sh`'s `--image` feeds codex only; whether kimi's CLI reads a PNG from the
  tree is checked at batch 2's dispatch, and a script change to hand kimi images is filed if
  it cannot. Batch 1 ran on the roster as it was (Opus built, codex reviewed).
  **Corrected after batch 2 (2026-11-05):** kimi's CLI cannot open a PNG at all — its Read
  tool answers "is an image file. Only text files can be read" (`review/editor-ux/
  kimi-image-probe.txt`) — so the UI eye over the stills is **the planner's alone**; the
  handoff still has the executor describe each still beside its path. And gemini's round 7
  ran blind (agy's reviewer gate denies the tool names the current build calls; 13 min, 3 of 5
  findings false), so by Jesse's word the same day the second reviewer **from batch 3 on is
  deepseek** (`--reviewer=deepseek`, Claude Code on DeepSeek's endpoint): the reviewers are
  **kimi and deepseek**, the planner's own review beside them.
- A batch section is re-verified against the tree before its handoff is written, and the
  correction committed with its own pathspec.

## The batches

**Batch 0 — planner only.** This plan reviewed by kimi, gemini and codex (round 1, and round 2
over v3 with the rulings folded in), adjudicated with Jesse, revised until settled; the rulings recorded here; one issue per batch filed on
`insiculous_2d` (batch 5's site half on `insiculous_web`) under a new milestone **Sprint: Editor
UX** whose description carries the batch order and the gates, and a board Sprint option; #179
folded into batch 2's issue; `coordination/editor-ux/reviewer-comparison.md` started with the
round-1 rows. The plan commits with `ADV_REVIEWED=1` once round 1 is adjudicated (it is over the
hook's threshold). **Done 2026-11-03:** plan v6 committed (2c4353e); milestone *Sprint: Editor UX*
on `insiculous_2d` (#15) and `insiculous_web` (#14) with the order and gates in its description;
the board's Sprint option added with every existing id preserved (35 → 36, 119 values intact);
issues #240–#248 and insiculous_web#79 filed, fields set, `check-sprint-sync.sh` clean; #179
commented, left open for #241's commit to close.

**Batch 1 — the toolbar as a toolbar (§ A). Issue #240.** Files: `crates/editor/src/icons.rs` (new),
`toolbar.rs`, `toolbar_strip.rs`, `play_controls.rs`, `view_toggles.rs`, `overflow_menu.rs` (its
rows gain the icon before the name), `theme/mod.rs` + `theme/tests.rs` (two tokens, two guard
rows), `crates/editor/CLAUDE.md` (the strip's paragraph), `coordination/PROGRESS.md`. Target
shapes: § A. Gates: the standard set; `check_games.sh` (the editor crate). Reviewers: kimi, codex
with the stills § A names. Leaves out: § A's list. Executor's first step: report the measured
`TOOLBAR_STRIP_MIN_WIDTH` before and after, and the dock's narrow threshold with it.

**Re-verified 2026-11-05 against 564de49 (External Tools done; the tree is clean on `jesse`).**
Every file § What the tree holds names for the strip is where it said, at the sizes it said
(`toolbar_strip.rs` 449, `toolbar.rs` 376, `play_controls.rs` 301, `view_toggles.rs` 303,
`overflow_menu.rs` 331, `theme/mod.rs` 445, `theme/tests.rs` 199); `icons.rs` does not exist;
`Rect::expand`/`shrink` exist (`common/src/rect.rs:132,143`); the eleven theme tokens the strip
and this batch read are all declared (`toolbar_active` :82, `menu_open_highlight` :86,
`shortcut_hint` :90, `play_button_bg` :118, `stop_button_bg` :120, `separator` :124, `surface_1`
:19, `accent_blue` :36, `accent_cyan` :38, `border_subtle` :42, `text_primary` :46). What moved
or was misread, and binds the executor:

- **The ui draws strokes with `line`, `circle` and `rect_border` only** (`ui/src/context/mod.rs:
  349-377`): `circle` is a *filled* disc and there is no arc command. So § A's `Stroke` is
  `Line(a, b) | Disc(c, r) | Ring(c, r)` — a ring is `rect_border` on the circle's bounding
  square with `corner_radius = r` (the stroke width is `weight`); an arc (Rotate's open ring,
  Resume's curve) is a polyline of `line` segments, eight to a quarter turn, and `icon_strokes`
  returns the segments so the inside-the-box test still covers it. No new draw command in the
  ui crate: the chevrons (`dock/render.rs:57-72`) and the overflow dots (`toolbar.rs:249-254`)
  already live on these three.
- **The tool button width is a fixed 64.0** at `Toolbar::new` (`toolbar.rs:101`), with
  `button_height` 30.0 and `spacing` 6.0; the doc comment at :78-80 claims it is sized for the
  longest tool name — it is not, and nothing measures text. § A's "sized for the longest tool
  name at the body size (:79-83)" is corrected to this; the batch replaces all three with
  `TOOL_BUTTON_SIZE` and the segment's divider gap, and the comment goes with them.
- **The strip's tests start at `toolbar_strip.rs:208`**, not :224; the seven test names are
  `test_split_tiles_the_content_area_into_a_strip_and_the_viewport`,
  `test_split_never_yields_a_negative_viewport`,
  `test_every_tool_shows_and_the_play_controls_hold_the_centre_when_there_is_room`,
  `test_the_groups_never_overlap_or_clip_at_the_narrow_width_or_a_comfortable_one`,
  `test_view_group_shows_whole_at_minimum_and_sheds_one_pixel_under`,
  `test_below_the_minimum_width_the_play_controls_take_the_right_edge_and_tools_shed`,
  `test_a_modal_scrim_keeps_the_strips_play_controls_inert`. The four § A calls "the layout
  tests" are the third through sixth. `play_controls.rs`'s pin is
  `test_the_widest_state_is_the_constant_the_strip_budgets_for` (:224, `WIDEST_CONTENT_WIDTH`
  176.0 at :69); `view_toggles.rs`'s is `test_a_toggles_hint_advertises_the_key_that_flips_it`
  (:276); `overflow_menu.rs`'s three tests build their strip from `TOOLBAR_STRIP_MIN_WIDTH - 1.0`
  (:236, :283) and keep passing against the new constant. `dock/mod.rs:16`'s
  `MIN_CENTER_WIDTH = TOOLBAR_STRIP_MIN_WIDTH` is the narrow threshold the first step reports.
- **Tooltips exist on every strip button already** — `toolbar.rs:232` (`tool.hint()`),
  `play_controls.rs:156-200` (the transport's, each ending in its chord), `view_toggles.rs:230,254`
  (`toggle.hint()`, Reset Layout's sentence). The batch *rewrites* them to the one shape
  "Name (Shortcut) — hint", not adds them; `ViewToggle::shortcut` returns `Option` (:65), so a
  toggle without a key has no parenthesis. `test_tool_shortcut_hints_match_the_editor_bindings`
  (`toolbar.rs:286`) reads the shortcut from the caption today; after the batch it reads the
  tooltip text, and `view_toggles.rs:276`'s test does the same.
- **The overflow menu's rows** are `OverflowRowItem { label, hint, checked, kind }`
  (`overflow_menu.rs:45-50`) drawn with `ui.button(id, item.label, row)` (:154-155); the icon
  goes at the row's left and the label shifts right by `TOOL_BUTTON_SIZE` — the menu's width
  constant (:12) grows by the same.
- **Two guides name the numbers that change**: `crates/editor/CLAUDE.md:37` says
  `VIEW_GROUP_WIDTH` is 140 px and `TOOLBAR_STRIP_MIN_WIDTH` 391 px — both are re-pinned to the
  new values in the same batch; `crates/editor_integration/CLAUDE.md:61` describes the strip's
  draw order and stays true, read and left alone unless the batch moves the order.
- **The stills.** No headless draw-list dump of the strip exists in the tree (Playground UX's
  stills were real screenshots; `test_support::frame` drives the input handler only, and
  `GameHarness::ui_commands` is the engine's draw list, not a renderer). The stills are real
  screenshots of `cargo run --example editor_demo --features editor` (1280×720, `editor_demo.rs:
  30`): the strip in each of the three play states at that width, and one at a window narrowed
  under `TOOLBAR_STRIP_MIN_WIDTH` with the overflow menu open — four PNGs under
  `review/editor-ux/batch1-strip-{editing,playing,paused,overflow}.png`, copied to
  `coordination/editor-ux/screenshots/batch-1/` and staged. The 1200 px draw-list stills § A
  names are replaced by these; the `icon_strokes` unit test is the headless half.
- **`editor_prefs.json` is tracked** and an editor run rewrites it; the executor restores it
  after the stills and never stages it.
- **The comment-tag grep's two standing hits** are string literals the batch does not touch:
  `engine_core/src/scene_serializer/dynamic_and_scripts_tests.rs:74` and
  `editor/src/command_api/specs.rs:99`.
- **The code review is round 5** of `review/editor-ux/` (plan rounds 1–4 are spent); reviewers
  kimi and codex with the four stills, the planner's own review beside them. **The executor is
  Claude Opus 5.5 in Jesse's other window**, as for every External Tools batch.

**Done 2026-11-05: 4df27b1** (executor Claude Opus 5.5; code round 5 — kimi 1, codex 1 over the
four stills, the planner 6 — and round 6 on the planner's fix). Widths: `VIEW_GROUP_WIDTH` 140 →
160, `TOOLBAR_STRIP_MIN_WIDTH` 391 → 312. The one accepted finding: the overflow menu, wider by
the icon lane and hung from a button that sits further right, ran past a 250 px window — now it
slides left to the window's edge as it already shifted up (`overflow_menu_bounds`, with a test
through the real layout). Rebutted as policy in round 6: the narrow-mode side overlay covering
the menu is #233, batch 3's. Follow-up lines for the close-out: `row_button` in
`overflow_menu.rs` copies the stock button's painting because `ui.button` draws a placeholder
bar for an empty label (the ui crate's to fix); an `On` strip button shows no hover feedback
(Astra's call). Codex raised nothing on the fifteen shapes.

**Batch 2 — UI scale (§ B), closes #179. Issue #241.** Files: `crates/ui/src/context/mod.rs` (+ the
`to_physical`/`to_points` seam; split `context/scale.rs` if `mod.rs` nears the ceiling),
`context/text.rs`, `context/widgets.rs`, `interaction/` (the tooltip's recorded scale),
`font/layout.rs`; `crates/engine_core/src/contexts.rs` + `game.rs` (`display_scale`, the
`ui_scale_for_engine_tail` request) + `game/frame_tail.rs` (the tail's bracket),
`renderer/src/window.rs` (the web ratio helper, beside `shown_size`);
`crates/editor/src/editor_preferences.rs` (`ui_scale`, `resolve_ui_scale`), `editor_input.rs`
(`update_state` takes the scale; three actions + bindings), `viewport/mod.rs` + `viewport/tests.rs`
(`to_window_render_camera(window_physical, scale)`), `menu/mod.rs` (**the nested dropdown first**,
then the UI Scale submenu — split `menu/dropdown.rs` out; 538 lines today), `view_toggles.rs` is
NOT touched; `crates/editor_integration/src/editor_game/mod.rs` (the bracket; split first — 567
lines), `game_impl.rs`, `viewport_interaction.rs`, `gizmo_drag.rs`, `menu_actions.rs`,
`preferences.rs`; `crates/ui/CLAUDE.md`, `crates/editor/CLAUDE.md`, `docs/WEB_PLAYGROUND.md`
(§ Preferences: the key), `training.md` § Immediate-Mode UI (one paragraph: points). Target
shapes: § B. Gates: the standard set; `check_wasm.sh` (ui, engine_core, renderer);
`check_games.sh`. Reviewers: kimi, gemini (the boundary rule is the seam), codex (the 100 %/150 %
stills and the 390 px playground). Leaves out: § B's list. Standing rule from Playground UX
batch 10b that binds here: a gesture that reads raw mouse state must run the blocking test on its
arming press — at scale, in the same units.

**Re-verified 2026-11-05 against 4d3c0b9 (batch 1 landed; the tree is clean on `jesse`).**
§ B's seams are where it says: `ui/src/context/mod.rs:148 begin_frame_dt`, `:183 window_size`,
`:188 mouse_pos`, `:231 scroll_delta`; `context/text.rs:240-252` the three `measure_text*`;
`font/layout.rs:71` hands `font_size` to `glyph_cache.rs:81 get_or_rasterize`;
`interaction/mod.rs:141 blocking_regions`, `:370 push_blocking_rect`; `engine_core/src/game.rs:
274-287 build_context!`, `:416 window_size`, `:484 update_ui_begin`, `:532` the `post_update` call
(`game/frame_tail.rs:31`, `:119 draw_scene_ui`), `contexts.rs:217 clip_engine_ui`;
`window_manager.rs:232 scale_factor`; `renderer/src/window.rs:136-143 shown_size` reading
`device_pixel_ratio`; `editor/src/editor_input.rs:459-472 update_state` with the one raw
`mouse_position()` at :460; `viewport/mod.rs:256 to_window_render_camera`, `:200 screen_to_world`,
`:216 world_to_screen`, `:113 camera_zoom`; `game_impl.rs:138-147` the camera and the scissor;
`editor_game/mod.rs:342-346` the font swap the scale bracket joins; `menu/mod.rs:524-527` the
dead submenu button; `confirm_dialog.rs:153 panel_rect`; `constants.rs:20 MIN_EDITOR_WINDOW_WIDTH`.
None of `pixels_per_point`, `display_scale`, `points_window_size`, `resolve_ui_scale`, `UiScale`
exists yet. #179 is open. What moved or was misread, and binds the executor:

- **Paths.** The scale-factor event is handled in `engine_core/src/game/app_handler.rs:337`
  (under `game/`); the tooltip lives in `ui/src/context/tooltip.rs` (`:122 tooltip`, `:152
  end_frame_tooltip`), not under `interaction/` — the recorded scale goes there; `WHEEL_STEP` is
  the editor's (`editor/src/scroll.rs:32,54`), not the ui's: the ui's `scroll_delta` stays in
  notches and the editor multiplies, so neither file converts it; `ui_theme()` is at
  `theme/mod.rs:351`.
- **The chrome consumers of `ctx.window_size`**, counted: `editor_game/mod.rs:399
  update_layout` (the dock), `:437 → console.rs:47 draw_status_bar`, `menu_actions.rs:46` (the
  menu bar's width), `scene_confirm.rs:158`, `stop_confirm.rs:68`; the add-component popup
  reads `ui.window_size()` (`add_component_popup.rs:130`) and follows the ui's conversion for
  free. Six sites take `points_window_size()`; `game_impl.rs:138,143` keep the physical size.
- **`engine_core/src/game.rs` is 568 lines — at the ceiling.** `display_scale` on the context and
  the `ui_scale_for_engine_tail` request add to `build_context!` and `GameContext`
  (`contexts.rs`, 287), not to `game.rs`'s body; if a line must go into `game.rs`, split a
  module out first as its own step and say so. `editor_game/mod.rs` is 561 and `menu/mod.rs`
  537 — the splits the section already orders (`editor_game/` first, `menu/dropdown.rs`) stand;
  `interaction/mod.rs` is 517 and gains the point conversion of `is_blocked_at` — watch it.
- **Batch 1's constants are points.** `TOOL_BUTTON_SIZE`, `SEGMENT_DIVIDER_GAP`, `ICON_BOX_SIZE`,
  `ICON_STROKE_WEIGHT` (`toolbar.rs`), `GROUP_GAP`, `TOOLBAR_STRIP_MIN_WIDTH` (`toolbar_strip.rs`),
  `OVERFLOW_MENU_WIDTH` and `overflow_menu_bounds`'s window clamp (`overflow_menu.rs`) ride the
  scale like `layout.rs`'s; `draw_icon`'s `weight` is a scalar length (`to_physical_length`), and
  `icons.rs`'s inside-the-box test holds in points. The dock's narrow threshold is
  `MIN_CENTER_WIDTH` in points against a points window, so at 200 % a 1024 px window is 512 pt
  and goes narrow — § B's *Minimums* says that is designed.
- **The stills, under the new roster** (the executor is Astra; kimi and the planner are the UI
  eye): three real screenshots — the editor_demo at 100 % and at 150 % at 1280×720
  (`editor_demo.rs:30`), and the playground at 390 px via `scripts/build_wasm.sh … --serve` and
  Chromium — saved as `review/editor-ux/batch2-scale-{100,150,playground-390}.png`, copied to
  `coordination/editor-ux/screenshots/batch-2/` and staged, each described in one line in the
  report beside its path. `editor_prefs.json` is restored after every editor run.
- **The code review is round 7**; reviewers kimi and gemini (the ui boundary is the seam), the
  planner's own review beside them; codex does not review its own vendor's batch.

**Done 2026-11-05: dc5a7f9** (executor Astra/codex, `report-2.md` COMPLETE, 54 files; code
round 7 — kimi 3, gemini 5 (2 real, 3 false; its gate denied every repo read), the planner 2 +
the UI eye over five stills, the executor's three and the planner's two of View ▸ UI Scale
(`batch2-menu-scale-{100,125}.png`) — and round 8 (kimi 1) on the planner's fix diff). The
accepted findings, fixed by the planner: a child dropdown opened by the same frame's
move-and-press took that press on its first row (now a press frame opens no child; the release
does); Enter with nothing highlighted fired the first row; the four scale actions are refused
while a pointer gesture is live (gizmo drag, pan, marquee, scrub — round 8 widened it from the
gizmo alone); the scissor snaps from its edges, not `ceil(width × scale)`; `stepped` keeps the
preference past 200 % and below 100 %. Rebutted: a physical-pointer caller of
`is_input_blocked_at` that does not exist, UI Scale "dropped while Playing" (the deny list),
a divide-by-zero no code path reaches. Closes #241 and #179. Filed: #249 — a Follow-display
scale change (the window carried to another monitor) during a live gesture still rebases its
point anchors, cancel or rebase being the design call. Follow-up line for the close-out: the
inner game's key handlers run at the editor's scale (only `update` is bracketed at 1.0; no game
draws from one). The gemini gate's stale tool names are insiculous#45 (comment text in the
planner's scratchpad, the comment itself denied here).

**Batch 3 — the dock: tab groups, the Window menu, the keyboard route, the vertical budget
(§ C). After External Tools batch 7 (ruling (e)). Issue #242.** Files:
`crates/editor/src/dock/{mod,groups,layout,render,focus,tests}.rs` (the split first),
`context/default_layout.rs`, `context/mod.rs` (`show_panel` callers), `editor_preferences.rs`
(`GroupPrefs` + the migration), `menu/mod.rs` (Window menu on batch 2's nested dropdown),
`context_menu.rs` (`DockAction`), `editor_input.rs` (`F6`, the tab keys),
`crates/editor_integration/src/editor_game/{mod,menu_actions,panel_press,composer_keys,
asset_browser_keys,console,settings_keys}.rs`, `panel_renderer/mod.rs`, `crates/editor/CLAUDE.md`
(the dock paragraph is rewritten), `docs/` where panels are named, `coordination/PROGRESS.md`.
Target shapes: § C. Gates: the standard set; `check_games.sh`. Reviewers: kimi, codex (three
stills). Leaves out: § C's list. Re-read #233 and #99's Preferences line in the report.

**Re-verified 2026-11-05 against dc5a7f9 (batch 2 landed; the tree is clean on `jesse`).**
What moved or was misread since § C was written, and binds the executor:

- **Files.** `composer_keys.rs` is `editor_game/conductor_keys.rs` (the Conductor rename): its
  dock writes are `:71-72` (`set_panel_visible`, `set_panel_collapsed`), its `settle_conductor`
  `:87`. The hosts do not each read `get_panel` for bounds and visibility: they all go through
  `panel_press` (`panel_press.rs:35`, the one `get_panel` reader at `:37` and `:48`; `Neither`
  at `:40` and `:56`) — `settle_asset_browser` `asset_browser_keys.rs:40`, `settle_console`
  `console.rs:55`, `settle_settings` `settings_keys.rs:36`. So re-threading the hosts is
  `panel_press` plus the "active tab" rule in each `settle_*`. The other `get_panel` readers
  outside tests: `menu_actions.rs:21` (the View checks), `context/mod.rs:473`
  (`panel_content_bounds`).
- **The dock today**: `dock/mod.rs` 505 lines (`DockPanel` `:87` with `id, title, hint, position,
  bounds, size, min_size, visible, resizable, collapsed` — exactly the set § C moves to the
  group; `DockPosition::Floating` `:82`, matched at `:462` and `render.rs:182`; `MIN_CENTER_WIDTH`
  `:16` `= TOOLBAR_STRIP_MIN_WIDTH`, 312 points since batch 1 — `MIN_CENTER_HEIGHT` joins it
  there; `get_panel` `:236`, `set_panel_visible` `:261`, `show_panel` `:271`,
  `toggle_panel_visible` `:287`, `set_panel_collapsed` `:295`, `is_narrow` `:316`,
  `narrow_overlay`/`open_`/`close_` `:323-336`, `layout` `:411`, `expanded_content_bounds`
  `:185`), `dock/render.rs` 411 (`render` `:80`, the narrow overlay at `:97`, `handle_resize`
  `:153`), `dock/tests.rs` 417 (31 tests, the executor's first gate). The split § C orders
  (`groups.rs`, `layout.rs`, `focus.rs`) is the first step, before any behaviour changes.
- **Preferences.** `PanelPrefs { id, visible, collapsed, size }` `editor_preferences.rs:71`,
  `panels: Vec<PanelPrefs>` `:101`; the file is 433 lines after batch 2's `UiScale`, so
  `GroupPrefs` and the migration go in a sibling (`editor_preferences/groups.rs`) if they would
  pass 600. `docs/WEB_PLAYGROUND.md` § Preferences names the panel layout and gets the `groups`
  key.
- **The context menu**: `ContextMenuAction` (`context_menu.rs:30`) has one variant,
  `Open(OpenRequest)`; `Dock(DockAction)` joins it; `open(anchor, items, note)` `:83`; the one
  dispatch is `asset_browser_keys.rs:114`, and the dock actions dispatch in `menu_actions.rs`
  as § C says.
- **The Window menu** goes between View (`menu/mod.rs:227-243`) and Entity (`:258`), on batch 2's
  nested dropdown (`menu/dropdown.rs`, `menu/keyboard.rs`, `MenuItem::submenu` `:73`); its
  panel rows are rebuilt each frame from the dock the way `menu/scale.rs`'s `sync_ui_scale`
  rebuilds the UI Scale rows — that is the model to copy, a `menu/window.rs`. Batch 2's
  keyboard rule applies: Enter with no row highlighted fires nothing.
- **The keyboard route.** `F6` is unbound (`editor_input.rs`, 504 lines). The ui has no public
  traversal walk to ride: `focused_widget` (`ui/src/context/mod.rs:212`), `set_focus` and
  `clear_focus` (`interaction/mod.rs:407,412`) and the theme's `focus_ring` /
  `focus_ring_width` (`ui/src/style.rs:141`) are what exists, so the strip focus is the dock's
  own state (`dock/focus.rs`), drawn with `ui.rect_border(tab, theme.focus_ring, 2.0, 2.0)` as
  the dropdown draws its cursor. The route is a `KeyRoute::DockTabs(..)` arm beside
  `Settings(..)` in `shortcuts.rs` — **569 lines, at the ceiling**: the handler lives in a new
  `editor_game/dock_keys.rs` on `settings_keys.rs`'s model (75 lines), `shortcuts.rs` gains the
  arm only.
- **Near the ceiling, besides**: `context/mod.rs` 549 (`show_panel` has one non-test caller,
  `shortcuts.rs`), `panel_renderer/mod.rs` 532 (the narrow overlay's band at `:20-28`),
  `overflow_menu.rs` 487 (the `»` shape to reuse: `overflow_menu_bounds` `:36`,
  `render_overflow_menu` `:104`).
- **#233, re-read**: still open; nothing in batches 1–2 changed its picture (batch 1's clamp moved
  the overflow menu along x only). § C's "`narrow_overlay` is per group" is where the band
  decision falls: the dropdown and the overflow menu either move to `Modal` (and close under a
  dialog, as the context menu does) or a band between `Floating` and `Modal` is added for the
  three menus. Decide, and either land it with its test or write the line on #233 in the report.
  **#99's Preferences line**: the selected tool, collider visibility, hierarchy expand state,
  selection, window geometry and recent files are still not persisted; batch 3 adds `groups`
  and touches none of those — say so in the report.
- **Default layout**: `context/default_layout.rs` (56 lines) adds Hierarchy (Left), Inspector
  (Right), Scene (Center), Assets (Bottom), Conductor (Bottom, hidden), Console (Bottom,
  hidden), External Tools (Right, hidden) in that order — ruling (d)'s groups fall out of it.
- **The stills, under the roster as corrected**: three real screenshots of the editor_demo at
  1280×800 — the default layout; the bottom group with Console active; a two-tab right group —
  saved as `review/editor-ux/batch3-dock-{default,console,right-tabs}.png`, copied to
  `coordination/editor-ux/screenshots/batch-3/` and staged, each described in one line in the
  report. `editor_prefs.json` is restored after every run.
- **The code review is round 9**; reviewers kimi and deepseek, the planner's own review and the
  UI eye beside them; codex does not review its own vendor's batch.

**Done 2026-11-05: 32682df** (executor Astra/codex, `report-3.md` COMPLETE, 50 files; code round 9 —
kimi 5 (6 m 28 s), deepseek 7 (16 m 30 s, the arm's first run), the planner 6 with the UI eye over
the three stills — all three "do not merge" on one regression: Assets and the Conductor as tabs of
one group left a dragged sample no Conductor to land on; round 10 on the planner's fix diff — kimi 1,
deepseek 5). The accepted findings, fixed by the planner: spring-loaded tabs under a drag (a
payload resting on a tab for `DRAG_DWELL_FRAMES` brings it forward, within a group already
showing content, never under an overlay); tabs sized by their titles (48–192 pt) instead of an
even split; a `Show` item instead of the executor's reveal on submenu open (§ C's correction
below); unfocused Enter and the menu key forwarded to the game; a tab click, an overflow row and
a move through `DockArea::reveal`, which keeps the saved collapse flag inside the narrow overlay;
View → a panel brings a visible back tab forward; the stale overflow index cleared in `layout`;
right-press menus on narrow tabs and collapsed strips; `MIN_CENTER_HEIGHT` counting the scene
header (the budget had left 96 pt of viewport); the budget ranked by distance from the centre
across both edges. Rebutted: a `panels` downgrade projection (this section's one-way migration);
the hover reveal (removed, not completed). Jesse's two rulings during the batch: the migration
clamps Assets' legacy 180 pt to the group's 200 pt minimum; Ctrl++ / Ctrl+- step the UI scale with
their hints in View ▸ UI Scale. #233 closed by the batch (menus and both overflow menus on Modal
above the dock's overlays). Filed: #250 (the first content row draws under the header —
pre-existing, in batch 2's stills too). `rebuttal-9.md`, `rebuttal-10.md`; the ledger's rounds 9
and 10.

**Batch 4a — the asset browser: folders and create (§ D, the first half). Issue #243.** Files:
`crates/common/src/vfs/mod.rs` (+ `vfs/entries.rs`: `list_entries`, `create_dir_all`, `exists` —
424 lines today), `crates/editor/src/asset_browser/{mod,tree}.rs` (the split; the companion
classification; the router), `icons.rs` (`Folder`, `Up`, `New`), `context_menu.rs`
(`ContextMenuAction::Asset(AssetAction)`), `prompt_dialog.rs` (new),
`crates/engine_core/src/sheet_file.rs` (`write_sheet_file`),
`crates/editor_integration/src/panel_renderer/asset_browser/{header,mutate}.rs` (new, the split)
+ `asset_browser.rs`, `tile.rs` (the badge), `keyboard.rs`, `editor_game/menu_actions.rs`, the
crate guides, `docs/EDITOR_COMMAND_API.md` only if a verb is added (none is planned);
`insiculous_web/src/scripts/playground-scripts-panel.ts` + `playground-embed.ts` (the
`files-changed` response — reviewed in `../insiculous_web/review/editor-ux/`). Target shapes: § D
through *Create (4a)*, `vfs::write_new` included. Gates: the standard set; `check_wasm.sh` (common,
engine_core); `check_games.sh`. Reviewers: kimi, gemini (the vfs seam, native and web), codex
(three stills). Leaves out: rename and delete (4b); § D's list; the convention doc and the trees
(batch 6).

**Re-verified 2026-11-05 against 32682df (batch 3 landed; the tree is clean on `jesse`).**
Every file, line and count above and in § D checked; what the executor builds on:

- **Roster (the ground rule's correction):** reviewers kimi and deepseek (deepseek takes gemini's
  angle, the vfs seam native and web), the planner is the UI eye over the stills; codex is the
  executor and never reviews. Where § D's *Tests* says "stills for codex", read the planner.
  The code review is round 11.
- **Lines that hold exactly:** `vfs/mod.rs` 424 (`list_entries`, `create_dir_all`, `exists`,
  `write_new` are all new → `vfs/entries.rs`; `set_write_observer` :29; the memory store's
  `remove_prefix` slash :396); `asset_browser.rs` 433, `kind_for_path` :176 with the `.ron` arm
  :183; `panel_renderer/asset_browser.rs` 583 (the split into `header.rs` and `mutate.rs` comes
  first, behaviour unchanged, `cargo test -p editor_integration` green between); `archive.rs:255`;
  `song/io/samples.rs:55`; `file_type.rs:173`; `EDITOR_COMMAND_API.md:193`;
  `playground-scripts-panel.ts:20,36`; `sheet_file.rs:188` (273 lines; `write_sheet_file` is new).
  `persist/queue.rs`'s `on_vfs_remove` is at :83 now (4b's item).
- **Stale:** `external-tools/plan.md:1626` — External Tools landed 2026-11-03 (its plan is archived
  under `review/_archive/external-tools/`) and shipped no File ▸ New Song and no
  `EditorAction::NewSong` (the composer pivot, #217 re-scoped); nothing changes for 4a: `song init`
  (`song/io/init.rs`) stays the sidecar's one creation, and *New Song sidecar* calls it.
- **Batch 3 moved these:** `ContextMenuAction` is `Open | Dock(DockAction)` (`context_menu.rs:30`,
  264 lines) — `Asset(AssetAction)` joins it; the one dispatch is `run_context_menu_action` in
  `editor_game/menu_actions.rs:14` (220 lines), not `asset_browser_keys.rs:114` — the Asset arm
  goes beside Dock. The Assets panel is a tab of the bottom group: its content is `Rect::default()`
  while another tab is active, `panel_press` already refuses an inactive tab, and the browser's
  keyboard is released by `release_inactive_panel_focus` — the prompt dialog needs nothing from the
  dock but `dialog_up()`: `game_impl.rs:101-105` closes the menu bar, the toolbar overflow and the
  tab overflow while a dialog is up, so `dialog_up` must count the prompt (the confirm dialog's
  model). The drag's spring-loaded tabs (`tab_rested_on`) are a dock rule; a folder tile is not a
  drop target in 4a.
- **Named today, by name:** `AssetBrowserState.selected: Option<usize>` (`asset_browser.rs:59-85`,
  an index into `entries`) → `TileId` as § D says; `scan_assets(base, daw) -> Vec<AssetEntry>`
  :195 → `AssetScan`; `confirm_dialog.rs` (297) and `text_field.rs` (87) are the model and the
  field for `prompt_dialog.rs`; `icons.rs` (292) holds the strip's glyphs only — `Folder`, `Up`,
  `New` join `Icon`; "through the serializer" is `scene_serializer::serialize_to_ron(&SceneData)`
  (:78); the sheet sidecar's default cell is the image's size as the host already reads it for
  the thumbnail (`panel_renderer/asset_browser.rs:265`, `assets.get_texture(..).width/height`),
  so *New Sheet sidecar* is enabled once the selected PNG's thumbnail has loaded and says
  "loading…" before; `write_new`'s web half is the memory store's compare-and-swap
  (`playground/src/store/memory.rs:51-81`, `base_revision`), as § D says.
- **The site half, corrected:** the bridge's response channel is one JSON line per dispatched
  line and the embed shifts a sink per line (`playground-embed.ts:163-168`), so an unsolicited
  `files-changed` line there would misattribute the next answer. The notice gets its own drain:
  `playground_poll_file_changes() -> string[]` in `playground/src/bridge.rs` (relative paths the
  editor wrote or created since the last poll, fed by the write observer persistence already
  registers), polled by the embed beside `pollResponses` and handed to a hook the scripts panel
  registers, which re-lists through `playground_list_files` (its `enable()` list, :122-129,
  becomes a `refreshList` it calls then and on the hook). Site branch `jesse`, clean; the site's
  gate is `npm run verify`; its diff is reviewed under `../insiculous_web/review/editor-ux/`
  (the planner creates it).
- **Sizes near the ceiling:** `panel_renderer/asset_browser.rs` 583 and `asset_browser.rs` 433
  (both split, above); `vfs/mod.rs` 424 (`entries.rs`); `editor_preferences.rs` 401 —
  `current_folder` is one `#[serde(default)]` field there. `check_wasm.sh` applies (common,
  engine_core); `check_project_layout.sh` does not exist yet (batch 6), so it is no gate here.
- **Stills:** `batch4a-assets-{root,sprites,new-menu}.png` at 1280×800 — the root with its folder
  tiles and breadcrumb, inside `sprites/` with a badged tile, the New ▾ menu open.

**Done 2026-11-05: 1d18b1a (engine) and insiculous_web 8f2fb19 (site)** (executor Astra/codex,
`report-4a.md` COMPLETE, 49 engine files and a 17-path site half; engine round 11 — kimi 3
(4 m 51 s), deepseek 8 (11 m 47 s), the planner 6 with the UI eye over the three stills; site
round 1, the new subject `../insiculous_web/review/editor-ux/` — kimi 3 (2 m 51 s), deepseek 6
(5 m 45 s); the fix rounds, engine 12 — kimi 3 (2 m 06 s), deepseek 4 (5 m 19 s) — and site 2 —
kimi 2 (2 m 20 s), deepseek 4 (8 m 34 s); every finding applied in its named shape, no round 13).
Both commits are Jesse's, made from the IDE on 2026-11-05 under its generated summary ("Remove
editor asset browser module and add midi-evil VCS mapping" — it is this batch: the asset browser
*module* became the `asset_browser/` directory), verified by the planner on 2026-11-06 as
byte-for-byte the reviewed fixed snapshots (`review/editor-ux/draft-4a-fixed.diff`; the site's
`draft-4a-site-fixed.diff` over its reviewed files) plus, in the engine, the ledger's rows and, in
the site, the generated v3 bundle and Jesse's own devlog post. The message carries no `fixes`
line, so #243 closes by hand. Accepted and fixed by the planner: the half file a failed `write_new`
left (removed by inode; the atomic publish waits for 4b's hard link); `pending_folders` retired;
a linked asset root followed, a bad entry skipped; Assign gated on Play alone; the web tile menu's
desktop-only note; Open sidecar gated like the New items; the hot-reload hold reading
`dialog_up()`; a folder double-click's release selecting nothing; the prompt's error lines
word-wrapped, an overlong word split; the guides' one Pitfalls table; `FILE_CHANGES` as a
`RefCell`; on the site the dirty-buffer guards, the bytes-compared reload, the feature-detected
drain, the refresh's own error alone, and the missing-file copy. **The big one was the planner's:**
the handoff said `--version v2 --sync`; the site's README forbids overwriting a deployed version
folder and `build_wasm.sh` refused without `--force`, which the executor passed as told — v2
restored as deployed, the crate's five-place version v3, the bundle built and synced as
`/playground/v3/`, the embed, the preview page, the postbuild check and `WEB_PLAYGROUND.md`
naming it. Rebutted: the Conductor demo scene's missing wiring (pre-existing; #251, batch 6's);
removals in the drain (4b's); the v3 record schema in v2's database (verified: `deleted` is
`serde(default)`, `DB_VERSION` 1 — the reverse hazard, a cached v2 glue after a v3 session
reading a tombstone as an empty file, is the tombstone design's own, noted here for the next
bundle bump); the "bundled project changed" note every stored project shows once after a bump
(the compare's designed meaning; batch 6's migration may revisit it). Lessons: a reviewer packet
cut with `git diff <tree> -- src` picks up another author's untracked-then-staged files (exclude
`src/content`); the harness cwd follows the last `cd` in parallel calls (absolute paths in every
heredoc); a handoff that names a bundle version says "a NEW version folder". `rebuttal-11.md`,
`rebuttal-12.md`, the site's `rebuttal-1.md` and `rebuttal-2.md`; the ledger's eight rows for
rounds 11, 12, site 1 and site 2 (landed in 1d18b1a).

**Batch 4b — the asset browser: rename and delete (§ D, *Rename and delete*). Gated on 4a; Jesse
kept it 2026-11-03. Issue #244.** Files: `vfs/entries.rs` (`remove_file`, `remove_dir_all`, `rename` — the two
native algorithms — the web ordering, the failure-injection hook on the memory store),
`crates/playground/src/persist/queue.rs` + `store/` (the bundled-only tombstone),
`crates/editor/src/song/io/` (the `LICENSE` stanza follows the rename),
`insiculous_web/src/scripts/playground-scripts-panel.ts` (rename and delete of the open script), `asset_browser/tree.rs` (the pair
family, the reference scan), `panel_renderer/asset_browser/mutate.rs`, `editor_game/scene_io.rs`
+ `song_io.rs` (the open document's path follows a rename; a delete above it is refused),
`context_menu.rs`, the guides, `docs/WEB_PLAYGROUND.md` (the ordering contract and the duplicate
as recovery). Gates: as 4a, plus `check_audio_provenance.sh` over a renamed song. Reviewers: kimi,
gemini (the ordering), codex (the rename prompt and the refusal, two stills). Leaves out: a journal;
undo.

**Re-verified 2026-11-06 against 1d18b1a (batch 4a landed and pushed; the tree is clean on
`jesse`).** Every file, line and count above and in § D's *Rename and delete* checked; what the
executor builds on:

- **Roster (the ground rule's correction):** reviewers kimi and deepseek (deepseek takes gemini's
  angle — the web ordering and the tombstone), the planner is the UI eye over the two stills;
  codex is the executor and never reviews. The engine's code review is round 14 (round 13 reviewed
  this re-verification), the site half's round 3 under `../insiculous_web/review/editor-ux/`.
  **Ruling (j) is Jesse's since 2026-11-06:** the residual is accepted — one sentence in the panel's
  doc comment names the two adjacent syscalls, no `libc` — so the handoff follows this
  re-verification.
- **Lines that hold, moved by 4a:** `vfs/entries.rs` 186 (`list_entries` :21, `create_dir_all`
  :43, `exists` :58, `write_new` :72, the `MemFs` twins :100–150, `remove_if_same_file` :160) —
  `remove_file`, `remove_dir_all` and `rename` join it with their twins; `vfs/tests.rs` is 459, so
  4b's vfs tests go in a new `vfs/mutate_tests.rs`. `vfs/mod.rs` 427: the public `remove_prefix`
  is :225, `MemFs::remove_prefix` :396 (the slash; the dropped keys returned); **`set_remove_observer`
  (:50) already exists**, firing once per key `remove_prefix` drops on wasm — the web halves notify
  through it, and a web `rename` is its destination puts through the write observer *then* its
  source removes through the remove observer: the ordering contract, made of the two observers.
- **The bundled-only tombstone, precisely:** `persist/queue.rs` (269) `on_vfs_remove` :83–98 returns
  when the path has no chain (`is_none_or(conflicted)`), and a chain exists only for a path seeded
  from the store (`persist/mod.rs:137`) or written in this tab — a bundled-only path has none, so
  `Operation::Remove` (:28) is never queued for it. All three stores answer an absent record at
  base 0 with `Ok(())` — `store/memory.rs:87`, `store/directory.rs:149`, `store/indexed_db/mod.rs:349`
  — § D's no-op. 4b: `on_vfs_remove` creates the chain at revision 0 for a project-relative path
  without one, and each store writes a tombstone for an absent record at base 0 (`deleted`, empty
  bytes, revision 1, the current `bundle_version`); the boot already drops a tombstoned path's
  bundled copy (`WEB_PLAYGROUND.md` :172–179; the playground guide's row :47). **Two seams the
  tombstone must carry with it (round 13, kimi F1/F2 = deepseek F2/F1):** `ProjectStore::remove_file`
  (`store.rs:85`) takes the manifest and **upserts it as `put` does** (`memory.rs:75–80`'s rule, in
  all three stores, atomically with the tombstone) — the boot (`web_entry.rs:193–200`) wipes stored
  files that have no stored manifest as an interrupted import, so a visitor whose one change is a
  bundled-only delete would see the file back on reload, and `projects.rs:76–86` would never mark
  the project modified; and it **returns the resulting revision** (`Result<u64, _>`), so the queue's
  `start_operation` drops its "at revision zero nothing was tombstoned" case (`queue.rs:234–238`,
  which records `revision_after = 0` and makes the next put on the path — a recreate, a rename back —
  conflict terminally against its own revision-1 tombstone). Tests: a first-ever bundled-only delete
  run through the boot's stored-file selection over a tombstone-only store, and a same-`Chains`
  delete-then-recreate with no reseed. Testable natively in
  `store/tests.rs` (224) and `persist/tests/`; the memory store has **no failure-injection hook
  yet** (`store/memory.rs`, 174) — 4b adds the one § D's test needs.
- **The family, in one place:** `asset_browser/tree.rs` (154) has `companion_path(primary)` :40
  (PNG → `.sheet.ron`, MIDI → `<path>.ron`) and `kind_for_path` :22, which classes every
  `.sheet.ron` and `<midi>.ron` as `AssetKind::Companion`, which `visible_tiles`
  (`asset_browser/mod.rs:171`) hides — so **an orphan companion is hidden today**; 4b shows a
  companion whose primary is absent as a tile with an "orphan" badge (text, never colour alone)
  that offers *Delete…* alone — *Rename…* is disabled on it with the note "an orphan companion is
  deleted, or paired by adding its primary" (`validate_name` refuses every `Companion`,
  `tree.rs:121`, and no inverse of `companion_path` exists or is built — round 13, kimi F3,
  deepseek F5) — and adds `asset_family(primary) -> Vec<String>` (primary, companion, the
  `.recovered-*` siblings) beside `companion_path`, read by rename, delete and the collision
  preflight alike. **The rename validator is its own** (kimi F3): `validate_name` (:112–133) is
  creation-specific — Song accepts `.mid` only, Sound `.wav` only — so a pre-filled `theme.midi`
  would be refused; `validate_rename(name, kind, current_extension)` beside it keeps the asset's
  own extension (`.midi`, `.ogg`, `.jpg`) and applies the snake_case rule to the stem, with tests
  for `.midi` and an alternate sound and image extension.
  **No writer of `.recovered-*` names exists in the tree** (the pattern is only in
  `file_type.rs:280`'s test): the family keeps the pattern for files brought in by hand, tested
  with a fixture, and 4b builds no writer.
- **Named today, by name:** `TileId` (`asset_browser/mod.rs:82`), `selected: Option<TileId>` :105,
  `apply_scan` :139 (a selection no longer visible clears itself, so a deleted tile's does; a
  renamed tile is reselected by its new identity as `confirm_creation` does). `context_menu.rs`
  (277): `AssetAction` :41 gains `Rename(TileId)` and `Delete(TileId)`, dispatched by the one arm
  in `menu_actions.rs:18`; the tile menu is `header.rs:59–90` (`open_new_menu`; Open sidecar :83)
  — *Rename…* and *Delete…* go before the Open items. `panel_renderer/asset_browser/mutate.rs`
  (216): `begin_creation` :14 (`editor.asset_creation: Option<AssetAction>` is the pending
  operation, `editor.asset_prompt` the dialog), `render_prompt` :38, `confirm_creation` :52 —
  rename and delete go in a sibling `remove.rs` (delete and its confirm) and `rename.rs`;
  `render_prompt` routes a confirmed name by the pending action's variant. `prompt_dialog.rs`
  (158): `open(title, initial)` :34 pre-fills, but the button says "Create" — a confirm-label
  parameter joins it. `confirm_dialog.rs` (297): `ConfirmDialog` is plain data (:27–35) with one
  constructor per prompt (:45–142) — `delete_asset(name, file_count)` (Delete · Cancel),
  `rename_despite_references(count, names)` (Rename anyway · Cancel) and
  `save_and_rename(document)` (Save and rename · Cancel) join them, parked and routed through
  `scene_confirm.rs`'s `PendingSceneAction` (:27–43 — one variant each), so `dialog_up`
  (`panel_press.rs:29`, already counting the asset prompt) counts them for free.
- **Open documents:** the scene's path is `EditorContext::scene_path()` / `set_scene_path`
  (`context/mod.rs:365,370`); the song's is `SongDocument::path()` (`song/mod.rs:110`; the field
  :69 is private, no setter) — 4b adds `SongDocument::follow_rename(new_path)`, path only,
  fingerprints kept, and `song_io.rs:93` then saves to it. The hot-reload watcher recomputes
  `watched_paths` from the documents each tick (`hot_reload.rs:149`), so a renamed open scene is
  watched at its new path once `set_scene_path` ran — but the old path's disappearance reaches
  `dispatch_file_change` (:240–246) as a non-`Changed` event and prints "was deleted on disk"; a
  rename must not report its own move — and `note_written` cannot carry it: with no stamp the
  worker only echoes `Noted` (`hot_reload/worker.rs:106–110`), and two polls before the host's
  next tick still emit `Deleted` (kimi F5, round 13) — so the watcher gets a removal-specific note
  (`WatchMessage::Removed(path)`, recorded as absent) sent before the move, and the test runs two
  old-path polls before the host tick and asserts no such line. Natively only; the web has no
  watcher. **Save and rename has three outcomes** (deepseek F4): `save_song(None)`
  (`song_io.rs:63–74`) answers `Ok(Some)` saved — the rename proceeds; `Ok(None)` parked behind
  the Overwrite prompt — the rename is dropped with the status line "the save is waiting on
  Overwrite — rename after it"; `Err` — the rename aborts naming the error. And *Rename…* of the
  open scene or song is refused outright while a confirm is parked (`scene_confirm.pending_action`)
  or a reload waits (`waiting_scene`, `waiting_song_map`, `hot_reload.rs:82`), so `follow_rename`
  never races a parked old path.
- **Songs — the planner's finding, a correction to § D's family:** a song's sample folder is named
  from its *path* (`song/io/paths.rs:92 sample_directory`: `music/theme.mid` →
  `music/samples/theme`, `music/a/theme.mid` → `music/samples/a__theme`) and that folder's
  `LICENSE` names its owner (`samples.rs:129 check_folder_owner` reads `Source:`). After a rename
  the sidecar's sample paths still point at the old folder (the song loads), but the next Save
  computes the *new* folder, finds it free, copies the samples there and leaves the old folder
  behind with a `LICENSE` naming a song that no longer exists — an orphan the provenance gate
  passes. **4b's rule:** *Rename…* of a song whose sample folder exists is **refused**, naming the
  folder and pointing at the Conductor panel's Save As; *Delete…* of a song includes its existing
  sample folder in the family (the confirm names it and its count). A `.mid` rename target must
  pass `saveable_song_path` (`paths.rs:53`), or the renamed song could never save. **A file under a
  song's sample folder** (`music/samples/<flat>/`, `SAMPLES_FOLDER` at `paths.rs:11`) is a `Sound`
  tile today and **refuses both *Rename…* and *Delete…***, naming the song ("ping.wav is one of
  ai_demo.mid's samples — edit it from the Conductor panel"): a renamed sample breaks the sidecar
  that names it, and the folder's own `LICENSE` is neither a home `LICENSE` nor `sounds/LICENSE`
  (deepseek F3, round 13). **The stanza:**
  `rewrite_home_license` (`samples.rs:195`) and `paragraphs`/`files_of`/`with_files` (:250–300)
  are private; 4b adds one public helper beside them, `rename_in_license(text, &[(old, new)]) ->
  Option<String>` (the whole family's renames at once, so a song stanza's `.mid` and `.mid.ron`
  tokens move together — deepseek F6), rewriting the `Files:` tokens of the stanza that names the
  file — a song's home
  `LICENSE` and a sound's `sounds/LICENSE` alike (Debian copyright format, whole tokens with
  continuation lines, as `check_audio_provenance.sh` reads them); the preflight computes the new
  text before anything moves.
- **The site half, corrected:** the bridge has **no JS-callback hook** — `Hooks` (`bridge.rs:21–32`)
  are Rust-side. So: a new export `playground_set_file_mutation_guard(function: js_sys::Function)`
  keeps the function in a thread-local, in a new `bridge/files.rs` (`bridge.rs` is 546 lines;
  `bridge/tests.rs` already makes it a directory); `EditorRunOptions` (`run_options.rs:14–43`)
  gains `file_mutation_guard: Option<FileMutationGuard>`, `pub type FileMutationGuard =
  fn(&FileMutation) -> Result<(), String>`, `FileMutation { phase: Prepare | Done | Failed,
  removed: Vec<String>, renamed: Vec<(String, String)> }` project-relative; the web entry (`web_entry.rs:294–307`) installs the
  `fn` that calls the stored function with the mutation as JSON and reads a string back (empty =
  proceed, text = the refusal under the prompt's field); natively `None`; rename and delete call
  the guard *before* their first vfs step. The scripts panel registers its guard beside
  `registerFileChanges` (`playground-scripts-panel.ts:13–15`): a dirty open script refuses its
  delete, asks before its rename through `window.confirm` (the call is synchronous inside the
  frame), saves through `saveCurrent` (:57) when told to, and **blocks its own saves** until the
  outcome: the guard is called twice — `Prepare` before the first vfs step, `Done` or `Failed`
  after — and the panel moves `currentPath` (:23) to the new path only on `Done`, releasing the
  block on either (kimi F4, round 13: a path moved at Prepare and a rename that then failed made
  the next Save create the new path beside the untouched old one); a guard-approved rename whose
  vfs step fails is tested; `refreshList` (:156) already copes with the open file leaving the list
  (:183–190). The drain: `record_file_change` (`bridge.rs:142`) records writes only — 4b records
  removals and both ends of a rename, so the list refreshes after the move (site round 1, F6).
  `playground-embed.ts` (548) gains the one registration line.
- **The bundle is v4.** A new export changes the glue, and the site never rebuilds a deployed
  version folder in place (its README :136; `build_wasm.sh` refuses a populated target): the five
  places (`web_entry.rs` header :3–9, `ASSET_BASE` :30, `BUNDLE_VERSION` :32; `--version v4 --sync
  ../insiculous_web/public`; `projects.json`; the site's `PlaygroundEmbed.astro`, `preview.astro`,
  `scripts/postbuild-check.mjs`) plus `WEB_PLAYGROUND.md` § The bundle contract (:55) and every
  playground-bundle v3 line in it; `public/playground/v3/` stays untouched. The record schema does
  not change; every visitor with stored files sees the "bundled project changed" note once.
- **Sizes near the ceiling:** `bridge.rs` 546 (→ `bridge/files.rs`); `vfs/tests.rs` 459 (a new
  test file); `song/io.rs` 493 (the helper lives in `samples.rs`, 305); `panel_renderer/
  asset_browser.rs` 451; `tile.rs` 355; `shortcuts.rs` 583 (untouched: no new key).
- **Gates:** 4a's set; `check_wasm.sh` (common changes); `check_audio_provenance.sh <root>` over a
  scratch copy of `examples/` in which the batch's own `vfs::rename` plus the stanza helper renamed
  `sounds/snd_jump.wav` to `sounds/jump.wav` — a one-file stanza and no sample folder; `ai_demo.mid`
  has one, so the UI refuses it, and the gate never reads `Source:` (deepseek F6, round 13); the
  script takes a root, :22–26 — its clean output in the report beside the engine-root run, plus the
  two unit tests (a song stanza's two tokens moved together; the sample-folder refusal); the site's
  `npm run verify` over v4.
- **Docs the batch changes:** `WEB_PLAYGROUND.md` (414) § Persistence :150–180 (the ordering
  contract, "a duplicate is the recovery", the bundled-only tombstone) and § The bridge's table
  :214–235 (the guard export, the drain's removals); the guides' rows — `common/CLAUDE.md` :10–11,
  :25, :29–32; `editor/CLAUDE.md` :100, :174; `editor_integration/CLAUDE.md` :52, :71, :78;
  `playground/CLAUDE.md` :21, :27, :45–47, :59 — and `training.md` where it names the browser.
- **Stills:** `batch4b-assets-{rename,refusal}.png` at 1280×800 — the rename prompt pre-filled over
  a badged tile, and a refusal (the open scene's delete, named).
- **Leaves out, beyond § D's:** a Delete or F2 key on a selected tile (menu items only; a follow-up
  at close-out); renaming a song with samples (Save As is its path); `EDITOR_COMMAND_API.md` is
  unchanged (#112's per-file delete verb is what this batch's web delete enables).

**Done 2026-11-09: b443470 (engine) and insiculous_web 75c5405 (site)** (executor Astra/codex,
`report-4b.md` COMPLETE, 54 engine files and 45 site paths with the v4 bundle; engine round 14 —
kimi 5 (5 m 27 s), deepseek 10 (15 m 11 s), the planner 9 with the UI eye over the three stills;
site round 3 — kimi 4, deepseek 5, both finding the guard registered before the wasm module was
initialised, which the planner's own review missed and which `verify` did not catch (#80 filed);
site rounds 4–6 settled the site; engine fix rounds 15 to 28 and 30 — round 29 is no review, the
session ended under it — each kimi and deepseek over the whole fixed snapshot, rebuttal-14 to
-28 and -30). Sixteen fix rounds is the effort's record and the lesson is in the last one:
round 30's deepseek found a critical in the planner's own round-27 fix — the sample-owner guard
resolved a LICENSE's `Source:` against the home folder, `music/music/<song>`, so no real song's
samples were protected, and the tests passed because they spelled the header relative to the home,
which nothing writes. **A test fixture is spelled as production writes it, and a guard's test is
mutation-checked once** (the old line restored, the test seen to fail). Corrections to § D the
rounds settled: `remove_dir_all` and `set_asset_root` are gone — a folder is emptied file by file
and its folders go deepest first; a companion is paired in any case of its suffix and opened as
spelled; a delete gets the reference scan and the LICENSE stanza removal; the scan reads unsaved
documents; `asset_family` returns `io::Result` and a folder read short refuses the mutation;
`write_new` publishes through a hard link with a direct-write fallback on FAT and exFAT; the
bundle is v4, rebuilt from the final tree before the site commit (the executor's bundle predated
the fix rounds). Follow-ups: #252 (Windows file rename), #80 (a boot smoke test in `verify`), a
backlog line on #99 (a mis-cased sidecar is paired by the browser and unread by the loaders).

**Batch 5 — the editor in its own window (§ E, ruling (h′)). Issues #245 (engine) and
insiculous_web#79 (site). SETTLED 2026-11-11; runs after batch 6.** Site + the playground
crate's stores and boot + one term in `editor_integration`'s dirty mirror. The files, the
gates and what is left out are § E's *What changes where*, *Gates* and *Leaves out*, which are
this batch's section. In order: the store and boot hardening first, with its tests (strict
manifest and file reads, the sweep over the validated list, `replace_project`'s revisions, a
`move=1` boot that does not fall back) — it stands alone and is worth landing if the rest
slips; then the two exports and the dirty term; then the site (the lock, the move, the window
and parked pages, the fenced import); then the new bundle version folder — v6, batch 6 having
shipped v5 — and the browser test. Executor and reviewers: Jesse's roster at the handoff
(batch 6's was Claude Opus 5.5 with kimi and codex); three stills for the UI eye — the window,
the parked card, the owner card. Both issues were filed blocked on the review and are
unblocked by this line; their bodies still describe the transfer and are rewritten from § E
when the handoff is written.

**Done 2026-11-11: db9a569 (engine), insiculous_web 613143d (site)** (executor Claude Opus 5.5,
`report-5.md` COMPLETE with one stated limit; 22 engine paths, 64 site paths with the v6 bundle;
code round 38 — kimi 4 (2 m 02 s), codex 2 (4 m 51 s), the planner 2; site round 7 — kimi 3
(4 m 13 s), codex 2 with the three stills (5 m 21 s), the planner 4; the planner's fix hunks
went to kimi as engine round 39 (none) and site round 8 (1, applied in its shape);
`rebuttal-38.md`, `rebuttal-39.md` here and `rebuttal-7.md`, `rebuttal-8.md` in the site's
`review/editor-ux/`). Follow-ups filed: #254 (an open API batch against the scene-loss guards
and the GUI save), #255 (a project generation for pages that hold no lock), insiculous_web#81
(headless Chromium loses the WebGPU device, so five of § E's browser scenarios are a person's
check), insiculous_web#82 (`playground-embed.ts` at 600 lines: split before its next change).

**§ E as built, where it differs from § E as written:**
- **The sweep reads the manifest keys once more.** `sweep_orphans(known)` in IndexedDB reads the
  projects store's keys in its own transaction, never decoded, and spares a slug that holds a
  manifest record at that moment: without it a project another tab imported between the boot's
  manifest read and its sweep was deleted. "Reads the projects store no second time" is wrong.
- **The windowed layout keeps the wordmark nav and a one-line footer.** The announce gate requires
  a `navigation` and a `contentinfo` landmark on every page, and the window is not exempt.
- **A third refusal before the window opens:** this page's Play preview open → "Close the preview
  window first — it stays with this page". The preview's reservation names the source tab, so the
  window could not re-take it and its next Play would start a second simulation.
- **One more stop in the boot:** the opened project's own manifest record undecodable, bundled or
  not, files or none. Opened, its first save wrote files beside a record `put` takes for a
  manifest, and every later boot stopped.
- **A window whose navigation throws is closed only while it still reads `about:blank`.**
- **A boot card hides the Project field**, which the engine never ran to fill.
- **The revision stamp is best effort for a page holding no lock** (a first-time create, or a
  Reset or an empty import between two imports, leaves nothing to conflict with): #255.
- The cards name the project by its slug; the title is not known before the engine boots.

**Batch 6 — the convention: `docs/PROJECT_LAYOUT.md`, the checker, the trees (§ D's other
half). Issue #246.** Files: `docs/PROJECT_LAYOUT.md` (new), `scripts/check_project_layout.sh` (new),
`scripts/check_games.sh` (calls it per game), `examples/assets/images/` → `sprites/` with
sidecars for its PNG (`hello_world.scene.ron:73` — resolve the `assets/` prefix question in the
report), `examples/assets/sounds/snd_jump.wav` → `jump.wav` (its `LICENSE` and every caller),
`crates/playground/assets/projects/pong/assets/images/` → `sprites/` + two sidecars + the scene's
paths, `../games/game-template/assets/images/` → `sprites/` + sidecar + `main.scene.ron` + its
`README.md` (its own repo, its own commit on its default branch, by the planner), `training.md`
(§ Directory Map's examples lines, § Sprite Sheet Pattern's path examples where they say
`images/`), `crates/editor/src/asset_browser/tree.rs` (the kind router reads the table — a const
table in `editor`, the doc quotes it; the checker greps the same table so the three agree), the
v3 playground bundle (#159 — the bundled pong's tree changed; the bundle's version token moves in
the five places `WEB_PLAYGROUND.md` § The bundle contract lists, and the site's `public/playground/
v3/` lands with it), and **a visitor's stored pre-convention scenes keep loading** (codex F3): a saved scene
over the bundled pong names `images/paddle_16px.png`, and this batch's bundle has only
`sprites/`. **Ruled 2026-11-11 (Jesse): boot aliases, not a store migration** — the journaled
migration this paragraph carried through plan rounds 3 and 4 (puts, references, tombstones, a
marker in the manifest) drew a data-loss critical in each of rounds 31 and 32 and is withdrawn;
the re-verification block below holds the design that replaces it, and nothing in this batch
writes to a visitor's store. **First step:** run the new checker over every current
tree and list each violation in the report before a file moves (kimi F11). Gates: the standard
set; the checker over every tree it names; `check_wasm.sh`; `npm run verify` on the site for the
bundle. Reviewers: kimi, codex (the checker, the bundle bump and the aliases are seams). Leaves
out: the six games' `sprites/` (DEION_STYLE's); renaming anything under `deion_assets`; a
font's filename (the stated exception). **Corrected 2026-11-06:** this batch's bundle is **v5**, not v3 — 4a shipped v3 and 4b
ships v4, because the site never rebuilds a deployed version folder in place (the 4a done-mark);
"v3" in this paragraph names the bundle after 4b's and "v2" a visitor's pre-convention scenes,
and the aliases' trigger is "stored version predates this batch's", whatever the numbers.

**Re-verified 2026-11-09 against batch 4b's tree (as reviewed through round 30).** Every file,
line and count above and in § D's *The convention* checked; where this block and the paragraph
above disagree, this block is the batch.

- **Roster (Jesse, 2026-11-11):** executor Claude Opus 5.5 in Jesse's window, as in batch 1;
  reviewers kimi and codex (Astra reviews what she did not build), the planner's own review
  beside them. No still: the batch draws nothing new. Engine code review is the next free round under
  `review/editor-ux/`, the site half's under `../insiculous_web/review/editor-ux/`.
- **Already there, so not built here:** the kind table. 4a landed it as `CONVENTIONS` in
  `crates/editor/src/asset_browser/tree.rs:194` (eight rows: kind, root, extension; `kind_root`,
  `validate_name`, `validate_folder`, `route_new_file` read it, and its doc comment carries the
  nine-row folder table). This batch changes no line of the router: `docs/PROJECT_LAYOUT.md`
  quotes the table, and the checker parses the ordered `(root, extension)` pairs out of the
  `Convention { … }` rows in that file and out of the doc's table, and fails on any difference —
  a row changed, duplicated, reordered or dropped on either side (a count alone would pass a
  renamed root; round 31 kimi F3). `ai/` and `LICENSE` are the checker's own two additions,
  stated in the doc and excluded from the comparison by name.
- **Stale, dropped:** `training.md` names no `images/` path and no `snd_` file any more (its
  scene example already says `sounds/jump.wav`); it gains one line pointing at
  `docs/PROJECT_LAYOUT.md` and nothing else. `docs/PROJECT_LAYOUT.md` and
  `scripts/check_project_layout.sh` do not exist; `scripts/check_games.sh:23` holds the `GAMES`
  list the per-game call joins (`game-template` is in it).
- **The trees, as they stand.** `examples/assets/images/wood_texture.png` (one file, no sidecar);
  `examples/assets/sounds/snd_jump.wav` with its stanza at `sounds/LICENSE:13`;
  `crates/playground/assets/projects/pong/assets/images/{ball_8px,paddle_16px}.png` (no sidecars),
  named at `pong.scene.ron:45,73,103`; `../games/game-template/assets/images/ball_8px.png`, named at
  `main.scene.ron:48,115` and `README.md:54`. Every other caller of an old path:
  `crates/editor_integration/src/editor_game/hot_reload_sheet_tests.rs:28` (the pong paddle by
  path), `crates/engine_core/tests/fixtures/hello_world_saved.scene.ron:66` (the golden fixture
  `tests/hello_world_golden.rs` compares a save against — it changes with the scene). **Left as
  they are:** `crates/playground/src/archive/tests.rs` (`assets/images/x.png` is an imported
  archive's own path, which the convention does not reach) and the `snd_jump.wav` strings in
  4b's `LICENSE` and rename tests (temporary directories, not the tree).
- **The `assets/` prefix, resolved (the question § What the tree holds left for here).**
  `hello_world.scene.ron` names three files with it — `:73` the texture, `:133` and `:290` the two
  sounds — because `hello_world.rs:24`, `editor_demo.rs:32` and `behavior_demo.rs:101` run with the
  asset base `examples/` (`EXAMPLES_DIR`), one folder above the asset root. On the web the
  project's base is `…/projects/examples/assets` (`projects.rs` `project_asset_base`), so the same
  three paths resolve to `…/assets/assets/…` and load nothing: the scene has never shown its
  texture or played its sounds in the playground. **This batch makes `examples/assets` the base,
  as `conductor_demo.rs` already has it:** the three examples pass `examples/assets`, their
  `with_locales_dir("assets/locales")` becomes `"locales"`, `hello_world/platformer.rs:157,223,232`
  and `behavior_demo.rs:39` lose the `assets/` segment, the scene's three paths become
  `sprites/wood_texture.png`, `sounds/jump.wav`, `sounds/beep_e.wav`, the golden fixture follows,
  and **`examples/assets/locales/pirate.ron:7`** — `font: Some("assets/fonts/BlackSamsGold-ej5e.ttf")`,
  resolved against the asset base — becomes `fonts/BlackSamsGold-ej5e.ttf`, its comment at `:6`
  ("the examples/ directory") corrected (round 31 deepseek F4; `en.ron` names no path). First step
  for this item: a test that loads `hello_world.scene.ron` against `examples/assets`, finds all
  three files and resolves the Pirate locale's font — it fails before the change and is the pin
  after.
- **The bundle carries three projects, not one** (`scripts/build_wasm.sh --project examples=… pong=…
  game-template=…`, the invocation of record in `WEB_PLAYGROUND.md:29`), and all three trees move.
- **A stored pre-convention project keeps loading through boot aliases — no store write, no
  marker, no migration** (Jesse's ruling, 2026-11-11, after round 32: kimi found a crash window
  between the source tombstone and its journaled phase and a two-tab resume, each deleting the
  migrated copy a scene already named; round 31 had found two more. A design that can lose a
  file to repair a path is the wrong size for five renamed files). The whole mechanism:

  | slug | old key (project-relative) | the bundle's file |
  |---|---|---|
  | `examples` | `assets/images/wood_texture.png` | `assets/sprites/wood_texture.png` |
  | `examples` | `assets/sounds/snd_jump.wav` | `assets/sounds/jump.wav` |
  | `pong` | `assets/images/ball_8px.png` | `assets/sprites/ball_8px.png` |
  | `pong` | `assets/images/paddle_16px.png` | `assets/sprites/paddle_16px.png` |
  | `game-template` | `assets/images/ball_8px.png` | `assets/sprites/ball_8px.png` |

  The table is exact files, not a prefix: it is the list of what this batch renames, it never
  grows with the bundle, and a sprite a later bundle adds is not mirrored under `images/`. The
  new sidecars are not aliased — the old paths had none, and an aliased file behaves as it did
  before this batch.
- **When: a row is aliased when a stored file names it.** A row's *reference token* is its old
  key without the `assets/` prefix — `images/paddle_16px.png`, `sounds/snd_jump.wav` — which is
  how a scene or a script spells the path (`pong.scene.ron:45`). At boot, over the same vector
  of stored files the overlay loop is about to lay down, a row of the project's slug is selected
  when any live (not tombstoned) stored record whose path ends in `.ron` or `.rhai` — **compared
  without regard to case**, as `SceneLoader::first_scene_in` and 4b's reference scan compare
  (`rename.rs:187`; round 36 kimi F1) — contains its token as a plain substring. Nothing else is consulted — not the manifest, not `bundle_version`. The first
  draft of this bullet gated on the manifest's version and round 33 broke it (codex F1): a tab
  left open across the deploy saves its old scene after a new tab's first save has already
  stamped the manifest v5, and the scene's `images/` paths resolve to nothing. The content rule
  has no such window, covers a hand-built archive, and still leaves a project that names only
  new paths with no `images/` folder. A plain substring over-selects on purpose — a stored
  `hello_world` scene's `"assets/images/wood_texture.png"` selects the texture's row, whose
  alias that scene's path does not reach — because a spare key costs nothing and a missed one
  is a scene that will not load. A project with no stored files, or whose stored files were
  dropped as an interrupted import (`stored_files_for_boot` → `None`), selects nothing.
- **Where and in what order.** `web_entry.rs` step 6 (`:193`), between the bundle's preload and
  the loop that lays the stored files over the memory filesystem: for each selected row, when
  the bundle's file is readable and the old key is absent, the bundle's bytes are inserted under
  the old key (`common::vfs::insert`, the call the overlay loop itself uses). Then the existing
  loop runs unchanged, so **a stored record at an old key replaces the alias and a stored
  tombstone at an old key removes it** — a customised paddle is still the visitor's, and an
  old key they deleted stays deleted. The write and remove observers are installed after step 7
  (`:233` on), so an alias queues nothing, seeds no chain and is never a stored record; a later
  save over an aliased path is the ordinary first put of a bundled-only file, and a delete of
  one is 4b's bundled-only tombstone.
- **One direction only, and its cost, stated.** The aliases put the bundle's file under an old
  key; nothing puts a visitor's old-key file under the new key. Rounds 34 to 36 tried that
  mirror and each form of it drew a critical of its own — a mirrored tombstone deleting the
  file a saved scene names, a deleted source taking the only stored copy of a customised image
  with it, a sidecar attached to another image, two tabs disagreeing about which key is newer
  (rebuttal-34 to -36) — because two keys for one file is a second store invented at boot. So:
  a visitor who changed one of the five files under v4 **without saving a scene** (4b's rename
  can do that) sees the stock file in the bundle's new scene; their file is still theirs, live
  and untouched at its old key in the `images/` folder, and pointing the scene at it is one
  assignment. A file they deleted under its old name is back under its new one. Nothing is
  lost, no stored scene fails to load, and nothing is written.
- **The shape.** A new `crates/playground/src/legacy_paths.rs`: the table as a `const`;
  `pub fn selected_aliases(slug: &str, stored_files: &[StoredFile]) -> Vec<(&'static str,
  &'static str)>` — pure, compiled natively, empty for an unbundled slug; and one function that
  lays a list of pairs over a filesystem through three closures (read, exists, insert), so the
  native tests drive it over `common::vfs::MemFs` (as `archive/tests.rs:44` does) and
  `web_entry.rs` passes the global's. `web_entry.rs` (323 lines) gains that one call. No trait
  method, no manifest field, no banner: `ProjectStore`, `ProjectManifest`, `persist/` and
  `bridge.rs` (548) are untouched by this batch.
- **Tests** (native, over `MemFs`, each fixture spelled as the store writes it — the stored
  scene is the real pre-batch `pong.scene.ron`, copied into the test's fixtures before the tree
  moves): a scene-only stored `pong` reads `assets/images/paddle_16px.png` and gets the
  bundle's bytes; **the same stored scene under a manifest stamped `v5`** is aliased all the
  same (round 33 codex F1's two-tab order); a stored customised
  `assets/images/paddle_16px.png` wins over the alias; a stored tombstone at the old key leaves
  it absent; a tombstoned scene selects nothing; a stored scene naming only `sprites/` paths, and
  a project with no stored files, get no old key; a stored `.rhai` naming `sounds/snd_jump.wav`
  selects the `examples` row; a row whose bundle file is missing inserts nothing. **A stored scene named
  `assets/scenes/LEVEL.SCENE.RON` selects as its lowercase twin does**; and the stated cost is
  pinned so it is never mistaken for a bug: a stored customised
  `assets/images/paddle_16px.png` with no stored scene leaves the old key holding the
  visitor's bytes and `assets/sprites/paddle_16px.png` holding the bundle's. And one that
  reads the table against the tree: every `examples` and `pong` row's bundle file exists under
  `examples/` or `crates/playground/assets/projects/pong/` after the moves and no row's old key
  does; **the `game-template` row is checked only when `../games/game-template` is there** — a
  standalone engine checkout has no sibling, and `cargo test --workspace` must pass in one
  (round 33 kimi F1; `preview.rs:233` returns early the same way for the deployed bundle).
- **What a visitor sees, stated.** A pre-convention project shows an `images/` folder holding
  the aliased files beside the bundle's `sprites/`; New and the router still place new files by
  the convention. Its export carries both copies (`collect_asset_entries` lists the filesystem),
  which makes the archive self-contained. **An import is not aliased and does not need to be:**
  an export carries every file under `assets/`, the bundled ones included (`archive.rs:144`), so
  an archive made by any earlier bundle holds its own `images/` files and resolves them from the
  store (round 32 kimi F3 named an archive that relies on the bundle for a file — only a
  hand-built one does, and the content rule covers that one too). **An export is a backup
  of what the visitor has, not a convention-clean tree:** unzipped over the template it brings
  its `images/` folder, and the new checker says so — which is the checker's job, and
  `docs/PROJECT_LAYOUT.md` says how to move the files (round 35 kimi F3). For the renamed
  sound, the `Files:` field of `snd_jump.wav`'s stanza in `examples/assets/sounds/LICENSE`
  becomes `jump.wav snd_jump.wav`, its comment saying the second is the file's former name
  — so an export of a project that stored no `LICENSE` of its own names both files
  (`check_audio_provenance.sh` asks only that each audio file is a token of a `Files:`
  field); a project whose stored `LICENSE` predates the batch keeps its own text, and no
  provenance is reconciled in memory (round 34 kimi F3, codex F2; round 35 kimi F4, codex
  F2). A stored `hello_world` scene's three `assets/…` paths are not helped:
  they never resolved on the web, and the aliases keep what this batch would break, not what
  was already broken. `list_projects`' `differs_from_bundle` (`projects.rs:82`) is unchanged.
- **Native has no compatibility layer, stated** (round 33 codex F2). `examples/` is the engine's
  own demo tree, not a project a user keeps: its three scenes are tracked
  (`behavior_demo`, `conductor_demo`, `hello_world` — only the last names an `assets/…` path)
  and all move with the base in this batch. A scratch scene someone saved under `examples/`
  from `editor_demo` before this batch names `assets/…` paths and is re-saved or deleted; the
  done line says so. **The template** is copied, not followed — *Use this template* cuts the
  history — so a clone keeps its own tree; for the one who merges it anyway, the template's
  `README.md` gains a line under its layout section naming the rename (`images/` →
  `sprites/`, scenes follow) (round 34 kimi F1).
- **The bundle is v5, a NEW version folder** — never a rebuild of `public/playground/v4/`. The
  five places as they stand: `web_entry.rs` header `:5–9`, `ASSET_BASE` `:30`, `BUNDLE_VERSION`
  `:32`; the build's `--version v5` (`build_wasm.sh:138` checks the constant);
  `projects.json`; the site's `src/components/PlaygroundEmbed.astro:26`,
  `src/pages/playground/preview.astro:31`, `scripts/postbuild-check.mjs:48–49`; plus
  `docs/WEB_PLAYGROUND.md` § The bundle (`:29`, `:45`) and § The bundle contract (`:55–64`),
  which also gains § Old paths under § The store (the table, the rule, the order).
- **Sizes near the ceiling:** `store/indexed_db/mod.rs` 493 and `bridge.rs` 548 (both untouched here),
  `editor_game/menu_actions.rs` 595 (untouched here), `vfs/entries.rs` 584 (untouched here).
- **The template repo** is on `jesse` on this machine, clean; its half (the folder, a sidecar for
  `ball_8px.png`, `main.scene.ron`, `README.md:54`) is staged there by the executor and committed
  by the planner on that branch, before the bundle is built from it.
- **Gates, beyond the standard set:** the new checker green over `examples/`, each
  `crates/playground/assets/projects/*`, and each game `check_games.sh` lists; `check_wasm.sh`
  (the playground crate changes); `scripts/check_audio_provenance.sh` (the `sounds/LICENSE`
  stanza moves with the file); `npm run verify` on the site over v5.
- **Leaves out, beyond the paragraph above:** any write to a visitor's store, and with it the
  rewriting of a stored scene's references; retiring the aliases (they cost five keys); a
  stored `hello_world` scene's `assets/…` paths; a stored non-bundled project; any change to
  the router.

**Done 2026-11-11: d848a9a (engine), insiculous_web 7c22f8d (site), game-template 38a458c**
(executor Claude Opus 5.5, `report-6.md` COMPLETE, 100 engine files, 45 site paths with the v5
bundle, 4 in the template; code round 37 — kimi 2 (2 m 23 s), codex 4 (4 m 29 s), the planner
4 — `rebuttal-37.md`; the planner's fix hunks touched no Rust). What the round settled beyond
the block: the gate accepts `music/imports/` (the playground's sample import writes there),
a recovered `.midi`, a recovered song's sample folder and `.jpg`/`.jpeg`/`.bmp` sprites, and
names a plain `.ron` under `scenes/` for what it is; the four new sidecars say `Linear`, the
filter each texture was drawn with before it had one; `EXAMPLES_DIR` is `ASSET_BASE`.
`examples/music/` and `examples/soundfonts/`, left outside the new base, were deleted on
Jesse's word (0795225). A scratch scene saved under `examples/` before this batch names
`assets/…` paths and is re-saved or deleted (codex, rounds 33 and 37, rebutted as policy).
Follow-up: #253 (the editor's default scene name, `scene.ron`, fails the gate).

**Batch 7 — tab drag (§ C, the gesture). Runs in this effort (Jesse, 2026-11-03), after batch 3
has shipped and Jesse has lived with the menu-driven model for at least one batch. Issue #247.** Drag a
tab (press, move past a threshold) shows a ghost on `UiLayer::DragGhost` and drop targets: every
group's tab strip (join) and four edge zones (new group); drop calls `group_panel_with` or
`move_panel_to_edge`; Escape cancels; the keyboard path is the menu (batch 3). Files: `dock/drag.rs`
(new), `render.rs`, `editor_game/mod.rs`, `drag_drop.rs` (a `DragPayload::Panel`), the guide.
Reviewers: kimi, codex. Leaves out: dragging a panel out to a floating window.

**Re-verified 2026-11-11 against 7b31178; where this and the paragraph above differ, this wins.**

*One change of design: the tab drag is the dock's own gesture, not a `DragPayload`.* The paragraph
above routes it through `drag_drop.rs`. Read against the tree, a `DragPayload::Panel` would have to
be refused at four consumers that call `take_drop_in` before they look at the payload
(`viewport_interaction.rs:34`, `texture_field.rs:97`, `sound_field.rs:51`,
`panel_renderer/conductor.rs:103`), added to three exhaustive matches (`drag_ghost.rs:13`,
`hierarchy/mod.rs:411–421`, `conductor/mod.rs:109–131`), and would count as a payload for the
spring-loaded tabs (`frame.rs:93`), so a dragged tab would bring other tabs forward as it passed.
A tab is already a widget — `ui.interact("dock_tab_{id}")`, `dock/render.rs:226` — whose
`InteractionResult.dragging` says the press is still held, and an active widget already makes
`ui.wants_mouse()` true, which is what keeps the viewport, the other panels and the scale actions
(`refuse_during_gesture`, `shortcuts.rs:190–199`) out of the gesture. So: **`drag_drop.rs` and
every file that matches on `DragPayload` are untouched**, and the asset drag's spring-loaded tabs
are untouched.

*What the tree holds.* `crates/editor/src/dock/`: `mod.rs` 288 (`DockArea` `:174`, its state
`:175–190`, `DockAction` `:101–106`, `DockPosition` `:85` — Left, Right, Top, Bottom, Center),
`groups.rs` 173 (`can_move_panel_to_edge` `:37`, `move_panel_to_edge` `:53`, `group_panel_with`
`:66` — it detaches and inserts after `other`, so within one group it reorders; `finish_move`
`:83–88`; `reveal` `:94–96`), `render.rs` 331 (`tab_layout` `:173–195`, `tab_at` `:137–147`, the
tab loop `:222–246` with the click deferred to `:118`, the collapsed strip `:207–215`, the narrow
tab `:294–299`, the overflow button and its menu `:246–277`), `layout.rs` 154, `focus.rs` 44,
`tests.rs` 416, `group_tests.rs` 303. `DRAG_THRESHOLD = 4.0` is `drag_drop.rs:14`, public.
`UiLayer::DragGhost` is `crates/ui/src/draw/mod.rs:40`. In `editor_integration`:
`run_dock_action` (`menu_actions.rs:396–413`; `MoveTo` checks the guard, says why not on the
status bar, moves, then `show_panel`; `GroupWith` calls `group_panel_with`), `render_panels`
(`frame.rs:102–137`, 299 lines), `dock_key_route` (`dock_keys.rs:29–43`, 66 lines, reached from
`shortcuts.rs:110` before the cancel cascade). **At the ceiling, not to grow:** `shortcuts.rs`
583 and `menu_actions.rs` 595 — this batch adds no line to either.

*The gesture.*
- **Start.** A left press on any drawn full tab — an expanded strip's, or a collapsed top or
  bottom group's, which keeps its tabs — that moves `DRAG_THRESHOLD` points from its press point
  while still held becomes a drag of that panel. Crossing the threshold clears text focus, as
  the tab's click does (`render.rs:118`): a focused inspector field owns the keyboard
  (`shortcuts.rs:65–67` returns before the dock's route), and Escape must reach the drag. It
  also latches the press: **its release is never a click**, dropped, cancelled or released back
  on its own tab, so a drag that moves nothing reveals nothing and changes no saved `active`.
  Below the threshold the release is today's click, unchanged. No drag starts from a collapsed
  side strip (it draws no tabs), a narrow tab, the overflow menu's rows, or while the narrow
  overlay is open. The right press is unchanged.
- **While dragging** the drag itself changes nothing in the layout, and it holds no rectangle
  from an earlier frame: the target is resolved each frame from that frame's layout. The dock's
  own keys (`dock_key_route`: F6, the arrows, Enter, the tab menu) are consumed without effect
  while a drag is live; Escape cancels it. A chip with the panel's title follows the
  pointer on `UiLayer::DragGhost`, and the one target under the pointer is drawn: a tab strip gets
  an insertion bar after the tab the panel would follow; an edge gets a band the width or height
  of a new group's header along that edge, outlined, with the edge's name in it ("Left", "Right",
  "Bottom", "Top") — text and shape, never colour alone, all from `EditorTheme` tokens.
- **Targets, in precedence order.** (1) Any drawn tab, the source's own group included and
  whatever its group's collapse: the panel is inserted after that tab
  (`GroupWith(panel, that_tab)`); over a strip's empty run, after its last tab; on its own tab,
  nothing. (2) A collapsed side strip, which draws no tabs: join that group after its active
  panel. (3) An edge zone — the outer 48 points of the dock's bounds on each of the four edges,
  corners going to the nearer side edge — only where `can_move_panel_to_edge` says yes; a zone
  that would refuse is not drawn and takes no drop. A tab or a strip always wins over the zone
  it sits in. Anywhere else there is no target.
- **Drop** on a target yields one `DockAction` — `GroupWith` or `MoveTo` — run through
  `run_dock_action`: the menu's path, its refusal message and its `show_panel` included. One
  path for the menu and the gesture. **The drop is taken and run in `render_panels` before
  `dock_area.render`**, resolved against this frame's layout (`tab_layout` needs no drawing), so
  the release frame already lays out and draws the new groups; taken after the render, as the
  click is, the frame would draw the old panels in the old rectangles
  (`frame.rs:105`, `:117–133`). A dragged panel that is hidden or gone by the release is a cancel.
- **Cancel.** Escape (through `dock_key_route`, ahead of the tab-focus release), a release over
  no target, a dialog or menu opening, Play starting and Reset Layout each end the drag with
  nothing moved. **Not a cancel, stated:** the window losing focus with the button held. Nothing
  in `input` or `engine_core` handles `WindowEvent::Focused(false)`, so a release the window
  never saw leaves every held gesture — this one, the gizmo's, the marquee's, the asset drag's —
  live until the next release; the chip is on screen meanwhile and Escape ends it. Filed as #256,
  not fixed here.
- **Persistence** is the moves' own: the settle rule saves the new groups
  (`preferences.rs:116–137`). No new preference.

*Files.* `crates/editor/src/dock/drag.rs` (new — the drag's state, the pure target resolution
from a pointer position and the frame's tab and group rectangles, the ghost and target drawing),
`dock/render.rs` (the tab loop reports the held press; the drop is taken beside the deferred
click), `dock/mod.rs` (the state field, the accessors, `mod drag`), `dock/drag_tests.rs` (new —
`tests.rs` is 416), `crates/editor_integration/src/editor_game/frame.rs` (the drop's action to
`run_dock_action`; the cancels named above), `dock_keys.rs` (Escape), a test file beside
`dock_keys_tests.rs` (203) or in it, both crate guides (a pitfall row per guard test),
`docs/WEB_PLAYGROUND.md` § the dock prose (`:488–497`) and the editor's user-facing dock text
wherever batch 3 put it. **No bundle in this batch:** the site carries tab drag from the next
bundle, which the close-out builds as v7.

*Tests (native, headless).* The target resolution, pure: each strip position, the empty run, the
source's own tab, a collapsed strip, each edge zone and a corner, a refusing edge, nowhere. Through
`DockArea` with a scripted pointer: a press released under the threshold reveals and moves
nothing; a drag dropped on another group's tab joins it after that tab; a drag within one group
reorders; a drop on a collapsed bottom group's tab joins it and makes no new group; a drop on an
edge makes the new group and drops the emptied one; Escape then the release, a drag released back
on its own tab, a release over nothing and Play each leave the layout and the captured
preferences identical — `active` included; no layout field changes on any frame before the drop;
F6, an arrow and Enter during a drag change nothing. Through the host (`EditorGame`): Escape
cancels a drag begun while an inspector text field held focus; the frame of a cross-group drop
draws the dragged panel's content in its new group's rectangle; the asset drag's `a_drag_resting_on_the_conductor_tab_brings_it_forward…` still passes
untouched. One mutation check, said in the report.

*Stills,* 1280×800, from `editor_demo`: `batch7-drag-strip.png` (mid-drag over another group's
strip, the chip and the insertion bar) and `batch7-drag-edge.png` (mid-drag over an edge band).
Reviewers: kimi, then codex with the stills; the UI eye is the planner's.

**Done 2026-11-11: 057d52e** (executor Claude Opus 5.5, `report-7.md` COMPLETE, 13 paths; code
round 41 — kimi 1 (4 m 01 s), codex 2 with the two stills (3 m 18 s), the planner 1; the planner's
fix hunks went to kimi as round 42 (2, both faults in those hunks, applied in their shapes);
`rebuttal-41.md`, `rebuttal-42.md`; Jesse drove the gesture in `editor_demo` before the commit).
Follow-ups filed: #256 (a window losing focus with a button held), #257 (a narrow side tab and a
group's first position as drop targets), #258 (the edge names in two tables).

**As built, where it differs from the block above:**
- **The drop runs in `update`, after the menu bar and before the toolbar strip** — where a
  Window-menu move runs — not inside `render_panels`: taken there, the release frame drew the
  strip in the old layout.
- **Escape takes any tab press, moved or not** (`DockArea::cancel_tab_press`, latched): a key
  arrives before the frame that promotes a press to a drag, and an Escaped press that had not
  moved must not click. A menu or a dialog opening still forgets a press under the threshold
  (`cancel_tab_drag`), so a press that closes a menu stays the tab's click.
- **The press has three phases** — held, dragging, ended — not a latch bit: a cancel ends the
  drag and keeps the latch until the release.
- **A panel alone in the dock's last group is offered no drop on its own edge**: that move
  detaches the group and pushes it back where it was. Any earlier lone group is still offered
  its edge — groups are allocated in their order, and re-pushing one last moves the corners.
- **A drag can start during Play**, as the Window menu's moves can; Escape is taken from a
  Playing game while a drag is live, and the strip's other keys stay the game's.
- The drop reaches `run_dock_action` through `run_context_menu_action`'s Dock arm, which is
  that call; the chip sits beside the pointer, and a side band's name is kept inside the dock.
- **The close-out builds bundle v7**: `docs/WEB_PLAYGROUND.md` already describes the gesture.

**Batch 8 — close-out (planner). Issue #248.** Milestone closed when `jesse` merges; `log_archive.md` §
Editor UX with the lessons; the scorecard's totals; the review record archived to
`review/_archive/editor-ux/`; the follow-ups filed (§ Follow-ups).

## Verification, end to end

After the last batch, on Jesse's machine: the editor_demo at 100 % and 150 %, the strip's icons
read without labels, tooltips name the shortcuts; View ▸ UI Scale ▸ 200 % keeps every panel usable
at 1280×800; Window ▸ Console ▸ Group with ▸ Inspector puts Console beside Inspector and a restart
keeps it there; Reset Layout restores the three default groups; New Scene from inside `sprites/`
lands in `scenes/` and opens; a renamed sprite warns about the scenes naming it; on the web,
*Open in a window* parks the page, the window edits and saves, closing it brings the page back
with the edit; the parked page's *Focus it* raises the window; a second tab on the open project
shows the owner card and *Take over* moves the editor there; drag the Console tab onto the right
edge, it becomes its own group and a restart keeps it; `npm run verify` green; `cargo test
--workspace` green; `check_games.sh` green; `check_project_layout.sh` green over every tree.

## Out of scope, said plainly

The menu bar's measured widths, mnemonics and arrow navigation, the status bar's readouts, a
command palette (#99's lines, not re-opened here — the nested dropdown batch 2 builds is the one
menu-bar change); a native file picker (#99); the games' HiDPI (#152); a separate font-size knob
(ruling (a), a follow-up); floating panel windows; a kind filter and multi-select in the browser;
undo of file operations and a persistence journal; a snapshot transfer for a dirty game page
(ruled out 2026-11-03, § E); the composer's own layout issues (#197 — External Tools batch 7's panel).

## Follow-ups to file at close-out

- A separate main-font-size knob if the one scale proves not enough (after Jesse has lived with
  batch 2).
- Drag a tile into a folder in the asset browser; a kind filter.
- A read-only second tab on an owned project (the owner card offers only *Take over* today).
- Floating panels (the drag batch's natural next step).
