//! `cargo run --release -- --measure [--seconds N]`: every number `K1_SPIKE.md` reports
//! from the native side, rendered offline (faster than real time) and printed.
//!
//! The pool comparison is how "no sounding note is cut" is checked against a real bank,
//! which exposes no voice count: the same storm is rendered with the proposed voice pool
//! and with one too large to ever fill. Voices take the same pool indices in both until
//! rustysynth's stealer first runs, so identical output proves it never ran; any
//! difference is exactly what its steals changed.

use std::path::Path;

use common::clock::Instant;

use crate::bank::{load_sound_font, preset_profile};
use crate::instrument_slot::{InstrumentSlot, SlotCapacity};
use crate::mixer::{Mixer, SlotTarget, SpikeCommand, BLOCK_FRAMES};
use crate::rig::{load_assets, worst_case_mixer, SpikeAssets, BANK_PATH, CAPACITY, SLOT_PRESETS};
use crate::sample_slot::{SampleSlot, TriggerMode};
use crate::storm::{Storm, StormConfig, SAMPLE_SLOT_ONLY, WORST_CASE};

/// Every preset the casting table names, as (bank, program).
const CAST_PRESETS: [(i32, i32); 12] =
    [(0, 0), (0, 6), (0, 48), (0, 56), (0, 57), (0, 13), (0, 9), (0, 47), (0, 45), (0, 68), (0, 58), (128, 0)];
/// rustysynth refuses a pool over 256 voices, so the comparison is 128 against 256.
const UNBOUNDED_POLYPHONY: usize = 256;

struct StormRun {
    block_microseconds: Vec<f64>,
    output: Vec<f32>,
    peak_sounding_notes: usize,
    peak_sample_voices: usize,
    largest_step: f32,
    steals: u64,
    retriggers: u64,
    lane_rejections: u64,
    reconstructions: u64,
    longest_reconstruction_microseconds: f64,
}

pub fn run(asset_base: &Path, arguments: &[String]) -> Result<(), String> {
    let seconds = arguments
        .windows(2)
        .find(|pair| pair[0] == "--seconds")
        .and_then(|pair| pair[1].parse::<f64>().ok())
        .unwrap_or(120.0);
    let memory_before = crate::memory::resident_mebibytes();
    let assets = load_assets(asset_base)?;
    println!(
        "== loaded memory: resident {memory_before:.1} MiB before the assets, {:.1} MiB after (the VFS bytes are dropped once parsed here)",
        crate::memory::resident_mebibytes()
    );
    print_load_cost(asset_base)?;
    print_profiles(&assets);
    for sample_rate in [44_100, 48_000] {
        let proposed = render_storm(&assets, sample_rate, WORST_CASE, seconds, CAPACITY.maximum_polyphony)?;
        print_storm("worst case", sample_rate, seconds, &proposed);
        if sample_rate == 44_100 {
            let unbounded = render_storm(&assets, sample_rate, WORST_CASE, seconds, UNBOUNDED_POLYPHONY)?;
            print_pool_comparison("worst case", &proposed, &unbounded);
            let sample_only = render_storm(&assets, sample_rate, SAMPLE_SLOT_ONLY, seconds, CAPACITY.maximum_polyphony)?;
            print_storm("sample slot only", sample_rate, seconds, &sample_only);
        }
    }
    print_burst(&assets)?;
    print_sine_bank_steps()?;
    print_sample_durations(&assets);
    Ok(())
}

/// The step figure on the committed sine bank, where a test can hold it without the
/// downloaded one: three held notes, a 4-note sounding cap, and one note struck and
/// released every 64 frames so tails overflow an 8-voice pool.
fn print_sine_bank_steps() -> Result<(), String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../crates/music_midi/fixtures/sine.sf2");
    let bank = load_sound_font(&std::fs::read(&path).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    for maximum_polyphony in [8, 256] {
        let mut mixer = Mixer::new(
            None,
            vec![InstrumentSlot::new(&bank, 44_100, SlotCapacity { sounding_cap: 4, maximum_polyphony }, 0, 0)
                .map_err(|error| error.to_string())?],
            None,
        );
        let target = SlotTarget::Instrument(0);
        for key in [45, 52, 57] {
            mixer.apply(SpikeCommand::NoteOn { target, source: 1, key, velocity: 100 });
        }
        let mut left = [0.0_f32; BLOCK_FRAMES];
        let mut right = [0.0_f32; BLOCK_FRAMES];
        let mut largest_step = 0.0_f32;
        for block in 0..800 {
            let key = 84 + (block % 12) as u8;
            mixer.apply(SpikeCommand::NoteOn { target, source: 2, key, velocity: 100 });
            largest_step = largest_step.max(mixer.render_block(&mut left, &mut right).largest_step);
            mixer.apply(SpikeCommand::NoteOff { target, source: 2, key });
            largest_step = largest_step.max(mixer.render_block(&mut left, &mut right).largest_step);
        }
        println!("== sine bank, tails overflowing the pool, pool {maximum_polyphony}: largest step {largest_step:.4}");
    }
    Ok(())
}

fn print_load_cost(asset_base: &Path) -> Result<(), String> {
    let mut read_milliseconds = Vec::new();
    let mut parse_milliseconds = Vec::new();
    let mut size = 0;
    for _ in 0..5 {
        let started = Instant::now();
        let bytes = common::vfs::read(&asset_base.join(BANK_PATH)).map_err(|error| error.to_string())?;
        read_milliseconds.push(started.elapsed().as_secs_f64() * 1000.0);
        size = bytes.len();
        let started = Instant::now();
        load_sound_font(&bytes).map_err(|error| error.to_string())?;
        parse_milliseconds.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    read_milliseconds.sort_by(f64::total_cmp);
    parse_milliseconds.sort_by(f64::total_cmp);
    println!("== bank load ({BANK_PATH}, {size} bytes = {:.2} MiB), median of 5", size as f64 / 1_048_576.0);
    println!("read {:.1} ms, parse {:.1} ms", read_milliseconds[2], parse_milliseconds[2]);
    for (label, megabits_per_second) in [("10 Mbit/s", 10.0), ("50 Mbit/s", 50.0), ("200 Mbit/s", 200.0)] {
        println!("download at {label}: {:.1} s", size as f64 * 8.0 / (megabits_per_second * 1_000_000.0));
    }
    Ok(())
}

fn print_profiles(assets: &SpikeAssets) {
    println!("== preset profiles (layering = most voices one note-on starts; release = longest region release)");
    for (bank, program) in CAST_PRESETS {
        match preset_profile(&assets.bank, bank, program) {
            Some(profile) => println!(
                "{bank:3}:{program:<3} {:24} layering {} release {:.2} s",
                profile.name, profile.layering_factor, profile.longest_release_seconds
            ),
            None => println!("{bank:3}:{program:<3} not in this bank"),
        }
    }
}

fn mixer_with_polyphony(assets: &SpikeAssets, sample_rate: u32, maximum_polyphony: usize) -> Result<Mixer, String> {
    if maximum_polyphony == CAPACITY.maximum_polyphony {
        return worst_case_mixer(assets, sample_rate, TriggerMode::OneShot);
    }
    let mut mixer = worst_case_mixer(assets, sample_rate, TriggerMode::OneShot)?;
    let capacity = SlotCapacity { maximum_polyphony, ..CAPACITY };
    mixer.slots.clear();
    for (bank, program, _) in SLOT_PRESETS {
        mixer.slots.push(
            InstrumentSlot::new(&assets.bank, sample_rate as i32, capacity, bank, program).map_err(|error| error.to_string())?,
        );
    }
    Ok(mixer)
}

fn render_storm(assets: &SpikeAssets, sample_rate: u32, config: StormConfig, seconds: f64, maximum_polyphony: usize) -> Result<StormRun, String> {
    let mut mixer = mixer_with_polyphony(assets, sample_rate, maximum_polyphony)?;
    let mut storm = Storm::new(config, 0x5EED);
    let total_blocks = (seconds * f64::from(sample_rate)) as usize / BLOCK_FRAMES;
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut commands = Vec::new();
    let mut run = StormRun {
        block_microseconds: Vec::with_capacity(total_blocks),
        output: Vec::with_capacity(total_blocks * BLOCK_FRAMES * 2),
        peak_sounding_notes: 0,
        peak_sample_voices: 0,
        largest_step: 0.0,
        steals: 0,
        retriggers: 0,
        lane_rejections: 0,
        reconstructions: 0,
        longest_reconstruction_microseconds: 0.0,
    };
    let seek_block = total_blocks / 2;
    for block in 0..total_blocks {
        let now_seconds = (block * BLOCK_FRAMES) as f64 / f64::from(sample_rate);
        storm.commands_until(now_seconds, &mut commands);
        if block == seek_block {
            commands.push(SpikeCommand::Seek { tick: 8 * 2 * 384 });
        }
        for command in commands.drain(..) {
            mixer.apply(command);
        }
        let started = Instant::now();
        let report = mixer.render_block(&mut left, &mut right);
        run.block_microseconds.push(started.elapsed().as_secs_f64() * 1_000_000.0);
        run.peak_sounding_notes = run.peak_sounding_notes.max(report.sounding_notes);
        run.peak_sample_voices = run.peak_sample_voices.max(report.sample_voices);
        run.largest_step = run.largest_step.max(report.largest_step);
        for (left_sample, right_sample) in left.iter().zip(right.iter()) {
            run.output.push(*left_sample);
            run.output.push(*right_sample);
        }
    }
    for slot in &mixer.slots {
        let counters = slot.counters();
        run.steals += counters.steals;
        run.retriggers += counters.retriggers;
        run.lane_rejections += counters.lane_rejections;
    }
    if let Some(background) = mixer.background.as_ref() {
        run.reconstructions = background.counters().reconstructions;
        run.longest_reconstruction_microseconds = background.counters().longest_reconstruction_microseconds;
    }
    Ok(run)
}

fn print_storm(label: &str, sample_rate: u32, seconds: f64, run: &StormRun) {
    let mut sorted = run.block_microseconds.clone();
    sorted.sort_by(f64::total_cmp);
    let mean = sorted.iter().sum::<f64>() / sorted.len().max(1) as f64;
    let percentile = |fraction: f64| sorted[((sorted.len() as f64 * fraction) as usize).min(sorted.len() - 1)];
    let budget_microseconds = BLOCK_FRAMES as f64 / f64::from(sample_rate) * 1_000_000.0;
    println!("== storm: {label} at {sample_rate} Hz, {seconds} s, pool {}", CAPACITY.maximum_polyphony);
    println!(
        "block render mean {mean:.1} us, p99 {:.1} us, longest {:.1} us, of a {budget_microseconds:.0} us block ({:.1}% mean load, {:.0}x real time)",
        percentile(0.99),
        percentile(1.0),
        mean / budget_microseconds * 100.0,
        budget_microseconds / mean,
    );
    println!(
        "peak sounding notes {} (8 slots x cap {}), peak sample voices {}, ledger steals {}, retriggers {}, lane rejections {}",
        run.peak_sounding_notes, CAPACITY.sounding_cap, run.peak_sample_voices, run.steals, run.retriggers, run.lane_rejections
    );
    println!(
        "largest sample-to-sample step {:.4}, seeks and loop wraps {}, longest reconstruction {:.1} us",
        run.largest_step, run.reconstructions, run.longest_reconstruction_microseconds
    );
}

fn print_pool_comparison(label: &str, proposed: &StormRun, unbounded: &StormRun) {
    let first_difference = proposed.output.iter().zip(&unbounded.output).position(|(left, right)| left != right);
    let largest_difference = proposed
        .output
        .iter()
        .zip(&unbounded.output)
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f32, f32::max);
    match first_difference {
        None => println!("== pool {} vs {UNBOUNDED_POLYPHONY} ({label}): bit-identical, the synthesizer's stealer never ran", CAPACITY.maximum_polyphony),
        Some(index) => println!(
            "== pool {} vs {UNBOUNDED_POLYPHONY} ({label}): first difference at {:.3} s, largest difference {largest_difference:.4}",
            CAPACITY.maximum_polyphony,
            (index / 2) as f64 / 44_100.0
        ),
    }
}

/// A 200-note burst inside one second on the two long-release slots (piano, strings),
/// one lane hammering one key throughout, rendered with the proposed pool and an
/// unbounded one.
fn print_burst(assets: &SpikeAssets) -> Result<(), String> {
    let sample_rate = 44_100;
    let mut outputs = Vec::new();
    for maximum_polyphony in [CAPACITY.maximum_polyphony, UNBOUNDED_POLYPHONY] {
        let capacity = SlotCapacity { maximum_polyphony, ..CAPACITY };
        let slots = vec![
            InstrumentSlot::new(&assets.bank, sample_rate, capacity, 0, 0).map_err(|error| error.to_string())?,
            InstrumentSlot::new(&assets.bank, sample_rate, capacity, 0, 48).map_err(|error| error.to_string())?,
        ];
        let mut mixer = Mixer::new(None, slots, None);
        let mut left = [0.0_f32; BLOCK_FRAMES];
        let mut right = [0.0_f32; BLOCK_FRAMES];
        let mut output = Vec::new();
        let mut largest_step = 0.0_f32;
        let blocks_per_second = sample_rate as usize / BLOCK_FRAMES;
        for block in 0..blocks_per_second * 6 {
            if block < blocks_per_second {
                for note in 0..3 {
                    let index = block * 3 + note;
                    if index < 200 {
                        let target = SlotTarget::Instrument(index % 2);
                        let key = 36 + (index * 7 % 48) as u8;
                        mixer.apply(SpikeCommand::NoteOn { target, source: (index % 40) as u32, key, velocity: 110 });
                    }
                }
                mixer.apply(SpikeCommand::NoteOn { target: SlotTarget::Instrument(0), source: 999, key: 60, velocity: 120 });
            }
            if block == blocks_per_second * 2 {
                mixer.apply(SpikeCommand::ReleaseAll);
            }
            largest_step = largest_step.max(mixer.render_block(&mut left, &mut right).largest_step);
            output.extend(left.iter().zip(right.iter()).flat_map(|(left_sample, right_sample)| [*left_sample, *right_sample]));
        }
        let steals: u64 = mixer.slots.iter().map(|slot| slot.counters().steals).sum();
        let retriggers: u64 = mixer.slots.iter().map(|slot| slot.counters().retriggers).sum();
        println!(
            "== burst: 200 notes in 1 s on piano + strings, one key hammered, pool {maximum_polyphony}: largest step {largest_step:.4}, ledger steals {steals}, retriggers {retriggers}"
        );
        outputs.push(output);
    }
    let first_difference = outputs[0].iter().zip(&outputs[1]).position(|(left, right)| left != right);
    let largest_difference = outputs[0].iter().zip(&outputs[1]).map(|(left, right)| (left - right).abs()).fold(0.0_f32, f32::max);
    match first_difference {
        None => println!("burst pool comparison: bit-identical, the synthesizer's stealer never ran"),
        Some(index) => println!(
            "burst pool comparison: first difference at {:.3} s, largest difference {largest_difference:.4}",
            (index / 2) as f64 / f64::from(sample_rate)
        ),
    }
    Ok(())
}

fn print_sample_durations(assets: &SpikeAssets) {
    let output_rate = 44_100;
    println!("== sample pitching ({} Hz source, root 60, one-shot, rendered at {output_rate} Hz)", assets.sample.sample_rate);
    let unpitched = rendered_one_shot_frames(assets, output_rate, 60);
    for key in [60, 67, 48] {
        let frames = rendered_one_shot_frames(assets, output_rate, key);
        println!(
            "key {key}: {frames} frames = {:.3} s, {:.3}x the root's length (expected {:.3}x)",
            frames as f64 / f64::from(output_rate),
            frames as f64 / unpitched as f64,
            2f64.powf(-f64::from(key - 60) / 12.0)
        );
    }
}

fn rendered_one_shot_frames(assets: &SpikeAssets, output_rate: u32, key: i32) -> usize {
    let mut slot = SampleSlot::new(assets.sample.clone(), 60, TriggerMode::OneShot, output_rate, 16);
    slot.note_on(0, key, 127);
    slot.note_off(0, key);
    let mut left = [0.0_f32; BLOCK_FRAMES];
    let mut right = [0.0_f32; BLOCK_FRAMES];
    let mut frames = 0;
    while slot.voice_count() > 0 {
        left.fill(0.0);
        right.fill(0.0);
        slot.render_add(&mut left, &mut right);
        frames += BLOCK_FRAMES;
    }
    frames
}
