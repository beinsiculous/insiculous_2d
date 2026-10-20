//! Fixtures and a hand-built-SMF helper shared by the crate's tests.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::song_data::{SongData, UnsupportedReport};

pub const BACH_INVENTION: &[u8] = include_bytes!("../fixtures/bach-invention-02.mid");
pub const NAPOLITAN_DANCE: &[u8] = include_bytes!("../fixtures/dansenapolitaine.mid");
pub const NAPOLITAN_DANCE_TYPE_ZERO: &[u8] =
    include_bytes!("../fixtures/dansenapolitaine-type0.mid");
pub const BAYADERE: &[u8] = include_bytes!("../fixtures/bayadere-ombres.mid");
pub const EGMONT: &[u8] = include_bytes!("../fixtures/Egmont.mid");

pub const ALL_FIXTURES: [(&str, &[u8]); 5] = [
    ("bach-invention-02.mid", BACH_INVENTION),
    ("dansenapolitaine.mid", NAPOLITAN_DANCE),
    ("dansenapolitaine-type0.mid", NAPOLITAN_DANCE_TYPE_ZERO),
    ("bayadere-ombres.mid", BAYADERE),
    ("Egmont.mid", EGMONT),
];

pub fn read(bytes: &[u8]) -> (SongData, UnsupportedReport) {
    SongData::from_smf(bytes).expect("fixture reads")
}

/// One event of a hand-built track: the ticks since the previous event, and the event.
pub type Step = (u32, TrackEventKind<'static>);

pub fn note_on(channel: u8, key: u8, velocity: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: MidiMessage::NoteOn {
            key: u7::new(key),
            vel: u7::new(velocity),
        },
    }
}

pub fn note_off(channel: u8, key: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: MidiMessage::NoteOff {
            key: u7::new(key),
            vel: u7::new(0),
        },
    }
}

pub fn program_change(channel: u8, program: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: MidiMessage::ProgramChange {
            program: u7::new(program),
        },
    }
}

pub fn controller(channel: u8, controller: u8, value: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: MidiMessage::Controller {
            controller: u7::new(controller),
            value: u7::new(value),
        },
    }
}

pub fn tempo(microseconds_per_quarter: u32) -> TrackEventKind<'static> {
    TrackEventKind::Meta(MetaMessage::Tempo(u24::new(microseconds_per_quarter)))
}

pub fn track_name(name: &'static str) -> TrackEventKind<'static> {
    TrackEventKind::Meta(MetaMessage::TrackName(name.as_bytes()))
}

/// A Standard MIDI File of the given layout and timing; each track gets its end-of-track
/// event appended.
pub fn smf_with_timing(format: Format, timing: Timing, tracks: &[Vec<Step>]) -> Vec<u8> {
    let tracks = tracks
        .iter()
        .map(|steps| {
            let mut events: Vec<TrackEvent<'static>> = steps
                .iter()
                .map(|(delta, kind)| TrackEvent {
                    delta: u28::new(*delta),
                    kind: *kind,
                })
                .collect();
            events.push(TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
            events
        })
        .collect();
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(format, timing),
        tracks,
    }
    .write_std(&mut bytes)
    .expect("hand-built SMF writes");
    bytes
}

/// A type-1 file at 384 ticks per quarter note.
pub fn smf(tracks: &[Vec<Step>]) -> Vec<u8> {
    smf_with_timing(Format::Parallel, Timing::Metrical(u15::new(384)), tracks)
}
