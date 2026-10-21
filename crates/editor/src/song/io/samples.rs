//! Where a save puts a song's samples, and the `LICENSE` it writes beside them.

use std::collections::BTreeMap;
use std::path::Path;

use music_midi::Provenance;

use super::{midi_fingerprint, SongSaveError, SongStore};

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

/// The song folder's `LICENSE` in the Debian copyright format the provenance gate reads:
/// one stanza per referenced sample, its file named relative to the `LICENSE`.
pub(super) fn license_text(samples: &[PlacedSample]) -> String {
    let single_line = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut stanzas: BTreeMap<&str, &Provenance> = BTreeMap::new();
    for sample in samples {
        stanzas
            .entry(sample.file_name.as_str())
            .or_insert(&sample.provenance);
    }
    let mut text = String::from(
        "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n\
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
