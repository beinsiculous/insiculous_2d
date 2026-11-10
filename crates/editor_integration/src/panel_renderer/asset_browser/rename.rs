//! Asset rename preparation and filesystem work. File operations have no undo.
//! Native file rename accepts the adjacent metadata-check and unlink syscalls' race with
//! an external replacement; on the web, destination puts precede removals and a duplicate is recovery.

use std::path::{Path, PathBuf};
use common::vfs;
use editor::{AssetKind, TileId, TileKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameRequest {
    pub tile: TileId,
    pub name: String,
    pub kind: AssetKind,
}

pub(crate) struct RenamePlan {
    pub source: TileId,
    pub destination: TileId,
    pub files: Vec<(String, String)>,
    /// Each LICENSE to rewrite: where it is read, where it is written after the move, the
    /// text the plan read, and the rewritten text — written only while the file still
    /// holds what was read.
    licenses: Vec<(PathBuf, PathBuf, String, String)>,
}

#[derive(Debug)]
pub(crate) struct MoveError {
    pub message: String,
    pub rescan: bool,
}

/// Validate the complete family and its provenance before any file moves.
pub(crate) fn prepare(base: &Path, request: &RenameRequest) -> Result<RenamePlan, String> {
    let source = &request.tile;
    super::remove::validate_tile(source)?;
    super::remove::refuse_sample(base, &source.path)?;
    let path = Path::new(&source.path);
    let lowered = source.path.to_ascii_lowercase();
    let extension = if source.kind == TileKind::Folder { "" } else if lowered.ends_with(".scene.ron") {
        &source.path[source.path.len() - ".scene.ron".len()..]
    } else { path.extension().and_then(|extension| extension.to_str()).unwrap_or("") };
    let name = editor::validate_rename(&request.name, request.kind, extension)?;
    // Browser ids join with `/` on every platform; `Path::join` would write `\` on Windows
    // and the renamed tile would match no id in the grid.
    let parent = source.path.rsplit_once('/').map_or("", |(parent, _)| parent);
    let in_parent = |name: &str| if parent.is_empty() { name.to_string() } else { format!("{parent}/{name}") };
    if source.kind == TileKind::Folder { editor::validate_folder(parent, &name)?; }
    let destination_path = in_parent(&name);
    if source.path == destination_path { return Err("Choose a different name".into()); }
    if vfs::exists(&base.join(&destination_path)) { return Err(format!("{destination_path} already exists; choose another name")); }
    let files = if source.kind == TileKind::Folder {
        let files = super::remove::files_under(base, &source.path)?;
        for file in &files { super::remove::refuse_sample(base, file).map_err(|error| format!("Cannot rename {}/: {error}", source.path))?; }
        files.into_iter().map(|file| {
            let suffix = file.strip_prefix(&source.path).unwrap_or("");
            (file.clone(), format!("{destination_path}{suffix}"))
        }).collect::<Vec<_>>()
    } else {
        if !vfs::exists(&base.join(&source.path)) { return Err(format!("{} no longer exists", source.path)); }
        // A folder put where the scanned file was would otherwise ride the directory branch of
        // the native rename, moving contents the plan and the host's guard never saw.
        if base.join(&source.path).is_dir() { return Err(format!("{} is a folder now, not a file; Rescan", source.path)); }
        let old_stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or("The asset has no filename")?;
        let destination = Path::new(&destination_path);
        let new_stem = destination.file_stem().and_then(|stem| stem.to_str()).ok_or("The target has no filename")?;
        let old_companion = editor::companion_path(&source.path);
        let new_companion = editor::companion_path(&destination_path);
        editor::asset_family(&base.join(&source.path).to_string_lossy())
            .map_err(|error| format!("Could not list {}'s family: {error}", source.path))?.into_iter().map(|file| {
            let relative = relative_path(base, Path::new(&file))?;
            let target = if relative == source.path { destination_path.clone() }
            else if old_companion.as_ref() == Some(&relative) { new_companion.clone().ok_or("The companion has no target")? }
            else {
                let filename = Path::new(&relative).file_name().and_then(|name| name.to_str()).ok_or("The recovered file has no name")?;
                let suffix = filename.strip_prefix(old_stem).ok_or("The recovered file does not belong to this family")?;
                in_parent(&format!("{new_stem}{suffix}"))
            };
            Ok((relative, target))
        }).collect::<Result<Vec<_>, String>>()?
    };
    if source.kind != TileKind::Folder {
        // A recovered copy or a companion — in any case of its suffix — already under the
        // new name would join the renamed asset's family, and its next delete would take
        // an unrelated file with it.
        let waiting: Vec<String> = editor::asset_family(&base.join(&destination_path).to_string_lossy())
            .map_err(|error| format!("Could not list {destination_path}'s folder: {error}"))?
            .into_iter().skip(1).filter_map(|file| relative_path(base, Path::new(&file)).ok()).collect();
        if let Some(file) = waiting.first() {
            return Err(format!("{file} already exists beside the new name; choose another name or delete it first"));
        }
    }
    for (old, new) in &files {
        if vfs::exists(&base.join(new)) { return Err(format!("{new} already exists; choose another name")); }
        // A sidecar already waiting at the new name would pair itself with the renamed asset
        // and hand it another sheet's clips or another song's map.
        if let Some(companion) = editor::companion_path(new).filter(|companion| vfs::exists(&base.join(companion))) {
            return Err(format!("{companion} already exists beside the new name; choose another name or delete it first"));
        }
        if editor::external_tools::is_midi_path(Path::new(old)) {
            editor::song::saveable_song_path(Path::new(new))?;
            if let Some(folder) = editor::song::sample_directory(Path::new(old)).filter(|folder| vfs::exists(&base.join(folder))) {
                return Err(format!("{old} keeps samples in {folder}/; rename it with the Conductor panel's Save As"));
            }
            // A sample folder left by an earlier song under the new name would be taken for the
            // renamed song's own at its next save, and the save's prune would empty it.
            if let Some(folder) = editor::song::sample_directory(Path::new(new)).filter(|folder| vfs::exists(&base.join(folder))) {
                return Err(format!("{folder}/ already holds another song's samples; choose another name or delete that folder first"));
            }
        }
    }
    let mut plan = RenamePlan { source: source.clone(), destination: TileId { kind: source.kind, path: destination_path }, files, licenses: Vec::new() };
    plan.licenses = prepare_licenses(base, &plan)?;
    Ok(plan)
}

pub(crate) fn relative_path(base: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(base).map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|_| format!("{} is outside the asset root", path.display()))
}

/// What the reference scan skips as the mutation's own: the named files, a deleted folder
/// with everything under it, or a renamed folder alone — its files move, and their
/// contents name the old paths still.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReferenceScope { Files, RenamedFolder, DeletedFolder }

/// An unsaved document as it would be written, under the label the confirm shows, or
/// `None` for one that could not be rendered — which counts as a reference, since nothing
/// proves it holds none.
pub(crate) type LiveText = (String, Option<String>);

/// Whether `text` names `path`: a file anywhere in the text (a reference may carry a
/// folder in front of it), a folder only where its path starts a path — `props/` inside
/// `backdrops/` is another folder, and a one-letter folder would otherwise match everything.
/// Every file under the asset root for the reference scan, and the folders that could not
/// be listed: one such folder must not veto every mutation in the project — the scan
/// reports it as a possible reference instead, and the folder's own delete can proceed.
fn files_under_leniently(base: &Path) -> (Vec<String>, Vec<String>) {
    let mut folders = vec![base.to_path_buf()];
    let (mut files, mut unlistable) = (Vec::new(), Vec::new());
    while let Some(folder) = folders.pop() {
        let Ok(entries) = vfs::list_entries(&folder) else {
            unlistable.extend(relative_path(base, &folder).ok());
            continue;
        };
        for entry in entries {
            if entry.is_dir { folders.push(entry.path); }
            else { files.extend(relative_path(base, &entry.path).ok()); }
        }
    }
    files.sort();
    unlistable.sort();
    (files, unlistable)
}

fn names_path(text: &str, path: &str, is_folder: bool) -> bool {
    if !is_folder {
        return text.contains(path);
    }
    let needle = format!("{path}/");
    text.match_indices(&needle).any(|(index, _)| {
        !text[..index].chars().next_back().is_some_and(|before| before.is_alphanumeric() || matches!(before, '_' | '-' | '.' | '/'))
    })
}

/// Every RON and Rhai file is scanned recursively for any of `old_paths` — the whole family
/// that moves, since a scene may name the sidecar or a recovered sibling alone — then each
/// `live` document, since the disk does not hold what the person is editing. The files being
/// moved or deleted are no references of themselves, and a file that cannot be read is
/// reported as one rather than refusing the mutation: refusing would lock the person out of
/// deleting that very file.
pub(crate) fn reference_hits(base: &Path, old_paths: &[&str], scope: ReferenceScope, live: &[LiveText]) -> Result<Vec<String>, String> {
    let (files, unlistable) = files_under_leniently(base);
    let is_folder = scope != ReferenceScope::Files;
    let names_any = |text: &str| old_paths.iter().any(|old_path| names_path(text, old_path, is_folder));
    // The mutated set is no reference of itself — except the files a folder rename moves,
    // whose contents still name the folder's old paths after the move.
    let is_mutated = |file: &str| old_paths.iter().any(|old_path| {
        file == *old_path || (scope == ReferenceScope::DeletedFolder && file.starts_with(&format!("{old_path}/")))
    });
    let mut hits: Vec<String> = live.iter()
        .filter(|(_, text)| text.as_ref().is_none_or(|text| names_any(text)))
        .map(|(label, _)| label.clone())
        .collect();
    hits.extend(unlistable.into_iter().filter(|folder| !is_mutated(folder)).map(|folder| format!("{folder}/ (could not be listed)")));
    for file in files {
        let extension = Path::new(&file).extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase);
        if is_mutated(&file) || !matches!(extension.as_deref(), Some("ron" | "rhai")) { continue; }
        match vfs::read_to_string(&base.join(&file)) {
            Ok(text) => if names_any(&text) { hits.push(file); },
            Err(error) => hits.push(format!("{file} (could not be read: {error})")),
        }
    }
    Ok(hits)
}

/// The LICENSE files whose stanzas may name the given project-relative files, as
/// `check_audio_provenance.sh` finds them: for a song, a recorded sound or a SoundFont, every
/// `LICENSE` from the file's own folder up to the first component — the gate takes the
/// nearest, and an absent or silent one is skipped at the read.
fn license_locations<'a>(files: impl Iterator<Item = &'a str>) -> std::collections::BTreeSet<PathBuf> {
    let mut locations = std::collections::BTreeSet::new();
    for file in files {
        let path = Path::new(file);
        let extension = path.extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase);
        let audio = editor::external_tools::is_midi_path(path)
            || editor::external_tools::sidecar_midi_path(path).is_some()
            || matches!(extension.as_deref(), Some("wav" | "ogg" | "mp3" | "flac" | "sf2"));
        if !audio { continue; }
        let mut folder = path.parent();
        while let Some(current) = folder.filter(|folder| !folder.as_os_str().is_empty()) {
            locations.insert(current.join("LICENSE"));
            folder = current.parent();
        }
    }
    locations
}

/// Each LICENSE naming one of `files`, with its text after `rewrite` — given the text, the
/// files relative to the LICENSE's folder, and that folder — and checked writable natively
/// before any file moves. A LICENSE that is absent, or whose stanzas name none of them, is
/// skipped.
pub(crate) fn license_rewrites(
    base: &Path,
    files: &[&str],
    must_be_writable: bool,
    rewrite: impl Fn(&str, &[&str], &Path) -> Option<String>,
) -> Result<Vec<(PathBuf, String, String)>, String> {
    let mut licenses = Vec::new();
    for relative in license_locations(files.iter().copied()) {
        // A LICENSE among the files themselves moves or goes with them: inside a renamed folder
        // its tokens stay relative to it, inside a deleted one it is deleted — and a
        // writability check on it would only refuse a read-only file about to be removed.
        if files.iter().any(|file| Path::new(file) == relative) { continue; }
        let source = base.join(&relative);
        let text = match vfs::read_to_string(&source) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("Could not read {}: {error}", source.display())),
        };
        let parent = relative.parent().unwrap_or(Path::new(""));
        let in_folder: Vec<String> = files.iter()
            .filter_map(|file| Path::new(file).strip_prefix(parent).ok().map(|file| file.to_string_lossy().replace('\\', "/")))
            .collect();
        let borrowed: Vec<&str> = in_folder.iter().map(String::as_str).collect();
        let Some(rewritten) = rewrite(&text, &borrowed, parent).filter(|rewritten| *rewritten != text) else { continue; };
        // A rename must land its stanza, so an unwritable LICENSE refuses it before the move;
        // a delete's files are gone either way, and its stanza left stale is the warning.
        if must_be_writable {
            #[cfg(not(target_arch = "wasm32"))]
            std::fs::OpenOptions::new().write(true).open(&source)
                .map_err(|error| format!("Cannot update {} before changing the files it names: {error}", source.display()))?;
        }
        licenses.push((source, text, rewritten));
    }
    Ok(licenses)
}

fn prepare_licenses(base: &Path, plan: &RenamePlan) -> Result<Vec<(PathBuf, PathBuf, String, String)>, String> {
    let olds: Vec<&str> = plan.files.iter().map(|(old, _)| old.as_str()).collect();
    let rewrites = license_rewrites(base, &olds, true, |text, in_folder, parent| {
        // A LICENSE inside a renamed folder moves with it, so its tokens stay relative to
        // the folder's new path; one above it sees the folder's new name in each token.
        let new_parent = if plan.source.kind == TileKind::Folder && parent.starts_with(&plan.source.path) {
            Path::new(&plan.destination.path).join(parent.strip_prefix(&plan.source.path).unwrap_or(Path::new("")))
        } else { parent.to_path_buf() };
        let pairs: Vec<(String, String)> = in_folder.iter().filter_map(|old_in_folder| {
            let old = parent.join(old_in_folder);
            plan.files.iter().find(|(file, _)| Path::new(file) == old).and_then(|(_, new)| {
                Path::new(new).strip_prefix(&new_parent).ok()
                    .map(|new_in_folder| (old_in_folder.to_string(), new_in_folder.to_string_lossy().replace('\\', "/")))
            })
        }).collect();
        let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(old, new)| (old.as_str(), new.as_str())).collect();
        editor::song::rename_in_license(text, &borrowed)
    })?;
    rewrites.into_iter().map(|(source, original, rewritten)| {
        let relative = source.strip_prefix(base).map_err(|error| error.to_string())?.to_path_buf();
        let destination = if plan.source.kind == TileKind::Folder && relative.starts_with(&plan.source.path) {
            base.join(&plan.destination.path).join(relative.strip_prefix(&plan.source.path).map_err(|error| error.to_string())?)
        } else { source.clone() };
        Ok((source, destination, original, rewritten))
    }).collect()
}

/// A failed later member rolls earlier members back; a failed rollback names both paths.
#[cfg(not(target_arch = "wasm32"))]
fn move_family_with(files: &[(PathBuf, PathBuf)], mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>) -> Result<(), MoveError> {
    for (index, (source, destination)) in files.iter().enumerate() {
        if let Err(error) = rename(source, destination) {
            let mut message = format!("Could not rename {} → {}: {error}", source.display(), destination.display());
            let mut rescan = vfs::exists(destination);
            for (old, new) in files[..index].iter().rev() {
                if let Err(rollback) = rename(new, old) {
                    message.push_str(&format!("; rollback {} → {} failed: {rollback}", new.display(), old.display()));
                    rescan = true;
                }
            }
            return Err(MoveError { message, rescan });
        }
    }
    Ok(())
}

/// Move the files, then their provenance. A provenance failure reports the completed move.
pub(crate) fn execute(base: &Path, plan: &RenamePlan) -> Result<Option<String>, MoveError> {
    if plan.source.kind == TileKind::Folder {
        // The folder moves whole, so its files must still be the ones the person confirmed
        // and the host's guard was told of.
        let now = super::remove::files_under(base, &plan.source.path).map_err(|message| MoveError { message, rescan: true })?;
        let planned: Vec<&str> = plan.files.iter().map(|(old, _)| old.as_str()).collect();
        if now.iter().map(String::as_str).ne(planned.iter().copied()) {
            return Err(MoveError { message: format!("the files under {}/ changed since the confirm; Rescan and try again", plan.source.path), rescan: true });
        }
        vfs::rename(&base.join(&plan.source.path), &base.join(&plan.destination.path))
            .map_err(|error| MoveError { message: error.to_string(), rescan: true })?;
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let paths: Vec<_> = plan.files.iter().map(|(old, new)| (base.join(old), base.join(new))).collect();
            move_family_with(&paths, |from, to| {
                if !std::fs::symlink_metadata(from)?.is_file() {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} is no longer a file", from.display())));
                }
                vfs::rename(from, to)
            })?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut written: Vec<PathBuf> = Vec::new();
            for (old, new) in &plan.files {
                let result = vfs::read(&base.join(old)).and_then(|bytes| vfs::write_new(&base.join(new), &bytes));
                if let Err(error) = result {
                    let mut message = format!("Could not create {new}: {error}");
                    let mut rescan = false;
                    for path in written { if let Err(rollback) = vfs::remove_file(&path) { message.push_str(&format!("; rollback {} failed: {rollback}", path.display())); rescan = true; } }
                    return Err(MoveError { message, rescan });
                }
                written.push(base.join(new));
            }
            for (old, new) in &plan.files {
                vfs::remove_file(&base.join(old)).map_err(|error| MoveError { message: format!("Created {new}, but could not remove {old}: {error}; both copies remain"), rescan: true })?;
            }
        }
    }
    for (_, destination, original, text) in &plan.licenses {
        let moved = || plan.files.iter().map(|(old, new)| format!("{old} → {new}")).collect::<Vec<_>>().join(", ");
        // Another program's edit to the LICENSE in the meantime is kept; the stanza is theirs
        // to carry then, and the warning says so.
        if vfs::read_to_string(destination).ok().as_deref() != Some(original.as_str()) {
            return Ok(Some(format!("Moved {}; {} changed meanwhile, so its stanza was left for you to update", moved(), destination.display())));
        }
        if let Err(error) = vfs::write(destination, text.as_bytes()) {
            return Ok(Some(format!("Moved {}; the LICENSE stanza was not updated ({}): {error}", moved(), destination.display())));
        }
    }
    Ok(None)
}

#[cfg(all(test, unix))]
mod tests;
