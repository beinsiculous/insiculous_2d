//! Inspector editor for `SoundEmitter`: a kind row cycling between a recorded sound and a
//! song binding, the sound slot (or the binding's name), and the rows the kind allows.
//!
//! **The inspector cannot author what the loader refuses**: cycling to Binding clears
//! `spatial` and `looping`, and a binding hides those rows and `volume` — the song owns a
//! binding's dynamics. `volume` and `max_distance` hold their hard floors.
//!
//! A named binding gets a button that opens the song's MIDI file in the music tool at the
//! part's track. Which song is the host's rule; when the rule reaches a project with
//! several songs and none playing or open, a Song row above the button chooses among them.
//! The button is drawn where the Open source button is — never on the web.

use std::ops::RangeInclusive;

use ecs::{SoundEmitter, SoundRef, Spatial};

use crate::component_editors::ComponentEdit;
use crate::physical_floors::{clamp_sound_emitter, MAX_DISTANCE_FLOOR};
use crate::{EditResult, EditableInspector};

/// The kind row's values, in cycle order.
const KINDS: [&str; 2] = ["Path", "Binding"];

/// Soft: a level above one is clamped where the sound plays.
const VOLUME: RangeInclusive<f32> = 0.0..=1.0;
/// Soft, in world units.
const MAX_DISTANCE: RangeInclusive<f32> = MAX_DISTANCE_FLOOR..=50.0;

/// The binding button's state between the inspector pass and the host, on the editor's
/// context for the session and never in the preferences.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditBindingState {
    /// The songs the Song row cycles, written by the host before the inspector draws: the
    /// project's Songs list when its rule reaches the list with several, else empty, and no
    /// row is drawn.
    pub song_choices: Vec<String>,
    /// The Song row's choice, by path; the first song while none is chosen.
    pub chosen_song: Option<String>,
    /// The action a press of the button asked to open, taken by the host.
    pub requested: Option<String>,
}

/// Edit a SoundEmitter component.
pub fn edit_sound_emitter(
    inspector: &mut EditableInspector<'_>,
    emitter: &SoundEmitter,
    extras: &mut crate::InspectorExtras<'_>,
) -> Option<ComponentEdit<SoundEmitter>> {
    let mut new = emitter.clone();
    let mut hint = None;
    let is_binding = emitter.sound.is_binding();

    inspector.header("SoundEmitter");

    let kind_index = usize::from(is_binding);
    let kind = inspector.cycle("Kind", KINDS[kind_index], kind_index, KINDS.len());
    let sound = match &emitter.sound {
        SoundRef::Path(path) => match inspector.sound("Sound", path, extras.drag_drop) {
            EditResult::Changed(dropped) => EditResult::Changed(SoundRef::Path(dropped)),
            EditResult::Unchanged => EditResult::Unchanged,
        },
        SoundRef::Binding(name) => match inspector.string_edit("Binding", name) {
            EditResult::Changed(typed) => EditResult::Changed(SoundRef::Binding(typed)),
            EditResult::Unchanged => EditResult::Unchanged,
        },
    };
    sound.assign(&mut new.sound, &mut hint, "sound");
    // After the name row, so a name committed by the click on an arrow is replaced by
    // the new kind's empty one rather than replacing it.
    if let EditResult::Changed(index) = kind {
        new.sound = if index == 1 { SoundRef::Binding(String::new()) } else { SoundRef::Path(String::new()) };
        if new.sound.is_binding() {
            new.spatial = None;
            new.looping = false;
        }
        hint = Some("sound");
    }

    if let SoundRef::Binding(action) = &emitter.sound {
        edit_binding_in_music_tool(inspector, action, extras);
    }

    if !is_binding {
        inspector.f32("Volume", emitter.volume, VOLUME).assign(&mut new.volume, &mut hint, "volume");
    }
    inspector.bool("Play On Spawn", emitter.play_on_spawn).assign(&mut new.play_on_spawn, &mut hint, "play_on_spawn");
    if !is_binding {
        inspector.bool("Looping", emitter.looping).assign(&mut new.looping, &mut hint, "looping");
        if let EditResult::Changed(spatial) = inspector.bool("Spatial", emitter.spatial.is_some()) {
            new.spatial = spatial.then(Spatial::default);
            hint = Some("spatial");
        }
        if let Some(spatial) = emitter.spatial {
            if let EditResult::Changed(max_distance) = inspector.f32("Max Distance", spatial.max_distance, MAX_DISTANCE) {
                new.spatial = Some(Spatial { max_distance });
                hint = Some("spatial");
            }
        }
    }

    clamp_sound_emitter(&mut new);
    hint.map(|field_hint| ComponentEdit { new_value: new, field_hint })
}

/// The Song row, when the host offers a choice, and the button that asks the host to open
/// `action`'s part: neither for an empty name, nor where no external tool can open a file.
fn edit_binding_in_music_tool(inspector: &mut EditableInspector<'_>, action: &str, extras: &mut crate::InspectorExtras<'_>) {
    if !extras.can_open_source || action.trim().is_empty() {
        return;
    }
    let choices = &extras.song_choices;
    if !choices.is_empty() {
        let chosen = extras.chosen_song.as_ref().and_then(|song| choices.iter().position(|choice| choice == song));
        let index = chosen.unwrap_or(0);
        if let EditResult::Changed(next) = inspector.cycle("Song", &choices[index], index, choices.len()) {
            extras.chosen_song = choices.get(next).cloned();
        }
    }
    if inspector.action_button(&extras.edit_binding_label) {
        extras.edit_binding = Some(action.to_string());
    }
}
