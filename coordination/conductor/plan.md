# Conductor (Phase K) — the plan

**This is the settled plan** (Jesse, 2026-10-20, after three review rounds: 61 findings, 57
accepted, 4 rebutted — `review/conductor/rebuttal-{1,2,3}.md`; the scorecard beside this file
carries every row). Batch 1's handoff is `review/conductor/handoff-1.md`.

**v4 — 2026-10-20, after round 3** (`review/conductor/rebuttal-3.md`: kimi 13, gemini 8, all
accepted; codex sat out, no screen changed). Edges made explicit: the flush command carries an
epoch and retries like transport; triggers carry a producer timestamp and stale ones are
dropped, which also measures quantization lateness from the action, not the drain; sample
slots track voices directly (no lanes) with their own live-voice cap; the background sequencer
coalesces same-key overlaps; `write_smf` excludes edited notes from the retained events; only
trigger commands are sorted; composer-saved samples carry provenance into the `LICENSE`; the
decode cache is per dependency file, capped; Play is a two-frame transition; recorded emitters
are tracked and stopped on despawn.

**v3 — 2026-10-20, after round 2** (`review/conductor/rebuttal-2.md`: kimi 6, gemini 8,
codex 7; 19 accepted, 2 rebutted on the issues' settled text). The release-tail *estimate*
v2 added is gone: rustysynth's stealer always takes a releasing voice before a sounding one
(verified in its source), so the ledger bounds sounding notes exactly and the guarantee is a
bounded discontinuity, not an estimate. Also: lazy source activation (no activate command),
lanes leased while sounding, the `.new` pair as the atomic recovery unit with web
acknowledgement before cleanup, controller-state reconstruction on seek and loop, a
game-owned source for sounds that outlive their entity, samples namespaced per song,
±24 semitones, fingerprint-keyed decode cache, exports carrying unsaved samples.

**v2 — 2026-10-20, after round 1** (`review/conductor/rebuttal-1.md`: kimi 10 findings, 8
accepted; gemini 7, all accepted, one in part; codex 8, all accepted, one as a restriction).
The big ones: one bounded command channel with reserve accounting instead of two queues; the
voice ledger budgets releasing voices; the spike is scaffolded from `game-template` so
`build_wasm.sh` can build it; the save journal recovers by fingerprint; both `assets/music/`
and `assets/ai/music/` are home directories; spatial emitters compute their own 2D pan.

**v1 — 2026-10-20, drafted.** Written by the planner (Claude Fable) with Jesse's four kickoff
rulings: public-domain music sequenced in-house, the casting table as the starting point, a
Claude Opus session as executor, one shared SoundFont subset. Review round 1 (kimi, gemini,
codex, one snapshot) dispatched the same day; this file is revised after each rebuttal and the
plan is settled when Jesse says so.

Effort directory: `coordination/conductor/` (this plan, `reviewer-comparison.md`). Review
transients: `review/conductor/` (the plan snapshot, `review-N[-reviewer].md`, `rebuttal-N.md`,
`handoff-N.md`, `report-N.md`, `draft-N.diff`); game halves from `games/<g>/review/conductor/`.
Roster: Claude Fable plans, adjudicates with Jesse, applies accepted fixes and commits; a
Claude Opus session (Jesse's other window) executes each handoff; kimi reviews every plan
round and diff; gemini (the Antigravity CLI) reviews every harness, fixture and seam — every
code batch here; Astra (codex) reviews every plan round and every batch with a screen. A
vendor never reviews its own vendor's work, so no Claude reviews.

Sprint: milestone 2 "Conductor (Phase K)" on `beinsiculous/insiculous_2d` — #60 K1, #61 K2,
#62 K3, #140 K7, #63 K4, #174 K9, #141 K8, #64 K5, #65 K6 — plus the games' halves on their
own repos, held together by the Studio Board's Sprint field.

## Context

The engine's "Conductor (Phase K)" milestone (insiculous_2d milestone 2, nine open issues
#60 K1, #61 K2, #62 K3, #140 K7, #63 K4, #141 K8, #64 K5, #65 K6, #174 K9) is the next sprint,
named by Jesse on 2026-10-19 after the Pattern Audit closed. The issues were adversarially
reviewed and settled on 2026-09-10; they are the contract. This plan turns them into handoff
batches, fixes the music-sourcing rule Jesse gave at kickoff (2026-10-20: real songs and MIDI
files are public-domain classical music, or another public-domain genre that fits a game's
theme — nothing original), and records the architectural constraints found before batch 1.

## Facts established before drafting (2026-10-20)

- rustysynth 1.3.8 (MIT, std only, no deps, nothing blocking wasm32): `note_off(channel, key)`
  releases every voice matching (channel, key); no per-voice handle is public; polyphony
  overflow steals the lowest-priority voice with a hard restart (no fade); `render(left,
  right)` takes any length and chunks internally by `block_size` (8..=1024, default 64);
  `maximum_polyphony` 8..=256 (layered regions cost more than one voice per note);
  `SoundFont::new(&mut impl Read)` — a `Cursor<&[u8]>` works; SF3 is rejected
  (`UnsupportedSampleFormat`), so the shipping bank is SF2.
- Consequence for K1/K3: isolation = one `Synthesizer` per instrument slot (bounded), one MIDI
  channel per active source within it (16 lanes per instrument), background score on its own
  synthesizer; a voice ledger owned by `music_midi` keeps each synth under
  `maximum_polyphony` minus a measured layering headroom so that stealing is *our* `note_off`
  (a real envelope release), never the synth's hard cut. K1 measured it: layering ≤ 2 for
  every cast preset, pool 128 bit-identical to pool 256 under the worst-case storm.
  **Verified in rustysynth's source (round 2):** a voice's steal priority is `4 + value` in
  delay, `3 + value` in attack, `2 + value` in hold, `1 + value` in decay/sustain and plain
  `value` (≤ 1) in release, `0` once inaudible; the stealer takes the lowest, oldest on ties —
  **a releasing voice always ranks below every sounding voice.** The release time constant is
  per region, read from the bank, unscaled by key or velocity (velocity only lowers the
  starting level, so it shortens a tail); a voice ends no later than `release` seconds after
  note-off. `note_on` on an already-sounding (channel, key) starts a second voice; an
  exclusive-class region (drum kits) restarts the voice abruptly.
- Public-domain music: the composition and a given MIDI file carry separate rights. A MIDI
  file someone else sequenced is theirs unless its page says otherwise. Mutopia tags each piece
  individually (Public Domain, CC-BY, CC-BY-SA); the per-piece line is authoritative.
- SoundFont candidates: FluidR3 GM (MIT, 141 MB) and MuseScore General (MIT, SF2 form) are the
  permissive banks; neither is small. The shipping bank is a documented *subset* (named
  presets only) of one of them, 1–2 MB, with the MIT notice carried in the provenance file.
  TimGM6mb (5.7 MB) is GPL and excluded.

## Where the plan lives, who does what

- Tracked plan: `insiculous_2d/coordination/conductor/plan.md` (engine effort, like pattern-audit;
  arcade-shelf sat at the root only because it was cross-repo). Game halves (K6) are reviewed
  from `games/<g>/review/conductor/` because `request-review.sh` refuses artifacts outside its
  own repo. Transients: `insiculous_2d/review/conductor/`. Scorecard:
  `insiculous_2d/coordination/conductor/reviewer-comparison.md`, pattern-audit's header.
- Roster (`roles`, with Jesse's call of 2026-10-20): planner Claude Fable; **executor a Claude
  Opus session driven by Jesse in another window from each handoff**; quality reviewer kimi
  on every diff; gemini (the Antigravity CLI) as the seam reviewer on every batch that touches
  a harness, a fixture or a public seam (that is every code batch here); artist Astra
  (`codex`) on every plan round and every batch with a screen — K9's popup and inspector
  rows, K8's three editor batches, K5's beat pulse, K6's beat visuals. Vendor rule holds:
  Claude authors and executes, so Claude never reviews; kimi, gemini and codex do. Plan
  review: kimi + gemini + codex on one snapshot per round, one rebuttal per round.
- Branch: engine `jesse` from Iroh (main, dev and jesse all sit at 7808478 today), merged to
  dev at the end as the Pattern Audit was; games on `main` (each is its own repo).

## Music: the sourcing rule (Jesse, 2026-10-20) made concrete

**Rule.** Every song and MIDI file is a public-domain composition — classical by default, or
another public-domain genre that fits the game's theme. Nothing original. Three consequences:

1. **Shipping songs are sequenced in-house, from public-domain scores, in the K8 composer.**
   A MIDI file someone else sequenced carries that person's rights, so none is shipped. K6
   follows K8 and K5, so the composer exists when the games' songs are made; each song is a
   16–64 bar arrangement of a public-domain work. If an agent sequences the first version it is
   AI output under `DEION_STYLE.md` §6: it lives at `assets/ai/music/ai_<slug>.mid`, ships free
   tier only, and Jesse's or M's pass in the composer (the source-agnostic human-cleanup
   crossing) is what moves it to `assets/music/<slug>.mid`. Same policy as the art.
2. **Test fixtures for K1–K4 and K7 come from Mutopia, verbatim, only from pieces whose page
   says "Public Domain"** (not the CC-BY/CC-BY-SA ones; the per-piece line is authoritative),
   plus a generated one-sine-sample SF2 and synthetic WAVs. Needed shapes: one SMF type 1 with
   several tracks and a tempo change; one SMF type 0 mixing melody and percussion channels
   (if Mutopia has none, K1 generates it from the type-1 piece with a documented script); one
   time-signature change.
3. **Provenance travels with every audio file.** A `LICENSE` beside each `assets/music/`,
   `assets/soundfonts/` and fixture directory in the Debian copyright format already used by
   `crates/editor/assets/fonts/LICENSE`: composer, work, year, death year, source edition and
   URL, who sequenced it and when, license. The SoundFont subset lists its source bank, its
   preset numbers and the MIT notice.

**SoundFont.** An SF2 subset of an MIT bank — FluidR3 GM, measured by K1: the twelve cast
presets are 32.6 MiB, 22.0 MiB without the full drum kit (kit 10.6, piano 7.5, strings 5.3,
the rest under 2.7 each); MuseScore General is larger (its piano alone 98 MiB). **The 2 MB
target is waived (Jesse, 2026-10-20: "who cares? we need music").** What replaces it, for K2
to freeze: a per-game audio budget of **≤ 24 MiB downloaded** (bank ≤ 22, songs ≤ 256 KiB,
samples ≤ 2 MiB) and **≤ 50 MiB loaded**, with three levers — a key-range kit subset (snare,
bass drum, a cymbal, not the whole kit), per-game preset lists, and fetching the bank after
the first frame instead of blocking boot (every asset is prefetched today; 27 s at 10 Mbit/s,
5.5 s at 50). The hard line is hosting (Cloudflare's 25 MiB per static file); the one site
URL shared by the six games, so the browser caches it once, is K2's call. SF3 is out
(rustysynth rejects it); GPL banks are out. Subsetting is not authoring (K's exclusions stand);
`scripts/subset_soundfont.py` does it byte-reproducibly.

**Casting (proposal, all public domain; the plan review and Jesse's ear adjust it):**

| game | Deion title | song | why |
|---|---|---|---|
| pong | Tong | Bizet, *Toreador Song* (Carmen, 1875) | a grill showdown is a bullfight; strong 2-bar refrain for the K5 shared part |
| breakout | The Food Pyramid | Verdi, *Triumphal March* (Aida, 1871) | pyramids; a climb that pays off at the top (Maxwell finale) |
| space_invaders | Toast Busters | Saint-Saëns, *Danse Macabre* (1874) | ghosts; the waltz's bones-rattling xylophone is the Toasted |
| snake | Bratdog | J. Strauss II, *Tritsch-Tratsch Polka* (1858) | a bratwurst dog's kitchen romp wants a polka |
| asteroids | Meatieroids | Holst, *Mars* (The Planets, 1914–16; published 1921) | the space brawl; 5/4 is a tempo-map test in itself |
| frogger | Chicken Coop | Rossini, *William Tell Overture* finale (1829); alt. Daly, *Chicken Reel* (1910) | the slapstick chase; Chicken Reel is the cartoon-chicken tune |

Menus use the same song's slowest phrase through the `menu` binding (K6's standard name).

## What the tree holds (verified 2026-10-20)

- `crates/audio` is small and rodio-contained: `manager/mod.rs` 348 (device, SFX, the one
  generic `start_sink<S: Source + Send + 'static>(output, source, base, bus, looping)` at
  :293), `manager/music.rs` 176 (play/stop/pause/resume, `pending_music` for the disabled
  and pre-gesture state, three buses `base * bus * master`), `manager/tests.rs` 417,
  `sound.rs` 80. rodio 0.20 `symphonia-all` natively, `wasm-bindgen` feature on wasm32.
  Gesture upgrade: `engine_core/src/game/web.rs` `upgrade_audio_on_gesture`, called before
  key dispatch in `game/app_handler.rs:325`. No position, seek or beat query exists.
- No MIDI, tempo, beat or sequencer code anywhere; nothing depends on midly, rustysynth or
  hound. Workspace edition 2021, rustc 1.94, no MSRV pin.
- No game calls `ctx.audio` today; Pong's paddle beep was dropped 2026-09-17 (1549577).
  `examples/assets/sounds/{beep_e,snd_jump}.wav` are the only audio files in any source tree.
- `common::vfs` is std::fs natively and the prefetched `MemFs` on wasm; `build_wasm.sh`
  manifests every file under `assets/`, so `.mid`/`.sf2`/`.wav` ship with no script change.
- `AssetManager` (`engine_core/src/assets.rs`, 552) has textures and sheets, "audio (future)"
  in its doc, and the `TextureResolver` seam (`texture_ref.rs:31-49`) that K9 copies.
  `ComponentData` typed variants are reserved for load-time resolution (`scene_data.rs:380`).
  `scene_retired_components.rs:14` drops `AudioSource`/`AudioListener`/`PlaySoundEffect`
  while unregistered.
- Scripting: `ScriptCommand` enum (`engine_core/src/scripting/commands.rs:55-77`) applied in
  `runner.rs:472-502`; rhai verbs one `register_fn` each in `rhai_bindings.rs`; `Target`
  resolves `Entity(EntityId)` through `world.validate_entity`; `EntityId { id, generation }`
  (`ecs/src/entity.rs:9-14`) is the generation-safe identity K3 needs.
- Editor: one scene document, one `CommandHistory` (world-bound `EditorCommand`), no audio
  anywhere in `editor`, `editor_integration` or `playground`; `playground_is_dirty` =
  history flag OR pending puts; `playground_export_zip` swaps only the live scene entry;
  `archive::import_project` whitelists `project.ron` and `assets/**` by extension.
  `AssetKind` is Image/Scene/Script; `PanelId::CONSOLE` (4) is unused. Largest files:
  `panel_renderer/asset_browser.rs` 598, `persist/mod.rs` 586, `commands/mod.rs` 555.
- Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  `cargo clippy --features editor --all-targets`, `cargo check --workspace --all-targets`,
  `scripts/check_wasm.sh` (whole workspace, no `--all-targets`), `scripts/check_games.sh
  [--test]` (seven games incl. game-template), `check_guide_tests.sh`, `check_features.sh`,
  the comment-tag grep, 600 lines, no `#[allow]`, no `unwrap` outside tests.
- The retired design (`review/plan-conductor.md`, Aug 28, survives only in
  `~/RustroverProjects/insiculous_2d/`) settled: the `music_midi` crate shape, no Mutex (commands
  over `std::sync::mpsc`, position in atomics), clock advances by samples rendered, 64-frame
  blocks, pause-menu jamming by construction. The issues superseded its trigger model
  (parts + instruments + bindings instead of one track per action); the threading model
  stands and is restated in D3.

## Decisions of record (the planner's, 2026-10-20; confirmed by Jesse where marked)

1. **Public-domain music, sequenced in-house** — § Music above. Confirmed by Jesse
   2026-10-20: the songs are sequenced in the composer (an agent's first version under
   `ai/`, a human pass promotes it); the casting table is the starting point, adjusted per
   game by ear at K6; **one shared SoundFont subset (~2 MB) copied into each game's assets**,
   one preset list, one `LICENSE`, one subsetter run. The Mutopia-fixture choice is the
   planner's, open to the review.
2. **Isolation: one rustysynth `Synthesizer` per instrument slot, one channel per active
   source.** The background score has its own synthesizer (16 GM channels, program and
   control events honoured). Each action instrument gets a slot with its own synthesizer
   whose 16 channels are *source lanes*: a source holds a lane while it has sounding voices,
   so two sources on the same instrument at the same pitch are different channels and
   `note_off` cannot cross them. Preset state is per slot, so a program change on one
   instrument never retimbres another. Capacities (K1 measures, K2 freezes): 8 instrument
   slots, 16 lanes per slot, 256 queued commands, 512 pending scheduled events, and per slot
   a **sounding-note cap** of 24 with the synth pool sized so sounding voices can never
   fill it: `polyphony ≥ sounding_cap × max layering + tail headroom` — 128, measured by K1 (7% of
   real time natively under the worst case, layering ≤ 2; rustysynth's maximum is 256).
   **15 lanes per slot, not 16:** rustysynth's channel 9 is General MIDI percussion and adds
   128 to its bank, so lanes skip it. **When no lane is free, lanes steal like voices:** the
   oldest source's sounding notes are released and its lane taken, oldest first — a 16th
   one-note source is never starved (K1's code rejected it; the rule is batch 4's). The ledger knows the
   sounding set exactly (it issued every note-on and note-off) and steals among *sounding*
   notes only: the oldest gets our `note_off` (a real release) to make room for the
   newcomer; above the per-instrument polyphony cap the newcomer is rejected without
   consuming the cursor. Tails are the synth's: when the pool is full its stealer cuts a
   releasing voice, never a sounding one (the priority rule above), so the guarantee is
   **no sounding note is ever cut by the synth, and a cut tail's discontinuity is bounded** —
   K1's stress test (the worst-case storm, long-release presets, one lane hammering one key)
   measures the maximum sample-to-sample delta and the plan freezes it as the threshold the
   test asserts from then on. No release-time estimate anywhere; the per-region release
   maxima read from the bank serve K2's load-time check of the background score (its
   maximum simultaneous note-ons × layering against the pool, a warning over) and
   diagnostics only. A one-shot sample voice gets a 5 ms linear fade. **A same-pitch
   retrigger on a lane** sends `note_off` for the old note (it becomes a tail, no longer
   sounding), cancels its pending note-off, then `note_on` for the new. **A lane is leased
   while its source has sounding notes** and freed at the last note-off; a tail may ring on
   a lane a new source takes (a same-key note-off from the new lessee reaches only a tail
   already releasing); more than 16 sources sounding at once on one instrument is overflow
   by the per-instrument cap anyway. **Lanes are a synth-slot concept:** a sample slot has no
   MIDI channels and tracks its voices directly; a one-shot is *sounding* until it ends (its
   length is exact — the pitch-scaled sample length) and counts against the sample slot's
   own live-voice cap (16 proposed, K1 measures) with oldest-voice steal through the 5 ms
   fade, so no lane can wedge and no tail goes uncounted. **`SourceId` carries an activation epoch** beside the
   entity generation: re-entry is a new epoch, and queued work for an old epoch is dropped.
   **Activation is lazy:** a source's first trigger activates it; there is no activate
   command, so a trigger can never arrive before its activation. *Rejected:* a synth per
   entity (the issue forbids it); forking rustysynth for voice handles (KISS); a measured
   release estimate (round 2: unsafe across velocity, key zones and controllers, and
   unnecessary given the priority rule); round-robin lanes per note.
3. **Threading as the retired plan settled it.** The `Conductor` moves into a rodio
   `Source<Item = f32>` in `crates/audio` (`ConductorSource`); the game thread keeps **one**
   `std::sync::mpsc::SyncSender<ConductorCommand>` (bounded, `try_send`, never blocks) and
   `Arc<SharedPosition>` (AtomicU64 sample position, AtomicU64 tick, AtomicBool ended,
   overflow counters). One channel, so FIFO holds between a trigger and the deactivation
   that follows it (round 1: two queues had no cross-channel order). The lifecycle reserve
   is capacity accounting, not a second channel: an `AtomicUsize` in-flight counter the
   producer increments on send and the consumer decrements on drain; a trigger is admitted
   while `in_flight < capacity − lifecycle_reserve`, a lifecycle command while `in_flight <
   capacity`. A deactivation the host cannot send this frame stays pending in
   `SoundSources` and is retried next frame, so a stalled consumer never loses one; order
   does not matter for it because it names an epoch and touches only that epoch's voices.
   Transport commands (pause, resume, stop, seek) that fail admission stay pending in the
   manager and are retried from `update()` before any trigger is admitted. World replacement
   or a song change flushes `SoundSources` and sends `StopAllSources { flush_epoch }` through
   the reserve — it kills only sources activated before that epoch, and while pending it is
   retried from `update()` before any trigger is admitted, like transport — so the next
   trigger from a still-valid entity re-activates it under a new epoch and survives the
   flush (kimi's round-2 and round-3 cases). **Every `Trigger` carries the producer's
   `common::clock` instant**; the consumer drops a trigger older than the late tolerance and
   counts it (a suspended tab's backlog never replays as a burst), and quantization measures
   lateness from that stamp, not from the drain, so a frame of transport latency is not
   lateness. A lifecycle command is never dropped for age. **On the web the stamp is not
   optional:** cpal's WebAudio backend pulls 2048-frame buffers from the main thread, so a
   command waits up to 53–61 ms before it is drained (K1 measured); K3 places a trigger
   inside the burst from its stamp, or every web trigger quantizes to about 46 ms. Every output sample passes a
   finite guard (a non-finite value becomes 0 and is counted) so nothing can poison WebAudio. Bus gains cross as three `Arc<AtomicU32>` f32 bit patterns read per
   block; the source mixes `music_gain * background + sfx_gain * instruments` and the sink
   volume carries master. The score clock advances by samples rendered, so `time_scale = 0`
   never stops it and zero music gain never stops it either. On wasm the same code runs on
   the main thread inside cpal's callback. No Mutex, no new threading dependency. **In
   disabled mode** (headless tests, `GameHarness`, no device) the manager advances a virtual
   position from `update()`'s delta so `song_position` and `beat_crossed` are testable; a
   song pending the web gesture reports not-playing, as K4 says, with no fallback clock.
   The loop seam is sample-exact: `render` splits a block at the seam into pre-seam and
   post-seam sub-renders, so the loop length never rounds to a block. **Seek and loop wrap
   reconstruct state:** all-notes-off (release) on every synth, part cursors set to the
   destination tick, and the per-channel program, bank, controllers and pitch bend folded
   from the score up to that tick re-applied; a note spanning the destination is not
   re-sounded. `beat_crossed` reports at most one crossing per `update()` (a latched edge,
   never a count flood after a suspended tab); position resumes from the rendered sample,
   and a resync policy after suspension is a named follow-up owned by batch 6.
4. **One music slot, last request wins across kinds.** `play_song` stops recorded music and
   `play_music` stops a song; a song requested before the gesture is `pending_song`,
   mirrored on `pending_music` (one `PendingMusic` enum with two variants, not two fields).
5. **Sample instruments decode in `crates/audio`, render in `music_midi`.** The WAV loader
   uses the rodio `Decoder` already in the crate and yields `OwnedPcm { sample_rate,
   channels, frames: Arc<[f32]> }`; `music_midi` never decodes. Limits (K1 proposes, K7
   freezes; K1 enforced them before allocation): ≥ 1 ms and ≤ 10 s decoded, 8–96 kHz, mono
   or stereo, finite samples only, and **±24
   semitones from the root** (validated in `SongFile`, clamped in `SampleVoice`; the
   composer's root-note field says pitching up aliases, since the resampler has no filter). Resampling
   is linear interpolation at `2^(semitones/12) * source_rate / output_rate`; duration
   changes with pitch and the composer says so beside the root-note field. **Limits bind
   before allocation** (round 1): byte limits are checked before parsing (MIDI ≤ 1 MiB, SF2
   ≤ 8 MiB, WAV ≤ 4 MiB), event counts and the per-song aggregate during parsing, and a
   decode stops at the frame limit — nothing oversized is built in full and then rejected;
   K1 proposes the numbers, K2 and K7 freeze them.
6. **K9's component is new-named.** `SoundEmitter { sound: SoundRef, volume, play_on_spawn,
   looping, spatial: Option<Spatial { max_distance }> }` where `SoundRef` is a recorded
   file path or a binding name — never `AudioSource`/`AudioListener`/`PlaySoundEffect`, so
   `scene_retired_components.rs` needs no wire-version note. Resolution is lazy, in a
   frame-tail `SoundEmitterSystem` that has both `ctx.assets` and `ctx.audio`, through a
   `SoundLibrary` (path → `SoundHandle`) resource — the asset tier #174 asks for, placed
   where both managers are reachable. Spatial = distance attenuation and stereo pan from the
   main camera, computed in 2D by the system itself in world units (`distance /
   RENDER_UNIT`; `max_distance` is in world units) and applied through `SoundSettings` gain
   and a new pan field — no `rodio::SpatialSink` (its 3D model in metres would silence
   anything a few pixels away); no occlusion. A **binding-backed** emitter supports
   play-on-spawn, trigger and stop (stop deactivates its source, which the conductor
   already scopes per source); `spatial` on a binding emitter is a scene-load error naming
   the entity — conductor voices are musical, not positional.
7. **Dependencies: exactly `midly` and `rustysynth`, in `crates/music_midi` only** (K2);
   K1's spike project carries its own copies outside the workspace. Any third dependency is
   a plan change, not an executor's call. The handoff template's "no new dependencies" rule
   is amended by this line for those two crates in that one `Cargo.toml`.
8. **Gates land before the batches that need them.** `scripts/check_audio_provenance.sh`
   (every file under an `assets/music`, `assets/soundfonts`, `assets/sounds` or a crate
   `fixtures/` directory is named in a `LICENSE` beside it) lands in batch 1 with the first
   fixtures and runs in every later batch; the SoundFont subsetter and the sine-SF2
   generator are scripts committed with their outputs, so every binary asset is
   regenerable.
9. **Order of the batches.** K9 runs after K4 and before K8: it needs `trigger` through the
   audio API and its asset-browser `Sound` kind is what K8's instrument picker reuses.
   K7 runs before K4 as the milestone says (K4 consumes the WAV loader). Everything else
   follows the milestone's order: K1 → K2 → K3 → K7 → K4 → K9 → K8 → K5 → K6.
10. **Batch size.** Each handoff is one reviewable diff of a few thousand lines at most;
    the Large issues split (K2 in two, K4 in two, K8 in three, K6 in three game pairs).
    One batch out at a time, sequential, as the house rule says.
11. **Branch and merge.** Engine on `jesse`, merged to `dev` when the effort is done; each
    game on its own `main`. The effort name is `conductor`.
12. **The composer is a second document, not a scene component.** `SongDocument` is a pure
    type in `crates/editor` (`song/`), owned by `EditorGame` beside `command_history`, with
    its own `SongHistory` that copies only the scene history's watermark contract (next id
    from 1, `saved_id`, dirty = top id ≠ saved id, merge reassigns the id and clears redo,
    `break_merge` seals a gesture, cap 100) — *not* a generic `DocumentHistory<D>`:
    `CommandHistory` is world-bound in four places and its play-session floor has no song
    analogue, so extracting a core would touch every scene command for no reuse. One
    mutation path, `SongDocument::apply(SongEdit, gesture)`, serves the GUI and the command
    API's `song edit <json>` verb, which is what makes parity structural. Song writes are
    refused in Playing *and* Paused (stricter than scene writes, which allow Paused; two
    guards, documented). The Play prompt on a dirty song offers Save / Play the saved version
    / Cancel — "discard" on a Play click would destroy work, so the Alt arm keeps the edits
    (a deliberate narrowing of the issue's save/discard/cancel wording). The save is a
    journal pair (`.new` files, a fingerprint in the sidecar, recovery on open) because
    `common::vfs::write` has no rename on either target. **The `.new` pair is the atomic
    recovery unit:** recover only when both files are present and the MIDI bytes match the
    sidecar's fingerprint; otherwise the intact finals stand and the journal is dropped with
    a report. A lone `.mid.new` is offered as an *import* (dirty, `path: None`), never
    written over a final — work is not silently lost, and a mismatched pair is never
    produced. On the web the pair is removed only after the persistence chain reports both
    finals acknowledged (`PathState::Idle`); a `SongSaveTransaction` polled in `finish_frame`
    owns the sequence and `mark_saved` fires at its end; the transaction has a timeout and an
    error terminal (report, finals-or-journal kept consistent, the song stays dirty with the
    reason on the status bar), and save-while-saving is refused with a status-bar message.
    A leftover `.new` that matches the finals' fingerprint is purged silently on open; a
    `.mid.ron.new` without its partner is reported and removed. Samples are copied under
    `assets/music/samples/<song stem>/` (an unsaved song stages them under
    `samples/_unsaved/`, remapped into `<stem>/` by Save or Save As in the same journal); a
    byte mismatch on a same-named file inside that directory gets a short content-hash
    suffix and the instrument path follows — never a refusal. **A sample instrument carries
    `provenance { author, license }`, required;** Save writes it into the `LICENSE` beside
    the sample directory as part of the journal and refuses to save without it, so the
    provenance gate stays whole over files the composer itself writes. On Play, the saved
    song's dependency closure is loaded and validated *before* the preview is stopped, and
    Play is a two-frame transition: the request frame validates and drains the preview's
    stop in `finish_frame`; the session (snapshot, `init`) starts the next frame, so a game's
    `play_song` in `init` is never clobbered; a failure keeps the preview. **Path policy:** `assets/music/` and `assets/ai/music/` are both
    home directories for save, Save As and the web byte upload; an agent-authored song is
    saved under `ai/` with the `ai_` prefix, and promotion is a human Save As into
    `assets/music/` without it. The zip export carries the live, unsaved song exactly as it
    carries the live, unsaved scene (`playground/src/bridge.rs:373`); the docs say so. The preview session lives in
    `editor_integration` (`song_preview.rs`) as pure intents drained into `ctx.audio` in
    `finish_frame`, so the headless suite drives Play/Pause/Stop interplay without audio.
    *Rejected:* songs as scene component edits (the issue forbids it); a native file dialog
    (none exists; the asset-folder convention and `song open <path>` on the API stand in).

## Ground rules for every batch

The pattern-audit list verbatim, plus the sprint's own:

- Engine gates: `cargo test --workspace` (0 failed, 0 ignored), `cargo clippy --workspace
  --all-targets` (0 warnings), `cargo clippy --features editor --all-targets`, `cargo check
  --workspace --all-targets`, every touched file ≤ 600 lines, no new `#[allow]`, no
  `unwrap()` outside tests, no dependency beyond decision 7, the comment-tag grep prints
  nothing, `scripts/check_guide_tests.sh` and `scripts/check_features.sh` exit 0.
- `scripts/check_wasm.sh` on every batch that touches `crates/music_midi`, `crates/audio`,
  `crates/engine_core`, `crates/common`, `crates/editor*` or `crates/playground` (that is
  every code batch here); `scripts/check_games.sh` on every batch that changes a public
  item of `engine_core`, `ecs` or `audio`, `--test` when a game's behaviour is exercised.
- `scripts/check_audio_provenance.sh` from batch 1 on. `deion_assets/scripts/check_no_ai_assets.sh`
  is unchanged: AI-sequenced music sits under `ai/` with the `ai_` prefix like AI art.
- Review and commit as the house loop says: `git diff --cached > review/conductor/draft-<batch>.diff`,
  kimi always, gemini as the seam reviewer (deepseek stands in if gemini is out or if a
  batch is ever handed to agy), codex when the batch has a screen (K9's popup, every K8 batch, K5's pulse, every K6
  batch), the planner's own review, adjudication with Jesse, `rebuttal-N.md`, planner-applied
  fixes enumerated, then `ADV_REVIEWED=1 git -C insiculous_2d commit -F <message-file>
  --pathspec-from-file=<scope-file>`. Game halves the same way from `games/<g>`. Never a
  skip trailer.
- Pathspec commits only; done marks are their own commits on `coordination/conductor/plan.md`.
- Before each handoff the planner re-verifies the batch section against the tree and commits
  the corrections into the section. One batch out at a time; the planner neither edits nor
  runs cargo in that checkout while a batch is out.
- Every deletion shows its grep in the report; a guide, `training.md` line or pitfall row
  describing a changed thing is a defect in the batch that changed it.
- **A listen check is a gate, not a courtesy.** K1, K3, K7, K4, K5 and every K6 batch end
  with Jesse (or M) listening natively and in the browser before the batch is marked done;
  the report names what to run and what should be heard.
- Anything deferred is filed with `/file-issue` before the effort reports done.

## Batch 0 — planner only: the effort directory, the gates' plan, the plan review — DONE 2026-10-20 (116a8f0; three review rounds; the board's Sprint field was already set on all nine issues)

Files: `coordination/conductor/plan.md` (this file, moved to its tracked path),
`coordination/conductor/reviewer-comparison.md` (pattern-audit's header), `review/conductor/`.
Plan review rounds: kimi + gemini + codex on one snapshot per round until Jesse calls it
settled; decisions recorded here, rebuttals in `review/conductor/rebuttal-N.md`. Then the
Studio Board: the nine issues are already on milestone 2; set the board's Sprint field.

## Batch 1 — K1 spike: prove the architecture, choose the assets (#60) — DONE 2026-10-20 (62686ae)

Authored by the executor (a Claude Opus session) from `review/conductor/handoff-1.md`; report
`review/conductor/report-1.md`; reviewed by kimi (`review-4.md`), gemini (`review-4-gemini.md`,
recovered from agy's kept output) and the planner (`review-4-claude.md`); adjudicated in
`rebuttal-4.md` (10 accepted, 3 rebutted); Jesse's listen check passed natively and in both
browsers. **The architecture holds.** What the measurements changed, each carried into the
section that acts on it: 15 lanes per slot (rustysynth's channel 9 is percussion); the web
backend pulls 2048-frame buffers, so K3's timestamps are mandatory; the three fixtures have
no tempo or time-signature change, so batch 2 adds *Egmont*; the 2 MB SoundFont target is
waived (Jesse, 2026-10-20) and a per-game audio budget replaces it; the spike's known limits
(`coordination/K1_SPIKE.md` § Known limits) are rules in batches 3–5. Pins for batch 2:
rustysynth 1.3.8, midly 0.5.3. The provenance gate is live; wiring the hand-run gates into CI
is filed as insiculous_2d#176.

Executor: the Opus session. Reviewers: kimi, gemini (a harness and fixtures). Listen check: Jesse.

Files (new): `coordination/conductor/spike/` — a standalone cargo project scaffolded from
`games/game-template` (its own `[workspace]` with `[profile.wasm-release]`, a cdylib lib,
`src/web_entry.rs` with `ASSET_BASE`, a binary that answers `--achievements-manifest` with an
empty manifest — the game contract `scripts/build_wasm.sh` enforces at :176, :228 and :325);
depends on `../../../crates/engine_core` by path for the game loop and the gesture gate, and
on `rodio 0.20`, `rustysynth 1.3`, `midly 0.5` directly, its `Cargo.lock` committed and the
resolved versions named in `K1_SPIKE.md` (batch 2's workspace pins must equal them);
`coordination/K1_SPIKE.md`; `crates/music_midi/fixtures/` (the Mutopia files below, verbatim,
plus `sine.sf2` and `sine_{48000,22050}_{mono,stereo}.wav`) with its `LICENSE`;
`scripts/make_sine_soundfont.py` and `scripts/make_sine_wav.py` (stdlib only; outputs
byte-reproducible); `examples/assets/sounds/LICENSE` (the two engine-authored WAVs there,
so the provenance gate is green over the whole tree from day one, and its test carries a
negative fixture — a stray file with no LICENSE fails it); `scripts/subset_soundfont.py` (reads an SF2, keeps the named presets
and their instruments and samples, writes a valid SF2; Polyphone by hand is the fallback if
the script is not done in a day, recorded as such); `scripts/check_audio_provenance.sh`;
`assets/soundfonts/` is not created yet — the chosen subset is reported, not shipped, until K2.
One engine change: `AudioManager::play_source<S: rodio::Source<Item = f32> + Send + 'static>(source, bus)`
in `manager/mod.rs` (a public wrapper over `start_sink`, ~20 lines, kept — K4 uses it), and
`AudioManager::is_enabled()` if absent.

**Target shape.** The spike answers K1's six criteria in order, each with a test or a
measurement in `K1_SPIKE.md`:
1. rustysynth `SoundFont::new(&mut Cursor)` from bytes; `Synthesizer: Send` asserted by a
   compile-time `fn assert_send<T: Send>()`; a `SpikeSource` implementing `rodio::Source`
   that drains a `SyncSender` of `(channel, key, velocity, on/off)` per 64-frame block.
2. One phrase (Bach Invention 2's first two bars, read from the fixture with midly) played
   alternately and concurrently through two slots (two synthesizers, two presets); a
   program change on slot A and a same-pitch note-off on slot A leave slot B's voice
   sounding (asserted on rendered RMS per slot, and heard). Capacity: pool 128, sounding cap
   24 per slot; the stress test — a 200-note burst over long-release presets (strings,
   piano), one lane hammering one key, the worst-case storm of criterion 4 — measures the
   maximum sample-to-sample delta on the rendered output and the report proposes it as the
   threshold K3's test asserts; no sounding note may be cut (asserted by the ledger against
   per-note RMS). The layering factor per candidate preset and the per-region release maxima
   read from the bank go in the report. A same-pitch retrigger on one lane keeps the second
   attack (heard). The storm has a one-shot-only variant on a sample slot.
3. A WAV decoded with rodio's `Decoder` into `OwnedPcm`, pitched by linear interpolation
   from root 60 to 67 and 48, mixed with slot output; duration ratio measured and stated;
   one-shot vs gated note-off demonstrated.
4. Render cost per 64-frame block at 44.1 kHz on Iroh (`cargo run --release`), peak voices,
   the same at 48 kHz output; the browser run via `scripts/build_wasm.sh coordination/conductor/spike
   conductor-spike --version v1` served locally: gesture-gated start, no underruns in a
   two-minute run at **decision 2's full worst case** (background score + 8 slots at the
   ledger budget + sample voices, 16 triggers/s, long-release presets), frame time with and
   without the source, in Firefox and Chromium, with one seek and one loop wrap inside the
   capture. Also measured once per target: the
   scheduled-versus-rendered offset (the beat-visual latency the deferred issue will carry)
   and the load + decode time of the shipped-size SF2 plus a worst-case sample set (if it
   exceeds a frame, batch 6 prewarms). Asset sizes and loaded memory recorded, and a
   **per-game audio budget** (SF2 + songs + samples per bundle) proposed for K2 to freeze —
   #127's budget (closed) is the *playground* bundle's, `PLAYGROUND_SIZE_BUDGET_MIB` in
   `build_wasm.sh`, and K8 is measured against that one because the composer grows it.
5. The SoundFont: the subsetter applied to FluidR3 GM for the presets the casting needs
   (piano, harpsichord, strings, trumpet, trombone, xylophone, glockenspiel, timpani, pizzicato,
   oboe, tuba, snare — the final list is the report's) measured at or under 2 MB; over
   2 MB escalates — the MuseScore General SF2 subset is tried, and if that is over too the
   batch stops INCOMPLETE for Jesse: the report never drops a cast preset on its own.
   Provenance recorded in the `LICENSE`. The Mutopia
   fixtures: Bach Invention 2 (id 58), Tchaikovsky *Napolitan Dance* arr. Sardain (id 896,
   trumpet + percussion + piano), Minkus *La Bayadère* variation (id 895), each page's
   "Copyright: Public Domain" line and URL recorded; the report states each file's SMF type,
   **division type (SMPTE is rejected by K2)**, track count, tempo and time-signature events,
   names an alternate Public-Domain piece per required shape so a rejection is a swap and
   not a replan, and generates a type-0 fixture from the type-1 one with a committed script
   if none is type 0.
6. Any failed criterion stops the batch: INCOMPLETE with the measurement, and the plan is
   revised with Jesse before batch 2.

**Gates:** the spike's own `cargo test` and `cargo clippy`; the engine's full list (the one
engine hunk touches `crates/audio`, so `check_wasm.sh`); `check_audio_provenance.sh` green
over `crates/music_midi/fixtures`; Jesse's listen check native and browser.

**Leaves out:** any production code in the workspace beyond `play_source`; the sidecar schema
(K2); the composer. The spike project stays committed as the reproducible measurement.

## Batch 2 — K2a `crates/music_midi`: the song schema, owned MIDI data, the tempo map (#61) — DONE 2026-10-20 (837aff6)

Authored by the executor (a Claude Opus session) from `review/conductor/handoff-2.md`; report
`review/conductor/report-2.md`; reviewed by kimi (`review-5.md`), gemini (`review-5-gemini.md`)
and the planner (`review-5-claude.md`); adjudicated in `rebuttal-5.md` (7 accepted, 3 rebutted).
Landed as specified, two things differently and rightly: the "background channels against
the capacities" check is absent because a four-bit MIDI channel always fits the background
synthesizer's sixteen, and four of five fixtures round-trip event-identically rather than
byte-identically because midly's writer always uses running status (Bach, which never repeats
a status byte, is byte-identical). Egmont's tempi run **84 → 168 → 152** (the first report had
the order wrong). Carried to batch 3's fix-up list: the background `soundfont` path checked
against the loaded resources; a tick cap at edit time; `pub use rustysynth` behind `synth`.

Executor: the Opus session. Reviewers: kimi, gemini. Size: about +2200 lines.

Files (new): `crates/music_midi/{Cargo.toml, CLAUDE.md}`, `src/{lib.rs, error.rs,
song_file.rs, song_file/validate.rs, song_data.rs, song_data/flatten.rs, tempo_map.rs}`,
`src/song_file/tests.rs`, `src/song_data/tests.rs`, `src/tempo_map/tests.rs`,
`docs/SONG_FORMAT.md`; root `Cargo.toml` member + `[workspace.dependencies]` for `midly` and
`rustysynth` (rustysynth is wired in batch 3 but pinned here so one commit owns both pins).

**Target shape.**
- `SongFile` (RON, `version: 1`) with three named collections — `parts: [Part { name, track:
  TrackSelector::Name(String) | Index(usize), mode: PartMode::{Loop, Sequence,
  AtSongPosition}, quantize: Quantize::{Off, Beat, Eighth, Sixteenth} }]`, `instruments:
  [Instrument { name, source: InstrumentSource::SoundFont { path, bank, preset } | Sample
  { path, root_note, gain, playback: OneShot | Gated } }]`, `bindings: [Binding { action,
  part, instrument }]` — plus `soundfont: path` for the background score, `looping: bool`,
  and `resources: ResourceMetadata { preset_layering: {...}, sample_seconds: {...} }`
  supplied by the loader so `validate` can check capacities without I/O. Validation
  (`validate.rs`): unique names and action ids, every binding reference resolves, duplicate
  track names need the index escape hatch, non-finite gain and out-of-range notes rejected,
  trigger parts non-empty, instrument count ≤ the slot capacity, background channels and
  instruments against decision 2's capacities, unknown version rejected. Errors are an
  enum naming the collection and the entry.
- `SongData::from_smf(bytes)` flattens midly's zero-copy types to owned events per track with
  absolute ticks; SMF 0 is normalized into per-channel selectable parts with global tempo
  and meta events kept; SMF 2 rejected; SMPTE division rejected; defaults 120 BPM and 4/4
  when absent; `units: Vec<MusicalUnit { tick, notes: Vec<Note { key, velocity, duration_ticks }> }>`
  per part (simultaneous note-ons grouped); every note carries a stable `NoteId` (never a
  vector index — the composer's selection and undo address notes by it); the original
  event list is retained for round-trip export, and `from_smf` returns an
  `UnsupportedReport` naming any event it cannot carry opaquely, so the composer can refuse
  editing that file with the reason instead of dropping controls or tempo metadata.
  Mutation helpers (`add_note`, `remove_notes`, `move_notes`, `resize_notes`,
  `set_velocity`, `set_pitch`, `assign_part`, tempo and time-signature setters) return the
  affected notes' before-images so an inverse edit is computed, not guessed; a `write_smf`
  re-emits the retained events *excluding* every note pair whose `NoteId` was edited or
  deleted, plus the live notes, through midly's writer — an unedited file round-trips
  byte-identically and export → reimport → diff is a test. `UnsupportedReport` also lists
  same-key overlaps per channel as informational (the composer shows them, nothing refuses).
  The crate has a `synth` feature (default on) that owns rustysynth; the schema and data
  model compile without it, which is how `crates/editor` depends on it.
- `TempoMap`: tick ↔ seconds ↔ samples ↔ (bar, beat, beat_in_bar) across tempo and
  time-signature changes, integer boundary counting for beat crossings.
- **A fourth fixture first:** none of batch 1's three has a tempo or time-signature change.
  Download Beethoven's *Overture to Egmont* (Mutopia id 547, its page "Copyright: Public
  Domain", 15 tracks, 84 → 168 → 152 bpm, 3/2 → 3/4 → 4/4) verbatim into
  `crates/music_midi/fixtures/` with the same `LICENSE` entry shape (page line, URLs, sha256)
  before the tempo-map tests are written; the provenance gate must stay green.
- Pins: `rustysynth = "=1.3.8"`, `midly = "=0.5.3"` — the spike's resolved versions.
- Tests: the fixtures from batch 1 plus *Egmont* (the type-1 Tchaikovsky, the type-0 generated
  one, the Bach invention, the overture), two bindings over one part without note
  duplication, tempo and time-signature changes, the reject list, schema round trip. Docs: the grammar with
  examples in `SONG_FORMAT.md`; the crate guide with its pitfall table (`check_guide_tests.sh`
  reads it, so every cited test must exist); a `crates/music_midi/` entry in `training.md`
  § Directory Map and a row in `CLAUDE.md`'s crate → agent table, and `training.md`'s audio
  line (`manager.rs`) brought current — a new crate nobody's map names is a defect.

**Gates:** the standard list; `check_wasm.sh` (new crate in the workspace). **Leaves out:**
the synthesizer (batch 3), triggering (batch 4), samples (batch 5), any I/O.

## Batch 3 — K2b `crates/music_midi`: the sequencer, the conductor, background playback (#61)

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse (through the spike's
`play_source`, a one-screen example in `examples/`); the report pastes the native run's
`conductor=on` log lines — drain gap and frame times — which K1's listen run did not keep.
Size: about +2000 lines.

Files (new): `src/{sequencer.rs, conductor.rs, conductor/slots.rs, conductor/ledger.rs,
commands.rs, position.rs}` and their `tests.rs`; `examples/conductor_background.rs`
(plays a fixture through `AudioManager::play_source` — the only consumer until K4; it takes
the SoundFont path as its first argument, defaulting to the spike's
`coordination/conductor/spike/assets/soundfonts/cast.sf2` when that exists and to the sine
fixture otherwise, and reads its assets by `with_asset_base_path` like `behavior_demo.rs`).
Two one-line enablers outside the crate: the root `Cargo.toml` `[dependencies]` gains
`music_midi = { path = "crates/music_midi" }` (an internal crate, like `physics` there), and
`crates/engine_core/src/prelude.rs` re-exports `audio::Bus` beside `AudioManager`, so an
example or a game names the bus without depending on `audio` itself.

**Batch 2's fix-ups, first** (accepted in `rebuttal-5.md`, behaviour so not patched there):
`ResourceMetadata` gains `soundfonts: BTreeSet<String>` (the paths the loader decoded) and
`check_against` refuses a `SongFile.soundfont` not in it with its own `SongFileProblem`
variant and test; `add_note` and `move_notes` refuse a tick above `u28::MAX` (the largest
MIDI delta) so every exported gap fits, the error naming the note; `#[cfg(feature =
"synth")] pub use rustysynth;` in `lib.rs` so no other crate names it (decision 7). Read
retained events through `RetainedEvent::kind()`, never the stored field — SysEx and text
payloads live beside it.

**Target shape.** `Sequencer` (pure): given `SongData` + `TempoMap`, yields the background
events whose tick falls inside `[block_start_sample, block_end_sample)`, wraps on loop
(all-notes-off at the seam, cursors reset); `Conductor::new(ConductorConfig { sample_rate,
block_size: 64, capacities })` — rejecting a zero sounding cap and a pool below `cap ×
layering` at construction — owns the background `Synthesizer` and `InstrumentSlots`
(decision 2) and `render(&mut self, left: &mut [f32], right: &mut [f32], buses: BusGains)`;
`ConductorCommand` enum (`Play, Stop, Pause, Resume, Seek(tick)`; triggers come in batch 4);
`Conductor::load(Arc<Song>)` takes an in-memory song (the composer previews unsaved
documents; the file path is the audio crate's concern);
`SharedPosition` published once per block; `VoiceLedger` bounding sounding notes (decision
2); a block that crosses the loop seam is rendered in two sub-blocks so the loop is
sample-exact; the sequencer coalesces same-key overlaps per channel — it counts sounding
instances per (channel, key) and emits the note-off when the last ends, so a piano
reduction's pedalled repeats sustain (rustysynth's `note_off` would otherwise release both),
tested on the Tchaikovsky fixture; `Seek` and loop wrap reconstruct channel state (decision 3) — a
`ChannelStateFold` over the score's program, bank, controller and pitch-bend events (state
events fold `<=` the target tick, note events `<`, so a seek to 0 keeps tick-0 tempo and
programs), tested
by comparing seek-to-tick against linear playback on a fixture with state changes, the fold
cached at loop points if batch 1's capture shows its cost; K2's load-time check of the
background score's peak polyphony against the pool. The
non-silent-render test with `sine.sf2`; the loop-wrap test asserting that the seam releases
every pre-seam note (tails only, no sustained energy at their pitches after the release) and
that the downbeat attacks on the exact sample the loop's first tick maps to; `Send` asserted
for `Conductor`. **Gates:** standard; `check_wasm.sh`.
**Leaves out:** triggers, sources, samples, rodio.

## Batch 4 — K3: shared-part triggering, musical timing, bounded voice ownership (#62)

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse (the example gains a
keyboard trigger). Size: about +2600 lines.

Files: `src/{triggers.rs, triggers/cursor.rs, triggers/quantize.rs, sources.rs,
overflow.rs}` + tests; `commands.rs` grows `Trigger { binding, source: SourceId }`
(activation is lazy — the first trigger activates), `DeactivateSource`, `StopAllSources`,
`PauseMusic`, `ResumeMusic`; `conductor.rs` dispatches.

**Target shape.** `SourceId { id: u64, generation: u64 }` (mirrors `EntityId`, built by the
host); per-source lane ownership in a slot; one shared cursor per part for `Loop`
(advances and wraps) and `Sequence` (skips passed units, exhausts, resets on song loop);
`AtSongPosition` reads the unit active at the (quantized) target tick without a cursor;
quantization resolves at the target tick through the tempo map with the one-block late
tolerance measured from the trigger's stamped action time (a trigger stamped within one block
after a boundary plays at once; one stamped later schedules to the next boundary; one older
than the tolerance at drain is dropped and counted); overflow = reject newest trigger without consuming the cursor, counted on
`SharedPosition.rejected_triggers`; one channel with the lifecycle reserve by in-flight
accounting (decision 3), so saturation by triggers cannot drop a stop or a deactivation and
a trigger enqueued before a deactivation is always drained before it (asserted under a
10 000-trigger storm, a lifecycle storm, and a consumer that makes no progress for ten
blocks); activation epochs on `SourceId`; lanes stealing oldest-source-first when none is
free (15 one-note sources plus a 16th, the 16th sounds); the synth-side discontinuity threshold from
batch 1 asserted under the storm, with no sounding note cut; the single-lane single-key
burst; a song change under a full channel where the new song's first triggers survive the
flush; 200 triggers queued and a consumer resuming two seconds later where none sounds; deactivation fades the source's voices (release for synth,
5 ms for one-shots) and marks the generation dead; loop wrap fades everything and drops
cross-wrap pending events; explicit pause rejects triggers and clears pending work. The
event-trace tests the issue lists, each named for its behaviour, plus the synth-backed
isolation tests from batch 1 made permanent. `docs/SONG_FORMAT.md` § Modes shows the
Loop-versus-background-chord caveat the composer will surface. **Gates:** standard;
`check_wasm.sh`. **Leaves out:** samples (next), the audio crate (batch 6).

## Batch 5 — K7: user sample instruments (#140)

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse. Size: about +1600 lines.

Files: `crates/audio/src/sample_loader.rs` (+ tests; `load_sample_pcm(path) -> AudioResult<OwnedPcm>`
through `common::vfs` and the rodio `Decoder`, WAV only accepted as a *pitched* import, the
limits of decision 5, typed errors naming the path); `crates/music_midi/src/{pcm.rs,
sample_voice.rs, sample_bank.rs}` + tests (`OwnedPcm` lives in `music_midi` so audio depends
on it, not the reverse); `conductor/slots.rs` learns a `SampleSlot` (no lanes: a direct voice list with its own
live-voice cap and oldest-voice steal through the fade, **and a hard ceiling of the live cap
plus a fade budget that cuts the oldest fading voice**, so a burst inside one block cannot
pile up voices; a one-shot is sounding until its pitch-scaled end); a sample shorter than
1 ms is rejected before allocation; fixtures from batch 1.
Tests: known-frequency pitch at 48 kHz and 22.05 kHz sources rendered at 44.1 kHz (zero
crossings counted), duration behaviour, stereo/mono, gated vs one-shot, two concurrent
voices of one sample with independent note-off, oversized and non-finite input rejected
without touching the loaded song, combined sample + SoundFont transport, output finite
and click-free on steal (max sample-to-sample delta bounded). **Gates:** standard;
`check_wasm.sh`. **Leaves out:** time stretch, multisample maps, the picker (K8).

## Batch 6 — K4a `crates/audio`: the song manager and the conductor source (#63)

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse. Size: about +2000 lines.

Files: `crates/audio/src/manager/song.rs` (`play_song(path)` and `play_song_in_memory(Arc<Song>)`
— the composer's preview — `stop_song`, `pause_song`, `resume_song`, `seek_song(beat)`,
`is_song_playing`, `song_position() -> Option<SongPosition { bar, beat, beat_in_bar,
seconds, latency_samples }>`, `beat_crossed(Subdivision)`, `trigger(binding: &str, source:
SourceId)`, `activate_source`, `deactivate_source`), `manager/song/load.rs`
(the dependency closure through `common::vfs`: `.mid`, `.mid.ron`, every SoundFont and
sample, validated in full — disabled mode included, byte limits before parsing — before
the current playback is replaced; decoded *dependency files* (SF2, WAV) are cached by each
file's own fingerprint — never per song, so a composer edit creates no entry and the preview's
in-memory song shares them — capped at the current song's closure plus the previous one, so a
changed file at the same path is a different key and a second `play_song` of unchanged files
does not decode again; a game may prewarm at `init`), `conductor_source.rs` (the rodio `Source`, decision 3), `manager/song/tests.rs`;
`manager/music.rs` learns the shared slot (decision 4); `lib.rs` re-exports the public
`music_midi` types; `crates/audio/CLAUDE.md` rewritten (the "no spatial, no component" line
goes in batch 8). Tests: pending/start/last-wins across both kinds, volume per bus (music 0
keeps triggers audible, sfx 0 mutes them, the clock runs at both), beat boundaries by
integer counting, the disabled-mode virtual clock advancing `beat_crossed` under `update()`,
one crossing at most per `update()` across a many-beat gap, pending transport commands
retried from `update()`, a song change flushing sources and the next trigger re-activating, dead receiver marks stopped with a rate-limited log, dependency failure
preserves the current song, lifecycle under saturation through the public API. **Gates:**
standard; `check_wasm.sh`; `check_games.sh` (public items of `audio` changed). **Leaves
out:** the host (next), the editor.

## Batch 7 — K4b `engine_core`: the host owns sources, scripts trigger bindings (#63)

Executor: the Opus session. Reviewers: kimi, gemini. Size: about +1800 lines.

Files: `crates/engine_core/src/sound_sources.rs` (+ tests; `SoundSources` resource:
`EntityId → SourceId` with generations, `reconcile(world, audio)` deactivating sources whose
entity no longer validates — run in the frame tail after despawns and before command
submission, and on world replacement/reset; a deactivation whose send fails stays pending
and is retried next frame; flushed on a song change; a `Menu` source and a **game-owned
`SourceId::game()`** independent of the world and never deactivated by reconciliation — a
sound that must outlive its entity, a brick's explosion, is triggered on it, which is the
game's arrangement choice under K3's "fade owned voices" rule),
`frame_tail.rs` hook, `scripting/commands.rs` (`ScriptCommand::TriggerBinding { target,
binding }`), `rhai_bindings.rs` (`cmd.trigger(me, "hit")`), `runner.rs` (same-frame **trigger** commands
collected and sorted by `(entity id, enqueue order)` before submission — every other command
keeps today's instance order, and `docs/SCRIPTING.md` says so — tested on the real path),
`contexts.rs` (nothing new: `ctx.audio` already carries the manager), `docs/SCRIPTING.md`,
`training.md` § Conductor (first cut; K5 completes it). Integration tests in
`crates/engine_core/tests/`: script-trigger then `cmd.despawn` with no second cleanup call,
direct Rust `destroy_entity`, world reset, stable same-frame ordering, Music/SFX gains
independent, recorded SFX unchanged beside a song, `play_music` → `play_song` with live
entities still triggering, a rejected deactivation followed by re-entry before the retry. **Gates:** standard; `check_wasm.sh`;
`check_games.sh --test`. **Leaves out:** camera eligibility (a game policy, K6), K9.

## Batch 8 — K9: scene-authored audio — `SoundEmitter`, the sound tier, the asset kind (#174)

Executor: the Opus session. Reviewers: kimi, gemini, codex (the popup and the inspector rows).
Size: about +1900 lines.

Files: `crates/ecs/src/sound_emitter.rs` (the component, decision 6, registered as a builtin),
`crates/engine_core/src/sound_library.rs` (path → handle, loads through `ctx.audio` lazily,
cleared on scene load), `sound_emitter_system.rs` (frame tail: play-on-spawn once, looping
kept alive, spatial attenuation and pan from the main camera, `max_distance` floored at
0.01 world units by validation, an `EntityId → playing instance` table so an emitter whose
entity no longer validates is stopped next frame and on scene load, binding-named emitters go
through `trigger`), `scene_data.rs` (`ComponentData::SoundEmitter`, typed because its path
resolves at load like `Sprite.texture`), `scene_serializer/components.rs`,
`scripting` (`cmd.play_sound(target)`, `cmd.stop_sound(target)`), `crates/editor/src/asset_browser.rs`
(`AssetKind::Sound` for wav/mp3/ogg/mid/sf2, `DragPayload::Sound { path }`),
`stored_component/mod.rs` (one registry line, `ComponentCategory::Audio` back),
`component_editors/sound_emitter.rs`, `panel_renderer/asset_browser.rs` is at 598 lines —
the drop target for a sound path is written in a new `asset_browser/drop.rs`, `crates/audio`
gains a pan field on `SoundSettings` (applied as a stereo gain pair; no `SpatialSink`). Docs: `crates/audio/CLAUDE.md`'s
first paragraph, `training.md`, the scene RON example. Tests: a scene names a sound by path
and the harness hears a play call (`GameHarness`), a missing file is a load warning not a
panic, the emitter on a despawned entity stops, serializer round trip, popup row present,
attenuation and pan in world units at `max_distance` and at half of it, `spatial` on a
binding emitter refused at load, two binding emitters on one slot where only one stops, a
looping emitter's entity destroyed and its sink gone next frame.
**Gates:** standard; `check_wasm.sh`; `check_games.sh`. **Leaves out:** occlusion, a
listener component (the main camera is the listener), the composer.

## Batch 9 — K8a: the song document, the command API, persistence (#141)

Executor: the Opus session. Reviewers: kimi, gemini (the API seam). Headless; no screen.
Size: about +2800 lines.

Files (new): `crates/editor/src/song/{mod.rs, edit.rs, history.rs, io.rs, selection.rs}` and
their tests (each ≤ 400 lines), `crates/editor/src/song/parity_tests.rs`,
`crates/editor/src/command_api/song.rs` (+ rows in `specs.rs`, an arm in `parse.rs`),
`crates/editor_integration/src/editor_game/song_io.rs` + `song_io_tests.rs`,
`docs/EDITOR_COMMAND_API.md` § Songs. Touched: `editor_game/mod.rs` (the `song:
Option<SongDocument>` field; `sync_dirty_mirror` :421-436 becomes scene-dirty OR song-dirty
into `history_dirty_flag`, so `playground_is_dirty` aggregates the song with no bridge
change), `editor_game/scene_confirm.rs` (`PendingSceneAction::{NewSong, OpenSong, ImportSong,
CloseSong, StartPlay}`, `request_song_replace`, `ConfirmDialog::unsaved_song`), `api.rs`
(routing), `editor/src/context/mod.rs` (a separate `song_dirty` mirror; `title_bar_text`
shows both marks), `crates/editor/Cargo.toml` (+ `music_midi` by path with `default-features = false` — the
schema and data model, no synth; an internal crate, within decision 7). The report measures
the playground bundle delta against `PLAYGROUND_SIZE_BUDGET_MIB`.

**Target shape.** Decision 12's `SongDocument { file: SongFile, data: SongData, history:
SongHistory, path: Option<PathBuf>, imported_from: Option<PathBuf>, editing: EditingPolicy
{ Full | Refused { reason } } }`; `SongEdit` enum (AddNote, DeleteNotes, MoveNotes,
ResizeNotes, SetVelocity, SetPitch, AssignPart, SetTempo, SetTimeSignature, SetLoop,
SetQuantize, SetPartMode, Add/Set/RemoveInstrument, Set/RemoveBinding, Add/RemovePart) with
`try_merge` (same gesture: deltas sum, absolutes replace); `apply` validates (pitch 0..=127,
velocity 1..=127, ticks ≥ 0, part exists, `Refused` rejects), calls the `music_midi`
helpers, stores the inverse. Save (`io.rs`): copy new samples under `assets/music/samples/`
(under `assets/music/samples/<song stem>/`, or `samples/_unsaved/` for a song never saved,
remapped by Save or Save As; a same-named file with different bytes gets a content-hash
suffix and the instrument path follows; each sample instrument's required `provenance {
author, license }` is written into the directory's `LICENSE` in the same journal, and a
missing one refuses the save), write `<stem>.mid.ron.new` (it carries `midi_fingerprint`)
then `<stem>.mid.new`, write the finals, remove the `.new` pair; open first purges any
leftover `.new` that matches the finals' fingerprint, recovers the pair only when both are
present and the MIDI matches the fingerprint, otherwise the finals stand and the journal is
dropped with a reported warning (a `.mid.ron.new` without its partner included), and an
unmatched lone `.mid.new` is offered as an import; `mark_saved` only after the last step (on
the web, after both finals are acknowledged — batch 11's transaction). An imported `.mid` outside the two home
directories (`assets/music/`, `assets/ai/music/`) or without a sidecar opens with `path:
None` and its first Save is a Save As into one of them; the original is never written. API verbs: `song` (query), `song notes [part]`, `song edit <json>` (a serde
`SongEdit`), `song undo`/`song redo` (song-scoped), `song select`, hosted `song new|open|
save|close`, and `song play|stop|seek|trigger` (enqueue intents; batch 11 drains them).
Refused in Playing and Paused and while a dialog is pending; `song edit` inside an open
scene batch is refused. **Tests:** watermark/merge/break (mirror `commands/dirty_tests.rs`),
apply/inverse per variant, refusal matrix, tempdir round trip, interrupted save at each
step recovers or leaves the finals intact (a matching leftover is purged, an unmatched lone
`.mid.new` becomes an import, a lone `.mid.ron.new` is removed), imported original
untouched, a sample without provenance refuses the save and one with it lands in the
`LICENSE`, an unsaved song's staged sample is remapped by Save As, a save under
`assets/ai/music/ai_x.mid` reopens and saves in place, scene save leaves the song dirty and
vice versa, New/Open Song and Play park behind the prompt, the parity table (one row per
`SongEdit` variant, GUI path versus API path, exhaustive match so a new variant without a
row fails to compile; and an edit on a document whose `UnsupportedReport` is not editable
is refused by both paths with the report's reason — the gate gemini's batch 2 review asked
for lives here, not in the data model), headless `run_headless_editor_api` answering `song`
lines.
**Gates:** standard; `check_wasm.sh` (editor crates are in the wasm workspace).
**Leaves out:** any pixel (batch 10), audio (batch 11), multi-song tabs, scripting access.

## Batch 10 — K8b: the piano roll, the note list, the sidebar (#141)

Executor: the Opus session. Reviewers: kimi, gemini, codex (every screen). Size: about
+3000 lines.

Files (new): `crates/editor/src/composer/{mod.rs, layout.rs, keys.rs, piano_roll.rs,
note_list.rs, sidebar.rs, transport.rs}` + tests, `crates/editor_integration/src/panel_renderer/
composer.rs` + `composer_drag.rs`, theme tokens `note_fill`, `note_selected_border`,
`key_black`, `playhead` with contrast guard tests. Touched: `editor/src/dock/mod.rs`
(`PanelId::COMPOSER = PanelId(5)`, CONSOLE left alone), `context/mod.rs` (Bottom, hidden by
default, size 260, min 200; `ComposerState` beside `asset_browser`), `menu/mod.rs` +
`menu/actions.rs` (View → Composer; File → New/Open/Save/Save Song As…),
`editor_input.rs` (`EditorAction::{NewSong, OpenSong, SaveSong, SaveSongAs, FocusComposer,
SongPlayStop}`, all but `FocusComposer` denied while Playing), `editor_game/shortcuts.rs`
(`KeyRoute::Composer` after the keyboard-owned gate, Escape first in the cancel cascade,
key-release seals a nudge), `editor_game/mod.rs:265` (dispatch branch for the composer
panel).

**Target shape.** `layout::compute(bounds, state) -> ComposerLayout { transport, sidebar,
note_list, roll: PianoRollGeometry, mode: Full | Compact (< 720 px: sidebar as tabs) | Tabs
(< 480 px: one area at a time) }`; `PianoRollGeometry` with `tick_to_x`/`x_to_tick`/
`pitch_to_y`/`y_to_pitch`, `note_rect`, `beat_lines(TempoMap, TimeSignature)` at the
`grid_segments` LOD; two `ScrollState`s. Focus model: `ComposerFocus { None, Transport,
PianoRoll, NoteList, Sidebar }`, entered by click or Ctrl+Shift+M; Tab/Shift+Tab cycle areas
(ui's traversal only runs for a focused field, so no clash); arrows move cursor/row,
Shift+arrows nudge selected notes (merging per hold), Enter/Insert add, Delete delete, `[`/`]`
resize, `+`/`-` velocity, Space preview, Escape clears focus. While the composer holds
focus the single-key editor toggles are unreachable — intentional and documented. Visible
focus: a 2 px `focus_ring` on the focused area and on the focused note or row; selection is
never colour-only (thicker border plus a glyph; rows get the accent fill plus a leading
"▸"). The sidebar shows parts (with the "unassigned background" badge and the bindings
using each), instruments (root/gain/one-shot fields; the duration-changes-with-pitch note
beside the root field), bindings (name → part + instrument, a Trigger button each), and the
mode caveat text for `Loop`. The drop site splits by extension: `.wav` imports a sample,
`.mid` opens a song, `.sf2` adds a SoundFont instrument, `.mp3`/`.ogg` are refused with a
tooltip naming WAV as the pitched-sample format. A drag records one history entry (every drag frame applies
with the gesture id; release calls `break_merge`); Escape mid-drag restores. **Tests:**
geometry round trips and LOD, mode thresholds, the key map table including Tab cycle and
Escape, a 30-frame drag yields one entry, selection marker present in the draw list (as
`menu/tests.rs` does), tokens pass contrast, composer keys never reach the game while
focused and do once unfocused. **Gates:** standard; `check_wasm.sh`; native screenshots
for codex. **Leaves out:** audio (next), CC and pitch-bend lanes (preserved opaquely only).

## Batch 11 — K8c: transport, Play interplay, the web bridge, the accessibility pass (#141)

Executor: the Opus session. Reviewers: kimi, gemini (the bridge seam), codex (the transport
and the keyboard walk). Listen check and keyboard walk: Jesse, native and browser. Size:
about +2500 lines, plus a small `insiculous_web` half reviewed from
`insiculous_web/review/conductor/`.

Files (new): `editor_integration/src/editor_game/song_preview.rs` + tests (`SongPreview {
state, intents: Vec<SongAudioIntent>, generation }`, `SongAudioIntent::{LoadAndPlay { song:
Arc<Song>, from_beat }, Stop, Seek, Trigger { binding }, PauseGameplay, ResumeGameplay,
StopAll}`; `song_request` pure, `apply_song_intents(audio)` drained in `finish_frame`),
`crates/playground/src/bridge.rs` `playground_write_file_bytes(path, bytes)` (same
`validate_bridge_path` guard, an allowlist of `assets/music/` and `assets/ai/music/`,
persisted by the vfs observer), the site's second file input beside the zip import
(`insiculous_web/src/scripts/playground-embed.ts`). Touched: `play_session.rs`
(Play is a two-frame transition: the request frame loads and validates the saved song's
dependency closure and drains the preview's stop in `finish_frame`; the session — snapshot,
`init` — starts the next frame, so a game's `play_song` in `init` is never clobbered; a
failure keeps the preview and shows the error;
pause → `PauseGameplay`; resume → `ResumeGameplay`; `stop_with_paused_edits` → `StopAll`
after the restore), `editor_game/mod.rs:380-383` (idle throttle off while previewing; the
playhead redraws from `audio.song_position()`), `snapshot.rs` (`SceneSnapshot.live_files`
carrying the unsaved song's MIDI bytes and sidecar text), `archive.rs` (export splices every
live file and drops `*.new`; import validates `.mid.ron` by parse and `.mid` by header, a
sidecar without its `.mid` is `InvalidSong`; the export carries the live, unsaved song as
it carries the live scene, stated in the README the archive writes), `docs/WEB_PLAYGROUND.md`
§ Acceptance (the composer rows) and § Export (the live-state sentence). `live_files` also
carries any unsaved sample's bytes at its archive path under the song's sample directory, so
an export before the first save is a complete closure. On the web a `SongSaveTransaction`
(polled in `finish_frame`) removes the `.new` pair and fires `mark_saved` only once the
persistence chain reports both finals `Idle`, with a timeout and an error terminal (report,
finals-or-journal consistent, dirty with the reason) and save-while-saving refused on the
status bar; the saved indicator rides `save_status`. Accessibility: every control labelled, units in text ("120 BPM", "4/4",
"1/16"), the focus ring in every area, keyboard transport; blockers filed as issues, never
claimed. **Tests:** intent sequences for Play/Pause/Resume/Stop and preview-before-Play,
throttle state while previewing, the snapshot carries song bytes and leaves history and
path alone, export replaces saved song bytes and drops `.new`, import rejects a bad
fingerprint, `playground_is_dirty` reads song-only dirtiness, the memory store rejecting a
put at each boundary of the transaction and a reload between boundaries recovers or reports
and never leaves a mismatched pair, preview → edit → save → Play hears the new bytes, export
before first save with an external sample imports into an empty project, a missing sample
after each Play-prompt option keeps the preview, the game's `init` song is playing after the
two-frame transition, a transaction that never acknowledges ends in its error terminal.
**Gates:** standard;
`check_wasm.sh`; the site's `npm run verify` for its half; the manual keyboard and listen
check, documented in the report. **Leaves out:** live mutation of a running score, MIDI
recording, a native file dialog, Exit prompting (filed as its own issue — it prompts for
nothing today, scene included).

## Batch 12 — K5: the Pong demo, the two-enemy example scene, the web proof (#64)

Executor: the Opus session (the song itself is sequenced in the composer by the executor and
lands under `assets/ai/music/ai_tong.mid` until Jesse's pass promotes it). Reviewers: kimi,
gemini, codex (the beat pulse and the editor screenshots). Listen check: Jesse, native and
browser.

Files: `games/pong/{assets/ai/music/ai_tong.mid, ai_tong.mid.ron, assets/soundfonts/
tong.sf2 + LICENSE, src/audio.rs, src/gameplay/scoring.rs, src/ui.rs (pause jam), src/
effects.rs (beat pulse)}`; `insiculous_2d/examples/conductor_demo.rs` +
`examples/assets/scenes/conductor_demo.scene.ron` (two enemy kinds — a SoundFont preset and
the imported sample — one shared melody, visibility eligibility, despawn and re-entry,
dense activity, pause input); `training.md` § Conductor completed; `PROJECT_ROADMAP.md`
row K. Pong: paddle hits and scoring are bindings over one part of the *Toreador Song*'s
refrain; the pause menu jams a `Sequence` part on the `menu` binding; a beat-driven pulse
on the grill flare; the recorded-SFX fallback path stays when no song is active. Web: the
exported project runs in the browser with requested-before-gesture startup. **Gates:**
standard; `check_wasm.sh`; `check_games.sh --test`; Pong's own tests and clippy; bundle
size against #127; the browser stress run of decision 2's worst case on the demo scene,
repeated from batch 1's method, before any K6 batch starts; screenshots of the authoring;
the listen check. **Leaves out:** the other five games.

## Batches 13–15 — K6: the six games, in pairs (#65)

Executor: the Opus session per pair, each game in its own repo with its own review
directory. Reviewers: kimi, gemini, codex (the beat visuals). Listen check: Jesse per game.

- **13 — Tong's full per-game acceptance + The Food Pyramid** (brick, paddle, loss and clear
  voices over the *Triumphal March*; the Maxwell finale takes the march's cadence).
- **14 — Bratdog + Toast Busters** (food, turn and death voices over the polka; shots, each
  Toasted kind on one shared part with its own instrument, UFO and hit voices over *Danse
  Macabre*).
- **15 — Meatieroids + Chicken Coop** (shots, three rock sizes as three instruments on one
  part, thrust and hyperspace over *Mars*; hops, death and home-slot voices over the
  *William Tell* finale).

Per game: the song sequenced in the composer under `assets/ai/music/ai_<slug>.mid` with its
sidecar, the SoundFont subset and `LICENSE` beside them; bindings through the host's
lifecycle (despawn, visibility exit by the game's camera policy, world reset — a sound that
must outlive its entity, a brick's or an alien's death, is triggered on `SourceId::game()`);
each bundle's audio against the per-game budget K2 froze; a tested
stable same-frame order; the `menu` binding; the beat visual; old beeps gated only where a
binding replaces them (no game has any today); the per-game checklist on #65 ticked only
after Jesse heard it. **Gates:** each game's `cargo test` and clippy, `check_games.sh
--test`, `build_wasm.sh` bundle size against #127,
`../../insiculous_2d/scripts/check_audio_provenance.sh .` run from the game (the same
relative-path precondition every game's `Cargo.toml` carries), the listen check. **Leaves out:** intensity layers, harmony generation (excluded by the milestone).

## Batch 16 — planner only: close-out

The milestone's nine issues closed by their commits; `PROJECT_ROADMAP.md` row K and
`CLAUDE.md`'s "K1 gates all of Conductor" line updated; `log_archive.md` lessons; the
follow-ups filed (output-latency compensation, cross-stop asset caching, cross-wrap trigger
re-aim, song-swap position save/restore, the human promotion of each `ai_` song to the
shipping path); `review/conductor/` archived; the scorecard's totals.

## Verification (end to end)

- Every batch: the gates above, run by the planner on the staged tree, never from the report.
- After batch 7: `cargo run --example conductor_background` natively and the spike page in
  the browser play the Bach fixture; a key triggers a binding and the trace log shows one
  unit consumed per press.
- After batch 11: in the native editor and at `/playground/`, create a four-bar melody,
  undo one note, bind two actions to it with two instruments (one a WAV), audition both,
  save, reload the page, reopen, export the zip, import it elsewhere — no loss.
- After batch 12: Tong on Iroh and in Firefox and Chromium — hits walk the refrain, the
  pause menu jams, the pulse lands on the beat, music at 0 keeps the hits, SFX at 0 keeps
  the song.
- After batch 15: each game's checklist on #65 ticked, with the browser bundle sizes in the
  scorecard and Jesse's listen note per game.
- Final: `grep -rn "AudioSource\|PlaySoundEffect\|AudioListener" crates src examples ../games
  --include=*.rs` shows only `scene_retired_components.rs`; `check_audio_provenance.sh`
  green over the seven repos; the working set's `scripts/check-skill-parity.sh` (at
  `/home/jedi/projects/insiculous/scripts/`) green.
