//! The mutation helpers. Each checks every note it names before touching any, so a
//! refused edit changes nothing, and each returns what an inverse needs: the edited
//! notes' before-images (undo with [`SongData::restore_notes`]), the new note's id (undo
//! with [`SongData::remove_notes`]), the added track's place (undo with
//! [`SongData::remove_added_track`]), or the tempo or time signature that was replaced.

use std::collections::BTreeSet;

use midly::num::{u4, u7};
use midly::{MetaMessage, MidiMessage, TrackEventKind};

use super::{
    AddedTrack, MusicalUnit, NewNote, NewTrack, Note, NoteId, NoteImage, NoteLocation,
    RetainedEvent, SmfFormat, SongData, SongTrack,
};
use crate::error::EditError;
use crate::tempo_map::{TempoChange, TimeSignature};

const HIGHEST_KEY: u8 = 127;
const HIGHEST_VELOCITY: u8 = 127;
const HIGHEST_PROGRAM: u8 = 127;
const HIGHEST_CHANNEL: u8 = 15;
/// General MIDI's percussion channel (`channel 10` counted from 1): the synthesizer plays
/// it from the drum bank, so a track only lands there when it asks for it.
const PERCUSSION_CHANNEL: u8 = 9;
/// MIDI's largest delta time. A note ending past it could leave a gap between two events
/// that no Standard MIDI File can write, so the edit is refused rather than the export.
const LAST_WRITABLE_TICK: u64 = 0x0FFF_FFFF;

impl SongData {
    /// Add a note to selectable track `track`, written on that track's channel.
    pub fn add_note(&mut self, track: usize, note: NewNote) -> Result<NoteId, EditError> {
        self.check_writable_track(track)?;
        check_key(note.key)?;
        check_velocity(note.velocity)?;
        check_duration(note.duration_ticks)?;
        if !ends_by_last_writable_tick(note.tick, note.duration_ticks) {
            return Err(EditError::NewNotePastSongEnd {
                tick: note.tick,
                key: note.key,
                limit: LAST_WRITABLE_TICK,
            });
        }
        let id = NoteId(self.next_note_id);
        self.next_note_id += 1;
        let placed = Note {
            id,
            key: note.key,
            velocity: note.velocity,
            duration_ticks: note.duration_ticks,
        };
        self.place_note(track, note.tick, placed);
        Ok(id)
    }

    /// Remove notes. Their original MIDI events are left out of every later export.
    pub fn remove_notes(&mut self, ids: &[NoteId]) -> Result<Vec<NoteImage>, EditError> {
        let images = self.images_of(ids)?;
        for image in &images {
            self.take_note(image.id);
            self.original_notes.remove(&image.id);
        }
        Ok(images)
    }

    /// Shift notes in time by `tick_delta` (negative moves earlier).
    pub fn move_notes(
        &mut self,
        ids: &[NoteId],
        tick_delta: i64,
    ) -> Result<Vec<NoteImage>, EditError> {
        let images = self.images_of(ids)?;
        let mut destinations = Vec::with_capacity(images.len());
        for image in &images {
            let tick = image
                .tick
                .checked_add_signed(tick_delta)
                .filter(|tick| ends_by_last_writable_tick(*tick, image.duration_ticks))
                .ok_or(if tick_delta < 0 {
                    EditError::BeforeSongStart { note: image.id }
                } else {
                    EditError::PastSongEnd {
                        note: image.id,
                        limit: LAST_WRITABLE_TICK,
                    }
                })?;
            destinations.push(tick);
        }
        for (image, tick) in images.iter().zip(destinations) {
            self.relocate(image.id, image.track, tick)?;
        }
        Ok(images)
    }

    /// Give notes a new length.
    pub fn resize_notes(
        &mut self,
        ids: &[NoteId],
        duration_ticks: u64,
    ) -> Result<Vec<NoteImage>, EditError> {
        check_duration(duration_ticks)?;
        for image in self.images_of(ids)? {
            if !ends_by_last_writable_tick(image.tick, duration_ticks) {
                return Err(EditError::PastSongEnd {
                    note: image.id,
                    limit: LAST_WRITABLE_TICK,
                });
            }
        }
        self.modify_notes(ids, |note| note.duration_ticks = duration_ticks)
    }

    /// Give notes a new velocity.
    pub fn set_velocity(
        &mut self,
        ids: &[NoteId],
        velocity: u8,
    ) -> Result<Vec<NoteImage>, EditError> {
        check_velocity(velocity)?;
        self.modify_notes(ids, |note| note.velocity = velocity)
    }

    /// Give notes a new key.
    pub fn set_pitch(&mut self, ids: &[NoteId], key: u8) -> Result<Vec<NoteImage>, EditError> {
        check_key(key)?;
        self.modify_notes(ids, |note| note.key = key)
    }

    /// Move notes to another selectable track — the source a part plays — keeping their
    /// ticks. They are written on the destination track's channel.
    pub fn assign_part(
        &mut self,
        ids: &[NoteId],
        track: usize,
    ) -> Result<Vec<NoteImage>, EditError> {
        self.check_writable_track(track)?;
        let images = self.images_of(ids)?;
        for image in &images {
            self.relocate(image.id, track, image.tick)?;
        }
        Ok(images)
    }

    /// Put notes back exactly as their images describe them, re-creating removed ones
    /// under their old ids. An image whose `original_pair` is set re-links the note to
    /// its original MIDI events.
    pub fn restore_notes(&mut self, images: &[NoteImage]) -> Result<(), EditError> {
        for image in images {
            self.check_writable_track(image.track)?;
            check_key(image.key)?;
            check_velocity(image.velocity)?;
        }
        for image in images {
            self.take_note(image.id);
            let note = Note {
                id: image.id,
                key: image.key,
                velocity: image.velocity,
                duration_ticks: image.duration_ticks,
            };
            self.place_note(image.track, image.tick, note);
            if image.original_pair {
                self.original_notes.insert(image.id);
            } else {
                self.original_notes.remove(&image.id);
            }
            self.next_note_id = self.next_note_id.max(image.id.0 + 1);
        }
        Ok(())
    }

    /// Add an empty track at the end of a type-1 song: a file track of its own carrying its
    /// name and, at tick 0, the program change that makes it play `program` and keeps its
    /// channel when the file is read back. A channel is free when no track plays on it and
    /// no retained event of any track names it.
    pub fn add_track(&mut self, track: NewTrack) -> Result<AddedTrack, EditError> {
        if self.format == SmfFormat::SingleTrack {
            return Err(EditError::SingleTrackSong);
        }
        if track.program > HIGHEST_PROGRAM {
            return Err(EditError::ProgramOutOfRange {
                program: track.program,
            });
        }
        let occupied = self.occupied_channels();
        let channel = match track.channel {
            Some(channel) if channel > HIGHEST_CHANNEL => {
                return Err(EditError::ChannelOutOfRange { channel })
            }
            Some(channel) if occupied.contains(&channel) => {
                return Err(EditError::ChannelInUse { channel })
            }
            Some(channel) => channel,
            None => (0..=HIGHEST_CHANNEL)
                .find(|channel| *channel != PERCUSSION_CHANNEL && !occupied.contains(channel))
                .ok_or(EditError::NoFreeChannel)?,
        };
        let midi_channel = u4::new(channel);
        self.retained_tracks.push(vec![
            RetainedEvent::retain(
                0,
                TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes())),
            ),
            RetainedEvent::retain(
                0,
                TrackEventKind::Midi {
                    channel: midi_channel,
                    message: MidiMessage::ProgramChange {
                        program: u7::new(track.program),
                    },
                },
            ),
            RetainedEvent::retain(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]);
        self.tracks.push(SongTrack {
            name: Some(track.name),
            channel: Some(channel),
            source_track: self.retained_tracks.len() - 1,
            units: Vec::new(),
        });
        self.added_tracks += 1;
        Ok(AddedTrack {
            track: self.tracks.len() - 1,
            channel,
        })
    }

    /// Take away the track [`SongData::add_track`] added last, with whatever notes it holds,
    /// returning their before-images. Only the last track, only while an added one remains,
    /// and only one that is a file track of its own, can go: an imported file's own tracks are
    /// never taken away, and every other track's index is what a part's selector names.
    pub fn remove_added_track(&mut self, track: usize) -> Result<Vec<NoteImage>, EditError> {
        self.check_track(track)?;
        let last_source = self.retained_tracks.len() - 1;
        let removable = self.added_tracks > 0
            && track + 1 == self.tracks.len()
            && self.tracks[track].source_track == last_source
            && self
                .tracks
                .iter()
                .filter(|candidate| candidate.source_track == last_source)
                .count()
                == 1;
        if !removable {
            return Err(EditError::NotTheLastTrack { track });
        }
        let ids: Vec<NoteId> = self.tracks[track]
            .units
            .iter()
            .flat_map(|unit| unit.notes.iter().map(|note| note.id))
            .collect();
        let images = self.remove_notes(&ids)?;
        self.tracks.pop();
        self.retained_tracks.pop();
        self.added_tracks -= 1;
        Ok(images)
    }

    /// Every channel a track plays on or a retained event names.
    pub(super) fn occupied_channels(&self) -> BTreeSet<u8> {
        let event_channels = self
            .retained_tracks
            .iter()
            .flatten()
            .filter_map(|event| match event.kind {
                TrackEventKind::Midi { channel, .. } => Some(channel.as_int()),
                _ => None,
            });
        self.tracks
            .iter()
            .filter_map(|track| track.channel)
            .chain(event_channels)
            .collect()
    }

    /// Set the tempo from `change.tick` on, returning the change it replaced on that
    /// tick (undo: set that back, or [`SongData::remove_tempo`] when there was none).
    pub fn set_tempo(&mut self, change: TempoChange) -> Result<Option<TempoChange>, EditError> {
        change.check()?;
        self.tempo_edited = true;
        Ok(upsert_by_tick(
            &mut self.tempo_changes,
            change,
            |existing| existing.tick,
        ))
    }

    /// Remove the tempo change on `tick`, returning it.
    pub fn remove_tempo(&mut self, tick: u64) -> Option<TempoChange> {
        let index = self
            .tempo_changes
            .iter()
            .position(|change| change.tick == tick)?;
        self.tempo_edited = true;
        Some(self.tempo_changes.remove(index))
    }

    /// Set the time signature from `signature.tick` on, returning the one it replaced on
    /// that tick.
    pub fn set_time_signature(
        &mut self,
        signature: TimeSignature,
    ) -> Result<Option<TimeSignature>, EditError> {
        signature.check(self.ticks_per_quarter)?;
        self.time_signatures_edited = true;
        Ok(upsert_by_tick(
            &mut self.time_signatures,
            signature,
            |existing| existing.tick,
        ))
    }

    /// Remove the time signature on `tick`, returning it.
    pub fn remove_time_signature(&mut self, tick: u64) -> Option<TimeSignature> {
        let index = self
            .time_signatures
            .iter()
            .position(|signature| signature.tick == tick)?;
        self.time_signatures_edited = true;
        Some(self.time_signatures.remove(index))
    }

    /// Every named note's image, each once, or the first unknown id.
    fn images_of(&self, ids: &[NoteId]) -> Result<Vec<NoteImage>, EditError> {
        let unique: BTreeSet<NoteId> = ids.iter().copied().collect();
        unique
            .into_iter()
            .map(|id| self.note_image(id).ok_or(EditError::UnknownNote(id)))
            .collect()
    }

    fn modify_notes(
        &mut self,
        ids: &[NoteId],
        change: impl Fn(&mut Note),
    ) -> Result<Vec<NoteImage>, EditError> {
        let images = self.images_of(ids)?;
        for image in &images {
            let mut note = self
                .take_note(image.id)
                .ok_or(EditError::UnknownNote(image.id))?;
            change(&mut note);
            self.place_note(image.track, image.tick, note);
            self.original_notes.remove(&image.id);
        }
        Ok(images)
    }

    fn relocate(&mut self, id: NoteId, track: usize, tick: u64) -> Result<(), EditError> {
        let note = self.take_note(id).ok_or(EditError::UnknownNote(id))?;
        self.place_note(track, tick, note);
        self.original_notes.remove(&id);
        Ok(())
    }

    /// Remove a note from its unit (dropping the unit if it empties) and forget where it
    /// was. Its `original_notes` membership is the caller's to settle.
    fn take_note(&mut self, id: NoteId) -> Option<Note> {
        let location = self.note_locations.remove(&id)?;
        let units = &mut self.tracks.get_mut(location.track)?.units;
        let unit_index = units
            .binary_search_by_key(&location.tick, |unit| unit.tick)
            .ok()?;
        let notes = &mut units[unit_index].notes;
        let note_index = notes.iter().position(|note| note.id == id)?;
        let note = notes.remove(note_index);
        if notes.is_empty() {
            units.remove(unit_index);
        }
        Some(note)
    }

    /// Put a note in its unit on `tick`, creating the unit if none starts there. `track`
    /// is checked by every caller before anything is changed.
    fn place_note(&mut self, track: usize, tick: u64, note: Note) {
        let units = &mut self.tracks[track].units;
        match units.binary_search_by_key(&tick, |unit| unit.tick) {
            Ok(index) => units[index].notes.push(note),
            Err(index) => units.insert(
                index,
                MusicalUnit {
                    tick,
                    notes: vec![note],
                },
            ),
        }
        self.note_locations
            .insert(note.id, NoteLocation { track, tick });
    }

    fn check_track(&self, track: usize) -> Result<(), EditError> {
        if track >= self.tracks.len() {
            return Err(EditError::TrackOutOfRange {
                track,
                track_count: self.tracks.len(),
            });
        }
        Ok(())
    }

    fn check_writable_track(&self, track: usize) -> Result<(), EditError> {
        self.check_track(track)?;
        if self.tracks[track].channel.is_none() {
            return Err(EditError::TrackHasNoChannel { track });
        }
        Ok(())
    }
}

/// Insert `item` in tick order, replacing (and returning) one already on its tick.
fn upsert_by_tick<T: Copy>(sorted: &mut Vec<T>, item: T, tick_of: impl Fn(&T) -> u64) -> Option<T> {
    let tick = tick_of(&item);
    match sorted.binary_search_by_key(&tick, &tick_of) {
        Ok(index) => Some(std::mem::replace(&mut sorted[index], item)),
        Err(index) => {
            sorted.insert(index, item);
            None
        }
    }
}

fn check_key(key: u8) -> Result<(), EditError> {
    if key > HIGHEST_KEY {
        return Err(EditError::KeyOutOfRange { key });
    }
    Ok(())
}

fn check_velocity(velocity: u8) -> Result<(), EditError> {
    if velocity == 0 || velocity > HIGHEST_VELOCITY {
        return Err(EditError::VelocityOutOfRange { velocity });
    }
    Ok(())
}

fn ends_by_last_writable_tick(tick: u64, duration_ticks: u64) -> bool {
    tick.checked_add(duration_ticks)
        .is_some_and(|end_tick| end_tick <= LAST_WRITABLE_TICK)
}

fn check_duration(duration_ticks: u64) -> Result<(), EditError> {
    if duration_ticks == 0 {
        return Err(EditError::ZeroDuration);
    }
    Ok(())
}
