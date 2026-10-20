use std::collections::BTreeSet;

use super::*;

/// A source by number, every one a first activation.
fn source(id: u64) -> SourceId {
    SourceId {
        id,
        generation: 0,
        epoch: 0,
    }
}

const LANES: usize = 15;
const CAP: usize = 24;

/// What the synthesizer behind a ledger would be holding: every (lane, key) sounding.
#[derive(Default)]
struct Synthesizer {
    sounding: BTreeSet<(usize, u8)>,
    actions: Vec<LedgerAction>,
}

impl Synthesizer {
    fn act(&mut self, action: LedgerAction) {
        match action {
            LedgerAction::NoteOn { lane, key, .. } => {
                self.sounding.insert((lane, key));
            }
            LedgerAction::NoteOff { lane, key } => {
                assert!(
                    self.sounding.remove(&(lane, key)),
                    "a note-off reaches only a note the ledger sounded: lane {lane} key {key}"
                );
            }
        }
        self.actions.push(action);
    }
}

#[test]
fn a_sixteenth_source_takes_the_oldest_sources_lane_and_is_never_refused() {
    let mut ledger = VoiceLedger::new(LANES, CAP);
    let mut synthesizer = Synthesizer::default();
    for number in 0..LANES as u64 {
        ledger.note_on(source(number), 60, 100, |action| synthesizer.act(action));
    }
    let oldest_lane = ledger
        .lane_of(source(0))
        .expect("the first source holds a lane");
    synthesizer.actions.clear();

    let start = ledger.note_on(source(99), 72, 100, |action| synthesizer.act(action));

    assert!(start.lane_stolen);
    assert_eq!(
        synthesizer.actions,
        vec![
            LedgerAction::NoteOff {
                lane: oldest_lane,
                key: 60
            },
            LedgerAction::NoteOn {
                lane: oldest_lane,
                key: 72,
                velocity: 100
            },
        ],
        "the oldest source's notes are released, then the newcomer sounds on its lane"
    );
    assert_eq!(ledger.lane_of(source(0)), None);
    assert!(
        ledger.lane_of(source(1)).is_some(),
        "only the oldest is taken"
    );
    assert_eq!(ledger.counters().lane_steals, 1);
}

#[test]
fn past_the_sounding_cap_the_oldest_note_is_released_and_the_count_never_exceeds_it() {
    let mut ledger = VoiceLedger::new(LANES, CAP);
    let mut synthesizer = Synthesizer::default();
    let mut seed = 0x2545_f491_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut tickets = Vec::new();
    for _ in 0..20_000 {
        let source = source(next() % 40);
        let key = 36 + (next() % 48) as u8;
        if next() % 3 == 0 && !tickets.is_empty() {
            let ticket = tickets.swap_remove(next() as usize % tickets.len());
            ledger.note_off(ticket, |action| synthesizer.act(action));
        } else {
            let start = ledger.note_on(source, key, 90, |action| synthesizer.act(action));
            tickets.push(start.ticket);
        }
        assert!(ledger.sounding_count() <= CAP);
        assert_eq!(
            synthesizer.sounding.len(),
            ledger.sounding_count(),
            "the ledger knows the sounding set exactly"
        );
    }
    assert!(ledger.counters().steals > 0 && ledger.counters().lane_steals > 0);
}

#[test]
fn a_same_key_strike_on_a_lane_releases_the_old_note_before_the_new_attack() {
    let mut ledger = VoiceLedger::new(LANES, CAP);
    let mut synthesizer = Synthesizer::default();
    ledger.note_on(source(1), 60, 100, |action| synthesizer.act(action));
    synthesizer.actions.clear();

    let start = ledger.note_on(source(1), 60, 80, |action| synthesizer.act(action));

    assert!(start.retriggered);
    let lane = ledger.lane_of(source(1)).expect("leased");
    assert_eq!(
        synthesizer.actions,
        vec![
            LedgerAction::NoteOff { lane, key: 60 },
            LedgerAction::NoteOn {
                lane,
                key: 60,
                velocity: 80
            },
        ]
    );
    assert_eq!(ledger.sounding_count(), 1);
}

#[test]
fn a_note_off_for_a_stolen_note_sends_nothing_to_the_lane_another_source_now_holds() {
    let mut ledger = VoiceLedger::new(1, CAP);
    let mut synthesizer = Synthesizer::default();
    let stolen = ledger.note_on(source(1), 60, 100, |action| synthesizer.act(action));
    ledger.note_on(source(2), 60, 100, |action| synthesizer.act(action));
    synthesizer.actions.clear();

    ledger.note_off(stolen.ticket, |action| synthesizer.act(action));

    assert!(
        synthesizer.actions.is_empty(),
        "the second source's note keeps sounding"
    );
    assert_eq!(synthesizer.sounding.len(), 1);
}

#[test]
fn a_note_off_ends_only_its_own_note_when_the_source_has_struck_the_key_again() {
    let mut ledger = VoiceLedger::new(LANES, CAP);
    let mut synthesizer = Synthesizer::default();
    let first = ledger.note_on(source(1), 60, 100, |action| synthesizer.act(action));
    let second = ledger.note_on(source(1), 60, 100, |action| synthesizer.act(action));
    synthesizer.actions.clear();

    ledger.note_off(first.ticket, |action| synthesizer.act(action));

    assert!(
        synthesizer.actions.is_empty(),
        "the first note was released by the retrigger; its note-off ends nothing"
    );
    assert!(ledger.is_sounding(second.ticket));
    ledger.note_off(second.ticket, |action| synthesizer.act(action));
    assert_eq!(ledger.sounding_count(), 0);
    assert_eq!(ledger.lane_of(source(1)), None, "the lane is free again");
}
