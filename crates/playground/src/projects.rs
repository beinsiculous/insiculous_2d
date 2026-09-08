//! Project manifests, discovery, and path construction for the playground.

use serde::{Deserialize, Serialize};

/// Origin of a project in the playground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectOrigin {
    /// Pre-packaged with the engine bundle.
    Bundled,
    /// Created or modified locally by user saves.
    Saved,
    /// Imported from an archive zip.
    Imported,
}

/// Metadata describing a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectManifest {
    /// Unique project identifier, matching `^[a-z0-9_-]{1,32}$`.
    pub slug: String,
    /// User-facing project title.
    pub title: String,
    /// Bundle version under which this manifest was written (e.g. "v1").
    pub bundle_version: String,
    /// Content hash computed by the bundle script (or empty for user projects).
    pub content_hash: String,
    /// Project provenance.
    pub origin: ProjectOrigin,
}

/// Merged project entry for display in the playground UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectEntry {
    /// Active manifest (stored manifest wins over bundled if modified).
    pub manifest: ProjectManifest,
    /// Whether this project slug exists in the bundled package.
    pub is_bundled: bool,
    /// Whether this project has saved files in local storage.
    pub has_stored_files: bool,
    /// Whether stored content hash or bundle version differs from the bundled one.
    pub differs_from_bundle: bool,
}

/// Validate that a slug matches `^[a-z0-9_-]{1,32}$`.
pub fn validate_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 32 {
        return false;
    }
    slug.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
    })
}

/// Compute the base-joined canonical VFS project root.
pub fn project_root(base: &str, slug: &str) -> String {
    let base = base.trim_end_matches('/');
    format!("{base}/projects/{slug}")
}

/// Compute the asset base path under a project root.
pub fn project_asset_base(root: &str) -> String {
    let root = root.trim_end_matches('/');
    format!("{root}/assets")
}

/// Merge bundled project manifests with stored project manifests.
///
/// If a project exists in both, the stored manifest takes precedence.
/// Returns entries in bundled order first, followed by any stored-only
/// projects sorted by slug.
pub fn list_projects(
    bundled: &[ProjectManifest],
    stored: &[ProjectManifest],
) -> Vec<ProjectEntry> {
    let mut entries = Vec::new();
    let mut handled_slugs = std::collections::HashSet::new();

    for bundled_manifest in bundled {
        handled_slugs.insert(bundled_manifest.slug.clone());
        if let Some(stored_manifest) = stored.iter().find(|manifest| manifest.slug == bundled_manifest.slug) {
            let differs = stored_manifest.content_hash != bundled_manifest.content_hash || stored_manifest.bundle_version != bundled_manifest.bundle_version;
            entries.push(ProjectEntry {
                manifest: stored_manifest.clone(),
                is_bundled: true,
                has_stored_files: true,
                differs_from_bundle: differs,
            });
        } else {
            entries.push(ProjectEntry {
                manifest: bundled_manifest.clone(),
                is_bundled: true,
                has_stored_files: false,
                differs_from_bundle: false,
            });
        }
    }

    let mut stored_only: Vec<&ProjectManifest> = stored
        .iter()
        .filter(|stored_manifest| !handled_slugs.contains(&stored_manifest.slug))
        .collect();
    stored_only.sort_by(|first, second| first.slug.cmp(&second.slug));

    for stored_manifest in stored_only {
        entries.push(ProjectEntry {
            manifest: (*stored_manifest).clone(),
            is_bundled: false,
            has_stored_files: true,
            differs_from_bundle: false,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_slug_accepts_valid_and_rejects_invalid() {
        assert!(validate_slug("pong"));
        assert!(validate_slug("my-game_123"));
        assert!(validate_slug("a"));
        assert!(validate_slug("12345678901234567890123456789012")); // 32 chars

        assert!(!validate_slug("")); // empty
        assert!(!validate_slug("123456789012345678901234567890123")); // 33 chars
        assert!(!validate_slug("Pong")); // uppercase
        assert!(!validate_slug("my/game")); // slash
        assert!(!validate_slug("my game")); // space
        assert!(!validate_slug("game!")); // symbol
    }

    #[test]
    fn test_project_paths_formatting() {
        assert_eq!(
            project_root("/playground/v1/assets", "pong"),
            "/playground/v1/assets/projects/pong"
        );
        assert_eq!(
            project_asset_base("/playground/v1/assets/projects/pong"),
            "/playground/v1/assets/projects/pong/assets"
        );
    }

    #[test]
    fn test_list_projects_merging() {
        let bundled = vec![
            ProjectManifest {
                slug: "examples".to_string(),
                title: "Examples".to_string(),
                bundle_version: "v1".to_string(),
                content_hash: "hash_orig".to_string(),
                origin: ProjectOrigin::Bundled,
            },
            ProjectManifest {
                slug: "pong".to_string(),
                title: "Pong".to_string(),
                bundle_version: "v1".to_string(),
                content_hash: "hash_pong".to_string(),
                origin: ProjectOrigin::Bundled,
            },
        ];

        let stored = vec![
            ProjectManifest {
                slug: "examples".to_string(),
                title: "Examples Modified".to_string(),
                bundle_version: "v1".to_string(),
                content_hash: "hash_new".to_string(),
                origin: ProjectOrigin::Saved,
            },
            ProjectManifest {
                slug: "custom".to_string(),
                title: "Custom Project".to_string(),
                bundle_version: "v1".to_string(),
                content_hash: "".to_string(),
                origin: ProjectOrigin::Imported,
            },
        ];

        let entries = list_projects(&bundled, &stored);
        assert_eq!(entries.len(), 3);

        // First is examples: shadowed by stored, differs_from_bundle is true
        assert_eq!(entries[0].manifest.slug, "examples");
        assert_eq!(entries[0].manifest.title, "Examples Modified");
        assert!(entries[0].is_bundled);
        assert!(entries[0].has_stored_files);
        assert!(entries[0].differs_from_bundle);

        // Second is pong: untouched bundled
        assert_eq!(entries[1].manifest.slug, "pong");
        assert!(entries[1].is_bundled);
        assert!(!entries[1].has_stored_files);
        assert!(!entries[1].differs_from_bundle);

        // Third is custom: stored only
        assert_eq!(entries[2].manifest.slug, "custom");
        assert!(!entries[2].is_bundled);
        assert!(entries[2].has_stored_files);
        assert!(!entries[2].differs_from_bundle);
    }
}
