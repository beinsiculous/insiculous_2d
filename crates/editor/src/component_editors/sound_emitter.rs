//! Inspector editor for `SoundEmitter`: a kind row cycling between a recorded sound and a
//! song binding, the sound slot (or the binding's name), and the rows the kind allows.
//!
//! **The inspector cannot author what the loader refuses**: cycling to Binding clears
//! `spatial` and `looping`, and a binding hides those rows and `volume` — the song owns a
//! binding's dynamics. `volume` and `max_distance` hold their hard floors.

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
