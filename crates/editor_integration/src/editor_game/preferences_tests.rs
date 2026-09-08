use std::path::PathBuf;
use glam::Vec2;
use super::test_support::editor_game;
use editor::PlayControlAction;

struct TempSlot(PathBuf);
impl TempSlot {
    fn new(name: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "insiculous_{name}_{}_{}.json",
            std::process::id(),
            id
        ));
        Self(path)
    }
}
impl Drop for TempSlot {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn test_preferences_settle_after_half_second_and_skipped_during_play() {
    let slot = TempSlot::new("prefs_settle");
    let mut editor = editor_game();
    editor.prefs_slot = slot.0.clone();

    // 1. Initial state: slot does not exist.
    assert!(engine_core::save_store::read(&editor.prefs_slot).unwrap().is_none());

    // 2. Change camera offset.
    editor.editor.set_camera_offset(Vec2::new(123.0, 456.0));

    // 3. Tick 29 times at 1/60s (0.4833s < 0.5s) — slot remains unwritten.
    let dt = 1.0 / 60.0;
    for _ in 0..29 {
        editor.save_preferences_if_changed(dt);
        assert!(
            engine_core::save_store::read(&editor.prefs_slot).unwrap().is_none(),
            "slot must remain unwritten before 0.5s of stability"
        );
    }

    // 4. Tick once more (30th tick = 0.5000s) — settles and writes!
    editor.save_preferences_if_changed(dt);
    let written = engine_core::save_store::read(&editor.prefs_slot).unwrap();
    assert!(written.is_some(), "preferences must be written once stable for 0.5s");
    let json = written.unwrap();
    assert!(json.contains("123"), "written prefs must contain the new camera position");

    // 5. Stable half second during Play writes nothing.
    let mut world = ecs::World::new();
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    assert!(editor.editor.is_playing());

    // Change camera during play:
    editor.editor.set_camera_offset(Vec2::new(999.0, 999.0));
    for _ in 0..35 {
        editor.save_preferences_if_changed(dt);
    }
    // Slot still has the previous json (123.0, not 999.0):
    let current_slot = engine_core::save_store::read(&editor.prefs_slot).unwrap().unwrap();
    assert_eq!(current_slot, json, "settle writes must be skipped during play");
}

#[test]
fn test_play_transition_writes_immediately_with_editing_camera() {
    let slot = TempSlot::new("prefs_play");
    let mut editor = editor_game();
    editor.prefs_slot = slot.0.clone();

    editor.editor.set_camera_offset(Vec2::new(77.0, 88.0));
    assert!(engine_core::save_store::read(&editor.prefs_slot).unwrap().is_none());

    // Entering play mode writes immediately without waiting for settle.
    let mut world = ecs::World::new();
    editor.handle_play_action(PlayControlAction::Play, &mut world);
    let written = engine_core::save_store::read(&editor.prefs_slot).unwrap().expect("written on play");
    assert!(written.contains("77"), "must capture editing camera before play session starts");
}

#[test]
fn test_load_preferences_after_settle_restores_camera_from_slot() {
    let slot = TempSlot::new("prefs_roundtrip");
    let mut editor = editor_game();
    editor.prefs_slot = slot.0.clone();

    editor.editor.set_camera_offset(Vec2::new(314.0, 159.0));
    editor.save_preferences_now();

    // Fresh editor reading from the same slot
    let mut second_editor = editor_game();
    second_editor.prefs_slot = slot.0.clone();
    second_editor.load_preferences();

    assert_eq!(second_editor.editor.camera_offset(), Vec2::new(314.0, 159.0));
}
