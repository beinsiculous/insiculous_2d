# External Tools — opening project files in the programs you use

The editor edits scenes and maps songs. Everything else a project holds is written in
another program: scripts in a code editor, scores in MIDI Evil, samples in a sound editor,
art in Aseprite. **External Tools** is how the editor hands a file to that program and how
it notices when the program saves it back.

## What it is

**Edit → External Tools…** opens the External Tools panel (it docks on the right, beside
the Inspector). The panel holds:

- **Four categories, a tool each.** Every file type belongs to one category: **Code**,
  **Music**, **Sample** or **Art**. Each category's **Tool** row cycles through the system
  default, the presets made for that category, and a custom command.
- **An override per file type**, under **Overrides**. An override row starts at
  *Inherit (the category's tool)*; any other value opens that one type with a tool of its
  own — Rust sources in one code editor and scenes in another, say.
- **The system default when nothing is set.** Every category starts at *System default*:
  the operating system's own association for the file (§ Platform notes). The editor never
  reads `$VISUAL` or `$EDITOR` — they almost always name a terminal editor, which cannot
  run without a terminal around it.
- **A Test button per category**, which opens the project's first file of that kind with
  the category's tool and prints the exact command it ran in the console (for a Windows
  program under WSL, the command on the Linux paths with a note that each is translated —
  § Platform notes, *WSL*).
- **DAW project extensions**: the extensions a music workstation's own project files carry
  (`.evil .prj .nsp` by default), so their tiles in the asset browser open with the Music tool.

Every change is written to the editor's preferences (`editor_prefs.json`) once it settles.
The browser playground cannot start another program: there the panel's Test buttons and
every open action are disabled (§ Platform notes, *The web*).

## Which file opens where, and what reloads on save

One table decides it, `crates/editor/src/external_tools/file_type.rs`: a file's extension
gives its type, the type gives the category whose tool opens it, the folder `{project}`
names for it, and what the editor does when the file changes on disk. The reload is not a
setting: you pick the tools, the engine knows what a file means.

| type | extensions | category | `{project}` | when it changes on disk |
|---|---|---|---|---|
| Rust source | `.rs` | Code | the Cargo workspace | nothing: a Rust change needs a rebuild, which is yours to run |
| Rhai script | `.rhai` | Code | the Cargo workspace | recompile the script |
| RON file | `.ron` — a scene, a `.sheet.ron`, a song's `.mid.ron` | Code | the Cargo workspace | by its suffix: reload the scene; reload the sheet; reload the song |
| MIDI file | `.mid`, `.midi` (any case) | Music | the asset folder | reload the song |
| Audio sample | `.wav`, `.ogg`, `.flac`, `.mp3` | Sample | the asset folder | reload the sample; reload the playing song when it names the sample |
| DAW project | the DAW project extensions (`.evil`, `.prj`, `.nsp` by default) | Music | the asset folder | nothing: a workstation's project is not an asset |
| SoundFont | `.sf2` | Music | the asset folder | reload the playing song when it names the bank |
| Image | `.png`, `.jpg`, `.jpeg`, `.bmp`, `.gif` | Art | the asset folder | nothing here; the texture reload and `.aseprite` files are the Aseprite work's (`coordination/aseprite-native/plan.md`) |

A file of any other type has no tool: a request for one says "No external tool opens <file>:
its type is unknown". A song's sidecar is the MIDI file's whole name with `.ron` added
(`theme.midi` pairs with `theme.midi.ron`). § Hot reload says what each reload does.

## The resolution order

When a file is opened, the editor picks the tool in this order:

1. **The file type's override, if one is present** — whatever it says. An override set to
   *System default* is honoured even when its category has a tool; only *Inherit* removes it.
2. **The category's tool.**
3. **The system default**, when the category is left at *System default*.

Three requests read the order differently:

- **Edit → Open Project** opens the project rather than a file, through the **Code
  category's** tool alone — an override says how a *file* opens, and a project is none. It
  runs a preset's project command or a custom tool's Project command. The system default
  has none, so the menu item reads "Open Project with the system default" and is disabled;
  a custom tool without a Project command answers "The custom command has no command for
  opening a project — set a code tool in Edit → External Tools…".
- **Open containing folder** always goes to the system default's opener, whatever the
  category says.
- **Test** tries the category's own tool, past any override on the file it opens.

The paths a command receives:

- **`{file}` is always absolute**: the file's path under the asset folder, made absolute.
- **`{project}`** for a Code file is **the Cargo workspace**: walking up from the asset
  folder, the nearest `Cargo.toml` that declares a `[workspace]`, else the nearest
  `Cargo.toml`, else the asset folder's parent. For `../games/pong` that is the game's own
  folder; for the engine's `examples/assets` it is the engine; for a data project with no
  manifest it is the project folder. For a Music, Sample or Art file `{project}` is the
  asset folder, where a song's samples and SoundFonts resolve.

## The presets

Each preset is a row of `crates/editor/src/external_tools/presets.rs`, its source page in
the row's doc comment, so the next check is a diff against that page. Every row was
verified against its source on 2026-10-26.

| preset | program | file command | project command | source | verified |
|---|---|---|---|---|---|
| RustRover | `rustrover` — JetBrains Toolbox's launcher script; none is installed by default | `rustrover [--line {line}] [--column {column}] {file}` | `rustrover {project}` | jetbrains.com/help/rust/opening-files-from-command-line.html | 2026-10-26 |
| VS Code | `code` — on macOS, installed by VS Code's *Shell Command* (§ Set up VS Code) | `code {project} --goto {file}[:{line}[:{column}]]` | `code {project}` | code.visualstudio.com/docs/configure/command-line | 2026-10-26 |
| VSCodium | `codium` | `codium {project} --goto {file}[:{line}[:{column}]]` | `codium {project}` | github.com/VSCodium/vscodium/blob/master/docs/usage.md (the flags are VS Code's, which that page does not state) | 2026-10-26 |
| Zed | `zed` — on Linux commonly `zed` or `zeditor` | `zed {project} {file}[:{line}[:{column}]]` | `zed {project}` | zed.dev/docs/reference/cli.md | 2026-10-26 |
| Helix | `hx` — a terminal editor | `hx [-w {project}] {file}[:{line}[:{column}]]` | `hx -w {project}` | helix-term's help text: `file[:row[:col]]`, `-w, --working-dir` | 2026-10-26 |
| MIDI Evil | `midi-evil` | `midi-evil [--project {project}] [--track {track}] [--track-index {track_index}] {file}` | — | the contract this engine defines (§ MIDI Evil, the contract across the crossing) | 2026-10-26 |
| Aseprite | `aseprite`; after `PATH`, the Steam install `~/.steam/root/steamapps/common/Aseprite/aseprite`; Flatpak `org.aseprite.Aseprite` | `aseprite {file}` | — | aseprite.org/docs/cli — `--frame-range` and `--tag` act only in batch mode, so no location is passed | 2026-10-26 |
| Audacity | `audacity`; Flatpak `org.audacityteam.Audacity` | `audacity {file}` | — | `AudacityApp::ParseCommandLine` in Audacity 3.7.8: a file name and no time position | 2026-10-26 |

RustRover, VS Code, VSCodium, Zed and Helix are Code presets; MIDI Evil is Music; Audacity
is Sample; Aseprite is Art. Only a Code preset has a project command.

**The Program field.** Under a preset, the **Program** field names the program to run in
the preset's place — a path (`/opt/midi-evil/target/release/midi-evil`) or another name
(`zeditor`). Blank runs the preset's own. The field is one value, never a command: a path
with spaces is typed bare, without quotes. A program named in its place:

- **runs, or is reported not found — never replaced.** The preset's fallback locations
  (Aseprite's Steam install) and its Flatpak hint belong to its own binary, so they are not
  tried or offered for another one.
- **keeps the preset's command line** and the preset's way of wording an early exit.

Typing the preset's own name there is the preset's own program, fallbacks and all — in any
case, with or without a Windows extension (`exe`, `com`, `bat`, `cmd`): `Aseprite.exe` is
`aseprite`, while `aseprite.beta` is another program.

**Where a program is looked for.** A program naming a path is checked as that file and
nothing else. A bare name is looked for on the `PATH` the editor was started with (on
Windows with each `PATHEXT` extension, so `code` finds `code.cmd`), then at the preset's
fallback locations, `~/` being your home folder. A miss is one sentence on the status bar
and in the console — "rustrover: not found on PATH (searched 11 directories) — set the tool
in Edit → External Tools…" — and, for a preset with a Flatpak id, the sentence ends "(a
Flatpak install runs as the Custom command `flatpak run org.aseprite.Aseprite {file}`)".

**A program that exits at once.** The editor never waits for a tool: it starts it with no
shell, its output discarded and its error output read in the background. A tool that exits
with a failure within five seconds is reported with the first line it wrote to its error
output; one closed later is not news. A custom command, or a preset marked as a terminal
program (Helix), that fails within half a second reads "hx exited at once — a terminal
editor needs a terminal; set the tool in Edit → External Tools…", since a terminal editor
started without a terminal around it dies that way. Give such an editor a terminal with a
custom command — `gnome-terminal -- hx {file}[:{line}[:{column}]]`, say. Any other preset
that fails is told plainly: "zed failed (exit status: 2) — <its first error line>".

## Writing a command

A **custom command** is the *Custom command* value of any Tool or override row. Its
**Command** field takes a command template; the Code category's custom tool also has a
**Project command** for Edit → Open Project, blank meaning it has none.

A template is a sequence of spans — bare or quoted text, a `{placeholder}`, or an optional
group `[ … ]` holding spans of its own, nesting allowed:

- **Whitespace outside quotes separates arguments.** The program is the first argument,
  and nothing passes through a shell: no `|`, no `$VARIABLE`, no globbing.
- **A placeholder's value is substituted whole and never split again**, so `{file}` needs
  no quoting even when the path holds spaces.
- **Double quotes** keep whitespace and brackets literal while a placeholder inside still
  expands (`"--file={file}"`); **single quotes** keep everything literal; a backslash takes
  the next character literally.
- **A group whose own placeholders all have values expands to its spans; one missing any
  value expands to nothing.** A nested group drops on its own, so `[:{line}[:{column}]]`
  keeps the line when only the column is missing.

The six placeholders:

| placeholder | value |
|---|---|
| `{file}` | the file's absolute path; always present |
| `{project}` | the project folder (§ The resolution order) |
| `{line}`, `{column}` | where to open a script or a source file, when the request names a place (a console link, a Test) |
| `{track}` | the name of the song track to select (the track's name event), when it has one |
| `{track_index}` | that track's 0-based index in the file's track order |

Three worked examples:

- **`[--line {line}]`** — a bare group: with a line it is two arguments, `--line` and `42`;
  without one it is nothing. RustRover's `rustrover [--line {line}] [--column {column}] {file}`
  opens `rustrover /game/src/main.rs` when no line is known.
- **`{file}[:{line}[:{column}]]`** — a group written against a word joins that word:
  `/game/src/main.rs:42:10` with a line and a column, `/game/src/main.rs:42` with a line
  alone, the bare path with neither. This is the `--goto` shape VS Code and Zed read.
- **A quoted program path** — `"/opt/My Tools/bin/editor" --new-window {file}` runs
  `/opt/My Tools/bin/editor` as one program; `'C:\Program Files\Tool\tool.exe' {file}`
  keeps its backslashes inside single quotes.

**What is refused when you type it.** The panel checks a command when you press Enter or
leave the field: an unknown placeholder (`{path}`), a `{`, a `[` or a quote never closed, a
`]` that closes no group, and a Command that never names `{file}` ("the command never names
{file}, so the file would not reach the tool"). A refused command is never written over the
last good one: the text stays in the field as a draft, and a row under it says in words why,
"Error: " first. Escape in the field drops the draft and shows the saved command. One thing
is refused only when a file is opened: a placeholder outside any group that this request
has no value for — `--line {line}` when no line is known — which a group (`[--line {line}]`)
avoids.

## Where you open things from

- **The asset browser.** Double-click a tile — a second press within 400 ms and 4 px of the
  first, with no drag started between them. From the keyboard: a press in the Assets panel
  gives it the keyboard, the arrows move the selected tile, **Enter** opens it, the menu key
  or **Shift+F10** opens its context menu, Escape lets the keyboard go.
- **The context menu.** A right press on a tile opens a menu of two items: **Open in
  <tool>** — named for the tool the file resolves to: "Open in Aseprite", "Open with the
  custom command", "Open with the system default" — and **Open containing folder**. On the
  web both are disabled.
- **The hierarchy's Scripts rows.** Double-click a script's row under its entity to open
  its source file; a right press opens the same context menu over it. A native script with
  no source file says so on the status bar.
- **The inspector's Open Source button**, on a `Scripts` component's script.
- **The console's links.** A script error that names its file and line — `scripts/paddle.rhai:3:
  …` — is drawn underlined; a click on it, or Enter on it while the console holds the
  keyboard, opens the file at that line. The console is hidden until **View → Console**
  shows it, and it keeps every script error, launch result and reload line.
- **The status bar's error**, when it names a file: a click on it opens the file at its
  line, and resting the pointer on it says what the click opens.
- **Edit → Open Project in <tool>** (§ The resolution order).
- **Edit in MIDI Evil**, on a `SoundEmitter` whose sound is a song's binding. The button is
  named for the Music tool ("Edit in MIDI Evil", "Edit with the system default") and opens
  the song at the track the binding's part plays — by the track's name and its index, so the
  second of two tracks named `drums` is the one opened.

**A song opens at a track.** A `.mid` opened from its tile, its context menu or a Test opens
at its first part's track, read from its sidecar; a song with no sidecar, an unreadable one
or one with no parts opens with the file alone, and an information line says why.

**Which song is "the" song**, for Edit in MIDI Evil, in order: during a play session, the
song playing in the music slot; else the song open in the Conductor panel, which is what the
next Play plays; else the song the game's `init` asked for; else the project's songs — every
MIDI file under `music/` and `ai/music/`. One song there is the song. Several put a **Song**
row above the button to choose among them. None is an error: "No song in this project: a
song is a .mid under music/ or ai/music/".

## Hot reload

The desktop editor watches the files the open project uses, polling them every 500 ms on a
thread of its own and acting on a change once the file has held still for one further poll,
so a file written in steps is never read half-written. It watches the open scene, every
`.rhai` the scene's `Scripts` name, the `.sheet.ron` of every sprite sheet the scene's
animations use, every recorded sound the scene names or the game has loaded, the open
song's `.mid` and `.mid.ron`, and — during a play session — the playing song's two files
and every SoundFont and sample they name. Each change puts a line in the console —
"scripts/paddle.rhai changed on disk (recompile script)" — then:

| reload | while editing | in Play or Paused |
|---|---|---|
| recompile the script | nothing more: the next Play compiles what is on disk | recompiled at once; every entity running it switches on its next frame, its state kept; a syntax error keeps the old version running and links its line in the console — `docs/SCRIPTING.md` § [Reloading a script during Play](SCRIPTING.md#reloading-a-script-during-play) |
| reload the scene (the open scene only) | a scene without unsaved edits reloads, its undo history going with the old document; one with edits asks "Scene changed on disk — Reload / Keep mine", where Enter and Escape keep yours and Reload, which discards them, is a click; nothing happens under a drag, a focused field or a dialog until it ends | waits for Stop, then the same rule |
| reload the sheet | the sidecar is read and its texture loaded first — a failure keeps the old sheet and says why — then every animation on that sheet takes the new cells, clips and texture; a frame past its new clip's end is clamped, with a warning | the same, and Stop applies the sheet on disk again over the restored scene |
| reload the song | the open song follows: a new `.mid` is adopted with the map and its unsaved edits kept, the track diff's lines in the console; a new `.mid.ron` reloads the map, or asks Reload / Keep mine over unsaved edits | the same for the open song; the playing song is replaced at its next bar line — unless the track diff refuses the new score (a track a part selects was renamed), and then the old song plays on and the console says why — `docs/SONG_FORMAT.md` § [How the Conductor panel saves](SONG_FORMAT.md#how-the-conductor-panel-saves) |
| reload the sample | the recorded sound's bytes are replaced behind its handle; a sound that had failed to load is tried again | the same: a one-shot already playing finishes on the old sound, a loop takes the new one at its next pass (at once when the channel count or the sample rate changed); a reload that fails keeps the old sound and says so |

Some things are never reloaded:

- **The editor's own saves.** A scene, a sheet, a song or a preferences file the editor
  wrote itself is never reported back as a change.
- **Rust source.** No `.rs` is watched, and the editor never runs `cargo`: a change to the
  game's code needs a rebuild, which is yours to start.
- **Images and DAW projects** (§ Which file opens where).
- **A rewrite inside the clock's grain.** A file rewritten at the same length within the
  filesystem's timestamp granularity keeps its stamp and is not seen.
- **The browser playground** watches nothing; a score there comes back by import
  (§ Platform notes, *The web*).

## MIDI Evil, the contract across the crossing

The score belongs to MIDI Evil, which exports the `.mid`; the editor's Conductor panel writes
the map, the `.mid.ron`. Composition left the engine for MIDI Evil, and this is the command
line the editor starts it with — the engine spawns, never waits, never reads stdout:

```
midi-evil [--project <dir>] [--track <name>] [--track-index <n>] <file.mid>
```

| element | meaning | required of MIDI Evil |
|---|---|---|
| `<file.mid>` | the absolute path of the Standard MIDI File (type 0 or 1) to open — for MIDI Evil, an **import** into a working `.evil` beside it or in its own store | open it; if an instance already shows it, focus that instance (single-instance forwarding is MIDI Evil's job) |
| `--track <name>`, `--track-index <n>` | the MIDI track-name meta event (`FF 03`) of the track to select (UTF-8, spaces allowed, one argv element) and its 0-based index in the file's track order, the index winning when both are given — the second of two `drums` is named exactly | select it and scroll to it; unknown → open anyway, say so, exit 0 |
| `--project <dir>` | the game's asset base, where the sidecar's samples and SoundFonts resolve | optional; a hint for dialogs |
| exit status | 0 once accepted (the UI may still be loading), non-zero with one stderr line otherwise | the engine logs stderr's first line |
| **the export back** | MIDI Evil writes the `.mid` the engine opened: format 1, the tempo and **the time signature as written**, every track's name preserved (the join key to the sidecar), **a muted track as a named track with no notes** (indices hold), a program change at tick 0 and CC 7 / CC 10 from the track's patch, gain and pan, ticks-per-quarter as the author set it (480 default for the engine), complete tracks closed with `EndOfTrack`; written **atomically** (a temp file in the same directory, then rename), so the watcher never reads a half-written file; never `<file>.mid.ron` | hard requirements for hot reload to be trustworthy |
| not in the contract | bar position, loop range, instruments, the sidecar's fingerprint, a lock (each file has one writer) | — |

**MIDI Evil's side, as of 2026-11-03** (`docs/insiculous-crossing.md` in MIDI Evil's own
repository carries it): the export with the rules above, opening a `.mid` given on the
command line and exporting back to it on save, `--track` and `--track-index` (the index
winning), single-instance forwarding, `--project`, and a fixture pair for the track diff
are all done. What remains is finding the program: the MIDI Evil preset has no fallback
location and no Flatpak id, so a `midi-evil` the editor can run is either **on `PATH`** —
`cargo install --path crates/midi-evil` in MIDI Evil's checkout, or a symbolic link to its
`target/release/midi-evil` — or **named in the preset's Program field** by its path.

**Every song launch names a track.** A song whose sidecar has parts opens at its first
part's track, `--track <name> --track-index <n>`; a nameless track gets `--track-index <n>`
alone; a song with no sidecar, or one with no parts, opens with the file alone. The Music
Test prints the exact command it ran:
`Test Music: midi-evil --project /game/assets --track drums --track-index 2 /game/assets/music/theme.mid`.

## Platform notes

**Everywhere:** a tool is started with its arguments as a list, never through a shell, so
no path is ever re-parsed — a space, an `&` or a comma in it reaches the tool as written.

**Linux.** The system default is `xdg-open <file>`, your desktop's association.

- **Snap** puts its commands in `/snap/bin`. The editor searches the `PATH` it was started
  with; if `/snap/bin` is not on it, name the program by its path (`/snap/bin/code`) in the
  Program field.
- **Flatpak** apps are not on `PATH` by their binary's name. For Aseprite and Audacity the
  not-found sentence names the custom command that runs the Flatpak —
  `flatpak run org.aseprite.Aseprite {file}`; for any other Flatpak, write the same shape
  with its application id. A Flatpak runs in a sandbox and sees only the folders its
  permissions grant. The editor cannot see that grant: a project the app cannot reach opens
  as an error in the app, not in the editor. Grant the folder with
  `flatpak override --user --filesystem=<project folder> <application id>`.

**macOS.** The system default is `open <file>`. An application that has no command-line
program of its own opens through `open -a` as a custom command — `open -a Aseprite {file}`
— which passes no line. VS Code's `code` exists only once VS Code's *Shell Command* has
installed it (§ Set up VS Code).

**Windows.** The system default is the file's association in Windows — what a double-click
on it in Explorer opens — reached through the shell's `ShellExecuteW` with no process of the
editor's own in between. When Windows cannot open the file, the editor says so: "Windows
could not open <file> (error N) — set the tool in Edit → External Tools…". A bare program name is looked
for with each `PATHEXT` extension, so the VS Code preset's `code` runs VS Code's
`code.cmd`. A `.cmd` or `.bat` runs through `cmd.exe`, and Rust's `std::process::Command`
quotes each argument for it; an argument it cannot quote safely is refused as an error
rather than passed on.

**WSL.** The editor knows it runs under the Windows Subsystem for Linux when
`/proc/version` names Microsoft.

- **The system default** is wslu's `wslview` on the file's Windows path, which `wslpath -w`
  gives: a Linux program cannot reach Windows' associations itself. Without wslu the
  sentence says so: "wslview: not found on PATH (searched N directories) — set the tool in
  Edit → External Tools… — install wslu for the system default under WSL".
- **A Windows program** — any tool whose program ends in `.exe`, in any case, whether it is
  a custom command's first word or a preset's Program field — reads Windows paths, so the
  file and the project go through `wslpath -w` first, off the editor's frame, and the
  command is built from the translated paths: `notepad++.exe {file}[:{line}]` receives
  `\\wsl.localhost\<distro>\home\…\paddle.rhai:12` (`\\wsl$\…` on an older WSL). A
  path `wslpath` cannot translate is one sentence naming the path and the program —
  "wslpath could not translate /home/…/paddle.rhai for notepad++.exe — <why>" — and nothing
  starts. Only `.exe` is translated: a `.bat` or a
  `.cmd`, and every Linux program, receives Linux paths. Because the Windows paths are known
  only once `wslpath` has answered, off the frame, such a launch reports "Starting <file> in
  <program> once wslpath has translated its paths" rather than "Opened …", a failure
  following as an error, and its Test prints the command on the Linux paths with
  "— each path as wslpath -w translates it".

**The web.** The browser playground cannot start a program, so every open action and Test
button is disabled, and a request says "external tools open it from the desktop editor".
Nothing is watched there. A song's score goes out and comes back by file:
`docs/WEB_PLAYGROUND.md` § [Export and import](WEB_PLAYGROUND.md#export-and-import) — export
the project, edit the `.mid` in MIDI Evil, and import it back.

## Set up VS Code and open a script at a line

This walkthrough needs nothing but the engine's checkout and VS Code. It uses the
playground's Pong, which carries Rhai scripts.

1. **Install VS Code** from code.visualstudio.com.
   - **Linux**: the `.deb`, `.rpm` or Snap package puts `code` on `PATH`.
   - **macOS**: open VS Code, open the Command Palette (Cmd+Shift+P), and run
     **Shell Command: Install 'code' command in PATH**.
   - **Windows**: keep the installer's **Add to PATH** option checked.
2. **Check it from a terminal**: `code --version` prints a version. Keep using that
   terminal — the editor searches the `PATH` it was started with.
3. **Copy the project** somewhere you can edit it, so the engine's own copy stays as it is:
   ```bash
   cp -r crates/playground/assets/projects/pong /tmp/pong-tools
   ```
4. **Start the editor on it**, from the engine's folder:
   ```bash
   cargo run --bin editor --features editor -- /tmp/pong-tools
   ```
5. **Choose VS Code**: **Edit → External Tools…**. Under **Code**, click the **Tool** row
   until it reads **VS Code**. Leave **Program** blank (or type the path to `code` if step 2
   needed one).
6. **Test it**: press **Code**'s **Test** button. The console (View → Console) shows the
   command it ran — `Test Code: code /tmp/pong-tools --goto /tmp/pong-tools/assets/scripts/<a script>.rhai:1:1`
   — then "Opened <a script>.rhai in VS Code", and VS Code opens the folder with that script
   at line 1.
7. **Open a script from the scene**: in the hierarchy, the row under **Left Paddle** is its
   script. Double-click it: VS Code opens `paddle.rhai`.
8. **Open it at a line**: in VS Code, break a line of `paddle.rhai` — change
   `let y = me.position.y;` to `let y = ;` — and save. Press **Play** in the editor. The script
   fails to compile, and the status bar and the console show the error with its file and
   line, the console's entry underlined. Click either: VS Code jumps to that line.
9. **Fix it during Play**: put the line back and save. The console says
   "scripts/paddle.rhai changed on disk (recompile script)", and the paddles run the fixed
   script without a Stop.

Any other code tool follows the same steps with its own preset, or with a custom command
whose template names `{file}` and, for a line, `{line}`.
