//! Criterion 2: one phrase through two instruments, alternately and together; program
//! changes and same-pitch note-offs that cannot reach the other instrument or the other
//! source; a finite capacity that, when exceeded, never corrupts a sounding note.

use super::{amplitude_at, fixture, hertz_of, render_slot, root_mean_square, sine_bank, sine_slot, SAMPLE_RATE};
use crate::instrument_slot::NoteOutcome;
use crate::mixer::{Mixer, SlotTarget, SpikeCommand, BLOCK_FRAMES};
use crate::phrase::{opening_bars, CommandQueue};
use crate::score::parse_score;

const QUARTER_SECOND: usize = SAMPLE_RATE as usize / 4;

#[test]
fn test_program_change_and_note_off_on_one_instrument_leave_the_other_bit_identical() {
    let bank = sine_bank();
    let mut slot_a = sine_slot(&bank, 24, 128);
    let mut slot_b = sine_slot(&bank, 24, 128);
    let mut twin_of_b = sine_slot(&bank, 24, 128);
    slot_b.note_on(1, 69, 100);
    twin_of_b.note_on(1, 69, 100);
    slot_a.note_on(1, 69, 100);
    render_slot(&mut slot_a, QUARTER_SECOND);

    slot_a.program_change(0, 5);
    slot_a.note_off(1, 69);
    let a_after = render_slot(&mut slot_a, QUARTER_SECOND);
    let b_after = render_slot(&mut slot_b, QUARTER_SECOND * 2);
    let twin_after = render_slot(&mut twin_of_b, QUARTER_SECOND * 2);

    assert!(root_mean_square(&a_after[QUARTER_SECOND / 2..]) < 1e-4, "A's note was released");
    assert_eq!(b_after, twin_after, "B never saw A's program change or same-pitch note-off");
    assert!(root_mean_square(&b_after) > 0.02, "B is still sounding");
}

#[test]
fn test_two_sources_on_one_instrument_at_one_pitch_are_two_lanes() {
    let bank = sine_bank();
    let mut slot = sine_slot(&bank, 24, 128);
    let mut reference = sine_slot(&bank, 24, 128);
    slot.note_on(1, 69, 100);
    slot.note_on(2, 69, 100);
    reference.note_on(2, 69, 100);
    assert_ne!(slot.lane_of(1), slot.lane_of(2), "each source leases its own lane");

    slot.note_off(1, 69);
    assert_eq!(slot.lane_of(1), None, "a silent source gives its lane back");
    assert!(slot.lane_of(2).is_some());
    let mixed = render_slot(&mut slot, SAMPLE_RATE as usize);
    let alone = render_slot(&mut reference, SAMPLE_RATE as usize);
    let after_the_tail = SAMPLE_RATE as usize / 2..;
    for (index, (mixed_sample, alone_sample)) in mixed[after_the_tail.clone()].iter().zip(&alone[after_the_tail]).enumerate() {
        assert!((mixed_sample - alone_sample).abs() < 1e-5, "frame {index}: source 2 was cut by source 1's note-off");
    }
}

#[test]
fn test_a_same_pitch_retrigger_on_one_lane_keeps_the_second_attack() {
    let bank = sine_bank();
    let mut slot = sine_slot(&bank, 24, 128);
    assert_eq!(slot.note_on(1, 69, 100), NoteOutcome::Started);
    render_slot(&mut slot, QUARTER_SECOND);
    assert_eq!(slot.note_on(1, 69, 100), NoteOutcome::StartedAfterRetrigger);
    assert_eq!(slot.sounding_count(), 1);
    let after = render_slot(&mut slot, SAMPLE_RATE as usize);
    assert!(root_mean_square(&after[SAMPLE_RATE as usize / 2..]) > 0.02, "the second strike sustains past the first's tail");
}

#[test]
fn test_at_the_sounding_cap_the_oldest_note_is_released_and_the_rest_keep_sounding() {
    let bank = sine_bank();
    let mut slot = sine_slot(&bank, 3, 128);
    for key in [57, 64, 69] {
        assert_eq!(slot.note_on(1, key, 100), NoteOutcome::Started);
    }
    assert_eq!(slot.note_on(2, 76, 100), NoteOutcome::StartedAfterSteal);
    assert_eq!(slot.sounding_count(), 3);
    render_slot(&mut slot, QUARTER_SECOND);
    let held = render_slot(&mut slot, QUARTER_SECOND);
    assert!(amplitude_at(&held, hertz_of(57)) < 0.002, "the oldest note was released");
    for key in [64, 69, 76] {
        assert!(amplitude_at(&held, hertz_of(key)) > 0.05, "key {key} still sounds");
    }
    assert_eq!(slot.counters().steals, 1);
}

#[test]
fn test_a_full_voice_pool_steals_release_tails_never_sounding_notes() {
    let bank = sine_bank();
    let held_keys = [45, 52, 57];
    let mut renders = Vec::new();
    for maximum_polyphony in [8, 256] {
        let mut slot = sine_slot(&bank, 4, maximum_polyphony);
        for key in held_keys {
            slot.note_on(1, key, 100);
        }
        let mut output = Vec::new();
        for block in 0..400 {
            let key = 84 + (block % 12);
            slot.note_on(2, key, 100);
            output.extend(render_slot(&mut slot, BLOCK_FRAMES));
            slot.note_off(2, key);
            output.extend(render_slot(&mut slot, BLOCK_FRAMES));
        }
        assert_eq!(slot.sounding_count(), held_keys.len(), "the storm never pushed a held note out");
        renders.push(output);
    }
    let difference: Vec<f32> = renders[0].iter().zip(&renders[1]).map(|(small, large)| small - large).collect();
    assert!(root_mean_square(&difference) > 1e-4, "the small pool filled: rustysynth's own stealer ran");
    for key in held_keys {
        let lost = amplitude_at(&difference, hertz_of(key));
        assert!(lost < 1e-3, "key {key}: the stealer took a sounding voice (lost amplitude {lost})");
    }
}

#[test]
fn test_one_phrase_plays_alternately_and_together_on_two_instruments() {
    let phrase = opening_bars(&parse_score(&fixture("bach-invention-02.mid")).expect("the Bach fixture parses"), 2);
    assert!(phrase.len() >= 16, "two bars of the invention: {} notes", phrase.len());
    let phrase_seconds = phrase.iter().map(|note| note.end_seconds).fold(0.0, f64::max);
    let bank = sine_bank();
    let mut mixer = Mixer::new(None, vec![sine_slot(&bank, 24, 128), sine_slot(&bank, 24, 128)], None);
    let mut queue = CommandQueue::default();
    queue.push_phrase(&phrase, SlotTarget::Instrument(0), 1, 0.0);
    queue.push_phrase(&phrase, SlotTarget::Instrument(1), 1, phrase_seconds);
    queue.push_phrase(&phrase, SlotTarget::Instrument(0), 2, phrase_seconds * 2.0);
    queue.push_phrase(&phrase, SlotTarget::Instrument(1), 2, phrase_seconds * 2.0);

    let mut left = [0.0; BLOCK_FRAMES];
    let mut right = [0.0; BLOCK_FRAMES];
    let mut commands = Vec::new();
    let mut peak_sounding = [0; 3];
    let total_blocks = (phrase_seconds * 3.0 * f64::from(SAMPLE_RATE)) as usize / BLOCK_FRAMES;
    for block in 0..total_blocks {
        let now = (block * BLOCK_FRAMES) as f64 / f64::from(SAMPLE_RATE);
        queue.drain_due(now, &mut commands);
        for command in commands.drain(..) {
            mixer.apply(command);
        }
        let report = mixer.render_block(&mut left, &mut right);
        let third = ((now / phrase_seconds) as usize).min(2);
        peak_sounding[third] = peak_sounding[third].max(report.sounding_notes);
        assert_eq!(report.non_finite_samples, 0);
    }
    let slots_used = |mixer: &Mixer| mixer.slots.iter().map(|slot| slot.counters().notes_started).collect::<Vec<_>>();
    assert_eq!(slots_used(&mixer), vec![phrase.len() as u64 * 2, phrase.len() as u64 * 2]);
    assert!(peak_sounding[2] > peak_sounding[0], "together, both instruments sound at once");
    mixer.apply(SpikeCommand::ReleaseAll);
}
