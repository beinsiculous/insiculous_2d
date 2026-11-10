use super::*;

#[test]
fn asset_families_keep_existing_png_and_midi_companions_and_recovered_siblings() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    for (primary, companion) in [("hero.png", "hero.sheet.ron"), ("theme.mid", "theme.mid.ron"), ("march.midi", "march.midi.ron")] {
        let stem = Path::new(primary).file_stem().unwrap().to_str().unwrap();
        let suffix = &primary[stem.len()..];
        let recovered = format!("{stem}.recovered-20261106{suffix}");
        let recovered_companion = companion_path(&recovered).unwrap();
        let notes = format!("{stem}.recovered-notes.txt");
        for name in [primary, companion, &recovered, &recovered_companion, &notes, "unrelated.recovered-20261106.mid"] {
            std::fs::write(directory.path().join(name), b"asset")?;
        }
        std::fs::create_dir_all(directory.path().join(format!("{stem}.recovered-folder{suffix}")))?;
        let family = asset_family(directory.path().join(primary).to_str().unwrap())?;
        let expected: Vec<String> = [primary, companion, &recovered, &recovered_companion].into_iter()
            .map(|name| directory.path().join(name).to_string_lossy().into_owned()).collect();
        assert_eq!(family, expected, "a recovered copy carries the asset's own extension and brings its companion; a note or a folder under the prefix is not family");
    }
    std::fs::write(directory.path().join("loud.PNG"), b"image")?;
    std::fs::write(directory.path().join("loud.recovered-1.png"), b"image")?;
    let loud = directory.path().join("loud.PNG");
    assert_eq!(asset_family(loud.to_str().unwrap())?.len(), 2, "a recovered copy's extension is matched in any case, as the scanner classes files");
    std::fs::write(directory.path().join("Loud.recovered-2.png"), b"another asset's copy")?;
    assert_eq!(asset_family(loud.to_str().unwrap())?.len(), 2, "the stem is matched as spelled: Loud is another asset");
    std::fs::write(directory.path().join("shout.png"), b"image")?;
    std::fs::write(directory.path().join("shout.SHEET.RON"), b"sheet")?;
    let shout = directory.path().join("shout.png");
    assert_eq!(asset_family(shout.to_str().unwrap())?, [shout.to_string_lossy().into_owned(), directory.path().join("shout.SHEET.RON").to_string_lossy().into_owned()], "a companion spelled in another case is the companion the scanner classes it as");
    std::fs::write(directory.path().join("hero.recovered-😀a"), b"a name with a multibyte char where the suffix would be cut")?;
    std::fs::write(directory.path().join("éa.sheet.ron"), b"a sidecar whose name is another's byte length")?;
    std::fs::write(directory.path().join("a.x.png"), b"image")?;
    let hero = directory.path().join("hero.png");
    assert!(!asset_family(hero.to_str().unwrap())?.iter().any(|member| member.contains("😀")), "a multibyte name near the suffix is compared, never sliced");
    assert!(!tree::same_companion_name("a.x.sheet.ron", "éa.sheet.ron"));
    std::fs::write(directory.path().join("shout.sheet.ron"), b"a second spelling")?;
    assert!(asset_family(shout.to_str().unwrap()).is_err(), "two spellings of one companion refuse the family");
    std::fs::remove_file(directory.path().join("shout.sheet.ron"))?;
    std::fs::write(directory.path().join("SHOUT.PNG"), b"another asset")?;
    let other = directory.path().join("SHOUT.PNG");
    assert_eq!(asset_family(other.to_str().unwrap())?, [other.to_string_lossy()], "the stem as spelled: shout's sheet is not SHOUT's");
    std::fs::write(directory.path().join("boxed.png"), b"image")?;
    std::fs::create_dir(directory.path().join("boxed.sheet.ron"))?;
    let boxed = directory.path().join("boxed.png");
    assert_eq!(asset_family(boxed.to_str().unwrap())?, [boxed.to_string_lossy()], "a directory wearing the sidecar's name is no companion");
    std::fs::write(directory.path().join("unpaired.jpg"), b"image")?;
    let unpaired = directory.path().join("unpaired.jpg");
    assert_eq!(asset_family(unpaired.to_str().unwrap())?, [unpaired.to_string_lossy()]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_unlistable_folder_refuses_the_family_instead_of_hiding_recovered_files() -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("hero.png"), b"image")?;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o300))?;
    let family = asset_family(directory.path().join("hero.png").to_str().unwrap());
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
    assert!(family.is_err(), "a folder that cannot be read for recovered siblings is an error, not an empty family");
    std::fs::write(directory.path().join("hero.sheet.ron"), b"sheet")?;
    std::fs::set_permissions(directory.path().join("hero.sheet.ron"), std::fs::Permissions::from_mode(0o000))?;
    let family = asset_family(directory.path().join("hero.png").to_str().unwrap());
    std::fs::set_permissions(directory.path().join("hero.sheet.ron"), std::fs::Permissions::from_mode(0o644))?;
    assert!(family.is_err(), "a companion that cannot be read is an error, not an absent one");
    Ok(())
}

#[test]
fn rename_names_keep_midi_ogg_jpg_and_font_extensions_and_reject_invalid_stems() {
    for (kind, extension, name) in [
        (AssetKind::Song, ".midi", "march.midi"),
        (AssetKind::Sound, "ogg", "jump.ogg"),
        (AssetKind::Image, ".jpg", "portrait.jpg"),
        (AssetKind::Scene, ".scene.ron", "level_two.scene.ron"),
        (AssetKind::Font, ".ttf", "BlackSamsGold-ej5e.ttf"),
        (AssetKind::SoundFont, "sf2", "GeneralUser-GS.sf2"),
    ] {
        assert_eq!(validate_rename(name, kind, extension).unwrap(), name);
        let stem = name.strip_suffix(&format!(".{}", extension.trim_start_matches('.'))).unwrap();
        assert_eq!(validate_rename(stem, kind, extension).unwrap(), name);
    }
    for invalid in ["March.midi", "has space.midi", "two__words.midi"] {
        assert!(validate_rename(invalid, AssetKind::Song, ".midi").unwrap_err().contains("ASCII snake_case"));
    }
    assert!(validate_rename("jump.wav", AssetKind::Sound, ".ogg").is_err());
    assert!(validate_rename("../song.midi", AssetKind::Song, ".midi").is_err());
    assert!(validate_rename("orphan.sheet.ron", AssetKind::Companion, ".sheet.ron").unwrap_err().contains("paired by adding its primary"));
    assert!(validate_rename("My Bank", AssetKind::SoundFont, "sf2").unwrap_err().contains("whitespace"), "a SoundFont keeps its spelling but never a space");
}

#[test]
fn an_orphan_companion_is_a_visible_companion_until_its_primary_arrives() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("hero.sheet.ron"), b"sheet")?;
    let mut browser = AssetBrowserState::default();
    browser.apply_scan(scan_assets(directory.path(), &[]));
    assert_eq!(browser.visible_tiles().into_iter().map(|tile| tile.id).collect::<Vec<_>>(), [TileId::file("hero.sheet.ron")]);
    browser.selected = Some(TileId::file("hero.sheet.ron"));
    assert_eq!(browser.selected_entry().unwrap().kind, AssetKind::Companion, "the host draws the orphan badge from this kind");
    std::fs::write(directory.path().join("hero.png"), b"image")?;
    browser.apply_scan(scan_assets(directory.path(), &[]));
    assert_eq!(browser.visible_tiles().into_iter().map(|tile| tile.id).collect::<Vec<_>>(), [TileId::file("hero.png")]);
    assert!(browser.selected.is_none(), "a now-hidden companion cannot stay selected");
    Ok(())
}
