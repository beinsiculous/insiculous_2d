//! The sidecar's three lists as sidebar blocks: each part with the track it reads, its mode
//! (and the Loop caveat), its quantize and the bindings that play it; the tracks no part
//! reads, marked as unassigned background; each instrument with the fields its source
//! holds; each binding with its Trigger. Every block ends in a remove button, so an
//! accidental drop never traps a save.

use music_midi::{
    Instrument, InstrumentSource, Part, PartMode, Provenance, Quantize, SamplePlayback, TrackSelector,
};
use ui::{FloatFieldOpts, WidgetId};

use super::walk::SidebarWalk;
use crate::composer::state::selectable_tracks;
use crate::composer::{note_name, track_label, unique_name, ComposerAction};
use crate::song::{SongDocument, SongEdit};

/// `docs/SONG_FORMAT.md` § Modes asks the composer to show this beside a Loop part.
const LOOP_CAVEAT: [&str; 3] = [
    "Loop plays this part's notes in order whenever",
    "its action fires, over whatever chord is sounding:",
    "write it to fit every chord, or use AtSongPosition.",
];

fn id(name: &str, index: usize) -> WidgetId {
    WidgetId::from_str_index(name, index)
}

/// The track a part's selector reads, if the song has it.
pub fn resolve_track(song: &SongDocument, selector: &TrackSelector) -> Option<usize> {
    let tracks = song.data().tracks();
    match selector {
        TrackSelector::Index(index) => (*index < tracks.len()).then_some(*index),
        TrackSelector::Name(name) => tracks.iter().position(|track| track.name() == Some(name.as_str())),
    }
}

fn mode_label(mode: PartMode) -> &'static str {
    match mode {
        PartMode::Loop => "Loop",
        PartMode::Sequence => "Sequence",
        PartMode::AtSongPosition => "AtSongPosition",
    }
}

fn next_mode(mode: PartMode) -> PartMode {
    match mode {
        PartMode::Loop => PartMode::Sequence,
        PartMode::Sequence => PartMode::AtSongPosition,
        PartMode::AtSongPosition => PartMode::Loop,
    }
}

fn next_quantize(quantize: Quantize) -> Quantize {
    match quantize {
        Quantize::Off => Quantize::Beat,
        Quantize::Beat => Quantize::Eighth,
        Quantize::Eighth => Quantize::Sixteenth,
        Quantize::Sixteenth => Quantize::Off,
    }
}

/// The Add-track row's program, kept on the composer's state between frames, and whether
/// Enter went down this frame: the name field's commit adds the track only on Enter.
pub(super) struct TrackDraft<'a> {
    pub program: &'a mut u8,
    pub enter_pressed: bool,
}

pub(super) fn parts(walk: &mut SidebarWalk, song: &SongDocument, active_track: Option<usize>, draft: TrackDraft) {
    walk.heading("Parts");
    let file = song.file();
    if file.parts.is_empty() {
        walk.note("No parts: every track plays as background");
    }
    for (index, part) in file.parts.iter().enumerate() {
        let track = resolve_track(song, &part.track);
        let target = match track {
            Some(track) => track_label(song, track),
            None => format!("{} (not in this file)", part.track),
        };
        if walk.button(id("composer_part_track", index), &format!("{} → {target}", part.name), track.is_some()) {
            if let Some(track) = track {
                walk.push(ComposerAction::SetActiveTrack(track));
            }
        }
        walk.tooltip("Show this part's track in the roll and the notes");
        if walk.button(id("composer_part_mode", index), &format!("Mode: {}", mode_label(part.mode)), true) {
            walk.push(ComposerAction::Edit(SongEdit::SetPartMode {
                part: part.name.clone(),
                mode: next_mode(part.mode),
            }));
        }
        if part.mode == PartMode::Loop {
            for line in LOOP_CAVEAT {
                walk.note(line);
            }
        }
        if walk.button(id("composer_part_quantize", index), &format!("Quantize: {:?}", part.quantize), true) {
            walk.push(ComposerAction::Edit(SongEdit::SetQuantize {
                part: part.name.clone(),
                quantize: next_quantize(part.quantize),
            }));
        }
        let players: Vec<&str> = file
            .bindings
            .iter()
            .filter(|binding| binding.part == part.name)
            .map(|binding| binding.action.as_str())
            .collect();
        if players.is_empty() {
            walk.note("No binding plays it");
        } else {
            walk.note(&format!("Played by: {}", players.join(", ")));
        }
        if walk.button(id("composer_part_remove", index), &format!("Remove part {}", part.name), true) {
            walk.push(ComposerAction::Edit(SongEdit::RemovePart { name: part.name.clone() }));
        }
    }
    for track in selectable_tracks(song) {
        let assigned = file.parts.iter().any(|part| resolve_track(song, &part.track) == Some(track));
        if !assigned {
            walk.note(&format!("{} · unassigned background", track_label(song, track)));
        }
    }
    if let Some(track) = active_track {
        if walk.button(WidgetId::hashed("composer_part_add"), "Add a part for the shown track", true) {
            walk.push(ComposerAction::Edit(SongEdit::AddPart { part: new_part(song, track) }));
        }
    }
    add_track_row(walk, draft);
}

/// A name field and a program field: Enter in the name adds the track on the next free
/// channel playing that program, as one edit.
fn add_track_row(walk: &mut SidebarWalk, draft: TrackDraft) {
    walk.heading("Add track");
    let program = walk.number(
        WidgetId::hashed("composer_track_program"),
        "Program",
        f32::from(*draft.program),
        FloatFieldOpts::hard(0.0, 127.0),
    );
    if let Some((value, _)) = program {
        *draft.program = value.round().clamp(0.0, 127.0) as u8;
    }
    walk.note("General MIDI 0–127: trumpet 56, trombone 57, timpani 47");
    let named = walk.field(WidgetId::hashed("composer_track_name"), "Name", "");
    walk.note("Type a name, Enter adds the track on the next free channel");
    let Some(name) = named.filter(|_| draft.enter_pressed) else {
        return;
    };
    let name = name.trim();
    if !name.is_empty() {
        walk.push(ComposerAction::Edit(SongEdit::AddTrack {
            name: name.to_string(),
            channel: None,
            program: *draft.program,
        }));
    }
}

/// A part reading `track`: by its name when no other track shares it, else by index.
fn new_part(song: &SongDocument, track: usize) -> Part {
    let tracks = song.data().tracks();
    let track_name = tracks.get(track).and_then(|song_track| song_track.name());
    let selector = match track_name {
        Some(name) if tracks.iter().filter(|other| other.name() == Some(name)).count() == 1 => {
            TrackSelector::Name(name.to_string())
        }
        _ => TrackSelector::Index(track),
    };
    let base = track_name.map(str::to_string).unwrap_or_else(|| format!("part_{track}"));
    Part {
        name: unique_name(&base, song.file().parts.iter().map(|part| part.name.as_str())),
        track: selector,
        mode: PartMode::Loop,
        quantize: Quantize::Off,
    }
}

pub(super) fn instruments(walk: &mut SidebarWalk, song: &SongDocument) {
    walk.heading("Instruments");
    if song.file().instruments.is_empty() {
        walk.note("Drop a .wav or a .sf2 from Assets here to add one");
    }
    for (index, instrument) in song.file().instruments.iter().enumerate() {
        let edited = match &instrument.source {
            InstrumentSource::SoundFont { path, bank, preset } => {
                walk.note(&format!("{} · SoundFont {path}", instrument.name));
                sound_font_fields(walk, index, instrument, *bank, *preset)
            }
            InstrumentSource::Sample { path, root_note, gain, playback } => {
                walk.note(&format!("{} · sample {path}", instrument.name));
                sample_fields(walk, index, instrument, (*root_note, *gain, *playback))
            }
        };
        if let Some((edited, landing)) = edited {
            let edit = SongEdit::SetInstrument {
                name: instrument.name.clone(),
                instrument: edited,
            };
            walk.push(match landing {
                Landing::Edit => ComposerAction::Edit(edit),
                Landing::Field { first } => ComposerAction::FieldEdit { edit, first },
            });
        }
        if walk.button(id("composer_instrument_remove", index), &format!("Remove instrument {}", instrument.name), true) {
            walk.push(ComposerAction::Edit(SongEdit::RemoveInstrument { name: instrument.name.clone() }));
        }
    }
}

fn with_source(instrument: &Instrument, source: InstrumentSource) -> Instrument {
    Instrument { source, ..instrument.clone() }
}

/// How an instrument's edit lands.
#[derive(Debug, Clone, Copy)]
enum Landing {
    /// A button or a text field's commit: an edit of its own.
    Edit,
    /// A frame of a number field's gesture; `first` opens the gesture's entry.
    Field { first: bool },
}

/// The instrument as edited this frame, and how the edit lands.
type InstrumentChange = Option<(Instrument, Landing)>;

fn sound_font_fields(walk: &mut SidebarWalk, index: usize, instrument: &Instrument, bank: u16, preset: u8) -> InstrumentChange {
    let InstrumentSource::SoundFont { path, .. } = &instrument.source else {
        return None;
    };
    let bank_field = walk.number(id("composer_instrument_bank", index), "Bank", f32::from(bank), FloatFieldOpts::hard(0.0, 128.0));
    walk.note("Bank 128 is General MIDI percussion");
    let preset_field = walk.number(id("composer_instrument_preset", index), "Preset", f32::from(preset), FloatFieldOpts::hard(0.0, 127.0));
    // Both fields fold into the one edit: a frame can change both (an arrow in the focused
    // one while the pointer scrubs the other), and the gesture is the first's.
    let (_, first) = bank_field.or(preset_field)?;
    let bank = bank_field.map_or(bank, |(value, _)| value.round() as u16);
    let preset = preset_field.map_or(preset, |(value, _)| value.round() as u8);
    Some((
        with_source(instrument, InstrumentSource::SoundFont { path: path.clone(), bank, preset }),
        Landing::Field { first },
    ))
}

fn sample_fields(
    walk: &mut SidebarWalk,
    index: usize,
    instrument: &Instrument,
    (root_note, gain, playback): (u8, f32, SamplePlayback),
) -> InstrumentChange {
    let InstrumentSource::Sample { path, .. } = &instrument.source else {
        return None;
    };
    let source = |root_note, gain, playback| InstrumentSource::Sample { path: path.clone(), root_note, gain, playback };
    let root = walk.number(id("composer_instrument_root", index), "Root note", f32::from(root_note), FloatFieldOpts::hard(0.0, 127.0));
    walk.note(&format!("Root {} (key {root_note}): played higher it is shorter,", note_name(root_note)));
    walk.note("lower it is longer — pitch changes its duration");
    let gain_field = walk.number(id("composer_instrument_gain", index), "Gain", gain, FloatFieldOpts::hard(0.0, 8.0).with_step(0.05));
    walk.note("Gain 1.0 plays a sample near one SoundFont note");
    let playback_label = match playback {
        SamplePlayback::OneShot => "Playback: one-shot (plays to its end)",
        SamplePlayback::Gated => "Playback: gated (fades when the note ends)",
    };
    let toggled = walk.button(id("composer_instrument_playback", index), playback_label, true);
    let provenance = instrument.provenance.clone().unwrap_or(Provenance { author: String::new(), license: String::new() });
    let author = walk.field(id("composer_instrument_author", index), "Author", &provenance.author);
    let license = walk.field(id("composer_instrument_license", index), "License", &provenance.license);
    if provenance.author.trim().is_empty() || provenance.license.trim().is_empty() {
        walk.warning("Author and license: needed before Save");
    }
    if let Some((root, first)) = root {
        return Some((with_source(instrument, source(root.round() as u8, gain, playback)), Landing::Field { first }));
    }
    if let Some((gain, first)) = gain_field {
        return Some((with_source(instrument, source(root_note, gain, playback)), Landing::Field { first }));
    }
    if toggled {
        let flipped = match playback {
            SamplePlayback::OneShot => SamplePlayback::Gated,
            SamplePlayback::Gated => SamplePlayback::OneShot,
        };
        return Some((with_source(instrument, source(root_note, gain, flipped)), Landing::Edit));
    }
    let committed = Provenance {
        author: author.unwrap_or_else(|| provenance.author.clone()),
        license: license.unwrap_or_else(|| provenance.license.clone()),
    };
    // A field committed with its own value is no edit, as the song's are not.
    (committed != provenance).then(|| (Instrument { provenance: Some(committed), ..instrument.clone() }, Landing::Edit))
}

/// Each binding with its Trigger, which plays the binding's part as its action would — heard
/// only while the preview sounds; pressed otherwise, the host says why.
pub(super) fn bindings(walk: &mut SidebarWalk, song: &SongDocument, preview_audible: bool) {
    walk.heading("Bindings");
    if song.file().bindings.is_empty() {
        walk.note("No bindings: a binding names a game action");
    }
    for (index, binding) in song.file().bindings.iter().enumerate() {
        walk.note(&format!("{} → part {} on {}", binding.action, binding.part, binding.instrument));
        if walk.button(id("composer_binding_trigger", index), &format!("Trigger {}", binding.action), preview_audible) {
            walk.push(ComposerAction::PreviewTrigger(index));
        }
        walk.tooltip(if preview_audible {
            "Play this binding's part as its action would"
        } else {
            "Triggers sound while the song plays: press Play first"
        });
        if walk.button(id("composer_binding_remove", index), &format!("Remove binding {}", binding.action), true) {
            walk.push(ComposerAction::Edit(SongEdit::RemoveBinding { action: binding.action.clone() }));
        }
    }
}
