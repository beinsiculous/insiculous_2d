//! Test-only: the conductor's sine fixtures, a hand-built song in a temporary asset
//! directory, and the trigger source a keyboard would be.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::SourceId;

pub(crate) const SINE_FONT: &[u8] = include_bytes!("../../music_midi/fixtures/sine.sf2");
pub(crate) const SINE_WAV: &[u8] = include_bytes!("../../music_midi/fixtures/sine_48000_mono.wav");
pub(crate) const OTHER_SINE_WAV: &[u8] =
    include_bytes!("../../music_midi/fixtures/sine_22050_mono.wav");

/// Ticks per quarter note of every song built here.
pub(crate) const QUARTER: u32 = 384;
/// 600 BPM: ten beats a second, so a short step of the clock crosses several.
pub(crate) const TEN_BEATS_A_SECOND: u32 = 100_000;
/// Beats in the song before it loops: four bars of 4/4.
pub(crate) const SONG_BEATS: u32 = 16;

/// The bank, the sample and the song's sidecar name these, relative to the asset base.
pub(crate) const FONT_PATH: &str = "sine.sf2";
pub(crate) const SAMPLE_PATH: &str = "samples/sine.wav";

/// A source that never leaves, activated at epoch 1.
pub(crate) fn keyboard() -> SourceId {
    source(1, 1)
}

pub(crate) fn source(id: u64, epoch: u32) -> SourceId {
    SourceId {
        id,
        generation: 0,
        epoch,
    }
}

fn note(channel: u8, key: u8, on: bool) -> TrackEventKind<'static> {
    let (key, vel) = (u7::new(key), u7::new(if on { 100 } else { 0 }));
    TrackEventKind::Midi {
        channel: u4::new(channel),
        message: if on {
            MidiMessage::NoteOn { key, vel }
        } else {
            MidiMessage::NoteOff { key, vel }
        },
    }
}

/// Sixteen beats of 4/4 at ten beats a second: a control track, a `melody` track of
/// quarter notes for triggers to walk, and — with `background` — a `pad` track holding
/// one low note for the background to play.
pub(crate) fn song_midi(background: bool) -> Vec<u8> {
    let end_tick = SONG_BEATS * QUARTER;
    let control = vec![
        (
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(TEN_BEATS_A_SECOND))),
        ),
        (
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        ),
        (
            end_tick,
            TrackEventKind::Midi {
                channel: u4::new(15),
                message: MidiMessage::Controller {
                    controller: u7::new(7),
                    value: u7::new(100),
                },
            },
        ),
    ];
    let mut melody = vec![(0, TrackEventKind::Meta(MetaMessage::TrackName(b"melody")))];
    for key in [60, 62, 64, 65]
        .into_iter()
        .cycle()
        .take(SONG_BEATS as usize)
    {
        melody.push((0, note(0, key, true)));
        melody.push((QUARTER, note(0, key, false)));
    }
    let mut tracks = vec![control, melody];
    if background {
        tracks.push(vec![
            (0, TrackEventKind::Meta(MetaMessage::TrackName(b"pad"))),
            (0, note(1, 48, true)),
            (end_tick - 1, note(1, 48, false)),
        ]);
    }
    write_smf(&tracks)
}

fn write_smf(tracks: &[Vec<(u32, TrackEventKind<'static>)>]) -> Vec<u8> {
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
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(QUARTER as u16))),
        tracks,
    }
    .write_std(&mut bytes)
    .expect("a hand-built SMF writes");
    bytes
}

/// The sidecar of [`song_midi`]: the melody part, the sine preset (`tone`) and the sine
/// sample (`blip`) bound to it, and `extra_instrument` with its binding when given.
pub(crate) fn sidecar(extra_instrument: Option<(&str, &str)>) -> String {
    let (instrument, binding) = extra_instrument.unwrap_or_default();
    format!(
        r#"SongFile(
            version: 1,
            soundfont: "{FONT_PATH}",
            parts: [(name: "melody", track: Name("melody"))],
            instruments: [
                (name: "sine", source: SoundFont(path: "{FONT_PATH}", bank: 0, preset: 0)),
                (name: "wav", source: Sample(path: "{SAMPLE_PATH}", root_note: 60, gain: 1.0, playback: OneShot)),
                {instrument}
            ],
            bindings: [
                (action: "tone", part: "melody", instrument: "sine"),
                (action: "blip", part: "melody", instrument: "wav"),
                {binding}
            ],
        )"#
    )
}

/// A temporary asset directory holding the sine bank and sample; removed when dropped.
pub(crate) struct AssetDirectory(PathBuf);

impl AssetDirectory {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "insiculous_song_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let directory = Self(path);
        directory.write(FONT_PATH, SINE_FONT);
        directory.write(SAMPLE_PATH, SINE_WAV);
        directory
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// Write `bytes` at `relative`, directories included.
    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(relative);
        common::vfs::write(&path, bytes).expect("the temporary directory is writable");
        path
    }

    /// A song `<stem>.mid` with its sidecar `<stem>.mid.ron`; the MIDI file's path.
    pub(crate) fn write_song(&self, stem: &str, midi: &[u8], sidecar: &str) -> PathBuf {
        self.write(&format!("{stem}.mid.ron"), sidecar.as_bytes());
        self.write(&format!("{stem}.mid"), midi)
    }
}

impl Drop for AssetDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}
