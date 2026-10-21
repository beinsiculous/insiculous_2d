//! `SongDocument::apply`, the one mutation path: what it refuses (and that a refusal
//! changes nothing), how a batch lands as one entry or not at all, how a gesture's frames
//! merge, and that undo and redo replay exactly — a redone note keeps its id.

use music_midi::{
    Binding, Instrument, InstrumentSource, Part, PartMode, Quantize, SongFileProblem, TrackSelector,
};

use super::test_support::{add_note, exported, melody, note_numbers};
use super::{Gesture, SongDocument, SongEdit, SongEditError};

fn sound_font_instrument(name: &str) -> Instrument {
    Instrument {
        name: name.to_string(),
        source: InstrumentSource::SoundFont {
            path: "soundfonts/cast.sf2".to_string(),
            bank: 0,
            preset: 0,
        },
        provenance: None,
    }
}

fn melody_part() -> Part {
    Part {
        name: "lead".to_string(),
        track: TrackSelector::Name("melody".to_string()),
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    }
}

/// Apply `edit` expecting a refusal, and check it changed neither the song nor its history.
fn refused(document: &mut SongDocument, edit: SongEdit) -> SongEditError {
    let (file, midi, entries) = (
        document.file().clone(),
        exported(document),
        document.history().undo_count(),
    );
    let error = document
        .apply(edit, Gesture::Discrete)
        .expect_err("refused");
    assert_eq!(
        document.file(),
        &file,
        "a refused edit leaves the sidecar alone"
    );
    assert_eq!(exported(document), midi, "and the notes");
    assert_eq!(
        document.history().undo_count(),
        entries,
        "and records nothing"
    );
    error
}

#[test]
fn the_ninth_instrument_is_refused_at_apply_by_the_schema_check() {
    let mut document = melody();
    for index in 0..8 {
        let instrument = sound_font_instrument(&format!("instrument {index}"));
        document
            .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
            .expect("eight fit");
    }
    let error = refused(
        &mut document,
        SongEdit::AddInstrument {
            instrument: sound_font_instrument("ninth"),
        },
    );
    assert!(matches!(
        error,
        SongEditError::Schema(SongFileProblem::TooManyInstruments {
            count: 9,
            capacity: 8
        })
    ));
}

#[test]
fn out_of_range_values_unknown_names_and_a_dangling_binding_are_refused() {
    let mut document = melody();
    let first = note_numbers(&document)[0];
    let notes = vec![first];
    assert!(matches!(
        refused(
            &mut document,
            SongEdit::SetVelocity {
                notes: notes.clone(),
                velocity: 0
            }
        ),
        SongEditError::Edit(_)
    ));
    assert!(matches!(
        refused(
            &mut document,
            SongEdit::SetPitch {
                notes: notes.clone(),
                key: 128
            }
        ),
        SongEditError::Edit(_)
    ));
    assert!(matches!(
        refused(
            &mut document,
            SongEdit::MoveNotes {
                notes: notes.clone(),
                tick_delta: -1
            }
        ),
        SongEditError::Edit(_)
    ));
    assert_eq!(
        refused(&mut document, SongEdit::DeleteNotes { notes: vec![9_999] }),
        SongEditError::UnknownNote(9_999)
    );
    assert_eq!(
        refused(&mut document, SongEdit::DeleteNotes { notes: Vec::new() }),
        SongEditError::NoNotes
    );
    assert_eq!(
        refused(
            &mut document,
            SongEdit::AssignPart {
                notes,
                part: "lead".to_string()
            }
        ),
        SongEditError::UnknownPart("lead".to_string())
    );
    assert_eq!(
        refused(
            &mut document,
            SongEdit::SetQuantize {
                part: "lead".to_string(),
                quantize: Quantize::Beat
            }
        ),
        SongEditError::UnknownPart("lead".to_string())
    );

    document
        .apply(
            SongEdit::AddPart {
                part: melody_part(),
            },
            Gesture::Discrete,
        )
        .expect("adds the part");
    let binding = Binding {
        action: "jump".to_string(),
        part: "lead".to_string(),
        instrument: "absent".to_string(),
    };
    assert!(matches!(
        refused(&mut document, SongEdit::SetBinding { binding }),
        SongEditError::Schema(SongFileProblem::UnknownInstrument { .. })
    ));
    let instrument = sound_font_instrument("organ");
    document
        .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
        .expect("adds it");
    let binding = Binding {
        action: "jump".to_string(),
        part: "lead".to_string(),
        instrument: "organ".to_string(),
    };
    document
        .apply(SongEdit::SetBinding { binding }, Gesture::Discrete)
        .expect("binds it");
    assert!(
        matches!(
            refused(
                &mut document,
                SongEdit::RemovePart {
                    name: "lead".to_string()
                }
            ),
            SongEditError::Schema(SongFileProblem::UnknownPart { .. })
        ),
        "a part a binding plays cannot go"
    );
}

#[test]
fn a_batch_is_one_entry_and_a_refused_member_lands_none_of_it() {
    let mut document = melody();
    let numbers = note_numbers(&document);
    let entries = document.history().undo_count();
    let before = exported(&document);

    let error = refused(
        &mut document,
        SongEdit::Batch(vec![
            SongEdit::SetVelocity {
                notes: numbers.clone(),
                velocity: 40,
            },
            SongEdit::SetPitch {
                notes: numbers.clone(),
                key: 200,
            },
        ]),
    );
    assert!(
        matches!(error, SongEditError::Edit(_)),
        "the second member's refusal"
    );
    assert_eq!(
        refused(&mut document, SongEdit::Batch(Vec::new())),
        SongEditError::EmptyBatch
    );

    let transpose: Vec<SongEdit> = [(numbers[0], 62), (numbers[1], 65), (numbers[2], 69)]
        .into_iter()
        .map(|(number, key)| SongEdit::SetPitch {
            notes: vec![number],
            key,
        })
        .chain([add_note(1440, 72)])
        .collect();
    document
        .apply(SongEdit::Batch(transpose), Gesture::Discrete)
        .expect("the batch lands");
    assert_eq!(document.history().undo_count(), entries + 1, "one entry");
    assert_eq!(document.data().note_count(), 4);

    assert_eq!(document.undo().expect("undoes"), Some("Batch"));
    assert_eq!(
        exported(&document),
        before,
        "one undo takes back every member"
    );
    document.redo().expect("redoes");
    assert_eq!(document.data().note_count(), 4);
}

#[test]
fn a_redone_note_keeps_its_id_so_the_edits_after_it_redo_too() {
    let mut document = SongDocument::new_song().expect("a blank song");
    document
        .apply(add_note(0, 60), Gesture::Discrete)
        .expect("adds");
    let number = note_numbers(&document)[0];
    document
        .apply(
            SongEdit::SetVelocity {
                notes: vec![number],
                velocity: 30,
            },
            Gesture::Discrete,
        )
        .expect("softens it");
    let edited = exported(&document);

    document.undo().expect("undoes the velocity");
    document.undo().expect("undoes the note");
    assert_eq!(document.data().note_count(), 0);
    document.redo().expect("redoes the note");
    assert_eq!(
        document.redo().expect("redoes the velocity"),
        Some("SetVelocity")
    );
    assert_eq!(note_numbers(&document), vec![number]);
    assert_eq!(exported(&document), edited);
}

#[test]
fn a_gesture_merges_deltas_by_summing_and_absolute_values_by_replacing() {
    let move_by = |tick_delta| SongEdit::MoveNotes {
        notes: vec![2, 1],
        tick_delta,
    };
    let merged = move_by(10).try_merge(&SongEdit::MoveNotes {
        notes: vec![1, 2],
        tick_delta: 5,
    });
    assert_eq!(merged, Some(move_by(15)), "the same notes in any order");
    assert_eq!(
        move_by(10).try_merge(&SongEdit::MoveNotes {
            notes: vec![1],
            tick_delta: 5
        }),
        None
    );

    let velocity = |velocity| SongEdit::SetVelocity {
        notes: vec![1],
        velocity,
    };
    assert_eq!(velocity(10).try_merge(&velocity(90)), Some(velocity(90)));
    let tempo = |tick, microseconds_per_quarter| SongEdit::SetTempo {
        tick,
        microseconds_per_quarter,
    };
    assert_eq!(
        tempo(0, Some(1)).try_merge(&tempo(0, Some(2))),
        Some(tempo(0, Some(2)))
    );
    assert_eq!(
        tempo(0, Some(1)).try_merge(&tempo(480, Some(2))),
        None,
        "another tick"
    );
    assert_eq!(
        velocity(10).try_merge(&SongEdit::SetLoop { looping: false }),
        None
    );

    let renamed = SongEdit::SetInstrument {
        name: "a".to_string(),
        instrument: sound_font_instrument("b"),
    };
    let after = SongEdit::SetInstrument {
        name: "a".to_string(),
        instrument: sound_font_instrument("a"),
    };
    assert_eq!(
        renamed.try_merge(&after),
        None,
        "a rename is not one gesture with an edit"
    );
}

#[test]
fn tempo_and_meter_edits_land_on_their_tick_and_undo_to_what_was_there() {
    let mut document = melody();
    let tempo = |microseconds_per_quarter| SongEdit::SetTempo {
        tick: 0,
        microseconds_per_quarter,
    };
    document
        .apply(tempo(Some(400_000)), Gesture::Discrete)
        .expect("sets the tempo");
    assert_eq!(
        document.data().tempo_changes()[0].microseconds_per_quarter,
        400_000
    );
    document.undo().expect("undoes");
    assert_eq!(
        document.data().tempo_changes()[0].microseconds_per_quarter,
        500_000,
        "the blank song's 120 BPM"
    );

    document
        .apply(tempo(None), Gesture::Discrete)
        .expect("removes the tempo on tick 0");
    assert!(document.data().tempo_changes().is_empty());
    assert_eq!(
        refused(&mut document, tempo(None)),
        SongEditError::NoTempoChange(0)
    );

    let meter = |numerator, denominator| SongEdit::SetTimeSignature {
        tick: 1920,
        meter: Some(super::Meter {
            numerator,
            denominator,
        }),
    };
    document
        .apply(meter(3, 4), Gesture::Discrete)
        .expect("a waltz from bar two");
    assert_eq!(document.data().time_signatures().len(), 2);
    assert!(matches!(
        refused(&mut document, meter(3, 5)),
        SongEditError::Edit(_)
    ));
}
