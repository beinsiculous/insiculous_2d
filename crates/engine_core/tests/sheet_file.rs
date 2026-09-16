use std::path::Path;

use engine_core::sheet_file::parse_sheet_file;
use renderer::TextureFilter;

#[test]
fn test_tong_left_sheet_file_fixture_contract() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ai_tong_left.sheet.ron");
    assert!(fixture_path.exists(), "fixture must be checked in");

    let text = std::fs::read_to_string(&fixture_path)
        .expect("read ai_tong_left.sheet.ron fixture");

    let sheet = parse_sheet_file("ai_tong_left.sheet.ron", &text)
        .expect("parse ai_tong_left.sheet.ron fixture");

    assert_eq!(sheet.version, 1);
    assert_eq!(sheet.cell, (64, 96));
    assert_eq!(sheet.filter, TextureFilter::Nearest);

    let (grid, clips, filter) = sheet
        .into_parts("ai_tong_left.sheet.ron", 256, 672)
        .expect("resolve into_parts with 256x672 PNG dimensions");

    assert_eq!((grid.cols, grid.rows), (4, 7));
    assert_eq!(grid.cell_count(), 28);
    assert_eq!(filter, TextureFilter::Nearest);

    let total_frames: usize = clips.iter().map(|(_, c)| c.frame_indices.len()).sum();
    assert_eq!(total_frames, 28);

    let clip_names: Vec<&str> = clips.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        clip_names,
        vec![
            "open_up",
            "closing_up",
            "closed_up",
            "opening_up",
            "scored_on_up",
            "open_down",
            "closing_down",
            "closed_down",
            "opening_down",
            "scored_on_down",
        ]
    );

    // The transitions and the scored-on twitch play once (`repeat = 1` in the master); the held
    // states loop. A regenerated sidecar that flips one of these is drift, and nothing else in
    // this crate would notice.
    let looping: Vec<bool> = clips.iter().map(|(_, c)| c.looping).collect();
    assert_eq!(
        looping,
        vec![true, false, true, false, false, true, false, true, false, false]
    );
}
