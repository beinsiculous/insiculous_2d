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
| `RecompileScript` | a console line (the runner compiles at session start) | `ctx.scripts.reload_source(path)`: recompile; on success swap the AST for every instance of that path, clear its `failed` and quarantine entries, keep blackboards and params; on a syntax error keep the old AST running and put the error, with its line, in the console as a link; a runtime error on the first tick after a reload (the state's shape changed) quarantines as any runtime error does and the console says "reloaded script failed on its first frame — Stop and Play to reset its state", the blackboard kept on purpose (round 2, gemini F7) |
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

**Batch 3 — the watcher and code reload: #208 (E6), #209 (E7), #210 (E8); #211 (E9) closed
won't-do by ruling (l).** `FileWatch`
on its worker thread with its registered set, debounced tombstones, companion files and
`note_written` baselines, its MIDI and sidecar rules batch 1's `external_tools::is_midi_path` and
`midi_sidecar_path`, never its own; `hot_reload.rs`
with the registered set's rebuild rules; Rhai recompile on save in a Play session
(`ScriptRunner::reload_source`, the AST swapped, blackboards kept, a syntax error keeping the old
AST and linking the line); scene and sheet reload through the existing guards and dialog, the sheet as a transaction and
re-applied on Stop, the first-tick rule after a script reload. Target shapes: § File watching and hot reload (the
first three policy rows). Reviewers: kimi, gemini (the watcher's debounce and tombstones, the
runner's swap and Stop's re-apply are harness and seam). Leaves out: the song and sample policies
(batches 4 and 5), any `cargo` spawn.

**Batch 4 — music entry points and the sample reload: #212 (E10), #215 (E13).** `scan_songs`
salvaged into `external_tools/songs.rs` and pointed at `is_midi_path` (`composer/state.rs:399`
accepts `mid` alone, so a `.midi` the asset browser lists since batch 1 is missing from File →
Open Song until then); the Edit in MIDI Evil button on a `Binding` emitter with
the action → part → track resolution; a `.mid` tile opening at its first part's track
(`Location::Track` from its sidecar) through every entry batch 2a gave it — the double-click,
Enter and the context menu's Open in <tool> — where 2a opens it at `Location::None` (every tile
kind opens from 2a on); recorded sounds reloading in place behind their handle, the
library's loaded and failed paths registered, a failure cleared on success (the sound half of #180,
that issue updated). Target shapes: § Entry points rows 1 and 6, "Which song is the song", the
`ReloadSample` row. Reviewers: kimi, gemini (the sound library's reload), codex (one still: the
emitter's settings with the button). Leaves out: the song swap.

**Batch 5 — the song follows the score: #213 (E11), #214 (E12).** The open song document's
two-file policy by content fingerprint with the score always adopted, the sidecar-only save over
the loaded bytes, Save As writing the whole song, the transition's score-authoring rule, the
Overwrite / Cancel prompt before a sidecar write and its headless error, recovery that keeps and
never promotes, the prune moved after the finals with recoveries as roots, the map edits on mixed-channel MIDI,
`song init`, and the panel's Save dispatching the reload, with their migration tests (§ The open
song document); the song's files through batch 1's helper — `plan_save`'s `NotMidi` check
(`song/io.rs:347`, `mid` alone), the sidecar names (`with_suffix(.., ".ron")` at
`song/io.rs:335,411,456`, `song/io/open.rs:76,247`, `editor_game/song_io.rs:249`) through
`midi_sidecar_path`, and the web upload's kind (`playground/src/upload.rs:38`) through
`is_midi_path` — with an end-to-end `.midi` open → edit → save test (batch 1 lists a `.midi` as a
Song tile, which the composer opens and Save refuses until this lands); the silent part in `music_midi`'s `validate` and trigger engine;
`replace_song_at_seam` with the worker's `PreparedPlayback` and its request id, the host's swap
on an observed bar-line crossing with the bar-and-fraction conversion, the stationary-transport,
stopped-at-end, end-while-pending, short-loop and staged-refusal rules, the host's re-asserted
transport and lifecycle queue, its one-transition adoption, retired sources dropped on a worker,
the closure re-registered (§ MIDI reload); the
`conductor_background` example's `--replace-after <bars>` flag; the `ReloadSong` dispatch with the
debounce → prepare → diff → swap chain and `track_diff.rs` over resolved parts. Reviewers: kimi,
gemini (the audio seam; the half-written-file and the renamed-track fixtures). Listen check:
Jesse, Tong in the editor, a note changed in MIDI Evil (or any editor that writes a `.mid`) and
saved mid-bar, heard from the next bar with the hits on the beat before and after — a frame of
lateness passes; a seam Jesse hears sends § Appendix to a batch of its own. Leaves out: anything
in MIDI Evil's repo; the fixture pair until MIDI Evil ships it (a hand-made pair in
`music_midi/fixtures/` with a LICENSE line stands in); the audio-thread swap.

**Batch 6 — the settings panel: #216 (E14).** `settings_panel.rs` over `EditableInspector`: a
cycle row per category for the preset, a string field for the Custom template with the inline red
invalid state, per-type overrides, the MIDI Evil binary field, the DAW project extensions, a Test
button per category; Edit → External Tools… (every launch error since batch 1 points there);
`ToolChoice::Custom`'s second, project field reading batch 1's on-disk shape
`{"Custom": "<template>"}` still, as a project-less custom tool (a test loads that JSON); keyboard-walkable with a visible focus ring, labelled
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
- After batch 3: the editor on pong, `touch assets/scripts/paddle.rhai` from a shell → the console
  says so within a second; Ctrl+S on the scene → no reload; in Play, change `speed` in
  `paddle.rhai` and save → the paddle moves faster, no Stop; break the syntax → the old speed holds
  and the console links the line; a sheet changed in Play → still changed after Stop.
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
