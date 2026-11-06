//! The editor's hot reload: which of the project's files are watched, the worker's changes
//! drained once a frame, and each one dispatched by its file type's reload policy.
//!
//! The watched set is what this reloads, computed from the world, the sound tier and the
//! songs on the watcher's tick: the open scene, every `.rhai` a `Scripts` component names, the
//! `.sheet.ron` beside every sheet a `SpriteAnimation` names, every recorded sound a
//! `SoundEmitter` names and every path the sound tier loaded or failed, the open song's two
//! files and — in a play session — its song's two files and closure, each resolved against
//! the asset base. A failed sound's path, and a missing one in the song's closure, go to the
//! worker marked as expected absent, so a file arriving before the worker's first look is
//! still a change. The set is kept as a map from each path to what named it, and a change
//! reaches its owners through that map — nothing strips the asset base back off a path. The
//! editor's own saves are the watcher's baseline, and a change the worker read before it took
//! the save's note is dropped, since the file now holds the editor's own bytes.

mod sample;
mod sheet;
mod song;
mod worker;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ecs::script::Scripts;
use audio::AudioManager;
use ecs::sprite_components::SpriteAnimation;
use ecs::{SoundEmitter, SoundRef, World};
use editor::console::Severity;
use editor::external_tools::{FileChange, FileType, ReloadPolicy, WatchEvent};
use engine_core::assets::AssetManager;
use engine_core::contexts::GameContext;
use engine_core::scripting::ScriptRunner;
use engine_core::Game;

use super::console::script_error_location;
use super::scene_confirm::PendingSceneAction;
use super::EditorGame;

use sheet::ReloadedSheet;
pub(super) use song::SongOwner;
use song::SongChanges;

/// How often the worker reads the watched files, and the host recomputes what they are.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// What named a watched path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum WatchOwner {
    /// The open scene's file.
    Scene,
    /// A `.rhai` source path, as a `Scripts` component names it.
    Script(String),
    /// A sheet's PNG path, as a `SpriteAnimation` names it; the watched path is its sidecar.
    Sheet(String),
    /// A recorded sound's path, as a `SoundEmitter` or the sound tier names it.
    Sound(String),
    /// A file of a song: the open document's or the play session's.
    Song(SongOwner),
}

/// The watcher's side of the editor: its worker, the watched set and what a change waits on.
#[derive(Default)]
pub(super) struct HotReload {
    /// Started at the first frame's drain, so a headless session never starts a thread.
    pub(super) worker: Option<worker::WatchWorker>,
    pub(super) watched: BTreeMap<PathBuf, BTreeSet<WatchOwner>>,
    /// The watched paths the sound tier had failed to load when the worker last heard the
    /// set: a path joining them is news the worker must hear, since its file may already be
    /// there (`FileWatch::set_registered`).
    pub(super) expected_absent: BTreeSet<PathBuf>,
    since_tick_seconds: f32,
    /// Events taken from the worker and not yet dispatched, in the order it sent them.
    pub(super) incoming: Vec<WatchEvent>,
    /// Each path the editor saved while the worker ran, with how many of those saves it has
    /// not yet acknowledged: a change for it until then was read before the save.
    pub(super) awaiting_note: BTreeMap<PathBuf, u32>,
    /// The open scene's change, waiting for a gesture to end or a play session to stop.
    pub(super) waiting_scene: Option<PathBuf>,
    /// The open song's `.mid` whose sidecar changed under unsaved edits, waiting to ask as
    /// the scene's change waits.
    pub(super) waiting_song_map: Option<PathBuf>,
    /// Each sheet the play session reloaded, by PNG path, for Stop to apply again.
    session_sheets: BTreeMap<String, ReloadedSheet>,
}

/// A worker on its own, for the thread's test.
#[cfg(test)]
pub(super) fn spawn_worker(interval: Duration) -> worker::WatchWorker {
    worker::WatchWorker::spawn(interval)
}

/// What a dispatch may change.
pub(super) struct ReloadTargets<'a> {
    pub world: &'a mut World,
    pub assets: &'a mut AssetManager,
    pub scripts: &'a mut ScriptRunner,
    pub audio: &'a mut AudioManager,
    /// A text field holds the keyboard: an edit is in flight.
    pub edit_field_focused: bool,
}

impl<G: Game> EditorGame<G> {
    /// The frame's drain, after the dirty mirror and before the status bar: recompute the
    /// watched set on the tick, take the worker's changes and dispatch them.
    pub(super) fn drain_file_changes(&mut self, ctx: &mut GameContext) {
        self.tick_file_watch(ctx.world, ctx.assets, ctx.audio, ctx.delta_time);
        if let Some(worker) = &self.hot_reload.worker {
            self.hot_reload.incoming.extend(worker.take_events());
        }
        let mut targets = ReloadTargets {
            world: ctx.world,
            assets: ctx.assets,
            scripts: ctx.scripts,
            audio: ctx.audio,
            edit_field_focused: ctx.ui.wants_keyboard(),
        };
        self.dispatch_file_changes(&mut targets);
    }

    fn tick_file_watch(&mut self, world: &World, assets: &AssetManager, audio: &AudioManager, delta_time: f32) {
        if cfg!(target_arch = "wasm32") {
            return;
        }
        let watch = &mut self.hot_reload;
        watch.since_tick_seconds += delta_time;
        if watch.worker.is_some() && watch.since_tick_seconds < POLL_INTERVAL.as_secs_f32() {
            return;
        }
        watch.since_tick_seconds = 0.0;
        let mut watched = self.watched_paths(world, assets);
        let mut expected_absent = self.expected_absent_paths(assets);
        expected_absent.extend(self.absent_song_paths(audio));
        self.watch_songs(&mut watched, audio);
        let watch = &mut self.hot_reload;
        let started = watch.worker.is_none();
        let worker = watch.worker.get_or_insert_with(|| worker::WatchWorker::spawn(POLL_INTERVAL));
        // A path can fail to load after it was registered (a scene's emitter names a file
        // that is not there yet), so the marks decide with the keys.
        if started || !watched.keys().eq(watch.watched.keys()) || expected_absent != watch.expected_absent {
            worker.register(watched.keys().cloned().collect(), expected_absent.clone());
        }
        watch.watched = watched;
        watch.expected_absent = expected_absent;
    }

    /// Every path the world and the sound tier name now, each with what named it; the tick
    /// adds the songs' (`watch_songs`).
    pub(super) fn watched_paths(&self, world: &World, assets: &AssetManager) -> BTreeMap<PathBuf, BTreeSet<WatchOwner>> {
        let mut watched: BTreeMap<PathBuf, BTreeSet<WatchOwner>> = BTreeMap::new();
        if let Some(scene) = self.editor.scene_path() {
            watched.entry(self.resolve_asset_path(scene)).or_default().insert(WatchOwner::Scene);
        }
        for entity in world.entity_ids() {
            for script in world.get::<Scripts>(entity).map(|scripts| scripts.0.as_slice()).unwrap_or_default() {
                // A native script's source path is empty, and names no file.
                if script.source_path.ends_with(".rhai") {
                    let path = self.resolve_asset_path(Path::new(&script.source_path));
                    watched.entry(path).or_default().insert(WatchOwner::Script(script.source_path.clone()));
                }
            }
            let sheet = world.get::<SpriteAnimation>(entity).and_then(|animation| animation.sheet.as_deref());
            if let Some(png) = sheet.filter(|png| !png.is_empty()) {
                let path = self.resolve_asset_path(&engine_core::sidecar_path_for(png));
                watched.entry(path).or_default().insert(WatchOwner::Sheet(png.to_string()));
            }
            if let Some(SoundEmitter { sound: SoundRef::Path(sound), .. }) = world.get::<SoundEmitter>(entity) {
                if !sound.is_empty() {
                    watched.entry(self.resolve_asset_path(Path::new(sound))).or_default().insert(WatchOwner::Sound(sound.clone()));
                }
            }
        }
        for sound in assets.sound_paths() {
            let path = self.resolve_asset_path(Path::new(sound));
            watched.entry(path).or_default().insert(WatchOwner::Sound(sound.to_string()));
        }
        watched
    }

    /// The watched sounds the sound tier failed to load: missing files, as far as it knows;
    /// the tick adds the song closure's (`absent_song_paths`).
    pub(super) fn expected_absent_paths(&self, assets: &AssetManager) -> BTreeSet<PathBuf> {
        assets.failed_sound_paths().map(|sound| self.resolve_asset_path(Path::new(sound))).collect()
    }

    /// After a successful scene save: the file holds the editor's own bytes, so a change for
    /// it already waiting is dropped — parked behind a gesture or as the Reload prompt — and
    /// the file is noted as written.
    pub(super) fn note_scene_written(&mut self, path: &Path) {
        let path = self.resolve_asset_path(path);
        if self.hot_reload.waiting_scene.as_deref() == Some(path.as_path()) {
            self.hot_reload.waiting_scene = None;
        }
        if matches!(&self.scene_confirm.pending_action, Some(PendingSceneAction::ReloadFromDisk(parked)) if *parked == path) {
            self.scene_confirm.pending_action = None;
            self.scene_confirm.pending_choice = None;
        }
        self.note_written(&path);
    }

    /// After the editor's own successful write of `path`, resolved: the stamp the file has now
    /// is the watcher's baseline, and a change for it is dropped until the worker acknowledges
    /// the note. The one stamp the frame ever reads.
    pub(super) fn note_written(&mut self, path: &Path) {
        // Before the worker starts nothing it sends can predate the save, and a count no
        // acknowledgement answers would hold back every later change to the file.
        let Some(worker) = &self.hot_reload.worker else { return };
        let stamp = common::vfs::change_stamp(path).ok().flatten();
        worker.note_written(path.to_path_buf(), stamp);
        *self.hot_reload.awaiting_note.entry(path.to_path_buf()).or_default() += 1;
    }

    /// Dispatch every event taken, in order — a song's changes once per owner after them —
    /// then the scene's and the song map's changes if they wait. World-only but for the asset
    /// manager and the audio, so the headless suite drives it.
    pub(super) fn dispatch_file_changes(&mut self, targets: &mut ReloadTargets<'_>) {
        let mut songs = SongChanges::default();
        for event in std::mem::take(&mut self.hot_reload.incoming) {
            match event {
                WatchEvent::Noted(path) => self.acknowledge_note(&path),
                WatchEvent::Change(change) if self.hot_reload.awaiting_note.contains_key(change.path()) => {}
                WatchEvent::Change(change) => self.dispatch_file_change(change, targets, &mut songs),
            }
        }
        self.dispatch_song_changes(songs, targets.audio);
        self.try_waiting_scene_reload(targets);
        self.try_waiting_song_map(targets.edit_field_focused);
    }

    fn acknowledge_note(&mut self, path: &Path) {
        let awaiting = &mut self.hot_reload.awaiting_note;
        if let Some(count) = awaiting.get_mut(path) {
            *count -= 1;
            if *count == 0 {
                awaiting.remove(path);
            }
        }
    }

    fn dispatch_file_change(&mut self, change: FileChange, targets: &mut ReloadTargets<'_>, songs: &mut SongChanges) {
        let Some(owners) = self.hot_reload.watched.get(change.path()).cloned() else { return };
        let path = change.path().to_path_buf();
        let shown = self.shown_path(&path, &owners);
        let FileChange::Changed(_) = change else {
            self.report(Severity::Info, format!("{shown} was deleted on disk"), None);
            return;
        };
        // The open scene is a scene whatever its name: one saved as `<name>.sheet.ron` reads
        // as a sheet by its suffix, and no sheet owns it. A sample only a song names is the
        // song's.
        let song_only = owners.iter().any(|owner| matches!(owner, WatchOwner::Song(_)))
            && !owners.iter().any(|owner| matches!(owner, WatchOwner::Sound(_)));
        let policy = if owners.contains(&WatchOwner::Scene) {
            ReloadPolicy::ReloadDocument
        } else if song_only {
            ReloadPolicy::ReloadSong
        } else {
            FileType::of(&path, &[]).map_or(ReloadPolicy::None, |file_type| file_type.reload_policy(&path))
        };
        self.report(Severity::Info, format!("{shown} changed on disk ({})", policy_label(policy)), None);
        match policy {
            ReloadPolicy::RecompileScript if self.editor.in_play_session() => {
                for owner in &owners {
                    if let WatchOwner::Script(source_path) = owner {
                        self.reload_script(targets.scripts, source_path);
                    }
                }
            }
            ReloadPolicy::ReloadDocument if owners.contains(&WatchOwner::Scene) => {
                if self.editor.in_play_session() {
                    self.report(Severity::Info, "The scene reloads from disk when Play stops", None);
                }
                self.hot_reload.waiting_scene = Some(path);
            }
            ReloadPolicy::ReloadSheet => {
                for owner in &owners {
                    if let WatchOwner::Sheet(png) = owner {
                        self.reload_sheet(targets.world, targets.assets, png);
                    }
                }
            }
            ReloadPolicy::ReloadSample => {
                for owner in &owners {
                    if let WatchOwner::Sound(sound) = owner {
                        self.reload_sample(targets.assets, targets.audio, sound);
                    }
                }
                songs.collect(&path, &owners);
            }
            ReloadPolicy::ReloadSong => songs.collect(&path, &owners),
            _ => {}
        }
    }

    /// The path as what named it gave it: a script's source path, a sheet's sidecar beside
    /// its PNG, the scene as it was opened.
    fn shown_path(&self, path: &Path, owners: &BTreeSet<WatchOwner>) -> String {
        match owners.iter().next() {
            Some(WatchOwner::Script(source_path)) => source_path.clone(),
            Some(WatchOwner::Sheet(png)) => engine_core::sidecar_path_for(png).display().to_string(),
            Some(WatchOwner::Sound(sound)) => sound.clone(),
            Some(WatchOwner::Scene) => self.editor.scene_path().unwrap_or(path).display().to_string(),
            Some(WatchOwner::Song(_)) => self.asset_relative(path),
            None => path.display().to_string(),
        }
    }

    fn reload_script(&mut self, scripts: &mut ScriptRunner, source_path: &str) {
        match scripts.reload_source(source_path) {
            Ok(()) => self.report(Severity::Info, format!("{source_path} reloaded"), None),
            Err(error) => {
                let location = script_error_location(&error);
                self.report(Severity::Error, error.to_string(), location);
            }
        }
    }

    /// The open scene's change, once nothing holds it: not in a play session (Stop's restore
    /// comes first), not under a gesture or a dialog. A change to a scene no longer open is
    /// dropped. A clean scene reloads, its undo history going with the old document; a dirty
    /// one asks first.
    fn try_waiting_scene_reload(&mut self, targets: &mut ReloadTargets<'_>) {
        let Some(path) = self.hot_reload.waiting_scene.clone() else { return };
        let open_scene = self.editor.scene_path().map(|scene| self.resolve_asset_path(scene));
        if open_scene.as_deref() != Some(path.as_path()) {
            self.hot_reload.waiting_scene = None;
            return;
        }
        if self.reload_is_held(targets.edit_field_focused) {
            return;
        }
        self.hot_reload.waiting_scene = None;
        if self.command_history.is_dirty() {
            self.scene_confirm.pending_action = Some(PendingSceneAction::ReloadFromDisk(path));
            return;
        }
        self.reload_scene_from_disk(targets.world, targets.assets, &path);
    }

    /// Whether a document's change must wait: a play session, a gizmo drag, a focused field
    /// or a dialog holds it.
    fn reload_is_held(&self, edit_field_focused: bool) -> bool {
        let gesture_live = self.editor.gizmo_has_priority() || edit_field_focused;
        // One rule for "a dialog is up", the prompt included: a reload under a modal would
        // replace the document the modal is asking about, or stack a confirm under it.
        self.editor.in_play_session() || gesture_live || self.dialog_up()
    }

    /// Replace the open scene with the file on disk; a file that does not load leaves the
    /// live world as it was and says why.
    pub(super) fn reload_scene_from_disk(&mut self, world: &mut World, assets: &mut AssetManager, path: &Path) {
        match self.load_scene(world, assets, path) {
            Ok(()) => self.report(Severity::Info, "Scene reloaded from disk", None),
            Err(error) => {
                let message = format!("{} changed on disk but did not load, the open scene is kept — {error}", path.display());
                self.report(Severity::Error, message, None);
            }
        }
    }
}

fn policy_label(policy: ReloadPolicy) -> &'static str {
    match policy {
        ReloadPolicy::None => "nothing to reload",
        ReloadPolicy::RecompileScript => "recompile script",
        ReloadPolicy::ReloadDocument => "reload scene",
        ReloadPolicy::ReloadSheet => "reload sheet",
        ReloadPolicy::ReloadSong => "reload song",
        ReloadPolicy::ReloadSample => "reload sample",
    }
}
