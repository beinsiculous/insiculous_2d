//! The song's hot reload through the real `EditorGame`: what the open song and a play
//! session put in the watched set, the editor's own song save as the watcher's baseline, the
//! open document following its score and its map — the map's prompt answered as the scene's
//! is — the session's song replaced at its next bar line or refused by the track diff, and the
//! web's import mailbox. Every wait is a bounded number of steps, each a step of the song's
//! virtual clock and the same step of wall time for the worker that prepares the song.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::clock::Instant;
use editor::console::Severity;
use editor::external_tools::{midi_sidecar_path, FileChange, WatchEvent};
use editor::{ConfirmChoice, PlayControlAction, SongDocument};
use engine_core::prelude::SoundSources;
use winit::keyboard::KeyCode;

use super::hot_reload::{SongOwner, WatchOwner};
use super::hot_reload_tests::Session;
use super::run_options::SongImport;
use super::scene_confirm::PendingSceneAction;
use super::test_support::{api_line, quick_melody_and_bass_song, stock_sound_font};

const SONG: &str = "music/theme.mid";

/// At most this many steps of [`STEP`] wait for a swap: four seconds of a song whose bar
/// lasts a tenth of one, on the virtual clock.
const SWAP_STEPS: usize = 2000;
const STEP: Duration = Duration::from_millis(2);

fn json(response: &str) -> serde_json::Value {
    serde_json::from_str(response).expect("one line of JSON")
}

fn edit(session: &mut Session, line: &str) {
    let response = json(&api_line(&mut session.editor, &mut session.world, line));
    assert_eq!(response["ok"], true, "{line}: {response}");
}

/// A session over a temporary project holding the bank and the melody saved at [`SONG`] —
/// a quarter every 25 ms, so a bar lasts a tenth of a second, a second track `bass`, and a
/// part `lead` on the melody track by its name — the save acknowledged and the set watched.
fn session_with_a_saved_song() -> Session {
    let mut session = Session::new();
    stock_sound_font(session.project.path());
    session.audio.set_asset_base(session.project.path());
    session.editor.song = Some(quick_melody_and_bass_song());
    session.editor.follow_song();
    edit(&mut session, r#"song edit {"AddPart": {"part": {"name": "lead", "track": {"Name": "melody"}}}}"#);
    edit(&mut session, &format!("song save {SONG}"));
    acknowledge(&mut session);
    tick(&mut session);
    session
}

/// The worker's acknowledgement of every save so far, dispatched.
fn acknowledge(session: &mut Session) {
    let acknowledgements = session.acknowledgements();
    session.dispatch_events(acknowledgements, false);
}

/// The watcher's tick: the marks read against the set as the worker last heard it, then the
/// set as the world, the open song and the session name it now.
fn tick(session: &mut Session) {
    let mut expected_absent = session.editor.expected_absent_paths(&session.assets);
    expected_absent.extend(session.editor.absent_song_paths(&session.audio));
    let mut watched = session.editor.watched_paths(&session.world, &session.assets);
    session.editor.watch_songs(&mut watched, &session.audio);
    session.editor.hot_reload.watched = watched;
    session.editor.hot_reload.expected_absent = expected_absent;
}

/// Play, carried through its two frames: the saved song plays, then the session starts.
fn play(session: &mut Session) {
    let mut sources = SoundSources::default();
    session.editor.handle_play_action(PlayControlAction::Play, &mut session.world);
    let report = session.editor.apply_song_intents(&mut session.audio, &mut sources);
    assert_eq!(report.session_play, Some(Ok(())));
    session.editor.start_armed_play(&mut session.world);
    assert!(session.editor.editor.is_playing());
    tick(session);
}

/// Run the session's song on its virtual clock, `steps` steps of [`STEP`] from `now` — each
/// step also waits [`STEP`] of wall time, since the song is prepared on a worker thread and a
/// clock that only pretends to advance would outrun it — each update's replacement outcomes
/// reported as the frame reports them; stops at the first.
fn run(session: &mut Session, now: &mut Instant, steps: usize) {
    for _ in 0..steps {
        std::thread::sleep(STEP);
        *now += STEP;
        session.audio.update_at(*now);
        session.editor.report_song_replacements(&mut session.audio);
        if session.console_mentions("song replaced in bar") || session.console_mentions("was not replaced") {
            return;
        }
    }
}

fn midi_path(session: &Session) -> PathBuf {
    session.path(SONG)
}

fn sidecar_path(session: &Session) -> PathBuf {
    midi_sidecar_path(&midi_path(session))
}

fn sidecar_text(session: &Session) -> String {
    std::fs::read_to_string(sidecar_path(session)).expect("the sidecar")
}

/// The saved score with its melody track renamed `melodx`: the name's bytes replaced in
/// place, the chunk's length unchanged.
fn renamed_score(session: &Session) -> Vec<u8> {
    let mut bytes = std::fs::read(midi_path(session)).expect("the score");
    let at = bytes.windows(6).position(|window| window == b"melody").expect("the track's name");
    bytes[at + 5] = b'x';
    bytes
}

/// The velocity [`score_with_a_softer_first_note`] gives the melody's first note.
const SOFTER: u8 = 0x40;

/// The saved score with the melody's first note played softer, as MIDI Evil would export
/// it: the note-on's velocity replaced in place, the chunk's length unchanged.
fn score_with_a_softer_first_note(session: &Session) -> Vec<u8> {
    let mut bytes = std::fs::read(midi_path(session)).expect("the score");
    let at = bytes.windows(3).position(|window| window == [0x90, 0x3C, 0x64]).expect("C4's note-on");
    bytes[at + 2] = SOFTER;
    bytes
}

/// The velocity of the first note on the melody (track 1) of `data`.
fn first_melody_velocity(data: &audio::SongData) -> u8 {
    data.tracks()[1].units()[0].notes[0].velocity
}

fn console_errors(session: &Session) -> Vec<String> {
    session.console().into_iter().filter(|(severity, _)| *severity == Severity::Error).map(|(_, text)| text).collect()
}

fn song(session: &Session) -> &SongDocument {
    session.editor.song.as_ref().expect("a song is open")
}

#[test]
fn the_watched_set_names_the_open_songs_two_files_and_in_a_session_its_saved_song_and_closure() {
    let mut session = session_with_a_saved_song();
    let (midi, sidecar) = (midi_path(&session), sidecar_path(&session));
    let bank = session.path(editor::song::DEFAULT_SOUND_FONT_PATH);
    let owners = |session: &Session, path: &PathBuf| session.editor.hot_reload.watched.get(path).cloned().unwrap_or_default();
    let document = WatchOwner::Song(SongOwner::Document);
    let in_session = WatchOwner::Song(SongOwner::Session);

    assert!(owners(&session, &midi).contains(&document) && owners(&session, &sidecar).contains(&document));
    assert!(owners(&session, &bank).is_empty(), "the bank is the session's, and none runs");

    play(&mut session);
    for path in [&midi, &sidecar, &bank] {
        assert!(owners(&session, path).contains(&in_session), "{} is the session's: {:?}", path.display(), session.editor.hot_reload.watched);
    }
    assert!(owners(&session, &midi).contains(&document), "and the open song's still");
    assert!(session.editor.hot_reload.expected_absent.is_empty(), "every file of the closure is there");
}

#[test]
fn a_song_save_notes_the_files_it_wrote_and_a_change_already_queued_for_them_reloads_nothing() {
    let mut session = session_with_a_saved_song();
    let (midi, sidecar) = (midi_path(&session), sidecar_path(&session));
    session.editor.hot_reload.incoming.push(WatchEvent::Change(FileChange::Changed(sidecar.clone())));

    edit(&mut session, r#"song edit {"SetLoop": {"looping": false}}"#);
    edit(&mut session, &format!("song save {SONG}"));
    let noted: Vec<PathBuf> = session.editor.hot_reload.awaiting_note.keys().cloned().collect();
    assert_eq!(noted, [sidecar], "a map save leaves the score, and notes the sidecar alone");
    acknowledge(&mut session);
    assert!(!session.console_mentions("changed on disk"), "{:?}", session.console());
    assert!(!song(&session).file().looping, "the map the editor saved is the one open");

    let copy = session.path("music/copy.mid");
    edit(&mut session, "song save music/copy.mid");
    let noted: Vec<PathBuf> = session.editor.hot_reload.awaiting_note.keys().cloned().collect();
    assert_eq!(noted, [copy.clone(), midi_sidecar_path(&copy)], "a Save As writes and notes both files");
    let written = std::fs::read(&copy).expect("the copied score");
    assert_eq!(written, std::fs::read(&midi).expect("the score"), "the copy is the loaded bytes");
}

#[test]
fn a_save_over_the_changed_map_dismisses_its_reload_prompt() {
    let mut session = session_with_a_saved_song();
    edit(&mut session, r#"song edit {"SetLoop": {"looping": false}}"#);
    let outside = sidecar_text(&session).replace("\"lead\"", "\"melody_part\"");
    std::fs::write(sidecar_path(&session), outside).expect("another program writes the map");
    session.dispatch(&[FileChange::Changed(sidecar_path(&session))], false);
    let midi = midi_path(&session);
    assert_eq!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::ReloadSongMap(midi)));

    // The save's own path, as the scene's test saves past the dialog: the API refuses any
    // line while a dialog is up.
    let saved = session.editor.save_song_to(midi_path(&session), editor::song::SaveIntent::OVERWRITE);
    assert_eq!(saved, Ok(Some(midi_path(&session))));

    assert_eq!(session.editor.scene_confirm.pending_action, None, "the sidecar holds the editor's own map now");
    assert!(!song(&session).file().looping, "the edit the prompt asked about is the one saved");
}

#[test]
fn a_changed_score_is_adopted_with_the_diffs_lines_and_the_map_and_its_edits_kept() {
    let mut session = session_with_a_saved_song();
    edit(&mut session, r#"song edit {"SetLoop": {"looping": false}}"#);
    std::fs::write(midi_path(&session), renamed_score(&session)).expect("another program writes the score");

    session.dispatch(&[FileChange::Changed(midi_path(&session))], false);

    let names: Vec<Option<String>> = song(&session).data().tracks().iter().map(|track| track.name().map(str::to_string)).collect();
    assert!(names.contains(&Some("melodx".to_string())), "the score on disk is the song's: {names:?}");
    let errors = console_errors(&session);
    assert!(errors.iter().any(|line| line.contains("lead")), "the diff refuses the part it cannot play: {errors:?}");
    assert!(!song(&session).file().looping && song(&session).is_dirty(), "the map and its unsaved edit are kept");
    assert!(song(&session).history().can_undo(), "the map's entry survives the score's adoption");
}

#[test]
fn a_clean_map_reloads_and_a_dirty_one_asks_where_enter_and_escape_keep_the_edits_and_reload_is_a_click() {
    let mut session = session_with_a_saved_song();
    let sidecar = sidecar_path(&session);
    let midi = midi_path(&session);
    std::fs::write(&sidecar, sidecar_text(&session).replace("looping: true", "looping: false")).expect("written");
    session.dispatch(&[FileChange::Changed(sidecar.clone())], false);
    assert!(!song(&session).file().looping, "a clean song takes the map at once");
    assert!(session.console_mentions("map reloaded from disk"), "{:?}", session.console());

    edit(&mut session, r#"song edit {"SetLoop": {"looping": true}}"#);
    std::fs::write(&sidecar, sidecar_text(&session).replace("\"lead\"", "\"melody_part\"")).expect("written");
    for key in [KeyCode::Enter, KeyCode::Escape] {
        session.dispatch(&[FileChange::Changed(sidecar.clone())], false);
        assert_eq!(session.editor.scene_confirm.pending_action, Some(PendingSceneAction::ReloadSongMap(midi.clone())));
        assert!(session.editor.confirm_dialog_consumes_key(key), "the prompt takes {key:?}");
        let choice = session.editor.scene_confirm.pending_choice.take();
        assert_eq!(choice, Some(ConfirmChoice::Cancel), "{key:?} keeps mine: the prompt appears unasked");
        let action = PendingSceneAction::ReloadSongMap(midi.clone());
        session.editor.resolve_song_choice(action, choice.expect("the key's choice"));
        assert_eq!(session.editor.scene_confirm.pending_action, None);
        assert!(song(&session).file().looping && song(&session).is_dirty(), "{key:?} kept the edit");
    }

    session.dispatch(&[FileChange::Changed(sidecar)], false);
    session.editor.resolve_song_choice(PendingSceneAction::ReloadSongMap(midi), ConfirmChoice::Confirm);
    let parts: Vec<String> = song(&session).file().parts.iter().map(|part| part.name.clone()).collect();
    assert_eq!(parts, ["melody_part"], "Reload takes the map on disk");
    assert!(!song(&session).file().looping, "and drops the edit");
}

#[test]
fn a_saved_song_rewritten_on_disk_during_play_swaps_at_the_next_bar_and_the_console_says_so() {
    let mut session = session_with_a_saved_song();
    play(&mut session);
    let before = Arc::clone(session.audio.playing_song().expect("the saved song plays"));
    std::fs::write(midi_path(&session), score_with_a_softer_first_note(&session)).expect("another program writes the score");
    let mut now = Instant::now();

    session.dispatch(&[FileChange::Changed(midi_path(&session))], false);
    run(&mut session, &mut now, SWAP_STEPS);

    assert!(session.console_mentions("song replaced in bar"), "{:?}", session.console());
    let after = session.audio.playing_song().expect("the new song plays");
    assert!(!Arc::ptr_eq(&before, after));
    assert_eq!(first_melody_velocity(after.data()), SOFTER, "the score on disk is what plays");
    assert_eq!(first_melody_velocity(song(&session).data()), SOFTER, "and what the open song follows");
}

#[test]
fn a_track_renamed_in_the_new_score_refuses_the_swap_and_the_old_song_plays_on() {
    let mut session = session_with_a_saved_song();
    play(&mut session);
    let before = Arc::clone(session.audio.playing_song().expect("the saved song plays"));
    std::fs::write(midi_path(&session), renamed_score(&session)).expect("another program writes the score");
    let mut now = Instant::now();

    session.dispatch(&[FileChange::Changed(midi_path(&session))], false);
    run(&mut session, &mut now, 100);

    assert!(console_errors(&session).iter().any(|line| line.contains("lead")), "{:?}", session.console());
    assert!(session.console_mentions("not replaced, the song plays on as it was"), "{:?}", session.console());
    assert!(!session.console_mentions("song replaced in bar"), "no request was made");
    assert!(Arc::ptr_eq(&before, session.audio.playing_song().expect("the old song plays on")));
    assert!(session.audio.is_song_playing());
}

#[test]
fn a_sidecar_that_changes_a_selector_on_purpose_is_accepted_and_swaps() {
    let mut session = session_with_a_saved_song();
    play(&mut session);
    let reselected = sidecar_text(&session).replace("Name(\"melody\")", "Name(\"bass\")");
    assert_ne!(reselected, sidecar_text(&session), "the part's selector is in the sidecar as written");
    std::fs::write(sidecar_path(&session), reselected).expect("another program writes the map");
    let mut now = Instant::now();

    session.dispatch(&[FileChange::Changed(sidecar_path(&session))], false);
    run(&mut session, &mut now, SWAP_STEPS);

    assert!(session.console_mentions("`lead` now selects"), "{:?}", session.console());
    assert!(session.console_mentions("song replaced in bar"), "{:?}", session.console());
    let part = &session.audio.playing_song().expect("plays").file().parts[0];
    assert_eq!(part.track.to_string(), "track name 'bass'", "the new map plays");
}

#[test]
fn a_new_sidecar_naming_an_absent_bank_fails_and_the_bank_is_watched_as_expected_absent() {
    let mut session = session_with_a_saved_song();
    play(&mut session);
    let before = Arc::clone(session.audio.playing_song().expect("the saved song plays"));
    let bank = editor::song::DEFAULT_SOUND_FONT_PATH;
    let absent = sidecar_text(&session).replace(bank, "soundfonts/absent.sf2");
    std::fs::write(sidecar_path(&session), absent).expect("another program writes the map");
    let mut now = Instant::now();

    session.dispatch(&[FileChange::Changed(sidecar_path(&session))], false);
    run(&mut session, &mut now, SWAP_STEPS);

    let errors = console_errors(&session);
    assert!(errors.iter().any(|line| line.contains("was not replaced") && line.contains("absent.sf2")), "{errors:?}");
    assert!(Arc::ptr_eq(&before, session.audio.playing_song().expect("the old song plays on")));
    // The bank arrives before the tick: the mark is the failed preparation's, not a look at
    // the disk, so its arrival is still a change to the worker.
    let absent_bank = session.path("soundfonts/absent.sf2");
    std::fs::copy(session.path(bank), &absent_bank).expect("the bank arrives");
    tick(&mut session);
    let owners = session.editor.hot_reload.watched.get(&absent_bank).cloned().unwrap_or_default();
    assert!(owners.contains(&WatchOwner::Song(SongOwner::Session)), "{:?}", session.editor.hot_reload.watched);
    assert!(session.editor.hot_reload.expected_absent.contains(&absent_bank), "its arrival is a change");
}

#[test]
fn an_external_change_stops_a_sounding_preview_as_an_edit_does() {
    let mut session = session_with_a_saved_song();
    let mut sources = SoundSources::default();
    session.editor.preview_play(0).expect("the preview plays");
    session.editor.apply_song_intents(&mut session.audio, &mut sources);
    assert!(session.editor.song_preview.is_playing(), "the preview sounds the saved score");
    std::fs::write(midi_path(&session), score_with_a_softer_first_note(&session)).expect("another program writes the score");

    session.dispatch(&[FileChange::Changed(midi_path(&session))], false);

    assert!(!session.editor.song_preview.is_playing(), "the prepared score is the old one");
    assert!(session.console_mentions("Song preview stopped: the song changed on disk"), "{:?}", session.console());
    session.editor.apply_song_intents(&mut session.audio, &mut sources);
    assert!(!session.audio.is_song_playing(), "the drain stopped the slot");
}

#[test]
fn an_unmapped_songs_missing_sidecar_is_marked_expected_absent() {
    let mut session = session_with_a_saved_song();
    let bare = session.path("music/bare.mid");
    std::fs::copy(midi_path(&session), &bare).expect("a bare score");
    edit(&mut session, &format!("song open {}", bare.display()));
    assert!(song(&session).map_fingerprint().is_none(), "opened in place, no sidecar on disk");

    tick(&mut session);

    let sidecar = midi_sidecar_path(&bare);
    assert!(session.editor.hot_reload.watched.contains_key(&sidecar), "{:?}", session.editor.hot_reload.watched);
    assert!(session.editor.hot_reload.expected_absent.contains(&sidecar), "a sidecar another program writes is a change");
}

#[test]
fn an_import_of_the_open_songs_midi_is_adopted_and_one_for_another_song_is_dropped_with_a_line() {
    let mut session = session_with_a_saved_song();
    let mailbox = Arc::new(Mutex::new(None));
    session.editor.song_import = Some(Arc::clone(&mailbox));
    let bytes = renamed_score(&session);

    *mailbox.lock().expect("the mailbox") = Some(SongImport { path: midi_path(&session), bytes: bytes.clone() });
    session.editor.drain_song_import();
    assert!(mailbox.lock().expect("the mailbox").is_none(), "the frame takes it");
    assert_eq!(song(&session).source_bytes(), bytes.as_slice(), "the open song follows the import");
    assert!(console_errors(&session).iter().any(|line| line.contains("lead")), "{:?}", session.console());

    *mailbox.lock().expect("the mailbox") = Some(SongImport { path: session.path("music/other.mid"), bytes: Vec::new() });
    session.editor.drain_song_import();
    assert!(session.console_mentions("music/other.mid was imported, but it is no longer the open song"));
    assert_eq!(song(&session).source_bytes(), bytes.as_slice(), "the open song is left alone");
}
