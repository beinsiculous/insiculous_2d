//! Synthetic event mixes through the real conductor, buses and runtime limiter.
//! Producer events drain at the next block boundary, just as in runtime playback.
//! Exact preset coverage is required; fallback timbres would conceal authoring errors.
use std::{error::Error, fs, io::Write, path::Path, sync::Arc, time::Duration};
use common::clock::Instant;
use music_midi::{BusGains, Conductor, ConductorCommand, ConductorConfig, Song, SongData, SourceId, BLOCK_FRAMES};

#[path = "../../audio/src/conductor_source/limiter.rs"]
mod limiter;
#[path = "audition/coverage.rs"]
mod coverage;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 6 { return Err("usage: audition ASSETS MIDI EVENTS.tsv SECONDS OUTPUT.wav".into()); }
    let base = Path::new(&arguments[1]);
    let midi = base.join(&arguments[2]);
    let file = music_midi::parse_song_file("audition", &fs::read_to_string(format!("{}.ron", midi.display()))?)?;
    let (data, _) = SongData::from_smf(&fs::read(midi)?)?;
    let mut paths = vec![file.soundfont.clone()];
    for instrument in &file.instruments {
        if let music_midi::InstrumentSource::SoundFont { path, .. } = &instrument.source { paths.push(path.clone()); }
    }
    paths.sort(); paths.dedup();
    let banks: Vec<_> = paths.iter().map(|path| fs::read(base.join(path))).collect::<Result<_, _>>()?;
    let named: Vec<_> = paths.iter().zip(&banks).map(|(path, bytes)| (path.as_str(), bytes.as_slice())).collect();
    let song = Arc::new(Song::new(file, data, &named, &[])?);
    coverage::validate(&song)?;
    println!("background_peak={:?} end_tick={}", song.background_peak(), song.background().end_tick());
    let mut events = Vec::new();
    for line in fs::read_to_string(&arguments[3])?.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 3 { return Err("event requires seconds action source".into()); }
        let seconds: f64 = fields[0].parse()?;
        if !seconds.is_finite() || seconds < 0.0 { return Err("invalid event time".into()); }
        let binding = song.binding_index(fields[1]).ok_or("unknown action")?;
        events.push((seconds, binding, fields[2].parse::<u64>()?));
    }
    events.sort_by(|left, right| left.0.total_cmp(&right.0));
    let seconds: f64 = arguments[4].parse()?;
    if !seconds.is_finite() || seconds <= 0.0 || seconds > 600.0 { return Err("duration must be positive and at most 600 seconds".into()); }
    let rate = 44100;
    let frames = (seconds * f64::from(rate)) as usize;
    if frames == 0 || events.iter().any(|event| event.0 > ((frames - 1) / BLOCK_FRAMES * BLOCK_FRAMES) as f64 / f64::from(rate)) {
        return Err("duration must contain a frame and all events must precede the final block".into());
    }
    let (mut conductor, handle) = Conductor::new(ConductorConfig::new(rate))?;
    conductor.load(song)?;
    handle.send(ConductorCommand::Play)?;
    let mut limiter = limiter::Limiter::new(rate, BLOCK_FRAMES);
    let mut pcm = Vec::with_capacity(frames * 4);
    let mut event_index = 0;
    let mut peak = 0.0_f32;
    let mut squares = 0.0_f64;
    let mut nonzero = 0_u64;
    let mut clipped = 0_u64;
    let mut limited = 0_u64;
    let start = Instant::now();
    for offset in (0..frames).step_by(BLOCK_FRAMES) {
        let now = start + Duration::from_secs_f64(offset as f64 / f64::from(rate));
        while event_index < events.len() && events[event_index].0 <= offset as f64 / f64::from(rate) {
            let (time, binding, id) = events[event_index];
            handle.send(ConductorCommand::Trigger { binding, source: SourceId { id, generation: 0, epoch: 1 },
                stamp: start + Duration::from_secs_f64(time) })?;
            event_index += 1;
        }
        let count = BLOCK_FRAMES.min(frames - offset);
        let mut left = vec![0.0; count]; let mut right = vec![0.0; count];
        conductor.render_at(now, &mut left, &mut right, BusGains { music: 1.0, sfx: 1.0 });
        limited += limiter.apply(&mut left, &mut right);
        for (left, right) in left.iter().zip(&right) {
            if left.abs().max(right.abs()) > 0.00001 { nonzero += 1; }
            for sample in [left, right] {
                if !sample.is_finite() { return Err("nonfinite output".into()); }
                peak = peak.max(sample.abs()); squares += f64::from(*sample).powi(2);
                if sample.abs() >= 1.0 { clipped += 1; }
                pcm.extend_from_slice(&((*sample * 32767.0).round() as i16).to_le_bytes());
            }
        }
    }
    let mut output = fs::File::create(&arguments[5])?;
    output.write_all(b"RIFF")?; output.write_all(&(36 + pcm.len() as u32).to_le_bytes())?;
    output.write_all(b"WAVEfmt \x10\0\0\0\x01\0\x02\0")?;
    output.write_all(&rate.to_le_bytes())?; output.write_all(&(rate * 4).to_le_bytes())?;
    output.write_all(b"\x04\0\x10\0data")?; output.write_all(&(pcm.len() as u32).to_le_bytes())?; output.write_all(&pcm)?;
    println!("peak={peak:.6} rms={:.6} nonzero_frames={nonzero} clipped={clipped} limited_samples={limited} events={event_index}", (squares / (frames * 2) as f64).sqrt());
    println!("{:?}", handle.position().snapshot());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn preview_uses_runtime_limiter() {
        let mut limiter = super::limiter::Limiter::new(44100, 64);
        let mut left = [2.0; 64];
        let mut right = [2.0; 64];
        assert_eq!(limiter.apply(&mut left, &mut right), 128);
        assert!(limiter.gain() <= 0.45);
    }
}
