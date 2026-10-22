//! Where a save puts a song's samples, and the two `LICENSE` files it maintains: the sample
//! folder's, whose `Source:` header names the song that owns the folder, and the stanza
//! naming the song's own two files in its home folder's.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use music_midi::Provenance;

use super::{midi_fingerprint, slash_path, SongSaveError, SongStore};

const FORMAT_LINE: &str = "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/";

/// One sample the save puts in the song's folder.
pub(super) struct PlacedSample {
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub provenance: Provenance,
    /// Whether the folder already holds these bytes under that name.
    pub already_there: bool,
}

/// Choose the name a sample takes in the song's folder and add it to `samples` unless
/// it is there already: its own name (whitespace made `_`, which a `LICENSE` field
/// cannot hold) when that is free or holds the same bytes, otherwise the name with a
/// content-hash suffix.
pub(super) fn place_sample(
    store: &impl SongStore,
    sample_directory: &Path,
    samples: &mut Vec<PlacedSample>,
    source_path: &str,
    bytes: Vec<u8>,
    provenance: &Provenance,
) -> Result<String, SongSaveError> {
    let original = Path::new(source_path);
    let name_of = |name: Option<&std::ffi::OsStr>| -> String {
        name.map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sample".to_string())
            .chars()
            .map(|character| {
                if character.is_whitespace() {
                    '_'
                } else {
                    character
                }
            })
            .collect()
    };
    let plain = name_of(original.file_name());
    let suffixed = {
        let stem = name_of(original.file_stem());
        let hash = format!("{:08x}", midi_fingerprint(&bytes) as u32);
        match original.extension() {
            Some(extension) => format!("{stem}-{hash}.{}", extension.to_string_lossy()),
            None => format!("{stem}-{hash}"),
        }
    };
    for file_name in [plain, suffixed.clone()] {
        if let Some(placed) = samples.iter().find(|placed| placed.file_name == file_name) {
            if placed.bytes == bytes {
                return Ok(file_name);
            }
            continue;
        }
        let on_disk = store.read(&sample_directory.join(&file_name)).ok();
        if on_disk.as_ref().is_some_and(|existing| *existing != bytes) {
            continue;
        }
        samples.push(PlacedSample {
            file_name: file_name.clone(),
            already_there: on_disk.is_some(),
            bytes,
            provenance: provenance.clone(),
        });
        return Ok(file_name);
    }
    Err(SongSaveError::SampleNameTaken {
        directory: sample_directory.to_path_buf(),
        file_name: suffixed,
    })
}

/// A field's value on one line: a `LICENSE` field's continuation lines are its own syntax.
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The sample folder's `LICENSE` in the Debian copyright format the provenance gate reads:
/// a header whose `Source:` names `song`, the folder's owner, then one stanza per
/// referenced sample, its file named relative to the `LICENSE`.
pub(super) fn license_text(samples: &[PlacedSample], song: &str) -> String {
    let mut stanzas: BTreeMap<&str, &Provenance> = BTreeMap::new();
    for sample in samples {
        stanzas
            .entry(sample.file_name.as_str())
            .or_insert(&sample.provenance);
    }
    let mut text = format!(
        "{FORMAT_LINE}\nSource: {song}\n\
         Comment: Written by the composer's Save from each sample instrument's provenance,\n \
         and rewritten by every save. Edit the provenance in the song, not here.\n",
    );
    for (file_name, provenance) in stanzas {
        text.push_str(&format!(
            "\nFiles: {file_name}\nCopyright: {}\nLicense: {}\n",
            single_line(&provenance.author),
            single_line(&provenance.license),
        ));
    }
    text
}

/// The song named by the `Source:` field of a `LICENSE`'s header, if it has one.
fn source_of(license: &str) -> Option<String> {
    paragraphs(license)
        .first()?
        .iter()
        .find_map(|line| line.strip_prefix("Source:"))
        .map(|source| source.trim().to_string())
}

/// Refuse a save into a sample folder another song owns: one whose `LICENSE` names another
/// song, or one holding files with no owner named whose name is not `song`'s stem (the
/// layout from before folders named their owner, adopted only by its own song). A folder
/// that does not exist yet is free.
pub(super) fn check_folder_owner(
    store: &impl SongStore,
    sample_directory: &Path,
    song: &str,
    relative: &Path,
) -> Result<(), SongSaveError> {
    let license = store
        .read(&sample_directory.join("LICENSE"))
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    match license.as_deref().and_then(source_of) {
        Some(owner) if owner == song => Ok(()),
        Some(owner) => Err(SongSaveError::SampleFolderTaken {
            folder: sample_directory.to_path_buf(),
            owner,
            song: song.to_string(),
        }),
        None => {
            let listed = store.list_files(sample_directory).map_err(|error| {
                SongSaveError::Write {
                    path: sample_directory.to_path_buf(),
                    reason: format!("listing it failed: {error}"),
                }
            })?;
            if listed.is_empty() || sample_directory.file_name() == relative.file_stem() {
                Ok(())
            } else {
                Err(SongSaveError::SampleFolderUnowned {
                    folder: sample_directory.to_path_buf(),
                    song: song.to_string(),
                })
            }
        }
    }
}

/// The home folder's `LICENSE` with the stanza of the song at `relative` (under the asset
/// base, in `home`) written from `provenance`, or `None` when it already reads so. Its
/// `.mid` path alone finds the stanza — a stanza a person wrote naming only the `.mid`
/// included — which is rewritten to name both files; a stanza naming another song's files
/// as well keeps them and loses only this song's.
pub(super) fn home_license(
    store: &impl SongStore,
    home_directory: &Path,
    home: &str,
    relative: &Path,
    provenance: &Provenance,
) -> Result<Option<(PathBuf, String)>, SongSaveError> {
    let path = home_directory.join("LICENSE");
    let existing = match store.read(&path) {
        Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(SongSaveError::Write {
                path,
                reason: format!("reading it failed: {error}"),
            })
        }
    };
    let midi = slash_path(relative.strip_prefix(home).unwrap_or(relative));
    let text = rewrite_home_license(existing.as_deref(), &midi, provenance);
    Ok((existing.as_deref() != Some(text.as_str())).then_some((path, text)))
}

/// See [`home_license`]; `midi` is the song's `.mid` relative to the home folder.
fn rewrite_home_license(existing: Option<&str>, midi: &str, provenance: &Provenance) -> String {
    let sidecar = format!("{midi}.ron");
    let stanza = vec![
        format!("Files: {midi} {sidecar}"),
        format!("Copyright: {}", single_line(&provenance.author)),
        format!("License: {}", single_line(&provenance.license)),
    ];
    let mut kept = existing.map(paragraphs).unwrap_or_default();
    let has_header = kept
        .first()
        .is_some_and(|header| header.iter().any(|line| line.starts_with("Format:")));
    if !has_header {
        kept.insert(
            0,
            vec![
                FORMAT_LINE.to_string(),
                "Comment: Each song's stanza is written by the composer's Save from the song's".to_string(),
                " provenance and rewritten by every save; edit the provenance in the song.".to_string(),
                " Stanzas naming anything else are left as they are.".to_string(),
            ],
        );
    }
    let mut written = vec![kept.remove(0)];
    let mut placed = false;
    for paragraph in kept {
        let files = files_of(&paragraph);
        if !files.iter().any(|file| file == midi) {
            written.push(paragraph);
            continue;
        }
        let others: Vec<String> = files
            .into_iter()
            .filter(|file| *file != midi && *file != sidecar)
            .collect();
        if !others.is_empty() {
            written.push(with_files(&paragraph, &others));
        }
        if !placed {
            written.push(stanza.clone());
            placed = true;
        }
    }
    if !placed {
        written.push(stanza);
    }
    let mut text = written
        .iter()
        .map(|paragraph| paragraph.join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    text.push('\n');
    text
}

/// A `LICENSE`'s paragraphs, each its lines, split at blank lines.
fn paragraphs(text: &str) -> Vec<Vec<String>> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line.to_string());
        }
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }
    paragraphs
}

/// Whether `line` continues the field above it.
fn is_continuation(line: &str) -> bool {
    line.starts_with(' ') || line.starts_with('\t')
}

/// The paths of a paragraph's `Files:` field, its continuation lines included, split on
/// whitespace as the provenance gate splits them.
fn files_of(paragraph: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    let mut in_files = false;
    for line in paragraph {
        if let Some(first) = line.strip_prefix("Files:") {
            in_files = true;
            files.extend(first.split_whitespace().map(str::to_string));
        } else if in_files && is_continuation(line) {
            files.extend(line.split_whitespace().map(str::to_string));
        } else {
            in_files = false;
        }
    }
    files
}

/// The paragraph with its `Files:` field naming exactly `files`.
fn with_files(paragraph: &[String], files: &[String]) -> Vec<String> {
    let mut rewritten = Vec::new();
    let mut in_files = false;
    for line in paragraph {
        if line.starts_with("Files:") {
            in_files = true;
            rewritten.push(format!("Files: {}", files.join(" ")));
        } else if !(in_files && is_continuation(line)) {
            in_files = false;
            rewritten.push(line.clone());
        }
    }
    rewritten
}
