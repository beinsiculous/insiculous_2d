# Finite sample banks

`python3 scripts/pack_soundfont.py recipe.json output.sf2` packages mono PCM16
WAVs without looping or resynthesis. WAV sample rates (8–96 kHz) are preserved.
The complete bank must fit 4 MiB. Input errors and budget failures leave the old
output intact; the final file is replaced atomically. Only Python's standard
library is needed. Run `python3 scripts/test_pack_soundfont.py` for refusal tests.

The version-1 JSON recipe has `name`, `copyright`, and ordered `presets`. Each
preset has a 1–19 ASCII-character `name`, `bank` (0 melodic, 128 percussion),
`program` (0–127), and `zones`. Each zone specifies:

```json
{"wav":"rendered/lead_74.wav","keys":[74,74],"root":74,
 "release_seconds":0.04,"longest_note_seconds":0.24}
```

Paths are relative to the recipe. Key zones cannot overlap within a preset.
Sample length must cover the longest note plus release, adjusted for the
highest playback pitch in that zone. Samples carry 46 zero guard points and
non-looping sample mode. At least 48 frames are required; unused loop markers
are eight frames inside each end for portable headers. Note durations must be finite
and positive. No timestamps or platform byte ordering enter output.
Use deterministic input WAVs and the same recipe ordering for identical bytes.
The author must supply honest source provenance and the corresponding asset LICENSE.

Decode and verify coverage through the real engine with:

```sh
cargo run -p music_midi --example audition -- ASSETS ai/music/SONG.mid EVENTS.tsv 121 OUTPUT.wav
```

Events are `seconds action source_id`, one per line, with `#` comment lines.
The preview requires a positive duration of at most 600 seconds, with events no later
than the start of its last block. Events drain at the next block boundary, matching
the producer/consumer runtime contract; queue overflow fails rather than retiming.
The preview supports SoundFont-only songs, checks every background and cue key
against exact decoded preset/instrument regions (no fallback timbres), then renders through Conductor::render_at.
It includes the production audio limiter source directly, with unity music/SFX/master
bus levels and production block size. It reports peak, RMS, nonzero frames, clipped
samples, trigger counters, peak background voices and maximum layers per note.
These are synthetic event mixes, not recorded game sessions.

`cargo run -p editor --example remap_soundfont -- ASSETS ai/music/SONG.mid BANK AUTHOR LICENSE`
changes bank paths while retaining programs, parts and bindings, and saves through
SongDocument, after decoding the candidate song and bank successfully. This tool
supports SoundFont-only maps and reports open warnings. The MIDI must already be exported and its previous sidecar retained.

Remapping also loads the candidate into a native conductor, checking every declared
instrument and its worst-case layering, including unused instruments. Exact background
presets/keys are required; fallback timbres are rejected deliberately. Export and remap
serially, with no concurrent external edits. If the editor save adopts an external
score, its notes are reported and the adopted score is validated before success is
reported; that detection does not roll back another application's MIDI export.

Any declared Sample instrument, bound or unused, makes a map ineligible for this
SoundFont-only tool; edit mixed maps with the Conductor panel instead.
