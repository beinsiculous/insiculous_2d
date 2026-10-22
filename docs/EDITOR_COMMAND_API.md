# Editor Command API

The editor answers structured questions about the open scene over a
line-oriented protocol: **text requests in, single-line JSON responses
out**. This covers Stages A and B of the audit §9 command layer: read-only
queries plus write commands. Every write routes through `CommandHistory`,
so every API mutation is undoable in the GUI exactly as if clicked.

The dispatch is transport-agnostic (`editor::command_api::dispatch_line`);
the shipped transport is the native editor binary's stdin/stdout:

```bash
printf 'scene\nlist\n' | cargo run --bin editor --features editor -- ../games/pong --api
```

`--api` spawns a stdin reader thread; requests are answered once per frame,
in order, on stdout (flushed per batch). Requests received during a live
gizmo drag stay queued and are answered when the drag ends. A future web
editor feeds the same dispatch over a WebSocket.

## Requests

One request per line. Tokens split on whitespace; double-quote a token to
include spaces (`describe "Left Paddle"`). No escape sequences. Blank
lines are ignored (no response). Unknown verbs and trailing tokens are
parse errors.

| Request | Answer |
|---------|--------|
| `list` | every entity |
| `list <filter>` | entities whose display name contains `<filter>` (case-insensitive) |
| `describe <entity>` | one entity with all registry component values |
| `selection` | current selection (primary + all, insertion order) |
| `scene` | scene path, the scene's dirty state, the song's (`song_dirty`) and either's (`any_dirty`), entity count, play state |
| `commands` | self-description: every verb (usage/example/summary/writes/undoable) plus the LIVE `settable`, `addable`, and `archetypes` name lists |

Write verbs (Stage B — one undo entry each unless noted):

| Request | Effect |
|---------|--------|
| `set <entity> <Component> <json>` | shallow-patch the component with the REST OF THE LINE as raw JSON (never tokenized). Unknown fields are an `invalid` error listing the real ones; a non-object serialization (externally-tagged enums like `Behavior`) is a whole-value replace. Absent component → `invalid` ("add it first"). `set <entity> Name ...` is `invalid` — Name goes through `rename`. A `texture_handle` / `tileset` the session's resolver never issued is `invalid` (#66). |
| `add <entity> <Component> [json]` | add default-valued (optionally patched — still ONE undo entry; a patch carrying an unissued texture handle is `invalid` and the add is rolled back) |
| `remove <entity> <Component>` | remove (undo restores the VALUE, not a default) |
| `rename <entity> <name>` | assign/replace `Name` — works on unnamed entities; undo restores no-Name; empty/unchanged names are `invalid` |
| `create <archetype> [name] [x y]` | spawn an archetype (see `commands` for the list) at the viewport center or an explicit position; an empty name is `invalid` |
| `delete <entity>` | delete (children reparent to the grandparent; undo resurrects; the selection drops it) |
| `select <entity>` / `select none` | replace/clear the selection — never on the undo stack (GUI parity) |
| `undo` / `redo` | `{"undid"/"redid": <command name>}`, `null` on an empty stack (not an error); `refused` while a batch is open |
| `save [path]` | save through the editor's mandatory choke point (a relative path resolves against the asset root — it used to land in the process cwd; pass an absolute path to keep that) |
| `batch begin [name]` / `batch end` / `batch abort` | group writes into ONE undo entry / roll them back |

Write semantics worth knowing:

- **Guards**: all writes are `refused` while **Playing** (Paused edits are
  allowed — inspector parity). `save` is additionally refused during any
  play session (Paused included) and while a batch is open.
- **Sanitation**: non-finite numbers anywhere in a JSON value are
  `invalid`; the same hard physical floors the GUI enforces apply
  (collider extents/radius, scale, a `SoundEmitter`'s volume and
  `max_distance`). Texture handles
  (`Sprite.texture_handle`, `Tilemap.tileset` — one id space) must have
  been issued by the session's resolver, or the write is `invalid`:
  handle 0 is always `#white`; anything else comes from a texture the
  session loaded (windowed) or a reference it recorded (headless).
- **Batches are NOT transactions**: each command executes immediately; an
  error mid-batch leaves earlier effects applied and the batch open —
  `batch abort` reverse-undoes what was collected. Pressing Play commits
  an open batch (its commands are in the world the snapshot captures);
  Stop commits a batch opened while Paused into the session and the
  Keep/Discard dialog decides its fate with the other paused edits. A
  GUI edit interleaved with an open cross-frame batch lands on the history
  BEFORE the batch's macro — known reordering limitation; keep batches
  within one request burst.
- Two consecutive `set` lines on the same field are two undo entries (API
  writes never merge).

`<entity>` is resolved **name-first**:

- `Player`, `"Left Paddle"` — **exact, case-sensitive** match on the
  `Name` component (the `list` *filter* is case-insensitive; feed back the
  exact `name` field from its output). Names are not unique by
  construction; an ambiguous name is an **error** listing the matching ids
  (sorted), never a silent first match.
- `#7` — the session-local numeric id shown in the hierarchy/inspector
  (`EntityId::value()`). Ids are NOT stable across sessions or Play/Stop
  cycles; names are the durable address.

Synthesized display names ("Sprite (Entity 5)") are labels, not addresses —
use the `#id`.

## Responses

Exactly one line of JSON per non-blank request, in request order:

```json
{"ok":true,"data":{...}}
{"ok":false,"error":{"kind":"<kind>","message":"<human text>"}}
```

`kind` ∈ `parse` | `not_found` | `ambiguous_name` | `invalid` | `refused`.
An `ambiguous_name` error additionally carries `"matches":[<id>,...]`.
`invalid` = the arguments are unusable (unknown component/field, bad JSON,
non-finite number, unissued texture handle); `refused` = the editor's current state forbids the
request (Playing, batch rules, a write over the read-only dispatch).

Every entity appears as the same record shape:

```json
{"id":7,"generation":1,"name":"Player","display":"Player"}
```

`name` is the `Name` component (`null` if absent); `display` is what the
hierarchy panel shows (falls back to "Sprite (Entity 7)" etc.).

### `list`

```json
{"entities":[{"id":7,"generation":1,"name":"Player","display":"Player"}, ...]}
```

Sorted by id.

### `describe <entity>`

The entity record plus a `components` map — every present registry
component (builtin + removable, registry order), each as its serde value:

```json
{"id":7,"generation":1,"name":"Player","display":"Player",
 "components":{"Transform2D":{"position":[3.0,4.0],"rotation":0.0,"scale":[1.0,1.0]},
               "Sprite":{...}}}
```

Hidden registry entries (`GlobalTransform2D`, `BehaviorState`) are internal
and not listed in `components`. `Name` is an editable registry component
since #32, but `describe` still filters it out of `components` — the name is
surfaced exactly once, as the top-level `name` field (it is the API's entity
address; a duplicate component entry could diverge from it). A component
that fails to serialize contributes the string `"!serialize error: <e>"` for
its key so the map stays total.

### `selection`

```json
{"primary":{"id":7,...},"all":[{"id":7,...},{"id":9,...}]}
```

`primary` is `null` when nothing is selected; `all` is insertion-ordered.

### `scene`

```json
{"path":"assets/scenes/level.ron","dirty":false,"song_dirty":true,"any_dirty":true,"entity_count":12,"play_state":"editing"}
```

`path` is `null` for an unsaved new scene. `dirty` is the scene's alone, from
the `CommandHistory` watermark (the same source as the title bar's first `*`);
`song_dirty` is the open song's (§ Songs — `false` with no song open), and
`any_dirty` is either, which is what "is there unsaved work?" asks.
`play_state` ∈ `editing` | `playing` | `paused`.

## Songs

The composer's song is a **second document** beside the scene: its own
history, its own save watermark, its own selection. Every `song …` verb
addresses it; none touches the scene's `CommandHistory`, and `undo` /
`redo` never touch the song.

| Request | Answer / effect |
|---------|-----------------|
| `song` | the open song (below), or `{"open": false}` |
| `song notes [part]` | every note, or those of the track `part` plays: `{"notes":[{"id":3,"track":1,"tick":0,"key":60,"velocity":100,"duration_ticks":240}, ...]}` |
| `song edit <json>` | one `SongEdit` (the REST OF THE LINE, never tokenized) through `SongDocument::apply`, the mutation path the composer's GUI uses — one entry on the song's history. Answers `{"command":"song edit","edit":"<variant>","added_notes":[<id>...],"dirty":<bool>}` |
| `song undo` / `song redo` | the song's history only: `{"undid"/"redid": <variant>\|null}` |
| `song select <note>...` / `song select none` | select exactly these notes by id — never on the undo stack (GUI parity) |
| `song new [discard]` | a new, never-saved song (one tempo track, a `melody` track on channel 1, 120 BPM in 4/4, the background on `soundfonts/cast.sf2`) |
| `song open <path> [discard]` | open a `.mid` (relative to the asset root), settling what an interrupted save left — the answer's `notes` say what was settled |
| `song save [path [overwrite]]` | save to `path`, else the song's own path, else — never saved — `music/untitled.mid` (numbered while its `.mid` or its `.mid.ron` is taken). A `path` whose `.mid` or `.mid.ron` exists and is not the song's own file is another song: `refused` unless the line ends in `overwrite` (its own word — it replaces that song's files, where `discard` drops this song's edits); the composer's path field asks Overwrite / Cancel instead |
| `song close [discard]` | close the song |
| `song play [beat]` | play the song as the document holds it — the composer's preview — from a whole beat counted from zero (from the top without one): `{"preview":"playing","from_tick":<tick>}`. `refused` with no song open; a beat that is fractional, negative or not finite is `invalid`. It is heard on the next frame (before the web's first click it waits, and the composer says so) |
| `song stop` | stop the preview: `{"preview":"stopped"}`; nothing while it is stopped |
| `song seek <beat>` | move the playing preview to a whole beat; `refused` while it is stopped (`song play <beat>` starts there) |
| `song trigger <action>` | play the binding's part on its instrument as the game's action would: `{"triggered":"<action>"}`. `not_found` for an action no binding has; `refused` until the preview is heard |

**`song` answers:**

```json
{"open":true,"path":"assets/music/theme.mid","imported_from":null,"dirty":false,
 "editable":true,"refused_reason":null,"ticks_per_quarter":480,
 "soundfont":"soundfonts/cast.sf2","looping":true,
 "tracks":[{"index":0,"name":null,"channel":null,"note_count":0},
           {"index":1,"name":"melody","channel":1,"note_count":3}],
 "parts":[...],"instruments":[...],"bindings":[...],
 "tempo_changes":[{"tick":0,"microseconds_per_quarter":500000,"beats_per_minute":120.0}],
 "time_signatures":[{"tick":0,"numerator":4,"denominator":4}],
 "provenance":{"author":"2026 Jesse","license":"CC-BY-4.0"},
 "selection":[3],"can_undo":true,"can_redo":false}
```

`channel` counts from 1, as musicians count (and as a type-0 file's
`channel N` tracks are named). `parts`, `instruments` and `bindings` are
the sidecar's entries as JSON, in `SongEdit`'s shapes. A song is `dirty`
when its history moved past its last save **or it has never been saved**.
`editable: false` is a MIDI file the editing model cannot represent (a
track whose notes span channels); every `song edit` on it is `refused`
with `refused_reason`, and it still opens and saves as it was.

**`SongEdit`** is serde's externally tagged JSON. Notes are named by id
(the `id` of `song notes`); parts, instruments and bindings by name:

```json
{"AddNote": {"track": 1, "tick": 0, "key": 60, "velocity": 100, "duration_ticks": 240}}
{"DeleteNotes": {"notes": [3, 4]}}
{"MoveNotes": {"notes": [3], "tick_delta": -120}}
{"ResizeNotes": {"notes": [3], "duration_ticks": 480}}
{"SetVelocity": {"notes": [3], "velocity": 90}}
{"SetPitch": {"notes": [3], "key": 62}}
{"AssignPart": {"notes": [3], "part": "lead"}}
{"SetTempo": {"tick": 0, "microseconds_per_quarter": 400000}}
{"SetTimeSignature": {"tick": 1920, "meter": {"numerator": 3, "denominator": 4}}}
{"SetLoop": {"looping": false}}
{"SetQuantize": {"part": "lead", "quantize": "Beat"}}
{"SetPartMode": {"part": "lead", "mode": "Sequence"}}
{"AddInstrument": {"instrument": {"name": "blip", "source": {"Sample": {"path": "sounds/blip.wav", "root_note": 60}},
                                  "provenance": {"author": "2026 Jesse", "license": "MIT"}}}}
{"SetInstrument": {"name": "organ", "instrument": {"name": "organ", "source": {"SoundFont": {"path": "soundfonts/cast.sf2", "bank": 0, "preset": 19}}}}}
{"RemoveInstrument": {"name": "blip"}}
{"SetBinding": {"binding": {"action": "jump", "part": "lead", "instrument": "organ"}}}
{"RemoveBinding": {"action": "jump"}}
{"AddPart": {"part": {"name": "lead", "track": {"Name": "melody"}, "mode": "Loop", "quantize": "Off"}}}
{"RemovePart": {"name": "lead"}}
{"SetProvenance": {"provenance": {"author": "2026 Jesse", "license": "CC-BY-4.0"}}}
{"Batch": [ {"SetPitch": {"notes": [3], "key": 62}}, {"SetPitch": {"notes": [4], "key": 65}} ]}
```

`SetTempo`'s `microseconds_per_quarter` and `SetTimeSignature`'s `meter`
take `null` to remove the change on that tick. `Batch` is the song's
composite: every member is checked before any lands, a refused member lands
none of it, and the whole is one undo entry. An edit to the sidecar is
checked by `SongFile::check_schema` on the file it would produce, so a ninth
instrument, a repeated name or a binding naming nothing is `invalid` and the
editor never holds a sidecar the loader refuses.

Song semantics worth knowing:

- **Guards**: every song write — the pure ones and `song new|open|save|close`
  — is `refused` in **Playing and Paused** (stricter than scene writes: the
  running game owns the music), while a dialog is pending, and **while a
  scene batch is open** (that batch is one undo entry over world commands; a
  song edit has no place in it — use `{"Batch": [...]}`).
- **No silent loss**: `song new`, `song open` and `song close` on a dirty
  song are `refused` unless the line ends in `discard` — the API has no
  dialog. The editor's GUI asks Save / Discard / Cancel instead, and Play
  over a dirty song asks Save / Play saved / Cancel (Save / Cancel for a song
  never saved): a click on Play never discards.
- **Separate watermarks**: saving the scene never marks the song clean, and
  saving the song never marks the scene clean.
- **Save is a journal** (`docs/SONG_FORMAT.md` § How the composer saves):
  the song needs its own `provenance` (author and license — set it with
  `{"SetProvenance": …}`, which undo covers like any edit) and so does every
  `Sample` instrument, or the save is `refused`, naming the song or the
  instrument and the field, and nothing is written; the path is held to the
  rule that file describes (no component starting or ending with `_`, no
  `__`, no whitespace, nothing under a home's `samples/`); samples are copied
  into the song's own folder (`music/samples/<stem>/` for a song directly in
  `music/`) with a `LICENSE` naming each and the song that owns the folder,
  and the home folder's `LICENSE` gets the song's own stanza; the `.mid` and `.mid.ron` land through a `.new` pair that
  `song open` recovers if the save was interrupted. A song saves only under
  `music/` or `ai/music/` in the asset folder; a `.mid` opened from anywhere
  else (or without a sidecar) is an import — `path` is `null` and its first
  save is a save to a new path, the original never written.

## Stages (audit §9.6)

- **Stage A** — query-only. Shipped.
- **Stage B (this document)** — write commands through `CommandHistory`
  (undoable in the GUI), `commands` self-description. Shipped.
- **Stage C** — headless mode (below). Shipped.
- **Stage D** — function bridge for the web editor and playground (`crates/playground`). Shipped.

## Stage C — headless mode (issue #45)

```bash
cargo run --bin editor --features editor -- /path/to/project --headless
```

No window, no GPU, no frame loop: the binary opens the project's first
scene (sorted, `assets/scenes/*.ron`) through the SAME load path as the
GUI, then answers the identical line protocol on stdin/stdout until EOF —
one JSON line per request, flushed per line. Logging stays on stderr.
`editor_integration::run_headless_editor_api(scene, input, output)` is the
library entry (CI tests drive it with in-memory buffers).

Semantics identical to the windowed `--api` mode, with these limits:

- **No Play.** There is no play verb in the protocol; the session stays
  `Editing` forever, so the writes-refused-while-Playing rule is
  unreachable. Physics never steps.
- **Textures are recorded, not loaded.** A path-recording resolver dedupes
  every texture reference to a stable handle and writes the same reference
  back on save — image files are never opened or validated, so a missing
  texture cannot fail headless authoring. Handles ARE validated against
  that resolver (#66): `set`/`add` refuse a handle it never issued, so a
  saved scene never carries a `#texture_N` placeholder.
- **`.sheet.ron` sidecars ARE consulted** (pure file I/O against the
  project's `assets/` directory), so a headless save bakes the current
  sidecar snapshot exactly like the windowed editor. Sessions started
  without an asset base (library use with `asset_base: None`) fall back to
  the scene's baked animation values.
- **Game-registered components** are visible only if the hosting process
  registered them; the stock editor binary links no game crate, so a scene
  containing game components refuses to load (fail-loud, #43). Author such
  scenes from the game's own binary.

### Decision of record — audit §6.6(5): the editor does not own the build

Resolved with Stage C (they hinge on the same file, `src/bin/editor.rs`):
the standalone binary stays a generic, data-only host wrapping `ProjectHost`;
games that want in-process editing embed via `run_game_with_editor`. The
API edits DATA (scenes), never code. Combined with §6.6(4)'s
relaunch-over-dylib decision, any future editor-triggered rebuild is
"spawn cargo, relaunch self" — which this shape already permits. No build
system work now (scripting Stage 5 revisits).
