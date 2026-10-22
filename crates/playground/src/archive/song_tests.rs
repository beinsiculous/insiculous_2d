//! An archive's songs: Export carries the open song's live files and drops only its own
//! journal pair; import refuses a final sidecar without its MIDI file and a `.mid` that is
//! not MIDI, and leaves out a journal leftover that fails the same checks.

use super::tests::{create_test_zip, two_scene_project, valid_manifest_ron};
use super::*;

const SIDECAR: &[u8] = br#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2")"#;
const MIDI: &[u8] = b"MThd\0\0\0\x06\0\x01\0\x01\x01\xe0MTrk\0\0\0\x04\0\xff\x2f\0";

fn paths(files: &[crate::store::StoredFile]) -> Vec<&str> {
    let mut paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    paths.sort_unstable();
    paths
}

#[test]
fn export_carries_the_live_song_over_its_saved_files_and_drops_only_its_own_journal_pair() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();
    let manifest = two_scene_project(root);
    let music = root.join("assets/music");
    std::fs::create_dir_all(&music).unwrap();
    for (name, bytes) in [
        ("theme.mid", MIDI),
        ("theme.mid.ron", SIDECAR),
        ("theme.mid.new", MIDI),
        ("theme.mid.ron.new", SIDECAR),
        ("other.mid", MIDI),
        ("other.mid.ron", SIDECAR),
        ("other.mid.new", MIDI),
    ] {
        std::fs::write(music.join(name), bytes).unwrap();
    }
    let live_sidecar = br#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2", looping: false)"#.to_vec();
    let live = [
        ("assets/music/theme.mid".to_string(), MIDI.to_vec()),
        ("assets/music/theme.mid.ron".to_string(), live_sidecar.clone()),
    ];

    let zip_bytes = export_snapshot(
        root,
        &manifest,
        "assets/scenes/main.scene.ron",
        r#"SceneData(name: "main", entities: [])"#,
        &live,
    )
    .unwrap();
    let (_, files) = import_project(&zip_bytes, "v1").unwrap();

    let music_paths: Vec<&str> = paths(&files).into_iter().filter(|path| path.contains("/music/")).collect();
    assert_eq!(
        music_paths,
        [
            "assets/music/other.mid",
            "assets/music/other.mid.new",
            "assets/music/other.mid.ron",
            "assets/music/theme.mid",
            "assets/music/theme.mid.ron",
        ],
        "another song's interrupted save keeps its recovery"
    );
    let sidecar = files.iter().find(|file| file.path == "assets/music/theme.mid.ron").unwrap();
    assert_eq!(sidecar.bytes, live_sidecar, "the live sidecar, not the saved one");
}

#[test]
fn export_adds_a_never_saved_songs_files_and_refuses_one_outside_the_assets() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();
    let manifest = two_scene_project(root);
    let scene = r#"SceneData(name: "main", entities: [])"#;
    let live = [
        ("assets/music/untitled.mid".to_string(), MIDI.to_vec()),
        ("assets/music/untitled.mid.ron".to_string(), SIDECAR.to_vec()),
    ];
    let zip_bytes = export_snapshot(root, &manifest, "assets/scenes/main.scene.ron", scene, &live).unwrap();
    let (_, files) = import_project(&zip_bytes, "v1").unwrap();
    assert!(paths(&files).contains(&"assets/music/untitled.mid.ron"));

    let outside = [("music/untitled.mid".to_string(), MIDI.to_vec())];
    let refused = export_snapshot(root, &manifest, "assets/scenes/main.scene.ron", scene, &outside);
    assert!(matches!(refused, Err(ArchiveError::OutsideProject(_))), "{refused:?}");
}

#[test]
fn import_refuses_a_final_sidecar_without_its_midi_file_and_a_mid_that_is_not_midi() {
    let manifest = valid_manifest_ron("songs");
    let orphan = create_test_zip(&[("project.ron", manifest.as_bytes()), ("assets/music/theme.mid.ron", SIDECAR)]);
    let refused = import_project(&orphan, "v1");
    assert!(
        matches!(&refused, Err(ArchiveError::InvalidSong(reason)) if reason.contains("theme.mid is not in the archive")),
        "{refused:?}"
    );

    let not_midi = create_test_zip(&[("project.ron", manifest.as_bytes()), ("assets/music/theme.mid", b"RIFF")]);
    let refused = import_project(&not_midi, "v1");
    assert!(matches!(&refused, Err(ArchiveError::InvalidSong(reason)) if reason.contains("MThd")), "{refused:?}");

    let bad_sidecar = create_test_zip(&[
        ("project.ron", manifest.as_bytes()),
        ("assets/music/theme.mid", MIDI),
        ("assets/music/theme.mid.ron", b"SongFile(version: 9)"),
    ]);
    assert!(matches!(import_project(&bad_sidecar, "v1"), Err(ArchiveError::InvalidSong(_))));
}

#[test]
fn import_takes_a_lone_journal_sidecar_and_leaves_out_a_truncated_one_with_a_note() {
    let manifest = valid_manifest_ron("songs");
    let lone = create_test_zip(&[("project.ron", manifest.as_bytes()), ("assets/music/theme.mid.ron.new", SIDECAR)]);
    let (_, files) = import_project(&lone, "v1").unwrap();
    assert_eq!(paths(&files), ["assets/music/theme.mid.ron.new"], "a journal file needs no pair");

    let mut entries = vec![
        ("assets/music/theme.mid.new".to_string(), b"MTh".to_vec()),
        ("assets/music/theme.mid.ron.new".to_string(), b"SongFile(vers".to_vec()),
        ("assets/music/theme.mid".to_string(), MIDI.to_vec()),
    ];
    let notes = songs::check_song_entries(&mut entries).expect("journal leftovers never refuse");
    assert_eq!(entries.len(), 1, "both truncated journal files are left out");
    assert_eq!(notes.len(), 2);
    assert!(notes.iter().all(|note| note.contains("was left out")), "{notes:?}");

    let truncated = create_test_zip(&[
        ("project.ron", manifest.as_bytes()),
        ("assets/music/theme.mid", MIDI),
        ("assets/music/theme.mid.ron", SIDECAR),
        ("assets/music/theme.mid.new", b"MTh"),
    ]);
    let (_, files) = import_project(&truncated, "v1").unwrap();
    assert_eq!(paths(&files), ["assets/music/theme.mid", "assets/music/theme.mid.ron"]);
}
