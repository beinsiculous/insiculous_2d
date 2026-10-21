//! Fixtures, a hand-built-SMF helper and the sine WAVs as decoded audio, shared by the
//! crate's tests.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::pcm::OwnedPcm;
use crate::song_data::{SongData, UnsupportedReport};

pub const BACH_INVENTION: &[u8] = include_bytes!("../fixtures/bach-invention-02.mid");
pub const NAPOLITAN_DANCE: &[u8] = include_bytes!("../fixtures/dansenapolitaine.mid");
pub const NAPOLITAN_DANCE_TYPE_ZERO: &[u8] =
    include_bytes!("../fixtures/dansenapolitaine-type0.mid");
pub const NAPOLITAN_DANCE_CAST: &[u8] = include_bytes!("../fixtures/dansenapolitaine-cast.mid");
pub const BAYADERE: &[u8] = include_bytes!("../fixtures/bayadere-ombres.mid");
pub const EGMONT: &[u8] = include_bytes!("../fixtures/Egmont.mid");

pub const ALL_FIXTURES: [(&str, &[u8]); 6] = [
    ("bach-invention-02.mid", BACH_INVENTION),
    ("dansenapolitaine.mid", NAPOLITAN_DANCE),
    ("dansenapolitaine-type0.mid", NAPOLITAN_DANCE_TYPE_ZERO),
    ("dansenapolitaine-cast.mid", NAPOLITAN_DANCE_CAST),
    ("bayadere-ombres.mid", BAYADERE),
    ("Egmont.mid", EGMONT),
];

/// One second of middle C (261.63 Hz), written by `scripts/make_sine_wav.py`: 16-bit PCM
/// with a 5 ms fade at each end; a stereo file's right channel is half its left.
pub const SINE_48000_MONO: &[u8] = include_bytes!("../fixtures/sine_48000_mono.wav");
pub const SINE_48000_STEREO: &[u8] = include_bytes!("../fixtures/sine_48000_stereo.wav");
pub const SINE_22050_MONO: &[u8] = include_bytes!("../fixtures/sine_22050_mono.wav");
pub const SINE_22050_STEREO: &[u8] = include_bytes!("../fixtures/sine_22050_stereo.wav");

/// The key the sine WAVs play unpitched, and its frequency.
#[cfg(feature = "synth")]
pub const SINE_WAV_ROOT_NOTE: u8 = 60;
#[cfg(feature = "synth")]
pub const SINE_WAV_HERTZ: f64 = 261.625_565_300_598_6;

/// A sine fixture as the audio crate's loader would hand it over. The crate itself never
/// decodes: this reads only the generator's own layout (a `fmt ` chunk, then 16-bit PCM in
/// a `data` chunk), and only in tests.
pub fn fixture_pcm(wav: &[u8]) -> OwnedPcm {
    let little_endian_u16 = |at: usize| u16::from_le_bytes([wav[at], wav[at + 1]]);
    let little_endian_u32 =
        |at: usize| u32::from_le_bytes([wav[at], wav[at + 1], wav[at + 2], wav[at + 3]]);
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    let (mut channels, mut sample_rate, mut samples) = (0, 0, Vec::new());
    let mut chunk = 12;
    while chunk + 8 <= wav.len() {
        let body = chunk + 8;
        let length = little_endian_u32(chunk + 4) as usize;
        match &wav[chunk..chunk + 4] {
            b"fmt " => {
                channels = little_endian_u16(body + 2);
                sample_rate = little_endian_u32(body + 4);
                assert_eq!(little_endian_u16(body + 14), 16, "16-bit PCM");
            }
            b"data" => {
                samples = wav[body..body + length]
                    .chunks_exact(2)
                    .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32_768.0)
                    .collect();
            }
            _ => {}
        }
        chunk = body + length + length % 2;
    }
    OwnedPcm::new(sample_rate, channels, samples).expect("a sine fixture is within the limits")
}

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
