//! Shared fixtures for the editor-integration tests.
//!
//! Three facts every test file used to restate: the inner game is inert
//! (the `EditorGame` wrapper is the subject), an entity under test is a
//! `Transform2D` at a position, and a gizmo drag begins by capturing each
//! selection root's transform and collider. They live here once. The
//! GPU-free scene helpers (`StubResolver`, `test_texture_path`) come from
//! `engine_core::test_support`.

use ecs::{EntityId, World};
use engine_core::contexts::GameContext;
use engine_core::Game;
use glam::Vec2;

use super::gizmo_drag::{DragEntity, GizmoDragState};
use super::EditorGame;

/// An inner game that does nothing — the editor wrapper is the subject.
pub(super) struct DummyGame;
impl Game for DummyGame {
    fn update(&mut self, _ctx: &mut GameContext) {}
}

/// Press Play and carry it through the two frames it takes — the drain that plays the
/// saved song on a headless manager, the next frame's start — so a test reads the session
/// Play began. Resume, and a Play refused or parked behind a prompt, pass through it alike.
pub(super) fn press_play<G: Game>(editor: &mut EditorGame<G>, world: &mut World) {
    editor.handle_play_action(editor::PlayControlAction::Play, world);
    complete_play(editor, world);
}

/// The two frames an armed Play waits for, on a headless manager: what the dirty
/// prompt's answer leaves to the frame after it.
pub(super) fn complete_play<G: Game>(editor: &mut EditorGame<G>, world: &mut World) {
    let mut audio = engine_core::prelude::AudioManager::disabled();
    audio.set_asset_base(&editor.asset_base);
    let mut sources = engine_core::prelude::SoundSources::default();
    editor.apply_song_intents(&mut audio, &mut sources);
    editor.start_armed_play(world);
}

/// A fresh editor session wrapping [`DummyGame`], in the Editing state.
pub(super) fn editor_game() -> EditorGame<DummyGame> {
    static SLOT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let slot_id = SLOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut editor_game = EditorGame::new(DummyGame);
    // Pressing Play writes the preferences; the default slot would land in the crate
    // directory, and tests run in parallel, so each session gets its own slot.
    editor_game.prefs_slot = std::env::temp_dir()
        .join(format!("insiculous_test_editor_prefs_{}_{slot_id}.json", std::process::id()));
    editor_game
}

/// A world entity with only a `Transform2D` at `pos`.
pub(super) fn spawn_at(world: &mut World, pos: Vec2) -> EntityId {
    let entity = world.create_entity();
    world.add_component(&entity, common::Transform2D::new(pos)).ok();
    entity
}

/// The entity's local position.
pub(super) fn position(world: &World, entity: EntityId) -> Vec2 {
    world
        .get::<common::Transform2D>(entity)
        .map(|t| t.position)
        .expect("entity has a Transform2D")
}

/// A session whose history already holds one recorded command (an
/// entity creation), so the scene reads dirty.
pub(super) fn dirty_editor(world: &mut World) -> EditorGame<DummyGame> {
    let mut editor = editor_game();
    let entity = world.create_entity();
    editor.command_history.execute(
        Box::new(editor::commands::CreateEntityCommand::already_created(world, entity)),
        world,
    );
    assert!(editor.command_history.is_dirty(), "fixture: a recorded command dirties the scene");
    editor.sync_dirty_mirror();
    editor
}

/// The drag-start capture `handle_gizmo` makes for `ids`: every root's
/// transform and collider exactly as they are now.
pub(super) fn drag_state_for(world: &World, ids: &[EntityId]) -> GizmoDragState {
    GizmoDragState {
        entities: ids
            .iter()
            .map(|&id| DragEntity {
                id,
                start: *world.get::<common::Transform2D>(id).expect("drag root has a transform"),
                start_collider: world.get::<physics::components::Collider>(id).cloned(),
            })
            .collect(),
        accumulated_rotation: 0.0,
    }
}

/// One translate-tool frame with the given cumulative screen offset.
pub(super) fn translate_interaction(cumulative: Vec2) -> editor::GizmoInteraction {
    editor::GizmoInteraction {
        handle: Some(editor::GizmoHandle::Center),
        translation: cumulative,
        ..Default::default()
    }
}

/// One scale-tool frame with the given cumulative scale factor.
pub(super) fn scale_interaction(factor: Vec2) -> editor::GizmoInteraction {
    editor::GizmoInteraction {
        handle: Some(editor::GizmoHandle::ScaleCorner(editor::Corner::BottomRight)),
        scale_factor: factor,
        ..Default::default()
    }
}

/// Answer one command-API line against a session whose resolver issued
/// only the built-in `#white` (handle 0).
pub(super) fn api_line(editor: &mut EditorGame<DummyGame>, world: &mut World, line: &str) -> String {
    let resolver = |handle: u32| (handle == 0).then(|| "#white".to_string());
    let responses = editor.answer_api_lines(&[line.to_string()], world, &resolver);
    responses.into_iter().next().expect("one response per request line")
}

/// The response line is an `ok` envelope.
pub(super) fn assert_ok(response: &str) {
    assert!(response.contains("\"ok\":true"), "expected ok: {response}");
}

// Copied from crates/editor/src/test_support.rs:66-99 (the third copy after ui and editor).

/// Move the pointer to `position` and press the left button for the coming frame.
pub(super) fn press_mouse(input: &mut input::InputHandler, position: Vec2) {
    input.update();
    input.mouse_mut().update_position(position.x, position.y);
    input.mouse_mut().handle_button_press(input::prelude::MouseButton::Left);
}

/// Release the left button for the coming frame (the pointer stays put).
pub(super) fn release_mouse(input: &mut input::InputHandler) {
    input.update();
    input.mouse_mut().handle_button_release(input::prelude::MouseButton::Left);
}

/// Run one UI frame at `window_size` around `body`.
pub(super) fn ui_frame(
    ui: &mut ui::UIContext,
    input: &input::InputHandler,
    window_size: Vec2,
    body: impl FnOnce(&mut ui::UIContext),
) {
    ui.begin_frame(input, window_size);
    body(ui);
    ui.end_frame();
}

/// A track named `melody` on channel 1 — its program change at tick 0 — holding three
/// eighths: C4 at tick 0, E4 at 480 and G4 at 960.
const THREE_EIGHTHS: &[u8] = &[
    0x00, 0xFF, 0x03, 0x06, b'm', b'e', b'l', b'o', b'd', b'y', 0x00, 0xC0, 0x00,
    0x00, 0x90, 0x3C, 0x64, 0x81, 0x70, 0x80, 0x3C, 0x00, // C4 at 0, off an eighth later
    0x81, 0x70, 0x90, 0x40, 0x64, 0x81, 0x70, 0x80, 0x40, 0x00, // E4 at 480
    0x81, 0x70, 0x90, 0x43, 0x64, 0x81, 0x70, 0x80, 0x43, 0x00, // G4 at 960
    0x00, 0xFF, 0x2F, 0x00,
];

/// The microseconds a quarter lasts at 120 BPM.
const QUARTER_AT_120_BPM: u32 = 500_000;

/// A song signed as [`signed_song`] is, whose melody track (track 1) holds three eighths:
/// C4 at tick 0, E4 at 480 and G4 at 960.
pub(super) fn melody_song() -> editor::SongDocument {
    signed(&type_one_file(QUARTER_AT_120_BPM, &[THREE_EIGHTHS]))
}

/// [`melody_song`]'s melody at a quarter every 25 ms — a bar lasts a tenth of a second —
/// beside an empty track named `bass` (track 2, channel 2, program 32), signed the same.
pub(super) fn quick_melody_and_bass_song() -> editor::SongDocument {
    const BASS: &[u8] = &[
        0x00, 0xFF, 0x03, 0x04, b'b', b'a', b's', b's', 0x00, 0xC1, 0x20, 0x00, 0xFF, 0x2F, 0x00,
    ];
    signed(&type_one_file(25_000, &[THREE_EIGHTHS, BASS]))
}

/// The music crate's one-preset sine bank, under the name a new song plays its background
/// on, so a song saved in `folder` plays.
pub(super) fn stock_sound_font(folder: &std::path::Path) {
    const SINE_FONT: &[u8] = include_bytes!("../../../music_midi/fixtures/sine.sf2");
    common::vfs::write(&folder.join(editor::song::DEFAULT_SOUND_FONT_PATH), SINE_FONT).expect("the bank is written");
}

/// A song whose one note track, `melody` on channel 1 (track 1), is empty, carrying the
/// author and license Save requires: one `SetProvenance` entry.
pub(super) fn signed_song() -> editor::SongDocument {
    const EMPTY_MELODY: &[u8] = &[
        0x00, 0xFF, 0x03, 0x06, b'm', b'e', b'l', b'o', b'd', b'y', 0x00, 0xC0, 0x00, 0x00, 0xFF, 0x2F, 0x00,
    ];
    signed(&type_one_file(QUARTER_AT_120_BPM, &[EMPTY_MELODY]))
}

/// The song in `bytes`, never saved, signed by one `SetProvenance` entry.
fn signed(bytes: &[u8]) -> editor::SongDocument {
    let mut song = editor::SongDocument::from_smf(bytes, editor::song::default_song_file()).expect("the fixture reads");
    let provenance = fixture_provenance();
    song.apply(editor::SongEdit::SetProvenance { provenance }, editor::Gesture::Discrete)
        .expect("the provenance lands");
    song
}

/// The author and license every fixture song is signed with.
pub(super) fn fixture_provenance() -> editor::song::Provenance {
    editor::song::Provenance {
        author: "2026 Jesse".to_string(),
        license: "CC-BY-4.0".to_string(),
    }
}

/// One MIDI track chunk around `events` (each already a delta and an event).
fn track_chunk(events: &[u8]) -> Vec<u8> {
    let mut chunk = b"MTrk".to_vec();
    chunk.extend_from_slice(&(events.len() as u32).to_be_bytes());
    chunk.extend_from_slice(events);
    chunk
}

/// A type-1 file at 480 ticks a quarter: the timing track — a quarter lasting
/// `microseconds_per_quarter`, in 4/4 — then each of `tracks`.
fn type_one_file(microseconds_per_quarter: u32, tracks: &[&[u8]]) -> Vec<u8> {
    let [_, tempo_high, tempo_middle, tempo_low] = microseconds_per_quarter.to_be_bytes();
    let timing = [
        0x00, 0xFF, 0x51, 0x03, tempo_high, tempo_middle, tempo_low,
        0x00, 0xFF, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, // 4/4
        0x00, 0xFF, 0x2F, 0x00,
    ];
    let mut bytes = b"MThd".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 6, 0, 1, 0, tracks.len() as u8 + 1, 0x01, 0xE0]);
    bytes.extend(track_chunk(&timing));
    for track in tracks {
        bytes.extend(track_chunk(track));
    }
    bytes
}

/// A track named `pad` on channel 1, empty.
const PAD: &[u8] = &[0x00, 0xFF, 0x03, 0x03, b'p', b'a', b'd', 0x00, 0xC0, 0x00, 0x00, 0xFF, 0x2F, 0x00];

/// A track named `lead` on channel 2 holding one C5, a quarter long.
const LEAD: &[u8] = &[
    0x00, 0xFF, 0x03, 0x04, b'l', b'e', b'a', b'd', 0x00, 0xC1, 0x00,
    0x00, 0x91, 0x48, 0x5A, // C5 on, channel 2
    0x83, 0x60, 0x81, 0x48, 0x00, // off a quarter later
    0x00, 0xFF, 0x2F, 0x00,
];

/// A song whose first track with a channel (`pad`, track 1) is empty and whose second
/// (`lead`, track 2) holds one C5.
pub(super) fn second_track_holds_the_notes() -> editor::SongDocument {
    let bytes = type_one_file(QUARTER_AT_120_BPM, &[PAD, LEAD]);
    editor::SongDocument::from_smf(&bytes, editor::song::default_song_file()).expect("the fixture reads")
}

/// [`second_track_holds_the_notes`] with a whole C3 on the pad from tick 0 — 1920 ticks, a
/// background the song lasts by while the lead's part is bound.
pub(super) fn pad_under_the_lead_song() -> editor::SongDocument {
    const PAD_WITH_A_WHOLE_NOTE: &[u8] = &[
        0x00, 0xFF, 0x03, 0x03, b'p', b'a', b'd', 0x00, 0xC0, 0x00,
        0x00, 0x90, 0x30, 0x5A, // C3 on
        0x8F, 0x00, 0x80, 0x30, 0x00, // off a whole note later
        0x00, 0xFF, 0x2F, 0x00,
    ];
    let bytes = type_one_file(QUARTER_AT_120_BPM, &[PAD_WITH_A_WHOLE_NOTE, LEAD]);
    editor::SongDocument::from_smf(&bytes, editor::song::default_song_file()).expect("the fixture reads")
}

/// A song of one track holding timing alone: nowhere a note could land.
pub(super) fn timing_only_song() -> editor::SongDocument {
    let bytes = type_one_file(QUARTER_AT_120_BPM, &[]);
    editor::SongDocument::from_smf(&bytes, editor::song::default_song_file()).expect("the fixture reads")
}

/// Hold `song` as the open song, as an open does: a new generation, the Conductor panel following
/// it, the dirty mirrors synced.
pub(super) fn hold_song<G: Game>(editor: &mut EditorGame<G>, song: editor::SongDocument) {
    editor.song = Some(song);
    editor.song_generation += 1;
    editor.follow_song();
    editor.sync_dirty_mirror();
}

/// A session whose asset base is `folder`, holding `song`, with the Conductor panel shown in a
/// 1280 × 720 dock and its sidebar holding the keyboard.
pub(super) fn conductor_session(folder: &std::path::Path, song: editor::SongDocument) -> EditorGame<DummyGame> {
    let mut editor = editor_game();
    editor.asset_base = folder.to_path_buf();
    stock_sound_font(folder);
    editor.song = Some(song);
    editor.follow_song();
    editor.editor.dock_area.set_bounds(ui::Rect::new(0.0, 24.0, 1280.0, 672.0));
    editor.show_conductor(editor::conductor::ConductorFocus::Sidebar);
    editor
}
