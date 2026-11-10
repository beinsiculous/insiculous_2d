    use super::*;
    use std::fs;

    fn touch(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b"x")
    }

    fn image_entry(name: &str) -> AssetEntry {
        AssetEntry {
            name: name.into(),
            relative_path: name.into(),
            kind: AssetKind::Image,
            texture_handle: None,
            load_failed: false,
            has_sidecar: false, image_size: None,
        }
    }

    #[test]
    fn test_nested_images_and_scenes_listed_with_slash_joined_relative_paths_while_txt_is_ignored() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        for file in [
            "player.png",
            "brick.JPG",
            "sprites/nested/coin.png",
            "scenes/level1.scene.ron",
            "scripts/paddle.rhai",
            "scripts/main.rs",
            "sounds/jump.WAV",
            "music/theme.ogg",
            "fonts/font.ttf",
            "notes.txt",
        ] {
            touch(&dir.path().join(file))?;
        }

        let entries = scan_assets(dir.path(), &[]).files;

        let listed: Vec<(&str, AssetKind, &str)> = entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind, entry.relative_path.as_str()))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("brick.JPG", AssetKind::Image, "brick.JPG"),
                ("coin.png", AssetKind::Image, "sprites/nested/coin.png"),
                ("player.png", AssetKind::Image, "player.png"),
                ("level1.scene.ron", AssetKind::Scene, "scenes/level1.scene.ron"),
                ("main.rs", AssetKind::Script, "scripts/main.rs"),
                ("paddle.rhai", AssetKind::Script, "scripts/paddle.rhai"),
                ("jump.WAV", AssetKind::Sound, "sounds/jump.WAV"),
                ("theme.ogg", AssetKind::Sound, "music/theme.ogg"),
            ],
            "images first by name, then scenes, then scripts, then sounds; nested files have forward-slash relative paths; txt is ignored"
        );
        assert!(
            scan_assets(Path::new("/definitely/not/a/real/dir"), &[]).files.is_empty(),
            "a missing folder is an empty browser, not a crash"
        );
        Ok(())
    }

    #[test]
    fn test_kind_for_path_classifies_rhai_and_rs_as_script() {
        let kind = |path: &str| kind_for_path(Path::new(path), &[]);
        assert_eq!(kind("a.rhai"), Some(AssetKind::Script));
        assert_eq!(kind("a.RS"), Some(AssetKind::Script));
        assert_eq!(kind("a.png"), Some(AssetKind::Image));
        assert_eq!(kind("a.ron"), Some(AssetKind::Scene));
        assert_eq!(kind("a.FLAC"), Some(AssetKind::Sound));
        assert_eq!(kind("a.mid"), Some(AssetKind::Song), "a song is listed as a song, not a recorded sound");
        assert_eq!(kind("a.SF2"), Some(AssetKind::SoundFont));
        assert_eq!(kind("a.txt"), None);
    }

    #[test]
    fn test_every_song_and_daw_project_extension_reaches_a_tile() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let daw_project_extensions = vec![".evil".to_string(), "PRJ".to_string(), ".nsp".to_string()];
        for file in ["music/a.mid", "music/b.MIDI", "music/c.evil", "music/d.prj", "music/e.NSP", "music/f.als"] {
            touch(&dir.path().join(file))?;
        }

        let entries = scan_assets(dir.path(), &daw_project_extensions).files;

        let listed: Vec<(&str, AssetKind)> = entries.iter().map(|entry| (entry.name.as_str(), entry.kind)).collect();
        assert_eq!(
            listed,
            vec![
                ("a.mid", AssetKind::Song),
                ("b.MIDI", AssetKind::Song),
                ("c.evil", AssetKind::DawProject),
                ("d.prj", AssetKind::DawProject),
                ("e.NSP", AssetKind::DawProject),
            ],
            "both MIDI extensions are songs; each listed DAW extension, dotted or not, is a tile; an unlisted one is not"
        );
        Ok(())
    }

    #[test]
    fn test_apply_scan_preserves_loaded_handles_by_path() {
        let mut state = AssetBrowserState::default();
        state.apply_scan(AssetScan { files: vec![image_entry("a.png")], folders: Vec::new() });
        state.entries[0].texture_handle = Some(5);

        // A rescan finds the same file plus a new one.
        state.apply_scan(AssetScan { files: vec![image_entry("a.png"), image_entry("b.png")], folders: Vec::new() });

        assert_eq!(state.entries[0].texture_handle, Some(5), "a rescan must not reload what is already on the GPU");
        assert_eq!(state.entries[1].texture_handle, None, "a new entry starts unloaded");
        assert!(state.scanned);
    }

    fn browser_of(count: usize, columns: usize) -> AssetBrowserState {
        let mut state = AssetBrowserState::default();
        state.apply_scan(AssetScan { files: (0..count).map(|index| image_entry(&format!("tile_{index}.png"))).collect(), folders: Vec::new() });
        state.columns = columns;
        state
    }

    #[test]
    fn test_a_double_click_files_one_request_for_the_tiles_path_and_a_triple_press_still_one() {
        let mut state = browser_of(2, 2);
        let start = Instant::now();
        let at = Vec2::new(50.0, 50.0);
        let presses: Vec<Option<OpenRequest>> = (0..3)
            .map(|press| state.press_tile(&TileId::file("tile_1.png"), start + std::time::Duration::from_millis(press * 150), at))
            .collect();
        assert_eq!(presses, vec![None, Some(open_file_request("tile_1.png")), None]);
        assert_eq!(presses[1].as_ref().map(|request| &request.location), Some(&Location::None));
    }

    #[test]
    fn test_a_press_on_another_tile_or_after_a_drag_started_files_nothing() {
        let mut state = browser_of(2, 2);
        let start = Instant::now();
        let at = Vec2::new(50.0, 50.0);
        state.press_tile(&TileId::file("tile_0.png"), start, at);
        assert_eq!(state.press_tile(&TileId::file("tile_1.png"), start, at), None, "the second press was on another tile");

        state.press_tile(&TileId::file("tile_0.png"), start, at);
        state.last_press.forget();
        assert_eq!(state.press_tile(&TileId::file("tile_0.png"), start, at), None, "a drag started between the presses");
    }

    #[test]
    fn test_the_arrows_step_a_tile_and_a_row_and_stop_at_the_grids_edges() {
        let mut state = browser_of(5, 2);
        state.step_selection(TileStep::Down);
        assert_eq!(state.selected_index(), Some(0), "with nothing selected the first tile is");
        let steps = [TileStep::Right, TileStep::Down, TileStep::Down, TileStep::Right, TileStep::Up, TileStep::Left];
        let walked: Vec<Option<usize>> = steps
            .iter()
            .map(|&step| {
                state.step_selection(step);
                state.selected_index()
            })
            .collect();
        assert_eq!(
            walked,
            vec![Some(1), Some(3), Some(3), Some(4), Some(2), Some(1)],
            "Down from the last full row stays when no tile is under it; Right wraps into the next row"
        );
        assert!(state.reveal_selected, "a step asks the render to bring the tile into view");
    }

    #[test]
    fn test_a_folder_request_names_the_tiles_folder_and_the_root_for_a_top_level_file() {
        assert_eq!(open_folder_request("sprites/nested/coin.png").path, PathBuf::from("sprites/nested"));
        assert_eq!(open_folder_request("hero.png").path, PathBuf::from("."));
    }

    #[test]
    fn test_thumbnail_fit_preserves_aspect_and_centers_in_the_slot() {
        let slot = common::Rect::new(10.0, 10.0, 64.0, 64.0);

        let wide = fit_rect(128, 64, slot);
        assert_eq!((wide.width, wide.height, wide.y), (64.0, 32.0, 26.0), "width-bound, vertically centered");
        let tall = fit_rect(32, 64, slot);
        assert_eq!((tall.width, tall.height, tall.x), (32.0, 64.0, 26.0), "height-bound, horizontally centered");
        assert_eq!(fit_rect(0, 10, slot), slot, "degenerate sizes fall back to the slot");
    }

#[test]
fn folders_precede_root_files_and_companions_and_markers_never_become_tiles() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    for name in ["sprites/props/hero.png", "sprites/props/hero.sheet.ron", "music/theme.midi", "music/theme.midi.ron", "scenes/levels/.keep", "root.png", "orphan.sheet.ron"] {
        touch(&directory.path().join(name))?;
    }
    let mut browser = AssetBrowserState::default();
    browser.apply_scan(scan_assets(directory.path(), &[]));
    let names: Vec<_> = browser.visible_tiles().into_iter().map(|tile| tile.name).collect();
    assert_eq!(names, ["music", "scenes", "sprites", "root.png", "orphan.sheet.ron"]);
    browser.enter("sprites/props");
    assert_eq!(browser.visible_tiles().iter().map(|tile| tile.name.as_str()).collect::<Vec<_>>(), ["..", "hero.png"]);
    browser.selected = Some(TileId::file("sprites/props/hero.png"));
    assert!(browser.selected_entry().unwrap().has_sidecar);
    std::fs::remove_file(directory.path().join("sprites/props/hero.sheet.ron"))?;
    browser.apply_scan(scan_assets(directory.path(), &[]));
    assert_eq!(browser.selected, Some(TileId::file("sprites/props/hero.png")));
    assert!(!browser.selected_entry().unwrap().has_sidecar);
    browser.enter("music");
    assert_eq!(browser.visible_tiles().len(), 2);
    assert_eq!(kind_for_path(Path::new("theme.MIDI.ron"), &[]), Some(AssetKind::Companion));
    Ok(())
}

#[test]
fn folder_entry_parent_and_a_missing_saved_folder_keep_navigation_inside_the_tree() -> std::io::Result<()> {
    let directory = tempfile::tempdir()?;
    let deep = "sprites/one/two/three/four/five/six/seven";
    std::fs::create_dir_all(directory.path().join(deep))?;
    let mut browser = AssetBrowserState::default();
    browser.apply_scan(scan_assets(directory.path(), &[]));
    browser.enter(deep);
    assert_eq!(browser.current_folder, deep);
    browser.selected = Some(TileId::folder("sprites/one/two/three/four/five/six"));
    assert!(browser.open_selected().is_none());
    assert_eq!(browser.current_folder, "sprites/one/two/three/four/five/six");
    browser.up();
    assert_eq!(browser.current_folder, "sprites/one/two/three/four/five");
    browser.current_folder = "sprites/gone".into();
    browser.apply_scan(scan_assets(directory.path(), &[]));
    assert_eq!(browser.current_folder, "");
    browser.enter("../../outside");
    assert_eq!(browser.current_folder, "");
    Ok(())
}

#[test]
fn new_names_follow_the_kind_root_and_companions_follow_their_primary() {
    assert_eq!(route_new_file("scenes/levels", AssetKind::Scene, "level_two.scene.ron"), "scenes/levels/level_two.scene.ron");
    assert_eq!(route_new_file("sprites", AssetKind::Scene, "level_two.scene.ron"), "scenes/level_two.scene.ron");
    assert_eq!(route_new_file("scenes_extra", AssetKind::Scene, "level_two.scene.ron"), "scenes/level_two.scene.ron");
    assert_eq!(companion_path("sprites/props/hero.png"), Some("sprites/props/hero.sheet.ron".into()));
    assert_eq!(companion_path("music/nested/theme.midi"), Some("music/nested/theme.midi.ron".into()));
    assert_eq!(validate_name("level_two", AssetKind::Scene).unwrap(), "level_two.scene.ron");
    assert_eq!(validate_name("level_two.scene.ron", AssetKind::Scene).unwrap(), "level_two.scene.ron");
    for invalid in ["Level", "has space", "non_áscii", "_leading", "trailing_", "two__words"] {
        let message = validate_name(invalid, AssetKind::Scene).unwrap_err();
        assert!(message.contains("ASCII snake_case"), "{invalid}: {message}");
    }
    assert!(validate_name("level.one.scene.ron", AssetKind::Scene).unwrap_err().contains("one dot"));
    assert!(validate_folder("sprites", "scenes").unwrap_err().contains("kind folders"));
    assert!(validate_folder("sprites", "props").is_ok());
    assert!(validate_folder("", "scenes").is_ok());
    assert_eq!(validate_name("BlackSamsGold-ej5e.ttf", AssetKind::Font).unwrap(), "BlackSamsGold-ej5e.ttf");
}
