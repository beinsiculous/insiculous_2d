//! The samples one song plays: each sample path the song names, with its decoded audio,
//! held once however many instruments name it.
//!
//! The loader decodes each unique sample once, before anything renders, and supplies the
//! results to [`Song::new`](crate::Song::new); the bank keeps those the song names, and
//! every sample slot built from it shares the one decoded buffer.

use std::collections::BTreeMap;

use crate::pcm::OwnedPcm;
use crate::song_file::{InstrumentSource, SongFile};

/// See the module docs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SampleBank {
    samples: BTreeMap<String, OwnedPcm>,
}

impl SampleBank {
    /// The samples `file`'s instruments name, taken from `supplied` (path → decoded audio,
    /// as the sidecar names them); the first supplied for a path wins, and a sample the
    /// song does not name is left out.
    pub fn new(file: &SongFile, supplied: &[(&str, OwnedPcm)]) -> Self {
        let mut samples = BTreeMap::new();
        for instrument in &file.instruments {
            let InstrumentSource::Sample { path, .. } = &instrument.source else {
                continue;
            };
            if samples.contains_key(path) {
                continue;
            }
            if let Some((_, pcm)) = supplied.iter().find(|(supplied, _)| supplied == path) {
                samples.insert(path.clone(), pcm.clone());
            }
        }
        Self { samples }
    }

    /// The decoded audio of the sample at `path`, as the sidecar names it.
    pub fn get(&self, path: &str) -> Option<&OwnedPcm> {
        self.samples.get(path)
    }

    /// Each sample's length in seconds by path, as
    /// [`ResourceMetadata::sample_seconds`](crate::ResourceMetadata::sample_seconds) holds it.
    pub fn seconds(&self) -> BTreeMap<String, f64> {
        self.samples
            .iter()
            .map(|(path, pcm)| (path.clone(), pcm.seconds()))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}
