//! The edits that reach past one block of the map: the SoundFont the background plays on,
//! and a part whose track name the score carries twice.

use music_midi::midly::num::{u15, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::{Part, PartMode, Quantize, TrackSelector};

use super::test_support::melody;
use super::{default_song_file, Gesture, SongDocument, SongEdit, SongEditError};

/// Apply `edit` expecting a refusal that changed neither the map nor its history.
fn refused(document: &mut SongDocument, edit: SongEdit) -> SongEditError {
    let (file, entries) = (document.file().clone(), document.history().undo_count());
    let error = document.apply(edit, Gesture::Discrete).expect_err("refused");
    assert_eq!(document.file(), &file, "a refused edit leaves the map alone");
    assert_eq!(document.history().undo_count(), entries, "and records nothing");
    error
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

    assert_eq!(document.undo(), Some("SetSoundFont"));
    assert_eq!(document.file().soundfont, original);
}

#[test]
fn a_part_on_a_name_two_tracks_carry_is_refused_and_by_index_it_lands() {
    let mut document = doubled_melody_song();
    let doubled = |track| Part {
        name: "doubled".to_string(),
        track,
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    };
    let error = refused(
        &mut document,
        SongEdit::AddPart {
            part: doubled(TrackSelector::Name("melody".to_string())),
        },
    );
    assert_eq!(
        error,
        SongEditError::TrackNameAmbiguous { part: "doubled".to_string(), name: "melody".to_string() }
    );
    assert_eq!(
        error.to_string(),
        "part \"doubled\" selects the track named \"melody\", which two tracks carry; select one by index"
    );
    document
        .apply(SongEdit::AddPart { part: doubled(TrackSelector::Index(0)) }, Gesture::Discrete)
        .expect("by index it lands");
}

/// A type-1 file whose two note tracks both carry the name "melody", part `a` on that name
/// and part `b` on the second track by index: a song the editor meets only by import, since
/// the score is MIDI Evil's to write.
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
