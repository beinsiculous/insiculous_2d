//! The composer's state: Tab's cycle over the areas, the track a song opens on, the
//! selector's tracks, the reset when the song is replaced, and the Songs list's scan.

use super::state::{default_track, next_track, scan_songs, selectable_tracks, ComposerFocus, ComposerState};
use super::test_support::{melody, second_track_holds_the_notes, timing_only};

#[test]
fn tab_walks_the_four_areas_in_a_cycle_and_shift_tab_walks_it_back() {
    let mut focus = ComposerFocus::None;
    let mut walked = Vec::new();
    for _ in 0..5 {
        focus = focus.next();
        walked.push(focus);
    }
    assert_eq!(
        walked,
        [
            ComposerFocus::Transport,
            ComposerFocus::PianoRoll,
            ComposerFocus::NoteList,
            ComposerFocus::Sidebar,
            ComposerFocus::Transport,
        ],
        "from no focus Tab enters at the transport and wraps after the sidebar"
    );
    assert_eq!(ComposerFocus::Transport.previous(), ComposerFocus::Sidebar);
    assert_eq!(ComposerFocus::None.previous(), ComposerFocus::Sidebar);
    assert!(!ComposerFocus::None.is_held());
}

#[test]
fn a_song_opens_on_its_first_track_with_a_note_and_the_selector_skips_the_timing_track() {
    let song = second_track_holds_the_notes();
    assert_eq!(
        default_track(&song),
        Some(2),
        "the lead holds the note though the empty pad comes first"
    );
    assert_eq!(selectable_tracks(&song), vec![1, 2], "track 0 holds timing only");
    assert_eq!(next_track(&song, Some(2), true), Some(1), "the selector wraps over the two");
    assert_eq!(next_track(&song, Some(1), false), Some(2));

    let blank = melody();
    assert_eq!(default_track(&blank), Some(1), "a new song's melody track, never the timing track");

    let timing = timing_only();
    assert_eq!(default_track(&timing), None, "a file of timing alone has no note track");
    assert_eq!(next_track(&timing, None, true), None);
}

#[test]
fn a_replaced_song_resets_what_pointed_into_the_old_one_and_keeps_the_songs_list() {
    let mut state = ComposerState::default();
    let song = second_track_holds_the_notes();
    assert!(state.follow(1, Some(&song)), "the first song is a change");
    assert_eq!(state.active_track, Some(2));
    state.focus = ComposerFocus::PianoRoll;
    state.note_list_row = 3;
    state.songs = vec!["music/theme.mid".to_string()];
    state.songs_scanned = true;

    assert!(!state.follow(1, Some(&song)), "the same generation changes nothing");
    assert_eq!(state.focus, ComposerFocus::PianoRoll);

    let replacement = melody();
    assert!(state.follow(2, Some(&replacement)));
    assert_eq!(state.focus, ComposerFocus::None, "the keyboard is dropped");
    assert_eq!(state.active_track, Some(1), "the new song's own track");
    assert_eq!(state.note_list_row, 0);
    assert_eq!(state.songs, vec!["music/theme.mid".to_string()], "the folders did not change");

    assert!(state.follow(3, None));
    assert_eq!(state.active_track, None, "no song, no track");
}

#[test]
fn the_songs_list_finds_every_mid_in_both_home_folders_by_its_folder_qualified_path() {
    let folder = tempfile::tempdir().expect("an asset folder");
    for file in [
        "music/theme.mid",
        "music/levels/boss.MID",
        "ai/music/theme.mid",
        "music/theme.mid.ron",
        "music/samples/theme/kick.wav",
        "sounds/beep.mid",
    ] {
        common::vfs::write(&folder.path().join(file), b"x").expect("fixture written");
    }
    assert_eq!(
        scan_songs(folder.path()),
        vec!["ai/music/theme.mid", "music/levels/boss.MID", "music/theme.mid"],
        "two same-named songs list apart by folder; a sidecar, a sample and a .mid outside \
         the song folders are not songs here"
    );
    let empty = tempfile::tempdir().expect("an empty asset folder");
    assert!(scan_songs(empty.path()).is_empty(), "a missing folder is an empty list");
}
