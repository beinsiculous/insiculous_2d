//! What a new score does to the parts a sidecar declares: each part's track before and after,
//! by its selector's own semantics. A `Name` part is its name, so a uniquely named track that
//! moved keeps its part; an `Index` part is the name at that index, so a rename there is
//! caught; a part whose selector the author changed is a mapping made on purpose, accepted
//! and reported. Pure over the two listings, so the open, an adoption and the audio swap all
//! read the same lines.

use music_midi::{SongData, SongFile, SongFileProblem, TrackSelector};

use super::config::Location;
use super::songs::part_location;
use crate::console::Severity;

/// One side of a diff: the song's track names, by their index in `SongData::tracks`, and
/// each declared part as `part_location` resolves it there.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackListing {
    pub tracks: Vec<Option<String>>,
    pub parts: Vec<ResolvedPart>,
}

/// A part, its selector, and where that selector lands — or why it lands nowhere.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPart {
    pub name: String,
    pub selector: TrackSelector,
    /// Where the part lives in the MIDI file, as a music tool is sent it.
    pub location: Result<Location, SongFileProblem>,
    /// The index in `SongData::tracks` it reads, which a selector's index counts; `None`
    /// when it resolves nowhere.
    pub track: Option<usize>,
}

impl TrackListing {
    /// `file`'s parts resolved over `data`, and `data`'s track names.
    pub fn of(file: &SongFile, data: &SongData) -> Self {
        Self {
            tracks: data
                .tracks()
                .iter()
                .map(|track| track.name().map(str::to_string))
                .collect(),
            parts: file
                .parts
                .iter()
                .map(|part| ResolvedPart {
                    name: part.name.clone(),
                    selector: part.track.clone(),
                    location: part_location(part, data),
                    track: part.resolve_track(data).ok(),
                })
                .collect(),
        }
    }

    /// A song with no tracks and no parts: what an open diffs against, having no old score.
    pub fn empty() -> Self {
        Self {
            tracks: Vec::new(),
            parts: Vec::new(),
        }
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.tracks.get(index).and_then(Option::as_deref)
    }
}

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub text: String,
    /// `Error` for a part the new score cannot play as the sidecar says.
    pub severity: Severity,
}

/// A diff's lines, refusals first: the first line is the one a status bar shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackDiff {
    pub lines: Vec<DiffLine>,
}

impl TrackDiff {
    /// Whether any part cannot play on the new score as its sidecar declares it.
    pub fn refuses(&self) -> bool {
        self.lines.iter().any(|line| line.severity == Severity::Error)
    }

    /// The lines' text, in order.
    pub fn texts(&self) -> Vec<String> {
        self.lines.iter().map(|line| line.text.clone()).collect()
    }
}

/// Diff `new` against `old`, by each selector's own semantics (see the module docs).
pub fn diff_tracks(old: &TrackListing, new: &TrackListing) -> TrackDiff {
    let mut refusals = Vec::new();
    let mut accepted = Vec::new();
    for part in &new.parts {
        let before = old.parts.iter().find(|candidate| candidate.name == part.name);
        match before {
            Some(before) if before.selector != part.selector => match &part.location {
                Ok(location) => accepted.push(format!(
                    "part `{}` now selects {}",
                    part.name,
                    shown_track(location, &part.selector)
                )),
                Err(problem) => refusals.push(unresolved(part, problem, Some(before), new)),
            },
            Some(before) => refusals.extend(unchanged_selector(before, part, old, new)),
            None => {
                if let Err(problem) = &part.location {
                    refusals.push(unresolved(part, problem, None, new));
                }
            }
        }
    }
    let mut lines: Vec<DiffLine> = refusals
        .into_iter()
        .map(|text| DiffLine {
            text,
            severity: Severity::Error,
        })
        .collect();
    let background = new_unselected_tracks(old, new);
    lines.extend(
        accepted
            .into_iter()
            .chain(background)
            .map(|text| DiffLine {
                text,
                severity: Severity::Info,
            }),
    );
    TrackDiff { lines }
}

/// A part whose selector stayed the same: a `Name` is unchanged wherever its track moved,
/// an `Index` is refused when the name at it changed.
fn unchanged_selector(
    before: &ResolvedPart,
    part: &ResolvedPart,
    old: &TrackListing,
    new: &TrackListing,
) -> Option<String> {
    if let Err(problem) = &part.location {
        return Some(unresolved(part, problem, Some(before), new));
    }
    let TrackSelector::Index(index) = part.selector else {
        return None;
    };
    let (was, is) = (old.name_at(index), new.name_at(index));
    (was != is).then(|| {
        format!(
            "part `{}` selects index {index}, which was {} and is now {}",
            part.name,
            quoted(was),
            quoted(is)
        )
    })
}

/// The refusal for a part that resolves nowhere in the new score.
fn unresolved(
    part: &ResolvedPart,
    problem: &SongFileProblem,
    before: Option<&ResolvedPart>,
    new: &TrackListing,
) -> String {
    if let SongFileProblem::AmbiguousTrackName { name, .. } = problem {
        return format!("`{name}` names two tracks; select one by `Index`");
    }
    let old_track = before.and_then(|before| before.track);
    let gone = match &part.selector {
        TrackSelector::Name(name) => format!("track `{name}`"),
        TrackSelector::Index(index) => format!("track {index}"),
    };
    let renamed_index = match &part.selector {
        TrackSelector::Name(_) => old_track,
        TrackSelector::Index(_) => None,
    };
    match renamed_index.and_then(|index| new.name_at(index).map(|name| (index, name))) {
        Some((index, name)) => format!(
            "{gone} is gone; index {index} is now `{name}` — renamed in MIDI Evil? Edit the sidecar's part `{}`",
            part.name
        ),
        None => format!("{gone} is gone; part `{}` selects nothing", part.name),
    }
}

/// Every track new in `new` that no part selects: by its name when it has one, else by
/// an index past the old song's last.
fn new_unselected_tracks(old: &TrackListing, new: &TrackListing) -> Vec<String> {
    // With no old score every background track is new, which says nothing: an open after
    // an external change reports the refusals alone.
    if old.tracks.is_empty() {
        return Vec::new();
    }
    let selected: Vec<usize> = new.parts.iter().filter_map(|part| part.track).collect();
    new.tracks
        .iter()
        .enumerate()
        .filter(|(index, _)| !selected.contains(index))
        .filter_map(|(index, name)| match name {
            Some(name) if !old.tracks.iter().any(|old_name| old_name.as_deref() == Some(name)) => {
                Some(format!("track `{name}` is new and no part selects it; it plays as background"))
            }
            None if index >= old.tracks.len() => Some(format!(
                "track {index} is new and no part selects it; it plays as background"
            )),
            _ => None,
        })
        .collect()
}

/// How a line names the track a location found.
fn shown_track(location: &Location, selector: &TrackSelector) -> String {
    match location {
        Location::Track {
            name: Some(name), ..
        } => format!("`{name}`"),
        Location::Track { index, .. } => format!("track {index}"),
        _ => selector.to_string(),
    }
}

fn quoted(name: Option<&str>) -> String {
    name.map_or_else(|| "a nameless track".to_string(), |name| format!("`{name}`"))
}

#[cfg(test)]
#[path = "track_diff_tests.rs"]
mod track_diff_tests;
