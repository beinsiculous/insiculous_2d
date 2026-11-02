//! Open keeps what an interrupted save left and never promotes it: each journal file is
//! renamed beside the song to a recovery and noted, and the finals open as they are; with
//! no final at all the journal becomes the finals. One test per journal shape — the pair, a
//! lone sidecar, a lone `.mid.new`, a journal beside a `.mid` changed outside, a journal
//! with no finals — each asserting the recovery's name or the finished save, the note, and
//! the finals untouched.

use super::io::open::open_from;
use super::io::VfsStore;
use super::test_support::{add_note, exported, melody, AssetFolder, InterruptedStore};
use super::{midi_fingerprint, Gesture, SaveIntent, SongDocument, SongEdit, SongOpenError};

const SONG: &str = "music/theme.mid";
const SIDECAR: &str = "music/theme.mid.ron";

/// A melody saved at [`SONG`] (the old song), and the same melody with a fourth note (the
/// new one, unsaved) — a score edit, so its save writes the `.mid` and the journal's pair.
fn old_and_new(folder: &AssetFolder) -> (Vec<u8>, SongDocument) {
    let mut document = melody();
    document
        .save(&folder.path(SONG), folder.base())
        .expect("saves the old song");
    let old = exported(&document);
    document
        .apply(add_note(1440, 72), Gesture::Discrete)
        .expect("edits");
    (old, document)
}

fn reopen(folder: &AssetFolder) -> Result<super::OpenedSong, SongOpenError> {
    open_from(&mut VfsStore, &folder.path(SONG), folder.base())
}

/// Save `document` at [`SONG`] through a store that stops after `operations`.
fn interrupt(folder: &AssetFolder, document: &mut SongDocument, operations: usize) {
    let mut store = InterruptedStore {
        operations_left: operations,
    };
    document
        .save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default())
        .expect_err("interrupted");
}

/// The recovery a journal holding `bytes` (or a pair whose MIDI file holds them) is kept as
/// beside [`SONG`].
fn recovery(bytes: &[u8]) -> String {
    format!("theme.recovered-{:016x}.mid", midi_fingerprint(bytes))
}

fn finals(folder: &AssetFolder) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    (folder.read(SONG), folder.read(SIDECAR))
}

fn sorted(names: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    names.sort();
    names
}

#[test]
fn a_save_interrupted_at_any_step_opens_the_old_score_or_the_new_and_keeps_every_journal_file() {
    // An edited score's save without samples makes seven writes and removals: the pair
    // (sidecar, then MIDI), the finals (MIDI, then sidecar), the pair's two removals, and the
    // empty sample folder's removal — the home LICENSE already names the song. The score
    // opened is the final `.mid`, whatever the journal holds.
    for operations in 0..=7 {
        let folder = AssetFolder::new();
        let (old, mut document) = old_and_new(&folder);
        let new = exported(&document);
        let mut store = InterruptedStore {
            operations_left: operations,
        };
        let saved = document.save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default());
        assert_eq!(saved.is_ok(), operations == 7, "after {operations} operations");
        assert!(document.is_dirty() || operations == 7, "an interrupted save keeps the song dirty");

        let opened = reopen(&folder).unwrap_or_else(|error| panic!("after {operations}: {error}"));
        let expected = if operations >= 3 { &new } else { &old };
        assert_eq!(&exported(&opened.document), expected, "after {operations}: {:?}", opened.notes);
        let left = folder.files_under("music");
        assert!(
            left.iter().all(|name| !name.ends_with(".new")),
            "after {operations}: no journal stays a journal: {left:?}"
        );
        let journal_files_left = match operations {
            1 | 5 => 1,
            2..=4 => 2,
            _ => 0,
        };
        let recoveries = left.iter().filter(|name| name.contains(".recovered-")).count();
        assert_eq!(recoveries, journal_files_left, "after {operations}: {left:?}");
        assert_eq!(
            opened.notes.iter().filter(|note| note.contains("kept as")).count(),
            journal_files_left,
            "after {operations}: each kept file is noted: {:?}",
            opened.notes
        );
    }
}

#[test]
fn a_pair_is_kept_as_one_recovery_that_opens_as_the_new_song_and_the_finals_open_untouched() {
    let folder = AssetFolder::new();
    let (old, mut document) = old_and_new(&folder);
    let new = exported(&document);
    let before = finals(&folder);
    interrupt(&folder, &mut document, 2);

    let opened = reopen(&folder).expect("opens");
    assert_eq!(exported(&opened.document), old, "the finals open as they are");
    assert_eq!(finals(&folder), before, "the finals are untouched");
    let kept = recovery(&new);
    let kept_sidecar = format!("{kept}.ron");
    assert_eq!(
        folder.files_under("music"),
        sorted(&["LICENSE", "theme.mid", "theme.mid.ron", &kept, &kept_sidecar]),
        "the pair is named by its MIDI file, so its sidecar sits beside it"
    );
    assert!(opened.notes.iter().any(|note| note.contains(&kept)), "{:?}", opened.notes);

    let recovered = open_from(&mut VfsStore, &folder.path(&format!("music/{kept}")), folder.base())
        .expect("the recovery opens as a song");
    assert_eq!(exported(&recovered.document), new);
    assert!(recovered.notes.is_empty(), "the kept pair matches itself: {:?}", recovered.notes);
}

#[test]
fn a_lone_sidecar_journal_from_a_map_save_is_kept_as_a_recovered_sidecar() {
    let folder = AssetFolder::new();
    let mut document = melody();
    document.save(&folder.path(SONG), folder.base()).expect("saves");
    document
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");
    let before = finals(&folder);
    interrupt(&folder, &mut document, 1);
    let journal = folder
        .read("music/theme.mid.ron.new")
        .expect("a map save's journal is its sidecar alone");
    assert_eq!(folder.read("music/theme.mid.new"), None);

    let opened = reopen(&folder).expect("opens");
    assert!(opened.document.file().looping, "the finals' map opens");
    assert_eq!(finals(&folder), before, "the finals are untouched");
    let kept = format!("{}.ron", recovery(&journal));
    assert_eq!(folder.read(&format!("music/{kept}")), Some(journal));
    assert!(opened.notes.iter().any(|note| note.contains(&kept)), "{:?}", opened.notes);
}

#[test]
fn a_lone_midi_journal_is_kept_as_a_recovered_midi_file() {
    let folder = AssetFolder::new();
    let (old, document) = old_and_new(&folder);
    let new = exported(&document);
    folder.write("music/theme.mid.new", &new);
    let before = finals(&folder);

    let opened = reopen(&folder).expect("opens");
    assert_eq!(exported(&opened.document), old);
    assert_eq!(finals(&folder), before, "the final is never written over");
    let kept = recovery(&new);
    assert_eq!(folder.read(&format!("music/{kept}")), Some(new));
    assert!(opened.notes.iter().any(|note| note.contains(&kept)), "{:?}", opened.notes);
}

#[test]
fn a_journal_beside_a_midi_changed_outside_is_kept_and_the_changed_score_opens_with_a_note() {
    let folder = AssetFolder::new();
    let (_, document) = old_and_new(&folder);
    let new = exported(&document);
    folder.write(SONG, &new);
    let sidecar = folder.read(SIDECAR).expect("the sidecar");
    folder.write("music/theme.mid.ron.new", &sidecar);
    let before = finals(&folder);

    let opened = reopen(&folder).expect("a changed score never refuses the open");
    assert_eq!(exported(&opened.document), new, "the score on disk opens");
    assert_eq!(finals(&folder), before);
    let kept = format!("{}.ron", recovery(&sidecar));
    assert!(opened.notes.iter().any(|note| note.contains(&kept)), "{:?}", opened.notes);
    assert!(
        opened.notes.iter().any(|note| note.contains("changed outside the editor")),
        "{:?}",
        opened.notes
    );
}

#[test]
fn a_journal_with_no_finals_becomes_the_finals_and_a_lone_sidecar_there_opens_nothing() {
    let folder = AssetFolder::new();
    let mut document = melody();
    let new = exported(&document);
    interrupt(&folder, &mut document, 2);
    assert_eq!(folder.files_under("music"), ["theme.mid.new", "theme.mid.ron.new"]);

    let opened = reopen(&folder).expect("the journal is the song's only copy");
    assert_eq!(exported(&opened.document), new);
    assert!(opened.notes[0].contains("finished the first save"), "{:?}", opened.notes);
    assert_eq!(folder.files_under("music"), ["theme.mid", "theme.mid.ron"]);
    assert_eq!(opened.document.path(), Some(folder.path(SONG).as_path()));
    assert!(!opened.document.is_dirty(), "the finals are what it opened");

    let lone = AssetFolder::new();
    interrupt(&lone, &mut melody(), 1);
    let missing = reopen(&lone).expect_err("a sidecar journal holds no score");
    assert!(matches!(missing, SongOpenError::Missing { .. }));
    assert_eq!(lone.files_under("music"), ["theme.mid.ron.new"], "nothing was written");
}

#[test]
fn a_recovery_name_another_file_holds_moves_to_the_next_free_suffix() {
    let folder = AssetFolder::new();
    let (_, document) = old_and_new(&folder);
    let new = exported(&document);
    let taken = recovery(&new);
    folder.write(&format!("music/{taken}"), b"another file");
    folder.write("music/theme.mid.new", &new);

    reopen(&folder).expect("opens");
    assert_eq!(folder.read(&format!("music/{taken}")).as_deref(), Some(&b"another file"[..]));
    let next = taken.replace(".mid", "-2.mid");
    assert_eq!(folder.read(&format!("music/{next}")), Some(new));
}

#[test]
fn a_midi_file_without_a_sidecar_opens_in_place_on_the_projects_first_bank_with_its_map_unsaved() {
    let folder = AssetFolder::new();
    folder.write("soundfonts/strings.sf2", b"a bank");
    let lonely = folder.write("music/lonely.mid", &exported(&melody()));

    let opened = SongDocument::open(&lonely, folder.base()).expect("opens");
    assert_eq!(opened.document.path(), Some(lonely.as_path()), "in place, not as an import");
    assert!(opened.document.is_dirty(), "the sidecar is not on disk yet");
    let file = opened.document.file();
    assert_eq!(file.soundfont, "soundfonts/strings.sf2");
    assert!(file.parts.is_empty() && file.bindings.is_empty() && file.looping);
    assert!(opened.notes[0].contains("no sidecar"), "{:?}", opened.notes);

    let missing = SongDocument::open(&folder.path("music/absent.mid"), folder.base());
    assert!(matches!(missing, Err(SongOpenError::Missing { .. })));
}

#[test]
fn a_midi_its_sidecar_fingerprints_differently_opens_on_the_disk_score_and_the_next_save_records_it() {
    let folder = AssetFolder::new();
    let (_, document) = old_and_new(&folder);
    let new = exported(&document);
    folder.write(SONG, &new);

    let mut opened = reopen(&folder).expect("opens").document;
    assert_eq!(exported(&opened), new);
    opened.save(&folder.path(SONG), folder.base()).expect("a map save");
    assert_eq!(folder.read(SONG), Some(new.clone()), "the map save leaves the score");
    let reopened = reopen(&folder).expect("reopens");
    assert!(reopened.notes.is_empty(), "the fingerprint is the disk's now: {:?}", reopened.notes);
}

#[test]
fn a_midi_song_saves_reopens_and_recovers_under_its_whole_name() {
    let folder = AssetFolder::new();
    let target = folder.path("music/theme.midi");
    let mut document = melody();
    document.save(&target, folder.base()).expect("saves");
    document.apply(add_note(1440, 72), Gesture::Discrete).expect("edits");
    let new = exported(&document);
    let mut store = InterruptedStore { operations_left: 2 };
    document
        .save_to(&mut store, &target, folder.base(), SaveIntent::default())
        .expect_err("interrupted after the pair");
    assert_eq!(
        folder.files_under("music"),
        ["LICENSE", "theme.midi", "theme.midi.new", "theme.midi.ron", "theme.midi.ron.new"]
    );

    open_from(&mut VfsStore, &target, folder.base()).expect("opens");
    let kept = format!("theme.recovered-{:016x}.midi", midi_fingerprint(&new));
    let kept_sidecar = format!("{kept}.ron");
    assert_eq!(
        folder.files_under("music"),
        sorted(&["LICENSE", "theme.midi", "theme.midi.ron", &kept, &kept_sidecar])
    );
}

/// Every sample each sidecar beside [`SONG`] — the final and every recovery — names.
fn named_samples(folder: &AssetFolder) -> Vec<(String, String)> {
    folder
        .files_under("music")
        .into_iter()
        .filter(|name| name.ends_with(".mid.ron"))
        .flat_map(|name| {
            let text = String::from_utf8(folder.read(&format!("music/{name}")).expect("read")).expect("text");
            let file = music_midi::parse_song_file(&name, &text).expect("parses");
            file.instruments
                .into_iter()
                .filter_map(move |instrument| match instrument.source {
                    music_midi::InstrumentSource::Sample { path, .. } => Some((name.clone(), path)),
                    music_midi::InstrumentSource::SoundFont { .. } => None,
                })
        })
        .collect()
}

fn with_samples(folder: &AssetFolder, names: &[&str]) -> SongDocument {
    let mut document = melody();
    for name in names {
        let path = format!("sounds/{name}.wav");
        folder.write(&path, format!("RIFF {name}").as_bytes());
        let instrument = super::test_support::sample_instrument(name, &path);
        document
            .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
            .expect("adds the sample");
    }
    document
}

#[test]
fn every_sample_a_sidecar_beside_the_song_names_is_on_disk_wherever_its_save_was_interrupted() {
    // The old song plays two samples; the new map drops one, so its save prunes that file.
    // A map save makes five writes and removals: the journal's sidecar, the final sidecar,
    // the journal's removal, then the prune — the dropped file and the folder's LICENSE.
    for operations in 0..=5 {
        let folder = AssetFolder::new();
        let mut document = with_samples(&folder, &["blip", "boop"]);
        document.save(&folder.path(SONG), folder.base()).expect("saves the old song");
        document
            .apply(SongEdit::RemoveInstrument { name: "boop".to_string() }, Gesture::Discrete)
            .expect("drops one");
        let mut store = InterruptedStore {
            operations_left: operations,
        };
        let saved = document.save_to(&mut store, &folder.path(SONG), folder.base(), SaveIntent::default());
        assert_eq!(saved.is_ok(), operations == 5, "after {operations} operations");

        reopen(&folder).unwrap_or_else(|error| panic!("after {operations}: {error}"));
        for (sidecar, sample) in named_samples(&folder) {
            assert!(
                folder.read(&sample).is_some(),
                "after {operations} operations: {sidecar} names {sample}, which is gone"
            );
        }
    }
}

#[test]
fn a_recovery_kept_through_a_later_save_still_opens_with_every_sample_it_names() {
    let folder = AssetFolder::new();
    let mut document = with_samples(&folder, &["blip"]);
    document.save(&folder.path(SONG), folder.base()).expect("saves the old song");
    folder.write("sounds/boop.wav", b"RIFF boop");
    let boop = super::test_support::sample_instrument("boop", "sounds/boop.wav");
    document
        .apply(SongEdit::AddInstrument { instrument: boop }, Gesture::Discrete)
        .expect("adds a sample");
    document.apply(add_note(1440, 72), Gesture::Discrete).expect("and a note");
    // The new sample, the journal's sidecar and its MIDI file: the pair lands, the finals do not.
    interrupt(&folder, &mut document, 3);

    let mut reopened = reopen(&folder).expect("opens the old finals").document;
    let kept = folder
        .files_under("music")
        .into_iter()
        .find(|name| name.contains(".recovered-") && name.ends_with(".mid"))
        .expect("the pair was kept");
    reopened
        .apply(SongEdit::SetLoop { looping: false }, Gesture::Discrete)
        .expect("a map edit");
    reopened.save(&folder.path(SONG), folder.base()).expect("saves the song, which names blip alone");

    let recovered = open_from(&mut VfsStore, &folder.path(&format!("music/{kept}")), folder.base())
        .expect("the recovery opens");
    let names: Vec<&str> = recovered
        .document
        .file()
        .instruments
        .iter()
        .map(|instrument| instrument.name.as_str())
        .collect();
    assert_eq!(names, ["blip", "boop"]);
    for (sidecar, sample) in named_samples(&folder) {
        assert!(folder.read(&sample).is_some(), "{sidecar} names {sample}, which the prune removed");
    }
    let license = String::from_utf8(folder.read("music/samples/theme/LICENSE").expect("the LICENSE")).expect("text");
    assert!(license.contains("Files: boop.wav"), "a kept file is named, or the gate fails: {license}");
}

#[test]
fn song_init_writes_the_scaffold_beside_an_unmapped_midi_and_refuses_a_mapped_one() {
    let folder = AssetFolder::new();
    folder.write("soundfonts/strings.sf2", b"a bank");
    let score = exported(&melody());
    let lonely = folder.write("music/lonely.mid", &score);

    let sidecar = super::init_song(&lonely, folder.base()).expect("maps it");
    assert_eq!(sidecar, folder.path("music/lonely.mid.ron"));
    let opened = SongDocument::open(&lonely, folder.base()).expect("opens");
    assert!(opened.notes.is_empty(), "the scaffold fingerprints the score: {:?}", opened.notes);
    assert!(!opened.document.is_dirty(), "the sidecar is on disk");
    let file = opened.document.file();
    assert_eq!(file.soundfont, "soundfonts/strings.sf2");
    assert!(file.parts.is_empty() && file.instruments.is_empty() && file.bindings.is_empty() && file.looping);
    assert_eq!(folder.read("music/lonely.mid"), Some(score), "the score is never written");

    let mapped = super::init_song(&lonely, folder.base()).expect_err("it has a sidecar now");
    assert!(matches!(mapped, super::SongInitError::Mapped { .. }), "{mapped}");
    let outside = folder.write("downloads/tune.mid", &exported(&melody()));
    assert!(matches!(
        super::init_song(&outside, folder.base()),
        Err(super::SongInitError::OutsideHome { .. })
    ));
    assert!(folder.read("downloads/tune.mid.ron").is_none());
}
