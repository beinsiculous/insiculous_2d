//! The SoundFont: loading it from bytes, and the two figures the voice budget is built
//! from, read from the bank itself rather than measured by ear.

use std::io::Cursor;
use std::sync::Arc;

use rustysynth::{SoundFont, SoundFontError};

/// Parse a SoundFont from bytes already in memory (the VFS holds every asset as bytes on
/// both targets, so nothing here touches a file system).
pub fn load_sound_font(bytes: &[u8]) -> Result<Arc<SoundFont>, SoundFontError> {
    let mut reader = Cursor::new(bytes);
    SoundFont::new(&mut reader).map(Arc::new)
}

/// What one preset costs the voice pool.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetProfile {
    pub name: String,
    pub bank: i32,
    pub program: i32,
    /// The most voices one note-on can start: the largest number of (preset region,
    /// instrument region) pairs that match any single key and velocity.
    pub layering_factor: usize,
    /// The longest release, in seconds, of any region pair the preset can play. A voice
    /// ends no later than this after its note-off.
    pub longest_release_seconds: f32,
}

/// The profile of the preset at `bank:program`, or `None` if the bank has no such preset.
pub fn preset_profile(sound_font: &SoundFont, bank: i32, program: i32) -> Option<PresetProfile> {
    let preset = sound_font
        .get_presets()
        .iter()
        .find(|preset| preset.get_bank_number() == bank && preset.get_patch_number() == program)?;
    let instruments = sound_font.get_instruments();
    let mut layering_factor = 0;
    let mut longest_release_seconds = 0.0_f32;
    for key in 0..128 {
        for velocity in 1..128 {
            let mut voices_for_this_note = 0;
            for preset_region in preset.get_regions() {
                if !preset_region.contains(key, velocity) {
                    continue;
                }
                let Some(instrument) = instruments.get(preset_region.get_instrument_id()) else {
                    continue;
                };
                for instrument_region in instrument.get_regions() {
                    if instrument_region.contains(key, velocity) {
                        voices_for_this_note += 1;
                        // A preset region's generators add to its instrument region's in
                        // timecents, so the seconds multiply.
                        let release_seconds = instrument_region.get_release_volume_envelope()
                            * preset_region.get_release_volume_envelope();
                        longest_release_seconds = longest_release_seconds.max(release_seconds);
                    }
                }
            }
            layering_factor = layering_factor.max(voices_for_this_note);
        }
    }
    Some(PresetProfile {
        name: preset.get_name().to_string(),
        bank,
        program,
        layering_factor,
        longest_release_seconds,
    })
}
