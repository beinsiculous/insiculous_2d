//! Adding a track: its channel, its program, what an export writes for it and what a reopen
//! reads back, and its inverse.

use midly::{MidiMessage, TrackEventKind};

use super::*;
use crate::error::EditError;
use crate::test_support::*;

fn trumpet(channel: Option<u8>) -> NewTrack {
    NewTrack {
        name: "brass".to_string(),
        channel,
        program: 56,
    }
}

/// Every program change of the exported file as `(track, channel, program)`.
fn program_changes(bytes: &[u8]) -> Vec<(usize, u8, u8)> {
    let smf = midly::Smf::parse(bytes).expect("the export parses");
    smf.tracks
        .iter()
        .enumerate()
        .flat_map(|(index, track)| {
            track.iter().filter_map(move |event| match event.kind {
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::ProgramChange { program },
                } => Some((index, channel.as_int(), program.as_int())),
                _ => None,
            })
        })
        .collect()
}

#[test]
fn an_added_track_takes_the_next_free_channel_past_percussion_and_writes_its_program() {
    let (mut song, _) = read(BACH_INVENTION);
    let before = song.tracks().len();
    let occupied = song.occupied_channels();

    let added = song.add_track(trumpet(None)).expect("a type-1 song takes a track");

    let expected = (0..=15)
        .find(|channel| *channel != 9 && !occupied.contains(channel))
        .expect("Bach leaves channels free");
    assert_eq!(added, AddedTrack { track: before, channel: expected });
    let track = &song.tracks()[added.track];
    assert_eq!((track.name(), track.channel(), track.note_count()), (Some("brass"), Some(expected), 0));
    let exported = song.write_smf().expect("writes");
    assert!(program_changes(&exported).contains(&(song.retained_tracks().len() - 1, expected, 56)));

    let mut crowded = song.clone();
    for _ in 0..16 {
        if crowded.add_track(trumpet(None)).is_err() {
            break;
        }
    }
    assert!(
        crowded.tracks().iter().all(|track| track.channel() != Some(9)),
        "percussion is never handed out"
    );
    assert_eq!(crowded.add_track(trumpet(None)), Err(EditError::NoFreeChannel));
}

#[test]
fn an_empty_added_track_saved_and_reopened_keeps_its_name_channel_and_program() {
    let (mut song, _) = read(BACH_INVENTION);
    let added = song.add_track(trumpet(Some(5))).expect("channel 6 is free");

    let exported = song.write_smf().expect("writes");
    let (reopened, report) = SongData::from_smf(&exported).expect("reads back");

    assert!(report.is_editable());
    let track = &reopened.tracks()[added.track];
    assert_eq!((track.name(), track.channel()), (Some("brass"), Some(5)));
    assert_eq!(program_changes(&exported).last(), Some(&(added.track, 5, 56)));
}

#[test]
fn notes_on_an_added_track_export_and_reimport_on_its_channel() {
    let (mut song, _) = read(BACH_INVENTION);
    let added = song.add_track(trumpet(None)).expect("adds");
    let chord = [60, 64, 67].map(|key| NewNote {
        tick: 384,
        key,
        velocity: 90,
        duration_ticks: 192,
    });
    for note in chord {
        song.add_note(added.track, note).expect("the added track is writable");
    }

    let (reopened, _) = SongData::from_smf(&song.write_smf().expect("writes")).expect("reads");

    let track = &reopened.tracks()[added.track];
    assert_eq!(track.channel(), Some(added.channel));
    assert_eq!(track.units().len(), 1);
    let keys: Vec<u8> = track.units()[0].notes.iter().map(|note| note.key).collect();
    assert_eq!(keys, vec![60, 64, 67]);
}

#[test]
fn a_channel_already_used_out_of_range_or_a_bad_program_is_refused_and_changes_nothing() {
    let (mut song, _) = read(BACH_INVENTION);
    let used = song.tracks()[1].channel().expect("Bach's first voice has a channel");
    let tracks = song.tracks().len();

    assert_eq!(
        song.add_track(trumpet(Some(used))),
        Err(EditError::ChannelInUse { channel: used })
    );
    assert_eq!(song.add_track(trumpet(Some(16))), Err(EditError::ChannelOutOfRange { channel: 16 }));
    let loud = NewTrack { program: 128, ..trumpet(None) };
    assert_eq!(song.add_track(loud), Err(EditError::ProgramOutOfRange { program: 128 }));
    assert_eq!(song.tracks().len(), tracks);
    assert_eq!(song.retained_tracks().len(), tracks);

    let added = song.add_track(trumpet(None)).expect("adds");
    assert_eq!(
        song.add_track(trumpet(Some(added.channel))),
        Err(EditError::ChannelInUse { channel: added.channel }),
        "an empty added track holds its channel by its program change"
    );
}

#[test]
fn a_single_track_song_refuses_a_track_because_its_channels_are_its_tracks() {
    let (mut song, _) = read(NAPOLITAN_DANCE_TYPE_ZERO);
    let tracks = song.tracks().len();

    assert_eq!(song.add_track(trumpet(None)), Err(EditError::SingleTrackSong));
    assert_eq!(song.tracks().len(), tracks);
}

#[test]
fn taking_an_added_track_away_removes_it_with_its_notes_and_only_the_last_can_go() {
    let (mut song, _) = read(BACH_INVENTION);
    let original = song.write_smf().expect("writes");
    let imported_last = song.tracks().len() - 1;
    assert_eq!(
        song.remove_added_track(imported_last),
        Err(EditError::NotTheLastTrack { track: imported_last }),
        "an imported file's last track was never added"
    );
    let first = song.add_track(trumpet(None)).expect("adds");
    let second = song.add_track(trumpet(None)).expect("adds another");
    let note = NewNote { tick: 0, key: 72, velocity: 80, duration_ticks: 96 };
    let id = song.add_note(second.track, note).expect("adds a note");

    assert_eq!(
        song.remove_added_track(first.track),
        Err(EditError::NotTheLastTrack { track: first.track })
    );
    assert_eq!(
        song.remove_added_track(1),
        Err(EditError::NotTheLastTrack { track: 1 }),
        "a file's own track is never taken away"
    );
    let images = song.remove_added_track(second.track).expect("the last added goes");
    assert_eq!(images.iter().map(|image| image.id).collect::<Vec<_>>(), vec![id]);
    assert_eq!(song.note_image(id), None);
    song.remove_added_track(first.track).expect("then the one before it");
    assert_eq!(
        song.remove_added_track(imported_last),
        Err(EditError::NotTheLastTrack { track: imported_last }),
        "and with every added track gone, the file's own last track still stays"
    );

    assert_eq!(song.write_smf().expect("writes"), original, "back to the file it was");
}
