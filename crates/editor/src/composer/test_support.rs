//! Fixtures for the composer's tests: songs of known shapes, opened from MIDI bytes as an
//! exported file would be.

use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::song::{default_song_file, SongDocument};

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

/// A track named `name` on `channel`: a program change, then an eighth note on each of `keys`,
/// one after the other from tick 0, a quarter apart.
fn note_track(name: &'static [u8], channel: u8, keys: &[u8]) -> Vec<TrackEvent<'static>> {
    let mut events = vec![
        event(0, TrackEventKind::Meta(MetaMessage::TrackName(name))),
        event(0, TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::ProgramChange { program: u7::new(0) },
        }),
    ];
    for (index, key) in keys.iter().enumerate() {
        let rest_before = if index == 0 { 0 } else { 240 };
        events.push(event(rest_before, TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::NoteOn { key: u7::new(*key), vel: u7::new(100) },
        }));
        events.push(event(240, TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::NoteOff { key: u7::new(*key), vel: u7::new(0) },
        }));
    }
    events.push(event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)));
    events
}

/// A type-1 song as MIDI bytes at 480 ticks a quarter: the timing track at 120 BPM in 4/4,
/// then each of `tracks`.
fn song_bytes(tracks: Vec<Vec<TrackEvent<'static>>>) -> Vec<u8> {
    let mut all_tracks = vec![tempo_track()];
    all_tracks.extend(tracks);
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks: all_tracks,
    }
    .write_std(&mut bytes)
    .expect("the fixture writes");
    bytes
}

fn song_of(tracks: Vec<Vec<TrackEvent<'static>>>) -> SongDocument {
    SongDocument::from_smf(&song_bytes(tracks), default_song_file()).expect("the fixture reads")
}

/// A song holding three eighths on its melody track (track 1, channel 1): C4 at tick 0, E4
/// at 480 and G4 at 960.
pub(super) fn melody() -> SongDocument {
    song_of(vec![note_track(b"melody", 0, &[60, 64, 67])])
}

/// A type-1 song: the timing track, an empty `pad` track on channel 1, and a `lead` track
/// on channel 2 holding one note — so the first track with a note is not the first with
/// a channel.
pub(super) fn second_track_holds_the_notes() -> SongDocument {
    song_of(vec![note_track(b"pad", 0, &[]), note_track(b"lead", 1, &[72])])
}

/// A type-1 song of one track holding timing alone: no track a note could land on.
pub(super) fn timing_only() -> SongDocument {
    song_of(Vec::new())
}
