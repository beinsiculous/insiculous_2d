//! A `.sheet.ron` edited on disk, through the real `EditorGame`: the grid, the clips and the
//! texture reach every animation cut from its PNG together, a sprite showing another texture
//! is left alone, a sidecar that does not parse keeps the old sheet, a playing frame past the
//! new clip clamps, a playing clip the new sheet lacks is named, and Stop puts what a session
//! reloaded back on the world it restores.

use common::SheetGrid;
use ecs::sprite_components::{Sprite, SpriteAnimation};
use ecs::{EntityId, World};
use editor::console::Severity;

use super::hot_reload_tests::Session;
use super::play_session::PausedEdits;
use super::test_support::press_play;

const PNG: &str = "sprites/paddle.png";
const SIDECAR: &str = "sprites/paddle.sheet.ron";

/// A sidecar over the 16 × 16 test PNG: square cells of `cell` pixels, sampled with
/// `filter`, and one clip, `walk`.
fn sidecar(cell: u32, filter: &str, walk_frames: &str) -> String {
    format!("SheetFile(version: 1, cell: ({cell}, {cell}), filter: {filter}, clips: [(\"walk\", (frames: [{walk_frames}], fps: 6.0))])")
}

/// A project holding the playground's 16 × 16 paddle as a sheet cut by `sidecar_text`.
fn sheet_session(sidecar_text: &str) -> Session {
    let session = Session::new();
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../playground/assets/projects/pong/assets/images/paddle_16px.png");
    std::fs::create_dir_all(session.path("sprites")).expect("the sprites folder");
    std::fs::copy(source, session.path(PNG)).expect("a copy of the test PNG");
    session.write(SIDECAR, sidecar_text);
    session
}

/// An entity built from the sheet as a game builds one, playing `walk`.
fn animated(session: &mut Session) -> EntityId {
    let sheet = session.assets.load_sprite_sheet(PNG).expect("the sheet loads");
    let entity = session.world.create_entity();
    let mut animation = sheet.animation();
    assert!(animation.play("walk"), "the sheet has a walk clip");
    session.world.add_component(&entity, sheet.sprite()).ok();
    session.world.add_component(&entity, animation).ok();
    entity
}

fn animation(world: &World, entity: EntityId) -> &SpriteAnimation {
    world.get::<SpriteAnimation>(entity).expect("the animation")
}

fn texture(world: &World, entity: EntityId) -> u32 {
    world.get::<Sprite>(entity).expect("the sprite").texture_handle
}

#[test]
fn a_cell_size_change_while_editing_recuts_every_animation_and_dirties_nothing() {
    let mut session = sheet_session(&sidecar(8, "Nearest", "0, 1"));
    let first = animated(&mut session);
    let second = animated(&mut session);
    assert_eq!(animation(&session.world, first).grid, SheetGrid::new(2, 2));
    session.watch_now();

    session.write(SIDECAR, &sidecar(4, "Nearest", "0, 1, 2, 3, 4"));
    session.changed(SIDECAR);

    for entity in [first, second] {
        let animation = animation(&session.world, entity);
        assert_eq!(animation.grid, SheetGrid::new(4, 4), "a new cell size is a new grid");
        assert_eq!(animation.clips[0].1.frame_indices, vec![0, 1, 2, 3, 4], "with the new clips");
    }
    assert!(!session.editor.command_history.is_dirty(), "the sidecar's own truth, not an edit");
    assert!(session.console_mentions("sprites/paddle.sheet.ron changed on disk (reload sheet)"), "{:?}", session.console());
}

#[test]
fn a_cell_size_change_in_play_survives_stop() {
    let mut session = sheet_session(&sidecar(8, "Nearest", "0, 1"));
    let entity = animated(&mut session);
    session.watch_now();
    press_play(&mut session.editor, &mut session.world);

    session.write(SIDECAR, &sidecar(4, "Nearest", "0, 1, 2"));
    session.changed(SIDECAR);
    assert_eq!(animation(&session.world, entity).grid, SheetGrid::new(4, 4));

    session.editor.stop_play_session(&mut session.world, PausedEdits::Discard);
    assert_eq!(animation(&session.world, entity).grid, SheetGrid::new(4, 4), "the snapshot never brings the old cut back");
    assert_eq!(animation(&session.world, entity).clips[0].1.frame_indices, vec![0, 1, 2]);
    assert!(!session.editor.command_history.is_dirty());
}

#[test]
fn a_new_filter_moves_the_sheets_sprites_to_the_new_texture_and_leaves_another_texture_alone_at_reload_and_at_stop() {
    let mut session = sheet_session(&sidecar(8, "Nearest", "0, 1"));
    let on_the_sheet = animated(&mut session);
    let on_another = animated(&mut session);
    session.world.get_mut::<Sprite>(on_another).expect("the sprite").texture_handle = 0;
    let old_handle = texture(&session.world, on_the_sheet);
    session.watch_now();
    press_play(&mut session.editor, &mut session.world);

    session.write(SIDECAR, &sidecar(8, "Linear", "0, 1"));
    session.changed(SIDECAR);
    let new_handle = texture(&session.world, on_the_sheet);
    assert_ne!(new_handle, old_handle, "a new filter is a new handle");
    assert_eq!(texture(&session.world, on_another), 0, "a sprite showing another texture keeps it");

    session.editor.stop_play_session(&mut session.world, PausedEdits::Discard);
    assert_eq!(texture(&session.world, on_the_sheet), new_handle, "Stop puts the new texture back");
    assert_eq!(texture(&session.world, on_another), 0, "and leaves the other alone");
}

#[test]
fn a_sidecar_that_does_not_parse_keeps_the_old_sheet_and_says_why() {
    let mut session = sheet_session(&sidecar(8, "Nearest", "0, 1"));
    let entity = animated(&mut session);
    let handle = texture(&session.world, entity);
    session.watch_now();

    session.write(SIDECAR, "SheetFile(version: 1, cell: (");
    session.changed(SIDECAR);

    assert_eq!(animation(&session.world, entity).grid, SheetGrid::new(2, 2));
    assert_eq!(texture(&session.world, entity), handle);
    let (severity, line) = session.console().last().cloned().expect("the report");
    assert_eq!(severity, Severity::Error);
    assert!(line.contains("the sheet did not reload, the old one is kept"), "{line}");
}

#[test]
fn a_playing_frame_past_the_new_clips_end_clamps_to_its_last_frame_and_warns() {
    let mut session = sheet_session(&sidecar(4, "Nearest", "0, 1, 2, 3, 4, 5"));
    let entity = animated(&mut session);
    session.world.get_mut::<SpriteAnimation>(entity).expect("the animation").current_frame = 5;
    session.watch_now();

    session.write(SIDECAR, &sidecar(8, "Nearest", "0, 1"));
    session.changed(SIDECAR);

    assert_eq!(animation(&session.world, entity).current_frame, 1);
    assert!(session.console_mentions("its frame 5 clamped to 1"), "{:?}", session.console());
}

#[test]
fn a_playing_clip_the_reloaded_sheet_no_longer_has_is_named_and_stays_selected() {
    let mut session = sheet_session(&sidecar(8, "Nearest", "0, 1"));
    let entity = animated(&mut session);
    session.watch_now();

    session.write(SIDECAR, "SheetFile(version: 1, cell: (8, 8), filter: Nearest, clips: [(\"run\", (frames: [2, 3], fps: 6.0))])");
    session.changed(SIDECAR);

    assert_eq!(animation(&session.world, entity).current_clip.as_deref(), Some("walk"));
    assert!(session.console_mentions("clip 'walk' is no longer in the sheet"), "{:?}", session.console());
}
