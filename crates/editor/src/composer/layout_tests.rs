//! The composer's layout: the three modes by width, the roll's geometry round trips, and
//! the bar and beat lines over the tempo map — meter changes and the LOD thinning.

use ui::Rect;

use super::layout::{compute, BeatLineKind, ComposerMode, PianoRollGeometry, COMPACT_BELOW_WIDTH, TABS_BELOW_WIDTH};
use super::state::{ComposerFocus, ComposerState};
use super::test_support::melody;
use crate::song::{Gesture, Meter, SongDocument, SongEdit};

fn panel(width: f32) -> Rect {
    Rect::new(0.0, 400.0, width, 260.0)
}

#[test]
fn the_mode_follows_the_panel_width_at_the_two_thresholds() {
    let cases = [
        (1280.0, ComposerMode::Full),
        (COMPACT_BELOW_WIDTH, ComposerMode::Full),
        (COMPACT_BELOW_WIDTH - 1.0, ComposerMode::Compact),
        (TABS_BELOW_WIDTH, ComposerMode::Compact),
        (TABS_BELOW_WIDTH - 1.0, ComposerMode::Tabs),
        (320.0, ComposerMode::Tabs),
    ];
    for (width, mode) in cases {
        assert_eq!(ComposerMode::for_width(width), mode, "{width} px");
    }
}

#[test]
fn each_mode_lays_out_its_areas_without_overlap_and_tabs_shows_one_area() {
    let state = ComposerState::default();
    let full = compute(panel(1280.0), &state, 480);
    let (sidebar, note_list, roll) = (
        full.sidebar.expect("a sidebar"),
        full.note_list.expect("a note list"),
        full.roll.expect("a roll").bounds,
    );
    assert!(sidebar.right() <= roll.x && roll.right() <= note_list.x, "left to right, apart");
    assert!(full.section_tabs.is_none(), "the full width shows every section");
    assert!(full.transport.bottom() <= roll.y);

    let compact = compute(panel(640.0), &state, 480);
    assert!(compact.section_tabs.is_some(), "one sidebar section at a time");
    assert!(compact.roll.is_some() && compact.note_list.is_some());

    let mut state = ComposerState::default();
    for area in [ComposerFocus::PianoRoll, ComposerFocus::NoteList, ComposerFocus::Sidebar] {
        state.focus_area(area);
        let tabs = compute(panel(400.0), &state, 480);
        let shown = [tabs.roll.is_some(), tabs.note_list.is_some(), tabs.sidebar.is_some()];
        assert_eq!(shown.iter().filter(|shown| **shown).count(), 1, "{area:?} alone");
        assert!(tabs.area_tabs.is_some());
    }
}

#[test]
fn the_roll_maps_ticks_and_keys_to_pixels_and_back_exactly_finitely_and_in_order() {
    for song in [melody(), SongDocument::new_song().expect("a blank song")] {
        let ticks_per_quarter = song.data().ticks_per_quarter();
        let state = ComposerState::default();
        let roll = PianoRollGeometry::new(Rect::new(0.0, 0.0, 800.0, 200.0), &state, ticks_per_quarter);
        let mut previous_x = f32::NEG_INFINITY;
        for tick in [0, 1, 120, 479, 480, 1920, 96_000] {
            let x = roll.tick_to_x(tick);
            assert!(x.is_finite() && x > previous_x, "tick {tick} lands right of the last");
            assert_eq!(roll.x_to_tick(x), tick as i64, "tick {tick} round trips");
            previous_x = x;
        }
        let mut previous_y = f32::NEG_INFINITY;
        for key in (0..=127u8).rev() {
            let y = roll.pitch_to_y(key);
            assert!(y > previous_y, "a lower key sits lower");
            assert_eq!(roll.y_to_pitch(y + roll.key_height / 2.0), i32::from(key));
            previous_y = y;
        }
        assert!(roll.x_to_tick(roll.grid.x - 50.0) < 0, "left of tick 0 reads negative");
    }
    let empty = ComposerState::default();
    let no_division = PianoRollGeometry::new(Rect::new(0.0, 0.0, 400.0, 100.0), &empty, 0);
    assert!(no_division.pixels_per_tick.is_finite(), "a zero division divides by one");
}

/// Every line's tick, its kind, and whether the walk stayed in order.
fn lines(song: &SongDocument, pixels_per_quarter: f32) -> Vec<(u64, BeatLineKind)> {
    let mut state = ComposerState::default();
    state.pixels_per_quarter = pixels_per_quarter;
    let roll = PianoRollGeometry::new(Rect::new(0.0, 0.0, 1200.0, 200.0), &state, song.data().ticks_per_quarter());
    let beat_lines = roll.beat_lines(&song.data().tempo_map());
    for pair in beat_lines.windows(2) {
        assert!(pair[0].tick < pair[1].tick && pair[0].x < pair[1].x, "monotonic");
    }
    beat_lines.into_iter().map(|line| (line.tick, line.kind)).collect()
}

#[test]
fn bar_lines_follow_a_meter_change_onto_its_own_ticks() {
    let mut song = melody();
    song.apply(
        SongEdit::SetTimeSignature {
            tick: 1920,
            meter: Some(Meter { numerator: 3, denominator: 4 }),
        },
        Gesture::Discrete,
    )
    .expect("a 3/4 bar from bar 2");
    let bars: Vec<u64> = lines(&song, 16.0)
        .into_iter()
        .filter(|(_, kind)| *kind == BeatLineKind::Bar)
        .map(|(tick, _)| tick)
        .take(4)
        .collect();
    assert_eq!(bars, vec![0, 1920, 3360, 4800], "four quarters, then three to a bar");
}

#[test]
fn the_lines_thin_as_the_roll_zooms_out_and_sixteenths_show_only_zoomed_in() {
    let song = SongDocument::new_song().expect("a blank song");
    let close = lines(&song, 64.0);
    assert!(close.iter().any(|(_, kind)| *kind == BeatLineKind::Subdivision), "sixteenths at 64 px a quarter");
    assert!(close.iter().any(|(_, kind)| *kind == BeatLineKind::Beat));

    let middle = lines(&song, 16.0);
    assert!(middle.iter().all(|(_, kind)| *kind != BeatLineKind::Subdivision), "no sixteenths at 16 px");
    assert!(middle.iter().any(|(_, kind)| *kind == BeatLineKind::Beat));

    let far = lines(&song, 4.0);
    assert!(far.iter().all(|(_, kind)| *kind == BeatLineKind::Bar), "4 px a quarter: bars alone");
    assert!(!far.is_empty(), "never no grid at all");
}
