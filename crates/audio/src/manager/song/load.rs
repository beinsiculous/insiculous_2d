//! A song's dependency closure, loaded through `common::vfs`: the `.mid`, the `.mid.ron`
//! sidecar beside it, every SoundFont and every sample the sidecar names — each refused by
//! its size before it is read, then parsed, decoded and checked — into a [`Song`] its
//! conductor has already accepted.
//!
//! **Nothing is replaced until everything is loaded.** Every step that can fail runs
//! before the caller touches what is playing, in disabled mode too, so a missing sample, a
//! missing bank or an oversized file is an error naming the song, the sidecar entry and
//! the path, and the song already playing plays on.
//!
//! **Decoded dependencies are cached by content.** A SoundFont's parse (and its presets'
//! layering) and a sample's decode are kept per file under a fingerprint of the file's
//! bytes, so a second load of unchanged files decodes nothing and a file changed at the
//! same path is decoded again. The cache holds the current song's closure and the
//! previous one's; anything else is dropped after every load.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use music_midi::rustysynth::SoundFont;
use music_midi::{
    parse_song_file, preset_layering, Conductor, ConductorConfig, ConductorError, ConductorHandle,
    InstrumentSource, OwnedPcm, PresetKey, ResourceMetadata, SampleBank, Song, SongData, SongError,
    SongFile, SongFileError, MIDI_BYTE_LIMIT,
};

use crate::error::{AudioError, AudioResult};
use crate::sample_loader::{decode_sample_pcm, SampleProblem, SAMPLE_BYTE_LIMIT};

use super::SONG_SAMPLE_RATE;

/// The largest sidecar read, in bytes, checked before reading: a sidecar is a few
/// kilobytes of text.
pub const SONG_FILE_BYTE_LIMIT: usize = 256 * 1024;

/// The largest SoundFont read, in bytes, checked before reading: the conductor cast's
/// bank is 32.6 MiB.
pub const SOUND_FONT_BYTE_LIMIT: usize = 48 * 1024 * 1024;

/// Why one file of a song's closure cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum SongProblem {
    #[error("cannot be read: {0}")]
    Unreadable(#[source] io::Error),
    #[error("is {bytes} bytes; at most {limit} are read")]
    TooManyBytes { bytes: usize, limit: usize },
    #[error("is not UTF-8 text")]
    NotText,
    #[error("{0}")]
    Midi(#[from] SongError),
    #[error("{0}")]
    Sidecar(#[from] SongFileError),
    #[error("{0}")]
    Sample(#[from] SampleProblem),
    #[error("{0}")]
    Conductor(#[from] ConductorError),
}

/// A song built and loaded into its own conductor, ready to play.
pub(crate) struct ReadySong {
    /// The `.mid` path, or the label of a song built in memory.
    pub label: String,
    pub song: Arc<Song>,
    pub conductor: Conductor,
    pub handle: ConductorHandle,
}

impl ReadySong {
    /// Load `song` into a new conductor at the song sample rate. A refusal names `label`.
    pub(crate) fn new(label: &str, song: Arc<Song>) -> AudioResult<Self> {
        let refused = |error: ConductorError| song_error(label, "the song", label, error.into());
        let (mut conductor, handle) =
            Conductor::new(ConductorConfig::new(SONG_SAMPLE_RATE)).map_err(refused)?;
        let report = conductor.load(Arc::clone(&song)).map_err(refused)?;
        for warning in report.warnings {
            log::warn!("Song '{label}': {warning:?}");
        }
        Ok(Self {
            label: label.to_string(),
            song,
            conductor,
            handle,
        })
    }
}

/// The path of the sidecar beside a song's MIDI file: `<path>.ron`.
pub(crate) fn sidecar_path(midi_path: &Path) -> PathBuf {
    let mut sidecar = midi_path.as_os_str().to_owned();
    sidecar.push(".ron");
    PathBuf::from(sidecar)
}

/// Read the MIDI file at `midi_path` and its sidecar, parsed and nothing else checked.
pub(crate) fn read_song_files(midi_path: &Path) -> AudioResult<(SongFile, SongData)> {
    let label = midi_path.display().to_string();
    let midi_bytes = read_limited(midi_path, MIDI_BYTE_LIMIT)
        .map_err(|problem| song_error(&label, "the MIDI file", &label, problem))?;
    let (data, _) = SongData::from_smf(&midi_bytes)
        .map_err(|error| song_error(&label, "the MIDI file", &label, error.into()))?;
    let sidecar = sidecar_path(midi_path);
    let sidecar_label = sidecar.display().to_string();
    let sidecar_error = |problem| song_error(&label, "the sidecar", &sidecar_label, problem);
    let text = read_limited(&sidecar, SONG_FILE_BYTE_LIMIT)
        .and_then(|bytes| String::from_utf8(bytes).map_err(|_| SongProblem::NotText))
        .map_err(sidecar_error)?;
    let file =
        parse_song_file(&sidecar_label, &text).map_err(|error| sidecar_error(error.into()))?;
    Ok((file, data))
}

fn song_error(song: &str, entry: &str, path: &str, problem: SongProblem) -> AudioError {
    AudioError::Song {
        song: song.to_string(),
        entry: entry.to_string(),
        path: path.to_string(),
        problem: Box::new(problem),
    }
}

/// Read `path` whole, refusing it by its size, before reading, when it is over `limit`.
fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, SongProblem> {
    let size = common::vfs::file_size(path).map_err(SongProblem::Unreadable)?;
    if size > limit as u64 {
        return Err(SongProblem::TooManyBytes {
            bytes: usize::try_from(size).unwrap_or(usize::MAX),
            limit,
        });
    }
    common::vfs::read(path).map_err(SongProblem::Unreadable)
}

/// FNV-1a over `bytes`: a content fingerprint, so an edited file at an unchanged path is
/// told apart from the file the cache decoded.
fn fingerprint(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

/// Each preset's layering by bank and preset; the sidecar's path joins the key when a
/// song names the bank.
type PresetLayering = BTreeMap<(u16, u8), usize>;

enum Decoded {
    SoundFont {
        parsed: Arc<SoundFont>,
        layering: PresetLayering,
    },
    Sample(OwnedPcm),
}

/// A song's SoundFonts and samples, by the paths its sidecar names them.
#[derive(Default)]
struct Dependencies {
    sound_fonts: Vec<(String, Arc<SoundFont>)>,
    samples: Vec<(String, OwnedPcm)>,
}

impl Dependencies {
    fn named_sound_fonts(&self) -> Vec<(&str, Arc<SoundFont>)> {
        self.sound_fonts
            .iter()
            .map(|(path, parsed)| (path.as_str(), Arc::clone(parsed)))
            .collect()
    }

    fn named_samples(&self) -> Vec<(&str, OwnedPcm)> {
        self.samples
            .iter()
            .map(|(path, pcm)| (path.as_str(), pcm.clone()))
            .collect()
    }
}

struct CachedFile {
    fingerprint: u64,
    decoded: Decoded,
}

/// See the module docs.
#[derive(Default)]
pub(crate) struct DependencyCache {
    files: HashMap<PathBuf, CachedFile>,
    current_closure: BTreeSet<PathBuf>,
    previous_closure: BTreeSet<PathBuf>,
    decodes: u64,
}

/// Where a song's dependency paths lead, and what names the song in errors.
pub(crate) struct Closure<'a> {
    pub label: &'a str,
    pub asset_base: &'a Path,
}

impl DependencyCache {
    /// Files decoded since the cache was made: a cached file costs nothing.
    #[cfg(test)]
    pub(crate) fn decodes(&self) -> u64 {
        self.decodes
    }

    /// Build `file`'s song over `data`: every SoundFont and sample it names loaded through
    /// the cache from `closure.asset_base`, the resources filled in, the sidecar validated
    /// against them and the song accepted by a conductor.
    pub(crate) fn prepare(
        &mut self,
        closure: &Closure<'_>,
        mut file: SongFile,
        data: SongData,
    ) -> AudioResult<ReadySong> {
        let mut touched = BTreeSet::new();
        let result = self
            .resolve(closure, &mut file, &mut touched)
            .and_then(|loaded| {
                file.validate(closure.label, &data).map_err(|error| {
                    song_error(closure.label, "the sidecar", closure.label, error.into())
                })?;
                let song = Song::with_sound_fonts(
                    file,
                    data,
                    &loaded.named_sound_fonts(),
                    &loaded.named_samples(),
                )
                .map_err(|error| {
                    song_error(closure.label, "the song", closure.label, error.into())
                })?;
                ReadySong::new(closure.label, Arc::new(song))
            });
        if result.is_ok() {
            self.previous_closure = std::mem::replace(&mut self.current_closure, touched);
        }
        let (current, previous) = (&self.current_closure, &self.previous_closure);
        self.files
            .retain(|path, _| current.contains(path) || previous.contains(path));
        result
    }

    /// Load every dependency `file` names and record what was loaded in its resources.
    fn resolve(
        &mut self,
        closure: &Closure<'_>,
        file: &mut SongFile,
        touched: &mut BTreeSet<PathBuf>,
    ) -> AudioResult<Dependencies> {
        let mut resources = ResourceMetadata::default();
        let mut loaded = Dependencies::default();
        let mut font_entries = vec![("soundfont".to_string(), file.soundfont.clone())];
        for instrument in &file.instruments {
            let entry = format!("instruments['{}']", instrument.name);
            match &instrument.source {
                InstrumentSource::SoundFont { path, .. } => {
                    font_entries.push((entry, path.clone()));
                }
                InstrumentSource::Sample { path, .. } => {
                    if loaded.samples.iter().any(|(known, _)| known == path) {
                        continue;
                    }
                    let resolved = closure.asset_base.join(path);
                    touched.insert(resolved.clone());
                    let pcm = self
                        .sample(&resolved)
                        .map_err(|problem| song_error(closure.label, &entry, path, problem))?;
                    loaded.samples.push((path.clone(), pcm));
                }
            }
        }
        for (entry, path) in font_entries {
            if resources.soundfonts.contains(&path) {
                continue;
            }
            let resolved = closure.asset_base.join(&path);
            touched.insert(resolved.clone());
            let (parsed, layering) = self
                .sound_font(&resolved)
                .map_err(|problem| song_error(closure.label, &entry, &path, problem))?;
            for ((bank, preset), voices) in layering {
                let key = PresetKey {
                    soundfont: path.clone(),
                    bank,
                    preset,
                };
                resources.preset_layering.insert(key, voices);
            }
            resources.soundfonts.insert(path.clone());
            loaded.sound_fonts.push((path, parsed));
        }
        resources.sample_seconds = SampleBank::new(file, &loaded.named_samples()).seconds();
        file.resources = resources;
        Ok(loaded)
    }

    /// The bytes at `path`, refused by size first, and their fingerprint.
    fn read_fingerprinted(path: &Path, limit: usize) -> Result<(Vec<u8>, u64), SongProblem> {
        let bytes = read_limited(path, limit)?;
        let fingerprint = fingerprint(&bytes);
        Ok((bytes, fingerprint))
    }

    fn cached(&self, path: &Path, fingerprint: u64) -> Option<&Decoded> {
        self.files
            .get(path)
            .filter(|cached| cached.fingerprint == fingerprint)
            .map(|cached| &cached.decoded)
    }

    fn sound_font(&mut self, path: &Path) -> Result<(Arc<SoundFont>, PresetLayering), SongProblem> {
        let (bytes, fingerprint) = Self::read_fingerprinted(path, SOUND_FONT_BYTE_LIMIT)?;
        if let Some(Decoded::SoundFont { parsed, layering }) = self.cached(path, fingerprint) {
            return Ok((Arc::clone(parsed), layering.clone()));
        }
        let label = path.display().to_string();
        let parsed = Arc::new(music_midi::parse_sound_font(&label, &bytes)?);
        let layering: PresetLayering = preset_layering(&label, &parsed)
            .into_iter()
            .map(|(key, voices)| ((key.bank, key.preset), voices))
            .collect();
        let decoded = Decoded::SoundFont {
            parsed: Arc::clone(&parsed),
            layering: layering.clone(),
        };
        self.insert(path, fingerprint, decoded);
        Ok((parsed, layering))
    }

    fn sample(&mut self, path: &Path) -> Result<OwnedPcm, SongProblem> {
        let (bytes, fingerprint) = Self::read_fingerprinted(path, SAMPLE_BYTE_LIMIT)?;
        if let Some(Decoded::Sample(pcm)) = self.cached(path, fingerprint) {
            return Ok(pcm.clone());
        }
        let pcm = decode_sample_pcm(bytes)?;
        self.insert(path, fingerprint, Decoded::Sample(pcm.clone()));
        Ok(pcm)
    }

    fn insert(&mut self, path: &Path, fingerprint: u64, decoded: Decoded) {
        self.decodes += 1;
        self.files.insert(
            path.to_path_buf(),
            CachedFile {
                fingerprint,
                decoded,
            },
        );
    }
}
