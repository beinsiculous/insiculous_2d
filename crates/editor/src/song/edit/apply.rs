//! Applying a [`SongEdit`]: each variant checked and carried out on the song's sidecar,
//! returning the exact changes that undo and redo it.

use music_midi::{Instrument, Part, SongData, SongFile};

use super::{Applied, SongEdit, SongEditError};
use crate::song::change::Change;

impl SongEdit {
    /// Check and apply the edit to `file`, returning its undo and redo; `data` is the score
    /// a part's track name is resolved against. A refused edit changes nothing.
    pub(in crate::song) fn apply_to(
        &self,
        data: &SongData,
        file: &mut SongFile,
    ) -> Result<Applied, SongEditError> {
        match self {
            SongEdit::Batch(edits) => apply_batch(edits, data, file),
            sidecar_edit => {
                let parts_before = file.parts.clone();
                edit_file(file, |candidate| {
                    sidecar_edit.apply_to_file(candidate)?;
                    refuse_new_ambiguous_parts(&parts_before, candidate, data)
                })
            }
        }
    }

    /// The edit, on a candidate copy of the file.
    fn apply_to_file(&self, file: &mut SongFile) -> Result<(), SongEditError> {
        use SongEdit as E;
        match self {
            E::SetLoop { looping } => file.looping = *looping,
            E::SetProvenance { provenance } => file.provenance = Some(provenance.clone()),
            E::SetSoundFont { path } => file.soundfont = path.clone(),
            E::SetQuantize { part, quantize } => part_named(file, part)?.quantize = *quantize,
            E::SetPartMode { part, mode } => part_named(file, part)?.mode = *mode,
            E::AddInstrument { instrument } => file.instruments.push(instrument.clone()),
            E::SetInstrument { name, instrument } => {
                *instrument_named(file, name)? = instrument.clone()
            }
            E::RemoveInstrument { name } => {
                let index = file
                    .instruments
                    .iter()
                    .position(|instrument| instrument.name == *name)
                    .ok_or_else(|| SongEditError::UnknownInstrument(name.clone()))?;
                file.instruments.remove(index);
            }
            E::SetBinding { binding } => {
                match file
                    .bindings
                    .iter_mut()
                    .find(|existing| existing.action == binding.action)
                {
                    Some(existing) => *existing = binding.clone(),
                    None => file.bindings.push(binding.clone()),
                }
            }
            E::RemoveBinding { action } => {
                let index = file
                    .bindings
                    .iter()
                    .position(|binding| binding.action == *action)
                    .ok_or_else(|| SongEditError::UnknownBinding(action.clone()))?;
                file.bindings.remove(index);
            }
            E::AddPart { part } => file.parts.push(part.clone()),
            E::RemovePart { name } => {
                let index = file
                    .parts
                    .iter()
                    .position(|part| part.name == *name)
                    .ok_or_else(|| SongEditError::UnknownPart(name.clone()))?;
                file.parts.remove(index);
            }
            E::Batch(_) => log::error!("a Batch is applied member by member, never to a file whole"),
        }
        Ok(())
    }
}

/// A part that this edit added or re-pointed must select one track: `check_schema` sees no
/// tracks, so a part naming a name two of the score's tracks carry is refused here. A
/// candidate part carrying a before-part's name and selector is that part untouched (part
/// names are unique, which `check_schema` holds) and is not re-judged; a part removed and
/// re-added in one batch has no such twin and is judged as new.
fn refuse_new_ambiguous_parts(
    parts_before: &[Part],
    candidate: &SongFile,
    data: &SongData,
) -> Result<(), SongEditError> {
    for part in &candidate.parts {
        let music_midi::TrackSelector::Name(name) = &part.track else {
            continue;
        };
        let untouched = parts_before
            .iter()
            .any(|before| before.name == part.name && before.track == part.track);
        if untouched {
            continue;
        }
        let carriers = data.tracks().iter().filter(|track| track.name() == Some(name.as_str())).count();
        if carriers > 1 {
            return Err(SongEditError::TrackNameAmbiguous {
                part: part.name.clone(),
                name: name.clone(),
            });
        }
    }
    Ok(())
}

/// A sidecar edit made on a copy, kept only when the copy passes the schema check.
fn edit_file(
    file: &mut SongFile,
    edit: impl FnOnce(&mut SongFile) -> Result<(), SongEditError>,
) -> Result<Applied, SongEditError> {
    let mut candidate = file.clone();
    edit(&mut candidate)?;
    candidate.check_schema()?;
    let before = std::mem::replace(file, candidate);
    Ok(Applied {
        undo: Change::File(Box::new(before)),
        redo: Change::File(Box::new(file.clone())),
    })
}

/// Every member applied to a copy of the file; the copy replaces the file only when all of
/// them land.
fn apply_batch(
    edits: &[SongEdit],
    data: &SongData,
    file: &mut SongFile,
) -> Result<Applied, SongEditError> {
    if edits.is_empty() {
        return Err(SongEditError::EmptyBatch);
    }
    let mut scratch_file = file.clone();
    let mut undos = Vec::with_capacity(edits.len());
    let mut redos = Vec::with_capacity(edits.len());
    for edit in edits {
        let applied = edit.apply_to(data, &mut scratch_file)?;
        undos.push(applied.undo);
        redos.push(applied.redo);
    }
    *file = scratch_file;
    undos.reverse();
    Ok(Applied {
        undo: Change::Several(undos),
        redo: Change::Several(redos),
    })
}

fn part_named<'a>(file: &'a mut SongFile, name: &str) -> Result<&'a mut Part, SongEditError> {
    file.parts
        .iter_mut()
        .find(|part| part.name == name)
        .ok_or_else(|| SongEditError::UnknownPart(name.to_string()))
}

fn instrument_named<'a>(
    file: &'a mut SongFile,
    name: &str,
) -> Result<&'a mut Instrument, SongEditError> {
    file.instruments
        .iter_mut()
        .find(|instrument| instrument.name == name)
        .ok_or_else(|| SongEditError::UnknownInstrument(name.to_string()))
}
