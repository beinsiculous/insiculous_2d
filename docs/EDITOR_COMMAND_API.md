# Editor Command API

The editor answers structured questions about the open scene over a
line-oriented protocol: **text requests in, single-line JSON responses
out**. This is Stage A of the audit §9 command layer — read-only queries.
Write commands (Stage B) will route through `CommandHistory` so every API
mutation is undoable in the GUI.

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

`kind` ∈ `parse` | `not_found` | `ambiguous_name`. An `ambiguous_name`
error additionally carries `"matches":[<id>,...]`.

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

Hidden registry entries (`Name`, `GlobalTransform2D`, `BehaviorState`) are
internal and not listed in `components` — `Name` is the top-level `name`
field. A component that fails to serialize contributes the string
`"!serialize error: <e>"` for its key so the map stays total.

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

- **Stage A (this document)** — query-only. Shipped.
- **Stage B** — write commands over the existing `EditorCommand` set,
  dispatched through `CommandHistory`; `ListCommands` self-description.
- **Stage C** — headless `--api` (no window).
- **Stage D** — WebSocket transport for the web editor.
