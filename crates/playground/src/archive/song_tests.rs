//! An archive's songs: Export carries the open song's live files over its saved ones and
//! every other file unchanged; import refuses a sidecar without its MIDI file and a `.mid`
//! that is not MIDI, and takes a `.new` an older project left as any other file.

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
fn export_carries_the_live_song_over_its_saved_files_and_every_other_file_unchanged() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();
    let manifest = two_scene_project(root);
    let music = root.join("assets/music");
    std::fs::create_dir_all(&music).unwrap();
    for (name, bytes) in [
        ("theme.mid", MIDI),
        ("theme.mid.ron", SIDECAR),
        ("theme.mid.new", MIDI),
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
            "assets/music/theme.mid.new",
            "assets/music/theme.mid.ron",
        ],
        "a file an older project's save left is carried as any other file"
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
fn import_takes_a_new_file_an_older_project_left_as_any_other_file_whatever_it_holds() {
    let manifest = valid_manifest_ron("songs");
    let leftovers = create_test_zip(&[
        ("project.ron", manifest.as_bytes()),
        ("assets/music/theme.mid", MIDI),
        ("assets/music/theme.mid.ron", SIDECAR),
        ("assets/music/theme.mid.new", b"MTh"),
        ("assets/music/theme.mid.ron.new", b"SongFile(vers"),
    ]);
    let (_, files) = import_project(&leftovers, "v1").unwrap();
    assert_eq!(
        paths(&files),
        [
            "assets/music/theme.mid",
            "assets/music/theme.mid.new",
            "assets/music/theme.mid.ron",
            "assets/music/theme.mid.ron.new",
        ],
        "a truncated .new is no song's file, so it neither refuses the archive nor is left out"
    );
}
