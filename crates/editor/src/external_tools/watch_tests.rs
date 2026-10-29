//! `FileWatch` driven poll by poll: stamps faked for the debounce, the tombstones and the
//! companions, real files in a temporary directory for the editor's own writes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::vfs::ChangeStamp;

use super::*;

/// What a fake disk holds: each path's reading, a missing key being a missing file.
#[derive(Default)]
struct FakeDisk(BTreeMap<PathBuf, ChangeStamp>);

impl FakeDisk {
    fn write(&mut self, path: &str, length_bytes: u64) {
        self.0.insert(PathBuf::from(path), ChangeStamp { length_bytes, modified_nanoseconds: u128::from(length_bytes) });
    }

    fn remove(&mut self, path: &str) {
        self.0.remove(Path::new(path));
    }

    fn read(&self, path: &Path) -> Option<Observed> {
        Some(self.0.get(path).map_or(Observed::Absent, |stamp| Observed::Present(*stamp)))
    }
}

fn watching(paths: &[&str]) -> FileWatch {
    let mut watch = FileWatch::new();
    watch.set_registered(paths.iter().map(PathBuf::from).collect::<BTreeSet<_>>(), &BTreeSet::new());
    watch
}

fn poll(watch: &mut FileWatch, disk: &FakeDisk) -> Vec<FileChange> {
    watch.poll_with(|path| disk.read(path))
}

fn changed(path: &str) -> FileChange {
    FileChange::Changed(PathBuf::from(path))
}

#[test]
fn a_change_that_holds_one_poll_is_reported_once() {
    let mut disk = FakeDisk::default();
    disk.write("scene.ron", 10);
    let mut watch = watching(&["scene.ron"]);
    assert!(poll(&mut watch, &disk).is_empty(), "the first poll reads the baseline");

    disk.write("scene.ron", 11);
    assert!(poll(&mut watch, &disk).is_empty(), "a change is held for one poll");
    assert_eq!(poll(&mut watch, &disk), vec![changed("scene.ron")]);
    assert!(poll(&mut watch, &disk).is_empty(), "and reported once");
}

#[test]
fn a_path_changing_every_poll_is_reported_once_it_settles() {
    let mut disk = FakeDisk::default();
    disk.write("song.mid", 1);
    let mut watch = watching(&["song.mid"]);
    poll(&mut watch, &disk);

    for length in 2..6 {
        disk.write("song.mid", length);
        assert!(poll(&mut watch, &disk).is_empty(), "still being written at length {length}");
    }
    assert_eq!(poll(&mut watch, &disk), vec![changed("song.mid")]);
}

#[test]
fn an_unlink_then_create_inside_the_window_is_one_change_and_no_deletion() {
    let mut disk = FakeDisk::default();
    disk.write("paddle.rhai", 10);
    let mut watch = watching(&["paddle.rhai"]);
    poll(&mut watch, &disk);

    disk.remove("paddle.rhai");
    assert!(poll(&mut watch, &disk).is_empty());
    disk.write("paddle.rhai", 12);
    assert!(poll(&mut watch, &disk).is_empty());
    assert_eq!(poll(&mut watch, &disk), vec![changed("paddle.rhai")]);
    assert!(poll(&mut watch, &disk).is_empty());
}

#[test]
fn a_path_gone_for_many_polls_is_deleted_once_and_its_late_return_is_a_change() {
    let mut disk = FakeDisk::default();
    disk.write("tiles.sheet.ron", 10);
    let mut watch = watching(&["tiles.sheet.ron"]);
    poll(&mut watch, &disk);

    disk.remove("tiles.sheet.ron");
    let reports: Vec<FileChange> = (0..8).flat_map(|_| poll(&mut watch, &disk)).collect();
    assert_eq!(reports, vec![FileChange::Deleted(PathBuf::from("tiles.sheet.ron"))]);

    disk.write("tiles.sheet.ron", 10);
    poll(&mut watch, &disk);
    assert_eq!(poll(&mut watch, &disk), vec![changed("tiles.sheet.ron")], "the tombstone was kept");
}

#[test]
fn a_path_registered_while_missing_reports_its_creation() {
    let mut disk = FakeDisk::default();
    let mut watch = watching(&["new.rhai"]);
    poll(&mut watch, &disk);

    disk.write("new.rhai", 3);
    poll(&mut watch, &disk);
    assert_eq!(poll(&mut watch, &disk), vec![changed("new.rhai")]);
}

#[test]
fn a_midi_file_and_its_sidecar_settle_together() {
    let mut disk = FakeDisk::default();
    disk.write("theme.mid", 10);
    disk.write("theme.mid.ron", 20);
    let mut watch = watching(&["theme.mid", "theme.mid.ron"]);
    poll(&mut watch, &disk);

    disk.write("theme.mid", 11);
    assert!(poll(&mut watch, &disk).is_empty());
    disk.write("theme.mid.ron", 21);
    assert!(poll(&mut watch, &disk).is_empty(), "the .mid has settled, but its sidecar moves");
    disk.write("theme.mid.ron", 22);
    assert!(poll(&mut watch, &disk).is_empty(), "the sidecar still moves");
    assert_eq!(poll(&mut watch, &disk), vec![changed("theme.mid"), changed("theme.mid.ron")], "one poll, both");
}

#[test]
fn a_path_dropped_from_the_set_is_forgotten_and_re_registered_reads_a_new_baseline() {
    let mut disk = FakeDisk::default();
    disk.write("a.rhai", 1);
    let mut watch = watching(&["a.rhai"]);
    poll(&mut watch, &disk);
    disk.write("a.rhai", 2);
    poll(&mut watch, &disk);

    watch.set_registered(BTreeSet::new(), &BTreeSet::new());
    assert!(poll(&mut watch, &disk).is_empty(), "an unregistered path is never reported");
    watch.set_registered([PathBuf::from("a.rhai")].into(), &BTreeSet::new());
    assert!(poll(&mut watch, &disk).is_empty(), "re-registered, it starts from how it stands");
    assert!(poll(&mut watch, &disk).is_empty());
}

#[test]
fn a_path_registered_as_expected_absent_and_found_present_is_a_change_from_its_first_poll() {
    let mut disk = FakeDisk::default();
    disk.write("sounds/late.wav", 10);
    disk.write("sounds/known.wav", 10);
    let mut watch = FileWatch::new();
    let paths = ["sounds/late.wav", "sounds/known.wav"].map(PathBuf::from);
    watch.set_registered(paths.iter().cloned().collect(), &[paths[0].clone()].into());

    assert!(poll(&mut watch, &disk).is_empty(), "found present at the first poll, it is held one poll");
    assert_eq!(
        poll(&mut watch, &disk),
        vec![changed("sounds/late.wav")],
        "the marked path's arrival is a change; the unmarked one's first reading stays its baseline"
    );

    watch.set_registered(paths.iter().cloned().collect(), &[paths[1].clone()].into());
    assert!(poll(&mut watch, &disk).is_empty(), "newly marked, the observed path is taken as absent and its presence held one poll");
    assert_eq!(
        poll(&mut watch, &disk),
        vec![changed("sounds/known.wav")],
        "the host believed it missing after reading it, so the file present is its arrival"
    );
    watch.set_registered(paths.iter().cloned().collect(), &[paths[1].clone()].into());
    assert!(poll(&mut watch, &disk).is_empty(), "the same mark again is no news and reports nothing");
}

fn write_and_stamp(path: &Path, text: &str) -> ChangeStamp {
    std::fs::write(path, text).expect("the write");
    common::vfs::change_stamp(path).expect("the stamp").expect("a native filesystem keeps one")
}

#[test]
fn the_editors_own_write_noted_after_a_poll_caught_it_reports_nothing_and_a_later_outside_write_is_reported() {
    let project = tempfile::tempdir().expect("a temporary project");
    let scene = project.path().join("level.scene.ron");
    std::fs::write(&scene, "first").expect("the scene");
    let mut watch = FileWatch::new();
    watch.set_registered([scene.clone()].into(), &BTreeSet::new());
    assert!(watch.poll().is_empty());

    let saved = write_and_stamp(&scene, "the editor's save");
    assert!(watch.poll().is_empty(), "the poll catches the save mid-debounce");
    watch.note_written(&scene, saved);
    assert!(watch.poll().is_empty());
    assert!(watch.poll().is_empty(), "the editor's own write is the baseline");

    std::fs::write(&scene, "another program's longer write").expect("the outside write");
    assert!(watch.poll().is_empty());
    assert_eq!(watch.poll(), vec![FileChange::Changed(scene)]);
}
