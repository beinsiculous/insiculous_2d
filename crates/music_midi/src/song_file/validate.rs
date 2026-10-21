//! Song sidecar validation: the schema checks [`parse_song_file`](super::parse_song_file)
//! runs on the text alone, and the checks [`SongFile::validate`] adds against the MIDI
//! data and the loaded resources.
//!
//! Two capacities are checked and kept apart. *Slot capacity* — every instrument takes
//! one of [`INSTRUMENT_SLOT_CAPACITY`] slots, each with its own synthesizer, so a program
//! change on one never retimbres another. *Voice capacity* — a slot holds at most
//! [`SOUNDING_NOTE_CAP`] sounding notes, so a preset layering more voices a note than
//! [`SYNTHESIZER_POLYPHONY`] can hold at that cap is refused. The background score plays
//! on a synthesizer of its own with all sixteen MIDI channels, which any Standard MIDI
//! File fits by construction, so it adds nothing to either count.

use std::collections::BTreeSet;

use super::{Binding, InstrumentSource, PresetKey, SongFile};
use crate::error::{Collection, SongFileProblem};
use crate::song_data::SongData;
use crate::{
    INSTRUMENT_SLOT_CAPACITY, SAMPLE_SECONDS_MAXIMUM, SAMPLE_SECONDS_MINIMUM,
    SAMPLE_SEMITONE_RANGE, SOUNDING_NOTE_CAP, SYNTHESIZER_POLYPHONY,
};

const HIGHEST_NOTE: u8 = 127;
const HIGHEST_PRESET: u8 = 127;
/// SoundFont banks run 0–127 for melodic presets; 128 is General MIDI percussion.
const HIGHEST_BANK: u16 = 128;

impl SongFile {
    /// Everything checkable from the file alone: the version, the background SoundFont's
    /// path, empty or repeated names, a binding naming nothing, an instrument's numbers and
    /// the instrument cap. Pure, so a composer runs it on the file an edit would produce
    /// and never writes a sidecar [`parse_song_file`](super::parse_song_file) refuses.
    pub fn check_schema(&self) -> Result<(), SongFileProblem> {
        if self.version != super::SONG_FILE_VERSION {
            return Err(SongFileProblem::UnsupportedVersion {
                found: self.version,
                supported: super::SONG_FILE_VERSION,
            });
        }
        if self.soundfont.is_empty() {
            return Err(SongFileProblem::EmptyPath {
                entry: "soundfont".to_string(),
            });
        }
        check_names(
            Collection::Parts,
            self.parts.iter().map(|part| part.name.as_str()),
        )?;
        check_names(
            Collection::Instruments,
            self.instruments
                .iter()
                .map(|instrument| instrument.name.as_str()),
        )?;
        check_names(
            Collection::Bindings,
            self.bindings.iter().map(|binding| binding.action.as_str()),
        )?;
        for binding in &self.bindings {
            if !self.parts.iter().any(|part| part.name == binding.part) {
                return Err(SongFileProblem::UnknownPart {
                    action: binding.action.clone(),
                    part: binding.part.clone(),
                });
            }
            if !self
                .instruments
                .iter()
                .any(|instrument| instrument.name == binding.instrument)
            {
                return Err(SongFileProblem::UnknownInstrument {
                    action: binding.action.clone(),
                    instrument: binding.instrument.clone(),
                });
            }
        }
        for instrument in &self.instruments {
            check_instrument_shape(&instrument.name, &instrument.source)?;
        }
        if self.instruments.len() > INSTRUMENT_SLOT_CAPACITY {
            return Err(SongFileProblem::TooManyInstruments {
                count: self.instruments.len(),
                capacity: INSTRUMENT_SLOT_CAPACITY,
            });
        }
        Ok(())
    }

    /// Everything that needs the MIDI data or the resources.
    pub(super) fn check_against(&self, song: &SongData) -> Result<(), SongFileProblem> {
        if !self.resources.soundfonts.contains(&self.soundfont) {
            return Err(SongFileProblem::SoundFontNotSupplied {
                path: self.soundfont.clone(),
            });
        }
        for part in &self.parts {
            let track = part.resolve_track(song)?;
            if song.tracks()[track].note_count() == 0 {
                return Err(SongFileProblem::EmptyTriggerPart {
                    part: part.name.clone(),
                    track,
                });
            }
        }
        for binding in &self.bindings {
            self.check_sample_range(song, binding)?;
        }
        for instrument in &self.instruments {
            self.check_resources(&instrument.name, &instrument.source)?;
        }
        Ok(())
    }

    /// Every key of the bound part must lie within the sample's pitch range.
    fn check_sample_range(
        &self,
        song: &SongData,
        binding: &Binding,
    ) -> Result<(), SongFileProblem> {
        let instrument = self
            .instruments
            .iter()
            .find(|instrument| instrument.name == binding.instrument)
            .ok_or_else(|| SongFileProblem::UnknownInstrument {
                action: binding.action.clone(),
                instrument: binding.instrument.clone(),
            })?;
        let InstrumentSource::Sample { root_note, .. } = instrument.source else {
            return Ok(());
        };
        let part = self
            .parts
            .iter()
            .find(|part| part.name == binding.part)
            .ok_or_else(|| SongFileProblem::UnknownPart {
                action: binding.action.clone(),
                part: binding.part.clone(),
            })?;
        let track = &song.tracks()[part.resolve_track(song)?];
        for unit in track.units() {
            for note in &unit.notes {
                if note.key.abs_diff(root_note) > SAMPLE_SEMITONE_RANGE {
                    return Err(SongFileProblem::NoteOutsideSampleRange {
                        action: binding.action.clone(),
                        instrument: instrument.name.clone(),
                        key: note.key,
                        tick: unit.tick,
                        root_note,
                        range: SAMPLE_SEMITONE_RANGE,
                    });
                }
            }
        }
        Ok(())
    }

    fn check_resources(
        &self,
        instrument: &str,
        source: &InstrumentSource,
    ) -> Result<(), SongFileProblem> {
        match source {
            InstrumentSource::SoundFont { path, bank, preset } => {
                let key = PresetKey {
                    soundfont: path.clone(),
                    bank: *bank,
                    preset: *preset,
                };
                let Some(&layering) = self.resources.preset_layering.get(&key) else {
                    return Err(SongFileProblem::PresetNotInSoundFont {
                        instrument: instrument.to_string(),
                        soundfont: path.clone(),
                        bank: *bank,
                        preset: *preset,
                    });
                };
                let needed = layering.saturating_mul(SOUNDING_NOTE_CAP);
                if needed > SYNTHESIZER_POLYPHONY {
                    return Err(SongFileProblem::LayeringExceedsPolyphony {
                        instrument: instrument.to_string(),
                        layering,
                        sounding_cap: SOUNDING_NOTE_CAP,
                        needed,
                        polyphony: SYNTHESIZER_POLYPHONY,
                    });
                }
            }
            InstrumentSource::Sample { path, .. } => {
                let Some(&seconds) = self.resources.sample_seconds.get(path) else {
                    return Err(SongFileProblem::SampleNotSupplied {
                        instrument: instrument.to_string(),
                        path: path.clone(),
                    });
                };
                if !(SAMPLE_SECONDS_MINIMUM..=SAMPLE_SECONDS_MAXIMUM).contains(&seconds) {
                    return Err(SongFileProblem::SampleLengthOutOfRange {
                        instrument: instrument.to_string(),
                        seconds,
                        minimum: SAMPLE_SECONDS_MINIMUM,
                        maximum: SAMPLE_SECONDS_MAXIMUM,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Names in one collection are non-empty and unique.
fn check_names<'a>(
    collection: Collection,
    names: impl Iterator<Item = &'a str>,
) -> Result<(), SongFileProblem> {
    let mut seen = BTreeSet::new();
    for (index, name) in names.enumerate() {
        if name.is_empty() {
            return Err(SongFileProblem::EmptyName { collection, index });
        }
        if !seen.insert(name) {
            return Err(SongFileProblem::DuplicateName {
                collection,
                name: name.to_string(),
            });
        }
    }
    Ok(())
}

fn check_instrument_shape(
    instrument: &str,
    source: &InstrumentSource,
) -> Result<(), SongFileProblem> {
    let empty_path = || SongFileProblem::EmptyPath {
        entry: format!("instruments['{instrument}']"),
    };
    match source {
        InstrumentSource::SoundFont { path, bank, preset } => {
            if path.is_empty() {
                return Err(empty_path());
            }
            if *bank > HIGHEST_BANK || *preset > HIGHEST_PRESET {
                return Err(SongFileProblem::PresetOutOfRange {
                    instrument: instrument.to_string(),
                    bank: *bank,
                    preset: *preset,
                });
            }
        }
        InstrumentSource::Sample {
            path,
            root_note,
            gain,
            ..
        } => {
            if path.is_empty() {
                return Err(empty_path());
            }
            if *root_note > HIGHEST_NOTE {
                return Err(SongFileProblem::RootNoteOutOfRange {
                    instrument: instrument.to_string(),
                    root_note: *root_note,
                });
            }
            if !gain.is_finite() || *gain < 0.0 {
                return Err(SongFileProblem::InvalidGain {
                    instrument: instrument.to_string(),
                    gain: *gain,
                });
            }
        }
    }
    Ok(())
}
