//! Applying a [`SongEdit`]: each variant checked and carried out on the song's data or
//! its sidecar, returning the exact changes that undo and redo it.

use music_midi::{
    EditError, Instrument, NewNote, NewTrack, NoteId, NoteImage, Part, SongData, SongFile,
    TempoChange, TimeSignature,
};

use super::{note_ids, Applied, SongEdit, SongEditError};
use crate::song::change::Change;

impl SongEdit {
    /// Check and apply the edit to `data` and `file`, returning its undo and redo. A
    /// refused edit changes neither.
    pub(in crate::song) fn apply_to(
        &self,
        data: &mut SongData,
        file: &mut SongFile,
    ) -> Result<Applied, SongEditError> {
        use SongEdit as E;
        match self {
            E::AddNote {
                track,
                tick,
                key,
                velocity,
                duration_ticks,
            } => {
                let new_note = NewNote {
                    tick: *tick,
                    key: *key,
                    velocity: *velocity,
                    duration_ticks: *duration_ticks,
                };
                let id = data.add_note(*track, new_note)?;
                Ok(Applied {
                    undo: Change::RemoveNotes(vec![id]),
                    redo: Change::RestoreNotes(images(data, &[id])),
                })
            }
            E::DeleteNotes { notes } => {
                let ids = note_ids(data, notes)?;
                let before = data.remove_notes(&ids)?;
                Ok(Applied {
                    undo: Change::RestoreNotes(before),
                    redo: Change::RemoveNotes(ids),
                })
            }
            E::MoveNotes { notes, tick_delta } => {
                edit_notes(data, notes, |data, ids| data.move_notes(ids, *tick_delta))
            }
            E::ResizeNotes {
                notes,
                duration_ticks,
            } => edit_notes(data, notes, |data, ids| {
                data.resize_notes(ids, *duration_ticks)
            }),
            E::SetVelocity { notes, velocity } => {
                edit_notes(data, notes, |data, ids| data.set_velocity(ids, *velocity))
            }
            E::SetPitch { notes, key } => {
                edit_notes(data, notes, |data, ids| data.set_pitch(ids, *key))
            }
            E::AssignPart { notes, part } => {
                let part = file
                    .parts
                    .iter()
                    .find(|candidate| candidate.name == *part)
                    .ok_or_else(|| SongEditError::UnknownPart(part.clone()))?;
                let track = part.resolve_track(data)?;
                edit_notes(data, notes, |data, ids| data.assign_part(ids, track))
            }
            E::SetTempo {
                tick,
                microseconds_per_quarter,
            } => {
                let after = microseconds_per_quarter.map(|microseconds| TempoChange {
                    tick: *tick,
                    microseconds_per_quarter: microseconds,
                });
                let before = match after {
                    Some(change) => data.set_tempo(change)?,
                    None => Some(
                        data.remove_tempo(*tick)
                            .ok_or(SongEditError::NoTempoChange(*tick))?,
                    ),
                };
                Ok(Applied {
                    undo: Change::Tempo {
                        tick: *tick,
                        change: before,
                    },
                    redo: Change::Tempo {
                        tick: *tick,
                        change: after,
                    },
                })
            }
            E::SetTimeSignature { tick, meter } => {
                let after = meter
                    .map(|meter| TimeSignature::new(*tick, meter.numerator, meter.denominator));
                let before = match after {
                    Some(signature) => data.set_time_signature(signature)?,
                    None => Some(
                        data.remove_time_signature(*tick)
                            .ok_or(SongEditError::NoTimeSignature(*tick))?,
                    ),
                };
                Ok(Applied {
                    undo: Change::TimeSignature {
                        tick: *tick,
                        signature: before,
                    },
                    redo: Change::TimeSignature {
                        tick: *tick,
                        signature: after,
                    },
                })
            }
            E::AddTrack {
                name,
                channel,
                program,
            } => {
                // A part selecting a name resolves it only while one track carries it: a
                // second would break a part that worked, through an accepted edit.
                let carried = data.tracks().iter().any(|track| track.name() == Some(name.as_str()));
                let selected = file.parts.iter().any(|part| {
                    matches!(&part.track, music_midi::TrackSelector::Name(selected) if selected == name)
                });
                if carried && selected {
                    return Err(SongEditError::TrackNameSelected(name.clone()));
                }
                let channel = match channel {
                    Some(number @ 1..=16) => Some(number - 1),
                    Some(number) => return Err(SongEditError::ChannelOutOfRange(*number)),
                    None => None,
                };
                let added = data.add_track(NewTrack {
                    name: name.clone(),
                    channel,
                    program: *program,
                })?;
                Ok(Applied {
                    undo: Change::RemoveAddedTrack(added.track),
                    redo: Change::AddTrack(NewTrack {
                        name: name.clone(),
                        channel: Some(added.channel),
                        program: *program,
                    }),
                })
            }
            E::Batch(edits) => apply_batch(edits, data, file),
            sidecar_edit => {
                let parts_before = file.parts.clone();
                edit_file(file, |candidate| {
                    sidecar_edit.apply_to_file(candidate)?;
                    refuse_new_ambiguous_parts(&parts_before, candidate, data)
                })
            }
        }
    }

    /// The sidecar edits, on a candidate copy of the file.
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
            E::AddNote { .. }
            | E::DeleteNotes { .. }
            | E::MoveNotes { .. }
            | E::ResizeNotes { .. }
            | E::SetVelocity { .. }
            | E::SetPitch { .. }
            | E::AssignPart { .. }
            | E::SetTempo { .. }
            | E::SetTimeSignature { .. }
            | E::AddTrack { .. }
            | E::Batch(_) => log::error!("{} is not a sidecar edit", self.name()),
        }
        Ok(())
    }
}

/// The notes' images as they stand now.
fn images(data: &SongData, ids: &[NoteId]) -> Vec<NoteImage> {
    ids.iter().filter_map(|id| data.note_image(*id)).collect()
}

/// A note edit: its before-images undo it, the images after it redo it.
fn edit_notes(
    data: &mut SongData,
    numbers: &[u64],
    edit: impl FnOnce(&mut SongData, &[NoteId]) -> Result<Vec<NoteImage>, EditError>,
) -> Result<Applied, SongEditError> {
    let ids = note_ids(data, numbers)?;
    let before = edit(data, &ids)?;
    Ok(Applied {
        undo: Change::RestoreNotes(before),
        redo: Change::RestoreNotes(images(data, &ids)),
    })
}

/// A part that this edit added or re-pointed must select one track: `check_schema` sees no
/// tracks, so a part naming a name two tracks carry — a duplicate track added before the part,
/// the order `AddTrack`'s own guard cannot see — is refused here. A candidate part carrying a
/// before-part's name and selector is that part untouched (part names are unique, which
/// `check_schema` holds) and is not re-judged; a part removed and re-added in one batch has no
/// such twin and is judged as new.
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

/// Every member applied to copies; the copies replace the song only when all of them land.
fn apply_batch(
    edits: &[SongEdit],
    data: &mut SongData,
    file: &mut SongFile,
) -> Result<Applied, SongEditError> {
    if edits.is_empty() {
        return Err(SongEditError::EmptyBatch);
    }
    let (mut scratch_data, mut scratch_file) = (data.clone(), file.clone());
    let mut undos = Vec::with_capacity(edits.len());
    let mut redos = Vec::with_capacity(edits.len());
    for edit in edits {
        let applied = edit.apply_to(&mut scratch_data, &mut scratch_file)?;
        undos.push(applied.undo);
        redos.push(applied.redo);
    }
    *data = scratch_data;
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
