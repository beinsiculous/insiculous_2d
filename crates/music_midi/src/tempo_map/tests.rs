use super::*;
use crate::test_support::*;

const CD_RATE: u32 = 44_100;
const STUDIO_RATE: u32 = 48_000;
/// Egmont's tempo and meter both change here: 84 → 168 BPM, 3/2 → 3/4.
const ALLEGRO_TICK: u64 = 55_296;
/// And again here: 168 → 152 BPM, 3/4 → 4/4.
const CODA_TICK: u64 = 357_120;
const EGMONT_LAST_TICK: u64 = 449_664;

fn egmont_map() -> TempoMap {
    read(EGMONT).0.tempo_map()
}

fn assert_seconds(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} s, expected {expected} s"
    );
}

#[test]
fn egmont_reads_its_three_tempi_and_three_meters() {
    let (song, _) = read(EGMONT);

    let tempi: Vec<_> = song
        .tempo_changes()
        .iter()
        .map(|change| (change.tick, change.beats_per_minute().round()))
        .collect();
    let meters: Vec<_> = song
        .time_signatures()
        .iter()
        .map(|signature| (signature.tick, signature.numerator, signature.denominator))
        .collect();

    assert_eq!(
        tempi,
        vec![(0, 84.0), (ALLEGRO_TICK, 168.0), (CODA_TICK, 152.0)]
    );
    assert_eq!(
        meters,
        vec![(0, 3, 2), (ALLEGRO_TICK, 3, 4), (CODA_TICK, 4, 4)]
    );
}

#[test]
fn egmont_ticks_map_to_seconds_and_samples_across_every_tempo_change() {
    let map = egmont_map();

    let at_44100: Vec<u64> = [ALLEGRO_TICK, CODA_TICK, EGMONT_LAST_TICK]
        .iter()
        .map(|tick| map.sample_at_tick(*tick, CD_RATE))
        .collect();
    let at_48000: Vec<u64> = [ALLEGRO_TICK, CODA_TICK, EGMONT_LAST_TICK]
        .iter()
        .map(|tick| map.sample_at_tick(*tick, STUDIO_RATE))
        .collect();

    // 144 quarters at 714 285 µs, then 786 at 357 142 µs, then 241 at 394 736 µs.
    assert_seconds(map.seconds_at_tick(ALLEGRO_TICK), 102.857_04);
    assert_seconds(map.seconds_at_tick(CODA_TICK), 383.570_652);
    assert_seconds(map.seconds_at_tick(EGMONT_LAST_TICK), 478.702_028);
    assert_eq!(
        at_44100,
        vec![4_535_996, 16_915_466, 21_110_760],
        "the first sample at or after each tick"
    );
    assert_eq!(at_48000, vec![4_937_138, 18_411_392, 22_977_698]);
    assert_eq!(map.microseconds_per_quarter_at(CODA_TICK - 1), 357_142);
    assert_eq!(map.microseconds_per_quarter_at(CODA_TICK), 394_736);
    assert_eq!(map.tick_at_seconds(383.570_652 + 0.000_5), CODA_TICK);
}

#[test]
fn a_tick_lands_on_the_first_sample_at_or_after_it_and_maps_back_exactly() {
    let map = egmont_map();
    let ticks = (1..EGMONT_LAST_TICK).step_by(997).chain([
        ALLEGRO_TICK - 1,
        ALLEGRO_TICK,
        CODA_TICK - 1,
        CODA_TICK,
    ]);

    for tick in ticks {
        let sample = map.sample_at_tick(tick, CD_RATE);
        assert_eq!(
            map.tick_at_sample(sample, CD_RATE),
            tick,
            "tick {tick} at sample {sample}"
        );
        assert!(
            map.tick_at_sample(sample - 1, CD_RATE) < tick,
            "sample {} is before tick {tick}",
            sample - 1
        );
    }
}

#[test]
fn egmont_bars_and_beats_follow_its_three_meters() {
    let map = egmont_map();
    let position = |bar, beat, beat_in_bar| MusicalPosition {
        bar,
        beat,
        beat_in_bar,
    };

    // 3/2 beats are half notes (768 ticks), 3/4 and 4/4 beats quarters (384).
    assert_eq!(map.position_at_tick(ALLEGRO_TICK - 1), position(23, 71, 2));
    assert_eq!(map.position_at_tick(ALLEGRO_TICK), position(24, 72, 0));
    assert_eq!(
        map.position_at_tick(ALLEGRO_TICK + 384 * 4),
        position(25, 76, 1)
    );
    assert_eq!(map.position_at_tick(CODA_TICK - 1), position(285, 857, 2));
    assert_eq!(map.position_at_tick(CODA_TICK), position(286, 858, 0));
    assert_eq!(
        map.position_at_tick(CODA_TICK + 384 * 11),
        position(288, 869, 3)
    );
    assert_eq!(map.tick_at_beat(71), ALLEGRO_TICK - 768);
    assert_eq!(map.tick_at_beat(72), ALLEGRO_TICK);
    assert_eq!(map.tick_at_beat(73), ALLEGRO_TICK + 384);
    assert_eq!(map.tick_at_beat(858), CODA_TICK);
}

#[test]
fn beat_crossings_are_counted_by_integer_boundaries_across_a_tempo_and_meter_change() {
    let map = egmont_map();
    let boundary = |beat| map.sample_at_tick(map.tick_at_beat(beat), CD_RATE);
    let from = boundary(70) - 1;
    let to = boundary(75);

    let crossing_samples: Vec<u64> = (from..to)
        .filter(|sample| map.beats_crossed(*sample, sample + 1, CD_RATE) == 1)
        .map(|sample| sample + 1)
        .collect();
    let stepped: u64 = (from..to)
        .map(|sample| map.beats_crossed(sample, sample + 1, CD_RATE))
        .sum();

    assert_eq!(
        map.beats_crossed(from, to, CD_RATE),
        6,
        "beats 70 to 75, each once"
    );
    assert_eq!(
        map.beats_crossed(boundary(70), to, CD_RATE),
        5,
        "a boundary on the start sample was already crossed"
    );
    assert_eq!(
        stepped, 6,
        "no boundary is counted twice or skipped sample by sample"
    );
    assert_eq!(
        crossing_samples,
        (70..=75).map(boundary).collect::<Vec<_>>()
    );
    assert_eq!(map.beats_crossed(to, from, CD_RATE), 0);
}

#[test]
fn a_file_without_tempo_or_meter_plays_at_120_bpm_in_four_four() {
    let bytes = smf(&[vec![(0, note_on(0, 60, 100)), (384, note_off(0, 60))]]);
    let (mut bach, _) = read(BACH_INVENTION);
    let bach_own = bach.tempo_map();
    bach.remove_tempo(0);
    bach.remove_time_signature(0);

    for map in [read(&bytes).0.tempo_map(), bach.tempo_map()] {
        assert_eq!(
            map.microseconds_per_quarter_at(0),
            DEFAULT_MICROSECONDS_PER_QUARTER
        );
        assert_seconds(map.seconds_at_tick(384), 0.5);
        assert_eq!(
            map.position_at_tick(384 * 4),
            MusicalPosition {
                bar: 1,
                beat: 4,
                beat_in_bar: 0
            }
        );
    }
    assert_seconds(bach_own.seconds_at_tick(384), 0.9375);
}

#[test]
fn a_tempo_after_the_start_leaves_the_default_in_force_before_it() {
    let map = TempoMap::new(
        384,
        &[TempoChange {
            tick: 768,
            microseconds_per_quarter: 1_000_000,
        }],
        &[],
    );

    assert_seconds(map.seconds_at_tick(768), 1.0);
    assert_seconds(map.seconds_at_tick(768 + 384), 2.0);
}

#[test]
fn a_time_signature_landing_mid_bar_closes_the_partial_bar() {
    let signatures = [
        TimeSignature::new(0, 4, 4),
        TimeSignature::new(384 * 6, 3, 4),
    ];
    let map = TempoMap::new(384, &[], &signatures);

    assert_eq!(
        map.position_at_tick(384 * 5),
        MusicalPosition {
            bar: 1,
            beat: 5,
            beat_in_bar: 1
        }
    );
    assert_eq!(
        map.position_at_tick(384 * 6),
        MusicalPosition {
            bar: 2,
            beat: 6,
            beat_in_bar: 0
        }
    );
    assert_eq!(
        map.position_at_tick(384 * 9),
        MusicalPosition {
            bar: 3,
            beat: 9,
            beat_in_bar: 0
        }
    );
}
