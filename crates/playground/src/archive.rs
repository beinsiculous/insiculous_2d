//! Project export and import archive handling for the playground.
//!
//! Provides target-agnostic zip archiving and extraction with strict
//! security and content validation.

use std::collections::HashSet;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use engine_core::prelude::World;
use engine_core::scene_loader::SceneLoader;
use engine_core::sheet_file::parse_sheet_file;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::projects::{validate_slug, ProjectManifest, ProjectOrigin};
use crate::store::StoredFile;

/// Maximum allowable size for an incoming project zip archive (64 MiB).
const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;

/// Maximum cumulative uncompressed byte budget across all entries (64 MiB).
const MAX_DECOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;

/// Errors returned by project archive export and import validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    /// The archive or its decompressed contents exceed the 64 MiB limit.
    TooLarge,
    /// Zip format or compression failure.
    Zip(String),
    /// Unsafe entry path (traversal, absolute path, or drive prefix).
    UnsafePath(String),
    /// Duplicate entry path in archive.
    DuplicateEntry(String),
    /// Entry outside allowed root (not project.ron and not under assets/).
    OutsideProject(String),
    /// The archive is missing a project.ron manifest.
    MissingManifest,
    /// The project.ron file could not be parsed as a ProjectManifest.
    InvalidManifest(String),
    /// Project slug in manifest failed slug validation.
    InvalidSlug(String),
    /// Sprite sheet sidecar file failed validation.
    InvalidSheet { entry: String, reason: String },
    /// Scene file failed parse or dry-run instantiation.
    InvalidScene { entry: String, reason: String },
    /// Input/output error while reading or writing archive bytes.
    Io(String),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(formatter, "archive or decompressed content exceeds 64 MiB limit"),
            Self::Zip(error) => write!(formatter, "zip error: {error}"),
            Self::UnsafePath(entry) => write!(formatter, "unsafe path in archive entry '{entry}'"),
            Self::DuplicateEntry(entry) => write!(formatter, "duplicate entry in archive: '{entry}'"),
            Self::OutsideProject(entry) => write!(
                formatter,
                "entry '{entry}' is outside project; entries must be project.ron or under assets/"
            ),
            Self::MissingManifest => write!(formatter, "archive is missing project.ron manifest"),
            Self::InvalidManifest(error) => write!(formatter, "invalid project.ron manifest: {error}"),
            Self::InvalidSlug(slug) => write!(formatter, "invalid project slug in manifest: '{slug}'"),
            Self::InvalidSheet { entry, reason } => {
                write!(formatter, "invalid sheet file in '{entry}': {reason}")
            }
            Self::InvalidScene { entry, reason } => {
                write!(formatter, "invalid scene file in '{entry}': {reason}")
            }
            Self::Io(error) => write!(formatter, "io error: {error}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

/// Export the open project into a zip archive.
///
/// Archives all files under `assets/`, plus the serialized `project.ron`
/// manifest and a generated `README.md`.
pub fn export_project(
    project_root: &Path,
    manifest: &ProjectManifest,
) -> Result<Vec<u8>, ArchiveError> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip_writer = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let files = common::vfs::list_files(project_root)
        .map_err(|error| ArchiveError::Io(error.to_string()))?;

    // Sorted so two exports of the same tree produce the same entry order.
    let mut asset_files: Vec<(String, PathBuf)> = Vec::new();
    for file_path in files {
        let relative_path = match file_path.strip_prefix(project_root) {
            Ok(relative) => relative,
            Err(_) => continue,
        };
        let relative_string = relative_path.to_string_lossy().replace('\\', "/");
        let clean_path = relative_string.trim_start_matches('/').to_string();
        if clean_path.starts_with("assets/") {
            asset_files.push((clean_path, file_path));
        }
    }
    asset_files.sort_by(|(path_first, _), (path_second, _)| path_first.cmp(path_second));

    for (entry_path, full_path) in asset_files {
        let file_bytes = common::vfs::read(&full_path)
            .map_err(|error| ArchiveError::Io(error.to_string()))?;
        zip_writer
            .start_file(&entry_path, options)
            .map_err(|error| ArchiveError::Zip(error.to_string()))?;
        zip_writer
            .write_all(&file_bytes)
            .map_err(|error| ArchiveError::Io(error.to_string()))?;
    }

    let ron_manifest = ron::ser::to_string_pretty(manifest, ron::ser::PrettyConfig::default())
        .map_err(|error| ArchiveError::InvalidManifest(error.to_string()))?;
    zip_writer
        .start_file("project.ron", options)
        .map_err(|error| ArchiveError::Zip(error.to_string()))?;
    zip_writer
        .write_all(ron_manifest.as_bytes())
        .map_err(|error| ArchiveError::Io(error.to_string()))?;

    let readme_content = format!(
        "# {}\n\nExported project archive for the Insiculous Web Playground.\n\nSee https://github.com/beinsiculous/insiculous_2d/blob/main/docs/WEB_PLAYGROUND.md for documentation on the project structure and editor usage.\n",
        manifest.title
    );
    zip_writer
        .start_file("README.md", options)
        .map_err(|error| ArchiveError::Zip(error.to_string()))?;
    zip_writer
        .write_all(readme_content.as_bytes())
        .map_err(|error| ArchiveError::Io(error.to_string()))?;

    zip_writer
        .finish()
        .map_err(|error| ArchiveError::Zip(error.to_string()))?;

    // The importer refuses an archive larger than this, so the producer refuses it first;
    // the importer's cap on DECOMPRESSED bytes has no counterpart here.
    if buffer.get_ref().len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveError::TooLarge);
    }

    Ok(buffer.into_inner())
}

/// Validate and import a project zip archive into stored files and manifest.
///
/// Refuses invalid paths, oversized content, and malformed scene/sheet files
/// before touching any store.
pub fn import_project(
    bytes: &[u8],
    bundle_version: &str,
) -> Result<(ProjectManifest, Vec<StoredFile>), ArchiveError> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveError::TooLarge);
    }

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|error| ArchiveError::Zip(error.to_string()))?;

    let mut seen_entries = HashSet::new();
    let mut total_decompressed_bytes: u64 = 0;
    let mut project_manifest_text: Option<String> = None;
    let mut asset_entries: Vec<(String, Vec<u8>)> = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| ArchiveError::Zip(error.to_string()))?;

        // Windows archivers write backslashes; `zip -r x.zip .` writes `./` first.
        let raw_name = entry.name().replace('\\', "/");
        let mut normalized_slice = raw_name.as_str();
        while let Some(stripped) = normalized_slice.strip_prefix("./") {
            normalized_slice = stripped;
        }

        // `zip -r` and Finder emit directory entries.
        if entry.is_dir() || normalized_slice.ends_with('/') || normalized_slice.is_empty() {
            continue;
        }

        if !crate::bridge::relative_path_is_safe(normalized_slice) {
            return Err(ArchiveError::UnsafePath(normalized_slice.to_string()));
        }

        // Stored paths are exact string keys, so `assets/./x` and `assets//x` must become
        // `assets/x` here or the file is unreachable under the name a scene uses.
        let canonical_name = Path::new(normalized_slice)
            .components()
            .filter_map(|component| match component {
                Component::Normal(segment) => segment.to_str(),
                _ => None,
            })
            .collect::<Vec<&str>>()
            .join("/");
        let normalized_slice = canonical_name.as_str();

        if normalized_slice == "README.md" {
            continue;
        }

        if !seen_entries.insert(normalized_slice.to_string()) {
            return Err(ArchiveError::DuplicateEntry(normalized_slice.to_string()));
        }

        if normalized_slice != "project.ron" && !normalized_slice.starts_with("assets/") {
            return Err(ArchiveError::OutsideProject(normalized_slice.to_string()));
        }

        let remaining_budget = MAX_DECOMPRESSED_BYTES.saturating_sub(total_decompressed_bytes);
        let mut entry_bytes = Vec::new();
        let mut limited_reader = (&mut entry).take(remaining_budget.saturating_add(1));
        limited_reader
            .read_to_end(&mut entry_bytes)
            .map_err(|error| ArchiveError::Io(error.to_string()))?;

        if entry_bytes.len() as u64 > remaining_budget {
            return Err(ArchiveError::TooLarge);
        }
        total_decompressed_bytes += entry_bytes.len() as u64;

        if normalized_slice == "project.ron" {
            let manifest_string = std::str::from_utf8(&entry_bytes)
                .map_err(|error| ArchiveError::InvalidManifest(error.to_string()))?
                .to_string();
            project_manifest_text = Some(manifest_string);
        } else {
            asset_entries.push((normalized_slice.to_string(), entry_bytes));
        }
    }

    let raw_manifest = project_manifest_text.ok_or(ArchiveError::MissingManifest)?;
    let parsed_manifest: ProjectManifest = ron::from_str(&raw_manifest)
        .map_err(|error| ArchiveError::InvalidManifest(error.to_string()))?;

    if !validate_slug(&parsed_manifest.slug) {
        return Err(ArchiveError::InvalidSlug(parsed_manifest.slug));
    }

    for (entry_name, entry_bytes) in &asset_entries {
        if entry_name.ends_with(".sheet.ron") {
            let sheet_text = std::str::from_utf8(entry_bytes).map_err(|error| {
                ArchiveError::InvalidSheet {
                    entry: entry_name.clone(),
                    reason: error.to_string(),
                }
            })?;
            parse_sheet_file(entry_name, sheet_text).map_err(|error| {
                ArchiveError::InvalidSheet {
                    entry: entry_name.clone(),
                    reason: error.to_string(),
                }
            })?;
        } else if entry_name.ends_with(".scene.ron") {
            let scene_text = std::str::from_utf8(entry_bytes).map_err(|error| {
                ArchiveError::InvalidScene {
                    entry: entry_name.clone(),
                    reason: error.to_string(),
                }
            })?;
            let scene_data = SceneLoader::parse(scene_text).map_err(|error| {
                ArchiveError::InvalidScene {
                    entry: entry_name.clone(),
                    reason: error.to_string(),
                }
            })?;
            let mut test_world = World::new();
            let mut test_assets = editor_integration::HeadlessAssets::new();
            SceneLoader::instantiate(&scene_data, &mut test_world, &mut test_assets).map_err(
                |error| ArchiveError::InvalidScene {
                    entry: entry_name.clone(),
                    reason: error.to_string(),
                },
            )?;
        }
    }

    let stored_files = asset_entries
        .into_iter()
        .map(|(path, file_data)| StoredFile {
            project: parsed_manifest.slug.clone(),
            path,
            bytes: file_data,
            revision: 1,
            bundle_version: bundle_version.to_string(),
        })
        .collect();

    let final_manifest = ProjectManifest {
        slug: parsed_manifest.slug,
        title: parsed_manifest.title,
        bundle_version: bundle_version.to_string(),
        content_hash: String::new(),
        origin: ProjectOrigin::Imported,
    };

    Ok((final_manifest, stored_files))
}

#[cfg(test)]
mod tests;
