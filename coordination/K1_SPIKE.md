# K1 — Conductor spike findings

**Date:** 2026-10-20 · **Spike code:** `coordination/conductor/spike/` (a standalone cargo
project outside the workspace, committed as the reproducible measurement) · **Plan:**
`coordination/conductor/plan.md` § Batch 1 · **Machine:** Iroh — AMD Ryzen 9 9900X
(12 cores), Linux, rustc 1.94.0.

## Verdict

**The architecture holds on native and on the web.** One rustysynth synthesizer per
instrument slot, one MIDI channel ("lane") per source inside it, and a ledger that
releases its own oldest note at a 24-note cap keep instruments and sources isolated. They
keep rustysynth's hard-cut stealer off every sounding note, and the decision-2 worst case
renders at 7% of real time natively and about 7.5% in both browsers, with no growing
lag over two minutes.

**Listen check: PASSED** (Jesse, 2026-10-20, native and in the browser, every step of
the report's script: the storm, the phrase on two instruments alone and together, held
strings under piano taps, the program change, the retrigger, the pitched sample, the
gated release, the seek).

Five findings change the plan's numbers or wording, all small:

1. **15 lanes per slot, not 16.** rustysynth hard-wires channel index 9 as General MIDI
   percussion and adds 128 to any bank selected there, so a melodic preset cannot live on
   it. The spike's lanes skip it (`instrument_slot.rs`, `PERCUSSION_CHANNEL`).
2. **rustysynth's pool tops out at 256 voices** (`MaximumPolyphonyOutOfRange` above it).
   Decision 2's 128 is inside the range.
3. **On the web, the conductor is pulled in 2048-frame bursts, not 64-frame blocks.**
   cpal's WebAudio backend keeps two 2048-frame buffers in flight and refills one from an
   `onended` callback on the main thread. All 32 blocks of a buffer render back to back,
   so a command waits up to one buffer period (measured: up to 53–61 ms) before it is
   drained. Native drains per device period. This is what decision 3's producer timestamps
   are for: K3 has to place a trigger inside the burst from its timestamp, or every
   web trigger is quantized to about 46 ms.
4. **The 2 MB SoundFont target is waived** (Jesse, 2026-10-20). The cast subset of
   FluidR3 GM is 32.6 MiB; criterion 5 below has the numbers that replace the target.
5. **The three chosen Mutopia pieces miss two required shapes.** None has a tempo change
   or a time-signature change (all are one tempo, one meter), and none is type 0.
   Verified Public-Domain alternates are named below. The type 0 fixture is derived, as
   the plan allowed.

## Criteria

### 1. In-memory SoundFont, Send, externally fed events — PASS

- `SoundFont::new(&mut Cursor<&[u8]>)` builds from bytes the VFS already holds
  (`bank.rs`); no file system is touched on either target.
- `Synthesizer`, the `Mixer` and the `SpikeSource` are `Send`, asserted at compile time
  (`fn assert_send<T: Send>()` in `tests/criterion_1_seam.rs`).
- `SpikeSource` implements `rodio::Source<Item = f32>` and drains a bounded
  `std::sync::mpsc::SyncSender` (256 commands, `try_send`, never blocks) once per 64-frame
  block. A command sent mid-block lands on the next block boundary (tested bit-exactly),
  a full channel refuses and counts, and `Stop` ends the source. The engine side is
  `AudioManager::play_source(source, Bus)`, added in this batch.
- The sequencer is owned (`score.rs`), not rustysynth's `MidiFileSequencer`, which takes
  the synthesizer and leaves no room for live triggers.

### 2. One phrase, two instruments, isolation and capacity — PASS

- Bach's Invention 2, bars 1–2 (read from the fixture with midly), plays alternately and
  concurrently through two slots (keys 1, 2 and 3 in the spike; tested headless).
- **Instrument isolation:** after a program change and a same-pitch note-off on slot A,
  slot B's output is **bit-identical** to a twin of B that never saw A's events.
- **Source isolation:** two sources on one slot at one pitch hold two lanes; the first
  source's note-off leaves the second sounding, equal to it playing alone to 1e-5 once the
  first tail has rung out.
- **Retrigger:** a same-pitch strike on one lane releases the old note (a tail) and keeps
  the second attack sounding.
- **Capacity, explicit and finite:** 8 instrument slots × 15 lanes, a 24-note sounding
  cap per slot, a 128-voice pool per synthesizer, a sample slot with 16 live voices, 256
  queued commands.
- **No sounding note is ever cut by the synthesizer:**
  - *Sine bank, headless.* Three notes are held while one note is struck and released
    every 64 frames, so the tails overflow an 8-voice pool. Rendering with pool 8 and with
    pool 256 differs (rustysynth's stealer ran), but the difference holds no energy at the
    held notes' frequencies (Goertzel, < 1e-3).
  - *FluidR3 cast, worst case.* Rendering the two-minute storm with pool 128 and with
    pool 256 gives **bit-identical** output. The stealer never ran, because the ledger
    caps sounding voices at 24 × layering 2 = 48, which leaves 80 voices for tails.
  - *The burst.* 200 notes in 1 s on piano and strings, with one key hammered on one lane
    every block (688 retriggers, each leaving a 1.2 s piano tail), overflows both pools.
    By rustysynth's priority rule only tails are taken: largest step 0.0573 at pool 128
    against 0.0549 at pool 256.
- **Proposed thresholds for K3's test:**
  - On the committed sine bank under the overflow pattern above, at the spike's 0.3 mix
    gain, assert **largest sample-to-sample step ≤ 0.02**. Measured: 0.0126 with the pool
    overflowing, 0.0137 without; a cut tail is no larger a step than the music's own.
  - On the cast bank, the worst-case storm's largest step is 0.204 at 44.1 kHz and 0.153
    at 48 kHz; **≤ 0.25** if a test ever uses it.
  - The stronger assertion is structural: pool-128 output equals pool-256 output under
    the ledger's cap.
- **Preset profiles read from the bank** (layering = most voices one note-on starts;
  release = longest region release):

  | preset | layering | longest release |
  |---|---|---|
  | 0:0 Yamaha Grand Piano | 2 | 1.20 s |
  | 0:6 Harpsichord | 1 | 0.54 s |
  | 0:48 Strings | 2 | 2.40 s |
  | 0:56 Trumpet | 1 | 0.30 s |
  | 0:57 Trombone | 1 | 0.30 s |
  | 0:13 Xylophone | 2 | 12.00 s |
  | 0:9 Glockenspiel | 1 | 8.40 s |
  | 0:47 Timpani | 2 | 12.00 s |
  | 0:45 Pizzicato Section | 2 | 18.00 s |
  | 0:68 Oboe | 2 | 0.48 s |
  | 0:58 Tuba | 1 | 0.48 s |
  | 128:0 Standard kit | 2 | 50.01 s |

  The maximum layering is 2, so `polyphony ≥ 24 × 2 + headroom` holds at 128 with 80
  voices of tail headroom. The release maxima are long for the percussion and plucked
  presets; they matter to K2's load-time check of a background score, not to the ledger.

### 3. A user WAV, pitched and mixed — PASS

- `decode_sample` decodes with rodio's `Decoder` into `OwnedPcm { sample_rate, channels,
  frames: Arc<[f32]> }`, outside rendering. The proposed limits bind before allocation:
  bytes are checked before parsing, the decode stops one sample past 10 s, and 8–96 kHz,
  mono/stereo and finite samples are all enforced (tested).
- Pitched by linear interpolation at `2^(semitones/12) · source_rate / output_rate`,
  clamped to ±24 semitones. Measured one-shot lengths (48 kHz source, root 60, rendered
  at 44.1 kHz):

  | key | length | ratio to the root | expected |
  |---|---|---|---|
  | 60 | 1.001 s | 1.000 | 1.000 |
  | 67 | 0.668 s | 0.667 | 0.667 |
  | 48 | 2.001 s | 1.999 | 2.000 |

  **Duration changes with pitch**, and there is no time stretching. There is no
  interpolation filter either, so pitching up aliases. The composer's root-note field
  must say both. An octave up plays at 523 Hz (tested by zero crossings).
- One-shot ignores note-off and plays to its end; gated fades over 5 ms after note-off
  (tested to the frame). At the 16-voice cap the oldest voice fades out over 5 ms; peak
  was 17 voices during a fade.
- Mixed with SoundFont output: the mix equals the sum of its parts to 1e-5 (tested).

### 4. Render cost, the browser, frame time — PASS

**Native, offline** (`cargo run --release -- --measure --seconds 120`, the decision-2
worst case: the background score looping on its own synthesizer, 8 slots held at the
24-note cap, the sample slot, 16 triggers a second of 4-note chords held 3 s, one key
hammered, one seek, five loop wraps):

| output | block mean | p99 | longest | load | peaks |
|---|---|---|---|---|---|
| 44.1 kHz, 64 frames (1451 µs) | 101.9 µs | 112.4 µs | 279.3 µs | 7.0% (14× real time) | 192 sounding notes, 7 sample voices |
| 48 kHz, 64 frames (1333 µs) | 101.6 µs | 111.8 µs | 200.8 µs | 7.6% (13× real time) | the same |
| sample slot only, 44.1 kHz | 9.8 µs | 14.2 µs | 107.8 µs | 0.7% | 17 sample voices |

Ledger counters over the two minutes: 6832 steals, 2015 retriggers, 0 lane rejections.
Seek and loop-wrap state reconstruction took at most 4.6 µs natively.

**Browser** (`scripts/build_wasm.sh coordination/conductor/spike conductor-spike --version
v1`, served by `python3 -m http.server` from `dist/`; 25 s before the gesture, one click
on the canvas, then 125 s of the same worst case with an automatic seek at 60 s; both
browsers headless with audio muted, driven by Playwright and geckodriver):

| | Chromium 151.0.7922.34 | Firefox 157.0.1 |
|---|---|---|
| gesture-gated start | audio disabled until the click; "Web audio enabled after user gesture" then the source starts | the same |
| frame mean, off → on | 16.67 → 16.67 ms (vsync) | 16.67 → 16.67 ms |
| frame p99, off → on | 16.8 → 18.6–20.0 ms | 18.0 → 20.0 ms |
| frame longest, on | ≤ 20.9 ms | ≤ 22.0 ms |
| block mean (64 frames) | 104 µs | 109 µs |
| longest drain gap | 60.7 ms | 53.0 ms |
| lag (wall clock − rendered) | −37 … +0 ms, not growing | −33 … +15 ms, not growing |
| non-finite samples, rejected commands | 0, 0 | 0, 0 |
| seeks and loop wraps | 5 | 5 |
| reconstruction | under the 100 µs timer | under the 1 ms timer |

**Underruns.** The lag between the wall clock and the samples rendered stayed within
±40 ms for the whole capture in both browsers and never grew, so the source kept pace
and no sustained underrun occurred. A gap long enough to matter would show as a lag step
larger than one 46 ms buffer; none did at the 10 s logging grain. Shorter clicks are
what the listen check is for.

The p99 frame time rises by about 3 ms because the web backend renders on the main
thread: 32 blocks × ~105 µs ≈ 3.4 ms per 46 ms burst. Browsers coarsen
`performance.now()` (100 µs in Chromium, 1 ms in Firefox), so block "longest" figures
there are timer steps, not costs.

**Scheduled versus rendered offset** (the beat-visual latency the deferred issue
carries):
- Natively, a command lands on the next 64-frame block: 1.45 ms of quantization, plus
  the device period, which the listen run's log prints as `drain_gap_longest`.
- On the web, a command waits for the next 2048-frame refill (up to 53–61 ms measured),
  and that buffer plays up to two buffers after it is rendered (cpal schedules the first
  25 ms ahead). Expect about 46–100 ms from send to sound unless K3 schedules inside the
  burst by timestamp.

**Load.**

| | time |
|---|---|
| bank read (native) | 5.9 ms |
| bank parse (native) | 6.8 ms |
| all four assets loaded and decoded (wasm, after preload) | 15.0–17.5 ms |
| preload of the whole bundle over localhost | 44–51 ms |

None of it is over a frame once the bytes are in memory, so batch 6 needs no prewarm
for decode. Downloading the bank, though, is a boot-time cost (see criterion 5).

**Memory.** wasm linear memory went from 56.6 MiB to 122.1 MiB at load (+65.5 MiB, about
2× the bank): the VFS keeps the fetched bytes, and rustysynth copies the samples into
its own `Vec<i16>`. Natively the read buffer is dropped after parsing: resident memory
went from 4.9 MiB to 40.9 MiB (+36 MiB, about 1.1× the bank).

**Sizes.** The spike's `game_bg.wasm` is 6.00 MiB (the engine, rustysynth, midly and the
spike together). Its assets are 34 MB: the bank and four small fixtures.

### 5. The SoundFont and the fixtures — DONE DIFFERENTLY (the 2 MB target waived by Jesse, 2026-10-20)

**What was asked:** a subset at or under 2 MB, MuseScore General if FluidR3 was over, and
a stop for Jesse if both were. Both were. The cast subset of FluidR3 GM is 32.6 MiB, and
MuseScore General's piano alone is 98 MiB. Jesse waived the target on 2026-10-20 ("build
it like it's been approved"), and the planner's note asked for the numbers that replace
it. They follow.

**Source bank:** FluidR3 GM, MIT (Frank Wen), from Debian's `fluid-soundfont-gm`
3.1-5.3 (`apt download`, `dpkg -x`).
- `FluidR3_GM.sf2` sha256 `74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0`
  (148,398,306 bytes).
- `.deb` sha256 `eadee8a07697a15789c16c542b355cdf5bc01f97f7c19d1ac8ae237c7d73fce4`.

MuseScore General 0.2 (MIT) was measured as the escalation:
`MuseScore_General.sf2` sha256
`ee51d2c4b1525e70f19a45909c4fd7a2e26d91d115fa89dbf5a6bc413d8b9bf3`, its Grand Piano
98.0 MiB.

**The subset:** `scripts/subset_soundfont.py` (stdlib; finished, so no Polyphone) with
the presets `0:0 0:6 0:48 0:56 0:57 0:13 0:9 0:47 0:45 0:68 0:58 128:0`.
- **34,226,764 bytes (32.64 MiB)**, sha256
  `dc691913ab31355c2716d5b3cce1c9424903bf2dc9fae6274a49317820ae4903`.
- It reproduces byte for byte: `coordination/conductor/spike/prepare_assets.sh` downloads
  the bank, checks its sha256 and writes the subset with its `LICENSE` (MIT notice).
- It is **not** under any game's `assets/`: the plan reports it in K1 and ships it in K2.

| preset | sample data |
|---|---|
| 128:0 Standard kit | 10.62 MiB |
| 0:0 Yamaha Grand Piano | 7.47 MiB |
| 0:48 Strings | 5.34 MiB |
| 0:45 Pizzicato Section | 2.66 MiB |
| 0:13 Xylophone | 1.73 MiB |
| 0:6 Harpsichord | 1.33 MiB |
| 0:47 Timpani | 1.33 MiB |
| 0:68 Oboe | 0.87 MiB |
| 0:9 Glockenspiel | 0.55 MiB |
| 0:57 Trombone | 0.43 MiB |
| 0:56 Trumpet | 0.22 MiB |
| 0:58 Tuba | 0.19 MiB |

No two presets share a sample. Kit, piano and strings are 72% of the file; without the
kit the cast is 22.03 MiB.

**What it costs.**
- *Decode at load:* 6.8 ms natively, 15–17.5 ms in wasm for all assets.
- *Download:* 27.4 s at 10 Mbit/s, 5.5 s at 50 Mbit/s, 1.4 s at 200 Mbit/s.
- *Boot:* the bundle's boot step prefetches every file before the game starts, so the
  download is a boot delay today.
- *Memory:* about 2× its size once loaded (+65.5 MiB in wasm).

**Proposed per-game audio budget, for K2 to freeze:**
- **≤ 24 MiB downloaded:** bank ≤ 22 MiB, songs ≤ 256 KiB, samples ≤ 2 MiB.
- **≤ 50 MiB loaded.**

That is the cast without the full kit. K2 has three levers for it:
- extend the subsetter to keep a kit's zones by key range (the snare, the bass drum, a
  cymbal) instead of the whole 10.6 MiB kit;
- or ship each game only its own song's presets;
- and, separately, decide whether the bank must block boot or can be fetched after the
  first frame, and whether the VFS drops the bytes once parsed (which halves loaded
  memory).

The shared-hosting question stays open for K2.

**Fixtures** (`crates/music_midi/fixtures/`, verbatim, each page's "Copyright: Public
Domain" line in the `LICENSE`):

| file | SMF type | division | tracks (names) | tempo events | time signatures | length |
|---|---|---|---|---|---|---|
| `bach-invention-02.mid` (id 58) | 1 | 384 ticks/quarter | 3 (control track, one, two) | 1: 64 bpm at 0 | 4/4 at 0 | 27 bars |
| `dansenapolitaine.mid` (id 896) | 1 | 384 ticks/quarter | 5 (control track, flute, tambourin, uppera, lowera) | 1: 72 bpm at 0 | 2/4 at 0 | 53.5 bars |
| `bayadere-ombres.mid` (id 895) | 1 | 384 ticks/quarter | 5 (control track, flute, triangle, uppera, lowera) | 1: 72 bpm at 0 | 2/4 at 0 | 35 bars |
| `dansenapolitaine-type0.mid` (derived) | 0 | 384 ticks/quarter | 1 | 1: 72 bpm at 0 | 2/4 at 0 | 53.5 bars |

- None is SMPTE, which K2 rejects.
- None is type 0. `scripts/make_type0_fixture.py` merges the Napolitan Dance and moves
  its tambourine to channel 10 as key 54. LilyPond writes an unpitched staff as middle C
  on a melodic channel, which would sound as piano, so without the move the file would
  hold no percussion channel.
- None has a tempo change or a time-signature change.

**Alternates, each verified Public Domain on its own page and parsed:**
- *Several tracks with a tempo change, and a time-signature change in the same file:*
  Beethoven, *Overture to Egmont*, id 547. 15 tracks; 84 → 152 → 168 bpm; 3/2 → 3/4 → 4/4.
  https://www.mutopiaproject.org/ftp/BeethovenLv/O84/Egmont/Egmont.mid
- *A small time-signature change:* Grieg, *Wedding-day at Troldhaugen*, id 781. 4/4 ↔
  2/4; 90 → 120 bpm.
  https://www.mutopiaproject.org/ftp/GriegE/O65/troldhaugen/troldhaugen.mid
- *A tempo change alone:* Grieg, *In the Hall of the Mountain King*, id 1888. 138 → 154 bpm.
  https://www.mutopiaproject.org/ftp/GriegE/O46/Dans_l_antre_du_roi_de_la_montagne/Dans_l_antre_du_roi_de_la_montagne.mid
- *Type 0:* every Mutopia file is LilyPond type 1, so the derived file stands. The
  generator applies to any of the above.

**Generated fixtures:** `sine.sf2` (`scripts/make_sine_soundfont.py`) and
`sine_{48000,22050}_{mono,stereo}.wav` (`scripts/make_sine_wav.py`). Both scripts are
stdlib-only, and their output is identical on a second run (`cmp`).

### 6. Stop on failure — not triggered

No architecture criterion failed. Criterion 5's size target was waived rather than
failed, by Jesse's call; the plan correction can point at this section.

## Versions batch 2 must pin

From the spike's committed `Cargo.lock`: **rodio 0.20.1** (equal to the workspace's),
**rustysynth 1.3.8**, **midly 0.5.3**.

## How to reproduce

```sh
cd coordination/conductor/spike
./prepare_assets.sh                        # downloads FluidR3 GM, checks it, writes assets/
cargo test && cargo clippy --all-targets -- -D warnings
cargo run --release -- --measure           # every native number above
cargo run --release                        # the game: the storm plus keyboard triggers
cd ../../.. && scripts/build_wasm.sh coordination/conductor/spike conductor-spike --version v1
cd coordination/conductor/spike/dist && python3 -m http.server 8000
# open http://127.0.0.1:8000/games/conductor-spike/index.html, click, read the console
```

The game logs one line every ten seconds: frame mean, p99 and longest; memory; and, once
the conductor runs, its render cost, drain gap, lag, peaks, steps and seek/wrap count.

## Not done, for the planner to file

- The native drain gap and frame times from a windowed run. Jesse's listen run passed,
  but its log lines were not captured; the next native run's `conductor=on` line has them.
- The wasm code-size cost of rustysynth and midly alone (the 6.00 MiB above is the whole
  spike). K8 measures it against `PLAYGROUND_SIZE_BUDGET_MIB`, since the composer is
  what brings them into the playground bundle.

## Known limits of the spike (code review, 2026-10-20)

The spike is the measurement record, so these are recorded rather than patched; none moved
a measured number, and each maps to a rule the production batches carry.

- **Loop wrap renders one block without the downbeat and drifts 64 frames per wrap**
  (`score.rs` `render`): after `seek(loop_start)` the wrap block applies no events and the
  position advances a whole block. Five wraps in the two-minute capture; the listen check
  heard clean wraps. Production: the sequencer renders the seam as two sub-blocks.
- **`channel_states_at` folds events strictly before the target tick**, so a seek to tick 0
  misses tick-0 tempo and program events for that one block (the next block applies them as
  ordinary events). Production: state events fold `<=` the target, note events `<`.
- **A 16th one-note source is rejected before the steal runs** (`instrument_slot.rs`
  `note_on` leases a lane before checking the cap). The storm never hit it (0 lane
  rejections). Production: with no free lane, the oldest source's notes are released and
  its lane taken, oldest first.
- **An empty WAV underflows `frame_count - 1`; a one-frame sample is mute**
  (`sample_slot.rs`). Production: a sample shorter than 1 ms is rejected before allocation.
- **Fading sample voices have no ceiling**: a burst of N triggers in one block leaves N
  voices rendering for the 5 ms fade. Production: a hard ceiling of the live cap plus a fade
  budget; over it the oldest fading voice is cut.
- **Spike tooling edges:** a `Stop` sent on a full channel is dropped and the game believes
  the conductor is off; a failed `start_source` retries every frame and unlocks the
  achievement anyway; `--measure --seconds 0` panics. Production: decision 3's lifecycle
  reserve and pending transport retry.
