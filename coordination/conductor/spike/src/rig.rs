//! The spike's assets and the mixer they build: decision 2's worst case, the same on
//! native and in the browser.

use std::path::Path;
use std::sync::Arc;

use rustysynth::SoundFont;

use crate::bank::load_sound_font;
use crate::instrument_slot::{InstrumentSlot, SlotCapacity};
use crate::mixer::Mixer;
use crate::sample_slot::{decode_sample, OwnedPcm, SampleSlot, TriggerMode, PROPOSED_LIMITS};
use crate::score::{parse_score, BackgroundScore, Score};

/// Where each asset sits under the asset base (`prepare_assets.sh` puts them there).
pub const BANK_PATH: &str = "soundfonts/cast.sf2";
pub const BACKGROUND_SCORE_PATH: &str = "music/dansenapolitaine-type0.mid";
pub const PHRASE_PATH: &str = "music/bach-invention-02.mid";
pub const SAMPLE_PATH: &str = "sounds/sine_48000_mono.wav";

/// The proposed capacity of every instrument slot.
pub const CAPACITY: SlotCapacity = SlotCapacity { sounding_cap: 24, maximum_polyphony: 128 };
/// The proposed live-voice cap of the sample slot.
pub const SAMPLE_LIVE_VOICE_CAP: usize = 16;
/// The sample's root: the fixture WAVs are a middle C.
pub const SAMPLE_ROOT_KEY: i32 = 60;

/// The eight instrument slots of the worst case, as (bank, program, name). Piano and
/// strings are the long-release presets the stress test leans on.
pub const SLOT_PRESETS: [(i32, i32, &str); 8] = [
    (0, 0, "piano"),
    (0, 48, "strings"),
    (0, 56, "trumpet"),
    (0, 57, "trombone"),
    (0, 13, "xylophone"),
    (0, 9, "glockenspiel"),
    (0, 45, "pizzicato"),
    (0, 68, "oboe"),
];

/// The background score's loop: bars 1–16 of the 2/4 Napolitan Dance (384 ticks a
/// quarter), so a two-minute run wraps several times.
pub const BACKGROUND_LOOP_TICKS: (u64, u64) = (0, 16 * 2 * 384);

/// Everything decoded once, before any audio runs.
pub struct SpikeAssets {
    pub bank: Arc<SoundFont>,
    pub background: Score,
    pub phrase: Score,
    pub sample: OwnedPcm,
    /// Bytes read for each asset, in the order above.
    pub sizes_bytes: [usize; 4],
}

/// Read and decode every asset under `asset_base` through the engine's VFS (the
/// prefetched map on the web, the file system natively).
pub fn load_assets(asset_base: &Path) -> Result<SpikeAssets, String> {
    let read = |relative: &str| {
        common::vfs::read(&asset_base.join(relative)).map_err(|error| format!("{relative}: {error}"))
    };
    let bank_bytes = read(BANK_PATH)?;
    let background_bytes = read(BACKGROUND_SCORE_PATH)?;
    let phrase_bytes = read(PHRASE_PATH)?;
    let sample_bytes = read(SAMPLE_PATH)?;
    let sizes_bytes = [bank_bytes.len(), background_bytes.len(), phrase_bytes.len(), sample_bytes.len()];
    Ok(SpikeAssets {
        bank: load_sound_font(&bank_bytes).map_err(|error| format!("{BANK_PATH}: {error}"))?,
        background: parse_score(&background_bytes)?,
        phrase: parse_score(&phrase_bytes)?,
        sample: decode_sample(sample_bytes, PROPOSED_LIMITS).map_err(|error| format!("{SAMPLE_PATH}: {error}"))?,
        sizes_bytes,
    })
}

/// The worst-case mixer: the background score looping, eight instrument slots, and a
/// sample slot in `sample_mode`.
pub fn worst_case_mixer(assets: &SpikeAssets, sample_rate: u32, sample_mode: TriggerMode) -> Result<Mixer, String> {
    let mut background = BackgroundScore::new(&assets.bank, assets.background.clone(), sample_rate, CAPACITY.maximum_polyphony)
        .map_err(|error| error.to_string())?;
    background.set_loop(Some(BACKGROUND_LOOP_TICKS));
    let mut slots = Vec::with_capacity(SLOT_PRESETS.len());
    for (bank, program, _) in SLOT_PRESETS {
        slots.push(
            InstrumentSlot::new(&assets.bank, sample_rate as i32, CAPACITY, bank, program)
                .map_err(|error| error.to_string())?,
        );
    }
    let samples = SampleSlot::new(assets.sample.clone(), SAMPLE_ROOT_KEY, sample_mode, sample_rate, SAMPLE_LIVE_VOICE_CAP);
    Ok(Mixer::new(Some(background), slots, Some(samples)))
}
