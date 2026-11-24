//! Replace a song's bank through the same edit/save path as the Conductor panel.
use std::{error::Error, fs, path::Path, sync::Arc};
use editor::song::{Gesture, SongDocument, SongEdit};
use music_midi::{Conductor, ConductorConfig, InstrumentSource, Provenance, Song};

#[path = "../../music_midi/examples/audition/coverage.rs"]
mod coverage;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 6 {
        return Err("usage: remap_soundfont ASSETS MIDI BANK AUTHOR LICENSE".into());
    }
    let assets = Path::new(&arguments[1]);
    let path = assets.join(&arguments[2]);
    remap(assets, &path, &arguments[3], &arguments[4], &arguments[5])
}

fn remap(assets: &Path, path: &Path, bank: &str, author: &str, license: &str) -> Result<(), Box<dyn Error>> {
    if !editor::external_tools::midi_sidecar_path(path).is_file() {
        return Err("remapping requires an existing sidecar".into());
    }
    let opened = SongDocument::open(path, assets)?;
    for note in opened.notes { eprintln!("{note}"); }
    let mut document = opened.document;
    if document.file().instruments.iter().any(|instrument| matches!(instrument.source, InstrumentSource::Sample { .. })) {
        return Err("remap_soundfont supports SoundFont-only maps; use the Conductor panel for mixed maps".into());
    }
    let actions = document.file().bindings.clone();
    let mut edits = vec![SongEdit::SetSoundFont { path: bank.to_owned() },
        SongEdit::SetProvenance { provenance: Provenance { author: author.to_owned(), license: license.to_owned() } }];
    for instrument in &document.file().instruments {
        let mut replacement = instrument.clone();
        if let InstrumentSource::SoundFont { path, .. } = &mut replacement.source {
            *path = bank.to_owned();
            edits.push(SongEdit::SetInstrument { name: instrument.name.clone(), instrument: replacement });
        }
    }
    document.apply(SongEdit::Batch(edits), Gesture::Discrete)?;
    if document.file().bindings != actions { return Err("bindings changed".into()); }
    validate_candidate(&document, assets, bank)?;
    let validated_score = document.score_fingerprint();
    for note in document.save(path, assets)? { eprintln!("{note}"); }
    if document.score_fingerprint() != validated_score {
        // Save may adopt an external MIDI export; never report success on an unchecked score.
        validate_candidate(&document, assets, bank).map_err(|error|
            format!("MIDI changed during save; the sidecar was already written, but the adopted score failed validation: {error}. Stop concurrent exports and remap the corrected score."))?;
    }
    println!("saved {} with {} unchanged bindings", path.display(), actions.len());
    Ok(())
}


fn validate_candidate(document: &SongDocument, assets: &Path, bank: &str) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(assets.join(bank))?;
    let candidate = Song::new(document.file().clone(), document.data().clone(), &[(bank, &bytes)], &[])?;
    coverage::validate(&candidate)?;
    let (mut conductor, _) = Conductor::new(ConductorConfig::new(44100))?;
    for warning in conductor.load(Arc::new(candidate))?.warnings { eprintln!("load warning: {warning:?}"); }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use music_midi::{Binding, Instrument, Part, PartMode, Quantize, TrackSelector};

    #[test]
    fn mixed_map_refusal_preserves_both_files() -> Result<(), Box<dyn Error>> {
        let folder = tempfile::tempdir()?;
        let assets = folder.path();
        fs::create_dir_all(assets.join("music"))?;
        let path = assets.join("music/ai_demo.mid");
        let sidecar = editor::external_tools::midi_sidecar_path(&path);
        let midi = include_bytes!("../../../examples/assets/music/ai_demo.mid");
        let map = include_bytes!("../../../examples/assets/music/ai_demo.mid.ron");
        fs::write(&path, midi)?;
        fs::write(&sidecar, map)?;
        let error = remap(assets, &path, "sine.sf2", "Test", "CC0").err().ok_or("expected refusal")?;
        assert!(error.to_string().contains("SoundFont-only maps"));
        assert_eq!(fs::read(&path)?, midi);
        assert_eq!(fs::read(&sidecar)?, map);
        Ok(())
    }

    #[test]
    fn covered_bank_remap_keeps_the_score() -> Result<(), Box<dyn Error>> {
        let folder = tempfile::tempdir()?;
        let assets = folder.path();
        fs::create_dir_all(assets.join("music"))?;
        let path = assets.join("music/song.mid");
        let midi = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk\0\0\0\x0c\0\x90\x3c\x64\x60\x80\x3c\0\0\xff\x2f\0";
        fs::write(&path, midi)?;
        let mut document = SongDocument::open(&path, assets)?.document;
        document.apply(SongEdit::SetProvenance { provenance: Provenance {
            author: "Test fixture".into(), license: "CC0-1.0".into(),
        } }, Gesture::Discrete)?;
        document.save(&path, assets)?;
        document.apply(SongEdit::Batch(vec![
            SongEdit::AddPart { part: Part { name: "contact".into(), track: TrackSelector::Index(0), mode: PartMode::Loop, quantize: Quantize::Off } },
            SongEdit::AddInstrument { instrument: Instrument { name: "tone".into(), provenance: None, source: InstrumentSource::SoundFont { path: "old.sf2".into(), bank: 0, preset: 0 } } },
            SongEdit::SetBinding { binding: Binding { action: "hit".into(), part: "contact".into(), instrument: "tone".into() } },
        ]), Gesture::Discrete)?;
        document.save(&path, assets)?;
        let before = fs::read(&path)?;
        fs::write(assets.join("sine.sf2"), include_bytes!("../../music_midi/fixtures/sine.sf2"))?;
        remap(assets, &path, "sine.sf2", "Replacement", "CC0-1.0")?;
        assert_eq!(fs::read(&path)?, before);
        let reopened = SongDocument::open(&path, assets)?.document;
        assert_eq!(reopened.file().soundfont, "sine.sf2");
        assert_eq!(reopened.file().bindings, document.file().bindings);
        let mut invalid = reopened;
        invalid.apply(SongEdit::AddInstrument { instrument: Instrument {
            name: "unused".into(), provenance: None,
            source: InstrumentSource::SoundFont { path: "sine.sf2".into(), bank: 0, preset: 1 },
        } }, Gesture::Discrete)?;
        invalid.save(&path, assets)?;
        let sidecar = editor::external_tools::midi_sidecar_path(&path);
        let before_map = fs::read(&sidecar)?;
        let error = remap(assets, &path, "sine.sf2", "Test", "CC0").err().ok_or("expected missing preset")?;
        assert!(error.to_string().contains("preset"));
        assert_eq!(fs::read(&sidecar)?, before_map);
        Ok(())
    }

    #[test]
    fn failed_bank_remap_preserves_midi_and_sidecar() -> Result<(), Box<dyn Error>> {
        let folder = tempfile::tempdir()?;
        let assets = folder.path();
        fs::create_dir_all(assets.join("music"))?;
        let path = assets.join("music/song.mid");
        let midi = include_bytes!("../../music_midi/fixtures/dansenapolitaine.mid");
        fs::write(&path, midi)?;
        let mut document = SongDocument::open(&path, assets)?.document;
        document.apply(SongEdit::SetProvenance { provenance: Provenance {
            author: "Test fixture".into(), license: "CC0-1.0".into(),
        } }, Gesture::Discrete)?;
        document.save(&path, assets)?;
        let sidecar = editor::external_tools::midi_sidecar_path(&path);
        let before_midi = fs::read(&path)?;
        let before_map = fs::read(&sidecar)?;
        for bank in ["missing.sf2", "invalid.sf2"] {
            if bank == "invalid.sf2" { fs::write(assets.join(bank), b"not a bank")?; }
            assert!(remap(assets, &path, bank, "Test", "CC0").is_err());
            assert_eq!(fs::read(&path)?, before_midi);
            assert_eq!(fs::read(&sidecar)?, before_map);
        }
        let mut wrong_preset = include_bytes!("../../music_midi/fixtures/sine.sf2").to_vec();
        let header = wrong_preset.windows(4).position(|bytes| bytes == b"phdr").ok_or("fixture preset header")?;
        wrong_preset[header + 28] = 1;
        let decoded = rustysynth::SoundFont::new(&mut std::io::Cursor::new(&wrong_preset))?;
        assert!(decoded.get_presets().iter().all(|preset| preset.get_patch_number() != 0));
        fs::write(assets.join("sine.sf2"), wrong_preset)?;
        assert!(remap(assets, &path, "sine.sf2", "Test", "CC0").is_err(), "program zero is absent from the valid bank");
        assert_eq!(fs::read(&sidecar)?, before_map);
        fs::remove_file(&sidecar)?;
        assert!(remap(assets, &path, "missing.sf2", "Test", "CC0").is_err());
        assert!(!sidecar.exists());
        Ok(())
    }
}
