//! What the queries say in the update that swapped a replacement in, before its conductor
//! has rendered a block: the landing, never the fresh conductor's zeros.

use super::*;

#[test]
fn before_the_new_conductors_first_block_the_position_is_the_landing() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing(&assets);
    run(&mut manager, &mut now, 5);
    manager.pause_song();
    manager.seek_song(6);
    run(&mut manager, &mut now, 3);
    request(
        &mut manager,
        &write_shaped(&assets, "same", SONG_SHAPE, true, None),
    );
    let (_, swapped_to, steps, _) = run_until_swapped(&mut manager, &mut now, 1);
    assert_eq!(steps, 1);

    let position = manager.song_position().expect("a song is held");
    assert_eq!(
        (position.bar, position.beat_in_bar),
        (1, 2),
        "beat 6, not the fresh conductor's zero"
    );
    assert_eq!(position.beat, swapped_to.beat);
    assert!(
        !manager.is_song_playing() && !manager.song_ended(),
        "paused, as before the swap"
    );
    run(&mut manager, &mut now, 2);
    assert_eq!(
        manager.song_position().expect("held").beat,
        6,
        "and the conductor agrees once it renders"
    );
}

#[test]
fn before_the_new_conductors_first_block_a_song_stopped_at_the_end_still_reads_ended() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing_shaped(&assets, SONG_SHAPE, false);
    run(&mut manager, &mut now, 4 * STEPS_A_BAR + 10);
    assert!(manager.song_ended());
    request(
        &mut manager,
        &write_shaped(&assets, "longer", shape(4, 24), false, None),
    );
    let (_, _, steps, _) = run_until_swapped(&mut manager, &mut now, 1);
    assert_eq!(steps, 1);

    assert!(manager.song_ended(), "ended before the swap, ended after it");
    assert!(!manager.is_song_playing());
    assert!(
        manager.song_label().is_none() && manager.song_request().is_none(),
        "an ended song is no song to a caller deciding whether to request music"
    );
    assert_eq!(
        manager.song_position().expect("a song is held").beat,
        24,
        "at the new end"
    );
    run(&mut manager, &mut now, 3);
    assert!(manager.song_ended() && !manager.is_song_playing());
}

#[test]
fn a_paused_song_in_a_bar_the_new_song_lacks_is_paused_at_the_new_end_not_ended() {
    let assets = AssetDirectory::new();
    let (mut manager, mut now) = playing_shaped(&assets, SONG_SHAPE, false);
    run(&mut manager, &mut now, 3 * STEPS_A_BAR + 5);
    manager.pause_song();
    run(&mut manager, &mut now, 2);
    request(
        &mut manager,
        &write_shaped(&assets, "shorter", shape(4, 8), false, None),
    );
    let (bar, position, steps, others) = run_until_swapped(&mut manager, &mut now, 1);

    assert!(others.is_empty(), "{others:?}");
    assert_eq!((bar, steps), (3, 1));
    assert_eq!(position.beat, 8, "the new end");
    run(&mut manager, &mut now, 5);
    let held = snapshot(&manager);
    assert_eq!(held.transport, TransportState::Paused, "{held:?}");
    assert!(!held.ended && !manager.song_ended() && !manager.is_song_playing());
    assert_eq!(held.tick, u64::from(8 * QUARTER));
    manager.resume_song();
    run(&mut manager, &mut now, 3);
    assert!(manager.song_ended(), "played from its end, it ends on its first block");
}
