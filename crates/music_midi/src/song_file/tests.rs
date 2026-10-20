use super::*;
use crate::test_support::*;

const LABEL: &str = "music/napolitan.mid.ron";
const CAST: &str = "soundfonts/conductor_cast.sf2";
const TAMBOURINE: &str = "music/samples/napolitan/tambourine.wav";

/// The full example in the author's guide, so the guide cannot drift from the schema.
fn documented_example() -> &'static str {
    let guide = include_str!("../../../../docs/SONG_FORMAT.md");
    let start = guide
        .find("```ron\nSongFile(")
        .expect("the guide has a full example")
        + "```ron\n".len();
    let length = guide[start..]
        .find("```")
        .expect("the example block closes");
    &guide[start..start + length]
}

/// What a loader would report for the cast SoundFont (every preset layering two voices)
/// and the tambourine sample.
fn loaded_resources() -> ResourceMetadata {
    let mut resources = ResourceMetadata::default();
    for preset in 0..=127 {
        resources.preset_layering.insert(
            PresetKey {
                soundfont: CAST.to_string(),
                bank: 0,
                preset,
            },
            2,
        );
    }
    resources.sample_seconds.insert(TAMBOURINE.to_string(), 0.4);
    resources
}

fn parse(text: &str) -> Result<SongFile, SongFileError> {
    parse_song_file(LABEL, text)
}

fn with_resources(text: &str) -> SongFile {
    let mut file = parse(text).expect("parses");
    file.resources = loaded_resources();
    file
}

fn assert_refused(
    result: Result<impl std::fmt::Debug, SongFileError>,
    case: &str,
    needles: &[&str],
) {
    let error = result.expect_err(case);
    let message = error.to_string();
    assert!(
        message.starts_with(LABEL),
        "{case}: names the file: {message}"
    );
    for needle in needles {
        assert!(
            message.contains(needle),
            "{case}: expected {needle:?} in {message}"
        );
    }
}

#[test]
fn the_documented_example_parses_validates_and_round_trips() {
    let (napolitan, _) = read(NAPOLITAN_DANCE);
    let mut file = parse(documented_example()).expect("the guide's example parses");
    file.resources = loaded_resources();

    let written =
        ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default()).expect("serializes");
    let reread = parse(&written).expect("its own output parses");

    assert_eq!(file.validate(LABEL, &napolitan), Ok(()));
    assert_eq!(file.parts.len(), 3);
    assert_eq!(file.parts[1].track, TrackSelector::Index(2));
    assert_eq!(file.parts[2].mode, PartMode::Sequence);
    assert_eq!(file.bindings[2].instrument, "tambourine");
    assert_eq!(
        reread,
        SongFile {
            resources: ResourceMetadata::default(),
            ..file
        },
        "resources never reach the file"
    );
}

#[test]
fn omitted_fields_take_their_defaults() {
    let file = parse(
        r#"SongFile(version: 1, soundfont: "a.sf2",
            parts: [(name: "lead", track: Index(1))],
            instruments: [(name: "blip", source: Sample(path: "blip.wav", root_note: 60))])"#,
    )
    .expect("parses");

    assert!(file.looping, "a song loops unless told otherwise");
    assert_eq!(
        (file.parts[0].mode, file.parts[0].quantize),
        (PartMode::Loop, Quantize::Off)
    );
    assert_eq!(
        file.instruments[0].source,
        InstrumentSource::Sample {
            path: "blip.wav".to_string(),
            root_note: 60,
            gain: 1.0,
            playback: SamplePlayback::OneShot
        }
    );
    assert!(file.bindings.is_empty());
}

#[test]
fn a_future_version_is_refused_for_its_version_not_its_new_fields() {
    let future = r#"SongFile(version: 2, soundfont: "a.sf2", swing: 0.6)"#;

    assert_refused(
        parse(future),
        "version 2",
        &["unsupported song version 2", "reads version 1"],
    );
}

#[test]
fn schema_errors_fail_loud_naming_the_collection_and_the_entry() {
    let file = |parts: &str, instruments: &str, bindings: &str| {
        format!(
            r#"SongFile(version: 1, soundfont: "a.sf2", parts: [{parts}], instruments: [{instruments}], bindings: [{bindings}])"#
        )
    };
    let lead = r#"(name: "lead", track: Index(1))"#;
    let piano = r#"(name: "piano", source: SoundFont(path: "a.sf2", bank: 0, preset: 0))"#;
    let sample = |root_note: &str, gain: &str| {
        format!(
            r#"(name: "blip", source: Sample(path: "blip.wav", root_note: {root_note}, gain: {gain}, playback: Gated))"#
        )
    };
    let nine_instruments = (0..9)
        .map(|index| {
            format!(r#"(name: "i{index}", source: SoundFont(path: "a.sf2", bank: 0, preset: 0))"#)
        })
        .collect::<Vec<_>>()
        .join(",");

    let mut cases = vec![
        (
            r#"SongFile(version: 1, soundfont: "a.sf2", tempo: 120)"#.to_string(),
            vec!["malformed", "tempo"],
        ),
        (
            r#"SongFile(version: 1, soundfont: "")"#.to_string(),
            vec!["soundfont has an empty path"],
        ),
        (
            file(r#"(name: "", track: Index(0))"#, "", ""),
            vec!["parts[0] has an empty name"],
        ),
        (
            file(&format!("{lead},{lead}"), "", ""),
            vec!["parts['lead'] is declared more than once"],
        ),
        (
            file("", &format!("{piano},{piano}"), ""),
            vec!["instruments['piano'] is declared more than once"],
        ),
        (
            file(
                lead,
                piano,
                r#"(action: "jump", part: "lead", instrument: "piano"), (action: "jump", part: "lead", instrument: "piano")"#,
            ),
            vec!["bindings['jump'] is declared more than once"],
        ),
        (
            file(
                lead,
                piano,
                r#"(action: "jump", part: "solo", instrument: "piano")"#,
            ),
            vec!["bindings['jump']", "part 'solo'"],
        ),
        (
            file(
                lead,
                piano,
                r#"(action: "jump", part: "lead", instrument: "harp")"#,
            ),
            vec!["bindings['jump']", "instrument 'harp'"],
        ),
        (
            file(
                "",
                r#"(name: "piano", source: SoundFont(path: "a.sf2", bank: 0, preset: 128))"#,
                "",
            ),
            vec!["instruments['piano']", "preset 128"],
        ),
        (
            file(
                "",
                r#"(name: "piano", source: SoundFont(path: "a.sf2", bank: 129, preset: 0))"#,
                "",
            ),
            vec!["instruments['piano']", "bank 129"],
        ),
        (
            file(
                "",
                r#"(name: "piano", source: SoundFont(path: "", bank: 0, preset: 0))"#,
                "",
            ),
            vec!["instruments['piano'] has an empty path"],
        ),
        (
            file("", &sample("128", "1.0"), ""),
            vec!["instruments['blip']", "root note 128"],
        ),
        (
            file("", &nine_instruments, ""),
            vec!["9 entries", "8 instrument slots"],
        ),
    ];
    // RON parses `inf` and `NaN`, so the validator has to catch them.
    for gain in ["NaN", "inf", "-inf", "-0.5"] {
        cases.push((
            file("", &sample("60", gain), ""),
            vec!["instruments['blip']", "gain"],
        ));
    }

    for (text, needles) in cases {
        assert_refused(parse(&text), &text, &needles);
    }
}

#[test]
fn validation_against_the_midi_file_names_the_failing_entry() {
    let (napolitan, _) = read(NAPOLITAN_DANCE);
    let song_file = |part: &str, instrument: &str| {
        with_resources(&format!(
            r#"SongFile(version: 1, soundfont: "{CAST}", parts: [{part}], instruments: [{instrument}],
                bindings: [(action: "hit", part: "lead", instrument: "voice")])"#
        ))
    };
    let flute = r#"(name: "lead", track: Name("flute"))"#;
    let trumpet =
        format!(r#"(name: "voice", source: SoundFont(path: "{CAST}", bank: 0, preset: 56))"#);
    let sample_at = |root_note: u8| {
        format!(r#"(name: "voice", source: Sample(path: "{TAMBOURINE}", root_note: {root_note}))"#)
    };

    let cases = [
        (
            song_file(r#"(name: "lead", track: Name("oboe"))"#, &trumpet),
            vec!["parts['lead']", "track name 'oboe'", "5 tracks"],
        ),
        (
            song_file(r#"(name: "lead", track: Index(5))"#, &trumpet),
            vec!["parts['lead']", "track index 5"],
        ),
        (
            song_file(r#"(name: "lead", track: Index(0))"#, &trumpet),
            vec!["parts['lead'] plays track 0, which has no notes"],
        ),
        // The flute plays keys 65 to 79; its first note more than 24 above 40 is named.
        (
            song_file(flute, &sample_at(40)),
            vec![
                "bindings['hit']",
                "key 72 at tick 960",
                "root note 40",
                "24 semitones",
            ],
        ),
        (
            song_file(
                flute,
                &format!(
                    r#"(name: "voice", source: SoundFont(path: "{CAST}", bank: 128, preset: 0))"#
                ),
            ),
            vec!["instruments['voice']", "bank 128 preset 0", "does not have"],
        ),
        (
            song_file(
                flute,
                r#"(name: "voice", source: Sample(path: "missing.wav", root_note: 72))"#,
            ),
            vec!["instruments['voice']", "'missing.wav'", "did not decode"],
        ),
    ];
    for (file, needles) in cases {
        assert_refused(
            file.validate(LABEL, &napolitan),
            &format!("{:?}", file.parts),
            &needles,
        );
    }
    assert_eq!(
        song_file(flute, &sample_at(72)).validate(LABEL, &napolitan),
        Ok(()),
        "65 to 79 is within 24 of 72"
    );
}

#[test]
fn a_preset_layering_too_thickly_for_the_voice_pool_is_refused_and_one_that_fits_passes() {
    let (napolitan, _) = read(NAPOLITAN_DANCE);
    let mut file = with_resources(&format!(
        r#"SongFile(version: 1, soundfont: "{CAST}", instruments: [(name: "strings", source: SoundFont(path: "{CAST}", bank: 0, preset: 48))])"#
    ));
    let strings = PresetKey {
        soundfont: CAST.to_string(),
        bank: 0,
        preset: 48,
    };

    file.resources.preset_layering.insert(strings.clone(), 5);
    let fits = file.validate(LABEL, &napolitan);
    file.resources.preset_layering.insert(strings, 6);
    let overflows = file.validate(LABEL, &napolitan);

    assert_eq!(fits, Ok(()), "5 voices × 24 notes = 120 of 128");
    assert_refused(
        overflows,
        "layering 6",
        &[
            "instruments['strings']",
            "layers 6",
            "need 144 voices",
            "128",
        ],
    );
}

#[test]
fn a_sample_outside_the_decoded_length_limits_is_refused() {
    let (napolitan, _) = read(NAPOLITAN_DANCE);
    let mut file = with_resources(&format!(
        r#"SongFile(version: 1, soundfont: "{CAST}", instruments: [(name: "hit", source: Sample(path: "{TAMBOURINE}", root_note: 60))])"#
    ));

    for seconds in [0.000_5, 10.5] {
        file.resources
            .sample_seconds
            .insert(TAMBOURINE.to_string(), seconds);
        assert_refused(
            file.validate(LABEL, &napolitan),
            &format!("{seconds} s"),
            &["instruments['hit']", "samples run 0.001 s to 10 s"],
        );
    }
}

#[test]
fn a_shared_track_name_needs_the_index_escape_hatch_which_is_never_refused() {
    let lead_track = || {
        vec![
            (0, track_name("lead")),
            (0, note_on(0, 60, 100)),
            (96, note_off(0, 60)),
        ]
    };
    let bytes = smf(&[vec![], lead_track(), lead_track()]);
    let (song, _) = read(&bytes);
    let song_file = |selector: &str| {
        with_resources(&format!(
            r#"SongFile(version: 1, soundfont: "{CAST}", parts: [(name: "melody", track: {selector})])"#
        ))
    };

    let by_name = song_file(r#"Name("lead")"#).validate(LABEL, &song);
    let by_index = song_file("Index(2)").validate(LABEL, &song);

    assert_refused(
        by_name,
        "a shared name",
        &["parts['melody']", "'lead'", "tracks 1 and 2", "Index"],
    );
    assert_eq!(by_index, Ok(()));
}

#[test]
fn two_bindings_play_one_part_on_two_instruments_from_one_copy_of_its_notes() {
    let (napolitan, _) = read(NAPOLITAN_DANCE);
    let notes_before = napolitan.note_count();
    let file = with_resources(&format!(
        r#"SongFile(version: 1, soundfont: "{CAST}",
            parts: [(name: "melody", track: Name("flute"))],
            instruments: [
                (name: "trumpet", source: SoundFont(path: "{CAST}", bank: 0, preset: 56)),
                (name: "oboe", source: SoundFont(path: "{CAST}", bank: 0, preset: 68)),
            ],
            bindings: [
                (action: "coin", part: "melody", instrument: "trumpet"),
                (action: "combo", part: "melody", instrument: "oboe"),
            ])"#
    ));

    let tracks: Vec<usize> = file
        .bindings
        .iter()
        .map(|binding| {
            let part = file
                .parts
                .iter()
                .find(|part| part.name == binding.part)
                .expect("declared");
            part.resolve_track(&napolitan).expect("resolves")
        })
        .collect();

    assert_eq!(file.validate(LABEL, &napolitan), Ok(()));
    assert_eq!(
        tracks,
        vec![1, 1],
        "both bindings read the flute track itself"
    );
    assert_eq!(napolitan.note_count(), notes_before);
    assert_eq!(napolitan.tracks()[1].note_count(), 248);
}

#[test]
fn the_percussion_channel_of_a_type_zero_file_is_selectable_as_a_part() {
    let (type_zero, _) = read(NAPOLITAN_DANCE_TYPE_ZERO);
    let file = with_resources(&format!(
        r#"SongFile(version: 1, soundfont: "{CAST}",
            parts: [(name: "drums", track: Name("channel 10"), mode: AtSongPosition)],
            instruments: [(name: "tambourine", source: Sample(path: "{TAMBOURINE}", root_note: 54))],
            bindings: [(action: "bump", part: "drums", instrument: "tambourine")])"#
    ));

    let track = file.parts[0].resolve_track(&type_zero).expect("resolves");
    let percussion = &type_zero.tracks()[track];
    let keys: std::collections::BTreeSet<u8> = percussion
        .units()
        .iter()
        .flat_map(|unit| unit.notes.iter().map(|note| note.key))
        .collect();

    assert_eq!(file.validate(LABEL, &type_zero), Ok(()));
    assert_eq!(percussion.channel(), Some(9), "General MIDI percussion");
    assert_eq!(percussion.note_count(), 110);
    assert_eq!(keys, [54].into(), "every note is the tambourine");
}
