//! The demo through `GameHarness`, its song on the virtual clock: the two kinds walk one
//! part in turn, the view gates every attack and ends the source of an enemy leaving it, a
//! despawn ends its source in that frame's tail, the stress shapes run, a respawn joins
//! its kind's phase, a hitch lands both kinds, and the two pauses are told apart.

use std::time::Duration;

use engine_core::prelude::*;
use engine_core::test_support::{GameHarness, InputEvent};
use music_midi::{PartMode, PositionSnapshot, Quantize};

use super::enemies::Kind;
use super::{ConductorDemo, Settings, Stress, ASSET_BASE};

const FRAME: f32 = 1.0 / 60.0;

fn harness(settings: Settings) -> GameHarness<ConductorDemo> {
    let config = GameConfig::new("Conductor Demo").with_size(800, 600).with_asset_base_path(ASSET_BASE);
    let mut harness = GameHarness::new(ConductorDemo::new(settings), config);
    harness.step(FRAME, &[]);
    harness
}

/// `frames` frames of game time with a little wall time between them, so the virtual
/// clock renders and the conductor drains.
fn run(harness: &mut GameHarness<ConductorDemo>, frames: usize) {
    for _ in 0..frames {
        std::thread::sleep(Duration::from_millis(2));
        harness.step(FRAME, &[]);
    }
}

fn conductor(harness: &GameHarness<ConductorDemo>) -> PositionSnapshot {
    harness.audio().song_snapshot().expect("the demo's song plays").conductor
}

fn entity_of(harness: &GameHarness<ConductorDemo>, kind: Kind) -> EntityId {
    harness.game().enemies().iter().find(|enemy| enemy.kind == kind).expect("the kind is in the scene").entity
}

fn press(harness: &mut GameHarness<ConductorDemo>, code: KeyCode) {
    harness.step(FRAME, &[InputEvent::KeyPressed(code)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(code)]);
}

#[test]
fn both_kinds_play_the_one_part_in_turn() {
    let text = std::fs::read_to_string(format!("{ASSET_BASE}/music/ai_demo.mid.ron")).expect("the sidecar");
    let file = music_midi::parse_song_file("ai_demo.mid.ron", &text).expect("parses");
    let parts: Vec<&str> = ["hum", "ping"]
        .iter()
        .map(|action| file.bindings.iter().find(|binding| binding.action == *action).expect(action).part.as_str())
        .collect();
    assert_eq!(parts, ["melody", "melody"], "one shared part, two instruments");
    let melody = file.parts.iter().find(|part| part.name == "melody").expect("the part");
    assert_eq!((melody.mode, melody.quantize), (PartMode::Loop, Quantize::Off));

    let mut harness = harness(Settings::default());
    let accepted = conductor(&harness).accepted_triggers;
    run(&mut harness, 40);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1, "the pinger at half a period");
    assert!(harness.sound_sources().source_of(entity_of(&harness, Kind::Pinger)).is_some());
    run(&mut harness, 30);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 2, "then the hummer at one");
    assert!(harness.sound_sources().source_of(entity_of(&harness, Kind::Hummer)).is_some());
}

#[test]
fn an_enemy_out_of_view_mints_nothing_its_source_ends_as_it_leaves_and_its_next_attack_back_in_activates_it() {
    let mut harness = harness(Settings::default());
    let pinger = entity_of(&harness, Kind::Pinger);
    run(&mut harness, 40);
    assert!(harness.sound_sources().source_of(pinger).is_some(), "it attacked at half a period");

    let index = harness.game().enemies().iter().position(|enemy| enemy.entity == pinger).expect("listed");
    harness.game_mut().enemies_mut()[index].home = Vec2::new(1000.0, 0.0);
    run(&mut harness, 1);
    assert_eq!(harness.sound_sources().source_of(pinger), None, "ended on the frame it left");
    let attempted = harness.game().attempted();
    run(&mut harness, 130);
    assert_eq!(harness.game().attempted(), attempted + 2, "only the hummer's two slots were minted");

    harness.game_mut().enemies_mut()[index].home = Vec2::new(150.0, -80.0);
    run(&mut harness, 60);
    assert!(harness.sound_sources().source_of(pinger).is_some(), "its first attack back in activates it anew");
}

#[test]
fn a_despawn_mid_note_ends_its_source_in_that_frames_tail() {
    let mut harness = harness(Settings::default());
    run(&mut harness, 40);
    let pinger = entity_of(&harness, Kind::Pinger);
    assert!(harness.sound_sources().source_of(pinger).is_some());
    let index = harness.game().enemies().iter().position(|enemy| enemy.entity == pinger).expect("listed");
    harness.game_mut().enemies_mut()[index].home = Vec2::ZERO;
    run(&mut harness, 1);
    let removed = harness.context(|game, ctx| game.despawn_nearest(ctx.world));
    assert_eq!(removed, Some(pinger), "the nearest to the camera");
    run(&mut harness, 1);
    assert_eq!(harness.sound_sources().source_of(pinger), None);
}

#[test]
fn every_stress_shape_runs_and_a_despawn_still_ends_its_source() {
    for stress in [Stress::Spread, Stress::Burst, Stress::Saturate] {
        let mut harness = harness(Settings { stress: Some(stress), quit_after_seconds: None });
        assert_eq!(harness.game().enemies().len(), if stress == Stress::Saturate { 40 } else { 48 });
        run(&mut harness, 130);
        assert!(harness.game().attempted() >= 40, "{stress:?}: every enemy in view attacked");
        let removed = harness.context(|game, ctx| game.despawn_nearest(ctx.world)).expect("one to despawn");
        run(&mut harness, 1);
        assert_eq!(harness.sound_sources().source_of(removed), None, "{stress:?}");
    }
}

#[test]
fn a_respawned_enemy_attacks_first_on_its_kinds_phase() {
    let mut harness = harness(Settings::default());
    run(&mut harness, 45);
    let pinger = entity_of(&harness, Kind::Pinger);
    harness.context(|game, ctx| {
        let index = game.enemies().iter().position(|enemy| enemy.entity == pinger).expect("listed");
        let enemy = game.enemies_mut().remove(index);
        ctx.world.remove_entity(&enemy.entity).ok();
    });
    run(&mut harness, 3);
    press(&mut harness, KeyCode::KeyR);
    let respawned = entity_of(&harness, Kind::Pinger);
    assert_ne!(respawned, pinger);
    let mut first_attack = None;
    for _ in 0..90 {
        run(&mut harness, 1);
        if first_attack.is_none() && harness.sound_sources().source_of(respawned).is_some() {
            first_attack = Some(harness.game().clock());
        }
    }
    let clock = first_attack.expect("it attacked");
    assert!((1.5..1.55).contains(&clock), "on the pinger's 1.5, not the hummer's 1.0 or 2.0: {clock}");
}

#[test]
fn a_hitch_frame_lands_both_kinds_attacks_at_once() {
    let mut harness = harness(Settings::default());
    for _ in 0..53 {
        harness.step(FRAME, &[]);
    }
    let before = harness.game().clock();
    assert!((0.85..0.95).contains(&before), "{before}");
    let attempted = harness.game().attempted();
    harness.step(0.7, &[]);
    assert_eq!(harness.game().attempted(), attempted + 2, "one frame across 1.0 and 1.5");
}

#[test]
fn the_games_pause_mints_nothing_while_the_song_plays_and_the_songs_pause_rejects_a_trigger() {
    let mut harness = harness(Settings::default());
    run(&mut harness, 5);
    press(&mut harness, KeyCode::Space);
    let (attempted, blocks) = (harness.game().attempted(), conductor(&harness).blocks_rendered);
    run(&mut harness, 120);
    assert_eq!(harness.game().attempted(), attempted, "the game's clock held");
    assert!(conductor(&harness).blocks_rendered > blocks, "the song played on");
    press(&mut harness, KeyCode::Space);

    press(&mut harness, KeyCode::KeyP);
    run(&mut harness, 5);
    let rejected = conductor(&harness).rejected_triggers;
    let attempted = harness.game().attempted();
    run(&mut harness, 40);
    assert!(harness.game().attempted() > attempted, "the game ran on and minted");
    assert!(conductor(&harness).rejected_triggers > rejected, "a paused song rejects, its cursor kept");
}

#[test]
fn the_command_line_names_a_stress_shape_and_a_duration_or_is_refused() {
    let parse = |line: &str| Settings::parse(line.split_whitespace().map(str::to_string));
    assert_eq!(parse(""), Ok(Settings::default()));
    assert_eq!(
        parse("--stress burst --seconds 30"),
        Ok(Settings { stress: Some(Stress::Burst), quit_after_seconds: Some(30.0) })
    );
    assert!(parse("--stress loud").is_err());
    assert!(parse("--seconds").is_err());
    assert!(parse("--verbose").is_err());
}

#[test]
fn a_respawned_hummer_patrols_as_the_scenes_did() {
    let mut harness = harness(Settings::default());
    run(&mut harness, 5);
    let hummer = entity_of(&harness, Kind::Hummer);
    harness.context(|game, ctx| {
        let index = game.enemies().iter().position(|enemy| enemy.entity == hummer).expect("listed");
        let enemy = game.enemies_mut().remove(index);
        ctx.world.remove_entity(&enemy.entity).ok();
    });
    press(&mut harness, KeyCode::KeyR);
    let respawned = entity_of(&harness, Kind::Hummer);
    assert_ne!(respawned, hummer);
    let hummer_at = |harness: &GameHarness<ConductorDemo>| {
        let game = harness.game();
        let enemy = game.enemies().iter().find(|enemy| enemy.entity == respawned).expect("respawned");
        assert!(enemy.patrols, "the respawned hummer patrols");
        enemy.position(game.clock())
    };
    let before = hummer_at(&harness);
    run(&mut harness, 30);
    assert_ne!(hummer_at(&harness), before, "and moves along its sweep");
}
