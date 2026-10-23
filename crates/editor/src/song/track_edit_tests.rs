//! The two edits that reach past the notes: a track added to the song, and the SoundFont its
//! background plays on.

use music_midi::midly::num::{u15, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::{EditError, Part, PartMode, Quantize, TrackSelector};

use super::test_support::{exported, melody};
use super::{default_song_file, Gesture, SongDocument, SongEdit, SongEditError};

fn add_track(channel: Option<u8>) -> SongEdit {
    SongEdit::AddTrack {
        name: "brass".to_string(),
        channel,
        program: 57,
    }
}

/// Apply `edit` expecting a refusal that changed neither the song nor its history.
fn refused(document: &mut SongDocument, edit: SongEdit) -> SongEditError {
    let (midi, entries) = (exported(document), document.history().undo_count());
    let error = document.apply(edit, Gesture::Discrete).expect_err("refused");
    assert_eq!(exported(document), midi, "a refused edit leaves the song alone");
    assert_eq!(document.history().undo_count(), entries, "and records nothing");
    error
}

#[test]
fn an_added_track_takes_the_next_free_channel_counted_from_one_and_plays_its_program() {
    let mut document = melody();
    document.apply(add_track(None), Gesture::Discrete).expect("adds");
    document
        .apply(add_track(Some(10)), Gesture::Discrete)
        .expect("percussion when it is named");

    let tracks: Vec<_> = document
        .data()
        .tracks()
        .iter()
        .map(|track| (track.name(), track.channel()))
        .collect();
    assert_eq!(
        tracks[2..],
        [(Some("brass"), Some(1)), (Some("brass"), Some(9))],
        "the melody holds channel 1, so the first free is 2; 10 is stored as 9"
    );
}

#[test]
fn a_channel_outside_one_to_sixteen_or_already_playing_is_refused() {
    let mut document = melody();
    assert_eq!(
        refused(&mut document, add_track(Some(0))),
        SongEditError::ChannelOutOfRange(0)
    );
    assert_eq!(
        refused(&mut document, add_track(Some(17))),
        SongEditError::ChannelOutOfRange(17)
    );
    let error = refused(&mut document, add_track(Some(1)));
    assert_eq!(error, SongEditError::Edit(EditError::ChannelInUse { channel: 0 }));
    assert_eq!(error.to_string(), "channel 1 is already in use");
}

/// A type-0 file: one track, its notes on channels 1 and 2.
fn single_track_song() -> SongDocument {
    let event = |delta, kind| TrackEvent {
        delta: u28::new(delta),
        kind,
    };
    let note = |channel, on| TrackEventKind::Midi {
        channel: u4::new(channel),
        message: if on {
            MidiMessage::NoteOn {
                key: u7::new(60),
                vel: u7::new(100),
            }
        } else {
            MidiMessage::NoteOff {
                key: u7::new(60),
                vel: u7::new(0),
            }
        },
    };
    let track = vec![
        event(0, note(0, true)),
        event(240, note(0, false)),
        event(0, note(1, true)),
        event(240, note(1, false)),
        event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480))),
        tracks: vec![track],
    }
    .write_std(&mut bytes)
    .expect("writes");
    SongDocument::from_smf(&bytes, default_song_file()).expect("reads")
}

#[test]
fn a_single_track_song_refuses_a_track_because_its_channels_are_its_tracks() {
    let mut document = single_track_song();
    let error = refused(&mut document, add_track(None));
    assert_eq!(
        error.to_string(),
        "this song is a single-track MIDI file; its channels are its tracks"
    );
}

#[test]
fn a_batch_adding_a_track_its_part_and_a_note_undoes_in_reverse_and_redoes_with_the_same_ids() {
    let mut document = melody();
    let before = exported(&document);
    let part = Part {
        name: "accompaniment".to_string(),
        track: TrackSelector::Name("brass".to_string()),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    };
    document
        .apply(
            SongEdit::Batch(vec![
                add_track(None),
                SongEdit::AddPart { part },
                SongEdit::AddNote {
                    track: 2,
                    tick: 0,
                    key: 53,
                    velocity: 90,
                    duration_ticks: 960,
                },
            ]),
            Gesture::Discrete,
        )
        .expect("the batch lands whole");
    let after = exported(&document);
    let file_after = document.file().clone();
    assert_eq!(document.data().tracks()[2].note_count(), 1);

    document.undo().expect("undoes");
    assert_eq!(exported(&document), before, "the note, the part and the track are gone");
    assert_eq!(document.data().tracks().len(), 2);
    assert!(document.file().parts.is_empty());

    document.redo().expect("redoes");
    assert_eq!(exported(&document), after, "the same track, note and note id");
    assert_eq!(document.file(), &file_after);
}

#[test]
fn the_soundfont_is_one_undoable_edit_and_two_in_a_gesture_merge() {
    let mut document = melody();
    let original = document.file().soundfont.clone();
    let entries = document.history().undo_count();

    for path in ["soundfonts/a.sf2", "soundfonts/b.sf2"] {
        document
            .apply(
                SongEdit::SetSoundFont {
                    path: path.to_string(),
                },
                Gesture::Continuing,
            )
            .expect("sets the bank");
    }
    assert_eq!(document.file().soundfont, "soundfonts/b.sf2");
    assert_eq!(document.history().undo_count(), entries + 1, "one entry");

    document.undo().expect("undoes");
    assert_eq!(document.file().soundfont, original);
}

#[test]
fn a_name_a_part_selects_is_refused_for_a_second_track_and_free_otherwise() {
    let mut document = melody();
    let named_melody = |program| SongEdit::AddTrack {
        name: "melody".to_string(),
        channel: None,
        program,
    };
    document.apply(named_melody(57), Gesture::Discrete).expect("no part selects the name, so a second melody track is the author's business");
    let by_name = |track| Part {
        name: "doubled".to_string(),
        track,
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    };
    assert_eq!(
        refused(&mut document, SongEdit::AddPart { part: by_name(TrackSelector::Name("melody".to_string())) }),
        SongEditError::TrackNameAmbiguous { part: "doubled".to_string(), name: "melody".to_string() },
        "a part added after the duplicate cannot select the shared name"
    );
    document
        .apply(SongEdit::AddPart { part: by_name(TrackSelector::Index(2)) }, Gesture::Discrete)
        .expect("by index it lands");
    document.apply(SongEdit::Batch(vec![add_track(None), SongEdit::AddPart { part: Part {
        name: "accompaniment".to_string(),
        track: TrackSelector::Name("brass".to_string()),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    } }]), Gesture::Discrete).expect("a part on the brass track by name");

    let error = refused(&mut document, add_track(None));
    assert_eq!(error, SongEditError::TrackNameSelected("brass".to_string()));
    assert_eq!(
        error.to_string(),
        "a part selects the track named \"brass\"; a second track of that name would leave it ambiguous"
    );
}

/// A type-1 file whose two note tracks both carry the name "melody", part `a` on that name
/// and part `b` on the second track by index: a song the editor meets only by import, since no
/// sequence of its own edits doubles a selected name.
fn doubled_melody_song() -> SongDocument {
    let event = |delta, kind| TrackEvent {
        delta: u28::new(delta),
        kind,
    };
    let named_track = |channel: u8| {
        vec![
            event(0, TrackEventKind::Meta(MetaMessage::TrackName(b"melody"))),
            event(
                0,
                TrackEventKind::Midi {
                    channel: u4::new(channel),
                    message: MidiMessage::NoteOn {
                        key: u7::new(60),
                        vel: u7::new(100),
                    },
                },
            ),
            event(
                240,
                TrackEventKind::Midi {
                    channel: u4::new(channel),
                    message: MidiMessage::NoteOff {
                        key: u7::new(60),
                        vel: u7::new(0),
                    },
                },
            ),
            event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]
    };
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks: vec![named_track(0), named_track(1)],
    }
    .write_std(&mut bytes)
    .expect("writes");
    let part = |name: &str, track| Part {
        name: name.to_string(),
        track,
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    };
    let mut file = default_song_file();
    file.parts = vec![
        part("a", TrackSelector::Name("melody".to_string())),
        part("b", TrackSelector::Index(1)),
    ];
    SongDocument::from_smf(&bytes, file).expect("reads")
}

#[test]
fn a_part_removed_and_re_added_on_a_doubled_name_in_one_batch_is_refused() {
    let mut document = doubled_melody_song();
    let re_added = Part {
        name: "b".to_string(),
        track: TrackSelector::Name("melody".to_string()),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    };
    let error = refused(
        &mut document,
        SongEdit::Batch(vec![
            SongEdit::RemovePart { name: "b".to_string() },
            SongEdit::AddPart { part: re_added },
        ]),
    );
    assert_eq!(
        error,
        SongEditError::TrackNameAmbiguous { part: "b".to_string(), name: "melody".to_string() },
        "a re-added part has no untouched twin and is judged as new"
    );
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a part that already selected the doubled name is not re-judged by an unrelated edit");
}
