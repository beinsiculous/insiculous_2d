//! GUI and command API are one mutation path: for one row per [`SongEdit`] variant, the
//! composer's call (`SongDocument::apply`) and a `song edit <json>` line leave the same
//! song, the same history, and undo back to the same song. A variant without a row does
//! not compile ([`variant_index`] matches exhaustively).

use ecs::World;
use music_midi::{
    Binding, Instrument, InstrumentSource, Part, PartMode, Provenance, Quantize, TrackSelector,
};

use super::test_support::melody;
use super::{Gesture, SongDocument, SongEdit};
use crate::command_api::write::{self, WriteCtx};
use crate::command_api::{parse_line, ApiError, Request, WriteCmd};
use crate::commands::CommandHistory;
use crate::play_state::EditorPlayState;
use crate::selection::Selection;

const VARIANT_COUNT: usize = 13;

/// Every variant's place in the table. Exhaustive on purpose: a new variant fails to
/// compile here until it has a row.
fn variant_index(edit: &SongEdit) -> usize {
    match edit {
        SongEdit::SetLoop { .. } => 0,
        SongEdit::SetQuantize { .. } => 1,
        SongEdit::SetPartMode { .. } => 2,
        SongEdit::AddInstrument { .. } => 3,
        SongEdit::SetInstrument { .. } => 4,
        SongEdit::RemoveInstrument { .. } => 5,
        SongEdit::SetBinding { .. } => 6,
        SongEdit::RemoveBinding { .. } => 7,
        SongEdit::AddPart { .. } => 8,
        SongEdit::RemovePart { .. } => 9,
        SongEdit::SetProvenance { .. } => 10,
        SongEdit::Batch(_) => 11,
        SongEdit::SetSoundFont { .. } => 12,
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

fn rows() -> Vec<SongEdit> {
    vec![
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
        SongEdit::SetProvenance {
            provenance: Provenance {
                author: "2026 M".to_string(),
                license: "CC-BY-4.0".to_string(),
            },
        },
        SongEdit::SetSoundFont {
            path: "soundfonts/tong.sf2".to_string(),
        },
        SongEdit::Batch(vec![
            SongEdit::SetLoop { looping: false },
            SongEdit::SetQuantize {
                part: "lead".to_string(),
                quantize: Quantize::Eighth,
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

#[test]
fn every_variant_lands_the_same_through_the_gui_and_the_api_and_undoes_the_same() {
    let pristine = fixture();
    let mut covered = [false; VARIANT_COUNT];
    for row in rows() {
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
            api.song().source_bytes(),
            pristine.source_bytes(),
            "{name}: and the score untouched"
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

        assert_eq!(gui.undo(), Some(name), "{name}: the GUI undoes it");
        api.run("song undo").expect("the API undoes");
        for (path, document) in [("GUI", &gui), ("API", api.song())] {
            assert_eq!(
                document.file(),
                pristine.file(),
                "{name}: the {path}'s undo restores the sidecar"
            );
        }
    }
    assert_eq!(covered, [true; VARIANT_COUNT], "a row for every variant");
}
