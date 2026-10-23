//! Standard MIDI File bytes → [`SongData`]: midly's zero-copy events become owned events
//! at absolute ticks, note-ons are paired with their note-offs, and the notes are laid
//! out as selectable tracks.
//!
//! Pairing is first-in first-out per channel and key, so a key struck again before its
//! note-off is released in the order it was struck; the overlap is reported. A note-off
//! that closes nothing stays a retained event and links to no note. A note still open at
//! the end of its track lasts until that track's last event.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use midly::{Format, MetaMessage, MidiMessage, Timing, TrackEventKind};

use super::{
    MusicalUnit, Note, NoteId, NoteLocation, RetainedEvent, SameKeyOverlap, SmfFormat, SongData,
    SongTrack, UnsupportedFeature, UnsupportedReport,
};
use crate::error::SongError;
use crate::tempo_map::{TempoChange, TimeSignature};
use crate::{MIDI_BYTE_LIMIT, MIDI_EVENT_LIMIT};

/// A note-on waiting for its note-off.
struct OpenNote {
    id: NoteId,
    tick: u64,
    velocity: u8,
}

/// A paired note, before it is placed in a selectable track.
struct PairedNote {
    id: NoteId,
    source_track: usize,
    channel: u8,
    tick: u64,
    key: u8,
    velocity: u8,
    duration_ticks: u64,
}

/// An overlap, before its selectable track is known.
struct RawOverlap {
    source_track: usize,
    channel: u8,
    key: u8,
    tick: u64,
}

pub(super) fn flatten(bytes: &[u8]) -> Result<(SongData, UnsupportedReport), SongError> {
    if bytes.len() > MIDI_BYTE_LIMIT {
        return Err(SongError::TooLarge {
            bytes: bytes.len(),
            limit: MIDI_BYTE_LIMIT,
        });
    }
    let (header, track_iterator) = midly::parse(bytes).map_err(malformed)?;
    let format = match header.format {
        Format::SingleTrack => SmfFormat::SingleTrack,
        Format::Parallel => SmfFormat::Parallel,
        Format::Sequential => return Err(SongError::SequentialFormat),
    };
    let ticks_per_quarter = match header.timing {
        Timing::Metrical(ticks) => ticks.as_int(),
        Timing::Timecode(frames_per_second, _) => {
            return Err(SongError::SmpteDivision {
                frames_per_second: frames_per_second.as_int(),
            })
        }
    };
    if ticks_per_quarter == 0 {
        return Err(SongError::ZeroDivision);
    }

    let mut retained_tracks = read_tracks(track_iterator)?;
    if retained_tracks.is_empty() {
        return Err(SongError::NoTracks);
    }
    let (tempo_changes, time_signatures) = collect_timing(&retained_tracks, ticks_per_quarter)?;

    let mut next_note_id = 0;
    let mut paired_notes = Vec::new();
    let mut raw_overlaps = Vec::new();
    for (source_track, events) in retained_tracks.iter_mut().enumerate() {
        pair_notes(
            source_track,
            events,
            &mut next_note_id,
            &mut paired_notes,
            &mut raw_overlaps,
        );
    }

    let (mut tracks, track_of) = match format {
        SmfFormat::Parallel => parallel_tracks(&retained_tracks),
        SmfFormat::SingleTrack => channel_tracks(&retained_tracks),
    };
    let mut report = UnsupportedReport::default();
    if format == SmfFormat::Parallel {
        report.unsupported = mixed_channel_tracks(&mut tracks, &paired_notes);
    }
    report.same_key_overlaps = raw_overlaps
        .iter()
        .filter_map(|overlap| {
            let track = *track_of.get(&(overlap.source_track, overlap.channel))?;
            Some(SameKeyOverlap {
                track,
                channel: overlap.channel,
                key: overlap.key,
                tick: overlap.tick,
            })
        })
        .collect();

    let mut note_locations = BTreeMap::new();
    let mut original_notes = BTreeSet::new();
    paired_notes.sort_by_key(|note| note.id);
    for paired in &paired_notes {
        let Some(&track) = track_of.get(&(paired.source_track, paired.channel)) else {
            continue;
        };
        let units = &mut tracks[track].units;
        let note = Note {
            id: paired.id,
            key: paired.key,
            velocity: paired.velocity,
            duration_ticks: paired.duration_ticks,
        };
        // Note ids follow note-on order, which is tick order within a track.
        match units.last_mut() {
            Some(unit) if unit.tick == paired.tick => unit.notes.push(note),
            _ => units.push(MusicalUnit {
                tick: paired.tick,
                notes: vec![note],
            }),
        }
        note_locations.insert(
            paired.id,
            NoteLocation {
                track,
                tick: paired.tick,
            },
        );
        original_notes.insert(paired.id);
    }

    let song = SongData {
        format,
        ticks_per_quarter,
        retained_tracks,
        tracks,
        tempo_changes,
        time_signatures,
        original_notes,
        note_locations,
        tempo_edited: false,
        time_signatures_edited: false,
        next_note_id,
        added_tracks: 0,
    };
    Ok((song, report))
}

fn malformed(error: midly::Error) -> SongError {
    SongError::Malformed {
        reason: error.to_string(),
    }
}

/// Every track's events at absolute ticks, counting events as they are parsed.
fn read_tracks(track_iterator: midly::TrackIter<'_>) -> Result<Vec<Vec<RetainedEvent>>, SongError> {
    let mut event_count = 0usize;
    let mut retained_tracks = Vec::new();
    for track in track_iterator {
        let mut tick = 0u64;
        let mut events = Vec::new();
        for event in track.map_err(malformed)? {
            let event = event.map_err(malformed)?;
            event_count += 1;
            if event_count > MIDI_EVENT_LIMIT {
                return Err(SongError::TooManyEvents {
                    limit: MIDI_EVENT_LIMIT,
                });
            }
            tick += u64::from(event.delta.as_int());
            events.push(RetainedEvent::retain(tick, event.kind));
        }
        retained_tracks.push(events);
    }
    Ok(retained_tracks)
}

/// The tempo changes and time signatures from every track, sorted by tick. Where two
/// fall on one tick the later in file order wins, as a sequencer playing the file would
/// leave it.
fn collect_timing(
    retained_tracks: &[Vec<RetainedEvent>],
    ticks_per_quarter: u16,
) -> Result<(Vec<TempoChange>, Vec<TimeSignature>), SongError> {
    let mut tempo_changes = Vec::new();
    let mut time_signatures = Vec::new();
    for (track, events) in retained_tracks.iter().enumerate() {
        for event in events {
            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                    let microseconds_per_quarter = tempo.as_int();
                    if microseconds_per_quarter == 0 {
                        return Err(SongError::ZeroTempo {
                            track,
                            tick: event.tick,
                        });
                    }
                    tempo_changes.push(TempoChange {
                        tick: event.tick,
                        microseconds_per_quarter,
                    });
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(
                    numerator,
                    denominator_exponent,
                    clocks_per_click,
                    thirty_second_notes_per_quarter,
                )) => {
                    // An exponent past 7 names a note value no u8 denominator holds.
                    let signature = 1u8
                        .checked_shl(u32::from(denominator_exponent))
                        .map(|denominator| TimeSignature {
                            tick: event.tick,
                            numerator,
                            denominator,
                            clocks_per_click,
                            thirty_second_notes_per_quarter,
                        })
                        .filter(|signature| signature.ticks_per_beat(ticks_per_quarter).is_some());
                    let Some(signature) = signature else {
                        return Err(SongError::InvalidTimeSignature {
                            track,
                            tick: event.tick,
                            numerator,
                            denominator_exponent,
                            ticks_per_quarter,
                        });
                    };
                    time_signatures.push(signature);
                }
                _ => {}
            }
        }
    }
    tempo_changes.sort_by_key(|change| change.tick);
    time_signatures.sort_by_key(|signature| signature.tick);
    Ok((
        keep_last_per_tick(tempo_changes, |change| change.tick),
        keep_last_per_tick(time_signatures, |signature| signature.tick),
    ))
}

fn keep_last_per_tick<T>(sorted: Vec<T>, tick_of: impl Fn(&T) -> u64) -> Vec<T> {
    let mut kept: Vec<T> = Vec::with_capacity(sorted.len());
    for item in sorted {
        if kept
            .last()
            .is_some_and(|last| tick_of(last) == tick_of(&item))
        {
            kept.pop();
        }
        kept.push(item);
    }
    kept
}

fn pair_notes(
    source_track: usize,
    events: &mut [RetainedEvent],
    next_note_id: &mut u64,
    paired_notes: &mut Vec<PairedNote>,
    raw_overlaps: &mut Vec<RawOverlap>,
) {
    let mut open: BTreeMap<(u8, u8), VecDeque<OpenNote>> = BTreeMap::new();
    let track_end_tick = events.last().map_or(0, |event| event.tick);
    for event in events.iter_mut() {
        let TrackEventKind::Midi { channel, message } = event.kind else {
            continue;
        };
        let channel = channel.as_int();
        match message {
            MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                let key = key.as_int();
                let id = NoteId(*next_note_id);
                *next_note_id += 1;
                let waiting = open.entry((channel, key)).or_default();
                if !waiting.is_empty() {
                    raw_overlaps.push(RawOverlap {
                        source_track,
                        channel,
                        key,
                        tick: event.tick,
                    });
                }
                waiting.push_back(OpenNote {
                    id,
                    tick: event.tick,
                    velocity: vel.as_int(),
                });
                event.note = Some(id);
            }
            MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                let key = key.as_int();
                let Some(opened) = open.get_mut(&(channel, key)).and_then(VecDeque::pop_front)
                else {
                    continue;
                };
                event.note = Some(opened.id);
                paired_notes.push(PairedNote {
                    id: opened.id,
                    source_track,
                    channel,
                    tick: opened.tick,
                    key,
                    velocity: opened.velocity,
                    duration_ticks: event.tick - opened.tick,
                });
            }
            _ => {}
        }
    }
    for ((channel, key), waiting) in open {
        for opened in waiting {
            paired_notes.push(PairedNote {
                id: opened.id,
                source_track,
                channel,
                tick: opened.tick,
                key,
                velocity: opened.velocity,
                duration_ticks: track_end_tick - opened.tick,
            });
        }
    }
}

/// Which selectable track holds a (source track, channel)'s notes.
type TrackIndex = BTreeMap<(usize, u8), usize>;

/// Type 1: the file's tracks, index for index. A track's channel starts as its first
/// channel event's; `mixed_channel_tracks` moves it to its first note's.
fn parallel_tracks(retained_tracks: &[Vec<RetainedEvent>]) -> (Vec<SongTrack>, TrackIndex) {
    let mut track_of = TrackIndex::new();
    let tracks = retained_tracks
        .iter()
        .enumerate()
        .map(|(source_track, events)| {
            for channel in channels_used(events) {
                track_of.insert((source_track, channel), source_track);
            }
            SongTrack {
                name: track_name(events),
                channel: first_channel(events),
                source_track,
                units: Vec::new(),
            }
        })
        .collect();
    (tracks, track_of)
}

/// Type 0: one selectable track per channel the file uses, in channel order.
fn channel_tracks(retained_tracks: &[Vec<RetainedEvent>]) -> (Vec<SongTrack>, TrackIndex) {
    let mut track_of = TrackIndex::new();
    let mut tracks = Vec::new();
    for (source_track, events) in retained_tracks.iter().enumerate() {
        for channel in channels_used(events) {
            track_of.insert((source_track, channel), tracks.len());
            tracks.push(SongTrack {
                name: Some(format!("channel {}", channel + 1)),
                channel: Some(channel),
                source_track,
                units: Vec::new(),
            });
        }
    }
    (tracks, track_of)
}

/// A type-1 track whose notes span channels is reported; its channel becomes its first
/// note's, so new notes land with the music rather than with an earlier program change.
fn mixed_channel_tracks(
    tracks: &mut [SongTrack],
    paired_notes: &[PairedNote],
) -> Vec<UnsupportedFeature> {
    let mut note_channels: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    let mut first_note: BTreeMap<usize, (NoteId, u8)> = BTreeMap::new();
    for note in paired_notes {
        let channels = note_channels.entry(note.source_track).or_default();
        if !channels.contains(&note.channel) {
            channels.push(note.channel);
        }
        let first = first_note
            .entry(note.source_track)
            .or_insert((note.id, note.channel));
        if note.id < first.0 {
            *first = (note.id, note.channel);
        }
    }
    for (track, (_, channel)) in &first_note {
        tracks[*track].channel = Some(*channel);
    }
    note_channels
        .into_iter()
        .filter(|(_, channels)| channels.len() > 1)
        .map(|(track, mut channels)| {
            channels.sort_unstable();
            UnsupportedFeature::MixedChannelTrack { track, channels }
        })
        .collect()
}

fn channels_used(events: &[RetainedEvent]) -> BTreeSet<u8> {
    events
        .iter()
        .filter_map(|event| match event.kind {
            TrackEventKind::Midi { channel, .. } => Some(channel.as_int()),
            _ => None,
        })
        .collect()
}

fn first_channel(events: &[RetainedEvent]) -> Option<u8> {
    events.iter().find_map(|event| match event.kind {
        TrackEventKind::Midi { channel, .. } => Some(channel.as_int()),
        _ => None,
    })
}

fn track_name(events: &[RetainedEvent]) -> Option<String> {
    events.iter().find_map(|event| match event.kind() {
        TrackEventKind::Meta(MetaMessage::TrackName(bytes)) => {
            Some(String::from_utf8_lossy(bytes).into_owned())
        }
        _ => None,
    })
}
