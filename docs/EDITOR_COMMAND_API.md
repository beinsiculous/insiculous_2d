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
| `scene` | scene path, dirty state, entity count, play state |
| `commands` | self-description: every verb (usage/example/summary/writes/undoable) plus the LIVE `settable`, `addable`, and `archetypes` name lists |

Write verbs (Stage B — one undo entry each unless noted):

| Request | Effect |
|---------|--------|
| `set <entity> <Component> <json>` | shallow-patch the component with the REST OF THE LINE as raw JSON (never tokenized). Unknown fields are an `invalid` error listing the real ones; a non-object serialization (externally-tagged enums like `Behavior`) is a whole-value replace. Absent component → `invalid` ("add it first"). `set <entity> Name ...` is `invalid` — Name goes through `rename`. |
| `add <entity> <Component> [json]` | add default-valued (optionally patched — still ONE undo entry) |
| `remove <entity> <Component>` | remove (undo restores the VALUE, not a default) |
| `rename <entity> <name>` | assign/replace `Name` — works on unnamed entities; undo restores no-Name; empty/unchanged names are `invalid` |
| `create <archetype> [name] [x y]` | spawn an archetype (see `commands` for the list) at the viewport center or an explicit position; an empty name is `invalid` |
| `delete <entity>` | delete (children reparent to the grandparent; undo resurrects; the selection drops it) |
| `select <entity>` / `select none` | replace/clear the selection — never on the undo stack (GUI parity) |
| `undo` / `redo` | `{"undid"/"redid": <command name>}`, `null` on an empty stack (not an error); `refused` while a batch is open |
| `save [path]` | save through the editor's mandatory choke point |
| `batch begin [name]` / `batch end` / `batch abort` | group writes into ONE undo entry / roll them back |

Write semantics worth knowing:

- **Guards**: all writes are `refused` while **Playing** (Paused edits are
  allowed — inspector parity). `save` is additionally refused during any
  play session (Paused included) and while a batch is open.
- **Sanitation**: non-finite numbers anywhere in a JSON value are
  `invalid`; the same hard physical floors the GUI enforces apply
  (collider extents/radius, scale, audio volume/pitch).
- **Batches are NOT transactions**: each command executes immediately; an
  error mid-batch leaves earlier effects applied and the batch open —
  `batch abort` reverse-undoes what was collected. Pressing Play commits
  an open batch (its commands are in the world the snapshot captures);
  Stop DISCARDS a batch opened while Paused (its commands reference the
  runtime world the snapshot restore throws away). A
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
non-finite number); `refused` = the editor's current state forbids the
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
{"path":"assets/scenes/level.ron","dirty":false,"entity_count":12,"play_state":"editing"}
```

`path` is `null` for an unsaved new scene. `dirty` comes from the
`CommandHistory` watermark (the same source as the title bar's `*`).
`play_state` ∈ `editing` | `playing` | `paused`.

## Stages (audit §9.6)

- **Stage A** — query-only. Shipped.
- **Stage B (this document)** — write commands through `CommandHistory`
  (undoable in the GUI), `commands` self-description. Shipped.
- **Stage C** — headless `--api` (no window).
- **Stage D** — WebSocket transport for the web editor.
