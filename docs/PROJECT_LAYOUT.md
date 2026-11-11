# Project layout — where a project's files live and what they are called

Every project the engine opens follows one layout under its asset root (`assets/`): a game in
`../games/`, the engine's own `examples/`, the playground's bundled projects in
`crates/playground/assets/projects/`, and the browser's *New* menu, which places each new
file by the same table. `scripts/check_project_layout.sh <project>` enforces it.

## The folders

The first two columns quote `CONVENTIONS` in `crates/editor/src/asset_browser/tree.rs`, the
table the asset browser's router reads — each kind's folder and the extension *New* gives it,
in the same order. The checker compares the two and fails on any difference, so a row
changed on one side alone is caught. The `ai/` and `LICENSE` rows are this page's own
additions, which the router has no row for; the checker leaves them out of that comparison by
name.

| folder | New's extension | holds | names |
|---|---|---|---|
| `scenes/` | `.scene.ron` | `<name>.scene.ron` | `snake_case`; a two-player variant `<name>_2p` |
| `scripts/` | `.rhai` | `<name>.rhai` | the entity or role it drives: `paddle.rhai`, `goal.rhai` |
| `sprites/` | `.png` | `<name>.png` + `<name>.sheet.ron`, always the pair (a `.jpg`, `.jpeg` or `.bmp` image sits here too, with no sidecar) | `DEION_STYLE.md` §6: `snake_case`, a size suffix names the cell or the sheet (`_32x32`, `_480x288`), `ai_` prefix on an AI first cut |
| `sounds/` | `.wav` | recorded one-shots and loops (`.wav`, `.ogg`, `.mp3`, `.flac`) + `LICENSE` | `snake_case`, for the action: `jump.wav`, not `snd_jump.wav` |
| `music/` | `.mid` | `<stem>.mid` + `<stem>.mid.ron` + `LICENSE`; `samples/` per `SONG_FORMAT.md` | `SONG_FORMAT.md` |
| `soundfonts/` | `.sf2` | `<name>.sf2` + `LICENSE` | the game's or the bank's name |
| `fonts/` | `.ttf` | `.ttf`/`.otf` + `LICENSE` where the licence asks | **the face's distributed filename, kept as is** (`BlackSamsGold-ej5e.ttf`) |
| `locales/` | `.ron` | `<code>.ron` | the locale code: `en.ron`, `pirate.ron` |
| `ai/` | — | an agent's first cut, mirroring the folders above (`ai/music/` today) | `ai_` prefix as well, so the paid-tier purge is greppable either way |
| `LICENSE` | — | the provenance of every file in an audio folder, in the Debian copyright format | `LICENSE`, the one name the `snake_case` rule does not reach besides a font's |

## The rules

- **One folder per kind, at the asset root.** Nothing else sits at the root but the folders
  above. A subfolder *within* a kind is the author's (`sprites/props/`); a kind folder inside
  another (`sprites/scenes/`) is not allowed.
- **No kind in another's folder.** A folder holds its own kind and that kind's companions:
  a `.png` lives under `sprites/`, never under `scenes/`.
- **Every `.png` under `sprites/` has its `.sheet.ron`**, and every sidecar its `.png`.
- **Every audio folder has its `LICENSE`** naming each file in it — `sounds/`, `music/`,
  `soundfonts/` and `ai/music/`. `scripts/check_audio_provenance.sh` is that rule; the
  layout checker runs it over the project rather than repeating it.
- **Names are `snake_case`, ASCII, without spaces, with one dot before the extension chain**
  (`level1_2p.scene.ron`, `paddle_16px.sheet.ron`). Folder names follow the same rule.

## The exceptions, stated

Two kinds of file keep a name the rule above would refuse:

- **A font's filename** is the face's distributed one. A face is a vendored artefact, often
  under a licence that names its file, and `locales/*.ron` name these files. The checker
  reports each font it does not judge rather than passing it silently. `font.ttf` is the one
  the engine looks for first.
- **`LICENSE`**, in each audio folder (and in `fonts/` where a licence asks for one).

And the engine writes a few names of its own, which the checker accepts:

| name | written by | where |
|---|---|---|
| `<stem>-<8 hex digits>.<ext>` | a song's Save, when a sample's name is taken in the song's folder by other bytes (`song/io/samples.rs`) | `music/samples/<flat>/` |
| `<flat>` with `__` between its parts | a song's sample folder, its path under its home with each `/` written `__` (`SONG_FORMAT.md`); a recovered score's folder carries its `.recovered-<token>` | `music/samples/` |
| `<stem>.recovered-<token>.mid` or `.midi` (and its `.ron` sidecar) | a recovered score kept beside the song it was recovered from | `music/` |
| `imports/<name>.wav` | the playground's sample import, before a song's Save copies it into that song's folder | `music/imports/` |
| `.keep` | the web export, for a folder with nothing in it | anywhere |

## A game's synced sprites

A game's `sprites/` is filled from `deion_assets` by `sync_sprites.py`, which owns the whole
folder and names each copy as `DEION_STYLE.md` §6 does. Where `sprites/sync.list` exists the
checker takes it as the authority for what is there: every file is the list itself, a `.png`
it names or that `.png`'s sidecar, and every pair it names is present. It does not re-judge
those names.

## Moving a pre-convention project

A project made before this page names its images under `images/`. Move them, and re-point the
scenes:

1. Move `assets/images/` to `assets/sprites/`, and give each `.png` a `.sheet.ron` (a single
   cell the size of the image is a whole sheet).
2. In every scene and script, replace `images/` with `sprites/` in each path that names one.

## What the checker enforces

`scripts/check_project_layout.sh <project>` reads `<project>/assets/` and prints one line per
violation, exiting 1 on any:

- the two tables — `CONVENTIONS` and this page's — agree row for row and in order;
- every entry at the asset root is one of the folders above;
- every file in a kind folder is that kind or its companion, a nested kind folder is refused,
  and every sprite has its sidecar (a synced `sprites/` is checked against its `sync.list`);
- every name follows the rule above, or is a stated exception or one of the engine's names;
- `scripts/check_audio_provenance.sh` passes over the project.

`scripts/check_games.sh` runs it over every game it checks. The examples and each bundled
playground project are checked by hand before a change to them lands:

```sh
scripts/check_project_layout.sh examples
scripts/check_project_layout.sh crates/playground/assets/projects/pong
```
