# Music MIDI Crate — Agent Context

The conductor's song core: the `.mid.ron` song sidecar, a Standard MIDI File as owned and
editable data, and the tempo map. `docs/SONG_FORMAT.md` is the author's guide to the
sidecar; this file is the code's.

## The contract
- **No I/O, no rodio.** Bytes in, data out. The audio crate reads files, decodes
  SoundFonts and WAVs, and hands this crate the MIDI bytes, the sidecar text and a
  `ResourceMetadata` describing what it loaded. Fixtures reach tests by `include_bytes!`.
- **Exactly two dependencies of its own**, `midly` and `rustysynth`, pinned in the root
  `[workspace.dependencies]`; any third is a plan change. `midly` is re-exported
  (`music_midi::midly`) so no other crate names it. `serde`, `ron` and `thiserror` are
  the workspace's.
- **The `synth` feature** (default on) owns rustysynth for the synthesizer side. The
  schema and the data model build and test with `--no-default-features`, which is how
  the editor depends on the crate.
- **Authored data fails loud**, naming the file, the collection and the entry
  (`parts['drums']`), in the manner of `engine_core`'s `sheet_file.rs`.

## File Map
- `lib.rs` — crate docs, re-exports, and the frozen capacities: 8 instrument slots, 24
  sounding notes a slot, a 128-voice synthesizer pool, ±24 semitones for a sample, 1 ms
  to 10 s of sample, 1 MiB and 262 144 events of MIDI.
- `error.rs` — `SongError` (reading and writing MIDI), `EditError` (a refused mutation),
  `SongFileError` = the file's label + a `SongFileProblem`, and `Collection`.
- `song_file.rs` — **the sidecar schema**: `SongFile` v1, `Part`, `TrackSelector`,
  `PartMode`, `Quantize`, `Instrument`, `InstrumentSource`, `SamplePlayback`, `Binding`,
  `ResourceMetadata`/`PresetKey`; `parse_song_file` (version probed first, unknown
  fields refused); `SongFile::validate`; `Part::resolve_track`.
- `song_file/validate.rs` — every sidecar check: the schema checks `parse_song_file`
  runs and the checks against `SongData` and `ResourceMetadata`.
- `song_data.rs` — the data model: `SongData`, `SongTrack` (a selectable track),
  `MusicalUnit`, `Note`, `NoteId`, `NoteImage`, `RetainedEvent`, `UnsupportedReport`.
- `song_data/flatten.rs` — `SongData::from_smf`: limits, type and division checks,
  owned events at absolute ticks, first-in first-out note pairing, type-0 channel
  normalization, the report.
- `song_data/edit.rs` — the mutation helpers and `restore_notes`; each validates every
  note before changing any and returns before-images.
- `song_data/export.rs` — `SongData::write_smf`: retained events minus edited notes'
  original pairs, plus live notes and edited timing, through midly's writer.
- `tempo_map.rs` — `TempoMap` (exact integer tick ↔ seconds ↔ sample ↔ bar),
  `TempoChange`, `TimeSignature`, `MusicalPosition`.
- `test_support.rs` — test-only: the fixtures and a hand-built-SMF helper.
- `fixtures/` — the Mutopia MIDI files, the derived type-0 file, the generated sine
  SoundFont and WAVs; each named in `fixtures/LICENSE` (the provenance gate checks).

## Pitfalls and their guard tests
| Pitfall | Guard Test |
|---|---|
| A type-0 file is one track; its parts select channels, named `channel N` from 1, so percussion is `channel 10` | `src/song_file/tests.rs the_percussion_channel_of_a_type_zero_file_is_selectable_as_a_part` |
| midly's writer always uses running status: an unedited export is event-identical, byte-identical only when the original never repeats a status byte | `src/song_data/tests.rs unedited_fixtures_write_back_the_same_events_in_the_same_order` |
| An edited note's original pair must leave the export, or the note sounds twice | `src/song_data/tests.rs an_edited_note_leaves_its_original_pair_out_of_the_export_and_is_written_live` |
| Undo must re-link an original note to its events (`NoteImage::original_pair`), or an undone edit still rewrites the file | `src/song_data/tests.rs restoring_before_images_undoes_an_edit_down_to_the_exported_bytes` |
| A refused edit must change nothing: every note is checked before any is touched | `src/song_data/tests.rs a_refused_edit_changes_nothing` |
| Same-key overlaps pair first-in first-out and are reported, never refused | `src/song_data/tests.rs same_key_overlaps_pair_first_in_first_out_and_are_reported_without_refusing` |
| Tick → sample rounds up and sample → tick rounds down; mixing the rules double-counts or skips a beat crossing | `src/tempo_map/tests.rs beat_crossings_are_counted_by_integer_boundaries_across_a_tempo_and_meter_change` |
| A beat is the time signature's denominator note (a half note in 3/2), not always a quarter | `src/tempo_map/tests.rs egmont_bars_and_beats_follow_its_three_meters` |
| 120 BPM and 4/4 hold before the first tempo and time-signature event, and in a file with neither | `src/tempo_map/tests.rs a_file_without_tempo_or_meter_plays_at_120_bpm_in_four_four` |
| RON parses `inf` and `NaN`, so the gain check must refuse them | `src/song_file/tests.rs schema_errors_fail_loud_naming_the_collection_and_the_entry` |
| A future sidecar must be refused for its version, not for the fields it added — the version is probed before the full parse | `src/song_file/tests.rs a_future_version_is_refused_for_its_version_not_its_new_fields` |
| A shared track name is refused by `Name` and never by `Index` | `src/song_file/tests.rs a_shared_track_name_needs_the_index_escape_hatch_which_is_never_refused` |
| The documented example must stay valid; the test reads it from `docs/SONG_FORMAT.md` | `src/song_file/tests.rs the_documented_example_parses_validates_and_round_trips` |
| midly's default `parallel` feature pulls rayon; the workspace pin turns default features off and keeps `std` | — none |
| midly parses leniently: a track cut short reads as a shorter track instead of failing | — none |
| Export normalizes track ends: every exported track closes with one EndOfTrack at its last event or the original end, so a track that had none (a lenient parse) gains one, and a tail of deleted notes leaves the original end in place — a song's length is authored, not derived | — none |
| Once a tempo or time signature is edited, every timing event is re-emitted on track 0; a file that kept them elsewhere comes back rearranged (reimport is identical) | `src/song_data/tests.rs export_then_reimport_reproduces_every_kind_of_edit` |

## Testing
- `cargo test -p music_midi` and `cargo test -p music_midi --no-default-features` — 0
  failed, 0 ignored
- `scripts/check_audio_provenance.sh` after touching `fixtures/`
