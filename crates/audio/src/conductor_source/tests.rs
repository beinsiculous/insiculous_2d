use std::sync::Arc;

use music_midi::midly::num::{u15, u24, u28, u4, u7};
use music_midi::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use music_midi::{
    parse_song_file, BusGains, Conductor, ConductorCommand, ConductorConfig, ConductorHandle, Song,
    SongData, SourceId,
};

use super::*;
use crate::sample_loader::decode_sample_pcm;
use crate::test_support::*;

const SAMPLE_RATE: u32 = 44_100;

fn song(midi: &[u8], sidecar: &str) -> Arc<Song> {
    let file = parse_song_file("test.mid.ron", sidecar).expect("the sidecar parses");
    let (data, _) = SongData::from_smf(midi).expect("the MIDI reads");
    let pcm = decode_sample_pcm(SINE_WAV.to_vec()).expect("the sine sample decodes");
    Arc::new(
        Song::new(file, data, &[(FONT_PATH, SINE_FONT)], &[(SAMPLE_PATH, pcm)])
            .expect("the song builds"),
    )
}

/// A conductor playing `song`, its handle, and the source over it at `gains`.
fn playing(
    song: &Arc<Song>,
    gains: BusGains,
) -> (ConductorSource, ConductorHandle, Arc<SourceMeters>) {
    let (conductor, handle) = loaded(song);
    let meters = Arc::new(SourceMeters::default());
    let buses = Arc::new(BusLevels::new(gains.music, gains.sfx));
    (
        ConductorSource::new(conductor, buses, Arc::clone(&meters)),
        handle,
        meters,
    )
}

fn loaded(song: &Arc<Song>) -> (Conductor, ConductorHandle) {
    let (mut conductor, handle) =
        Conductor::new(ConductorConfig::new(SAMPLE_RATE)).expect("the frozen config builds");
    conductor.load(Arc::clone(song)).expect("the song loads");
    handle.send(ConductorCommand::Play).expect("queued");
    (conductor, handle)
}

fn trigger(handle: &ConductorHandle, song: &Song, action: &str, source: SourceId) {
    let binding = song.binding_index(action).expect("the action is bound");
    handle
        .send(ConductorCommand::Trigger {
            binding,
            source,
            stamp: Instant::now(),
        })
        .expect("queued");
}

/// `frames` frames of the source, pulled as rodio pulls them, left channel then right.
fn pull(source: &mut ConductorSource, frames: usize) -> Vec<f32> {
    (0..frames * 2)
        .map(|_| source.next().unwrap_or(f32::NAN))
        .collect()
}

fn root_mean_square(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

#[test]
fn music_at_zero_keeps_triggers_audible_and_sfx_at_zero_mutes_them_while_the_clock_runs() {
    let song = song(&song_midi(true), &sidecar(None));
    let tenth_of_a_second = SAMPLE_RATE as usize / 10;

    let (mut source, handle, _) = playing(
        &song,
        BusGains {
            music: 0.0,
            sfx: 1.0,
        },
    );
    let background_alone = pull(&mut source, tenth_of_a_second);
    trigger(&handle, &song, "tone", keyboard());
    let with_trigger = pull(&mut source, tenth_of_a_second);
    assert_eq!(
        peak(&background_alone),
        0.0,
        "music at zero mutes the background"
    );
    assert!(
        root_mean_square(&with_trigger) > 0.01,
        "the trigger stays audible at music zero"
    );

    let (mut quiet, quiet_handle, _) = playing(
        &song,
        BusGains {
            music: 1.0,
            sfx: 0.0,
        },
    );
    let (mut triggered, triggered_handle, _) = playing(
        &song,
        BusGains {
            music: 1.0,
            sfx: 0.0,
        },
    );
    pull(&mut quiet, tenth_of_a_second);
    pull(&mut triggered, tenth_of_a_second);
    trigger(&triggered_handle, &song, "tone", keyboard());
    let without = pull(&mut quiet, tenth_of_a_second);
    let with = pull(&mut triggered, tenth_of_a_second);
    assert!(
        root_mean_square(&without) > 0.01,
        "the background plays at sfx zero"
    );
    assert_eq!(with, without, "sfx at zero mutes the trigger exactly");
    let position = triggered_handle.position().snapshot();
    assert_eq!(
        position.accepted_triggers, 1,
        "the trigger still played, silently"
    );
    assert!(
        position.sample_position >= 2 * tenth_of_a_second as u64 - BLOCK_FRAMES as u64,
        "the clock ran: {position:?}"
    );
    assert_eq!(quiet_handle.position().snapshot().accepted_triggers, 0);
}

/// The gain the limiter left on each frame of a constant signal, block by block.
fn gains_over(levels: &[f32]) -> Vec<f32> {
    let mut limiter = Limiter::new(SAMPLE_RATE, BLOCK_FRAMES);
    let mut gains = Vec::new();
    for block in levels.chunks(BLOCK_FRAMES) {
        let mut left = block.to_vec();
        let mut right = block.to_vec();
        limiter.apply(&mut left, &mut right);
        gains.extend(left.iter().zip(block).map(|(after, before)| after / before));
    }
    gains
}

#[test]
fn a_quiet_stream_passes_the_limiter_untouched() {
    let mut limiter = Limiter::new(SAMPLE_RATE, BLOCK_FRAMES);
    let quiet: Vec<f32> = (0..BLOCK_FRAMES).map(|frame| 0.5 * (frame as f32 * 0.1).sin()).collect();
    let (mut left, mut right) = (quiet.clone(), quiet.clone());

    assert_eq!(limiter.apply(&mut left, &mut right), 0);
    assert_eq!(left, quiet);
    assert_eq!(limiter.gain(), 1.0);
}

#[test]
fn a_loud_passage_is_turned_down_ahead_of_it_never_past_the_threshold_and_let_back_up() {
    let quiet = 0.3;
    let loud = 3.0;
    let loud_from = 10 * BLOCK_FRAMES + BLOCK_FRAMES / 2;
    let loud_until = loud_from + 20 * BLOCK_FRAMES;
    let frames = loud_until + SAMPLE_RATE as usize;
    let levels: Vec<f32> = (0..frames)
        .map(|frame| if (loud_from..loud_until).contains(&frame) { loud } else { quiet })
        .collect();

    let gains = gains_over(&levels);

    let attack_step = 1.0 / (LIMITER_ATTACK_SECONDS * SAMPLE_RATE as f32);
    for (frame, (gain, level)) in gains.iter().zip(&levels).enumerate() {
        assert!(
            gain * level <= LIMITER_THRESHOLD + f32::EPSILON,
            "frame {frame} passes the threshold"
        );
    }
    let steepest_fall = gains
        .windows(2)
        .map(|pair| pair[0] - pair[1])
        .fold(0.0, f32::max);
    assert!(
        steepest_fall <= attack_step + 1e-6,
        "the gain falls at the attack rate, never in one step: {steepest_fall}"
    );
    assert!(gains[loud_from - 1] < 1.0, "it falls before the loud frame, which it saw coming");
    assert!((gains[loud_from] - LIMITER_THRESHOLD / loud).abs() < 1e-6);
    let after = &gains[loud_until..];
    assert!(after[0] < 0.5, "the release is gradual");
    assert_eq!(*after.last().expect("frames"), 1.0, "and back to unity within a second");
}

/// A song of the storm: a 24-note chord for the synthesizer's sounding cap and sixteen
/// single notes for the sample's live voices.
fn storm_song() -> Arc<Song> {
    let quarter = u28::new(QUARTER);
    let event = |delta: u28, kind| TrackEvent { delta, kind };
    let note = |key: u8, on: bool| TrackEventKind::Midi {
        channel: u4::new(0),
        message: if on {
            MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(127),
            }
        } else {
            MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            }
        },
    };
    let end = TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    };
    let control = vec![
        event(
            u28::new(0),
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
        ),
        event(
            u28::new(QUARTER * 64),
            TrackEventKind::Meta(MetaMessage::Marker(b"end")),
        ),
        end,
    ];
    let mut chord = vec![event(
        u28::new(0),
        TrackEventKind::Meta(MetaMessage::TrackName(b"chord")),
    )];
    for key in 48..72 {
        chord.push(event(u28::new(0), note(key, true)));
    }
    for (index, key) in (48..72).enumerate() {
        let delta = if index == 0 {
            u28::new(QUARTER * 32)
        } else {
            u28::new(0)
        };
        chord.push(event(delta, note(key, false)));
    }
    chord.push(end);
    let mut hits = vec![event(
        u28::new(0),
        TrackEventKind::Meta(MetaMessage::TrackName(b"hits")),
    )];
    for _ in 0..16 {
        hits.push(event(u28::new(0), note(60, true)));
        hits.push(event(quarter, note(60, false)));
    }
    hits.push(end);
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(QUARTER as u16))),
        tracks: vec![control, chord, hits],
    }
    .write_std(&mut bytes)
    .expect("a hand-built SMF writes");
    song(
        &bytes,
        &format!(
            r#"SongFile(
                version: 1,
                soundfont: "{FONT_PATH}",
                parts: [(name: "chord", track: Name("chord")), (name: "hits", track: Name("hits"))],
                instruments: [
                    (name: "sine", source: SoundFont(path: "{FONT_PATH}", bank: 0, preset: 0)),
                    (name: "wav", source: Sample(path: "{SAMPLE_PATH}", root_note: 60, gain: 1.0, playback: OneShot)),
                ],
                bindings: [
                    (action: "chord", part: "chord", instrument: "sine"),
                    (action: "hit", part: "hits", instrument: "wav"),
                ],
            )"#
        ),
    )
}

#[test]
fn the_limiter_holds_the_worst_case_storm_at_its_threshold_with_every_sample_finite() {
    let song = storm_song();
    let (mut raw, raw_handle) = loaded(&song);
    let (mut limited, handle, meters) = playing(&song, BusGains::UNITY);
    let mut left = [0.0; BLOCK_FRAMES];
    let mut right = [0.0; BLOCK_FRAMES];
    let mut raw_output = Vec::new();
    let mut limited_output = Vec::new();
    let mut most_sample_voices = 0;
    for block in 0..(SAMPLE_RATE as usize / BLOCK_FRAMES) {
        if block == 1 {
            for sender in [&raw_handle, &handle] {
                trigger(sender, &song, "chord", source(1, 1));
            }
        }
        if (2..18).contains(&block) {
            for sender in [&raw_handle, &handle] {
                trigger(sender, &song, "hit", source(100 + block as u64, 1));
            }
        }
        raw.render_at(Instant::now(), &mut left, &mut right, BusGains::UNITY);
        raw_output.extend(left.iter().chain(right.iter()));
        limited_output.extend(pull(&mut limited, BLOCK_FRAMES));
        most_sample_voices = most_sample_voices.max(handle.position().snapshot().sample_voices);
    }

    let raw_peak = peak(&raw_output);
    let limited_peak = peak(&limited_output);
    println!("storm peak before the limiter {raw_peak}, after {limited_peak}");
    let position = handle.position().snapshot();
    assert_eq!(position.accepted_triggers, 17, "{position:?}");
    assert_eq!(
        most_sample_voices, 16,
        "sixteen sample voices sound at once"
    );
    assert!(
        raw_peak > LIMITER_THRESHOLD,
        "the storm reaches the knee: {raw_peak}"
    );
    assert!(
        limited_peak < 1.0,
        "the limited storm stays under full scale: {limited_peak}"
    );
    assert!(limited_output.iter().all(|sample| sample.is_finite()));
    assert_eq!(position.non_finite_samples, 0);
    assert!(
        meters.readings().limited_samples > 0,
        "the limiter's work is counted"
    );
}

#[test]
fn the_frames_of_a_block_not_yet_handed_out_are_what_the_latency_counts() {
    let song = song(&song_midi(true), &sidecar(None));
    let (mut source, _, meters) = playing(&song, BusGains::UNITY);

    pull(&mut source, 10);
    assert_eq!(
        meters.readings().undelivered_frames,
        (BLOCK_FRAMES - 10) as u64
    );
    pull(&mut source, BLOCK_FRAMES - 10);
    assert_eq!(meters.readings().undelivered_frames, 0);
    source.advance_frames(5);
    assert_eq!(
        meters.readings().undelivered_frames,
        (BLOCK_FRAMES - 5) as u64
    );
}

/// Render `seconds` of `song` raw (no limiter), firing `action` every 33 ms from the first
/// tenth of a second — what holding a key does at the keyboard's repeat rate.
fn held_key(song: &Arc<Song>, action: &str, seconds: f64) -> Vec<f32> {
    let (mut conductor, handle) = loaded(song);
    let (mut left, mut right) = ([0.0; BLOCK_FRAMES], [0.0; BLOCK_FRAMES]);
    let mut output = Vec::new();
    let mut next_trigger_seconds = 0.1;
    let blocks = (seconds * f64::from(SAMPLE_RATE)) as usize / BLOCK_FRAMES;
    for block in 0..blocks {
        let block_seconds = (block * BLOCK_FRAMES) as f64 / f64::from(SAMPLE_RATE);
        if block_seconds >= next_trigger_seconds {
            trigger(&handle, song, action, keyboard());
            next_trigger_seconds += 0.033;
        }
        conductor.render_at(Instant::now(), &mut left, &mut right, BusGains::UNITY);
        output.extend(left.iter().chain(right.iter()));
    }
    output
}

#[test]
fn a_key_held_on_a_sample_instrument_plays_near_the_level_of_one_held_on_a_preset() {
    let song = song(&song_midi(false), &sidecar(None));

    let preset = root_mean_square(&held_key(&song, "tone", 2.0));
    let sample = root_mean_square(&held_key(&song, "blip", 2.0));
    println!("held key rms: preset {preset}, sample {sample}");

    assert!(
        sample < 2.0 * preset,
        "a sample voice sits near a preset's level: sample {sample}, preset {preset}"
    );
}
