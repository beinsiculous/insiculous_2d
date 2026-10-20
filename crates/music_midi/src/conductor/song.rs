//! A song ready for the conductor: its sidecar, its MIDI data, its SoundFonts parsed from
//! bytes already in memory, and what the conductor needs to know before loading it — the
//! background score, the instruments' layering and the background's peak voice count.
//! Everything costly happens here, on the thread that builds the song, so
//! [`Conductor::load`](super::Conductor::load) only wires it up.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer};

use crate::error::ConductorError;
use crate::sequencer::{BackgroundScore, ChannelMessage};
use crate::song_data::SongData;
use crate::song_file::{InstrumentSource, PresetKey, SongFile};

const HIGHEST_KEY: i32 = 127;
const HIGHEST_VELOCITY: i32 = 127;
const PERCUSSION_BANK: i32 = 128;
const BANK_SELECT: u8 = 0;

/// The most voices the background score starts at once, and the first tick it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackgroundPeak {
    pub voices: usize,
    pub tick: u64,
}

/// A song in memory, ready to load. See the module docs.
pub struct Song {
    file: SongFile,
    data: SongData,
    sound_fonts: BTreeMap<String, Arc<SoundFont>>,
    background: Arc<BackgroundScore>,
    instrument_layering: BTreeMap<PresetKey, usize>,
    background_peak: BackgroundPeak,
}

impl Song {
    /// Parse every SoundFont the song names from `sound_fonts` (path → bytes, as the
    /// sidecar names them) and prepare the song. Each part's track is a trigger track and
    /// is left out of the background. Validate the sidecar first: an unresolvable part is
    /// refused here, not explained.
    pub fn new(
        file: SongFile,
        data: SongData,
        sound_fonts: &[(&str, &[u8])],
    ) -> Result<Self, ConductorError> {
        let mut needed: BTreeSet<&str> = BTreeSet::from([file.soundfont.as_str()]);
        for instrument in &file.instruments {
            if let InstrumentSource::SoundFont { path, .. } = &instrument.source {
                needed.insert(path);
            }
        }
        let mut parsed = BTreeMap::new();
        for path in needed {
            let bytes = sound_fonts
                .iter()
                .find(|(supplied, _)| *supplied == path)
                .map(|(_, bytes)| *bytes)
                .ok_or_else(|| ConductorError::SoundFontNotSupplied {
                    path: path.to_string(),
                })?;
            let sound_font = SoundFont::new(&mut Cursor::new(bytes)).map_err(|error| {
                ConductorError::SoundFont {
                    path: path.to_string(),
                    reason: error.to_string(),
                }
            })?;
            parsed.insert(path.to_string(), Arc::new(sound_font));
        }

        let mut trigger_tracks = BTreeSet::new();
        for part in &file.parts {
            trigger_tracks.insert(
                part.resolve_track(&data)
                    .map_err(ConductorError::SongDoesNotFit)?,
            );
        }
        let background = Arc::new(BackgroundScore::new(&data, &trigger_tracks));

        let mut instrument_layering = BTreeMap::new();
        for instrument in &file.instruments {
            let InstrumentSource::SoundFont { path, bank, preset } = &instrument.source else {
                continue;
            };
            let key = PresetKey {
                soundfont: path.clone(),
                bank: *bank,
                preset: *preset,
            };
            let Some(sound_font) = parsed.get(path) else {
                continue;
            };
            if let Some(index) = find_preset(sound_font, i32::from(*bank), i32::from(*preset)) {
                instrument_layering.insert(key, layering(sound_font, index));
            }
        }
        let background_font =
            parsed
                .get(&file.soundfont)
                .ok_or_else(|| ConductorError::SoundFontNotSupplied {
                    path: file.soundfont.clone(),
                })?;
        let background_peak = background_peak(&background, background_font);
        Ok(Self {
            file,
            data,
            sound_fonts: parsed,
            background,
            instrument_layering,
            background_peak,
        })
    }

    pub fn file(&self) -> &SongFile {
        &self.file
    }

    pub fn data(&self) -> &SongData {
        &self.data
    }

    /// A SoundFont the song names, by the path the sidecar gives it.
    pub fn sound_font(&self, path: &str) -> Option<&Arc<SoundFont>> {
        self.sound_fonts.get(path)
    }

    pub fn background(&self) -> &Arc<BackgroundScore> {
        &self.background
    }

    /// The most voices one note of an instrument's preset starts, or `None` when its
    /// SoundFont lacks the preset.
    pub fn instrument_layering(&self, preset: &PresetKey) -> Option<usize> {
        self.instrument_layering.get(preset).copied()
    }

    pub fn background_peak(&self) -> BackgroundPeak {
        self.background_peak
    }
}

/// The preset at `bank:preset`, exactly.
fn find_preset(sound_font: &SoundFont, bank: i32, preset: i32) -> Option<usize> {
    sound_font.get_presets().iter().position(|candidate| {
        candidate.get_bank_number() == bank && candidate.get_patch_number() == preset
    })
}

/// The preset a synthesizer channel plays for `bank:program`, falling back as rustysynth
/// does: the same program in bank 0 for a melodic bank, the standard kit (128:0) for a
/// percussion bank, then the preset with the lowest bank and program.
fn resolve_preset(sound_font: &SoundFont, bank: i32, program: i32) -> Option<usize> {
    find_preset(sound_font, bank, program)
        .or_else(|| {
            if bank < PERCUSSION_BANK {
                find_preset(sound_font, 0, program)
            } else {
                find_preset(sound_font, PERCUSSION_BANK, 0)
            }
        })
        .or_else(|| {
            sound_font
                .get_presets()
                .iter()
                .enumerate()
                .min_by_key(|(_, preset)| (preset.get_bank_number(), preset.get_patch_number()))
                .map(|(index, _)| index)
        })
}

/// The most voices one note-on of a preset starts: the most (preset region, instrument
/// region) pairs containing any one key and velocity. Regions are rectangles over key and
/// velocity, so the maximum falls on a velocity where some region begins; only those
/// velocities are tried.
fn layering(sound_font: &SoundFont, preset_index: usize) -> usize {
    let preset = &sound_font.get_presets()[preset_index];
    let instruments = sound_font.get_instruments();
    let mut velocities = BTreeSet::from([1]);
    for preset_region in preset.get_regions() {
        velocities.insert(preset_region.get_velocity_range_start());
        if let Some(instrument) = instruments.get(preset_region.get_instrument_id()) {
            for instrument_region in instrument.get_regions() {
                velocities.insert(instrument_region.get_velocity_range_start());
            }
        }
    }
    let mut most = 0;
    for key in 0..=HIGHEST_KEY {
        for &velocity in velocities
            .iter()
            .filter(|velocity| (1..=HIGHEST_VELOCITY).contains(*velocity))
        {
            let voices: usize = preset
                .get_regions()
                .iter()
                .filter(|preset_region| preset_region.contains(key, velocity))
                .filter_map(|preset_region| instruments.get(preset_region.get_instrument_id()))
                .map(|instrument| {
                    instrument
                        .get_regions()
                        .iter()
                        .filter(|region| region.contains(key, velocity))
                        .count()
                })
                .sum();
            most = most.max(voices);
        }
    }
    most
}

/// The background score's peak of simultaneous voices: each sounding note counts the
/// layering of the preset its channel had selected when it started. Tails are not
/// counted; the pool's headroom is for them.
fn background_peak(score: &BackgroundScore, sound_font: &SoundFont) -> BackgroundPeak {
    let mut banks = [0i32; 16];
    banks[Synthesizer::PERCUSSION_CHANNEL] = PERCUSSION_BANK;
    let mut programs = [0i32; 16];
    let mut layering_by_preset: BTreeMap<usize, usize> = BTreeMap::new();
    let mut voices_of_note: BTreeMap<usize, usize> = BTreeMap::new();
    let mut sounding = 0usize;
    let mut peak = BackgroundPeak::default();
    for (index, event) in score.events().iter().enumerate() {
        let channel = usize::from(event.channel) % 16;
        match event.message {
            ChannelMessage::Controller {
                controller: BANK_SELECT,
                value,
            } => {
                let percussion_offset = if channel == Synthesizer::PERCUSSION_CHANNEL {
                    PERCUSSION_BANK
                } else {
                    0
                };
                banks[channel] = i32::from(value) + percussion_offset;
            }
            ChannelMessage::Program { program } => programs[channel] = i32::from(program),
            ChannelMessage::NoteOn { .. } => {
                let voices = resolve_preset(sound_font, banks[channel], programs[channel]).map_or(
                    0,
                    |preset| {
                        *layering_by_preset
                            .entry(preset)
                            .or_insert_with(|| layering(sound_font, preset))
                    },
                );
                voices_of_note.insert(index, voices);
                sounding += voices;
                if sounding > peak.voices {
                    peak = BackgroundPeak {
                        voices: sounding,
                        tick: event.tick,
                    };
                }
            }
            ChannelMessage::NoteOff { .. } => {
                sounding -= voices_of_note.remove(&event.note_on_index()).unwrap_or(0);
            }
            ChannelMessage::Controller { .. } | ChannelMessage::PitchBend { .. } => {}
        }
    }
    peak
}
