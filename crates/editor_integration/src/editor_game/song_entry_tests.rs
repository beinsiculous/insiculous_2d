//! The music entry points through the real `EditorGame` in a temporary project: a MIDI file
//! opened at its first part's track (resolved, never launched), which song is "the" song arm
//! by arm, and the emitter's button — its request, its errors and its Song row.

use std::path::PathBuf;

use ecs::{SoundEmitter, SoundRef, World};
use editor::console::Severity;
use editor::external_tools::{CommandTemplate, CustomTool, Launch, Location, OpenRequest, Resolution, ToolChoice};

use super::hot_reload_tests::Session;
use super::init_music::InitMusic;
use super::song_entry::{SongByRule, NO_SONG};
use super::test_support::melody_song;

const NAPOLITAN: &[u8] = include_bytes!("../../../music_midi/fixtures/dansenapolitaine.mid");

/// The Napolitan dance's parts: `melody` on the track named `flute` (index 1), `drums` on
/// index 2 (`tambourin`), `bass_line` on the track named `lowera` (index 4).
const SIDECAR: &str = r#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2",
    parts: [(name: "melody", track: Name("flute")), (name: "drums", track: Index(2)), (name: "bass_line", track: Name("lowera"))],
    instruments: [(name: "tuba", source: SoundFont(path: "soundfonts/cast.sf2", bank: 0, preset: 58))],
    bindings: [
        (action: "coin", part: "melody", instrument: "tuba"),
        (action: "bump", part: "drums", instrument: "tuba"),
        (action: "step", part: "bass_line", instrument: "tuba"),
    ])"#;

/// The same dance with `coin` on the drums.
const COIN_ON_THE_DRUMS: &str = r#"SongFile(version: 1, soundfont: "soundfonts/cast.sf2",
    parts: [(name: "drums", track: Index(2))],
    instruments: [(name: "tuba", source: SoundFont(path: "soundfonts/cast.sf2", bank: 0, preset: 58))],
    bindings: [(action: "coin", part: "drums", instrument: "tuba")])"#;

fn with_song(session: &Session, midi: &str, sidecar: Option<&str>) {
    common::vfs::write(&session.path(midi), NAPOLITAN).expect("the song is written");
    if let Some(text) = sidecar {
        session.write(&format!("{midi}.ron"), text);
    }
}

fn track(name: &str, index: usize) -> Location {
    Location::Track { name: Some(name.to_string()), index }
}

fn request(path: &str) -> OpenRequest {
    OpenRequest { path: PathBuf::from(path), location: Location::None }
}

/// What the launcher would run for `request` after the upgrade, with the music tool an
/// `echo` that names the track — resolved, never spawned.
fn echoed(session: &mut Session, request: OpenRequest) -> Vec<String> {
    session.editor.editor.external_tools.music =
        ToolChoice::Custom(CustomTool::file_only(CommandTemplate("echo {file} [--track {track}] --track-index {track_index}".to_string())));
    let upgraded = session.editor.at_first_part_track(request);
    match session.editor.resolve_open_request(&upgraded) {
        Ok((_, Resolution::Tool(Launch { args, .. }))) => args,
        other => panic!("the custom command resolves: {other:?}"),
    }
}

#[test]
fn a_mid_beside_its_sidecar_launches_at_its_first_parts_track_and_a_midi_the_same() {
    let mut session = Session::new();
    with_song(&session, "music/dance.mid", Some(SIDECAR));
    with_song(&session, "music/waltz.midi", Some(SIDECAR));

    let file = session.path("music/dance.mid").to_string_lossy().into_owned();
    assert_eq!(echoed(&mut session, request("music/dance.mid")), [file, "--track".into(), "flute".into(), "--track-index".into(), "1".into()]);
    let waltz = echoed(&mut session, request("music/waltz.midi"));
    assert_eq!(waltz[1..], ["--track", "flute", "--track-index", "1"], "a .midi reads its .midi.ron");
    assert!(session.console().is_empty(), "nothing to explain: {:?}", session.console());
}

#[test]
fn a_mid_alone_opens_at_no_track_and_the_console_says_why() {
    let mut session = Session::new();
    with_song(&session, "music/alone.mid", None);

    let upgraded = session.editor.at_first_part_track(request("music/alone.mid"));

    assert_eq!(upgraded, request("music/alone.mid"));
    let sidecar = session.path("music/alone.mid.ron");
    let expected = format!("alone.mid opens at no track: {} does not exist", sidecar.display());
    assert_eq!(session.console(), [(Severity::Info, expected)]);
}

#[test]
fn a_request_for_another_file_or_already_at_a_location_is_untouched() {
    let mut session = Session::new();
    with_song(&session, "music/dance.mid", Some(SIDECAR));
    let at_a_line = OpenRequest { path: PathBuf::from("music/dance.mid"), location: Location::Line { line: 3, column: None } };

    assert_eq!(session.editor.at_first_part_track(request("scripts/paddle.rhai")), request("scripts/paddle.rhai"));
    assert_eq!(session.editor.at_first_part_track(at_a_line.clone()), at_a_line);
    assert!(session.console().is_empty());
}

#[test]
fn the_rule_takes_the_slot_in_a_session_then_the_open_song_then_the_init_song_then_the_list() {
    let mut session = Session::new();
    with_song(&session, "music/listed.mid", Some(SIDECAR));
    let song = |path: &str| SongByRule::Song(PathBuf::from(path));
    assert_eq!(session.editor.song_by_rule(), song("music/listed.mid"), "the list's one song");

    session.editor.init_music = InitMusic::Song(session.path("music/init.mid"));
    assert_eq!(session.editor.song_by_rule(), song("music/init.mid"), "what init asked for before the list");

    let mut open = melody_song();
    open.save(&session.path("music/open.mid"), session.project.path()).expect("the song saves");
    session.editor.song = Some(open);
    assert_eq!(session.editor.song_by_rule(), song("music/open.mid"), "the open song, which the next Play plays, before what init asked for");

    session.editor.slot_song = Some(session.path("music/slot.mid"));
    assert_eq!(session.editor.song_by_rule(), song("music/open.mid"), "the slot speaks only in a play session");
    session.editor.editor.set_play_state(editor::EditorPlayState::Paused);
    assert_eq!(session.editor.song_by_rule(), song("music/slot.mid"), "and there it wins");
}

#[test]
fn a_list_never_scanned_is_scanned_before_the_rule_reads_it_and_an_empty_project_is_the_error() {
    let mut session = Session::new();
    with_song(&session, "ai/music/only.mid", Some(SIDECAR));
    assert!(!session.editor.editor.composer.songs_scanned, "the composer never drew");
    assert_eq!(session.editor.the_song(), Ok(PathBuf::from("ai/music/only.mid")));

    let mut empty = Session::new();
    assert_eq!(empty.editor.the_song(), Err(NO_SONG.to_string()));
    empty.editor.editor.edit_binding.requested = Some("coin".to_string());
    empty.editor.open_requested_binding();
    assert_eq!(empty.console(), [(Severity::Error, NO_SONG.to_string())]);
    assert!(empty.editor.editor.open_requests.is_empty());
}

#[test]
fn the_button_files_the_songs_mid_at_its_bindings_track_and_an_unknown_action_lists_the_actions() {
    let mut session = Session::new();
    with_song(&session, "music/dance.mid", Some(SIDECAR));

    session.editor.editor.edit_binding.requested = Some("step".to_string());
    session.editor.open_requested_binding();
    assert_eq!(
        std::mem::take(&mut session.editor.editor.open_requests),
        [OpenRequest { path: PathBuf::from("music/dance.mid"), location: track("lowera", 4) }],
        "the part's track by its name and index, not the binding's position"
    );

    session.editor.editor.edit_binding.requested = Some("jump".to_string());
    session.editor.open_requested_binding();
    assert!(session.editor.editor.open_requests.is_empty());
    assert_eq!(
        session.console(),
        [(Severity::Error, "music/dance.mid.ron binds no action 'jump' — its actions are coin, bump, step".to_string())]
    );
}

#[test]
fn two_songs_offer_the_song_row_to_a_selected_binding_and_the_choice_decides_which_sidecar_is_read() {
    let mut session = Session::new();
    with_song(&session, "music/a.mid", Some(SIDECAR));
    with_song(&session, "music/b.mid", Some(COIN_ON_THE_DRUMS));
    let mut world = World::new();
    let emitter = world.create_entity();
    world
        .add_component(&emitter, SoundEmitter { sound: SoundRef::Binding("coin".into()), ..SoundEmitter::default() })
        .ok();

    session.editor.offer_song_choices(&world);
    assert!(session.editor.editor.edit_binding.song_choices.is_empty(), "nothing selected, nothing listed");

    session.editor.editor.selection.select(emitter);
    session.editor.offer_song_choices(&world);
    assert_eq!(session.editor.editor.edit_binding.song_choices, ["music/a.mid", "music/b.mid"]);

    let opened = |session: &mut Session| {
        session.editor.editor.edit_binding.requested = Some("coin".to_string());
        session.editor.open_requested_binding();
        std::mem::take(&mut session.editor.editor.open_requests)
    };
    assert_eq!(opened(&mut session), [OpenRequest { path: "music/a.mid".into(), location: track("flute", 1) }], "the first until one is chosen");
    session.editor.editor.edit_binding.chosen_song = Some("music/b.mid".to_string());
    assert_eq!(opened(&mut session), [OpenRequest { path: "music/b.mid".into(), location: track("tambourin", 2) }]);
}

#[test]
fn a_hidden_composer_does_not_make_the_rule_scan_the_song_folders_every_frame() {
    let mut session = Session::new();
    with_song(&session, "music/a.mid", Some(SIDECAR));
    with_song(&session, "music/b.mid", Some(SIDECAR));
    let entity = session.world.create_entity();
    session.world.add_component(&entity, SoundEmitter { sound: SoundRef::Binding("coin".to_string()), ..SoundEmitter::default() }).expect("the emitter");
    session.editor.editor.selection.select(entity);
    let input = input::InputHandler::new();
    let mut ui = ui::UIContext::new();
    ui.begin_frame(&input, glam::Vec2::new(1280.0, 720.0));
    assert!(!session.editor.editor.composer.panel_drawn, "the composer never drew");

    session.editor.settle_composer(&ui);
    session.editor.offer_song_choices(&session.world);
    assert_eq!(session.editor.editor.edit_binding.song_choices, ["music/a.mid", "music/b.mid"]);
    assert!(session.editor.editor.composer.songs_scanned, "scanned once for the rule");

    session.editor.settle_composer(&ui);
    assert!(session.editor.editor.composer.songs_scanned, "a hidden frame does not forget the scan: only a close does");
    session.editor.editor.composer.panel_drawn = true;
    session.editor.settle_composer(&ui);
    assert!(!session.editor.editor.composer.songs_scanned, "the frame after the panel drew and is gone is the close");
    assert!(!session.editor.editor.composer.panel_drawn);
    ui.end_frame();
}
