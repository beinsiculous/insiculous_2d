# External Tools — the plan

**v5 — 2026-10-26, after round 4** (`review/external-tools/rebuttal-4.md`: kimi 8, gemini 8,
codex 5 — twenty-one findings, every one accepted; the planner recommends settling here). What
moved: two rulings for Jesse — **(g) a lock in the crossing's contract**, because check-then-write
is not atomic and the race came back in four rounds, and **(h) `ShellExecuteW` for Windows**,
because three shell-free trampolines failed in review; **a stable `ActionId`** on every trigger, so
any revision's table answers it without strings or kept tables; **the position crosses as the bar
and the fraction of the bar**, clamped inside the new bar, so a meter change and a partial last bar
are defined; a song that ends with a replacement armed takes it at once; `SharedPosition` publishes
under a version so position and revision are one snapshot; the retired conductor goes into a
preallocated slot; preparation runs on a worker; recovery records the journal's target
fingerprints too, so the engine's own partial commit is finished rather than misread, a lone
sidecar journal checks both finals, and conflict files never collide; `Location::Track` and the CLI
carry the track index; the reaper joins the reader; the watcher polls on a worker and debounces
`.mid` with its `.mid.ron`.

**v4 — 2026-10-26, after round 3** (`review/external-tools/rebuttal-3.md`: gemini 9, kimi 9,
codex 6 — twenty-four findings, every one accepted). What moved: **the panel's own Save dispatches
the reload** — `note_written` silences only the watcher, so an in-engine sidecar repair reaches
playback; **the document guards both files by content fingerprint**, keeps the exact bytes it
loaded for a map save's fingerprint, writes a conflict file rather than over another writer, and in
the transition never overwrites a `.mid` whose bytes are not the ones it loaded; **recovery records
both finals** in `.new.prev`, takes a lone sidecar journal, and the prune moves after the finals;
**the position crosses as a musical position** (bar, beat, fraction), never seconds, so a tempo
change keeps the bar; a transport stopped at its end takes the replacement stopped at the new end;
a stale trigger is translated by its action name, not dropped; map-dependent commands carry
musical positions; a `Name` part's identity is its name; deletions are debounced; `song init`
stays to scaffold a sidecar; a looping sample restarts on its new bytes; Windows and WSL open
files through `rundll32.exe url.dll,FileProtocolHandler`, every `cmd /C start` sentence gone; each
Code preset has a project template; an explicit `OsDefault` override is honoured; the console's
collapse key includes the message.

**v3 — 2026-10-26, after round 2** (`review/external-tools/rebuttal-2.md`: kimi 13, gemini 9,
codex 8 — thirty findings, twenty-nine accepted, one rebutted: the engine has no loop start). What
moved: **a map save is sidecar-only** — the document never rewrites the `.mid` for a map edit, and
tracks both files' stamps, so an external sidecar edit reloads or prompts and Save refuses over
either file changed underneath it; **journal recovery decides by fingerprint, not by stamp**, and
keeps both sides when the finals changed outside (`<path>.new.prev`); **a selected track with no
notes is a silent part** — the engine's `validate` refused the contract's own muted track; the
replacement protocol is written out: every command drained while a replacement is armed applies to
the old conductor and the swap fires at the first bar line after the transport's position when it
fires, triggers carry the playback generation, each replacement has a revision with the states
pending → armed → active / superseded → retired, the channel and the position belong to the
source and not to a conductor, the seam re-checks the mapped position allocation-free, the
position crosses as the exact tick, a SoundFont or a sample in the closure dispatches its song's
reload, and the identity guard compares only parts whose selector is unchanged; map edits are
allowed on mixed-channel MIDI; the Windows OS default is `explorer.exe`, not `cmd /C start`; the
launcher reads stderr on its own thread beside the reaper; a legacy `ide_command` gains `{file}`; a
sheet reload is a transaction; a reloaded script's first-tick error names the reset; the repeat
collapse keys on the location; the double-click rule reads "no drag started".

**v2 — 2026-10-26, after round 1** (`review/external-tools/rebuttal-1.md`: kimi 14, gemini 9,
codex 9 — thirty-two findings, every one accepted, gemini's flag-day line as a gate). What moved:
**the open song document follows the score** (an external export reloads a clean document, prompts a
dirty one, and Save refuses over a score that changed underneath it; the engine keeps writing
`midi_fingerprint` for its own crash recovery and reads a mismatch with no journal present as an
external edit; a stale journal is never promoted over a newer `.mid`); **the swap is a prepared
conductor, not a rebuild** (built off the audio thread, handed over as a lifecycle command in the
one ordered channel with its playback generation, armed and taken at the first bar line at or
after — at once for a stationary transport — the old conductor dropped on the game thread, the
host's song, bindings and beat edges moved as one generation; a shorter song wraps when it loops
and is refused when it does not; pending replacements die with `play_song`, `stop_song` and the
session); **the track diff compares each part's resolved track**, and a muted track is exported as
an empty named track so indices hold; **the template grammar is group-first**; the watcher keeps
tombstones, takes the editor's own writes as its baseline, watches the open document's song and
the session song whether or not anything plays, and lists `examples/`, `tests/` and `build.rs`; a
sheet reloaded in Play is re-applied on Stop; the asset browser learns `.midi` and the DAW project
kinds; code-resolved sounds and cached failures join the sample reload; a child's stderr is
drained; a program given as a path is a file first; console repeats collapse; **batch 7 is gated
on MIDI Evil's export round-tripping a shipped song**, the authoring verbs stay until then, and the
browser's reduction is said plainly.

**v1 — 2026-10-26, drafted.** Written by the planner (Claude Fable) from the design note another
Fable session wrote the same day (`docs/sprints/external-tools.md`, 683 lines, approved by Jesse
"looks great, go ahead"; its eighteen issues are filed as `beinsiculous/insiculous_2d` #203–#220
under the milestone "Sprint: External Tools"). That note was a sprint-planning artifact that
reviewed itself; this plan is the house shape — the one the Conductor effort ran — so the other
roles attack it before anything is handed off. Jesse's kickoff rulings of 2026-10-26 are in § The
rulings; the biggest is the pivot the note did not know: **the composer is not retired, composition
moves to MIDI Evil, and the engine keeps the conductor's half.** Review round 1 (kimi, gemini,
codex, one snapshot) dispatched the same day; this file is revised after each rebuttal and the plan
is settled when Jesse says so. The design note is deleted when this plan lands as settled; nothing
in it is lost — § Design carries it.

Effort directory: `coordination/external-tools/` (this plan, `reviewer-comparison.md`). Review
transients: `review/external-tools/` (the plan snapshot, `review-N-<reviewer>.md`, `rebuttal-N.md`,
`handoff-N.md`, `report-N.md`, `draft-N.diff`). Roster: Claude Fable plans, adjudicates with
Jesse, applies accepted fixes and commits; a Claude Opus session (Jesse's other window) executes
each handoff; kimi reviews every plan round and every diff; gemini (the Antigravity CLI) reviews
every harness, fixture and seam — the launcher, the watcher, the bar-seam swap, the track diff;
Astra (codex) reviews every plan round and every batch with a screen — the context menu, the
console, the settings panel, the Conductor panel. A vendor never reviews its own vendor's work.

Sprint: milestone "Sprint: External Tools" on `beinsiculous/insiculous_2d`, #203–#220 (E1–E18 in
the note's lettering, kept here so the issues read against the plan), plus one issue this plan
files at settling: the rename of the composer to the Conductor panel. The milestone's description
and #217/#218 are reworded when the plan settles (§ The rulings, (c)); nothing on the board moves
before Jesse has ruled.

## Context

**Goal.** From the editor, open a project file in the right external app, landing on the right
place — a line for code, a track for MIDI — and hot-reload when the file changes on disk. One
mechanism serves code (Rust, Rhai, RON) and music (MIDI, samples, MIDI Evil projects). Scripts and
music become open, editable and hot-swappable with an external editor; the engine stops being
where notes are written.

**The pivot.** The note's decision A read the brief as "the in-engine composer retires in favour
of the DAW" and gated its removal on a roadmap reversal. Jesse, 2026-10-26: "I'm going to be
fusing the work we've been doing in the composer with this other project … separate out music
composition from the engine … retired is a bit dramatic, as it'll still be used, just more like a
conductor and less like a composer." The other project is **MIDI Evil** (`../midi-evil`, a Rust
music workstation; Dr. Maxwell is its mascot): the music maker. The engine's side "is just telling
the engine when to play what sound (and potentially at what volume) at what note" — the parts,
instruments and bindings the `.mid.ron` sidecar already holds, the audition, the Save. So the
composer panel becomes **the Conductor panel**, the score's authoring (the piano roll, the note
list, note edits) leaves, and the September ruling that the in-engine tool sets the engine apart
stands for the half that stays. The seam is the one the sidecar schema already draws between the
score and the map.

**What MIDI Evil is today** (read 2026-10-26, `../midi-evil/README.md`): crates `evil-project`,
`evil-render`, `evil-synth`, `evil-performance`, `midi-evil` (the desktop app: track panel,
patch/gain/pan/mute, a full-MIDI piano roll, a note inspector, Preview and Render WAV), and a
`nanostudio` CLI with NanoStudio compatibility as a validation oracle. Its native document is
`.evil`, versioned JSON: song name, tempo, meter, timeline notes, per-track patch reference, gain,
pan, mute, master settings; 128 ticks a quarter by default; patches are its own synthesis, not a
sample library. It **imports** a Standard MIDI File into `.evil` (`project import-midi`), and
**exports** Standard MIDI from a NanoStudio project (`midi export MySong.prj MySong.mid`: format 1,
tempo, a 4/4-based meter, track names, 128 ticks a quarter). An `.evil` → `.mid` export is not in
the README; it is the first thing the crossing needs (§ MIDI Evil's half). MIDI Evil is in its
infancy and will build up as fast as agents can (Jesse), so no engine batch but the last waits on
it: the Custom field carries the crossing until the flags and the export exist. The note the
planner left its agents is `../midi-evil/docs/insiculous-crossing.md`.

## The rulings (Jesse, 2026-10-26, at kickoff)

- **(a) The crossing is `.mid`, a standard file.** The engine reads what it reads today — a
  Standard MIDI File and its `.mid.ron` sidecar — and MIDI Evil exports one. What `.evil` carries
  that `.mid` lacks (per-track gain, pan, mute, the patch reference, 128-tick timing) is written
  on export as standard events the engine's background already folds: channel volume (CC 7), pan
  (CC 10), a program change at tick 0, **a muted track as a named track with no notes** (never
  left out: a sidecar may select by index, and a missing track shifts every index after it —
  round 1), and the time signature as written (not 4/4-based — a song without one is 4/4 and the
  pulse's bars land on every other bar line, the Conductor's lesson), at a ticks-per-quarter the
  export lets the author set (480, the six games' value). Nothing of `.evil` the engine would want
  is lost that way, and the engine's loader, tempo map and selectors stay untouched. The patch
  itself cannot cross — MIDI Evil synthesizes, the engine plays SoundFont presets — and the
  sidecar's `instruments` is where the engine's voice is chosen, as today.
- **(b) Composition leaves the engine for MIDI Evil; the engine keeps the conductor's half.**
  Approved by Jesse 2026-10-26 ("that all looks good to me") and written into `PROJECT_ROADMAP.md`
  under the phase table the same day (00ce4cc):

  > **Oct 26 2026 (Jesse): composition leaves the engine for MIDI Evil.** The music maker is
  > `../midi-evil` (Dr. Maxwell's workstation); the engine keeps the conductor's half. The composer
  > panel becomes the **Conductor panel**: a `.mid` comes in, and the panel tells the engine when to
  > play what sound, at what note and at what volume — parts, instruments, bindings, audition, Save
  > of the sidecar and its LICENSE stanza. The note editing (the piano roll, the note list, `song
  > new` and the `AddNote` family) is MIDI Evil's. The Sep 25 "K8 is not a cut candidate" ruling
  > stands for the half that stays; External Tools #217 is the re-scope, and the rename follows it.

- **(c) The panel is renamed from the composer to the Conductor panel**, in the UI, the docs and
  the editor's module (`crates/editor/src/composer/` → `conductor/`), after the re-scope trims it.
  The runtime is already `music_midi::Conductor`; the panel is its desk, and the two guides say
  which is which. The rename is its own issue, filed at settling; #217 is reworded from "retire"
  to the re-scope (§ The Conductor panel) and #218 from "trim or remove the song document" to
  "trim the song API to the map verbs"; the milestone's description drops "retires in favour of
  the DAW".
- **(d) The note's decisions B to E stand**: the settings ride `editor_prefs.json` as an
  `external_tools` field (one persistence path, JSON, `#[serde(default)]`); Helix ships as a
  preset with the "terminal editor" note and nothing guesses a terminal; `{track}` expands to the
  track's name and a nameless `Index` track opens unfocused with a notice. Decision D's unknown
  extension is known now: the `DawProject` type's default list is `.evil` (with `.prj` and `.nsp`,
  the NanoStudio files MIDI Evil also opens), editable.
- **(e) Open for Jesse, named not ruled: volume.** "At what volume" is two things: the note's
  velocity, which the `.mid` carries and the conductor already plays, and a per-binding level the
  sidecar does not have. If the second is wanted, it is one optional field `gain: f32` on a
  binding (default 1.0, refused outside 0–2 by `validate`), applied by the trigger engine as a
  velocity scale, with a sidebar row in the Conductor panel — a Small addition to batch 7, not to
  the audio seam. The planner recommends waiting for a game to want it; the field is cheap to add
  later and nothing shipped needs it.
- **(g) A lock in the crossing's contract — proposed, Jesse's call** (round 4). Check-then-write
  is not atomic on any filesystem, and four rounds found the race between the engine's fingerprint
  check and its write. `<file>.mid.lock`, created `O_EXCL` with the writer's name, pid and stamp
  inside, stale after 30 s: the engine takes it around every write of a `.mid` or `.mid.ron` (the
  finals, a journal's promotion, a conflict file), MIDI Evil around its export (its contract).
  Inside the lock the comparison is exact; a lock held by the other side makes the engine wait up
  to two seconds, then refuse with "MIDI Evil is writing the score; try again". A writer that does
  not take the lock (a hand edit in a text editor) is outside the guarantee, which the docs say;
  the conflict file still catches it after the fact.
- **(h) Windows opens a file through `ShellExecuteW` — proposed, Jesse's call** (round 4). The
  OS-default launch on Windows has failed three ways in review — `cmd /C start` re-splits the
  path, `explorer.exe` opens a folder and exits 1, `rundll32 url.dll,FileProtocolHandler`
  mishandles commas, quotes and UNC paths — so the correct API is named: `ShellExecuteW` through
  the `windows-sys` crate behind `cfg(windows)`, the sprint's one new dependency, taken because the
  simple things failed first. The alternative, if no dependency is wanted, is `cmd.exe /D /C start
  "" "{file}"` with the path double-quoted, verified on the Windows machine in batch 8. WSL uses
  the same mechanism on the translated path.
- **(f) The fingerprint is the engine's, not MIDI Evil's** (round 1, kimi F2/F3, codex F1/F8).
  The engine's Save keeps writing `midi_fingerprint`: it is what the `.new` save journal pairs a
  recovered MIDI with its sidecar by (`song/io/open.rs:161-173`). MIDI Evil never touches the
  sidecar, so after an export the fingerprint no longer matches the bytes — and that is read as
  "the score changed outside", not as a refusal: § The open song document. Recovery of an
  interrupted save decides by the fingerprints of both finals, never by stamps (rounds 2 and 3).

## What the tree holds (verified 2026-10-26 by the note's author; the files re-checked by the planner)

| concern | what is there | verdict |
|---|---|---|
| Open in IDE | `editor_integration/src/editor_game/open_source.rs`: `ide_command` (a string in `editor_prefs.json`, `editor_preferences.rs:42`) split on whitespace, `{file}` appended, spawned with inherited stdio, the child reaped on a thread; the Scripts **Open Source** button is its one caller | **Replace** with the launcher; the button keeps working through it |
| Preferences | `editor/src/editor_preferences.rs`: `EditorPreferences`, JSON through `save_store` (native file / wasm localStorage), settle-rule autosave, `#[serde(default)]` | **Extend** with `external_tools` (ruling (d)) |
| Asset browser | `editor/src/asset_browser.rs` (`AssetKind`: Image, Scene, Script `.rhai`/`.rs`, Sound, Song `mid` only, SoundFont; `kind_for_extension` at :98), `panel_renderer/asset_browser/tile.rs`; tiles select on press and arm a drag; no double-click, no context menu (#137 wants one in the hierarchy) | **Extend**: `.midi`, a `DawProject` kind, double-click, a first context menu |
| Console | `PanelId::CONSOLE` reserved in `dock/mod.rs`, "no panel implementation yet"; the View item disabled (`menu/mod.rs`); script errors go to the status bar through `script_status.rs` | **Build**, scope held to a list with clickable locations |
| Scripts | `ScriptRunner` compiles a `.rhai` on first use per `(entity, script)`, caches by `source_path`, quarantines failures; `reset` rebuilds at session start (`engine_core/src/scripting/runner.rs`) | **Extend** with `reload_source(path)` |
| Scene RON | `scene_io.rs` dry-runs into a scratch world; save and open refused mid-session; `ConfirmDialog` for unsaved work (`scene_confirm.rs`); the `.sheet.ron` sidecar is SSOT and `clear_sidecar_cache()` exists; Stop restores the pre-Play world snapshot | **Extend**: reload from disk through the same guards; a sheet reloaded in Play re-applied on Stop |
| File watching | None. `common::vfs::change_stamp` (length + mtime, `None` on wasm) exists; the audio dependency cache keys freshness on it (`audio/src/manager/song/load.rs`) | **Build** a polling watcher over it, no dependency |
| Songs | `AudioManager::play_song(path)` reads `.mid` + `<path>.ron`, prepares the whole closure through the stamp-fresh cache, builds a conductor **off the audio thread** and a `ConductorSource` for it (`song.rs:284`; one conductor a song), then swaps the source; `SongPlayback` (`:135-148`) holds the `Arc<Song>`, the binding table, the handle, the requested transport, the pending lifecycle queue and the `BeatEdges`; `Conductor::load` allocates (slots, two background synthesizers, the trigger engine) | **Extend**: a prepared replacement conductor swapped at the bar, the host's state moved as one generation |
| The sidecar | `music_midi/src/song_file.rs`, `docs/SONG_FORMAT.md`: `parts` (name → `Name(..)` or `Index(n)`), `instruments`, `bindings`, provenance, `midi_fingerprint` (optional on read); `validate` refuses a part whose track has no notes (`EmptyTriggerPart`, `song_file/validate.rs:96-103`) | **One change**: a selected track with no notes becomes a silent part (ruling (a)'s muted track); otherwise the manifest the track diff validates against |
| The song document | `editor/src/song/`: `SongDocument` with its own `SongData`; `save` (`io.rs:310`) writes the `.mid` and the sidecar from memory with the fingerprint (`:281`); `open` (`io/open.rs:89`) promotes a `.new` journal pair that matches itself (`recover_pair`, `:161-173`, without looking at the finals) and refuses finals whose fingerprint differs (`MismatchedFinals`, `:130-133`); `apply` refuses every `SongEdit` under `EditingPolicy::Refused` (`song/mod.rs:172-179`, mixed-channel tracks) | **Extend**: the document follows the score, a map save is sidecar-only, recovery by fingerprint, map edits on any playable MIDI (§ The open song document) |
| Recorded sounds | `AssetManager::resolve_sound` caches handles by path and remembers failures until the base path changes (`sound_library.rs:21-26`); never invalidates (#180) | **Fix** the sound half as the sample reload, failures included |
| Which song is the project's | `init_music.rs` captures the song the game's `init` asked for; the composer scans `music/` and `ai/music/` for its Songs list | **Keep** both; the scan is salvaged into `external_tools/songs.rs` |
| The build | Decision of record (`docs/EDITOR_COMMAND_API.md`, "the editor does not own the build"): any rebuild is "spawn cargo, relaunch self"; no build-system work | **Notice only** for `.rs` changes |
| The composer | 8,322 lines incl. tests across three layers: the GUI (`editor/src/composer/`, 21 files: state, layout, keys, piano roll, note list, sidebar, sound_fonts, transport; `editor_integration`'s `composer_keys.rs`, `composer_drag.rs`, `panel_renderer/composer.rs`), the song document (above), the command API (`command_api/song.rs`: `song new\|edit\|save\|open\|close\|play\|stop\|seek\|trigger`, `song`, `song notes`) | **Re-scope** into the Conductor panel (§ The Conductor panel): the map stays, the score's authoring goes — once the round trip exists |
| `song_preview.rs`, `init_music.rs` | the preview transport and the session's music; the Conductor effort's close-out landed 2026-10-26 (8061b1c), so this effort may edit them | **Keep**; the session-music half is how Play hears the game's song |

## Design

### One table, one mechanism

```
FileType  ──►  ToolCategory  ──►  ToolChoice (per category, overridable per type)  ──►  Launch
   │                 │
   └── ReloadPolicy  └── Code | Music | Sample     (the category decides the presets and the env fallback)
```

`FileType` (`crates/editor/src/external_tools/file_type.rs`, the one place the mapping lives):

| type | extensions | category | `{project}` | reload policy |
|---|---|---|---|---|
| `RustSource` | `.rs` | Code | Cargo workspace root | `RebuildNotice` |
| `RhaiScript` | `.rhai` | Code | Cargo workspace root | `RecompileScript` |
| `Ron` | `.ron` (scene), `.sheet.ron`, `.mid.ron` | Code | Cargo workspace root | `ReloadDocument` (scene) / `ReloadSheet` / `ReloadSong` (by sidecar suffix) |
| `Midi` | `.mid`, `.midi` | Music | asset base | `ReloadSong` |
| `AudioSample` | `.wav`, `.ogg`, `.flac`, `.mp3` | Sample | asset base | `ReloadSample` |
| `DawProject` | `.evil`, `.prj`, `.nsp` by default, editable (ruling (d)) | Music | asset base | `None` |
| `SoundFont` | `.sf2` | Music | asset base | `ReloadSong` (for every song whose closure names it — round 2, codex F7) |

The reload policy is data in this table, not a setting: users pick tools; the engine knows what a
file means. The asset browser's `AssetKind` follows the table: `Song` gains `midi`, and a
`DawProject` kind over the config's extension list draws its own tile (round 1, kimi F11), so every
advertised extension reaches a tile and the context menu; a launch test per extension.

### Types

```rust
// crates/editor/src/external_tools/
pub enum ToolCategory { Code, Music, Sample }
pub enum ToolChoice { Preset(PresetId), Custom(CommandTemplate), OsDefault }
pub struct ExternalToolsConfig {            // on EditorPreferences, #[serde(default)]
    pub code: ToolChoice,                    // default: OsDefault (so $VISUAL/$EDITOR is tried first)
    pub music: ToolChoice,                   // default: OsDefault
    pub sample: ToolChoice,                  // default: OsDefault
    pub overrides: BTreeMap<FileType, ToolChoice>,
    pub daw_project_extensions: Vec<String>, // default [".evil", ".prj", ".nsp"]
}
pub struct CommandTemplate(String);          // "rustrover [--line {line}] [--column {column}] {file}"
pub struct Launch { pub program: String, pub args: Vec<String>, pub preset: Option<PresetId> }

pub struct OpenRequest { pub path: PathBuf, pub location: Location }     // replaces pending_open_source
pub enum Location { None, Project, Line { line: usize, column: Option<usize> }, Track { name: Option<String>, index: usize } }

pub enum ReloadPolicy { None, RecompileScript, ReloadDocument, ReloadSheet, ReloadSong, ReloadSample, RebuildNotice }
```

Resolution (`config.rs`, pure, tested), in order: `overrides[file_type]` **if present** — an
absent entry inherits the category, a present one is honoured whatever it says, an explicit
`OsDefault` included (round 3, codex F6); the category's choice if not `OsDefault`; **Code only**: `$VISUAL`, then `$EDITOR`,
each tokenized as a template with `{file}` appended (no line — these editors are unknown; a
terminal editor fails at spawn and the status bar says so); the OS default: Linux `xdg-open
{file}`, macOS `open {file}`, Windows the shell association through `ShellExecuteW` (ruling (h); no
trampoline process at all, so nothing to reap or report).

### Command templates (`template.rs`, pure, tested)

The grammar is **group-first** (round 1, gemini F6): a template is a sequence of spans, each a
quoted or bare word, a placeholder, or an optional group `[ … ]` holding spans of its own, nesting
allowed. Expansion: a placeholder resolves or does not; a group whose placeholders all resolve
expands to its own spans, one with any unresolved placeholder expands to nothing. Then the
expanded text is split into argv **per span**: a bare group's spans become their own tokens
(`[--line {line}]` → `--line`, `42`), and a group written against a word without whitespace
(`{file}[:{line}[:{column}]]`) concatenates into that word's token (`main.rs:42:10`). Quoting
(double and single, backslash escapes) survives a path with spaces inside a word; a placeholder's
value is substituted whole and never re-split, so `{file}` never needs quoting. Tests: a path with
spaces survives; RustRover's template with no line yields `[rustrover, file]`; `{file}[:{line}]`
with a line yields `file:42`; `[--line {line}]` with a line yields two tokens; a quoted program
path tokenizes as one program. An unknown `{name}` is an error at config time (the settings panel
shows it; the launcher refuses). `{file}` is always absolute (`resolve_asset_path`); `{project}` per
the table. About 90 lines, no dependency.

### Presets — verified 2026-10-26 against the vendors' pages

| preset | binary | template | source |
|---|---|---|---|
| RustRover | `rustrover` (the Toolbox script; the page says no launcher is installed by default) | `rustrover [--line {line}] [--column {column}] {file}` | jetbrains.com/help/rust/opening-files-from-command-line.html, verbatim `rustrover [--line <number>] [--column <number>] <path>`; project-plus-file in one call is not stated, so "Open project" is a second launch `rustrover {project}` |
| VS Code | `code` (macOS needs the Shell Command install) | `code {project} --goto {file}[:{line}[:{column}]]` | code.visualstudio.com/docs/configure/command-line — `--goto` "file:line{:character}"; folder-plus-`--goto` in one call is two documented arguments, flagged for the manual test |
| VSCodium | `codium` | as VS Code | github.com/VSCodium/vscodium/blob/master/docs/usage.md; the flags being VS Code's is not stated there — manual test |
| Zed | `zed` (Linux "commonly `zed` or `zeditor`") | `zed {project} {file}[:{line}[:{column}]]` | zed.dev/docs/reference/cli.md, verbatim `zed myfile.txt:42:10` |
| Helix | `hx` | `hx [-w {project}] {file}[:{line}[:{column}]]` | helix-term's help text `file[:row[:col]]`, `-w, --working-dir`; a terminal editor, ruling (d) |
| **MIDI Evil** | `midi-evil` (the desktop crate's binary; `cargo run -p midi-evil` today) | `midi-evil [--project {project}] [--track {track}] {file}` | § MIDI Evil's half, the contract this plan defines; until MIDI Evil has the flags, the Custom field with `{file}` alone works |
| Audacity | `audacity` (Flatpak `org.audacityteam.Audacity`) | `audacity {file}` | `AudacityApp::ParseCommandLine` (3.7.8): a file-name parameter, no time-position option; re-verify at Audacity 4 |
| Custom command | any | the user's template | — |

The preset table is data (`presets.rs`), each row's source URL in a doc comment, so the next
verification is a diff against the page. **Each Code preset carries a second, `project`
template** for Edit → Open project (round 3, kimi F8): RustRover `rustrover {project}`, VS Code
`code {project}`, VSCodium `codium {project}`, Zed `zed {project}`, Helix `hx -w {project}`; a
Custom tool's project template is a second field, empty meaning the action is disabled; Music and
Sample tools have none. A test per preset.

### The launcher (`editor_integration/src/editor_game/external_tools.rs`, replaces `open_source.rs`)

Per frame, drain `EditorContext.open_requests: Vec<OpenRequest>` (a Vec: a context menu and a
console click can land in one frame). For each: resolve `FileType` → `Launch`; find the program —
**a program containing a path separator is checked as a file first** (round 1, gemini F8), else
looked up on `PATH` yourself (iterate `PATH`, honour `PATHEXT` on Windows) so a missing binary
reads `rustrover: not found on PATH (searched 11 directories) — set the tool in Edit → External
Tools…` rather than the OS's `No such file`; spawn with `std::process::Command` (argv, never a
shell), **stdout to null and stderr piped**; **two threads per launch** (round 1, gemini F7;
round 2, gemini F6): one reads stderr into a bounded buffer (the last 4 KiB) until EOF, so a
long-lived GUI child never blocks on a full pipe and a descendant holding the pipe blocks only
that reader; the other waits the child, as today's reaper does (`open_source.rs:43`), **joins the reader
(bounded at 200 ms)** and then reports every non-zero exit whenever it comes, the first 500 ms
choosing the wording "exited at once — a terminal editor needs a terminal; set a tool in Edit →
External Tools…" with the buffer's first line (round 4, gemini F6). A legacy `ide_command` migrates into
`code: Custom(..)` with ` {file}` appended when it has no file placeholder, so `code --wait` keeps
opening the file (round 2, kimi F12). A
status bar and console line for success and failure. `cfg(target_arch = "wasm32")` keeps today's
hint. A spawn never blocks the frame and never panics.

Cargo workspace root (`workspace_root(asset_base)`): walk up from the asset base; the nearest
`Cargo.toml` with `[workspace]` wins, else the nearest `Cargo.toml`, else the asset base's parent.
For `games/pong` that is `games/pong/`; for the engine's examples the engine root; for a data
project with no manifest, the project directory. Computed at init, recomputed on Open.

### Entry points

| entry | where | request |
|---|---|---|
| Double-click a tile in the asset browser | `panel_renderer/asset_browser/tile.rs` — a second press within 400 ms and 4 px of the first, on the same tile, with no drag **started** between them (the tile arms a drag on every press, `tile.rs:14`; a drag that passed the threshold suppresses the click, and the double-click with it); the tile's own state remembers the last press; the `ui` crate gains nothing | `OpenRequest { path, Location::None }`; a `.mid` tile → `Location::Track` of the first part's track when the sidecar has one; a `DawProject` tile → the Music tool |
| Double-click a Scripts row (inspector, hierarchy pseudo-row) | `script_editor.rs`, `hierarchy/` | the script's `source_path`, `Location::None` |
| Open Source button (exists) | `script_editor.rs` | as above, through the new request type |
| Context menu: Open in external tool / Open containing folder / Edit in MIDI Evil | a first `ContextMenu` widget on `UiLayer::Floating` (the menu bar's dropdown geometry reused), right-press on a tile or a Scripts row; the menu key and Shift+F10 open it from the keyboard | per item |
| Click a `file:line` in the console | `panel_renderer/console.rs` — an entry with a `Location` draws underlined; Enter on the focused entry does the same; the status bar's current error, when it carries a location, opens on click too | `Location::Line` |
| Edit in MIDI Evil on an entity's sound settings | `component_editors/sound_emitter.rs`: a `SoundEmitter` whose sound is `Binding(action)` gets the button; the song (below) → `bindings[action].part` → `parts[part].track` → `Location::Track { name, index }` — the track's name and its index in the file, so the second of two `drums` is named exactly (round 4, kimi F4) | `OpenRequest { song.mid, Location::Track { name, index } }` |
| Edit → Open project in code tool | menu action | `OpenRequest { project, Location::Project }`, expanded through the preset's project template, never the file template |

**Which song is "the" song**, in order: the song a Play session is playing or the game's `init`
asked for (`InitMusic`, by path); else the song open in the Conductor panel; else the one `.mid`
under the asset base's `music/` or `ai/music/` (`scan_songs`, salvaged from the composer); several
→ the sound settings show a chooser row; none → "no song in this project".

**The console** (`editor/src/console.rs`, `ConsoleLog`: a bounded `Vec` of 500 entries with a
`Location` and a severity each; `panel_renderer/console.rs`; the dock's reserved panel, hidden by
default, bottom beside Assets; the View item enabled): every script error, launch result and
reload notice lands there; **an entry collapses into the last with a repeat count** when it shares its location, severity
and text with runs of digits normalized (round 1, gemini F9; round 2, gemini F8; round 3, kimi F9),
so an error every frame is one line whatever number it carries, and a different message at the
same location starts a new entry; the shared `ScrollState`, a Clear
button, the "Error: " prefix and the status bar's amber (the a11y rule of #195).

### File watching and hot reload

`FileWatch` (`editor/src/external_tools/watch.rs`, pure over `common::vfs::change_stamp`) keeps
**two things apart** (round 1, kimi F8, codex F4): the *registered* set — what the editor wants
watched, rebuilt only when the owners change — and the *observed* stamps. `poll()` returns the
paths whose stamp changed and has then stayed unchanged for one further poll (debounce — an app
may write in steps, and a half-written `.mid` must not be read). A path that is gone stays
registered as a **tombstone**: reported as deleted only when absent across two polls, and its
reappearance is a change — an unlink-then-create save that returns inside the window is one
change, never a deletion (round 3, gemini F7). **The editor's own
writes are the baseline**: every internal save — scene, sheet, song, preferences — calls
`note_written(path)` after the write, which takes the new stamp as the path's last, so Ctrl+S never
reloads the scene it just wrote (round 1, kimi F7, gemini F4); a test proves an internal save
yields no watcher event. **`note_written` silences only the watcher**: an internal save of a file
in the session song's closure (the sidecar from the Conductor panel, a `song save`) dispatches the
song's reload itself after a successful commit, through the same prepare → diff → replace chain,
so an in-engine repair of the map reaches playback (round 3, gemini F1, codex F2). Polled every 500 ms on native **by a worker thread** — its stats and its listings never on the
frame (round 4, gemini F7) — the frame draining its events in `finish_frame`; never on wasm. **A
`.mid` and its `.mid.ron` are companions in the debounce**: a change to either waits for both to
settle before one reload is dispatched (round 4, gemini F8). The registered set: the open scene's path and
every `source_path` on every `Scripts` component (after a scene load and after each history step
that touches `Scripts`); **the open song document's `.mid` and sidecar and the session song's
closure whether or not anything plays** (round 1, codex F5) — the `.mid`, the `.mid.ron`, every
sample and SoundFont they name (the closure the audio cache computed); every `SoundEmitter(Path)`
sound **and every path the sound library has loaded or failed** (round 1, kimi F12); **every path a
failed song preparation attempted**, a missing one as a tombstone, so creating the sample a
hand-edited sidecar named retries the preparation (round 2, kimi F9); every `.sheet.ron` the
scene's sprites reference; and — for `RebuildNotice` only — the `.rs` files under
the workspace root's `src/`, `crates/*/src/`, `examples/`, `tests/` and every crate-root `build.rs`
(listed with `vfs::list_files`, re-listed every 30th poll; manifests stay outside the notice and
the doc says so — round 1, kimi F14). The mtime-granularity pitfall (same length within the
filesystem's granularity keeps its stamp) is accepted and documented.

Dispatch (`editor_integration/src/editor_game/hot_reload.rs`), by the table's policy:

| policy | editing | Play / Paused |
|---|---|---|
| `RecompileScript` | a console line (the runner compiles at session start) | `ctx.scripts.reload_source(path)`: recompile; on success swap the AST for every instance of that path, clear its `failed` and quarantine entries, keep blackboards and params; on a syntax error keep the old AST running and put the error, with its line, in the console as a link; a runtime error on the first tick after a reload (the state's shape changed) quarantines as any runtime error does and the console says "reloaded script failed on its first frame — Stop and Play to reset its state", the blackboard kept on purpose (round 2, gemini F7) |
| `ReloadDocument` (scene) | history clean → `load_scene` from disk (the dry-run guard keeps a bad file from the live world); dirty → `ConfirmDialog` "scene changed on disk: Reload (discard my edits) / Keep mine" | deferred: a notice now, the same rule at Stop |
| `ReloadSheet` | a transaction: the sidecar parsed and the texture resolved **before** any live state changes, a failure keeping the old sheet and reporting (round 2, kimi F13); then `clear_sidecar_cache()` + the sheet's texture and clips for every `SpriteAnimation` naming it; a clip index past the new frame count clamps and warns | same; **and Stop re-applies the on-disk sheet state to the restored world** (the sidecar is SSOT; the snapshot restore would otherwise bring the old clips back with the stamp already consumed — round 1, kimi F10), guard-tested |
| `RebuildNotice` | "Rust source changed (N files) — rebuild and relaunch to see it", sticky until relaunch; a console line per file; no `cargo` is spawned | same |
| `ReloadSong` (the `.mid`, the `.mid.ron`, a SoundFont or a sample in a song's closure — any dispatches the song that owns it) | the open document follows the score (below) and the track diff runs against the last loaded song, so a rename is caught before Play | the open document follows the score; the bar-seam swap below |
| `ReloadSample` | a recorded sound: the `AudioManager` reloads the bytes behind the same `SoundHandle`, so every emitter and `resolve_sound` entry stays valid (the sound half of #180); **a path the library had marked failed is retried, and a success clears the remembered failure and serves the handle** (round 1, kimi F12); a failed reload keeps the old bytes and reports; a song sample is in the song's closure → the swap | same; a one-shot already playing finishes on the old bytes, a looping instance restarts on the new at its next loop boundary with the engine's fade (round 3, gemini F9) |

#### The open song document follows the score

The Conductor panel's `SongDocument` holds the score it opened; MIDI Evil owns the score from now
on, so the document **never writes it from memory over a newer file** (rounds 1 to 3):

- **A map save is sidecar-only.** Save writes the `.mid.ron` (and the LICENSE stanza) and leaves
  the `.mid` untouched unless the document's score was changed in the engine — which, after batch
  7, nothing can do, and before it only a score-authoring edit does. **The document keeps the
  exact bytes it loaded** (`source_bytes`): a map save fingerprints those for `midi_fingerprint`,
  never `write_smf()`'s re-encoding, which can differ from the file's (`io.rs:277-283`; round 3,
  kimi F5), with a test on a MIDI whose parse and export differ.
- **In the transition, a score-authoring save never overwrites a `.mid` whose bytes are not the
  ones this document loaded or last wrote** (round 3, kimi F1): the fingerprint is compared
  immediately before the finals' write; on a difference the journal's score is kept as
  `<stem>.recovered.mid` with a notice and the on-disk score is untouched. The window is the write
  itself; after batch 7 the case does not exist.
- **The document guards both files by content fingerprint**, not by stamp (round 3, kimi F4): it
  remembers the `.mid`'s and the `.mid.ron`'s fingerprints from its last read, compared
  immediately before each write. An external change of either: a clean document reloads that file
  — the score with the map kept, or the map with the score kept — and the status bar says it was
  edited outside; a dirty one prompts through the existing `ConfirmDialog` machine ("the score
  changed on disk: Reload my map onto it / Keep mine", or "the map changed on disk: Reload / Keep
  mine"). **Every write of either file is taken under the crossing's lock** (ruling (g)), so the comparison
immediately before the write is exact; a conflict found there (a same-length edit inside the mtime
granularity, or a writer that took no lock) writes the document's version as
`<stem>.conflict-<fingerprint>.mid.ron` beside the file — a name that never collides, never
replacing an existing file (round 4, kimi F3) — with a notice naming both; never silently, never
over the other writer's. Same-stamp, write-during-commit and locked-by-MIDI-Evil tests. **The document's reload is never
  gated on the audio swap's validation**: a renamed track refuses the playback replacement, but
  the document still holds the new score with the old map, shows the diff's lines, and lets the
  map be fixed and saved — and that Save dispatches the replacement itself (gemini F1 of rounds 2
  and 3).
- **Map edits are allowed on any MIDI the loader plays.** `EditingPolicy::Refused` (mixed-channel
  tracks) scopes to score edits; parts, instruments, bindings, provenance and Save work for every
  playable file, with a test on a two-channel track (round 2, kimi F8).
- **A sidecar can be scaffolded**: `song init <path.mid>` stays as a map verb — a default sidecar
  with the project's first bank, no parts, no bindings, `looping: true` — and the Conductor panel
  opens an unmapped `.mid` the same way, the sidecar marked unsaved (round 3, gemini F8).
- **The fingerprint** (ruling (f)): Save keeps writing `midi_fingerprint`; on open, a fingerprint
  that differs from the bytes **with no journal present** means the score changed outside — the
  song opens on the new MIDI with its sidecar, the status bar says so, and the next Save rewrites
  the fingerprint; a mismatch **with** a journal is an interrupted save.
- **Recovery decides by fingerprint, never by stamp, over both finals** (round 2, kimi F3; round
  3, kimi F3, codex F1). The journal gains a third file, `<path>.new.prev`, holding **both finals' previous fingerprints —
absence included — and the journal's target fingerprints** at the moment the save began. On open
each final is classified as *previous* (untouched since), *target* (this save's own partial
commit) or *other* (changed outside): two previous → promote the pair; two targets → the save
finished, remove the journal; a target and a previous → complete the remaining write; an *other*
anywhere → open the finals as they are and keep the journal under recovered names
(`<stem>.recovered-<fingerprint>.mid` and/or `.mid.ron`) with a notice — nothing is deleted
(round 3, gemini F2; round 4, kimi F2, gemini F1, codex F2–F3). A lone `.mid.ron.new` (a map save
interrupted before its sidecar write) follows the same table over the one final it would write
**and the `.mid` it leaves alone**: both must be *previous*. A test per state. **The prune of
  unreferenced samples moves after the finals are written and the journal removed** (`io.rs:462`
  prunes between them today; round 3, kimi F2), so an interruption at any step leaves every sample
  the chosen song names on disk; the existing recovery and prune guard tests keep their contracts
  and gain the external-export cases: an engine-saved song then a MIDI Evil export then open; an
  export between an interrupted save and the reopen; a hand-edited sidecar between them.
- **The API** (`song open`, `song save`, `song init`) follows the same rules, so the headless path
  and the panel cannot disagree.

#### MIDI reload: swap at the next bar, keep playing

The replacement is **a prepared conductor, not a rebuild** (round 1, kimi F9, gemini F2, codex F2).
`AudioManager::replace_song_at_seam(path)` prepares the new closure **on a worker thread** (the
result over a channel, polled from the frame; the game thread only adopts the prepared conductor,
so a 48 MiB bank never stalls a frame — round 4, kimi F8) exactly as `play_song` does (the
same cache, so only the changed file is decoded again; the same `validate`, so a refusal leaves
the playing song untouched and names the sidecar entry and the track; **a selected track with no
notes is a silent part** — a trigger on it is a rest, counted in `triggers_silent`, the cursor held
— so ruling (a)'s muted track passes, round 2, kimi F4; a score with no notes at all,
`end_tick == 0`, is refused as "the score is empty"), builds a whole conductor for it **on the game
thread** — loaded, its slots and background synthesizers built, its bindings resolved by name into
its parts — and parks it in a small table the playback owns under a `ReplacementSlot`.

**Ownership** (round 2, kimi F6): the command channel, its `ConductorHandle` and `SharedPosition`
belong to the playback's `ConductorSource`, never to a conductor; a replacement swaps the conductor
inside the source, so the host's handle and position reader never change and no "output dead" can
follow a swap.

**The protocol** (round 1, kimi F6, gemini F5; round 2, kimi F5, gemini F5, codex F3, F6). Each
replacement carries the **playback generation** it was prepared for and its own **revision**;
its states are *pending* (prepared on the game thread), *armed* (drained by the conductor),
*active* (swapped), *superseded* (a newer revision armed before the seam) and *retired* (handed
back for drop), each transition owned by one thread and tested with A active while B is being
prepared. It travels as `ConductorCommand::Replace { generation, revision, slot }`, `Copy` like
every command, in the one ordered channel; the conductor refuses one for another generation.
**Every command drained while a replacement is armed applies to the old conductor** — the one
running — and the host converts beats to ticks with the old map until it sees `SongReplaced`; the
armed swap fires at **the first bar line at or after the transport's position when it fires**, so a
seek while armed moves the old song and the swap follows it; a stationary transport — paused, or
stopped at its end — takes it at once at its held position; a loop shorter than a bar takes it at
its wrap. A second save before the seam supersedes the armed revision; the superseded conductor is
dropped on the game thread. **Triggers carry the playback generation and a stable `ActionId`** (rounds 2 to 4): the host
assigns an `ActionId` per action name for the playback's life, never reused, and every revision's
binding table maps `ActionId` → part, so a trigger resolved under any revision is answered by the
active one with one integer lookup — no string on the audio thread, no kept tables, R0 → R1 → R2
before an acknowledgement included — and dropped, counted as stale, only when the action is gone.
So a notes-only edit silences no effect in the window between the swap and the host's next
`update`. **Map-dependent transport commands carry a musical position**
— `seek_song` sends bar, beat and fraction, converted by the conductor with its active map, never
a tick the host computed — and a position query decodes through the revision the snapshot names
(round 3, kimi F6, codex F3).

**At the seam** the audio thread does only this, allocation-free: (1) converts the position as **the bar index and the fraction of the bar in the old map — to the
tick at that fraction of the same bar in the new map, the fraction applied to the new bar's actual
length** (a partial last bar included), so bar 4 of the new score follows bar 3 of the old whatever
the tempo or meter did, a paused half-beat survives, a 4/4 → 3/4 change lands in the same bar at
the same proportion, and nothing maps past `end_tick` (rounds 2 to 4); a looping song takes the
bar modulo the new song's bar count (the engine's loop is the whole song, `sequencer.rs:61-126`;
there is no loop start); precedence: clamp inside the bar, then wrap for a looping song, then the
running non-looping refusal; for a running
non-looping song **re-checked here** (round 2, kimi F7, gemini F2, codex F5): a mapped position at
or past the new end refuses the swap — `ReplaceRefused { revision, reason }` published, the old
conductor playing on, the slot dropped by the game thread — instead of ending the transport, while a transport **stopped at the old song's end takes the replacement stopped at the new song's
end**, so a seek or a replay plays the new score (round 3, gemini F4), and **a running song that
reaches its end with a replacement armed takes it at once there** — playing on when the position
is inside the new song, an extended outro continuing, else stopped at the new end (round 4, gemini
F3); (2) swaps the conductor pointer,
seeks the new one to that tick instead of resetting its transport, so position, play state and the
looping flag carry over; (3) seeds the new conductor's `SourceTable` from the old one's
(activations, epochs, the flush floor — `keep_flush_floor_of` exists) and re-seats each part cursor
at the unit at or after the position; (4) publishes `SongReplaced { generation, revision }` in `SharedPosition` — **under a seqlock-style
version**, odd while writing, the reader retrying on a change, so position, transport and revision
are one coherent snapshot (`position.rs:72-73` allows a straddle today; round 4, kimi F6) — and
parks the retired conductor in **a preallocated slot in the source** that the game thread empties
and drops, never a channel that could fill (round 4, gemini F4). A trigger landing in the one block of the swap plays against whichever conductor owns
that block (1.45 ms, under the grid). The swap is a seek, and a seek releases every triggered note,
so a hit sounding across the bar line is cut at the bar — audible once per save, documented.

**The host's transition** (round 1, kimi F5): `play_song`, `stop_song` and the editor's session
end drop pending and armed replacements (codex F7). On the next `update` after `SongReplaced`, the
host adopts **that revision's** `Arc<Song>`, binding table and `BeatEdges` on the new tempo map as
one transition, so `trigger("boom")` after a reordered sidecar reaches the right part, an added
action resolves, and `beat_crossed` counts on the new map. Tests through the public `AudioManager`
API: a reordered binding, an added action, a tempo change, a
ticks-per-quarter change with a seek queued before, while and after arming and one sent between
the swap and the acknowledgement, a trigger sent in that window on an unchanged sidecar (plays the
same part), a replacement prepared for a song that was then stopped, a replacement over a transport
stopped at its end, B prepared while A swaps, a bank-only edit
(the SoundFont changed, the MIDI and the sidecar not). The timing fixture is the largest shipped instrument set (Chicken Coop's five) — the seam block's cost and the worker's
preparation time measured under `cargo test --release` and quoted. The guarantee: the background score and every later trigger stay on the same beat grid
as before the save.

#### Manifest validation on reload, and the track diff

Before a replacement is prepared, with both `SongData`s in hand, the host diffs **what each part
resolves to** (round 1, kimi F4, codex F3), not the name sets alone: for every part **whose selector is
unchanged between the old sidecar and the new**, its track's identity before and after, **by the
selector's own semantics** (round 3, codex F4): a `Name` part's identity is its name, so a uniquely
named track that moved index reloads unchanged; an `Index` part's identity is the name at that
index, so drift is caught; a part whose selector the author changed is an
explicit mapping, accepted and reported ("part `flute` now selects `Flute (lead)`"), so the repair
the diagnostic asks for goes through (round 2, codex F8). A part's track **missing** → `validate`
refuses, the error shows, the old song plays on; an unchanged selector whose resolved track
**changed identity** (an `Index(n)` that now lands on a different name) → refused, the message
naming both; a track **new** in the file that no part selects → "track `X` is new and no part
selects it; it plays as background" (every unselected track is background by design); a part's
track missing **and** a new name at the same index → "track `flute` is gone; index 1 is now `Flute
(lead)` — renamed in MIDI Evil? Edit the sidecar's part"; a `Name(..)` now **ambiguous** →
`validate` refuses and the message says to use `Index`. Every line to the console, the first to the
status bar. A `.mid.ron` saved by hand takes the same path. The diff is pure
(`external_tools/track_diff.rs`) over two resolved part lists.

### MIDI Evil's half — the contract across the crossing

The engine spawns, never waits, never reads stdout:

```
midi-evil [--project <dir>] [--track <name>] [--track-index <n>] <file.mid>
```

| element | meaning | required of MIDI Evil |
|---|---|---|
| `<file.mid>` | the absolute path of the Standard MIDI File (type 0 or 1) to open — for MIDI Evil, an **import** into a working `.evil` beside it or in its own store | open it; if an instance already shows it, focus that instance (single-instance forwarding is MIDI Evil's job) |
| `--track <name>`, `--track-index <n>` | the MIDI track-name meta event (`FF 03`) of the track to select (UTF-8, spaces allowed, one argv element) and its 0-based index in the file's track order, the index winning when both are given — the second of two `drums` is named exactly (round 4, kimi F4) | select it and scroll to it; unknown → open anyway, say so, exit 0 |
| `--project <dir>` | the game's asset base, where the sidecar's samples and SoundFonts resolve | optional; a hint for dialogs |
| exit status | 0 once accepted (the UI may still be loading), non-zero with one stderr line otherwise | the engine logs stderr's first line |
| **the export back** | MIDI Evil writes the `.mid` the engine opened: format 1, the tempo and **the time signature as written**, every track's name preserved (the join key to the sidecar), **a muted track as a named track with no notes** (indices hold), a program change at tick 0 and CC 7 / CC 10 from the track's patch, gain and pan, ticks-per-quarter as the author set it (480 default for the engine), complete tracks closed with `EndOfTrack`; written **atomically** (a temp file in the same directory, then rename), so the watcher never reads a half-written file; never `<file>.mid.ron` | hard requirements for hot reload to be trustworthy |
| **the lock** | `<file>.mid.lock`, `O_EXCL`, the writer's name, pid and stamp inside, removed after the rename, stale after 30 s — ruling (g) | taken around every export; the engine takes the same lock around its own writes and waits up to two seconds for MIDI Evil's |
| not in the contract | bar position, loop range, instruments, the sidecar's fingerprint | — |

MIDI Evil's issues, in its own repo, the contract's other half (`../midi-evil/docs/insiculous-crossing.md`
carries them for its agents): (0) `.evil` → `.mid` export with the rules above (today's export is
NanoStudio-only and 4/4-based at 128 ticks); (1) open a `.mid` given positionally and export back to
it on save; (2) `--track <name>`; (3) single instance; (4) `--project <dir>`; (5) the lock around every export; (6) a fixture pair the
engine's track diff can use — a `.mid` and the same file with one track renamed — checked into MIDI
Evil's fixtures and mirrored into `crates/music_midi/fixtures/` with a LICENSE line. Until (0) and
(1) exist, the engine side works with the Custom field (`cargo run -p midi-evil -- {file}`), and the
round trip is import → edit → export by hand; **batch 7 waits on (0) and (1)** (§ The batches).

### Cross-platform rules

Argv everywhere; `std::process::Command` quotes for `CreateProcess` itself; **no shell call anywhere** — the Windows OS default is `ShellExecuteW` (ruling (h)), the
association without a parse and without a trampoline (round 3, kimi F7, codex F5; round 4, gemini
F5).
A Flatpak app is not on `PATH` as its binary name: the "not found" error suggests `flatpak run
<app-id> …` as a Custom command, and the docs note the sandbox's filesystem grant. Snap binaries
live in `/snap/bin`. WSL: a resolved program ending in `.exe` on Linux gets every path placeholder
through `wslpath -w`, a failed conversion an error naming it; the OS default under WSL (`/proc/version` contains "microsoft") is ruling (h)'s mechanism on the
translated path, through `cmd.exe`'s host side only if no Windows binary of the engine is at hand
— the manual matrix says which. macOS: `open -a <App> <file>` is the "an app, not a CLI" escape hatch. A missing binary
is one sentence, never a panic. Web: the whole module is `cfg(not(target_arch = "wasm32"))` except
the types; the entry points draw disabled with today's hint.

### Patterns

Command (an `OpenRequest` is data the host executes later; a replacement is a command in the one
channel); Observer by polling (the watcher), over OS notification because the rule is no new
dependency until the simple thing has failed; the type table is a flyweight of behaviour per file
type. Nothing is a service locator: the host owns the watcher, the config rides the preferences,
the audio manager owns the swap.

### The Conductor panel (the composer re-scoped, then renamed)

| piece | fate | why |
|---|---|---|
| the piano roll, the note list, note editing (keys, drag, the note dialogs), the authoring verbs of the song API (`song new` as a score maker, `AddNote`, `MoveNotes`, `ResizeNotes`, `AddTrack`, `SetTempo`, `SetTimeSignature`) and `song/selection.rs` with the GUI-only helpers — `song init <path.mid>`, the sidecar scaffold, stays | **go — once MIDI Evil's export round-trips a shipped song** (§ The batches, batch 7's gate); until then the authoring verbs stay, and the GUI's piano roll may go first only once the verbs cover what it did headlessly | the score is MIDI Evil's; the engine never has a day with no way to write notes (round 1, gemini F1, codex F9) |
| the sidebar's parts, instruments and bindings, the SoundFont and Songs scans, the transport and Play audition, Save writing the sidecar and the LICENSE stanza (and the `.mid` only when the document's score is the file's — § The open song document), the song document's history for those edits, the map verbs (`AddPart`, `AddInstrument`, `SetBinding`, `SetProvenance`, `song`, `song notes`, `song save\|open\|close\|play\|stop\|seek\|trigger`) | **stay** | the conductor's half; the six games' songs were mapped through those verbs |
| `song_preview.rs`, `init_music.rs` | **stay** | how a Play session hears the game's song |
| `midi_fingerprint` | **kept, written by the engine's Save**; a mismatch with no journal is an external edit (ruling (f)) | the `.new` journal pairs by it |
| `scan_sound_fonts`, the Songs scan | **salvaged** into `external_tools/songs.rs` first (batch 4), then the panel reads them from there | the entry points need them before the re-scope |
| `editor/src/composer/` → `conductor/`, `composer_*` files, `PanelId::COMPOSER`, the View item "Composer" → "Conductor", the theme tokens' names, the guides' sections | **renamed** after the trim | ruling (c) |
| **the web playground** | **keeps the Conductor panel and its `.mid`/`.wav` import**; score editing on the web is download → MIDI Evil → re-import, and the browser's piano roll goes only with the native one, after the round trip is demonstrated | said plainly (round 1, codex F9): external launch is disabled on the web, so the browser's workflow is the file round trip |
| `docs/SONG_FORMAT.md` § How the composer saves, `docs/EDITOR_COMMAND_API.md` § Songs, the games' guides' "open the song in the composer" sentences, the roadmap's Phase K row | **rewritten** | the docs follow the ruling |

A song's authoring verbs, once gone, refuse with a message naming MIDI Evil; the open issues #200,
#197, #188, #187, #186 are re-read at batch 7: closed as superseded where they were the piano
roll's, kept where they are the map's. #183 and #182 (the song's save files) stay.

## The batches

Each batch is one handoff to the Opus executor from `review/external-tools/handoff-N.md`, reported
at `review/external-tools/report-N.md`, reviewed in code mode (kimi always; gemini on a harness,
fixture or seam; codex on a screen, from stills the report names) and committed by the planner on
Jesse's word. Issue numbers are the milestone's; the batch names what the issues do not.

**Gates, mechanical, every batch:** `cargo test --workspace` (0 failed, 0 ignored), `cargo clippy
--workspace --all-targets` (0 warnings), the comment-tag grep from `CLAUDE.md` printing nothing,
`scripts/check_wasm.sh`; `scripts/check_games.sh` when `engine_core` or `audio` changed;
`scripts/check_guide_tests.sh` when a guide's pitfall table changed; the 600-line ceiling over
every touched `.rs`; `scripts/check_audio_provenance.sh` when a fixture lands. The planner reruns
every gate on the staged tree before the code round.

**Batch 1 — the foundation: #203 (E1), #204 (E2), #205 (E3).** The pure half (the type table, the
group-first templates, the config, the presets with MIDI Evil's row, the resolution order), the
asset browser's `midi`, `SoundFont` and `DawProject` kinds, and the launcher replacing
`open_source.rs` (the path-first program lookup, the two threads with the reaper joining the
reader, ruling (h)'s Windows default, the project templates, the explicit `OsDefault` override, the
`ide_command` migration with `{file}` appended, the Open Source button working through it). Files:
new `crates/editor/src/external_tools/{mod,file_type,template,config,presets}.rs`,
`editor_preferences.rs` (`ide_command` read into `code: Custom(..)` when set, no longer written),
`asset_browser.rs`, new `editor_integration/src/editor_game/external_tools.rs` (`open_source.rs`
deleted, its headless tests carried over), `editor/src/context/mod.rs` (`open_requests:
Vec<OpenRequest>`), `script_editor.rs`. Target shapes: § One table, § Types, § Command templates,
§ Presets, § The launcher. Reviewers: kimi, gemini (the launcher's lookup, spawn and stderr drain
are a seam). Leaves out: every entry point but the button, the watcher, the settings UI. Under
1,300 lines.

**Batch 2 — code entry points and the console: #206 (E4), #207 (E5).** Double-click on tiles and
Scripts rows, the first context menu (keyboard-reachable: the menu key, Shift+F10, arrows), the
Console panel as § Entry points describes it, repeats collapsed, the status bar's error clickable.
Reviewers: kimi, codex (two stills: the context menu open over a tile; the console with a clickable
error and a collapsed repeat). Leaves out: Edit in MIDI Evil (batch 4), filters or log levels.

**Batch 3 — the watcher and code reload: #208 (E6), #209 (E7), #210 (E8), #211 (E9).** `FileWatch`
on its worker thread with its registered set, debounced tombstones, companion files, `note_written`
baselines and the rebuild roots; `hot_reload.rs`
with the registered set's rebuild rules; Rhai recompile on save in a Play session
(`ScriptRunner::reload_source`, the AST swapped, blackboards kept, a syntax error keeping the old
AST and linking the line); scene and sheet reload through the existing guards and dialog, the sheet as a transaction and
re-applied on Stop, the first-tick rule after a script reload; the sticky rebuild notice. Target shapes: § File watching and hot reload (the
first four policy rows). Reviewers: kimi, gemini (the watcher's debounce and tombstones, the
runner's swap and Stop's re-apply are harness and seam). Leaves out: the song and sample policies
(batches 4 and 5), any `cargo` spawn.

**Batch 4 — music entry points and the sample reload: #212 (E10), #215 (E13).** `scan_songs`
salvaged into `external_tools/songs.rs`; the Edit in MIDI Evil button on a `Binding` emitter with
the action → part → track resolution; `.mid`, `DawProject` and sample tiles opening with their
tools; the context menu's item enabled; recorded sounds reloading in place behind their handle, the
library's loaded and failed paths registered, a failure cleared on success (the sound half of #180,
that issue updated). Target shapes: § Entry points rows 1 and 6, "Which song is the song", the
`ReloadSample` row. Reviewers: kimi, gemini (the sound library's reload), codex (one still: the
emitter's settings with the button). Leaves out: the song swap.

**Batch 5 — the song follows the score: #213 (E11), #214 (E12).** The open song document's
two-file policy by content fingerprint, the sidecar-only save over the loaded bytes, the conflict
file, the transition's score-authoring rule, the `.new.prev` journal over both finals and the journal's targets with its three states, the
lone sidecar journal, the lock (ruling (g)), the prune moved after the finals, the map edits on
mixed-channel MIDI, `song init`, and the panel's Save dispatching the reload, with their migration
tests (§ The open song document); the silent
part in `music_midi`'s `validate` and trigger engine; `replace_song_at_seam` with the prepared conductor,
the `Replace` lifecycle command with its generation and revision and the five states, the armed
seam, the stationary-transport, stopped-at-end and short-loop rules, the bar-and-fraction crossing, the end-while-armed rule, the seam's own re-check, the stable
`ActionId` on every trigger, musical-position transport commands, the seqlock snapshot, the
retired conductor's slot, the worker-thread preparation, the source's ownership of the channel, the retired conductor
dropped on the game thread, the host's revision transition (§ MIDI reload); the `conductor_background` example's `--replace-after
<bars>` flag; the `ReloadSong` dispatch with the debounce → prepare → diff → replace chain and
`track_diff.rs` over resolved parts. Reviewers: kimi, gemini (the audio seam; the half-written-file
and the renamed-track fixtures). Listen check: Jesse, Tong in the editor, a note changed in MIDI
Evil (or any editor that writes a `.mid`) and saved mid-bar, heard from the next bar with the hits
on the beat before and after. Leaves out: anything in MIDI Evil's repo; the fixture pair until MIDI
Evil ships it (a hand-made pair in `music_midi/fixtures/` with a LICENSE line stands in). The
largest batch; if the report runs long it splits at the document policy (5a) and the swap (5b).

**Batch 6 — the settings panel: #216 (E14).** `settings_panel.rs` over `EditableInspector`: a
cycle row per category for the preset, a string field for the Custom template with the inline red
invalid state, per-type overrides, the MIDI Evil binary field, the DAW project extensions, a Test
button per category; Edit → External Tools…; keyboard-walkable with a visible focus ring, labelled
controls, no colour-only state; narrow mode as the dock's overlay. Closes #102. Target shapes:
§ Types. Reviewers: kimi, codex (two stills: the panel at rest; a template error shown). Leaves
out: nothing of the brief's settings.

**Batch 7 — the Conductor panel: #217 (E15, reworded), #218 (E16, reworded), the rename (filed at
settling).** **Gated on MIDI Evil's `.evil` → `.mid` export and positional open round-tripping one
of the six shipped songs through the engine** — import, a note changed, export, `cargo test` in
that game green over the exported file and the song opening in the panel — demonstrated and quoted
in the handoff. Then the trim: the piano roll, note list, note editing and the authoring verbs go;
the sidebar, transport, audition and Save stay; the authoring verbs refuse naming MIDI Evil; the
five composer-defect issues re-read; the playground's export still carries the open song's two
files and the browser's reduction is written into `docs/WEB_PLAYGROUND.md`. Then the rename, as
its own commit: module, panel id, View item, theme tokens, guides. Target shapes: § The Conductor
panel. Reviewers: kimi, gemini (the API's refusals and the playground's export are seams), codex
(two stills: the Conductor panel on `ai_tong.mid`, native and in the playground). Leaves out:
ruling (e)'s gain field unless Jesse rules it in. If the gate is not met when batch 6 lands, batch
8 goes first and batch 7 waits; the sprint does not close without it.

**Batch 8 — platform polish and the docs: #219 (E17), #220 (E18).** WSL paths, the Flatpak hint,
the three OS-default fallbacks and the WSL one exercised by hand on a Linux, macOS and Windows
machine with a path containing a space and an `&` (the results recorded in the issue); `docs/EXTERNAL_TOOLS.md` (the presets with
their sources and dates, the template grammar, the resolution order, the reload table, MIDI Evil's
contract, the platform notes), the SSOT row in `CLAUDE.md`, `training.md`'s directory map, the two
editor guides, `docs/SCRIPTING.md`, `docs/SONG_FORMAT.md` (the fingerprint's rule; MIDI Evil's
export rules), the games' guides' composer sentences, the roadmap. Reviewers: kimi. Leaves out:
nothing; this closes the sprint, and the close-out (the milestone's issues closed by their commits,
the lessons to `log_archive.md`, `review/external-tools/` archived, the scorecard's totals) is the
planner's.

**Order.** 1 → 2 → 3 in sequence (each builds on the last's types and console); 4 after 2; 5 after
3 and 4; 6 any time after 1; 7 after 4 and 5 and its gate; 8 after 6, before 7 if 7's gate is not
yet met. One batch out at a time, as the Conductor ran it.

## Verification, end to end

- Every batch: the gates above, run by the planner on the staged tree, never from the report.
- After batch 3: the editor on pong, `touch assets/scripts/paddle.rhai` from a shell → the console
  says so within a second; Ctrl+S on the scene → no reload; in Play, change `speed` in
  `paddle.rhai` and save → the paddle moves faster, no Stop; break the syntax → the old speed holds
  and the console links the line; a sheet changed in Play → still changed after Stop.
- After batch 5: Tong in the editor, Play; a note changed in MIDI Evil (or any editor that writes a
  `.mid`) and saved mid-bar → heard from the next bar, the hits on the beat before and after; the
  `flute` track renamed → the status bar says the part's track is gone and the console suggests
  the rename; the part's selector fixed in the panel and saved → the song swaps at the bar from the
  panel's own Save; a binding changed in the panel and saved
  after an export → the exported notes survive.
- After batch 7: the editor on pong opens `ai_tong.mid`: the sidebar shows its parts, instruments
  and bindings, Play auditions it, no note can be moved; `song open` / `song notes melody` / `song
  save` answer over `--headless`; a saved song passes the provenance gate; a `.mid` tile
  double-clicks into MIDI Evil; the gate's round trip is quoted in the handoff.
- Final: a reader with no context sets up VS Code from `docs/EXTERNAL_TOOLS.md` alone and opens a
  script at a line; `scripts/check_wasm.sh`, `check_games.sh`, `check_guide_tests.sh` green; no doc
  tells a reader to open the composer.

## Out of scope, said plainly

No embedded editor of any kind; no web editor work beyond keeping the gate green and writing the
browser's reduction down; no MIDI Evil code in this repo; no `cargo` spawn (#211's follow-up names
it); no texture hot reload (#180's other half stays open); no `notify`/inotify dependency until
polling has failed in practice; no locale or `GameConfig` RON reload; no new sidecar field until
ruling (e) is taken; no manifest (`Cargo.toml`) watching.

## Carried from the Conductor effort (the lessons that bind every batch here)

The planner's gates run on the staged tree, never from the report; every wait in a test is a
bounded poll or a step count and the executor greps its own test file for `while`/`loop`; a retry
loop re-reads every id a restart replaces; a match start or a seek is asserted by the event's
counter, not by the clock; a draft diff that omits binaries or the lockfile says so in its prompt;
codex is dispatched after kimi exits; kimi streams its file, so nothing polls non-emptiness; the
site's two-name contract and the headless virtual clock are the two things gemini misreads every
round; a CDP-driven menu is confirmed open before its moves; a tall canvas's stills want a window
where the canvas lays out at its own size; the audio thread never allocates, parses or drops.
