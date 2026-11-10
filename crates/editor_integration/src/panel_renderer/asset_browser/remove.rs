//! Asset deletion preflight and file families, including a song's sample directory.

use std::path::{Component, Path, PathBuf};
use common::vfs;
use editor::{TileId, TileKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeletePlan {
    pub tile: TileId,
    pub files: Vec<String>,
    /// Every subfolder under a folder tile or a sample folder at the confirm, empty ones
    /// included: a folder that appeared since is kept with its contents.
    pub folders: Vec<String>,
    /// The sample folder of every song in the family — a recovered sibling's included.
    pub sample_folders: Vec<String>,
    /// The `.ron` and `.rhai` files naming the path, told in the confirm; they keep it.
    pub references: Vec<String>,
    /// Whether `references` includes the unsaved documents: the menu's entry points have no
    /// world, so the host rescans the plan with them the first time the confirm renders.
    pub live_scanned: bool,
    /// Each LICENSE whose stanza names a deleted file, with its text after the removal.
    /// Each LICENSE to rewrite, the text the plan read and the rewritten text — written only
    /// while the file still holds what was read.
    licenses: Vec<(PathBuf, String, String)>,
}

impl DeletePlan {
    pub fn label(&self) -> String {
        if self.sample_folders.is_empty() {
            return self.tile.path.clone();
        }
        let folders: Vec<String> = self.sample_folders.iter().map(|folder| format!("{folder}/")).collect();
        format!("{} and {}", self.tile.path, folders.join(", "))
    }
}

pub(crate) fn validate_tile(tile: &TileId) -> Result<(), String> {
    if tile.path.is_empty() || Path::new(&tile.path).components().any(|component| !matches!(component, Component::Normal(_))) {
        return Err("Choose an asset inside the asset root; the root cannot be changed".into());
    }
    Ok(())
}

/// List all depths through the same no-symlink seam the browser scans.
pub(crate) fn files_under(base: &Path, relative: &str) -> Result<Vec<String>, String> {
    entries_under(base, relative).map(|(files, _)| files)
}

/// Every file and every subfolder under `relative`, each sorted; a folder that cannot be
/// listed is the error, since a folder operation must know the whole set it acts on.
pub(crate) fn entries_under(base: &Path, relative: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let mut pending = vec![base.join(relative)];
    let (mut files, mut folders) = (Vec::new(), Vec::new());
    while let Some(folder) = pending.pop() {
        for entry in vfs::list_entries(&folder).map_err(|error| format!("Could not list {}: {error}", folder.display()))? {
            if entry.is_dir {
                folders.push(super::rename::relative_path(base, &entry.path)?);
                pending.push(entry.path);
            } else {
                files.push(super::rename::relative_path(base, &entry.path)?);
            }
        }
    }
    files.sort();
    folders.sort();
    Ok((files, folders))
}

/// A sample belongs to the song named by its folder's LICENSE, never to the asset menu.
pub(crate) fn refuse_sample(base: &Path, relative: &str) -> Result<(), String> {
    for home in editor::song::HOME_DIRECTORIES {
        let prefix = format!("{home}/samples/");
        let Some(rest) = relative.strip_prefix(&prefix) else { continue; };
        let folder = rest.split('/').next().unwrap_or("");
        if folder.is_empty() { continue; }
        // A file directly under samples/ sits in no song's folder; a folder there reads as none.
        if !rest.contains('/') && vfs::read(&base.join(relative)).is_ok() { return Ok(()); }
        let license = base.join(format!("{prefix}{folder}/LICENSE"));
        let owner = vfs::read_to_string(&license).ok().and_then(|text| {
            text.lines().find_map(|line| line.strip_prefix("Source:").map(|source| source.trim().to_string()))
        });
        // Samples whose song is gone — a delete that stopped after the song's own files, a
        // folder brought in by hand — belong to nobody and may go.
        if let Some(owner) = &owner {
            if !vfs::exists(&base.join(owner)) { return Ok(()); }
        }
        let owner = owner.unwrap_or_else(|| format!("the song owning {prefix}{folder}/"));
        return Err(format!("{relative} is one of {owner}'s samples — edit it from the Conductor panel"));
    }
    Ok(())
}

pub(crate) fn prepare(base: &Path, tile: &TileId, live: &[super::rename::LiveText]) -> Result<DeletePlan, String> {
    validate_tile(tile)?;
    refuse_sample(base, &tile.path)?;
    let mut folders = Vec::new();
    let mut files = if tile.kind == TileKind::Folder {
        let (files, subfolders) = entries_under(base, &tile.path)?;
        folders = subfolders;
        // A folder above a song's samples would take them with it; the song owning them is
        // deleted first, which removes its own folder.
        for file in &files {
            refuse_sample(base, file).map_err(|error| format!("Cannot delete {}/: {error}", tile.path))?;
            // A song's samples live beside its home folder, not under it; a folder delete
            // would leave them owned by a song that no longer exists and refusing every delete.
            if editor::external_tools::is_midi_path(Path::new(file)) {
                if let Some(folder) = editor::song::sample_directory(Path::new(file)).filter(|folder| vfs::exists(&base.join(folder))) {
                    return Err(format!("Cannot delete {}/: {file} keeps samples in {folder}/; delete the song first", tile.path));
                }
            }
        }
        files
    } else {
        if !vfs::exists(&base.join(&tile.path)) { return Err(format!("{} no longer exists", tile.path)); }
        if base.join(&tile.path).is_dir() { return Err(format!("{} is a folder now, not a file; Rescan", tile.path)); }
        editor::asset_family(&base.join(&tile.path).to_string_lossy())
            .map_err(|error| format!("Could not list {}'s family: {error}", tile.path))?.into_iter()
            .map(|file| super::rename::relative_path(base, Path::new(&file))).collect::<Result<Vec<_>, _>>()?
    };
    let sample_folders: Vec<String> = if tile.kind == TileKind::File {
        files.iter().filter(|file| editor::external_tools::is_midi_path(Path::new(file)))
            .filter_map(|file| editor::song::sample_directory(Path::new(file)).filter(|folder| vfs::exists(&base.join(folder))))
            .collect()
    } else { Vec::new() };
    // The samples go first: a delete that stops among them leaves the song, whose own
    // refusal rule and Conductor panel still reach the folder.
    let mut sample_files = Vec::new();
    for folder in &sample_folders {
        let (in_folder, sample_subfolders) = entries_under(base, folder)?;
        sample_files.extend(in_folder);
        folders.extend(sample_subfolders);
    }
    sample_files.append(&mut files);
    let files = sample_files;
    let olds: Vec<&str> = if tile.kind == TileKind::Folder { vec![tile.path.as_str()] } else { files.iter().map(String::as_str).collect() };
    let scope = if tile.kind == TileKind::Folder { super::rename::ReferenceScope::DeletedFolder } else { super::rename::ReferenceScope::Files };
    let references = super::rename::reference_hits(base, &olds, scope, live)?;
    let removed: Vec<&str> = files.iter().map(String::as_str).collect();
    let licenses = super::rename::license_rewrites(base, &removed, false, |text, in_folder, _| editor::song::remove_from_license(text, in_folder))?;
    Ok(DeletePlan { tile: tile.clone(), files, folders, sample_folders, references, live_scanned: false, licenses })
}

/// Remove the files, then their provenance. A stanza that could not be rewritten is the
/// warning returned; the files are gone either way.
pub(crate) fn execute(base: &Path, plan: &DeletePlan) -> Result<Option<String>, String> {
    let removed: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let outcome = execute_with(plan,
        |path| vfs::remove_file(&base.join(path)).map_err(|error| error.to_string()).map(|()| removed.borrow_mut().push(path.to_string())),
        |path| remove_emptied_folder(base, path, &plan.folders));
    if let Err(error) = outcome {
        // The stanzas of the files that did go are dropped now, so a retry starts from a
        // LICENSE that names what is left, not what is gone.
        let gone_owned = removed.into_inner();
        let gone: Vec<&str> = gone_owned.iter().map(String::as_str).collect();
        if let Ok(rewrites) = super::rename::license_rewrites(base, &gone, false, |text, in_folder, _| editor::song::remove_from_license(text, in_folder)) {
            for (license, original, text) in rewrites {
                if vfs::exists(&license) && vfs::read_to_string(&license).ok().as_deref() == Some(original.as_str()) { let _ = vfs::write(&license, text.as_bytes()); }
            }
        }
        return Err(error);
    }
    for (license, original, text) in &plan.licenses {
        if !vfs::exists(license) { continue; }
        if vfs::read_to_string(license).ok().as_deref() != Some(original.as_str()) {
            return Ok(Some(format!("Deleted {} files; {} changed meanwhile, so its stanza was left for you to update", plan.files.len(), license.display())));
        }
        if let Err(error) = vfs::write(license, text.as_bytes()) {
            return Ok(Some(format!("Deleted {} files; the LICENSE stanza was not updated ({}): {error}", plan.files.len(), license.display())));
        }
    }
    Ok(None)
}

fn execute_with(plan: &DeletePlan, mut remove_file: impl FnMut(&str) -> Result<(), String>, mut remove_folder: impl FnMut(&str) -> Result<(), String>) -> Result<(), String> {
    if plan.tile.kind == TileKind::Folder {
        // The planned files go one by one, so a file that arrived since the confirm is never
        // removed with them: the folder itself goes only once nothing else is left in it.
        for (index, file) in plan.files.iter().enumerate() {
            remove_file(file).map_err(|error| format!("Deleted {index} files; could not delete {file}: {error}"))?;
        }
        return remove_folder(&plan.tile.path).map_err(|error| format!("Deleted the {} files; {error}", plan.files.len()));
    }
    // Every planned file goes one by one, a song's samples included; each sample folder is
    // then removed emptied — on the web it is already gone with its last key.
    for (index, file) in plan.files.iter().enumerate() {
        remove_file(file).map_err(|error| format!("Deleted {index} files; could not delete {file}: {error}"))?;
    }
    for folder in &plan.sample_folders {
        remove_folder(folder).map_err(|error| format!("Deleted the song files; {error}"))?;
    }
    Ok(())
}

/// Remove a folder whose planned files are gone; one that still holds a file, or a subfolder
/// the confirm never saw — either arrived since — stays, named, for the person to look at,
/// and so does one that cannot be listed. On the web a folder is its keys, so one left with
/// none is already gone.
fn remove_emptied_folder(base: &Path, relative: &str, planned_folders: &[String]) -> Result<(), String> {
    let folder = base.join(relative);
    let (remaining, subfolders) = match entries_under(base, relative) {
        Ok(entries) => entries,
        Err(error) if !vfs::exists(&folder) => { let _ = error; return Ok(()); }
        Err(error) => return Err(format!("{error}; the folder is kept — Rescan")),
    };
    if let Some(first) = remaining.first() {
        return Err(format!("{first} appeared in {relative}/ since the confirm ({} files); the folder is kept — Rescan", remaining.len()));
    }
    if let Some(newcomer) = subfolders.iter().find(|subfolder| !planned_folders.contains(subfolder)) {
        return Err(format!("{newcomer}/ appeared in {relative}/ since the confirm; the folder is kept — Rescan"));
    }
    // Deepest first and never recursively: a file arriving between the listing and the
    // removal makes its folder refuse, and is kept with it, instead of going with the tree.
    let mut deepest_first = subfolders.clone();
    deepest_first.sort_by_key(|folder| std::cmp::Reverse(folder.matches('/').count()));
    for subfolder in deepest_first.iter().chain(std::iter::once(&relative.to_string())) {
        match vfs::remove_dir(&base.join(subfolder)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{subfolder}/ was not empty when its turn came ({error}); the folder is kept — Rescan")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn deleting_a_song_removes_its_implicit_sample_folder_once_and_keeps_its_neighbor() {
        let base = Path::new("/project/assets");
        let filesystem = RefCell::new(vfs::MemFs::default());
        let files = ["music/theme.mid", "music/theme.mid.ron", "music/samples/theme/ping.wav", "music/samples/theme/LICENSE"];
        for path in files.into_iter().chain(["music/samples/theme_other/ping.wav"]) {
            filesystem.borrow_mut().insert(base.join(path).to_string_lossy().into_owned(), b"bytes".to_vec());
        }
        let plan = DeletePlan { tile: TileId::file("music/theme.mid"), files: files.map(String::from).into(), folders: Vec::new(), sample_folders: vec!["music/samples/theme".into()], references: Vec::new(), live_scanned: false, licenses: Vec::new() };
        execute_with(&plan,
            |path| filesystem.borrow_mut().remove_file(&base.join(path)).map_err(|error| error.to_string()),
            |path| filesystem.borrow().remove_dir(&base.join(path)).map_err(|error| error.to_string())).unwrap();
        for path in files { assert!(!filesystem.borrow().exists(&base.join(path))); }
        assert!(filesystem.borrow().exists(&base.join("music/samples/theme_other/ping.wav")));
    }

    #[test]
    fn deleting_a_sound_names_its_references_and_drops_its_license_stanza() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("sounds/snd_jump.wav"), b"sound").unwrap();
        vfs::write(&base.join("sounds/LICENSE"), b"Files: snd_jump.wav\nLicense: CC0-1.0\n\nFiles: beep.wav\nLicense: MIT\n").unwrap();
        vfs::write(&base.join("scenes/level.scene.ron"), b"SoundEmitter(sound: Path(\"sounds/snd_jump.wav\"))").unwrap();
        let plan = prepare(base, &TileId::file("sounds/snd_jump.wav"), &[]).unwrap();
        assert_eq!(plan.references, vec!["scenes/level.scene.ron"]);
        let live = [("the open scene (unsaved)".to_string(), Some("sounds/snd_jump.wav".to_string()))];
        assert_eq!(prepare(base, &TileId::file("sounds/snd_jump.wav"), &live).unwrap().references, vec!["the open scene (unsaved)", "scenes/level.scene.ron"]);
        assert_eq!(execute(base, &plan).unwrap(), None);
        assert!(!base.join("sounds/snd_jump.wav").exists());
        assert_eq!(vfs::read_to_string(&base.join("sounds/LICENSE")).unwrap(), "Files: beep.wav\nLicense: MIT\n");
    }

    #[cfg(unix)]
    #[test]
    fn deleting_a_sound_under_a_read_only_license_deletes_it_and_warns_about_the_stanza() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("sounds/hop.wav"), b"sound").unwrap();
        let license = base.join("sounds/LICENSE");
        vfs::write(&license, b"Files: hop.wav\nLicense: CC0-1.0\n").unwrap();
        std::fs::set_permissions(&license, std::fs::Permissions::from_mode(0o444)).unwrap();
        let plan = prepare(base, &TileId::file("sounds/hop.wav"), &[]).unwrap();
        let warning = execute(base, &plan).unwrap();
        std::fs::set_permissions(&license, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!base.join("sounds/hop.wav").exists(), "the delete went through");
        assert!(warning.unwrap().contains("LICENSE stanza was not updated"));
    }

    #[cfg(unix)]
    #[test]
    fn deleting_a_folder_with_a_read_only_license_inside_it_is_not_refused() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("sounds/vendor/hop.wav"), b"sound").unwrap();
        let license = base.join("sounds/vendor/LICENSE");
        vfs::write(&license, b"Files: hop.wav\nLicense: CC0-1.0\n").unwrap();
        std::fs::set_permissions(&license, std::fs::Permissions::from_mode(0o444)).unwrap();
        let plan = prepare(base, &TileId::folder("sounds/vendor"), &[]).unwrap();
        assert_eq!(execute(base, &plan).unwrap(), None);
        assert!(!base.join("sounds/vendor").exists());
    }

    #[test]
    fn deleting_a_folder_keeps_a_file_that_arrived_since_the_confirm() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("scripts/old/paddle.rhai"), b"script").unwrap();
        vfs::write(&base.join("scripts/old/nested/ball.rhai"), b"script").unwrap();
        let plan = prepare(base, &TileId::folder("scripts/old"), &[]).unwrap();
        vfs::write(&base.join("scripts/old/new.rhai"), b"arrived after the confirm").unwrap();
        let error = execute(base, &plan).err().unwrap();
        assert!(error.contains("new.rhai appeared"), "{error}");
        assert!(base.join("scripts/old/new.rhai").exists() && !base.join("scripts/old/paddle.rhai").exists(), "the planned files went, the newcomer stayed");
        std::fs::remove_file(base.join("scripts/old/new.rhai")).unwrap();
        std::fs::create_dir(base.join("scripts/old/planned_empty")).unwrap();
        let plan = prepare(base, &TileId::folder("scripts/old"), &[]).unwrap();
        assert!(plan.folders.contains(&"scripts/old/planned_empty".to_string()));
        std::fs::create_dir(base.join("scripts/old/review")).unwrap();
        let error = execute(base, &plan).err().unwrap();
        assert!(error.contains("review/ appeared"), "{error}");
        assert!(base.join("scripts/old/review").is_dir(), "an empty folder that appeared since the confirm is kept");
        std::fs::remove_dir(base.join("scripts/old/review")).unwrap();
        let plan = prepare(base, &TileId::folder("scripts/old"), &[]).unwrap();
        assert_eq!(execute(base, &plan).unwrap(), None, "a planned empty subfolder goes with its folder");
        assert!(!base.join("scripts/old").exists());
    }

    #[test]
    fn a_delete_that_stops_partway_drops_the_stanzas_of_what_went() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("music/theme.mid"), b"score").unwrap();
        vfs::write(&base.join("music/theme.mid.ron"), b"map").unwrap();
        vfs::write(&base.join("music/LICENSE"), b"Files: theme.mid theme.mid.ron\nLicense: CC0-1.0\n").unwrap();
        let plan = prepare(base, &TileId::file("music/theme.mid"), &[]).unwrap();
        std::fs::remove_file(base.join("music/theme.mid.ron")).unwrap();
        std::fs::create_dir(base.join("music/theme.mid.ron")).unwrap();
        assert!(execute(base, &plan).is_err(), "the folder in the sidecar's place stops the delete");
        assert!(!base.join("music/theme.mid").exists());
        assert_eq!(vfs::read_to_string(&base.join("music/LICENSE")).unwrap(), "Files: theme.mid.ron\nLicense: CC0-1.0\n", "the stanza names what is left");
    }

    #[test]
    fn a_license_edited_since_the_plan_read_it_is_left_alone_with_a_warning() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("sounds/hop.wav"), b"sound").unwrap();
        vfs::write(&base.join("sounds/LICENSE"), b"Files: hop.wav\nLicense: CC0-1.0\n").unwrap();
        let plan = prepare(base, &TileId::file("sounds/hop.wav"), &[]).unwrap();
        let edited = "Files: hop.wav\nLicense: CC0-1.0\n\nFiles: added.wav\nLicense: MIT\n";
        vfs::write(&base.join("sounds/LICENSE"), edited.as_bytes()).unwrap();
        let warning = execute(base, &plan).unwrap().unwrap();
        assert!(warning.contains("changed meanwhile"), "{warning}");
        assert_eq!(vfs::read_to_string(&base.join("sounds/LICENSE")).unwrap(), edited, "the other program's edit is kept");
    }

    #[test]
    fn deleting_an_orphan_song_sidecar_drops_its_token() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("music/loud.mid.RON"), b"map").unwrap();
        vfs::write(&base.join("music/LICENSE"), b"Files: loud.mid.RON\nLicense: CC0-1.0\n").unwrap();
        let plan = prepare(base, &TileId::file("music/loud.mid.RON"), &[]).unwrap();
        assert_eq!(plan.licenses.len(), 1, "an orphan sidecar in another case is a song's to the LICENSE scan too");
        assert_eq!(execute(base, &plan).unwrap(), None);
        assert!(!base.join("music/LICENSE").exists() || !vfs::read_to_string(&base.join("music/LICENSE")).unwrap().contains("loud.mid.RON"));
        vfs::write(&base.join("music/theme.mid.ron"), b"map").unwrap();
        vfs::write(&base.join("music/LICENSE"), b"Files: theme.mid theme.mid.ron\nLicense: CC0-1.0\n").unwrap();
        let plan = prepare(base, &TileId::file("music/theme.mid.ron"), &[]).unwrap();
        assert_eq!(execute(base, &plan).unwrap(), None);
        assert_eq!(vfs::read_to_string(&base.join("music/LICENSE")).unwrap(), "Files: theme.mid\nLicense: CC0-1.0\n");
    }

    #[test]
    fn deleting_a_sound_font_drops_its_stanza_too() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("soundfonts/sine.sf2"), b"bank").unwrap();
        vfs::write(&base.join("soundfonts/LICENSE"), b"Files: sine.sf2\nLicense: CC0-1.0\n\nFiles: other.sf2\nLicense: MIT\n").unwrap();
        let plan = prepare(base, &TileId::file("soundfonts/sine.sf2"), &[]).unwrap();
        assert_eq!(execute(base, &plan).unwrap(), None);
        assert_eq!(vfs::read_to_string(&base.join("soundfonts/LICENSE")).unwrap(), "Files: other.sf2\nLicense: MIT\n");
    }

    #[test]
    fn deleting_a_song_takes_a_recovered_siblings_sample_folder_too() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("music/theme.mid"), b"score").unwrap();
        vfs::write(&base.join("music/theme.recovered-1.mid"), b"score").unwrap();
        let recovered_samples = editor::song::sample_directory(Path::new("music/theme.recovered-1.mid")).unwrap();
        vfs::write(&base.join(&recovered_samples).join("ping.wav"), b"sound").unwrap();
        let plan = prepare(base, &TileId::file("music/theme.mid"), &[]).unwrap();
        assert_eq!(plan.sample_folders, vec![recovered_samples.clone()]);
        assert!(plan.files.iter().any(|file| file.starts_with(&recovered_samples)));
        assert_eq!(execute(base, &plan).unwrap(), None);
        assert!(!base.join(&recovered_samples).exists(), "the recovered song's samples go with the family");
    }

    #[test]
    fn deleting_a_folder_whose_song_keeps_samples_elsewhere_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        vfs::write(&base.join("music/covers/theme.mid"), b"score").unwrap();
        let samples = editor::song::sample_directory(Path::new("music/covers/theme.mid")).unwrap();
        vfs::write(&base.join(&samples).join("ping.wav"), b"sound").unwrap();
        let error = prepare(base, &TileId::folder("music/covers"), &[]).err().unwrap();
        assert!(error.contains(&samples) && error.contains("delete the song first"), "{error}");
        let plan = prepare(base, &TileId::file("music/covers/theme.mid"), &[]).unwrap();
        assert!(plan.files.first().unwrap().starts_with(&samples), "the samples go before the song: {:?}", plan.files);
        vfs::write(&base.join(&samples).join("LICENSE"), b"Source: music/covers/theme.mid\nFiles: ping.wav\nLicense: CC0-1.0\n").unwrap();
        std::fs::remove_file(base.join("music/covers/theme.mid")).unwrap();
        assert!(prepare(base, &TileId::folder(&samples), &[]).is_ok(), "a sample folder whose song is gone belongs to nobody and may go");
        vfs::write(&base.join("music/samples/stray.wav"), b"sound").unwrap();
        assert!(prepare(base, &TileId::file("music/samples/stray.wav"), &[]).is_ok(), "a file directly under samples/ is in no song's folder");
    }
}
