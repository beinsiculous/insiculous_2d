use midly::num::u15;
use midly::{Format, Fps, MetaMessage, Smf, Timing, TrackEventKind};

use super::*;
use crate::error::EditError;
use crate::test_support::*;

/// Every note of every track as `(tick, key, velocity, duration)`, sorted, ids aside —
/// what a listener would hear, and what an export and reimport must preserve.
fn heard_notes(song: &SongData) -> Vec<Vec<(u64, u8, u8, u64)>> {
    song.tracks()
        .iter()
        .map(|track| {
            let mut notes: Vec<_> = track
                .units()
                .iter()
                .flat_map(|unit| {
                    unit.notes
                        .iter()
                        .map(move |note| (unit.tick, note.key, note.velocity, note.duration_ticks))
                })
                .collect();
            notes.sort_unstable();
            notes
        })
        .collect()
}

/// Every track's events at absolute ticks, as midly parses them.
fn parsed_events(bytes: &[u8]) -> (midly::Header, Vec<Vec<(u64, TrackEventKind<'_>)>>) {
    let smf = Smf::parse(bytes).expect("parses");
    let tracks = smf
        .tracks
        .iter()
        .map(|track| {
            let mut tick = 0;
            track
                .iter()
                .map(|event| {
                    tick += u64::from(event.delta.as_int());
                    (tick, event.kind)
                })
                .collect()
        })
        .collect();
    (smf.header, tracks)
}

fn first_note(song: &SongData, track: usize) -> NoteImage {
    let id = song.tracks()[track].units()[0].notes[0].id;
    song.note_image(id).expect("first note exists")
}

#[test]
fn type_one_fixtures_read_every_track_with_its_name_channel_and_notes() {
    let (bach, bach_report) = read(BACH_INVENTION);
    let (egmont, _) = read(EGMONT);

    let bach_tracks: Vec<_> = bach
        .tracks()
        .iter()
        .map(|track| (track.name(), track.channel(), track.note_count()))
        .collect();
    let egmont_counts: Vec<_> = egmont.tracks().iter().map(SongTrack::note_count).collect();

    assert_eq!(bach.format(), SmfFormat::Parallel);
    assert_eq!(bach.ticks_per_quarter(), 384);
    assert_eq!(
        bach_tracks,
        vec![
            (Some("control track"), None, 0),
            (Some("one"), Some(1), 328),
            (Some("two"), Some(2), 306)
        ]
    );
    assert!(bach_report.is_editable());
    assert_eq!(
        egmont_counts,
        vec![0, 425, 393, 783, 843, 832, 577, 339, 472, 214, 1437, 1480, 1379, 859, 648]
    );
    assert_eq!(
        egmont.tracks()[10].channel(),
        Some(10),
        "LilyPond skips the percussion channel"
    );
    assert_eq!(egmont.note_count(), egmont_counts.iter().sum::<usize>());
}

#[test]
fn simultaneous_note_ons_group_into_one_unit_keeping_each_velocity_and_duration() {
    let bytes = smf(&[vec![
        (0, note_on(0, 60, 90)),
        (0, note_on(0, 64, 80)),
        (0, note_on(0, 67, 70)),
        (96, note_off(0, 67)),
        (96, note_off(0, 64)),
        (192, note_off(0, 60)),
        (0, note_on(0, 72, 100)),
        (384, note_off(0, 72)),
    ]]);

    let (song, _) = read(&bytes);
    let units = song.tracks()[0].units();
    let chord: Vec<_> = units[0]
        .notes
        .iter()
        .map(|note| (note.key, note.velocity, note.duration_ticks))
        .collect();

    assert_eq!(units.len(), 2);
    assert_eq!(units[0].tick, 0);
    assert_eq!(chord, vec![(60, 90, 384), (64, 80, 192), (67, 70, 96)]);
    assert_eq!(units[1].tick, 384);
    assert_eq!(units[1].notes.len(), 1);
}

#[test]
fn type_zero_file_becomes_one_selectable_track_per_channel_keeping_global_tempo_and_meta() {
    let (type_zero, report) = read(NAPOLITAN_DANCE_TYPE_ZERO);
    let (type_one, _) = read(NAPOLITAN_DANCE);

    let tracks: Vec<_> = type_zero
        .tracks()
        .iter()
        .map(|track| (track.name(), track.channel(), track.note_count()))
        .collect();
    let original = &type_zero.retained_tracks()[0];
    let keeps = |wanted: fn(&MetaMessage) -> bool| {
        original
            .iter()
            .any(|event| matches!(event.kind(), TrackEventKind::Meta(meta) if wanted(&meta)))
    };

    assert_eq!(type_zero.format(), SmfFormat::SingleTrack);
    assert_eq!(type_zero.retained_tracks().len(), 1);
    assert_eq!(
        tracks,
        vec![
            (Some("channel 2"), Some(1), 248),
            (Some("channel 4"), Some(3), 453),
            (Some("channel 5"), Some(4), 74),
            (Some("channel 10"), Some(9), 110),
        ]
    );
    assert!(type_zero
        .tracks()
        .iter()
        .all(|track| track.source_track() == 0));
    assert_eq!(
        heard_notes(&type_zero)[0],
        heard_notes(&type_one)[1],
        "the flute is the same notes in both files"
    );
    assert_eq!(type_zero.tempo_changes(), type_one.tempo_changes());
    assert_eq!(type_zero.time_signatures(), type_one.time_signatures());
    assert!(keeps(
        |meta| matches!(meta, MetaMessage::TrackName(name) if name.starts_with(b"Napolitan Dance"))
    ));
    assert!(keeps(|meta| matches!(meta, MetaMessage::KeySignature(..))));
    assert!(report.is_editable());
}

#[test]
fn same_key_overlaps_pair_first_in_first_out_and_are_reported_without_refusing() {
    let bytes = smf(&[vec![
        (0, note_on(3, 60, 100)),
        (96, note_on(3, 60, 50)),
        (96, note_off(3, 60)),
        (96, note_off(3, 60)),
        (0, note_on(3, 62, 90)),
    ]]);

    let (song, report) = read(&bytes);
    let notes: Vec<_> = heard_notes(&song).remove(0);

    assert_eq!(
        notes,
        vec![(0, 60, 100, 192), (96, 60, 50, 192), (288, 62, 90, 0)]
    );
    assert_eq!(
        report.same_key_overlaps,
        vec![SameKeyOverlap {
            track: 0,
            channel: 3,
            key: 60,
            tick: 96
        }]
    );
    assert!(report.is_editable(), "an overlap is informational");
}

#[test]
fn a_type_one_track_playing_notes_on_two_channels_is_reported_not_editable() {
    let bytes = smf(&[vec![
        (0, note_on(0, 60, 100)),
        (0, note_on(1, 64, 100)),
        (96, note_off(0, 60)),
        (0, note_off(1, 64)),
    ]]);

    let (song, report) = read(&bytes);

    assert!(!report.is_editable());
    assert_eq!(
        report.unsupported,
        vec![UnsupportedFeature::MixedChannelTrack {
            track: 0,
            channels: vec![0, 1]
        }]
    );
    assert_eq!(song.note_count(), 2, "the notes still play");
}

#[test]
fn unreadable_and_unsupported_midi_files_are_rejected_with_the_reason() {
    let one_note = vec![(0, note_on(0, 60, 100)), (96, note_off(0, 60))];
    let sequential = smf_with_timing(
        Format::Sequential,
        Timing::Metrical(u15::new(384)),
        std::slice::from_ref(&one_note),
    );
    let smpte = smf_with_timing(
        Format::Parallel,
        Timing::Timecode(Fps::Fps25, 40),
        std::slice::from_ref(&one_note),
    );
    let zero_division = smf_with_timing(
        Format::Parallel,
        Timing::Metrical(u15::new(0)),
        std::slice::from_ref(&one_note),
    );
    let no_tracks = smf(&[]);
    let zero_tempo = smf(&[vec![(0, tempo(0))]]);
    let odd_meter = smf(&[vec![(
        0,
        TrackEventKind::Meta(MetaMessage::TimeSignature(3, 8, 24, 8)),
    )]]);
    let mut truncated = smf(&[one_note]);
    truncated.truncate(10);
    let too_many_events = smf(&[(0..=crate::MIDI_EVENT_LIMIT)
        .map(|_| (0, note_on(0, 60, 0)))
        .collect()]);

    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        (
            "not a MIDI file",
            b"RIFF? no".to_vec(),
            "not a readable Standard MIDI File",
        ),
        (
            "a header cut short",
            truncated,
            "not a readable Standard MIDI File",
        ),
        (
            "over the byte limit",
            vec![0; crate::MIDI_BYTE_LIMIT + 1],
            "the limit is 1048576",
        ),
        (
            "over the event limit",
            too_many_events,
            "more than 262144 events",
        ),
        ("type 2", sequential, "SMF type 2"),
        (
            "SMPTE division",
            smpte,
            "SMPTE time division (25 frames a second)",
        ),
        (
            "zero division",
            zero_division,
            "zero ticks per quarter note",
        ),
        ("no tracks", no_tracks, "no tracks"),
        ("a zero tempo", zero_tempo, "tempo of zero"),
        (
            "3/256 at 384 ticks a quarter",
            odd_meter,
            "time signature 3/2^8",
        ),
    ];
    for (case, bytes, expected) in cases {
        let Err(error) = SongData::from_smf(&bytes) else {
            panic!("{case} must be rejected");
        };
        assert!(
            error.to_string().contains(expected),
            "{case}: expected {expected:?} in {error}"
        );
    }
}

#[test]
fn unedited_fixtures_write_back_the_same_events_in_the_same_order() {
    for (name, bytes) in ALL_FIXTURES {
        let (song, _) = read(bytes);

        let exported = song.write_smf().expect("writes");
        let (original_header, original_tracks) = parsed_events(bytes);
        let (exported_header, exported_tracks) = parsed_events(&exported);

        assert_eq!(exported_header, original_header, "{name}: header");
        assert_eq!(
            exported_tracks, original_tracks,
            "{name}: every event, tick and order"
        );
    }
}

#[test]
fn an_unedited_file_that_never_repeats_a_status_byte_writes_back_byte_identically() {
    let (song, _) = read(BACH_INVENTION);

    assert_eq!(song.write_smf().expect("writes"), BACH_INVENTION);
}

#[test]
fn an_edited_note_leaves_its_original_pair_out_of_the_export_and_is_written_live() {
    let (mut song, _) = read(BACH_INVENTION);
    let note = first_note(&song, 1);

    song.set_velocity(&[note.id], 100).expect("edits");
    let exported = song.write_smf().expect("writes");
    let (_, tracks) = parsed_events(&exported);
    let strikes: Vec<u8> = tracks[1]
        .iter()
        .filter_map(|(tick, kind)| match kind {
            TrackEventKind::Midi {
                message: midly::MidiMessage::NoteOn { key, vel },
                ..
            } if *tick == note.tick && key.as_int() == note.key && vel.as_int() > 0 => {
                Some(vel.as_int())
            }
            _ => None,
        })
        .collect();

    assert_ne!(note.velocity, 100);
    assert_eq!(strikes, vec![100], "one strike, the live one");
    assert_eq!(heard_notes(&read(&exported).0), heard_notes(&song));
}

#[test]
fn export_then_reimport_reproduces_every_kind_of_edit() {
    let (mut song, _) = read(BACH_INVENTION);
    let upper: Vec<NoteId> = song.tracks()[1]
        .units()
        .iter()
        .flat_map(|unit| unit.notes.iter().map(|note| note.id))
        .collect();
    let lower: Vec<NoteId> = song.tracks()[2]
        .units()
        .iter()
        .flat_map(|unit| unit.notes.iter().map(|note| note.id))
        .collect();

    song.remove_notes(&upper[0..2]).expect("removes");
    song.move_notes(&upper[10..12], 41_472)
        .expect("moves past the last bar");
    song.resize_notes(&[upper[20]], 7).expect("resizes");
    song.set_pitch(&[upper[30]], 127).expect("re-pitches");
    song.set_velocity(&lower[5..9], 1).expect("softens");
    song.assign_part(&[lower[40]], 1).expect("reassigns");
    song.add_note(
        2,
        NewNote {
            tick: 3,
            key: 0,
            velocity: 127,
            duration_ticks: 50_000,
        },
    )
    .expect("adds");
    song.set_tempo(TempoChange {
        tick: 1536,
        microseconds_per_quarter: 400_000,
    })
    .expect("sets a tempo");
    song.set_time_signature(TimeSignature::new(3072, 3, 8))
        .expect("sets a meter");
    let (reimported, _) = read(&song.write_smf().expect("writes"));

    assert_eq!(heard_notes(&reimported), heard_notes(&song));
    assert_eq!(reimported.tempo_changes(), song.tempo_changes());
    assert_eq!(reimported.time_signatures(), song.time_signatures());
    assert_eq!(reimported.note_count(), song.note_count());
}

#[test]
fn restoring_before_images_undoes_an_edit_down_to_the_exported_bytes() {
    let (mut song, _) = read(BACH_INVENTION);
    let unedited = song.write_smf().expect("writes");
    let first = first_note(&song, 1);
    let second = first_note(&song, 2);

    let moved = song.move_notes(&[first.id, second.id], 96).expect("moves");
    let removed = song.remove_notes(&[first.id]).expect("removes");
    song.restore_notes(&removed).expect("restores the removal");
    song.restore_notes(&moved).expect("restores the move");

    assert_eq!(moved, vec![first, second]);
    assert_eq!(song.note_image(first.id), Some(first));
    assert_eq!(
        song.write_smf().expect("writes"),
        unedited,
        "the original pairs are linked again"
    );
}

#[test]
fn an_edit_ending_past_the_last_writable_midi_tick_is_refused_naming_the_note() {
    const LAST_WRITABLE_TICK: u64 = 0x0FFF_FFFF;
    let (mut song, _) = read(BACH_INVENTION);
    let note = first_note(&song, 1);
    let before = heard_notes(&song);
    let to_the_limit = (LAST_WRITABLE_TICK - note.tick - note.duration_ticks) as i64;
    let new_note = |tick, duration_ticks| NewNote {
        tick,
        key: 60,
        velocity: 64,
        duration_ticks,
    };

    assert_eq!(
        song.move_notes(&[note.id], to_the_limit + 1),
        Err(EditError::PastSongEnd {
            note: note.id,
            limit: LAST_WRITABLE_TICK
        })
    );
    assert_eq!(
        song.resize_notes(&[note.id], LAST_WRITABLE_TICK),
        Err(EditError::PastSongEnd {
            note: note.id,
            limit: LAST_WRITABLE_TICK
        })
    );
    assert_eq!(
        song.add_note(1, new_note(LAST_WRITABLE_TICK, 1)),
        Err(EditError::NewNotePastSongEnd {
            tick: LAST_WRITABLE_TICK,
            key: 60,
            limit: LAST_WRITABLE_TICK
        })
    );
    assert_eq!(heard_notes(&song), before, "a refused edit changes nothing");

    song.move_notes(&[note.id], to_the_limit)
        .expect("a note may end on the last writable tick");
    song.add_note(1, new_note(LAST_WRITABLE_TICK - 1, 1))
        .expect("a new note may end on it too");
    let reread = SongData::from_smf(&song.write_smf().expect("every gap fits a MIDI delta"))
        .expect("the export reads back");
    assert_eq!(heard_notes(&reread.0), heard_notes(&song));
}

#[test]
fn a_refused_edit_changes_nothing() {
    let (mut song, _) = read(BACH_INVENTION);
    let note = first_note(&song, 1);
    let ghost = NoteId(u64::MAX);
    let before = heard_notes(&song);

    let refusals = [
        (
            song.move_notes(&[note.id, ghost], 96),
            EditError::UnknownNote(ghost),
        ),
        (
            song.move_notes(&[note.id], -(note.tick as i64) - 1),
            EditError::BeforeSongStart { note: note.id },
        ),
        (
            song.set_velocity(&[note.id], 0),
            EditError::VelocityOutOfRange { velocity: 0 },
        ),
        (
            song.set_pitch(&[note.id], 128),
            EditError::KeyOutOfRange { key: 128 },
        ),
        (song.resize_notes(&[note.id], 0), EditError::ZeroDuration),
        (
            song.assign_part(&[note.id], 0),
            EditError::TrackHasNoChannel { track: 0 },
        ),
        (
            song.assign_part(&[note.id], 3),
            EditError::TrackOutOfRange {
                track: 3,
                track_count: 3,
            },
        ),
    ];
    let added = song.add_note(
        0,
        NewNote {
            tick: 0,
            key: 60,
            velocity: 64,
            duration_ticks: 1,
        },
    );

    for (result, expected) in refusals {
        assert_eq!(result, Err(expected));
    }
    assert_eq!(added, Err(EditError::TrackHasNoChannel { track: 0 }));
    assert_eq!(
        song.set_tempo(TempoChange {
            tick: 0,
            microseconds_per_quarter: 0
        })
        .map(|_| ()),
        Err(EditError::TempoOutOfRange {
            microseconds_per_quarter: 0
        })
    );
    assert!(matches!(
        song.set_time_signature(TimeSignature::new(0, 3, 6)),
        Err(EditError::InvalidTimeSignature { .. })
    ));
    assert_eq!(heard_notes(&song), before);
    assert_eq!(song.note_image(note.id), Some(note));
}

#[test]
fn note_ids_survive_edits_and_are_never_reused() {
    let (mut song, _) = read(BACH_INVENTION);
    let removed = first_note(&song, 1);
    let neighbour = first_note(&song, 2);

    song.remove_notes(&[removed.id]).expect("removes");
    let added = song
        .add_note(
            1,
            NewNote {
                tick: 0,
                key: 60,
                velocity: 64,
                duration_ticks: 96,
            },
        )
        .expect("adds");
    song.move_notes(&[neighbour.id], 384).expect("moves");

    assert!(added > removed.id && added.number() >= song.note_count() as u64);
    assert_eq!(song.note_image(removed.id), None);
    assert_eq!(
        song.note_image(neighbour.id).map(|image| image.tick),
        Some(neighbour.tick + 384)
    );
}

#[test]
fn removing_the_tempo_and_meter_returns_them_for_undo() {
    let (mut song, _) = read(BACH_INVENTION);
    let tempo_before = song.tempo_changes().to_vec();

    let removed_tempo = song.remove_tempo(0);
    let removed_meter = song.remove_time_signature(0);
    let replaced = removed_tempo.map(|change| song.set_tempo(change));

    assert_eq!(
        removed_tempo,
        Some(TempoChange {
            tick: 0,
            microseconds_per_quarter: 937_500
        })
    );
    assert_eq!(
        removed_meter.map(|signature| (
            signature.numerator,
            signature.denominator,
            signature.clocks_per_click
        )),
        Some((4, 4, 18))
    );
    assert_eq!(
        replaced,
        Some(Ok(None)),
        "nothing was on tick 0 after the removal"
    );
    assert_eq!(song.tempo_changes(), tempo_before.as_slice());
    assert_eq!(song.remove_tempo(5), None);
}
