# Web Saves — the localStorage contract

The engine and the website (`beinsiculous.com`, repo `insiculous_web`) share
one origin, so **localStorage is the integration surface** for player
progress. This document is the contract; the site's reader
(`insiculous_web/src/lib/games-achievements.js`) and the engine's save-store seam
(`crates/engine_core/src/save_store/mod.rs`) both conform to it. Settled on
beinsiculous/insiculous_2d#17.

## Keys

One key per game and per document type. `<slug>` is the site's
`public/games/<slug>` directory name:

| Game repo | Slug |
|---|---|
| pong | `pong` |
| breakout | `breakout` |
| space_invaders | `invaders` |
| snake | `snake` |
| asteroids | `asteroids` |
| frogger | `frogger` |

| Key | Contents |
|---|---|
| `beinsiculous.games.<slug>.achievements` | Achievement unlocks |
| `beinsiculous.games.<slug>.scores` | High scores (Pong writes none — versus play has no single score) |
| `beinsiculous.games.<slug>.input` | Player input bindings |

Keys are deliberately separate per game and per document type (the site's
`myfort-store.js` precedent: sharing a key lets one document migrate into
another when either changes shape).

The Web Playground (`/playground/`) stores editor UI preferences under the
`beinsiculous.playground.editor_prefs` key via the same `SaveStore` contract
— the same JSON `EditorPreferences` document the native `editor_prefs.json`
holds, written on the editor's settle rule. The project files themselves live in
IndexedDB, not in `SaveStore`; `docs/WEB_PLAYGROUND.md` is that contract.

## Values — byte-identical to the native save files

Every value is exactly the pretty-printed JSON the engine writes natively
(`saves/<game>_achievements.json` etc.). There is no web-specific format and
no migration: the engine's save/load code runs unchanged on both targets, and
`GameConfig`'s save-path strings double as the localStorage keys on wasm.

### `.achievements` — `SaveFile` (`crates/engine_core/src/achievements/mod.rs`)

```json
{ "unlocks": { "<achievement id>": { "unlocked_at": 1724700000 } } }
```

`unlocked_at` is unix **seconds**. The save carries ids only; the site
prettifies ids for display.

### `.scores` — `ScoresFile` (`crates/engine_core/src/scores.rs`)

```json
{ "modes": { "<mode>": [ { "score": 123, "at": 1724700000 } ] } }
```

Per mode: at most 10 entries, sorted score-descending, ties oldest-first.
Mode strings are game-defined (lowercase, stable — e.g. `"single"`,
`"coop"`, `"versus"`). `at` is unix seconds.

### `.input` — `SettingsFile` v1 (`crates/engine_core/src/input_settings_io.rs`)

```json
{ "version": 1, "players": [ { "pad": 0, "bindings": [ { "action": "...", "sources": [ ... ] } ] } ] }
```

The site does not read this key; it exists so bindings survive a reload.

## The save event

Same-tab localStorage writes never fire the browser's `storage` event, so
after **every successful localStorage persist** the engine dispatches a
`CustomEvent` on `window`:

- **name:** `insiculous-save`
- **`event.detail`:** the localStorage key that was written (a string)

It fires synchronously mid-frame — listeners must be cheap (note the key,
schedule a re-read). This is how a game page updates its achievements panel
live while the game runs.

## Degrade rules

- **Storage unavailable** (private browsing, storage blocked): the engine
  warns once on boot and keeps saves in memory for the session — no keys are
  written and no events fire.
- **Write failures** (e.g. quota) are logged, never fatal. Saves are
  write-through full-document rewrites, so a later successful save
  self-heals earlier failures.
- **Multi-tab:** achievements and scores **merge on save** (a tab unions the
  stored document into its own before writing — unlocks keep the earliest
  timestamp, scores dedup + re-rank). This protects the common case of tabs
  saving at different times; the read-merge-write is not atomic, so two tabs
  persisting in the same instant can still race, in which case the losing
  tab's entry is restored by that tab's *next* save (its in-memory state
  still holds it). Input bindings are last-writer-wins by design (they are a
  preference, not accumulated progress).
- **Quota is per-origin** and shared by every game on the site. All payloads
  are KB-scale by design (score lists are capped at 10 per mode).
- **Corrupt documents** are warned about and treated as empty; the next save
  replaces them.

## Follow-up (not implemented)

If the site's boards ever want real achievement names/descriptions instead of
prettified ids, the plan of record is a per-game `achievements.json` manifest
exported beside the wasm bundle (definitions today live in each game's Rust
registration and the localized games' locale RON files).
