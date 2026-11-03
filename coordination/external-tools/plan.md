# External Tools — the plan

**Settled at v7 — 2026-10-27, Jesse.** Rounds 1–5 are adjudicated in `review/external-tools/`;
#211 is closed won't-do, #217 and #218 are reworded, the rename is #227, the milestone's
description carries the batch order, and the design note `docs/sprints/external-tools.md` is
folded into § Design and deleted. Ruling (k) stands with the batch-5 listen check as its judge
(gemini's round-5 F1 put to Jesse, who kept the ruling). Batch 1's handoff is
`review/external-tools/handoff-1.md`.

**v7 — 2026-10-27, after round 5** (`review/external-tools/rebuttal-5.md`: kimi 11, gemini 8,
codex 6 — twenty-five findings, twenty-one accepted, two rebutted against the tree and the
standard library, two put to Jesse as arguments against rulings (i) and (k)). What moved: **the
score and the map are guarded separately** — an external score change is always adopted, the map
kept with its unsaved edits, and Overwrite / Cancel is the sidecar's prompt alone; **Save As writes
the whole song** (the loaded score's exact bytes and the map), in-place Save the map; the web
re-import keeps the sidecar; a headless conflict is a structured error answered by `overwrite`;
one helper owns the MIDI extensions and the sidecar suffix, recovered names included; recovered
sidecars are roots for the prune; **the swap is due on a bar-line crossing observed between two
updates**, the wrap included, a seek resetting the baseline; **every preparation carries a request
id** and older results are dropped; the host re-asserts its transport and lifecycle queue on the
new source; a refused preparation stays staged for the next stationary transport; retired sources
drop on a worker; the closure re-registers after every preparation; `{track_index}` joins the
placeholders and the MIDI Evil preset; a launch's exit is reported only in its first five seconds;
WSL's OS default is `wslview`; and ruling (k)'s cost is restated in gemini F1's words — the swap
lands a frame plus the device's lead after the bar line, cutting the old downbeat and skipping
the new one — for Jesse to weigh now.

**v6 — 2026-10-27, Jesse's simplicity pass and the art half** (no round between v5 and v6; round 5
reviews this). Jesse asked for a once-over against the house ideals — simple, accessible, clean — and
took four simplifications the planner proposed, each reversing findings accepted in rounds 1–4
(§ The rulings, (i)–(l)): **two writers, two files** — the crossing lock and the conflict files go,
because after batch 7 no file has two writers; **recovery keeps, never promotes** — the `.new.prev`
journal and its three-state table go, an interrupted save's journal is kept under a recovered name,
and batch 7 deletes the journal with the trim; **the bar swap is the host's first** — the new song is
prepared on a worker and the host swaps the source at the first frame at or past the next bar line,
so the `Replace` command, the revisions and their five states, the seqlock, the `ActionId` and the
retired slot go, the v5 text held in § Appendix for the day the listen check fails; **the Rust
rebuild notice goes** (#211 closes as won't-do). Also: **art joins the table** — an `Art` tool
category, an `Image` row and an Aseprite preset (ruling (m)), and Enter on a focused tile as the
double-click's keyboard twin, the one accessibility gap the pass found. **Native `.aseprite` loading
and the texture reload are their own effort**, `coordination/aseprite-native/plan.md`, reviewed in
parallel with batches 1–3 and built after batch 3; nothing here decodes one.

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
and #217/#218 are reworded when the plan settles (§ The rulings, (c)), and #211 closes as won't-do
(ruling (l)); nothing on the board moves before Jesse has ruled.

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
- **(g) A lock in the crossing's contract — withdrawn by (i)** (round 4 proposed `<file>.mid.lock`,
  `O_EXCL`, stale after 30 s, around every write of either file; v6 removes the second writer
  instead of locking against it).
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
  "the score changed outside", not as a refusal: § The open song document. An interrupted save's
  journal is kept, never promoted (ruling (j)).
- **(i) Two writers, two files** (Jesse, 2026-10-27). After batch 7 the engine writes only the
  `.mid.ron` and MIDI Evil only the `.mid`, so no file has two writers and nothing needs a lock.
  The lock of (g) and the conflict files of v4–v5 go. A change found under a dirty document
  immediately before a write prompts Overwrite / Cancel through the existing `ConfirmDialog`;
  never a third file, never a knowing overwrite — the window between that compare and the
  rename is the accepted cost, and two engine instances on one project are unsupported (round 5,
  kimi F3, rebutted as this ruling). In the transition — before batch 7 — a
  score-authoring save keeps its fingerprint precheck and the window between the check and the
  write is accepted and documented: it is one developer's machine for a few weeks, and the
  recovered file is what it costs.
- **(j) Recovery keeps, never promotes** (Jesse, 2026-10-27). The `.new.prev` journal and its
  three-state classification go. On open, a `.new` journal — the pair, or a lone sidecar — is
  renamed to `<stem>.recovered-<fingerprint>.<ext>` with a notice and the finals open as they
  are; nothing is deleted, nothing is promoted. After batch 7 a save is one file by
  temp-and-rename and the journal is deleted with the trim.
- **(k) The bar swap is the host's first** (Jesse, 2026-10-27). The new song is prepared on a
  worker (kept) and the host swaps the playback's source at the first `update` at or past the
  next bar line, the position carried as the bar and the fraction of the bar converted with
  both tempo maps on the host. Gone: the `Replace` lifecycle command, the revisions and their
  five states, the seqlock snapshot, the stable `ActionId`, the retired slot, the seam's
  allocation-free re-check and the acknowledgement window. Kept: the worker preparation, the
  track diff, the host's generation, the stationary-transport and stopped-at-end rules, the
  shorter-song rule. The cost, said plainly (round 5, gemini F1 names it exactly): the swap
  lands up to one frame plus the device's lead after the bar line — the old score's downbeat has
  begun and is cut with the seek's release, and the new score's notes that began at the bar line
  are skipped. The batch-5 listen check
  decides; if Jesse hears it, the audio-thread seam of v5 returns as its own batch — its text
  is held verbatim in § Appendix, not deleted. The simplest thing that works, machinery only
  after the simple version has actually failed.
- **(l) The Rust rebuild notice goes** (Jesse, 2026-10-27). #211 closes as won't-do; the `.rs`
  tree listing, the five watched roots and the sticky notice leave the watcher;
  `RebuildNotice` leaves `ReloadPolicy`; a `.rs` opens in the Code tool with policy `None`.
  You edited the Rust; you know it needs a rebuild.
- **(m) Art joins the table; the native half is its own effort** (Jesse, 2026-10-27). An `Art`
  tool category with an `Image` row and an Aseprite preset land here, as table rows in the one
  mechanism. Reading `.aseprite` files directly, the texture reload behind a handle and the
  change to how art enters a game are `coordination/aseprite-native/plan.md`, reviewed in
  parallel and built after batch 3; its `.aseprite` row and `ReloadTexture` policy join this
  table when it lands.
- **(n) No `$VISUAL`/`$EDITOR` fallback** (Jesse, 2026-10-27, batch 1's code round, kimi F2 and
  gemini F3). Both name a terminal editor on most machines, which cannot run under the launcher's
  null stdio, so `EDITOR=vim` turned the first Open Source into an error. Code's `OsDefault` is
  the platform's opener like every category's; a GUI editor in `$VISUAL` is chosen as a Custom
  tool. Ruling (d)'s Helix preset stands.

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
| The song document | `editor/src/song/`: `SongDocument` with its own `SongData`; `save` (`io.rs:310`) writes the `.mid` and the sidecar from memory with the fingerprint (`:281`); `open` (`io/open.rs:89`) promotes a `.new` journal pair that matches itself (`recover_pair`, `:161-173`, without looking at the finals) and refuses finals whose fingerprint differs (`MismatchedFinals`, `:130-133`); `apply` refuses every `SongEdit` under `EditingPolicy::Refused` (`song/mod.rs:172-179`, mixed-channel tracks) | **Extend**: the document follows the score, a map save is sidecar-only, a journal is kept rather than promoted, map edits on any playable MIDI (§ The open song document) |
| Recorded sounds | `AssetManager::resolve_sound` caches handles by path and remembers failures until the base path changes (`sound_library.rs:21-26`); never invalidates (#180) | **Fix** the sound half as the sample reload, failures included |
| Which song is the project's | `init_music.rs` captures the song the game's `init` asked for; the composer scans `music/` and `ai/music/` for its Songs list | **Keep** both; the scan is salvaged into `external_tools/songs.rs` |
| The build | Decision of record (`docs/EDITOR_COMMAND_API.md`, "the editor does not own the build"): any rebuild is "spawn cargo, relaunch self"; no build-system work | **Nothing** for `.rs` changes (ruling (l)) |
| The composer | 8,322 lines incl. tests across three layers: the GUI (`editor/src/composer/`, 21 files: state, layout, keys, piano roll, note list, sidebar, sound_fonts, transport; `editor_integration`'s `composer_keys.rs`, `composer_drag.rs`, `panel_renderer/composer.rs`), the song document (above), the command API (`command_api/song.rs`: `song new\|edit\|save\|open\|close\|play\|stop\|seek\|trigger`, `song`, `song notes`) | **Re-scope** into the Conductor panel (§ The Conductor panel): the map stays, the score's authoring goes — once the round trip exists |
| `song_preview.rs`, `init_music.rs` | the preview transport and the session's music; the Conductor effort's close-out landed 2026-10-26 (8061b1c), so this effort may edit them | **Keep**; the session-music half is how Play hears the game's song |

## Design

### One table, one mechanism

```
FileType  ──►  ToolCategory  ──►  ToolChoice (per category, overridable per type)  ──►  Launch
   │                 │
   └── ReloadPolicy  └── Code | Music | Sample | Art     (the category decides the presets and the env fallback)
```

`FileType` (`crates/editor/src/external_tools/file_type.rs`, the one place the mapping lives):

| type | extensions | category | `{project}` | reload policy |
|---|---|---|---|---|
| `RustSource` | `.rs` | Code | Cargo workspace root | `None` (ruling (l)) |
| `RhaiScript` | `.rhai` | Code | Cargo workspace root | `RecompileScript` |
| `Ron` | `.ron` (scene), `.sheet.ron`, `.mid.ron` | Code | Cargo workspace root | `ReloadDocument` (scene) / `ReloadSheet` / `ReloadSong` (by sidecar suffix) |
| `Midi` | `.mid`, `.midi` — one helper decides the MIDI extension, case-insensitive, and its sidecar suffix `<name>.<ext>.ron`, used by save (`io.rs:347` accepts `mid` alone today), the Songs scan, the watcher, the upload, this table and the recovered names (round 5, kimi F7, gemini F6) | Music | asset base | `ReloadSong` |
| `AudioSample` | `.wav`, `.ogg`, `.flac`, `.mp3` | Sample | asset base | `ReloadSample` |
| `DawProject` | `.evil`, `.prj`, `.nsp` by default, editable (ruling (d)) | Music | asset base | `None` |
| `SoundFont` | `.sf2` | Music | asset base | `ReloadSong` (for every song whose closure names it — round 2, codex F7) |
| `Image` | `.png`, `.jpg`, `.jpeg`, `.bmp`, `.gif` (the asset browser's list, `asset_browser.rs:98-108`) | **Art** | asset base | `None` here; the Aseprite effort adds `ReloadTexture` and its own `.aseprite` row |

The reload policy is data in this table, not a setting: users pick tools; the engine knows what a
file means. The asset browser's `AssetKind` follows the table: `Song` gains `midi`, and a
`DawProject` kind over the config's extension list draws its own tile (round 1, kimi F11), so every
advertised extension reaches a tile and the context menu; a launch test per extension.

### Types

```rust
// crates/editor/src/external_tools/
pub enum ToolCategory { Code, Music, Sample, Art }
pub enum ToolChoice { Preset(PresetId), Custom(CommandTemplate), OsDefault }
pub struct ExternalToolsConfig {            // on EditorPreferences, #[serde(default)]
    pub code: ToolChoice,                    // default: OsDefault (the platform's opener, ruling (n))
    pub music: ToolChoice,                   // default: OsDefault
    pub sample: ToolChoice,                  // default: OsDefault
    pub art: ToolChoice,                     // default: OsDefault
    pub overrides: BTreeMap<FileType, ToolChoice>,
    pub daw_project_extensions: Vec<String>, // default [".evil", ".prj", ".nsp"]
}
pub struct CommandTemplate(String);          // "rustrover [--line {line}] [--column {column}] {file}"
pub struct Launch { pub program: String, pub args: Vec<String>, pub preset: Option<PresetId> }

pub struct OpenRequest { pub path: PathBuf, pub location: Location }     // replaces pending_open_source
pub enum Location { None, Project, Line { line: usize, column: Option<usize> }, Track { name: Option<String>, index: usize } }

pub enum ReloadPolicy { None, RecompileScript, ReloadDocument, ReloadSheet, ReloadSong, ReloadSample }
```

Resolution (`config.rs`, pure, tested), in order: `overrides[file_type]` **if present** — an
absent entry inherits the category, a present one is honoured whatever it says, an explicit
`OsDefault` included (round 3, codex F6); the category's choice if not `OsDefault`; no
environment variable is read (ruling (n)); the OS default: Linux `xdg-open
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
path tokenizes as one program. The placeholders are `{file}`, `{project}`, `{line}`, `{column}`, `{track}` and
`{track_index}` (round 5, gemini F4, codex F5). An unknown `{name}` is an error at config time
(the settings panel shows it; the launcher refuses). `{file}` is always absolute (`resolve_asset_path`); `{project}` per
the table. About 90 lines, no dependency.

### Presets — verified 2026-10-26 against the vendors' pages

| preset | binary | template | source |
|---|---|---|---|
| RustRover | `rustrover` (the Toolbox script; the page says no launcher is installed by default) | `rustrover [--line {line}] [--column {column}] {file}` | jetbrains.com/help/rust/opening-files-from-command-line.html, verbatim `rustrover [--line <number>] [--column <number>] <path>`; project-plus-file in one call is not stated, so "Open project" is a second launch `rustrover {project}` |
| VS Code | `code` (macOS needs the Shell Command install) | `code {project} --goto {file}[:{line}[:{column}]]` | code.visualstudio.com/docs/configure/command-line — `--goto` "file:line{:character}"; folder-plus-`--goto` in one call is two documented arguments, flagged for the manual test |
| VSCodium | `codium` | as VS Code | github.com/VSCodium/vscodium/blob/master/docs/usage.md; the flags being VS Code's is not stated there — manual test |
| Zed | `zed` (Linux "commonly `zed` or `zeditor`") | `zed {project} {file}[:{line}[:{column}]]` | zed.dev/docs/reference/cli.md, verbatim `zed myfile.txt:42:10` |
| Helix | `hx` | `hx [-w {project}] {file}[:{line}[:{column}]]` | helix-term's help text `file[:row[:col]]`, `-w, --working-dir`; a terminal editor, ruling (d) |
| **MIDI Evil** | `midi-evil` (the desktop crate's binary; `cargo run -p midi-evil` today) | `midi-evil [--project {project}] [--track {track}] [--track-index {track_index}] {file}` | § MIDI Evil's half, the contract this plan defines; until MIDI Evil has the flags, the Custom field with `{file}` alone works |
| Audacity | `audacity` (Flatpak `org.audacityteam.Audacity`) | `audacity {file}` | `AudacityApp::ParseCommandLine` (3.7.8): a file-name parameter, no time-position option; re-verify at Audacity 4 |
| **Aseprite** | `aseprite`; after `PATH`, the Steam install `~/.steam/root/steamapps/common/Aseprite/aseprite` (the path `deion_assets/scripts/export_sheet.py:28` already tries); the not-found message names the Flatpak id `org.aseprite.Aseprite` | `aseprite {file}` | aseprite.org/docs/cli: a file argument opens it in the GUI; `--frame-range` and `--tag` act only in batch mode (`-b`), so there is no location placeholder — re-verify at the next Aseprite release (1.3.18 at writing) |
| Custom command | any | the user's template | — |

The preset table is data (`presets.rs`), each row's source URL in a doc comment, so the next
verification is a diff against the page. **Each Code preset carries a second, `project`
template** for Edit → Open project (round 3, kimi F8): RustRover `rustrover {project}`, VS Code
`code {project}`, VSCodium `codium {project}`, Zed `zed {project}`, Helix `hx -w {project}`; a
Custom tool's project template is a second field, empty meaning the action is disabled; Music,
Sample and Art tools have none. A test per preset.

### The launcher (`editor_integration/src/editor_game/external_tools.rs`, replaces `open_source.rs`)

Per frame, drain `EditorContext.open_requests: Vec<OpenRequest>` (a Vec: a context menu and a
console click can land in one frame). For each: resolve `FileType` → `Launch`; find the program —
**a program containing a path separator is checked as a file first** (round 1, gemini F8), else
looked up on `PATH` yourself (iterate `PATH`, honour `PATHEXT` on Windows), then at the preset's
fallback locations if it names any (Aseprite's Steam install), so a missing binary
reads `rustrover: not found on PATH (searched 11 directories) — set the tool in Edit → External
Tools…` rather than the OS's `No such file`; spawn with `std::process::Command` (argv, never a
shell), **stdout to null and stderr piped**; **two threads per launch** (round 1, gemini F7;
round 2, gemini F6): one reads stderr into a bounded buffer (the last 4 KiB) until EOF, so a
long-lived GUI child never blocks on a full pipe and a descendant holding the pipe blocks only
that reader; the other waits the child, as today's reaper does (`open_source.rs:43`), **joins the reader
(bounded at 200 ms, through a channel's `recv_timeout` — std has no timed join)** and reports a
non-zero exit only within the first five seconds, reaping a later one silently (a GUI closed
hours later is not news — round 5, gemini F3), the first 500 ms
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
| Double-click a tile in the asset browser | `panel_renderer/asset_browser/tile.rs` — a second press within 400 ms and 4 px of the first, on the same tile, with no drag **started** between them (the tile arms a drag on every press, `tile.rs:14`; a drag that passed the threshold suppresses the click, and the double-click with it); the tile's own state remembers the last press; the `ui` crate gains nothing | `OpenRequest { path, Location::None }`; a `.mid` tile → `Location::Track` of the first part's track when the sidecar has one (batch 4; batch 2a opens every tile at `Location::None`); a `DawProject` tile → the Music tool; an `Image` tile → the Art tool |
| Enter on a focused tile | the asset browser holds the keyboard as the composer does (Jesse, 2026-10-27, at batch 2's re-verification: no tile could take focus — Tab traversal reaches only edit fields): a press in the Assets panel takes it; arrows move the selected tile, Enter opens it, the menu key or Shift+F10 opens its context menu, Escape or a press elsewhere lets it go; the selected tile draws the focus ring — the keyboard twin of the double-click (ruling (m)) | as the double-click |
| Double-click a hierarchy `Scripts` pseudo-row | `hierarchy/` — not the inspector's script rows, which are the "Script id" and "Source" text fields, where a double-click selects a word; those keep the Open Source button | the script's `source_path`, `Location::None` |
| Open Source button (exists) | `script_editor.rs` | as above, through the new request type |
| Context menu: Open in <tool> (the item names the resolved tool — "Open in Aseprite", "Open in RustRover", "Open with the system default") / Open containing folder / Edit in MIDI Evil | a first `ContextMenu` widget on `UiLayer::Floating` (the menu bar's dropdown geometry reused), right-press on a tile or a Scripts row (the `ui` input gains the right button's press, its one change); the menu key and Shift+F10 open it from the keyboard | per item |
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
hand-edited sidecar named retries the preparation (round 2, kimi F9) — **the closure
re-registered after every preparation, successful or failed**, so a dependency a sidecar edit
added is watched from then on (round 5, kimi F11); every `.sheet.ron` the
scene's sprites reference. No `.rs` is watched (ruling (l)). The mtime-granularity pitfall (same length within the
filesystem's granularity keeps its stamp) is accepted and documented.

Dispatch (`editor_integration/src/editor_game/hot_reload.rs`), by the table's policy:

| policy | editing | Play / Paused |
|---|---|---|
| `RecompileScript` | a console line (the runner compiles at session start) | `ctx.scripts.reload_source(path)`: recompile; on success swap the AST for every instance of that path, clear its `failed` and quarantine entries, keep blackboards and params; on a syntax error keep the old AST running and put the error, with its line, in the console as a link; a runtime error on the first tick after a reload (the state's shape changed) quarantines the instance whatever its kind — elsewhere only a runaway loop or a blown limit does — and the console says "reloaded script failed on its first frame — Stop and Play to reset its state", the blackboard kept on purpose (round 2, gemini F7) |
| `ReloadDocument` (scene) | history clean → `load_scene` from disk (the dry-run guard keeps a bad file from the live world); dirty → `ConfirmDialog` "scene changed on disk: Reload (discard my edits) / Keep mine" | deferred: a notice now, the same rule at Stop |
| `ReloadSheet` | a transaction: the sidecar parsed and the texture resolved **before** any live state changes, a failure keeping the old sheet and reporting (round 2, kimi F13); then `clear_sidecar_cache()` + the sheet's texture and clips for every `SpriteAnimation` naming it; a clip index past the new frame count clamps and warns | same; **and Stop re-applies the on-disk sheet state to the restored world** (the sidecar is SSOT; the snapshot restore would otherwise bring the old clips back with the stamp already consumed — round 1, kimi F10), guard-tested |
| `ReloadSong` (the `.mid`, the `.mid.ron`, a SoundFont or a sample in a song's closure — any dispatches the song that owns it) | the open document follows the score (below) and the track diff runs against the last loaded song, so a rename is caught before Play | the open document follows the score; the bar-seam swap below |
| `ReloadSample` | a recorded sound: the `AudioManager` reloads the bytes behind the same `SoundHandle`, so every emitter and `resolve_sound` entry stays valid (the sound half of #180); **a path the library had marked failed is retried, and a success clears the remembered failure and serves the handle** (round 1, kimi F12); a failed reload keeps the old bytes and reports; a song sample is in the song's closure → the swap | same; a one-shot already playing finishes on the old bytes, a looping instance restarts on the new at its next loop boundary with the engine's fade (round 3, gemini F9) |

#### The open song document follows the score

The Conductor panel's `SongDocument` holds the score it opened; MIDI Evil owns the score from now
on, so the document **never writes it from memory over a newer file** (rounds 1 to 3) — and after
batch 7 **each file has one writer**, the engine the `.mid.ron` and MIDI Evil the `.mid`, which is
what lets the rest be simple (ruling (i)):

- **Save As writes the whole song; Save writes the map** (round 5, kimi F2). `song save <path>` to
  another target, and the panel's Save As, write the loaded score's exact bytes and the map — a
  copy, the one case the engine writes a `.mid` after batch 7; an existing target needs
  `overwrite` (the API has it, `song_io.rs:190-194`) and both its files are replaced. Tests for
  a new and an overwritten target.
- **A map save is sidecar-only.** Save writes the `.mid.ron` (and the LICENSE stanza) and leaves
  the `.mid` untouched unless the document's score was changed in the engine — which, after batch
  7, nothing can do, and before it only a score-authoring edit does. **The document keeps the
  exact bytes it loaded** (`source_bytes`): a map save fingerprints those for `midi_fingerprint`,
  never `write_smf()`'s re-encoding, which can differ from the file's (`io.rs:277-283`; round 3,
  kimi F5), with a test on a MIDI whose parse and export differ.
- **In the transition, a score-authoring save never overwrites a `.mid` whose bytes are not the
  ones this document loaded or last wrote** (round 3, kimi F1): the fingerprint is compared
  immediately before the finals' write; on a difference the journal's score is kept as
  `<stem>.recovered-<fingerprint>.mid` with a notice and the on-disk score is untouched. The
  window between that comparison and the write is accepted and written down (ruling (i)): it
  exists only until batch 7, on one developer's machine, and the recovered file is what it costs.
- **The document guards the two files by content fingerprint, and separately** (round 3, kimi
  F4; round 5, gemini F2, kimi F9): it remembers the `.mid`'s and the `.mid.ron`'s fingerprints
  from its last read. **An external change of the score is always adopted**, clean or dirty —
  MIDI Evil owns the score (ruling (b)) — the map kept with its unsaved edits, the track diff run
  and its lines shown, the status bar saying the score was edited outside; there is no "Keep
  mine" for a score. An external change of the map: a clean document reloads it; a dirty one
  prompts through the existing `ConfirmDialog` machine ("the map changed on disk: Reload / Keep
  mine"). **Immediately before each write both fingerprints are compared again**: a changed
  score is adopted first (reload, keep the map, re-run the diff) and the save then fingerprints
  the score as it is on disk; a changed sidecar — a same-length edit inside the mtime
  granularity, a write between the watcher's poll and the Save — prompts "the map changed on
  disk since you loaded it: Overwrite / Cancel". Never a knowing overwrite, never a third file:
  ruling (i) retires the conflict file. Same-stamp and write-during-commit tests for each file. **The document's reload is never gated on the audio swap's
  validation**: a renamed track refuses the playback replacement, but the document still holds
  the new score with the old map, shows the diff's lines, and lets the map be fixed and saved —
  and that Save dispatches the replacement itself (gemini F1 of rounds 2 and 3).
- **Map edits are allowed on any MIDI the loader plays.** `EditingPolicy::Refused` (mixed-channel
  tracks) scopes to score edits; parts, instruments, bindings, provenance and Save work for every
  playable file, with a test on a two-channel track (round 2, kimi F8).
- **A sidecar can be scaffolded**: `song init <path.mid>` stays as a map verb — a default sidecar
  with the project's first bank, no parts, no bindings, `looping: true` — and the Conductor panel
  opens an unmapped `.mid` the same way, the sidecar marked unsaved (round 3, gemini F8).
- **The fingerprint** (ruling (f)): Save keeps writing `midi_fingerprint`; on open, a fingerprint
  that differs from the bytes **with no journal present** means the score changed outside — the
  song opens on the new MIDI with its sidecar, the status bar says so, and the next Save rewrites
  the fingerprint; a mismatch **with** a journal is an interrupted save, below.
- **Recovery keeps, never promotes** (ruling (j)). On open, a `.new` journal — the pair, or a lone
  `.mid.ron.new` from a map save — is renamed to `<stem>.recovered-<fingerprint>.mid` and/or
  `<stem>.recovered-<fingerprint>.mid.ron` — the compound suffix split off first by the MIDI
  helper, never `theme.mid.recovered-….ron` (round 5, gemini F6); a name that never collides,
  never replacing an existing file — with a notice naming
  it, and the finals open as they are. Nothing is deleted, nothing is promoted, and the finals
  are not classified: the person opens the recovered file if they want that save. This replaces
  `recover_pair` (`song/io/open.rs:161-173`) and `MismatchedFinals` (`:130-133`) with one rule
  and one test per journal shape (pair, lone sidecar, journal plus an externally changed `.mid`);
  the existing recovery and prune guard tests are rewritten to it. **The prune of unreferenced
  samples runs after the finals are written and the journal removed** (`io.rs:462` prunes between
  them today; round 3, kimi F2), so an interruption at any step leaves every sample the chosen
  song names on disk — **and every `<stem>.recovered-*.mid.ron` beside the song is a root for the
  prune**, so a kept recovery can still be opened (round 5, codex F4; an interruption → recovery
  → save → open-the-recovery test). **Batch 7 deletes the journal**: once Save writes one file, it is a temp
  file in the same directory and a rename, and `.new` ceases to exist.
- **The API** (`song open`, `song save`, `song init`) follows the same rules, so the headless path
  and the panel cannot disagree: `song save` answers a sidecar conflict with a structured error
  naming both files and each one's state (today it says "the save was parked",
  `song_io.rs:196-201`); `song save <path> overwrite` is the non-interactive overwrite and `song
  open` the reload (round 5, kimi F8), tested over `--headless`.

#### MIDI reload: swap at the next bar, keep playing

The replacement is **a prepared source, swapped by the host** (ruling (k)); the audio thread
learns nothing new. `AudioManager::replace_song_at_seam(path)` prepares the new closure **on a
worker thread** (the result over a channel, polled from `update`; the game thread only adopts the
result, so a 48 MiB bank never stalls a frame — round 4, kimi F8) exactly as `play_song` does: the
same cache, so only the changed file is decoded again; the same `validate`, so a refusal leaves
the playing song untouched and names the sidecar entry and the track; **a selected track with no
notes is a silent part** — a trigger on it is a rest, counted in `triggers_silent`, the cursor
held — so ruling (a)'s muted track passes (round 2, kimi F4); a score with no notes at all,
`end_tick == 0`, is refused as "the score is empty". The worker builds the whole `ConductorSource`
— the conductor loaded, its slots and background synthesizers built, its bindings resolved by
name into its parts, its own command channel and `SharedPosition` — and hands it over as
`PreparedPlayback { generation, song: Arc<Song>, bindings, beat_edges, source }`, parked on
`SongPlayback` as `pending_replacement: Option<PreparedPlayback>`. **Every preparation carries a
monotonically increasing request id** (round 5, kimi F4, codex F3): a result or a failure older
than the latest request is dropped unread, so a slow bank preparation can never overwrite a
newer sidecar-only one — the newer wins by request order, never by finish order, with B finishing
before A as the test; a preparation for another playback generation is dropped the same way.
Superseded and retired sources are dropped on a worker thread, never on the frame (round 5,
gemini F7).

**The swap**, on the first `update` where it is due: the host reads the old transport's position
and converts it as **the bar index and the fraction of the bar in the old map — to the tick at
that fraction of the same bar in the new map**, the fraction applied to the new bar's actual
length (a partial last bar included), so bar 4 of the new score follows bar 3 of the old whatever
the tempo or meter did, a paused half-beat survives, a 4/4 → 3/4 change lands in the same bar at
the same proportion, and nothing maps past `end_tick`; a looping song takes the bar modulo the new
song's bar count (the engine's loop is the whole song, `sequencer.rs:61-126`; there is no loop
start); precedence: clamp inside the bar, then wrap for a looping song, then the non-looping
refusal. Then the host seeks the new source to that tick with the old transport's play state and
looping flag, replaces the slot's source — the path `play_song` already takes (`song.rs:340`) —
and adopts **that preparation's** `Arc<Song>`, binding table and `BeatEdges` on the new tempo map
as one transition, so `trigger("boom")` after a reordered sidecar reaches the right part, an added
action resolves, and `beat_crossed` counts on the new map. **When it is due**: a running
transport — the first `update` that observes a bar-line crossing since the previous `update`,
forward or by the loop's wrap, so a replacement prepared in a loop's last bar is due at the wrap
and the swap lands within one frame after the bar line; a seek the host itself sent resets the
baseline and never counts as a crossing (round 5, kimi F6, codex F1 — backward and forward seeks
after the preparation, a loop's last bar and a one-bar loop are tests); a stationary
transport — paused, or stopped at its end — at once, at its held position; a loop shorter than a
bar at its wrap; a running song that reaches its end with a replacement pending takes it there,
playing on when the position is inside the new song, else stopped at the new end (round 4,
gemini F3). For a running non-looping song a mapped position at or past the new end refuses the
swap — `ReplaceRefused { reason }` to the console, the old playing on, **the preparation kept
staged and taken by the next stationary transport**, a stop, a seek or a replay (round 5, gemini
F5) — instead of ending the transport; a transport **stopped at the old song's end takes the
replacement stopped at the new song's end**, so a seek or a replay plays the new score (round 3,
gemini F4). `play_song`, `stop_song` and the editor's session end drop a pending preparation
(round 1, kimi F5; codex F7).

**What it costs, said plainly** (ruling (k); round 5, gemini F1 names the mechanism): the
published position runs ahead of what is heard by `latency_samples` and the device's buffer (up
to 61 ms in a browser), so at the frame after the bar line the old source has already rendered
the downbeat: the swap cuts it with the seek's release, and the new source starts past the notes
that began at the bar line, so the new score's downbeat is skipped — audible once per save, at
the downbeat. The command channel, the handle and the position reader belong to a source, so the
swap replaces them with the new source's; **the host re-asserts its own `requested` transport
(`song.rs:146`) and re-sends its pending lifecycle queue to the new source**, so a seek or a
pause the old channel accepted and had not drained is not lost; the triggers of the swap frame
are, once per save, and the doc says so; a deactivation is moot on the new source's fresh table,
since the host's `SoundSources` knows which sources ended (round 5, kimi F5, codex F2 — tested
with the old source prevented from draining). Nothing crosses the audio thread but the source swap the engine already does for every
`play_song`. **The listen check of batch 5 decides**: "heard from the next bar, the hits on the
beat before and after" passes with that frame of lateness; if Jesse hears the seam, the
audio-thread swap of v5 — held verbatim in § Appendix — returns as its own batch.

Tests through the public `AudioManager` API, headless on the virtual clock: a reordered binding,
an added action, a tempo change, a 4/4 → 3/4 change, a ticks-per-quarter change, a seek queued
before and after the preparation arrives, a replacement prepared for a song that was then
stopped, a replacement over a transport stopped at its end, a second preparation before the first
swaps, a shorter looping song, a bank-only edit (the SoundFont changed, the MIDI and the sidecar
not). The timing fixture is the largest shipped instrument set (Chicken Coop's five): the
worker's preparation time measured under `cargo test --release` and quoted. The guarantee: the
background score and every later trigger stay on the same beat grid as before the save, within a
frame.

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
| not in the contract | bar position, loop range, instruments, the sidecar's fingerprint, a lock (ruling (i): each file has one writer) | — |

MIDI Evil's issues, in its own repo, the contract's other half (`../midi-evil/docs/insiculous-crossing.md`
carries them for its agents): (0) `.evil` → `.mid` export with the rules above (today's export is
NanoStudio-only and 4/4-based at 128 ticks); (1) open a `.mid` given positionally and export back to
it on save; (2) `--track <name>` and `--track-index <n>`; (3) single instance; (4) `--project <dir>`; (5) a fixture pair the
engine's track diff can use — a `.mid` and the same file with one track renamed — checked into MIDI
Evil's fixtures and mirrored into `crates/music_midi/fixtures/` with a LICENSE line. Until (0) and
(1) exist, the engine side works with the Custom field (`cargo run -p midi-evil -- {file}`), and the
round trip is import → edit → export by hand; **batch 7 waits on (0) and (1)** (§ The batches).

### Cross-platform rules

Argv everywhere; `std::process::Command` quotes for `CreateProcess` itself; **no shell call anywhere** — the Windows OS default is `ShellExecuteW` (ruling (h)), the
association without a parse and without a trampoline (round 3, kimi F7, codex F5; round 4, gemini
F5); `windows-sys` is already in `Cargo.lock` four times over through winit, so ruling (h) adds a
direct dependency on a crate the tree already builds, not new code.
A Flatpak app is not on `PATH` as its binary name: the "not found" error suggests `flatpak run
<app-id> …` as a Custom command, and the docs note the sandbox's filesystem grant. Snap binaries
live in `/snap/bin`. WSL: a resolved program ending in `.exe` on Linux gets every path placeholder
through `wslpath -w`, a failed conversion an error naming it; the OS default under WSL (`/proc/version` contains "microsoft") is `wslview <translated path>`
— wslu's launcher, argv, no shell; absent, the not-found message names `wslu` — because a Linux
binary cannot call `ShellExecuteW` (round 5, gemini F8, codex F6); the manual matrix exercises it
with a space, an `&` and a comma in the path. macOS: `open -a <App> <file>` is the "an app, not a CLI" escape hatch. A missing binary
is one sentence, never a panic. Web: the whole module is `cfg(not(target_arch = "wasm32"))` except
the types; the entry points draw disabled with today's hint.

### Patterns

Command (an `OpenRequest` is data the host executes later; a replacement is a prepared source the
host swaps when it is due); Observer by polling (the watcher), over OS notification because the rule is no new
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
| **the web playground** | **keeps the Conductor panel and its `.mid`/`.wav` import**; **a re-imported `.mid` keeps its sidecar** (`upload.rs:191-195` removes it and both journals today) and the open document follows the score as on native, with an end-to-end test that the notes change and the map survives (round 5, kimi F1); score editing on the web is download → MIDI Evil → re-import, and the browser's piano roll goes only with the native one, after the round trip is demonstrated | said plainly (round 1, codex F9): external launch is disabled on the web, so the browser's workflow is the file round trip |
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

**Batch 1 — the foundation: #203 (E1), #204 (E2), #205 (E3). DONE 2026-10-27, insiculous_2d
378f4c2.** The pure half (the type table with
the `Art` category and its `Image` row, `ReloadPolicy` without `RebuildNotice`, the group-first
templates, the config with its `art` choice, the presets with MIDI Evil's and Aseprite's rows, the
`{track_index}` placeholder and the preset fallback locations, the resolution order, the MIDI
extension helper), the
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

**What batch 1 left (2026-10-27).** Landed from `report-1.md` (COMPLETE) after code rounds 6 and 7
(kimi 4 then 1, gemini 6, the planner's own 3; `rebuttal-6.md`, `rebuttal-7.md`). Ruling (n)
removed the `$VISUAL`/`$EDITOR` fallback; the planner fixed a tools section that fails to parse
(now defaulted alone, the rest of the prefs kept), the terminal wording (a custom command or a
`needs_terminal` preset only) and the legacy migration (each word carried literally, a `{file}`
inside one kept). The executor's own calls stand: `windows-sys` needs
`Win32_UI_WindowsAndMessaging` beside `Win32_UI_Shell`; `workspace_root` is computed per launch; a
project request resolves as `RustSource`; stdin is null too; an opener's fast failure is told
plainly. What it left is written into the batches that act on it — the MIDI helper's other callers
(3, 4, 5), the Custom tool's on-disk shape (6), the WSL `.exe` rule, the first Windows compile and
the `ShellExecuteW` stall check (8). 2,254 lines inserted against the 1,300 target, about 800 of
them tests.

**Batch 2, split at its re-verification (Jesse, 2026-10-27): 2a the entry points, 2b the console**
— batch 1 ran 2,254 lines against a 1,300 target, and the two halves share nothing but the
launcher. The tree as batch 1 left it: `OpenRequest`, `Location` and `EditorContext.open_requests`
exist and the launcher resolves every file type; the status bar is the launch's only report.
Files near the 600-line ceiling, so new state goes in new files or the sub-states named:
`editor/src/context/mod.rs` 572, `panel_renderer/asset_browser.rs` 581, `editor/src/hierarchy/mod.rs`
554, `editor_game/composer_keys.rs` 537, `editor_game/mod.rs` 531, `panel_renderer/mod.rs` 526,
`editor/src/menu/mod.rs` 523, `editor_game/shortcuts.rs` 515.

**Batch 2a — the entry points: #206 (E4). DONE 2026-10-27, insiculous_2d 69d9e3f, after code
rounds 8, 9 and 10.** A **double-click** — a second left press on the same
asset tile within 400 ms and 4 px of the first, with no drag started between them (the tile arms a
drag on every press, `panel_renderer/asset_browser/tile.rs`; `DragDropState::suppresses_click`
already suppresses a click after a drag, and the double-click goes with it) — files
`OpenRequest { path: <the tile's relative path>, Location::None }` for **every tile kind** (Jesse,
2026-10-27); the last press (which tile, when by `common::clock::Instant`, where) lives in
`AssetBrowserState`, and the `ui` crate gains nothing for it. The same rule on a hierarchy `Scripts`
pseudo-row (`HierarchyClick::Script { entity, index }` reports its click today,
`hierarchy/mod.rs:349-350`) opens the script's `source_path`; a script with none is a native one
and says so on the status bar. **The asset browser holds the keyboard** on the composer's model
(`KeyRoute::Composer`, `settle_composer`): a press inside the Assets panel takes it; a press
elsewhere in the dock, the panel hidden or collapsed, Escape, and a play session's entry let it go;
while it holds it, its keys decide before the editor's — Left and Right step the selected tile, Up
and Down step a row of the grid (the selection scrolled wholly into view through the panel's
`ScrollState`), Enter opens it as the double-click does, the menu key (`KeyCode::ContextMenu`) or
Shift+F10 opens its context menu at the tile — so an arrow never nudges a scene entity behind it;
a key it does not use goes on to the editor; the selected tile draws the focus ring
(`style.focus_ring`, `ui/src/context/edit_field.rs:236-241` is the ring's model) only while the
browser holds the keyboard. **The first context menu**: `editor/src/context_menu.rs`, a
`ContextMenu` on `UiLayer::Floating` that reuses the menu bar's dropdown — `dropdown_bounds`,
`measured_width` and `render_dropdown_static` (`menu/mod.rs:393-433`, private to `MenuBar` today)
made shareable, not copied — opened by a right press on a tile or a `Scripts` pseudo-row (the `ui`
input reads the left button alone, `ui/src/input_state.rs:183`; it gains the right button's press,
the batch's one `ui` change) or from the keyboard as above; Up and Down walk its enabled items,
Enter chooses, Escape or a press outside closes it, and it blocks input beneath it as the
dropdown does. Its items: **Open in <tool>**, naming what the file resolves to — "Open in
RustRover", "Open in Aseprite", "Open with the custom command", "Open with the system default" —
from one pure function in `external_tools/config.rs` that `resolve`'s own messages share; and
**Open containing folder**, an `OpenRequest` for the file's folder, which the launcher sends
straight to the platform's opener when the path is a directory, never through a file type. On the
web both items draw disabled with the Open Source button's desktop hint. Target shapes: § Entry
points rows 1 to 5. Reviewers: kimi, codex (one still: the context menu open over a tile, the
browser holding the keyboard). Leaves out: the console (2b); a `.mid` tile's track location and
Edit in MIDI Evil (batch 4); keyboard-only entry into a panel, which no panel has yet (#144's gap);
the inspector's script rows.

**Batch 2b — the console: #207 (E5). DONE 2026-10-27, insiculous_2d b73f8a6, after code
rounds 11, 12 and 13.** Re-verified 2026-10-27 against 69d9e3f.
`editor/src/console.rs`: `ConsoleLog`, bounded to 500 entries with the oldest dropped first, each
a severity, its text, an optional location — the `OpenRequest` its click files: the path at
`Location::Line { line, column: None }`, or at `Location::None` for a line of 0 — and a repeat
count; an entry collapses into the last when its location, severity and text match with runs of
digits normalized, keeping the newest text, so an error every frame is one line whatever number it
carries and a different message at one location is a new entry. `panel_renderer/console.rs` draws
it, one arm in `render_panel_body` (`panel_renderer/mod.rs:46-68`). The reserved
`PanelId::CONSOLE` (`dock/mod.rs:40`; "no panel implementation yet" in `panel_id_for_menu_label`'s
doc, `:53-54`) becomes a panel, hidden by default, on the bottom edge: the dock lays an edge's
panels one after another and never side by side or as tabs (`dock/mod.rs:430-439`), so the console
is added after the composer in `default_dock_area` (`context/mod.rs:122-154`) and stacks above
whichever bottom panels show. View → Console is enabled (`menu/mod.rs:225` draws it disabled, the
comment at `:216-217` saying why; `panel_id_for_menu_label` maps it, `dock/mod.rs:55-63`) and
carries its check mark (the label list at `editor_game/menu_actions.rs:16`). The tests that pin its
absence (`dock/tests.rs:118-120`, `menu/tests.rs:122`) and the default layout
(`context/tests.rs:47`) change with it, and a prefs file written before it existed leaves it hidden
(the composer's precedent, `context/tests.rs:65`). What lands in it: every script error
(`editor_game/script_status.rs` shows them on the status bar one at a time; `ScriptError { file,
line, kind, message }`, `engine_core/src/scripting/runner.rs:39-44`, a line of 0 meaning none;
`file` is the `.rhai` source path, except for an unresolved native script, where it is the script
id and the entry carries no location, `runner.rs:286-291`; `ScriptErrors` keeps one per file, line
and kind a session, so the repeat count serves the launcher's lines and batch 3's reload notices),
and every launch result (the launcher's status-bar lines, its reaper's reports and the web's
desktop hint, `editor_game/external_tools.rs:46-77`). Both reach the status bar and the console
through one host function, so neither surface misses a line the other shows; the bar's other lines
stay the bar's. An entry with a location draws underlined, asks for `CursorIcon::Pointer` and opens
on click. The console holds the keyboard on 2a's model (`editor_game/asset_browser_keys.rs`): a
press of either button in the panel takes it; one elsewhere in the dock, the panel hidden or
collapsed, the composer taking the keyboard, Escape and a play session's entry (`enter_playing`,
`play_session.rs:139-148`) let it go — so it never holds while a session runs, and a click on an
entry still opens. While it holds the keyboard, Up and Down walk its entries (the focused one
scrolled into view) and Enter opens the focused one's location. The shared `ScrollState`, a Clear
button, the "Error: " prefix and the status bar's `status_error` amber for an error (the a11y rule
of #195). The status bar's current error, when it carries a location, opens on click too, and its
tooltip says so: the bar keeps the location beside its message (a message that replaces the error
drops it) and reports the click, and the host queues the request; the next frame's drain launches
it, because the bar renders after `open_requested_files` (`editor_game/mod.rs:422-424`). The
files 2b touches, measured at 69d9e3f: `context/mod.rs` 575 (a field and the dock panel; moving
`default_dock_area` into its own file is in scope if they would cross 590), `editor_game/mod.rs`
534, `panel_renderer/mod.rs` 528, `editor_game/shortcuts.rs` 523 and `menu/mod.rs` 519 each take a
line or an arm; the host's console code (its settle, its key route, the one reporting function)
goes in a new `editor_game/` file, with one `KeyRoute` arm and one call in `route_editor_key`.
Reviewers: kimi, codex (one still: the console with a clickable error and a collapsed repeat).
Leaves out: filters, log levels; batch 3's reload notices (the console takes them when they
exist); the status bar's other lines.

**Batch 3 — the watcher and code reload: #208 (E6), #209 (E7), #210 (E8), #235; #211 (E9) closed
won't-do by ruling (l). DONE 2026-10-28, insiculous_2d 895d7dc, after code rounds 15, 16 and
17.** Re-verified 2026-10-27 against b73f8a6. Jesse's two calls that day: the
watched set is what batch 3 reloads, and #235 is folded in. His calls at review, 2026-10-28:
- the worker acknowledges each noted save on its own channel, and a change for a path is dropped
  until every save of it is acknowledged, since it was read before; a save before the worker
  starts is not counted;
- a save drops the scene's change waiting for it, the parked Reload prompt included;
- Enter and Escape keep mine on the changed-on-disk prompt, and Reload is a click;
- the open scene's path reloads as the scene whatever its suffix;
- a playing clip the reloaded sheet no longer has is named on the console.

An undo across a sheet reload bringing the old cut back is #236. The issues predate rounds 1 to 5;
where they differ (a `.rs` listing, a poll on the frame) this section and § File watching and hot
reload win.
- **The watcher.** `FileWatch` (`editor/src/external_tools/watch.rs`, pure) works over
  `common::vfs::change_stamp` (`common/src/vfs/mod.rs:98`: `None` on wasm, a missing file an
  error), as § File watching and hot reload describes:
  - the registered set kept apart from the observed stamps;
  - `poll()` reporting a path once its stamp changed and then held for one further poll;
  - a gone path kept as a tombstone: reported deleted once it is absent across two polls, and kept
    as long as the host still wants it, so its return, however late, is a change;
  - a `.mid` and its `.mid.ron` settling together, by batch 1's `is_midi_path` and
    `midi_sidecar_path` (`external_tools/file_type.rs:132`, `:140`), never a rule of its own;
  - `note_written(path, stamp)` taking the stamp the host read right after its own write as the
    path's last, **and dropping a change pending its debounce**: a poll that caught the write
    before the note reports nothing, and a write by another program after it still differs from
    that stamp, so it is reported.

  A worker thread polls every 500 ms on native, so every stat stays off the frame. It takes two
  messages, a new registered set and a written path; the frame drains its events in
  `finish_frame`, and nothing runs on wasm. The tests drive `poll()` directly; the worker's own
  test waits by a bounded poll.
- **What it watches (Jesse: what batch 3 reloads):**
  - the open scene's file;
  - every non-empty `.rhai` `Scripts` source path (a native script's is empty);
  - the `.sheet.ron` of every non-empty `SpriteAnimation.sheet` (`sheet_file.rs:78`,
    `sidecar_path_for`).

  Each is resolved against the asset base. The host computes the set from the world on the
  watcher's tick and hands the worker a new set only when it differs, so there is no hook on a
  scene load or a history step to miss. The set is of paths, so many entities sharing a script
  are one entry. The host keeps the set as a map from each watched path to what named it (the
  scene, the `source_path`s, the sheet's PNG paths), and a dispatch reads its owners from the
  map. Nothing strips the asset base back off a path, which breaks on a relative base or another
  separator. The songs, the sound emitters and the sound library's
  paths join the set with their policies, in batches 4 and 5.
- **The editor's own writes.** The scene's save (`save_scene_with`, `scene_io.rs:89`, the one
  choke point) calls `note_written` after a successful write. An event already on its way when
  the save happened is dropped too: the drain discards events for a path saved since the last
  drain, since the file now holds the editor's own bytes. Tests: Ctrl+S yields no event, polled
  directly and with an event already queued. The editor writes no `.sheet.ron` and does not watch `editor_prefs.json`; the song's
  saves call it from batch 5, once its files are watched.
- **The console.** Every line goes through 2b's `report` (`editor_game/console.rs`):
  - "`<path>` changed on disk (<policy>)" as information;
  - a reload's result;
  - a reload's error at error severity, linked by `script_error_location` when it is a script's.
- **`hot_reload.rs`** (`editor_integration/src/editor_game/`) dispatches by
  `FileType::reload_policy` (`file_type.rs:103`), the first three rows of § File watching and hot
  reload's table:
  - **`RecompileScript`, while editing:** the console line only. The next session's `reset`
    compiles the new source (`runner.rs:185-194`; the backend recompiles a source whose hash
    changed, `rhai_backend.rs:64-86`), and a guard test pins it.
  - **`RecompileScript`, in Play or Paused:** `ScriptRunner::reload_source(source_path)` for each
    `source_path` the host's map gives the watched path, as the `Scripts` components name it.
    - It reads and recompiles. The backend keeps one cached unit per path and inserts only on
      success (`rhai_backend.rs:76-78`), so a success swaps every instance and a syntax error
      leaves the old unit running.
    - A success clears the path's `failed` entries and the quarantine of its instances
      (`runner.rs:129-132`), keeps the blackboard (a world resource), and leaves `frames_run`
      alone. The params need nothing: the runner composes them every frame from the cached unit's
      header defaults and the component's own values (`runner.rs:467-472`), so a changed `@param`
      default takes effect at once and a value the scene set stays.
    - `reload_source` returns its error rather than pushing it, and the host reports it.
      `ScriptErrors` keeps one error per `(file, line, kind)` a session (`runner.rs:60-94`), so a
      syntax error that came back after a fix would otherwise be swallowed.
    - A success also forgets the path's keys in `ScriptErrors.seen`, so the new source's runtime
      errors are reported fresh.
    - A runtime error on an instance's first tick after a reload quarantines it whatever its
      kind — elsewhere only a runaway loop or a blown limit does — and its message gains "reloaded script failed on its first frame — Stop and Play to reset its
      state"; the blackboard is kept on purpose (round 2, gemini F7).
    - **`runner.rs` is 597 lines**, so the reload lives in a child module
      (`scripting/runner/reload.rs`, one `mod` line), which reaches the runner's private fields.
  - **#235, folded in (Jesse):** a header error carries its script's `source_path`.
    `param_header.rs:27`, and ten more down to `:136`, build `file: "header"`, and
    `compile_fresh` passes them on (`rhai_backend.rs:94`). Fixed, the error reads
    `scripts/paddle.rhai:3: [Header] …` and links at its line. `check_source`'s `"source"`
    (`param_header.rs:151-164`) is its caller's, the playground's, which knows the path. Tests:
    one in `engine_core` on the file, one in `console_tests.rs` on the link.
  - **`ReloadDocument` (the scene).** An event acts only when its path is the open scene's, so
    one queued before Open Scene never reloads the old file over the new.
    - It waits out a gesture: while a gizmo or composer drag is live (`take_api_lines`' guard,
      `api.rs:216`) or an edit field holds the keyboard, it stays queued for the next frame
      without one.
    - Editing with a clean history: `load_scene` from disk (`scene_io.rs:156`, whose dry-run
      keeps a bad file from the live world), reported as reloaded from disk. The undo history
      goes with the old document (Jesse: no prompt for a clean scene).
    - Dirty: a new `PendingSceneAction::ReloadFromDisk` on the confirm machine
      (`scene_confirm.rs:27-37`), "scene changed on disk: Reload (discard my edits) / Keep
      mine". Keep leaves the file watched and the line in the console.
    - Play or Paused: a console line now, and the same rule once Stop has restored the world
      (a guard test, the mid-simulation pitfall).
  - **`ReloadSheet`.**
    - A transaction: the sidecar is parsed and its texture resolved before any live state
      changes, and a failure keeps the old sheet and reports (round 2, kimi F13).
    - Then `clear_sidecar_cache()` (`assets/sprite_sheet.rs:256`), and the sheet's grid, clips
      and texture go into every `SpriteAnimation` naming it (`ecs/src/sprite_components.rs:238-249`)
      together: a new cell size is a new grid. A clip's frame past the new count clamps and warns.
    - A `Sprite`'s texture changes only where it holds that sheet's old handle (a new filter is a
      new handle). A sprite showing another texture, one set while Paused and kept included, is
      left alone.
    - In Play the same, and **Stop re-applies the on-disk sheet to the restored world** (round 1,
      kimi F10). The host keeps each sheet the session reloaded (its grid, clips, and old and new
      texture handles) and applies them by the same rules after `restore_snapshot`
      (`play_session.rs:171`, inside `stop_with_paused_edits` at `:208`). Stop stays world-only
      and drivable headless, and is guard-tested with a cell-size change.
    - The re-apply is no edit and marks nothing dirty: the sidecar is the sheet's source of truth
      and wins on every load (`training.md` § Sprite Sheet Pattern), so the saved scene opens the
      same.
- **Files near the ceiling:**
  - `runner.rs` 597 (above);
  - `editor_game/mod.rs` 539: the module lines and the watcher's fields only, the host's code in
    `hot_reload.rs`;
  - `sprite_sheet.rs` 428, `play_session.rs` 347, `scene_io.rs` 317.
- **Docs:** `docs/SCRIPTING.md` (reload in a session, the first-tick rule); both crate guides and
  `engine_core`'s; pitfall rows for the watcher's baseline, the reload's error reaching the console
  past the dedup, and the sheet at Stop.
- **The manual check** (§ Verification) runs on temporary copies, since `games/pong` has no
  `.rhai`: the playground's pong project (`crates/playground/assets/projects/pong`) for the
  scripts, and breakout's `assets/` (its scenes name sheets) for the sheet.

Target shapes: § File watching and hot reload (the first three policy rows). Reviewers: kimi, and
gemini, since the watcher's debounce and tombstones, the runner's swap and Stop's re-apply are
harness and seam. Leaves out: the song and sample policies, and their paths in the watched set
(batches 4 and 5); any `cargo` spawn.

**Batch 4 — music entry points and the sample reload: #212 (E10), #215 (E13). DONE 2026-10-29,
insiculous_2d 650bdc8, after code rounds 19, 20 and 21.** Re-verified 2026-10-29 against 3f8d7fa,
the tree batch 3 left. The issues predate rounds 1 to 5; where they
differ (`{track}` as the name alone, a nameless track opening unfocused with a notice, a loop
finishing on its old bytes) this section and the design win, and the issues are reworded at
close. Four calls are the planner's, made 2026-10-29 for Jesse's word at this section's review:
- a looping instance takes the new bytes **at its next loop boundary, with no fade**: the
  recorded tier has none (the 5 ms fade is the sample voice's, `music_midi`'s), and the boundary
  is the seam the loop already has;
- a `.mid` opens at its first part's track from **one point, the host's launcher**, so the
  double-click, Enter and the context menu's item all gain it with no change to the `editor`
  crate's requests;
- a part selecting a nameless track by index opens **by the index alone**, no notice: round 4's
  `{track_index}` placeholder supersedes the issue's line;
- the emitter's button is named by the resolved tool, "Edit in MIDI Evil" / "Edit in Reaper" /
  "Edit with the system default", from the same `ToolName` the context menu's item uses;
- (round 18, gemini F1 and codex F3) a reload that changes a sound's **channel count or sample
  rate** restarts each active loop on a fresh sink at the reload instead of the boundary swap:
  rodio rebuilds its converter from the source's reported format only at a span boundary, and
  `Repeat`'s shape reports the old pass's format there, so a new format would play one span
  wrong; the same-format case keeps the boundary.

- **The scans, salvaged** (`editor/src/external_tools/songs.rs`, pure): `scan_songs` moves from
  `composer/state.rs:385` and reads `is_midi_path` (`file_type.rs:132`) in place of its own
  `eq_ignore_ascii_case("mid")` (`state.rs:399`), so a `.midi` the asset browser has listed since
  batch 1 reaches the Songs list and File → Open Song, and the song's save accepts the same
  (`plan_save`, `song/io.rs:347`, refuses every extension but `mid`; its journal already names
  by the whole name plus `.ron`, `with_suffix`, so nothing else changes — round 18, codex F1);
  `composer/sound_fonts.rs` (139 lines,
  tests included) moves to `external_tools/sound_fonts.rs` unchanged. `composer/mod.rs:49` and
  `state.rs:15` re-export from the new homes, so the five `rescan_songs` callers
  (`composer_keys.rs:350`, `:387`, `song_io.rs:71`, `:98`, `:117`) and `song_io.rs:91`'s bank
  lookup do not move; `HOME_DIRECTORIES` stays `song`'s. The composer's `songs` list
  (`state.rs:183`, refreshed on open, save and the browser's Rescan, `asset_browser.rs:77`) is
  the one song list the chooser below reads — rescanned first when `songs_scanned` is false
  (`composer_keys.rs:386`'s own call), since the list is empty until the composer has drawn
  (round 18, kimi F1, gemini F5).
- **The track resolution** (`songs.rs`, pure over `music_midi`): `Part::resolve_track`
  (`song_file.rs:303`) gives the index, `SongData::tracks()[index].name()` (`song_data.rs:329`,
  `:122`) the name, and the pair is `Location::Track { name, index }` (`config.rs:179`; a
  nameless track `name: None`, so `{track}`'s group drops and `{track_index}` carries it,
  `config.rs:314-317`). Three functions: a part's location; a binding's — action →
  `bindings[action].part` → `parts[part]` → the part's, an unknown action an error naming the
  sidecar and listing its actions, an unknown part an error naming both, a `resolve_track`
  problem passed through; and the first part's, `None` when the sidecar has no parts. The pair
  is read without the document: `common::vfs::read` on the `.mid`, `SongData::from_smf`
  (`song_data.rs:316`), the sidecar at `midi_sidecar_path` through `parse_song_file`
  (`song_file.rs:251`) — never `SongDocument::open` (`song/io/open.rs:89`), which settles an
  interrupted save and refuses a path outside the song folders. Tests over the Napolitan fixture
  (`crates/music_midi/fixtures/dansenapolitaine.mid`, the sidecar built in code as
  `song_file/tests.rs` does): an `Index` part resolves to its track's name and index; a `Name`
  part moved to another index resolves there; an unknown action names the actions; a nameless
  track gives `name: None`.
- **A `.mid` opens at its first part's track.** `open_in_external_tool`
  (`editor_game/external_tools.rs:65`) — the one point every request passes, the double-click
  (`asset_browser.rs:129`), Enter (`:134`) and the context menu's Open in <tool>
  (`context_menu.rs:98-102`) included — takes a request for a MIDI path at `Location::None`, reads
  the pair and upgrades it to the first part's location before resolving; a missing or
  unparseable sidecar, or one with no parts, opens at `Location::None` with the reason as an
  information line. A new host file holds it with the rule below (`editor_game/song_entry.rs`;
  `editor_game/mod.rs` is 549 lines, module line and fields only). Batch 2a's test that a
  double-click files one request for its path whatever the kind (`tile.rs:181`) stands: the
  upgrade is the launcher's. Tests: a `.mid` beside a sidecar launches with the track; a `.mid`
  alone launches at none and says why; a `.midi` the same; a non-MIDI request is untouched.
- **Which song is "the" song**, the design's order, on the host: during a play session the slot's
  song (`song_request()`, `manager/song/telemetry.rs:38`; the inspector pass has no `ctx.audio`,
  so the song phase that already reads the slot in `finish_frame` keeps the path on the host each
  frame), else what the game's `init` asked for (`InitMusic::Song(path)`,
  `editor_game/init_music.rs`, `mod.rs:78`), else the open document's path (`EditorGame.song`,
  `SongDocument::path`, `song/mod.rs:153`), else the composer's Songs list: one → it, several →
  a chooser, none → "no song in this project" as an error. The chooser is an
  `inspector.cycle("Song", …)` row over the list, drawn only when the rule reaches it, its choice
  kept on `EditorContext` for the session and never written to the preferences.
- **The button** (`component_editors/sound_emitter.rs:43`, the `Binding` arm): under the Binding
  row, `inspector.action_button(label)` gated as the Open source button is
  (`script_editor.rs:140-148`: `extras.can_open_source`, which is `cfg!(not(wasm32))`;
  `editable_inspector.rs:475`'s rule draws nothing live while read-only). `InspectorExtras`
  (`texture_field.rs:16`) gains the label the host computed (`ExternalToolsConfig::tool_name
  (FileType::Midi)`, as `context_menu.rs:92` does), the song list and the chooser's state, and
  `edit_binding: Option<String>`, the action the button asked for; the host builds them at
  `panel_renderer/inspector.rs:149` and takes `edit_binding` beside `open_source` at `:254`. The
  host then resolves: the song by the rule, the binding's location by `songs.rs`, and pushes
  `OpenRequest { song's .mid relative to the asset base, Location::Track { name, index } }` onto
  `editor.open_requests` — or reports the error through `report` at error severity, naming the
  sidecar. An empty binding name draws no button, and the web draws none, as it draws no Open
  source button (round 18, kimi F4: the gate hides, it does not disable). Tests: the button files
  the request for a known action; an unknown action is an error naming the sidecar; no song is
  the error above, and a list never scanned is scanned first; several songs draw the chooser and
  the choice decides; a `Path` emitter draws no button; the web draws none.
- **The recorded tier reloads behind its handle.**
  - `AudioManager::reload_sound(handle, bytes) -> AudioResult<()>` (`manager/mod.rs`, 484
    lines): the new bytes are decoded once, on the frame thread, and **pulled to the end** —
    `Decoder::new` parses headers alone (`:76`), so a stream that fails or yields no sample is
    refused as `DecodeError` with the old bytes kept, while a shorter body than before is the
    author's shorter sound (a half-written file is the debounce's to hold back — round 18, kimi
    F3); an unknown handle is `InvalidHandle`; on success the `SoundData.bytes` Arc is replaced.
    A one-shot sink already playing holds the old Arc and finishes on it. A looping instance
    plays through a repeat of the manager's own, `audio/src/loop_source.rs`: rodio's `Repeat`
    (`repeat.rs`, `inner` and `next` as `Buffered` clones of one decode) cannot be handed a
    second source, so the manager's keeps the same two fields and a slot
    (`Arc<Mutex<Option<Buffered<Decoder<…>>>>>`) it `try_lock`s only when `inner` runs out:
    a filled slot becomes `next`, `inner` its clone, so every later pass is the new sound (round
    18, gemini F2 asked); `WouldBlock` — the frame thread is filling it — plays the old `next`
    once more and tries again at the following boundary, so the loop never stops (round 18, kimi
    F2). The reload compares the new decode's channels and sample rate with the old's: equal →
    the slot of every active loop of the handle is filled, and each takes the new sound where its
    old pass ends; different → each active loop is restarted on a fresh sink at the reload under
    its own `PlaybackId`, pan, volume and speed (`start_playback`, `:355-383`, over the entry's
    fields), with a console line saying the format changed — the fifth call above. `ActiveSound`
    (`:53-66`) gains the slot, `None` for a one-shot. A held loop (the disabled manager's,
    `playback.rs`) needs nothing: `start_held_loops` decodes the bytes current when the device
    arrives. Tests in a new `manager/reload_tests.rs` (`manager/tests.rs` is 573 lines): the
    pure source — pulled past its boundary with the slot filled, the samples after it are the new
    source's and the pass before it is uncut, and the pass after that is still the new source's;
    an empty slot repeats as before; a slot held locked at the boundary plays the old pass again
    and takes the new one at the next — and the manager headless: a reload keeps the handle valid
    for `play`, an unknown handle is refused, a header-only or empty stream keeps the old bytes
    (the test reads the private field, as `tests.rs` does), a format change is told apart from a
    same-format reload.
  - `AssetManager::reload_sound(path, audio)` (`assets/sound_library.rs`, 146 lines): a path in
    `loaded` → the bytes read through the asset base and `audio.reload_sound(handle, bytes)`, the
    handle unchanged so every `resolve_sound` caller (`sound_emitters.rs:363`) and emitter stays
    valid; a path in `failed` → the failure forgotten and `resolve_sound` run again, a success
    serving the handle from then on, a failure remembered again; a path in neither → nothing to
    do, reported so. `sound_paths()` lists `loaded`'s and `failed`'s keys for the watched set.
    Tests beside the existing two: a changed file is served from the same handle; a failed path
    that appears is retried once and serves; a reload that fails to decode keeps the old sound
    and the path loaded.
- **The watched set gains the sounds** (`hot_reload.rs:120-141`, `watched_paths`, which now takes
  the asset manager too — `tick_file_watch` and the tests' `watch_now` with it): every non-empty
  `SoundEmitter { sound: Path(p) }` (`ecs/src/sound_emitter.rs:21`) and every path
  `sound_paths()` names, as `WatchOwner::Sound(p)`, each through `resolve_asset_path`. A failed
  path registers as **expected absent**: `FileWatch::set_registered` (`watch.rs:87`) takes,
  beside the set, which paths the host believes missing, and seeds their observation as absent,
  so a file present at the worker's first look is a change — left to the plain first observation
  (`poll_with`, `:115`, a baseline with no event) a file that arrived between the failed load and
  the first poll would stay failed for the session (round 18, codex F2). The worker's set message
  carries the marks. The mirror window — a loaded sound's file rewritten between its load and the
  watcher's first observation — is accepted and documented beside the stamp granularity. A song's
  samples and SoundFonts are batch 5's closure and are not watched here; a `.wav` nothing names
  and nothing loaded is not watched, and one an emitter names that nothing has played needs no
  reload when it arrives — the next play reads it (round 18, gemini F7). Tests: an emitter's
  sound and a loaded path are watched, a `.wav` nobody names is not, a failed path whose file is
  present at the first poll is reported as a change.
- **`ReloadSample`** (`dispatch_file_change`, `hot_reload.rs:186-224`; `ReloadTargets` at `:73`
  gains `audio: &mut AudioManager` from `ctx.audio`, `contexts.rs:66`): for each
  `WatchOwner::Sound(path)`, `assets.reload_sound(path, audio)`, its result a line through
  `report` — reloaded (the format change named when there was one), now loads, not loaded yet,
  or the error at error severity with the old sound kept. Editing, Playing and Paused alike; nothing at Stop, since a sound is not in the
  snapshot. Tests in a new `editor_game/hot_reload_sample_tests.rs` over `Session`
  (`hot_reload_tests.rs` is 577 lines): a changed `.wav` is served from the same handle after the
  dispatch; a failed path that appears serves after it; a bad file keeps the old sound and
  reports; the same in a play session.
- **Rebutted at round 18, so not rebuilt:** the song is the one that plays, so an action another
  song defines is an error naming the sidecar and listing its actions, never a search of every
  sidecar (gemini F3); the track index is `resolve_track`'s, bounds-checked at
  `song_file.rs:310-312` (gemini F4); the pair is read at the resolved path (gemini F6).
- **Files near the ceiling:** `hot_reload_tests.rs` 577, `manager/tests.rs` 573, `editor_game/
  mod.rs` 549, `song_preview.rs` 591 (the slot's path is read where the song phase already reads
  it; a new line or two, or the reading moves into `song_entry.rs`), `manager/mod.rs` 484,
  `asset_browser.rs` 433.
- **Docs:** `crates/audio/CLAUDE.md` (the reload and the loop source); `crates/engine_core/
  CLAUDE.md` (the sound tier's reload, and a pitfall row: a failed path is retried when its file
  arrives); `crates/editor/CLAUDE.md` (`external_tools/` for `songs.rs` and `sound_fonts.rs`,
  the composer row's scan, the emitter editor's button); `crates/editor_integration/CLAUDE.md`
  (the launcher's upgrade, `song_entry.rs`, the hot reload's sounds, and a pitfall row: a loop
  takes its new bytes at the boundary, a one-shot finishes on the old).
- **The still**, for codex: the emitter's settings with the button and a console line from its
  click, on a temporary copy of `../games/breakout/assets` (its scenes, and
  `ai/music/ai_food_pyramid.mid` with six bindings: `brick`, `armor`, `paddle`, `loss`, `clear`,
  `menu`) — no shipped scene carries a `Binding` emitter, so the executor adds one in the editor.
- **#180:** the sound half closes with this batch, the texture half stays; the planner updates the
  issue at the commit.

Target shapes: § Entry points rows 1 and 6, "Which song is the song", the `ReloadSample` row.
Reviewers: kimi, gemini (the sound library's reload, the loop source's seam), codex (the still).
Leaves out: the song swap and the closure (batch 5); the Conductor rename and the panel's reads
of the salvaged scans beyond the re-exports (batch 7); any change to how a tile's request is
built in the `editor` crate.

**Batch 5, split at its re-verification (Jesse, 2026-10-29, as batch 2 was): 5a the song
document follows the score, 5b the audio seam, 5c the host's song reload.** Reviewed as round 22
(rebuttal-22.md). Re-verified against the tree batch 4 leaves (dafb9b7 plus batch 4 staged). The one
section held three seams nothing forces together — the `editor` crate's document, the
`audio`/`music_midi` swap, and the host's wiring — and the first two do not depend on each
other, while the third needs both. `manager/song.rs` is **603 lines**, over the ceiling: 5b
splits it before it adds a line. Order: 5a → 5b → 5c, one out at a time; the listen check closes
5c. Where the one section's text and the three below differ, these win.

**Batch 5a — the song document follows the score: #214's document half, the `.midi`
remainder, `song init`, the track diff. DONE 2026-11-02, insiculous_2d 21e05ab, after code
rounds 23 to 27** (the dirty rule's lesson: a flag derived from the history is wrong three ways —
a merged gesture, the cap's eviction, an undo past a save — so the score is the engine's exactly
while the data's export differs from the export at its last load or write; a score the data
cannot export is the file's for good, `EditingPolicy::for_score` the one rule). The `editor`
crate, the host's save and the web upload; no audio. § The open song document follows the score is the target shape, and § Manifest
validation on reload names the diff's lines.
- **The document keeps the bytes it loaded.** `SongDocument` (`song/mod.rs:94-104`) gains
  `source_bytes: Vec<u8>` and two fingerprints — the `.mid`'s and the `.mid.ron`'s as last read
  — set by `document_from` (`song/io/open.rs:259-270`) and by every save and adoption. A **map
  save** writes the sidecar and the home `LICENSE` stanza and leaves the `.mid` alone: `render`
  (`io.rs:279-281`) fingerprints `write_smf()` today, which can differ from the file's bytes
  (`music_midi`'s own pitfall: byte-identical only when no status byte repeats); the map save
  fingerprints `source_bytes`, with a test on a fixture whose export differs (`bach-invention-02`
  or whichever `write_smf` rewrites — the test asserts the inequality first). **Save As** (another
  target through `save_song_to`, `editor_game/song_io.rs:62`, and `song save <path> [overwrite]`,
  `:183-203`) writes `source_bytes` and the map — the one case the engine writes a `.mid` — an
  existing target still needing `overwrite`; in the transition a document whose score the engine
  edited writes `write_smf()`'s bytes instead, and `source_bytes` and both fingerprints become
  what was written (round 22, kimi F1: a Save As of an edited score wrote the loaded one).
- **The transition's score-authoring rule.** A score edit — `AddNote`, `DeleteNotes`,
  `MoveNotes`, `ResizeNotes`, `SetVelocity`, `SetPitch`, `AssignPart`, `SetTempo`,
  `SetTimeSignature`, `AddTrack` (`song/edit.rs:35-73`, `:115`), and a `Batch` with any of
  them in it, members inspected recursively (round 22, kimi F2: a drag's `Batch` would have
  passed as a map edit); the rest are map edits — marks the document's score as the engine's. Its save writes the `.mid` only after comparing
  the on-disk `.mid`'s fingerprint with the one loaded or last written, immediately before the
  finals; a difference keeps the journal's score as `<stem>.recovered-<fingerprint>.mid` with
  a notice and leaves the disk's `.mid`. The window between the compare and the write is
  accepted and written down (ruling (i)).
- **The two files guarded separately, by content.** `SongDocument::adopt_score(bytes)` —
  the new `.mid` taken whole, the map and its unsaved edits kept, `EditingPolicy` recomputed
  from the new report, the history cleared of score entries (a note edit against a score that
  is gone cannot replay), and the track diff run against the old `SongData`, its lines returned
  — and `SongDocument::reload_map(text)`, the sidecar re-read. **Immediately before each
  write** `save_to` (`io.rs:314`) compares both fingerprints again through the store: a changed
  score is adopted first — for a map-only document; a document whose score the engine edited
  takes the transition's rule above instead, its score kept as the recovery, so no edit is lost
  to an adoption (round 22, gemini F1) — and the save then fingerprints the disk's bytes; a
  changed sidecar refuses with `SongSaveError::SidecarChanged { sidecar, loaded, on_disk }`,
  which the host turns into "the map changed on disk since you loaded it: Overwrite / Cancel" on
  the existing `PendingSceneAction::OverwriteSong` prompt (`scene_confirm.rs`) and the API into a
  structured `refused` naming both files and each one's state (`song_io.rs:196-201` says "the
  save was parked" today). **The overwrite is an intent `save_to` takes** — `SaveIntent {
  sidecar: Keep | Overwrite }` — carried by the prompt's Overwrite (`scene_confirm.rs:188-192`
  calls `save_song_to` with nothing today) and by `song save <path> overwrite`, so both routes
  pass the compare they asked to pass (round 22, kimi F5); a successful overwrite is tested on
  each. The watcher's
  side — when the host learns a file changed — is 5c's; 5a's tests drive the two methods and
  the compare directly, same-stamp and write-during-commit included.
- **The diff is pure**: `editor/src/external_tools/track_diff.rs` over two resolved part lists
  (`part_location`'s shape, batch 4), by each selector's own semantics (§ Manifest validation):
  unchanged selector, changed identity → refused naming both; a track new and unselected → "plays
  as background"; gone with a new name at its index → the rename hint; a `Name` now ambiguous →
  the `Index` hint; a changed selector → accepted and reported. Tests over the Napolitan fixture
  and a renamed copy built in the test with midly (the hand-made pair, until MIDI Evil ships one).
- **Map edits on any playable MIDI.** `apply` (`song/mod.rs:173-178`) refuses every edit under
  `EditingPolicy::Refused` (`:65-72`); it refuses the score edits above only, so parts,
  instruments, bindings, loop, quantize, modes, provenance and Save work on a mixed-channel file
  (a two-channel-track fixture built in the test).
- **Recovery keeps, never promotes.** `open_from` (`song/io/open.rs:100-158`) promotes a `.new`
  pair (`recover_pair`, `:161-173`), refuses `MismatchedFinals` (`:53`, `:130-136`), opens a
  differing lone `.mid.new` as an import and removes a lone `.mid.ron.new`
  (`open_without_finals`, `:204-242`). Replaced by one rule: any journal file is renamed to
  `<stem>.recovered-<fingerprint>.mid` / `.mid.ron` — the compound suffix split by
  `midi_sidecar_path`'s inverse, never `theme.mid.recovered-….ron` — under a name that never
  collides, with a notice naming it; the finals open as they are; a fingerprint that differs
  from the `.mid` with no journal means the score changed outside: opened on the new bytes with
  its sidecar, the status bar saying so, the diff's lines shown, the next Save rewriting the
  fingerprint. **With no final at all** — a first save interrupted before them — the journal is
  the song's only incarnation, and it becomes the finals: nothing is replaced, which is what
  ruling (j) forbids, and a note says the save was finished (round 22, gemini F2: the rule as
  written would have renamed the only copy away and opened nothing). `open_tests.rs` (`:37-200`, seven recovery tests) is rewritten to the rule: one
  test per journal shape (the pair, a lone sidecar, a lone `.mid.new`, a journal beside an
  externally changed `.mid`). **The prune moves after the finals**: `commit` (`io.rs:448-470`)
  prunes at `:462`, between the journal and the finals; it prunes after `:467` and the journal's
  removal, and every `<stem>.recovered-*.mid.ron` beside the song is a root (an interruption →
  recovery → save → open-the-recovery test; `every_sample_the_reopened_song_names_is_on_disk…`
  rewritten).
- **`song init <path.mid>`** (`command_api/song.rs:82-96` has `new|open|close|save`): a default
  sidecar — the project's first bank, no parts, no bindings, `looping: true` — written beside an
  unmapped `.mid`; the panel opens an unmapped `.mid` the same way, in place, the sidecar marked
  unsaved (today `open_without_finals` imports it with `default_song_file`). `song open` is the
  reload; the API follows every rule above, tested over `--headless`.
- **`.midi` everywhere batch 1's helper is not yet used:** `with_suffix(.., ".ron")` at
  `io.rs:335`, `:411`, `:456`, `open.rs:76`, `:247`; `song_files_exist` (`song_io.rs:245-250`,
  pushes `.ron`); the playground's `upload.rs:176`, `:193`, `:202` (its own `with_suffix`),
  `:139` (`.ends_with(".mid.ron")`), `:152` (`trim_end_matches(".ron")`) and `:38` (`"mid"`
  alone) — the playground depends on `editor` (`playground/Cargo.toml:12`), so `is_midi_path` and
  `midi_sidecar_path` reach it. **The web re-import keeps the sidecar** (`upload.rs:191-196`
  removes it and both journals today): a re-imported `.mid` is the score changed outside, the
  map kept — `sidecar_lost` becomes "the score changed; the map is kept" — with an end-to-end test
  that the notes change and the map survives (round 5, kimi F1 — the table row at § The
  Conductor panel, which no batch had claimed). A re-import of **the open song** stays refused
  here (`upload.rs:184-188`) and is lifted by 5c, which routes it through `adopt_score` — the
  web has no watcher, so the import is how the open document learns the score changed (round
  22, kimi F6).
- **Docs:** `docs/SONG_FORMAT.md` § How the composer saves (`:302-360`) rewritten — a map save,
  Save As, the transition's rule, recovery keeps; `docs/EDITOR_COMMAND_API.md` § Songs
  (`:190-191`) — `song init`, the conflict's structured error, `song open` as the reload;
  `crates/editor/CLAUDE.md`'s `song/` and `external_tools/` rows; pitfall rows citing the
  migration tests.
- **Files near the ceiling:** `song/io.rs` 505, `song/mod.rs` 320, `open.rs` 302,
  `editor_game/song_io.rs` 251, `upload.rs` 209; a rewrite of `open.rs` may want `io/recover.rs`.

Target shapes: § The open song document follows the score, § Manifest validation on reload.
Reviewers: kimi; gemini (the journal's recovery and the prune order are a seam; the
write-during-commit fixture). Leaves out: anything the watcher reports (5c); the audio swap
(5b); the panel's Save dispatching a reload (5c); the composer's rename (batch 7).

**Batch 5b — the audio seam: #213 (E11), the silent part, the example's flag. DONE 2026-11-02,
insiculous_2d 26eb737, after code rounds 28 and 29** (what the rounds taught: a host seek is told
landed by its ordinal among the host's commands against the conductor's `commands_applied` — the
sequencer's `reconstructions` also rise on a wrap or a resume, and two seeks before a block renders
collapse — and the re-seed waits one reading past that, since the conductor publishes its counters
before its position; the swap waits while a seek is in flight and the landing takes a stationary
song's or a staged preparation; the fresh conductor publishes zeros until its first block, so the
queries answer from the landing until then). `audio` and `music_midi`; no host wiring. § MIDI
reload: swap at the next bar is the target shape, § Appendix held.
- **`manager/song.rs` is 603 lines**: before anything else it splits — the transport and the
  queries (`stop_song` … `beat_crossed`, `:352-478`, 127 lines) to `song/transport.rs` as a
  child module reaching the private fields, which leaves `song.rs` at about 476 — its own
  hunk, behaviour-free, the first thing the report's gates show green; the swap then lives in
  `song/replace.rs` with its tests in `song/tests/replace.rs` (`song/tests.rs` declares
  `loading` and `telemetry` the same way).
- **The silent part** (`song_file/validate.rs:96-103`, `EmptyTriggerPart`): a selected track with
  no notes is a part whose every trigger is a rest — `triggers.rs` already counts `silent`
  (`:79`, `:252`; `position.rs:64` publishes it; `triggers/cursor.rs:31` already names "a part
  with no notes" a rest) — so the refusal goes and a part with no units rests; a score with no
  notes at all (`BackgroundScore::end_tick`, `sequencer/score.rs:247`, reached by
  `song.background()`, is 0) is refused as "the score is empty". Tests in `music_midi`: the
  refusal's case in `song_file/tests.rs:268-271` ("parts['lead'] plays track 0, which has no
  notes") flips to an acceptance, and a trigger on the silent part counts in `silent`.
- **The worker and the preparation.** `AudioManager::replace_song_at_seam(path)` and its
  in-memory twin prepare on a worker thread exactly as `play_song` does (`prepare_song`,
  `:226`; `DependencyCache::prepare`, `load.rs:237`): the worker prepares on a **clone** of the
  cache (`SongDeck.cache`, `:84-86`; `DependencyCache` gains `Clone`, cheap: its `CachedFile`s
  hold an `Arc<SoundFont>` with its `PresetLayering` — a `BTreeMap<(u16, u8), usize>`,
  `load.rs:157` — or an `OwnedPcm`, whose samples are an `Arc<[f32]>`, `pcm.rs:29`; the two
  closure sets and the decode count copy) and hands it back with the result, merged on the
  frame thread, so `play_song`
  never waits on a preparation (round 22, gemini F3: a lock held through a 48 MiB decode would
  stall the frame it exists to spare); the result is `PreparedPlayback { request_id, generation,
  ready: ReadySong }` (`ReadySong`, `load.rs:65`, already carries the song, the conductor, the
  handle and a start tick) over a channel polled from `update_song` (`:552`), parked as
  `pending_replacement`; request ids monotonic, an older result or failure dropped unread, a
  result for another generation dropped the same way; superseded and retired sources dropped on
  the worker. **On wasm32 there is no thread**: `audio` is in every game's web bundle and
  `std::thread::spawn` panics there, so the worker is `cfg(not(target_arch = "wasm32"))` and on
  wasm32 `replace_song_at_seam` prepares on the frame at the call, as `play_song` does today,
  and parks the same `PreparedPlayback` — the swap's code is one path either way (the watcher
  makes the same cut: `hot_reload.rs:113` returns on wasm32). `scripts/check_wasm.sh` and
  `scripts/check_games.sh`'s wasm checks are the gate.
- **The swap**, in `update_song` on the first update where it is due: the old transport's
  position as the bar and the fraction (`TempoMap::position_at_tick`, `tempo_map.rs:214`, gives
  bar and beat; a `bar_span(bar) -> (start, end)` is added beside `tick_at_beat`, `:225`, with its
  own tests), the fraction applied to the new map's same bar — the design's order verbatim: the
  fraction clamps inside the new bar, a looping song takes the bar modulo the new song's bar
  count, and a running non-looping song whose mapped position is at or past the new end refuses
  the swap (round 22, gemini F8 asked for the sentence); the new source built as `start_song` builds it (`:274-312`: `ConductorSource::new`,
  `song_output`) with the old playback's `requested` transport (`:145`), `pending_lifecycle`
  re-sent, `path` and `label` kept, the new `Arc<Song>`, binding table and `BeatEdges`. **When
  due**: a running transport on the first update whose `edges.crossed(Bar)` (`edges.rs:74`) is
  true since the preparation arrived — the host keeps the `(sample_position, loop_wraps)` the
  snapshot held when the preparation arrived, and a seek the host sent re-seeds it; `BeatEdges`
  has no reset today, and seeding its `seen` with `None` would report a crossing on the next
  update (`edges.rs:100`, `None => true`), so the re-seed is a `BeatEdges::reseed(snapshot)`
  that takes the current reading, never `None`; a backward seek reports nothing by the edges'
  own rule (`:102`), a forward one counts, which is why the host's seek re-seeds — a stationary
  one at once,
  a loop shorter than a bar at its wrap, the end-while-pending, stopped-at-end and staged-refusal
  rules as the design says. `play_song`, `stop_song` and the session's end drop a pending
  preparation. **The host learns the outcome by draining** (added at the re-verification,
  2026-11-02: the design names `ReplaceRefused { reason }` "to the console" and the example
  logs the swap, and neither had a path out of the manager): `AudioManager::take_song_replacements()
  -> Vec<SongReplacement>`, `Swapped { bar, position: SongPosition }`, `Refused { reason }`,
  `Failed(AudioError)` (the worker's refusal, the sidecar entry and path named as `play_song`
  names them), `Dropped` (superseded or dropped by a stop) — the example drains it each frame;
  5c routes it to the console and the status bar.
- **`conductor_background --replace-after <bars>`** (`examples/conductor_background.rs:97-130`,
  `Settings::from_arguments`, parses `--fixture`, `--background-only`, `--sample`, `--seconds`;
  the file is 431 lines): after that many bars,
  `replace_song_at_seam` to the cast fixture (`dansenapolitaine-cast.mid`, the same tracks on
  other programs), the swap's block and bar logged.
- **Tests**, through the public API on the virtual clock (`song/tests/`, a new `replace.rs`):
  the design's list — a reordered binding, an added action, a tempo change, a 4/4 → 3/4 change,
  a ticks-per-quarter change, seeks before and after the preparation, a replacement over a
  stopped song and over one stopped at its end, a second preparation before the first swaps (B
  finishing before A), a shorter looping song, a bank-only edit, the one-bar loop. The timing
  fixture: the largest shipped instrument set (Chicken Coop's five,
  `../games/frogger/assets/ai/music/ai_chicken_coop.mid` with its sidecar, five SoundFont
  instruments — Bratdog's, `../games/snake/assets/ai/music/ai_bratdog.mid`, has five too; the
  Food Pyramid's four, Tong's two), the worker's preparation measured under `cargo test
  --release` and quoted in the report. The tests drive the clock with `update_at`
  (`manager/mod.rs:486`, `pub(crate)`) as `song/tests.rs` does; `SongOutput::Detached`
  (`song.rs:130`, test-only) stands for a device dropping the source. Near the ceiling:
  `song/tests.rs` 477 (which is why the new tests are a child file), `load.rs` 467,
  `manager/mod.rs` 501, `manager/tests.rs` 573.
- **Docs:** `crates/audio/CLAUDE.md` (the `manager/song.rs` row and a row for each new file; the
  worker, the swap and its cost as § MIDI reload says it; pitfall rows for the swap's rules,
  each citing a test), `crates/music_midi/CLAUDE.md` (the silent part), `docs/SONG_FORMAT.md`
  (`:228` lists "a part whose track has no notes" among `validate`'s refusals; it becomes the
  silent part, and the empty score joins the list), `training.md` § Conductor Pattern (`:926`; a
  sentence: a song reloaded at the bar).

Reviewers: kimi, gemini (the audio seam). Leaves out: every host change (5c); the listen check,
which needs 5c; the audio-thread swap (§ Appendix).

**Batch 5c — the host's song reload: #214's host half, the watched closure, the panel's Save.
DONE 2026-11-02, insiculous_2d 82b6957, after code rounds 30 and 31** (what the rounds taught: the
watcher's absent marks come from the loader's own knowledge of the file it could not find, never
from a stat at the tick — a file arriving inside the poll interval was a silent baseline, and a
mark once set latched; an external adoption stops a sounding preview explicitly, since a clean
document's history revision does not move; the listen check, § Verification "After batch 5", is
Jesse's and still open — a gap he hears sends § Appendix to a batch).
`editor_integration`, over 5a's document and 5b's seam. § File watching and hot reload's
`ReloadSong` row and the paragraph before the table are the target shape — minus the sentence
"`note_written` silences only the watcher … dispatches the song's reload itself", retired by
Jesse's ruling of 2026-10-29 (below). **Re-verified 2026-11-02 against the tree 5b leaves:**
`hot_reload.rs` is 333 lines — `WatchOwner { Scene, Script, Sheet, Sound }` (`:45-55`),
`HotReload` (`:58-78`), `ReloadTargets` (`:86-93`, with `audio`), `drain_file_changes` (`:97-110`,
called from `finish_frame`, `mod.rs:420-432`, after `apply_song_intents` and the dirty mirror),
`watched_paths` (`:137-166`, no song yet), `expected_absent_paths` (`:169-171`), `note_scene_written`
(`:177-194`), `dispatch_file_change` (`:211-263`: `ReloadSong` has no arm and falls to `_ => {}`;
`policy_label` already says "reload song") — so the song's arm is a child, `hot_reload/song.rs`,
as `sheet.rs` and `sample.rs` are. `song_io.rs` is 330: `song_change_refusal` `:42-46`, `save_song`
`:63`, `save_song_to(target, intent)` `:80-91`, `write_song` `:97` (where a successful save notes
its files), `open_song` `:156-174`; the API's refusal is `command_api/song.rs:19`
`song_write_refusal`. The session plays the **saved** song by path (`play_armed_song`,
`song_preview.rs:493`, `audio.play_song(path)`), so `song_request()` names it; the Editing preview
plays the document in memory (`:452-453`) and owns no files but the document's two. **The manager
exposes neither the playing song nor the closure yet**: 5c adds `AudioManager::playing_song() ->
Option<&Arc<Song>>` (its `file()` and `data()`, `conductor/song.rs:141-145`, are what the diff
reads) and `song_closure()` over `DependencyCache.current_closure` (`load.rs:219`) plus the paths a
failed preparation attempted — `prepare` keeps `touched` only on success (`:267`, `:285-286`), so the
cache records `last_attempted` on every call, the worker's copy carrying it back (`Preparation.cache`
arrives on failure too, `replace.rs` `take_in`). The diff is `editor::external_tools::{diff_tracks,
TrackListing::of(file, data), TrackDiff::refuses, texts}` (`track_diff.rs:17-96`). The scene's
"Reload / Keep mine" is `PendingSceneAction::ReloadFromDisk(PathBuf)` (`scene_confirm.rs:40`,
routed by `resolve_scene_reload_choice`; Enter and Escape keep mine); the song map's prompt is a
sibling variant. The web: `EditorGame.open_song_mirror` (`mod.rs:136`) is read by the bridge
(`bridge.rs:179-181`) into `import_file(project_root, name, bytes, open_song, replace)`
(`upload.rs:171-197`), whose refusal of the open song's `.mid` (`:189-191`) goes; the inverse channel
is a mailbox beside the mirror in `EditorRunOptions` — `(path, bytes)` the bridge fills and
`finish_frame` drains before `drain_file_changes` into `adopt_score`. A `.mid` and its `.mid.ron`
arrive as two `WatchEvent::Change`s in one poll (the watch settles them together): the host
dispatches the song once per owner per frame. Near the ceiling: `song_preview.rs` 592,
`hot_reload_tests.rs` 580, `bridge.rs` 569, `mod.rs` 557, `song_io_tests.rs` 434 — the new tests are
`hot_reload_song_tests.rs`.
- **The watched set gains the songs** (`hot_reload.rs`, `watched_paths`): the open document's
  `.mid` and sidecar (`EditorGame.song`, `SongDocument::path`); the session song's closure —
  `AudioManager::song_closure()` (above) plus the `.mid` and the `.mid.ron` of `song_request()`;
  every path a failed preparation attempted (`last_attempted`, above), the missing ones marked
  expected absent (batch 4's mark); the closure re-read on the watch's 500 ms tick, which already
  re-registers when the keys or the marks differ. `WatchOwner::Song(SongOwner::Document |
  SongOwner::Session)`.
- **Every song save is noted** as the scene's is: `note_scene_written` (`hot_reload.rs:177-194`)
  splits into the generic `note_written(path)` — the worker told, the save counted until its
  `Noted`, a change for the path dropped until then, a save before the worker starts not counted
  — and the scene's own parking; `write_song` (`song_io.rs:97`, the save every route takes) notes
  its files after a successful commit — the sidecar always, the `.mid` when the save wrote it —
  and drops a parked map prompt for the song as the scene's note drops its; the web import notes
  nothing (no worker).
- **`ReloadSong` dispatch**: the companion debounce is the watcher's (batch 3); a change in the
  open document's files → `adopt_score` / `reload_map` (5a), a clean map reloaded, a dirty one
  prompted through `ConfirmDialog` ("the map changed on disk: Reload / Keep mine"), the diff's
  lines to the console and the first to the status bar, while Editing too; a change in the
  session song's closure → **the diff first**: the old session song's resolved parts (the
  playing `Arc<Song>`'s `SongData` and sidecar, which `AudioManager` exposes) against the new
  pair read at the path, through `track_diff` — a refusal line keeps the old song playing and
  says why, an accepted diff goes to `replace_song_at_seam` (5b), whose own `validate` refusal
  reaches the console the same way; the open document adopts the score either way (round 22,
  kimi F4: `validate` cannot see an `Index` part whose track changed identity). **The session is
  read-only, as it has been** (`song_change_refusal`, `song_io.rs:21-26`; the API's refusal at
  `:149-153`; the Conductor effort's guard test `playing_and_paused_refuse_every_composer_mutation…`):
  a map repair is an Editing-time act — Stop, fix the selector, Save, Play — and the next session
  plays the saved song, so no save dispatches a reload and nothing is lost to `note_written`'s
  silence; the mid-session swap is the external writer's case, which is the feature. This
  retires the "panel's own Save dispatches the replacement" rule of § File watching and § The
  open song document (round 3's gemini F1, codex F2), which assumed a save during Play that the
  session refuses (round 22, kimi F3 found the contradiction; gemini F5 would have moved the
  dispatch into the common save path, which the retirement makes moot) — Jesse's ruling,
  2026-10-29. **The web's import** of the open song is accepted and routed to
  `adopt_score` through a mailbox the host drains in `finish_frame`, since the web has no
  watcher (5a keeps the refusal until here). Play and Editing alike for the document; the
  session only while one runs.
- **The listen check** (§ Verification, "After batch 5") closes this batch, with its repair
  step read under the rule above: the renamed track's refusal shows during Play; Stop, fix the
  selector in the panel, Save, Play → the fixed song plays; a binding changed and saved while
  Editing after an export keeps the exported notes.
- **The outcomes drained**: `take_song_replacements()` (5b) every frame in `finish_frame`, after
  `apply_song_intents` — `Swapped { bar, .. }` an information line naming the bar, `Refused` and
  `Failed` error lines, `Dropped` nothing — through `report`, so the bar and the console agree.
- **Docs:** `crates/editor_integration/CLAUDE.md` (`hot_reload.rs` — its "a song's samples and
  SoundFonts are not watched yet" goes — and `song_io.rs`), the pitfall rows; `crates/audio/CLAUDE.md`
  (`playing_song`, `song_closure`); `training.md` § Conductor Pattern's "In the editor" paragraph
  (a saved song changed on disk during Play is heard from the next bar); `docs/SCRIPTING.md`
  unchanged.

Reviewers: kimi, gemini (the note-written widening and the closure are harness and seam). Leaves
out: nothing of batch 5 — the three together are the one section above as reviewed.

**Batch 6 — the settings panel: #216 (E14). DONE 2026-11-03, insiculous_2d 50962e2, after code
rounds 32 to 38** (what the rounds taught: the panel's walk reads the ui's focus once at its start
and applies a cycle's choice after the row's fields draw — the ui keeps a focus until the field is
drawn, and an earlier field's click takes it before a later field commits; a failed Enter keeps the
field, revealed with its error row, while a failed press away lets go, since a re-focus there held
the game's keys when the press was on Play; a program named in a preset's place skips the preset's
fallback locations and keeps the preset for its diagnostics; a project request and a Test resolve
through the category's choice, overrides being a file's; two side panels on one edge stack their
narrow tabs, each holding its slot; a fixes snapshot stages a new file first or the reviewers read
it as missing). `settings_panel.rs` over the inspector's widgets: a
cycle row per category for the preset, a string field for the Custom template with the inline red
invalid state, per-type overrides, a preset's program field (MIDI Evil's binary among them), the
DAW project extensions, a Test button per category; Edit → External Tools… (every launch error
since batch 1 points there) and, beside it, Edit → Open Project (§ Entry points row 7, claimed by
no batch until now); `ToolChoice::Custom`'s second, project field reading batch 1's on-disk shape
`{"Custom": "<template>"}` still, as a project-less custom tool (a test loads that JSON);
keyboard-walkable with a visible focus ring, labelled controls, no colour-only state; narrow mode
as the dock's overlay. Closes #216 and #102. Target shapes: § Types and the re-verification below.
Reviewers: kimi, codex (two stills: the panel at rest; a template error shown). Leaves out:
nothing of the brief's settings.

**Re-verified 2026-11-02 against 24b27b2 (the tree batch 5c left).** Where this and § Types
disagree, this wins. The handoff carries the `file:line`s.
- **What exists.** `ExternalToolsConfig` (`external_tools/config.rs`: the four category choices,
  `overrides`, `daw_project_extensions` defaulting to `.evil .prj .nsp` — ruling (d); #216's body
  says "empty by default" from the deleted note, and the plan wins); `ToolChoice` externally
  tagged (`{"Preset": "Zed"}`, `{"Custom": "<template>"}`); `resolve` refusing `Location::Project`
  for Custom and OsDefault with `NoProjectCommand`, whose message — and the launcher's not-found
  sentence — already sends the user to "Edit → External Tools…", an item no menu has;
  `CommandTemplate::validate` and `names_file` are the field's check; `PRESETS` (eight rows with
  `category`, `project_template`, `fallback_locations`, `needs_terminal`) is the cycle rows' data.
  No settings panel, no `PanelId` past `COMPOSER = 5`, no caller of `Location::Project` outside
  the launcher.
- **The program field.** No row carries a program override: MIDI Evil's `midi-evil` is `cargo run
  -p midi-evil` today and Aseprite's Steam path is a fallback location. So the "MIDI Evil binary
  field" is a per-preset program: `ExternalToolsConfig.programs: BTreeMap<PresetId, String>`
  (`#[serde(default)]`; a path or another name), applied by `resolve` after expansion as
  `Launch.program`, the preset kept for its diagnostics and the launch marked `named_program` for a
  name other than the preset's own bare name, so the launcher checks a path as a file and a name on
  `PATH` and skips the preset's fallback locations — its own binary's — for it (the rounds' correction
  of this section's first cut, which kept the fallbacks). Drawn under any preset row, blank meaning
  the preset's own.
- **The Custom tool's project field.** `ToolChoice::Custom(CustomTool)`, `CustomTool { file:
  CommandTemplate, project: Option<CommandTemplate> }` in a new `external_tools/custom_tool.rs`
  (`config.rs` is at 540 lines): serialized as the struct, deserialized through an untagged shape
  that reads batch 1's bare string as `{ file, project: None }`; `resolve`'s Custom arm expands
  `project` for `Location::Project`, `NoProjectCommand` when `None`; `adopt_legacy_ide_command`
  builds `project: None`. Seventeen `ToolChoice::Custom(` sites in four files;
  `CustomTool::file_only(template)` is the one-line migration.
- **Edit → Open Project** joins, or that field ships unreachable: "Open Project in <tool>" above
  "External Tools…", named by `tool_name(FileType::RustSource)` as the context menu names its
  item (the system default's draws disabled, as does the item on the web; a Custom with no
  project command draws enabled and reports `NoProjectCommand`); `EditorAction::OpenProject` →
  `OpenRequest { path: PathBuf::new(), location: Location::Project }`.
- **The panel.** `PanelId::SETTINGS = PanelId(6)`, "External Tools", `DockPosition::Right`, added
  after the console in `context/mod.rs`'s build so a prefs file from before leaves it hidden (the
  composer's test is the model; `visible = false`; 320 wide, 240 minimum). The dock's first pass
  gives each visible right panel the full remaining height, so a second right panel sits beside
  the Inspector, not under it — accepted for this batch; a stacked right edge is its own issue if
  Jesse wants one. Shown by Edit → External Tools… (`EditorAction::ShowExternalTools`: visible,
  uncollapsed, the narrow overlay when the dock is narrow); hidden by its header as any panel; no
  View item. `panel_renderer/mod.rs`'s match gains an arm into a new `panel_renderer/settings.rs`.
- **Over the inspector's widgets, with the sidebar's cursor.** `EditableInspector` (596 lines, its
  walk private) has no cursor, and the ui's Tab traversal reaches edit fields alone, so
  "keyboard-walkable" is the composer sidebar's model (`composer/sidebar/walk.rs`: a control
  counter, `take_control`, `ring` in `theme.focus_ring` at 2 px, `inside`): the panel is a walk of
  its own in `crates/editor/src/settings_panel.rs` (`SettingsPanelState` on
  `EditorContext.settings`; rows split into `settings_panel/rows.rs` before 600 nears) laying the
  inspector's widgets — `field_widgets::edit_cycle`, `text_field::edit_string`, `ui.button` —
  with `theme.editable_field_style()`. Keys routed as the console's (`KeyRoute::Settings` after
  `Console`; `editor_game/settings_keys.rs`; `settle_settings` after `settle_console` by
  `panel_press`'s rule): Up and Down move the cursor, Enter activates (a button presses, a field
  takes focus through `focus_text_input`, a cycle steps forward), Left and Right step a cycle row,
  Escape releases; never during a play session.
- **The rows**, top to bottom, every one labelled: per category (Code, Music, Sample, Art) a cycle
  row over "System default", the category's presets and "Custom command"; under a preset its
  Program field; under Custom its Command field and, for Code, its Project command field; a Test
  button. Then **Overrides**: one cycle row per `FileType` over "Inherit (<the category's tool>)",
  the presets, "Custom command", "System default" — a present entry honoured whatever it says
  (round 3, codex F6) — with the same sub-rows. Then **DAW project extensions**: one field, split
  on whitespace and commas, dots optional. Every edit writes `EditorContext.external_tools`; the
  settle rule captures it, and `save_preferences_now` at Stop lands an edit made during Play.
- **The invalid state.** `UIContext::text_input` passes `|_| true` to `edit_field`, whose
  `is_valid` draws `border_invalid` while editing: the ui gains `text_input_validated(id, value,
  bounds, is_valid)` and `text_input` becomes its `|_| true` call — the one `ui` change;
  `text_field.rs` gains `edit_string_validated`. A template field validates by `validate` plus
  `names_file` as the user types; a commit that fails leaves the config untouched, keeps the
  draft in the field (`focus_text_input(id, draft)`), and draws a note row under it reading
  "Error: <TemplateError>" in `status_error` — words, not colour alone.
- **Test.** The button pushes its category onto `EditorContext.tool_tests: Vec<ToolCategory>`; the
  launcher drains it beside `open_requests`. The fixture is the first entry of the category in
  the browser's scan (scanned first when it has not run: Code → `Script` else `Scene`; Music →
  `Song` else `DawProject` else `SoundFont`; Sample → `Sound`; Art → `Image`), none → an error
  line "no <category> file in this project to test with"; Code at line 1 column 1, a `.mid`
  through `at_first_part_track` as every request, the rest `Location::None`. Before spawning, an
  information line "Test <category>: <program> <args>" (a token with a space quoted) or "Test
  <category>: the system default opens <file>" through `report`, then `spawn::launch` — a missing
  binary is the launcher's own sentence. On the web the button draws disabled over the hint.
- **Docs.** `crates/editor/CLAUDE.md` (the Panels line, a `settings_panel.rs` row, the
  `external_tools/` row, the `menu/` row, the pitfall table); `crates/editor_integration/CLAUDE.md`
  (the `external_tools.rs` row — delete "so a hand edit of `editor_prefs.json` … is overwritten by
  the next autosave", name the panel and the Test drain — the `panel_renderer/` row, `shortcuts.rs`'s
  route list, a `settings_keys.rs` row). #102's second "done looks like" bullet is this panel.

**Batch 7, split at its re-verification (planner, 2026-11-03, on the plan's own "the rename, as its
own commit"): 7a the trim (#217, #218), 7b the rename (#227), one out at a time, 7b after 7a lands.
7a DONE 2026-11-03, insiculous_2d e7dd097, after code round 39** (what the round taught: all three
reviewers read the retired journal as data loss — a Save As whose sidecar fails after the `.mid`
landed, a `<stem>.recovered-*` sidecar's samples no longer prune roots, a legacy `.new` pair no
longer opened — and each is rulings (i) and (j) applied, with no such file in the working set; the
one accepted finding was `song init` missing from the command docs since batch 5; Ctrl+A now passes
to the scene's Select All as every unbound Ctrl chord does, a rule for the Editor UX plan to revisit
in one piece; the executor's one deviation, `ComposerMode::Tabs` removed since it laid out as Compact
with the roll gone, stands, so #197's "the Tabs layout" now means Compact). **The gate was MET
2026-11-03** (`review/external-tools/roundtrip-7.md`): pong's `ai_tong.mid` imported by
MIDI Evil's `oracle project import-midi`, one trombone note added, `export-midi`, pong's 57 tests green
over the exported file (baseline 57), the committed file restored; the panel's own open of the exported
file and MIDI Evil's desktop Save-back were not driven headless.

**Re-verified 2026-11-03 against 5440b10** — what the section below assumed, corrected:
- **The score verbs are ten, not six**: `apply.rs:212-222` groups `AddNote`, `DeleteNotes`, `MoveNotes`,
  `ResizeNotes`, `SetVelocity`, `SetPitch`, `AssignPart`, `SetTempo`, `SetTimeSignature`, `AddTrack` as
  the score's; `Batch` stays for the map. `song select` / `PureWrite::SongSelect` / `SongSelection`
  (`song/selection.rs`, 49 lines) are the roll's and go with it; `song undo|redo` stay for the map's history.
- **`song new` is the score maker**: `HostedWrite::SongNew`, `EditorAction::NewSong`, File → "New Song"
  (`menu/mod.rs:191`, `actions.rs:41`), `PendingSceneAction::NewSong` (`scene_confirm.rs:31,68,84,296`),
  `song_io.rs:143 new_song`, `SongDocument::new_song`/`new_song_on` (`song/mod.rs:147,153`) go; `song init`
  and the unmapped `.mid` opened in place are how a sidecar begins. The test fixtures built on
  `new_song` + `AddNote` (`song/test_support.rs:68-88`, `composer/test_support.rs:12-20`,
  `editor_integration/.../test_support.rs:162,177`) become MIDI bytes through `music_midi`'s
  `SongData` helpers + `write_smf` → `SongDocument::from_smf` (`:227,233` already do).
- **The journal goes with the trim** (rulings (i), (j); § The open song document's last sentence):
  `io/commit.rs`'s `.new` pair (`:45-52`), `guard_sidecar`'s journal removal, `recovery_samples`'s
  journal root (`:242-255`), `io/open.rs`'s `SongFiles.journal_*`, `finish_first_save`, `keep_journals`,
  `io/recover.rs` whole; the transition's score guard — `is_score_authored`, `baseline_export`
  (`song/mod.rs:141,228`, `follow.rs:24-28,55,61`), `score_guard` and `committed.recovered` →
  `adopt_score` (`io.rs:326-350`) — and `EditingPolicy` (`song/mod.rs:75-110`, the sidebar's note
  `sidebar.rs:195`, `song`'s `editable`/`refused_reason`, `command_api/song.rs:179-180`). A save is
  each final by temp-and-rename in its own directory (Save As writes two: the loaded bytes and the
  sidecar). The playground's `.new` branches follow: `archive.rs:130-134`, `archive/songs.rs:26-33`,
  `archive/song_tests.rs`, the `persist/queue.rs:7-8` sentence, `WEB_PLAYGROUND.md:164,293-295,330`.
- **The playground's re-import already keeps the sidecar** (5a: `upload.rs:13-14,171-173`, the
  `sidecar_lost` outcome) **and its export already carries the live song's two files**
  (`archive.rs:107-134`, `song_tests.rs:18`); 7a changes only the journal rule there and writes the
  browser's reduction into `docs/WEB_PLAYGROUND.md`.
- **The GUI**: `composer/` is 18 files, 3,852 lines; goes: `piano_roll.rs` (292) + tests (193),
  `note_list.rs` (157), `layout.rs`'s `PianoRollGeometry`/`BeatLine` (`:147-260`) and the roll/list
  areas, `mod.rs`'s `RollPoint`, `MovePreview`, `clamp_move`, `moved_notes_edit`, `track_notes`,
  `note_images`, `note_name`, `ComposerAction::{Select, BeginDrag}`, `state.rs`'s `RollCursor`,
  `ComposerFocus::{PianoRoll, NoteList}`, `tabs_area`, the roll scrolls, `cursor`, `note_list_row`,
  `new_track_program`, `selected_on_track`; `keys.rs`'s `Nudge`, `DeleteNotes`, `Shorten`, `Lengthen`,
  `VelocityUp/Down`, `SelectAll` rows; the theme's `note_fill`, `note_selected_border`, `key_black`,
  `playhead` (`theme/mod.rs:165-175,300-304`) with `theme/tests.rs:119-146`. Stays: `transport.rs`,
  `sidebar.rs` + `sidebar/{blocks,walk}.rs`, the Songs/SoundFont scans, `drop_action` (a `.wav` →
  sample instrument), `ComposerMode` (Compact's section tabs are the sidebar's).
- **The host**: goes: `composer_drag.rs` (112) + tests (156), `EditorGame.composer_drag`
  (`editor_game/mod.rs:85,160`), `drop_composer_drag` (`shortcuts.rs:168,495`), `api.rs`'s
  composer-drag skip; `composer_keys.rs` (529) loses `move_composer_cursor`, `selected_note_images`,
  `nudge_notes`, `resize_notes`, `change_velocity`, `add_note_at_cursor`, `move_note_cursor`
  (`:172-289,507`) and `run_composer_actions`'s `Select`/`BeginDrag` arms; `panel_renderer/composer.rs`
  (266) loses the roll and list renders and `render_area_tabs`. The salvaged scans are read through
  `composer/mod.rs:49` and `state.rs:14,19` re-exports — 7a reads `external_tools::{scan_songs,
  scan_sound_fonts, SoundFontScan}` directly and drops the re-exports.
- **Docs 7a rewrites** (the name "composer" stays until 7b): `docs/SONG_FORMAT.md` § How the composer
  saves (`:307-430`: the transition paragraph, steps 3, 5's rule and 6, the recovery paragraph and its
  "No final at all" bullet go; Save As's two renames said); `docs/EDITOR_COMMAND_API.md` § Songs
  (`:177-300`: `song edit`'s ten score shapes, `song select`, `song new`, `selection`/`editable`/
  `refused_reason`, the "Save is a journal" paragraph, `song open`'s recovery clause); `training.md`
  § Authoring a song (`:1002-1010`); `docs/WEB_PLAYGROUND.md` (`:164,291-295,329-330,354`); both
  editor guides' rows and pitfall tables (26 + 31 mentions); `log_archive.md`'s dated entry. The
  games' guides, `PROJECT_ROADMAP.md`, `music_midi/CLAUDE.md` wait for 7b's sweep.
- **Sizes**: `editor_game/mod.rs` 567, `context/mod.rs` 549, `menu/mod.rs` 538, `shortcuts.rs` 538,
  `panel_renderer/mod.rs` 532, `song/io.rs` 526 — every one shrinks. `music_midi` is not touched.
- **Issues re-read, the planner's reading for the take-back**: #200 (the roll's wheel) superseded;
  #197 (Compact tabs + the sidebar button's clipping), #188 (Add binding), #187 (the canvas's
  accessibility tree), #186 (Stop while the song waits) are the map's or the editor's and stay; #141
  is already closed.

**7b re-verified 2026-11-03 against 51fda21, after 7a** — what the rename finds in the tree:
- **Already gone with 7a, nothing to rename**: the theme tokens and their guard tests, `composer_drag.rs`,
  `song/selection.rs` — #227's text predates the trim on those three.
- **The names**: `crates/editor/src/composer/` (`lib.rs:45`) → `conductor/`; `editor_game/composer_keys.rs`
  + `_tests.rs` → `conductor_keys*.rs`; `panel_renderer/composer.rs` → `conductor.rs`; the types
  `ComposerState`, `ComposerFocus`, `ComposerAction`, `ComposerKeyAction`, `ComposerKeyRoute`,
  `ComposerKeyBinding`, `ComposerMode`, `ComposerLayout`, `ComposerView`, `COMPOSER_KEYS`; `PanelId::COMPOSER`
  (its number 5 stays — the prefs key panels by `PanelId.0`, so an old `editor_prefs.json` keeps the layout);
  `EditorAction::FocusComposer` (Ctrl+Shift+M stays); `KeyRoute::Composer`; `EditorContext.composer`; the
  host's `composer_holds_focus`, `show_composer`, `settle_composer`, `leave_composer`, `run_composer_key`,
  `run_composer_actions`, `composer_write_refusal`, `apply_composer_edit`, `render_composer`,
  `render_composer_panel`, `resolve_composer_key`; the test helper `composer_session`; 23 widget ids
  `composer_*`; 22 test names; the label "Composer" at six sites (View, `panel_id_for_menu_label`,
  `menu_actions.rs:16`, `default_layout.rs:33`, `ComposerFocus::None`, `context/tests.rs`).
- **No collision**: `music_midi` has `Conductor`, `ConductorConfig`, `ConductorCommand`, `ConductorHandle`,
  `audio` has `ConductorSource`; none of the editor's new names repeats one. **The prose rule**: the
  editor's desk is *the Conductor panel*, always with "panel"; *the conductor* bare stays the audio
  thread's (`music_midi::Conductor`), which the editor crates already name five times.
- **The word in prose, beyond the plan's list**: doc comments in `audio/src/manager/song.rs:276` (+2 tests),
  `engine_core/src/game_music.rs:109`, `music_midi/src/{lib.rs:7, song_file.rs ×6, song_file/validate.rs:31,
  sequencer/score.rs:7}` (`song_data.rs`'s six are `UnsupportedReport`'s, #239's: reworded "the editor" only),
  `playground/src/{upload.rs ×3, upload/tests.rs ×2, persist/tests/song_saves.rs}`, `scripts/build_wasm.sh:63`,
  `crates/editor/Cargo.toml:16`; **the LICENSE `Comment:` line the engine's Save writes**
  (`song/io/samples.rs:103,211`) and its copies in this repo (`examples/music/LICENSE`,
  `examples/assets/music/LICENSE`, `examples/assets/music/samples/ai_demo/LICENSE`).
  `music_midi/fixtures/LICENSE:102` ("composers' deaths") names real composers and stays.
- **Docs in this repo**: `editor_integration/CLAUDE.md` 23, `editor/CLAUDE.md` 22 (citations to
  `src/composer/*` and `composer_keys_tests.rs` follow the moves — `check_guide_tests.sh`),
  `docs/SONG_FORMAT.md` 18 (§ How the composer saves → § How the Conductor panel saves), `training.md` 9,
  `docs/WEB_PLAYGROUND.md` 7, `docs/EDITOR_COMMAND_API.md` 7, `music_midi/CLAUDE.md` 4, `playground/CLAUDE.md` 2,
  `audio/CLAUDE.md` 1, `CLAUDE.md:53` (the SSOT row), `PROJECT_ROADMAP.md:92` (the paragraph; row K's title
  is the phase's name as it was and stays, Jesse's dated reversal line being his). `log_archive.md` is history.
- **One repo, one commit — 7b is `insiculous_2d` alone.** The six games' guides (30 mentions, each with a
  `song new` sentence that is now history) and their `assets/ai/music/LICENSE` `Comment:` lines are six
  repos' commits and go with batch 8's docs pass (#220), which already names them; the site's three strings
  are insiculous_web#78; the readerless `UnsupportedReport` is #239 (the backlog's, outside the sprint).
- **Stills**: the View menu open on "Conductor" with the panel on `ai_tong.mid`, native; the panel in the
  playground. Reviewers kimi, gemini, codex. Sizes: a rename grows nothing.

**Batch 7 as settled — the Conductor panel: #217 (E15, reworded), #218 (E16, reworded), the rename (filed at
settling).** **Gated on MIDI Evil's `.evil` → `.mid` export and positional open round-tripping one
of the six shipped songs through the engine** — import, a note changed, export, `cargo test` in
that game green over the exported file and the song opening in the panel — demonstrated and quoted
in the handoff. Then the trim: the piano roll, note list, note editing and the authoring verbs go;
the sidebar, transport, audition and Save stay; the authoring verbs refuse naming MIDI Evil; the
five composer-defect issues re-read; the playground's export still carries the open song's two
files, its re-import keeps the sidecar, and the browser's reduction is written into
`docs/WEB_PLAYGROUND.md`. Then the rename, as
its own commit: module, panel id, View item, theme tokens, guides. Target shapes: § The Conductor
panel. Reviewers: kimi, gemini (the API's refusals and the playground's export are seams), codex
(two stills: the Conductor panel on `ai_tong.mid`, native and in the playground). Leaves out:
ruling (e)'s gain field unless Jesse rules it in. If the gate is not met when batch 6 lands, batch
8 goes first and batch 7 waits; the sprint does not close without it.

**Batch 8 — platform polish and the docs: #219 (E17), #220 (E18).** WSL paths and `wslview` —
batch 1 shipped `wslview` only, and § Cross-platform rules' `.exe` rule (every path placeholder
through `wslpath -w` for a program ending in `.exe`) needs the program found before the template
expands, a two-phase resolution batch 1's shapes lack — the Flatpak hint; the first Windows
compile of `spawn.rs`'s `cfg(windows)` hunk (`windows-sys` 0.61 declares `ShellExecuteW` only with
`Win32_UI_WindowsAndMessaging` beside `Win32_UI_Shell`); `ShellExecuteW` on the frame thread checked
by opening an unassociated type (a `.rhai` on a fresh profile) and a file on an offline network
share — a stalled frame moves it to a worker thread with `CoInitializeEx`, its result through the
launcher's channel (batch 1's code round, gemini F2); `code` resolving to `code.cmd` through
`PATHEXT`, and `cmd.exe`'s argument escaping, on the matrix;
the three OS-default fallbacks and the WSL one exercised by hand on a Linux, macOS and Windows
machine with a path containing a space and an `&` (the results recorded in the issue); `docs/EXTERNAL_TOOLS.md` (the presets with
their sources and dates, Aseprite's among them, the template grammar, the resolution order, the
reload table with the Art rows and a cross-link to the Aseprite effort's doc, MIDI Evil's
contract, the platform notes), the SSOT row in `CLAUDE.md`, `training.md`'s directory map, the two
editor guides, `docs/SCRIPTING.md`, `docs/SONG_FORMAT.md` (the fingerprint's rule; MIDI Evil's
export rules), the games' guides' composer sentences, the roadmap. Reviewers: kimi. Leaves out:
nothing; this closes the sprint, and the close-out (the milestone's issues closed by their commits,
the lessons to `log_archive.md`, `review/external-tools/` archived, the scorecard's totals) is the
planner's.

**Order.** 1 → 2a → 2b → 3 in sequence (each builds on the last's types and console); 4 after 2b; 5 after
3 and 4; 6 any time after 1; 7 after 4 and 5 and its gate; 8 after 6, before 7 if 7's gate is not
yet met. One batch out at a time, as the Conductor ran it.

## Verification, end to end

- Every batch: the gates above, run by the planner on the staged tree, never from the report.
- After batch 3: the editor on a temporary copy of the playground's pong project (`games/pong`
  has no `.rhai`), `touch assets/scripts/paddle.rhai` from a shell → the console says so within
  a second; Ctrl+S on the scene → no reload; in Play, change `speed` in
  `paddle.rhai` and save → the paddle moves faster, no Stop; break the syntax → the old speed holds
  and the console links the line; on a temporary copy of breakout's `assets/`, a sheet changed in
  Play → still changed after Stop.
- After batch 5: Tong in the editor, Play; a note changed in MIDI Evil (or any editor that writes a
  `.mid`) and saved mid-bar → heard from the next bar, the hits on the beat before and after (a
  frame of lateness at the bar passes; a gap Jesse hears sends § Appendix to a batch); the
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
browser's reduction down; no MIDI Evil code in this repo; no `cargo` spawn and no rebuild notice
(ruling (l)); no texture hot reload and no `.aseprite` decoding here — both are the Aseprite
effort's (`coordination/aseprite-native/plan.md`); no `notify`/inotify dependency until
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

## Appendix — the audio-thread seam (held)

The v5 design of the swap, taken only if batch 5's listen check fails ruling (k). Held verbatim
so nothing the four rounds found is lost; not a target shape for any batch until Jesse says so.

#### MIDI reload (v5): swap at the next bar, keep playing

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
