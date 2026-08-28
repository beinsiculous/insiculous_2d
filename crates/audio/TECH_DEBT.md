# Technical Debt: audio — LIVE (open items only)

Last audited: June 11, 2026 (July 2026: Game Programming Patterns audit).
Resolved history: root `log_archive.md` § audio.

## Game Programming Patterns Audit (July 2026 — closed; history in `log_archive.md`)
No open items (GPP-L3 resolved Jul 13 2026 — see `log_archive.md`).

## Open Items

None.

## Known Limitations (By Design — current constraints, not open work)

1. **No streaming for large files** — all sounds loaded into memory for instant playback
2. **No spatial audio in this crate** — `AudioSource`/`AudioListener`/`PlaySoundEffect` live in `crates/ecs/src/audio_components.rs`, editor-inspectable data only
3. **No audio effects** — no reverb, echo, or other DSP
4. **Single music track** at a time
5. **Disabled mode reports `is_music_playing() == false`** even after a successful `play_music` call — documented choice. The request is remembered (`pending_music`, last one wins) and starts when `enable_output()` connects a device — the web H7 path
6. **`IoError` carries no file path** — `#[from] io::Error` loses path context (decode errors include the path)
7. **No AudioContext resume after tab-switch suspension (web)** — a browser may suspend the context in a background tab; rodio/cpal expose no resume hook, so audio can stay silent until reload. Accepted #8 scope cut (adversarial review F5)

## Future Enhancements (Not Technical Debt)

1. Streaming for large music files
2. Runtime audio system bridging the ECS audio components (spatial playback)
3. Crossfade for music transitions
4. Audio effects processing
5. Generic bus/group API (currently fixed master/sfx/music buses)
6. Audio occlusion

## Metrics

| Metric | Value |
|--------|-------|
| Test count | 27 (26 unit + 1 doc, all headless) |
| High priority open | 0 |
| Medium priority open | 0 |
| Low priority open | 0 |
