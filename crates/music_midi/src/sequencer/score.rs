//! The background score: every channel event the background synthesizer plays, built once
//! from a [`SongData`] and sorted into the order a sequencer sends it.
//!
//! **Playback is what an export would write.** A note still linked to its original MIDI
//! events plays from them, on the channel the file wrote it on; an edited or added note
//! plays from the data model on its track's channel; a removed note does not play. So the
//! composer's preview of an unsaved song sounds exactly like the file it would save.
//!
//! **Trigger tracks are left out whole** — their notes and their program, controller and
//! pitch-bend events — because a part's track plays only on its binding's instrument.
//!
//! **Order within a tick:** state events first (so a program change sounds on the note
//! beside it, as a seek's fold would apply it), then the note-offs of notes that began
//! earlier, then note-ons, then the note-offs of zero-length notes starting here.
//! Aftertouch is dropped: the synthesizer ignores it.

use std::collections::{BTreeMap, BTreeSet};

use midly::{MidiMessage, TrackEventKind};

use crate::song_data::{NoteId, SmfFormat, SongData};
use crate::tempo_map::TempoMap;

/// A channel message the background synthesizer understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMessage {
    NoteOn {
        key: u8,
        velocity: u8,
    },
    NoteOff {
        key: u8,
    },
    Controller {
        controller: u8,
        value: u8,
    },
    Program {
        program: u8,
    },
    /// The 14-bit bend, 8192 at the centre.
    PitchBend {
        value: u16,
    },
}

impl ChannelMessage {
    /// Whether the message changes what a channel will sound like rather than starting or
    /// stopping a note.
    pub fn is_state(&self) -> bool {
        !matches!(
            self,
            ChannelMessage::NoteOn { .. } | ChannelMessage::NoteOff { .. }
        )
    }
}

/// One event of the background score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreEvent {
    pub tick: u64,
    pub channel: u8,
    pub message: ChannelMessage,
    /// For a note-off, the index in [`BackgroundScore::events`] of the note-on it closes;
    /// for every other event, its own index.
    note_on_index: usize,
}

impl ScoreEvent {
    /// The index of the note-on a note-off closes (an event's own index otherwise).
    pub fn note_on_index(&self) -> usize {
        self.note_on_index
    }
}

/// The background score of a song: its events in sending order, its timing, and its
/// length. See the module docs.
#[derive(Debug, Clone)]
pub struct BackgroundScore {
    events: Vec<ScoreEvent>,
    tempo_map: TempoMap,
    end_tick: u64,
}

/// Where an event goes among the events on its tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TickOrder {
    State,
    NoteOffOfEarlierNote,
    NoteOn,
    NoteOffOfZeroLengthNote,
}

/// An event before sorting: `note` ties a note-on to its note-off.
struct PendingEvent {
    tick: u64,
    order: TickOrder,
    channel: u8,
    message: ChannelMessage,
    note: Option<NoteId>,
}

impl BackgroundScore {
    /// The score of every selectable track of `song` not in `trigger_tracks` (indexes
    /// into [`SongData::tracks`]; an index past the end selects nothing).
    pub fn new(song: &SongData, trigger_tracks: &BTreeSet<usize>) -> Self {
        let is_trigger_channel = |source_track: usize, channel: u8| {
            trigger_tracks.iter().any(|&index| {
                song.tracks().get(index).is_some_and(|track| {
                    track.source_track() == source_track
                        && (song.format() == SmfFormat::Parallel
                            || track.channel() == Some(channel))
                })
            })
        };
        let mut pending = Vec::new();
        let mut end_tick = 0;
        for (source_track, events) in song.retained_tracks().iter().enumerate() {
            let mut opened = Vec::new();
            let mut closed = BTreeSet::new();
            for event in events {
                end_tick = end_tick.max(event.tick());
                let TrackEventKind::Midi { channel, message } = event.kind() else {
                    continue;
                };
                let channel = channel.as_int();
                if is_trigger_channel(source_track, channel) {
                    continue;
                }
                let original_note = event
                    .note()
                    .and_then(|id| song.note_image(id))
                    .filter(|image| image.original_pair);
                let (order, message, note) = match message {
                    MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                        let Some(image) = original_note else {
                            continue;
                        };
                        opened.push((image.id, channel, key.as_int()));
                        let message = ChannelMessage::NoteOn {
                            key: key.as_int(),
                            velocity: vel.as_int(),
                        };
                        (TickOrder::NoteOn, message, Some(image.id))
                    }
                    MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                        let Some(image) = original_note else {
                            continue;
                        };
                        closed.insert(image.id);
                        let order = note_off_order(image.tick, event.tick());
                        let message = ChannelMessage::NoteOff { key: key.as_int() };
                        (order, message, Some(image.id))
                    }
                    MidiMessage::Controller { controller, value } => {
                        let message = ChannelMessage::Controller {
                            controller: controller.as_int(),
                            value: value.as_int(),
                        };
                        (TickOrder::State, message, None)
                    }
                    MidiMessage::ProgramChange { program } => {
                        let message = ChannelMessage::Program {
                            program: program.as_int(),
                        };
                        (TickOrder::State, message, None)
                    }
                    MidiMessage::PitchBend { bend } => {
                        let message = ChannelMessage::PitchBend {
                            value: bend.0.as_int(),
                        };
                        (TickOrder::State, message, None)
                    }
                    MidiMessage::Aftertouch { .. } | MidiMessage::ChannelAftertouch { .. } => {
                        continue;
                    }
                };
                pending.push(PendingEvent {
                    tick: event.tick(),
                    order,
                    channel,
                    message,
                    note,
                });
            }
            close_unterminated_notes(song, &opened, &closed, &mut pending);
        }
        for (index, track) in song.tracks().iter().enumerate() {
            if trigger_tracks.contains(&index) {
                continue;
            }
            // Only an edit makes a live note, and every edit refuses a track without a
            // channel, so a channel-less track's notes all play from their original events.
            let Some(channel) = track.channel() else {
                continue;
            };
            for unit in track.units() {
                for note in &unit.notes {
                    let original = song
                        .note_image(note.id)
                        .is_some_and(|image| image.original_pair);
                    if original {
                        continue;
                    }
                    let end = unit.tick.saturating_add(note.duration_ticks);
                    end_tick = end_tick.max(end);
                    pending.push(PendingEvent {
                        tick: unit.tick,
                        order: TickOrder::NoteOn,
                        channel,
                        message: ChannelMessage::NoteOn {
                            key: note.key,
                            velocity: note.velocity,
                        },
                        note: Some(note.id),
                    });
                    pending.push(PendingEvent {
                        tick: end,
                        order: note_off_order(unit.tick, end),
                        channel,
                        message: ChannelMessage::NoteOff { key: note.key },
                        note: Some(note.id),
                    });
                }
            }
        }
        // A stable sort keeps file order, and earlier tracks first, among equals.
        pending.sort_by_key(|event| (event.tick, event.order));
        Self {
            events: link_note_offs(pending),
            tempo_map: song.tempo_map(),
            end_tick,
        }
    }

    /// Every event, in the order a sequencer sends them.
    pub fn events(&self) -> &[ScoreEvent] {
        &self.events
    }

    pub fn tempo_map(&self) -> &TempoMap {
        &self.tempo_map
    }

    /// The song's authored length: its last event, end-of-track included, or the end of
    /// its last live note. A looping song wraps here.
    pub fn end_tick(&self) -> u64 {
        self.end_tick
    }
}

fn note_off_order(note_on_tick: u64, note_off_tick: u64) -> TickOrder {
    if note_off_tick > note_on_tick {
        TickOrder::NoteOffOfEarlierNote
    } else {
        TickOrder::NoteOffOfZeroLengthNote
    }
}

/// A note still open at the end of its track lasts until that track's last event, as
/// the data model says; its note-off is written here.
fn close_unterminated_notes(
    song: &SongData,
    opened: &[(NoteId, u8, u8)],
    closed: &BTreeSet<NoteId>,
    pending: &mut Vec<PendingEvent>,
) {
    for &(id, channel, key) in opened {
        if closed.contains(&id) {
            continue;
        }
        let Some(image) = song.note_image(id) else {
            continue;
        };
        let end = image.tick.saturating_add(image.duration_ticks);
        pending.push(PendingEvent {
            tick: end,
            order: note_off_order(image.tick, end),
            channel,
            message: ChannelMessage::NoteOff { key },
            note: Some(id),
        });
    }
}

/// Give every note-off the index of its note-on; a note-off whose note-on is absent is
/// dropped, since it could only release another note's voice.
fn link_note_offs(pending: Vec<PendingEvent>) -> Vec<ScoreEvent> {
    let mut note_on_index: BTreeMap<NoteId, usize> = BTreeMap::new();
    let mut events = Vec::with_capacity(pending.len());
    for event in pending {
        let index = events.len();
        let linked_index = match (event.message, event.note) {
            (ChannelMessage::NoteOn { .. }, Some(id)) => {
                note_on_index.insert(id, index);
                index
            }
            (ChannelMessage::NoteOff { .. }, Some(id)) => match note_on_index.get(&id) {
                Some(&opening) => opening,
                None => continue,
            },
            _ => index,
        };
        events.push(ScoreEvent {
            tick: event.tick,
            channel: event.channel,
            message: event.message,
            note_on_index: linked_index,
        });
    }
    events
}
