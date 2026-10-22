//! Fixtures for the composer's tests: songs of known shapes, built from MIDI bytes where
//! the shape is one no edit can make (a second note track, a file of timing alone).

use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::song::{default_song_file, Gesture, SongDocument, SongEdit};

/// A new song holding three eighths on its melody track (track 1): C4 at tick 0, E4 at
/// 480 and G4 at 960.
pub(super) fn melody() -> SongDocument {
    let mut song = SongDocument::new_song().expect("a blank song");
    for (tick, key) in [(0, 60), (480, 64), (960, 67)] {
        song.apply(add_note(1, tick, key), Gesture::Discrete).expect("the note lands");
    }
    song
}

pub(super) fn add_note(track: usize, tick: u64, key: u8) -> SongEdit {
    SongEdit::AddNote {
        track,
        tick,
        key,
        velocity: 100,
        duration_ticks: 240,
    }
}

fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent { delta: u28::new(delta), kind }
}

fn tempo_track() -> Vec<TrackEvent<'static>> {
    vec![
        event(0, TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000)))),
        event(0, TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8))),
        event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ]
}

/// A track named `name` on `channel`: a program change, then one note when `key` is some.
fn note_track(name: &'static [u8], channel: u8, key: Option<u8>) -> Vec<TrackEvent<'static>> {
    let mut events = vec![
        event(0, TrackEventKind::Meta(MetaMessage::TrackName(name))),
        event(0, TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::ProgramChange { program: u7::new(0) },
        }),
    ];
    if let Some(key) = key {
        events.push(event(0, TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::NoteOn { key: u7::new(key), vel: u7::new(90) },
        }));
        events.push(event(480, TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::NoteOff { key: u7::new(key), vel: u7::new(0) },
        }));
    }
    events.push(event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)));
    events
}

fn song_of(tracks: Vec<Vec<TrackEvent<'static>>>) -> SongDocument {
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks,
    }
    .write_std(&mut bytes)
    .expect("the fixture writes");
    SongDocument::from_smf(&bytes, default_song_file()).expect("the fixture reads")
}

/// A type-1 song: the timing track, an empty `pad` track on channel 1, and a `lead` track
/// on channel 2 holding one note — so the first track with a note is not the first with
/// a channel.
pub(super) fn second_track_holds_the_notes() -> SongDocument {
    song_of(vec![
        tempo_track(),
        note_track(b"pad", 0, None),
        note_track(b"lead", 1, Some(72)),
    ])
}

/// A type-1 song of one track holding timing alone: no track a note could land on.
pub(super) fn timing_only() -> SongDocument {
    song_of(vec![tempo_track()])
}
