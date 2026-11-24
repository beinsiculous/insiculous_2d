//! Refuse silent keys and excessive layering in the bank actually decoded by the engine.
//! Shared by this audition and editor/examples/remap_soundfont.rs; workspace checks
//! exercise both callers when this example module changes.
use std::error::Error;
use music_midi::{ChannelMessage, InstrumentSource, Song};
use rustysynth::SoundFont;

fn layers(font: &SoundFont, bank: i32, program: i32, key: u8, velocity: u8) -> usize {
    let Some(preset) = font.get_presets().iter().find(|preset|
        preset.get_bank_number() == bank && preset.get_patch_number() == program) else { return 0; };
    let key = i32::from(key);
    let velocity = i32::from(velocity);
    preset.get_regions().iter().filter(|region|
        (region.get_key_range_start()..=region.get_key_range_end()).contains(&key)
        && (region.get_velocity_range_start()..=region.get_velocity_range_end()).contains(&velocity))
        .flat_map(|region| font.get_instruments()[region.get_instrument_id()].get_regions())
        .filter(|region| (region.get_key_range_start()..=region.get_key_range_end()).contains(&key)
            && (region.get_velocity_range_start()..=region.get_velocity_range_end()).contains(&velocity)).count()
}

pub fn validate(song: &Song) -> Result<(), Box<dyn Error>> {
    let font = song.sound_font(&song.file().soundfont).ok_or("missing background bank")?;
    let mut programs = [0; 16];
    let mut banks = [0; 16];
    let mut checked = 0;
    let mut maximum = 0;
    for event in song.background().events() {
        let channel = usize::from(event.channel);
        match event.message {
            ChannelMessage::Program { program } => programs[channel] = i32::from(program),
            ChannelMessage::Controller { controller: 0, value } => banks[channel] = i32::from(value),
            ChannelMessage::NoteOn { key, velocity } => {
                let bank = banks[channel] + if channel == 9 { 128 } else { 0 };
                let count = layers(font, bank, programs[channel], key, velocity);
                if count == 0 || count >= 6 { return Err(format!("uncovered/overlayered background key {key}, bank {bank}, program {}", programs[channel]).into()); }
                maximum = maximum.max(count); checked += 1;
            }
            _ => {}
        }
    }
    for binding in &song.file().bindings {
        let instrument = song.file().instruments.iter().find(|instrument| instrument.name == binding.instrument).ok_or("missing instrument")?;
        let InstrumentSource::SoundFont { path, bank, preset } = &instrument.source else {
            return Err("this preview driver supports SoundFont instruments only".into());
        };
        let font = song.sound_font(path).ok_or("missing cue bank")?;
        let part = song.file().parts.iter().find(|part| part.name == binding.part).ok_or("missing part")?;
        let track = part.resolve_track(song.data())?;
        for unit in song.data().tracks()[track].units() {
            for note in &unit.notes {
                let count = layers(font, i32::from(*bank), i32::from(*preset), note.key, note.velocity);
                if count == 0 || count >= 6 { return Err(format!("uncovered/overlayered cue {} key {}", binding.action, note.key).into()); }
                maximum = maximum.max(count); checked += 1;
            }
        }
    }
    println!("covered_notes={checked} maximum_layers_per_note={maximum}");
    Ok(())
}
