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
`scripts/subset_soundfont.py` does it byte-reproducibly. The shipping subset's preset list is
the casting's **plus every program the sequenced songs and the fixtures use** — the spike's
cast bank has no flute (0:73), so the cast fixture's flute plays as a piano on it.

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
   entity generation — one host-wide count that every activation of any source advances,
   so re-entry is a new epoch, queued work for an old epoch is dropped, and a flush can
   name a point in that count below which every activation is over.
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
   capacity`. A deactivation that fails admission stays pending in the manager
   (`pending_lifecycle`, batch 6) and is retried from `update()` before any trigger, so a
   stalled consumer never loses one and the host forgets the entry the moment it asks; order
   does not matter for it because it names an epoch and touches only that epoch's voices.
   Transport commands (pause, resume, stop, seek) that fail admission stay pending in the
   manager and are retried from `update()` before any trigger is admitted. World replacement
   flushes `SoundSources` and sends `StopAllSources { flush_epoch }` through
   the reserve — it kills only sources activated before that epoch, and while pending it is
   retried from `update()` before any trigger is admitted, like transport — so the next
   trigger from a still-valid entity re-activates it under a new epoch and survives the
   flush (kimi's round-2 and round-3 cases). *Amended after batch 6 (kimi's batch 7 plan
   review, F2):* a song change needs **no host flush** — each song has its own conductor
   and source table, the manager re-sends the standing floor to the new conductor, and a
   live entity keeps its `SourceId` and activates lazily in the new song on its next
   trigger. **Every `Trigger` carries the producer's
   `common::clock` instant**; the consumer drops a trigger older than the late tolerance and
   counts it (a suspended tab's backlog never replays as a burst), and quantization measures
   lateness from that stamp, not from the drain, so a frame of transport latency is not
   lateness. A lifecycle command is never dropped for age. **The stamp is not optional
   anywhere:** cpal's WebAudio backend pulls 2048-frame buffers from the main thread, so a
   command waits up to 53–61 ms before it is drained (K1 measured), and natively a command
   waits up to the device period, 42.6 ms (batch 3 measured); K3 places a trigger inside
   the burst from its stamp, or every trigger quantizes to the drain. **Placement is to a
   64-frame boundary** (1.45 ms): rustysynth applies a message only at the start of its next
   internal block, and nothing but `reset()` moves that grid — which is also why the
   **background plays on two synthesizers that take turns** (batch 3): at every seek, wrap or
   resume the playing one is released to its tails and the other is reset on the exact
   frame and given the folded state, so the downbeat lands on its sample, bit-exactly; the
   one edge is a tail still ringing on the idle synthesizer at the *next* reconstruction,
   which is cut. Every output sample passes a
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
   *Amended by batch 6:* **one conductor per song**, built on the game thread with its own
   source and sink (the audio thread never loads); **nothing is replaced until the new
   request has loaded**, recorded track or song (a failing load leaves what plays alone;
   H7's older rule for the recorded path is superseded); and each kind's stop clears its
   own playing and pending state — `stop_music` never touches a song, `stop_song` never a
   track — the guide carrying the stop-both line.
5. **Sample instruments decode in `crates/audio`, render in `music_midi`.** The WAV loader
   uses the rodio `Decoder` already in the crate and yields `OwnedPcm { sample_rate,
   channels, frames: Arc<[f32]> }`; `music_midi` never decodes. Limits (K1 proposes, K7
   freezes; K1 enforced them before allocation; the loader's byte limits are 48 MiB for a
   SoundFont and 256 KiB for a MIDI file, set by batch 6 after the 2 MB waiver): ≥ 1 ms and ≤ 10 s decoded, 8–96 kHz, mono
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
    the song's own flat folder in its home, `<home>/samples/<flat>/` (batch 11; an unsaved
    song stages nothing — its instruments name the imported files where they lie until the
    first save copies them); a
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

## Batch 3 — K2b `crates/music_midi`: the sequencer, the conductor, background playback (#61) — DONE 2026-10-20 (e5fef7a)

Authored by the executor (a Claude Opus session) from `review/conductor/handoff-3.md`; report
`review/conductor/report-3.md`; reviewed by kimi (`review-6.md`), gemini (`review-6-gemini.md`)
and the planner (`review-6-claude.md`); adjudicated in `rebuttal-6.md` (4 accepted, 6
rebutted); Jesse's listen check passed natively. **One design decision taken by the executor
and accepted:** the background plays on two synthesizers that take turns at every
reconstruction, because rustysynth applies a message only at its next internal block and
only `reset()` moves that grid — decision 3 now says so. Measured: the native drain gap is a
42.6 ms device period (so K3's timestamps are mandatory everywhere), ~9 µs a block, the
state fold under 41 µs on Egmont's 25 000 events, so it is not cached. Carried to batch 4's
fix-up list: `seek_to_sample` folds state on the destination sample, `Seek` while paused
moves the sequencer, `Play` releases the slots. The example plays its fixtures entirely on
the piano because the Mutopia files set no programs — batch 4 derives a *cast* fixture.

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

## Batch 4 — K3: shared-part triggering, musical timing, bounded voice ownership (#62) — DONE 2026-10-20 (fe1bd5f)

Authored by the executor (a Claude Opus session) from `review/conductor/handoff-4.md`; report
`review/conductor/report-4.md`; reviewed by kimi (`review-7.md`), gemini (`review-7-gemini.md`)
and the planner (`review-7-claude.md`); adjudicated in `rebuttal-7.md` (4 accepted, 4
rebutted); Jesse's listen check passed natively. Six executor decisions accepted: the epoch
is one host-wide activation count; the per-instrument cap is the slot's waiting note-ons;
every ledger note has a `NoteTicket` so a scheduled note-off ends only its own note; a song
change flushes before it loads; `AtSongPosition` reads the last-started unit; note ends are
measured from the play sample. The discontinuity threshold is frozen at 0.02 (measured
0.0131 overflowing). **Carried to batch 5's fix-up list:** kimi's catch — a trigger admits
its source before the checks that may refuse it, and a full table's admission evicts a live
source — and the drain clock advanced per block inside a multi-block `render`.

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse (the example gains a
keyboard trigger). Size: about +2600 lines.

Files: `src/{triggers.rs, triggers/cursor.rs, triggers/quantize.rs, sources.rs,
overflow.rs}` + tests; `commands.rs` grows `Trigger { binding, source: SourceId, stamp }`
(activation is lazy — the first trigger activates), `DeactivateSource`, `StopAllSources {
flush_epoch }` (the existing `Pause`/`Resume` are the music pause; no second pair);
`conductor.rs` dispatches; `SourceId { id, generation, epoch }` replaces batch 3's
`SourceKey(u64)` stand-in.

**Batch 3's fix-ups, first** (accepted in `rebuttal-6.md`): `Sequencer::seek_to_sample`
includes the state events on the destination sample (the `<=` rule `seek_to_tick` keeps),
with a test that Resume at a sample carrying a program change restores it; `Seek` while
paused moves the sequencer as well as the resume point, so `Stop` after it publishes the
seeked position; `Play` releases every slot's notes as `Stop` does. And a **cast fixture for
the listen checks:** `scripts/make_cast_fixture.py` derives `dansenapolitaine-cast.mid` from
the type-0 file by adding program changes per channel (flute → 73, the pianos → 0, the
tambourine already on the kit), licensed in `fixtures/LICENSE` like the type-0 derivation;
the example plays it by default from this batch on. A slot's trigger sounds on the next
64-frame boundary of its synthesizer (decision 3): the tests assert placement to that grid,
never to the sample.

**Target shape.** `SourceId { id: u64, generation: u64, epoch: u32 }` (id and generation
mirror `EntityId`, built by the host; the epoch counts activations of that identity, decision
2); per-source lane ownership in a slot; one shared cursor per part for `Loop`
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

## Batch 5 — K7: user sample instruments (#140) — DONE 2026-10-20 (577b53b)

Authored by the executor (a Claude Opus session) from `review/conductor/handoff-5.md`; report
`review/conductor/report-5.md`; reviewed by kimi (`review-8.md`), gemini (`review-8-gemini.md`)
and the planner (`review-8-claude.md`); adjudicated in `rebuttal-8.md` (2 accepted, 5
rebutted); Jesse's listen check passed natively. Measured: pitch within 0.0001 %, durations
within a frame, the ceiling held under three bursts of 64. **Carried to batch 6's fix-up
list:** the loader checks the file's size before reading it (kimi), a one-shot fades its
last 5 ms instead of stopping dead when the WAV does not end at silence (kimi), the sample
slot refuses the newest trigger at its ceiling instead of cutting a fading voice (Jesse's
call, 2026-10-20: no click under a scripted burst), and the final output limiter K7's issue
asks for, which belongs after the bus gains in the audio source.

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse. Size: about +1600 lines.

**Batch 4's fix-ups, first** (accepted in `rebuttal-7.md`): `TriggerEngine::trigger` admits
its source only after every check that may refuse the trigger (the slot, the playing gate,
lateness, the unit selection and the capacity check need no source), so a refused, late or
paused trigger is a no-op and a full table never evicts a live source for nothing — with
kimi's test: a full table, sounding notes, refused and late triggers, no live source evicted;
and `Conductor::render_at` advances `now` by each block's duration before `begin_block`, so a
2048-frame pull reads its 32 blocks as 32 instants and the documented stamp bound holds for
any call length.

Files: `crates/audio/src/sample_loader.rs` (+ tests; `load_sample_pcm(path) -> AudioResult<OwnedPcm>`
through `common::vfs` and the rodio `Decoder`, WAV only accepted as a *pitched* import, the
limits of decision 5, typed errors naming the path); `crates/music_midi/src/{pcm.rs,
sample_voice.rs, sample_bank.rs}` + tests (`OwnedPcm` lives in `music_midi` so audio depends
on it, not the reverse); `conductor/slots.rs` learns a `SampleSlot` (no lanes: a direct voice list with its own
live-voice cap and oldest-voice steal through the fade, **and a hard ceiling of the live cap
plus a fade budget that cuts the oldest fading voice**, so a burst inside one block cannot
pile up voices; a one-shot is sounding until its pitch-scaled end); a sample shorter than
1 ms is rejected before allocation; fixtures from batch 1. **The trigger path learns sample
slots:** a binding to a sample instrument resolves to a sample slot (batch 4 left it `None`
and rejected the trigger), the conductor's `SlotSink` plays either kind — an `enum` over
`InstrumentSlot` and `SampleSlot`, or a trait both implement — and a sample slot issues
`NoteTicket`s like a synth slot so scheduled note-offs, `release_source`, wrap and `Play`
treat both alike; a one-shot ignores its scheduled note-off, a gated voice fades over 5 ms
on it. `crates/audio` gains `music_midi = { path, default-features = false }` (the PCM type
and nothing of the synth; batch 6 turns `synth` on there).
Tests: known-frequency pitch at 48 kHz and 22.05 kHz sources rendered at 44.1 kHz (zero
crossings counted), duration behaviour, stereo/mono, gated vs one-shot, two concurrent
voices of one sample with independent note-off, oversized and non-finite input rejected
without touching the loaded song, combined sample + SoundFont transport, output finite
and click-free on steal (max sample-to-sample delta bounded). **Gates:** standard;
`check_wasm.sh`. **Leaves out:** time stretch, multisample maps, the picker (K8).

## Batch 6 — K4a `crates/audio`: the song manager and the conductor source (#63) — DONE 2026-10-20 (51704ed)

**Landed as reviewed** (`rebuttal-9.md`), with the executor's decisions accepted in `review-9-claude.md`: **one conductor per song** (decision 4 amended — the conductor lives on the audio thread, so loading a second song into it would need a lock or an allocation there; each song gets its own conductor, source and sink, the flush floor carried as the new conductor's first command, the sink swap a hard cut like `play_music`'s); `SOUND_FONT_BYTE_LIMIT` 48 MiB and `SONG_FILE_BYTE_LIMIT` 256 KiB (decision 5 amended); `AudioManager::set_asset_base` for the sidecar's relative paths (batch 7 wires it); the dependency cache keyed by a content fingerprint (24.5 ms a load on the cast bank — batch 9's fix-up); `SAMPLE_VOICE_LEVEL = 0.2` and a look-ahead gain limiter (threshold 0.9, attack 1 ms, release 80 ms) from Jesse's first listen round; `latency_samples` counts only the engine's own block. Planner hunks (`draft-6-fixes.diff`): **the recorded path reads and decodes before it stops anything** — one rule for both kinds, nothing replaced until the new request has loaded, inverting H7's documented choice with Jesse's confirmation; a sample shorter than the fade ramps from full level over its own length; the disabled-mode docs name the song's virtual clock; the playground bundle measured at 11.11 MiB (+0.50 MiB for rustysynth and the song seam) under its 12 MiB gate. Carried: the stop-both idiom to batch 7's guide; the cache key to batch 9; the level note to batch 10; the per-game bundle measurement to batch 12; the engine's key repeats reaching `on_key_pressed` filed as #177. Jesse's listen check passed natively.

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse. Size: about +2000 lines.

**Batch 5's fix-ups, first** (accepted in `rebuttal-8.md`): `common::vfs::file_size(path)`
(native metadata; the memory map's length on wasm) and `load_sample_pcm` refuses a file over
`SAMPLE_BYTE_LIMIT` by size *before* reading it; `SampleVoice` starts its 5 ms fade when its
position comes within one fade of the sample's last frame, so a one-shot whose WAV ends at
full scale ends on the same ramp as every other end (test: a hand-built WAV ending at 1.0);
`SampleSlot::note_on` at the ceiling **refuses the newest voice** — the trigger engine's
`admit` treats the sample slot's ceiling like every other overflow (newest refused, cursor
kept, counted) — and the burst test's step threshold is re-frozen at the fade's own; a
**soft peak limiter on the combined stream after the bus gains** in `ConductorSource`
(threshold and knee as constants, its action counted on `SharedPosition`), measured on the
worst-case storm so 24 synth notes and 16 sample voices cannot clip the device.

The loader fills every field of `ResourceMetadata`, the `soundfonts` set included (the
membership check lands in batch 3 and refuses a song whose background bank it does not
list) and `sample_seconds` from `SampleBank::seconds()`, decoding each sample path once
into a map so `Song::new` never sees a duplicate; a song change sends `StopAllSources` and loads only once it is drained (a
`BindingIndex` is an index into one song's bindings, and the conductor keeps the flush
floor across the swap); the rodio adapter renders one block per pull so each block drains
with its own clock; `song_position` reports block starts, which is where a scheduled
trigger fires (up to 63 frames early, never late); the root `[dev-dependencies] rodio` that batch 3's example needed goes when this
batch's adapter lands and the example switches to it; `SharedPosition::snapshot` may
straddle one block (documented) — consumers read one field, `beat_crossed` the sample
position alone, and a seqlock is added only if a consumer needs two fields together.

Files: `crates/audio/src/manager/song.rs` (`play_song(path)` and `play_song_in_memory(Arc<Song>)`
— the composer's preview — `stop_song`, `pause_song`, `resume_song`, `seek_song(beat)`,
`is_song_playing`, `song_position() -> Option<SongPosition { bar, beat, beat_in_bar,
seconds, latency_samples }>`, `beat_crossed(Subdivision)`, `trigger(action: &str, source:
SourceId)` — the action resolved to a `BindingIndex` on the game thread through
`Song::binding_index`, cached per loaded song, the stamp taken there — `deactivate_source`,
`stop_all_sources` (there is no activate call: the first trigger activates, decision 2);
`PendingMusic` becomes an enum over the recorded track and a song, one field, last request
wins across kinds; `crates/audio/Cargo.toml` turns `music_midi`'s default `synth` feature
on (rustysynth compiles for wasm, K1 proved it); `manager/mod.rs` is 387 lines and
`music.rs` 181, so the song manager is its own file from the start), `manager/song/load.rs`
(the dependency closure through `common::vfs`: `.mid`, `.mid.ron`, every SoundFont and
sample, validated in full — disabled mode included, byte limits before parsing — before
the current playback is replaced; decoded *dependency files* (SF2, WAV) are cached by each
file's own fingerprint — never per song, so a composer edit creates no entry and the preview's
in-memory song shares them — capped at the current song's closure plus the previous one, so a
changed file at the same path is a different key and a second `play_song` of unchanged files
does not decode again; a game may prewarm at `init`), `conductor_source.rs` (the rodio `Source`, decision 3), `manager/song/tests.rs`;
`manager/music.rs` learns the shared slot (decision 4); `lib.rs` re-exports the public
`music_midi` types; `crates/audio/CLAUDE.md` rewritten (the "no spatial, no component" line
goes in batch 8); `examples/conductor_background.rs` drops its hand-made adapter, loader
calls and `Song::new` for `ctx.audio.play_song(path)` / `trigger` / `song_position`, and the
root `[dev-dependencies] rodio` goes with it (`audio` stays, the manager is there);
`crates/common/src/vfs/mod.rs` gains `file_size` for the size-before-read fix-up (internal,
both targets, with a test beside `read`'s). Tests: pending/start/last-wins across both kinds, volume per bus (music 0
keeps triggers audible, sfx 0 mutes them, the clock runs at both), beat boundaries by
integer counting, the disabled-mode virtual clock advancing `beat_crossed` under `update()`,
one crossing at most per `update()` across a many-beat gap, pending transport commands
retried from `update()`, a song change flushing sources and the next trigger re-activating, dead receiver marks stopped with a rate-limited log, dependency failure
preserves the current song, lifecycle under saturation through the public API. **Gates:**
standard; `check_wasm.sh`; `check_games.sh` (public items of `audio` changed). **Leaves
out:** the host (next), the editor.

## Batch 7 — K4b `engine_core`: the host owns sources, scripts trigger bindings (#63) — DONE 2026-10-21 (83f4eed)

**Landed as reviewed** (`rebuttal-11.md`, adjudicated under Jesse's delegation: kimi 3 findings, 2 accepted; gemini 4, all rebutted on a misread asset base), with the executor's `SoundSources` as a runner field lent as `ctx.sources`, the scripted triggers queued in `scripting/triggers.rs` and submitted by the frame tail before the reconcile, the flush at the advanced epoch on a hierarchy reset, no host flush on a song change, the asset base seeded from the game's config, and the headless runner on the disabled manager. Planner hunks (`draft-7-fixes.diff`): `docs/SONG_FORMAT.md` names the runner-seeded asset base; the trigger queue is capped at `TRIGGER_QUEUE_CAPACITY` with a counted drop. Follow-up: #178 (the harness suite advances the virtual clock by sleeping). **Jesse's listen check passed natively 2026-10-21**: Left Shift walks the melody's shared cursor on the grid beside keys 1–4, M keeps the triggers, S keeps the background.

Executor: the Opus session. Reviewers: kimi, gemini. Listen check: Jesse (the example gains a
scripted trigger). Size: about +1800 lines.

**Re-verified against the tree 2026-10-20, after batch 6.** Three corrections to the first
cut: the engine never calls the script runner — a game steps it from `ctx.scripts`
(`early_update` / `update`, game-template's `gameplay.rs:20-29`) and the runner applies each
phase's commands itself (`ScriptCommands::apply(world, physics, delta_time, errors)` from
`runner.rs:502`), so a trigger command cannot reach `ctx.audio` there; the runner already
walks its instances in `BTreeMap<(EntityId, usize)>` order, so same-frame order is
structural, not a sort to add; and `World::clear` keeps resources, `EntityId`s are never
recycled, and the editor's snapshot restore revives an id's exact generation
(`create_entity_with_id`), so the reset hook must be an explicit flush, not a scan.

Files: `crates/engine_core/src/sound_sources.rs` (+ tests; **`SoundSources`, a runner
field lent to the context as `ctx.sources`**, the way `scripts` is — not a world resource:
the frame tail needs it beside `audio_manager` while it walks the world — holding
`EntityId → SourceId { id: entity id, generation: entity generation, epoch }` with **one
host-wide epoch counter** (decision 2): `entity(id) -> SourceId` returns the live entry or
mints one with the next epoch (a re-entry is a new epoch); `game() -> SourceId` and
`menu() -> SourceId` are two fixed ids above any entity's (`EntityId`s are monotonic
`u64`s from zero; the top two values are the host's), **entries in the same table** that
validation never reconciles away: they too hold the live entry or mint one with the next
epoch, so a flush, which forgets every entry, re-mints them on their next call — never a
fresh epoch per call, which would release the previous activation's notes and break a
shared-cursor walk (kimi F1); a sound that must outlive its entity, a brick's explosion,
is triggered on `game()`, the game's arrangement choice under K3's "fade owned voices" rule; `reconcile(world, audio)` sends
`deactivate_source` for every entry whose entity no longer `validate_entity`s and forgets
the entry — the manager itself keeps a refused lifecycle send and retries it from
`update()` (`pending_lifecycle`), so the host holds no retry queue; `flush(audio)` advances
the counter and sends `stop_all_sources` at the **advanced** value, so an epoch minted this
frame falls below the floor (the table refuses epochs strictly below it — kimi F4), then
forgets every entry; `refused_triggers: u64` counts every trigger the manager refused
(`CommandRefused`, `SongNotPlaying`, `UnknownAction`), readable by tests, since the
conductor's `rejected_triggers` sees only what reached it (kimi F5); the epoch counter is
the `u32` the `SourceId` carries and never rewinds — 2^32 activations is 49 days at a
thousand a second, documented as the bound, not guarded), `game.rs` (the field, `build_context!`, and **`audio_manager.set_asset_base
(config.asset_base())` right after construction** — batch 6's loader resolves sidecar paths
against it, and no game calls it; `GameRunner::headless` goes through `new`, so the harness
gets it too), `contexts.rs` (`sources`), `game/frame_tail.rs` (after `step_world_systems`
and before the hierarchy, in this order: **if `TransformHierarchyReset` is filed — read with
`has_resource` before `TransformHierarchySystem::update`, which consumes the marker —
`flush` first and drop this frame's queued triggers, which belong to the old world; else
submit the scripted triggers queued this frame, then `reconcile`**, so an entity a script
triggers and despawns in one frame plays its note and has it released by the reconcile —
the documented rule; a trigger the manager refuses is dropped, counted and logged at debug
(rate-limited), never a script error — a sound is best-effort), `scripting/commands.rs` (`ScriptCommand::TriggerBinding { target:
Option<Target>, binding }`; `apply` cannot reach audio, so **a new `scripting/triggers.rs`**
holds the queue the runner fills as it applies — `runner.rs` is at 594 lines — in the order
applied: `(EntityId, enqueue order)` by construction, which a test on the real path pins;
**the scripting module and `ctx.scripts` sit behind the `physics` feature** (`game.rs:263,
286`), so the queue, the command and the verbs are gated with them while `SoundSources`,
`ctx.sources`, the reconcile and the flush are not — a game built without scripting still
reconciles; `check_features.sh` holds it — kimi F6),
`rhai_bindings.rs` (`cmd.trigger(me, "hit")`, `cmd.trigger("name", "hit")` and
`cmd.trigger("hit")` — the last on the game-owned source; three `register_fn`s like
`despawn`'s), `test_support.rs` (`GameHarness::audio()` and `audio_mut()`, so a test reads
`song_snapshot().conductor.accepted_triggers` and the live-source state), `docs/SCRIPTING.md`
(an **Audio** group in the command list, and § Order gains where a trigger is submitted),
`training.md` § Conductor (first cut, after § Script Pattern; K5 completes it) — stating
`play_song`/`play_music` as one slot, that **`stop_music` and `stop_song` each stop their
own kind, pending included** (the stop-both line, kimi's batch 6 F2), the virtual clock
headless, `ctx.sources` and the three source kinds. Integration tests in
`crates/engine_core/tests/` through `GameHarness` (`--features test-support`, a headless
manager playing the song on the virtual clock, the song and the sidecar written to a
tempdir the harness's `GameConfig` names as its asset base): a script triggers then
`cmd.despawn`s with no second cleanup call and the source is gone by the next frame;
direct Rust `destroy_entity`; a world replacement through `request_transform_hierarchy_reset`
flushes; stable same-frame order across two entities and two instances; Music and SFX gains
independent; recorded SFX unchanged beside a song; `play_music` → `play_song` with live
entities still triggering — the same `SourceId`, admitted into the new song's table (no
host flush on a song change, decision 3 as amended); a flush then a `menu()` trigger,
admitted under a re-minted epoch; a trigger queued in the frame of a world replacement
never sounds in the new world; a refused deactivation (the queue filled by a
storm) followed by re-entry before the retry — the older generation never revives. The
example gains a `.rhai` script on one entity that triggers on a key through `cmd.trigger`
(the listen check). **Gates:** standard; `check_wasm.sh`; `check_games.sh --test`.
**Leaves out:** camera eligibility (a game policy, K6), K9, the editor's play-session
flush (batch 11 calls `flush` from `song_preview`).

## Batch 8 — K9: scene-authored audio — `SoundEmitter`, the sound tier, the asset kind (#174) — DONE 2026-10-21 (cf02197)

**Landed as reviewed** (`rebuttal-13.md` and `rebuttal-14.md`, adjudicated under Jesse's delegation: kimi 4, gemini 5, codex 5 on the diff, then kimi 3 and gemini 5 on the planner's fixes), with the executor's placements accepted in `review-13-claude.md`: `manager/playback.rs` for the instance table, one held list for both disabled modes, `PanSource` yielding the input's own sample type with a gain of exactly one skipped, the entity named on every component error by prefixing at the caller. Planner hunks (`draft-8-fixes.diff` and two after it): the floors seam strips `spatial` and `looping` from a binding emitter (three reviewers; it overturns `rebuttal-12`'s floors-not-semantics line, the API's own contract being GUI parity); **the pass runs every frame with first sight alone gated on a live time scale** — a paused game's requests are served, not held to the cap (this amends the section's "its queued requests wait"), and a loop whose entity goes while paused stops; a binding stop in the spawn frame marks the entity seen; a one-shot released by a newer play, and a detached one, stay owned until they end so `clear` stops them; **`play(entity, world)`** carries the emitter and position so a Rust play-then-despawn sounds like a script's; a re-added emitter spawns again; `clear` forgets the play counts; the gesture test runs an `update` so it passes without a device. Rebutted: a rescan invalidation (neither tier has one — #180 filed), the borrow objection to `play`'s world, the detached mix at the origin, a stop consuming first sight. Jesse's listen check passed natively 2026-10-21 (the coin's beep panned right of centre, the jump on every press at its old level).

Executor: the Opus session. Reviewers: kimi, gemini, codex (the popup, the inspector rows,
the asset tile, the drag ghost). Size: about +2500 lines (up from +1900 on re-verification:
the instance API, the pending loops and the pan adapter of correction 1).

**Re-verified against the tree 2026-10-21, after batch 7, and reviewed** (`review-12.md`,
`review-12-gemini.md`, `review-12-codex.md`, adjudicated in `rebuttal-12.md`: 19 findings,
18 accepted). Six corrections to the first cut, each carrying what the round added:

1. **`crates/audio` has no playback instance.** `play_with_settings` returns `AudioResult<()>`
   and `stop(handle)` stops every instance of a sound (`manager/mod.rs:303, 393`), so "an
   `EntityId → playing instance` table" had nothing to hold and "only one stops" nothing to
   do it with. The batch gives the manager a **`PlaybackId`** (a `Copy` id from a
   manager-local counter, minted in disabled mode too so the host's table is uniform),
   returned by `play_with_settings` and `play` (callers: `manager/tests.rs`,
   `tests/song_triggers.rs:286`, the example — none reads the `()`), `stop_playback(id)`
   (that instance alone; an ended or unknown id is a no-op), `set_playback_mix(id, volume,
   pan)` (the instance's base volume re-derived through the bus and master as
   `update_all_volumes` does, and its pan) and **`is_playing(id)`** (true while the instance's
   sink is in the table, which `update_at` already prunes at `:416`). **Disabled mode keeps a
   ledger** so the host and the headless tests see one contract: a looping instance is
   reported playing until `stop_playback`, a one-shot is reported ended at the next
   `update`; and the **gesture-gated mode keeps a looping instance pending** — the handle,
   its settings and its pan atomic — started from `enable_output` exactly as `pending_music`
   is (`manager/mod.rs:222`), dropped by `stop_playback`; a one-shot played before the
   gesture is dropped. Pan is live: rodio's `ChannelVolume` holds fixed gains, so a new
   **`pan_source.rs`** wraps the decoder as a `Source` reading the pan from an
   `Arc<AtomicU32>` f32 bit pattern once a frame (the conductor's bus-gain idiom): two
   channels for mono input (each sample emitted twice) and stereo input (alternated); an
   input above two channels is reported at its own channel count and passed through
   unpanned. **The law is unity at centre**, `left = min(1, 1 − pan)`, `right = min(1, 1 +
   pan)`, so a default play is byte-identical to today's; pan clamped to −1..=1 at the point
   of use like volume. `SoundSettings` gains `pan: f32` (default 0, `with_pan`).
   `play_source` and recorded music are untouched. Unit tests: the adapter's sample pairs at
   pan −1, 0 and 1 on mono and stereo input, a default play untouched, a six-channel input
   passed through, the clamp; the pending loop surviving a failed `enable_output` (matched on
   the result, as the gesture tests do).
2. **"Cleared on scene load" is gone.** The runner has no scene-load hook — a game calls
   `SceneLoader::load_and_instantiate` itself — and the texture cache this tier sits beside
   is never cleared in a session (`AssetManager`'s `(path, filter)` dedupe); a clear that
   left the handles loaded would leak a `SoundData` per reload. The library lives in
   `AssetManager` as its sound tier, **`assets/sound_library.rs`** beside
   `assets/sprite_sheet.rs` (`assets.rs` is at 552: the field only there, the impl block in
   the child module): `AssetManager::resolve_sound(&mut self, path: &str, audio: &mut
   AudioManager) -> Result<SoundHandle, AudioError>` — `path` relative to the asset base
   like a texture, loaded through `audio.load_sound` on first sight and cached by path for
   the session; a path that failed is remembered with its error and warned **once**, so a
   missing file costs one log line, not one a frame. `set_base_path` clears the map as it
   clears the texture map (`assets.rs:469`); the handles stay loaded, a base change being a
   project switch. That method is the public handle lookup #174 asks for: game code
   resolves and plays through `ctx.audio` itself.
3. **The system is a runner field, not a world resource; it runs after the hierarchy, and
   not at all while the engine is frozen.** `crates/engine_core/src/sound_emitters.rs`:
   **`SoundEmitters`**, lent to the context as `ctx.emitters` the way `sources` is, with
   `play(entity)` and `stop(entity)` queueing a request (game code; scripts reach the same
   system through correction 5), `playing(entity) -> Option<PlaybackId>`, `plays(entity) ->
   u64` (starts, kept across evictions), `last_mix(entity) -> Option<(f32, f32)>` (the gain
   and pan last sent — what a headless test asserts), `failed_loads`, `detached_plays` and
   `refused` counters (a request on an entity without an emitter is refused, counted and
   logged at debug); and `update(world, assets, audio, sources)`, run by the frame tail
   **after `transform_hierarchy.update`** (a child emitter's position is this frame's
   `GlobalTransform2D`, which the grids read the same way; `Transform2D` when there is none)
   and before the grids — **skipped whole while `time_scale == 0`**: the editor and the
   playground freeze the engine that way outside Play (`editor_game/mod.rs:172-176`,
   `preview.rs:162-165`), so Editing records no first sight and starts nothing, Play's first
   frame is the first sight and the spawn sounds fire then, and a paused game keeps its
   loops and starts no spawn sound while its requests are still served (amended on landing:
   the pass runs every frame, first sight alone gated). The pass: an entity seen for the
   first time with `play_on_spawn` plays; the requests drain; every playing spatial
   instance's mix is recomputed; an entry whose instance ended (`is_playing` false), whose
   entity no longer `validate_entity`s, **or whose entity no longer carries the component**
   (an inspector `[X]`, an undo) is stopped and forgotten in that same tail, and a stop
   request acts on the table whether or not the component is still there. **`play` on a
   playing entry**: a one-shot starts a new instance and the previous one plays out
   untracked, its mix frozen where it was; a looping emitter's loop already running is a
   no-op. A path emitter plays through the library and the manager (looping and volume from
   the component, pan from the mix); **a binding emitter plays through
   `SoundSources::trigger(audio, binding, sources.entity(entity))`** (`sound_sources.rs:86`)
   and stops through the new `end`. The world-replacement branch of `settle_sound_sources`
   calls `clear(audio)` whatever the time scale: every instance stopped, the table and the
   requests dropped — the flush rule, applied to emitters. **The component carries no
   runtime state**: first sight, the instance and the last mix live in the system's table,
   so `SoundEmitter` serializes and snapshots as plain data.
4. **The spatial math, frozen for the tests.** `d` is the distance from the emitter to the
   main camera (`render_manager::main_camera_pose`, which the render path already trusts)
   in world units, pixels over `RENDER_UNIT`; `gain = volume · (1 − d / max_distance)`
   clamped to 0..=1; `pan = (emitter.x − camera.x) / RENDER_UNIT / max_distance` clamped to
   −1..=1; zoom ignored; with no main camera the mix is `(volume, 0)`. So at `max_distance`
   the gain is 0, at half of it `volume / 2`, and half of `max_distance` to the right pans
   0.5. A pure `spatial_mix` function carries it and **applies the 0.01 floor to
   `max_distance` itself** (the loader floors a file's value; code builds components too)
   and returns `(0, 0)` for a non-finite distance, so no NaN reaches a sink; unit-tested
   with those numbers, a zero `max_distance` included.
5. **Scripts cannot reach the emitters from `apply`** (`ScriptCommands::apply` has the world
   and physics alone — batch 7's rule), so `cmd.play_sound(me | name)` and
   `cmd.stop_sound(me | name)` become `ScriptCommand::PlaySound { target }` /
   `StopSound { target }`, whose `apply` resolves the target like `despawn`'s and pushes
   onto **one FIFO in `scripting/triggers.rs`** (100 lines): `QueuedSound::{Trigger { source,
   binding }, Play { entity, emitter, position }, Stop { entity }}` under the one capacity,
   so `stop_sound(me); trigger(me, "hit")` keeps its order. `submit` walks it in order: a
   trigger is sent, a binding emitter's request is served there through `SoundSources`, a
   path emitter's request is handed on in order to the pass. **A play request carries the
   emitter and the entity's position as they were at apply**, so `play_sound(me); despawn(me)`
   in one script still sounds: a one-shot whose entity is gone by the tail plays detached at
   that position, untracked and counted in `detached_plays`; a loop on a gone entity is
   refused — a loop belongs to a living owner, the recorded twin of the trigger rule.
   **`commands.rs` is at 583 lines** and two variants with their arms cross 600: the
   executor first moves `Target` and its resolution (`commands.rs:22-53` and the resolve
   block at :261-296) into a new `scripting/target.rs` — a pure move, re-exported where it
   was — and only then adds the commands. `rhai_bindings.rs` is at 539 (six `register_fn`s
   fit), `runner.rs` at 597 (untouched), `game.rs` at 562 (the field, the construction and
   the `build_context!` line fit). The script queue sits behind `physics` with the rest of
   scripting; `SoundEmitters`, `ctx.emitters` and the tail's pass do not —
   `check_features.sh`.
6. **The editor side, placed where it fits.** `panel_renderer/asset_browser.rs` is at 598,
   so the *arming* — `TileInteraction` and `tile_interaction`, which gain the `Sound`
   press — moves to a new `panel_renderer/asset_browser/tile.rs` (child module of the file;
   `render_tile`'s glyph arm becomes `AssetKind::Script | AssetKind::Sound`, the extension
   in accent). The *drop target* is the inspector's sound slot: a new
   `crates/editor/src/sound_field.rs`, the twin of `texture_field.rs`, showing the path and
   taking `DragPayload::Sound { path }`; `drag_ghost.rs`'s Sound arm shares the Script arm's
   name pill (one helper, two callers); `hierarchy/mod.rs:397`'s exhaustive match gains the
   arm (a sound dropped on a row does nothing — a row drop that adds an emitter is left
   out). **The inspector cannot author what the loader refuses**: cycling the kind to
   Binding clears `spatial` and `looping`, and the Binding view hides their rows and the
   `volume` row (the conductor owns a binding's dynamics; the loader keeps a binding's
   `volume` as harmless data rather than refusing it); `volume` (0) and `max_distance`
   (0.01) are hard floors in `physical_floors.rs`, shared with the command API's sanitizer
   as every floor there is. `AssetKind::Sound` is **wav/mp3/ogg/flac — what `load_sound`
   decodes**; `.mid` and `.sf2` are song kinds and come with K8's picker in batch 10, the
   batch that needs them (decision 9's reuse is the WAV sample instrument).
   `ComponentCategory::Audio` is appended to `ALL` (its length literal is 5) with the label
   "Audio"; `stored_component/tests.rs`'s exhaustive dispatch test covers the popup row by
   construction.

Also: `looping` on a binding emitter is refused at load beside `spatial`, for the same
reason (a binding plays a part; the conductor has no loop for it), as
`SceneLoadError::ComponentError` naming the entity — no new error variant. Stopping a binding
emitter needs **`SoundSources::end(entity, audio)`** (deactivate and forget, so the next
trigger re-mints an epoch; about ten lines and a unit test in `sound_sources.rs`, 207
lines). The playground's import keeps every asset entry whatever its extension
(`archive.rs:305-352` validates sheets, scenes and scripts and stores the rest), so a sound
imports with no change there, and the scene dry-run through `HeadlessAssets` runs the
emitter's load-time checks too.

Files: `crates/ecs/src/sound_emitter.rs` (decision 6's component: `SoundEmitter { sound:
SoundRef, volume: f32, play_on_spawn: bool, looping: bool, spatial: Option<Spatial> }`,
`SoundRef::{Path(String), Binding(String)}`, `Spatial { max_distance: f32 }` in world units;
`Default` is a path emitter with an empty path, volume 1, nothing else set; registered as a
builtin in `component_registry/mod.rs:330-356`; re-exported), `crates/audio/src/{sound.rs,
manager/mod.rs, pan_source.rs}` (correction 1; `manager/mod.rs` is at 429 — the instance
methods and the pending loops may not fit beside the device code: if they cross 600, a
`manager/playback.rs` takes the instance table, the way `music.rs` takes the track),
`crates/engine_core/src/assets/sound_library.rs` plus the field in `assets.rs` (correction
2), `sound_emitters.rs` (correction 3, ≤ 450 lines with its unit tests; if the pass and the
request handling push it over, `sound_emitters/update.rs` takes the pass), `game.rs`,
`contexts.rs` (`emitters`, after `sources`), `game/frame_tail.rs` (correction 3's two call
sites), `sound_sources.rs` (`end`), `scene_data.rs` (`ComponentData::SoundEmitter { sound,
volume, play_on_spawn, looping, spatial }` — typed for the load-time checks and the
serializer row, not for resolution, which is lazy: nothing in the loader touches audio),
`scene_loader_components.rs` (515 lines: the arm with the checks — `spatial` or `looping` on
a binding emitter → `ComponentError` naming the entity; `max_distance` below 0.01 floored to
0.01; a non-finite or negative `volume` → `ComponentError`), `scene_serializer/components.rs`
(the row), `scene_serializer/tests.rs` (the round trip), `scripting/{target.rs, commands.rs,
triggers.rs, rhai_bindings.rs}` (correction 5), `crates/editor/src/{asset_browser.rs,
drag_drop.rs, sound_field.rs, physical_floors.rs, stored_component/{mod.rs, category.rs},
component_editors/sound_emitter.rs}` (the rows: a `cycle` between Path and Binding, the
sound slot or a `string_edit` for the binding name, `volume` as a numeric row floored at 0,
`play_on_spawn` and `looping` as `bool` rows, `spatial` as a `bool` row that reveals
`max_distance` floored at 0.01 — `component_editors/grid_backdrop.rs` is the model),
`crates/editor_integration/src/panel_renderer/{asset_browser.rs, asset_browser/tile.rs,
drag_ghost.rs}`, `crates/editor/src/hierarchy/mod.rs` (the arm), `test_support.rs`
(`GameHarness::sound_emitters()`). The example: `hello_world.scene.ron`'s `player` gains
`SoundEmitter(sound: Path("assets/sounds/snd_jump.wav"), volume: 0.8)` and `platformer.rs`
plays the jump through `ctx.emitters.play(player, ctx.world)` (the entity from
`SceneInstance::get_entity("player")`) instead of its hand-loaded handle, which goes;
`coin_1` gains a `play_on_spawn` `beep_e.wav` emitter with `spatial: Some(Spatial(max_distance:
8.0))`, so the scene's first frame pans a beep by where the coin sits relative to the
camera — the native listen check, Jesse's, and not a gate for K9. Docs: `crates/audio/CLAUDE.md`
(the first paragraph's "no spatial audio … no audio component" is false after this batch;
Files and Key Types for `pan_source.rs`, `PlaybackId`, `pan`, the pending loops beside
pending music, the disabled-mode ledger), `crates/engine_core/CLAUDE.md` (file map and
pitfall rows: the once-only warning, the after-the-hierarchy order, the time-scale gate),
`crates/ecs/CLAUDE.md`, `crates/editor/CLAUDE.md:65` (the `AssetKind` list) and its
component list, `crates/editor_integration/CLAUDE.md` (file map), `docs/SCRIPTING.md` § 4's
Audio group (`play_sound`, `stop_sound`) and § Order (one queue with the triggers; a path
emitter's request is served after the hierarchy; a one-shot outlives its entity, a loop does
not), `training.md` (the scene RON example at :513 and `README.md:306` gain the emitter
line; § Conductor Pattern gains the emitter paragraph: the fields, the public lookup,
`ctx.emitters`, the after-the-hierarchy rule, the freeze).

Tests, each named for its behaviour: `crates/engine_core/tests/sound_emitters.rs` through
`GameHarness` (`--features test-support`; the harness's temp asset base receives a copy of
`examples/assets/sounds/beep_e.wav`, the `song_triggers.rs` pattern, and the scene names it
relative to that base): a scene naming a sound by path with `play_on_spawn` has played after
the first frame with its mix recorded; a missing file is one warning and `failed_loads == 1`
after ten frames, never a panic; a one-shot's entry is gone the frame after it ends and
`plays` still counts it; `play` on a one-shot entity twice counts two starts (the jump);
`play` on a running loop is one start; a looping emitter's entity destroyed in `update` is
stopped and forgotten in that frame's tail; a looping emitter's component removed leaves
the entity and stops the loop; a scripted `cmd.play_sound(me)` plays on the real path;
`play_sound(me); despawn(me)` counts one detached play, the looping variant one refusal;
`stop_sound` then `trigger` on one binding entity leaves the new activation live, and the
reverse order ends it; nothing starts while `time_scale == 0` and the spawn sound fires on
the first unfrozen frame; a world replacement clears every instance; two binding emitters
on one slot — `stop_sound` on one deactivates its source alone (`sources.source_of(other)`
still live, the song's snapshot still counting the other's notes), riding
`tests/song_triggers.rs`'s fixture copy; `spatial` and `looping` on a binding emitter
refused at load naming the entity; the serializer round trip. In `editor_integration`'s
play-session tests: a `play_on_spawn` scene plays nothing in Editing, plays on Play's first
frame, is cleared by Stop and stays silent after. In `editor`: the exhaustive dispatch test
plus one asserting the Audio group's label and membership; cycling to Binding clears
`spatial` and `looping`; the floors hold a typed `-1`. In `audio`: the adapter's pairs, the
default play untouched, the six-channel passthrough, the pending loop across a failed
enable, the disabled ledger (a loop playing until stopped, a one-shot ended at the next
update). In `sound_emitters.rs`: `spatial_mix` at `max_distance`, at half of it, half to the
right, with no camera, and at zero `max_distance`.

**Gates:** standard; `check_wasm.sh`; `check_games.sh` (public items of `engine_core`, `ecs`
and `audio` change). **Leaves out:** occlusion, a listener component (the main camera is
the listener), a hierarchy-row drop that adds an emitter, pausing recorded instances with
`time_scale` (a paused game keeps its loops), `.mid`/`.sf2` asset kinds (batch 10), a
command-API semantic check beyond the floors (the loader names the entity), the composer.

## Batch 9 — K8a: the song document, the command API, persistence (#141)

**DONE 2026-10-21** (8929cd6). Landed as planned with
four rulings made in flight. Jesse's, in the executor's session: `write_song_file` was added
to `music_midi` beside `parse_song_file` (the editor has no RON writer; the sidecar's inverse
belongs with its parser), and `EditorAction::{NewSong, OpenSong, CloseSong}` were plumbed
through `dispatch_file_action` so the confirm flow's song arms have a caller (no key binding
or menu label — batch 10's). `ImportSong` never landed: it has no path to carry until batch
10's drop or picker, and `song open` of a `.mid` outside the song folders already opens as an
import; `request_scene_replace` became `request_document_action`. The planner's, from the
code rounds (`rebuttal-16.md`, `rebuttal-17.md`): **the prune waits for the pair** —
correction 8's order (prune, then pair) deleted a removed instrument's sample while the old
finals still named it, so a save now writes the new samples, the pair, the prune and
`LICENSE`, the finals, then removes the pair, the pair being the commit point; Open Song
targets an `untitled.mid` beside the open song's file (the `default_scene_path` precedent),
never the open song's own file; a save target not under the asset base is refused; the
play-session refusal answers a preview verb before "not available yet". The bundle measures
11.83 MiB of 12 (+0.54 over batch 8), so batch 10 and 11 need a budget decision before
batch 10 goes out. Carried as issues: #182 (two songs with one stem share one sample folder)
and #183 (a saved song's own `.mid` and `.mid.ron` are named by no `LICENSE`, so a saved
song does not pass the gate as it stands, against correction 1's claim; `assets/ai/music` is
not scanned at all) — both into batch 10 with the sidebar's provenance field and Save As.

**Carried from batch 6 (`review-9-claude.md` F5):** the dependency cache's key is the content fingerprint today, 24.5 ms a load on the cast bank against a 6.8 ms parse, so a composer preview that re-prepares on every edit pays it each time. Fix-up first, as correction 2 below shapes it.

Executor: the Opus session. Reviewers: kimi, gemini (the API seam and the journal). Headless;
no screen, so codex sits out. Size: about +3400 lines (up from +2800 on re-verification and
review: the two schema fields, the vfs seam, the confirm flow's second document, the journal's
rules and the composite edit).

**Re-verified against the tree 2026-10-21, after batch 8, and reviewed** (`review-15.md`,
`review-15-gemini.md`, adjudicated in `rebuttal-15.md`: 16 findings, 14 accepted). The
corrections to the first cut, each carrying what the round added:

1. **`provenance` and `midi_fingerprint` do not exist.** `SongFile` has seven fields and
   `Instrument` is `{ name, source }`, both under `#[serde(deny_unknown_fields)]`
   (`music_midi/src/song_file.rs:60, 143`); decision 12's two fields are this batch's to
   add, each `#[serde(default)]` so every sidecar on disk still loads and `validate.rs`
   refuses nothing for them at load. `Provenance { author: String, license: String }` as
   `provenance: Option<Provenance>` on `Instrument`: **Save requires it on every `Sample`
   instrument** (a SoundFont preset is covered by the bank's own `LICENSE`), refusing with a
   message that names the instrument and the field — `SetInstrument` carries it, and batch
   10's sidebar surfaces it — and writes the song's sample directory's `LICENSE` in the
   Debian copyright format `scripts/check_audio_provenance.sh` parses: one `Files: <path
   relative to the LICENSE>` / `Copyright: <author>` / `License: <license>` stanza per sample,
   the shape `examples/assets/sounds/LICENSE` shows, no globs, so a saved song passes the
   gate as it stands. `midi_fingerprint: Option<u64>` on `SongFile`: FNV-1a over the `.mid`
   bytes exactly as Save writes them (the audio crate's `fingerprint` is the model), the
   journal's truth (correction 8). `docs/SONG_FORMAT.md` § The grammar gains both.
   `SongFile::check_schema` (`validate.rs:30`, `pub(super)`) becomes `pub`: it is the pure
   text-level check — the instrument cap, repeated or empty names, a binding naming nothing,
   out-of-range numbers — that `SongDocument::apply` runs on the file an edit would produce
   and refuses the edit on, and Save runs once more before any journal step, so the editor
   can never write a sidecar `parse_song_file` would refuse (nine `AddInstrument`s stop at
   the ninth). The batch touches `crates/music_midi` for the two fields, the visibility and
   the doc only; decision 7's dependencies are untouched.
2. **The cache fix-up needs a vfs seam.** `common::vfs` has `read`, `file_size`, `write`,
   `list_files` and `remove_prefix` — no metadata call. The fix-up adds
   `vfs::change_stamp(path) -> io::Result<Option<ChangeStamp>>` (`len` and the modified
   time as nanoseconds, from `std::fs::metadata` natively; `None` on wasm, where the map
   keeps no clock), and `DependencyCache` (`load.rs`, 350 lines; its map is by path,
   `files: HashMap<PathBuf, CachedFile>` at :185, so the key below is the entry's
   freshness, never its identity) keeps a file's entry fresh by its stamp when there is one
   and by the content fingerprint otherwise, so a native load costs one metadata call, not a
   hash of two megabytes. The documented pitfall: a file rewritten at the same length inside
   the filesystem's modified-time granularity is not decoded again (ext4 counts nanoseconds;
   FAT two seconds) — a thing the composer never does to a bank or a sample. The audio
   guide's cache row changes its words and keeps its tests (`loading.rs`'s changed-file
   case writes different bytes, so a new stamp); a unit test pins the fallback.
   `crates/common` is public, so `check_games.sh` runs.
3. **The confirm flow sees one document.** `PendingSceneAction` is a `pub(super)` enum of
   `NewScene` and `OpenScene(PathBuf)` with a `phrase()`, `request_scene_replace` checks
   `command_history.is_dirty()` alone, and `ConfirmDialog::unsaved_changes` says "the
   current scene". The song's actions join the same machine, not a second one: the enum
   gains `NewSong`, `OpenSong(PathBuf)`, `ImportSong(PathBuf)`, `CloseSong` and
   `StartPlay`, each with its phrase and a `document()` (Scene or Song) that the request
   reads to check the right dirtiness — a scene action never prompts for the song, since
   opening another scene loses no song edit, the song being a second document — and the
   router's Save branch saves the document the action names. Two constructors beside
   `unsaved_changes`: `ConfirmDialog::unsaved_song(phrase)` (Save / Discard / Cancel) and
   `ConfirmDialog::play_with_unsaved_song(saved_exists)` — Save / Play saved / Cancel when a
   saved file exists (decision 12's narrowing, no Discard arm on Play), Save / Cancel when
   none does, so "Play saved" never names a file that is not there. **A never-saved song is
   dirty** (`SongDocument::is_dirty` is the history's watermark OR `path.is_none()`; the
   watermark alone calls a fresh history clean), so `StartPlay` parks `handle_play_action
   (Play)` (`play_session.rs`, 314 lines) behind the prompt for a new song too; its Save arm
   writes to `default_song_path()` — `assets/music/untitled.mid`, numbered if taken, the
   `default_scene_path` precedent (`scene_io.rs:256`) — since no file picker exists and
   batch 10's Save As is pixels. "Play saved" proceeds with the song's file as saved; the
   two-frame transition and the preview are batch 11's. `scene_confirm.rs` is 146 lines,
   its tests 67.
4. **The API's contexts see no song.** `QueryCtx { world, selection, scene_path, dirty,
   play_state }` and `WriteCtx { world, history, selection, play_state, batch,
   texture_known }` (`command_api/mod.rs:33`, `write/mod.rs:35`). `QueryCtx` gains `song:
   Option<&SongDocument>` and `WriteCtx` gains `song: &mut Option<SongDocument>`; `Query`
   gains `Song` and `SongNotes { part }`, and `SceneInfo`'s answer keeps `dirty` scene-only
   and gains `song_dirty` and `any_dirty`, documented; `PureWrite` gains
   `SongEdit(SongEdit)`, `SongUndo`, `SongRedo` and `SongSelect { notes }`, served in the
   editor crate through `SongDocument::apply` — the one mutation path the parity table
   holds the GUI to — and **every one of them is refused while a scene batch is open** (the
   batch is a `MacroCommand` over world commands; a song write has no place in it, and an
   undo inside it would void the batch's atomicity); `HostedWrite` gains `SongNew`,
   `SongOpen { path }`, `SongSave { path }` and `SongClose`, routed by `run_hosted_write`
   (`api.rs`, 253 lines) to `song_io.rs` the way `Save` reaches `save_scene_with`, and
   **`SongNew`, `SongOpen` and `SongClose` refuse while the song is dirty** unless the line
   carries `discard` (`song close discard`) — the API has no dialog, and a silent drop is
   data loss; and `SongPlay { from_beat }`, `SongStop`, `SongSeek { beat }` and
   `SongTrigger { binding }` parse now and are answered `Refused` naming batch 11, so the
   verb table is whole and the drift test — `test_parser_verbs_match_docs` in `specs.rs`'s
   own test module (`specs.rs:199-210`) — holds every row. A new `command_api/song.rs`
   holds the `song …` parser arm and the two query answers, so `parse.rs` (283) and
   `specs.rs` (211) gain only their rows.
5. **The data model builds without the synthesizer, as the manifest promises.**
   `music_midi` without `synth` exposes `song_data`, `song_file`, `tempo_map` and `error`:
   `SongData::from_smf -> (SongData, UnsupportedReport)`, `UnsupportedReport::is_editable`
   with `UnsupportedFeature: Display` (the one feature today is a mixed-channel track) —
   `EditingPolicy::Refused { reason }` is that report's features joined — the edit helpers
   `add_note(track, NewNote)`, `remove_notes`, `move_notes`, `resize_notes`, `set_velocity`,
   `set_pitch`, `assign_part`, `restore_notes`, `set_tempo`/`remove_tempo`,
   `set_time_signature`/`remove_time_signature`, each returning the before-image the
   inverse needs, and `write_smf`. The sidecar's `Part { name, track: TrackSelector, mode:
   PartMode, quantize: Quantize }`, `Instrument`, `Binding { action, part, instrument }` and
   `SongFile.looping` are the targets of `SetPartMode`, `SetQuantize`, the instrument and
   binding edits and `SetLoop`. **`SongEdit::Batch(Vec<SongEdit>)`** is one more variant: a
   paste or a transposition of twenty notes is one entry, every member validated before any
   applies and the inverses undone in reverse — the API's composite, since the scene batch
   is closed to it. The editor's `Cargo.toml` line is `music_midi = { path =
   "../music_midi", default-features = false }`; the workspace build unifies `synth` in
   through `audio`, so `cargo test -p editor` alone is what proves the no-synth build and
   belongs in the gates.
6. **The journal's removal step uses what exists.** `vfs::remove_prefix` removes exactly
   the path it is given on both targets (natively a file or a directory, a missing path
   `Ok`; on wasm the key equal to the prefix and those under `prefix/`, `vfs/mod.rs:168-197,
   324-331`), so the `.new` files are removed with it and no `remove_file` is added. The
   web's write observer persists writes only (`playground/src/web_entry.rs:228`), so on the
   web a removed `.new` lingers in the store until batch 11's transaction deletes it through
   the persistence chain; correction 8's open-time rules are what keep that safe meanwhile,
   and the report says so.
7. **Sizes and placements.** `editor_game/mod.rs` 464 (the field and the mirror fit),
   `context/mod.rs` 528 (`song_dirty` and the title fit), `commands/mod.rs` 555 —
   `SongHistory` is its own file under `song/`, copying the watermark contract from
   `commands/mod.rs:123-215` (`next_id` from 1, `saved_id`, `is_dirty = top_id !=
   saved_id`, a merge reassigns the id and clears redo, `break_merge`, cap 100) and nothing
   of the session floor; `commands/dirty_tests.rs` (108) is its tests' model;
   `editor/src/lib.rs` (166) gains `pub mod song` and the exports; `headless.rs`'s
   `run_headless_editor_api(asset_base, initial_scene, input, output)` answers `song` lines
   through the same `answer_api_lines` with no new parameter.
8. **The journal's rules, stated once.** Save, in order: refuse first (a `Sample`
   instrument without provenance, a file `check_schema` would refuse) and write nothing;
   write the song's samples into `assets/music/samples/<stem>/` (a never-saved song's
   samples are held in the document — bytes and provenance — until its first Save, never
   staged on disk; a same-named file with different bytes gets a content-hash suffix and
   the instrument path follows); prune from `<stem>/` every file the song no longer
   references and rewrite that directory's `LICENSE` with exactly the referenced samples'
   stanzas — the directory is the song's own, so both are safe, and a shared directory is
   never managed; write `<stem>.mid.ron.new` carrying the fingerprint of the bytes about to
   be written, then `<stem>.mid.new`; write the final `.mid`, then the final `.mid.ron`;
   remove the `.new` pair; `mark_saved`. **Open, in order:** a `.new` pair whose `.mid.new`
   bytes match its `.mid.ron.new` fingerprint is the truth — it is recovered over the
   finals whatever state they are in (a crash between the two final writes leaves them
   mismatched, and the pair, removed last, is still there); otherwise, finals that match
   each other (the final `.mid` against the final `.mid.ron`'s fingerprint) stand, and any
   leftover is purged when it matches them and reported when it does not — a lone
   `.mid.new` that differs from the final `.mid` is offered as an import (dirty, `path:
   None`), a lone `.mid.ron.new` is removed; finals that do not match each other and no
   valid pair is an error naming both files, never a silent load. The finals' fingerprint
   decides a purge; the pair's own fingerprint decides a recovery — never the other way.

Files (new): `crates/editor/src/song/{mod.rs, edit.rs, history.rs, io.rs, selection.rs}` and
their tests (each ≤ 400 lines; `io.rs` holds the journal and `io/open.rs` the open-time
rules if the two cross the ceiling together), `crates/editor/src/song/parity_tests.rs`,
`crates/editor/src/command_api/song.rs` (+ rows in `specs.rs`, an arm in `parse.rs`, the
variants in `mod.rs`), `crates/editor_integration/src/editor_game/song_io.rs` +
`song_io_tests.rs`, `docs/EDITOR_COMMAND_API.md` § Songs. Touched: `editor_game/mod.rs`
(the `song: Option<SongDocument>` field; `sync_dirty_mirror` :421-436 becomes scene-dirty OR
song-dirty into `history_dirty_flag`, so `playground_is_dirty` aggregates the song with no
bridge change), `editor_game/scene_confirm.rs` and `crates/editor/src/confirm_dialog.rs`
(correction 3), `editor_game/play_session.rs` (`StartPlay` parked), `editor_game/scene_io.rs`
(`default_song_path` beside `default_scene_path`), `api.rs` (routing and the `discard`
word), `editor/src/context/mod.rs` (a separate `song_dirty` mirror; `title_bar_text` shows
both marks), `crates/editor/Cargo.toml` (correction 5), `crates/music_midi/src/song_file.rs`,
`song_file/validate.rs` and `docs/SONG_FORMAT.md` (correction 1), `crates/common/src/vfs/mod.rs`
(correction 2), `crates/audio/src/manager/song/load.rs`, its guide row and `tests/loading.rs`
(correction 2). The report measures the playground bundle delta against
`PLAYGROUND_SIZE_BUDGET_MIB` (`scripts/build_wasm.sh:63`, 12 MiB; batch 6 measured 11.11).

**Target shape.** Decision 12's `SongDocument { file: SongFile, data: SongData, history:
SongHistory, path: Option<PathBuf>, imported_from: Option<PathBuf>, editing: EditingPolicy
{ Full | Refused { reason } }, pending_samples }`; `SongEdit` enum (AddNote, DeleteNotes,
MoveNotes, ResizeNotes, SetVelocity, SetPitch, AssignPart, SetTempo, SetTimeSignature,
SetLoop, SetQuantize, SetPartMode, Add/Set/RemoveInstrument, Set/RemoveBinding,
Add/RemovePart, Batch) with `try_merge` (same gesture: deltas sum, absolutes replace);
`apply` validates (pitch 0..=127, velocity 1..=127, ticks ≥ 0, part exists, `check_schema`
on the resulting file, `Refused` rejects), calls the `music_midi` helpers, stores the
inverse. Save and open are correction 8. An imported `.mid` outside the two home directories
(`assets/music/`, `assets/ai/music/`) or without a sidecar opens with `path: None` and its
first Save is a Save As into one of them; the original is never written. API verbs: `song`
(query), `song notes [part]`, `song edit <json>` (a serde `SongEdit`), `song undo`/`song
redo` (song-scoped), `song select`, hosted `song new|open|save|close` (`discard` to drop a
dirty song), and `song play|stop|seek|trigger` (parsed, refused until batch 11). Refused in
Playing and Paused and while a dialog is pending; every song write is refused inside an open
scene batch. **Tests:** watermark/merge/break (mirror `commands/dirty_tests.rs`), a fresh
never-saved document is dirty, apply/inverse per variant including `Batch` (all-or-nothing,
one entry), `check_schema` refusing the ninth instrument at apply and at save, refusal
matrix (play states, the pending dialog, the scene batch for every song write, the dirty
song on `new|open|close` with and without `discard`), tempdir round trip, interrupted save
at each step — after the pair lands with the finals untouched recovers; after the final
`.mid` alone recovers; a matching leftover is purged; an unmatched lone `.mid.new` becomes
an import and a lone `.mid.ron.new` is removed; mismatched finals with no pair is the named
error — imported original untouched, a sample without provenance refuses the save naming
the instrument and the field and writes nothing, one with it lands in the `LICENSE` whose
`Files:` tokens are exactly the referenced samples' relative paths, a removed instrument's
file is pruned and its stanza gone, a same-named sample with different bytes gets its
suffix and the instrument path follows, a never-saved song's samples reach `<stem>/` on
its first Save, a save under `assets/ai/music/ai_x.mid` reopens and saves in place, scene
save leaves the song dirty and vice versa and `scene info` reports both, New/Open Song and
Play park behind the prompt (Play's arms by `saved_exists`) and a scene action does not
prompt for the song, the parity table (one row per `SongEdit` variant, GUI path versus API
path, exhaustive match so a new variant without a row fails to compile; and an edit on a
document whose `UnsupportedReport` is not editable is refused by both paths with the
report's reason — the gate gemini's batch 2 review asked for lives here, not in the data
model), headless `run_headless_editor_api` answering `song` lines, the stamp fallback and
the cache kept fresh by it.
**Gates:** standard; `cargo test -p editor` alone (the no-synth build); `check_wasm.sh`
(editor crates are in the wasm workspace); `check_games.sh` (`common` is public);
`check_audio_provenance.sh` run over a saved song's directory, in the report.
**Leaves out:** any pixel (batch 10), audio (batch 11), multi-song tabs, scripting access,
the web's deletion of a removed `.new` (batch 11), a native file dialog.

## Batch 10 — K8b: the piano roll, the note list, the sidebar (#141) — DONE 2026-10-22 (9e045cc)

**DONE 2026-10-22** (9e045cc). Landed as planned by the Opus executor (`report-10.md`,
+4961/−174 before fixes), with three placements the executor moved and declared:
`composer_drag.rs` in `editor_game/` (its owner is `EditorGame`), the panel rendered by a
host branch in `render_panels` (the song is the host's), the key-release seal in
`game_impl.rs`; `sidebar/walk.rs` and `sidebar/blocks.rs` split out to keep the sidebar
under 400; an "Add a part for the shown track" button the section had not listed (a new song
could gain no binding without it); and one decision to look at — a Ctrl chord the composer's
table lacks (Ctrl+S, Ctrl+C/V/X/D) goes on to the editor and acts on the scene. Three code
rounds (`rebuttal-21.md`, `-22.md`, `-23.md`): the planner's fixes brought Save Song As…'s
field into view before focusing it (a focused undrawn field owned the keyboard), made
scrubbed number fields land (the ui's scrub release reports the input value), walked the
Songs list with no song open, named the sample folder in the overwrite prompt, and compared
the song's own path by components; round 22 then found the field gesture's seal leaking
(keyboard nudges never raise the ui's commit, an undrawn row never commits, and the ui's
one-shot latch was shared with the inspector), so the sidebar's walk now owns the gesture —
`FieldEdit { edit, first }`, the first frame sealing whatever was open, closed on the field's
commit, on focus loss with no scrub, and on an undrawn frame — and no ui flag is read; round
23 found the song's `undo`/`redo` leaving the merge open (a continuing edit after an undo
merged into the older entry), fixed in `SongDocument` for every path, and the SoundFont
fields folding into one edit again — those two landed without a fourth round. Gemini's first
run produced nothing (its output limit on the 6,122-line diff); its retry's four findings
were all rebutted. Bundle 11.94 MiB of 16. Carried as issues: #184 (the three screenshots,
never captured — the screen is unreviewed) and #185 (the ui's edit-commit latch and a
per-widget scrub query); #182 and #183 go on to batch 11 as planned.

**Carried from batch 6 (F4):** a sample voice plays at `SAMPLE_VOICE_LEVEL` (0.2,
`music_midi/src/lib.rs:147`) before its gain, so the sidebar's root-note and gain fields say
"gain 1.0 plays a sample near one SoundFont note" beside the pitch-changes-duration note.

Executor: the Opus session. Reviewers: kimi, gemini, codex (every screen). Size: about
+3800 lines (up from +3000 on re-verification and two plan rounds: the songs list and the
path field that stand in for a file picker, the two asset kinds the drop site needs, the
overwrite prompt, the drag transaction, the read-only session, the active track).

**Re-verified against the tree 2026-10-21, after batch 9, and reviewed in three rounds**
(`review-18*.md`, `review-19*.md`, `review-20*.md`, adjudicated in `rebuttal-18.md`,
`rebuttal-19.md` and `rebuttal-20.md`). What batch 9 already landed, so
this batch builds on it and does not redo it: `EditorAction::{NewSong, OpenSong, CloseSong}`
(`editor_input.rs:98-104`, denied while Playing, dispatched in `shortcuts.rs:284-294`
through `request_document_action` → `perform_song_action`); the confirm machine's song arms
(`PendingSceneAction::{NewSong, OpenSong, CloseSong, StartPlay}`, `ConfirmDialog::unsaved_song`
and `play_with_unsaved_song`); `EditorGame.song`, `save_song(Option<PathBuf>)`, `new_song`,
`open_song`, `close_song` (`song_io.rs`); `SongMirror` and the two-mark title;
`SongDocument::{apply(edit, Gesture), undo, redo, break_merge, select_notes(&[u64]),
note_id, stage_sample, import, open, save}` with `Gesture::{Discrete, Continuing}`
(`song/mod.rs`, `song/history.rs`); the song's sidecar reads through `file()`, the notes
through `data().tracks()[i].units()` and the timing through `data().tempo_changes()` /
`time_signatures()` / `ticks_per_quarter()`, exactly as `command_api/song.rs` reads them.
The corrections to the first cut:

1. **No file picker exists, so the sidebar is the picker.** Batch 9 left out the native
   dialog, and Open Song today targets a placeholder (`song_path_to_open`: an `untitled.mid`
   beside the open song). The sidebar's song header gains a **Songs** list — every `.mid`
   under `music/` and `ai/music/`, found through `vfs::list_files` and rescanned when the
   panel opens and on the asset browser's Rescan — whose click runs
   `request_document_action(OpenSong(path))`; and a **path field** (the song's path, or the
   `music/untitled.mid` default for a never-saved song) whose Enter runs
   `save_song(Some(path))`, a refusal (outside the song folders, not a `.mid`) landing on the
   status bar. **A target whose `.mid` or `.mid.ron` exists and is not the song's own file
   is another song**: the save parks as `PendingSceneAction::OverwriteSong(path)` behind
   `ConfirmDialog::overwrite_song(path)` (Overwrite / Cancel — `alt_label: None`), and the
   API's `song save <path>` refuses it unless the line ends in `overwrite` (the
   `discard` precedent, its own word because it replaces a file, not edits); Cancel leaves
   the document's path and dirtiness alone. Only the path field and the API reach this
   prompt: the confirm dialog's own Save branch calls `save_song(None)`, which targets the
   song's own file or `default_song_path()`'s *free* name (`scene_io.rs:263-277`), so it
   never overwrites and never nests a second prompt — `default_song_path`'s probe covers
   both finals (today it asks for the `.mid` only, `scene_io.rs:266-279`, so an orphan
   `untitled.mid.ron` would have been overwritten); "inside the song folders" is anywhere
   under `music/` or `ai/music/`, subfolders included, the same rule for the list and the
   field. **The Overwrite arm saves through an internal `save_song_to(path)` that skips the
   existence check** (as `resolve_song_choice` bypasses the dirty gate), so one click
   completes the save; its test is "Overwrite replaces the file, closes the prompt, one
   Song saved". The field is Save As's input only:
   File → Save Song saves to the document's path whatever the field holds, and the field
   shows the document's path again when it loses focus. Each Songs row shows its
   folder-qualified path (`music/theme.mid`, `ai/music/theme.mid`), never a bare name; the
   open song's own row is marked "open" and is not a click target.
   File → New Song and Open Song… (and a `.mid` drop) make the panel visible and focus it,
   so the composer is found the first time a song exists. The Songs list covers both home
   folders;
   opening and saving under `ai/music/` is batch 9's behaviour and license-agnostic here
   (#183's "unscanned" is the provenance gate's script, not this list). A missing folder is
   an empty list (`vfs::list_files` is `NotFound` on a missing directory natively,
   `vfs/mod.rs:162`; map it as `VfsStore::list_files` does, `song/io.rs:114`); the scan
   lives in `ComposerState.songs` and runs on panel open, on Rescan and after every song
   save, new and open, never per frame.
   File gains New Song, Open Song… (focuses the composer and its Songs list),
   Save Song, Save Song As… (focuses the path field), Close Song; the labels map in
   `menu/actions.rs` (36 lines) and the two new actions are `EditorAction::{SaveSong,
   SaveSongAs}`. `song_path_to_open` and its test go with the placeholder they served;
   `OpenSong` becomes the focus action. No new chords for the song verbs — the menu is
   enough, and the chord table (Ctrl+Shift only, `editor_input.rs:7`) is crowded;
   `FocusComposer` is Ctrl+Shift+M, which nothing binds today.
2. **The drop site needs two asset kinds.** The asset browser lists no `.mid` or `.sf2`
   (`asset_browser.rs:99`: `kind_for_extension("mid")` is `None`, pinned at :235), so nothing
   can be dragged from it. `AssetKind` gains `Song` (`.mid`) and `SoundFont` (`.sf2`), each
   with a tile (`panel_renderer/asset_browser/tile.rs` arms the payload) and
   `DragPayload::{Song, SoundFont}` (`drag_drop.rs:18`). Dropped on the composer: a
   `Sound` payload whose path ends in `.wav` adds
   `Instrument { name: <stem>, source: Sample { path, root_note: 60, gain: 1.0, OneShot },
   provenance: None }` — the path is the asset's own (Save copies it into `<stem>/`,
   `plan_save` reads it from the asset base; `stage_sample` is for bytes from outside the
   asset folder, batch 11's file input); `.mp3`/`.ogg`/`.flac` are refused on the status bar
   naming WAV as the pitched-sample format (a tooltip needs a hover the drop does not
   give); a `SoundFont` payload adds `Instrument { SoundFont { path, bank: 0, preset: 0 } }`;
   a `Song` payload runs `request_document_action(OpenSong(path))`. A dropped sample's or
   SoundFont's instrument name is the file stem, suffixed `_2`, `_3`… while the song
   already has that name (`check_schema` refuses repeated names, so the drop may not create
   one).
3. **The instrument block edits what the source holds.** `Instrument.provenance` (author,
   license) is required of a `Sample` at Save (batch 9), so a Sample's block shows path,
   root note, gain, one-shot, author and license, with a "needed before Save" mark while
   author or license is empty; a SoundFont's block shows path, bank and preset (numeric
   fields, the sidecar's ranges — a dropped `.sf2` is not trapped on preset 0). Every
   instrument edit is one `SetInstrument` carrying the whole instrument, and every part,
   instrument and binding block carries a remove button (`RemovePart`, `RemoveInstrument`,
   `RemoveBinding` exist), so an accidental drop never traps a save. The song's own
   provenance and the shared `music/LICENSE` are #183, batch 11's.
4. **Selection by id, and the active track.** `SongDocument::select_notes` takes note
   numbers (the API's vocabulary); the roll holds `NoteId`s, and `SongSelection::notes()`
   yields `NoteId`s (`selection.rs:20`), so the document gains
   `select(impl IntoIterator<Item = NoteId>)` beside it — both replace the selection; a
   Ctrl+click toggles by reading the selection *as it was before the press* (a press with
   Ctrl held never runs the plain click-select) and selecting the new set; Ctrl+A selects
   every note of the active track. **The roll and the list show one track:**
   `ComposerState.active_track`, the first track holding a note when a song opens (else the
   first track that is not the tempo track; a tempo-only file has none — the roll says
   "this song has no note track" and Enter/Insert are refused on the status bar), chosen
   from a selector in the transport ("Track 1 · melody", cycling over the tracks whose
   `channel()` is `Some` — the timing track of a type-1 file has none and is never
   selectable, and a note never lands on it) and by clicking a part in the sidebar (the
   track it resolves to); **switching the active track clears the selection**, so no key
   ever edits a note the roll is not showing, and is refused while a drag is live;
   Enter/Insert add to it. A format-1 file whose track 0 holds only timing therefore never opens on a blank
   roll.
5. **The dock.** `PanelId::COMPOSER = PanelId(5)` (CONSOLE is 4, `dock/mod.rs:32-40`);
   `DockPanel::new(COMPOSER, "Composer", Bottom).with_size(260.0).with_min_size(200.0)
   .with_hint(…)` with `visible` set `false` after construction (there is no builder for it);
   the first layout pass takes panels in order, so the composer stacks above Assets on the
   bottom edge (`dock/mod.rs:427-436`), and with both open at the 720 px minimum the scene
   view keeps about 200 px — collapsing Assets is the user's move, the View menu the way.
   `panel_id_for_menu_label` gains `"Composer"` (so View → Composer is a `TogglePanel`),
   `apply_panels` skips an id with no pref so an old `editor_prefs.json` leaves it hidden,
   `capture_panels` persists it.
6. **Placements by size.** `context/mod.rs` is 558 lines: `ComposerState` (focus, scroll
   states, zoom, the drag in flight, the songs list) lives in `composer/state.rs` and the
   context gains one field; `panel_renderer/mod.rs` (524) gains one dispatch arm;
   `shortcuts.rs` (493) gains the `KeyRoute::Composer` variant and one call into
   `editor_game/composer_keys.rs`, which holds the key table; `editor_game/mod.rs` (483)
   the one-line branch; `editor_input.rs` (478) the five variants and their deny rows.
7. **Keys, and the play session.** `route_editor_key(key, keyboard_owned, modifiers)`
   (`shortcuts.rs:35`) checks the dialog, then `keyboard_owned`, then resolves the chord
   table and intercepts the play controls (:58-93); the composer check sits after those
   interceptions and before the Playing forward: a `ComposerFocus` other than `None`
   returns `KeyRoute::Composer` and the composer's table decides, Escape first (it is also
   the first step of `cancel_cascade`). **The composer is read-only for the whole play
   session**, Playing and Paused, as the inspector's live mode is: entering a session
   clears `ComposerFocus` and drops any drag, a click in the panel takes no focus while
   one runs, and every mutation — a key, a drag frame, a sidebar field, a drop, undo and
   redo — passes one `composer_write_refusal` (the host's `song_change_refusal`,
   `song_io.rs:21`) before it reaches `SongDocument`, drawing its fields the read-only way.
   So Stop and the play keys always reach the controls, and keys reach the game. **Undo and
   redo belong to the focused document** (#141's rule): Ctrl+Z, Ctrl+Shift+Z, Ctrl+Y and
   Edit → Undo/Redo call the song's `undo`/`redo` while the composer holds focus and the
   scene's otherwise, the status bar naming which ("Undid MoveNotes (song)"); the chords
   are rows of the composer's table, not a fall-through to the scene's. **Focus is
   explicit:** `ComposerFocus` is cleared by a click outside the composer panel, by hiding
   or collapsing the panel, by Reset Layout, by session entry and by Escape — each of
   which also drops a live drag, the drag's owner being `EditorGame`, not the panel's
   render; a menu click does not clear it (the menu is chrome), so Edit → Undo after a
   click in the roll undoes the song and after a click in the scene undoes the scene. A
   composer text field (the path, an author, a license) owns the keyboard like any field:
   Ctrl+Z in it does nothing, as in the inspector, and Edit → Undo blurs it and undoes the
   song. `FocusComposer` is denied while Playing with the rest (the panel is read-only for
   the session, so focus would buy nothing and would take the arrows from the game). Undo, redo and every history
   operation are refused while a roll drag is live ("Finish or Escape the drag first", the
   scene's `refuse_during_drag` precedent, `shortcuts.rs:132`), and `drain_api_requests`
   skips the frame while a composer drag is live as it does for a gizmo drag
   (`api.rs:231`), so no edit lands between a drag's frames. Space is `Pan`
   (`KeyAnyMods`, `editor_input.rs:291`) everywhere else; with the composer focused it is
   the preview toggle. Tab/Shift+Tab: the ui's traversal runs only for a focused text field
   (`ui/src/context/traversal_tests.rs`), so with none focused the composer owns Tab.
8. **The transport before the audio.** Space and the transport's Play/Stop raise
   `EditorAction::SongPlayStop`, and a binding row's Trigger button raises a trigger; both
   go through the hosted `song play|stop|trigger` path, which refuses them until batch 11
   (`run_hosted_song_write`, "the composer's preview transport arrives in Conductor batch
   11"); the transport draws Play, Stop and Trigger disabled with that sentence as their
   tooltip and the playhead stays at tick 0. Batch 11 enables them and nothing else moves.
9. **Geometry sources.** `TempoMap` and `TimeSignature` are no-synth exports
   (`music_midi/src/lib.rs:86`, `tempo_map.rs:63,136`); `beat_lines` takes the song's whole
   time-signature list (a song may change meter; `edit_tests.rs:368` has two), so every bar
   line after a change lands on its own tick; `grid_segments` (`grid.rs:157`) is the LOD
   model for it; `ScrollState` (`scroll.rs:36`) the two scroll states;
   the roll draws with `ui.rect`, `rect_border` and `line` (there is no `lines` plural;
   `ui/src/context/mod.rs:326-351`) under `push_clip_rect` (:359), as `render_grid_overlay`
   does. `focus_ring` exists (`theme/mod.rs:32`, guarded by
   `test_focus_ring_reads_over_every_surface_and_reaches_the_widget`); the four new tokens
   join `test_roles_that_must_read_apart_do` (`theme/tests.rs:102`).
10. **A drag is a preview; release is the one entry; Escape drops the preview.** The
    gizmo's model (`gizmo_drag.rs`: capture on press, idempotent apply, one entry on
    release, restored verbatim on Escape), without touching the document until release:
    on press the drag captures the dragged notes' images; each frame computes the *total*
    tick and key delta from the press, clamped with checked arithmetic so every dragged
    note starts at tick ≥ 0, ends by `LAST_WRITABLE_TICK` (`music_midi`'s
    `ends_by_last_writable_tick`) and sits in key 0..=127 (the tightest note bounds the
    whole set, so a chord keeps its intervals); a commit the document still refuses clears
    the drag and reports on the status bar;
    and the roll draws those notes at the previewed position while the document is
    untouched; release applies one `SongEdit::Batch` — `MoveNotes { notes, tick_delta }`
    then one `SetPitch { notes_at_that_key, key }` per distinct starting key, in
    starting-key order — with `Gesture::Discrete`, or nothing when the total delta is
    zero; Escape drops the preview and applies nothing, so no history entry, no redo entry
    and nothing older is touched. No frame can be refused (the deltas are clamped), no
    history operation runs during the drag (correction 7), and no API line lands between
    its frames. Nudges (Shift+arrows) are the same batch per key press with
    `Gesture::Continuing`, merging while the key is held — so `try_merge` gains a `Batch`
    arm (two batches of equal length merge member by member, `None` if any pair does not)
    — and the key release calls `break_merge` (the arrow-nudge precedent,
    `game_impl.rs:138-149`).
11. **Screenshots for codex.** The editor has no screenshot facility. The report names
    three PNGs under `review/conductor/shots-10/` — `full.png` from the native editor over
    `examples/` (a bottom panel spans the window, and the native minimum is 1024 × 720, so
    native is always Full mode), `compact.png` and `tabs.png` from the playground in a
    browser narrowed under 720 and 480 px (the only place those modes are reachable) —
    taken by Jesse or the executor; the planner dispatches codex with `--image` for each.
12. **Carried out, not in:** #182 (one sample folder per stem) and #183 (the song's own
    `LICENSE` stanza, `assets/ai/music` unscanned by the gate) go to batch 11 with its save
    transaction.
13. **Composer state follows the document.** `EditorGame` bumps a `song_generation`
    counter in `new_song`, `open_song` and `close_song` (the GUI's and the API's paths
    alike); `ComposerState` keeps the generation it was built against and resets focus,
    drag, scroll, active track and selection-dependent state when they differ, so a drag in
    flight when `song open … discard` replaces the song is dropped without an undo and its
    note ids never reach the new document.
14. **Scroll.** Four `ScrollState`s (`scroll.rs:36`, one axis each): the roll's horizontal
    and vertical, the note list's vertical, the sidebar's vertical.
15. **Screenshots have a fallback.** If the three PNGs are not in `review/conductor/shots-10/`
    when the code round closes, the batch lands on the diff review and the screenshots
    become a follow-up issue for codex's UI pass; it does not wait.

Files (new): `crates/editor/src/composer/{mod.rs, state.rs, layout.rs, keys.rs,
piano_roll.rs, note_list.rs, sidebar.rs, transport.rs}` + tests,
`crates/editor_integration/src/panel_renderer/composer.rs` + `composer_drag.rs`,
`crates/editor_integration/src/editor_game/composer_keys.rs` + tests, theme tokens
`note_fill`, `note_selected_border`, `key_black`, `playhead` with contrast guard tests.
Touched: `editor/src/dock/mod.rs` (`PanelId::COMPOSER`, the label), `context/mod.rs` (the
panel, the one field), `menu/mod.rs` + `menu/actions.rs` (View → Composer; File → New Song /
Open Song… / Save Song / Save Song As… / Close Song), `editor_input.rs`
(`EditorAction::{SaveSong, SaveSongAs, FocusComposer, SongPlayStop}`, all
four denied while Playing), `editor_game/shortcuts.rs` (`KeyRoute::Composer`,
Escape first in the cancel cascade, key-release seals a nudge), `editor_game/mod.rs` (the
dispatch branch), `asset_browser.rs` + `drag_drop.rs` + `panel_renderer/asset_browser/tile.rs`
(correction 2), `scene_io.rs` + `song_io_tests.rs` (`song_path_to_open` removed),
`song/mod.rs` + `song/edit.rs` (`select(ids)`, the `Batch` merge arm — corrections 4 and
10), `scene_confirm.rs` + `confirm_dialog.rs` (`OverwriteSong`, `overwrite_song`),
`song_io.rs` + `command_api/{mod,song,specs}.rs` + `docs/EDITOR_COMMAND_API.md`
(`HostedWrite::SongSave { path, overwrite }` and the word — correction 1; `song_generation`
— correction 13), `crates/editor/CLAUDE.md` and `crates/editor_integration/CLAUDE.md`
(file maps, pitfall rows).

**Target shape.** `layout::compute(bounds, state) -> ComposerLayout { transport, sidebar,
note_list, roll: PianoRollGeometry, mode: Full | Compact (< 720 px: sidebar as tabs) | Tabs
(< 480 px: one area at a time) }`; `PianoRollGeometry` with `tick_to_x`/`x_to_tick`/
`pitch_to_y`/`y_to_pitch`, `note_rect`, `beat_lines(TempoMap, TimeSignature)` at the
`grid_segments` LOD; the four scroll states (correction 14); the active track (correction
4). Focus model: `ComposerFocus { None, Transport,
PianoRoll, NoteList, Sidebar }`, entered by click or Ctrl+Shift+M; Tab/Shift+Tab cycle areas;
arrows move cursor/row, Shift+arrows nudge selected notes (merging per hold), Enter/Insert
add, Delete delete, `[`/`]` resize, `+`/`-` velocity, Space preview (refused until batch 11),
Escape clears focus. While the composer holds focus the single-key editor toggles are
unreachable — intentional and documented. Visible focus: a 2 px `focus_ring` on the focused
area and on the focused note or row; selection is never colour-only (thicker border plus a
glyph; rows get the accent fill plus a leading "▸"). The sidebar shows the song header (the
path field, the Songs list, dirty mark), parts (with the "unassigned background" badge and
the bindings using each), instruments (root/gain/one-shot fields, author and license, the
duration-changes-with-pitch note beside the root field), bindings (name → part +
instrument, a Trigger button each, disabled until batch 11), and the mode caveat text for
`Loop`. The drop site is correction 2. A drag is correction 10. **Tests:** geometry round
trips and LOD over a melody and over `new_song()` (finite, monotonic, nothing to divide by),
mode thresholds, the key map table including Tab cycle, Escape and the undo chords, a
30-frame diagonal drag yields one entry and Escape undoes it (and an unmoved drag's Escape
undoes nothing; a move after Escape applies nothing; a diagonal drag across two starting
keys), Undo while focused undoes the song and unfocused the scene with pending edits in both,
Playing and Paused refuse every composer mutation and clear the focus so keys reach the
game, Save As onto another song (either final present) parks behind the overwrite prompt
and Cancel leaves both untouched while the API needs `overwrite`, Enter in the path field
parks the prompt and the same Enter does not arm the dialog's `pending_choice`, a song
replaced under a drag drops it, a failed open after Discard keeps the old song with the
error on the status bar, the active track defaults to the first with notes and Enter adds
to it and switching it clears the selection, a tempo-only file refuses adds, a drag's
preview leaves the document untouched until release and a clamped chord keeps its
intervals, Ctrl+Z during a drag is refused, Ctrl+click adds to a prior selection, a drop
of a second `kick.wav` is named `kick_2`, two home folders' same-named songs list apart,
bar lines follow a meter change, a click in the scene then Edit → Undo undoes the scene,
an orphan `untitled.mid.ron` moves the default save path on, Overwrite completes in one
click, hiding the panel mid-drag drops the drag and frees undo and the API, a track switch
is refused mid-drag, a drag near `LAST_WRITABLE_TICK` clamps and commits,
selection marker present in the draw list (as
`menu/tests.rs:36-49` reads `ui.draw_list().commands()`), tokens pass contrast, composer
keys never reach the game while focused and do once unfocused, the Songs list finds a song
in each home folder and its click parks behind the dirty prompt, the path field's Enter
saves there and an outside path is refused on the status bar, each drop kind lands its
instrument or its open and a `.mp3` is refused, a Sample without provenance shows the
mark and `SetInstrument` carries it, View → Composer toggles the hidden panel and an old
prefs file leaves it hidden, the preview controls refuse naming batch 11. **Gates:**
standard; `check_wasm.sh`; the three screenshots (correction 11). **Leaves out:** audio
(next), CC and pitch-bend lanes (preserved opaquely only), #182 and #183 (batch 11), a
native file dialog.

## Batch 11 — K8c: transport, Play interplay, the web bridge, the accessibility pass (#141) — DONE 2026-10-22 (574e7e2; site 5375ebc)

**DONE 2026-10-22** (574e7e2 in the engine, 5375ebc in `insiculous_web`). Landed as planned by
the Opus executor (`report-11.md`, +5182/−1085 before fixes), with the differences it declared:
`PreviewState` has no `Paused` (the session's pause is `PauseSession`); `PlaySaved` is not an
intent — the armed `PendingPlay` carries the path and the drain runs it last; the preview stops
on a document change by watching `SongHistory::revision()` at the drain, not by hooks in
`apply`/`undo`/`redo`, which also catches the API's `song edit|undo|redo`; the playhead waits for
the song's first rendered block; the audio test starts at tick 3072 (the fixture is 16 beats); a
song's upload limit is `MIDI_BYTE_LIMIT` (1 MiB), the sidecar's 256 KiB being another file's;
`playground_write_file_bytes(name, bytes, replace)` decides the destination itself and answers
`{written, kind, destination, referenced_by, sidecar_lost}`; the open-song refusal rides a
`Hooks.open_song` mirror `sync_dirty_mirror` writes; a dropped `.new`'s note goes to the console;
the playground takes `music_midi` with its default features (the `.sf2` check is `synth`-only and
`audio` already turns it on). The grandfather clause covers a song directly in either home; the
home `LICENSE` is written only when its text changed; a stanza naming only this song's files is
replaced whole. The executor's native keyboard walk (XTEST on the session's display) found and
fixed, at Jesse's call and each with a guard test: sidebar notes wrapping, the arrows' reveal
inset, a provenance field committed unchanged being no edit, the transport disabled during a
session, the File menu's shortcut placement, the status bar keeping a long message whole, Play
from a cursor past the last note starting at the top, Left/Right stepping the full-width
sidebar's sections. Not heard by the executor: sound is inferred from the log and the playhead.

Code round 29 (kimi 3, gemini 5, codex 5, the planner 8; `rebuttal-29.md`) found the store's
queue cancelling a queued removal (the FIFO's one guarantee broken), the drain waiting for the
head alone (a switch right after a save lost the rest), a restored epoch issuing a head nobody
drove on wasm, a tombstone not unseating a preloaded bundled file, `StopAll` leaving a recorded
track, the page claiming a song opened whatever `song open` answered; fixed by the planner and
reviewed again as round 30 (codex 1, kimi 3, gemini 5; `rebuttal-30.md`), whose two accepted
findings — the drain not driving its own retry of a stranded head, and a head stranded under the
drain ending it as a success — landed enumerated. **The drain now refuses a switch over a
stranded head, naming the path** ("… is not saved to this browser — export your project, then
try again"): a change to the old "a stranded path never blocks a switch" contract, made because
the queue behind the head would go with the page; a conflicted head still never blocks (its
banner asks for the reload). Jesse may overrule it. Rulings held: (a) Ctrl+S saves the song while
the composer holds the keyboard, (b) provenance is required at Save (gemini re-raised it as a
block on legacy songs; rebutted), (d) subfolder and `ai/music` songs get new sample folders.
Gates in `gates-11.log` and `gates-11-fixes-3.log`; bundle 12.22 MiB in the executor's build
against 16 MiB. Filed: #186 (Stop while the session's song waits for the first click opens the
dialog over a running session), #187 (the composer reaches no accessibility tree), #188 (no
control adds a binding), #189 (the Firefox save-time measurement), #190 (tombstone compaction),
#191 (the playground's browser-only checks), #192 (Exit prompting); #112 and #184 commented.
Jesse's native listen check passed 2026-10-22; #141 stays open for #184's screenshots and
the K5 scene (batch 12).

Executor: the Opus session. Reviewers: kimi, gemini (the bridge and persistence seam), codex
(the transport and the keyboard walk). Listen check and keyboard walk: Jesse, native and
browser. Size: about +3400 lines (up from +2500 on re-verification: #182 and #183 absorbed,
the store's FIFO and removals, the file input, the two audio hunks), plus a small
`insiculous_web` half reviewed from `insiculous_web/review/conductor/`.

**Rulings (2026-10-22):** (a) **Ctrl+S saves the song while the composer holds the keyboard —
accepted by Jesse** ("Ctrl+S being a save hotkey, that's functionality I approve"; correction
8); (b) a song's own provenance is required at Save, legacy songs included (correction 7) and
(d) a song saved in a subfolder or under `ai/music` gets a new sample folder while
`music/<stem>.mid` keeps today's (correction 6) stand as proposed — Jesse sent the section to
commit with them in it. The former ruling (c), the web's durable order, is withdrawn: round 25
made the store's operations one FIFO, so the durable order is the write order.

**Re-verified against the tree 2026-10-22, after batch 10 (9e045cc), and reviewed in rounds
24 to 28** (`review-24*.md` … `review-28*.md`, adjudicated in `rebuttal-24.md` …
`rebuttal-28.md`; settled by Jesse after the fifth round, 2026-10-22). What earlier batches already landed, so this batch builds on it and does
not redo it: the audio side of the preview is whole — `AudioManager::{prepare_song(label,
file, data) -> Arc<Song>, play_song_in_memory, play_song(path), stop_song, pause_song,
resume_song, seek_song(beat: u64), trigger(action, source), is_song_playing, song_position()
-> Option<SongPosition { bar, beat, beat_in_bar, seconds, latency_samples }>}`
(`audio/src/manager/song.rs:224-466`), reachable from `editor_integration` as
`engine_core::prelude::AudioManager`; `is_song_playing` false and `song_position` `None`
while a song waits for the web's first gesture (`song.rs:407-415`); the conductor drains
every queued command before a block renders (`conductor.rs:431-437`);
`SoundSources::flush(audio) -> u32` (`sound_sources.rs:128`); the hosted verbs `SongPlay {
from_beat: Option<f64> }`, `SongStop`, `SongSeek { beat: f64 }`, `SongTrigger { binding }`
parse and are refused by `preview_verb` (`song_io.rs:228`) with the sentence
`PREVIEW_REFUSAL` (`composer/mod.rs:54`, six uses in `editor`), the transport's Play and Stop
drawn disabled with it (`transport.rs:57-64`), `ComposerAction::PreviewRefused`,
`EditorAction::SongPlayStop` denied while Playing (`editor_input.rs:172`) and dispatched to
`refuse_preview` (`composer_keys.rs:320,394`), the sidebar's Trigger buttons drawn disabled
(`sidebar/blocks.rs:267`, the literal `false`) and its Sample block's author and license
fields with the "needed before Save" mark (`blocks.rs:231-234`); `position_text(tempo_map,
tick)` (`composer/mod.rs:197`) and `status_text` (`transport.rs:4`);
`TempoMap::tick_at_seconds` (`tempo_map.rs:174`); a song with no path is dirty
(`song/mod.rs:162-164`), so Play over a never-saved or edited song parks behind
`PendingSceneAction::StartPlay` (`scene_confirm.rs:34`) and a session only ever starts over a
saved song or none; `begin_play` reached from `handle_play_action` (`play_session.rs:280`)
and from the dialog's answer (`scene_confirm.rs:189,230`); `song_generation` bumped by
new/open/close; `SceneSnapshot { scene_entry, ron }` (`editor_game/snapshot.rs:20`),
`export_snapshot(root, manifest, scene_entry, scene_ron)` and `import_project`
(`playground/src/archive.rs:105,214`); a reload seeding MemFs from the store directly, never
through `import_project` (`web_entry.rs:203`); `validate_bridge_path` and
`playground_write_file` (`bridge.rs:55,148`); the command API's double-quoted tokens
(`parse.rs:32-46`); a `.mid` with no sidecar already opening as an import
(`open.rs:203-215`); the loader's `SOUND_FONT_BYTE_LIMIT` of 48 MiB (`load.rs`);
`GameHarness` driving `EditorGame` with a real disabled `AudioManager` on the virtual clock
(`play_session_tests.rs:317-355`: `harness.audio()`, `sound_sources()`, `context()`); the
history-only dirty flag already carrying the song (`history_dirty_tests.rs`, batch 9), so
`playground_is_dirty` needs no change and no new test; the idle throttle engaging only after
half a second with no input, on the web alone (`app_handler.rs:23,110,125`). Corrections
after reading the tree and two rounds:

1. **The preview is intents, drained once a frame.** New
   `editor_integration/src/editor_game/song_preview.rs` (+ `song_preview_tests.rs`):
   `SongPreview { state: PreviewState::{Stopped, Playing, Paused}, generation: u64,
   pending_play: Option<PendingPlay>, position_seen: bool, intents: Vec<SongAudioIntent> }`
   and `SongAudioIntent::{PlayDocument { from_tick: u64 }, Stop, Seek { beat: u64 }, Trigger
   { binding: String }, PlaySaved(PathBuf), PauseSession, ResumeSession, StopAll }`. The
   composer's Play, Stop, Space, the hosted `song play|stop|seek|trigger` and the Trigger
   buttons push intents through pure methods on `EditorGame` taking `&mut World` (the
   headless suite drives them); `apply_song_intents(ctx.audio, ctx.sources) -> DrainReport`
   drains the vector in `finish_frame`, after `update_inner_game`. `PlayDocument` is
   `prepare_song(label, song.file().clone(), song.data().clone())` then
   `play_song_in_memory_at(song, from_tick)` — **a new function in `crates/audio`**, the tick
   carried by the ready song so `start_song` sends `Seek(tick)` before `Play` whether the
   song starts now or after the web's first gesture (two sends from the editor could straddle
   a block boundary, and before the gesture `seek_song` is a no-op: `song_transport`,
   `song.rs:387`). The conductor cannot honour that today — `Play` seeks to tick zero
   (`conductor.rs:357-361`) and `Seek` while `Stopped` has no arm (`:381-391`) — so
   **`music_midi` gains one semantic**: `Seek` while `Stopped` records a start tick, `Play`
   starts there (zero when none) and clears it; a conductor test pins it, and its guide notes
   the change (the senders today — `seek_song`, `song.rs:372`, and the example's K key —
   both seek a playing song). The transport's
   Play starts at the roll cursor's tick (`RollCursor.tick`); the API's `from_beat` converts
   through `tick_at_beat`. `play_song_in_memory` becomes the `at(…, 0)` case, with a test in
   `audio` that a song started at tick 7680 publishes no position before it. A `Seek` while
   `song_position()` is still `None` (the pending window) re-issues `PlayDocument {
   from_tick }` at the beat's tick — the pending song replaced by itself at the new place;
   Pause and Resume in that window are refused on the status bar, as triggers are, because
   `pause_song`/`resume_song` are no-ops without a playback (`song.rs:387-399`) and the
   state would part from the sound. A
   `prepare_song` error goes to the status bar and the state stays `Stopped`. `seek_song`
   takes a whole beat, so the API's `from_beat`/`beat` is `Invalid` unless finite, whole and
   not negative — "whole beats counted from zero"; the spec row and
   `docs/EDITOR_COMMAND_API.md:180` say so (that row loses its "refused until batch 11").
   The preview's triggers use `ctx.sources.menu()`. **The playhead:** each frame while
   `Playing`, the host writes `ComposerState.playhead_tick: Option<u64>` from
   `tempo_map.tick_at_seconds((seconds - latency_samples as f64 /
   f64::from(SONG_SAMPLE_RATE)).max(0.0))` — both operands cast before the division, and
   `SONG_SAMPLE_RATE` becomes a `pub` of `audio` (the second audio hunk; it is `pub(crate)`
   today) and `AudioManager::song_ended() -> bool` (`SharedPosition::has_ended`,
   `position.rs:129`) the third; the web's device buffer beyond `latency_samples` is unknowable, so the acceptance
   row expects a small residual lead there — guarded by `if let`: while `song_position()` is `None` the status says "Playing ·
   waiting for the first click". The roll draws it as a vertical line in a new `playhead`
   theme token with two contrast rows (against the roll's background and against the selected
   note's fill); the status text says "Playing · bar 2 · beat 3 · 120 BPM · 4/4"; Stop clears
   it to `None` and `status_text` says tick 0 again. **The preview's state follows the
   song**: once `song_position()` has read `Some` in this preview (`position_seen`), an
   `is_song_playing()` of false ends the preview: `Stopped` with the playhead cleared and no
   stop command sent — silently when `song_ended()` is true, with "playback stopped early" on
   the status bar when it is not (a device glitch, a suspended context); the poll runs only
   while Editing; before
   the first `Some` the poll asks `audio.is_enabled()`: not enabled is the gesture-pending
   window and the preview stays `Playing`; enabled with no position means the output died
   (`song_output` returns `Gone`, `song.rs:310-322`) and the preview goes `Stopped` with
   "the song could not reach the device" on the status bar. The
   preview also stops — with a `Stop` pushed — when the song is closed, replaced or
   re-opened under it (`song_generation` moved) and on any mutation of the document —
   `SongDocument::apply`, and `undo`/`redo`, which apply their entries directly
   (`song/mod.rs:189,209`), through the host's `song_history_step`: the prepared score is the
   old document, and live mutation is out by #141; the status bar says
   "preview stopped by the edit" once. `set_idle_throttle_ok` (`editor_game/mod.rs:400`, not
   :380) gains `&& !self.song_preview.is_audible()` — `Playing` with `position_seen` — so a
   song waiting for the gesture does not hold the page at full rate; the pending-play frame
   follows a click and is never throttled. `PREVIEW_REFUSAL`,
   `ComposerAction::PreviewRefused`, `refuse_preview` and `preview_verb` are deleted with
   their greps; `ComposerAction::{PreviewPlayStop, PreviewTrigger(usize)}` replace the first,
   the Trigger buttons are enabled while the preview is audible (`position_seen`; a trigger
   on a gesture-pending song is a silent `Ok` the conductor never hears, `song.rs:467-475`)
   and refused on the status bar otherwise, Play and Stop are enabled always, Stop doing nothing while `Stopped`.
2. **Play with a song open plays the saved file; the editor stops only what it started.**
   Nothing in the editor's hosts plays a song today — `ProjectHost` has no song verb and a
   wrapped game's `init` ran at editor startup — so the session's music is the editor's to
   start. `begin_play` (`play_session.rs:302`) becomes the one choke point and **arms**
   `PendingPlay { song: Option<PathBuf> }` instead of starting the session; both its callers
   — `handle_play_action(Play)` and the dialog's `perform_song_action(StartPlay)`, the
   Save-then-play branch included — go through it. The request frame's drain flushes the
   sources and, with a song open (saved by construction: a pathless song is dirty and
   parks), runs `play_song(path)` with `SongDocument::path()` as is (already asset-joined):
   the file's whole closure is loaded and validated before the playing song is touched
   (`load.rs`), the one music slot means the saved song replaces the preview with no separate
   stop, and the load may take the time a first SoundFont decode takes (cached by change stamp
   after). The drain's report says whether that play succeeded: on failure `PendingPlay` is
   dropped, the preview kept and the error shown; on success — or with no song open, when
   Play touches no music and a wrapped game's own song plays on — the next frame's
   `prepare_frame` starts the session (`start_play_session`), so a game's first `update` sees
   the song playing. **While armed**, Stop drops `PendingPlay` and pushes `StopAll`; Pause is
   a no-op. Pause in a session pushes `PauseSession` (`pause_song`), Resume `ResumeSession`
   (`resume_song`); `stop_with_paused_edits` pushes `StopAll` after the restore, defined
   once: the flush, and `stop_song` only if this Play started a song — a wrapped game's own
   music is the game's; the preview comes back `Stopped` with its playhead at 0, and the
   user's transport Play hears the document again. `begin_play` parks the preview — `Stopped`,
   playhead cleared, no sound command, the slot being taken — and while `PendingPlay` is
   armed, preview intents pushed in that frame are dropped and the drain runs the session's
   play last, so the saved song — never the in-memory one — is what the session hears. **The
   slot's contract, stated:** with a song open in the composer the session's music is the
   editor's, and Stop stops the slot whatever a wrapped game did to it; with no song open Stop
   touches no music. A wrapped game that plays music in its `update` takes the one slot from
   the saved song — "Play hears the saved song" holds on `ProjectHost`, where the acceptance
   runs, and until a game's code touches music otherwise. The existing harness test
   `test_a_spawn_sound_is_silent_in_editing_plays_on_play_and_is_cleared_by_stop`
   (`play_session_tests.rs:317`) steps twice after Play — the spawn is the frame after Play's
   request — and its pitfall row says so. Scope of `play_session.rs` (324): the arm, the
   cancel and the three pushes; the transition's logic lives in `song_preview.rs`. The hosted
   preview verbs stay refused in Playing and Paused by `song_write_refusal`, as every song
   write is.
3. **The web import is a write, not a staging area.** `stage_sample` and `pending_samples`
   (`song/mod.rs:99,260`) have four sites — `plan_save` (`io.rs:235`), the `song` query's
   `"pending_samples"` key (`command_api/song.rs:191`), its doc example
   (`EDITOR_COMMAND_API.md:193`) and `io_tests.rs` — and no caller outside tests; all four
   go, the key leaving the payload. A `Sample` path is written by three things, each with
   its bytes on disk or refused: the asset-browser drop (an asset in the project), the
   sidebar's path field (a typed path, refused at Save by the existing "a sample that cannot
   be read" rule when wrong) and the web upload below (written before the instrument exists;
   a test uploads, assigns and saves in one session). The site's file input writes bytes
   into the project through `playground_write_file_bytes(path, bytes)` (`bridge.rs`, the
   `validate_bridge_path` guard; the name refused before anything is written unless it
   matches `[A-Za-z0-9._-]+` (no whitespace: the provenance gate splits `Files:` on it,
   `check_audio_provenance.sh:28-31`, and `place_sample` already writes `_` for it) and, for a
   `.mid`, `editor::song::saveable_song_path` (the one rule Save uses, correction 6), tested
   with a quote, a space, a non-ASCII letter and a `__`; the whole file refused by size before any decode — the loader's own limits, all `pub` in
   `audio` (`SONG_FILE_BYTE_LIMIT` 256 KiB, `SAMPLE_BYTE_LIMIT` 4 MiB, `SOUND_FONT_BYTE_LIMIT`
   48 MiB; `lib.rs:61,68`), the refusal naming the limit — then validated by the loader that
   will read it before anything is written, in a new `playground/src/upload.rs` — a `.mid` by `SongData::from_smf`, a `.wav`
   by `audio::decode_sample_pcm` (`audio/src/lib.rs:68`; `audio` joins the playground's
   dependencies, a workspace path as `music_midi` does), an `.sf2` by `music_midi::parse_sound_font`, anything else refused naming the three, an empty, a three-byte and a header-only file of each kind tested — and the destination
   fixed by kind: `assets/music/<name>` for a song, `assets/music/imports/<name>` for a
   sample, `assets/soundfonts/<name>` for a SoundFont), persisted by the vfs observer like
   every write. A taken name is replaced after the page's confirm, the pattern of the zip
   import's discard prompt (`playground-embed.ts:413`); the confirm names every song whose
   sidecar references the path (`playground_sample_references(path)`, a scan of `*.mid.ron`
   under the two home folders with the recursive `vfs::list_files`) and, for a `.mid` with a
   sidecar beside it, says that song's bindings and instruments will be lost; a replaced
   `.mid` is refused while that path is the open document (the confirm says to close the song
   first); otherwise the bridge call removes its `.mid.ron` and its `.new` pair **first** and
   writes the MIDI **last**, so every durable prefix opens — the old MIDI as an import, then
   the new. `insiculous_web` reads no `pending_samples` key (grepped). The page then dispatches
   `song open "music/<name>"` — quoted, the tokenizer's rule — for a song, and announces a
   sample or SoundFont as imported and reachable from Assets after Rescan. The `_unsaved`
   sentence of decision 12 goes; `SONG_FORMAT.md` § How the composer saves step 2 loses "A
   song never saved holds its samples itself". `bridge.rs` is 541: the two exports are its
   only new lines. On the site: `src/components/PlaygroundToolbar.astro` gains the labelled
   input (`accept=".mid,.wav,.sf2"`) beside the zip import, the handler in a new
   `src/scripts/playground-import-file.ts` (the pattern of `playground-scripts-panel.ts`;
   `playground-embed.ts` is 529 and gains the interface lines and the wiring only), one store
   mutation at a time as the zip import does.
4. **The web persists in write order and removes what a save removes.** `ProjectStore` has
   no per-file delete (`store.rs:58-90`) and `vfs::remove_prefix` on wasm drops MemFs keys
   silently (`vfs/mod.rs:202`; the observer is write-only), so today the `.new` pair and
   every pruned sample stay in IndexedDB until the project is replaced; and the chains run
   one put per path concurrently, so a save's pair and finals land in no defined order. In
   place of the `SongSaveTransaction`, two changes to `Chains`. **One operation at a time, in
   write order:** `Chains` keeps a FIFO of store operations — puts and removals — and issues
   the next when the previous completes. **No coalescing:** every write is its own entry
   (a path written twice is two puts, in order; `Queued` now means "more entries behind this
   path's in-flight one"), so the durable sequence at any completion is a prefix of the
   write sequence — the states a native crash leaves, which the open-time rules (batch 9's correction 8:
   `open.rs`, `SONG_FORMAT.md` § How the composer saves) were written for. A path written behind its own *pending* removal cancels the removal; a
   write behind an *issued* removal waits for its result; the chain reads its base revision when an
   operation **issues**, never when it is enqueued.
   **Failure is head-of-line:** an operation that fails (Backend/Unavailable) strands the
   head, nothing behind it issues, `is_pending()` stays true, `save_status` reads Failed
   naming the path, and the head is retried on `visibilitychange` as a Stranded put is today
   (`reissue_stranded`) **and on the next enqueue** — a new write is evidence the tab is
   alive — the page's Save button (`playground_dispatch('save')`, `playground-embed.ts:301`)
   being the manual retry the Failed reason names — so a save whose final failed never reaches its removals, and a
   failed removal is neither dropped nor waited on in silence. A Conflicted head blocks the
   FIFO too: the existing banner says to export and reload. A project replace awaits `drain_then_epoch` — the FIFO's drain lets the head finish and
   issues nothing more (`persist/mod.rs:501-515`, `bridge.rs:411`) — then navigates to the
   imported project (`playground-embed.ts:428`), and the boot builds a fresh `Chains`, so no
   queue survives it. Saves serialise more round trips than today's concurrent puts; the
   report measures a sample-heavy save's indicator time in Firefox. Tombstones grow only with
   distinct pruned names (the journal pair's paths are reused by every save); a compaction
   pass is a filed follow-up. **Removals:**
   `ProjectStore::remove_file(slug, path, base_revision) -> Fut<Result<(), StoreError>>` on
   the trait and its three stores (`directory.rs`, `memory.rs`, `indexed_db/mod.rs`): **a
   tombstone, not a delete** — the record stays with `deleted: true`, empty bytes and
   revision + 1 (`StoredFile` gains `deleted`, serde default false), `load_project` returns tombstones too — `seed` records their revisions and `web_entry` inserts
   no tombstone's bytes into MemFs — and a put over one needs its revision like any put, so revisions stay monotonic
   across delete and recreate and a stale tab's removal is stale by revision; base revision
   zero on an absent record is `Ok`; a newer revision — another tab rewrote the file — is
   `StaleRevision`, on which the removal is **dropped with a log line** and the FIFO moves on,
   the other tab's write winning, never Conflicted over a journal file; `common::vfs::set_remove_observer(fn(&Path))`, the write
   observer's twin, fired once per dropped key by `MemFs::remove_prefix` on wasm and by
   nothing natively; `Chains::on_vfs_remove(path)` in a new `persist/queue.rs` — the FIFO and the removals
   both, `persist/mod.rs` (586) gaining only the calls — enqueuing the removal — nothing for a path with no chain, one never loaded or written
   (`seed` gives every loaded file a chain, `persist/mod.rs:131-137`); `is_pending()` is true while any operation is queued, in flight or
   stranded at the head, so the indicator says saving and the unload prompt fires. `mark_saved` fires as
   natively — the bytes are in MemFs, which is what Export reads. `WEB_PLAYGROUND.md`
   § Persistence gains the FIFO sentence. **Tests** (`persist/tests/chains.rs`, hand polled):
   two paths written in order land in that order and never overlap in flight; a path
   written twice lands twice in order; a removal behind a put waits for it and lands as a
   tombstone one revision up, which a later put over the path must name; a write behind a pending removal cancels it; a write
   behind an issued removal waits and lands at revision one; a removal of a never-persisted
   path enqueues nothing and of an absent record is `Ok`; a removal from a tab that loaded an
   older revision is dropped and the recreated file survives; a stranded head retries on the
   next enqueue; save, re-seed, save lands (the journal paths' tombstones carry their
   revisions); remove, re-seed, recreate lands; a stale removal is dropped and the
   FIFO moves on; a failed final strands the head, its removals never issue, `save_status`
   is Failed, and the retry lands the rest in order; a re-seed from the memory store after
   each completed operation of two overlapping saves — the second arriving while the first
   pair's MIDI is in flight — opens a song the open-time rules accept, the latest whose pair
   landed; a replaced upload's three operations in order leave an openable project at every
   boundary.
5. **Export and import.** `SceneSnapshot` gains `live_files: Vec<(String, Vec<u8>)>`: the
   open song's MIDI bytes and sidecar text at its archive path (`default_song_path` when
   never saved), rendered by a `SongDocument::rendered_files()` lifted from `plan_save`
   (`io.rs:256-264`) so the two never diverge — the bytes as the document holds them, no
   provenance refusal and no sample relocation, the fingerprint set; the snapshot still
   leaves history, path and the saved mark alone. `export_snapshot` takes the live files,
   splices each over the stored entry or adds it, and drops **only the open song's own
   `.new` pair** (its live files replace them); every other `.new` travels, so an
   interrupted save of another song keeps its recovery in the backup. The README the archive
   writes says the live scene and the live song travel. `import_project` step 14: every
   `*.mid.ron` parses as `SongFile` (`music_midi` joins `playground/Cargo.toml` as the editor
   has it, `default-features = false`), every `*.mid` opens with `MThd`, and a **final**
   sidecar without its `.mid` is `ArchiveError::InvalidSong(String)` — the archive refused
   whole, as a bad scene refuses it today (§ Failure contract); `*.mid.new` and `*.mid.ron.new` are checked the same way but a failure **drops the file
   with a note** instead of refusing the archive — a journal leftover is recovery data, not
   the project — paired or not, and the survivors are settled by the open-time rules when the
   song is next opened. `docs/WEB_PLAYGROUND.md`
   § Validation rows 14–15, § Export the live-song sentence, § Acceptance three composer
   rows (the preview plays after the first gesture and never before; Play hears the saved
   song and Stop returns the preview; a `.wav` imported, auditioned, saved, exported and
   re-imported plays).
6. **#182 — one sample folder per song, flat, in the song's home.** The sample directory
   is `<home>/samples/<flat>/`, where `<home>` is the song's home directory (`music` or
   `ai/music`) and `<flat>` is its path relative to that home, extension dropped, every `/`
   replaced by `__`: `music/theme.mid` → `music/samples/theme/` (today's layout, unchanged),
   `ai/music/theme.mid` → `ai/music/samples/theme/`, `music/a/theme.mid` →
   `music/samples/a__theme/`, `music/ai/theme.mid` → `music/samples/ai__theme/` — injective
   within a home, and no song's folder can contain another's (`plan_save`,
   `io.rs:226,253`; `SAMPLE_DIRECTORY` becomes a function of the home). **Ownership is
   written:** the **sample folder's** `LICENSE` header carries `Source: <the song's path
   relative to the asset base>`; a save into a folder whose header names another song is
   refused naming both songs; a folder with no header (the legacy layout) is adopted only
   when its name is the song's own stem and refused otherwise. **The path rule**, one `pub fn
   saveable_song_path(path) -> Result<(), String>` in `song/io.rs` that Save and the web
   upload both call: no component of the song's path may start or end with `_`, no `__` and no whitespace
   may appear, and the path may not lie under `<home>/samples/` (a song there would sit inside
   another song's prune, which lists recursively) — then `__` is an unambiguous separator (`music/a_/b.mid` and
   `music/a/_b.mid` are refused, not merged), and the gate can name the files; the refusal
   names Save As to a compliant path as the way out. A flat `music/<stem>.mid` is
   grandfathered whatever its stem holds short of whitespace (the gate cannot name a spaced
   file; Save As is the way out), its folder being stem-keyed with no encoding involved; the
   web upload holds new files to the full rule, which `SONG_FORMAT.md` says — and the collision that leaves is stated: a new song whose encoded name equals
   an existing folder it does not own (`music/x/y.mid` beside a legacy `music/x__y.mid`) is
   refused naming the owner, Save As elsewhere the way out, tested in both save orders. *Ruling (d):* songs in subfolders and under `ai/music` move to new
   folders on their next save, the old folder left behind with its `LICENSE` still naming its
   files; `music/<stem>.mid` keeps its folder. `SONG_FORMAT.md` step 2 says so; tests save
   `music/theme.mid`, `ai/music/theme.mid` and `music/ai/theme.mid` with one sample each and
   reopen all three with every sample on disk, a save into another song's folder is refused,
   a legacy folder is adopted by its own song and refused to another, `music/a_/b.mid` is
   refused by the path rule and a flat `music/x__y.mid` is not.
7. **#183 — the song names its own files.** `SongFile.provenance: Option<Provenance>`
   (`#[serde(default)]`, the sample's type at `song_file.rs:168`; the schema and the
   grammar in `SONG_FORMAT.md`), surfaced in the sidebar's Song section as author and license
   fields with the "needed before Save" mark, shown from the moment a song without it opens;
   the fields are one `SongEdit::SetProvenance { provenance }` through `apply`, so `song
   edit` sets it over the API, undo covers it, and `EDITOR_COMMAND_API.md` names the edit
   beside the refusal.
   *Ruling (b):* required at Save like a sample's — a song saved before this field exists
   refuses its next save until both are filled; when that refusal answers the Play dialog's
   Save, the error shows, the composer's Song section opens and the author field is focused
   (the Save As pattern), so the way out is on screen. Save maintains the **home folder's** shared
   `LICENSE` (`music/LICENSE` or `ai/music/LICENSE`, the one the gate reads for the song's own
   two files — not the sample folder's of correction 6) stanza by stanza — the stanza matched by the song's `.mid` path alone (a legacy
   one-file stanza matches too) and rewritten to name both files, never another song's — in
   `io/samples.rs`'s idiom; `SONG_FORMAT.md` step 4 loses "which a human writes".
   `check_audio_provenance.sh` scans `*/assets/ai/music` too (which covers
   `ai/music/samples`). Tests: two songs saved into one folder keep each other's stanzas; a
   legacy one-file stanza is rewritten, not duplicated; a saved song's folder passes the
   gate's rule (the stanza text pinned against the gate's `Files:` grammar).
8. **Ctrl+S follows the focused document — *ruling (a)*.** Batch 10 sent every Ctrl chord
   the composer's table lacks to the scene. Undo already belongs to the focused document
   (#141); Save should too: while the composer holds the keyboard — the ownership undo uses, `composer_key_route`
   answering `KeyRoute::Composer` under batch 10's `settle_composer` rules; a focused text
   field anywhere owns Ctrl+S as it owns every key today — Ctrl+S is `SaveSong` and
   Ctrl+Shift+S is `SaveSongAs` (the path field focused), both through
   `dispatch_song_action`; the status bar names the document saved ("Song saved ·
   music/theme.mid"); a menu click keeps the rule the menu names. If Jesse keeps batch
   10's rule instead, this correction is struck and nothing else moves.
9. **Sizes and placements.** `composer_keys.rs` 542, `bridge.rs` 541, `shortcuts.rs` 509,
   `editor_game/mod.rs` 502, `persist/mod.rs` 586, `playground-embed.ts` 529: each gains a
   handful of lines at most, every new thing goes where the corrections above put it
   (`song_preview.rs`, `upload.rs`, `persist/queue.rs`, `playground-import-file.ts`).
   `crates/music_midi` is touched for the schema field, `SetProvenance`'s data and the
   conductor's start tick only; `crates/audio` for the three hunks correction 1 names and nothing else (the playground gains it as a dependency, for `decode_sample_pcm`); `crates/engine_core` is not touched; `common::vfs` gains the remove
   observer.

Files (new): `editor_integration/src/editor_game/song_preview.rs` + tests,
`playground/src/upload.rs`, `playground/src/persist/queue.rs`, the site's
`src/scripts/playground-import-file.ts`. Touched: `play_session.rs`, `editor_game/mod.rs`
(the field, the drain call, the throttle), `composer_keys.rs` (the actions' dispatch),
`song_io.rs` (the hosted preview verbs answered), `scene_confirm.rs` (nothing but what
`begin_play`'s new meaning forces), `editor_game/snapshot.rs`, `song/mod.rs`, `song/io.rs`,
`song/io/samples.rs`, `composer/{mod,transport,state,piano_roll}.rs`,
`composer/sidebar/blocks.rs`, `theme/mod.rs` + `theme/tests.rs`, `command_api/{mod,specs,
song}.rs`, `audio/src/manager/song.rs` + `lib.rs` (the three hunks),
`playground/src/{bridge,archive,store}.rs`, `store/{directory,memory}.rs`,
`store/indexed_db/mod.rs`, `persist/mod.rs`, `common/src/vfs/mod.rs`,
`music_midi/src/song_file.rs` + `validate.rs`, `scripts/check_audio_provenance.sh`,
`docs/{WEB_PLAYGROUND,SONG_FORMAT,EDITOR_COMMAND_API}.md`, the crate guides
(`editor`, `editor_integration`, `playground`, `audio`, `common`), and on the site
`PlaygroundToolbar.astro` and `playground-embed.ts`. Accessibility: every new control
labelled, units in text ("120 BPM", "4/4"), the playhead never colour-only (the status text
carries its position), the focus ring in every area, keyboard transport; blockers filed as
issues, never claimed. **Tests:** intent sequences for Play/Pause/Resume/Stop and
preview-before-Play (pure, on `EditorGame` with `&mut World`); under `GameHarness`: a preview
is playing after Play on the transport and the saved song after the two-frame transition —
through `handle_play_action` and through the dirty prompt's "Play saved" — Stop while armed
drops the pending play, Pause holds `song_position`, Stop returns the preview `Stopped` and
flushes the sources, a missing sample in the saved file keeps the preview, shows the error
and starts no session, a preview reaching its end reads `Stopped`, Play before
`enable_output` on a gesture-gated manager stays `Playing` with its Trigger buttons disabled
and the throttle allowed, an edit and an undo under a playing preview each stop it; the throttle state while previewing; a song replaced under a playing preview stops
it; the snapshot carries song bytes and leaves history and path alone; export replaces saved
song bytes and drops only the open song's pair; import rejects a final sidecar without its
`.mid` and a `.mid` without `MThd` and accepts a lone `.mid.ron.new`; `upload.rs`'s three
kinds, its refusals, its names, its truncated and header-only files and a `__` song name; the FIFO and removal tests of
correction 4; the five tests of correction 6 and the three of correction 7; a fractional,
negative or non-finite beat on the API is `Invalid`; the audio crate's start tick and the
conductor's `Seek` while `Stopped`; Pause while gesture-pending is refused; a replace of the
open song's path is refused; a space in a song name is refused and a `.wav` with one lands
under `_`; a song path under `samples/` is refused; each upload kind is refused by size
before decode; a truncated `.new` in an archive is dropped with a note; `song edit` sets the
song's provenance; the preview is parked by Play and says "stopped early" on a dead output.
**Gates:** standard; `check_wasm.sh`; `check_games.sh` (public items of `audio` move);
`check_audio_provenance.sh` with its new pattern; the site's `npm run verify` for its half;
the bundle's size line (budget 16 MiB); the manual keyboard and listen check, documented in
the report. **Leaves out:** live mutation of a running score, MIDI recording, a native file
dialog, Exit prompting (filed as its own issue — it prompts for nothing today, scene
included), #184's screenshots (Jesse's capture, dispatched by the planner).

## Batch 12 — K5: the Pong demo, the two-enemy example scene, the web proof (#64) — DONE 2026-10-23 (d8f8a2e; pong d4e50bd; site db162d3)

**Landed 2026-10-23.** Built by the Opus executor session (`review/conductor/report-12.md`); Jesse's
listen check passed natively and in both browsers before the code round. Code round 44 (kimi 4 /
gemini 4 / codex 2 on the engine, kimi 0 / gemini 4 / codex 1 on pong, the planner 4), then the
planner's fixes in four enumerated passes, rounds 45–48 (`rebuttal-44..48.md`): the added-track count
guarding `remove_added_track`, `AddTrack` refusing a name a part selects and a sidecar edit refusing
a new selection of a doubled name, the composer's selector reconciled after an undo (per frame, after
the API's lines, before the render), the SoundFont row's `is_file` and depth-capped link walk, the
respawned hummer's patrol, pong's `Resumed` arm clearing the jam's pause history. **Ruling 3b (Jesse,
mid-batch, 2026-10-23): the editor starts silent** — `EditorGame::init` stops whatever music the
wrapped game's `init` asked for, and Stop stops the slot whoever filled it; it supersedes correction
4's editor-bundle paragraph and ruling (j)'s parenthetical below (left as written, for the record).
Jesse took **door (b)** the same day (8604ddb; pong's guide line d0a8240; #198 closed): the editor
remembers what the game's `init` asked the audio for (`init_music.rs`, over the audio's new
`song_request` / `music_request`) and asks for it again on each session's first frame when no composer
song is open; the platformer demo's music control reads the audio's state; rounds 49–51
(`rebuttal-49..51.md`). The K6 rule is in batches 13–15. Measured: Chromium's throttled cold-boot increment +1.71 s (gate 8 s); Firefox +0.07 s; the
stress run's longest drain gap 63.8 ms (Chromium) and 55.0 ms (Firefox), no two consecutive windows
over 92 ms; v7 `game_bg.wasm` 7.46 MiB (the carried batch-6 finding closes on the number); the bank
1.98 MiB. Follow-ups: #197 (the Compact layout's clipping — the executor's screen finding; codex
raised nothing on the three screenshots, so #184 closed with the commit), #198, #199 (no Stop signal
on the web editor bundle), #200 (the XTEST wheel notch); #159 carries the standalone playground
bundle's rebuild; #195 rescoped to the clipboard and the live region. Scorecard rows 44–48 beside this
file.

**Re-verified 2026-10-22 against the tree at edc1509** (engine), `games/pong` 4cb0d5c and
`insiculous_web` 5375ebc, after batch 11 landed; **v14, settled after plan rounds 31 to 43** (kimi 7 + 7 + 6 + 5 + 7 + 5 + 8 + 8 + 6 + 8 + 8 + 9 + 7, gemini 7 + 7 + 7 + 7 + 8 + 7 + 6 + 7 + 8 + 8 + 8 + 8 + 7, codex 4 + 3 + 3 + 4 + 3 + 2 + 2 + 2
+ 3 + 3 + 3 + 3 + 3; `rebuttal-31.md` … `rebuttal-43.md`; **the preloader's retry, designed over rounds 36 to 41, is out of this batch
and filed as #196**) and Jesse's playtest of the batch 11 composer in
`editor_demo` (correction 10). Ten corrections to the section as it stood since the plan
settled; each names what the tree says.

Executor: the Opus session, in three checkouts — `games/pong` (the game), `insiculous_2d` (the
example, `training.md`), `insiculous_web` (the deployed bundles and the game's page) — with one
report. Reviewers: kimi (everything), gemini (the example's harness and the web bundles), codex
(the beat pulse, the composer screenshots, the pause jam). Listen check: Jesse, native and in
Firefox and Chromium. Size: about +1,300 lines across the three repos plus the assets (a song,
its sidecar, a 2 MiB SoundFont subset, a scene, a sine bank); the game's diff is reviewed
from `games/pong/review/conductor/`, the engine's and the site's from
`insiculous_2d/review/conductor/`, each reviewer dispatched from the repo it reads (the fence
is per checkout).

**Carried from batch 6 (F6, kimi F1):** the audio crate puts rustysynth and the song seam in
every wasm bundle. This batch measures Tong's `game_bg.wasm` and its editor bundle before and
after, against the per-game audio budget (≤ 24 MiB downloaded: bank ≤ 22, songs ≤ 256 KiB,
samples ≤ 2 MiB; ≤ 50 MiB loaded — plan § SoundFont), in the scorecard; a `song` feature on
`audio` is the lever: **filed as a follow-up if v7's `game_bg.wasm` exceeds 10 MiB** (6.73 today;
the wasm budget that binds is the playground's 16 MiB, not a game's), else the measurement is
informational and the carried finding closes on the number.

### Correction 1 — Tong has no sound today, so there is nothing to preserve or to fall back to

#64 and the old section speak of a pause-menu jam to *preserve*, a recorded-SFX *fallback* and
beeps that must not *double-play*. The tree has none of it: `games/pong/assets/` holds `fonts`,
`locales` and `sprites`; `grep -rn "audio\|play_sound\|beep" src` finds nothing but the pause
menu's labels; the `beep_e.wav` in `dist/games/pong/v2/` is the pre-Tong bundle, not a source
file. The Tong re-skin (2026-09-15) shipped silent. So: the jam, the hits and the pulse are
**new**, nothing is gated behind a binding replacing a beep, and **ruling (e), proposed: no
recorded fallback** — when `play_song` fails at `init` the game logs the error once, every
trigger is refused and counted (`ctx.sources.refused_triggers()`), and Tong is as silent as it
is today. A fallback would mean authoring a hit sound this batch has no source for.

### Correction 2 — the bank is Tong's own subset, committed, about 2 MiB

The spike's `cast.sf2` is 32.6 MiB and uncommitted (`coordination/conductor/spike/assets/`,
rebuilt by its `prepare_assets.sh`): over the 22 MiB bank budget and over Cloudflare's 25 MiB
per-file limit, which `insiculous_web/scripts/postbuild-check.mjs` enforces. Tong ships
`games/pong/assets/soundfonts/tong.sf2`: FluidR3 GM presets **0:56 Trumpet (226 KiB), 0:57
Trombone (438 KiB), 0:47 Timpani (1,361 KiB)** — about 2 MiB of sample data by
`scripts/subset_soundfont.py --sizes` on the cast bank — plus any program the sequenced song
uses beyond them (the subset rule, plan § SoundFont). **No strings** (0:48 alone is 5.5 MiB —
the accompaniment is trombone chords over the timpani's march, a brass band's Toreador, decided
upfront rather than by a mid-batch re-voicing: round 38 modelled the 7.3 MiB bank's boot
increment at 9–10 s against an 8 s gate, and a fallback that re-sequences the song after the
screenshots is a batch within a batch) and no kit (the Standard kit is 10.6 MiB and the
subsetter cuts by preset, not key range). `games/pong/scripts/prepare_soundfont.sh`
mirrors the spike's script: FluidR3_GM.sf2 by path or by `apt download fluid-soundfont-gm=3.1-5.3`,
the sha256 pinned (when `apt download` cannot find the pinned version the script **fetches the
same package from snapshot.debian.org itself**, a real branch, and the sha256 check after it is
what proves the file is the same), the presets listed once, `subset_soundfont.py` run, the `LICENSE` written
beside the bank in the spike's exact shape (Upstream, Source with the bank's sha256, `Files:
tong.sf2`, the preset list, the output's sha256, MIT). **The subset is committed** (ruling (h),
proposed: a 2 MiB binary in the game repo, as the synced PNGs are; the spike's bank was not
committed because it was tens of megabytes). Downloaded: `game_bg.wasm` 6.73 MiB today
(`insiculous_web/public/games/tong/v6/`) + the bank + the song, under 24. Boot: every asset is
prefetched (`engine_core::web::preload_assets` fetches the manifest's every entry before the
first frame), so the bank adds its size to the boot — under 2 s at 10 Mbit/s; the report
records the measured increment (correction 6) and the fetch-after-first-frame lever is #194. The provenance gate walks `assets/soundfonts`
already.

### Correction 3 — the song is sequenced through the composer's API, checked, then saved by it

`games/pong/assets/ai/music/ai_tong.mid` and `ai_tong.mid.ron`: Bizet's *Toreador Song*
refrain (Carmen, 1875; public domain) in a short arrangement the executor sequences **in the
composer** — the editor on the pong project (`cargo run --bin editor --features editor -- .`
from `games/pong`, or the game with `--features editor`) driven through `--api` lines (`song
new`, `song edit {"AddNote": …}`, `{"SetTempo": …}`, `{"AddPart": …}`, `{"AddInstrument": …}`,
`{"SetBinding": …}` (the edit that adds or replaces a binding — there is no `AddBinding`),
`{"SetProvenance": …}`, `song save ai/music/ai_tong.mid`; `docs/EDITOR_COMMAND_API.md` § song),
so Save writes the sidecar, the fingerprint and the `LICENSE` stanza in
`assets/ai/music/LICENSE` — the authoring workflow #64 asks to prove, and the one
`training.md` describes (correction 7). **Before `song save`, the document is checked against
the intended shape**, not trusted: the `song` answer (parts, instruments, bindings,
`tempo_changes`, `provenance`) and `song notes <part>` per track (note counts, the first and
last tick) are compared with the shape below and quoted in the report; a refused edit is
visible in its answer (`refused_reason`) and is fixed before the save. The composer's panel is
on screen while the API runs, which is where the screenshots come from (correction 8). Shape,
which the executor tunes by ear and declares in the report:

- **sixteen bars** in F major at about 120 BPM (32 s a loop), looping **at a phrase boundary —
  the wrap releases every triggered note and restarts the `Loop` cursors at the first unit
  (`SONG_FORMAT.md` § Modes), so the refrain realigns with the accompaniment every loop and a
  hit sounding across the seam is cut there; the engine's contract since K3 (a cursor that
  outlived the wrap would walk the tune against a chord it was not written over). Sixteen bars
  is why: a rally of a hit every two seconds walks a sixteen-unit refrain in the time one loop
  takes, so the tune is heard whole far more often than it is cut; the jam's counter-line is
  shorter and restarts with the loop as a menu's music may**; the background is the refrain's
  accompaniment — trombone chords and the timpani's march — on their own tracks, **which the
  composer must first be able to make**: `song new` gives a tempo track and one `melody` track
  (`crates/editor/src/song/mod.rs:270-303`), `AddNote` refuses a track index past the last
  (`music_midi/src/song_data/edit.rs:275-288`), and no edit, verb or control adds a track —
  multi-track songs enter only by import today. So this batch adds **`SongEdit::AddTrack {
  name, channel, program }`** over a new **`SongData::add_track`** (a `SongTrack` with no source
  track, no units, its name and channel, **and one `RetainedEvent` at tick 0: the channel's
  `ProgramChange { program }`** — the background player reads programs from the MIDI, so a track
  with none plays on the bank's first preset whatever the arrangement meant, and a channel
  event is also what `flatten.rs` derives the track's channel from on reopen, so an empty track
  saved and reopened keeps its channel; `write_smf` writes the name, the program change and the
  notes; `check_track` accepts it), undoable (**the inverse removes the track with whatever notes it holds**, their before-images
  restoring them on redo — an inverse that left a filled track would desynchronise the history
  from the document; `RemoveTrack` as a user's edit is not this batch's), **the channel the next
  free one, counted 1 to 16 at the API and the GUI as the type-0 names count them (`channel N`
  from 1) and stored and written 0 to 15 as MIDI does — the conversion at the edit's boundary,
  once — skipping 10** (General MIDI's percussion channel, which rustysynth and the kit treat as
  the drum bank — a track that wants it names it), **refused with "no channel
  is free" when none is, and an API-supplied channel refused when it is outside 1–16 or already
  occupied — occupancy counted over every track's channel and every retained channel event
  (a controller or program change an imported track carries on a channel its notes do not use
  would otherwise reach the new track's voice)**
  (two tracks on one channel would write two program changes to one channel and the last would
  win for both); inside a `Batch`, members are undone in reverse order, so a batch of
  `[AddTrack, AddPart on it, AddNote on it]` undoes the note, the part and then the track —
  tested; **on a type-0 document the edit is refused** ("this song is a single-track MIDI file;
  its channels are its tracks") — a type-0 file's selectable tracks are rebuilt from its channels
  in channel order on reopen (`flatten.rs channel_tracks`), so an added physical track has no
  place in it, and the composer's songs are type 1 from `song new`; tested on an imported type-0
  fixture; the Add-track row takes the name and the program (a number field, General MIDI's
  0–127, the trombone 57 and the timpani 47 the accompaniment needs),
  the API's edit table and `EDITOR_COMMAND_API.md` gaining the line, the Parts section gaining
  an "Add track" row (a name field and a program number field, Enter committing both as one
  edit on the next free channel), with
  tests in `song/edit_tests.rs` and the parity test (`every_variant_lands_the_same_through_the_gui_…`)
  covering the variant, and an export-and-reimport test for a song with an added track in
  `music_midi`, plus one for **an empty added track saved and reopened keeping its channel and
  program**; the executor also reads the exported MIDI's program changes back (`song notes` and
  the sidecar say nothing of programs) and quotes them. The example's song (correction 5) needs it too;
- track `refrain`: the melody, the **one shared part** (`mode: Loop`, **`quantize: Off`** — a
  hit sounds within one block of the paddle contact; an `Eighth` grid at 120 BPM would hold a
  note up to 250 ms after the burst, and the player's rhythm is the point), played by two
  bindings: `hit` on the trumpet and `goal` on the timpani — the trombone carries the
  accompaniment — so a goal continues the tune where the hits left it, one cursor — **the
  goal's note is whatever unit the rally left the cursor on**, a sixteenth or a held quarter;
  that is what #64's one shared part means, and the unit plays whole on the game's source
  however short (the arrangement may write the refrain in even notes if the ear prefers it);
- track `jam`: a counter-line over the same bars, a **`Loop` part (`quantize: Off`)** on the
  `menu` binding, on the trumpet — **ruling (i), proposed**: #64's "Sequence jam" names a jam
  Pong never had (correction 1), and a `Sequence` part is exhausted after its last unit until
  the song loops (`docs/SONG_FORMAT.md` § Modes), which on a four-to-eight-unit counter-line
  is a menu that goes quiet after a few presses for up to the loop's length; `Sequence` stays
  demonstrated by `music_midi`'s tests and the format doc, not by the pause menu;
- `soundfont: "soundfonts/tong.sf2"`, `provenance: (author: "Georges Bizet, 1875; sequenced
  by the Opus executor for Be Insiculous, 2026", license: "CC0-1.0")`, no sample instrument
  (the sample case is the example's, correction 5).

The sidecar is the composer's output, never hand-edited after; `music_tests.rs` asserts its
structure (correction 4). The file sits under `ai/music` because an agent sequenced it (plan
§ Rulings); Jesse's pass promotes it to `music/` later, outside this batch.

### Correction 4 — the game's code: one module, three moments, a pulse

- **`src/music.rs`** (new, under 250 lines): `Music { refused_seen: u64, frame_times:
  FrameRing }` on `PongGame` (a 300-frame ring of `delta_time`, for the p95 the browser check
  quotes); `start(ctx)` called at the end of `init` (`src/lib.rs`),
  `ctx.audio.play_song(format!("{}/ai/music/ai_tong.mid", ctx.assets.base_path()))` with the
  failure logged once (correction 1); `hit(ctx, tong)` → `ctx.sources.trigger(ctx.audio,
  "hit", ctx.sources.entity(tong))` — **sent whatever song holds the slot**: an audition in the editor bundle may bind `hit` and
  `goal` itself, which is the point of auditioning an arrangement in the game, and one that does
  not refuses each as `UnknownAction`, counted by the manager (`refused_triggers`) at no cost
  worth a gate; the pulse follows whatever song plays; `goal(ctx)` → `ctx.sources.trigger(ctx.audio, "goal",
  ctx.sources.game())` (the game's source, so the goal's unit plays whole whatever the ball
  does next); `telemetry(ctx)` — every five seconds a `conductor=on` line, the shape
  `examples/conductor_background.rs:302` prints, **at `info` only when asked** (the web page's
  `?telemetry=1`, read through `engine_core::web::query_param`; natively `TONG_TELEMETRY=1`),
  and at `debug` otherwise, so a shipped demo does not fill the console for the life of the
  game (the web logger is fixed at `Info`, `web/mod.rs:242`), through **`SongSnapshot::log_line(&self)
  -> String` added to `crates/audio/src/manager/song.rs`** (the example formats its own today;
  it switches to the helper, so the two never drift), with the ring's
  p95 and mean frame time **and the game's own counts — `hits`, `goals`, `jam_units`, `pulsed`,
  `refused` — appended by the game**, so the browser checks that judge the jam and the pulse have
  a counter beside the ear; the telemetry the browser checks read. The source's two maxima
  (`SourceReadings.render_nanoseconds_longest`, `drain_gap_nanoseconds_longest`,
  `conductor_source.rs:74-75`) are cumulative `fetch_max` atomics, so a hitch during the boot
  would sit in every later line: **`AudioManager::take_song_maxima(&mut self) -> SongMaxima`**
  **swaps zero into both and returns what they held** (`AtomicU64::swap`, so a value the audio
  thread writes concurrently lands in the window just taken or the next one — with one edge: a
  `fetch_max` that read the old maximum just before the swap compares against it and may drop a
  smaller spike in that microsecond; the gate reads many windows, and the edge is stated, not
  closed). Every
  telemetry line prints what it takes, so each line is its own window; **`init` takes once when
  the title screen is up**, discarding the boot's maxima, so the first match's line is not the
  boot's; the stress gate reads the maximum over every line printed after the first match
  start. The counters are not reset. `init` ends with `log::info!("tong: init complete")`, the
  console line the boot measurement ends on (correction 6). The engine hunks this batch makes
  are four — `song_label`, `take_song_maxima`, `log_line`, the prelude's `Subdivision` — the
  track edit of correction 3, and the example's manifest lines; `jam(ctx)` → `"menu"` on
  `ctx.sources.menu()`; `pulse(ctx)` (below); `restart(ctx)`, called
  from `gameplay/flow.rs start_game` (:65), **decides by the slot's identity**, read through
  **`AudioManager::song_label(&self) -> Option<&str>`** — the label the playing or pending song
  was requested with (`SongPlayback.label`, the path's display for `play_song`; an accessor added
  to `crates/audio/src/manager/song.rs`, since `song_position()` says a song plays but never
  which), **`None` for a playback that has ended or whose output died, `Some` for a paused one** (ended
  and dead are the filter — a preview that played to its end leaves `songs.playback` in place
  with `has_ended()` true, since nothing reaps it, and that is "no song" to the game; a paused
  song is a live song another caller holds, and `restart` leaves it alone, since `install_song`
  stops what plays before it installs): **no song** (a composer preview that ended took the slot and left it,
  or the output died) → `play_song` of the same joined path `start` used (the asset base and
  `SONG_PATH`; a bare relative path would miss on the web), the game's re-arm — **retried at every match start**: a request that fails on a missing file fails before any
  parse and costs nothing worth avoiding, and a bank that appears later (natively, between
  matches) is heard at the next match; the first failure is logged at `error`, the later ones at
  `debug` (`Music.failures` counts them), which is what correction 1's "logged once" means; **Tong's own song** (the label
  **equals, compared as `Path`s, the joined path `start` requested** — the whole path, never a
  suffix, so `remixes/ai/music/ai_tong.mid` is another song; the composer's saved song is
  requested by its resolved path under the same asset base, so a save at Tong's path compares
  equal) → `seek_song(0)` and a telemetry line printed from `take_song_maxima()`
  (below), so every match begins at the top of the arrangement — the melody's cursor too: "a
  loop wrap, a seek and `Play` release every triggered note and drop every scheduled one; `Loop`
  and `Sequence` cursors restart at the first unit at or after the new position"
  (`SONG_FORMAT.md` § Modes), so the first hit of a match is the refrain's first unit; **another song** → nothing — in the editor bundle the
  composer's saved song is the session's music by batch 11's slot contract, and a match that
  replaced it would defeat the audition K8 exists for. A song the composer saved **at Tong's
  own path** (`ai/music/ai_tong.mid`, an `overwrite` away) is Tong's song by definition and is
  sought to the top at a match start as Tong's is — the one path, one file, one rule. The song is requested at `init` and plays
  through the menus; the start of the first match seeks it. Constants (`SONG_PATH`,
  the binding names) in `src/constants.rs`.
- **Hits** in `gameplay/scoring.rs spawn_paddle_hit_visuals` (:167), beside the burst and the
  ripple, one trigger per `(ball, side)` with the tong of `side` as the source
  (`self.playfield.paddle(side)` or the two fields) — **two balls hitting in one frame are two
  hits and two units**, as they are two bursts; Insane's speed step is not a second hit;
  **goals** in `score_ball` (:192) beside the explosion.
- **The pause jam** in `gameplay/mod.rs update_gameplay` (:48–66), after `pause.update`
  has returned `Idle` with the menu still active: the game reads **`MenuInput::read(ctx.input)`
  itself** — the same reader the menu navigates by, `pub up` and `pub down` just-pressed from
  W/S, the arrows, a d-pad or a stick crossing, with no key repeat — and `up || down` calls
  `jam`: one unit a move, confirm and Cancel nothing, **the open frame nothing** — `pause.update`
  returns `Idle` on the frame it opens with no `Opened` variant, so the game latches the
  transition itself (a `was_paused` bool: the jam reads nothing on a frame where the menu became
  active, which also covers W or S pressed on the same frame as Escape) — **and the mouse
  nothing** — `PauseMenu`
  moves its selection to the hovered row every frame the pointer rests on one
  (`pause.rs:152-154`), so a jam keyed to the selection would fire from pointer noise and on
  every reopen under a resting pointer; keyed to the navigation input it cannot. No engine
  getter is needed, and `Music` keeps no menu baseline. **Ruling (f), proposed: the song
  plays on under the pause menu.** The game's pause is `ctx.time_scale = 0` (the world and the
  emitters freeze; `training.md` § Conductor); the song has its own `pause_song`, which this
  game never calls — the jam needs the background under it, and a paused song drops triggers
  (§ Modes). The pause hint line (`pause.hint` in both locale files) gains "· move to jam" —
  one key each in `en.ron` and `pirate.ron`; `locale_files_have_matching_keys` holds them
  together.
- **The beat pulse** — `Music::pulse` each gameplay frame, **Serving and Playing while not
  paused** (`Subdivision` reaches the game through **`engine_core::prelude`, which gains the
  re-export** beside `AudioManager` and `SourceId` at `prelude.rs:89` — the one engine hunk
  the game needs; today the prelude names neither it nor any song type):
  `if ctx.audio.beat_crossed(Subdivision::Bar) { the full hit ripple under each grill }
  else if ctx.audio.beat_crossed(Subdivision::Beat) { a quarter of it }` — the bar line is a beat
  line too, so the bar is tested first and the two never stack; `ripple_grid` at (`±GRILL_X`,
  the grill's y). Not while paused: `GridBackdropSystem::update` discards the frame's impulses
  when its delta is zero (`grid/backdrop_system.rs:81-89`, "a frozen grid must not bank them for
  later"), so a pulse queued under the pause menu is wasted at best. The backdrop grid is the
  one visual that reads "pulse" without touching the art (every art sprite is white at emissive
  0; the grill's `score` clip is a goal's flare and stays that). `beat_crossed` is latched once
    an `update`, so a long frame is one pulse, not a flood. Codex judges the look from stills
  (correction 8) and **the timing is judged live** — the ripple against the heard timpani —
  by the executor on the session display (noted in the report) and by Jesse in the listen
  check; the ripple leads the heard beat by the output buffer (up to 61 ms on the web), which
  is the output-latency compensation follow-up batch 16 files, not this batch's; a different
  strength is a constant.
- **No volume control is added**: Tong has no settings menu, so "music at 0 keeps the hits,
  SFX at 0 keeps the song" (plan § Verification) is shown on the example's M and S keys
  (correction 5), and the engine's own tests hold the buses. The game's `Music` reads
  `refused_triggers()` once a second into a `log::debug!` line, the crowd counter a player
  never sees.
- **Tests** (`src/music_tests.rs`, `GameHarness` as `title_screen_tests.rs` builds it, the
  asset base the repo's own `assets/`, so the committed song and bank are what loads):
    - **the sidecar's structure**, parsed from the committed file with `music_midi::parse_song_file`
    (`music_midi` becomes pong's second dev-dependency, a workspace path like the engine's,
    for this test alone — `engine_core`'s prelude exports no sidecar parser; `Cargo.toml` is in
    the touched list):
    `hit` and `goal` name one part, that part is `Loop` with `quantize: Off`, `menu` names a
    `Loop` part, `provenance` is set — the shape the aggregate counters cannot see (a sidecar
    that gave each binding its own part would pass every trigger count);
  - the song plays after `init` (`harness.audio().song_snapshot()` is `Some`, `blocks_rendered`
    moves across frames with 3 ms between them, as `crates/engine_core/tests/song_triggers.rs
    settle` does);
      - a paddle hit in a frame is one accepted trigger (`conductor.accepted_triggers` +1) and
    the conductor's `live_sources` rises to one; **an entity's activation lives until `end`, its
    despawn or a flush** (`sound_sources.rs` module doc: "every later trigger reuses it"), so
    `source_of(tong)` is `Some` from the first hit to the match's end and a second hit adds no
    source; a goal is one more accepted trigger and one more live source (the game's), and its
    count survives the ball's respawn — `source_of(ball)` is `None` because the ball never
    triggers, which is not asserted as if it were a contract; two hits in one frame from two
    balls are two;
    - the pause jam, the `PauseMenu` driven through `harness.step` with key and mouse events:
    opening the menu jams nothing; one move jams one unit; confirm and Cancel none; a pointer
    resting on, or swept across, the rows jams nothing while the selection follows it; the
    menu left on row 1, resumed and reopened under that pointer jams nothing;
    - a frame on a beat queues the grill ripples (a `pulsed` counter on `Music`, since the
    frame tail drains the `GridImpulses` resource), and none while paused;
    - `restart` by identity: with no song in the slot `start_game` requests Tong's (the label
    becomes `SONG_PATH`); with Tong's own, it seeks (the position reads the top); with another
    song playing (the harness plays the sine fixture's song first) `start_game` leaves it — the
    label is unchanged and `accepted_triggers` of Tong's bindings do not move; **with another
    song that played to its end** (a non-looping fixture song run past its length on the virtual
    clock) `start_game` requests Tong's, since an ended playback is no song;
        - a missing bank — a scratch directory under `std::env::temp_dir()` named by the process id
    and the test, removed by a guard struct's `Drop` on success and on panic alike (pong adds
    no `tempfile`), holding **`assets/` without `soundfonts/`** — `sprites/`, `fonts/`, `locales/` and
    `ai/music/` as symbolic links to the repository's own directories — **absolute targets**,
    `CARGO_MANIFEST_DIR` joined, since a relative target would resolve under the scratch
    directory and dangle (`std::os::unix::fs::symlink`; **the test is `cfg(unix)`** —
    `std::fs::copy` copies no directory, and the gates run on Linux), nothing duplicated — the list is every directory `init` reads (`load_sheet`'s `expect` panics
    on a missing PNG, `lib.rs:58-62`), and the test's assertion that the song failure was reached
    is what proves the list complete — leaves the game running, silent, `Music.song_failed` set and every hit counted as skipped
    (the log line is written once, but the test asserts the flag and the counts — the harness
    captures no logs, and no test infrastructure is built to).
- **In the editor bundle the song runs until a composer preview takes the slot** (and a song
  the composer saved at Tong's own path with no `hit` binding makes every hit a counted refusal
  for that session — the one-path-one-file rule's consequence, stated):
  `/playground/tong/` opens the game in the editor with no composer song open, so Stop touches
  no music (batch 11's slot contract) and the song plays through Editing and across Stop; a
  visitor who opens the composer and previews a song replaces it in the one slot (the engine
  guarantees no continuity there); when that preview ends — stopped, or played to its end, which
  leaves an ended playback the game reads as no song — the next match start requests Tong's
  song again, and while the composer's saved song plays as the session's music a match leaves
  it alone (the audition) — **Tong's triggers are sent whatever song holds the slot**: an
  arrangement that binds `hit` and `goal` is heard through the rally (the audition), one that
  does not refuses them, counted; the pulse follows that song's beats. A session whose saved song failed to load (correction 10) starts
  silent when a preview held the slot, and otherwise with whatever already played — Tong's own
  song, requested at `init`, in the common case; its first match start then re-requests or seeks
  by the identity rule, and the game's own music plays through Editing as it always has — Stop
  stops the editor's songs, never the game's. An unsaved song cannot reach Play —
  the dirty prompt asks for Save or Cancel — so a preview never sounds under a session. That is the contract, stated here so codex's screenshot round has its answer; the
  report notes it under the browser checks, with a preview-then-Play check in the bundle.
- **`CLAUDE.md`** (pong) gains a "Music" section: the song's files, the three bindings, the
  jam, the pulse, the no-fallback rule, the bank's script and budget line, one sentence on the
  committed binary (it stays in the history whatever replaces it; the `apt download` path is
  Debian's, the script's `--bank <path>` is everyone else's, the sha256s are the provenance),
  the version bump this batch makes (correction 6). The `review/conductor/` directory in pong
  holds the game's reviews; the one report is the engine's.

### Correction 5 — the two-enemy demo is an example with its own tests, and it gates its triggers itself

`examples/conductor_demo.rs` + `examples/conductor_demo/enemies.rs` (the pure rules, shared the
way `hello_world/platformer.rs` is), **its asset base `examples/assets/` — unlike the other
examples, whose base is `examples/`** — so the sidecar's `soundfonts/sine.sf2` and
`music/samples/ai_demo/ping.wav` resolve, and `init` logs at `error` and shows "song: failed —
…" on its HUD line when `play_song` fails, so a wrong base names itself at first run;
registered in `Cargo.toml` as `[[example]] name =
"conductor_demo"` **with `test = true`**, so its inline `#[cfg(test)]` tests run under `cargo
test --workspace` (the existing examples set no `test`, the default `false`; this is the first
tested example). Assets, each with a `LICENSE` the gate reads:
`examples/assets/scenes/conductor_demo.scene.ron`; `examples/assets/music/ai_demo.mid` +
`.mid.ron` (a two-bar melody and a drone, sequenced through the composer's API as the song
is, checked the same way, saved under `music/` with `ai_` in the name since the folder is the
example's); `examples/assets/soundfonts/sine.sf2` (a copy of `crates/music_midi/fixtures/sine.sf2`,
the fixture `LICENSE` entry carried); `examples/assets/music/samples/ai_demo/ping.wav` (a copy of
`fixtures/sine_48000_mono.wav`, 48 kHz mono, root note 60 — **the imported sample**, placed
where a save places it so the sidecar's `Sample(path: "music/samples/ai_demo/ping.wav", …)`
resolves). The scene and the demo's `Game`:

- two enemy kinds, `Hummer` (a SoundFont instrument, the sine bank's one preset) and
    `Pinger` (the sample instrument, `playback: OneShot`), each an entity with a `#solid` sprite
  and a `Name` — **two bindings (`hum`, `ping`), one shared part** (`melody`, `Loop`,
  **`quantize: Off`**: an attack sounds when it is minted; a grid would gather two kinds' attacks
  onto one line as a dyad), so the kinds alternate through one tune, never two melody tracks
  and never a synthesizer per entity (the slot is per instrument);
- **the demo mints every trigger itself, and only for an eligible enemy**: `enemies.rs` holds
  each enemy's cadence — **in game time, not frames, on one clock**: the demo's `Game` keeps
  one `clock` advanced by `ctx.delta_time * ctx.time_scale`, and an enemy of period `p` and
  phase `φ` attacks when `floor((clock − φ) / p)` (an `i64`, negative before the phase is
  reached) has grown past the index it holds (`attack_due(clock, period, phase, last_index)
  -> Option<i64>`), **its index initialised at spawn to the current value (no clamp — a clamp made the first
  spawn and a respawn behave differently) and advanced every frame whether or not the enemy is
  eligible** (eligibility gates the mint, never the clock, so an enemy back from ten periods
  off-screen attacks at its kind's next slot, not on its first eligible frame; tested) — so a
  spawned or respawned enemy waits for its kind's next slot rather than attacking on its first
  frame; at the scene's start `Pinger` (half a period on) attacks at half a period and `Hummer`
  (phase 0) at one, then they alternate — which of the two leads is not a contract — at
  most once a frame (a slow frame mints one attack, never a burst); `Hummer` at phase 0 and `Pinger` half a period
  on, so the two alternate at any frame rate and **a respawned enemy joins the shared clock at
  its kind's phase**, never phase-locked to the other kind — and the eligibility rule
  (`eligible(position, camera, window) -> bool`, the game's policy — K3 #62: "game/host camera
  policy supplies on-screen eligibility"; a binding is never spatial, the scene loader refuses
  it), and the `Game`'s `update` calls `ctx.sources.trigger(ctx.audio, binding,
  ctx.sources.entity(enemy))` for each enemy whose attack falls this frame **and** who is
  eligible. No cadence script and no `SoundEmitter(Binding(…))`: a scripted `cmd.play_sound`
  reaches the conductor before the game can gate it, and an `emitters.stop` after the fact
    only ends the source — the trigger was accepted and the shared cursor moved (round 31,
  codex F1 / kimi F1). **Leaving the view ends the enemy's source** on the frame of the
  transition — `was_eligible && !eligible`, an edge, never every off-screen frame
  (`ctx.sources.end(enemy, ctx.audio)`: its pending and sounding notes stop — the policy for
  a note already accepted), so an off-screen enemy neither sounds nor advances the melody,
  the frame it leaves included; on re-entry its next attack activates it anew. **A hitch may
  land both kinds in one frame**: a frame from 0.9 s to 1.6 s makes both cadences due and
  both attack, two notes at once — permitted and tested as the stated policy; alternation is
  the ordinary frame's property, not a guarantee across a skipped interval;
- despawn and re-entry: the D key despawns the nearest enemy mid-note (its source ends in that
    frame's tail, `training.md` § the same-frame rule) and R respawns the kind; dense activity,
  **three workloads**: `--stress spread` spawns 48 enemies at a one-second period with phases
  spread over the second (48 attempts a second, sustained) and `--stress burst` the same 48 at
  phase 0 (48 attempts on one frame, every second) — **every stress enemy is placed on a grid
  inside the camera's view**, so eligibility passes all of them, and each log line prints the
  eligible attempts of its window beside the conductor's counts, so the report quotes attempted
  against accepted and rejected — both **measurements**, printing
  `conductor=on` log lines as `conductor_background.rs` prints them (attempted, accepted,
  rejected and late triggers, render cost, the longest drain gap, voices) every five seconds,
  so the report quotes measured rates; and **`--stress saturate`, the refusal shape**:
  40 `Pinger`s attacking on one frame, every second — the sample slot holds 16 live voices plus
  a fade budget of 16 and refuses the newest note whole past that ceiling, counted as rejected
  (`music_midi/CLAUDE.md`, "Fading voices are bounded without a cut"); the exact bound (the
  newest refused at the ceiling, no voice cut) is **`music_midi`'s existing test**
  `a_burst_of_sixty_four_triggers_in_one_block_refuses_the_newest_at_the_ceiling_and_stays_finite`,
  so the demo **asserts nothing about the count** — forty submissions from one game frame
  may straddle a block drain when the game thread is descheduled, and a run on a loaded machine
  may reject none — and `saturate` is a measurement whose rejections the report quotes; `quantize: Off` takes the immediate-start path, so the instrument's
  24-pending limit is not what this exercises, and the SoundFont slot past 24 sounding
  releases its oldest rather than refusing (§ Modes);
- **pause input**: Space is the game's pause (`ctx.time_scale = 0`: no attack is minted, the
  song plays on); P is the song's pause (`ctx.audio.pause_song`: the clock holds, triggers are
  dropped, the game runs on) — the pause distinction `training.md` gains; M and S mute the
  music and sfx buses (the melody under the song at 0, the song under the hits at 0).

Tests in the example (`GameHarness` over the demo's `Game`, the scene loaded from
`examples/assets`): both kinds play the one part in turn (`accepted_triggers` and the
sidecar's structure, as the game's tests assert it); an enemy outside the view mints nothing
(`accepted_triggers` unchanged across its attack frames) and its source is ended on the
frame it leaves (`source_of` `None`, a sounding note included), then activated again by its
first attack after re-entry (`source_of` `Some`); a despawn mid-note ends its source that
frame (`source_of` `None` after the tail); the three stress shapes run without a panic and a despawn's end
still lands under each (their counts are measured, not asserted; the ceiling's bound is
`music_midi`'s test); a respawned enemy's first attack falls on its kind's phase, not the other's; a hitch frame (one `step` of 0.7 s) lands both kinds' attacks in one frame; the game's pause mints nothing while `blocks_rendered` advances; the
song's pause drops a trigger as **`rejected_triggers` +1** (a trigger under a song that is not
playing is rejected with its cursor kept, `triggers.rs:223`; `stale` is another song's binding
or an ended source, `late` an old stamp). The demo's `Game` is under 600 lines
with `enemies.rs` beside it.

### Correction 6 — the web proof is Tong's two bundles and its page, on new versions, with a boot threshold

Both bundle versions are four-place contracts (`games/pong/src/web_entry.rs` names the lists):
the game goes **v6 → v7** (`ASSET_BASE = "/games/tong/v7/assets"`, `scripts/build_wasm.sh
../games/pong tong --version v7`, `insiculous_web/src/content/games/tong.md` `wasm:
'/games/tong/v7/game.js'`, `insiculous_web/public/games/tong/v7/`) and the editor bundle **v5 →
v6** (`EDITOR_ASSET_BASE = "/playground/pong/v6/assets"`, `--kind editor --version v6`,
`editor: '/playground/pong/v6/game.js'`, `public/playground/pong/v6/`), because the assets
changed (the bank and the song travel in both; `--kind editor` copies the game's whole
`assets/` — "the game's own assets are the bundle", `build_wasm.sh:105`). **`/playground/tong/`
is that editor bundle**, the game compiled with `--features editor` over its own assets — not
the playground's bundled `pong` *project* (`crates/playground/assets/projects/pong/`, 52 KiB of
scripts and two images, served at `/playground/`), which is untouched; the 16 MiB playground
budget is not in play. The old version directories stay until the deploy, as every bump has
left them. The site half is the two `public/` trees plus the content line; `npm run verify`
there, whose postbuild check holds every file under 25 MiB.

**The bank is a boot dependency, as every asset is**: `engine_core::web::preload_assets`
fetches the manifest's every entry before the first frame and `web_entry.rs start` aborts the
boot on any fetch failure with "Failed to load assets: …" on the page — a bank the network
drops is that failure, not a silent game (the silent path, correction 1, is a bank that loads
but a song that fails). Nothing in this batch makes an asset optional. **The boot threshold,
pinned before the work starts, is the bank's own cost**: on the built site served by `npm run
preview` (real files and sizes, not the dev server), in Firefox and Chromium with DevTools
network throttling at 10 Mbit/s **and CPU throttling at 4× slowdown** (the SF2 parse and the
wasm compile are main-thread work a network profile does not see), the executor measures from
navigation to the title screen: **the baseline is v7's own code built without its music** — `build_wasm.sh` over a copy of the
game directory **placed beside it in `games/` as `games/pong-baseline/`** (the engine is a
relative path dependency, `../../insiculous_2d/…`, so the copy must sit where pong sits; `target/`,
`dist/` and `review/` left out of the copy; deleted after the measurement) whose `assets/` lacks
`soundfonts/` and `ai/music/`, **built as version `v99`** with its `web_entry.rs` constants set
to `/games/tong/v99/assets` — `build_wasm.sh:114-135` pins the constant to the version, so the
two arms get two versions and **one `npm run preview` root serves both** — `astro preview`
serves the built `dist/`, not `public/`, and `v7` is not in `public/` until the checks pass, so
after `npm run build` the harness copies **both arms** from the game's `dist/` into
`insiculous_web/dist/games/tong/{v7,v99}/` and, since the game's page is Astro's from `tong.md`
and names one bundle, copies `dist/games/tong/index.html` beside each as the arm's host page with
its `game.js` path rewritten (never into `public/`; a second ad-hoc server would
serve the wasm as `application/octet-stream`, which the browser refuses); every cold run uses a
fresh `--user-data-dir` with `--no-first-run --no-default-browser-check` (Chromium's code cache
would skip the wasm compile, and a fresh profile's welcome dialog would take the focus), **the
cache disabled for the cold arm and enabled for the warm**, and a sanity check that each arm's
warm median is at or under its cold; the harness asserts per run that `v99`'s `manifest.json`
lists no `soundfonts/` entry and `v7`'s does, and that `v99` logs the one `play_song` failure line **at `error` level** (correction 1's
"logged once" is this line: `Tong's song could not play: <error>`; the web logger shows `info`
and above) — so both arms carry
the `tong: init complete` marker and differ by the bank and the song alone (the deployed v6 has
no marker to end on); each cold (cache cleared) and warm, **five
interleaved pairs (baseline, v7, baseline, v7, …) with nothing else running on the machine, the
median of each arm**, so drift on the shared display hits both arms alike;
plus `/playground/tong/` v6 cold once (the editor bundle boots the editor and the bank;
recorded, no threshold; **its stopwatch ends on the same `tong: init complete` line** — `EditorGame::init` runs the
wrapped game's `init` at boot (`game_impl.rs:34`), not at Play, so the line is logged there too
and the bank's parse is inside the measured interval); the `v7` and `v99` copies under `dist/` and the `games/pong-baseline/` copy are removed by a
shell `trap … EXIT` around the measurement, never staged, and a copy already present when the
batch starts stops the batch. **The 8 s line is a threshold with no fallback branch**: the bank is the brass-and-timpani
subset from the start (correction 2), so if v7's cold median exceeds the baseline's by more than
8 s in Chromium the cost is the parse's and the serial preload's; **the bundles are still built
and staged, the report flags the exceedance in its first lines, and Jesse decides before the
deploy push whether v7 goes out or waits for #194** — nothing is re-sequenced mid-batch. The CDP harness also asserts that the match's telemetry shows `accepted_triggers`
rising, so a song that failed to load (which ruling (e) keeps silent) cannot pass the browser
check unnoticed. The numbers go in the report and the
scorecard; a whole-page cold boot at 10 Mbit/s is 8 s on its own (6.7 MiB of wasm), which is
why the gate is the increment. **The measurement ends on the console line `tong: init
complete`** (correction 4), before the startup cards, which are a constant; `Page.loadEventFired`
fires long before the wasm has compiled and is not the signal. **The dry run also measures the
bank's own cost before v7 is built**: `tong.sf2`'s native load (the dependency cache's log line
on `play_song`) — the deployed v6 has no marker to time a boot on, so its boot is not part of
the dry run; the baseline arm is the comparison. `preload_assets` fetches the manifest's entries one after another, so a real user
pays a round trip per entry that the `npm run preview` host does not show — the measured
number is the localhost number, and the report says so. **Ruling (k), proposed: the bank is a
boot dependency until #194** — a fetch that fails fails the boot with "Failed to load assets",
as a sprite's does, where v6 had no bank to fail; the report and the scorecard name it. The preloader is **unchanged by this batch**: it has no retry and no deadline today — a
transient drop fails the boot at once, a stall holds the loading screen — and giving it both is
an engine-wide change every game inherits, with its own browser failure-path test and its own
ruling on the connections the pages support; rounds 36 to 41 designed it here and round 41
took it out as **#196**, its own batch. So ruling (k) is plain: the bank is one more asset a
dropped fetch can fail the boot on, as a sprite's always could.

**The browser method, dry-run on today's v6 before any bundle is built**: the two browsers
open on the session's display (`:1`, where batch 11's XTEST driver worked), keys reach them
through the same XTEST driver; Chromium is launched with `--remote-debugging-port`, and the
same DevTools Protocol session that sets the throttling (`Network.emulateNetworkConditions` at
10 Mbit/s **with `latency: 100` ms**, so the serial per-entry round trips the preloader pays
are in the number, `Emulation.setCPUThrottlingRate` 4, `Network.setCacheDisabled` for a cold
run) **reads the console through `Runtime.consoleAPICalled`** — the `tong: init complete` line
that ends the stopwatch and the `conductor=on` lines — rather than the process's stderr; Firefox runs with the pref
`devtools.console.stdout.content=true`, so its console lands in its stdout, and its throttling
is the Network panel's by hand with the CPU rate not available — **the throttled gate is
Chromium's; Firefox records unthrottled cold and warm boots, and a Firefox increment over 8 s
is flagged in the report without gating**, since its wasm compile and SF2 parse differ. **The
CDP order matters**: Chromium opens on `about:blank`, the script attaches, enables `Network`
and `Emulation`, sets the throttling and disables the cache, and only then sends
`Page.navigate` to the bundle — a browser launched on the bundle's URL fetches the wasm at
full speed before the script connects.
**Every browser run begins with a click on the canvas** — in Chromium through the protocol
itself (`DOM.getBoxModel` on the canvas, `Input.dispatchMouseEvent` at its centre), in Firefox
XTEST's mouse on the window brought to the front, as batch 11 pressed the toolbar; a page
navigated by the protocol holds no focus on the canvas — which is also the activation gesture — the engine's "Web audio enabled after user gesture" console line and the
song's start after it are the requested-before-gesture proof. The dry run is that click, one
key reaching the v6 title menu and one console line captured in each browser, quoted in the
report; a method that does not reach that point stops the batch before
the bundles are built. **The first gesture is a key, a click or a touch** (`engine_core::web
is_activation_gesture`): a player on a gamepad alone hears nothing until one — the engine's
standing web limitation since H7, filed as its own issue, not this batch's; the browser checks
use a key.

Browser checks, in Firefox and Chromium, **each a pass or a fail with a consequence**: a
functional check that fails (the song not heard after the gesture, a rally's hits not walking
the refrain, the jam silent, the pulse off the beat, the stress run over its gap criterion, the
preview-then-Play check, the SoundFont row) **stops the `--sync` into `insiculous_web/public/`**
— the bundles stay in the game's `dist/`, the report is INCOMPLETE naming the check, and the
planner and Jesse decide; nothing broken reaches the site's tree on a line in a report. The
checks, recorded in the report: **requested-before-gesture
startup** — the song is requested in `init` before any gesture and sounds on the first click
or key (the engine's first-gesture hook, batch 6's `PendingMusic`), with the title screen's
first menu press being that gesture; a rally's hits walk the refrain; the pause menu jams; the
pulse lands on the beat; the boot times above; `/playground/tong/` opens the game in the
editor with the song playing on Play, and in it File → New Song shows the SoundFont row listing
`tong.sf2` with no missing-bank warning (the row's wasm path, `vfs::list_files` over the
preloaded keys, exercised once on the real base string), and a song saved at `ai/music/
ai_tong.mid` there then Play and a match start reads the position at the top (the identity rule
on the two labels the editor and the game form, exercised once where both exist). **The browser stress run** (plan § Verification "before
any K6 batch starts"): batch 1's method — a two-minute run, frame time and underruns from the
`conductor=on` lines, one seek (a match restart) and one loop wrap — **on Tong's own v7
bundle in Insiculous mode with both balls** (ruling (g), proposed), **passing when no two consecutive telemetry windows show a drain gap over 92 ms — twice the
browser's 2048-frame pull, 46 ms, which is the backend's own floor (K1 measured 60.7 ms as the
longest gap of a healthy idle run, `K1_SPIKE.md:167`, so 61 ms would fail healthy audio on one
timer coarsening) — and the render cost stays under a block's duration over the run** (K1's
criterion in its own words, `K1_SPIKE.md:173-177`: no sustained underrun; the 61 ms
`DEVICE_PERIOD_ALLOWANCE` is the trigger's late tolerance, not a render budget) — read from **Tong's own `conductor=on` lines** (correction 4's telemetry — **every measurement
URL carries `?telemetry=1`**, or the lines are `debug` and the web logger drops them; **zero
lines after the click is a FAIL with the console transcript attached, never a reduction over an
empty set**; every line prints the maxima it takes, each line its own window, so the gate is
**the maximum over every line printed after the first match start**, reduced by the harness,
never the last line alone; **the throttling is reset before the stress run** —
`Emulation.setCPUThrottlingRate` 1, `Network.emulateNetworkConditions` off — since the boot
gate's 4× slowdown is not the stress run's condition; **and `blocks_rendered` must advance in
every window of the run**, since a stalled audio thread renders no block, records no gap and
swaps out zeros — a window without new blocks is a FAIL as an empty transcript is; the warm-up
is the song's first blocks after the activation click, so **the driver waits two telemetry
windows after the click before it starts the match**) — the longest gap and the game's windowed
p95 frame time quoted; **the XTEST driver restarts
the match at every game over** (a two-ball Insiculous match ends in about thirty seconds, and a
game-over screen measures nothing), each restart being one of the run's seeks: the realistic worst case
a game produces, where the K1 spike already proved decision 2's synthetic worst case in both
browsers (K1_SPIKE.md § 4, PASS). The demo's `--stress` is the synthetic case natively, with
the same log lines.

### Correction 7 — `training.md` § Conductor is completed, the roadmap row waits for batch 16

`training.md` § Conductor Pattern (77 lines today, the song, the sources, the same-frame
rule, scene sounds) gains #64's eight subjects as short paragraphs, each pointing at the doc
that holds the detail: **the authoring workflow** (the composer, File → New Song, the API's
`song` verbs, Save's journal and `LICENSE`, `ai/music` for an agent's first cut); **shared-part
modes** (`Loop`, `Sequence`, `AtSongPosition` in two lines each, the harmony caveat); **custom
instrument import** (a `.wav` dropped on the composer or imported on the web, `Sample(…)`, the
provenance it needs, `music/samples/<flat>/`); **source eligibility and lifecycle** (the three
sources, on-screen eligibility being the game's policy, despawn ending the source, re-entry
reactivating it); **crowd limits** (the 24 pending and 24 sounding per instrument, the 512
scheduled events, refused-whole triggers, the counters); **volume behaviour** (the music and
sfx buses: the background on music, every trigger on sfx, each alone at 0 keeps the other);
**beat timing** (`beat_crossed`'s once-an-update latch, the `latency_samples` a position
carries, the up-to-61 ms browser period); **pause distinctions** (the game's `time_scale`
pause against the song's `pause_song`, triggers dropped while the song is paused). The
Conductor Pattern's file list gains the example. `PROJECT_ROADMAP.md` row K and `CLAUDE.md`'s
"K1 gates all of Conductor" line are **batch 16's** (close-out), not this batch's — the old
section named the row; it moves. `CLAUDE.md`'s examples list and `training.md`'s directory map
name `conductor_demo`.

### Correction 8 — the screenshots are captured by the executor, and serve #184 too

Batch 11's executor captured the composer with `ffmpeg x11grab` on the session's display after
driving it with XTEST (report-11 § 11), so "no capture tool" no longer holds. This batch
delivers **three PNGs under `review/conductor/`** (engine): the composer at the full width with
the Tong song open and the preview playing (the playhead on the roll, the transport's
"Playing · bar …" text), the Song section with the provenance fields filled, and the compact
layout with the tabs — the same three #184 asks for, on the song this batch authors. Codex
reviews them with `--image` in the code round; **the executor also copies the three PNGs and the pulse stills to
`coordination/conductor/screenshots/batch-12/`**, a tracked directory the batch's commit carries,
so the evidence outlives `review/` (gitignored and cleaned) and #184 points at tracked paths — and #184 closes with the batch's commit if codex raises nothing the screen alone
shows, else its findings are the next batch's rows. The pulse is shown by **two or three PNG stills of a rally** at a pulse's peak and at rest
(the grid's lines under a grill displaced and flat), never a video: `review/` is gitignored
so nothing is committed either way, and a still pair is what codex can judge.

### Correction 9 — sizes, placements, gates

Files near the ceiling: none in `games/pong` (`types.rs` 472, `spawning.rs` 415, `jaw.rs` 407,
`scoring.rs` 386 gains about ten lines); the engine's `training.md` is prose. New files:
`games/pong/src/{music.rs, music_tests.rs}`, `games/pong/scripts/prepare_soundfont.sh`,
`games/pong/assets/{ai/music/ai_tong.mid, ai_tong.mid.ron, ai/music/LICENSE, soundfonts/tong.sf2,
soundfonts/LICENSE}`; `examples/conductor_demo.rs`, `examples/conductor_demo/enemies.rs`,
`examples/assets/{scenes/conductor_demo.scene.ron, music/ai_demo.mid, music/ai_demo.mid.ron,
music/LICENSE, music/samples/ai_demo/ping.wav, music/samples/ai_demo/LICENSE, soundfonts/sine.sf2,
soundfonts/LICENSE}`. Touched: pong `Cargo.toml` (the `music_midi` dev-dependency), `src/{lib,constants,types}.rs`,
`gameplay/{mod,scoring,flow}.rs`, `web_entry.rs`, `assets/locales/{en,pirate}.ron`, `CLAUDE.md`;
engine `Cargo.toml` (the `[[example]]` and **a `[dev-dependencies]` section — the root crate
has none today — with `engine_core = { path = "crates/engine_core", features = ["test-support"]
}`**: the example's test target compiles in the root crate, and `engine_core::test_support` is
gated on that feature, `engine_core/src/lib.rs:77-78`; examples build with the dev-dependencies, so `cargo run
--example conductor_demo` carries the feature too — which is harmless, `test-support` gating an
additive module and nothing else),
`crates/engine_core/src/prelude.rs` (the `Subdivision` re-export), `crates/audio/src/manager/
song.rs` (`song_label`, `take_song_maxima`, `SongSnapshot::log_line`),
`coordination/conductor/screenshots/batch-12/` (new, the PNGs), `docs/EDITOR_COMMAND_API.md` (the `song new` row no longer names `soundfonts/cast.sf2`,
the `AddTrack` and `SetSoundFont` lines, the `status` query),
`crates/editor/src/song/{mod,edit}.rs` and `edit/apply.rs` (`AddTrack`, `SetSoundFont`),
`crates/music_midi/src/song_data.rs` and `song_data/{edit,export}.rs` (`add_track`),
`crates/editor/src/composer/sidebar/blocks.rs` (the Add-track row), `crates/editor/src/{status_bar.rs,
theme/mod.rs, theme/tests.rs, command_api/mod.rs}` and `crates/editor_integration/src/editor_game/{song_io, song_preview}.rs`
with their tests and the composer's Song section (correction 10), `docs/EDITOR_COMMAND_API.md`,
`examples/soundfonts/{sine.sf2, LICENSE}`, `scripts/check_audio_provenance.sh` (the
`examples/soundfonts` pattern), `examples/conductor_background.rs` (uses `log_line`), `training.md`, `CLAUDE.md`; site
`src/content/games/tong.md`, `public/games/tong/v7/`, `public/playground/pong/v6/`.

**Gates:** in `insiculous_2d` the standard set (`cargo test --workspace` with the example's
tests in it — an example with `test = true` is built as a test and its unit tests run under a
plain `cargo test` — the Cargo reference, "Configuring a target", the `test` field: "Examples
are built by `cargo test` by default to ensure they continue to compile, but they are not
tested by default. Setting `test = true` for an example will also build it as a test and run
any unit tests"; and its `main` draws no dead-code warning under the test harness, as no
binary's does; a function the test build does not reach is exercised by a test or reorganised,
never `#[allow]`ed — the report quotes the `conductor_demo` test lines as the proof — both clippy runs, `check_wasm.sh`, `check_games.sh --test`, `check_guide_tests.sh`,
`check_features.sh`, `check_audio_provenance.sh` — which now also finds `examples/assets/music`
and `soundfonts` — the comment-tag grep, the 600-line check); in `games/pong` `cargo test`,
`cargo clippy --all-targets` with and without `--features editor`, **`../../insiculous_2d/
scripts/check_audio_provenance.sh "$PWD"`** from the game's directory — with no argument the
script walks the engine root (`check_audio_provenance.sh:19-23`), so the root is passed; the
executor breaks pong's `soundfonts/LICENSE` once to see the gate fail, restores it, and quotes
both runs in the report — the two `build_wasm.sh` runs with their `wasm size:` lines and the
bank's size in the report; in
`insiculous_web` `npm run verify`. The report carries the browser checks of correction 6 and
the screenshots' paths. **Leaves out:** a volume control in Tong, the recorded fallback (ruling
(e)), the fetch-after-first-frame lever (filed; the re-voicing is this batch's lever), a
`Sequence` part in Tong (ruling (i)), the roadmap row (batch 16), the other five games (K6), intensity layers and harmony generation (the milestone's
exclusions), Jesse's promotion of `ai_tong` to `music/`.

### Correction 10 — Jesse's playtest: a song that cannot load must not block Play, and a new song needs a bank it can find

Jesse's playtest of batch 11 (2026-10-22, `cargo run --example editor_demo --features editor`):
a new song made in the composer did not play on the transport's Play or Space, and the
editor's Play never started the game. The cause is in the tree: `editor_demo`'s asset base is
`examples/` (`EXAMPLES_DIR`), a new song's sidecar names `soundfonts/cast.sf2`
(`crates/editor/src/song/mod.rs:56 DEFAULT_SOUND_FONT_PATH`), and `examples/soundfonts/` does
not exist — so `prepare_song` failed ("Song preview could not play: …" in red on the status
bar) and, once the song was saved (`examples/music/untitled.mid` with its sidecar and the
`LICENSE` stanza, all correct), the saved file failed to load at Play and **the armed Play was
dropped** ("Play could not start the song: …"), as batch 11's contract said it should
(`song_preview.rs play_armed_song`, the pitfall row "a saved song that fails to load keeps the
preview Play parked, says why, and starts no session"). Reviewed five rounds and still a trap:
the editor's Play was hostage to the song's bank. Five changes, all in this batch (the last two from the second half of the report: the errors
were there, in red, unreadable to a red-green colourblind player and uncopyable — #195):

1. **A song that fails to load no longer blocks Play.** `play_armed_song`'s `Err` arm marks the
   pending Play `ready` all the same, **stops the preview's song first when the preview holds
   the slot** (`play_song` refuses before it touches the playing song, so a sounding preview
   would otherwise play on under a session that owns nothing — `audio.stop_song()`, the slot
   `Nobody`), so the next frame starts the session — **and what it hears depends on who held the slot**,
   which at that moment is one of two (Play is armed only from Editing, which the editor's Stop
   reaches with the slot `Nobody`, so `Session` is not a state this arm sees): a sounding preview
   is stopped, so the session is silent; and when the editor holds nothing (`Nobody`) whatever
   the game itself requested at `init`, Tong's own song in the editor bundle, keeps playing, since
   Stop stops the editor's songs and never the game's. The error
   stays on the status bar ("Play could not start the song: …; the game runs without it"); the
   parked preview stays parked and silent; two tests, one per holder. The test
   `a_saved_song_missing_a_sample_keeps_the_preview_shows_the_error_and_starts_no_session`
      becomes `…_shows_the_error_and_starts_the_session_without_music`, a second test starts from
   a sounding preview and asserts silence (`is_song_playing()` false) through Play and Stop, and
   the pitfall row in `editor_integration/CLAUDE.md` says so. Stop while that session runs
   flushes the sources and touches no slot.
2. **A new song's bank is one the project has, and the bank can be changed.** `new_song`
   (`song_io.rs`) sets the sidecar's
   `soundfont` to the first `.sf2` under `<asset base>/soundfonts/` in sorted order — listed
   through **`common::vfs::list_files`**, each path stripped of the asset base so the sidecar
   holds `soundfonts/x.sf2` as every sidecar path is, the seam
   that reads the directory natively and the preloaded `MemFs` keys on wasm (never
   `std::fs::read_dir`) — and to `DEFAULT_SOUND_FONT_PATH` only when there is none. The
   composer's Song section gains a **"SoundFont: soundfonts/x.sf2" row** that cycles through
   the project's `.sf2`s as the Loop switch cycles — **drawn disabled, naming the sidecar's path,
   while the project has none** (no cycle over an empty list, no modulo by zero), the warning
   below it saying how to add one — each click one **`SongEdit::SetSoundFont {
   path }`** (a new variant: `apply` sets `file.soundfont`, `try_merge` replaces like `SetLoop`,
   the API's edit table and `EDITOR_COMMAND_API.md` gain the line, undo covers it), and a
   warning row — the sidebar's `walk.warning`, as the provenance mark — "No SoundFont at
   soundfonts/cast.sf2 — put an .sf2 under soundfonts/ and pick it on the SoundFont row" (how a
   file gets there is the host's: the asset folder natively, the file input on the playground
   app, and nothing on the read-only editor bundle, where the warning cannot show because the
   bundle's bank is preloaded)  whenever the sidecar's `soundfont` resolves to no
   file under the asset base, from the moment the song exists; a drop of an `.sf2` on the
   composer adds an instrument (`composer::drop_action`) and does not change the background
   bank, which is why the row exists; **the row checks the file's existence and nothing of its
   contents** — the editor crate carries no synthesizer and cannot read a bank's preset list, so
   a bank lacking an instrument's preset fails the next preview with the loader's error naming
   the instrument and the preset (prefixed, amber and logged by item 4), and the Instruments
   section shows each instrument's bank and program beside it — and the row lists what `vfs::list_files` finds — on the
   web the preloaded keys and every file the page imported, since `playground_write_file_bytes`
   writes into `MemFs`; natively **real files only** (`list_files` skips symbolic links,
   `vfs/mod.rs:194-196`), and when the directory holds entries but no listed bank the warning
   says "banks must be files, not links". Tests: a project with `soundfonts/a.sf2` and `soundfonts/b.sf2` gives a new song
   `a.sf2`; one with none gives the default and the warning; the row's click sets `b.sf2` as
   one undoable edit; the warning clears when the named file appears (a rescan) or the row
   picks one that exists.
3. **`editor_demo` gets a bank**: `examples/soundfonts/sine.sf2` (the fixture's sine bank, a
   copy, with its `LICENSE` entry carried) under the example's asset base, so File → New Song
   in the editor demo previews and plays out of the box; the conductor demo's own copy under
   `examples/assets/soundfonts/` (correction 5) serves its different base. The provenance gate
   scans `*/assets/soundfonts`, which `examples/soundfonts` is not, so **the gate's `find`
   gains `-o -path './examples/soundfonts'`** (one line, with a fixture pair under
   `scripts/tests/audio_provenance/` as batch 11 added for `ai/music`), so the bank is held to
   the same `LICENSE` as every other.

4. **An error on the status bar is marked by more than its colour, holds, and is logged.** Jesse
   could hardly read the two errors above — red text (`theme.error_red`, `0xff4444`) on the
   dark bar, which a red-green colourblind reader sees as dark, and the error told from an
   ordinary message by colour alone (WCAG 1.4.1). `StatusBar::show_error` prefixes the shown
   text with "Error: " (the prefix is drawn, not stored, so callers and tests keep their words),
   the status bar's error text takes a **new token `status_error`**, an amber at ≥ 4.5:1 on
   `surface_0` and `surface_1` with a guard test beside the surface ladder's (`theme/tests.rs`)
   — `error_red` itself is untouched: it is also the invalid field border (`theme/mod.rs:380`),
   the palette's `error_color` (:339) and the asset browser's failed-load tint, and a music
   batch recolours none of them — and `show_error` logs the message at `error`
   level so the terminal (native) and the console (web) carry it; **an error is not displaced by the engine's own notices** — the automatic messages that land
   a frame after Play — the session start's snapshot warning (`play_session.rs:70`) and the API
   batch's "API batch committed by Play" (`play_session.rs:96`) — go through a `show_notice` that
   leaves a persistent error in place (tested with an open API batch); every message a user's own
   action produces (a save, a selection, a tool) replaces the error as it always has, so no
   feedback is ever suppressed and the load error survives the frame that would have erased it. Codex judges the colour.
5. **The message can be read back.** The command API gains the `status` query — `{"message":
   "…", "error": true}` or `{"message": null}`, the message run through the same formatter that
   draws it, prefix included — so an `--api` session, an agent or the playground's command input
   reads the text the bar shows; `docs/EDITOR_COMMAND_API.md`
   gains the row. Copying to the OS clipboard is a dependency decision and stays in #195, which the
   planner edits at the batch's close to reference this query and keep only the clipboard and
   the live-region halves.

Until the batch lands, Jesse's session works with the spike's bank copied in:
`cp coordination/conductor/spike/assets/soundfonts/cast.sf2 examples/soundfonts/` (34 MiB,
untracked, never committed).

**Rulings for Jesse** (sent with the section unless he says otherwise): (e) no recorded
fallback in Tong; (f) the song plays on under the pause menu, the jam over it; (g) the browser
stress run is Tong's own bundle in Insiculous mode, the synthetic case staying K1's; (h) the
Toreador presets — trumpet, trombone and timpani, no strings — a committed bank of about 2 MiB
in the game repo, the accompaniment a brass band's; (i) the hits and the jam
on `quantize: Off`, the jam a `Loop` part — #64's "Sequence jam" exhausts in a few presses;
(j) a song that fails to load no longer blocks the editor's Play — the session starts without
the editor's music (a sounding preview stopped; whatever the game itself plays, its own song in
the editor bundle, plays on) with the error on the status bar (correction 10, reversing batch
11's reviewed contract); (k)
the bank is a boot dependency on the web until #194 — a dropped fetch fails the boot as any
asset's does today, and the preloader's retry is #196's batch, not this one's.

## Batch 13 — K6: Tong's acceptance and The Food Pyramid (#65) — DONE 2026-10-24 (58a0601; pong 121d1ba; breakout 0b50546; site 59b3cd6)

**Re-verified 2026-10-23 against the tree at 02760a7** (engine), `games/pong` d0a8240,
`games/breakout` 8f7b109 and `insiculous_web` db162d3, after batch 12 and ruling 3b's door (b)
landed; **v6 after plan rounds 52 to 56** (52: codex 5, kimi 5 — codex's five, read from the
shared directory — gemini 6; 53: codex 1, kimi 7, gemini 6; 54: codex 3, kimi 4, gemini 6; 55:
codex 2, kimi 4, gemini 7; 56: codex 2, kimi 4, gemini 4; `rebuttal-52.md` to `rebuttal-56.md`;
**the planner recommends settling here** — no contract has moved since v3, and rounds 54 to 56
hardened gates). **Settled at v6 by Jesse on 2026-10-24**, when he asked for the handoff;
rulings (l) to (s) at the section's foot go out as proposed, his to reverse before the executor
starts. Ten corrections to the one bullet the section was; each names what the tree says.

**Landed 2026-10-24, listen check pending.** Built by the Opus executor session
(`review/conductor/report-13.md`, COMPLETE: the march sequenced from memory, the gates clean, the
cold boot 19.22 s under the 25 s ceiling, THE GAUNTLET's two minutes over 108 bricks and 84 paddle
hits, every browser check a pass in Chromium and Firefox, the editor bundle in both); reviewed as
round 57 (engine kimi 3, gemini 6, planner 3; pong kimi 1; breakout kimi 2, codex 0 with the
stills; site kimi 0, codex 0 — `rebuttal-57.md`: five doc lines accepted, eight rebutted, gemini's
two blockers false against `game_config.rs:23` and the passing fourth-arm test); committed on
`jesse` as engine 58a0601, pong 121d1ba, breakout 0b50546, site 59b3cd6. **DONE 2026-10-24:
Jesse's listen checks passed, The Food Pyramid's and Tong's after the migration**; #65's Pong and
Breakout boxes ticked, breakout#3 told the `clear` binding is the cadence its cameo inherits, and
the gesture-order correction written into Batches 14–15 (report § 8.3: `enable_output` logs
`Playing song:` before `Web audio enabled after user gesture`, on one instant — check "no song
before the gesture, both lines on it", never an order). Filed: insiculous_2d#202 (the pause
hint over the 300 px panel, both games), insiculous_web#76 (the profile prompt takes a running
game's keyboard), game-template#3 (a `GameMusic` in the template), a line on backlog #84
(`stop_binding` dead under `--no-default-features`).

Executor: the Opus session, in four checkouts — `games/breakout` (the game, its song, its bank,
its page versions), `games/pong` (the migration and three proofs), `insiculous_2d` (the shared
music module, the shared bank script, `training.md`), `insiculous_web` (the two bundles and two
content lines) — with **one report, the engine's `review/conductor/report-13.md`**. Reviewers:
kimi (everything), gemini (the engine module, the harness tests, the bank script), codex (the
beat pulse under the paddles, the two page sentences). Listen check: Jesse, natively and in
Firefox and Chromium, for The Food Pyramid; Tong's passed in batch 12 and is heard again
natively after the migration (the deployed Tong bundles keep batch 12's bytes, ruling (s)). Size: about +1,100 lines across the four repos plus the assets (a song, its
sidecar, a 2.2 MiB SoundFont subset); the games' diffs are reviewed from
`games/breakout/review/conductor/` and `games/pong/review/conductor/`, the engine's and the
site's from `insiculous_2d/review/conductor/`, each reviewer dispatched from the repo it reads.

**Standing from batch 12:** ruling (e) — no recorded fallback; (f) — the song plays on under the
pause menu and the jam reads `MenuInput`, never the selection; (h) — a committed per-game bank
with its `LICENSE`; (i) — hits on `quantize: Off`, the jam a `Loop` part; (k) — the bank is a
boot dependency on the web until #194; and **the K6 rule** (ruling 3b, door (b)): a game starts
its song where it naturally would and asks the audio for its state rather than trusting a flag,
since the editor stops and replays the slot around it.

### Correction 1 — Tong's acceptance is a migration and three proofs, not new sound

#65's per-game list, read against what batch 12 shipped for Tong: the song, the three bindings,
the pause jam, the beat visual, the browser build, the tests, the provenance and Jesse's ear are
done (`games/pong/CLAUDE.md` § Music; `report-12.md` § 4, § 5, § 8). What the list still asks
and the tree does not show is three things, none of them sound:

- **The host's lifecycle, stated.** Tong's trigger sources are the two tongs — spawned once in
  `init` (`src/lib.rs:110-115`) and never despawned; `start_game` (`gameplay/flow.rs:68-85`)
  resets positions and jaws and spawns no paddle — the game and the menu. No scene is loaded
  (`request_transform_hierarchy_reset` has no caller in the game), so the host's reconcile
  never ends a Tong source and the flush path is the editor's world replacement on Stop, which
  the engine's own tests hold (`tests/song_triggers.rs
  a_world_replacement_flushes_every_source_and_the_next_trigger_activates_anew`). That is the
  acceptance: **nothing in Tong despawns a source, by construction**; `CLAUDE.md` § Music gains
  the sentence. The Food Pyramid is where the lifecycle is exercised (correction 6).
- **The same-frame order, tested where it is a function.** `handle_paddle_hits_and_collect_goals`
  (`scoring.rs:108-150`) walks the frame's collisions in the order the physics step reported
  them and, inside each, the balls in `all_balls` order and the tongs left then right; the
  hits are triggered in that order (`spawn_paddle_hit_visuals`, :190-194) **before** any goal
  (`concede_goal`, :237), because `update_gameplay` runs the hit pass before the scoring pass.
  The order within a kind follows the physics step's contact order, which is rapier's and not
  the game's to pin; the order **between kinds** is the game's — one test: a frame holding a
  paddle hit and a goal (two balls) triggers `hit` then `goal`, observed as the `refrain`
  cursor's two units going trumpet-then-timpani — which **no conductor counter shows**
  (`SongSnapshot` keeps no per-trigger log, `music_midi/src/position.rs:37-70`), so **the
  order is observed at the game's own seam**: `Music` carries a `#[cfg(test)] triggered:
  Vec<&'static str>` that every moment pushes its binding name onto and `update` clears each
  frame — production's call order recorded, never reconstructed — and the test lands a paddle
  hit and a goal in one physics step (two balls, placed as `music_tests.rs:91-131` places
  them) and asserts the frame's trace is `["hit", "goal"]` beside the two accepted triggers on
  two sources (round 52, codex F5; round 54, kimi F2 — #65's "tested stable same-frame order"
  is this test and its twin in the Food Pyramid, correction 6). No new engine query is added
  for a test's sake.
- **The song reopens in the composer.** The executor opens the shipped song through the API
  (`song open ai/music/ai_tong.mid` on the pong project, as `report-12.md` § 4 drove `song new`),
  quotes the `song` answer (five tracks, two parts, three bindings, the provenance) and closes it
  **without saving** — the proof that #65's "reopen/tweak the song in K8" is possible on the
  committed file; the tweak itself is Jesse's pass, outside the batch.

And the code half: Tong's `music.rs` moves onto the engine's shared module (correction 3), every
one of its ten tests staying green, none deleted — **and the migrated Tong reaches a browser**
(round 52, codex F2): the executor rebuilds Tong's two bundles after the migration
(`scripts/build_wasm.sh ../games/pong tong --version v7` and `--kind editor --version v6`, into
the game's own `dist/`, **no `--sync`**), copies them into the site's built `dist/games/tong/v7/`
and `dist/playground/pong/v6/` under the one `npm run preview` — the way batch 12 served its
`v99` arm, never into `public/`, and **as the last staging step of correction 7's order** — after
the Food Pyramid's bundles are synced, the content lines edited and `npm run build` run once, so
no later build overwrites them (round 53, gemini F5; round 56, kimi F1), the executor quoting
`sha256sum` of each copied `game_bg.wasm` against the game-local build's — and runs Tong's
functional checks on them in Chromium: the
song on the title's first key, a rally's hits walking the refrain, the pause jam, the pulse,
and a `conductor=on` telemetry line in batch 12's exact shape; the two `wasm size:` lines are
quoted. **The deployed v7 and v6 keep batch 12's bytes**: a version directory is immutable
once deployed (`build_wasm.sh`'s header), the migration changes no behaviour, and the next
Tong bump ships the migrated code — the standing pattern for a live bundle behind its source
(#118, #159). **Ruling (s), proposed:** that, rather than a v8 / editor v7 bump for a refactor.
The #65 checklist's Pong and Breakout boxes are ticked by the planner at the done mark, after
Jesse's listen check.

### Correction 2 — The Food Pyramid has no sound today

`grep -rn -i "sound\|music\|audio\|beep" games/breakout/src games/breakout/README.md` prints
nothing; `assets/` holds `fonts`, `scenes` and `sprites`. As Tong (batch 12, correction 1):
nothing is gated behind a binding replacing a beep, there is no recorded fallback (ruling (e)
extends), and a song that fails at `init` leaves the game as silent as it is today, every
trigger refused and counted.

### Correction 3 — the game-side music module is promoted to the engine, and Tong migrates

`games/pong/src/music.rs` (272 lines) is nine tenths rules every game will carry: the request
at `init` with its once-logged failure, the match-start identity rule (`restart`: none →
request, own → seek to the top, another → leave it), the pause-jam latch over `MenuInput`
(`follow_pause`), the beat pulse's bar-first-else-beat read, the telemetry line (the frame-time
ring, the maxima taken, `?telemetry=1` / an environment variable), the once-a-second refusal
count. The Food Pyramid is the second game; #65 says "promote duplicated integration helpers
to engine_core when actual shared usage justifies it", and the house rule says a rule lives in
one place. **Ruling (l), proposed: promote now**, in this batch, so the Food Pyramid is the
first consumer rather than the first copy.

**`crates/engine_core/src/game_music.rs`** (new, under 300 lines, `pub mod game_music` in
`lib.rs`, `GameMusic` and `BeatPulse` re-exported from the prelude beside `SoundSources`):

```rust
/// A game's song: requested at `init`, re-aimed at every match start by the slot's identity,
/// jammed from a pause menu, pulsed on the beat, and reported every five seconds.
pub struct GameMusic { /* private: song_path, log_prefix, telemetry_env, telemetry_at_info,
    song_failed, failures, refused_seen, frame_times: FrameRing, was_paused, jam_units, pulsed,
    seconds_since_telemetry, seconds_since_refusals */ }

pub enum BeatPulse { Bar, Beat }

impl GameMusic {
    /// `song_path` relative to the asset base (`"ai/music/ai_tong.mid"`), `log_prefix` the
    /// game's name in its lines (`"tong"`), `telemetry_env` the native switch (`"TONG_TELEMETRY"`).
    pub fn new(song_path: &'static str, log_prefix: &'static str, telemetry_env: &'static str) -> Self;
    /// The joined path `start` requests and `restart` compares the slot against.
    pub fn song_path(&self, ctx: &GameContext) -> String;
    /// Request the song (web: it waits for the first gesture), drop the boot's maxima.
    pub fn start(&mut self, ctx: &mut GameContext);
    /// The identity rule, verbatim from Tong's `Music::restart` — which also clears the pause
    /// latch, so a Restart taken from the pause menu does not read the next pause's open
    /// frame as an already-open menu (Tong's `music.rs:133`; round 54, gemini F6).
    pub fn restart(&mut self, ctx: &mut GameContext);
    /// The pause menu's frame: a move up or down triggers `action` on `ctx.sources.menu()`
    /// — the one trigger the helper sends itself, since the menu's source is the engine's
    /// and the latch is the rule; the open frame, confirm, cancel and the pointer jam
    /// nothing. Returns whether a unit was jammed, so the game may count it.
    pub fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool, action: &str) -> bool;
    /// The beat this frame crossed, bar first: the game applies its own visual to it.
    pub fn beat_pulse(&mut self, ctx: &mut GameContext) -> Option<BeatPulse>;
    /// Every frame: the frame time into the ring, the telemetry line every five seconds with
    /// the game's own counts appended (`counts` is called only when a line prints), the
    /// refusals once a second at debug.
    pub fn update(&mut self, ctx: &mut GameContext, counts: impl FnOnce() -> String);
    pub fn song_failed(&self) -> bool;
    pub fn failures(&self) -> u64;
    pub fn jam_units(&self) -> u64;
    pub fn pulsed(&self) -> u64;
}
```

What stays in each game: the binding names and the song path (constants), the moments that
trigger (`ctx.sources.trigger` called by the game at each one — the helper has no general
trigger method, since every game names its own moments and counts them; **the one trigger the
helper sends is the jam**, inside `follow_pause`, on the menu's source — round 52, gemini F1,
where v1 said both things), the counts it appends, the pulse's anchors and strengths. `FrameRing` moves whole, private; `telemetry_asked` is `cfg`-split inside the
module (wasm: `web::query_param("telemetry") == Some("1")`, the module is in the crate that
owns `web`; native: `std::env::var(telemetry_env) == Ok("1")`). The telemetry line is
`"{snapshot.log_line()} frame_p95={:.2}ms frame_mean={:.2}ms {counts} refused={}"` — Tong's
shape with the game's counts where Tong's `hits=… goals=… jam_units=… pulsed=…` sat, so Tong
appends `hits={} goals={} jam_units={} pulsed={}` and the line batch 12's harness read is
unchanged byte for byte. **Tong's `Music` shrinks** to `{ game: GameMusic, counts: MusicCounts }`
with `start`/`restart`/`update` delegating, `hit`/`goal` counting and triggering as today,
`follow_pause` counting a jam the helper reports, `pulse` mapping `Bar` to `HIT_RIPPLE_STRENGTH`
and `Beat` to its quarter under the grills; `song_failed` and `failures` read through the
helper (`music_tests.rs` reads both — the test lines change from the field to the accessor,
nothing else). `TONG_TELEMETRY` keeps its name.

**Tests.** `crates/engine_core/tests/game_music.rs` (new, `GameHarness` over a minimal `Game`
holding a `GameMusic` on the sine fixture song, `tests/song_triggers.rs` the model) pins the
rules where they now live: the identity rule's four arms (an empty slot is requested into; the
own song is sought to the top; another playing song is left alone; another song that played to
its end is replaced), the open-frame jam latch (the frame the menu becomes active jams nothing,
one move one unit, confirm nothing, a reopen on the frame after a resume nothing), the pulse's
bar-before-beat and its once-an-update latch, a missing song counted as failed and every later
`restart` retrying it. Tong's ten tests **stay** — they pin the game's wiring through the real
menus, the real grills and the committed sidecar, which the engine's test cannot — so no test
is deleted and the report shows no deletion grep for one. `crates/engine_core/CLAUDE.md`'s file
map gains the module and its pitfall table two rows (the identity rule, the open-frame latch),
each citing a test in the new file; `training.md` § Conductor Pattern gains a paragraph **"A
game's music module"** after "In the editor", and the directory map a line.

### Correction 4 — the bank script is shared too, and the Food Pyramid's bank is Aida's brass

`games/pong/scripts/prepare_soundfont.sh` is 90 lines of which three are Tong's (the bank's
stem, the preset list, one comment). A second copy with three other lines is the drift the
house rule names. **`insiculous_2d/scripts/prepare_game_soundfont.sh [--bank path]
<assets/soundfonts dir> <stem> <bank:program>...`** takes the whole of pong's script — the
Debian package and version, the pinned sha256, `apt download` then snapshot.debian.org, the
sha256 refusal, `subset_soundfont.py`, the `LICENSE` in the spike's shape with both sha256s —
**with `--connect-timeout 20 --max-time 900` on every `curl`, `apt download` under `timeout
300` with its expiry taken as the signal to try snapshot.debian.org**, and a failure message
naming `--bank` as the offline path (round 53, kimi F7, and round 54, kimi F4: pong's script
bounds neither, so a stalled mirror would hold the batch) — and **pong's script becomes a
wrapper**: the stem `tong`, the three presets, **`"$@"` passed through
first** — `exec "$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" "$OUTPUT_DIR" tong 0:56
0:57 0:47`, so a caller's `--bank <path>` lands where the engine script parses its leading
options, not after the presets where it would be read as one (round 55, gemini F4) — `exec`ing
the engine's. The executor fetches the package **once**, keeps the extracted `FluidR3_GM.sf2`
in its scratchpad for the batch, and passes it with `--bank` to both wrappers, so the download
of about 130 MiB is paid once and not per game (round 55, gemini F3; a cache inside the script
is not built for two runs). The executor re-runs the wrapper and shows `sha256sum
assets/soundfonts/tong.sf2` equal to the `LICENSE`'s line and `git diff --stat
assets/soundfonts/` empty — the proof the promotion changed no byte. The `LICENSE`'s sentence
"Re-running scripts/prepare_soundfont.sh reproduces it byte for byte" stays true of the wrapper.

**The Food Pyramid's bank: `games/breakout/assets/soundfonts/food_pyramid.sf2`**, FluidR3 GM
**0:56 Trumpet (226 KiB), 0:57 Trombone (438 KiB), 0:58 Tuba (190 KiB), 0:47 Timpani (1,361
KiB)** — 2,215 KiB of sample data by `subset_soundfont.py --sizes` on the spike's cast bank
(measured 2026-10-23; the cast bank holds all four), about 2.2 MiB written — the Triumphal
March's brass and drums, no strings and no kit for batch 12's reasons. **Ruling (m), proposed:
these four presets, the bank committed, and a hard cap of 4 MiB on the file** — the cap is
what replaces batch 12's boot-increment measurement (correction 7), **and the shared script
enforces it**: `prepare_game_soundfont.sh` takes `--max-mib <n>` among its leading options
(each wrapper passes 4) and, for a bank over it, deletes the output, writes no `LICENSE` and
exits 1 — so a later preset change trips the cap where the bank is made, not in a report
nobody rereads (round 56, kimi F4). `games/breakout/scripts/
prepare_soundfont.sh` is the second wrapper (stem `food_pyramid`, four presets). FluidR3 is not
installed on Iroh (`apt-cache policy fluid-soundfont-gm`: none installed, 3.1-5.3 the candidate)
and the spike's `cast.sf2` is a subset whose sha256 the script refuses, so the script fetches the
package as it did in batch 12; `--bank` is for a machine that has the file.

### Correction 5 — the song: Verdi's Triumphal March, its shape, and the two modes Tong left

`games/breakout/assets/ai/music/ai_food_pyramid.mid` + `.mid.ron`: the *Triumphal March* from
*Aida* (Verdi, 1871; public domain) in a short arrangement the executor sequences **in the
composer on the breakout project**, as batch 12 did on pong's (`report-12.md` § 4 is the worked
record: `song new`, `AddTrack` for every track past `melody`, `AddNote`, `SetTempo`, `AddPart`,
`AddInstrument`, `SetBinding`, `SetProvenance`, the `song` and `song notes` answers checked
against the declared shape and quoted, then `song save ai/music/ai_food_pyramid.mid`). **The
bank is built before `song new`** (correction 4 before this one): `new_song` takes the project's
first `.sf2` under `soundfonts/`, so a project without one gets `soundfonts/cast.sf2` and the
missing-bank warning. **The executor cannot hear** (`report-12.md:310`, `:368`: "not heard; I
cannot hear"), so it sequences from the score, checks the arrangement by `song notes` and the
exported MIDI's readback, and declares the shape in the report; the ear that tunes it is Jesse's,
in the listen check, and a re-voicing he asks for is a follow-up through the composer (round
53, kimi F4). Shape:

- **sixteen bars** of 4/4 at about **112 BPM** (the march's tempo; a loop of about 34 s), in a
  key the bank's brass sit well in (the original's A♭ major, or the executor's transposition,
  declared), looping at a phrase boundary for batch 12's reasons (the wrap restarts every
  `Loop` cursor, so the fanfare realigns with the chords each loop);
- **background**: `chords` (trombone 57) and `drums` (timpani 47), the march's accompaniment,
  on their own tracks — a track a part selects is a trigger track and plays in no background
  (`music_midi` `BackgroundScore`: "what export would write, minus trigger tracks"), so the
  background and the bass line below are different tracks even where both would be the tuba's;
- **track `melody`** (`song new`'s own, trumpet): the fanfare, the **one shared part `march`**
  (`Loop`, `quantize: Off`), played by **two bindings on two instruments — `brick` on the
  trumpet and `armor` on the trombone** — a destroyed brick and a foil-wrapped one that survived
  its hit walk one tune, the survivor's note the muted echo of the fanfare's next unit: #65's
  "multiple enemy types share one part and select different instruments", literally — **a
  binding names a part for its notes and an instrument for its voice, and every instrument is
  its own synthesizer slot** (decision 2; `triggers.rs`), so the trombone's `armor` and the
  trumpet's `brick` read one track and touch neither its channel nor each other's timbre,
  exactly as Tong's `hit` (trumpet) and `goal` (timpani) share `refrain` today
  (`games/pong/assets/ai/music/ai_tong.mid.ron`, heard by Jesse in batch 12);
- **track `bass`** (tuba 58): the march's bass line written under `chords`, the part **`bass`
  in mode `AtSongPosition`** on the `paddle` binding — a paddle hit plays the bass note written
  for that moment of the march, always in the chord, never advancing a cursor (`SONG_FORMAT.md`
  § Modes: "harmony that always fits"), and a hit in a rest plays nothing, which a bass line
  with no rests avoids — **written gap-free from tick 0 to the loop's last tick**, each unit
  ending where the next begins, so no position in the loop is a rest (round 55, gemini F7),
  **held by a test over the committed MIDI**: `music_tests.rs` reads the `bass` track through
  `music_midi` and asserts its units tile the loop — the first at tick 0, each unit's end equal
  to the next unit's start, the last ending at the loop's last tick (round 56, kimi F3) — the mode Tong left undemonstrated, demonstrated where it fits;
- **track `dirge`** (timpani 47, a second track on the program, its own channel): **one low
  stroke, one onset** — a single note or a dyad — the part `loss` (`Loop`, `Off`) on the
  `loss` binding, one unit a lost ball. Not v2's "two-note fall": a unit is "the notes that
  start on one tick of one track" (`music_midi/src/song_data.rs:64-68`), so two successive
  notes are two units and alternate losses would hear one note each (round 53, kimi F1);
- **track `stinger`** (trumpet 56): the march's final chord as **one unit**, the part `clear`
  (**`Loop`, `quantize: Off`** — v2 quantized it to the beat, and round 53 (kimi F2) found the
  seam: a loop wrap drops every scheduled trigger (`SONG_FORMAT.md` § Modes: "a loop wrap, a
  seek and `Play` release every triggered note and drop every scheduled one";
  `conductor.rs:466-470`), so a clear in the loop's last beat would be parked on the grid line
  that is the loop end and never heard, one clear in sixty-four, silently. On `Off` the chord
  sounds the instant the last brick breaks, with its burst, and **every binding in this game is
  `Off`** — one rule. The seam is filed as an engine question for K3's follow-ups,
  insiculous_2d#201, not this batch's. **One limitation stands and is stated**: a wrap releases
  every triggered note (`conductor.rs:466-467`, `release_slots`), so a clear in the loop's last
  half-beat hears its chord cut at the seam — about one clear in thirty for a half-note
  stinger; the same seam as #201, recorded there, and not this batch's to close (round 54,
  codex F1)) on the `clear` binding. **`Sequence` is not this game's either**, and the reason is stated so
  it is not asked again: a `Sequence` part consumes its units by song time as well as by
  trigger ("skipping units whose time is behind the song", § Modes), so it suits a phrase
  pinned to a moment of the song, and a board clear falls anywhere in the loop; written at the
  loop's end it is silent until bar 15, written at its start it is exhausted after one beat.
  **One unit, full stop** — v1 allowed a V–I pair with the second triggered on the next beat
  crossing, and round 52 (codex F1) showed the trap: the crossing is read in gameplay code that
  stops running at `GameOver`, so the second half would never fire, or a pending one would
  leak into the next match. The march plays on under the game-over panel as Tong's does under
  its menus (the song is the game's, not the match's — batch 12, correction 4); the stinger is
  the tonic chord over a march whose bars are mostly tonic and dominant, so the clash is
  Jesse's ear to judge in the listen check, and if he wants silence after the
  stinger a `stop_song` on the win is a two-line follow-up the identity rule already supports
  (an empty slot is requested into at the next match start) — not built unless asked;
- **track `jam`** (trumpet 56): the counter-line, the part `jam` (`Loop`, `Off`) on `menu`, as
  Tong's;
- `soundfont: "soundfonts/food_pyramid.sf2"`; instruments `trumpet` (0:56), `trombone` (0:57),
  `tuba` (0:58), `timpani` (0:47), all from the one bank; `provenance: (author: "Giuseppe Verdi,
  1871; sequenced by the Opus executor for Be Insiculous, 2026", license: "CC0-1.0")`; no
  sample instrument.

Seven tracks past the tempo track, six bindings, four instruments; `AddTrack` takes the next
free channel 1–9 and 11–16 (batch 12's edit, `SongEdit::AddTrack { name, channel, program }`),
so seven fit. The sidecar is the composer's output, never hand-edited; `music_tests.rs` asserts
its structure (correction 6). The file sits under `ai/music` with the `ai_` prefix because an
agent sequenced it; Jesse's pass promotes it.

### Correction 6 — the game's code: six moments, two source kinds, the paddles' lifecycle

- **`src/music.rs`** (new, under 150 lines): the music constants at its top — **not in
  `constants.rs`, which is 590 lines, ten under the ceiling** (`SONG_PATH = "ai/music/
  ai_food_pyramid.mid"`, the six binding names, `BEAT_PULSE_FRACTION`, the pause hint) — and
  `Music { game: GameMusic, counts: MusicCounts { bricks, armor_hits, paddle_hits, losses,
  clears, jam_units, pulsed }, #[cfg(test)] triggered: Vec<&'static str> }` on `BreakoutGame`
  (`types.rs`) — the trace every moment pushes its binding name onto and `update` clears, test
  builds only, so a test reads production's order (correction 1) — with:
  `start(ctx)` at the end of `init` (`lib.rs:356`, after `rebuild_playfield`) followed by
  **`log::info!("food pyramid: init complete")`**, the boot marker; `update(ctx)` at the top of
  `update` (`lib.rs:364`); `restart(ctx)` at the end of `start_game` (`menu.rs:150-178` — every
  match start takes that function: the level select, the pause menu's Restart, the game-over
  launch); `brick(ctx)` → `ctx.sources.trigger(ctx.audio, "brick", ctx.sources.game())` from
  `destroy_brick_entity` (`bricks.rs:107-140`, beside the burst and the ripple — the game's
  source: the entity is destroyed in that call, and a brick's death must ring on); `armor(ctx,
  brick)` → `"armor"` on **`ctx.sources.entity(brick)`** from the `Damaged` arm (`bricks.rs:
  85-99`) — the brick's own source, so a survivor's note is released when a second ball kills
  it mid-note, the host's reconcile doing the releasing; `paddle(ctx, paddle)` → `"paddle"` on
  **`ctx.sources.entity(paddle)`** from `check_paddle_hits` (`paddles.rs:317-362`, beside the
  burst; a chomp is the same binding) — the paddle bodies are destroyed and respawned by
  `rebuild_playfield` at every match start (`spawning.rs:224`), so the old paddle's source ends
  in that frame's tail and the new one activates on its first hit; `loss(ctx)` → `"loss"` on
  `game()` once per lost ball from `check_ball_loss` (`balls.rs:139-158`, beside the splash;
  two balls lost in one frame are two); `clear(ctx)` → `"clear"` on `game()` from
  `check_win_condition` (`flow.rs:50-60`); `follow_pause` and `pulse` in `update_gameplay`
  (`gameplay/mod.rs:49-70`) exactly as pong's `gameplay/mod.rs:55-78` places them — the
  `Resumed` arm calls `follow_pause(ctx, false, MENU_BINDING)` before returning, the gate's
  end calls `follow_pause(ctx, pause.is_active(), MENU_BINDING)` and counts a `true` as a jam
  unit, and `pulse` runs after the gate while Serving or Playing and not paused.
- **The beat pulse**: `Bar` → the paddle-hit ripple (`GRID_IMPULSE_PADDLE_HIT_STRENGTH`,
  `_RADIUS`) under each live paddle — `self.paddle` and `self.paddle_top` are
  `Option<EntityId>` (`types.rs:238-240`), solo's top is `None` and is skipped, as Tong flattens
  its `[Option<Vec2>; 2]` — `Beat` → a quarter of it — the counter breathing under the tongs on the march's beat, the one surface that reads
  "pulse" without touching the art, as Tong's grills (bar full, beat a quarter, at 120 BPM —
  the shape Jesse heard and saw in batch 12). Codex judges the look from two stills (a pulse's
  peak and rest under the bottom paddle) and may move the anchors or cut the beats:
  `BEAT_PULSE_FRACTION` is the lever and zero is "bars only" (round 52, gemini F6 — a worry
  the stills answer, not the plan); the timing is judged live by the executor and by Jesse.
- **The pause hint**: the game draws the engine's default `PauseMenuLabels` today
  (`drawing.rs:191-194`, `pause.draw`); it switches to `pause.draw_labeled` with the default
  labels and a hint of its own, `"ESC resumes - SPACE/ENTER or click confirms - move to jam"`
  (no locale files in this game; one constant in `music.rs`).
- **The same-frame order**, tested where it is a function: `check_brick_hits` collects the
  bricks hit this frame, `sort_unstable`s and `dedup`s them and walks them **in descending
  index** (`bricks.rs:62-79`, so removals never shift a lower index) — that walk is the order
  the `brick` and `armor` triggers go out in, and it is the one same-kind order the game owns
  (the physics step's contact order decides which ball hit which paddle first, as in Tong). The
  executor lifts the collection into a pure function (`bricks_hit_this_frame(collisions,
  balls, bricks) -> Vec<usize>`, descending) and tests it with two bricks hit by two balls in
  one frame; **the order between kinds is tested through the trace**: one physics step in
  which one ball strikes the paddle and another destroys a brick (placed with `jaw_tests.rs`'s
  `place_ball`) leaves the frame's trace `["paddle", "brick"]` — the order `update_gameplay`
  runs the passes in, paddles then bricks then loss then clear, which the module doc also
  names (round 54, kimi F2). A second trace case lands two balls on two bricks of known
  indices in one step, one armored and one plain, and asserts the trace is the descending
  walk's order through the production path — not the pure function alone (round 55, codex F2).
  A third case leaves one brick, destroys it with a ball, and asserts `["brick", "clear"]` — the
  win decided on the frame the last brick breaks, through the production path (round 56,
  gemini F3).
- **Tests** (`src/music_tests.rs`, new, under 400 lines, on `test_support::harness`; the three
  helpers it needs that `flow_tests.rs` and `jaw_tests.rs` hold as private functions —
  `launch`, `place_ball`, `send_past_the_bottom` — **move to `test_support.rs`** as `pub(crate)`,
  their call sites following, which is the one edit `jaw_tests.rs` takes and it shrinks the
  file; round 56, gemini F4): the committed sidecar's structure (`brick` and `armor` name one
  part, `Loop` with `quantize: Off`, on two instruments; `paddle`'s part is `AtSongPosition`;
  `loss` a `Loop`; `clear` a `Loop`; every part `quantize: Off`; `menu` a `Loop`; provenance set);
  the song plays after `init` (`blocks_rendered` moves, as `song_triggers.rs settle` waits); a
  destroyed brick is one accepted trigger, the brick's `source_of` is `None` **and the
  conductor's `live_sources` still counts the game's source after that frame's tail** — the
  trigger was the game's, so the reconcile had nothing to end; a trigger wrongly sent on the
  brick's own source would leave `source_of` `None` just the same but `live_sources` back down
  (round 55, kimi F3); an armored brick's hit is one accepted trigger with
  `source_of(brick)` `Some`, and killing it ends that source in the tail (`None` after the
  frame); a paddle hit is one on `source_of(paddle)`, and `start_game` leaves the old paddle's
  `source_of` `None` — **the lifecycle**; two bricks destroyed in one frame are two triggers,
  in descending index (the pure function); a lost ball is one; **the clear is one accepted
  trigger and not a silent one** — `silent_triggers` unchanged (the conductor's counter for a
  trigger that found nothing to play, a rest or an exhausted part, `position.rs:64-65`) and
  `pending_events` — the trigger buffer's held note-ons and note-offs (`conductor.rs:541`,
  `triggers.pending_len()`), which the background player never touches — **from zero to above
  zero and back to zero**: the test clears a board on a fresh match in which nothing else has
  triggered (the harness empties `bricks` through the game's own list, no hit, no loss), so the
  count is zero on the frame before the clear, above it on the frame after (the stinger's notes
  start on the next block and their note-offs are held until each note's written end,
  `position.rs:67`), and zero again once the stinger's longest note has played — its length
  read from the committed MIDI through `music_midi` (`SongData::from_smf`, the `stinger`
  track's longest `duration_ticks` through the tempo map) plus one block of the virtual clock
  (round 53, codex F1 / kimi F6: "within a beat" would have failed a half-note chord; round 54,
  codex F2 / kimi F3: a baseline holding other triggers' note-offs moves while the stinger
  plays, so the test quiesces instead of differencing) — so the counters, not the trigger
  count alone, say the unit had notes and an instrument to play them on (round 52, codex F3) —
  **by bounded polling, never by counting frames**: the headless song runs on a virtual clock
  advanced by the wall time between `update`s (`training.md` § Conductor), so the test steps
  the harness with the 3 ms pause pong's `music_tests.rs settle` uses until `accepted_triggers`
  has moved and `pending_events` is above zero (bounded at two seconds, a failure past it),
  then until it is zero again (bounded at the stinger's longest note plus two seconds), reading
  the audio's counters and never assuming a frame is a block (round 56, codex F1); the wiring
  from the last brick to the trigger is the trace's third case; the pause jam through the
  real `PauseMenu` with keys and the pointer (pong's five cases, `music_tests.rs:152-190`);
  the pulse under the paddles while Serving and Playing and never under the pause menu (a
  `pulsed` count); `restart` by identity in the two arms the game can reach (an empty slot is
  requested into — the label becomes the song's path; its own is sought — the position reads
  the top), the fixture-song arms being the engine's tests now; the missing bank (pong's
  `cfg(unix)` scratch-directory shape with symbolic links to `fonts/`, `scenes/`, `sprites/`
  and `ai/music/`, no `soundfonts/`) leaves the game running silent, `song_failed` true and
  the triggers counted as refused.
- **`CLAUDE.md`** (breakout): a § Music modelled on pong's — the files, the six bindings and
  their modes, the two source kinds and why, the pulse, the hint, the no-fallback rule, the
  bank's wrapper and the budget line, the version bump — the module layout gaining `music.rs`,
  and the test list gaining `music_tests.rs` with its count (that guide keeps counts; the
  "107 tests" lines are updated to what `cargo test` prints).

### Correction 7 — the web proof for a K6 game: a byte cap, one recorded boot, pass-or-fail checks

Both bundle versions are four-place contracts (`games/breakout/src/web_entry.rs` names the
lists): the game **v5 → v6** (`ASSET_BASE = "/games/food-pyramid/v6/assets"`,
`scripts/build_wasm.sh ../games/breakout food-pyramid --version v6`, `insiculous_web/src/
content/games/food-pyramid.md` `wasm: '/games/food-pyramid/v6/game.js'`,
`public/games/food-pyramid/v6/`) and the editor bundle **v4 → v5** (`EDITOR_ASSET_BASE =
"/playground/breakout/v5/assets"` — the editor bundle's slug is still `breakout`, as the
deployed `public/playground/breakout/v4/` and the content line `editor: '/playground/breakout/
v4/game.js'` show — `--kind editor --version v5` with the slug `breakout`,
`public/playground/breakout/v5/`). The old directories stay until the deploy. **The checks run
on the exact bytes that will ship.** `build_wasm.sh --sync` is a second build, not a copy — it
`rm -rf`s the game's `dist/`, rebuilds, and copies the new output (`build_wasm.sh:230-234`,
`:446-451`) — so a bundle checked from one run and synced by another is two bundles (round 55,
kimi F1). So each bundle is built **once, with `--sync` up front**, into
`insiculous_web/public/games/food-pyramid/v6/` and `public/playground/breakout/v5/` — two new,
untracked directories — the content lines are edited to v6 and v5, `npm run build` copies them
into `dist/`, and every check runs on that `dist/` under the one `npm run preview`, beside the
Tong copies of correction 1 (round 54, kimi F1 — v3 staged Tong's bundles and forgot the Food
Pyramid's own). **The order is one sequence and its failure path one rollback unit** (round 56:
codex F2, kimi F1 and F2, gemini F1 and F2): (1) `web_entry.rs`'s two constants bumped; (2)
both bundles built once with `--sync` into `public/` — if the second build fails, the first's
synced directory is removed before any retry, since `--sync` refuses a non-empty target; (3)
the two content lines edited; (4) `npm run build` once; (5) the Tong copies of correction 1
overlaid into `dist/`; (6) `npm run preview` and every check; (7) the preview stopped by its
pid, then `npm run verify`. **A failure at any step rolls the unit back**: the two untracked
version directories removed, the content file and `web_entry.rs` restored with `git checkout --
<file>` in each repo (the file named, never `checkout -- .`), and the report INCOMPLETE naming
the step — the trees left as the batch found them; a pass stages all of it.

**Ruling (o), proposed: the boot gate for a K6 game is the bank's byte cap, not the five-pair
increment.** Batch 12 measured the method's answer once — a 1.98 MiB bank cost +1.71 s cold on
throttled Chromium against an 8 s gate, and `tong.sf2` parses in 7.5 ms natively — and the
K6 banks are the same kind of file at the same scale: a 4 MiB bank at 10 Mbit/s is 3.4 s of
transfer, and the parse is milliseconds. So: the bank **≤ 4 MiB** (the file's size, asserted in
the report), and **one** throttled Chromium cold boot of v6 recorded to the `food pyramid: init
complete` line (10 Mbit/s, 100 ms latency, 4× CPU, a fresh profile, the CDP order of batch 12
correction 6, `?telemetry=1`), with no baseline arm, no five pairs, no `games/breakout-baseline/`
copy; the number goes in the report and the scorecard beside Tong's — **under a ceiling with a
consequence** (round 52, codex F4): Tong's v7 measured **16.95 s** cold under the same
throttling (`report-12.md` § 5), so a Food Pyramid v6 cold boot over **25 s** — Tong's plus
the 8 s the increment was ever allowed, anchored to Tong because the two bundles are the same
size: the Food Pyramid's v5 `game_bg.wasm` is 7.35 MiB to Tong's v7 7.46
(`insiculous_web/public/`), its eight scenes and fifteen sheets are kilobytes, its bank 0.2 MiB
larger, and its preload manifest 48 entries to Tong's 36 (the two `dist/` manifests, counted
2026-10-23) — twelve more serial fetches at the emulated 100 ms, about 1.2 s (round 53, gemini
F2 guessed a 22–24 s baseline from no measurement; round 55, gemini F2 a waterfall the
manifests do not show) — is run twice more and the median of the three decides; **an attempt
that logs no `food pyramid: init complete` within 90 s is a failed attempt**, counted as over
the ceiling, and three of them are INCOMPLETE with the console transcript (round 55, kimi F4);
a median over 25 s removes the synced directories and the report is INCOMPLETE naming it. One
run alone is not a verdict either way: batch 12's fourth pair hit 59 s on both arms from
desktop activity on the shared display. **One bounded stress
run is kept** (round 53, kimi F5: four instrument slots and a brick burst under multiball are
not Tong's workload): two minutes of THE GAUNTLET — Insiculous, so every serve is two balls
and the speed climbs — on the v6 bundle in Chromium, unthrottled, the XTEST driver restarting
matches, **with a workload floor**: the run's last telemetry line shows at least 60 `bricks=`
and 60 `paddle_hits=`, or the run did not exercise what it gates and is INCOMPLETE, not a pass
(round 55, codex F1); batch 12's criterion verbatim — no two consecutive telemetry windows
with a drain gap over 92 ms, the render cost under a block, `blocks_rendered` advancing in
every window, zero lines a FAIL — and a fail removes the synced directories. The five-pair
boot measurement is not repeated: it proved the method once (ruling (o)).

**Browser checks, each a pass or a fail with a consequence, and every one a counter, a state
or a still — never an ear** (round 53, kimi F4: the executor cannot hear, so v2's "a lost ball
plays the dirge" was a check nobody could run): a fail removes the two synced version
directories from `public/` (above) and the report is INCOMPLETE naming the check. The checks: the song is requested before any gesture and starts on the
title's first key after the cards (the console's `Web audio enabled after user gesture` and
`Playing song:` lines, in that order); a rally in which the telemetry line's `bricks=`,
`armor_hits=`, `paddle_hits=` and `losses=` counts rise together with the conductor's
`accepted_triggers` while `refused=`, `silent_triggers`, `rejected_triggers`, `late_triggers`
and `stale_triggers` all hold at zero — a rally is not saturation, and a paddle hit whose
command was rejected, late or stale would leave `paddle_hits=` rising with nothing heard
(round 55, kimi F2); zero silent means every binding resolved to a unit with notes, the
`AtSongPosition` bass included: its line is written without a rest, so a paddle hit that finds
one is an arrangement defect this check exists to catch, not noise to allow (round 54, gemini
F4); `jam_units=` rising on a move in the pause menu and not on its open;
`pulsed=` rising while a match runs and holding under the pause menu, with the two stills;
`/playground/food-pyramid/` — the site's page, `src/pages/playground/[slug].astro` over the
game's content id `food-pyramid`, which loads the editor bundle from `/playground/breakout/v5/`;
the page route is the game's slug and the bundle directory the crate's, two names by the
site's standing contract (`web_entry.rs:9-10`, `CLAUDE.md:20`) — showing no `Playing song:`
while editing, the line on Play's first frame (the editor replays what `init` asked for —
door (b), `editor_integration/src/editor_game/init_music.rs`), a `reconstructions` step at the
first match start (the seek to the top), and no music line after Stop (#199: the bundle
exposes no silence signal, so the absence is what is checked). **What each of those sounds
like is Jesse's listen check, the gate before the done mark** — natively and in both browsers,
the fanfare on the bricks, the trombone's echo on a foil brick, the bass on the tong, the dirge
on a loss, the jam, the pulse on the beat, and **a cleared board** (CLASSIC, a few minutes; the
stinger with the last brick's burst), which no browser check reaches and the harness pins by
its counters (correction 6); the executor quotes `song notes stinger` so the unit's notes and
instrument are on record. Chromium drives the throttled boot and every check; Firefox (snap, the
`devtools.console.stdout.content` profile, XTEST) the functional checks unthrottled — **the
editor bundle in Firefox too**, which batch 12 left undone. Every measurement URL carries
`?telemetry=1`; the lines are quoted. `npm run verify` in the site. **Two stills** of the pulse
(peak and rest under the bottom paddle, a rally running) go to `review/conductor/` and to
`coordination/conductor/screenshots/batch-13/` (tracked), as batch 12's did.

### Correction 8 — the docs, and one sentence on each game's page

`training.md` § Conductor Pattern gains "A game's music module" (correction 3) and its file
list `engine_core/src/game_music.rs`; the directory map a line for it;
`crates/engine_core/CLAUDE.md` the file-map entry and two pitfall rows; `games/pong/CLAUDE.md`
§ Music is rewritten where it changes (the module stands on the engine's `GameMusic`; the bank
script is a wrapper over the engine's; the lifecycle sentence of correction 1) and nowhere
else; `games/breakout/CLAUDE.md` § Music is new (correction 6). **Each game's page gains one
sentence** (`insiculous_web/src/content/games/{tong,food-pyramid}.md`, the body, after the
chaos-modes paragraph): Tong's — "Over it all, Bizet's *Toreador Song*, played live by the
engine's conductor: every paddle hit walks the refrain, and the pause menu jams over it." — and
the Food Pyramid's — "The counter marches to Verdi's *Triumphal March*, played live by the
engine's conductor: every brick you break walks the fanfare, and the tong plays the bass."
Batch 12's site commit changed Tong's two version lines and nothing in the body, so the
shopfront says nothing of the music yet. Codex judges the two sentences with the stills.
`PROJECT_ROADMAP.md` row K and `CLAUDE.md`'s "K1 gates all of Conductor" line stay batch 16's.

### Correction 9 — sizes, placements, gates

Files near the ceiling: breakout `constants.rs` 590 (untouched — the music constants live in
`music.rs`), `levels_tests.rs` 569 (untouched); **`jaw_tests.rs` is 614 lines today**, over
the ceiling before this batch — this batch touches it only to move three private helpers to
`test_support.rs` (correction 6), which shrinks it; the split itself is breakout#7, filed by the
planner rather than have a music batch reshape a jaw test file. Pong: `types.rs` 475 (gains one line),
`music_tests.rs` 371 (gains the order test and the accessor edits, under 420), `music.rs` 272
shrinks. Engine: `prelude.rs` gains two names. New files: `crates/engine_core/src/game_music.rs`,
`crates/engine_core/tests/game_music.rs`, `scripts/prepare_game_soundfont.sh`;
`games/breakout/src/{music.rs, music_tests.rs}`, `games/breakout/scripts/prepare_soundfont.sh`,
`games/breakout/assets/{ai/music/ai_food_pyramid.mid, ai/music/ai_food_pyramid.mid.ron,
ai/music/LICENSE, soundfonts/food_pyramid.sf2, soundfonts/LICENSE}`,
`games/breakout/review/conductor/` (the game's reviews), `coordination/conductor/screenshots/
batch-13/`. Touched: engine `crates/engine_core/src/{lib,prelude}.rs`, `crates/engine_core/
CLAUDE.md`, `training.md`; pong `src/{music,music_tests,lib}.rs` (`lib.rs` only if the prelude
import changes), `scripts/prepare_soundfont.sh`, `CLAUDE.md`; breakout `Cargo.toml` (the
`music_midi` dev-dependency for the sidecar test, as pong's), `src/{lib,types,menu,drawing}.rs`,
`src/gameplay/{mod,bricks,paddles,balls,flow}.rs`, `src/web_entry.rs`, `CLAUDE.md`; site
`src/content/games/{tong,food-pyramid}.md`, `public/games/food-pyramid/v6/`,
`public/playground/breakout/v5/`.

**Gates:** in `insiculous_2d` the standard set (`cargo test --workspace`, both clippy runs,
`check_wasm.sh` — `engine_core` is touched — `check_games.sh --test` — a public item of
`engine_core` is added, and two games' behaviour is exercised — `check_guide_tests.sh`,
`check_features.sh`, `check_audio_provenance.sh`, the comment-tag grep, the 600-line check);
in each game `cargo test`, `cargo clippy --all-targets` with and without `--features editor`,
`../../insiculous_2d/scripts/check_audio_provenance.sh "$PWD"` (breakout's executor breaks its
`soundfonts/LICENSE` once to see the gate fail, restores it, quotes both runs); in pong the
wrapper's re-run reproducing `tong.sf2` byte for byte (correction 4) and the ten music tests
green; in breakout the two `build_wasm.sh` runs with their `wasm size:` lines, the bank's size
against the 4 MiB cap, the song under 256 KiB, **and correction 7's staged browser checks, the
recorded boot under its ceiling and the two-minute stress run on THE GAUNTLET, each pass or
fail, a fail removing the synced directories**; in `insiculous_web` `npm run verify` after the
checks. **Leaves
out:** the pyramid roster and the Maxwell cameo (correction 10), a volume control, a recorded
fallback (ruling (e)), a `Sequence` part in a game (correction 5 says why), the title and
level-select menus jamming (**ruling (p), proposed: the pause menu is the one menu that jams,
in every game** — Tong set it, and a level select that plays a note on every move is a sound
effect, not a jam), the five-pair boot measurement (ruling (o); the two-minute stress run is a
gate, not an exclusion — round 54, codex F3 caught v3 listing it in both), the other
four games, the game template (a `GameMusic` in the template is its own issue, filed by the
planner), intensity layers and harmony generation (the milestone's exclusions), Jesse's
promotion of either song to `music/`.

### Correction 10 — the Maxwell finale is breakout#3's, and the clear is the binding today

The section's bullet said "the Maxwell finale takes the march's cadence". The tree has no
Maxwell finale: `check_win_condition` (`flow.rs:50-60`) empties the balls and candies and sets
`GameOver { won: true }`, and the game-over panel says "BOARD CLEARED!" (`drawing.rs:181`).
Dr. Maxwell's taunt cameo on a clear is **breakout#3** — the pyramid level select, the gates,
the unlock save, "P2 soon, after 8c", a batch with its own plan revision, not this effort's.
So: **the `clear` binding is the finale's cadence today**, triggered where the win is decided,
and when #3 lands its cameo inherits the binding (one trigger, wherever the cameo plays). The
planner adds that sentence to #3 at the done mark. Nothing in this batch waits on #3.

**Rulings for Jesse** (sent with the section unless he says otherwise): (l) the game-side music
module is promoted to `engine_core::game_music::GameMusic` in this batch and Tong migrates onto
it, none of its tests deleted; (m) the Food Pyramid's bank is FluidR3's trumpet, trombone,
tuba and timpani, about 2.2 MiB, committed, under a 4 MiB cap; (n) the paddle hit plays the
bass line in `AtSongPosition`, the clear a one-unit `Loop` on `quantize: Off` like every
binding in the game, and `Sequence` stays out of the games for the reason correction 5 gives;
(o) a K6 game's boot gate is the bank's byte cap plus one recorded throttled cold boot under a
25 s ceiling, with one two-minute stress run on the game's own bundle — the five-pair increment
was batch 12's proof of the method and is not repeated per game;
(p) the pause menu is the one menu that jams, in every game; (q) the clear's cadence is the
`clear` binding, which breakout#3's Maxwell cameo inherits; (r) each game's site page gains
one sentence naming its music; (s) Tong's migrated bundles are built and browser-checked
locally and not deployed — the next Tong bump ships them — rather than bumped to v8 / editor
v7 for a refactor.

## Batches 14–15 — K6: the other four games, in pairs (#65)

Executor: the Opus session per pair, each game in its own repo with its own review
directory. Reviewers: kimi, gemini, codex (the beat visuals). Listen check: Jesse per game.

- **14 — Bratdog + Toast Busters** (food, turn and death voices over the polka; shots, each
  Toasted kind on one shared part with its own instrument, UFO and hit voices over *Danse
  Macabre*). **Its own section follows this one; it goes out as two handoffs, 14a and 14b.**
- **15 — Meatieroids + Chicken Coop** (shots, three rock sizes as three instruments on one
  part, thrust and hyperspace over *Mars*; hops, death and home-slot voices over the
  *William Tell* finale).

**The rule batch 12 left (ruling 3b, door (b) — Jesse, 2026-10-23, #198): the editor starts silent,
remembers what the game's `init` asked the audio for, and asks for it again on each Play session's
first frame when no composer song is open. So a game starts its music where it naturally would —
at `init`, or at a match start that decides by the slot's label as `GameMusic::restart` does
(none → request; its own → seek to the top; another → leave the audition alone) — and asks the
audio for its state (`is_music_playing`, `song_label`) rather than trusting a flag it set at `init`,
since the editor stops and restarts the slot around it. A song built in memory is not replayed, and
what the game set after the request (looping, a seek) is the game's to set again.**

**What batch 13 left (2026-10-24).** The rules above are code now: a game's `music.rs` is a thin
module over `engine_core::game_music::GameMusic` (prelude) — the song path and bindings as
constants, the moments that call `ctx.sources.trigger`, the counts the telemetry line appends,
the pulse's anchors — with `start` at `init`'s end, `restart` at every match start, `follow_pause`
at the pause gate's end and in the `Resumed` arm, `update` at the top of `update`; Tong
(`games/pong/src/music.rs`) and The Food Pyramid (`games/breakout/src/music.rs`) are the worked
cases, and `training.md` § Conductor "A game's music module" the contract. Each game's bank is cut
by a `scripts/prepare_soundfont.sh` wrapper over the engine's `prepare_game_soundfont.sh`
(`"$@"` first, then `--max-mib 4`, the stem and the presets), built once from one `--bank`
FluidR3 for the pair. The browser check on the gesture reads **"no `Playing song:` line before the
gesture, and both `Playing song:` and `Web audio enabled after user gesture` on it"** — never an
order between them: `enable_output` starts the pending song and logs it from inside, and the
`Web audio enabled` line is logged on its return (report-13 § 8.3). The pause hint "… - move to
jam" overflows the engine's 300 px pause panel in both games (insiculous_2d#202); a new game's
hint waits for that ruling rather than adding a third copy of the text.

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

## Batch 14 — K6: Bratdog and Toast Busters (#65) — DONE 2026-10-26: 14a Bratdog (snake 8aea9f3; site 829d2fe; stills 3ee2928), 14b Toast Busters (invaders eccd17e; site 8cf69f6; stills 3947985)

**Re-verified 2026-10-24 against the tree at ecaff0c** (engine), `games/snake` 590b2d1,
`games/space_invaders` a102eed and `insiculous_web` 59b3cd6, after batch 13 landed (the engine's
`GameMusic`, the shared bank script, two worked games). **Settled at v5 on 2026-10-26, rulings (t) to (aa) as proposed — no oboe swap, no cuts** (Jesse, on
the planner's recommendation after round 61). v5 after plan rounds 58 to 61 (61: kimi
2, gemini 5, codex 2 — the oboe swap reaches every clarinet track's program, Bratdog's four parts,
the bob rested when the state leaves Playing, the stress driver closed on the screencast's head
cell with a restart key in its cadence, the preview's pid named as the server's; `rebuttal-61.md`;
the planner recommended settling here — no contract has moved since v1 but the bob's host and
its state, and rounds 59 to 61 hardened gates. 60: kimi 5, gemini 6, codex 2 — `blocks=`
verbatim, the bob's peak stored and reset on pause and restart, Bratdog's stress driver a blind
boustrophedon with a floor it can reach, the preview stopped on every exit, the web unit's
ownership stated; `rebuttal-60.md`. 58 and 59 before it:
kimi 4, gemini 7, codex 3 — the `singe` count, the missing-bank fixtures, the rollback's
precondition, the meter in the composer recipe, the bob off the physics body, four-kind and
flyer-bust tests, bounded polling; gemini's two Criticals misread a `Loop` part and an instrument
slot. 59: kimi 6, gemini 6, codex 1 — the leftover "ten"s, Bratdog's seven tracks, the telemetry
keys verbatim, the meter edit's exact payload, the bob's decay state and its sign, the rollback by
reversing the executor's own edits with the version directories absent at step 0, FluidR3
fetched per handoff, the rulings' alternatives decided at settling; `rebuttal-58.md`,
`rebuttal-59.md`). Nine corrections to the two bullets the section was; each names what the tree
says. **The coordinate convention throughout: world Y is up** (the engine's `Transform2D` and
`Sprite.offset`, "in world units, Y up" — pong's guide), so a dip is a smaller y.

**What 14a left (2026-10-26).** Bratdog landed from `report-14a.md` (COMPLETE) after code round 62
(kimi 0, gemini 1 rebutted, codex 1 real; the site's content diff kimi 0, codex 0) and fix round 63
(kimi 0, codex 0); `rebuttal-62.md`, `rebuttal-63.md`, `review-62-claude.md`. Three lessons bind 14b.
**The pulse runs after the frame's ticks**, gated on `Playing`, before the frame's one map writer:
the executor placed it right after the pause gate, so a beat on a tick frame rippled the cell the
dog had just left (codex, round 62); the fix records the pulse's anchors in a test-only
`Music.pulsed_at` and asserts the beat frame's anchor is the head after that frame's tick — Toast
Busters' bob is written after the fleet's step for the same reason, and its test reads the
slices' offsets on the frame of the pulse. **A two-minute stress floor is computed at the game's
starting cadence, never its floor cadence**: correction 7's "a sweep every eighteen seconds" used
Bratdog's 0.05 s floor tick where a match starts at 0.10 s, so the section's row sweep could not
reach 120 turns; the executor turned the driver a quarter (columns, 15 cells) and met the floor
twice (173, 166). Toast Busters' 120 shots / 40 busted are re-checked against the cannon's fire
cooldown and the wave's slice count before `handoff-14b.md` is written. **The loop's end is a
written note's end** (`BackgroundScore::end_tick`): a staccato arrangement ends its background 40
ticks before the bar and slides the bar lines a little every loop; Bratdog's counter-line is legato
so its last note ends on tick 30720, and a test holds the meter and the end — Danse Macabre's
recipe does the same (the waltz bass's last note held to the loop's last tick). Also: the tune on a
trigger track means the background alone carries no melody — by design, and Jesse's ear is the
gate on whether the background still reads as the piece; the web preconditions, the pid-stopped
preview and the in-place rollback held with nothing to roll back; the first Chromium functional
load rebooted the game once (#222) and one stress window stalled for 2.1 s (#221), neither
reproduced; the executor's drivers, tracker and song generator are kept at
`review/conductor/batch14a-tools/` for 14b's session to start from.

**What 14b left (2026-10-26).** Toast Busters landed from `report-14b.md` (COMPLETE) after code
round 64 (kimi 0, gemini 1 rebutted — `mode: Loop` read as a note looping until despawn; codex 0
with the two bob stills; the site's content diff kimi 0, codex 0); `rebuttal-64.md`,
`review-64-claude.md`. One in-session ruling: **the tune is on a track named `tune`, and `song
new`'s `melody` ships empty** — `melody` is program 0, no `SongEdit` changes a track's program, and
a subset bank has no preset 0, so rustysynth would voice it on the bank's lowest preset (the
xylophone). Filed as insiculous_2d#223; **batch 15's two songs name their own tune track the same
way** rather than wait on the edit. Two lessons bind 15. **A sprite-offset beat visual has no
observable placement**: `Sprite.offset` is relative to its entity, so the bob draws the same
whether it is written before or after the fleet's step, and the ordering matters only on a frame
that ends the match, which `finish_game`'s `rest_bob` already covers — 14a's "after the frame's
ticks" lesson is for a world-space anchor (Bratdog's ripple), and a section must not spend a test
on the ordering of an entity-relative one. **A press on the web editor's toolbar reaches the game
as P1's `Action1` on the press frame** (Firefox, press and release frames apart; insiculous_2d#224):
an editor-bundle check that clicks Stop over a game-over panel will see one restart line and one
shot before the restore, and a report should name it rather than chase it. Also kept from 14b:
the stress floor at the starting cadence held (263 shots / 226 busted against 120 / 30), neither
of 14a's browser anomalies (#221, #222) recurred, and the method's tools and logs for batch 15 are
`review/conductor/batch14b-tools/` and `batch14b-logs/` (14a's copied beside them).

Executor: the Opus session, **one handoff per game, in order** (ruling (t)): **14a Bratdog** in
`games/snake` + `insiculous_web`, then **14b Toast Busters** in `games/space_invaders` +
`insiculous_web`; each with one report at the engine's `review/conductor/report-14a.md` /
`report-14b.md` (the engine's tree is not touched: batch 13 left nothing for it to carry). Reviewers
per handoff: kimi (everything), gemini (the game's tests and the bank wrapper), codex (the beat
visual from two stills, the page sentence). Listen check: Jesse per game, natively and in Firefox
and Chromium. Size: about +700 lines a game plus the assets (a song, its sidecar, a 3.5 MiB
SoundFont subset each); each game's diff is reviewed from its own `review/conductor/`, the site's
from `insiculous_2d/review/conductor/`.

**Standing:** batch 12's rulings (e) no recorded fallback, (f) the song plays on under the pause
menu and the jam reads `MenuInput`, (h) a committed per-game bank with its `LICENSE`, (i) hits on
`quantize: Off`, (k) the bank is a boot dependency on the web until #194; batch 13's (l) the game
module is `engine_core::game_music::GameMusic`, (m) a per-game bank under a **4 MiB cap** the
wrapper enforces, (o) the boot gate is the cap plus one recorded throttled cold boot under 25 s and
one two-minute stress run, (p) the pause menu is the one menu that jams, (r) one sentence on each
game's page; the K6 rule (3b, door (b)); and **"What batch 13 left"** in this section's parent
(the module shape, the wrapper with `"$@"` first, the gesture check as two lines on the gesture,
the pause hint waiting on #202).

### Correction 1 — what the two trees hold today

Neither game has a line of sound: `grep -rni "sound\|music\|audio\|beep\|conductor"` over each
`src/`, `Cargo.toml`, `scripts/` and `README.md` prints nothing. Both carry `log = "0.4"` **only
under the wasm target**, so a native `log::info!("<name>: init complete")` needs the dependency
moved to `[dependencies]` as breakout's and pong's are (`breakout/Cargo.toml:16`); neither has a
boot marker. Both pause gates draw the engine's defaults (`pause.draw`: snake `drawing.rs:116-118`,
invaders `drawing.rs:117-120`) and neither has locale files. Both guides' test counts are stale by
their title-screen tests (snake says 65, `cargo test` prints 67; invaders says 68, prints 73) — the
batch rewrites the counts anyway. Both editor bundles keep the crate's slug under the game's route,
the site's standing two-name contract (snake: `/playground/bratdog/` → `/playground/snake/v5/`,
`web_entry.rs:47`; invaders: `/playground/toast-busters/` → `/playground/invaders/v5/`, `:48`), so
the guides' "the editor at `/playground/<game>/`" lines are the route and are right. **Bratdog** has
the engine's `GridBackdrop` (`spawning.rs:125-135`, `GRID_IMPULSE_EAT_*` and `_DEATH_*` at
`constants.rs:160-163`) but its `ripple_grid` is a private `fn` (`gameplay/mod.rs:23`); it becomes
`pub(crate)` for `music.rs`, as breakout's is. **Toast Busters has no grid by Jesse's ruling**
(2026-10-13, "the countertop's grout is the grid", `CLAUDE.md:58`; `flow_tests.rs:261-272` asserts
none) and **no next wave** ("there is no level progression yet", `CLAUDE.md:22`): the win ends the
match (`combat.rs:301-306`). Its fleet moves continuously (`formation.rs:66-93`, `march_speed`
30 → 240 px/s as the fleet thins), not in stepped ticks — the classic heartbeat is the fleet's
speed, not a tempo, and **the song does not speed up with it** (intensity layers are the
milestone's exclusion; one tempo map a song).

### Correction 2 — one section, two handoffs

The parent section says "the Opus session per pair". Batch 13's one game with a migration ran the
executor about five hours and a 650-line report; two new games, two songs sequenced from memory,
two banks, two browser proofs and two stress runs in one handoff is a report nobody reads whole and
a batch that cannot land half. **Ruling (t), settled:** this section is one plan, settled once;
the executor takes **14a Bratdog** first and **14b Toast Busters** after 14a's review round, each
with its own report, review round, listen check and commits; the site's second commit carries the
second game's bundles. One batch out at a time stands: 14b is not handed off while 14a's report is
unreviewed. (The alternative, one handoff for the pair, was offered and declined.)

### Correction 3 — Bratdog's song: the Tritsch-Tratsch Polka, its shape, its bank

`games/snake/assets/ai/music/ai_bratdog.mid` + `.mid.ron`: Johann Strauss II's *Tritsch-Tratsch
Polka* (1858; public domain) in a short arrangement the executor sequences **in the composer on the
snake project** (as `report-13.md` § 6 did: the bank first, `song new`, `AddTrack`, `AddNote` in
`Batch`es, `SetTempo`, `AddPart`, `AddInstrument`, `SetBinding`, `SetProvenance`, the `song` and
`song notes` answers quoted, `song save ai/music/ai_bratdog.mid`), from memory as the march was —
the executor cannot hear; it declares the shape, and Jesse's ear is the gate. **The meter is set,
not assumed**: `song new` opens a 4/4 song, so the recipe's first edit after `SetTempo` is the
meter, sent exactly as the API takes it — `{"SetTimeSignature": {"tick": 0, "meter": {"numerator":
2, "denominator": 4}}}` (`crates/editor/src/song/edit.rs:73-76`, `docs/EDITOR_COMMAND_API.md:233`;
the `meter` object is required, inline fields are refused), its `"ok":true` required before the
first `AddNote`; the export writes an edited meter (`song_data/export.rs:120-125`); the `song` answer's
`time_signatures` is quoted, and `music_tests.rs` reads the committed MIDI's signature at tick 0
through `music_midi` and asserts 2/4 — else the conductor's bar lines fall every four quarters and
the pulse accents every other bar (round 58, codex F2). Shape:

- **thirty-two bars of 2/4 at about 124 BPM** (a loop of about 31 s — sixteen bars of 2/4 is
  fifteen seconds and would wrap under every match), in A major or the executor's declared key,
  looping at a phrase boundary;
- **background**: `counter` (clarinet 0:71, the second voice), `oom` (tuba 0:58, the downbeats)
  and `pah` (trombone 0:57, the off-beats) on their own tracks — a track a part selects plays in no
  background, so the trigger tracks below are separate even where an instrument repeats;
- **track `melody`** (`song new`'s own, accordion 0:21): the polka's tune, **the one shared part
  `romp`** (`Loop`, `quantize: Off`), played by **two bindings on two instruments — `snack` on the
  accordion and `snack_two` on the clarinet**: Frank's snacks walk the tune on the squeezebox, the
  second dog's the same tune on the clarinet (versus only; in solo `snack_two` is never sent), the
  Food Pyramid's brick/armor shape over two players instead of two brick kinds;
- **track `paws`** (woodblock 0:115): **one short tick, one unit**, the part `paws` (`Loop`, `Off`)
  on the `turn` binding — a dog's claws on the kitchen floor, one click a turn. Not the bass line
  in `AtSongPosition`: a polka's "oom" is the tuba's background already, and a turn is a step, not
  harmony. The woodblock is 82 KiB;
- **track `dirge`** (trombone 0:57, its own channel): **one low blat, one unit** — a single note —
  the part `dirge` (`Loop`, `Off`) on the `death` binding; a head-on death in versus sends it
  twice in one tick, the same one unit on the game's source;
- **track `jam`** (accordion): the counter-line, the part `jam` (`Loop`, `Off`) on `menu`;
- every part `quantize: Off`; `soundfont: "soundfonts/bratdog.sf2"`; instruments `accordion`
  (0:21), `clarinet` (0:71), `tuba` (0:58), `trombone` (0:57), `woodblock` (0:115);
  `provenance: (author: "Johann Strauss II, 1858; sequenced by the Opus executor for Be
  Insiculous, 2026", license: "CC0-1.0")`; no sample instrument. **Seven tracks past the tempo
  track** — `melody`, `counter`, `oom`, `pah`, `paws`, `dirge`, `jam` (channels 1–7; `song new`'s
  `melody` is the first, six `AddTrack`s follow) — **four parts** (`romp`, `paws`, `dirge`, `jam`
  and nothing else: `counter`, `oom` and `pah` are background and select no part; the sidecar test
  asserts these four by name), five bindings, five instruments.

**The bank: `games/snake/assets/soundfonts/bratdog.sf2`**, FluidR3 GM **0:21 Accordion (2,294
KiB), 0:71 Clarinet (609), 0:58 Tuba (190), 0:57 Trombone (438), 0:115 Woodblock (82)** —
3,613 KiB of sample data by `subset_soundfont.py --sizes` on the full bank (measured 2026-10-24 on
a fresh `apt download` of 3.1-5.3, sha256 `74594e8f…1cb0`), about **3.53 MiB against the 4 MiB
cap** (ruling (u), settled). The accordion is two thirds of it and is the polka; no strings.
`games/snake/scripts/prepare_soundfont.sh` is the third wrapper: `exec
"$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" --max-mib 4 "$OUTPUT_DIR" bratdog 0:21 0:71
0:58 0:57 0:115`. The executor fetches FluidR3 **once per handoff** into its scratchpad (`apt
download fluid-soundfont-gm=3.1-5.3`, two seconds on Iroh, the sha256 checked by the script) and
passes `--bank` to the wrapper; 14b's session fetches its own — a scratchpad does not outlive the
session, and the bank is not committed anywhere (round 59, gemini F4).

### Correction 4 — Bratdog's code: three moments a tick, two dogs, the dogs' lifecycle

- **`src/music.rs`** (new, under 150 lines): the music constants at its top (`SONG_PATH =
  "ai/music/ai_bratdog.mid"`, the five binding names, `BEAT_PULSE_FRACTION = 0.25`) and `Music {
  game: GameMusic, counts: MusicCounts { snacks, turns, deaths }, #[cfg(test)] triggered }` on
  `SnakeGame` (`types.rs:192-225`, its hand-written `Default` at `:227-248`), built with
  `GameMusic::new(SONG_PATH, "bratdog", "BRATDOG_TELEMETRY")`. Wiring: `start(ctx)` at the end of
  `init` (`lib.rs:97`, after `apply_theme`) followed by `log::info!("bratdog: init complete")`;
  `update(ctx)` at the top of `update` (`lib.rs:101`); `restart(ctx)` at the end of `start_game`
  (`gameplay/mod.rs:272-296` — every match start takes it: the mode select's confirm `menu.rs:130`,
  the pause menu's Restart `mod.rs:49`, the game-over restart `mod.rs:263`); `follow_pause` in the
  `Resumed` arm (`mod.rs:54`) and at the gate's end (`:60`), `pulse` after the gate while Playing.
- **The moments and their sources.** A snack: `eat_food(ctx, i, cell)` (`mod.rs:213-237`, the
  pellet's entity removed at `:221` in the same call) → `snack` for snake 0 and `snack_two` for
  snake 1, **on the dog's own source, `ctx.sources.entity(self.snakes[i].map)`** — the dog's one
  `Tilemap` entity (`types.rs:110`), never the pellet's: the pellet is gone in that call and a note
  on it would be released at once. A turn: `advance_single` applies `next_direction` at
  `mod.rs:158-159` (versus `:174-177`); the executor compares the applied heading against the one
  before it and sends `turn` on the dog's source when they differ — a turn is a changed heading at
  apply time, never a buffered key (`buffer_for_snake`, `:127-147`, queues two ahead). A death:
  `kill(ctx, i)` (`mod.rs:308-317`) → `death` **on the game's source** — the dog's body stays on
  the floor behind the game-over panel, so its own source would also serve, but a death's note must
  ring on whatever the next frame does, as a brick's does. The menu's jam is the helper's.
- **The lifecycle.** The dogs' entities are destroyed by `destroy_snakes_and_food`
  (`mod.rs:350-360`) at every `start_game` and `reset_to_title`, so a dog's source ends in that
  frame's tail and the next match's dog activates on its first snack (the paddle test's shape,
  `breakout/src/music_tests.rs a_tong_hit_plays_on_the_paddles_source_and_a_match_start_ends_it…`).
  A pellet's removal ends nothing: the snack was the dog's (the brick test's shape — `source_of
  (pellet)` `None` and `live_sources` steady).
- **The order within a tick, and ticks within a frame.** `advance_single` applies the turn, then
  steps, then eats or dies (`mod.rs:156-168`); versus applies both headings (snake 0 then 1), steps
  both, eats in index order (`:199-203`), then `finish_versus` kills each dead dog in index order
  (`:329-339`). So a tick's trace is `turn` (0, 1) → `snack`/`snack_two` (0, 1) → `death` (0, 1).
  The game runs **several ticks in one frame** when the frame is long (`while self.tick_timer <=
  0.0`, `mod.rs:81-84`), so the trace is per frame and holds whole ticks in order; at `FRAME` and
  `NORMAL_TICK = 0.14` (`constants.rs:30`) a test sees one tick about every eight frames, which
  `test_support::step_one_tick` (`:70-80`) already waits for. Tests: `["turn", "snack"]` — a
  buffered turn into the snack's cell in one tick; versus `["snack", "snack_two"]` — two dogs on
  two pellets in one tick (Ridiculous lays two, `place_dog` and `lay_snack` place them);
  `["death", "death"]` — a head-on, the one dirge unit twice, a `Draw`.
- **The beat pulse**: the backdrop grid under **each live dog's head** (`cell_to_world(*snake
  .cells.front())`, as `kill` reads it at `:312`) — `Bar` → `GRID_IMPULSE_EAT_STRENGTH` /
  `_RADIUS`, `Beat` → a quarter of it — while Playing and never under the pause menu; a dead dog's
  head is skipped. Codex judges from two stills (peak and rest under Frank's head mid-run).
- **The pause menu keeps the engine's labels** (`pause.draw` stays): the jam works on a move, and
  the hint that says so waits on #202 for both games (ruling (z)). `CLAUDE.md` says the hint is
  silent on it.
- **Tests** (`src/music_tests.rs`, new, under 400 lines, on `test_support::harness(mode, chaos)`
  which already calls `start_game` and drains the pellets; `settle` — six frames with 3 ms between
  them — goes into `test_support.rs` since nothing in snake waits on wall time today): the sidecar's
  structure (`snack` and `snack_two` one part on two instruments, `Loop`, `Off`; `turn`, `death`,
  `menu` each a `Loop`; every part `Off`; provenance); the song after `init`; a snack is one
  accepted trigger on `source_of(dog)` with the pellet's entity gone and `live_sources` steady; a
  turn is one on the same source and a buffered key that does not change the heading is none; a
  death is one on the game's source, `source_of(dog)` kept, and the next `start_game` leaves the
  old dog's `source_of` `None` — the lifecycle; the three trace cases; the pause jam through the
  real `PauseMenu` (open frame, one move, pointer sweep, confirm, reopen); the pulse while Playing
  and never under the pause menu; `restart` by identity (empty slot requested into; own sought to
  the top); the missing bank (the `cfg(unix)` scratch directory linking `fonts/`, `sprites/` **and
  `ai/music/`**, no `soundfonts/` — snake has no `scenes/`; the song and its sidecar are present so
  the failure is the bank's, as breakout's fixture has it — round 58, kimi F3) leaves the game
  running silent with every trigger refused and counted. **The waits are bounded polls, never fixed
  sleeps** (round 58, codex F3): `test_support.rs` gains `poll(harness, limit, what, done)` in
  breakout's shape (`music_tests.rs:998-1005` there — 3 ms between frames until the condition
  holds, a failure past the limit), and every assertion on `accepted_triggers`, `live_sources` or
  `source_of` after a trigger polls for its condition with a two-second limit rather than stepping
  six frames and hoping.
- **`CLAUDE.md`**: a § Music modelled on breakout's (the files, the five bindings' table with
  modes, instruments and sources, the dogs' lifecycle, the order within a tick, the identity rule,
  the pulse, the engine's pause labels and #202, no fallback, telemetry, the bank and budget, the
  versions); the module layout gains `music.rs`; the test list gains `music_tests.rs` and the
  counts become what `cargo test` prints. **Web**: `web_entry.rs` `ASSET_BASE` v6 → **v7**
  (`/games/bratdog/v7/assets`), `EDITOR_ASSET_BASE` v5 → **v6** (`/playground/snake/v6/assets`),
  the content file's two lines, `public/games/bratdog/v7/` and `public/playground/snake/v6/`.

### Correction 5 — Toast Busters' song: Danse Macabre, its shape, its bank

`games/space_invaders/assets/ai/music/ai_toast_busters.mid` + `.mid.ron`: Saint-Saëns's *Danse
Macabre* (1874; public domain), the waltz, sequenced in the composer on the invaders project as
above — **`{"SetTimeSignature": {"tick": 0, "meter": {"numerator": 3, "denominator": 4}}}` right
after `SetTempo`**, its `"ok":true` before the first note, the `song` answer's `time_signatures`
quoted, the committed meter asserted by a test, or the bob's bar accents drift across the waltz's
downbeats (round 58, codex F2). Shape:

- **thirty-two bars of 3/4 at about 144 BPM** (a loop of about 40 s), G minor, looping at a phrase
  boundary;
- **background**: `melody` (clarinet 0:71, the solo violin's waltz tune — the bank has no violin
  under the cap, correction 5's bank below) and `waltz_bass` (acoustic bass 0:32, the oom-pah-pah)
  on their own tracks;
- **track `bones`** (xylophone 0:13): the rattling-bones theme, **the one shared part `bones`**
  (`Loop`, `Off`), played by **four bindings on four instruments — one a Toasted kind**: `plain_wraith`
  on the xylophone, `rye_ip` on the clarinet, `bagelgeist` on the bells (a ring for the ring),
  `sour_doom` on the acoustic bass (the two-hit loaf, low) — #65's "multiple enemy types share one
  part and select different instruments", four ways; the casting table's own line ("the waltz's
  bones-rattling xylophone is the Toasted");
- **track `pluck`** (acoustic bass): a gap-free bass line under the whole loop, the part `pluck`
  in **`AtSongPosition`** on the `shot` binding — a cannon's shot plucks the bass note of that
  moment of the waltz, always in the chord (the mode the Food Pyramid's paddle demonstrated;
  the tiling test holds it here too);
- **track `chime`** (tubular bells 0:14): **one low stroke, one unit**, the part `chime` (`Loop`,
  `Off`) on `flyer` — midnight strikes as Maxwell's saucer (or Jammer) enters;
- **track `bonus`** (tubular bells): **one high chord, one unit**, the part `bonus` (`Loop`, `Off`)
  on `bonus` — the flyer busted;
- **track `hurt`** (clarinet): **one low held note, one unit**, the part `hurt` (`Loop`, `Off`) on
  `hit` — a cannon hit;
- **track `stinger`** (clarinet): the dance's last chord as **one unit**, the part `clear` (`Loop`,
  `Off`) on `clear` — the last slice busted;
- **track `jam`** (clarinet): the counter-line, the part `jam` on `menu`;
- **the eleventh binding, `singe`** → the part `bones` on the `bass` — a Sour-Doom's first hit walks
  the same tune on the same voice as its bust, so the loaf has one timbre; no track of its own
  (round 58, kimi F2 / codex F1: v1 wired it in correction 6 and left it out of this list);
- every part `quantize: Off`; `soundfont: "soundfonts/toast_busters.sf2"`; instruments `xylophone`
  (0:13), `clarinet` (0:71), `bells` (0:14), `bass` (0:32); `provenance: (author: "Camille
  Saint-Saëns, 1874; sequenced by the Opus executor for Be Insiculous, 2026", license: "CC0-1.0")`.
  **Nine tracks past the tempo track** (`AddTrack` takes channels 1–9 and 11–16, so nine fit),
  **eleven bindings, seven parts, four instruments** — the largest sidecar yet; **ruling (x)** offered
  `bonus` and `hit` as the two cuts and Jesse kept both: all eleven stand.

**On one track, four instruments** (round 58, gemini F1, a Critical that is not so): a binding
names a part for its notes and an instrument for its voice, and **every instrument is its own
synthesizer slot** — a trigger plays the part's units on the instrument's slot, never on the
track's MIDI channel, so no program change is ever issued and the track's channel is unused by
the triggers (decision 2; `triggers.rs`; batch 13 correction 5, the same sentence for the
Pyramid's `brick`/`armor` pair, which Jesse has heard). **And a `Loop` part does not loop a note**
(gemini F2, F4): `Loop` is the cursor's mode — the units walked in order and wrapping at the
part's end — and a unit's notes play to their written ends; `hurt`'s one held note and `chime`'s
one stroke end when written, as Tong's `goal` and the Pyramid's `loss` do.

**No binding for a Toasted's crumb** (`update_invader_fire`, `combat.rs:57-79`, a random draw a
frame): no player act is behind it, forty slices firing at random would be a trigger storm with no
melody in it, and a crumb that lands is the `hit`. Said here so it is not asked again.

**The bank: `games/space_invaders/assets/soundfonts/toast_busters.sf2`**, FluidR3 GM **0:13
Xylophone (1,773 KiB), 0:71 Clarinet (609), 0:14 Tubular Bells (274), 0:32 Acoustic Bass (905)** —
3,560 KiB, about **3.48 MiB against the 4 MiB cap** (ruling (w), settled). **There is no violin
under the cap**: FluidR3's Violin is 3,005 KiB alone, the Strings 5,466, the Pizzicato Section 2,729
— any of them with the xylophone is over 4 MiB — so the solo violin's tune is the clarinet's
(sardonic enough for a graveyard), and the Oboe (889 KiB, the dawn's cock-crow in the score) is
the alternative offered and declined: 3,840 KiB, 3.75 MiB, still under. The clarinet stands. The wrapper
is the fourth: `… "$@" --max-mib 4 "$OUTPUT_DIR" toast_busters 0:13 0:71 0:14 0:32`.

### Correction 6 — Toast Busters' code: ten moments, four source kinds, the flyer's lifecycle

- **`src/music.rs`** (new, under 190 lines — eleven bindings and the bob): the constants at its top
  (`SONG_PATH = "ai/music/ai_toast_busters.mid"`, **the eleven binding names** — `SOUR_DOOM_BINDING`,
  `BAGELGEIST_BINDING`, `RYE_IP_BINDING`, `PLAIN_WRAITH_BINDING`, `SINGE_BINDING`, `SHOT_BINDING`,
  `FLYER_BINDING`, `BONUS_BINDING`, `HIT_BINDING`, `CLEAR_BINDING`, `MENU_BINDING` —
  `BEAT_BOB_BAR_PIXELS = 4.0`, `BEAT_BOB_BEAT_PIXELS = 2.0`, `BEAT_BOB_SECONDS = 0.12`) and `Music
  { game, counts: MusicCounts { shots, busted, singes, flyers, bonuses, hits, clears },
  bob_pixels: f32, bob_peak: f32, #[cfg(test)] triggered }` on `SpaceInvadersGame`
  (`types.rs:281-324`, `Default` at `:326-354`) — **the bob's state is two numbers** (round 59,
  gemini F2; round 60, kimi F3 and gemini F3): a pulse sets both `bob_peak` and `bob_pixels` to the
  bar's or the beat's peak; every running frame after that computes `bob_pixels = (bob_pixels -
  bob_peak * ctx.delta_time / BEAT_BOB_SECONDS).max(0.0)` — linear, back to zero in
  `BEAT_BOB_SECONDS` whichever peak started it — and writes the slices; **`rest_bob(ctx)`** zeroes
  both and writes every live slice's `Sprite.offset.y` back to its spec's, called once when the
  pause gate finds the menu active, from `restart`, **and wherever the state leaves `Playing` —
  `finish_game` (`flow.rs:55-78`, a win or a loss) and `reset_to_title` (`:82-88`)** — so a fleet
  that lands or outlives the last cannon within a dip's 120 ms is drawn at rest behind the
  game-over panel (round 60, kimi F4, gemini F4; round 61, gemini F3, codex F2: a dip frozen under
  a panel, or thawed out of phase on resume, is a pulse with no beat under it) — built with
  `GameMusic::new(SONG_PATH, "toast busters", "TOAST_BUSTERS_TELEMETRY")`. Wiring: `start` at the
  end of `init` (`lib.rs:123`, after the countertop) + `log::info!("toast busters: init
  complete")`; `update` at the top of `update` (`lib.rs:127`); `restart` at the end of `start_game`
  (`gameplay/flow.rs:28-51` — the chaos select's confirm `menu.rs:136`, the pause menu's Restart
  `mod.rs:60`, the game-over restart `flow.rs:19`); `follow_pause` in the `Resumed` arm (`mod.rs:65`)
  and at the gate's end (`:73`); the bob (below) after the gate while Playing.
- **The moments and their sources.** A shot: `update_one_cannon_fire` (`players.rs:154-174`, the
  volley loop at `:169-171`) → `shot` **once per fire, not per shot** (Ridiculous's twin volley is
  one press), **on the cannon's own source, `ctx.sources.entity(player.entity)`**. A Toasted busted:
  `strike_invader`'s kill branch (`combat.rs:169-181`, the entity destroyed at `:181` in the same
  call) → the kind's binding by `invader.kind` (`types.rs:101`; `Toasted` at `art.rs:216-221`),
  **on the game's source** — the slice is gone in that call and its note rings on. A Sour-Doom
  singed: the absorbed branch (`combat.rs:160-167`, `hits_left` 2 → 1, `HURT` played, the slice
  survives) → `singe`, **on the slice's own source** — so a second icicle that busts it mid-note
  releases the note, the host's reconcile doing it (the foil brick's shape; `singe` plays `bones`
  on the bass, the same voice as `sour_doom`, so the loaf has one timbre). A flyer entering:
  `update_flyer`'s spawn arm (`flyers.rs:51-55`) → `flyer` **on the flyer's own source** — and
  **the flyer leaving the screen is the visibility exit #65 asks for**: `update_flyer` destroys it
  off-screen (`:67-69`, `destroy_flyer` `:125-129`), so the reconcile ends its source in that tail
  with no call from the game. A flyer busted: `check_flyer_hit` (`flyers.rs:87-123`, the flyer
  destroyed at `:121`) → `bonus` on the game's source. A cannon hit: `player_hit`
  (`players.rs:197-233`) → `hit` on the game's source — on the last life the cannon's entity is
  destroyed in the same call (`:224`), ending its `shot` source in that tail; a surviving hit keeps
  the entity and respawns it (`:231`, `respawn_cannon` `:100-111`), its source kept. The win:
  `check_win_condition` (`combat.rs:301-306`) → `clear` on the game's source. The menu's jam is the
  helper's. Bunker chips, shot-vs-shot cancels, Jammer's glob and the fleet's descent get no
  binding (correction 9 says why).
- **The order within a frame** is `update_gameplay`'s (`mod.rs:88-99`): `update_firing` (shot) →
  `update_flyer` (flyer) → `resolve_player_shots` (busts and singes, **players then shots** —
  `combat.rs:139-142`) → `check_flyer_hit` (bonus) → `resolve_enemy_shots` (hit) →
  `check_win_condition` (clear). Trace cases through the production path: `["shot", "plain_wraith"]`
  — a fire pressed on the frame an earlier icicle lands; `["sour_doom", "plain_wraith"]` — two
  icicles of one cannon on two slices in one frame, in the cannon's `shots` order; `["plain_wraith",
  "clear"]` — the last slice, `GameOver { won: true }` on that frame.
- **The beat visual, with no grid (ruling (y)):** the fleet **bobs on the beat, in its art, never
  in its bodies** — each slice's `Sprite.offset.y` is `spec.sprite_offset().y` (the spawn's,
  `spawning.rs:19`) **minus `bob_pixels` — world Y is up, so minus is the dip toward the
  countertop** — `BEAT_BOB_BAR_PIXELS` on a bar, `BEAT_BOB_BEAT_PIXELS` on a beat, decaying to zero
  over `BEAT_BOB_SECONDS` — written to every live slice's `Sprite` each running frame after the
  gate, so the Toasted dip together on the waltz's beats while their
  kinematic bodies, the descent (`formation_offset.y`, `formation.rs:83-85`) and the march speed are
  untouched. **Not the kinematic target** (round 58, gemini F3, real): `check_invasion`
  (`formation.rs:125-138`) and `chomp_bunkers` (`:98-121`) read the bodies' positions, so a 4 px dip
  of the body at the invasion line would land the fleet or chip a bunker on a bar line; an icicle
  that meets a dipped slice's body 4 px above its drawn crust for 120 ms is the tong's own
  art-beside-body rule, and no test holds a hit to the drawn pixel. Codex judges from two stills
  (on a beat, between beats) and may cut it to bars (`BEAT_BOB_BEAT_PIXELS = 0`); Jesse judges the
  timing live. Not the `MARCH` clip restarted on the beat: the slices are `ClipStateMachine`s
  (`spawning.rs:23-29`), and a transition to the state a machine is in is a no-op by design.
- **The pause menu keeps the engine's labels** (ruling (z)).
- **Tests** (`src/music_tests.rs`, new, under 450 lines — eleven bindings — on
  `test_support::harness(mode, chaos)`, `clear_field`, `drop_on_cannon`, `step_with`; `settle` added
  to `test_support.rs`): the sidecar's structure (the four kind bindings one part on four
  instruments, `shot`'s part `AtSongPosition`, the rest `Loop`, every part `Off`, provenance); **the
  `pluck` line tiles the loop** (the bass test's shape over the committed MIDI); the song after
  `init` and the committed meter 3/4; a shot is one accepted trigger on `source_of(cannon)` and a
  twin volley is still one; **one slice of each of the four kinds busted through the production
  path, the trace naming `sour_doom`, `bagelgeist`, `rye_ip` and `plain_wraith` in turn** — the
  kind-to-binding table is the batch's centre and a swap of two kinds would pass every counter
  (round 58, kimi F4); a busted Plain Wraith is one on the game's source with `live_sources` steady;
  a singed Sour-Doom is one on `source_of(slice)` `Some`, and busting it ends that source in the
  tail; a flyer spawned through `context` (`spawn_flyer`, `flyers.rs:54`) is one on
  `source_of(flyer)`, **and driving it off-screen leaves `source_of(flyer)` `None` after that
  frame's tail** — the visibility exit; **a flyer busted by an icicle is one `bonus` on the game's
  source and the flyer's source `None` after the tail** (kimi F4); a cannon's last life ends its
  source; the three trace cases; the stinger's notes held and released by bounded polling (the
  clear test's shape); the pause jam through the real menu; the bob: a slice's `Sprite.offset.y`
  **drops below the spec's** on a beat frame (Y up: smaller is lower) and is back at the spec's
  within `BEAT_BOB_SECONDS`, its body's position unchanged by it; **a pause opened on a dipped
  frame shows the slices at the spec's offset on its first frame and `pulsed` holds; a Restart
  from a dipped frame spawns the new fleet at the spec's; a loss on a dipped frame (the last
  cannon hit with slices alive) leaves every surviving slice at the spec's on the game-over
  frame**; `restart` by identity; the
  missing bank (`fonts/`, `sprites/` and `ai/music/` linked, no `soundfonts/` — kimi F3). Every
  wait a bounded `poll` from `test_support.rs`, as correction 4 says for Bratdog.
- **`CLAUDE.md`**: § Music (the eleven bindings' table, the four source kinds and why, the flyer's exit,
  the order, the bob, the engine's pause labels and #202, no fallback, telemetry, the bank, the
  versions, no crumb binding); the layout and counts. **Web**: `ASSET_BASE` v6 → **v7**
  (`/games/toast-busters/v7/assets`), `EDITOR_ASSET_BASE` v5 → **v6** (`/playground/invaders/v6/
  assets`), the content file's two lines, `public/games/toast-busters/v7/` and
  `public/playground/invaders/v6/`.

### Correction 7 — the web proof, per game, by batch 13's method

Batch 13's correction 7 is the method and is not restated: each bundle built **once with `--sync`**
into `public/`, the content lines edited, `npm run build` once, every check on that `dist/` under
one `npm run preview`, the preview stopped by its pid, `npm run verify`; a failure rolls the unit
back and the report is INCOMPLETE naming the step. **The rollback, made safe** (round 58, kimi F1;
round 59, kimi F1, codex F1, gemini F6; the preview is stopped by **the pid in
`insiculous_web/.astro/preview.json`, the server's own — a child of `npm`, so never `npm`'s `$!`
— its command line checked first**, as batch 13 did; round 61, gemini F2): step 0 shows `git status --porcelain -- src/web_entry.rs`
in the game and `-- src/content/games/<slug>.md` in the site **empty**, and **both destination
directories absent** (`public/games/<slug>/v7/`, `public/playground/<crate>/v6/` — a leftover from
an earlier attempt is reported, never deleted; `--sync` refuses a non-empty target anyway), and
stops INCOMPLETE otherwise. On a failure the executor **reverses its own edits in place** — the
two version constants back to v6 and v5, the two content lines back, the sentence out — **never
`git checkout`**, and it removes only the directories this attempt synced. **The web unit owns its
three checkouts while it runs** — the loop's standing rule: while a batch is out nobody else edits
or builds in its checkouts (`handoff-loop` § 2), and the executor's session is the one Jesse is
driving — so the lines it reverses and the directories it removes are its own; a concurrent edit
of the same line is not a case the rollback guards against, and three rounds of hardening it stop
here (round 60, kimi F1, codex F1). **The preview is stopped by its recorded pid on every exit** —
before the rollback and before an INCOMPLETE report as before a pass — so a retry never finds the
port held or serves a stale `dist/` (round 60, kimi F5). The game's music code and assets stay
staged: they are the report's, and the planner reads them whatever the web unit did. A leftover
directory from a crashed attempt is reported and the planner clears it by hand; the executor does
not guess what it holds. No Tong overlay this time. Per game:

- **the bank ≤ 4 MiB** (the file's size in the report) and **one throttled Chromium cold boot** of
  the new version to the game's `init complete` line (10 Mbit/s, 100 ms, 4× CPU, a fresh profile,
  batch 12's CDP order, `?telemetry=1`) **under 25 s** — the same ceiling as the Food Pyramid's,
  whose v6 bundle is the same size class (7.88 MiB; Bratdog's v6 and Toast Busters' v6 `game_bg.wasm`
  are both under 8 MiB in `public/`) with a bank 1.3 MiB larger, about 1.1 s more at the emulated
  rate; over 25 s, two more runs and the median; no marker within 90 s is a failed attempt;
- **one two-minute stress run, with a workload floor the driver can reach**: Bratdog on
  **Insiculous solo** (the fast tick, the wrap-around, two snacks) driven **blind, by a
  boustrophedon** — the counter's edge is open in Insiculous, so Frank runs each row to its end,
  steps down one row and runs back, wrapping at the walls; the path never crosses itself, so a dog
  of any length never bites its tail, every snack on the floor is eaten once a sweep (24 × 15
  cells at the 0.05 s floor tick is a sweep every eighteen seconds or so), and the two turns at
  each row's end fit the two-deep input buffer (`buffer_for_snake`) — **closed on the screencast,
  not on the clock** (round 61, gemini F1, codex F1): the driver reads Frank's head cell from the
  DevTools screencast frames batch 13's driver already read Deion from (the 24 × 15 grid's cell
  size and origin measured once from a still), sends the row-end turn pair when the head is seen
  in the row's last cell, never on an elapsed interval that a long frame's second tick would
  overrun; and it sends the confirm key every two seconds throughout — in play `Action1` is read
  nowhere (`gameplay/mod.rs:259-268` reads it in `GameOver` only; `buffer_for_snake` reads the
  four directions), so the key is inert mid-match and restarts a lost one within two seconds. No
  pathfinding to a snack: the sweep eats whatever lies on the floor. The last telemetry line
  shows at least **10 `snacks=` and 120 `turns=`**;
  Toast Busters on **Insiculous solo** (twin volleys) with the driver sweeping and firing, matches
  restarted — at least **120 `shots=` and 30 `busted=`** (amended 2026-10-26 at the starting
  cadence, as 14a's lesson asks: the cannon's `FIRE_COOLDOWN` is 0.4 s and Insiculous allows six
  shots in flight, so 120 fires in two minutes is under half the cannon's rate; a wave is
  `ROSTER.len() × FORMATION_COLUMNS` = 40 slices and the win ends the match, so 40 busted was one
  whole wave busted blind with its last slices at 240 px/s — 30 is three quarters of one, which a
  driver aiming at the fleet from the screencast reaches without a win); batch 12's criterion verbatim (no two
  consecutive windows with a drain gap over 92 ms, the render cost under a block, **`blocks=`
  advancing in every window** — the key as the line prints it, round 60, kimi F2 — zero lines a
  FAIL), a fail removing the synced directories;
- **the functional checks**, each a counter, a state or a still: the song requested before any
  gesture and started on **the first gesture, whichever it is** — the driver's is the title's first
  key after the cards, a person's may be a click on the canvas, and the pending song starts on
  either — **no `Playing song:` line before that gesture, and both `Playing song:` and `Web audio
  enabled after user gesture` on it** (never an order between them; round 58, gemini F7 imagined a
  click that enables the output and a key that starts the song, which is not how a pending song
  behaves); a rally in which the game's counts rise with **`triggers_accepted=`** while
  **`refused=`, `triggers_silent=`, `triggers_rejected=`, `triggers_late=` and `triggers_stale=`**
  hold at zero — **the keys as the line prints them** (`crates/audio/src/manager/song/telemetry.rs:
  73-76`; round 59, kimi F3), **and a line missing any of them is a failed check, never a zero**
  (Toast Busters' `AtSongPosition` pluck included — a rest in the bass line is an arrangement
  defect this catches); `jam_units=` rising on a move in the pause menu and not on its open;
  `pulsed=` rising while a match runs and holding under the pause menu, with the two stills; the
  editor bundle at the game's playground route showing no `Playing song:` while editing, the line
  on Play's first frame, a `reconstructions` step at the first match start, no music line after
  Stop; Firefox the functional checks unthrottled, the editor bundle included; `npm run verify`.
  Two stills per game to `review/conductor/` and `coordination/conductor/screenshots/batch-14/`.
- **The site's profile prompt** will take the keyboard on a fresh profile's first achievement
  (insiculous_web#76): the driver seeds `beinsiculous.achievements.profile-prompt = dismissed`
  before the page loads, as `report-13.md` § 7 did.

### Correction 8 — the docs, and one sentence on each game's page

Each game's `CLAUDE.md` § Music (corrections 4 and 6). `training.md` § Conductor "A game's music
module" names its two worked cases and stays; nothing in the engine changes. Each game's page gains
one sentence after its chaos-modes paragraph (`insiculous_web/src/content/games/{bratdog,
toast-busters}.md`): Bratdog's — "Frank romps to Strauss's *Tritsch-Tratsch Polka*, played live by
the engine's conductor: every snack walks the tune, every turn clicks a paw, and the second dog
answers on the clarinet." — and Toast Busters' — "The Toasted march to Saint-Saëns's *Danse
Macabre*, played live by the engine's conductor: every slice you bust rattles the bones on its own
instrument, and Maxwell's saucer rings midnight." Codex judges the two sentences with the stills.

### Correction 9 — sizes, placements, gates, what stays out

No file in either game is near the ceiling (the largest: snake `gameplay_tests.rs` 478, invaders
`gameplay_tests.rs` 369). New files per game: `src/{music.rs, music_tests.rs}`,
`scripts/prepare_soundfont.sh`, `assets/ai/music/{ai_<slug>.mid, .mid.ron, LICENSE}`,
`assets/soundfonts/{<stem>.sf2, LICENSE}`, `review/conductor/`. Touched: snake `Cargo.toml` (`log`
native, `music_midi` dev-dependency), `src/{lib,types,test_support,web_entry}.rs`,
`src/gameplay/mod.rs` (the moments, `ripple_grid` `pub(crate)`, the pulse), `CLAUDE.md`; invaders
`Cargo.toml` (the same two), `src/{lib,types,test_support,web_entry}.rs`, `src/gameplay/{mod,
players,combat,flyers,formation}.rs`, `CLAUDE.md`; site the two content files and four version
directories. Each game's `Cargo.lock` takes the engine's drift as breakout's did; stage it and say
so.

**Gates**, per handoff: in the game `cargo test`, `cargo clippy --all-targets` with and without
`--features editor`, `../../insiculous_2d/scripts/check_audio_provenance.sh "$PWD"` (broken once by
renaming `Files:`, restored, both runs quoted), the wrapper run with `--bank` and the bank's size
against the cap, the song under 256 KiB, the two `build_wasm.sh` runs with their `wasm size:`
lines, correction 7's checks; in the engine `scripts/check_games.sh --test` (two games' behaviour
changes; nothing else there runs — no engine file is touched); in the site `npm run verify`. The
comment-tag grep and the 600-line check over the game's `src`.

**Leaves out:** a violin (over the cap), a crumb binding (correction 5), bunker chips, shot
cancels, Jammer's glob and the descent as bindings (sound effects with no melody in them; a game
without a binding for a moment plays nothing there, as before), a next wave for Toast Busters
(there is none), the song speeding up with the fleet (intensity layers), the pause hint (#202), a
recorded fallback (e), `Sequence` (batch 13 said why), the other two games (batch 15), the game
template (game-template#3), Jesse's promotion of either song to `music/`.

**The rulings' alternatives were decided at settling and neither was taken (2026-10-26), so the
handoff carries the shape above; what either would have reached stays as the record** (round 59,
kimi F4): if Jesse cuts `bonus` or `hit` under (x), the cut removes that moment's track, part,
binding, `Music` method and call site, count and telemetry key, test and table row — one list, all
or none — and the plan is edited before the handoff is written; if he swaps the oboe for the
clarinet under (w), the wrapper's preset, the `clarinet` instrument's program **and the program of
every clarinet track — `melody`, `hurt`, `stinger`, `jam` — since a background track plays its own
program and a trigger track's program is what the exported MIDI declares** (round 61, kimi F1) and
the bank's size line change, the instrument renamed `oboe`, and nothing else. The executor never
sees an alternative.

**Rulings, settled 2026-10-26 (Jesse: everything as proposed):** **(t)** one section, two
handoffs — 14a Bratdog then 14b Toast Busters, each reviewed, heard and committed before the next;
**(u)** Bratdog's bank is FluidR3's accordion, clarinet, tuba, trombone and woodblock, about 3.5 MiB,
committed under the 4 MiB cap; **(v)** Bratdog's five bindings: `snack` and `snack_two` walk one
tune on the accordion and the clarinet on each dog's own source, `turn` a woodblock click on the
dog's source, `death` one trombone blat on the game's source, `menu` the jam; **(w)** Toast
Busters' bank is xylophone, clarinet, tubular bells and acoustic bass, about 3.5 MiB — no violin
fits under the cap, the clarinet carries the tune, the oboe offered as a swap and declined; **(x)**
Toast Busters' eleven bindings as correction 5 lists them, no crumb binding, `bonus` and `hit` offered
as cuts and kept; **(y)** the beat visual is the grid under each dog's head in Bratdog and the
fleet's bob in Toast Busters — in the slices' sprite offsets, never their bodies — no grid there
by the 2026-10-13 ruling, codex judging both from stills;
**(z)** the pause menu keeps the engine's labels in both games until #202 decides the hint;
**(aa)** each game's page gains the one sentence of correction 8.

## Batch 15 — K6: Meatieroids and Chicken Coop (#65)

**Re-verified 2026-10-26 against the tree at 88ebe82** (engine), `games/asteroids` f8cab8e,
`games/frogger` d9ae20c and `insiculous_web` 8cf69f6, after batch 14 landed (two more worked games,
`GameMusic` unchanged, the `tune` lesson and #223). **Settled at v4 on 2026-10-26, rulings (ab) to (ah) as proposed — `life` kept, one `death` binding, Mars at sixty seconds** (Jesse, on the planner's recommendation after round 67). v4 after plan rounds 65 to 67 — no contract has moved since v2's cues became notes or chords, and rounds 66 and 67 hardened gates and tests (67: kimi's findings never written, two lines from its log — the FluidR3 fetch, the match-ending frame's pulse; gemini 5 — the shared parts' register and a bank-region test, the fifth nest plays the cadence alone, the loop-wrap wait as the driver's poll; its source-pruning, `World` and `triggers_silent` claims false; codex 0; `rebuttal-67.md`. 66: kimi 2, gemini 5, codex 2 — the ostinato written in ticks and asserted per bar, the crash trace asserted on both deaths, Chicken Coop's hop floor at the road's cadence, the drivers' title walk stated, Chicken Coop's `ripple_grid` a free function, no pulse in the round-clear second, a live ship is one with an entity, one loop wrap recorded under the pause menu; `rebuttal-66.md`. 65: kimi 5, gemini 7, codex 1 — a cue is a note or a chord, never a phrase; the cadence one voice; the web precondition path-scoped; the tempo asserted; the missing-bank fixtures link the song; the counts session-cumulative; the break sent before the points; the stress floors read from the run's own page load; gemini's two Criticals misread the instrument slots and the loop seam; `rebuttal-65.md`). Nine corrections to the two
bullets the section was; each names what the tree says. **The coordinate convention throughout:
world Y is up.**

Executor: the Opus session, **one handoff per game, in order** (ruling (t) stands): **15a
Meatieroids** in `games/asteroids` + `insiculous_web`, then **15b Chicken Coop** in `games/frogger`
+ `insiculous_web`; each with one report at the engine's `review/conductor/report-15a.md` /
`report-15b.md` (the engine's tree is not touched beyond the stills). Reviewers per handoff: kimi
(everything), gemini (the game's tests and the bank wrapper), codex (the beat visual from two
stills, the page sentence). Listen check: Jesse per game, natively and in Firefox and Chromium.
Size: about +700 lines a game plus the assets (a song, its sidecar, a 2.5 and a 3.4 MiB SoundFont
subset); each game's diff is reviewed from its own `review/conductor/`, the site's from
`insiculous_2d/review/conductor/`.

**Standing:** batch 12's rulings (e) no recorded fallback, (f) the song plays on under the pause
menu and the jam reads `MenuInput`, (h) a committed per-game bank with its `LICENSE`, (i) hits on
`quantize: Off`, (k) the bank is a boot dependency on the web until #194; batch 13's (l) the game
module is `engine_core::game_music::GameMusic`, (m) a per-game bank under a **4 MiB cap** the
wrapper enforces, (o) the boot gate is the cap plus one recorded throttled cold boot under 25 s and
one two-minute stress run, (p) the pause menu is the one menu that jams, (r) one sentence on each
game's page; batch 14's (t) one section, two handoffs, (z) the pause menu keeps its labels until
#202; the K6 rule (3b, door (b)); **"What batch 13 left"** and **"What 14a left" / "What 14b
left"** in this section's parents — the module shape, the wrapper with `"$@"` first, the gesture
check as two lines on the gesture, **the pulse after the frame's movement for a world-space
anchor**, **a stress floor computed at the game's starting cadence**, **the loop's end is a written
note's end**, **the tune on its own named track with `melody` left empty** (#223), **no test on the
ordering of an entity-relative visual**.

### Correction 1 — what the two trees hold today

Neither game has a line of sound: `grep -rni "sound\|music\|audio\|beep\|conductor"` over each
`src/`, `Cargo.toml` and `scripts/` prints nothing; neither has `soundfonts/`, `ai/` or a `LICENSE`
under `assets/`. **Meatieroids** carries `log = "0.4"` only under the wasm target
(`Cargo.toml:26-31`), so it moves to `[dependencies]` as the other four games'; **Chicken Coop's
is native already** (`Cargo.toml` `[dependencies]`). Neither has `music_midi` as a dev-dependency.
Both guides' test counts are stale (asteroids says 65, `cargo test` prints 67 — `title_screen_tests`
(2) missing; frogger says 84, prints 87 — `title_screen_tests` (2) and `constants` (1) missing); the
batch rewrites the counts anyway. Both editor bundles keep the crate's slug under the game's route,
the site's two-name contract (asteroids: `/playground/meatieroids/` → `/playground/asteroids/v4/`,
`web_entry.rs:47`; frogger: `/playground/chicken-coop/` → `/playground/frogger/v4/`, `:47`), and
both game bundles are at **v5** (`web_entry.rs:36` in each; the site's `meatieroids.md:6-7`,
`chicken-coop.md:6-7`). Both carry ` M Cargo.lock` (the engine's drift, 27 lines each) — staged with
the batch as 14's were.

**Meatieroids** has the engine's `GridBackdrop` (`spawning.rs:118-128`, `BehindSprites`) and a free
`ripple_grid(world, pos, strength, radius)` at `gameplay/mod.rs:143-145`, already rippled on a rock
break (`GRID_IMPULSE_BREAK[size]`, `constants.rs:80-81`, `rocks.rs:244-245`) and a ship's death
(`GRID_IMPULSE_SHIP_DEATH`, `constants.rs:82`, `ship.rs:248-249`); it becomes `pub(crate)` for
`music.rs`, as Bratdog's did. **It has no hyperspace** — `grep -rni hyperspace` prints nothing; #65's
"thrust and hyperspace voices" names a move the re-skin never had (asteroids#2 is the next move,
the cake saucer) — **so there is no hyperspace binding**, and nothing else from #65's line is lost.
**Thrust is a held state, not an edge** (`ship.rs:163-166, :174-175` read `move_y > 0 ||
is_active(MoveUp)` each frame into `ship.thrusting` at `:132`); the previous frame's value is in
`self.ships[index].thrusting` right before that write, so a rising edge is one comparison there.
The match is endless (`CLAUDE.md:21`; `finish_game` `flow.rs:75-86` is reached only from
`kill_ship` `ship.rs:259-261` when every ship is out); waves follow each other through
`check_wave_clear` (`rocks.rs:306-330`: the clear edge at `:310-317` arms a 2 s breath, the spawn
edge at `:319-329` spawns the next, bigger wave). **A harness test pins every rock's `Sprite.offset`
to its spec each frame** (`harness_tests.rs:26-45`, the tumble test), so no bob on the rocks — the
grid is the visual here anyway. `gameplay_tests.rs` is **760 lines**, over the 600 cap before this
batch: not this batch's to split (asteroids#4); the batch adds no line there.

**Chicken Coop** has the `GridBackdrop` too (`spawning.rs:377-388`, `OverSprites`, alpha 0.25,
spawned per match at `flow.rs:63`), rippled on a death (`GRID_IMPULSE_DEATH`, `flow.rs:102`) and a
nest filled (`GRID_IMPULSE_HOME`, `flow.rs:139`), through a **method** `ripple_grid(&mut self,
world, position, (strength, radius))` at `gameplay/mod.rs:379-381` — already `pub(crate)`. It is
**localized** (`assets/locales/{en,pirate}.ron`, parity held by `locale_files_have_matching_keys`)
and its pause menu draws **its own localized labels** through `draw_labeled` (`drawing.rs:168-181`,
`pause.hint` from the locale) — the hint stays as it is (ruling (z), #202); no locale key is added
or changed by this batch (no string the music needs is on screen). There is **no physics crate**:
collisions are lane arithmetic in `rules.rs`; the chicken entity is **never destroyed** in a match
(hidden by `sprite.visible` `mod.rs:302-308`, reset in place by `step_respawns` `mod.rs:257-284`),
and the lane entities are spawned once per match on a torus (`LANE_PERIOD` 816, `constants.rs:31-33`;
`spawn_lane` `flow.rs:47-52`) and never despawned until `clear_match_entities` (`flow.rs:287-312`).
Rounds are endless (`advance_round` `flow.rs:196-213`); the match ends in `finish_game`
(`flow.rs:266-273`) from `step_respawns` when every chicken is retired. Its window is **720 × 768**
(`main.rs:155-156`) — the stills and the driver read that canvas, not 800 × 600. `update` has no
frame counter at its top (`lib.rs:121-130`): `music.update` is the first line. `reset_to_title`
takes `ctx` (`flow.rs:275-278`), `start_game` ends with `apply_theme` then `state = Playing`
(`flow.rs:77-78`).

### Correction 2 — one section, two handoffs

Ruling (t) stands as batch 14 ran it: **15a Meatieroids** first, **15b Chicken Coop** after 15a's
review round, each with its own report, review round, listen check and commits; the site's second
commit carries the second game's bundles. One batch out at a time: 15b is not handed off while 15a's
report is unreviewed. The handoffs start from 14b's kept tools (`review/conductor/batch14b-tools/`,
the drivers and the tracker, with 14a's helpers beside them) and logs.

### Correction 3 — Meatieroids' song: *Mars*, its shape, its bank

`games/asteroids/assets/ai/music/ai_meatieroids.mid` + `.mid.ron`: Gustav Holst's *Mars, the
Bringer of War* (*The Planets*, 1914–16, published 1921; public domain) in a short arrangement the
executor sequences **in the composer on the asteroids project** (14b's recipe: the bank first,
`target/debug/editor . --headless`, `song new`, `SetTempo`, **the meter**, `AddTrack` with
explicit channels, `AddNote` in `Batch`es, `AddPart`, `AddInstrument`, `SetBinding`,
`SetProvenance`, the `song` and `song notes` answers quoted, a dry run without the save, `song save
ai/music/ai_meatieroids.mid`), from memory — the executor cannot hear; it declares the shape, and
Jesse's ear is the gate. **The meter is 5/4, set, not assumed**: the first edit after `SetTempo` is
`{"SetTimeSignature": {"tick": 0, "meter": {"numerator": 5, "denominator": 4}}}`, its `"ok":true`
required before the first `AddNote`; `music_tests.rs` reads the committed MIDI's signature at tick 0
and asserts 5/4 — a 4/4 default would put the bar pulse one beat earlier every bar. **`melody` stays
empty** (14b's lesson, #223): every sounding track is an `AddTrack` with its program. Shape:

- **thirty-two bars of 5/4 at 160 BPM** (375,000 µs a quarter; 2,400 ticks a bar at 480; the loop
  ends at tick **76,800**, 60 s — Mars is a long crescendo and a match is long), on G (Holst's
  pedal), looping on the ostinato's downbeat; **the loop's end is a written note's end**: the tuba
  pedal's last note is held to tick 76,800, and a test holds the meter and the end;
- **background**: `ostinato` (timpani 0:47 — Mars's figure on G every bar: **a triplet on beat one, quarters on
  two and three, two eighths on four, a quarter on five — note starts at bar offsets 0, 160, 320,
  480, 960, 1,440, 1,680 and 1,920 ticks, the last ending on 2,400** (round 66, kimi F1, codex F1);
  in the score it is col legno strings, timpani and harp, and no string preset fits under the cap),
  `pedal` (tuba 0:58, the G pedal and the bass theme's low voice, the one held to the bar line) and
  `calls` (trumpet 0:56, the fanfare calls of the second phrase, sparse) on their own tracks — a
  track a part selects plays in no background, so the trigger tracks below are separate even where
  an instrument repeats;
- **track `theme`** (tuba 0:58 as its own program; the brass theme — Mars's rising first subject
  over the ostinato, written as quarter-or-longer units, 24 to 32 of them, **in the octave the
  tuba, the trombone and the trumpet all carry, G3 to D4** — round 67, gemini F1): **the one shared part
  `theme`** (`Loop`, `quantize: Off`), played by **three bindings on three instruments, one a rock
  size — `large` on the tuba, `medium` on the trombone (0:57), `small` on the trumpet (0:56)** —
  #65's "three rock sizes as three instruments on one part"; a wave is walked through the theme in
  the order the rocks break, the big ones low;
- **track `drum`** (timpani 0:47): a gap-free line of quarters on G from tick 0 to 76,800 — the
  ostinato's beat, one note a quarter, **every unit ending where the next begins** — the part `drum`
  in **`AtSongPosition`** (`Off`) on the `shot` binding: a shot drums the beat sounding at that
  moment, so a volley sits on the ostinato instead of beside it; a rest in that line is an
  arrangement defect the `triggers_silent=` key catches (14's rule);
- **track `surge`** (tuba): **one low held note, one unit** (two beats), the part `surge` (`Loop`,
  `Off`) on the `thrust` binding — the engines, once per burst of thrust;
- **track `crash`** (orchestra hit 0:55): **one chord, one unit** — Mars's last chord on the hit; **a
  unit is the notes starting on one tick (`MusicalUnit`), and `Loop` plays one unit a trigger, so a
  cue is a note or a chord, never a phrase** (round 65, codex F1) — the part `crash` (`Loop`, `Off`) on the `crash` binding, a ship lost;
- **track `fanfare`** (trumpet): **one chord, one unit** — the closing fifth of the trumpet call that
  opens the second phrase — the part `fanfare` (`Loop`, `Off`) on the `wave` binding — the next wave arriving;
- **track `life`** (trumpet): **one short high note, one unit**, the part `life` (`Loop`, `Off`) on the `life`
  binding — an extra ship at 10,000 (ruling (ad) offers it as a cut);
- **track `jam`** (trumpet): the calls as a slow counter-line, eight units, the part `jam` (`Loop`,
  `Off`) on `menu`;
- every part `quantize: Off`; `soundfont: "soundfonts/meatieroids.sf2"`; instruments `timpani`
  (0:47), `tuba` (0:58), `trombone` (0:57), `trumpet` (0:56), `hit` (0:55); `provenance: (author:
  "Gustav Holst, 1914–16; sequenced by the Opus executor for Be Insiculous, 2026", license:
  "CC0-1.0")`; no sample instrument. **Eleven tracks past the tempo track** — `melody` (empty),
  `ostinato`, `pedal`, `calls`, `theme`, `drum`, `surge`, `crash`, `fanfare`, `life`, `jam`
  (channels 1–9, 11, 12; 10 is percussion); **eight bindings** — `shot`, `large`, `medium`,
  `small`, `thrust`, `crash`, `wave`, `life` — plus `menu`: **nine**.

**No binding for hyperspace** (correction 1: there is none), **none for a wave's clear edge** (the
2 s breath is the signal; the `wave` fanfare lands when the rocks do), none for a bullet expiring,
a rock–rock bounce or a close call.

**The bank: `games/asteroids/assets/soundfonts/meatieroids.sf2`**, FluidR3 GM **0:47 Timpani
(1,361 KiB), 0:58 Tuba (190), 0:57 Trombone (438), 0:56 Trumpet (226), 0:55 Orchestra Hit (262)**
— about **2,477 KiB, 2.42 MiB**, under the cap with room. No strings (0:48 Strings 5,466 KiB;
0:44 Tremolo 5,466; 0:40 Violin 3,005; 0:45 Pizzicato 2,729 — any one with the timpani is over
4 MiB) and **no horns** (0:60 French Horns 2,953 — with the timpani 4,314 KiB, over): the brass theme
is tuba, trombone and trumpet. The wrapper is the fifth: `… "$@" --max-mib 4 "$OUTPUT_DIR"
meatieroids 0:47 0:58 0:57 0:56 0:55`.

### Correction 4 — Meatieroids' code: seven moments, two source kinds, the ships' lifecycle

- **`src/music.rs`** (new, under 190 lines): the constants at its top (`SONG_PATH =
  "ai/music/ai_meatieroids.mid"`, the nine binding names — `SHOT_BINDING`, `LARGE_BINDING`,
  `MEDIUM_BINDING`, `SMALL_BINDING`, `THRUST_BINDING`, `CRASH_BINDING`, `WAVE_BINDING`,
  `LIFE_BINDING`, `MENU_BINDING` — `BEAT_PULSE_STRENGTH`, `BEAT_PULSE_RADIUS` and
  `BEAT_PULSE_FRACTION = 0.25`), `Moment { Shot(EntityId), Break(AsteroidSize), Thrust(EntityId),
  Crash, Wave, Life }` and `Music { game, counts: MusicCounts { shots, broken: [u64; 3], thrusts,
  crashes, waves, lives }, #[cfg(test)] triggered, #[cfg(test)] pulsed_at }` — **the counts are
  session-cumulative, never reset by `restart`**, as Bratdog's and Toast Busters' are (14b's last
  line: 263 shots across eleven restarts; round 65, gemini F2) — on `AsteroidsGame`
  (`types.rs:175-225`, `Default` at `:227-253`), built with `GameMusic::new(SONG_PATH,
  "meatieroids", "MEATIEROIDS_TELEMETRY")`; one `play(ctx, Moment)` (14b's table shape: binding,
  source, count on one line a moment — `Break(size)` to the size's binding by `AsteroidSize::idx`,
  `types.rs:87-116`); `follow_pause`; `pulse(ctx, &ships)`; `update`. Wiring: `start` at the end of
  `init` (`lib.rs:112`, after the backdrop) + `log::info!("meatieroids: init complete")`; `update`
  at the top of `update` after the frame count (`lib.rs:116`); `restart` at the end of
  `start_game` (`flow.rs:30-44` — the mode select `menu.rs:132`, the pause menu's Restart
  `gameplay/mod.rs:57`, the game-over restart `flow.rs:21`); `follow_pause` in the `Resumed` arm
  (`gameplay/mod.rs:62`, today a bare `return`) and at the gate's end (`:69`); the pulse after
  `wrap_entities` (`:92`) while `Playing` — a world-space anchor, after the frame's movement (14a's
  lesson).
- **The moments and their sources.** A shot: `update_firing` (`ship.rs:182-197`, the one shot at
  `:195` after `can_fire` — cooldown 0.22 s, four live, no chaos variant) → `shot` **on the ship's
  own source, `ctx.sources.entity(entity)`** (`:192`). A rock broken: `shatter_asteroid`
  (`rocks.rs:221-264`; the parent destroyed at `:228` in the same call, its children spawned at
  `:263`) → the size's binding **on the game's source**, **sent right after the destroy at `:228`
  and before `award_points` at `:247`** (round 65, gemini F3) — the rock is gone in that call and
  its note rings on; a `Small` has no children and the same binding. Thrust: `update_ship_control`
  (`ship.rs:118-135`): **the rising edge** — `thrust && !self.ships[index].thrusting`, read before
  `ship.thrusting = thrust` at `:132` — → `thrust` **on the ship's source**; holding thrust sends
  nothing more (the unit ends by its length), releasing sends nothing, and the edge is never read
  off-Playing or for a dead ship (`:119-123, :127` force `thrusting` false, so the next thrust in
  a new match is an edge). A ship killed: `kill_ship` (`ship.rs:230-266`) → `crash` **on the game's
  source** (on the last life the entity is destroyed at `:256` in the same call, ending its `shot`
  and `thrust` source in that tail; a surviving ship is teleported and keeps its source). The next
  wave: `check_wave_clear`'s spawn edge (`rocks.rs:319-329`, after `spawn_wave`) → `wave` on the
  game's source. An extra ship: `award_points`'s `while` (`flow.rs:143-146`) → `life` on the game's
  source, once per life granted — `award_points` runs inside `shatter_asteroid`, so a break that
  earns a life traces `["large", "life"]`. The menu's jam is the helper's.
- **Nothing is ever off-screen by policy**: `wrap_entities` (`rocks.rs:283-302`) teleports every
  ship, bullet and rock at its margin, and a wave's rocks spawn on the border half in view
  (`border_point`, `rocks.rs:102-115`) — the game's camera policy is "everything in the window is
  visible", so there is **no visibility exit** and no `ctx.sources.end` call; a ship's source ends
  with its entity on the last life, a rock is never a source, the game's is never ended.
- **The order within a frame** is `update_gameplay`'s (`gameplay/mod.rs:42-95`): `update_ship_control`
  (thrust, then shot, each ship in index order) → physics → … → `resolve_bullet_hits` (breaks, and
  a `life` inside one) → `check_ship_hit` (crash) → `check_wave_clear` (wave). Trace cases through
  the production path: `["thrust", "shot"]` — a press of thrust and fire on one frame;
  `["large", "medium"]` — two bullets of one ship on two rocks in one frame, in the hit pairs'
  order; `["small", "wave"]` is not one frame (the breath is 2 s), so **`["small"]` then, 2 s of
  frames later, `["wave"]`** with the field empty between.
- **The beat visual (ruling (ac)): the grid ripples under each live ship** (one whose `entity` is
  `Some`) — Bar the full impulse,
  Beat `BEAT_PULSE_FRACTION` of it — through `ripple_grid` made `pub(crate)`, after `wrap_entities`
  so the ripple lands where the ship is drawn this frame; nothing while paused (the frozen grid
  discards impulses) and nothing in `GameOver`. In 5/4 that is a bar every 1.875 s and four lesser
  beats between; codex judges from two stills (a bar's frame, a rest frame) and may cut it to bars
  (`BEAT_PULSE_FRACTION = 0.0`). Not on the rocks' sprites (the tumble test, correction 1).
- **Tests** (`src/music_tests.rs`, new, under 450 lines; `test_support.rs` gains `press`/`release`
  (exist), `step_with`, `settle`, `poll`, `drawn_at` in 14b's shape — asteroids has no `step_with`
  or `drawn_at` today; `empty_field(mode, chaos)` (`test_support.rs:43`) is the match fixture):
  the sidecar's structure (the three size bindings one part on three instruments, `shot`'s part
  `AtSongPosition`, the rest `Loop`, every part `Off`, provenance, the bank's path, **and each cue
  part — `surge`, `crash`, `fanfare`, `life` — holding exactly one unit in the committed MIDI**);
  **every key of the shared `theme` track lies in a region of each instrument bound to it**,
  read from the committed bank through `music_midi::rustysynth` (a region without a key range
  covers every key; round 67); **the `drum` line tiles the loop**, the committed meter is 5/4 with
  the end at 76,800, **and the
  tick-0 tempo is 375,000 µs with the loop 60.0 s long through `TempoMap`** (round 65, kimi F4),
  **and every bar of the committed `ostinato` track starts its eight notes at the offsets above**
  (round 66); the song after
  `init`; a shot is one accepted trigger on `source_of(ship)` with `silent_triggers` unchanged, and
  a press under the cooldown or the cap sends nothing; **one rock of each size broken through the
  production path (a bullet aimed by `place_meatieroid` + `spawn_bullet`), the trace naming
  `large`, `medium`, `small` in turn** on the game's source with `live_sources` steady; a thrust
  press is one `thrust` on the ship's source and a held thrust over thirty frames is still one, a
  release and a second press two; a ship's last life ends its source, a surviving death keeps it, **each tracing `["crash"]` with
  `crashes` up by one** (round 66, kimi F2);
  the extra life traced inside the break; the `wave` on the spawn edge and nothing on the clear
  edge; the two one-frame trace cases; the pause jam through the real menu (14b's case); the
  pulse: `pulsed_at` is each live ship's position after that frame's wrap — **a live ship is one
  whose `entity` is `Some` (`types.rs:38`; `None` after the last life, `ship.rs:257`), Bratdog's
  `live_heads()` shape, no transform unwrapped** (round 66, gemini F5) — read on the beat frame,
  none while paused, a co-op ship out of lives is not an anchor, **and the frame that ends the
  match — the last life's crash — records no `pulsed_at`** (round 67, kimi); `restart` by identity; the missing bank (`fonts/`, `sprites/` **and `ai/music/`** linked, no
  `soundfonts/` — 14b's test 16's contract: `song_failed`, two `failures` after a match start, no
  label, a shot traced, refused and counted, the match still `Playing`; round 65, kimi F3, gemini
  F6). Every wait a bounded `poll`.
- **`CLAUDE.md`**: § Music (the nine bindings' table, the two source kinds, no hyperspace and why,
  the order, the pulse, the pause labels and #202, no fallback, telemetry, the bank, the versions);
  the layout and counts (67 → 67 + the music tests; the stale 65 corrected). **Web**: `ASSET_BASE`
  v5 → **v6** (`/games/meatieroids/v6/assets`), `EDITOR_ASSET_BASE` v4 → **v5**
  (`/playground/asteroids/v5/assets`), the content file's two lines, `public/games/meatieroids/v6/`
  and `public/playground/asteroids/v5/`.

### Correction 5 — Chicken Coop's song: the *William Tell* finale, its shape, its bank

`games/frogger/assets/ai/music/ai_chicken_coop.mid` + `.mid.ron`: Gioachino Rossini's *William
Tell* overture, the finale (1829; public domain), sequenced the same way on the frogger project.
**The meter is 2/4, set** (`{"numerator": 2, "denominator": 4}`), asserted by a test; `melody`
stays empty. Shape:

- **forty-eight bars of 2/4 at 152 BPM** (394,737 µs a quarter; 960 ticks a bar; the loop ends at
  tick **46,080**, about 38 s), in E major or the executor's declared key: the trumpet's opening
  call (eight bars) then the galop (forty), looping on the call's downbeat; **the tuba's last note
  held to tick 46,080**, a test holding the meter and the end;
- **background**: `bass` (tuba 0:58, the galop's oom), `chords` (trombone 0:57, the pah), `drums`
  (timpani 0:47, the galop's gallop rhythm ♪♪♩) and `counter` (piccolo 0:72, the tune's second
  voice in the galop) on their own tracks;
- **track `galop`** (trumpet 0:56): the galop's tune — the *Lone Ranger* theme — as eighths and
  quarters, **in the register the trumpet and the piccolo share, G4 to G5** (round 67, gemini F1), **the one shared part `galop`** (`Loop`, `Off`), played by **two bindings on two
  instruments — `hop` on the trumpet and `hop_two` on the piccolo**: the first chicken walks the
  tune a hop a note, the second the same tune on the piccolo (co-op only; in solo `hop_two` is
  never sent) — Bratdog's `snack`/`snack_two` shape;
- **track `squawk`** (trombone): **one low note, one unit** — the part `squawk` (`Loop`,
  `Off`) on the `death` binding, **every cause** (`DeathCause`, `types.rs:307-317`: cart, soup,
  swept, timeout, a missed nest, the bun) one binding; two chickens dying on one frame send it
  twice on the game's source;
- **track `fanfare`** (trumpet): **one chord, one unit** — the closing fifth of the overture's
  opening call (a cue is a note or a chord, correction 3) — the part `fanfare` (`Loop`, `Off`) on the `home` binding — a nest filled;
- **track `cadence`** (trumpet): the finale's closing chord, **one chord, one unit** — one track,
  one program, so the trumpet's alone (round 65, kimi F5, gemini F4) — the part `clear` (`Loop`, `Off`) on `clear` — the fifth
  nest, the round cleared;
- **track `jam`** (piccolo): the *Ranz des Vaches* — the overture's pastoral, the song's slowest
  phrase — eight units, the part `jam` (`Loop`, `Off`) on `menu`;
- every part `quantize: Off`; `soundfont: "soundfonts/chicken_coop.sf2"`; instruments `trumpet`
  (0:56), `piccolo` (0:72), `trombone` (0:57), `tuba` (0:58), `timpani` (0:47); `provenance:
  (author: "Gioachino Rossini, 1829; sequenced by the Opus executor for Be Insiculous, 2026",
  license: "CC0-1.0")`. **Ten tracks past the tempo track** — `melody` (empty), `bass`, `chords`,
  `drums`, `counter`, `galop`, `squawk`, `fanfare`, `cadence`, `jam` (channels 1–9, 11); **five
  bindings** — `hop`, `hop_two`, `death`, `home`, `clear` — plus `menu`: **six**.

**No binding for a cart passing, a raft, the bun snapping, the cracker sinking or the attempt
timer's last seconds** (hazards with no player act behind them — the timer's end is a `death`), and
**none for the +10 a new row pays** (it is the hop).

**The bank: `games/frogger/assets/soundfonts/chicken_coop.sf2`**, FluidR3 GM **0:56 Trumpet
(226 KiB), 0:72 Piccolo (1,177), 0:57 Trombone (438), 0:58 Tuba (190), 0:47 Timpani (1,361)** —
about **3,392 KiB, 3.31 MiB**, under the cap. No horns (2,953 with the piccolo and the timpani is
5,491 KiB), no strings; the galop's horn call is the trumpet's. The wrapper is the sixth: `… "$@"
--max-mib 4 "$OUTPUT_DIR" chicken_coop 0:56 0:72 0:57 0:58 0:47`.

### Correction 6 — Chicken Coop's code: five moments, two source kinds, chickens that never die

- **`src/music.rs`** (new, under 190 lines): the constants (`SONG_PATH = "ai/music/ai_chicken_coop.mid"`,
  `HOP_BINDING`, `HOP_TWO_BINDING`, `DEATH_BINDING`, `HOME_BINDING`, `CLEAR_BINDING`,
  `MENU_BINDING`, `BEAT_PULSE_STRENGTH`, `BEAT_PULSE_RADIUS`, `BEAT_PULSE_FRACTION = 0.25`),
  `Moment { Hop { chicken: EntityId, index: usize }, Death, Home, Clear }` and `Music { game, counts:
  MusicCounts { hops, deaths, homes, clears }, #[cfg(test)] triggered, #[cfg(test)] pulsed_at }` on
  `FroggerGame` (`types.rs:412-464`, `Default` at `:466-495`), built with `GameMusic::new(SONG_PATH,
  "chicken coop", "CHICKEN_COOP_TELEMETRY")`; `play(ctx, Moment)` sends `hop` for index 0 and
  `hop_two` for index 1 (`Pilot`'s shape: P1 the round chicken, P2 the tall, `types.rs:368-374`) on
  the chicken's source, the rest on the game's. Wiring: `start` at the end of `init` (`lib.rs:114`,
  after the background) + `log::info!("chicken coop: init complete")`; `update` as **the first line
  of `update`** (`lib.rs:121`); `restart` at the end of `start_game` (`flow.rs:78`, after `state =
  Playing` — the mode select `menu.rs:147`, the pause menu's Restart `gameplay/mod.rs:54`, the
  game-over restart `flow.rs:22`); `follow_pause` in the `Resumed` arm (`gameplay/mod.rs:59`) and at
  the gate's end (`:65`); the pulse after `sync_sprites` (`:90`) while `Playing`.
- **The moments and their sources.** A hop: `try_hop` (`gameplay/mod.rs:179-253`; the edge at
  `:186-188`, `just_activated`, one hop a chicken a frame, no repeat, no cooldown) → `hop` /
  `hop_two` **on the chicken's own source** (`ChickenState.entity`, `types.rs:262`), sent where the
  pose is set (`:216-218`) — **a clamped hop at an edge counts** (it poses and puffs, `CLAUDE.md`'s
  rule), and a hop that lands in a cart, the soup, a wall or the bun plays its note and then the
  `death` the landing resolves (`:225-252`), on the same frame. A death: `kill_chicken`
  (`flow.rs:84-111`) → `death` **on the game's source** — the chicken is not destroyed, it is hidden
  and reset in place, so its source lives the whole match and a note on it would be released by
  nothing; the squawk must ring whatever the chicken does next. A nest filled: `fill_home`
  (`flow.rs:116-159`, the chicken sent back to its start at `:153-154`) → `home` on the game's
  source **when the round is not complete after it**; the fill that completes the round sends
  `clear` alone from `award_round_clear` (`:156-158`) — a fanfare under the cadence is two trumpet
  chords with one meaning (round 67, gemini F3) — so the fifth fill traces `["hop", "clear"]`. The round cleared: `award_round_clear` (`flow.rs:166-191`) → `clear` on the game's
  source; `advance_round` (`:196-213`, 1 s later) sends nothing. The menu's jam is the helper's.
- **The lifecycle**: a chicken's source activates on its first hop and ends in `clear_match_entities`
  (`flow.rs:287-312`) when the match's entities go — a restart or the title — so the next match's
  first hop activates anew; the lanes' entities never despawn in a match and carry no binding; the
  seam's ghost sprites are not entities a sound could sit on. The camera policy is the window: a
  cart half past the seam is a drawn ghost, not an exit. No `ctx.sources.end` call.
- **The order within a frame** is `update_gameplay`'s (`gameplay/mod.rs:45-93`): `step_chickens`
  (`:166` → `try_hop`: hop, then its landing's death or home, chicken 0 then 1) → `step_respawns`.
  Trace cases: `["hop", "death"]` — a hop into a cart; `["hop", "home"]` — a nest; `["hop",
  "clear"]` — the fifth nest, no `home`;
  `["hop", "hop_two"]` — both chickens hopping on one frame in co-op, in index order.
- **The beat visual (ruling (ac)): the grid ripples under each active chicken** — Bar the full
  impulse, Beat a quarter — through `ripple_grid` **made a `pub(crate)` free function `ripple_grid(world, position,
  (strength, radius))`** in `gameplay/mod.rs` — today a method on `FroggerGame` (`:379-381`) that
  `Music::pulse` cannot call; its two callers (`flow.rs:102`, `:139`) updated (round 66, gemini
  F3) — after `sync_sprites` so the ripple sits under the drawn chicken; a hidden chicken (dead, waiting its 1 s respawn; retired)
  gets none; nothing while paused, nothing in `GameOver`, **and nothing during the round-clear second
  (`round_clear_in` is `Some`): `pulsed_at` empty, as under the pause menu** (round 66, gemini F4). Codex judges from two
  stills and may cut it to bars.
- **Tests** (`src/music_tests.rs`, new, under 450 lines; `test_support.rs` (144 lines) gains `press`,
  `release`, `step_with`, `settle`, `poll`, `drawn_at` in 14b's shape; `harness(mode, chaos)` and
  `start_match` exist at `:130-144`; `flow_tests.rs`'s `fill_nest`/`arm_beat` (`:30-41`) move to
  `test_support.rs` as `pub(crate)` for the music tests): the sidecar's structure (`hop` and
  `hop_two` one part on two instruments, every part `Loop` and `Off`, provenance, the bank, and
  each cue part — `squawk`, `fanfare`, `cadence` — holding exactly one unit, **and every key of
  `galop` in a region of the trumpet's and the piccolo's presets** — correction 4's bank test); the
  committed meter 2/4, the end at 46,080, the tick-0 tempo 394,737 µs and the loop about 37.9 s through `TempoMap`; the song after `init`; a hop is one accepted trigger on
  `source_of(chicken)` (a W press through the real input), a clamped hop too; in co-op the second
  chicken's hop is `hop_two` on its own source and in solo `hop_two` is never sent over a walk
  (counter zero); a hop into a cart traces `["hop", "death"]` with `source_of(chicken)` still `Some`
  after the tail (the entity lives); a drown, a timeout and a swept each trace `["death"]` (the
  cause table's three paths through `kill_chicken`); a nest `["hop", "home"]` and `homes` up by one; the fifth nest `["hop", "clear"]` with
  `homes` unchanged and `GameOver` not reached; a restart ends the chicken's source (`clear_match_entities`) and the next
  match's hop activates a new one; the pause jam through the real localized menu (`drawn_at` on the
  locale's "Resume"/"Restart" strings — the en locale under test); the pulse: `pulsed_at` is each
  active chicken's position on the beat frame, none for a hidden chicken, none while paused, none
  during the round-clear second, **none on the frame the last chicken retires** (round 67, kimi);
  `restart` by identity; the missing bank (`fonts/`, `locales/`, `sprites/` and `ai/music/` linked,
  no `soundfonts/`; 14b's test-16 contract). Every wait a bounded `poll`.
- **`CLAUDE.md`**: § Music (the six bindings' table, the chicken that never dies and what that
  means for sources, the order, the pulse, the localized pause labels and #202, no fallback,
  telemetry, the bank, the versions); the layout and counts (87 → 87 + the music tests; the stale 84
  corrected). **Web**: `ASSET_BASE` v5 → **v6** (`/games/chicken-coop/v6/assets`),
  `EDITOR_ASSET_BASE` v4 → **v5** (`/playground/frogger/v5/assets`), the content file's two lines,
  `public/games/chicken-coop/v6/` and `public/playground/frogger/v5/`.

### Correction 7 — the web proof, per game, by batch 14's method

Batch 14's correction 7 is the method and is not restated: each bundle built **once with `--sync`**
into `public/`, the content lines edited, `npm run build` once, every check on that `dist/` under one
`npm run preview`, the preview stopped by **the pid in `.astro/preview.json`** on every exit, `npm
run verify`; step 0 shows `git status --porcelain -- src/web_entry.rs` in the game and `-- src/content/games/
<slug>.md` in the site **empty** (the game's ` M Cargo.lock` is the engine's drift, expected and
staged — correction 1; round 65, kimi F2) and **both destination directories absent** (`public/games/<slug>/v6/`, `public/playground/<crate>/v5/`); a failure
reverses the executor's own edits in place — never `git checkout` — and removes only the directories
this attempt synced; the web unit owns its checkouts while it runs; the profile prompt seeded
`dismissed`. **Step 0 fetches the source bank as 14b did**: `apt download fluid-soundfont-gm=3.1-5.3`,
extracted in the executor's scratchpad, its sha256 `74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0`
against the engine script's pin, the wrapper run with `--bank <that path>` (round 67, kimi). Per
game:

- **the bank ≤ 4 MiB** and **one throttled Chromium cold boot** to the game's `init complete` line
  **under 25 s** (Bratdog 17.23 s, Toast Busters 19.62 s, the Food Pyramid 19.22 s); over 25 s,
  two more runs and the median;
- **one two-minute stress run, with a floor at the starting cadence** (14a's lesson), **its floors
  read from the run's own page load** — a fresh navigation whose counts are zero at its boot, the
  run's last line against the floor (round 65, kimi F1); **the driver walks the title first,
  deterministically** — 14b's driver's walk: the title's `1 Player` row, then the chaos select's last
  row (`ChaosMode::ALL` in order, Insiculous fourth), confirm each, the rows read from each game's
  `menu.rs` — and only then starts its blind loop (round 66, gemini F2): **Meatieroids
  on Insiculous solo** (five rocks a wave, a large splitting into three, rocks 1.6× faster) driven
  blind — **turn held, fire pressed every 0.25 s** (`FIRE_COOLDOWN` 0.22 s and four live bullets
  cap the rate at about four a second), **thrust tapped for a quarter second every four seconds**
  (an edge each, a ship that drifts dies oftener), the confirm key doubling as fire in play and as
  the restart on the game-over panel (`flow.rs:21`); in two minutes a spinning ship firing 480
  times at a field of twenty-odd rocks breaks a good share — the last telemetry line shows at least
  **150 `shots=`, 30 `broken=` (the three sizes summed) and 10 `thrusts=`**; **Chicken Coop on
  Insiculous solo** (1.5× lanes, the 25 s timer, sinking crackers, the bun) driven blind — **Up
  pressed every 0.4 s with a Left or Right every third press**, the confirm key every second
  (inert in play — `Action1` is read in `GameOver` only, `flow.rs:17-27`; in play the hop keys are
  the four directions) — a blind chicken into 1.5× traffic dies one to three hops in; three lives
  are about six seconds of hops, three one-second respawns and a game-over wait under a second, so
  a match is about ten seconds and a dozen fit in two minutes: at least **80 `hops=` and 15
  `deaths=`** (round 66, gemini F1 — the road's cadence, not the soup's), `homes=` reported with no floor (a blind chicken fills a nest by luck); batch
  12's criterion verbatim (no two consecutive windows with a drain gap over 92 ms, the render cost
  under a block, **`blocks=` advancing in every window**, zero lines a FAIL), a fail removing the
  synced directories;
- **the functional checks**, each a counter, a state or a still: no `Playing song:` line before the
  first gesture and both `Playing song:` and `Web audio enabled after user gesture` on it; a rally
  in which the game's counts rise with **`triggers_accepted=`** while **`refused=`,
  `triggers_silent=`, `triggers_rejected=`, `triggers_late=` and `triggers_stale=`** hold at zero,
  **the keys as the line prints them, a line missing any a failed check** (Meatieroids'
  `AtSongPosition` drum included); `jam_units=` rising on a move in the pause menu and not on its
  open; `pulsed=` rising while a match runs and holding under the pause menu, with the two stills;
  **one loop wrap recorded** — under the pause menu, the song playing on, the driver waits past the
  loop's length (Mars 60 s, the galop 38 s) and `wraps=` rises by one in the next line — the wait is the driver's own watch of the
  console lines, no CDP command outstanding, as the two-minute stress run already waits (round
  67, gemini F5): a stress run's restarts seek the song to its top and never cross the seam (14b's line read `wraps=0`;
  round 66, codex F2), and Jesse's listen check names the seam, a match left to run past it;
  the editor bundle at the game's playground route showing no `Playing song:` while editing, the
  line on Play's first frame, a `reconstructions` step at the first match start, no music line
  after Stop — **and the one restart line a Stop click over a game-over panel may log before the
  restore is #224, named in the report, not chased**; Firefox the functional checks unthrottled, the
  editor bundle included; `npm run verify`. Two stills per game (`meatieroids-pulse-{peak,rest}.png`,
  `chicken-coop-pulse-{peak,rest}.png`) to `review/conductor/` and
  `coordination/conductor/screenshots/batch-15/`. **Chicken Coop's canvas is 720 × 768**: the CDP
  viewport is set to it, and the tracker's cell size and origin are measured once from a still of
  that canvas, not 14b's.

### Correction 8 — the docs, and one sentence on each game's page

Each game's `CLAUDE.md` § Music (corrections 4 and 6). `training.md` § Conductor stays; nothing in
the engine changes. Each game's page gains one sentence after its chaos-modes paragraph
(`insiculous_web/src/content/games/{meatieroids,chicken-coop}.md`): Meatieroids' — "The Flying Funk
brawls to Holst's *Mars*, played live by the engine's conductor: every shot drums the ostinato,
every meatieroid you break walks the brass theme on its size's own horn, and a lost ship lands the
final chord." — and Chicken Coop's — "The chickens hop to Rossini's *William Tell* finale, played
live by the engine's conductor: every hop is a note of the galop, every nest filled sounds the
fanfare, and the second chicken answers on the piccolo." Codex judges the two sentences with the
stills. Chicken Coop's page is English prose; the game's pirate locale is untouched.

### Correction 9 — sizes, placements, gates, what stays out

Asteroids' `gameplay_tests.rs` is 760 lines before this batch (asteroids#4;
the batch adds nothing there); frogger's largest are `spawning.rs` 554 and `types.rs` 495 — the
`music` field is three lines in `types.rs`, `spawning.rs` is untouched. New files per game:
`src/{music.rs, music_tests.rs}`, `scripts/prepare_soundfont.sh`, `assets/ai/music/{ai_<slug>.mid,
.mid.ron, LICENSE}`, `assets/soundfonts/{<stem>.sf2, LICENSE}`, `review/conductor/`. Touched:
asteroids `Cargo.toml` (`log` native, `music_midi` dev-dependency), `src/{lib,types,test_support,
web_entry}.rs`, `src/gameplay/{mod,ship,rocks,flow}.rs` (the moments, `ripple_grid` `pub(crate)`,
the pulse), `CLAUDE.md`; frogger `Cargo.toml` (`music_midi`), `src/{lib,types,test_support,
web_entry,flow_tests}.rs`, `src/gameplay/{mod,flow}.rs`, `CLAUDE.md`; site the two content files
and four version directories; the engine `coordination/conductor/screenshots/batch-15/` only.
Each game's `Cargo.lock` takes the engine's drift; stage it and say so.

**Gates**, per handoff: in the game `cargo test`, `cargo clippy --all-targets` with and without
`--features editor`, `../../insiculous_2d/scripts/check_audio_provenance.sh "$PWD"` (broken once by
renaming `Files:`, restored, both runs quoted), the wrapper run with `--bank` and the bank's size
against the cap, the song under 256 KiB, the two `build_wasm.sh` runs with their `wasm size:`
lines, correction 7's checks; in the engine `scripts/check_games.sh --test`; in the site `npm run
verify`. The comment-tag grep and the 600-line check over the game's `src` (asteroids'
`gameplay_tests.rs` reported as pre-existing, not a fail). Frogger's locale parity test runs under
`cargo test` as it does today.

**Leaves out:** hyperspace (there is none), the cake saucer (asteroids#2), strings and horns (over
the cap), a wave-clear binding, a cart or raft binding, a bun or cracker binding, the +10 a row, the
pause hint (#202), a recorded fallback (e), `Sequence`, a song that speeds up with the round
(frogger's lanes ramp 15 % a round — intensity layers are the milestone's exclusion), Chicken
Reel (the casting table's alternative — the finale is the stronger hop tune and the bank is set),
the game template (game-template#3), Jesse's promotion of either song to `music/`, the split of
asteroids' `gameplay_tests.rs`.

**Rulings, settled 2026-10-26 (Jesse: everything as proposed; the alternatives were decided at settling and neither was taken, so the handoffs carry the shape above):**
**(ab)** Meatieroids' bank is FluidR3's timpani, tuba, trombone, trumpet and orchestra hit, about
2.4 MiB; Chicken Coop's trumpet, piccolo, trombone, tuba and timpani, about 3.3 MiB — no strings
or horns fit under the cap in either; **(ac)** the beat visual is the grid's ripple under each live
ship and under each active chicken (both games have a backdrop), codex judging from stills;
**(ad)** Meatieroids' nine bindings as correction 3 lists them — `life` offered as a cut (alternative:
seven bindings plus `menu`, the track, part, moment, count, key, test and table row removed as one
list); **(ae)** no hyperspace binding — the move does not exist; **(af)** Chicken Coop's six
bindings as correction 5 lists them, one `death` for every cause (alternative: a second binding
`drown` on the same `squawk` unit with a tuba voice for the soup deaths — offered, not
recommended: two voices for one bad hop); **(ag)** each game's page gains the one sentence of
correction 8; **(ah)** Mars loops at sixty seconds (alternative: twenty-four bars, forty-five
seconds — a shorter crescendo under a long match).

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
