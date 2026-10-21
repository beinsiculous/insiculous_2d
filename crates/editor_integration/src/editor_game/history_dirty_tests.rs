//! The history-only dirty mirror: the signal the playground's save indicator
//! reads, as opposed to the combined flag the window title renders.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ecs::World;

use super::test_support::dirty_editor;

fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

fn reads(flag: &AtomicBool) -> bool {
    flag.load(Ordering::Relaxed)
}

#[test]
fn test_history_dirty_flag_tracks_command_history_alone_not_the_combined_signal() {
    let mut world = World::new();
    let mut editor = dirty_editor(&mut world);

    let combined = flag();
    let history_only = flag();
    let persist_pending = flag();
    editor.dirty_flag = Some(combined.clone());
    editor.history_dirty_flag = Some(history_only.clone());
    editor.persist_pending = Some(persist_pending.clone());

    editor.sync_dirty_mirror();
    assert!(reads(&combined), "a recorded command reads dirty on the combined flag");
    assert!(reads(&history_only), "and on the history-only flag");

    // The save marks the history clean and issues its put; the put is still in
    // flight when that frame's mirror runs, so the combined flag reports it.
    persist_pending.store(true, Ordering::Relaxed);
    editor.command_history.mark_saved();
    editor.sync_dirty_mirror();
    assert!(reads(&combined), "the in-flight put keeps the combined flag dirty");
    assert!(!reads(&history_only), "but the history itself is saved");

    // The put finishes on its own future, between frames — nothing re-runs the
    // mirror until the next one. This window is where a reader may land.
    persist_pending.store(false, Ordering::Relaxed);
    assert!(reads(&combined), "the combined flag still reports the finished put");
    assert!(!reads(&history_only), "the history-only flag never saw it");

    // The next frame settles both.
    editor.sync_dirty_mirror();
    assert!(!reads(&combined), "the combined flag catches up with the put");
    assert!(!reads(&history_only));
}

#[test]
fn test_history_dirty_flag_carries_the_songs_edits_so_the_playground_sees_them() {
    let mut editor = super::test_support::editor_game();
    let combined = flag();
    let history_only = flag();
    editor.dirty_flag = Some(combined.clone());
    editor.history_dirty_flag = Some(history_only.clone());

    editor.sync_dirty_mirror();
    assert!(!reads(&history_only), "a clean scene with no song");

    editor.song = Some(editor::SongDocument::new_song().expect("a blank song"));
    editor.sync_dirty_mirror();
    assert!(reads(&history_only), "a never-saved song is unsaved work beside a clean scene");
    assert!(reads(&combined));
    assert!(!editor.editor.is_dirty(), "the scene's own mark stays clean");
    assert!(editor.editor.is_song_dirty(), "the song carries its own");

    editor.song = None;
    editor.sync_dirty_mirror();
    assert!(!reads(&history_only), "a closed song leaves nothing unsaved");
}
