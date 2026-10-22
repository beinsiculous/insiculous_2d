//! Where a save puts what a song owns: one flat sample folder per song in its home, owned
//! by the song its `LICENSE` names; the path rule that keeps the flattening unambiguous;
//! the song's own provenance, required at Save and written as its stanza in the home
//! folder's `LICENSE`.

use std::path::Path;

use music_midi::{InstrumentSource, Provenance};

use super::test_support::{melody, sample_instrument, AssetFolder};
use super::{
    sample_directory, saveable_song_path, Gesture, SongDocument, SongEdit, SongSaveError,
};

const BLIP: &[u8] = b"RIFF blip recording";

/// A melody playing one sample, `sounds/blip.wav`, written into `folder`.
fn melody_with_a_sample(folder: &AssetFolder) -> SongDocument {
    folder.write("sounds/blip.wav", BLIP);
    let mut document = melody();
    let instrument = sample_instrument("blip", "sounds/blip.wav");
    document
        .apply(SongEdit::AddInstrument { instrument }, Gesture::Discrete)
        .expect("the instrument lands");
    document
}

fn save(document: &mut SongDocument, folder: &AssetFolder, relative: &str) -> Result<(), SongSaveError> {
    document.save(&folder.path(relative), folder.base())
}

fn text(folder: &AssetFolder, relative: &str) -> String {
    String::from_utf8(folder.read(relative).expect("the file exists")).expect("text")
}

/// Every `Files:` paragraph of a `LICENSE` as its whitespace-split tokens, continuation
/// lines included — the provenance gate's grammar (`scripts/check_audio_provenance.sh`).
fn stanza_files(license: &str) -> Vec<Vec<String>> {
    let mut stanzas = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in license.lines() {
        if let Some(first) = line.strip_prefix("Files:") {
            stanzas.extend(current.take());
            current = Some(first.split_whitespace().map(str::to_string).collect());
        } else if line.starts_with([' ', '\t']) {
            if let Some(files) = current.as_mut() {
                files.extend(line.split_whitespace().map(str::to_string));
            }
        } else {
            stanzas.extend(current.take());
        }
    }
    stanzas.extend(current);
    stanzas
}

fn sample_paths(document: &SongDocument) -> Vec<String> {
    document
        .file()
        .instruments
        .iter()
        .filter_map(|instrument| match &instrument.source {
            InstrumentSource::Sample { path, .. } => Some(path.clone()),
            InstrumentSource::SoundFont { .. } => None,
        })
        .collect()
}

#[test]
fn each_song_gets_a_flat_sample_folder_in_its_home_and_reopens_with_every_sample() {
    let folder = AssetFolder::new();
    for (song, sample_folder) in [
        ("music/theme.mid", "music/samples/theme"),
        ("ai/music/theme.mid", "ai/music/samples/theme"),
        ("music/ai/theme.mid", "music/samples/ai__theme"),
    ] {
        assert_eq!(sample_directory(Path::new(song)).as_deref(), Some(sample_folder));
        let mut document = melody_with_a_sample(&folder);
        save(&mut document, &folder, song).unwrap_or_else(|error| panic!("{song}: {error}"));
        assert_eq!(sample_paths(&document), [format!("{sample_folder}/blip.wav")]);
        let license = text(&folder, &format!("{sample_folder}/LICENSE"));
        assert!(license.contains(&format!("\nSource: {song}\n")), "{license}");
    }
    for song in ["music/theme.mid", "ai/music/theme.mid", "music/ai/theme.mid"] {
        let opened = SongDocument::open(&folder.path(song), folder.base()).expect("reopens");
        for path in sample_paths(&opened.document) {
            assert_eq!(folder.read(&path).as_deref(), Some(BLIP), "{song} names {path}");
        }
    }
    assert_eq!(
        sample_directory(Path::new("music/a/theme.mid")).as_deref(),
        Some("music/samples/a__theme")
    );
}

#[test]
fn a_save_into_a_folder_another_song_owns_is_refused_naming_both_in_either_order() {
    for (first, second) in [
        ("music/x__y.mid", "music/x/y.mid"),
        ("music/x/y.mid", "music/x__y.mid"),
    ] {
        let folder = AssetFolder::new();
        save(&mut melody_with_a_sample(&folder), &folder, first).expect("the first song saves");
        let mut document = melody_with_a_sample(&folder);
        let error = save(&mut document, &folder, second).expect_err("the folder is the first's");
        assert!(
            matches!(&error, SongSaveError::SampleFolderTaken { owner, song, .. } if owner == first && song == second),
            "{error}"
        );
        assert!(error.to_string().contains("Save As"), "{error}");
        assert!(folder.read(second).is_none(), "nothing of {second} was written");
        assert_eq!(text(&folder, "music/samples/x__y/LICENSE").matches("Source:").count(), 1);
    }
}

#[test]
fn a_folder_from_before_owners_were_named_is_adopted_by_its_own_song_only() {
    let folder = AssetFolder::new();
    let unowned = "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n\n\
                   Files: old.wav\nCopyright: 2026 Jesse\nLicense: MIT\n";
    for legacy in ["music/samples/theme", "music/samples/x__y"] {
        folder.write(&format!("{legacy}/old.wav"), b"RIFF old");
        folder.write(&format!("{legacy}/LICENSE"), unowned.as_bytes());
    }

    let mut own = melody_with_a_sample(&folder);
    save(&mut own, &folder, "music/theme.mid").expect("its own stem adopts it");
    let license = text(&folder, "music/samples/theme/LICENSE");
    assert!(license.contains("Source: music/theme.mid"), "{license}");
    assert!(folder.read("music/samples/theme/old.wav").is_none(), "the prune took the rest");

    let mut other = melody_with_a_sample(&folder);
    let error = save(&mut other, &folder, "music/x/y.mid").expect_err("not its stem");
    assert!(matches!(error, SongSaveError::SampleFolderUnowned { .. }), "{error}");
    assert!(folder.read("music/samples/x__y/old.wav").is_some(), "left alone");
}

#[test]
fn the_path_rule_refuses_ambiguous_spaced_and_sample_folder_paths_and_grandfathers_flat_ones() {
    for refused in [
        "music/a_/b.mid",
        "music/a/_b.mid",
        "music/a__b/c.mid",
        "music/a/b c.mid",
        "music/samples/theme.mid",
        "ai/music/samples/x/theme.mid",
        "music/x__y.mid",
        "music/_theme.mid",
        "sounds/theme.mid",
    ] {
        let reason = saveable_song_path(Path::new(refused)).expect_err(refused);
        assert!(reason.contains(refused), "{reason}");
    }
    for accepted in ["music/theme.mid", "music/a/theme.mid", "ai/music/ai_theme.mid", "music/x_y.mid"] {
        assert_eq!(saveable_song_path(Path::new(accepted)), Ok(()), "{accepted}");
    }

    let folder = AssetFolder::new();
    let error = save(&mut melody(), &folder, "music/a_/b.mid").expect_err("ambiguous");
    assert!(matches!(&error, SongSaveError::PathRule(reason) if reason.contains("Save As")), "{error}");
    let error = save(&mut melody(), &folder, "music/samples/theme.mid").expect_err("in a sample folder");
    assert!(matches!(error, SongSaveError::PathRule(_)), "{error}");
    let error = save(&mut melody(), &folder, "music/my theme.mid").expect_err("spaced");
    assert!(matches!(error, SongSaveError::PathRule(_)), "{error}");
    save(&mut melody(), &folder, "music/x__y.mid").expect("a flat song keeps the looser rule");
    save(&mut melody(), &folder, "music/_theme_.mid").expect("whatever its stem holds");
}

#[test]
fn a_song_without_its_own_provenance_refuses_the_save_and_writes_nothing() {
    let folder = AssetFolder::new();
    let mut document = melody();
    document.file.provenance = None;
    let error = save(&mut document, &folder, "music/theme.mid").expect_err("refused");
    assert!(matches!(error, SongSaveError::MissingSongProvenance { field: "provenance" }), "{error}");
    assert!(error.to_string().contains("Author and License"), "{error}");

    let blank_author = Provenance {
        author: "  ".to_string(),
        license: "MIT".to_string(),
    };
    document
        .apply(SongEdit::SetProvenance { provenance: blank_author }, Gesture::Discrete)
        .expect("an edit, however incomplete");
    let error = save(&mut document, &folder, "music/theme.mid").expect_err("refused");
    assert!(matches!(error, SongSaveError::MissingSongProvenance { field: "provenance.author" }));
    assert!(folder.files_under("").is_empty(), "nothing was written");
}

#[test]
fn two_songs_saved_into_one_home_keep_each_others_stanzas_and_pass_the_gate_grammar() {
    let folder = AssetFolder::new();
    let mut theme = melody();
    save(&mut theme, &folder, "music/theme.mid").expect("saves");
    let mut waltz = melody();
    let provenance = Provenance {
        author: "2026 M".to_string(),
        license: "CC0-1.0".to_string(),
    };
    waltz
        .apply(SongEdit::SetProvenance { provenance }, Gesture::Discrete)
        .expect("lands");
    save(&mut waltz, &folder, "music/dances/waltz.mid").expect("saves");
    save(&mut theme, &folder, "music/theme.mid").expect("saves again");

    let license = text(&folder, "music/LICENSE");
    assert!(license.starts_with("Format: "), "{license}");
    assert_eq!(
        stanza_files(&license),
        [
            vec!["theme.mid".to_string(), "theme.mid.ron".to_string()],
            vec!["dances/waltz.mid".to_string(), "dances/waltz.mid.ron".to_string()],
        ],
        "{license}"
    );
    assert!(license.contains("Copyright: 2026 Jesse\nLicense: CC-BY-4.0"), "{license}");
    assert!(license.contains("Copyright: 2026 M\nLicense: CC0-1.0"), "{license}");
}

#[test]
fn a_stanza_a_person_wrote_for_the_mid_alone_is_rewritten_not_duplicated() {
    let folder = AssetFolder::new();
    folder.write(
        "music/LICENSE",
        b"Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n\
          Comment: kept\n\n\
          Files: theme.mid\nCopyright: 2025 someone\nLicense: MIT\n\n\
          Files: other.mid other.mid.ron\n jingle.mid\nCopyright: 2025 someone\nLicense: MIT\n",
    );
    save(&mut melody(), &folder, "music/theme.mid").expect("saves");
    let license = text(&folder, "music/LICENSE");
    assert!(license.contains("Comment: kept"), "{license}");
    assert_eq!(
        stanza_files(&license),
        [
            vec!["theme.mid".to_string(), "theme.mid.ron".to_string()],
            vec![
                "other.mid".to_string(),
                "other.mid.ron".to_string(),
                "jingle.mid".to_string()
            ],
        ],
        "{license}"
    );
    assert_eq!(license.matches("2025 someone").count(), 1, "{license}");

    folder.write(
        "music/LICENSE",
        b"Format: x\n\nFiles: theme.mid jingle.mid\nCopyright: 2025 someone\nLicense: MIT\n",
    );
    save(&mut melody(), &folder, "music/theme.mid").expect("saves");
    assert_eq!(
        stanza_files(&text(&folder, "music/LICENSE")),
        [
            vec!["jingle.mid".to_string()],
            vec!["theme.mid".to_string(), "theme.mid.ron".to_string()],
        ],
        "another song's file keeps its stanza"
    );
}
