# Music MIDI Crate — Agent Context

The conductor and its song core: the `.mid.ron` song sidecar, a Standard MIDI File as owned
and editable data, the tempo map, the background sequencer, and the conductor that renders a
song's background score and instrument slots. `docs/SONG_FORMAT.md` is the author's guide to
the sidecar; this file is the code's.

## The contract
- **No I/O, no rodio.** Bytes in, data out. The audio crate reads files, decodes
  SoundFonts and WAVs, and hands this crate the MIDI bytes, the sidecar text and a
  `ResourceMetadata` describing what it loaded. Fixtures reach tests by `include_bytes!`.
- **Exactly two dependencies of its own**, `midly` and `rustysynth`, pinned in the root
  `[workspace.dependencies]`; any third is a plan change. Both are re-exported
  (`music_midi::midly`, `music_midi::rustysynth`) so no other crate names them. `serde`,
  `ron` and `thiserror` are the workspace's.
- **The `synth` feature** (default on) owns rustysynth and everything that renders: the
  conductor, its commands and its published position. The schema, the data model and the
  sequencer build and test with `--no-default-features`, which is how the editor depends
  on the crate.
- **The audio thread never allocates or parses.** `Song::new` parses the SoundFonts and
  prepares the background score on the thread that builds it; `Conductor::load` only
  builds synthesizers; `Conductor::render` touches preallocated buffers.
- **Authored data fails loud**, naming the file, the collection and the entry
  (`parts['drums']`), in the manner of `engine_core`'s `sheet_file.rs`.

## File Map
- `lib.rs` — crate docs, re-exports, and the frozen capacities: 8 instrument slots, 15
  lanes and 24 sounding notes a slot, a 128-voice synthesizer pool, 16 sample voices, a
  64-frame block, 256 queued commands, 512 pending events (the trigger scheduler's bound,
  which arrives with triggering; the 16 sample voices are the sample slot's, which arrives
  with samples — until then neither bounds anything), ±24 semitones for a sample,
  1 ms to 10 s of sample, 1 MiB and 262 144 events of MIDI.
- `error.rs` — `SongError` (reading and writing MIDI), `EditError` (a refused mutation),
  `SongFileError` = the file's label + a `SongFileProblem`, `Collection`, and
  `ConductorError` (a conductor that cannot be built or a song it cannot load).
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
- `sequencer.rs` — **the background sequencer** (pure): `Sequencer` owns the song clock
  and drives a `SequencerSink` with messages on the sink's grain, the seam split, the
  reconstructions and same-key coalescing; `seek_to_tick`, `seek_to_sample`, `advance`.
- `sequencer/score.rs` — `BackgroundScore::new(&SongData, trigger tracks)`: every
  background channel event in sending order, exactly what an export would write;
  `ChannelMessage`, `ScoreEvent`.
- `sequencer/fold.rs` — `ChannelStateFold`: each channel's program, bank, controllers,
  registered parameters and pitch bend folded from the score, and their replay order.
- `conductor.rs` (`synth`) — `Conductor` (`new`, `load`, `render`, `apply`),
  `ConductorConfig`, `Capacities`, `BusGains`, `LoadReport`/`LoadWarning`; the block loop,
  the transport, the mix and the finite guard.
- `conductor/song.rs` — `Song::new`: SoundFonts from bytes, trigger tracks from the
  parts, the background score, instrument layering, the background's peak voice count.
- `conductor/background.rs` — `BackgroundPlayer`: two synthesizers taking turns as the
  sequencer's sink.
- `conductor/slots.rs` — `InstrumentSlot`: one synthesizer per instrument, lanes skipping
  the percussion channel.
- `conductor/ledger.rs` — `VoiceLedger` (pure): sounding notes bounded exactly, lanes
  leased while sounding and stolen oldest-first, `SourceKey`.
- `commands.rs` (`synth`) — `ConductorCommand`, `ConductorHandle` (the bounded, never
  blocking sender), `CommandRejected`.
- `position.rs` (`synth`) — `SharedPosition` (atomics published once a block),
  `PositionSnapshot`, `TransportState`.
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
| A note may not end past tick 268 435 455 (MIDI's largest delta), or the export could hold a gap no file can write; `add_note`, `move_notes` and `resize_notes` refuse it, naming the note | `src/song_data/tests.rs an_edit_ending_past_the_last_writable_midi_tick_is_refused_naming_the_note` |
| The background SoundFont must be one the loader decoded (`ResourceMetadata::soundfonts`) | `src/song_file/tests.rs a_background_soundfont_the_loader_did_not_decode_is_refused` |
| Playback reads notes the way export writes them: original events for linked notes, the data model for edited ones, nothing for removed ones | `src/sequencer/tests.rs an_edited_song_plays_exactly_what_its_export_would_write` |
| A part's track never plays in the background: in type 1 the whole track, in type 0 its channel | `src/sequencer/tests.rs a_trigger_track_never_plays_in_the_background` |
| rustysynth's `note_off` releases every voice of a key, so same-key overlaps coalesce per channel and only the last instance's note-off is sent | `src/sequencer/tests.rs a_pedalled_repeat_on_one_key_sustains_until_its_last_instance_ends` |
| A block crossing the loop end is split at the seam sample, with a reconstruction between the halves; a note-off on the seam is not sent, the seam releases it | `src/sequencer/tests.rs a_block_crossing_the_loop_end_splits_at_the_seam_and_the_downbeat_starts_it` `src/conductor/tests.rs the_loop_seam_releases_every_held_note_and_the_downbeat_attacks_on_its_exact_sample` |
| rustysynth applies a message only at its next internal block and only `reset` moves that grid (cutting every voice), so the background alternates two synthesizers: the outgoing one rings out its tails while the incoming one is reset on the reconstruction's exact frame. A tail still ringing on the idle one at the following reconstruction is cut | `src/conductor/tests.rs the_loop_seam_releases_every_held_note_and_the_downbeat_attacks_on_its_exact_sample` |
| The fold takes state events up to and including the destination tick and notes only before it, so a seek to 0 keeps tick 0's tempo, programs and controllers | `src/sequencer/tests.rs seeking_to_tick_zero_keeps_its_tempo_and_programs` `src/sequencer/tests.rs a_seek_rebuilds_the_same_channel_state_as_playing_up_to_the_destination` `src/conductor/tests.rs a_seek_to_tick_zero_restores_its_tempo_and_controllers_exactly` |
| A note spanning a seek's destination is not sounded again, and its note-off is not sent: it could release a newer note of the same key | `src/sequencer/tests.rs a_note_spanning_the_seek_destination_is_not_resounded_and_its_note_off_is_not_sent` |
| Controllers replayed by number send data entry (6) before the parameter selection (100, 101) it belongs to; the fold replays registered parameters as select-then-enter | `src/sequencer/tests.rs replaying_a_fold_rebuilds_its_state_with_registered_parameters_in_order` |
| The ledger knows the sounding set exactly and never holds more than the cap: past it, the oldest note gets a real note-off | `src/conductor/ledger/tests.rs past_the_sounding_cap_the_oldest_note_is_released_and_the_count_never_exceeds_it` |
| With every lane leased, the source that leased longest ago loses its notes and its lane; a new source is never refused | `src/conductor/ledger/tests.rs a_sixteenth_source_takes_the_oldest_sources_lane_and_is_never_refused` |
| A note-off for a note the ledger already released sends nothing: its lane may be another source's now | `src/conductor/ledger/tests.rs a_note_off_for_a_stolen_note_sends_nothing_to_the_lane_another_source_now_holds` |
| Every output sample passes the finite guard: a value that is not finite is written as 0 and counted | `src/conductor/tests.rs a_mix_that_is_not_finite_is_written_as_silence_and_counted` `src/conductor/tests.rs a_storm_of_random_notes_and_seeks_never_writes_a_non_finite_sample` |
| Pause releases every note and holds the clock; resume rebuilds the channels at the held sample, so nothing bursts | `src/conductor/tests.rs pause_holds_the_clock_and_resume_continues_from_the_same_sample` |
| Commands drain at block boundaries, not at the start of every `render` call: a render that ends mid-block leaves the next command for the call after | — none |
| rustysynth hard-wires channel 10 (index 9) as percussion and adds 128 to its bank, so a slot's lanes skip it | — none |

## Testing
- `cargo test -p music_midi` and `cargo test -p music_midi --no-default-features` — 0
  failed, 0 ignored
- `scripts/check_audio_provenance.sh` after touching `fixtures/`
