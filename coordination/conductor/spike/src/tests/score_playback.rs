//! The owned score: the fixtures parse with the shapes their LICENSE records, seeking
//! folds channel state to the destination, and a loop wraps and keeps playing.

use std::sync::Arc;

use super::{fixture, root_mean_square, sine_bank, SAMPLE_RATE};
use crate::score::{channel_states_at, parse_score, BackgroundScore, Score, ScoreEvent, ScoreMessage};

fn note_ons(score: &Score) -> usize {
    score.events.iter().filter(|event| matches!(event.message, ScoreMessage::NoteOn { .. })).count()
}

#[test]
fn test_the_fixtures_parse_with_their_recorded_shapes() {
    let bach = parse_score(&fixture("bach-invention-02.mid")).expect("Bach parses");
    assert_eq!((bach.ticks_per_quarter, bach.last_tick, note_ons(&bach)), (384, 41_472, 634));
    let type1 = parse_score(&fixture("dansenapolitaine.mid")).expect("the type 1 Napolitan Dance parses");
    let type0 = parse_score(&fixture("dansenapolitaine-type0.mid")).expect("the type 0 derivative parses");
    assert_eq!(note_ons(&type0), note_ons(&type1), "the merge lost no note");
    assert_eq!(type0.last_tick, type1.last_tick);
    let tambourine_hits = type0
        .events
        .iter()
        .filter(|event| event.channel == 9 && matches!(event.message, ScoreMessage::NoteOn { key: 54, .. }))
        .count();
    assert_eq!(tambourine_hits, 110, "every tambourine hit is General MIDI key 54 on channel 10");
    let mut note_channels: Vec<u8> = type0
        .events
        .iter()
        .filter(|event| matches!(event.message, ScoreMessage::NoteOn { .. }))
        .map(|event| event.channel)
        .collect();
    note_channels.sort_unstable();
    note_channels.dedup();
    assert_eq!(note_channels, vec![1, 3, 4, 9], "melody on its own channels, percussion on channel 10");
    let bayadere = parse_score(&fixture("bayadere-ombres.mid")).expect("La Bayadère parses");
    assert_eq!(note_ons(&bayadere), 481);
}

#[test]
fn test_seeking_folds_program_controllers_and_bend_but_not_mode_messages() {
    let event = |tick, channel, message| ScoreEvent { tick, channel, message };
    let events = [
        event(0, 0, ScoreMessage::Program(40)),
        event(0, 0, ScoreMessage::Tempo { microseconds_per_quarter: 400_000 }),
        event(10, 0, ScoreMessage::Controller { controller: 7, value: 90 }),
        event(20, 0, ScoreMessage::Controller { controller: 0, value: 1 }),
        event(30, 0, ScoreMessage::Controller { controller: 123, value: 0 }),
        event(40, 3, ScoreMessage::PitchBend(10_000)),
        event(50, 0, ScoreMessage::Program(41)),
    ];
    let (states, tempo) = channel_states_at(&events, 45);
    assert_eq!((states[0].program, states[0].bank, states[0].controllers[7]), (40, 1, Some(90)));
    assert_eq!(states[0].controllers[123], None, "all-notes-off is an action, not a state to restore");
    assert_eq!(states[3].pitch_bend, 10_000);
    assert_eq!(tempo, 400_000);
    assert_eq!(channel_states_at(&events, 60).0[0].program, 41, "the program at the destination wins");
}

#[test]
fn test_the_loop_wraps_at_its_end_and_keeps_playing() {
    let score = Score {
        ticks_per_quarter: 480,
        events: Arc::from(vec![
            ScoreEvent { tick: 0, channel: 0, message: ScoreMessage::NoteOn { key: 69, velocity: 100 } },
            ScoreEvent { tick: 400, channel: 0, message: ScoreMessage::NoteOff { key: 69 } },
        ]),
        last_tick: 400,
    };
    let mut player = BackgroundScore::new(&sine_bank(), score, SAMPLE_RATE, 32).expect("a player over the sine bank");
    player.set_loop(Some((0, 480)));
    let mut left = vec![0.0; 64];
    let mut right = vec![0.0; 64];
    let mut rendered = Vec::new();
    for _ in 0..(SAMPLE_RATE as usize * 2 / 64) {
        player.render(&mut left, &mut right);
        rendered.extend_from_slice(&left);
        assert!(player.position_ticks() < 480.0 + 64.0, "the position stays inside the loop");
    }
    assert!(player.counters().loop_wraps >= 3, "half-second loops wrap in two seconds: {:?}", player.counters());
    let last_half_second = &rendered[rendered.len() - SAMPLE_RATE as usize / 2..];
    assert!(root_mean_square(last_half_second) > 0.01, "the note re-sounds after every wrap");
}
