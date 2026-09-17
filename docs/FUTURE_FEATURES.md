# Future features

These are parked Games concepts, outside the current roadmap. They carry no
implementation commitment or delivery date.

## Phone game studio with AI authoring

**Status:** Parked — September 16, 2026.

### The idea

A phone app containing the engine, editor, and game exporter could let someone
create, preview, and package a game entirely on their device. Exported games would
be separate installable apps. AI assistance could require an internet connection;
editing, previewing, and packaging would happen on the phone.

A shared engine command API would serve CLI users and an optional MCP adapter.
Someone could ask their preferred compatible chat companion to build or change a
game, inspect the result, and iterate. Claude would be the first chat integration
to investigate; support for other clients would need verification.

### Existing foundation and possible approach

The [editor command API](EDITOR_COMMAND_API.md) already supports scene inspection,
editing, undo, and grouped changes. The [web playground](WEB_PLAYGROUND.md) exposes
commands and file access to its host page, imports and exports project archives,
and previews projects. [Rhai scripting](SCRIPTING.md) lets game logic change
without rebuilding Rust. Current archive export produces a project ZIP, not an
installable game app.

One possibility is to package scenes, assets, and scripts with a precompiled
runtime on the phone. This could avoid compiling Rust for each game export while
keeping the engine lightweight. Games would use the runtime's available components
and scripting API; new Rust capabilities would require a runtime rebuild. This is
an approach to investigate, not an approved design.

### Questions for a future evaluation

- Can the editor provide usable, accessible touch controls and script editing on
  a phone, with reliable storage, suspend/resume, and acceptable battery use?
- Should the app host the web runtime or the native engine, and which devices and
  platforms can each support? Android is a candidate, not a committed target.
- How would on-device export handle app identity, icons, packaging, signing keys,
  installation, and later updates?
- How would an external chat app reach the phone editor, including when it is in
  the background? MCP exposes tools but does not provide that connection by itself;
  client support, authentication, pairing, and any relay need evaluation.
- What preview and diagnostic feedback would let an AI check the game it creates?

### Why it is parked

The idea expands the product beyond the current focus. Preserve it for a future
direction-setting discussion; it adds no work to the current roadmap or sprint.
