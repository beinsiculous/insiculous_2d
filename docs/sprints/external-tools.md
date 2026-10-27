# Sprint: External Tools — design note and issue plan

**Part:** Games (the Insiculous 2D editor). **Status:** approved by Jesse 2026-10-26; the eighteen issues are filed as
`beinsiculous/insiculous_2d` #203–#220 under the milestone "Sprint: External Tools" (the map is in § 7). **Author:** Claude Fable (planner). Nothing here is code;
the issues in § 7 are what gets built, one at a time, each independently mergeable.

**Goal.** From the editor, open a project file in the right external app, landing on the right
place (a line for code, a track for MIDI), and hot-reload when the file changes on disk. One
mechanism serves code (Rust, Rhai, RON) and music (MIDI, samples, DAW projects).

---

## 1. What exists today (explored 2026-10-26)

| concern | what is there | verdict |
|---|---|---|
| Open in IDE | `editor_integration/src/editor_game/open_source.rs`: `ide_command` (a string in `editor_prefs.json`) split on whitespace, `{file}` appended, spawned, the child reaped on a thread. Reached only from the Scripts inspector's **Open Source** button (`editor/src/script_editor.rs:140`) through `EditorContext.pending_open_source: Option<String>`. No quoting, no line, no project, no presets. Issue #102 records that it can only be set by hand and the edit is lost while the editor runs. | **Replace**, keep its shape: a request field on `EditorContext`, drained once a frame by the host, native-only behind `cfg`, the zombie-reaping thread. `ide_command` is read on load and becomes the Custom code command. |
| Preferences | `editor/src/editor_preferences.rs`: `EditorPreferences`, **JSON**, through `save_store` (native file / wasm localStorage), settle-rule autosave in `preferences.rs`, `#[serde(default)]` for new fields. | **Reuse.** The brief says "RON config"; the editor's config is JSON today (see § 2, decision B). |
| Asset browser | `editor/src/asset_browser.rs` (`AssetKind`: Image, Scene, Script `.rhai`/`.rs`, Sound `.wav/.mp3/.ogg/.flac`, Song `.mid`, SoundFont), `panel_renderer/asset_browser.rs` + `tile.rs` (press arms a drag, click selects, hover tooltip). No double-click anywhere in `ui`, no context menu anywhere in the editor (#137 lists the hierarchy's absence). | **Extend**: a double-click rule in the tile and a first context menu. |
| Console | `PanelId::CONSOLE` (`dock/mod.rs:40`) is reserved, "no panel implementation yet"; the View menu's Console item is disabled (`menu/mod.rs:225`). Script errors go to the **status bar**, one at a time (`script_status.rs`), from `ScriptError { file, line, kind, message }` (`engine_core/scripting/runner.rs:39`). The status bar holds one message. | **Build** the panel the dock already reserves; `ScriptError` already carries file and line. |
| Scripts | `ScriptRunner` compiles a `.rhai` on first use per `(entity, script)` instance (`runner.rs:272`), caches by `source_path`, quarantines failures; `reset` rebuilds everything at session start. No reload path. | **Add** `reload_source(path)`. |
| Scene RON | `scene_io.rs` `load_scene` dry-runs into a scratch world; save and open refused mid-session; `ConfirmDialog` machine for unsaved work (`scene_confirm.rs`). `.sheet.ron` sidecar is SSOT at load, `clear_sidecar_cache()` exists. | **Reuse** for reload-from-disk. |
| File watching | None. `common::vfs::change_stamp` (length + mtime, `None` on wasm) exists and the audio dependency cache already keys freshness on it (`audio/src/manager/song/load.rs`). | **Build** a polling watcher on `change_stamp`; no `notify` dependency (the engine's "no new dependencies" rule and KISS both say so; a few dozen stats a second cost nothing). |
| Songs | `AudioManager::play_song(path)` reads `.mid` + `<path>.ron`, prepares the whole closure (SoundFonts, samples) through a stamp-fresh cache, **then** swaps — a failed load leaves the playing song alone. It always restarts from tick 0 and `Conductor::load` resets the transport. `ConductorCommand` is `Copy`, no song-replace command. `TempoMap::position_at_tick` gives bar/beat. `SongFile::validate` already refuses a part whose track is missing or ambiguous. The audio loader does **not** check `midi_fingerprint` (only the composer's open path does). | **Add** a seam swap; validation exists; the fingerprint is harmless to a DAW-written file. |
| Sound manifest | It is the `.mid.ron` sidecar (`music_midi/src/song_file.rs`, `docs/SONG_FORMAT.md`): `parts` (name → `track: Name(..) | Index(n)`), `instruments`, `bindings` (action → part + instrument). An entity's sound is `SoundEmitter { sound: Binding("hit") }`. | That is the "RON sound manifest" of the brief. Track → entity is action → binding → part → track. |
| Recorded sounds | `AssetManager::resolve_sound` caches handles by path, never invalidates (#180 names the gap). | **Fix** the sound half of #180 as the sample reload. |
| Which song is the project's | `init_music.rs` captures the song the game's `init` asked for (`audio.song_request()`); the composer scans `music/` and `ai/music/` for the Songs list. A data project (`ProjectHost`) has no `init`. | **Reuse** both for "Edit in DAW". |
| The build | Decision of record (`docs/EDITOR_COMMAND_API.md` § "the editor does not own the build"): any editor-triggered rebuild is "spawn cargo, relaunch self"; **no build-system work now**. | A `.rs` change shows a notice; no rebuild in this sprint. |
| The composer | ~10,700 lines incl. tests: `editor/src/composer/` (panel, keys, drag, sidebar, piano roll), `editor/src/song/` (the `SongDocument`, its history, save journal, LICENSE stanzas), `command_api/song.rs` (the `song …` verbs), `editor_integration` `composer_keys.rs`, `composer_drag.rs`, `song_io.rs`, `song_preview.rs`, `panel_renderer/composer.rs`, `scene_confirm.rs`'s song variants, `snapshot.rs`'s `live_files`, and the playground's song persistence/import (`playground/src/archive/songs.rs`, `upload.rs`, `persist/`). | See § 2, decision A, and § 6. |

---

## 2. Decisions — ruled by Jesse 2026-10-26 ("looks great, go ahead": each recommendation below stands)

**A. The composer's retirement collides with a standing ruling.** `PROJECT_ROADMAP.md` § Phase K
says, in Jesse's words of 2026-09-25, "the in-engine composer (K8) is a feature that sets the
engine apart … not a cut candidate"; #141 (K8) is open on the Conductor milestone and the
milestone's description repeats the ruling; the Conductor effort is mid-flight (batch 14 settled
2026-10-26). This brief says the composer is retired in favour of the DAW. Both cannot stand.
The plan treats retirement as the new ruling but **sequences removal last and gates it** on:
(1) a one-line roadmap edit by Jesse recording the reversal and its date, (2) #141 closed as
superseded (or re-scoped to "audition only"), (3) the Conductor effort's batch 15 and close-out
landed, so the two efforts never edit `song_preview.rs` at once.

**A2. How much of the song machinery goes.** The composer is three layers: the GUI (piano roll,
sidebar, keys, drag), the **song document** (`SongDocument`, history, the atomic save journal,
the LICENSE/provenance writer), and the **command API** `song …` verbs. The games' two songs
were sequenced by an agent *through those verbs* (`games/pong/CLAUDE.md:167`,
`games/breakout/CLAUDE.md:184`) — that is the AI-first north star's authoring path, and a DAW
has no API an agent can drive. Recommendation: **remove the GUI (§ 7, E15); keep the song
document and the `song …` verbs as the headless authoring and provenance path (E16 becomes a
trim, not a delete).** Alternative: remove all three and accept that an agent writes `.mid`
bytes and the sidecar by hand, and that nothing writes the LICENSE stanzas the audio
provenance gate requires. Say which.

**B. Where the settings live.** The brief says "the editor's RON config". The editor's one
config is `editor_prefs.json` (JSON, `save_store`, settle-rule autosave, wasm-safe). Adding a
second file in a second format for one feature fails DRY (two persistence paths) and KISS.
Recommendation: a `external_tools` field on `EditorPreferences`, JSON, `#[serde(default)]`.
If RON is the requirement, say so and E2 grows a second store.

**C. Helix is a terminal editor.** The verified Helix syntax is `hx [-w <dir>] file[:row[:col]]`,
but spawning `hx` from a GUI process gives it no terminal. The preset ships as documented with a
panel note "prefix your terminal's run command"; nothing guesses a terminal. Alternative: drop
the preset.

**D. DAW project files.** The brief lists "DAW projects" as a type; the DAW is early and its
project extension is unknown. The type exists with an **empty, editable extension list** until
the DAW has one. Confirm.

**E. A part that selects a nameless track by index.** `{track}` expands to the track's **name**
(the MIDI track-name meta event, which is what `TrackSelector::Name` matches). A part selecting
`Index(n)` whose track has no name gets a status notice and the DAW opens the file unfocused.
Alternative: a sixth placeholder `{track_index}`.

---

## 3. Design

### 3.1 One table, one mechanism

```
FileType  ──►  ToolCategory  ──►  ToolChoice (per category, overridable per type)  ──►  Launch
   │                 │
   └── ReloadPolicy  └── Code | Music | Sample     (the category decides the presets and the env fallback)
```

**`FileType`** (editor crate, `external_tools/file_type.rs`; the one place the mapping lives):

| type | extensions | category | `{project}` | reload policy |
|---|---|---|---|---|
| `RustSource` | `.rs` | Code | Cargo workspace root | `RebuildNotice` |
| `RhaiScript` | `.rhai` | Code | Cargo workspace root | `RecompileScript` |
| `Ron` | `.ron` (scene), `.sheet.ron`, `.mid.ron` | Code | Cargo workspace root | `ReloadDocument` (scene) / `ReloadSheet` / `ReloadSong` (by sidecar suffix) |
| `Midi` | `.mid`, `.midi` | Music | asset base | `ReloadSong` |
| `AudioSample` | `.wav`, `.ogg`, `.flac`, `.mp3` | Sample | asset base | `ReloadSample` |
| `DawProject` | configurable, empty by default (decision D) | Music | asset base | `None` |

The reload policy is **data in this table, not a setting** — the brief's "file type → tool +
command template + reload behavior" is satisfied with the behavior fixed per type and the tool
chosen per type. Users pick tools; the engine knows what a file means.

### 3.2 Types

```rust
// crates/editor/src/external_tools/
pub enum ToolCategory { Code, Music, Sample }
pub enum ToolChoice { Preset(PresetId), Custom(CommandTemplate), OsDefault }
pub struct ExternalToolsConfig {            // on EditorPreferences, #[serde(default)]
    pub code: ToolChoice,                    // default: OsDefault (so $VISUAL/$EDITOR is tried first)
    pub music: ToolChoice,                   // default: OsDefault
    pub sample: ToolChoice,                  // default: OsDefault
    pub overrides: BTreeMap<FileType, ToolChoice>,
    pub daw_project_extensions: Vec<String>,
}
pub struct CommandTemplate(String);          // "rustrover [--line {line}] [--column {column}] {file}"
pub struct Launch { pub program: String, pub args: Vec<String>, pub preset: Option<PresetId> }

pub struct OpenRequest { pub path: PathBuf, pub location: Location }     // replaces pending_open_source
pub enum Location { None, Line { line: usize, column: Option<usize> }, Track(String) }

pub enum ReloadPolicy { None, RecompileScript, ReloadDocument, ReloadSheet, ReloadSong, ReloadSample, RebuildNotice }
```

**Resolution** (`config.rs`, pure, tested) — the brief's order, verbatim:
1. `overrides[file_type]` if present and not `OsDefault`;
2. the category's choice (`code` / `music` / `sample`) if not `OsDefault`;
3. **Code only:** `$VISUAL`, then `$EDITOR`, each tokenized as a template with `{file}` appended
   (no line — these editors are unknown; a terminal editor will fail at spawn and the status bar
   says so);
4. the OS default: Linux `xdg-open {file}`, macOS `open {file}`, Windows `cmd /C start "" {file}`
   (the empty quoted first argument is `start`'s window title, which otherwise eats the path).

### 3.3 Command templates (`template.rs`, pure, tested)

- Tokenized **shell-like** (double and single quotes, backslash escapes, whitespace splits), in
  the editor crate, ~60 lines, no `shlex` dependency. A program path with spaces is quoted in
  the template; today's `ide_command` could not express one (its guide says so).
- Placeholders `{project}` `{file}` `{line}` `{column}` `{track}` are substituted **per token
  after tokenization**, so a path with spaces never splits and never needs quoting.
- **Optional groups** `[ … ]` are dropped whole when any placeholder inside is unresolved. They
  may nest and may attach to a token: `{file}[:{line}[:{column}]]`. This is how the RustRover
  doc writes its own syntax, and it is the only rule the presets need.
- Unknown `{name}` is an error at config time (the settings panel shows it; the launcher refuses).
- `{file}` is always **absolute** (`resolve_asset_path`); `{project}` per the table above.

### 3.4 Presets — verified 2026-10-26 against the vendors' own pages

| preset | binary (Linux / macOS / Windows) | template | source |
|---|---|---|---|
| RustRover | `rustrover` / `rustrover` / `rustrover.exe` (the page: "By default, RustRover does not provide a command-line launcher" — the Toolbox script on this machine is `~/.local/share/JetBrains/Toolbox/scripts/rustrover`) | `rustrover [--line {line}] [--column {column}] {file}` | jetbrains.com/help/rust/opening-files-from-command-line.html — verbatim `rustrover [--line <number>] [--column <number>] <path>`; passing a project **and** a file in one call is **not stated**, so `{project}` is not in the template: RustRover attaches the file to the project that contains it when that project is open. The panel's "Open project" action is a second launch, `rustrover {project}`. |
| VS Code | `code` (macOS needs the Shell Command install) | `code {project} --goto {file}[:{line}[:{column}]]` | code.visualstudio.com/docs/configure/command-line — `-g`/`--goto` "When used with file:line{:character}, opens a file at a specific line and optional character position"; a folder argument opens it as the workspace. The folder-plus-`--goto` combination is not shown as one example; it is two documented arguments in one call, flagged for the manual test. |
| VSCodium | `codium` | as VS Code | github.com/VSCodium/vscodium/blob/master/docs/usage.md confirms `codium`; the flags being VS Code's is **not stated** there (VSCodium is a VS Code build) — flagged for the manual test. |
| Zed | `zed` (Linux "commonly `zed` or `zeditor`") / `zed` / `zed.exe` | `zed {project} {file}[:{line}[:{column}]]` | zed.dev/docs/reference/cli.md — verbatim `zed myfile.txt:42:10  # Open at line 42, column 10` and `zed file1.txt file2.txt ~/projects/myproject`. |
| Helix | `hx` | `hx [-w {project}] {file}[:{line}[:{column}]]` | helix-term/src/main.rs help text: `<files>...  Set the input file to use, position can also be specified via file[:row[:col]]`, `-w, --working-dir <path>`; the book's command-line page documents the `:` prompt, not startup args. Terminal editor — decision C. |
| My DAW | per § 3.9 | `{daw} [--project {project}] [--track {track}] {file}` | the contract this note defines. |
| Audacity | `audacity` (Flatpak id `org.audacityteam.Audacity`, from the Flathub slug and manifest repo name) | `audacity {file}` | `AudacityApp::ParseCommandLine` (release-3.7.8): `AddParam(_("audio or project file name"), wxCMD_LINE_VAL_STRING, wxCMD_LINE_PARAM_MULTIPLE \| wxCMD_LINE_PARAM_OPTIONAL)`; no time-position option exists. The 3.x manual has no command-line page (404). Audacity 4 restructured the repo — re-verify when it ships. |
| Custom command | any | the user's template | — |

The preset table is data (`presets.rs`), each row carrying its source URL in a doc comment, so the
next verification is a diff against the page, not a hunt.

### 3.5 The launcher (`editor_integration/src/editor_game/external_tools.rs`, replaces `open_source.rs`)

Per frame, drain `EditorContext.open_requests: Vec<OpenRequest>` (a Vec, not an Option: a
context menu and a console click can land in one frame). For each: resolve `FileType` →
`Launch`; **look the program up on `PATH` yourself** (iterate `PATH`, honour `PATHEXT` on
Windows) so a missing binary is reported as `rustrover: not found on PATH (searched 11
directories) — set the tool in Edit → External Tools…` rather than the OS's `No such file`;
spawn with `std::process::Command` (argv, never a shell); reap on the existing thread; status
bar + console line for success and failure. `cfg(target_arch = "wasm32")` keeps today's hint.
A spawn never blocks the frame and never panics: every error is a `Result` into the console.

**Cargo workspace root** (`workspace_root(asset_base)`): walk up from the asset base; the nearest
`Cargo.toml` containing `[workspace]` wins, else the nearest `Cargo.toml`, else the asset base's
parent. For `games/pong` that is `games/pong/` (its own manifest, the engine a path
dependency) — exactly what rust-analyzer needs; for the engine's examples it is the engine root;
for a data project with no manifest, the project directory. Computed once at init, recomputed on
Open.

### 3.6 Entry points

| entry | where | request |
|---|---|---|
| **Double-click** a tile in the asset browser | `panel_renderer/asset_browser/tile.rs` — a second press within 400 ms and 4 px of the first, on the same tile, with no drag armed. The `ui` crate gains nothing; the tile's own state remembers the last press. | `OpenRequest { path, Location::None }`; a `.mid` tile → `Location::Track` of the first part's track if the sidecar has one |
| **Double-click** a Scripts row in the inspector / the hierarchy's `Scripts` pseudo-row | `script_editor.rs`, `hierarchy/` | the script's `source_path`, `Location::None` |
| **Open Source** button (exists) | `script_editor.rs:140` | as above, through the new request type |
| **Context menu** "Open in external tool" / "Open containing folder" / "Edit in DAW" | a first `ContextMenu` widget on `UiLayer::Floating` (the menu bar's dropdown geometry reused: `menu/` already draws an overlay band that blocks input), opened by right-press on a tile or a Scripts row; the editor has none today (#137) | per item |
| **Click a `file:line`** in the console | `panel_renderer/console.rs` — each entry with a `Location` draws underlined; click → request. The status bar's current error, when it carries a location, opens it on click too. | `Location::Line` |
| **Edit in DAW** on an entity's sound settings | `component_editors/sound_emitter.rs`: a `SoundEmitter` whose sound is `Binding(action)` gets an **Edit in DAW** action button. Resolution: the song (§ 3.7) → `bindings[action].part` → `parts[part].track` → `Name(n)` or the named track at `Index(i)` → `Location::Track(name)`. | `OpenRequest { song.mid, Location::Track }` |
| **Edit → Open project in code tool** | menu action | `OpenRequest { project, Location::None }` for `RustSource` |

### 3.7 Which song is "the" song

In order: the song a Play session is playing or the game's `init` asked for (`InitMusic`, by
path); else the one `.mid` under the asset base's `music/` or `ai/music/` (the composer's
Songs scan, salvaged as `scan_songs`); several → the sound settings show a chooser row; none →
status error "no song in this project". A data project with a `SoundEmitter(Binding(..))` and no
song is a scene-authoring error already (the binding never plays); the button says so.

### 3.8 File watching and hot reload

**`FileWatch`** (`editor/src/external_tools/watch.rs`, pure over `common::vfs::change_stamp`):
a registered set of absolute paths with their last stamps; `poll()` returns the paths whose stamp
changed **and has then stayed unchanged for one further poll** (debounce — a DAW or an IDE may
write in several steps, and a half-written `.mid` must not be read). Polled from `finish_frame`
every 500 ms on native (not every frame; the timer rides the frame clock like the preferences'
settle rule); never on wasm (`change_stamp` is `None` there). Deleted files report once and are
dropped from the set. The set is rebuilt when it would change: the open scene's path and every
`source_path` on every `Scripts` component (after a scene load and after each history step that
touches `Scripts`), the playing song's `.mid`, `.mid.ron` and every sample and SoundFont it names
(the closure the audio cache already computed), every `SoundEmitter(Path)` sound, every
`.sheet.ron` the scene's sprites reference, and — for `RebuildNotice` only — the `.rs` files under
the workspace root's `src/` and `crates/*/src/` (listed with `vfs::list_files`, re-listed every
30th poll so a new file is noticed). The mtime-granularity pitfall (`vfs` docs: same length within
the filesystem's granularity keeps its stamp) is accepted and documented; a second save a second
later is caught.

**Dispatch** (`editor_integration/src/editor_game/hot_reload.rs`), by the table's policy:

| policy | editing | Play / Paused |
|---|---|---|
| `RecompileScript` | nothing to do (the runner compiles at session start) — a console line "player.rhai changed" | `ctx.scripts.reload_source(path)`: recompile; on success swap the AST for every instance of that path, clear its `failed` and quarantine entries (the author fixed the file), keep blackboards and params; on a syntax error keep the old AST running and put the error, with its line, in the console as a link |
| `ReloadDocument` (scene) | history **clean** → `load_scene` from disk (the dry-run guard keeps a bad file from touching the live world), status "reloaded from disk"; **dirty** → `ConfirmDialog` "scene changed on disk: Reload (discard my edits) / Keep mine" | deferred: a notice now, the same rule applied at Stop |
| `ReloadSheet` | `clear_sidecar_cache()` + reload that sheet's texture/clips for every `SpriteAnimation` naming it | same (a sheet is an asset, not the simulated world) |
| `RebuildNotice` | status notice "Rust source changed — rebuild and relaunch to see it" + console line; sticky until relaunch (a counter of changed files) | same |
| `ReloadSong` | nothing plays while Editing; a console line | § 3.8.1 |
| `ReloadSample` | recorded sound: `AudioManager` reloads the bytes behind the **same `SoundHandle`** so every emitter and `resolve_sound` cache entry stays valid (the sound half of #180); a song sample: it is in the song's closure → § 3.8.1 | same; a looping instance already playing finishes with the old bytes, the next play has the new |

#### 3.8.1 MIDI reload: swap at the next bar, keep playing

`AudioManager::replace_song_at_seam(path)` prepares the new closure exactly as `play_song` does
(same cache: only the changed file is decoded again; same `validate`: a refusal **leaves the
playing song untouched** and returns the error naming the sidecar entry and the track), then hands
the `Arc<Song>` to the conductor through a **one-slot mailbox** (`Arc<Mutex<Option<Arc<Song>>>>`
beside the command channel; the audio thread `try_lock`s once a block and swaps a pointer — the
lock is held for one pointer move, never contended for longer; `ConductorCommand` stays `Copy`).
A second save before the seam replaces the pending song (last wins).

At the next block whose span contains a bar line (`TempoMap::position_at_tick` of the old map —
the seam is the old song's bar, which is what the listener hears), the conductor:
1. computes the musical position as **bar + beat** in the old map and converts it to a tick in the
   new map (ticks-per-quarter and tempo may both differ);
2. rebuilds slots and background as `load` does, then `Seek`s to that tick instead of resetting the
   transport — so position, play state and looping flag carry over;
3. keeps the `SourceTable` (activations, epochs, the flush floor — `keep_flush_floor_of` already
   exists), re-resolves every binding **by name** into the new song's part indices, and re-seats
   each part cursor at the unit at or after the position (the seek rule); a binding the new sidecar
   lacks is dropped with a console warning;
4. publishes a `SongReplaced` note in `SharedPosition` so the host can say so.

A trigger that lands in the one block of the swap plays against whichever song owns that block
— one block is 1.45 ms, below the quantization grid, so the beat is kept. The swap is a `Seek`,
and a seek releases every triggered note (the existing rule), so a hit sounding across the bar
line is cut at the bar: audible once per save, acceptable, documented. **The sync guarantee is
that the background score and every later trigger stay on the same beat grid as before the save.**

#### 3.8.2 Manifest validation on reload, and the track diff

Before the swap, with both `SongData`s in hand (old from the conductor's `song()`, new from the
prepare), the host diffs the **track-name sets**:

- a part's track **missing** → `validate` already refuses; the error is shown and the old song
  plays on (the brief's "missing" case is a refusal, not a warning — a silent part is worse);
- a track **new** in the file that no part selects → warning "track `X` is new and no part
  selects it; it plays as background" (in the engine's model every unselected track is background
  by design, so this is the brief's "unmapped" case);
- a part's track missing **and** a new name at the same index → "track `flute` is gone; index 1
  is now `Flute (lead)` — renamed in the DAW? Edit the sidecar's part" (the rename case);
- a `Name(..)` that became **ambiguous** (two tracks now share it) → `validate` refuses; the
  message says to use `Index`.

Every line goes to the console; the first goes to the status bar. The sidecar changing on its own
(`.mid.ron` saved by hand) takes the same path.

### 3.9 The DAW CLI contract (what the engine will call)

The engine spawns, never waits, never reads stdout. The DAW is launched as:

```
<daw> [--project <dir>] [--track <name>] <file.mid>
```

| element | meaning | required of the DAW |
|---|---|---|
| `<file.mid>` | absolute path of the Standard MIDI File (type 0 or 1) to open | open it; if an instance already shows it, **focus that instance** instead of opening a second (single-instance forwarding is the DAW's job; the engine cannot tell) |
| `--track <name>` | the MIDI track-name meta event (`FF 03`) of the track to focus/select; UTF-8; may contain spaces (one argv element) | select that track and scroll to it; unknown name → open the file and say so, exit 0 |
| `--project <dir>` | the game's asset base; where samples and SoundFonts the sidecar names resolve from | optional; a hint for file dialogs and relative paths |
| exit status | `0` once the request is accepted (the UI may still be loading), non-zero with one line on stderr otherwise | the engine logs stderr's first line in the console |
| saving | the DAW writes `.mid` **atomically** (temp file in the same directory + rename), so the engine's watcher never reads a half-written file; it **preserves track names** on save — names are the join key to the sidecar; it never writes `<file>.mid.ron` unless it understands the schema (`docs/SONG_FORMAT.md`) | hard requirements for hot reload to be trustworthy |
| not in the contract | bar position, loop range, instruments — the engine has nothing to say about them yet | — |

The engine side works **today** with "Custom command": a user writes `mydaw --track {track} {file}`
and the placeholders do the rest; the DAW preset is this exact template with `{daw}` as the
binary name the settings panel asks for.

### 3.10 Cross-platform rules

- **Spaces and quoting**: argv everywhere; `std::process::Command` quotes for `CreateProcess` on
  Windows itself. The one shell call is the Windows OS-default fallback (`cmd /C start "" <file>`),
  built with `CommandExt::raw_arg` so `start`'s title slot gets `""` and the path keeps its quotes.
- **Flatpak / Snap**: a Flatpak app is not on `PATH` as its binary name; the "not found" error
  suggests `flatpak run <app-id> …` as a Custom command (docs.flatpak.org: `flatpak run [OPTION...]
  REF [ARG...]`, "Extra arguments are passed on to the application"); the engine cannot see the
  sandbox's filesystem grants, so a Flatpak editor that opens nothing is noted in the docs
  (`--filesystem=home` or `host`). Snap binaries live in `/snap/bin`, normally on `PATH`; nothing
  special.
- **WSL**: when the resolved program ends in `.exe` on Linux, every path placeholder is converted
  with `wslpath -w` before substitution (Microsoft: "WSL can run Windows tools directly from the
  WSL command line using [tool-name].exe", e.g. `notepad.exe "C:\temp\foo.txt"`); the OS-default
  fallback under WSL (`/proc/version` contains "microsoft") is `cmd.exe /C start "" <wslpath -w>`.
- **macOS**: `open -a <App> <file>` is the "an app, not a CLI" escape hatch for the Custom field.
- **Missing binary**: § 3.5's own `PATH` lookup, one clear sentence, never a panic; a spawn error
  after a successful lookup (permissions, a broken launcher script) is reported with the OS text.
- **Web**: the whole module is `cfg(not(target_arch = "wasm32"))` except the types; the entry
  points draw disabled with today's hint. No web editor work beyond that gate.

### 3.11 Patterns

Command (an `OpenRequest` is data the host executes later, like every editor command);
Observer by polling (the watcher), chosen over OS notification because the engine's rule is no
new dependency until the simple thing has failed; the type table is a flyweight of behaviour per
file type, one place to read. Nothing is a service locator: the host owns the watcher, the config
rides the preferences, the audio manager owns the swap.

---

## 4. Adversarial self-review

| # | attack | verdict | revision |
|---|---|---|---|
| 1 | *A path with spaces splits the command.* Today's `ide_command` does exactly that. | real | tokenize first, substitute per token (§ 3.3); no quoting of `{file}` ever needed |
| 2 | *RustRover: `--line` with no line passes `--line` and nothing.* | real | optional groups `[--line {line}]`, the doc's own syntax |
| 3 | *Windows `start` eats the path as the window title.* | real | `start ""` via `raw_arg`; tested on a Windows machine before the issue closes (the manual step) |
| 4 | *A Flatpak editor opens, then shows "file not found" because the sandbox cannot see the path.* | real, undetectable by the engine | documented; the error hint names `flatpak run` and the filesystem grant |
| 5 | *WSL: `code.exe` gets a `/home/...` path.* | real | the `.exe` rule with `wslpath -w` (§ 3.10) |
| 6 | *A `.mid` saved mid-bar: the engine reads a half-written file, `midly` fails, the song dies.* | real | debounce to a stable stamp; a failed read keeps the old song (the loader's existing contract) and retries on the next poll; the DAW contract asks for atomic saves |
| 7 | *A track renamed in the DAW silences the enemy whose part names it.* | real | `validate` refuses the new song, the old one plays on, the diff names the likely rename (§ 3.8.2) |
| 8 | *Ticks-per-quarter differs after the DAW re-exports; a tick-preserving swap jumps bars.* | real | convert through bar + beat, never raw ticks (§ 3.8.1 step 1) |
| 9 | *Part indices differ after an edit; sources keyed by part index play the wrong part.* | real | re-resolve bindings by name, re-seat cursors (step 3) |
| 10 | *A lock on the audio thread.* | real concern | one `try_lock` per block, one pointer move; if the review prefers, the mailbox becomes a lock-free single slot (`AtomicPtr` or a `triple_buffer`-style pair) — same seam, decided in E11's plan review |
| 11 | *The watcher stats the whole `src/` tree every 500 ms.* | over-engineered as first drafted | `.rs` watching lists the tree every 30th poll and stats only the known files between; scripts, scene, song closure are a few dozen paths |
| 12 | *Auto-reloading the scene clobbers unsaved edits.* | real | dirty → prompt; mid-session → defer to Stop (§ 3.8) |
| 13 | *Reload behaviour as a user setting is three knobs nobody wants.* | over-engineered | policy is data in the type table; settings choose tools only |
| 14 | *A second settings file in RON.* | DRY/KISS | decision B |
| 15 | *A console panel is scope creep.* | partly | the brief asks for clickable `file:line` in "the console/errors" and the editor has no console; the dock reserves the id and the menu item exists disabled. Scope held to: a list of entries, a `Location` link per entry, Clear. No filters, no log levels UI, no search |
| 16 | *A context menu widget is new UI surface for one item.* | partly | three items (open, folder, DAW) and #137 asks for one in the hierarchy; the widget reuses the menu dropdown's overlay geometry. Kept, as its own issue so it can slip |
| 17 | *Double-click conflicts with drag-arm and click-select on the tile.* | real | a double-click fires only with no drag armed and the second press on the same tile; the first press still selects; the tile's own state, no `ui` change |
| 18 | *`$EDITOR=vim` spawns a process with no tty that exits at once; the user sees "Opened".* | real | the launcher reports exit status when the child exits within 500 ms with non-zero ("vim exited at once — a terminal editor needs a terminal; set a tool in Edit → External Tools…") |
| 19 | *Removing the composer deletes the agent's only song-authoring path and the only LICENSE writer.* | real | decision A2 |
| 20 | *Composer removal and the Conductor effort both edit `song_preview.rs` this month.* | real | removal is last and gated (decision A) |
| 21 | *Rhai hot reload resets a script's state mid-fight.* | would be real | keep blackboards and instance state; swap the AST only |
| 22 | *`.sheet.ron` reload while a clip is mid-frame indexes past the new frame count.* | real | `SpriteAnimation` already warns and holds on an unknown clip; a frame past the end clamps on the next tick — add the guard test in E8 |
| 23 | *The web build grows a `std::process` dependency.* | real | `cfg`-gated module; `scripts/check_wasm.sh` is in every batch gate |
| 24 | *Two requests in one frame (console click + menu) lose one.* | real | `open_requests: Vec` |
| 25 | *Audacity has no "jump to time", so the sample tool contract is just the file.* | fine | `{line}`/`{column}`/`{track}` unresolved → dropped; the template is `audacity {file}` |
| 26 | *The `.mid.ron` fingerprint goes stale after a DAW save and the file is refused.* | checked, not real | the audio loader never reads `midi_fingerprint`; only the composer's open path does, and it goes with the composer. E16 stops writing it |

---

## 5. Data flow, in one picture

```
 user gesture ──► OpenRequest{path, location} ──► EditorContext.open_requests
                                                        │ (once a frame, host)
                      FileType::of(path) ─► ToolsConfig::resolve ─► Launch{program,args}
                                                        │
                                     PATH lookup ─► spawn ─► reap thread ─► console + status bar

 disk ──► FileWatch::poll (500 ms, debounced) ──► changed paths ──► ReloadPolicy::of(FileType)
          │ RecompileScript ─► ScriptRunner::reload_source
          │ ReloadDocument  ─► load_scene | ConfirmDialog | defer to Stop
          │ ReloadSheet     ─► clear_sidecar_cache + reload sheet
          │ ReloadSong      ─► AudioManager::replace_song_at_seam ─► validate ─► track diff ─► mailbox ─► Conductor swaps at the bar
          │ ReloadSample    ─► AudioManager::reload_sound(handle) | the song path above
          └ RebuildNotice   ─► sticky status notice + console
```

---

## 6. The composer: remove or salvage

| piece | fate | why |
|---|---|---|
| `editor/src/composer/` (state, layout, keys, piano roll, note list, sidebar, sound_fonts, transport, drag model) and `editor_integration` `composer_keys.rs`, `composer_drag.rs`, `panel_renderer/composer.rs`, `PanelId::COMPOSER`, the View menu item, the composer's theme tokens, the drag payloads `Song`/`SoundFont`'s composer drop site | **remove** (E15) | the GUI the DAW replaces |
| `composer/sound_fonts.rs::scan_sound_fonts`, `ComposerState`'s Songs scan of `music/` and `ai/music/` | **salvage** into `editor/src/external_tools/songs.rs` (`scan_songs`) | § 3.7 needs it |
| `editor/src/song/` (`SongDocument`, history, `SongEdit::apply`, the save journal with its `.new` pair recovery, the sample folder + LICENSE stanzas, provenance) and `command_api/song.rs` | **keep** under decision A2's recommendation; trim what only the GUI used (`SongSelection`, the drag's `moved_notes_edit`, `clamp_move`) | the headless authoring and provenance path |
| `song_preview.rs` (the preview transport and the session's music), `init_music.rs`, the `song play/stop/seek/trigger` verbs | **keep**: the session-music half is how a Play session hears the game's song at all; the preview's Play/Stop stays as the API's audition | the Conductor effort owns this file until its close-out |
| `song_io.rs` (new/open/save/close verbs, the dirty-song prompts), `scene_confirm.rs`'s song variants, `snapshot.rs::live_files`, the playground's song persistence and `.wav`/`.mid` import, `history_dirty` carrying the song | **keep with A2's recommendation; remove under the alternative** | they exist for the open song document |
| `midi_fingerprint` | stop **writing** it; accepted on read | a DAW will never maintain it; "write new, refuse nothing" |
| `docs/SONG_FORMAT.md` § How the composer saves, `docs/EDITOR_COMMAND_API.md` § Songs, the games' guides' "open the song in the composer" sentences, `PROJECT_ROADMAP.md` Phase K row, #141 | **rewrite** (E18, after Jesse's roadmap line) | docs follow the ruling |

Open issues the removal touches: #200, #197, #188, #187, #186 (composer defects — close as
superseded when E15 lands), #183, #182 (song save files — stay if the document stays), #141.

---

## 7. Issues

Sizes: **S** ≤ half a day, **M** one to two days, **L** three or more, each with its tests and the
engine's gates (`cargo test --workspace`, `cargo clippy --workspace --all-targets`, the comment-tag
grep, `scripts/check_wasm.sh`; `scripts/check_games.sh` where `engine_core` or `audio` changes).
Labels: `editor`, `tooling`, `audio`, plus `size:S` / `size:M` / `size:L` (the repo has none of
these six; they are created with the milestone). Every issue body links its dependencies by
number.

| code | issue | code | issue | code | issue |
|---|---|---|---|---|---|
| E1 | #203 | E2 | #204 | E3 | #205 |
| E4 | #206 | E5 | #207 | E6 | #208 |
| E7 | #209 | E8 | #210 | E9 | #211 |
| E10 | #212 | E11 | #213 | E12 | #214 |
| E13 | #215 | E14 | #216 | E15 | #217 |
| E16 | #218 | E17 | #219 | E18 | #220 |

### Foundation

**E1 (#203) — `[editor][tooling] External tools: the file-type table and command templates`** · M
*Context:* § 3.1, § 3.3. The pure half, no process, no I/O.
*Files:* new `crates/editor/src/external_tools/{mod,file_type,template}.rs`, `editor/src/lib.rs`.
*Depends on:* nothing. *Gates:* E2.
*Acceptance:*
- [ ] `FileType::of(path)` classifies every extension in the table, `.sheet.ron` and `.mid.ron` by suffix before `.ron`, unknown → `None`
- [ ] each `FileType` answers `category()`, `reload_policy()`, `project_kind()` from one table
- [ ] `CommandTemplate::expand(&Placeholders) -> Result<Vec<String>>`: quotes and escapes, per-token substitution, nested optional groups dropped whole, unknown placeholder is an error naming it
- [ ] tests: a path with spaces survives whole; `rustrover [--line {line}] [--column {column}] {file}` with no line yields `[rustrover, file]`; `{file}[:{line}[:{column}]]` with line only yields `file:42`; `"C:\Program Files\x\code.exe"` tokenizes as one program
*Manual test:* none (pure); `cargo test -p editor`.

**E2 (#204) — `[editor][tooling] External tools config, presets and the resolution order`** · M
*Context:* § 3.2, § 3.4; decision B. Closes the data half of #102.
*Files:* `external_tools/{config,presets}.rs`, `editor/src/editor_preferences.rs` (new field, `#[serde(default)]`; `ide_command` read into `code: Custom(..)` when set, no longer written).
*Depends on:* E1. *Gates:* E3, E14.
*Acceptance:*
- [ ] `ExternalToolsConfig` round-trips through `EditorPreferences` JSON; an old prefs file loads with defaults; a prefs file with `ide_command` loads it as the Custom code command
- [ ] `resolve(file_type, env) -> Launch` follows override → category → `$VISUAL`/`$EDITOR` (Code only) → OS default, with a test per step and one for "Music never reads `$EDITOR`"
- [ ] the preset table carries the seven rows of § 3.4 with their source URLs in doc comments and a test that every preset template expands with and without a line
- [ ] the OS-default launch is per `cfg(target_os)` and the Windows one is `cmd /C start "" <file>`
*Manual test:* none (pure).

**E3 (#205) — `[editor_integration][tooling] The launcher: requests, PATH lookup, spawn, workspace root`** · M
*Context:* § 3.5. Replaces `open_source.rs`; the Scripts **Open Source** button keeps working through it (the one entry point that exists today).
*Files:* `editor_integration/src/editor_game/external_tools.rs` (new, `open_source.rs` deleted), `editor/src/context/mod.rs` (`pending_open_source` → `open_requests: Vec<OpenRequest>`), `script_editor.rs`, `panel_renderer/inspector.rs:254`, guide rows.
*Depends on:* E2. *Gates:* E4, E5, E10, E17.
*Acceptance:*
- [ ] every request is drained once a frame; two in one frame both launch
- [ ] `find_on_path(program)` honours `PATH` and Windows `PATHEXT`; a miss is one status-bar error naming the program and the settings action, never a panic; a spawn error after a hit reports the OS text
- [ ] the child is reaped (the existing thread); a child exiting non-zero within 500 ms is reported ("exited at once")
- [ ] `workspace_root()` picks `[workspace]` over a plain manifest, falls back to the project directory; tested on fixtures for a game, the engine, a data project
- [ ] wasm: the hint text, no `std::process` in the wasm build (`check_wasm.sh`)
- [ ] the headless tests of `open_source.rs` are carried over and pass against the new type
*Manual test:* `cargo run --bin editor --features editor -- ../games/pong`, select the paddle, click **Open Source** with no tool configured → the status bar names `$EDITOR` or the OS default; set `ide_command` in the prefs file to a nonsense program → the error names it and says where to set a tool.

### Code entry points

**E4 (#206) — `[editor] Double-click and a context menu open a script or asset in its tool`** · M
*Context:* § 3.6 rows 1, 2, 4. The editor's first context menu; #137 is the hierarchy's wish for one.
*Files:* `panel_renderer/asset_browser/tile.rs`, `editor/src/script_editor.rs`, `editor/src/hierarchy/`, new `editor/src/context_menu.rs` (overlay band, blocks input beneath like the menu dropdown), `editor_input.rs` (right press).
*Depends on:* E3. *Gates:* none.
*Acceptance:*
- [ ] a second press on the same tile within 400 ms and 4 px, with no drag armed, files an `OpenRequest`; the first press still selects; a drag is never confused for it (guard test)
- [ ] double-click on a Scripts row (inspector and hierarchy pseudo-row) opens the script's source
- [ ] right-press on a tile or Scripts row opens a context menu with **Open in external tool**, **Open containing folder** (the OS default on the directory), and, for `.mid` and `Binding` emitters, **Edit in DAW** (disabled until E10 lands); Escape and a press elsewhere close it; it draws on the floating band and blocks the tile beneath (the menu dropdown's guard test pattern)
- [ ] keyboard: the context menu opens on the menu key / Shift+F10 on the focused tile, its items walk with the arrows — no mouse-only path
*Manual test:* in the editor on pong, double-click `scripts/paddle.rhai` in Assets → it opens in the configured tool; right-click → the menu; Shift+F10 → the same menu.

**E5 (#207) — `[editor] A Console panel whose file:line entries open the file at the line`** · M
*Context:* § 3.6 row 5; § 1 "Console". The panel the dock reserves.
*Files:* new `editor/src/console.rs` (`ConsoleLog`: bounded `Vec<ConsoleEntry { when, text, location: Location, severity }>`), new `panel_renderer/console.rs`, `dock` (the panel, hidden by default, bottom position beside Assets), `menu/mod.rs:225` (enable the item), `script_status.rs` (errors also to the console), `status_bar.rs` (a click on an error with a location files a request).
*Depends on:* E3. *Gates:* E6 (the reload notices need a home).
*Acceptance:*
- [ ] every `ScriptError` of a session lands as an entry with `Location::Line`; every launch result and every reload notice lands as an entry
- [ ] an entry with a location draws underlined and files an `OpenRequest` on click; Enter on the focused entry does the same (keyboard path)
- [ ] the log is bounded (last 500 entries), has a Clear button, scrolls with the shared `ScrollState`; an error entry carries the "Error: " prefix and the status bar's amber (the a11y rule of #195)
- [ ] the status bar's current error, when it has a location, is clickable and says so in its tooltip
- [ ] the panel starts hidden; an old prefs file leaves it hidden (the composer's guard-test pattern)
*Manual test:* Play pong with a deliberate syntax error in `paddle.rhai`; the console shows `scripts/paddle.rhai:12: [Syntax] …`; click it → the editor opens at line 12.

### Code hot reload

**E6 (#208) — `[editor][editor_integration] A polling file watcher over the project's live files`** · M
*Context:* § 3.8 "FileWatch". No new dependency.
*Files:* new `editor/src/external_tools/watch.rs` (pure: set, stamps, debounce, `poll`), new `editor_integration/src/editor_game/hot_reload.rs` (the 500 ms timer in `finish_frame`, the registered set's rebuild rules, dispatch stub that only logs to the console), `scene_io.rs` (rebuild after load), `panel_renderer/mod.rs` (after a `Scripts` edit).
*Depends on:* E5. *Gates:* E7, E8, E9, E12, E13.
*Acceptance:*
- [ ] `FileWatch::poll` reports a path once its stamp changed and then held for one more poll; a path changing every poll is reported when it settles; a deleted path is reported once and dropped
- [ ] the set is rebuilt from the scene path, every `Scripts.source_path`, the song closure, every `SoundEmitter(Path)`, every referenced `.sheet.ron`; a test with a temp dir proves a touched script is reported and an unrelated file is not
- [ ] wasm: the watcher never runs (`change_stamp` is `None`); native cost: at most one `metadata` per registered path per 500 ms
- [ ] each change lands in the console as "`<path>` changed on disk" with its policy name
*Manual test:* open pong in the editor, `touch assets/scripts/paddle.rhai` from a shell → within a second the console says so.

**E7 (#209) — `[engine_core] Rhai scripts recompile on save during a Play session`** · M
*Context:* § 3.8 `RecompileScript`.
*Files:* `engine_core/src/scripting/runner.rs` (`reload_source(path) -> Result<(), ScriptError>`), `rhai_backend.rs` (recompile and replace one entry), `hot_reload.rs` (dispatch), `docs/SCRIPTING.md`.
*Depends on:* E6. *Gates:* none.
*Acceptance:*
- [ ] a reload swaps the compiled AST for every instance of that `source_path`; blackboards, params and instance state survive; `frames_run` is untouched
- [ ] a syntax error on reload keeps the old AST running and records a `ScriptError` with the line; a later good save clears the file's `failed` and quarantine entries so the instance runs again
- [ ] while Editing a change is a console line only; a session that starts after it compiles the new source (already true via `reset`; a guard test pins it)
- [ ] `GameHarness` test: change a script's constant on disk, call `reload_source`, step one frame, the entity's velocity reflects the new value
*Manual test:* Play pong, change `speed` in `paddle.rhai` and save → the paddle moves faster within a second, no Stop; break the syntax → the old speed holds and the console shows the error as a link.

**E8 (#210) — `[editor_integration] Scene and sheet RON reload from disk`** · M
*Context:* § 3.8 `ReloadDocument`, `ReloadSheet`; attack 12 and 22.
*Files:* `hot_reload.rs`, `scene_io.rs`, `scene_confirm.rs` (a `ReloadFromDisk` pending action with the existing dialog), `engine_core` `assets/sprite_sheet.rs` (reload one sheet by path).
*Depends on:* E6. *Gates:* none.
*Acceptance:*
- [ ] clean history + Editing → `load_scene` from disk; the scene's own guard (dry-run into a scratch world) keeps a bad file from touching the live world; status "reloaded from disk"
- [ ] dirty history → the dialog **Reload / Keep mine**; Keep leaves the file watched and the notice in the console
- [ ] Play or Paused → nothing now, the rule applied at Stop (guard test, the "mid-simulation" pitfall)
- [ ] a `.sheet.ron` change reloads that sheet's clips and filter for every `SpriteAnimation` naming it in Editing and in Play; a clip index past the new frame count clamps and warns (guard test)
*Manual test:* with pong open and clean, edit the scene file's paddle position in a text editor and save → the paddle moves in the viewport; make an edit in the editor first → the dialog appears instead.

**E9 (#211) — `[editor_integration] A "rebuild needed" notice when Rust source changes`** · S
*Context:* § 3.8 `RebuildNotice`; the decision of record forbids build work now.
*Files:* `hot_reload.rs`, `watch.rs` (the tree listing every 30th poll), `status_bar.rs` (a sticky notice kind), docs.
*Depends on:* E6. *Gates:* none.
*Acceptance:*
- [ ] `.rs` files under `<workspace root>/src/` and `crates/*/src/` are watched (never `target/`); a new file is noticed within 15 s
- [ ] the notice "Rust source changed (N files) — rebuild and relaunch to see it" is sticky until the process exits; each file also logs to the console
- [ ] no `cargo` is spawned; a follow-up issue names "spawn cargo, relaunch self" as the next step and cites the decision of record
*Manual test:* with the editor running on pong, save `games/pong/src/main.rs` → the notice appears and stays.

### Music entry points

**E10 (#212) — `[editor][audio] Edit in DAW: from an entity's sound settings, a .mid tile, and a sample`** · M
*Context:* § 3.6 rows 1 and 6, § 3.7, decision E.
*Files:* `editor/src/component_editors/sound_emitter.rs` (the **Edit in DAW** button for a `Binding`), new `editor/src/external_tools/songs.rs` (`scan_songs`, salvaged; the binding → part → track resolution over a `SongFile` + `SongData`), `hot_reload.rs`/`external_tools.rs` (the song's path: `InitMusic` first), `tile.rs` (`.mid` double-click with the first part's track; sample tiles → the Sample tool).
*Depends on:* E3, E4. *Gates:* E12 (shares `scan_songs`).
*Acceptance:*
- [ ] a `SoundEmitter { Binding(action) }` shows **Edit in DAW**; it resolves action → part → track name through the project's song and files `OpenRequest { .mid, Track(name) }`; a nameless `Index` track opens unfocused with a notice; an unknown action is a status error naming the sidecar
- [ ] the project's song is the session's or the `init`-time request, else the one `.mid` under `music/` or `ai/music/`; several → a chooser row; none → an error
- [ ] a `.mid` tile double-click opens with the Music tool; a `.wav`/`.ogg` tile with the Sample tool; the context menu's **Edit in DAW** enables
- [ ] a part selecting by `Index` resolves to the track's name when it has one (test over the Napolitan fixture: `Index(2)` → `tambourin`)
*Manual test:* pong, select the ball, its emitter is `Binding("hit")`: click **Edit in DAW** with the Music tool set to `mydaw --track {track} {file}` (or `echo` as a Custom command to see the argv in the console) → the DAW opens `tong.mid` on the hit track.

### MIDI and sample hot reload

**E11 (#213) — `[music_midi][audio] Replace the playing song at the next bar without stopping`** · L
*Context:* § 3.8.1; attacks 8, 9, 10.
*Files:* `music_midi/src/conductor.rs` (+ a `conductor/replace.rs`: the seam test, bar+beat conversion, `rebuild_keeping_sources`), `commands.rs`/`position.rs` (`SongReplaced` note), `audio/src/manager/song.rs` (`replace_song_at_seam(path)`, `replace_song_in_memory_at_seam`), `conductor_source.rs` (the mailbox read once a block), tests in both crates.
*Depends on:* nothing in this sprint (the audio seam stands alone); **must wait for the Conductor effort's close-out** (decision A). *Gates:* E12.
*Acceptance:*
- [ ] a pending song is taken at the first block containing a bar line of the **old** map; the new song starts at the tick the old bar+beat maps to in the new map (test: ticks-per-quarter 480 → 960 keeps the bar)
- [ ] play state, looping flag and the `SourceTable` (activations, epochs, flush floor) survive; bindings re-resolve by name; a part cursor re-seats at the unit at or after the position; a binding the new song lacks is dropped and reported
- [ ] a second pending song before the seam replaces the first; a song that fails `prepare`/`validate` never reaches the mailbox and the playing song is untouched (existing contract, pinned)
- [ ] the audio thread never blocks: `try_lock` once a block; a contended lock defers to the next block (test with a held lock)
- [ ] `conductor_background` example: a `--replace-after <bars>` flag swaps to a second fixture mid-play, logging the swap block and the bar
*Manual test (by ear):* run Tong with the editor, Play, let the song run; in the DAW change one note of the melody and save mid-bar → the change is heard from the next bar, the paddle hits stay on the beat before and after; a note sounding across the bar is cut at the bar (expected, documented).

**E12 (#214) — `[audio][editor_integration] MIDI reload: manifest validation and the track diff`** · M
*Context:* § 3.8.2; attacks 6, 7.
*Files:* `hot_reload.rs` (`ReloadSong` dispatch: debounce → prepare → diff → replace), new `editor/src/external_tools/track_diff.rs` (pure over two track-name lists: missing, new, likely-renamed by index, ambiguous), console wiring.
*Depends on:* E6, E10, E11. *Gates:* none.
*Acceptance:*
- [ ] a `.mid`, `.mid.ron`, sample or SoundFont change in the playing song's closure triggers one prepare (the cache re-decodes only the changed file — pinned by `decodes()`)
- [ ] a refusal (`validate`) shows the error, keeps the old song, and the console adds the diff's "likely renamed" line when it applies
- [ ] a new unselected track → warning "plays as background"; a `Name` now ambiguous → the validate error and the `Index` hint
- [ ] a half-written file (test: write the first half of a `.mid`) is never read before the stamp settles; a read that fails anyway keeps the old song and retries on the next poll
- [ ] while Editing (nothing plays): the diff still runs against the last loaded song and reports to the console, so a rename is caught before Play
*Manual test:* in the DAW rename the `flute` track and save → the status bar says the part's track is gone and the console suggests the rename; rename it back → the song swaps at the bar.

**E13 (#215) — `[engine_core][audio] Recorded sounds reload in place behind their handle`** · M
*Context:* § 3.8 `ReloadSample`; the sound half of #180.
*Files:* `audio/src/manager/mod.rs` (`reload_sound(handle, bytes)`), `engine_core/src/assets/sound_library.rs` (path → handle lookup, `reload(path)`), `hot_reload.rs`.
*Depends on:* E6. *Gates:* none.
*Acceptance:*
- [ ] a `.wav`/`.ogg` named by a `SoundEmitter(Path)` that changes on disk is decoded again and the same `SoundHandle` serves the new bytes; `resolve_sound`'s cache stays valid; a failed decode keeps the old bytes and reports
- [ ] a looping instance already playing finishes on the old bytes; the next `play` has the new (test with the headless manager)
- [ ] a song sample is **not** handled here — it rides E12 (a guard test proves a sample in the song's closure dispatches to `ReloadSong`)
- [ ] #180 is updated: the sound half closed by this issue, the texture half still open
*Manual test (by ear):* Play pong, replace `beep.wav` with a different beep in Audacity and save → the next paddle hit plays the new sound without Stop.

### Settings UI

**E14 (#216) — `[editor] A settings panel for external tools`** · M
*Context:* § 3.2; closes #102.
*Files:* new `editor/src/settings_panel.rs` (the walk, over `EditableInspector`: a cycle row per category for the preset, a string field for the Custom template, a rows block of per-type overrides, the DAW binary field, the DAW project extensions, a **Test** button per category), `panel_renderer/settings.rs`, `dock` (a right-side panel, hidden by default), `menu` (Edit → External Tools… shows it), the preferences settle rule carries the edits.
*Depends on:* E2, E3. *Gates:* none.
*Acceptance:*
- [ ] every field writes `ExternalToolsConfig` and is persisted by the existing settle rule within half a second; the panel reads back after restart
- [ ] a template with an unknown placeholder shows the error inline (red invalid state, the numeric fields' existing pattern) and is not saved over the last good one
- [ ] **Test** launches the category's tool on a fixture file (`{file}` = the panel's own temp file, `{line}` = 1) and reports in the console; a missing binary is the launcher's one-sentence error
- [ ] keyboard-walkable like the composer's sidebar (Tab/arrows, visible focus ring), labelled controls, no colour-only state (WCAG 2.2 AA bar); narrow mode shows it as the dock's overlay
- [ ] #102 closes with this issue
*Manual test:* Edit → External Tools…, choose RustRover for Code, Test → RustRover opens the fixture at line 1; switch to Custom `echo {project} {file}:{line}` → the console shows the expanded argv.

### Composer removal (gated — decision A)

**E15 (#217) — `[editor][editor_integration] Retire the composer panel`** · L
*Context:* § 6 rows 1–2. **Gated on** Jesse's roadmap line, #141's disposition, and the Conductor effort's close-out.
*Files:* delete `editor/src/composer/`, `composer_keys.rs`, `composer_drag.rs`, `panel_renderer/composer.rs`, their tests; `PanelId::COMPOSER`, the View item, the composer theme tokens and their guard tests, `DragPayload::Song/SoundFont`'s composer drop; salvage `scan_sound_fonts` and the Songs scan into `external_tools/songs.rs` (E10 may have done it already); the `editor` and `editor_integration` guides' composer rows and pitfall-table entries; `scripts/check_guide_tests.sh` green.
*Depends on:* E10 (the salvage). *Gates:* E16, E18.
*Acceptance:*
- [ ] no file under `crates/` names the composer; `cargo test --workspace`, both clippy runs, `check_wasm.sh`, `check_games.sh`, `check_guide_tests.sh` green
- [ ] the open song's keyboard focus, drag and dialogs are gone; Ctrl+S saves the scene, Undo undoes the scene, with no "focused document" rule left
- [ ] #200, #197, #188, #187, #186 closed as superseded with a comment naming this issue
- [ ] `log_archive.md` gets the composer's dated entry: what it was, why it went, what was salvaged
*Manual test:* the editor on pong boots with four panels, no Composer in View; a `.mid` tile still double-clicks into the DAW.

**E16 (#218) — `[editor][playground] Trim the song document to the headless path, or remove it`** · L (trim: M)
*Context:* § 6 rows 3–5, decision A2. Written for the recommendation (trim); the alternative deletes `editor/src/song/`, `command_api/song.rs`, `song_io.rs`, the song half of `song_preview.rs`, `scene_confirm.rs`'s song variants, `snapshot.rs::live_files`, the playground's song persistence and import, and the `history_dirty` song carry.
*Files (trim):* `song/selection.rs` and the GUI-only helpers deleted; `write_song_file` stops emitting `midi_fingerprint` (accepted on read); `song_io.rs`'s GUI prompts collapse to the API's `discard`/`overwrite` words; `docs/EDITOR_COMMAND_API.md` § Songs says the verbs are the authoring path.
*Depends on:* E15. *Gates:* E18.
*Acceptance:*
- [ ] `song new|edit|save|open|close|play|stop|seek|trigger` still answer over `--headless` (`api_tests.rs` green); a saved song still passes `scripts/check_audio_provenance.sh`
- [ ] `midi_fingerprint` is never written; a sidecar carrying one still loads
- [ ] the playground's export still carries the open song's two files (the bridge contract the site reads), or — under the alternative — the site's import of `.mid`/`.wav` is removed in `insiculous_web` by its own issue, filed from here
- [ ] no composer-only dialog remains reachable
*Manual test:* `cargo run --bin editor --features editor -- ../games/pong --headless`, send `song open ai/music/ai_tong.mid`, `song notes melody`, `song save` → answers as documented; `git status` shows only the `.mid.ron` rewritten without its fingerprint.

### Polish and docs

**E17 (#219) — `[editor_integration][tooling] Platform polish: WSL paths, Flatpak hint, the three OS-default fallbacks`** · S
*Context:* § 3.10; attacks 3, 4, 5.
*Files:* `external_tools.rs` (the `.exe`-on-Linux rule with `wslpath -w`; `/proc/version` detection; the "not found" hint's Flatpak sentence), the Windows `raw_arg` fallback verified on a Windows machine.
*Depends on:* E3. *Gates:* none.
*Acceptance:*
- [ ] under WSL a program ending in `.exe` receives Windows paths for every path placeholder (test: a fake `wslpath` on `PATH`)
- [ ] the not-found error mentions `flatpak run <app-id> …` when `/var/lib/flatpak` or `~/.local/share/flatpak` exists
- [ ] the OS-default fallback is exercised by hand on Linux (`xdg-open`), macOS (`open`) and Windows (`cmd /C start ""`) with a path containing spaces, and the results are recorded in the issue before it closes
*Manual test:* on Windows, a Custom command of `notepad {file}` on a path with spaces opens the file; the OS default on `my file.ron` opens its associated app.

**E18 (#220) — `[docs][tooling] docs/EXTERNAL_TOOLS.md, the DAW CLI contract, and the guides`** · S
*Context:* § 3.9, § 6 last row.
*Files:* new `docs/EXTERNAL_TOOLS.md` (presets with their verified sources and dates, the template grammar, the resolution order, the reload table, the DAW contract, the platform notes), `CLAUDE.md` (SSOT row: the file-type table), `training.md` (directory map), the two editor guides, `docs/SCRIPTING.md` (reload), `docs/SONG_FORMAT.md` (the fingerprint is read-only; the DAW's save rules), the games' guides' composer sentences, `PROJECT_ROADMAP.md` Phase K row after Jesse's line.
*Depends on:* E15, E16 (so the docs describe what shipped). *Gates:* none; closes the sprint.
*Acceptance:*
- [ ] every preset row cites its page and the verification date; the DAW contract is the § 3.9 table verbatim
- [ ] no doc under `docs/` or a crate guide tells a reader to open the composer
- [ ] `scripts/check_guide_tests.sh` green after the guide edits
*Manual test:* a reader with no context sets up VS Code from `docs/EXTERNAL_TOOLS.md` alone and opens a script at a line.

---

## 8. Sprint order and gates

Milestone **"Sprint: External Tools"** (`beinsiculous/insiculous_2d`), description to record:

> From the editor, open a project file in the right external app at the right place — a line for
> code, a track for MIDI — and hot-reload on save; the in-engine composer retires in favour of
> the DAW. ORDER: E1 → E2 → E3 (foundation; E3 keeps the Open Source button working); then E4,
> E5 (code entry points) and E6 (the watcher, after E5 for its console) in parallel; E7, E8, E9
> after E6; E10 after E4; E11 (the bar seam) independent of the editor work but AFTER the
> Conductor effort's close-out; E12 after E10 and E11; E13 after E6; E14 after E3; E15 → E16 →
> E18 last, E15 gated on Jesse's roadmap line reversing the 2026-09-25 K8 ruling and #141's
> disposition; E17 any time after E3. External gates: the Conductor effort (batch 15, close-out)
> before E11 and E15; a Windows and a macOS machine for E17's fallbacks. Excluded: any build
> system work (the decision of record; E9 is a notice), web editor work beyond the cfg gate,
> DAW features (its own repo, § 9), a `notify`-style dependency.

Board fields: Phase `Editor`, Priority `P1 next` for E1–E3, `P2 soon` for the rest; Sprint
option "Sprint: External Tools" added **with every existing option's id passed back** (the
sprint-planning skill's wiping pitfall); `scripts/check-sprint-sync.sh` exit 0 before reporting.

---

## 9. The DAW repo's issues (not filed here — the contract's other half)

1. **CLI: open a Standard MIDI File given as a positional absolute path** (type 0 and 1); exit 0
   once accepted, non-zero with one stderr line otherwise.
2. **`--track <name>`: select and scroll to the track whose name meta event matches**, exact
   match, UTF-8, spaces allowed; unknown name → open anyway, say so, exit 0.
3. **Single instance: a second invocation on an open file focuses the running window** and
   applies `--track`, instead of opening a second instance.
4. **`--project <dir>`: a hint for relative sample/SoundFont paths and file dialogs**; optional.
5. **Atomic save**: write to a temp file in the same directory and rename, so a watcher never
   reads a half-written `.mid`.
6. **Preserve track names on save** (the join key to the engine's sidecar); never rename a track
   implicitly on export.
7. **Never write `<file>.mid.ron`** unless the schema in `insiculous_2d/docs/SONG_FORMAT.md` is
   implemented; document which fields, if so.
8. **A test fixture pair** the engine's E12 can use: a `.mid` and the same file with one track
   renamed, checked into the DAW repo's fixtures and mirrored into
   `crates/music_midi/fixtures/` with a LICENSE line.

---

## 10. Out of scope, said plainly

No embedded editor of any kind; no web editor work beyond keeping the gate green; no DAW code in
this repo; no `cargo` spawn (E9's follow-up names it); no texture hot reload (#180's other half
stays open); no `notify`/inotify dependency until polling has failed in practice; no locale or
`GameConfig` RON reload.
