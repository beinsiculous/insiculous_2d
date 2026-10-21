use super::*;
use crate::test_support::{
    fixture_pcm, SINE_22050_MONO, SINE_22050_STEREO, SINE_48000_MONO, SINE_48000_STEREO,
};

fn silence(sample_rate: u32, channels: u16, frames: usize) -> Result<OwnedPcm, PcmError> {
    OwnedPcm::new(
        sample_rate,
        channels,
        vec![0.0; frames * usize::from(channels)],
    )
}

#[test]
fn the_four_sine_fixtures_are_one_second_within_every_limit() {
    for (wav, sample_rate, channels) in [
        (SINE_48000_MONO, 48_000, 1),
        (SINE_48000_STEREO, 48_000, 2),
        (SINE_22050_MONO, 22_050, 1),
        (SINE_22050_STEREO, 22_050, 2),
    ] {
        let pcm = fixture_pcm(wav);
        assert_eq!((pcm.sample_rate(), pcm.channels()), (sample_rate, channels));
        assert_eq!(pcm.frame_count(), sample_rate as usize);
        assert!((pcm.seconds() - 1.0).abs() < 1e-9);
    }
}

#[test]
fn a_sample_rate_outside_eight_to_ninety_six_kilohertz_is_refused_naming_the_range() {
    for sample_rate in [7_999, 96_001, 200_000] {
        let error = silence(sample_rate, 1, 48_000).expect_err("out of range");
        assert_eq!(
            error,
            PcmError::SampleRateOutOfRange {
                sample_rate,
                minimum: 8_000,
                maximum: 96_000
            }
        );
    }
    assert!(silence(8_000, 1, 800).is_ok());
    assert!(silence(96_000, 2, 960).is_ok());
}

#[test]
fn neither_mono_nor_stereo_is_refused() {
    for channels in [0, 3, 6] {
        assert_eq!(
            OwnedPcm::check_format(44_100, channels),
            Err(PcmError::UnsupportedChannels { channels })
        );
    }
}

#[test]
fn a_stereo_buffer_with_half_a_frame_is_refused() {
    assert_eq!(
        OwnedPcm::new(44_100, 2, vec![0.0; 2 * 441 + 1]),
        Err(PcmError::PartialFrame {
            samples: 883,
            channels: 2
        })
    );
}

#[test]
fn a_sample_shorter_than_a_millisecond_is_refused_and_one_exactly_a_millisecond_is_kept() {
    for frames in [0, 1, 47] {
        assert_eq!(
            silence(48_000, 1, frames),
            Err(PcmError::TooShort {
                frames,
                sample_rate: 48_000,
                minimum_seconds: 0.001
            })
        );
    }
    assert!(silence(48_000, 1, 48).is_ok());
    assert_eq!(OwnedPcm::minimum_frames(8_000), 8);
    assert_eq!(OwnedPcm::minimum_frames(22_050), 23);
}

#[test]
fn a_sample_longer_than_ten_seconds_is_refused_and_one_exactly_ten_seconds_is_kept() {
    assert_eq!(
        silence(8_000, 1, 80_001),
        Err(PcmError::TooLong {
            sample_rate: 8_000,
            maximum_seconds: 10.0
        })
    );
    assert!(silence(8_000, 1, 80_000).is_ok());
    assert_eq!(OwnedPcm::maximum_frames(44_100), 441_000);
}

#[test]
fn a_value_that_is_not_finite_is_refused_naming_where_it_is() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut samples = vec![0.25; 2 * 4_800];
        samples[777] = bad;
        assert_eq!(
            OwnedPcm::new(48_000, 2, samples),
            Err(PcmError::NotFinite { index: 777 })
        );
    }
}

#[test]
fn clones_share_one_decoded_buffer() {
    let pcm = fixture_pcm(SINE_48000_MONO);
    let copy = pcm.clone();
    let decoded_again = fixture_pcm(SINE_48000_MONO);

    assert!(copy.shares_samples_with(&pcm));
    assert!(!decoded_again.shares_samples_with(&pcm));
    assert_eq!(decoded_again, pcm);
}
