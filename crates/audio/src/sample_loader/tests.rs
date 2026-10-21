//! The sample loader against the conductor's sine fixtures and hand-built bad files.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

const PCM_FORMAT: u16 = 1;
const FLOAT_FORMAT: u16 = 3;

/// A conductor fixture by name, from the music crate beside this one.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../music_midi/fixtures")
        .join(name)
}

/// A WAV file: a `fmt ` chunk describing `data`, which holds `bits`-bit samples.
fn wav(format: u16, channels: u16, sample_rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let block_align = channels * bits / 8;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&format.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

/// `frames` frames of a quiet 16-bit ramp on every channel.
fn pcm_wav(channels: u16, sample_rate: u32, frames: usize) -> Vec<u8> {
    let data: Vec<u8> = (0..frames * usize::from(channels))
        .flat_map(|index| ((index % 200) as i16 * 10).to_le_bytes())
        .collect();
    wav(PCM_FORMAT, channels, sample_rate, 16, &data)
}

/// A temp file, removed when dropped (also on a failed assert).
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).ok();
    }
}

fn write_temp_file(name: &str, bytes: &[u8]) -> TempFile {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "insiculous_sample_test_{}_{}_{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes).expect("temp dir must be writable");
    TempFile(path)
}

#[test]
fn the_four_sine_fixtures_load_at_their_own_rate_and_channels() {
    for (name, sample_rate, channels) in [
        ("sine_48000_mono.wav", 48_000, 1),
        ("sine_48000_stereo.wav", 48_000, 2),
        ("sine_22050_mono.wav", 22_050, 1),
        ("sine_22050_stereo.wav", 22_050, 2),
    ] {
        let pcm = load_sample_pcm(fixture(name)).expect("a fixture loads");
        assert_eq!((pcm.sample_rate(), pcm.channels()), (sample_rate, channels));
        assert_eq!(
            pcm.frame_count(),
            sample_rate as usize,
            "{name} is one second"
        );
        let loudest = pcm
            .samples()
            .iter()
            .fold(0.0f32, |most, value| most.max(value.abs()));
        assert!(
            (loudest - 0.5).abs() < 0.01,
            "{name} peaks at half scale: {loudest}"
        );
    }
}

#[test]
fn a_file_that_cannot_be_read_is_an_error_naming_its_path() {
    let missing = std::env::temp_dir().join("insiculous_sample_test_definitely_missing.wav");

    let error = load_sample_pcm(&missing).expect_err("nothing there");

    assert!(matches!(
        &error,
        AudioError::Sample {
            problem: SampleProblem::Unreadable(_),
            ..
        }
    ));
    assert!(
        error.to_string().contains("definitely_missing.wav"),
        "{error}"
    );
}

#[test]
fn a_file_over_four_mebibytes_is_refused_before_it_is_decoded() {
    let mut bytes = pcm_wav(1, 44_100, 1);
    bytes.resize(SAMPLE_BYTE_LIMIT + 1, 0xFF);

    assert!(matches!(
        decode_sample_pcm(bytes),
        Err(SampleProblem::TooManyBytes { bytes, limit: SAMPLE_BYTE_LIMIT })
            if bytes == SAMPLE_BYTE_LIMIT + 1
    ));
}

/// A file the loader may not read but whose size it can see: refused for its size proves
/// the size was checked before any read was tried.
#[cfg(unix)]
#[test]
fn a_file_over_four_mebibytes_is_refused_by_its_size_before_it_is_read() {
    use std::os::unix::fs::PermissionsExt;
    let oversized = write_temp_file("oversized.wav", &pcm_wav(1, 44_100, 1));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&oversized.0)
        .expect("the temporary file opens");
    file.set_len(SAMPLE_BYTE_LIMIT as u64 + 1)
        .expect("the file grows sparsely");
    std::fs::set_permissions(&oversized.0, std::fs::Permissions::from_mode(0o000))
        .expect("the file is made unreadable");
    if std::fs::read(&oversized.0).is_ok() {
        // Run as root, every file is readable; the order cannot be told apart.
        return;
    }

    let error = load_sample_pcm(&oversized.0).expect_err("an oversized file");

    assert!(
        matches!(
            &error,
            AudioError::Sample { problem: SampleProblem::TooManyBytes { bytes, .. }, path }
                if *bytes == SAMPLE_BYTE_LIMIT + 1 && path.ends_with("oversized.wav")
        ),
        "{error}"
    );
}

#[test]
fn a_stream_that_is_not_wav_is_refused_naming_wav_whatever_the_file_is_called() {
    let ogg_named_wav =
        write_temp_file("disguised.wav", b"OggS\0\x02\0\0\0\0\0\0\0\0rest of a page");
    let mp3 = b"ID3\x04\0\0\0\0\0\x00rest of a tag".to_vec();

    let error = load_sample_pcm(&ogg_named_wav.0).expect_err("an OGG stream");

    assert!(matches!(
        &error,
        AudioError::Sample { problem: SampleProblem::NotWav, path } if path.ends_with("disguised.wav")
    ));
    assert!(
        error.to_string().contains("WAV is the supported format"),
        "{error}"
    );
    assert!(matches!(decode_sample_pcm(mp3), Err(SampleProblem::NotWav)));
    assert!(matches!(
        decode_sample_pcm(Vec::new()),
        Err(SampleProblem::NotWav)
    ));
}

#[test]
fn a_wav_outside_the_sample_limits_is_refused_naming_the_limit() {
    let refusals = [
        ("no frames", pcm_wav(1, 44_100, 0)),
        ("one frame", pcm_wav(1, 44_100, 1)),
        ("eleven seconds", pcm_wav(1, 8_000, 88_000)),
        ("200 kHz", pcm_wav(1, 200_000, 2_000)),
        ("three channels", pcm_wav(3, 44_100, 4_410)),
    ];
    let mut problems = Vec::new();
    for (what, bytes) in refusals {
        let problem = decode_sample_pcm(bytes).expect_err(what);
        problems.push((what, problem.to_string()));
        match (what, problem) {
            (
                "no frames" | "one frame",
                SampleProblem::OutsideLimits(PcmError::TooShort { .. }),
            ) => {}
            (
                "eleven seconds",
                SampleProblem::OutsideLimits(PcmError::TooLong {
                    maximum_seconds, ..
                }),
            ) => {
                assert_eq!(maximum_seconds, 10.0);
            }
            (
                "200 kHz",
                SampleProblem::OutsideLimits(PcmError::SampleRateOutOfRange {
                    sample_rate, ..
                }),
            ) => {
                assert_eq!(sample_rate, 200_000);
            }
            (
                "three channels",
                SampleProblem::OutsideLimits(PcmError::UnsupportedChannels { channels }),
            ) => {
                assert_eq!(channels, 3);
            }
            (what, problem) => panic!("{what}: refused for the wrong reason: {problem:?}"),
        }
    }
    assert!(
        problems.iter().any(|(_, message)| message.contains("10 s")),
        "{problems:?}"
    );
}

#[test]
fn a_float_wav_carrying_nan_decodes_finite_because_the_decoder_yields_sixteen_bit_integers() {
    let values: Vec<f32> = (0..4_410)
        .map(|frame| if frame == 100 { f32::NAN } else { 0.25 })
        .collect();
    let data: Vec<u8> = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();

    let pcm = decode_sample_pcm(wav(FLOAT_FORMAT, 1, 44_100, 32, &data))
        .expect("the decoder hands over integers, so the NaN arrives as a finite value");

    assert!(pcm.samples().iter().all(|value| value.is_finite()));
    assert_eq!(pcm.frame_count(), 4_410);
}
