# Editor UX — the plan

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
- **(h) On the web, opening the editor in a window is an acknowledged hand-off: one writer per
  project, always.** The engine's event loop cannot be stopped once booted
  (`playground-embed.ts:113-118`), and two editors on one IndexedDB project would write over each
  other. So a project has an **owner record** (`localStorage`, one key per slug) and a transfer is
  two-phase in both directions: the destination boots and loads, says *ready*; the source freezes,
  saves, drains its persistence, says *retired*, and parks itself on a card (*Focus it* · *Bring it
  back*); the destination unfreezes. A boot that fails, a window that closes early or a deadline
  that passes leaves the source editing. A second tab on the same project meets the owner record
  and offers *Take over* through the same transfer. The Playground refuses the window while its
  store is the in-memory fallback (its only copy would not survive the navigation); a Rust game's
  editor page, whose edits never persist, warns while dirty before opening. § Design E. *Amended
  after round 2 (2026-11-03): a Web Lock decides ownership and the record only describes it; the
  destination loads after the source retires; an invitation bypasses the card for one boot.*
- **(i) Rename and delete land with create, in their own batch (4b); there is no trash.** Delete
  asks through the existing confirm dialog and removes the file (or the folder, named with its
  count). A paired asset — a `.png` with its `.sheet.ron`, a `.mid` with its `.mid.ron` and any
  `.recovered-*` of its stem — is one asset to rename, delete and show. On the web a folder is the
  set of paths under it; an empty one is kept by a hidden `.keep` entry the scanner skips and the
  export *keeps* (the importer drops directory entries, so the file is what carries the folder
  through a round trip — round 2, kimi F11); a folder made today is there tomorrow (round 1,
  gemini F4); natively `mkdir` is enough.
- **(j) — PROPOSED after round 4, Jesse's to rule — the native file rename's residual.** A
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
    pub pending_folders: Vec<String>,           // created this session, empty so far (ruling (i))
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

**The pages.** Two routes per editor surface, one layout prop. `pages/playground/window.astro`
renders `PlaygroundEmbed` + `PlaygroundToolbar` (the Playground's shape, project controls
included); `pages/playground/[slug]/window.astro` renders `GameEmbed` with `fill` + the game
page's bar (the `[slug].astro` shape — a Rust game's editor is its own bundle through `GameEmbed`,
not the Playground's; codex F9). Both inside `AppLayout` with a new `windowed` prop: the studio
wordmark nav, the footer and the visible heading row go (the `h1` stays, visually hidden — every
page keeps its name for the audits), the Aa controls and the skip link stay, the bar is the
window's top edge, and `main` fills the viewport. `noindex`; the routes carry `data-wasm-src` like
the others, so `postbuild-check.mjs` resolves them. The dock (scripts, console) is in the
Playground's window too — it is the accessible half of the editor (#187) and travels with it.

**The owner and the lock** (ruling (h); round 1 gemini F2, kimi F4; round 2 kimi F2, F7, codex
F3, F5). An editor holds a **Web Lock** for its lifetime —
`navigator.locks.request('beinsiculous.playground.<surface>:<slug>', { ifAvailable: true }, hold)`
— because a `localStorage` read-then-write is not a lock: two tabs booting at once both read
nothing and both win (kimi F2). `<surface>` is `playground` or `game`, because the bundled data
project `pong` and the Rust game page `/playground/pong/` share a slug and nothing else (kimi F7);
the window name and every channel message carry the same pair. Beside the lock, the **owner
record** `localStorage['beinsiculous.playground.owner.<surface>:<slug>']` =
`{ token, generation, windowName?, windowRoute, returnRoute }` exists for the *text* of the cards,
for reacquiring the window by name and for the parked page's next generation number (a static
page has no other memory of `N`, gemini F3 round 3); it is never what decides ownership. A boot that cannot get the
lock stops at the card "This project is open in another tab or window — *Take over* · *Close*"
(Take over runs the transfer below with the holder as the source); a lock that is free is taken
and the record overwritten. **Storage that throws** (a browser policy, codex F5): the lock request
and the record read are each in try/catch; without the lock API or with storage denied, the
*Open in a window* button is disabled with its reason in the bar note and the page runs exactly
as today. Nothing here precedes the memory-store fallback. **Ownership is of the mutation's
target, not of the active page** (codex F3, round 3): `playground_import_zip` replaces the project
the *archive's* manifest names (`bridge.rs:437-455`), so import and reset act on the owned slug
only, and an archive naming another slug takes that slug's lock (`ifAvailable`) first or is refused
naming the holder. A tab from the release before this one holds no lock and is not fenced — today
two tabs already both write, so the window after a deploy is the state we have now, ending when
that tab reloads; `WEB_PLAYGROUND.md` says the guarantee is among clients of one release.

**The transfer** — one protocol, both directions, a named state machine with a commit point
(round 1 gemini F1, kimi F1–F2, codex F1, F10; round 2 kimi F1, F3, F4, gemini F1–F3, codex F1,
F2, F4, F14). `BroadcastChannel('beinsiculous.playground.transfer')`, every message carrying the
surface, the slug, a generation and the sender's token; `playground-window-protocol.ts` defines
the states and the messages as `playground-preview-protocol.ts` does for the preview, and its
reducer is unit-tested in TypeScript. **Every continuation re-reads the state and the generation
after each `await`; a cancelled transfer's continuations are no-ops** (codex F4: a `ready` that
lands at 19.9 s must not let the save's continuation park a source whose transfer already rolled
back).

1. **Open** (the bar button **Open in a window ↗**, beside Play ↗, enabled on `playground-ready`
   while the lock is held and storage works). In the click handler, *synchronously*: `handle =
   window.open('about:blank', windowName, 'popup,width=1280,height=800')` — the activation is
   spent here, before any `await` (gemini F3, round 1); `null` → the static `#window-blocked`
   alert with *Try again*, the preview's pattern. Then `handle.location = windowRoute +
   '?project=<slug>&invite=<token>:<N+1>'`, `N` the owner record's generation. The source keeps
   editing. State `inviting`.
2. **Ready — queued, without the project.** The window boots, reads the invitation, and **does
   not load the project**: an invited boot whose `invite` is `<holder token>:<holder generation
   + 1>` skips the owner card (kimi F3, gemini F2, round 2) — **one admission rule in both
   directions** (gemini F3, codex F5, round 4: the record holds `N`, every invitation is `N+1`,
   and the commit writes `N+1`); a stale, consumed or absent invitation stops at the card as any
   boot does. It **requests the lock with a normal, queued
   request** — not `ifAvailable` — under an `AbortSignal` of 30 s, so it stands in the
   LockManager's queue behind the source (kimi F3, gemini F1, codex F8, round 3: a third tab's
   `ifAvailable` request now sees a pending request ahead and gets `null`, and the grant cannot
   lose to the manager's latency). Then it shows "Taking over…" and posts `ready(generation)`.
3. **Retire.** The source, on `ready`, runs **one privileged operation, `playground_retire()`**
   (kimi F1, codex F7, round 3: the Save button's `playground_dispatch('save')` only `try_send`s
   onto a channel for a later frame, `bridge.rs:101-106`, and a frozen bridge would refuse it as
   the write it is): it **freezes** public mutation — the frame driver hands the editor an empty
   input frame (no key reaches `handle_editor_key`, no press reaches a widget: the ui's blocking
   region alone would leave Ctrl+Z live, codex F2), every public write export answers `frozen`,
   the embed disables every bar control (Project, Save, Export, Import, Reset — DOM, outside the
   ui, kimi F4) — then **settles** the DOM editors (the Scripts panel's `saveCurrent()`,
   `playground-scripts-panel.ts:50`, whose buffer the navigation would otherwise destroy; a canvas
   text field mid-edit by the path a focus loss takes today, which the executor names), **saves**
   the scene and the song through the internal save path, **drains** persistence through
   `drain_then_epoch` (`persist/mod.rs:333`, whose IndexedDB transactions settle on `oncomplete`,
   `store/idb_transaction.rs:46-83` — gemini F2 round 4, rebutted), **verifies** nothing is dirty,
   and resolves — or rejects with the first error, and the source thaws with the banner. **This
   operation needs a seam in `editor_integration`** (codex F6 round 4): keys reach the editor as
   `WindowEvent` callbacks into `Game::on_key_pressed` (`app_handler.rs:341`), not through a frame's
   input, and `EditorRunOptions` (`run_options.rs:15`) exposes neither a gate nor a save — so
   `EditorGame` gains `set_frozen(bool)` (drops key callbacks and widget input while set) and
   `retire() -> Result<(), String>` (the saves), reached through a host handle on the options; batch
   5 therefore touches `editor_integration` and ruling (e)'s exception is withdrawn. **On a game
   surface, this step is where the unconditional loss dialog lives** (kimi F3, codex F2 round 4):
   *Open in a window*, *Take over* and *Bring it back* all retire the source, and a game surface's
   retire asks "Your edits here will be lost — *Continue* · *Cancel*" every time; Cancel keeps
   ownership and editing where they are, and the window's own edits on return are covered by the
   same step. Then the embed **commits**: writes the owner record with the window's token and the
   generation *before* releasing anything — a write that throws aborts here and the source thaws
   (codex F5) — releases the lock, posts `retired(generation)`, and navigates to
   `/playground/parked/?project=<surface>:<slug>`.
4. **Live.** The window's queued request is **granted** — that grant, not `retired`, is the
   authority (a source that dies after releasing still hands the lock to the queued destination;
   `retired` is informational). On the grant the window **consumes the invitation**
   (`history.replaceState` strips `invite` so a reload is an ordinary boot, kimi F4 round 3),
   **loads the project** — the bytes the source just flushed, so there is no stale copy and no
   `StaleRevision` on its first save (round 2's three F1s) — thaws, and is the editor. **A load
   that fails after the grant is a recovery card** (kimi F5, codex F7 round 4): the boot's
   `load_project(...).await.unwrap_or_default()` (`web_entry.rs:193`) would show the bundled
   project as if nothing were stored; on this path a failed read keeps the destination frozen with
   *Retry* · *Return* and holds the lock, and a destination whose store is the memory fallback
   refuses before `ready`.
5. **Failure.** Before the commit: the window's boot fails, the window closes before `ready`, or
   20 s pass (`PREVIEW_BOOT_DEADLINE_MS`) — the source thaws, closes the handle, shows the banner
   "The window did not start — the editor stayed here", and holds the lock throughout; a
   `retire()` that rejects does the same with its error. After the commit the source never
   thaws: a destination whose grant does not come within its 30 s (the lock went elsewhere, or
   the manager never answered) aborts its request and stops at the owner card with *Take over*,
   and the parked page's *Bring it back* recovers the project.

**Bring it back** is the same transfer with the parked page as the destination: *Bring it back
here* reads `generation` from the owner record and navigates the parked page to `returnRoute +
'?project=<slug>&invite=<windowToken>:<N+1>'`; that page boots, finds the invitation valid, queues
for the lock, posts `ready` without loading; the window runs step 3 (`retire()`, commit, release,
`retired`) and shows its card "The editor moved back to the page — *Close*"; the page's grant
lands, it loads, goes live. The deadline on this side leaves the
page at the owner card with *Take over* (which repeats the request) and *Focus it*.

**The parked page** (`pages/playground/parked.astro`, static, `AppLayout`, no engine): "The editor
for *Pong* is open in its own window." · **Focus it** — pings the window (2 s, inside the click)
and, on an answer, `window.open('', windowName)` reacquires the live window by name, because the
original handle did not survive the source's navigation (kimi F2, round 1); no answer and a free
lock → the page returns by itself; no answer and a held lock → "the window is not answering",
nothing is opened (an unanswered `window.open` by name creates and flashes a blank popup, gemini
F8 round 3) · **Bring it back here**. **The page
never polls the name**: `window.open` from a timer has no activation and returns `null` or a
blocked popup, which proves nothing (gemini F3, codex F14). Instead the window posts
`closing(surface, slug, token)` on `pagehide` — **a hint, not proof** (kimi F4 round 3: a reload
fires it too): **the parked page never navigates on it** (gemini F1 round 4: a cold wasm reload takes longer
than any timer, and a timer that fires first steals the project from the reloading window): the
card changes to "The window closed — *Bring it back here*", and the click decides; the click's
own ping and `navigator.locks.query()` tell a reloaded window from a gone one. A parked page that
boots (or returns from a hidden tab, `visibilitychange`) pings once with a 2 s window, the one
ping the heartbeat refusal allows; no answer and a free lock → it returns by itself; no answer
and a held lock → the window is alive but throttled, and the card waits.

**A window closed by the browser** (round 1 gemini F1, kimi F2, codex F2): the window page sets
`beforeunload` to warn while any document is dirty (the browser's own prompt — the only one it
allows); an autosave in flight at unload is not guaranteed to land, and `WEB_PLAYGROUND.md` says
so: a direct close returns the page to the last *durable* save. The card's *Close* button is the
clean path (save, flush, release, close). The lock is released by the browser when the window
goes, whatever the path.

**Two refusals** (codex F2, round 1; gemini F4, round 2). The Playground's button is disabled, with
its reason in the bar note, while the store is the in-memory fallback (IndexedDB unavailable — the
banner already says edits will not persist; parking would discard the only copy). A game surface
**always** confirms before it retires — "**Your edits here will be lost** — the window starts from
the game's bundled scene. *Continue* · *Cancel*" — at every entry (v3 said "do not travel", which
implied they stayed; the source navigates away and they do not, gemini F4 round 2; it asked only
while *dirty*, but a game-page Save clears the flag while the bytes live only in that page's memory
store, codex F6 round 3; and it asked only at the button, while *Take over* and *Bring it back*
retire too, kimi F3 and codex F2 round 4 — so the dialog is unconditional and lives in the retire
step). A game page has no
store, so its transfer is steps 1, 2, the commit and the park with no save. A snapshot transfer or
an in-place park (the frozen engine kept alive under an overlay) is not built: the game bundle has
no load-from-snapshot path in editor mode, an in-place park keeps a second live engine per bundle,
and the page's contract is "nothing persists" — Jesse ruled 2026-11-03 that the dialog is enough.

**The playground crate.** Three bridge exports — `playground_retire()` (freeze, settle, save,
drain, verify — the one privileged path; `playground_set_frozen(bool)` is its first step and the
destination's own freeze), `playground_load_project()` (the deferred load a boot with an invitation
skips) and the `files-changed` response the Scripts panel reads (§ D) — in `bridge.rs`
+ `persist/` + `web_entry.rs` (the invitation read beside `?project=`; the lock and the record are
the site's, in TypeScript, because the Rust side never touches `localStorage` today). The freeze
lives in the playground crate, not in `editor_integration`, so batch 5 stays disjoint from
External Tools batch 7 (ruling (e)). No bundle version bump: the glue grows two exports the v2 bundle's
`game.js` must carry, so the bundle is rebuilt in place as part of the batch (as batch 6 of
Playground UX rebuilt it), and `WEB_PLAYGROUND.md` § The preview window gains a sibling § The
editor window with the protocol table.

**Gates.** `npm run verify` whole (axe over the four new pages, the phone and 125 % shots, the
announce check — the parked page's card and the owner card are live regions), the engine set +
`check_wasm.sh` for the crate; a Playwright test of the protocol over the memory store if the
site's screenshot harness can host two pages (the executor reports whether it can; if not, the
protocol's state machine is unit-tested in TypeScript and the two-page run is Jesse's). Jesse's
Chromium check: open in a window, edit, *Bring it back*, the edit is there; close the window with
the card's *Close*, the page returns; the blocked-popup path; *Focus it*; a second tab on the same
project meets the owner card and takes over.

**Leaves out.** Opening a *panel* in its own window (the engine has one canvas per page);
remembering the window's size and position (the browser does); Firefox's popup heuristics beyond
the blocked alert; a snapshot transfer for a dirty game page (ruled out 2026-11-03); a heartbeat
(the preview protocol's reasoning: a hidden window's timers are throttled, so silence cannot tell
dead from sleeping — `ping` is asked only at a boot, with a 2 s answer window, never as a pulse).

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

**Batch 5 — the editor in its own window (§ E). Issues #245 (engine) and insiculous_web#79
(site). NOT SETTLED — its design continues under
`review/editor-ux-window/`; its issue is filed blocked on that review. Site + playground crate +
the retire seam in `editor_integration` (codex F6 round 4), so it queues like every other batch.**
Files, `insiculous_2d`: `crates/editor_integration/src/editor_game/{mod,run_options}.rs` (the
`set_frozen` / `retire` seam on a host handle), `web_entry.rs:193` (the strict post-grant load);
`insiculous_web`: `src/layouts/AppLayout.astro` (`windowed`), `src/pages/playground/window.astro`,
`src/pages/playground/[slug]/window.astro`, `src/pages/playground/parked.astro`,
`src/components/PlaygroundToolbar.astro` (the button, the blocked alert, the owner card),
`src/components/GameEmbed.astro` (the game page's button and dialog),
`src/scripts/playground-window-protocol.ts` (new: the owner record, the channel, the message
shapes, the names), `playground-window-launch.ts` (new: the transfer's source side),
`playground-window-boot.ts` (new: the destination side — the owner check, `ready`, the freeze),
`playground-embed.ts` (wire both; every bar control disabled from launch to live-or-rollback),
`src/scripts/playground-window-protocol.ts` + `playground-window-launch.ts` (new: the state
machine and its reducer, the lock, the invitation, the `closing` listener — the preview pair's
shape), `scripts/postbuild-check.mjs` if the new routes need naming,
`README.md` § Content / the playground section, `docs/roadmap.md` one line. `insiculous_2d`:
`crates/playground/src/bridge.rs` + `persist/` (`playground_flush`, `playground_set_frozen`,
`playground_load_project`; the frozen refusal in every write export), `web_entry.rs` (the
invitation read beside `?project=`; the deferred load), `docs/WEB_PLAYGROUND.md` § The editor
window, the v2 bundle rebuilt and synced (`scripts/build_wasm.sh … --sync ../insiculous_web/public`,
the invocation of record in `WEB_PLAYGROUND.md` § The bundle). Gates: `npm run verify` in
`insiculous_web`; the standard engine set + `check_wasm.sh` in `insiculous_2d`; the protocol's
state machine unit-tested with the cancelled-continuation case; a two-tab lock race in the
browser harness if it can host two pages; Jesse's browser check (§ E). Reviewers: kimi, gemini
(the protocol), codex (the window, the parked card and the owner card — three stills). Leaves
out: § E's list.

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
v3/` lands with it), and **a visitor's stored v2 scenes migrated** (codex F3): a saved scene
over the bundled pong names `images/paddle_16px.png`, and the v3 bundle has only `sprites/`; the
store already knows a stored file's `bundle_version` (`projects.rs:25,82`), so the boot migrates
a project whose stored version predates v3 — **the whole `images/` family, not the references
alone** (kimi F5, codex F6): every stored file under `assets/images/` (a customised paddle, a
visitor's own `custom.png`, each sidecar) is put under `assets/sprites/` and its old record
tombstoned; a tombstone under `images/` becomes one under `sprites/`, so a deliberately deleted
bundled paddle stays deleted. **A content collision** — `images/hero.png` and `sprites/hero.png`
both stored and different — **leaves that family unmigrated and its references untouched**, named
in the console, never substituted (round 3 kimi F7, codex F1: the stored `images/` file still
loads from the map). **The order is puts, then references, then tombstones**, so a run cut at any
point is loadable — both paths exist until the scenes name the new one (gemini F4 round 3).
**Before a tombstone, the old record's content hash is compared with what was copied; changed,
the put is redone** (kimi F2 round 4: the editor opens between boots and a visitor may save over
the old path). **While the marker is present, a read of `assets/images/<x>` that is not stored
resolves to the bundle's `sprites/<x>`, tombstones respected** (codex F4 round 4: a visitor who
stored only the scene names a bundled `images/` file the v3 bundle no longer has). **A collision
goes to `skipped`**, beside `remaining`, so the migration completes and the banner shows once
(gemini F5 round 4). **Progress is a marker in the manifest**, `migration: { images_to_sprites: {
remaining: [...], skipped: [...] } }`, not `bundle_version` (round 3 kimi F2, codex F9: an ordinary save stamps v3 on an unconverted
scene, and a tombstone keeps v2 and would be "migrated" again into deleting the new file); each
transition is idempotent, a reboot is tested after every boundary, and the marker is cleared only
when nothing remains; a boot whose migration fails continues into the editor with a banner naming
the count left. Tests over a real v2 saved scene: a customised bundled file, a visitor-only file, a tombstone, a
collision left in place and listed as skipped, an ordinary save over the old path during a
half-done migration (the redone put), a scene-only visitor's bundled reference while incomplete,
and a reboot after each boundary. **First step:** run the new checker over every current
tree and list each violation in the report before a file moves (kimi F11). Gates: the standard
set; the checker over every tree it names; `check_wasm.sh`; `npm run verify` on the site for the
bundle. Reviewers: kimi, gemini (the checker, the bundle bump and the migration are seams). Leaves
out: the six games' `sprites/` (DEION_STYLE's); renaming anything under `deion_assets`; a
font's filename (the stated exception).

**Batch 7 — tab drag (§ C, the gesture). Runs in this effort (Jesse, 2026-11-03), after batch 3
has shipped and Jesse has lived with the menu-driven model for at least one batch. Issue #247.** Drag a
tab (press, move past a threshold) shows a ghost on `UiLayer::DragGhost` and drop targets: every
group's tab strip (join) and four edge zones (new group); drop calls `group_panel_with` or
`move_panel_to_edge`; Escape cancels; the keyboard path is the menu (batch 3). Files: `dock/drag.rs`
(new), `render.rs`, `editor_game/mod.rs`, `drag_drop.rs` (a `DragPayload::Panel`), the guide.
Reviewers: kimi, codex. Leaves out: dragging a panel out to a floating window.

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
