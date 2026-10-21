# Music MIDI Crate — Agent Context

The conductor and its song core: the `.mid.ron` song sidecar, a Standard MIDI File as owned
and editable data, the tempo map, the background sequencer, and the conductor that renders a
song's background score and instrument slots, and plays its parts when the game triggers
their bindings. `docs/SONG_FORMAT.md` is the author's guide to the sidecar; this file is
the code's.

## The contract
- **No I/O, no rodio.** Bytes in, data out. The audio crate reads files, decodes
  SoundFonts and WAVs (a sample's WAV into an `OwnedPcm`), and hands this crate the MIDI
  bytes, the sidecar text, the decoded samples and a `ResourceMetadata` describing what
  it loaded. Fixtures reach tests by `include_bytes!`; the sine WAVs reach them through
  `test_support::fixture_pcm`, a reader of the generator's own 16-bit layout that exists
  only in tests.
- **Exactly two dependencies of its own**, `midly` and `rustysynth`, pinned in the root
  `[workspace.dependencies]`; any third is a plan change. Both are re-exported
  (`music_midi::midly`, `music_midi::rustysynth`) so no other crate names them. `serde`,
  `ron` and `thiserror` are the workspace's, and so is the engine's own `common`, under
  `synth` only: a trigger's stamp is `common::clock::Instant`, the browser's clock on
  the web.
- **The `synth` feature** (default on) owns rustysynth and everything that renders: the
  conductor, its commands, its published position, the sources, the triggers and the
  sample voices. The schema, the data model, `OwnedPcm` and the sequencer build and test
  with `--no-default-features`, which is how the editor depends on the crate; the audio
  crate turns `synth` on, since the conductor plays there.
- **The audio thread never allocates or parses.** `Song::new` parses the SoundFonts,
  keeps the decoded samples in its `SampleBank` and prepares the background score on the
  thread that builds it; `Conductor::load` builds the synthesizers and the sample slots
  (their voice lists allocated to the live cap plus the fade budget) and sizes the
  trigger engine (parts, bindings, the source table, the scheduled-event buffer) to its
  capacities; `Conductor::render` touches preallocated buffers. A binding is resolved
  from its action on the game thread (`Song::binding_index`), so `render` never compares
  a string.
- **Authored data fails loud**, naming the file, the collection and the entry
  (`parts['drums']`), in the manner of `engine_core`'s `sheet_file.rs`.

## File Map
- `lib.rs` — crate docs, re-exports, and the frozen capacities: 8 instrument slots, 15
  lanes and 24 sounding notes a slot, a 128-voice synthesizer pool, 16 live voices a
  sample slot plus a fade budget of 16, a sample voice level of 0.2, the 5 ms sample fade, 8–96 kHz samples, a 64-frame
  block, 256 queued commands with 16 of them reserved for lifecycle and transport, 64
  commands drained a block (a full queue drains in 4 blocks, 5.8 ms at 44.1 kHz, after
  the drain gap), 512 scheduled events, 128 tracked sources, the 61 ms device-period
  allowance of the late tolerance, ±24 semitones for a sample, 1 ms to 10 s of sample,
  1 MiB and 262 144 events of MIDI.
- `error.rs` — `SongError` (reading and writing MIDI), `EditError` (a refused mutation),
  `SongFileError` = the file's label + a `SongFileProblem`, `Collection`, `PcmError`
  (decoded sample audio outside the limits, naming the limit), and `ConductorError` (a
  conductor that cannot be built or a song it cannot load).
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
- `pcm.rs` — `OwnedPcm`: a decoded sample (rate, mono or stereo, interleaved values in an
  `Arc`), refused by `new` outside 8–96 kHz, 1 ms–10 s or with a value not finite;
  `check_format` and `maximum_frames` let a decoder refuse before decoding. Builds
  without `synth`.
- `sequencer.rs` — **the background sequencer** (pure): `Sequencer` owns the song clock
  and drives a `SequencerSink` with messages on the sink's grain, the seam split, the
  reconstructions and same-key coalescing; `seek_to_tick`, `seek_to_sample`, `advance`.
- `sequencer/score.rs` — `BackgroundScore::new(&SongData, trigger tracks)`: every
  background channel event in sending order, exactly what an export would write;
  `ChannelMessage`, `ScoreEvent`.
- `sequencer/fold.rs` — `ChannelStateFold`: each channel's program, bank, controllers,
  registered parameters and pitch bend folded from the score, and their replay order.
- `conductor.rs` (`synth`) — `Conductor` (`new`, `load`, `render`, `render_at`, `apply`,
  `part_cursor`), `LoadReport`/`LoadWarning`; the block loop (a bounded drain, then the
  scheduled events due), the transport and what it releases, the command dispatch, the
  mix and the finite guard.
- `conductor/config.rs` — `ConductorConfig`, `Capacities`, `BusGains`, and every check
  `Conductor::new` makes of them.
- `conductor/song.rs` — `Song::new` (SoundFonts from bytes) and `Song::with_sound_fonts`
  (SoundFonts already parsed, so a loader's cache shares one parse between songs): the
  samples it names, trigger tracks from the parts, the background score, instrument
  layering, the background's peak voice count; `parse_sound_font`, and
  `preset_layering`, every preset's layering as `ResourceMetadata` carries it.
- `sample_bank.rs` (`synth`) — `SampleBank`: each sample path the song names with its
  decoded audio, held once however many instruments name it.
- `sample_voice.rs` (`synth`) — `SampleVoice`: one voice's position, pitch step (linear
  interpolation at `2^(semitones/12) × source_rate / output_rate`, ±24 semitones), gain
  and fade — the sample's own end included; mono to both channels, stereo kept.
- `conductor/background.rs` — `BackgroundPlayer`: two synthesizers taking turns as the
  sequencer's sink.
- `conductor/slots.rs` — `InstrumentSlot`: one synthesizer per SoundFont instrument, lanes
  skipping the percussion channel; `Slot`, the enum over it and `SampleSlot` every
  trigger plays through, and `SlotSink`, the slots as the trigger engine's sink.
- `conductor/sample_slot.rs` — `SampleSlot`: a sample instrument's voices, no lanes; the
  live cap (oldest live voice faded), the ceiling of the live cap plus the fade budget
  (the newest note-on refused, `free_voices` asked first), one-shot and gated note-offs,
  a `NoteTicket` per voice.
- `conductor/ledger.rs` — `VoiceLedger` (pure): sounding notes bounded exactly, lanes
  leased while sounding and stolen oldest-first, a `NoteTicket` for every note so a
  note-off ends that note and no other.
- `commands.rs` (`synth`) — `ConductorCommand` (transport, `Trigger`, `DeactivateSource`,
  `StopAllSources`), `ConductorHandle` (the bounded, never blocking sender with the
  in-flight count and the lifecycle reserve), `CommandRejected`.
- `position.rs` (`synth`) — `SharedPosition` (atomics published once a block),
  `PositionSnapshot` (the trigger counters and the sample voice count included),
  `TransportState`.
- `sources.rs` (`synth`) — `SourceId { id, generation, epoch }` and `SourceTable`: lazy
  activation, deactivation, supersession, the flush floor, bounded eviction.
- `triggers.rs` (`synth`) — `TriggerEngine`: bindings to parts and slots, a trigger's
  path from stamp to notes, the counters, `BindingIndex`, `Song::binding_index`.
- `triggers/cursor.rs` — `PartCursor`: `Loop`, `Sequence` and `AtSongPosition`, choosing
  a unit apart from consuming it.
- `triggers/quantize.rs` — the stamp's song sample and the grid placement, with the
  mapping's error bound.
- `overflow.rs` (`synth`) — `PendingEvents`, the scheduled-event buffer, and the rule that
  refuses a trigger before its notes would overflow it or its instrument.
- `test_support.rs` — test-only: the fixtures, a hand-built-SMF helper, and the sine WAVs
  as `OwnedPcm`.
- `fixtures/` — the Mutopia MIDI files, the derived type-0 file and its cast derivative
  (`scripts/make_cast_fixture.py`: a program on each melodic channel), the generated sine
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
| A resume rebuilds the channels by `seek_to_tick`'s rule: state events on the destination sample are folded, its notes sent again | `src/sequencer/tests.rs a_seek_to_the_sample_of_a_program_change_folds_it_into_the_rebuilt_state` `src/conductor/tests/triggering.rs resuming_on_a_sample_carrying_a_state_change_sounds_as_a_seek_to_it_does` |
| A seek while paused moves the sequencer as well as the resume point, or a stop after it publishes the position the pause held | `src/conductor/tests/triggering.rs a_seek_while_paused_is_the_position_a_stop_then_publishes` |
| `Play`, a seek and a loop wrap release every slot's notes and drop every scheduled event; a pause and a stop do too, keeping the cursors; scheduled events past the loop seam are never fired before the wrap drops them | `src/conductor/tests/triggering.rs play_releases_every_triggered_note` `src/conductor/tests/triggering.rs a_loop_wrap_and_a_pause_never_hang_a_triggered_note` |
| A note-off names its note's ticket, never its source and key: a source striking the same key again keeps the newer note its whole length | `src/conductor/ledger/tests.rs a_note_off_ends_only_its_own_note_when_the_source_has_struck_the_key_again` |
| The lifecycle reserve is accounting on one channel: the sender claims its place before `try_send` (and gives it back on failure), a trigger only below `capacity − reserve`, so two racing senders cannot both take the last place and a stop always fits | `src/conductor/tests/saturation.rs a_ten_thousand_trigger_storm_never_crowds_out_a_lifecycle_command_and_keeps_the_cursor` `src/conductor/tests/saturation.rs a_lifecycle_storm_past_the_queue_is_lost_by_none_once_the_host_retries` |
| One channel keeps order: a trigger sent before a deactivation is drained, and plays, before it | `src/conductor/tests/saturation.rs a_consumer_stalled_ten_blocks_drains_triggers_before_the_deactivation_that_followed_them` |
| A trigger is placed from its stamp, never from its drain, and one older than a block plus 61 ms when drained is dropped: a stalled consumer's backlog never bursts | `src/conductor/tests/saturation.rs two_hundred_triggers_queued_before_a_two_second_stall_never_sound` `src/triggers/tests.rs a_trigger_older_than_the_tolerance_is_dropped_and_its_cursor_kept` |
| Choosing a unit and consuming it are two steps, so a refused trigger — its instrument's waiting note-ons at the sounding cap, no room in the scheduler, or a sample slot without room under its ceiling — never moves the cursor | `src/triggers/tests.rs past_the_instruments_cap_the_newest_scheduled_trigger_is_refused_and_its_cursor_kept` `src/triggers/tests.rs a_full_scheduler_refuses_the_newest_trigger_and_keeps_its_cursor` |
| An epoch is one count across every source, so a flush can name a point in it; the floor outlives a load, so a straggler of a flushed epoch never activates on the next song | `src/sources/tests.rs a_flush_ends_only_sources_activated_before_its_epoch_and_refuses_those_epochs_after` `src/conductor/tests/saturation.rs a_flush_through_a_full_channel_spares_the_epoch_queued_behind_it_and_the_next_song_plays` |
| Naming a newer activation of an id ends the older one; a deactivation of the older one arriving late never touches the newer | `src/sources/tests.rs a_recycled_id_supersedes_its_old_generation_which_never_revives` `src/sources/tests.rs a_late_deactivation_of_an_older_epoch_never_touches_the_newer_one` |
| A full source table forgets a dead identity before it evicts the oldest live source; a forgotten one could be revived only by a trigger the host sends after deactivating it | `src/sources/tests.rs a_full_table_forgets_a_dead_identity_before_it_evicts_the_oldest_live_source` |
| A binding index belongs to one song: commands still queued at a `load` are drained against the new song, so a host reusing one conductor flushes and loads once the flush is drained — the audio crate instead gives each song a conductor of its own | — none |
| A trigger sounds on the first frame of a block, the slots' synthesizer grid; `load` restarts the block so that grid stays the conductor's | `src/conductor/tests/triggering.rs a_quantized_trigger_attacks_on_the_first_frame_of_the_block_holding_its_beat` |
| Under a storm overflowing the pool, the synthesizer's stealer takes only tails: no energy is lost at a held note's pitch, and the largest step stays under the frozen 0.02 | `src/conductor/tests/saturation.rs a_storm_overflowing_the_pool_steps_under_the_frozen_threshold_and_cuts_no_sounding_note` `src/conductor/tests/saturation.rs one_lane_hammering_one_key_steps_under_the_threshold_and_hangs_nothing` |
| `AtSongPosition` plays the unit that started last at or before its tick, what remains of it; when that unit has ended it is a rest, and an earlier, longer unit is not looked back for | `src/triggers/tests.rs at_song_position_plays_the_unit_sounding_through_a_tempo_change_and_nothing_in_a_rest` |
| rustysynth hard-wires channel 10 (index 9) as percussion and adds 128 to its bank, so a slot's lanes skip it | — none |
| A trigger's source is admitted only after every check that may refuse it: admission into a full table ends the oldest live source, which a refused, late or paused trigger must never cost | `src/triggers/tests.rs a_refused_late_or_paused_trigger_never_evicts_a_live_source_from_a_full_table` |
| A host pulling many blocks in one `render` call drains each block at its own instant (the call's `now` plus the frames before it), or a trigger drained late in the pull reads too young and misses its beat | `src/conductor/tests/triggering.rs a_long_pull_places_a_trigger_drained_in_its_last_block_as_block_by_block_rendering_does` |
| A sample is at least 1 ms (two frames at the least), or a voice has no frame to interpolate towards and an empty one underflows | `src/pcm/tests.rs a_sample_shorter_than_a_millisecond_is_refused_and_one_exactly_a_millisecond_is_kept` |
| Pitch is speed: a sample a fifth up lasts two thirds as long and an octave down twice as long, and the step carries the source-to-output rate ratio as well as the semitones | `src/conductor/sample_slot/tests.rs a_voice_lasts_its_pitch_scaled_length_two_thirds_a_fifth_up_and_double_an_octave_down` `src/conductor/sample_slot/tests.rs a_sample_from_either_source_rate_plays_its_written_pitch_at_the_root_a_fifth_up_and_an_octave_down` |
| A key further than 24 semitones from the root plays at 24: the voice clamps what the sidecar's validation should already have refused | `src/conductor/sample_slot/tests.rs a_key_more_than_two_octaves_from_the_root_plays_at_two_octaves` |
| A one-shot ignores its note-off and is sounding until its pitch-scaled end; a gated voice fades over 5 ms from it; every forced end (steal, deactivation, seek, wrap, pause, stop) fades a one-shot too | `src/conductor/sample_slot/tests.rs a_one_shot_plays_through_its_note_off_and_a_gated_voice_fades_out_from_it` `src/conductor/sample_slot/tests.rs every_forced_end_fades_without_a_step_steeper_than_the_sine_itself` `src/conductor/tests/samples.rs a_sample_and_a_preset_triggered_in_one_block_both_end_on_a_wrap_a_seek_a_pause_and_a_stop` |
| Fading voices are bounded without a cut: at the live cap plus the fade budget the newest note is refused — the trigger engine asks the slot's room first and refuses the whole trigger, cursor kept, counted as rejected — so a burst never clicks; its largest step is frozen at 0.083 under a 64-trigger burst at the sample voice level (measured 0.055; cutting fading voices at full level stepped 2.979) | `src/conductor/sample_slot/tests.rs a_burst_past_the_ceiling_refuses_the_newest_notes_and_cuts_no_voice` `src/conductor/tests/samples.rs a_burst_of_sixty_four_triggers_in_one_block_refuses_the_newest_at_the_ceiling_and_stays_finite` |
| A sample ends on the same 5 ms ramp as every other end: within one fade of its last frame a voice starts fading from the frames left, so a recording that stops at full scale reaches silence on its last frame instead of stepping. That shorter ramp is its own length, not a tail of the 5 ms one, or a sample shorter than the fade (or pitched up into it) would start quiet and pitch would set loudness | `src/conductor/sample_slot/tests.rs a_sample_ending_at_full_scale_fades_to_silence_on_its_last_frame` `src/conductor/sample_slot/tests.rs a_sample_shorter_than_the_fade_starts_at_full_level_and_still_lands_on_silence` |
| A loader's preset layering and a song's are one count, and a song built from parsed banks shares them rather than parsing again | `src/conductor/tests.rs a_loaders_preset_layering_and_a_song_built_from_parsed_banks_agree_with_the_song_it_parses` |
| Two sources on one sample hold voices of their own: a note-off names its voice's ticket, never the sample and key | `src/conductor/sample_slot/tests.rs two_sources_on_one_sample_hold_voices_with_their_own_positions_and_note_offs` |
| A sample instrument is slot `n` like any instrument; its sample must be in the song's bank, or `load` refuses the song and the loaded one plays on | `src/conductor/tests/samples.rs a_song_whose_sample_was_not_supplied_is_refused_and_the_loaded_song_plays_on` |

## Testing
- `cargo test -p music_midi` and `cargo test -p music_midi --no-default-features` — 0
  failed, 0 ignored
- `scripts/check_audio_provenance.sh` after touching `fixtures/`
