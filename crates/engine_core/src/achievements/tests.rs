//! Achievement persistence contracts: the save file survives a process
//! boundary, two writers merge instead of clobbering, the save is atomic,
//! and a bad path fails without taking the in-memory state down.

use super::*;
use tempfile::tempdir;

fn sample() -> Achievement {
    Achievement::new("test_id", "Test Achievement", "Do the test thing")
}

fn second() -> Achievement {
    Achievement::new("second", "Second", "Do it again")
}

#[test]
fn persistence_round_trip() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ach.json");

    // A missing save file is the first-run case, not an error.
    let fresh = AchievementManager::with_save_path(&path);
    assert_eq!(fresh.unlocked_count(), 0);

    {
        let mut manager = AchievementManager::with_save_path(&path);
        manager.set_toast_duration(2.0);
        manager.register(sample());
        manager.register(second());

        // Unlock semantics: first time true, repeats false, unregistered
        // ids false — and exactly one toast per real unlock.
        assert!(manager.unlock("test_id"));
        assert!(!manager.unlock("test_id"));
        assert!(!manager.unlock("never_registered"));
        assert_eq!(manager.unlocked_count(), 1);
        assert_eq!(manager.toasts.len(), 1);

        // A toast expires once its duration has ticked away.
        manager.tick(1.0);
        assert_eq!(manager.toasts.len(), 1);
        manager.tick(1.5);
        assert_eq!(manager.toasts.len(), 0);
    }

    let mut restored = AchievementManager::with_save_path(&path);
    restored.register(sample());
    restored.register(second());
    assert!(restored.is_unlocked("test_id"));
    assert!(!restored.is_unlocked("second"));
    assert_eq!(restored.unlocked_count(), 1);
}

#[test]
fn concurrent_managers_merge_unlocks_instead_of_clobbering() {
    // Two managers on the same slot = the browser's two-tabs scenario.
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ach.json");

    let mut tab_a = AchievementManager::with_save_path(&path);
    tab_a.register(sample());
    tab_a.register(second());
    let mut tab_b = AchievementManager::with_save_path(&path);
    tab_b.register(sample());
    tab_b.register(second());

    tab_a.unlock("test_id"); // writes {test_id}
    tab_b.unlock("second"); // writes last, must merge {test_id} back in

    let restored = AchievementManager::with_save_path(&path);
    assert!(restored.is_unlocked("test_id"), "tab A's unlock must survive tab B's save");
    assert!(restored.is_unlocked("second"));
}

#[test]
fn save_leaves_no_temp_file_behind() {
    let dir = tempdir().expect("tempdir");
    // Parent directories are created on demand (`saves/` on first run).
    let path = dir.path().join("nested/subdir/ach.json");
    let mut manager = AchievementManager::with_save_path(&path);
    manager.register(sample());

    manager.unlock("test_id");

    assert!(path.exists(), "unlock writes through to the save file");
    assert!(
        !path.with_extension("json.tmp").exists(),
        "atomic save must rename the temp file away"
    );

    // An explicit reset overwrites despite merge-on-save.
    manager.reset();
    assert_eq!(manager.unlocked_count(), 0);
    assert_eq!(manager.toasts.len(), 0);
    let restored = AchievementManager::with_save_path(&path);
    assert_eq!(restored.unlocked_count(), 0, "an explicit clear must actually clear");
}

#[test]
fn save_to_unwritable_path_errors_without_panicking() {
    let dir = tempdir().expect("tempdir");
    // Make the intended parent directory an existing FILE so both
    // create_dir_all and the temp-file write must fail.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("write blocker");
    let path = blocker.join("ach.json");

    let mut manager = AchievementManager::with_save_path(&path);
    manager.register(sample());
    // unlock() triggers a save internally; failure is logged, not panicked.
    manager.unlock("test_id");

    assert!(manager.save().is_err());
    assert!(manager.is_unlocked("test_id"), "in-memory state must survive");
}

#[test]
fn custom_toast_style_drives_the_drawn_panel_size() {
    let mut manager = AchievementManager::in_memory();
    manager.set_toast_style(ToastStyle { width: 240.0, height: 48.0, ..ToastStyle::default() });
    manager.register(sample());
    manager.unlock("test_id");

    let mut ui = ui::UIContext::new();
    ui.begin_frame(&input::InputHandler::new(), Vec2::new(800.0, 600.0));
    manager.draw_toasts(&mut ui, Vec2::new(800.0, 600.0));
    ui.end_frame();

    let panel = ui
        .draw_list()
        .commands()
        .iter()
        .find_map(|command| match command {
            ui::DrawCommand::Rect { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("the toast draws a panel");
    assert_eq!((panel.width, panel.height), (240.0, 48.0));
    assert_eq!(panel.x, 800.0 - 240.0 - manager.toast_style().margin, "anchored to the top-right corner");
}

#[test]
fn hidden_achievement_stays_hidden_and_unlocked_across_a_save_round_trip() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ach.json");
    {
        let mut manager = AchievementManager::with_save_path(&path);
        manager.register(Achievement::new("secret", "Secret", "Find the thing").hidden());
        manager.unlock("secret");
    }

    let mut restored = AchievementManager::with_save_path(&path);
    restored.register(Achievement::new("secret", "Secret", "Find the thing").hidden());

    assert!(restored.get("secret").expect("registered").hidden);
    assert!(restored.is_unlocked("secret"));
}

#[test]
fn all_yields_registration_order_and_reregistering_keeps_the_place() {
    let mut manager = AchievementManager::in_memory();
    manager.register(Achievement::new("first", "First", "First description"));
    manager.register(Achievement::new("second", "Second", "Second description"));
    manager.register(Achievement::new("third", "Third", "Third description"));

    manager.register(Achievement::new("first", "First Updated", "New description"));

    let ids: Vec<&str> = manager.all().map(|achievement| achievement.id.as_str()).collect();
    assert_eq!(ids, vec!["first", "second", "third"]);
    assert_eq!(
        manager.get("first").map(|achievement| achievement.name.as_str()),
        Some("First Updated")
    );
}

#[test]
fn manifest_json_is_the_documented_array_in_registration_order() {
    let mut manager = AchievementManager::in_memory();
    manager.register(Achievement::new("first", "First Name", "First description."));
    manager.register(Achievement::new("second", "Second Name", "Second description.").hidden());

    let json = manager.manifest_json().expect("manifest_json succeeds");
    let expected = "[\n  {\n    \"id\": \"first\",\n    \"name\": \"First Name\",\n    \"description\": \"First description.\",\n    \"hidden\": false\n  },\n  {\n    \"id\": \"second\",\n    \"name\": \"Second Name\",\n    \"description\": \"Second description.\",\n    \"hidden\": true\n  }\n]\n";
    assert_eq!(json, expected);
}

#[test]
fn manifest_export_path_reads_the_value_after_the_flag() {
    let args = ["--other", "--achievements-manifest", "out/a.json", "tail"]
        .into_iter()
        .map(OsString::from);
    let path = manifest_export_path_from(args).expect("manifest_export_path_from succeeds");
    assert_eq!(path, Some(PathBuf::from("out/a.json")));
}

#[test]
fn manifest_export_path_reads_the_equals_form() {
    let args = ["--achievements-manifest=out/a.json"]
        .into_iter()
        .map(OsString::from);
    let path = manifest_export_path_from(args).expect("manifest_export_path_from succeeds");
    assert_eq!(path, Some(PathBuf::from("out/a.json")));
}

#[test]
fn manifest_export_path_is_none_without_the_flag() {
    let empty_args = std::iter::empty::<OsString>();
    assert_eq!(
        manifest_export_path_from(empty_args).expect("manifest_export_path_from succeeds"),
        None
    );

    let other_args = ["--other"].into_iter().map(OsString::from);
    assert_eq!(
        manifest_export_path_from(other_args).expect("manifest_export_path_from succeeds"),
        None
    );
}

#[test]
fn manifest_export_path_refuses_a_flag_without_a_value() {
    let no_value = ["--achievements-manifest"].into_iter().map(OsString::from);
    assert!(matches!(
        manifest_export_path_from(no_value),
        Err(ManifestFlagError)
    ));

    let followed_by_flag = ["--achievements-manifest", "--verbose"]
        .into_iter()
        .map(OsString::from);
    assert!(matches!(
        manifest_export_path_from(followed_by_flag),
        Err(ManifestFlagError)
    ));

    let empty_equals = ["--achievements-manifest="].into_iter().map(OsString::from);
    assert!(matches!(
        manifest_export_path_from(empty_equals),
        Err(ManifestFlagError)
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn write_manifest_creates_the_parent_directory() {
    let mut manager = AchievementManager::in_memory();
    manager.register(Achievement::new("first", "First", "Desc 1"));
    manager.register(Achievement::new("second", "Second", "Desc 2"));

    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("nested").join("deep").join("manifest.json");

    assert!(!manifest_path.parent().expect("parent").exists());

    manager.write_manifest(&manifest_path).expect("write manifest");

    assert!(manifest_path.exists());
    let content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let parsed: Vec<Achievement> = serde_json::from_str(&content).expect("parse manifest");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].id, "first");
    assert_eq!(parsed[1].id, "second");
}
