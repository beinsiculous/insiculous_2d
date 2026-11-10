use super::*;

fn request(path: &str, name: &str, kind: AssetKind) -> RenameRequest {
    RenameRequest { tile: TileId::file(path), name: name.into(), kind }
}

#[test]
fn rename_moves_the_pair_and_rolls_back_when_the_sidecar_step_fails() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sprites/hero.png"), b"image").unwrap();
    vfs::write(&base.join("sprites/hero.sheet.ron"), b"sheet").unwrap();
    let plan = prepare(base, &request("sprites/hero.png", "deion", AssetKind::Image)).unwrap();
    let files: Vec<_> = plan.files.iter().map(|(old, new)| (base.join(old), base.join(new))).collect();
    let error = move_family_with(&files, |old, new| {
        if old.ends_with("hero.sheet.ron") { return Err(std::io::Error::other("injected sidecar failure")); }
        vfs::rename(old, new)
    }).err().unwrap();
    assert!(error.message.contains("injected sidecar failure"));
    assert!(vfs::exists(&base.join("sprites/hero.png")));
    assert!(!vfs::exists(&base.join("sprites/deion.png")));
    assert!(execute(base, &plan).is_ok());
    assert_eq!(vfs::read(&base.join("sprites/deion.png")).unwrap(), b"image");
    assert_eq!(vfs::read(&base.join("sprites/deion.sheet.ron")).unwrap(), b"sheet");
    assert!(!vfs::exists(&base.join("sprites/hero.png")));
}

#[test]
fn reference_scan_names_nested_scenes_scripts_and_song_sidecars() {
    let directory = tempfile::tempdir().unwrap();
    for path in ["scenes/levels/one.scene.ron", "scripts/nested/spawn.rhai", "music/theme.mid.ron"] {
        vfs::write(&directory.path().join(path), b"sprites/hero.png").unwrap();
    }
    vfs::write(&directory.path().join("scenes/other.scene.ron"), b"backdrops/sprites/sky.png").unwrap();
    vfs::write(&directory.path().join("scenes/LOUD.SCENE.RON"), b"sprites/hero.png").unwrap();
    assert!(reference_hits(directory.path(), &["sprites/hero.png"], ReferenceScope::Files, &[]).unwrap().contains(&"scenes/LOUD.SCENE.RON".to_string()), "an uppercase extension is scanned as the scanner classes it");
    std::fs::remove_file(directory.path().join("scenes/LOUD.SCENE.RON")).unwrap();
    vfs::write(&directory.path().join("sprites/hero.sheet.ron"), b"sprites/hero.png names itself").unwrap();
    assert!(!reference_hits(directory.path(), &["sprites/hero.png", "sprites/hero.sheet.ron"], ReferenceScope::Files, &[]).unwrap().iter().any(|hit| hit.contains("hero.sheet.ron")), "the family is no reference of itself");
    std::fs::remove_file(directory.path().join("sprites/hero.sheet.ron")).unwrap();
    vfs::write(&directory.path().join("notes.txt"), b"sprites/hero.png").unwrap();
    assert_eq!(reference_hits(directory.path(), &["s"], ReferenceScope::DeletedFolder, &[]).unwrap(), Vec::<String>::new(), "a folder matches at a path boundary only");
    assert_eq!(reference_hits(directory.path(), &["sprites"], ReferenceScope::DeletedFolder, &[]).unwrap().len(), 3, "`backdrops/sprites/` is another folder");
    vfs::write(&directory.path().join("sprites/props.scene.ron"), b"Sprite(texture: \"sprites/hero.png\")").unwrap();
    assert_eq!(reference_hits(directory.path(), &["sprites"], ReferenceScope::DeletedFolder, &[]).unwrap().len(), 3, "a deleted folder's own files go with it and reference nothing");
    assert!(reference_hits(directory.path(), &["sprites"], ReferenceScope::RenamedFolder, &[]).unwrap().contains(&"sprites/props.scene.ron".to_string()), "a renamed folder's own file names the old path still after the move");
    std::fs::remove_file(directory.path().join("sprites/props.scene.ron")).unwrap();
    vfs::write(&directory.path().join("scenes/recovered.scene.ron"), b"music/theme.recovered-1.mid").unwrap();
    assert_eq!(reference_hits(directory.path(), &["music/theme.mid"], ReferenceScope::Files, &[]).unwrap(), Vec::<String>::new());
    assert_eq!(reference_hits(directory.path(), &["music/theme.mid", "music/theme.recovered-1.mid"], ReferenceScope::Files, &[]).unwrap(), vec!["scenes/recovered.scene.ron"], "a scene naming a recovered sibling alone is a reference of the family");
    assert_eq!(reference_hits(directory.path(), &["sprites/hero.png"], ReferenceScope::Files, &[]).unwrap(), vec![
        "music/theme.mid.ron", "scenes/levels/one.scene.ron", "scripts/nested/spawn.rhai",
    ]);
    let live = [("the open scene (unsaved)".to_string(), Some("Sprite(texture: \"sprites/hero.png\")".to_string()))];
    assert_eq!(reference_hits(directory.path(), &["sprites/hero.png"], ReferenceScope::Files, &live).unwrap()[0], "the open scene (unsaved)");
    let unrendered = [("the open scene (unsaved, not scanned)".to_string(), None)];
    assert_eq!(reference_hits(directory.path(), &["sprites/hero.png"], ReferenceScope::Files, &unrendered).unwrap()[0], "the open scene (unsaved, not scanned)", "a document that could not be rendered is a hit");
}

#[test]
fn a_song_with_samples_refuses_rename_and_sample_tiles_refuse_both_mutations() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("music/ai_demo.mid"), b"score").unwrap();
    vfs::write(&base.join("music/samples/ai_demo/ping.wav"), b"sound").unwrap();
    vfs::write(&base.join("music/samples/ai_demo/LICENSE"), b"Source: music/ai_demo.mid\n").unwrap();
    let error = prepare(base, &request("music/ai_demo.mid", "theme", AssetKind::Song)).err().unwrap();
    assert!(error.contains("music/samples/ai_demo/") && error.contains("Save As"));
    let sample = "music/samples/ai_demo/ping.wav";
    let rename_error = prepare(base, &request(sample, "bell", AssetKind::Sound)).err().unwrap();
    let delete_error = super::super::remove::prepare(base, &TileId::file(sample), &[]).err().unwrap();
    assert!(rename_error.contains("ai_demo.mid") && rename_error.contains("Conductor panel"));
    assert_eq!(delete_error, rename_error);
    let deletion = super::super::remove::prepare(base, &TileId::file("music/ai_demo.mid"), &[]).unwrap();
    assert!(deletion.files.contains(&sample.to_string()));
    assert!(deletion.label().contains("music/samples/ai_demo/"));
    assert_eq!(deletion.sample_folders, vec!["music/samples/ai_demo"]);
    for folder in ["music/samples", "music"] {
        let tile = TileId::folder(folder);
        let folder_rename = prepare(base, &RenameRequest { tile: tile.clone(), name: "archive".into(), kind: AssetKind::Folder }).err().unwrap();
        let folder_delete = super::super::remove::prepare(base, &tile, &[]).err().unwrap();
        assert!(folder_rename.contains("ai_demo.mid") && folder_delete.contains("ai_demo.mid"), "{folder}: {folder_rename} / {folder_delete}");
    }
}

#[test]
fn a_sound_rename_carries_its_license_stanza() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sounds/snd_jump.wav"), b"sound").unwrap();
    vfs::write(&base.join("sounds/LICENSE"), b"Files: snd_jump.wav\nLicense: CC0-1.0\n").unwrap();
    let plan = prepare(base, &request("sounds/snd_jump.wav", "jump.wav", AssetKind::Sound)).unwrap();
    assert!(execute(base, &plan).is_ok());
    assert!(vfs::read_to_string(&base.join("sounds/LICENSE")).unwrap().contains("Files: jump.wav"));
    vfs::write(&base.join("sounds/hop.WAV"), b"sound").unwrap();
    vfs::write(&base.join("sounds/LICENSE"), b"Files: jump.wav hop.WAV\nLicense: CC0-1.0\n").unwrap();
    let plan = prepare(base, &request("sounds/hop.WAV", "leap", AssetKind::Sound)).unwrap();
    assert!(execute(base, &plan).is_ok());
    assert!(vfs::read_to_string(&base.join("sounds/LICENSE")).unwrap().contains("Files: jump.wav leap.WAV"), "an uppercase extension keeps its stanza");
    vfs::write(&base.join("sounds/weather/rain.wav"), b"sound").unwrap();
    vfs::write(&base.join("sounds/LICENSE"), b"Files: jump.wav leap.WAV weather/rain.wav\nLicense: CC0-1.0\n").unwrap();
    let plan = prepare(base, &request("sounds/weather/rain.wav", "storm", AssetKind::Sound)).unwrap();
    assert!(execute(base, &plan).is_ok());
    assert!(vfs::read_to_string(&base.join("sounds/LICENSE")).unwrap().contains("Files: jump.wav leap.WAV weather/storm.wav"), "a nested sound's stanza in the folder above follows, as the gate reads it");
}

#[test]
fn a_folder_rename_keeps_its_own_license_tokens_and_renames_them_in_a_license_above() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sounds/old/hop.wav"), b"sound").unwrap();
    vfs::write(&base.join("sounds/old/LICENSE"), b"Files: hop.wav\nLicense: CC0-1.0\n").unwrap();
    let plan = prepare(base, &RenameRequest { tile: TileId::folder("sounds/old"), name: "fresh".into(), kind: AssetKind::Folder }).unwrap();
    assert_eq!(execute(base, &plan).unwrap(), None);
    assert_eq!(vfs::read_to_string(&base.join("sounds/fresh/LICENSE")).unwrap(), "Files: hop.wav\nLicense: CC0-1.0\n", "a LICENSE that moved with its folder keeps its relative tokens");
    vfs::write(&base.join("music/old/theme.mid"), b"score").unwrap();
    vfs::write(&base.join("music/LICENSE"), b"Files: old/theme.mid\nLicense: CC0-1.0\n").unwrap();
    let plan = prepare(base, &RenameRequest { tile: TileId::folder("music/old"), name: "fresh".into(), kind: AssetKind::Folder }).unwrap();
    assert_eq!(execute(base, &plan).unwrap(), None);
    assert_eq!(vfs::read_to_string(&base.join("music/LICENSE")).unwrap(), "Files: fresh/theme.mid\nLicense: CC0-1.0\n", "a LICENSE above the folder sees its new name");
}

#[cfg(unix)]
#[test]
fn an_unreadable_scene_is_reported_as_a_reference_and_a_folder_in_a_files_place_refuses() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sprites/hero.png"), b"image").unwrap();
    vfs::write(&base.join("scenes/old.scene.ron"), b"sprites/hero.png").unwrap();
    std::fs::set_permissions(base.join("scenes/old.scene.ron"), std::fs::Permissions::from_mode(0o000)).unwrap();
    let hits = reference_hits(base, &["sprites/hero.png"], ReferenceScope::Files, &[]).unwrap();
    assert!(hits.iter().any(|hit| hit.starts_with("scenes/old.scene.ron (could not be read")), "{hits:?}");
    let own = super::super::remove::prepare(base, &TileId::file("scenes/old.scene.ron"), &[]).unwrap();
    assert!(own.references.is_empty(), "the unreadable file can still be deleted: {:?}", own.references);
    std::fs::set_permissions(base.join("scenes/old.scene.ron"), std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::remove_file(base.join("sprites/hero.png")).unwrap();
    std::fs::create_dir(base.join("sprites/hero.png")).unwrap();
    let error = prepare(base, &request("sprites/hero.png", "deion", AssetKind::Image)).err().unwrap();
    assert!(error.contains("is a folder now"), "{error}");
}

#[test]
fn a_sidecar_or_sample_folder_waiting_at_the_new_name_refuses_the_rename() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sprites/hero.png"), b"image").unwrap();
    vfs::write(&base.join("sprites/deion.sheet.ron"), b"another sheet").unwrap();
    let error = prepare(base, &request("sprites/hero.png", "deion", AssetKind::Image)).err().unwrap();
    assert!(error.contains("deion.sheet.ron already exists"), "{error}");
    std::fs::rename(base.join("sprites/deion.sheet.ron"), base.join("sprites/deion.SHEET.RON")).unwrap();
    let error = prepare(base, &request("sprites/hero.png", "deion", AssetKind::Image)).err().unwrap();
    assert!(error.contains("deion.SHEET.RON already exists"), "a sidecar in another case pairs itself all the same: {error}");
    std::fs::remove_file(base.join("sprites/deion.SHEET.RON")).unwrap();
    vfs::write(&base.join("sprites/deion.recovered-1.png"), b"an unrelated recovered copy").unwrap();
    let error = prepare(base, &request("sprites/hero.png", "deion", AssetKind::Image)).err().unwrap();
    assert!(error.contains("deion.recovered-1.png already exists"), "a recovered copy under the new name would join the family: {error}");
    vfs::write(&base.join("music/theme.mid"), b"score").unwrap();
    let stale = editor::song::sample_directory(Path::new("music/anthem.mid")).unwrap();
    vfs::write(&base.join(&stale).join("ping.wav"), b"an earlier song's sample").unwrap();
    let error = prepare(base, &request("music/theme.mid", "anthem", AssetKind::Song)).err().unwrap();
    assert!(error.contains(&stale) && error.contains("another song's samples"), "{error}");
}

#[test]
fn an_uppercase_scene_keeps_its_extension_chain_as_spelled() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("scenes/LEVEL.SCENE.RON"), b"scene").unwrap();
    let plan = prepare(base, &request("scenes/LEVEL.SCENE.RON", "level_two", AssetKind::Scene)).unwrap();
    assert_eq!(plan.destination.path, "scenes/level_two.SCENE.RON");
}

#[test]
fn a_folder_rename_whose_song_keeps_samples_beside_it_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("music/covers/theme.mid"), b"score").unwrap();
    let samples = editor::song::sample_directory(Path::new("music/covers/theme.mid")).unwrap();
    vfs::write(&base.join(&samples).join("ping.wav"), b"sound").unwrap();
    let error = prepare(base, &RenameRequest { tile: TileId::folder("music/covers"), name: "archive".into(), kind: AssetKind::Folder }).err().unwrap();
    assert!(error.contains(&samples) && error.contains("Save As"), "{error}");
}

#[test]
fn a_folder_whose_files_changed_since_the_confirm_does_not_move() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("scripts/old/paddle.rhai"), b"script").unwrap();
    let plan = prepare(base, &RenameRequest { tile: TileId::folder("scripts/old"), name: "fresh".into(), kind: AssetKind::Folder }).unwrap();
    vfs::write(&base.join("scripts/old/new.rhai"), b"arrived after the confirm").unwrap();
    let error = execute(base, &plan).err().unwrap();
    assert!(error.message.contains("changed since the confirm") && error.rescan, "{}", error.message);
    assert!(base.join("scripts/old/new.rhai").exists() && !base.join("scripts/fresh").exists());
}

#[cfg(unix)]
#[test]
fn an_unlistable_folder_is_a_notice_in_the_scan_not_a_veto() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sprites/raw/hidden.png"), b"image").unwrap();
    vfs::write(&base.join("scenes/level.scene.ron"), b"scene").unwrap();
    std::fs::set_permissions(base.join("sprites/raw"), std::fs::Permissions::from_mode(0o000)).unwrap();
    let hits = reference_hits(base, &["scenes/level.scene.ron"], ReferenceScope::Files, &[]).unwrap();
    std::fs::set_permissions(base.join("sprites/raw"), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(hits, vec!["sprites/raw/ (could not be listed)"]);
}

#[cfg(unix)]
#[test]
fn a_read_only_license_whose_tokens_would_not_change_does_not_refuse_a_folder_rename() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sounds/vendor/hop.wav"), b"sound").unwrap();
    let license = base.join("sounds/vendor/LICENSE");
    vfs::write(&license, b"Files: hop.wav\nLicense: CC0-1.0\n").unwrap();
    std::fs::set_permissions(&license, std::fs::Permissions::from_mode(0o444)).unwrap();
    let plan = prepare(base, &RenameRequest { tile: TileId::folder("sounds/vendor"), name: "parts".into(), kind: AssetKind::Folder }).unwrap();
    assert_eq!(execute(base, &plan).unwrap(), None);
    assert_eq!(vfs::read_to_string(&base.join("sounds/parts/LICENSE")).unwrap(), "Files: hop.wav\nLicense: CC0-1.0\n");
}

#[cfg(unix)]
#[test]
fn an_unwritable_license_refuses_before_the_first_file_moves() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    vfs::write(&base.join("sounds/jump.wav"), b"sound").unwrap();
    let license = base.join("sounds/LICENSE");
    vfs::write(&license, b"Files: jump.wav\nLicense: CC0-1.0\n").unwrap();
    std::fs::set_permissions(&license, std::fs::Permissions::from_mode(0o444)).unwrap();
    let result = prepare(base, &request("sounds/jump.wav", "leap", AssetKind::Sound));
    std::fs::set_permissions(&license, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(result.err().unwrap().contains("Cannot update"));
    assert!(vfs::exists(&base.join("sounds/jump.wav")));
    assert!(!vfs::exists(&base.join("sounds/leap.wav")));
}
