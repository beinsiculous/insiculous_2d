use std::collections::BTreeSet;
use std::sync::Arc;

use super::*;
use crate::song_data::{NewNote, SongData};
use crate::test_support::*;

/// A pure-sequencer rate: low enough that a one-frame grain walks a whole overture fast.
const SAMPLE_RATE: u32 = 1000;
/// A grain of one frame sends every event on its exact sample.
const EXACT_GRAIN: usize = 1;
const BLOCK_GRAIN: usize = 64;

/// Everything a sequencer did, stamped with the frame it was rendering at, and the
/// channel state its messages and reconstructions leave behind.
#[derive(Default)]
struct Recorder {
    rendered_frames: u64,
    sends: Vec<(u64, u8, ChannelMessage)>,
    reconstructions: Vec<u64>,
    releases: Vec<u64>,
    renders: Vec<usize>,
    state: ChannelStateFold,
}

impl SequencerSink for Recorder {
    fn send(&mut self, channel: u8, message: ChannelMessage) {
        self.sends.push((self.rendered_frames, channel, message));
        self.state.apply(channel, message);
    }

    fn reconstruct(&mut self, state: &ChannelStateFold) {
        self.reconstructions.push(self.rendered_frames);
        self.state = state.clone();
    }

    fn release_all(&mut self) {
        self.releases.push(self.rendered_frames);
    }

    fn render(&mut self, frames: usize) {
        self.rendered_frames += frames as u64;
        self.renders.push(frames);
    }
}

impl Recorder {
    fn note_offs(&self, channel: u8, key: u8) -> Vec<u64> {
        self.sends
            .iter()
            .filter(|(_, sent_channel, message)| {
                *sent_channel == channel && *message == ChannelMessage::NoteOff { key }
            })
            .map(|(frame, ..)| *frame)
            .collect()
    }

    fn note_ons_from(&self, frame: u64) -> Vec<(u64, u8, ChannelMessage)> {
        self.sends
            .iter()
            .filter(|(sent_frame, _, message)| {
                *sent_frame >= frame && matches!(message, ChannelMessage::NoteOn { .. })
            })
            .copied()
            .collect()
    }
}

fn background(song: &SongData) -> Arc<BackgroundScore> {
    Arc::new(BackgroundScore::new(song, &BTreeSet::new()))
}

fn sequencer(song: &SongData, grain_frames: usize, looping: bool) -> Sequencer {
    Sequencer::new(background(song), SAMPLE_RATE, grain_frames, looping)
}

/// A note-on or note-off as `(tick, channel, key)`.
type TimedKey = (u64, u8, u8);

/// Every note-on and every note-off, each sorted: what a listener hears.
fn heard(score: &BackgroundScore) -> (Vec<TimedKey>, Vec<TimedKey>) {
    let mut note_ons = Vec::new();
    let mut note_offs = Vec::new();
    for event in score.events() {
        match event.message {
            ChannelMessage::NoteOn { key, .. } => note_ons.push((event.tick, event.channel, key)),
            ChannelMessage::NoteOff { key } => note_offs.push((event.tick, event.channel, key)),
            _ => {}
        }
    }
    note_ons.sort_unstable();
    note_offs.sort_unstable();
    (note_ons, note_offs)
}

/// Two quarter notes at 240 BPM and 384 ticks a quarter: a downbeat on channel 0 under a
/// bass note held on channel 1 to the end, then on the second beat a program change, a
/// volume change and a pitch-bend range of an octave on channel 0 before its next note.
fn song_with_state_changes() -> SongData {
    let bytes = smf(&[vec![
        (0, tempo(250_000)),
        (0, program_change(0, 40)),
        (0, controller(0, 7, 90)),
        (0, note_on(0, 60, 100)),
        (0, note_on(1, 48, 100)),
        (384, note_off(0, 60)),
        (0, program_change(0, 41)),
        (0, controller(0, 7, 70)),
        (0, controller(0, 101, 0)),
        (0, controller(0, 100, 0)),
        (0, controller(0, 6, 12)),
        (0, note_on(0, 64, 100)),
        (384, note_off(0, 64)),
        (0, note_off(1, 48)),
    ]]);
    read(&bytes).0
}

#[test]
fn a_pedalled_repeat_on_one_key_sustains_until_its_last_instance_ends() {
    let bytes = smf(&[vec![
        (0, note_on(3, 60, 100)),
        (96, note_on(3, 60, 50)),
        (96, note_off(3, 60)),
        (96, note_off(3, 60)),
        (0, note_on(3, 62, 90)),
        (96, note_off(3, 62)),
    ]]);
    let (song, report) = read(&bytes);
    assert_eq!(
        report.same_key_overlaps.len(),
        1,
        "the repeat starts while the key sounds"
    );
    let mut sequencer = sequencer(&song, EXACT_GRAIN, false);
    let mut recorder = Recorder::default();
    let sample = |tick| song.tempo_map().sample_at_tick(tick, SAMPLE_RATE);

    sequencer.seek_to_tick(0, &mut recorder);
    sequencer.advance(sequencer.end_sample() as usize + 1, &mut recorder);

    let note_ons = recorder.note_ons_from(0);
    assert_eq!(
        note_ons
            .iter()
            .map(|(frame, ..)| *frame)
            .collect::<Vec<_>>(),
        vec![sample(0), sample(96), sample(288)],
        "the repeat strikes again"
    );
    assert_eq!(
        recorder.note_offs(3, 60),
        vec![sample(288)],
        "the first instance's end at tick 192 would release both voices; only the last end is sent"
    );
}

#[test]
fn a_block_crossing_the_loop_end_splits_at_the_seam_and_the_downbeat_starts_it() {
    let song = song_with_state_changes();
    let mut sequencer = sequencer(&song, BLOCK_GRAIN, true);
    let end_sample = sequencer.end_sample();
    assert_ne!(
        end_sample % BLOCK_GRAIN as u64,
        0,
        "the seam falls inside a block"
    );
    let mut recorder = Recorder::default();
    sequencer.seek_to_tick(0, &mut recorder);

    let blocks = end_sample as usize / BLOCK_GRAIN + 2;
    for _ in 0..blocks {
        sequencer.advance(BLOCK_GRAIN, &mut recorder);
    }

    assert_eq!(recorder.reconstructions, vec![0, end_sample]);
    assert_eq!(sequencer.loop_wraps(), 1);
    assert_eq!(
        recorder.rendered_frames - end_sample,
        sequencer.position_sample(),
        "the loop's length never rounds to a block"
    );
    let downbeat = recorder.note_ons_from(end_sample);
    assert_eq!(
        downbeat
            .iter()
            .take(2)
            .map(|(frame, channel, _)| (*frame, *channel))
            .collect::<Vec<_>>(),
        vec![(end_sample, 0), (end_sample, 1)],
        "the loop's first notes are sent on the seam's exact frame"
    );
    assert!(
        recorder
            .note_offs(0, 64)
            .iter()
            .all(|frame| *frame < end_sample)
            && recorder
                .note_offs(1, 48)
                .iter()
                .all(|frame| *frame < end_sample),
        "a note-off on the seam is not sent: the seam releases every note itself"
    );
    assert_eq!(
        recorder.state.program(0),
        Some(40),
        "the wrap restores the loop start's program"
    );
}

#[test]
fn seeking_to_tick_zero_keeps_its_tempo_and_programs() {
    let song = song_with_state_changes();
    let mut sequencer = sequencer(&song, EXACT_GRAIN, false);
    let mut recorder = Recorder::default();

    sequencer.seek_to_tick(0, &mut recorder);
    sequencer.advance(1, &mut recorder);

    assert_eq!(recorder.state.program(0), Some(40));
    assert_eq!(recorder.state.controller(0, 7), Some(90));
    assert_eq!(
        sequencer.end_sample(),
        500,
        "two quarter notes at tick 0's 240 BPM last half a second, not a whole one"
    );
}

#[test]
fn a_seek_rebuilds_the_same_channel_state_as_playing_up_to_the_destination() {
    let state_song = song_with_state_changes();
    let (egmont, _) = read(EGMONT);
    let mut destinations = vec![(&state_song, 384), (&state_song, 400)];
    let egmont_end = background(&egmont).end_tick();
    destinations.extend((1..5).map(|quarter| (&egmont, egmont_end * quarter / 5)));

    for (song, tick) in destinations {
        let mut linear_sequencer = sequencer(song, EXACT_GRAIN, false);
        let mut linear = Recorder::default();
        linear_sequencer.seek_to_tick(0, &mut linear);
        let destination_sample = song.tempo_map().sample_at_tick(tick, SAMPLE_RATE);
        linear_sequencer.advance(destination_sample as usize, &mut linear);
        let linear_notes_from = linear.sends.len();
        linear_sequencer.advance(1, &mut linear);

        let mut seeking_sequencer = sequencer(song, EXACT_GRAIN, false);
        let mut seeking = Recorder::default();
        seeking_sequencer.seek_to_tick(tick, &mut seeking);
        seeking_sequencer.advance(1, &mut seeking);

        assert_eq!(seeking.state, linear.state, "state at tick {tick}");
        let note_ons_at = |recorder: &Recorder, from: usize| {
            recorder.sends[from..]
                .iter()
                .filter(|(_, _, message)| matches!(message, ChannelMessage::NoteOn { .. }))
                .map(|(_, channel, message)| (*channel, *message))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            note_ons_at(&seeking, 0),
            note_ons_at(&linear, linear_notes_from),
            "the destination's own notes sound after a seek, at tick {tick}"
        );
    }
}

#[test]
fn a_note_spanning_the_seek_destination_is_not_resounded_and_its_note_off_is_not_sent() {
    let song = song_with_state_changes();
    let mut sequencer = sequencer(&song, EXACT_GRAIN, false);
    let mut recorder = Recorder::default();

    sequencer.seek_to_tick(400, &mut recorder);
    sequencer.advance(sequencer.end_sample() as usize, &mut recorder);

    assert!(
        recorder.sends.iter().all(|(_, channel, _)| *channel == 0),
        "the held bass note neither restarts nor sends its note-off: {:?}",
        recorder.sends
    );
    assert_eq!(
        recorder.releases.len(),
        1,
        "a song that does not loop releases at its end"
    );
    assert!(sequencer.has_ended());
}

#[test]
fn a_trigger_track_never_plays_in_the_background() {
    let (napolitan, _) = read(NAPOLITAN_DANCE);
    let (type_zero, _) = read(NAPOLITAN_DANCE_TYPE_ZERO);
    let percussion_part = type_zero
        .tracks()
        .iter()
        .position(|track| track.name() == Some("channel 10"))
        .expect("the type-0 file plays percussion");
    let busiest_track = (0..napolitan.tracks().len())
        .max_by_key(|track| napolitan.tracks()[*track].note_count())
        .expect("the file has tracks");

    let whole = BackgroundScore::new(&napolitan, &BTreeSet::new());
    let without_track = BackgroundScore::new(&napolitan, &BTreeSet::from([busiest_track]));
    let without_percussion = BackgroundScore::new(&type_zero, &BTreeSet::from([percussion_part]));

    let notes = |score: &BackgroundScore| heard(score).0.len();
    assert_eq!(
        notes(&whole) - notes(&without_track),
        napolitan.tracks()[busiest_track].note_count()
    );
    assert!(without_percussion
        .events()
        .iter()
        .all(|event| event.channel != 9));
    assert_eq!(
        notes(&without_percussion),
        type_zero.note_count() - type_zero.tracks()[percussion_part].note_count()
    );
}

#[test]
fn an_edited_song_plays_exactly_what_its_export_would_write() {
    let (mut bach, _) = read(BACH_INVENTION);
    let first = bach.tracks()[1].units()[0].notes[0].id;
    let second = bach.tracks()[1].units()[1].notes[0].id;
    bach.set_pitch(&[first], 72).expect("edits");
    bach.remove_notes(&[second]).expect("removes");
    bach.add_note(
        2,
        NewNote {
            tick: 96,
            key: 40,
            velocity: 80,
            duration_ticks: 48,
        },
    )
    .expect("adds");
    let (reread, _) = SongData::from_smf(&bach.write_smf().expect("writes")).expect("reads");

    assert_eq!(heard(&background(&bach)), heard(&background(&reread)));
}

#[test]
fn every_fixture_closes_every_note_it_opens() {
    for (name, bytes) in ALL_FIXTURES {
        let (song, _) = read(bytes);
        let score = background(&song);
        let (note_ons, note_offs) = heard(&score);
        assert_eq!(
            note_ons.len(),
            song.note_count(),
            "{name}: every note plays"
        );
        assert_eq!(note_offs.len(), note_ons.len(), "{name}: every note ends");
        for event in score.events() {
            if let ChannelMessage::NoteOff { key } = event.message {
                let opening = score.events()[event.note_on_index()];
                assert!(
                    matches!(opening.message, ChannelMessage::NoteOn { key: opened, .. } if opened == key)
                        && opening.channel == event.channel
                        && opening.tick <= event.tick,
                    "{name}: a note-off closes a note-on of its key"
                );
            }
        }
    }
}

#[test]
fn replaying_a_fold_rebuilds_its_state_with_registered_parameters_in_order() {
    let pitch_bend_range_of_an_octave = [
        ChannelMessage::Controller {
            controller: 101,
            value: 0,
        },
        ChannelMessage::Controller {
            controller: 100,
            value: 0,
        },
        ChannelMessage::Controller {
            controller: 6,
            value: 12,
        },
        ChannelMessage::Controller {
            controller: 100,
            value: 1,
        },
        ChannelMessage::Controller {
            controller: 6,
            value: 70,
        },
        ChannelMessage::Program { program: 33 },
        ChannelMessage::Controller {
            controller: 7,
            value: 80,
        },
        ChannelMessage::PitchBend { value: 9000 },
    ];
    let mut folded = ChannelStateFold::default();
    for message in pitch_bend_range_of_an_octave {
        folded.apply(4, message);
    }

    let mut replayed_messages = Vec::new();
    folded.replay(4, |message| replayed_messages.push(message));
    let mut rebuilt = ChannelStateFold::default();
    for message in &replayed_messages {
        rebuilt.apply(4, *message);
    }

    assert_eq!(rebuilt, folded);
    let data_entry_at = |value| {
        replayed_messages
            .iter()
            .position(|message| {
                *message
                    == ChannelMessage::Controller {
                        controller: 6,
                        value,
                    }
            })
            .expect("the data entry is replayed")
    };
    let selection_before = |index: usize| {
        replayed_messages[..index]
            .iter()
            .rev()
            .find_map(|message| match message {
                ChannelMessage::Controller {
                    controller: 100,
                    value,
                } => Some(*value),
                _ => None,
            })
    };
    assert_eq!(
        selection_before(data_entry_at(12)),
        Some(0),
        "the pitch-bend range is entered with its parameter selected, not in controller-number order"
    );
    assert_eq!(selection_before(data_entry_at(70)), Some(1));
}
