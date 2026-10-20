//! Criterion 1: a SoundFont built from bytes in memory, a synthesizer that may cross to
//! the audio thread, and a rodio source fed note events from outside, one block at a time.

use rodio::Source;
use rustysynth::Synthesizer;

use super::{root_mean_square, sine_bank, sine_slot, SAMPLE_RATE};
use crate::mixer::{Mixer, SlotTarget, SpikeCommand, BLOCK_FRAMES};
use crate::spike_source::{spike_channel, SpikeSource, COMMAND_CAPACITY};

fn assert_send<T: Send>() {}

#[test]
fn test_the_synthesizer_and_the_source_can_move_to_the_audio_thread() {
    assert_send::<Synthesizer>();
    assert_send::<Mixer>();
    assert_send::<SpikeSource>();
}

#[test]
fn test_a_sound_font_builds_from_bytes_already_in_memory() {
    let bank = sine_bank();
    assert_eq!(bank.get_presets().len(), 1);
    assert_eq!(bank.get_presets()[0].get_name(), "Sine");
    assert_eq!(bank.get_sample_headers()[0].get_original_pitch(), 69);
}

fn pull(source: &mut SpikeSource, samples: usize) -> Vec<f32> {
    (0..samples).map(|_| source.next().expect("the source is endless until stopped")).collect()
}

#[test]
fn test_note_events_from_the_game_thread_land_on_the_next_block_boundary() {
    let bank = sine_bank();
    let mixer = Mixer::new(None, vec![sine_slot(&bank, 24, 128)], None);
    let (mut source, sender) = spike_channel(mixer, SAMPLE_RATE);
    assert_eq!((source.channels(), source.sample_rate()), (2, SAMPLE_RATE));

    assert!(pull(&mut source, BLOCK_FRAMES * 2).iter().all(|sample| *sample == 0.0), "silence before any note");
    let half_block = pull(&mut source, BLOCK_FRAMES);
    assert!(sender.send(SpikeCommand::NoteOn { target: SlotTarget::Instrument(0), source: 1, key: 69, velocity: 127 }));
    let rest_of_block = pull(&mut source, BLOCK_FRAMES);
    assert!(
        half_block.iter().chain(&rest_of_block).all(|sample| *sample == 0.0),
        "a command sent mid-block waits for the block to end"
    );
    let next_blocks = pull(&mut source, BLOCK_FRAMES * 2 * 8);
    assert!(root_mean_square(&next_blocks) > 0.01, "the note sounds from the next block on");
}

#[test]
fn test_a_full_channel_refuses_without_blocking_and_stop_ends_the_source() {
    let bank = sine_bank();
    let mixer = Mixer::new(None, vec![sine_slot(&bank, 24, 128)], None);
    let (mut source, sender) = spike_channel(mixer, SAMPLE_RATE);
    let command = SpikeCommand::NoteOff { target: SlotTarget::Instrument(0), source: 1, key: 69 };
    for _ in 0..COMMAND_CAPACITY {
        assert!(sender.send(command));
    }
    assert!(!sender.send(command), "the bounded channel is full");
    assert_eq!(sender.rejected_commands(), 1);

    pull(&mut source, BLOCK_FRAMES * 2);
    assert!(sender.send(SpikeCommand::Stop));
    assert_eq!(source.next(), None, "the block after Stop is never rendered");
}
