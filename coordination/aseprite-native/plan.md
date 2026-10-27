# Aseprite Native — the plan

**Settled at v3 — 2026-10-27, Jesse.** Rounds 1–2 are adjudicated in `review/aseprite-native/`.
Sprint: milestone "Sprint: Aseprite Native" on `beinsiculous/insiculous_2d` — #228 (batch 1, the
reader and the parity gate), #229 (batch 2, the loader seams), #230 (batch 3, the texture reload),
#231 (batch 4, pong on masters and the docs) — and its art half `beinsiculous/deion_assets#19` in
that repo's milestone of the same name; the clip expansions are #232, outside the sprint. Nothing
is handed off before External Tools batch 3 lands (ruling (c)).

**v3 — 2026-10-27, after round 2** (`review/aseprite-native/rebuttal-2.md`: kimi 8, gemini 9,
codex 4 — twenty-one findings, eighteen accepted, two rebutted against the tree, one in part).
What moved: **the reload transaction lives at the host**, where the asset manager and the render
manager are both reachable, and ends by writing the displayed region of every sprite on the
master and replacing the prepared-sheet cache entry; **an unresolved clip keeps its name** and
stops on the first cell, so a renamed tag loses no authored binding; a removed cell falls back to
the first; **the scene loader sets a plain sprite's first cell** — a full-texture region on a
master means the first cell, by value, no wire change — and the undoable command is for the drop
and the texture field only; an encoded 64 MiB limit is checked by `vfs::file_size` before any
master is read; a cel is bounded by its own dimensions, never the canvas; indexed cel opacity is
refused with the layer rule; palette chunks honour their first index; the parity check compares
`fps` within the exporter's precision; a playground re-import reaches loaded textures; a sheet's
PNG with an invalid sidecar keeps both halves until the pair validates; the upload pipeline names
the master's destination, limit and validation.

**v2 — 2026-10-27, after round 1** (`review/aseprite-native/rebuttal-1.md`: kimi 9, gemini 9,
codex 6 — twenty-four findings, twenty-three accepted, one half-rebutted against the tree: a
dropped texture resource cannot crash a cached bind group, wgpu refcounts the view). What moved:
**explicit limits before any allocation** (canvas, frames, the composed sheet, total decoded
bytes), the cel-length rule withdrawn; the chunk-count fallback corrected (new unless zero, then
old); named palette entries skipped; negative cel offsets intersected; linked cels resolve to an
image cel in an earlier frame only; a zero duration refused; **more refusals, each naming its
layer or frame** — a group with opacity or a blend mode, a z-indexed cel, a palette after the
first frame — and the background layer opaque as in Aseprite; **the sheet is `min(frames, 4)`
columns**, as the exporter's is, and the parity check compares a transparent pixel by alpha
alone; **a plain `Sprite` gets the first cell's region** at a drop, an assignment or a scene
load, through the undoable path, and is re-mapped to its cell when a reload changes the grid; the
replacement goes through the renderer and evicts **both** pipelines' caches, the GPU write last;
a resized PNG with a sidecar runs the sheet transaction; a shrunk or removed active clip is
reconciled by name; a failed texture path is a tombstone; a half-written save keeps the old
pixels and retries; a game-held `SpriteSheet` is a stated limit.

**v1 — 2026-10-27, drafted.** Written by the planner (Claude Fable) from Jesse's brief of the same
day, given while the External Tools plan (`coordination/external-tools/plan.md`) was being revised
to v6: open art in an external editor too, Aseprite by default; and, since the editor can then
open Aseprite, read `.aseprite` files directly, so an animation is authored once as Aseprite tags
and never re-exported as a sheet plus a `.sheet.ron`. The launch half — an `Art` tool category, an
`Image` row and an Aseprite preset — is three table rows in External Tools' one mechanism and
landed there as v6, ruling (m). The native half is this effort: it needs a binary decoder the tree
does not have, a renderer seam that does not exist (nothing today can replace the pixels behind a
`TextureHandle`), and a reversal of a written deion_assets rule. Jesse's rulings at kickoff are in
§ The rulings. Review round 1 (kimi, gemini, codex, one snapshot) dispatched the same day; this
file is revised after each rebuttal and the plan is settled when Jesse says so. Nothing is handed
off before External Tools batch 3 has landed (ruling (c)).

Effort directory: `coordination/aseprite-native/` (this plan, `reviewer-comparison.md`). Review
transients: `review/aseprite-native/` (the plan snapshot, `review-N-<reviewer>.md`,
`rebuttal-N.md`, `handoff-N.md`, `report-N.md`, `draft-N.diff`). Roster: Claude Fable plans,
adjudicates with Jesse, applies accepted fixes and commits; a Claude Opus session (Jesse's other
window) executes each handoff; kimi reviews every plan round and every diff; gemini (the
Antigravity CLI) reviews the decoder (a parser is a seam), the texture replace and the sync
script; **Astra (codex) reviews every plan round and every batch** — an asset is in all of it. A
vendor never reviews its own vendor's work.

Sprint: a milestone "Sprint: Aseprite Native" on `beinsiculous/insiculous_2d`, its issues filed at
settling (`file-issue`), one per batch plus the follow-up § Out of scope names; the deion_assets
half gets its own issues on that repo. Nothing on the board moves before Jesse has ruled.

## Context

**Goal.** A sprite sheet's source of truth is its Aseprite master. The engine reads the master —
its frames, its layers composed, its tags as clips — and a game ships the master, so drawing in
Aseprite and seeing it in the editor is one save, and no PNG or `.sheet.ron` is generated for a
Deion game again. The traditional path — a PNG and its `.sheet.ron` sidecar — stays a first-class
feature for users who author sheets any other way (ruling (a)); the engine supports both, and
nothing in the frozen schema moves.

**What the pipeline is today** (read 2026-10-27; `deion_assets/scripts/export_sheet.py`, 280
lines; `scripts/sync_sprites.py`, 308 lines). Astra draws the master at
`ai/<style>/<subject>/ai_<subject>.aseprite` in indexed mode, material layers and named tags. The
exporter runs `aseprite -b <master> --sheet-type rows --sheet-columns 4 --sheet … --data …
--format json-array --list-tags`, validates the JSON and writes the PNG and the `.sheet.ron`
beside the master; all three are committed. Its grammar: the cell is the canvas size; frames sit
in a 4-column grid; a tag's name is lowercase snake_case; its direction is forward; its frames
share one duration, so `fps = 1000 / ms`; repeat 0 is `looping: true`, 1 is `false`, anything
else an error; `filter: Nearest` always. `--check` re-exports into a temp directory and compares
byte for byte. A game's `assets/sprites/sync.list` names PNG sources under a pinned deion_assets
sha; the sync refuses any source or destination that is not `.png`, derives the sidecar, reads
both from the sha with `git show`, keeps the `ai_` prefix and owns the folder;
`scripts/check-sprite-sync.sh` in the working set runs it over every game. Every rule that says
"PNG" is in § What the tree holds.

**What the engine is today.** `sheet_file.rs` holds the frozen `.sheet.ron` v1 schema and
`into_parts` turns a parsed sidecar and the PNG's dimensions into a `SheetGrid`, the clips and the
filter; `sprite_sheet.rs` prepares a sheet from its sidecar and loads the texture through the
`(path, filter)` dedupe; `TextureResolver::sheet_for` lets a scene's `SpriteAnimation` take the
sidecar over its baked snapshot. `ClipData` is `frames`, `fps`, `looping` — one rate per clip, no
ping-pong. A texture, once uploaded, is never written again: `insert_rgba` allocates a new handle
every time and the sprite pipeline caches a bind group per handle. `flate2` and `miniz_oxide` are
already in `Cargo.lock` through `png`, and the playground already depends on `flate2` directly.

## The rulings (Jesse, 2026-10-27, at kickoff)

- **(a) The sync carries the master; the PNG path stays.** deion_assets' sync copies the
  `.aseprite` master into a game, the `ai_` prefix rule unchanged; the PNG + `.sheet.ron` pair stays
  a first-class engine feature for users who author traditional sheets; `export_sheet.py` stays as
  deion_assets' own gate (palette, cell grid, byte-identical re-export) and for the site's
  capsules, which remain PNG.
- **(b) The engine reads what the exporter writes — one grammar.** The reader accepts exactly the
  exporter's tag grammar (forward, uniform durations, repeat 0 or 1, snake_case names) and lays
  the sheet out exactly as the exporter does (4 columns, cell = canvas), so a master decoded by
  the engine equals its committed PNG and sidecar byte for byte, and **that parity is the standing
  gate**. Reverse, ping-pong and non-uniform durations are lossless expansions filed as a
  follow-up, not done here; `ClipData` and `SheetFile` do not move. (Planner's proposal, standing
  until Jesse or a review overturns it.)
- **(c) Order.** This plan is reviewed now, in parallel with External Tools batches 1–3; its first
  handoff waits for External Tools batch 3 (the watcher) to land, and one batch is out at a time
  across both efforts.
- **(d) No new dependency.** The decoder is in-house over `flate2`, which the tree already builds;
  `asefile` and its kin are not taken (they pull `image` with their own features and 3,000 lines
  for a format whose needed subset is 500). The simplest thing that works.

## What the tree holds (verified 2026-10-27 by the planner)

| concern | what is there | verdict |
|---|---|---|
| The sheet schema | `crates/engine_core/src/sheet_file.rs`: `SheetFile` v1 (`cell`, `filter` defaulting Nearest, `clips: Vec<(String, ClipData)>`), `sidecar_path_for` (`:78`), `parse_sheet_file` (`:86`), `into_parts` (`:132-157`) → `SheetParts` = `(SheetGrid, Vec<(String, AnimationClip)>, TextureFilter)`; **frozen Jul 30 2026** | **Untouched.** A master yields the same parts by another reader |
| `ClipData` | `scene_data/wire.rs:52-60`: `frames: Vec<u32>`, `fps: f32`, `looping: bool`; `From` both ways with `ecs::AnimationClip` (`sprite_components.rs:166-173`); no per-frame duration, no ping-pong | **Untouched** (ruling (b)) |
| Sheet loading | `assets/sprite_sheet.rs`: `prepare_sheet(base, ref)` (`:97-125`: the sidecar's text, `png_dimensions` from the header only, `into_parts`) → `PreparedSheet { sheet: SheetData, filter }`; `SidecarCache` (`:155`, one read per path per scene load, `clear()`); `load_sprite_sheet` (`:236-253`) = prepare, then `load_texture_filtered(path, filter)`; `sidecar_filter`, `sidecar_sheet` | **Extend**: a `.aseprite`/`.ase` ref prepares through the reader; its composed pixels ride the same cache entry |
| The resolver | `texture_ref.rs`: `TextureResolver { resolve_texture, sheet_for, clear_sidecar_cache }` (`:31-49`), `SheetData { grid, clips }` (`:18-21`); `resolve_texture`'s file arm (`:68-101`) asks `sidecar_filter` then loads by path | **Extend**: the file arm loads a master's RGBA by path |
| Texture loading | `assets.rs`: `loaded_by_path: HashMap<(String, TextureFilter), TextureHandle>` and `handle_to_path` (`:131-145`); `load_texture_filtered` (`:219-254`) dedupes on the path as given; `load_texture_from_bytes` (`:304`) records no path; `create_texture_from_rgba` (`:416`) records the `"#rgba"` sentinel; `set_base_path` (`:472-476`) clears the dedupe. `renderer/texture.rs`: `TextureManager::load_texture` decodes with `image::load_from_memory` then `insert_rgba` (`:224-256`), which always allocates a new handle; `create_texture_from_rgba` (`:354-400`) one `queue.write_texture`, usage `TEXTURE_BINDING \| COPY_DST`; `TextureManager::headless()` (`:156-165`) validates and uploads nothing; `remove_texture` (`:326`) | **Extend**: a path-recording RGBA load; `replace_rgba` behind a handle |
| The sprite pipeline | `renderer/src/sprite/pipeline.rs:28` `texture_bind_group_cache: HashMap<TextureHandle, BindGroup>`; `cache_texture_bind_group` (`:194`) skips a handle already cached | **Extend**: `evict_texture_bind_group(handle)` on a rebuilt resource |
| Asset browser | `editor/src/asset_browser.rs`: `AssetKind { Image, Scene, Script, Sound, Song, SoundFont }` (`:15-30`), `kind_for_extension` (`:98-108`, `png\|jpg\|jpeg\|bmp` → Image), `scan_assets` over `vfs::list_files`; thumbnails through `assets.load_texture(relative_path)` (`panel_renderer/asset_browser.rs:229-248`, capped per frame); an Image tile drags `DragPayload::Texture { handle, path }` (`tile.rs:35-41`) onto the viewport or a texture field | **Extend**: an `Aseprite` kind with the first frame as its thumbnail, dragging as a texture |
| Scene round trip | `scene_loader_components.rs:345-380` `build_sprite_animation` asks `sheet_for(path)`; the sidecar wins over the baked `GridData`/`ClipData`; `scene_loader.rs:130` clears the sidecar cache per load; `scene_serializer` writes `sheet: Some(path)` with the baked snapshot | **Works as is** once `sheet_for` answers for a master; a round-trip test proves it |
| Wasm | `common/src/vfs/mod.rs`: `read` is a thread-local `MemFs` filled at boot from `manifest.json` (`engine_core/src/web/mod.rs:274-318`); `scripts/build_wasm.sh:316` lists every file under `assets/` with `find -type f` | **Works as is**; bytes are opaque |
| The playground | `crates/playground/src/archive.rs`: import has no extension allowlist (content checks by suffix for `.sheet.ron`, `.scene.ron`, `.rhai`, songs); `upload.rs:34-43` `UploadKind::of` allows `mid wav sf2` only; `flate2` with `rust_backend` already a dependency (`Cargo.toml:24`) | **Extend**: the upload allowlist gains `aseprite`/`ase`; the archive passes a master through as opaque bytes already |
| Inflate | `Cargo.lock`: `flate2` 1.1.5, `miniz_oxide` 0.8.9, `fdeflate` 0.3.7 (via `png`); `image` 0.25 with `png jpeg bmp gif` in the workspace manifest (`Cargo.toml:50`) | **Direct dependency** of `engine_core` on `flate2` with `rust_backend`, the playground's own line; no new code enters the tree (ruling (d)) |
| deion_assets | masters committed beside their PNG + RON; `export_sheet.py` (its grammar in § Context; the Aseprite binary from `$ASEPRITE`, then `PATH`, then `~/.steam/root/steamapps/common/Aseprite/aseprite`, `:21-28`; the version printed, never pinned); `sync_sprites.py` (`:90-103` refuse non-`.png`; `:105` derives the sidecar; `:141-160` reads from the sha; `:128-134` the `ai_` rule; `:280-284` owns the folder); `check_character_layout.py:18-19` wants master + PNG, or those + RON; `check_sheet.py` is the §3 palette gate over the PNG; `tests/test_sync_sprites.py`, `tests/test_export_sheet.py`; `tests/fixtures/make_fixture.lua` builds a master with `aseprite -b --script` | **Extend** the sync; the exporter and its gates stay (ruling (a)) |
| The rule text | `DEION_STYLE.md:624-627` (every sheet PNG ships with a sidecar), `:746-750` (the master is the metadata source), `:549` (sync is the only way art enters), `docs/BRANDING.md:117` ("do not copy Aseprite masters into a game"), `scripts/check-sprite-sync.sh:3-8` (D3, 2026-09-15: PNG and sidecar), `PROJECT_ROADMAP.md:42-43` ("tiles/props stay offline PNGs (never runtime rgba)") | **Rewrite** to ruling (a); the roadmap line meant "no procedural art at runtime" and is restated so |
| Fixtures and provenance | `crates/engine_core/tests/fixtures/`; `scripts/check_audio_provenance.sh` wants a LICENSE in Debian copyright format beside any fixture under `crates/<crate>/fixtures/`, audio only today | **Generalise** the script to every binary fixture — one rule, one script — and add a master fixture under it |
| Aseprite itself | `aseprite` on `PATH` on Iroh (`~/.local/bin/aseprite`, a Steam build; `--version` prints a stray Lua error from a user extension before the version — harmless, noted for the fixture step) | the fixture builder runs here |

## Design

### The reader (`crates/engine_core/src/aseprite/{mod,chunks,composite,clips}.rs`)

Pure: bytes in, `AsepriteSheet { width, height, rgba: Vec<u8>, grid: SheetGrid, clips:
Vec<(String, AnimationClip)> }` out; no GPU, no I/O, about 500 lines with its tests across the
four files, each under the ceiling. The subset of the documented format it reads
(aseprite.org/docs/ase-file-specs): the 128-byte header — magic `0xA5E0`, frame count, canvas
width and height, colour depth (32 RGBA, 16 grayscale, 8 indexed), the flags word (bit 0: layer
opacity is valid), the transparent index; each frame's 16-byte header — magic `0xF1FA`, duration
in ms, the chunk count (the new field, the old when it is `0xFFFF`); and chunks **Layer**
`0x2004` (flags: visible, reference; type: normal, group, tilemap; child level; blend mode;
opacity; name), **Cel** `0x2005` (layer index, x, y, opacity, type: raw, linked, compressed
image, compressed tilemap; for an image its width, height and zlib data), **Tags** `0x2018`
(from, to, direction, repeat, name), **Palette** `0x2019` (applied from its `first_color_index` to its `last_color_index`, never
from zero — round 2, gemini F8; an entry whose flags bit 0 is set carries a name string after
its colour, read and skipped — round 1, gemini F2) and the old
palettes `0x0004`/`0x0011`; every other chunk is skipped by its size. A frame's chunk count is
the new field unless it is zero, then the old one (round 1, codex F3), validated against the
frame's own byte length. Strings are a `u16` length and UTF-8; everything is
little-endian; every read is bounds-checked and a truncated or mis-sized file is
`AsepriteError::Truncated { offset, needed }` — one sentence naming the file and the offset,
never a panic. **Explicit limits before any allocation**, in checked arithmetic (round 1, kimi
F1, gemini F7, codex F1): **the encoded file at most 64 MiB, checked by `vfs::file_size` before
any read** — at load, thumbnail, reload and upload alike (round 2, kimi F1) — the canvas at most
4096 a side, at most 1024 frames, the composed sheet at most the renderer's
`max_texture_dimension_2d` a side (an editor error naming the limit, never a GPU failure; the
exporter's PNG of such a master has the same height and fails the same way today — round 2,
gemini F6), the total decoded bytes at most 64 MiB, a cel at most 4096 a side by its own
dimensions — never bounded by the canvas, since a cropped canvas keeps its cels whole (round 2,
kimi F8, gemini F7); a file past any of them is refused naming the limit, and nothing is sized
from the file's own counts until they pass. A cel's `x` and `y` are `i16`: the signed
intersection with the canvas is what is copied (round 1, gemini F4). A linked
cel must name an image cel in an earlier frame on its layer, as Aseprite writes them; a chain or
a missing target is refused naming the frame (round 1, gemini F8). The zlib data inflates
through `flate2::read::ZlibDecoder` bounded to its cel's `width × height × bytes_per_pixel`.

### Compose

Per frame, the visible, non-reference layers bottom to top in layer-chunk order; a hidden group
hides every layer whose child level puts it inside; each cel is placed at its offset and clipped
to the canvas; a linked cel reads the frame it links to; the blend is **normal** source-over with
the layer's opacity (255 when the header says opacity is not valid) times the cel's; indexed
pixels go through the palette, the transparent index to alpha 0; grayscale `(value, alpha)` to
RGBA. A background layer (flag bit 3) is opaque: the transparent index does not apply on it, as in
Aseprite (round 1, codex F2). **Refused, naming the layer or the frame**: a tilemap layer or cel
("layer `tiles` is a tilemap; export it as a PNG"); a blend mode other than normal on any
layer, a group included — a group has no cel, so a per-cel rule would miss it (round 1, kimi F2,
codex F2); a group with opacity under 255; layer or cel opacity under 255 in indexed mode (Aseprite
composes those by index, which this reader does not reproduce — round 2, kimi F5); a cel with a nonzero z-index; a
palette chunk in any frame but the first. Transparent pixels are written `(0, 0, 0, 0)`. The
sheet is the exporter's layout exactly — rows of **`min(frames, 4)` columns** (a one-frame master
is one cell wide, as every capsule and title in deion_assets is — round 1, gemini F3), the cell
the canvas size, padding cells transparent — so the composed pixels equal `export_sheet.py`'s PNG
wherever alpha is not zero, and `grid` is `SheetGrid::from_cell_size(sheet_width, sheet_height,
canvas_width, canvas_height)`. A master with one frame is a one-cell sheet; a plain `Sprite` given a master
shows its first frame.

### Clips (ruling (b))

The exporter's grammar, with its messages: a tag's name matches `^[a-z][a-z0-9]*(_[a-z0-9]+)*$`;
its direction is forward; its frames share one duration, `fps = 1000 / ms`, a zero duration refused with the sidecar's
finite-positive rule (round 1, kimi F9, gemini F9 — refused, not clamped, so the grammar stays
one); repeat 0 →
`looping: true`, 1 → `false`, any other count refused; untagged frames are cells in no clip; a
tag past the frame count is refused. The filter is `Nearest`. `clips` is in tag order, the order
`into_parts` keeps for a sidecar. What this refuses — reverse, ping-pong, mixed durations — is
one follow-up issue with its lossless expansions written down (reverse = the indices reversed;
ping-pong = `a b c b`; mixed durations = frames repeated at the GCD of the durations, capped), so
the day a Deion master wants one, the engine and the exporter grow the grammar together.

### Parity is the gate

`examples/aseprite_check.rs` (`test = true`, so its own tests run under `cargo test`) takes a
master and its PNG + `.sheet.ron` pair and compares: the composed RGBA against the PNG decoded by
`image` — an alpha-0 pixel compared by alpha alone, since Aseprite's PNG keeps the palette's RGB
under it (the first differing cell and pixel named), the grid and the clips against
`parse_sheet_file(...).into_parts(...)` — `fps` within 1e-3, the exporter's own three-decimal
precision (round 2, gemini F9) — the first differing clip named. The working-set script
`scripts/check-aseprite-parity.sh` (the mirror of `check-sprite-sync.sh`, beside it) runs the
example over every master under `deion_assets/` that has a committed pair and exits non-zero on
the first mismatch; it is the standing gate of every batch here from batch 1 on. The engine's own
fixture is `crates/engine_core/fixtures/aseprite/two_layers.aseprite` — two layers, three frames,
two tags (one looping, one once), indexed mode, built by `fixtures/aseprite/make_fixture.lua`
through `aseprite -b --script` (deion_assets' `tests/fixtures/make_fixture.lua` is the model; the
Lua is committed so the fixture is reproducible), with its PNG + `.sheet.ron` pair exported by the
same `aseprite -b` command the exporter uses, and a `LICENSE` naming all four files, CC0. The
provenance script is generalised from "every audio file" to "every file under
`crates/<crate>/fixtures/`", one rule; `check_guide_tests.sh` and the engine guide's pitfall
table gain the reader's rows. Unit tests build malformed files in memory with a small byte
builder in the test module: a wrong magic, a truncated cel, a chunk size past the frame, a
legacy chunk count, a tag past the last frame, a zero duration, a hidden group with a visible
child, a linked cel and a chained one, a cel at `(-3, -2)`, an indexed pixel at the transparent
index and the same on a background layer, a refused blend mode, a group with opacity, a
z-indexed cel, a palette in frame two, a named palette entry, a partial palette starting at index 16, an
indexed cel at opacity 128, a 64×64 cel at `(-16, -16)` on a 32×32 canvas, a compact file
declaring a huge
canvas and one declaring a huge frame count (each refused before allocating), a one-, three- and
five-frame master's cell positions. Reference fixtures exported by `aseprite -b` for the group,
the z-index and the indexed-background cases pin the refusals to what Aseprite draws.

### The loader seams

`sidecar_path_for` is untouched. `prepare_sheet` branches on the extension: `.png` reads the
sidecar as today; `.aseprite`/`.ase` runs the reader and caches `PreparedSheet { sheet, filter,
pixels: Option<ComposedPixels> }` in the same `SidecarCache` entry, so a scene load reads a master
once however many `SpriteAnimation`s name it. `load_sprite_sheet` and `resolve_texture`'s file
arm load those pixels through a new `AssetManager::load_texture_rgba_at_path(path, filter,
width, height, &rgba)` that records `handle_to_path` and `loaded_by_path` exactly as a file load
does — the `"#rgba"` sentinel stays for procedural textures — so the scene serializer writes the
master's path, the dedupe serves one handle per `(path, filter)`, and the thumbnail shares it.
**A plain `Sprite` given a master shows its first cell** (round 1, kimi F3, gemini F6, codex F4;
round 2, kimi F6, gemini F2): at a viewport drop and a texture-field assignment the first cell's
region is set through the existing undoable `SetComponentCommand` path — `assign_sprite_texture`
and the drop's spawn carry the region with the handle; at a scene load **the loader itself** sets
it, in `engine_core`, never through an editor command (which would dirty a freshly opened scene
and cannot run in a game build). The scene wire stores `tex_region` by value with the full
texture as its default and no presence flag (`scene_data.rs:174-179`), so the rule is by value:
**a full-texture region on a master means the first cell**, documented in `training.md`; a
`Sprite` that has a `SpriteAnimation` is left to it. The asset browser's `Aseprite`
kind draws its tile like an image (the thumbnail is the composed sheet's first cell, a
`tex_region` on the tile's `ui.image`) and drags as a texture with its first cell; External
Tools' Enter and double-click open it in the Art tool. The playground's upload pipeline gains
`UploadKind::Sprite` for the two extensions: destination `sprites/`, the encoded 64 MiB limit, and
a full reader validation before any byte is written, as the song upload validates today (round
2, kimi F7); a successful upload of a master or an image that is already loaded calls the same
`reload_texture` path Rescan uses, since the watcher never runs on wasm (round 2, kimi F3; a
same-name re-import after the master has rendered is the test). The archive already carries a
master as opaque bytes, and `check_wasm.sh` proves the reader builds for wasm32. A scene-round-trip test: a `SpriteAnimation` on a master,
saved, reloaded with the sidecar cache cleared, plays the same clip; a plain-sprite test at each
of the three entries, with undo and save/reload.

### The texture reload (the other half of #180)

**The transaction lives at the host** — External Tools' `hot_reload.rs` in `editor_integration`
— because the asset manager owns the `TextureManager` (`assets.rs:132`) and the `RenderManager`
owns both `SpritePipeline`s (`render_manager.rs:32-39`), and no one existing owner holds both
halves (round 2, kimi F2). Its steps, in order, for one path: **prepare** — the encoded size
checked, the file re-read, decoded (PNG and the rest through `image`, a master through the
reader), and for a sheet the clips and the grid validated, everything that can fail before
anything is touched; **replace** — `TextureManager::replace_rgba(handle, width, height, &rgba)`
for every `(path, filter)` entry (same size → one `queue.write_texture`; a new size → a new
`TextureResource` under the same handle); **evict** — `RenderManager::evict_texture(handle)`
over both pipelines' bind-group caches, so a stale image can never follow a resize (round 1,
kimi F6; a bind group still holding the old view keeps it alive through wgpu's reference
counting, so a missed eviction is a stale image, never a validation error); **commit** — the
prepared-sheet cache entry replaced (kept on failure — round 2, gemini F4, codex F2) and the
world reconciled. Headless, `replace_rgba` validates the byte count and the limit as
`insert_rgba` does and uploads nothing; a handle not live is an error. A decode failure keeps
the old pixels, the old metadata and the old cache entry, and reports one sentence; a
`Truncated` failure says "half-written?", and the finished file's stamp differs by its length
(`ChangeStamp` is the byte length and the modified time), so the next poll retries (round 1,
gemini F5; round 2, gemini F5 rebutted on that).

**Reconciliation**, the commit step's world half (round 1, kimi F7, codex F5, F6; round 2, kimi
F4, gemini F1, F3, codex F1, F3): for every `SpriteAnimation` naming the master, the new grid
and clips; its current clip matched by name — present, the position clamped to the clip's new
length; **absent, the clip keeps its name and stops** (`playing: false`, `finished` and timing
reset, a console line naming the tag), so a renamed tag loses no authored binding and resolves
again, stopped, when it returns — never `None`, never serialized away. Then **the displayed
region is written for every sprite on the master**, independently of playback: an animation
with a resolved frame shows that frame's new cell; one with no resolvable frame shows the first
cell; a plain `Sprite` whose region equals exactly one cell of the old grid is re-mapped to the
same cell index in the new grid, or to the first cell with a console line when that index no
longer exists; an authored crop that is not a cell is left alone. No new state.

In External Tools' table: `Image → ReloadTexture` and `Aseprite → ReloadSheet` both run the
transaction above. For a PNG the loader remembers whether the texture was loaded **as a sheet**
(a sidecar was present at load): a sheet's PNG keeps both pixels and metadata until the PNG and
its sidecar validate as a pair — an absent sidecar is a plain image, a present-but-invalid one
is a failure that changes nothing (round 1, kimi F5; round 2, codex F4; both write orders and a
permanently malformed sidecar are tests); a texture never loaded as a sheet is pixels only. Stop
re-applies the on-disk state as External Tools batch 3 wrote it. The watcher's registered set
gains every `handle_to_path` entry **and every path a texture load failed on, as a tombstone**, a
stamp change clearing the browser's `load_failed` and retrying (round 1, kimi F8); the asset
browser's Rescan, and a successful playground upload, call the transaction for every registered
path whose stamp changed, which closes #180 whole (External Tools batch 4 closes its sound half).
**A limit, said plainly** (round 1, kimi F4): the live-update guarantee covers components in the
world; a `SpriteSheet` a game holds from `load_sprite_sheet` is the game's copy, refreshed when
its `init` runs again (Stop → Play) — the limit the PNG sheet reload already has, written into
`training.md` and pinned by a test.

### The deion_assets half (ruling (a))

`sync_sprites.py` accepts a `.aseprite` source beside the `.png` form: the destination is a bare
`.aseprite` basename, the `ai_` rule holds, the file is read from the pinned sha with `git show`,
no sidecar is derived, and the folder is owned as before; `--check` compares the one file. The
parser's two refusals (`:94-95`, `:102-103`) become "a `.png` or a `.aseprite`, and the destination
keeps the source's extension", with tests for both forms and for a mixed list.
`check-sprite-sync.sh` is unchanged in shape. `DEION_STYLE.md` §6 and §9, `docs/BRANDING.md`
and the sync's header are rewritten to (a): the sync is still the one way art enters a game, and
what it carries is the master or the pair; the PNG and sidecar are never hand-edited and the
master is never hand-copied; the exporter is the repo's gate and the capsules' path. The roadmap
line becomes "tiles and props are authored art, never generated at runtime". **Pong moves first
as the proof**: its `sync.list` names the masters, the pairs leave `assets/sprites/`, its tests
and `check_games.sh` are green over the masters, `check-sprite-sync.sh` and the wasm bundle gate
are green, and the bundle size before and after is quoted in the report — a master keeps its
layers, and if the size bites, that game keeps the pair; the engine supports both, which is the
point of (a).

### Accessibility

Nothing new on screen but a tile kind, which inherits External Tools' tile rules (Enter opens,
the context menu keyboard-reachable, no colour-only state). The reader's errors are one sentence
naming the file, the layer or the tag, in `sheet_file.rs`'s tone, so the console line a person
reads says what to change in Aseprite.

### Patterns

Flyweight (one composed sheet per path, every `SpriteAnimation` a view on it); the type table of
External Tools decides the reload; the parity example is the oracle the exporter already is.
Nothing is a service locator: the asset manager owns the cache, the renderer owns the handle.

## The batches

Each batch is one handoff to the Opus executor from `review/aseprite-native/handoff-N.md`,
reported at `review/aseprite-native/report-N.md`, reviewed in code mode (kimi always; gemini on
the parser, the renderer seam and the sync script; codex on every batch, from stills the report
names where there is a screen) and committed by the planner on Jesse's word.

**Gates, mechanical, every batch:** `cargo test --workspace` (0 failed, 0 ignored), `cargo
clippy --workspace --all-targets` (0 warnings), the comment-tag grep from `CLAUDE.md` printing
nothing, `scripts/check_wasm.sh`, the 600-line ceiling over every touched `.rs`;
`scripts/check_games.sh` when `engine_core` or `renderer` changed; `scripts/check_guide_tests.sh`
when a guide's pitfall table changed; the generalised provenance script when a fixture lands;
`scripts/check-aseprite-parity.sh` from batch 1 on. The planner reruns every gate on the staged
tree before the code round.

**Batch 1 — the reader and the parity gate.** `crates/engine_core/src/aseprite/` (the four
files), `flate2` as a direct dependency of `engine_core`, `examples/aseprite_check.rs`, the Lua
fixture builder and the fixture with its pair and LICENSE, the provenance script generalised,
`scripts/check-aseprite-parity.sh` in the working set (its own repo: a second commit). Pure; no
editor, no GPU. Target shapes: § The reader, § Compose, § Clips, § Parity. Reviewers: kimi,
gemini (a parser), codex (the composite's fidelity to Aseprite). Leaves out: every seam into the
asset manager. Under 900 lines.

**Batch 2 — the loader seams.** `prepare_sheet`'s branch and the pixels on the cache entry,
`load_texture_rgba_at_path`, `resolve_texture`'s arm, `load_sprite_sheet` on a master, the
first-cell rule at the three entries (the loader's by value), the encoded size check, the asset
browser's kind and thumbnail, the playground's `UploadKind::Sprite`, the scene round-trip and
plain-sprite tests, the `hello_world` golden or a
sibling test over the fixture master. Target shapes: § The loader seams.
Reviewers: kimi, codex (one still: a master's tile beside its PNG twin, the same first frame).
Leaves out: any reload.

**Batch 3 — the texture reload.** `TextureManager::replace_rgba`, `RenderManager::evict_texture`
over both pipelines, the host transaction in External Tools' `hot_reload.rs` (prepare, replace,
evict, commit) for the `ReloadTexture` and `ReloadSheet` rows with the as-a-sheet rule, the
reconciliation by clip name and the displayed region for every sprite, the cache entry's
replacement, the registered set with its tombstones, Rescan and the upload calling it, the
held-sheet limit's test. **Needs External Tools batch 3.** Target shapes:
§ The texture reload. Reviewers: kimi, gemini (the renderer seam), codex (two stills: Aseprite
open on a sheet from the tile, and the viewport after a four-to-five-frame save — a plain
sprite, a paused animation and the thumbnail in one frame). Leaves out: the
sync.

**Batch 4 — the sync and the docs.** deion_assets' `sync_sprites.py` and its tests, the rule
rewrites, pong moved; `training.md`'s Sprite Sheet Pattern gains the master paragraph,
`CLAUDE.md`'s SSOT table a row for the reader, the engine guides' pitfall tables their rows,
`docs/EXTERNAL_TOOLS.md`'s Art rows cross-linked (or the doc this effort names, if External Tools
batch 8 has not landed). Two repos, two commits, plus pong's own. Target shapes: § The
deion_assets half. Reviewers: kimi, gemini (the sync script is a harness), codex (the policy and
the moved game's art). Leaves out: every game but pong; the capsules. Then the close-out — the
milestone's issues closed by their commits, the lessons to `log_archive.md`,
`review/aseprite-native/` archived, the scorecard's totals, the follow-up issue for the
expansions — is the planner's.

**Order.** 1 → 2 → 3 → 4; batch 1 goes out only after External Tools batch 3 has landed (ruling
(c)), and interleaves with its batches 4–8 one at a time.

## Verification, end to end

- Every batch: the gates above, run by the planner on the staged tree, never from the report.
- After batch 1: `scripts/check-aseprite-parity.sh` reports every deion_assets master equal to
  its committed pair, or names the first differing cell, clip or pixel; a master with a hidden
  group, a linked cel and a once-tag round-trips the fixture.
- After batch 2: the editor on pong opens a scene whose `SpriteAnimation.sheet` is a master; the
  clips play; save and reload keep the path; the tile shows its first frame; the playground
  imports a zip holding one and plays it.
- After batch 3: Aseprite opened from a tile, a pixel changed and saved → the viewport and the
  thumbnail change within a second with no Stop; a frame added to a tag → the clip plays it at
  once, a plain sprite still shows its cell, a paused animation holds its frame; the active tag
  shortened → the position clamps; deleted → the animation stops on the first cell, keeps its
  clip name and the console names it, and resolves again when the tag returns; a plain sprite on
  a removed cell → the first cell; a duplicate made after the reload carries the new clips; a
  corrupt save keeps the old pixels and the console names the offset; a PNG with a sidecar
  resized → its grid follows; a PNG whose sidecar is mid-write → nothing changes until both
  validate.
- After batch 4: pong's `assets/sprites/` holds masters only; `check-sprite-sync.sh` and the
  wasm bundle gate green; a reader with no context follows `training.md` to load either form.

## Out of scope, said plainly

Reverse, ping-pong and mixed tag durations (filed, lossless, later); slices; tilemap layers;
blend modes; a layer or a group as its own texture; writing `.aseprite`; a Lua script of ours
shipped to Aseprite; moving any game but pong; the site's capsules (PNG stays); an Aseprite
version pin (the exporter prints it and does not pin, the parity gate catches a change);
`.gif` or any other animated raster.

## Carried from External Tools and the Conductor (the lessons that bind every batch here)

The planner's gates run on the staged tree, never from the report; every wait in a test is a
bounded poll or a step count; a draft diff that omits binaries says so in its prompt, and a batch
that lands a binary fixture names its bytes' origin in the report; codex is dispatched after kimi
exits; a still of a tile wants the window where the panel lays out at its own size; the audio
thread never allocates, parses or drops — and neither does the frame: a master is decoded on the
load path and on the reload path, never per frame.
