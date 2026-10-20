//! [`SongData`] → Standard MIDI File bytes, through midly's writer.
//!
//! Each original track is re-emitted from its retained events, minus the original note
//! pair of every note that was edited or removed (and every tempo or time-signature
//! event, once those lists were edited), plus the live events: the edited and added
//! notes on their track's channel, and the edited tempo and time-signature lists on the
//! first track. The end-of-track event moves out if a live note now ends later.
//!
//! An unedited song re-emits the same events at the same ticks in the same order, and
//! the same bytes when the original never repeats a status byte. midly's writer always
//! uses running status, so a file that repeats status bytes instead (most LilyPond
//! exports do) comes back shorter but event for event identical.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use super::{SmfFormat, SongData};
use crate::error::SongError;

/// Where an event goes among the events on its tick. Retained events keep their file
/// order between the live ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TickOrder {
    /// A tempo or meter in force from this tick governs every note on it.
    LiveTiming,
    /// A live note ending here must not release a retained note that starts here.
    LiveNoteOff,
    Retained,
    /// A live note starting here must not be released by a retained note-off here.
    LiveNoteOn,
    /// A zero-length live note's own note-off follows its note-on.
    ZeroLengthNoteOff,
}

struct TimedEvent<'a> {
    tick: u64,
    order: TickOrder,
    kind: TrackEventKind<'a>,
}

impl SongData {
    /// Write the song as a Standard MIDI File of its original type and division.
    pub fn write_smf(&self) -> Result<Vec<u8>, SongError> {
        let format = match self.format {
            SmfFormat::SingleTrack => Format::SingleTrack,
            SmfFormat::Parallel => Format::Parallel,
        };
        let header = Header::new(format, Timing::Metrical(u15::new(self.ticks_per_quarter)));
        let tracks = (0..self.retained_tracks.len())
            .map(|source_track| self.export_track(source_track))
            .collect::<Result<Vec<_>, _>>()?;
        let mut bytes = Vec::new();
        Smf { header, tracks }
            .write_std(&mut bytes)
            .map_err(|error| SongError::WriteFailed {
                reason: error.to_string(),
            })?;
        Ok(bytes)
    }

    fn export_track(&self, source_track: usize) -> Result<Vec<TrackEvent<'_>>, SongError> {
        let mut timed = Vec::new();
        let mut end_tick = 0;
        for event in &self.retained_tracks[source_track] {
            let kind = event.kind();
            let left_out = match kind {
                TrackEventKind::Meta(MetaMessage::EndOfTrack) => {
                    end_tick = end_tick.max(event.tick);
                    true
                }
                TrackEventKind::Meta(MetaMessage::Tempo(_)) => self.tempo_edited,
                TrackEventKind::Meta(MetaMessage::TimeSignature(..)) => self.time_signatures_edited,
                _ => event
                    .note
                    .is_some_and(|id| !self.original_notes.contains(&id)),
            };
            if !left_out {
                timed.push(TimedEvent {
                    tick: event.tick,
                    order: TickOrder::Retained,
                    kind,
                });
            }
        }
        if source_track == 0 {
            self.push_live_timing(&mut timed);
        }
        self.push_live_notes(source_track, &mut timed);
        timed.sort_by_key(|event| (event.tick, event.order));

        let mut events = Vec::with_capacity(timed.len() + 1);
        let mut previous_tick = 0;
        for event in &timed {
            events.push(TrackEvent {
                delta: delta_ticks(event.tick - previous_tick)?,
                kind: event.kind,
            });
            previous_tick = event.tick;
        }
        let end_tick = end_tick.max(previous_tick);
        events.push(TrackEvent {
            delta: delta_ticks(end_tick - previous_tick)?,
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        Ok(events)
    }

    fn push_live_timing(&self, timed: &mut Vec<TimedEvent<'_>>) {
        if self.tempo_edited {
            for change in &self.tempo_changes {
                timed.push(TimedEvent {
                    tick: change.tick,
                    order: TickOrder::LiveTiming,
                    kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(
                        change.microseconds_per_quarter,
                    ))),
                });
            }
        }
        if self.time_signatures_edited {
            for signature in &self.time_signatures {
                timed.push(TimedEvent {
                    tick: signature.tick,
                    order: TickOrder::LiveTiming,
                    kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                        signature.numerator,
                        signature.denominator.trailing_zeros() as u8,
                        signature.clocks_per_click,
                        signature.thirty_second_notes_per_quarter,
                    )),
                });
            }
        }
    }

    fn push_live_notes(&self, source_track: usize, timed: &mut Vec<TimedEvent<'_>>) {
        for track in self
            .tracks
            .iter()
            .filter(|track| track.source_track == source_track)
        {
            // Notes reach a track without a channel only by an edit, and every edit
            // refuses such a track, so its notes are all original and already retained.
            let Some(channel) = track.channel else {
                continue;
            };
            let channel = u4::new(channel);
            for unit in &track.units {
                for note in unit
                    .notes
                    .iter()
                    .filter(|note| !self.original_notes.contains(&note.id))
                {
                    let key = u7::new(note.key);
                    timed.push(TimedEvent {
                        tick: unit.tick,
                        order: TickOrder::LiveNoteOn,
                        kind: TrackEventKind::Midi {
                            channel,
                            message: MidiMessage::NoteOn {
                                key,
                                vel: u7::new(note.velocity),
                            },
                        },
                    });
                    timed.push(TimedEvent {
                        tick: unit.tick.saturating_add(note.duration_ticks),
                        order: if note.duration_ticks == 0 {
                            TickOrder::ZeroLengthNoteOff
                        } else {
                            TickOrder::LiveNoteOff
                        },
                        kind: TrackEventKind::Midi {
                            channel,
                            message: MidiMessage::NoteOff {
                                key,
                                vel: u7::new(0),
                            },
                        },
                    });
                }
            }
        }
    }
}

fn delta_ticks(ticks: u64) -> Result<u28, SongError> {
    u32::try_from(ticks)
        .ok()
        .and_then(u28::try_from)
        .ok_or_else(|| SongError::WriteFailed {
            reason: format!("a gap of {ticks} ticks does not fit a MIDI delta time"),
        })
}
