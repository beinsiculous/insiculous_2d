# Song format — the `.mid.ron` sidecar

A song is a Standard MIDI File and a sidecar beside it under the same name plus `.ron`:
`music/napolitan.mid` is described by `music/napolitan.mid.ron`. The MIDI file holds the
notes; the sidecar says which of its tracks game actions play, on which instruments, and
how. Everything else in the MIDI file is the **background score**, played as written.

The schema and its validation live in one place, `crates/music_midi/src/song_file.rs`
(`SongFile`, `parse_song_file`, `SongFile::validate`); this page is the author's guide to
it. Songs a person wrote live under `assets/music/`; a song an agent sequenced lives under
`assets/ai/music/` with the `ai_` prefix until a person promotes it.

## A full example

This sidecar is parsed and validated against the Napolitan Dance fixture
(`crates/music_midi/fixtures/dansenapolitaine.mid`: a control track, then `flute`,
`tambourin`, `uppera` and `lowera`) by the crate's tests, so it cannot drift from the
schema.

```ron
SongFile(
    version: 1,
    soundfont: "soundfonts/conductor_cast.sf2",
    looping: true,
    parts: [
        (name: "melody", track: Name("flute"), mode: Loop, quantize: Eighth),
        (name: "drums", track: Index(2), mode: AtSongPosition, quantize: Sixteenth),
        (name: "bass_line", track: Name("lowera"), mode: Sequence, quantize: Beat),
    ],
    instruments: [
        (name: "trumpet", source: SoundFont(path: "soundfonts/conductor_cast.sf2", bank: 0, preset: 56)),
        (name: "tuba", source: SoundFont(path: "soundfonts/conductor_cast.sf2", bank: 0, preset: 58)),
        (name: "tambourine", source: Sample(path: "music/samples/napolitan/tambourine.wav", root_note: 60, gain: 0.8, playback: OneShot)),
    ],
    bindings: [
        (action: "coin", part: "melody", instrument: "trumpet"),
        (action: "combo", part: "melody", instrument: "tuba"),
        (action: "bump", part: "drums", instrument: "tambourine"),
        (action: "step", part: "bass_line", instrument: "tuba"),
    ],
)
```

`coin` and `combo` play the same part on two instruments: the notes are stored once, and
both bindings walk the same cursor. `tuba` serves two parts. The `uppera` track (the
piano's right hand) is selected by no part, so it is background, with the control track's
tempo and meter.

## The grammar

The struct names (`SongFile(`, `Part(` …) are optional in RON; the example writes only
the outer one. Unknown fields are an error, so a misspelt field fails loud instead of
being ignored.

### `SongFile`

| field | type | default | meaning |
|---|---|---|---|
| `version` | integer | required | must be `1`; any other version is refused before the rest is read |
| `soundfont` | path | required | the SoundFont that plays the background score |
| `looping` | bool | `true` | whether the song loops at its end |
| `parts` | list of `Part` | empty | the musical parts game actions play |
| `instruments` | list of `Instrument` | empty | the instruments parts are played on |
| `bindings` | list of `Binding` | empty | which action plays which part on which instrument |

Paths are relative to the game's asset directory, as every other asset path is (the
audio manager resolves them against `set_asset_base`, which the engine's runner sets from
`GameConfig::asset_base()` — `assets` by default — before a game's `init`; a host that
builds its own `AudioManager` sets it itself, or the paths resolve against the working
directory).

### `Part`

| field | type | default | meaning |
|---|---|---|---|
| `name` | string | required | unique among parts; bindings refer to it |
| `track` | `Name("…")` or `Index(n)` | required | the one MIDI track the part reads |
| `mode` | `Loop`, `Sequence` or `AtSongPosition` | `Loop` | how triggers walk the notes (§ Modes) |
| `quantize` | `Off`, `Beat`, `Eighth` or `Sixteenth` | `Off` | the grid a trigger snaps forward to |

`Name` matches a MIDI track name exactly. When two tracks share a name, `Name` is refused
and the error says to use `Index`, which is never refused for a name. Indexes count from
zero over the song's *selectable tracks*: a type-1 file's own tracks, index for index (the
control track is usually index 0); in a type-0 file, one track per MIDI channel in channel
order, each named `channel N` with N counted from 1 — so General MIDI percussion is
`Name("channel 10")`.

Quantization belongs to the part, not the binding, so the one cursor every binding of a
part shares has one timing policy.

### `Instrument`

| field | type | meaning |
|---|---|---|
| `name` | string | unique among instruments; bindings refer to it |
| `source` | `SoundFont(…)` or `Sample(…)` | where its sound comes from |

`SoundFont(path: "…", bank: b, preset: p)` — a preset of a SoundFont, which need not be
the background's. Banks run 0–127 for melodic presets, 128 for General MIDI percussion;
presets 0–127.

`Sample(path: "…", root_note: n, gain: g, playback: OneShot | Gated)` — a recorded WAV.
`root_note` (0–127) is the key at which it plays unchanged; other keys resample it by
linear interpolation, stepping `2^(semitones / 12) × the WAV's rate / the output rate`
through it, so **duration changes with pitch**: a fifth up lasts two thirds as long, an
octave up half, an octave down twice. There is no time stretch and no filter, so
**pitching up aliases**. Every note a bound part plays must lie within 24 semitones of the
root (a key past that plays at 24). `gain` defaults to `1.0` and must be finite and at
least zero; it scales the sample at velocity 127, and a note's velocity scales it
linearly below that. Every sample voice plays at a fifth of the recording's level before
`gain`, so a full-scale recording at `gain: 1.0` sits level with one SoundFont note; raise
`gain` for a quiet recording, not for a loud one. The output's limiter turns the whole mix
down while many loud voices pass 0.9 of full scale, and lets it back up after.

`playback` defaults to `OneShot`: the note-on starts the sample and it plays to its
(pitch-scaled) end whatever the note's length, sounding all that time. `Gated` plays it
until the note ends, then fades it out over 5 ms. Either way, a voice ended by force — a
steal, its source leaving, a seek, a loop wrap, a pause or a stop — fades over the same
5 ms, a one-shot included, so nothing clicks. A mono WAV plays alike on both channels; a
stereo one keeps its two.

**The sample's limits:** a WAV file (told by its `RIFF`/`WAVE` header, not its name — the
decoder reads more formats than a pitched sample is imported from) of at most 4 MiB,
recorded at 8 to 96 kHz, mono or stereo, 1 ms to 10 s long once decoded. A file past the
byte limit is refused before it is decoded, and a decode stops one frame past 10 s; a
refused sample never replaces the song already loaded. Two instruments naming one path
share one decoded copy.

### `Binding`

| field | type | meaning |
|---|---|---|
| `action` | string | the stable id game code triggers; unique among bindings |
| `part` | string | a part's name |
| `instrument` | string | an instrument's name |

A binding is one instrument. An ensemble is several bindings the game triggers together.
Any number of bindings may share a part or an instrument.

## Trigger tracks and the background

A track selected by any part is a **trigger track**: it plays only when its bindings fire,
never on its own. Every other track is the **background score**, played by the song's own
synthesizer with its program, channel, controller and pitch-bend events as written. A
part's own MIDI program and channel never override its binding's instrument — the
instrument always wins.

A key struck again on its channel while it still sounds (a pedalled repeat) sounds again
and keeps sounding until its last instance ends: one note-off would otherwise silence
both.

## Modes

- **`Loop`** (the default) — each accepted trigger plays the part's next unit (a chord, or
  a single note) and advances one cursor shared by every binding of the part; at the end
  it wraps to the first unit. The song's position never skips the cursor ahead.
- **`Sequence`** — each accepted trigger plays the next unit the song has not already
  passed, skipping units whose time is behind the song; when none is left the part is
  exhausted and triggers are silent until the song loops, which resets the cursor.
- **`AtSongPosition`** — a trigger plays whatever unit is sounding at the song position it
  lands on (after quantization, through every tempo change), with no cursor; a trigger in a
  rest plays nothing. "Sounding" means the unit that started last at or before that
  position: the trigger plays what remains of its notes, each ending where it was written
  to end. When that unit has already ended the position is a rest, even if an earlier,
  longer note still rings over it in the file.

Every mode plays a unit's notes at their own velocities and their own written lengths,
measured through the tempo map from where they play. A note shorter than a 64-frame block
(1.45 ms) sounds for one block.

**Loop and Sequence do not follow the background's harmony.** A `Loop` part plays its
authored notes in order whenever the player acts, so a unit written over one chord can
sound over another — the part's notes are guaranteed to be *its* notes, not to fit the
background at that moment. Write a looping part to suit every chord it may land on (a
pentatonic riff, a percussion line), or use `AtSongPosition`, which always plays the unit
written for that moment. The composer is to show this caveat beside the mode.

### Timing: the stamp and the grid

The game stamps every trigger with the moment the player acted, and the conductor places
the trigger from that stamp, not from when the audio thread got to it — the audio thread
takes commands only once a device period, every 43 ms natively and up to 61 ms in a
browser, so placing by the drain would put every trigger on the drain's beat. The stamp is turned into a song position to within one device period plus one
block; that is how precise "when the player acted" can be.

- **`quantize: Off`** — the notes start on the next 64-frame block, at once.
- **`Beat`, `Eighth`, `Sixteenth`** — the grid is the song's beats (the time signature's
  beat, so a half note in 3/2), or eighths or sixteenths laid from each beat's start. A
  trigger stamped no more than one block after a grid line was on time and plays at once;
  one stamped later waits for the next line. A trigger never plays earlier than it was
  drained.
- **Every start lands on a 64-frame block**, up to 1.45 ms before the exact sample: the
  synthesizer applies a note only at the start of its next block. Two triggers on the same
  grid line start together.
- **A trigger older than one block plus 61 ms when the conductor drains it is dropped**,
  so a tab that was asleep never wakes to a burst of old notes. So is any trigger while the
  song is paused or stopped.

A trigger that does not fit is refused whole and its part's cursor does not move: when the
conductor's 512 scheduled events are full, or when its instrument already waits on the
grid to start as many notes as it can hold sounding, 24. Past 24 notes sounding at once on one instrument, the oldest
is released to make room — it rings out its tail rather than being cut. A loop wrap, a
seek and `Play` release every triggered note and drop every scheduled one; `Loop` and
`Sequence` cursors restart at the first unit at or after the new position.

## What fails, and when

A song that fails validation does not load; the error names the file, the collection and
the entry, as `music/napolitan.mid.ron: parts['drums'] selects track index 9, which the
MIDI file does not have (5 tracks)`.

**From the text alone** (`parse_song_file`): an unknown version; malformed RON or an
unknown field; an empty or repeated name in any collection (actions included); an empty
path; a binding naming a part or an instrument that is not declared; a gain that is not
finite or is negative; a root note, bank or preset out of range; more than 8 instruments.

**Against the MIDI file and the loaded resources** (`SongFile::validate`): a background
SoundFont the loader did not decode; a part whose track does not exist; a part selecting a
shared name; a part whose track has no notes; a sample-bound part with a note more than 24
semitones from the root; a SoundFont preset the SoundFont does not have; a preset that
layers so many voices a note that 24 sounding notes would overflow the synthesizer's
128-voice pool (a layering of 6 or more); a sample the loader did not decode, or one
shorter than 1 ms or longer than 10 s.

The loader — not this crate — reads the files and supplies what it learned as
`ResourceMetadata` (the SoundFonts it decoded, each preset's layering, each sample's
length).

### Capacities

Each instrument takes one of **8 instrument slots**, so a program change on one never
changes another's sound. A SoundFont instrument's slot is its own synthesizer and keeps
at most **24 notes sounding**, releasing its oldest past that. A sample instrument's slot
holds at most **16 live voices**, fading the oldest out past that, and at most 16 more
fading; past those 32 the oldest fading voice is cut outright, so a burst of triggers
cannot pile voices up. The background score has a synthesizer of
its own with all 16 MIDI channels, which any MIDI file fits, so it takes no slot.

## The MIDI file

- **Types 0 and 1** are read. Type 2 (independent sequences) is refused.
- **Division** must be ticks per quarter note; an SMPTE (frames-a-second) division is
  refused. Re-export from the notation program if needed.
- **Tempo and meter** come from the file's tempo and time-signature events, on any track.
  Before the first of each — or in a file with none — the song plays at **120 BPM in
  4/4**. A tempo of zero, or a time signature that cannot divide the quarter note into
  whole beats, is refused.
- **Limits**, checked before anything is built in full: 1 MiB of MIDI, 262 144 events.
- **Overlapping notes.** A key struck again on its channel before its note-off is paired
  first-in first-out and reported to the composer as an overlap; nothing is refused.
- **Editing.** A type-1 track whose notes play on more than one channel loads and plays,
  but the composer refuses to edit the file and says why: a new note would have no single
  channel to be written on.
- **Saving.** The composer writes the file back through midly. An unedited file comes back
  event for event identical — byte for byte when the original never repeats a status
  byte, shorter when it does (midly always writes running status).

**What an export changes.** An unedited song writes back the same events in the same order
(byte-identical when the original never repeats a status byte; midly's writer always uses
running status). Two normalizations are deliberate: every exported track closes with one
EndOfTrack at its last event or at the original end — a tail of deleted notes leaves the
song's authored length alone, and a track that had no end gains one — and once a tempo or
time signature is edited, every timing event is written on the first track.

## Positions in the music

Bars, beats and beats-in-bar count from zero. A **beat is the time signature's
denominator note** — a half note in 3/2, a quarter in 3/4, an eighth in 6/8. A time
signature that arrives mid-bar closes the partial bar and starts a new one. An event at a
tick sounds on the first audio sample at or after its exact time; beat crossings are
counted on those samples, never by comparing floating-point times.

**The song's end** is its last event, end-of-track included — its authored length, not
its last note. A looping song wraps there on the exact sample: every note still sounding
is released, every channel's program, bank, controllers and pitch bend return to what
they are at the start (tick 0's own events included), and the first beat plays again, so
the loop never drifts by a fraction of an audio block. A seek rebuilds the channels the
same way, and a note that began before the seek's destination is not played again.

**What a game reads.** `ctx.audio.song_position()` gives the bar, the beat, the beat in
the bar and the seconds of the **next block the conductor renders** — samples rendered,
not yet at the speaker. It is where a scheduled trigger fires, so it runs up to 63 frames
ahead of the frame playing, and `latency_samples` says how far beyond that the engine
knows the sound to be: the frames of its current block the device has not yet taken. The
device's own buffer — a device period natively, up to a 2048-frame pull in a browser —
lies further still and is not reported, so a visual drawn on the position leads the sound
by at least `latency_samples`. `ctx.audio.beat_crossed(Subdivision::Beat)` (or `Bar`,
`Eighth`, `Sixteenth`, laid from each beat's start like a part's quantize) is true for one
update when the song crossed that boundary since the update before — once, however many
boundaries a long frame held. A loop wrap reports every subdivision, for the downbeat it
lands on; a seek backwards reports none.
