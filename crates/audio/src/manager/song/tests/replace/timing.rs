//! How long the worker takes over the largest instrument set a game ships, printed.

use std::path::Path;

use super::*;

/// The largest shipped instrument set: Chicken Coop's five SoundFont instruments on its
/// 3.4 MB bank, copied out of the frogger game into a temporary folder. Run it with
/// `cargo test --release -p audio chicken_coop -- --nocapture` for the number.
#[test]
fn chicken_coop_is_prepared_off_the_frame_and_its_preparation_time_printed() {
    let frogger = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../games/frogger/assets");
    if !frogger.is_dir() {
        eprintln!(
            "skipped: {} is absent, so there is no Chicken Coop to time",
            frogger.display()
        );
        return;
    }
    let assets = AssetDirectory::new();
    for relative in [
        "ai/music/ai_chicken_coop.mid",
        "ai/music/ai_chicken_coop.mid.ron",
        "soundfonts/chicken_coop.sf2",
    ] {
        let bytes = std::fs::read(frogger.join(relative)).expect("frogger's file reads");
        assets.write(relative, &bytes);
    }
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);

    let called = std::time::Instant::now();
    manager
        .replace_song_at_seam(assets.path().join("ai/music/ai_chicken_coop.mid"))
        .expect("Chicken Coop reads");
    let on_the_frame = called.elapsed();
    manager.wait_for_preparations_newest_first();
    let prepared = called.elapsed();
    println!(
        "chicken coop: {:.3} ms on the frame, prepared {:.1} ms after the call",
        on_the_frame.as_secs_f64() * 1_000.0,
        prepared.as_secs_f64() * 1_000.0
    );

    let (_, _, _, others) = run_until_swapped(&mut manager, &mut now, STEPS_A_BAR);
    assert!(others.is_empty(), "{others:?}");
    assert_eq!(
        manager.song_label(),
        Some(assets.path().join("song.mid").display().to_string()).as_deref()
    );
}
