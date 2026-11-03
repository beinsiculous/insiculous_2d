//! `SongDocument::apply`, the one mutation path: what it refuses (and that a refusal
//! changes nothing), how a batch lands as one entry or not at all, how a gesture's frames
//! merge, and that undo and redo replay exactly.

use music_midi::{
    Binding, Instrument, InstrumentSource, Part, PartMode, Quantize, SongFileProblem, TrackSelector,
};

use super::test_support::melody;
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
    let (file, entries) = (document.file().clone(), document.history().undo_count());
    let error = document
        .apply(edit, Gesture::Discrete)
        .expect_err("refused");
    assert_eq!(
        document.file(),
        &file,
        "a refused edit leaves the sidecar alone"
    );
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
fn unknown_names_and_a_dangling_binding_are_refused() {
    let mut document = melody();
    assert_eq!(
        refused(
            &mut document,
            SongEdit::RemoveInstrument {
                name: "absent".to_string()
            }
        ),
        SongEditError::UnknownInstrument("absent".to_string())
    );
    assert_eq!(
        refused(
            &mut document,
            SongEdit::RemoveBinding {
                action: "absent".to_string()
            }
        ),
        SongEditError::UnknownBinding("absent".to_string())
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
    let before = document.file().clone();

    let error = refused(
        &mut document,
        SongEdit::Batch(vec![
            SongEdit::SetLoop { looping: false },
            SongEdit::SetQuantize {
                part: "absent".to_string(),
                quantize: Quantize::Beat,
            },
        ]),
    );
    assert_eq!(
        error,
        SongEditError::UnknownPart("absent".to_string()),
        "the second member's refusal"
    );
    assert_eq!(
        refused(&mut document, SongEdit::Batch(Vec::new())),
        SongEditError::EmptyBatch
    );

    let mapping = vec![
        SongEdit::AddPart {
            part: melody_part(),
        },
        SongEdit::AddInstrument {
            instrument: sound_font_instrument("organ"),
        },
        SongEdit::SetBinding {
            binding: Binding {
                action: "jump".to_string(),
                part: "lead".to_string(),
                instrument: "organ".to_string(),
            },
        },
    ];
    document
        .apply(SongEdit::Batch(mapping), Gesture::Discrete)
        .expect("the batch lands");
    assert_eq!(document.history().undo_count(), 1, "one entry");
    let mapped = document.file().clone();
    assert_eq!(
        (mapped.parts.len(), mapped.instruments.len(), mapped.bindings.len()),
        (1, 1, 1)
    );

    assert_eq!(document.undo(), Some("Batch"));
    assert_eq!(document.file(), &before, "one undo takes back every member");
    assert_eq!(document.redo(), Some("Batch"));
    assert_eq!(document.file(), &mapped, "and one redo lands them again");
}

#[test]
fn a_gesture_merges_absolute_values_by_replacing_on_the_same_part_or_instrument_only() {
    let quantize = |part: &str, quantize| SongEdit::SetQuantize {
        part: part.to_string(),
        quantize,
    };
    assert_eq!(
        quantize("lead", Quantize::Beat).try_merge(&quantize("lead", Quantize::Off)),
        Some(quantize("lead", Quantize::Off)),
        "the later value replaces the earlier"
    );
    assert_eq!(
        quantize("lead", Quantize::Beat).try_merge(&quantize("bass", Quantize::Off)),
        None,
        "another part"
    );
    let looping = |looping| SongEdit::SetLoop { looping };
    assert_eq!(looping(true).try_merge(&looping(false)), Some(looping(false)));
    assert_eq!(
        looping(true).try_merge(&quantize("lead", Quantize::Beat)),
        None,
        "another variant"
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
    assert_eq!(after.try_merge(&after), Some(after.clone()));
}
