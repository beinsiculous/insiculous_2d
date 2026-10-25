//! A game's song through `GameMusic` and `GameHarness`: the identity rule a match start
//! decides by, the pause menu's open-frame latch, the beat pulse, and a song that will not
//! load. The harness's audio manager is the disabled one, so songs play on its virtual
//! clock, advanced by the wall time between frames.

use std::path::Path;
use std::time::Duration;

use engine_core::prelude::{BeatPulse, Game, GameConfig, GameContext, GameMusic, KeyCode, Subdivision};
use engine_core::test_support::{GameHarness, InputEvent};
use tempfile::TempDir;

const FRAME: f32 = 1.0 / 60.0;
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../music_midi/fixtures");
const SONG: &str = "songs/theme.mid";
const JAM: &str = "jam";

/// The cast fixture's flute track as the one part, on the sine bank, walked by `jam`.
const SIDECAR: &str = r#"SongFile(
    version: 1,
    soundfont: "soundfonts/sine.sf2",
    parts: [(name: "melody", track: Name("channel 2"))],
    instruments: [(name: "sine", source: SoundFont(path: "soundfonts/sine.sf2", bank: 0, preset: 0))],
    bindings: [(action: "jam", part: "melody", instrument: "sine")],
)"#;

/// What one update's two pulse requests answered, and whether that update crossed a bar.
#[derive(Debug, Clone, Copy)]
struct PulseFrame {
    first: Option<BeatPulse>,
    second: Option<BeatPulse>,
    bar_crossed: bool,
}

/// A game that holds nothing but its music: the test sets whether the pause menu is up and
/// whether the world runs, and reads back what the module answered.
struct Stage {
    music: GameMusic,
    paused: bool,
    running: bool,
    jams: u64,
    pulses: Vec<PulseFrame>,
}

impl Stage {
    fn new() -> Self {
        Self {
            music: GameMusic::new(SONG, "stage", "STAGE_TELEMETRY"),
            paused: false,
            running: false,
            jams: 0,
            pulses: Vec::new(),
        }
    }
}

impl Game for Stage {
    fn init(&mut self, ctx: &mut GameContext) {
        self.music.start(ctx);
    }

    fn update(&mut self, ctx: &mut GameContext) {
        let jams = self.jams;
        self.music.update(ctx, || format!("jams={jams}"));
        if self.music.follow_pause(ctx, self.paused, JAM) {
            self.jams += 1;
        }
        if self.running && !self.paused {
            let first = self.music.beat_pulse(ctx);
            let second = self.music.beat_pulse(ctx);
            let bar_crossed = ctx.audio.beat_crossed(Subdivision::Bar);
            self.pulses.push(PulseFrame { first, second, bar_crossed });
        }
    }
}

/// The song, its bank and a one-bar song beside it, under one asset base.
struct Fixture {
    directory: TempDir,
    harness: GameHarness<Stage>,
}

impl Fixture {
    fn new() -> Self {
        Self::with_song(true)
    }

    /// `with_theme` false leaves the game's own song out, so every request of it fails.
    fn with_song(with_theme: bool) -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        let place = |to: &str, bytes: &[u8]| {
            let target = directory.path().join(to);
            std::fs::create_dir_all(target.parent().expect("a parent")).expect("directories");
            std::fs::write(target, bytes).expect("writes");
        };
        let fixture = |name: &str| std::fs::read(Path::new(FIXTURES).join(name)).expect("the fixture reads");
        place("soundfonts/sine.sf2", &fixture("sine.sf2"));
        if with_theme {
            place(SONG, &fixture("dansenapolitaine-cast.mid"));
            place(&format!("{SONG}.ron"), SIDECAR.as_bytes());
        }
        for (stem, looping) in [("looping", true), ("ended", false)] {
            place(&format!("songs/{stem}.mid"), &one_bar_midi());
            let sidecar = format!(
                "SongFile(version: 1, soundfont: \"soundfonts/sine.sf2\", looping: {looping}, parts: [], instruments: [], bindings: [])"
            );
            place(&format!("songs/{stem}.mid.ron"), sidecar.as_bytes());
        }
        let config = GameConfig::new("Game music").with_asset_base_path(directory.path().to_string_lossy().to_string());
        let mut harness = GameHarness::new(Stage::new(), config);
        harness.step(FRAME, &[]);
        Self { directory, harness }
    }

    fn label_of(&self, relative: &str) -> String {
        format!("{}/{relative}", self.directory.path().to_string_lossy())
    }

    fn restart(&mut self) {
        self.harness.context(|game, ctx| game.music.restart(ctx));
    }

    /// Frames with wall time between them, so the virtual clock renders blocks and the
    /// conductor drains what the frames sent.
    fn settle(&mut self) {
        for _ in 0..6 {
            std::thread::sleep(Duration::from_millis(3));
            self.harness.step(FRAME, &[]);
        }
    }

    fn wait(&mut self, frames: usize, between: Duration) {
        for _ in 0..frames {
            std::thread::sleep(between);
            self.harness.step(FRAME, &[]);
        }
    }

    fn key(&mut self, code: KeyCode) {
        self.harness.step(FRAME, &[InputEvent::KeyPressed(code)]);
        self.harness.step(FRAME, &[InputEvent::KeyReleased(code)]);
    }

    fn accepted_triggers(&self) -> u64 {
        self.harness.audio().song_snapshot().expect("a song is in the slot").conductor.accepted_triggers
    }
}

/// Type 0, 480 ticks a quarter: one middle C held for a bar.
fn one_bar_midi() -> Vec<u8> {
    let track: &[u8] = &[
        0x00, 0x90, 60, 100, // note on
        0x8F, 0x00, 0x80, 60, 0, // 1920 ticks later, note off
        0x00, 0xFF, 0x2F, 0x00, // end of track
    ];
    let mut bytes = b"MThd".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0]);
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(track);
    bytes
}

#[test]
fn a_match_start_requests_the_song_into_an_empty_slot_and_takes_its_own_back_to_the_top() {
    let mut fixture = Fixture::new();
    let own = fixture.label_of(SONG);
    assert_eq!(fixture.harness.audio().song_label(), Some(own.as_str()), "start requested it");

    fixture.harness.audio_mut().stop_song();
    assert_eq!(fixture.harness.audio().song_label(), None);
    fixture.restart();
    assert_eq!(fixture.harness.audio().song_label(), Some(own.as_str()), "an empty slot is requested into");

    fixture.wait(3, Duration::from_millis(400));
    assert!(fixture.harness.audio().song_position().expect("playing").beat >= 1);
    fixture.restart();
    fixture.settle();
    let position = fixture.harness.audio().song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (0, 0), "its own song goes back to the top");
    assert_eq!(fixture.harness.game().music.failures(), 0);
}

#[test]
fn a_match_start_leaves_another_song_alone_unless_it_played_to_its_end() {
    let mut fixture = Fixture::new();
    let other = fixture.label_of("songs/looping.mid");
    fixture.harness.audio_mut().play_song(&other).expect("the other song loads");
    fixture.restart();
    fixture.settle();
    assert_eq!(fixture.harness.audio().song_label(), Some(other.as_str()), "another playing song is left alone");

    let ended = fixture.label_of("songs/ended.mid");
    fixture.harness.audio_mut().play_song(&ended).expect("the one-bar song loads");
    fixture.wait(4, Duration::from_millis(700));
    assert!(fixture.harness.audio().song_ended(), "a one-bar song at 120 BPM ends in two seconds");
    fixture.restart();
    assert_eq!(
        fixture.harness.audio().song_label(),
        Some(fixture.label_of(SONG).as_str()),
        "a song that played to its end is replaced"
    );
}

#[test]
fn the_pause_menus_opening_frame_jams_nothing_and_each_move_after_jams_one_unit() {
    let mut fixture = Fixture::new();
    fixture.harness.game_mut().paused = true;
    fixture.harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
    fixture.harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::KeyW)]);
    assert_eq!(fixture.harness.game().jams, 0, "the frame it opens jams nothing, a move on it included");

    let accepted = fixture.accepted_triggers();
    fixture.key(KeyCode::KeyS);
    assert_eq!(fixture.harness.game().jams, 1, "one move, one unit");
    fixture.settle();
    assert_eq!(fixture.accepted_triggers(), accepted + 1, "on the menu's source, accepted");
    fixture.key(KeyCode::Enter);
    assert_eq!(fixture.harness.game().jams, 1, "confirm jams nothing");

    fixture.harness.game_mut().paused = false;
    fixture.harness.step(FRAME, &[]);
    fixture.harness.game_mut().paused = true;
    fixture.harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
    fixture.harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::KeyW)]);
    assert_eq!(fixture.harness.game().jams, 1, "a reopen on the frame after a resume jams nothing");
    assert_eq!(fixture.harness.game().music.jam_units(), fixture.harness.game().jams);
}

#[test]
fn a_restart_clears_the_pause_latch_so_the_next_opening_frame_jams_nothing() {
    let mut fixture = Fixture::new();
    fixture.harness.game_mut().paused = true;
    fixture.harness.step(FRAME, &[]);
    fixture.restart();
    fixture.harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyS)]);
    assert_eq!(fixture.harness.game().jams, 0, "the frame after a Restart opens the menu afresh");
}

#[test]
fn a_bar_line_pulses_as_a_bar_and_an_update_pulses_at_most_once() {
    let mut fixture = Fixture::new();
    fixture.harness.game_mut().running = true;
    fixture.wait(40, Duration::from_millis(60));
    let pulses = fixture.harness.game().pulses.clone();
    assert!(pulses.iter().any(|frame| frame.bar_crossed), "the run crossed a bar line");
    assert!(pulses.iter().all(|frame| frame.second.is_none()), "a second request in one update answers nothing");
    for frame in pulses.iter().filter(|frame| frame.bar_crossed) {
        assert_eq!(frame.first, Some(BeatPulse::Bar), "a bar line is a bar, never a beat");
    }
    assert!(pulses.iter().any(|frame| frame.first == Some(BeatPulse::Beat)), "the other beats pulse as beats");
    let handed = pulses.iter().filter(|frame| frame.first.is_some()).count() as u64;
    assert_eq!(fixture.harness.game().music.pulsed(), handed);
}

#[test]
fn a_song_that_will_not_load_is_counted_failed_and_every_match_start_asks_again() {
    let mut fixture = Fixture::with_song(false);
    assert!(fixture.harness.game().music.song_failed());
    assert_eq!(fixture.harness.game().music.failures(), 1);
    assert_eq!(fixture.harness.audio().song_label(), None);
    fixture.restart();
    fixture.restart();
    assert_eq!(fixture.harness.game().music.failures(), 3, "an empty slot is requested into every time");
    assert!(fixture.harness.game().music.song_failed());
}
