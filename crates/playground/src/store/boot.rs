//! What a Playground boot reads from the store before the editor starts, and when it
//! stops instead of opening anything.
//!
//! Every stop is decided before the first write — the orphan sweep, the removal of an
//! interrupted import, the redirect away from an unknown project — so a boot that
//! cannot read the store leaves every record as it found it. After a move to a window
//! the destination's boot is the only reader of the project, so a read it cannot make
//! is never taken for "nothing stored".

use crate::projects::ProjectManifest;
use crate::store::{stored_files_for_boot, ManifestList, ProjectStore, StoredFile};

/// The startup-failed status of a hand-off's destination whose browser storage will
/// not open: the saved project is untouched in the store, and falling back to memory
/// would open the bundled project in its place.
pub(crate) const MOVE_BOOT_STORAGE_REFUSAL: &str = "This window could not open your saved project. \
     Close it and choose \"Bring the editor back here\" on the page.";

/// The slug opened when the bundled list names none.
const FALLBACK_SLUG: &str = "examples";

/// Whether the URL's `move` parameter marks a boot sent here by a hand-off: *Open in a
/// window*, *Bring the editor back here*, or the recovery navigation after a drain.
pub(crate) fn is_move_boot(move_parameter: Option<&str>) -> bool {
    move_parameter == Some("1")
}

/// The refusal a boot answers when its persistent store will not open, or `None` when
/// it may fall back to the in-memory store as an ordinary boot does.
pub(crate) fn memory_fallback_refusal(is_move_boot: bool) -> Option<&'static str> {
    is_move_boot.then_some(MOVE_BOOT_STORAGE_REFUSAL)
}

/// Which project the URL opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProjectChoice {
    /// Open this slug.
    Open(String),
    /// The requested slug is unknown: navigate to this bundled one instead.
    Redirect(String),
}

/// Choose the project from the `?project=` parameter, or the stop that names why not.
///
/// An unknown slug is redirected to the first bundled project only when every stored
/// manifest decoded: while one did not, the requested slug may be that one, and a
/// redirect would abandon an imported project the visitor asked for by name.
pub(crate) fn choose_project(
    bundled: &[ProjectManifest],
    stored: &ManifestList,
    requested: Option<&str>,
) -> Result<ProjectChoice, String> {
    let first_bundled = bundled
        .first()
        .map(|manifest| manifest.slug.clone())
        .unwrap_or_else(|| FALLBACK_SLUG.to_string());
    let Some(slug) = requested else {
        return Ok(ProjectChoice::Open(first_bundled));
    };
    let is_bundled = bundled.iter().any(|manifest| manifest.slug == slug);
    let is_stored = stored.manifests.iter().any(|manifest| manifest.slug == slug);
    if is_bundled || is_stored {
        Ok(ProjectChoice::Open(slug.to_string()))
    } else if !stored.undecodable.is_empty() {
        Err(format!(
            "a saved project's record could not be read, and it may be \"{slug}\"; nothing was changed"
        ))
    } else {
        Ok(ProjectChoice::Redirect(first_bundled))
    }
}

/// The slugs the orphan sweep keeps: every bundled one and every decoded manifest's.
pub(crate) fn known_slugs(bundled: &[ProjectManifest], stored: &ManifestList) -> Vec<String> {
    bundled
        .iter()
        .chain(stored.manifests.iter())
        .map(|manifest| manifest.slug.clone())
        .collect()
}

/// What the boot goes on to do.
#[derive(Debug)]
pub(crate) enum BootPlan {
    /// Open a project with these stored records.
    Open(OpenedProject),
    /// Navigate to this bundled slug; nothing is opened on this page.
    Redirect(String),
}

/// The project a boot opens, as the store holds it.
#[derive(Debug)]
pub(crate) struct OpenedProject {
    pub slug: String,
    /// The project's own stored manifest, when one decoded.
    pub stored_manifest: Option<ProjectManifest>,
    /// Its stored records, tombstones included; empty when nothing is stored.
    pub stored_files: Vec<StoredFile>,
    /// Every decoded stored manifest, for the project picker.
    pub stored_manifests: Vec<ProjectManifest>,
}

/// What the boot's reads decided, before anything is written.
enum ReadOutcome {
    Redirect(String),
    Open {
        slug: String,
        stored_manifest: Option<ProjectManifest>,
        /// `None` for an interrupted import: stored files with no manifest.
        kept_records: Option<Vec<StoredFile>>,
    },
}

/// Read the store for a boot and decide what it opens.
///
/// Stops with an `Err` — the page's "Startup failed" status — before any write, on: a
/// failed manifest read; the opened project's own manifest record undecodable, bundled
/// or not, files or none; a failed or undecodable record of the opened project; an
/// unknown slug while a manifest would not decode; stored files for the opened slug
/// with no manifest while a manifest would not decode. Then the sweep runs over the
/// known slugs, only when every manifest decoded, and its failure stops the boot too;
/// an interrupted import — stored files with no manifest in a complete list — is
/// removed last, and a failed removal only warns, as the page then opens the bundle.
pub(crate) async fn plan_boot(
    store: &dyn ProjectStore,
    bundled: &[ProjectManifest],
    requested: Option<&str>,
) -> Result<BootPlan, String> {
    let stored = store
        .manifests()
        .await
        .map_err(|error| format!("the saved projects could not be read ({error}); nothing was changed"))?;
    let outcome = match choose_project(bundled, &stored, requested)? {
        ProjectChoice::Redirect(first_bundled) => ReadOutcome::Redirect(first_bundled),
        ProjectChoice::Open(slug) => {
            // Opened with nothing stored, a save would write files beside a record `put`
            // takes for a manifest, and every later boot would stop on them.
            if stored.undecodable.contains(&slug) {
                return Err(format!(
                    "the project record of \"{slug}\" could not be read; nothing was changed"
                ));
            }
            let stored_manifest = stored.manifests.iter().find(|manifest| manifest.slug == slug).cloned();
            let records = store.load_project(&slug).await.map_err(|error| {
                format!("the saved files of \"{slug}\" could not be read ({error}); nothing was changed")
            })?;
            let kept_records = match stored_files_for_boot(records, stored_manifest.is_some()) {
                Some(records) => Some(records),
                None if !stored.undecodable.is_empty() => {
                    return Err(format!(
                        "\"{slug}\" has saved files but its project record could not be read; nothing was changed"
                    ));
                }
                None => None,
            };
            ReadOutcome::Open { slug, stored_manifest, kept_records }
        }
    };

    if stored.undecodable.is_empty() {
        store
            .sweep_orphans(&known_slugs(bundled, &stored))
            .await
            .map_err(|error| format!("the saved projects could not be tidied ({error})"))?;
    } else {
        log::warn!(
            "not sweeping orphaned files: the project records {} could not be decoded",
            stored.undecodable.join(", ")
        );
    }

    let (slug, stored_manifest, kept_records) = match outcome {
        ReadOutcome::Redirect(first_bundled) => {
            log::warn!("requested project {requested:?} is unknown; redirecting to '{first_bundled}'");
            return Ok(BootPlan::Redirect(first_bundled));
        }
        ReadOutcome::Open { slug, stored_manifest, kept_records } => (slug, stored_manifest, kept_records),
    };
    let stored_files = match kept_records {
        Some(records) => records,
        None => {
            log::warn!("project '{slug}' has stored files but no stored manifest (an interrupted import); removing them and loading bundled only");
            if let Err(error) = store.remove_project(&slug).await {
                log::warn!("could not remove the manifest-less files of '{slug}': {error}");
            }
            Vec::new()
        }
    };
    Ok(BootPlan::Open(OpenedProject {
        slug,
        stored_manifest,
        stored_files,
        stored_manifests: stored.manifests,
    }))
}
