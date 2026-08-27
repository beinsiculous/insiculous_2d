//! Tests for the achievement system (hoisted to a child file for size).

use super::*;
use tempfile::tempdir;

fn sample() -> Achievement {
    Achievement::new("test_id", "Test Achievement", "Do the test thing")
}

#[test]
fn in_memory_manager_starts_empty() {
    let mgr = AchievementManager::in_memory();
    assert_eq!(mgr.total(), 0);
    assert_eq!(mgr.unlocked_count(), 0);
}

#[test]
fn register_then_get() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    assert_eq!(mgr.total(), 1);
    assert_eq!(mgr.get("test_id").unwrap().name, "Test Achievement");
}

#[test]
fn unlock_returns_true_first_time_false_after() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    assert!(mgr.unlock("test_id"));
    assert!(!mgr.unlock("test_id"));
    assert_eq!(mgr.unlocked_count(), 1);
}

#[test]
fn unlock_unregistered_returns_false() {
    let mut mgr = AchievementManager::in_memory();
    assert!(!mgr.unlock("never_registered"));
    assert_eq!(mgr.unlocked_count(), 0);
}

#[test]
fn is_unlocked_tracks_state() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    assert!(!mgr.is_unlocked("test_id"));
    mgr.unlock("test_id");
    assert!(mgr.is_unlocked("test_id"));
}

#[test]
fn unlock_queues_one_toast() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    assert_eq!(mgr.toasts.len(), 0);
    mgr.unlock("test_id");
    assert_eq!(mgr.toasts.len(), 1);
    // Second unlock attempt does not queue another toast.
    mgr.unlock("test_id");
    assert_eq!(mgr.toasts.len(), 1);
}

#[test]
fn tick_expires_toasts() {
    let mut mgr = AchievementManager::in_memory();
    mgr.set_toast_duration(2.0);
    mgr.register(sample());
    mgr.unlock("test_id");
    assert_eq!(mgr.toasts.len(), 1);
    mgr.tick(1.0);
    assert_eq!(mgr.toasts.len(), 1);
    mgr.tick(1.5);
    assert_eq!(mgr.toasts.len(), 0);
}

#[test]
fn reset_clears_unlocks_and_toasts() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    mgr.unlock("test_id");
    mgr.reset();
    assert_eq!(mgr.unlocked_count(), 0);
    assert_eq!(mgr.toasts.len(), 0);
}

#[test]
fn persistence_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ach.json");

    {
        let mut mgr = AchievementManager::with_save_path(&path);
        mgr.register(sample());
        mgr.register(Achievement::new("second", "Second", "Do it again"));
        mgr.unlock("test_id");
    }

    assert!(path.exists(), "save file should have been written");

    let mut restored = AchievementManager::with_save_path(&path);
    restored.register(sample());
    restored.register(Achievement::new("second", "Second", "Do it again"));
    assert!(restored.is_unlocked("test_id"));
    assert!(!restored.is_unlocked("second"));
    assert_eq!(restored.unlocked_count(), 1);
}

#[test]
fn concurrent_managers_merge_unlocks_instead_of_clobbering() {
    // Two managers on the same slot = the browser's two-tabs scenario.
    let dir = tempdir().unwrap();
    let path = dir.path().join("ach.json");

    let mut tab_a = AchievementManager::with_save_path(&path);
    tab_a.register(sample());
    tab_a.register(Achievement::new("second", "Second", "Do it again"));
    let mut tab_b = AchievementManager::with_save_path(&path);
    tab_b.register(sample());
    tab_b.register(Achievement::new("second", "Second", "Do it again"));

    tab_a.unlock("test_id"); // writes {test_id}
    tab_b.unlock("second"); // writes last, must merge {test_id} back in

    let restored = AchievementManager::with_save_path(&path);
    assert!(restored.is_unlocked("test_id"), "tab A's unlock must survive tab B's save");
    assert!(restored.is_unlocked("second"));
}

#[test]
fn reset_clears_the_save_despite_merge_on_save() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ach.json");
    let mut mgr = AchievementManager::with_save_path(&path);
    mgr.register(sample());
    mgr.unlock("test_id");
    mgr.reset();

    let restored = AchievementManager::with_save_path(&path);
    assert_eq!(restored.unlocked_count(), 0, "an explicit clear must actually clear");
}

#[test]
fn persistence_creates_parent_dir() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested/subdir/ach.json");
    let mut mgr = AchievementManager::with_save_path(&path);
    mgr.register(sample());
    mgr.unlock("test_id");
    assert!(path.exists());
}

#[test]
fn save_leaves_no_temp_file_behind() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ach.json");
    let mut mgr = AchievementManager::with_save_path(&path);
    mgr.register(sample());
    mgr.unlock("test_id");
    assert!(path.exists());
    assert!(
        !path.with_extension("json.tmp").exists(),
        "atomic save must rename the temp file away"
    );
}

#[test]
fn save_to_unwritable_path_errors_without_panicking() {
    let dir = tempdir().unwrap();
    // Make the intended parent directory an existing FILE so both
    // create_dir_all and the temp-file write must fail.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let path = blocker.join("ach.json");

    let mut mgr = AchievementManager::with_save_path(&path);
    mgr.register(sample());
    // unlock() triggers a save internally; failure is logged, not panicked.
    mgr.unlock("test_id");
    assert!(mgr.save().is_err());
    assert!(mgr.is_unlocked("test_id"), "in-memory state must survive");
}

#[test]
fn missing_save_file_is_not_an_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("does_not_exist.json");
    let mgr = AchievementManager::with_save_path(&path);
    assert_eq!(mgr.unlocked_count(), 0);
}

#[test]
fn hidden_achievement_flag_persists() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(Achievement::new("secret", "Secret", "Find it").hidden());
    assert!(mgr.get("secret").unwrap().hidden);
}

#[test]
fn default_toast_style_matches_documented_appearance() {
    let style = ToastStyle::default();
    assert_eq!(style.width, 320.0);
    assert_eq!(style.height, 72.0);
    assert_eq!(style.margin, 16.0);
    assert_eq!(style.spacing, 8.0);
    assert_eq!(style.background, Color::new(0.08, 0.08, 0.12, 0.92));
    assert_eq!(style.border, Color::new(1.0, 0.82, 0.2, 1.0));
    assert_eq!(style.border_width, 2.0);
    assert_eq!(style.title_color, Color::new(1.0, 0.82, 0.2, 1.0));
    assert_eq!(style.name_color, Color::new(1.0, 1.0, 1.0, 1.0));
    assert_eq!(style.description_color, Color::new(0.8, 0.8, 0.85, 1.0));
    assert_eq!(style.title_size, 14.0);
    assert_eq!(style.name_size, 16.0);
    assert_eq!(style.description_size, 12.0);
}

#[test]
fn manager_uses_default_toast_style_until_overridden() {
    let mut mgr = AchievementManager::in_memory();
    assert_eq!(*mgr.toast_style(), ToastStyle::default());

    let custom = ToastStyle { width: 400.0, ..ToastStyle::default() };
    mgr.set_toast_style(custom.clone());
    assert_eq!(*mgr.toast_style(), custom);
}

#[test]
fn custom_toast_style_drives_drawn_panel_size() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    mgr.set_toast_style(ToastStyle {
        width: 444.0,
        height: 99.0,
        ..ToastStyle::default()
    });
    mgr.unlock("test_id");

    let input = input::InputHandler::new();
    let mut ui = UIContext::new();
    ui.begin_frame(&input, Vec2::new(800.0, 600.0));
    mgr.draw_toasts(&mut ui, Vec2::new(800.0, 600.0));
    ui.end_frame();

    let panel_drawn_with_custom_size = ui.draw_list().commands().iter().any(|cmd| {
        matches!(
            cmd,
            ui::DrawCommand::Rect { bounds, .. }
                if bounds.width == 444.0 && bounds.height == 99.0
        )
    });
    assert!(
        panel_drawn_with_custom_size,
        "toast panel should be drawn with the custom width/height"
    );
}

#[test]
fn all_iterator_yields_registered() {
    let mut mgr = AchievementManager::in_memory();
    mgr.register(sample());
    mgr.register(Achievement::new("second", "Second", "Desc"));
    let ids: std::collections::HashSet<_> = mgr.all().map(|a| a.id.as_str()).collect();
    assert!(ids.contains("test_id"));
    assert!(ids.contains("second"));
}
