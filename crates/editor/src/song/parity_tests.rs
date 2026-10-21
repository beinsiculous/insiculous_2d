//! GUI and command API are one mutation path: for one row per [`SongEdit`] variant, the
//! composer's call (`SongDocument::apply`) and a `song edit <json>` line leave the same
//! song, the same history, and undo back to the same song. A variant without a row does
//! not compile ([`variant_index`] matches exhaustively); a song the composer cannot edit is
//! refused by both paths with the report's reason.

use ecs::World;
use music_midi::midly::num::{u15, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::{Binding, Instrument, InstrumentSource, Part, PartMode, Quantize, TrackSelector};

use super::test_support::{exported, melody, note_numbers};
use super::{default_song_file, Gesture, Meter, SongDocument, SongEdit};
use crate::command_api::write::{self, WriteCtx};
use crate::command_api::{parse_line, ApiError, Request, WriteCmd};
use crate::commands::CommandHistory;
use crate::play_state::EditorPlayState;
use crate::selection::Selection;

const VARIANT_COUNT: usize = 20;

/// Every variant's place in the table. Exhaustive on purpose: a new variant fails to
/// compile here until it has a row.
fn variant_index(edit: &SongEdit) -> usize {
    match edit {
        SongEdit::AddNote { .. } => 0,
        SongEdit::DeleteNotes { .. } => 1,
        SongEdit::MoveNotes { .. } => 2,
        SongEdit::ResizeNotes { .. } => 3,
        SongEdit::SetVelocity { .. } => 4,
        SongEdit::SetPitch { .. } => 5,
        SongEdit::AssignPart { .. } => 6,
        SongEdit::SetTempo { .. } => 7,
        SongEdit::SetTimeSignature { .. } => 8,
        SongEdit::SetLoop { .. } => 9,
        SongEdit::SetQuantize { .. } => 10,
        SongEdit::SetPartMode { .. } => 11,
        SongEdit::AddInstrument { .. } => 12,
        SongEdit::SetInstrument { .. } => 13,
        SongEdit::RemoveInstrument { .. } => 14,
        SongEdit::SetBinding { .. } => 15,
        SongEdit::RemoveBinding { .. } => 16,
        SongEdit::AddPart { .. } => 17,
        SongEdit::RemovePart { .. } => 18,
        SongEdit::Batch(_) => 19,
    }
}

fn organ(name: &str, preset: u8) -> Instrument {
    Instrument {
        name: name.to_string(),
        source: InstrumentSource::SoundFont {
            path: "soundfonts/cast.sf2".to_string(),
            bank: 0,
            preset,
        },
        provenance: None,
    }
}

fn part(name: &str) -> Part {
    Part {
        name: name.to_string(),
        track: TrackSelector::Index(1),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    }
}

fn binding(action: &str) -> Binding {
    Binding {
        action: action.to_string(),
        part: "lead".to_string(),
        instrument: "organ".to_string(),
    }
}

/// The melody with a part, two instruments and a binding, built through `apply` itself.
fn fixture() -> SongDocument {
    let mut document = melody();
    for edit in [
        SongEdit::AddPart { part: part("lead") },
        SongEdit::AddPart {
            part: part("spare part"),
        },
        SongEdit::AddInstrument {
            instrument: organ("organ", 19),
        },
        SongEdit::AddInstrument {
            instrument: organ("spare", 0),
        },
        SongEdit::SetBinding {
            binding: binding("jump"),
        },
    ] {
        document
            .apply(edit, Gesture::Discrete)
            .expect("the fixture builds");
    }
    document
}

fn rows(first_note: u64) -> Vec<SongEdit> {
    let one = vec![first_note];
    vec![
        SongEdit::AddNote {
            track: 1,
            tick: 1440,
            key: 72,
            velocity: 80,
            duration_ticks: 120,
        },
        SongEdit::DeleteNotes { notes: one.clone() },
        SongEdit::MoveNotes {
            notes: one.clone(),
            tick_delta: 120,
        },
        SongEdit::ResizeNotes {
            notes: one.clone(),
            duration_ticks: 120,
        },
        SongEdit::SetVelocity {
            notes: one.clone(),
            velocity: 90,
        },
        SongEdit::SetPitch {
            notes: one.clone(),
            key: 62,
        },
        SongEdit::AssignPart {
            notes: one.clone(),
            part: "lead".to_string(),
        },
        SongEdit::SetTempo {
            tick: 0,
            microseconds_per_quarter: Some(400_000),
        },
        SongEdit::SetTimeSignature {
            tick: 1920,
            meter: Some(Meter {
                numerator: 3,
                denominator: 4,
            }),
        },
        SongEdit::SetLoop { looping: false },
        SongEdit::SetQuantize {
            part: "lead".to_string(),
            quantize: Quantize::Beat,
        },
        SongEdit::SetPartMode {
            part: "lead".to_string(),
            mode: PartMode::Sequence,
        },
        SongEdit::AddInstrument {
            instrument: organ("piano", 0),
        },
        SongEdit::SetInstrument {
            name: "organ".to_string(),
            instrument: organ("organ", 5),
        },
        SongEdit::RemoveInstrument {
            name: "spare".to_string(),
        },
        SongEdit::SetBinding {
            binding: binding("fall"),
        },
        SongEdit::RemoveBinding {
            action: "jump".to_string(),
        },
        SongEdit::AddPart { part: part("bass") },
        SongEdit::RemovePart {
            name: "spare part".to_string(),
        },
        SongEdit::Batch(vec![
            SongEdit::SetLoop { looping: false },
            SongEdit::SetVelocity {
                notes: one,
                velocity: 30,
            },
        ]),
    ]
}

/// The API's half: the write path the editor's `--api` transport runs, on a world and a
/// scene history of its own.
struct ApiRig {
    world: World,
    history: CommandHistory,
    selection: Selection,
    song: Option<SongDocument>,
}

impl ApiRig {
    fn with(song: SongDocument) -> Self {
        Self {
            world: World::new(),
            history: CommandHistory::new(),
            selection: Selection::new(),
            song: Some(song),
        }
    }

    fn run(&mut self, line: &str) -> Result<serde_json::Value, ApiError> {
        let Request::Write(WriteCmd::Pure(pure)) = parse_line(line)? else {
            panic!("a pure write: {line}");
        };
        let mut batch = None;
        let mut ctx = WriteCtx {
            world: &mut self.world,
            history: &mut self.history,
            selection: &mut self.selection,
            play_state: EditorPlayState::Editing,
            batch: &mut batch,
            texture_known: &|_| true,
            song: &mut self.song,
        };
        write::run(&pure, &mut ctx)
    }

    fn song(&self) -> &SongDocument {
        self.song.as_ref().expect("the rig holds a song")
    }
}

/// What a song plays: its timing and every note under its id. An undone tempo or meter
/// edit restores these exactly, though `SongData` then writes its timing events in its own
/// order, so the bytes are compared only between the two paths.
fn content(document: &SongDocument) -> String {
    let data = document.data();
    let notes: Vec<_> = data
        .tracks()
        .iter()
        .flat_map(|track| track.units())
        .flat_map(|unit| unit.notes.iter().map(move |note| (unit.tick, *note)))
        .collect();
    format!(
        "{:?} {:?} {notes:?}",
        data.tempo_changes(),
        data.time_signatures()
    )
}

#[test]
fn every_variant_lands_the_same_through_the_gui_and_the_api_and_undoes_the_same() {
    let pristine = fixture();
    let rows = rows(note_numbers(&pristine)[0]);
    let mut covered = [false; VARIANT_COUNT];
    for row in rows {
        covered[variant_index(&row)] = true;
        let mut gui = fixture();
        gui.apply(row.clone(), Gesture::Discrete)
            .unwrap_or_else(|error| panic!("{}: the GUI path refused: {error}", row.name()));
        let mut api = ApiRig::with(fixture());
        let line = format!(
            "song edit {}",
            serde_json::to_string(&row).expect("serializes")
        );
        api.run(&line)
            .unwrap_or_else(|error| panic!("{}: the API path refused: {error:?}", row.name()));

        let name = row.name();
        assert_eq!(api.song().file(), gui.file(), "{name}: the same sidecar");
        assert_eq!(
            exported(api.song()),
            exported(&gui),
            "{name}: the same notes"
        );
        assert_eq!(
            api.song().history().undo_count(),
            gui.history().undo_count(),
            "{name}: one entry each"
        );
        assert!(
            !api.history.can_undo(),
            "{name}: the scene history never moves"
        );

        gui.undo().expect("the GUI undoes");
        api.run("song undo").expect("the API undoes");
        for (path, document) in [("GUI", &gui), ("API", api.song())] {
            assert_eq!(
                document.file(),
                pristine.file(),
                "{name}: the {path}'s undo restores the sidecar"
            );
            assert_eq!(
                content(document),
                content(&pristine),
                "{name}: and the music"
            );
        }
    }
    assert_eq!(covered, [true; VARIANT_COUNT], "a row for every variant");
}

/// A type-1 file whose one track plays notes on two channels: the editing model has no
/// single channel to write that track's edits on.
fn mixed_channel_song() -> SongDocument {
    let event = |delta, kind| TrackEvent {
        delta: u28::new(delta),
        kind,
    };
    let note = |channel, key, on| TrackEventKind::Midi {
        channel: u4::new(channel),
        message: if on {
            MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(100),
            }
        } else {
            MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            }
        },
    };
    let track = vec![
        event(0, note(0, 60, true)),
        event(240, note(0, 60, false)),
        event(0, note(1, 64, true)),
        event(240, note(1, 64, false)),
        event(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))),
        tracks: vec![track],
    }
    .write_std(&mut bytes)
    .expect("writes");
    SongDocument::from_smf(&bytes, default_song_file()).expect("reads")
}

#[test]
fn a_song_the_composer_cannot_edit_is_refused_by_both_paths_with_the_reports_reason() {
    let gui_song = mixed_channel_song();
    let super::EditingPolicy::Refused { reason } = gui_song.editing().clone() else {
        panic!("a mixed-channel track is not editable");
    };
    let edit = SongEdit::SetLoop { looping: false };

    let mut gui = gui_song.clone();
    let gui_error = gui
        .apply(edit.clone(), Gesture::Discrete)
        .expect_err("refused");
    assert!(gui_error.to_string().contains(&reason), "{gui_error}");

    let mut api = ApiRig::with(gui_song);
    let line = format!(
        "song edit {}",
        serde_json::to_string(&edit).expect("serializes")
    );
    match api.run(&line) {
        Err(ApiError::Refused(message)) => assert!(message.contains(&reason), "{message}"),
        other => panic!("the API path must refuse with the reason: {other:?}"),
    }
    assert_eq!(api.song().history().undo_count(), 0);
}
